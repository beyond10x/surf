//! Cases 2, 3, 5, 6, 12, 13 and 14 of `story:agent-session-process`, on the built programs: the
//! first two through `surf-agent` and `surfd`, the others on a `surf-session` the test starts as
//! `surfd` starts it. Two more: a request slower than the read deadline is closed and the next
//! one is served, and an argument shape other than the story's exits 2 without `ready`.

#![forbid(unsafe_code)]

mod common;

use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::{
    PageServer, Started, TestDir, agent_call, agent_open, cmdline, contains, opened, path_block,
    regular_files_under, request, session_program, surfd_in_process, wire_as, wire_call, wire_open,
};
use surf_agent::wire::REQUEST_MAX;
use surfd::link::{self, LinkError, SessionState};

/// `look` with its own token on the session the test started.
fn look(session: &Started, token: &str) -> link::Answer {
    session.call(&request("look", Some(&session.id), Some(token), &[]))
}

#[test]
fn token_of_another_session_is_refused() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let first = agent_open(&dir, Some(&pages.url("/a")));
    let second = agent_open(&dir, None);
    assert_ne!(first.id, second.id);

    let before = agent_call(&dir, &["look"], &first.id, &first.token);
    assert_eq!(before.status.code(), Some(0));

    for words in [
        vec!["look"],
        vec!["go", "http://127.0.0.1:1/x"],
        vec!["close"],
    ] {
        let output = agent_call(&dir, &words, &first.id, &second.token);
        assert_eq!(output.status.code(), Some(1), "{words:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            format!("surf 1 session {} unknown\ndid refused token\n", first.id)
        );
    }

    let after = agent_call(&dir, &["look"], &first.id, &first.token);
    assert_eq!(after.status.code(), Some(0));
    assert_eq!(after.stdout, before.stdout, "look after the refusals");
}

#[test]
fn token_in_no_command_line_environment_or_file() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let session = agent_open(&dir, Some(&pages.url("/a")));
    let go = agent_call(&dir, &["go", &pages.url("/b")], &session.id, &session.token);
    assert_eq!(go.status.code(), Some(0));
    let look = agent_call(&dir, &["look"], &session.id, &session.token);
    assert_eq!(look.status.code(), Some(0));

    let token = session.token.as_bytes();
    let socket = dir.surf().join(&session.id).join("session.sock");
    let socket = socket.as_os_str().as_encoded_bytes();
    let (mut surfd, mut sessions) = (0, 0);
    for pid in dir.processes() {
        let arguments = cmdline(pid);
        if arguments
            .iter()
            .any(|argument| argument.as_slice() == socket)
        {
            sessions += 1;
        } else if arguments
            .first()
            .is_some_and(|program| program.ends_with(b"/surfd"))
        {
            surfd += 1;
        } else {
            continue;
        }
        let line = fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        let environ = fs::read(format!("/proc/{pid}/environ")).unwrap_or_default();
        assert!(
            !contains(&line, token),
            "the token is in the arguments of {pid}"
        );
        assert!(
            !contains(&environ, token),
            "the token is in the environment of {pid}"
        );
    }
    assert_eq!(
        (surfd, sessions),
        (1, 1),
        "the case's surfd and session process"
    );

    let files = regular_files_under(&dir.surf());
    for file in files {
        let bytes = fs::read(&file).unwrap_or_default();
        assert!(
            !contains(&bytes, token),
            "the token is in {}",
            file.display()
        );
    }
}

#[test]
fn idle_timeout_ends_the_session() {
    let dir = TestDir::new();
    let mut session = Started::start(&dir, &["--idle-timeout", "1"]);
    assert_eq!(session.line, "ready\n");
    session.open();
    let status = session.wait_within(Duration::from_secs(5));
    assert_eq!(
        status.map(|s| s.code()),
        Some(Some(0)),
        "the end within 5 seconds"
    );
    assert!(!session.socket.exists(), "the socket is gone");
}

#[test]
fn end_of_standard_input_ends_the_session() {
    let dir = TestDir::new();
    let mut session = Started::start(&dir, &[]);
    assert_eq!(session.line, "ready\n");
    session.open();
    drop(session.stdin.take());
    let status = session.wait_within(Duration::from_secs(10));
    assert_eq!(
        status.map(|s| s.code()),
        Some(Some(0)),
        "the end after the pipe closed"
    );
    assert!(!session.socket.exists(), "the socket is gone");
}

/// The requests of case 12; asserts each answer.
fn usage_requests(session: &Started, token: &str) {
    let before = path_block(&look(session, token).text);
    let id = session.id.as_str();
    for request in [
        request("look", Some(id), Some(token), &[("colour", "x")]),
        request("fly", Some(id), Some(token), &[]),
        request("go", Some(id), Some(token), &[]),
    ] {
        let answer = session.call(&request);
        assert_eq!(answer.status, 2, "{}: {answer:?}", request.command);
        let mut lines = answer.text.lines();
        assert_eq!(
            lines.next(),
            Some(format!("surf 1 session {id} open").as_str())
        );
        assert_eq!(
            lines.next(),
            Some("did refused usage"),
            "{}",
            request.command
        );
    }
    assert_eq!(
        path_block(&look(session, token).text),
        before,
        "the path afterwards"
    );
}

/// The requests of case 13; asserts each answer and that the same process answers `look` after.
fn malformed_requests(session: &mut Started, token: &str) {
    let answered_2_or_closed = |result: Result<link::Answer, LinkError>, what: &str| match result {
        Ok(answer) => assert_eq!(answer.status, 2, "{what}: {answer:?}"),
        Err(LinkError::Empty) => {}
        Err(error) => panic!("{what}: {error}"),
    };
    answered_2_or_closed(
        session.call_raw(b"GET / HTTP/1.1\r\n\r\nend\n"),
        "not the wire",
    );
    answered_2_or_closed(
        session.call_raw(&vec![b'a'; REQUEST_MAX + 1]),
        "one byte over",
    );
    answered_2_or_closed(
        session.call_raw(b"surf-wire 1\ncommand look\n"),
        "closed before end",
    );
    answered_2_or_closed(
        Ok(session.call(&request("open", None, None, &[]))),
        "a second open",
    );
    assert!(
        session.child.try_wait().unwrap().is_none(),
        "the session still runs"
    );
    let answer = look(session, token);
    assert_eq!(answer.status, 0, "{answer:?}");
    assert!(
        session.child.try_wait().unwrap().is_none(),
        "the same process answered"
    );
}

#[test]
fn unknown_command_and_argument_are_usage() {
    let dir = TestDir::new();
    let session = Started::start(&dir, &[]);
    let token = session.open();
    usage_requests(&session, &token);
}

#[test]
fn malformed_requests_do_not_end_the_session() {
    let dir = TestDir::new();
    let mut session = Started::start(&dir, &[]);
    let token = session.open();
    malformed_requests(&mut session, &token);
}

#[test]
fn standard_output_holds_ready_only() {
    let dir = TestDir::new();
    let mut session = Started::start(&dir, &[]);
    let token = session.open();
    usage_requests(&session, &token);
    malformed_requests(&mut session, &token);
    let close = session.call(&request("close", Some(&session.id), Some(&token), &[]));
    assert_eq!(close.status, 0);
    assert_eq!(close.state, SessionState::Closed);
    let status = session.wait_within(Duration::from_secs(10));
    assert_eq!(
        status.map(|s| s.code()),
        Some(Some(0)),
        "the end after the answer to close"
    );
    let rest = session.rest_of_stdout();
    let output = format!("{}{rest}", session.line);
    assert_eq!(output, "ready\n");
}

#[test]
fn request_slower_than_its_deadline_is_closed() {
    let dir = TestDir::new();
    let session = Started::start(&dir, &[]);
    let token = session.open();
    let mut slow = UnixStream::connect(&session.socket).unwrap();
    slow.write_all(b"surf-wire 1\ncommand look\n").unwrap();
    let started = Instant::now();
    let answer = look(&session, &token);
    assert_eq!(answer.status, 0);
    assert!(
        started.elapsed() < Duration::from_secs(50),
        "the next request waited {:?}, five times the read deadline of 10 seconds is 50",
        started.elapsed()
    );
}

#[test]
fn other_argument_shapes_exit_2() {
    let dir = TestDir::new();
    let socket = dir.session_socket("0b0e4f5a-6c7d-4e8f-9a0b-1c2d3e4f5a6b");
    let socket = socket.to_str().unwrap();
    for args in [
        vec![],
        vec!["0b0e4f5a-6c7d-4e8f-9a0b-1c2d3e4f5a6b"],
        vec!["0B0E4F5A-6C7D-4E8F-9A0B-1C2D3E4F5A6B", socket],
        vec!["not-an-identity", socket],
        vec![
            "--idle-timeout",
            "0",
            "0b0e4f5a-6c7d-4e8f-9a0b-1c2d3e4f5a6b",
            socket,
        ],
        vec![
            "--idle-timeout",
            "0b0e4f5a-6c7d-4e8f-9a0b-1c2d3e4f5a6b",
            socket,
        ],
        vec!["--verbose", "0b0e4f5a-6c7d-4e8f-9a0b-1c2d3e4f5a6b", socket],
        vec!["0b0e4f5a-6c7d-4e8f-9a0b-1c2d3e4f5a6b", socket, "extra"],
    ] {
        let output = Command::new(session_program())
            .args(&args)
            .env_clear()
            .env("XDG_RUNTIME_DIR", dir.runtime())
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(
            output.stdout.is_empty(),
            "{args:?} wrote {:?}",
            output.stdout
        );
        assert!(
            !std::path::Path::new(socket).exists(),
            "{args:?} bound the socket"
        );
    }
}

/// One request over one connection to the session socket, and every byte of its reply.
fn reply_bytes(session: &Started, bytes: &[u8]) -> Vec<u8> {
    let mut stream = UnixStream::connect(&session.socket).expect("the session listens");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    stream.write_all(bytes).unwrap();
    let _ = stream.shutdown(std::net::Shutdown::Write);
    let mut reply = Vec::new();
    stream.read_to_end(&mut reply).unwrap();
    reply
}

/// The session's page is at `address`, opened over the link; its token.
fn open_at(session: &Started, address: &str) -> String {
    let answer = session.call(&request("open", None, None, &[("url", address)]));
    assert_eq!(answer.status, 0, "open: {answer:?}");
    opened(&answer.text).token
}

/// A token that differs from `token` in its first digit.
fn other_than(token: &str) -> String {
    let first = if token.starts_with('0') { '1' } else { '0' };
    format!("{first}{}", &token[1..])
}

#[test]
fn a_refusal_on_the_session_socket_holds_no_address() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let session = Started::start(&dir, &[]);
    let path = "/kept-private?key=k3y#part";
    let address = pages.url(path);
    let token = open_at(&session, &address);
    let wrong = request("look", Some(&session.id), Some(&other_than(&token)), &[]);
    for (what, bytes, refusal) in [
        ("a wrong token", wrong.encode().unwrap(), "token"),
        (
            "a request that does not decode",
            b"GET / HTTP/1.1\r\n\r\nend\n".to_vec(),
            "usage",
        ),
    ] {
        let reply = reply_bytes(&session, &bytes);
        assert!(!reply.is_empty(), "{what}: no reply");
        assert!(
            !contains(&reply, b"kept-private") && !contains(&reply, b"k3y"),
            "{what}: the address is in the reply: {}",
            String::from_utf8_lossy(&reply)
        );
        let answer = link::Answer::decode(&reply).expect("a link answer");
        assert_eq!(answer.state, SessionState::Unknown, "{what}");
        assert_eq!((answer.address, answer.title), (None, None), "{what}");
        assert_eq!(
            answer.text,
            format!(
                "surf 1 session {} unknown\ndid refused {refusal}\n",
                session.id
            ),
            "{what}"
        );
    }
    let answer = look(&session, &token);
    assert_eq!(answer.status, 0, "{answer:?}");
    let page = format!("page 200 \"text/html\" {}", surf_answer::quote(&address));
    assert!(
        answer.text.lines().any(|line| line == page),
        "{}",
        answer.text
    );
    assert_eq!(answer.address, Some(link::Quoted::new(&address)));
}

#[test]
fn surfd_keeps_what_it_knew_after_a_refused_call() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let daemon = surfd_in_process(&dir, |_| {});
    let address = pages.url("/kept?key=k3y");
    let session = wire_open(&dir, Some(&address));
    let before = daemon.sessions().get(&session.id).cloned().expect("listed");
    assert_eq!(before.address, Some(link::Quoted::new(&address)));
    let refused = wire_call(
        &dir,
        &request(
            "look",
            Some(&session.id),
            Some(&other_than(&session.token)),
            &[],
        ),
    );
    assert_eq!(refused.status, 1, "{refused:?}");
    assert_eq!(
        refused.text,
        format!("surf 1 session {} unknown\ndid refused token\n", session.id)
    );
    let after = daemon.sessions().get(&session.id).cloned().expect("listed");
    assert_eq!(after, before, "the list after the refused call");
    let look = wire_as(&dir, &session, "look", &[]);
    assert_eq!(look.status, 0, "{look:?}");
    let page = format!("page 200 \"text/html\" {}", surf_answer::quote(&address));
    assert!(look.text.lines().any(|line| line == page), "{}", look.text);
}
