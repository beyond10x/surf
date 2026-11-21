//! The cases of `story:agent-surfd-supervisor` that run `surfd` in the test process, through
//! `Daemon::start`, speak to it with `surf_agent::wire` and read its list through
//! `Daemon::sessions()` (decision D29). Case 17 decides on metadata alone.

mod common;

use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant, SystemTime};

use common::*;
use surf_agent::wire;
use surfd::{Identities, State};

/// The entries of the case's `surf/`, sorted.
fn surf_entries(dir: &TestDir) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir.surf())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

const ONLY_DAEMON: [&str; 3] = ["surfd.lock", "surfd.sock", "view-url"];

#[test]
fn case_01_open_answers_with_the_token_and_lists_the_session() {
    let dir = TestDir::new();
    dir.set("address", "http://example.org/");
    let daemon = start(config(&dir));
    let before = SystemTime::now();
    let session = open(&dir);
    let lines: Vec<&str> = session.answer.text.lines().collect();
    assert_eq!(lines[0], format!("surf 1 session {} open", session.id));
    assert_eq!(lines[1], format!("token {}", session.token));
    assert_eq!(&lines[2..], ["did open", "page none"]);
    assert_eq!(session.token.len(), 64);
    // The token is the stand-in's: it answers a look with it.
    assert_eq!(look(&dir, &session).status, 0);

    let sessions = daemon.sessions();
    let listed = sessions.get(&session.id).expect("the session is listed");
    assert_eq!(listed.state, State::Open);
    assert!(listed.opened >= before, "{:?}", listed.opened);
    assert_eq!(listed.pid, dir.pid_of(&session.id));
    assert_eq!(
        listed.address.as_ref().map(|a| a.as_str()),
        Some("\"http://example.org/\"")
    );
    assert_eq!(listed.ended, None);
}

#[test]
fn case_03_each_session_gets_only_its_own_requests() {
    let dir = TestDir::new();
    let _daemon = start(config(&dir));
    let first = open(&dir);
    let second = open(&dir);
    assert_ne!(first.id, second.id);
    for session in [&first, &second, &first, &second, &second] {
        let answer = look(&dir, session);
        assert_eq!(answer.status, 0, "{answer:?}");
        assert!(
            answer
                .text
                .starts_with(&format!("surf 1 session {} open\n", session.id))
        );
    }
    for (session, other) in [(&first, &second), (&second, &first)] {
        let record = dir.requests(&session.id);
        let named: Vec<&str> = record
            .lines()
            .filter_map(|line| line.strip_prefix("session "))
            .collect();
        assert!(
            !named.is_empty(),
            "the stand-in of {} got no look",
            session.id
        );
        assert!(
            named.iter().all(|id| *id == session.id),
            "the stand-in of {} got {named:?}",
            session.id
        );
        assert!(!record.contains(&other.id));
        assert_eq!(record.matches("command open\n").count(), 1);
    }
}

#[test]
fn case_06_a_session_killed_with_sigkill_is_listed_died() {
    let dir = TestDir::new();
    let daemon = start(config(&dir));
    let killed = open(&dir);
    let other = open(&dir);
    let pid = daemon.sessions().get(&killed.id).unwrap().pid;
    signal_kill(pid);
    let answer = look(&dir, &killed);
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(answer.text, refusal(&killed.id, "died", "session-died"));
    assert!(
        daemon.wait_sessions(ALLOWED, |s| s.get(&killed.id).is_some_and(
            |session| session.state == State::Died && session.ended.is_some()
        ))
    );
    assert!(
        !dir.surf().join(&killed.id).exists(),
        "its directory is left"
    );
    let answer = look(&dir, &other);
    assert_eq!(answer.status, 0, "{answer:?}");
    assert_eq!(daemon.sessions().get(&other.id).unwrap().state, State::Open);
}

#[test]
fn case_07_a_session_that_exits_while_answering_is_died() {
    let dir = TestDir::new();
    let daemon = start(config(&dir));
    let session = open(&dir);
    dir.set("behaviour", "exit");
    let answer = look(&dir, &session);
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(answer.text, refusal(&session.id, "died", "session-died"));
    assert_eq!(
        daemon.sessions().get(&session.id).unwrap().state,
        State::Died
    );
}

#[test]
fn case_08_close_ends_the_session_and_the_next_call_is_refused() {
    let dir = TestDir::new();
    let daemon = start(config(&dir));
    let session = open(&dir);
    let pid = dir.pid_of(&session.id);
    let answer = call(
        &dir,
        &request("close", Some(&session.id), Some(&session.token)),
    );
    assert_eq!(answer.status, 0, "{answer:?}");
    assert_eq!(
        answer.text,
        format!("surf 1 session {} closed\ndid close\n", session.id)
    );
    assert!(dir.stand_in_ends(pid, ALLOWED), "the stand-in still runs");
    // Closed, not died: the stand-in ended with status 0 and surfd did not kill it.
    assert!(daemon.wait_sessions(ALLOWED, |s| {
        s.get(&session.id)
            .is_some_and(|session| session.state == State::Closed)
    }));
    let answer = look(&dir, &session);
    assert_eq!(answer.status, 1, "{answer:?}");
    assert_eq!(answer.text, refusal(&session.id, "closed", "closed"));
}

#[test]
fn case_09_a_session_that_never_reports_ready_is_killed() {
    let dir = TestDir::new();
    dir.set("ready", "never");
    let mut config = config(&dir);
    config.session_ready_timeout = Duration::from_secs(1);
    let daemon = start(config);
    let started = Instant::now();
    let answer = call(&dir, &request("open", None, None));
    let took = started.elapsed();
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(answer.text, "");
    assert!(
        answer
            .reason
            .as_deref()
            .is_some_and(|r| r.contains("did not report ready within 1 seconds")),
        "{answer:?}"
    );
    assert!(took < ALLOWED, "{took:?}");
    let starts = dir.starts();
    assert_eq!(starts.len(), 1);
    assert!(dir.stand_in_ends(starts[0].0, ALLOWED), "the stand-in runs");
    assert!(!alive(starts[0].0));
    assert_eq!(surf_entries(&dir), ONLY_DAEMON);
    assert!(daemon.sessions().is_empty());
}

#[test]
fn case_10_a_session_that_stalls_times_out_and_stays_open() {
    let dir = TestDir::new();
    let mut config = config(&dir);
    config.session_answer_timeout = Duration::from_secs(1);
    let daemon = start(config);
    let session = open(&dir);
    let hold = dir.hold();
    dir.set("behaviour", "stall");
    let started = Instant::now();
    let answer = look(&dir, &session);
    let took = started.elapsed();
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(answer.text, refusal(&session.id, "open", "session-timeout"));
    assert!(took < ALLOWED, "{took:?}");
    let listed = daemon.sessions();
    let listed = listed.get(&session.id).unwrap();
    assert_eq!(listed.state, State::Open);
    assert!(alive(listed.pid));
    drop(hold);
}

#[test]
fn case_11_a_session_that_breaks_the_link_is_ended_and_died() {
    let dir = TestDir::new();
    let daemon = start(config(&dir));
    let oversize = open(&dir);
    let unprintable = open(&dir);
    for (session, behaviour) in [(&oversize, "oversize"), (&unprintable, "unprintable")] {
        dir.set("behaviour", behaviour);
        let pid = dir.pid_of(&session.id);
        let answer = look(&dir, session);
        assert_eq!(answer.status, 3, "{behaviour}: {answer:?}");
        assert_eq!(answer.text, refusal(&session.id, "died", "session-died"));
        assert!(dir.stand_in_ends(pid, ALLOWED), "{behaviour}: still runs");
        assert!(
            daemon.wait_sessions(ALLOWED, |s| s
                .get(&session.id)
                .is_some_and(|session| session.state == State::Died)),
            "{behaviour}: not listed died"
        );
    }
}

#[test]
fn case_17_the_directory_decision_refuses_another_owner() {
    let dir = TestDir::new();
    let path = dir.path.join("d");
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let metadata = fs::symlink_metadata(&path).unwrap();
    let caller = wire::caller_uid().unwrap();
    assert_eq!(surfd::directory::decide(&path, &metadata, caller), Ok(()));
    // The same directory, decided for a caller who is not its owner.
    let refused = surfd::directory::decide(&path, &metadata, caller + 1).unwrap_err();
    assert!(
        refused.contains(&format!("owned by user {caller}")),
        "{refused}"
    );
    // Metadata owned by another user id than the caller's: the root directory.
    let root = fs::symlink_metadata("/").unwrap();
    if std::os::unix::fs::MetadataExt::uid(&root) != caller {
        let refused = surfd::directory::decide("/".as_ref(), &root, caller).unwrap_err();
        assert!(refused.contains("owned by user 0"), "{refused}");
    }
}

#[test]
fn case_21_the_65th_connection_is_closed_unanswered() {
    // The story's number, not the crate's constant: a changed limit must turn this case red.
    const HELD: usize = 64;
    let dir = TestDir::new();
    let config = config(&dir);
    assert_eq!(config.connection_limit, HELD, "the default limit is not 64");
    let _daemon = start(config);
    let session = open(&dir);
    let hold = dir.hold();
    dir.set("behaviour", "stall");
    let look = request("look", Some(&session.id), Some(&session.token))
        .encode()
        .unwrap();
    let mut held = Vec::new();
    for _ in 0..HELD {
        let mut stream = UnixStream::connect(dir.socket()).unwrap();
        stream.write_all(&look).unwrap();
        held.push(stream);
    }
    let mut extra = UnixStream::connect(dir.socket()).unwrap();
    extra.set_read_timeout(Some(ALLOWED)).unwrap();
    let _ = extra.write_all(&look);
    let mut back = Vec::new();
    match extra.read_to_end(&mut back) {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::ConnectionReset => {}
        Err(error) => panic!("the 65th connection was not closed: {error}"),
    }
    assert!(back.is_empty(), "the 65th connection was answered");
    drop(hold);
    for (index, mut stream) in held.into_iter().enumerate() {
        let answer = wire::read_answer(&mut stream, wire::ANSWER_TIMEOUT)
            .unwrap_or_else(|error| panic!("connection {index}: {error}"));
        assert_eq!(answer.status, 0, "connection {index}: {answer:?}");
    }
}

#[test]
fn case_22_a_planted_link_as_the_session_directory_refuses_open() {
    let dir = TestDir::new();
    let id = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
    let mut config = config(&dir);
    config.identities = Identities::Fixed(vec![id.to_string()]);
    let daemon = start(config);
    let target = dir.path.join("t");
    fs::create_dir(&target).unwrap();
    symlink(&target, dir.surf().join(id)).unwrap();
    let answer = call(&dir, &request("open", None, None));
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(answer.text, "");
    assert!(answer.reason.is_some());
    assert_eq!(
        fs::read_dir(&target).unwrap().count(),
        0,
        "created in the target"
    );
    assert_eq!(fs::read_link(dir.surf().join(id)).unwrap(), target);
    assert!(dir.starts().is_empty(), "a process started");
    assert!(daemon.sessions().is_empty());
}

#[test]
fn case_23_a_session_directory_that_exists_refuses_open() {
    let dir = TestDir::new();
    let id = "aaaaaaaa-bbbb-4ccc-8ddd-ffffffffffff";
    let mut config = config(&dir);
    config.identities = Identities::Fixed(vec![id.to_string()]);
    let daemon = start(config);
    let planted = dir.surf().join(id);
    fs::create_dir(&planted).unwrap();
    fs::set_permissions(&planted, fs::Permissions::from_mode(0o750)).unwrap();
    fs::write(planted.join("f"), "planted").unwrap();
    let answer = call(&dir, &request("open", None, None));
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(answer.text, "");
    assert!(
        answer
            .reason
            .as_deref()
            .is_some_and(|r| r.contains("exists before surfd created it")),
        "{answer:?}"
    );
    assert_eq!(fs::read_to_string(planted.join("f")).unwrap(), "planted");
    assert_eq!(
        fs::symlink_metadata(&planted).unwrap().permissions().mode() & 0o7777,
        0o750
    );
    assert!(dir.starts().is_empty(), "a process started");
    assert!(daemon.sessions().is_empty());
}

#[test]
fn case_24_an_open_with_a_nonzero_status_leaves_no_entry() {
    let dir = TestDir::new();
    let mut config = config(&dir);
    config.kill_grace = Duration::from_secs(1);
    let daemon = start(config);
    for (status, reason) in [(2u8, "usage"), (1, "token")] {
        dir.set("open-status", &status.to_string());
        let answer = call(&dir, &request("open", None, None));
        let (pid, id) = dir.starts().last().cloned().unwrap();
        assert_eq!(answer.status, status, "{answer:?}");
        assert_eq!(
            answer.text,
            format!("surf 1 session {id} open\ndid refused {reason}\n")
        );
        assert!(answer.reason.is_some());
        assert!(
            dir.stand_in_ends(pid, ALLOWED),
            "status {status}: still runs"
        );
        assert!(
            !dir.surf().join(&id).exists(),
            "status {status}: directory left"
        );
        assert!(daemon.sessions().is_empty(), "status {status}: listed");
    }
    assert_eq!(dir.starts().len(), 2);
}

#[test]
fn case_25_an_address_is_listed_quoted_and_read_back_exactly() {
    let dir = TestDir::new();
    let address = "a\"b\n";
    dir.set("address", address);
    let daemon = start(config(&dir));
    let session = open(&dir);
    let sessions = daemon.sessions();
    let listed = sessions.get(&session.id).unwrap().address.clone().unwrap();
    assert_eq!(listed.as_str(), "\"a\\x22b\\x0a\"");
    assert!(listed.as_str().bytes().all(|b| (b' '..=b'~').contains(&b)));
    assert_eq!(surf_answer::unquote(listed.as_str()).unwrap(), address);
}
