//! The cases of `story:agent-surfd-supervisor` that run the built programs: copies of
//! `examples/agent` as `surf-agent`, of the built `surfd`, and of the stand-in as
//! `surf-session`, beside each other. They assert what an answer prints, exit statuses, files
//! and processes, never the list (decision D29).

mod common;

use std::fs::{self, File};
use std::os::unix::fs::{FileTypeExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::thread;

use common::*;
use surf_agent::wire::{self, Answer, Request};
use surf_secret::Token;

/// The process ids of the case whose command name is `name`.
fn pids_named(dir: &TestDir, name: &str) -> Vec<u32> {
    let mut needle = b"STAND_IN_DIR=".to_vec();
    needle.extend_from_slice(dir.control().as_os_str().as_encoded_bytes());
    let mut pids = Vec::new();
    for entry in fs::read_dir("/proc").unwrap().flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        let comm = fs::read_to_string(entry.path().join("comm")).unwrap_or_default();
        let environ = fs::read(entry.path().join("environ")).unwrap_or_default();
        if comm.trim_end() == name
            && alive(pid)
            && environ
                .split(|&b| b == 0)
                .any(|pair| pair == needle.as_slice())
        {
            pids.push(pid);
        }
    }
    pids
}

/// A directory of the case whose path is exactly `length` bytes, mode 0700.
fn runtime_of_length(dir: &TestDir, length: usize) -> PathBuf {
    let base = dir.path.as_os_str().len() + 1;
    assert!(
        length > base,
        "the test directory is too long for {length} bytes"
    );
    let path = dir.path.join("x".repeat(length - base));
    assert_eq!(path.as_os_str().len(), length);
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

fn names_in(path: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn raw_request(command: &str, session: &str, token: &str) -> Vec<u8> {
    Request {
        command: command.to_string(),
        session: Some(session.to_string()),
        token: Some(Token::from_hex(token).unwrap()),
        args: Vec::new(),
    }
    .encode()
    .unwrap()
}

const NEVER_OPENED: &str = "0f0f0f0f-1e1e-4d2d-8c3c-4b4b4b4b4b4b";
const SOME_TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn case_02_eight_opens_at_once_start_one_surfd() {
    let dir = TestDir::new();
    bin_dir();
    let outputs: Vec<_> = thread::scope(|scope| {
        let runs: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| run(agent(&dir).arg("open"))))
            .collect();
        runs.into_iter().map(|run| run.join().unwrap()).collect()
    });
    let mut sessions = Vec::new();
    for output in &outputs {
        assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
        sessions.push(opened(Answer {
            status: 0,
            reason: None,
            text: stdout(output),
        }));
    }
    let mut ids: Vec<&str> = sessions.iter().map(|s| s.id.as_str()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 8, "the identities are not distinct");
    for session in &sessions {
        let output = agent_call(&dir, "look", session);
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        assert!(stdout(&output).starts_with(&format!("surf 1 session {} open\n", session.id)));
    }
    let lock = File::open(dir.surf().join("surfd.lock")).unwrap();
    assert!(
        matches!(lock.try_lock(), Err(fs::TryLockError::WouldBlock)),
        "the test could lock surfd.lock while surfd runs"
    );
    assert_eq!(pids_named(&dir, "surfd").len(), 1, "not one surfd runs");
}

#[test]
fn case_04_the_token_is_in_no_file_and_no_command_line() {
    let dir = TestDir::new();
    let session = agent_open(&dir);
    let token = session.token.as_bytes();
    for file in TestDir::files_under(&dir.surf()) {
        let kind = fs::symlink_metadata(&file).unwrap().file_type();
        if kind.is_socket() {
            continue;
        }
        let bytes = fs::read(&file).unwrap();
        assert!(
            !bytes.windows(token.len()).any(|w| w == token),
            "the token is in {}",
            file.display()
        );
    }
    let surfd = pids_named(&dir, "surfd");
    let stand_in = pids_named(&dir, "surf-session");
    assert_eq!(surfd.len(), 1);
    assert_eq!(stand_in, vec![dir.pid_of(&session.id)]);
    for pid in surfd.into_iter().chain(stand_in) {
        let cmdline = fs::read(format!("/proc/{pid}/cmdline")).unwrap();
        assert!(!cmdline.is_empty());
        assert!(
            !cmdline.windows(token.len()).any(|w| w == token),
            "the token is in the command line of {pid}"
        );
    }
}

#[test]
fn case_05_an_identity_never_opened_is_unknown() {
    let dir = TestDir::new();
    let surfd = Surfd::start(&dir, &[], |_| {});
    assert_eq!(surfd.line, "ready\n");
    let output = run(agent(&dir)
        .arg("look")
        .env("SURF_SESSION", NEVER_OPENED)
        .env("SURF_TOKEN", SOME_TOKEN));
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        refusal(NEVER_OPENED, "unknown", "no-such-session")
    );
    assert!(dir.starts().is_empty(), "a session was started");
    assert_eq!(names_in(&dir.surf()), ["surfd.lock", "surfd.sock"]);
}

#[test]
fn case_12_idle_exit_after_the_last_close() {
    let dir = TestDir::new();
    let mut surfd = Surfd::start(&dir, &["--idle-exit", "1"], |_| {});
    assert_eq!(surfd.line, "ready\n");
    let session = agent_open(&dir);
    let output = agent_call(&dir, "close", &session);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let status = surfd.ends_within(ALLOWED).expect("surfd still runs");
    assert!(status.success(), "{status:?}");
    assert!(
        fs::symlink_metadata(dir.socket()).is_err(),
        "surfd.sock is left"
    );
    let next = agent_open(&dir);
    assert_ne!(next.id, session.id);
    let running = pids_named(&dir, "surfd");
    assert_eq!(running.len(), 1);
    assert_ne!(running[0], surfd.pid(), "the old surfd answered");
}

#[test]
fn case_13_a_killed_surfd_leaves_no_session_behind() {
    let dir = TestDir::new();
    let mut surfd = Surfd::start(&dir, &[], |_| {});
    assert_eq!(surfd.line, "ready\n");
    let session = agent_open(&dir);
    let pid = dir.pid_of(&session.id);
    surfd.child.kill().unwrap();
    surfd.child.wait().unwrap();
    assert!(
        dir.stand_in_ends(pid, ALLOWED),
        "the session process still runs"
    );
    assert!(
        dir.surf().join(&session.id).exists(),
        "no stale directory to test"
    );
    let next = agent_open(&dir);
    assert_ne!(next.id, session.id);
    assert!(
        fs::symlink_metadata(dir.surf().join(&session.id)).is_err(),
        "the stale session directory is left"
    );
}

#[test]
fn case_14_no_runtime_dir_refuses_and_creates_nothing() {
    let dir = TestDir::new();
    let mut surfd = Surfd::start(&dir, &[], |command| {
        command.env_remove("XDG_RUNTIME_DIR");
    });
    assert!(surfd.line.starts_with("refused "), "{:?}", surfd.line);
    assert!(
        surfd.line.contains("XDG_RUNTIME_DIR is not set"),
        "{:?}",
        surfd.line
    );
    let status = surfd.ends_within(ALLOWED).expect("surfd still runs");
    assert!(!status.success());
    assert_eq!(names_in(&dir.path), ["c", "r"]);
    assert!(names_in(&dir.runtime()).is_empty());
}

#[test]
fn case_15_a_symbolic_link_under_the_runtime_dir_refuses() {
    let dir = TestDir::new();
    let target = dir.path.join("t");
    fs::create_dir(&target).unwrap();
    symlink(&target, dir.surf()).unwrap();
    let output = run(agent(&dir).arg("open"));
    assert_eq!(output.status.code(), Some(3));
    assert!(
        stderr(&output).contains(&format!("{} is a symbolic link", dir.surf().display())),
        "{}",
        stderr(&output)
    );
    assert!(
        names_in(&target).is_empty(),
        "something was created through the link"
    );
    assert_eq!(fs::read_link(dir.surf()).unwrap(), target);

    fs::remove_file(dir.surf()).unwrap();
    fs::create_dir(dir.surf()).unwrap();
    fs::set_permissions(dir.surf(), fs::Permissions::from_mode(0o700)).unwrap();
    let kept = dir.path.join("k");
    fs::create_dir(&kept).unwrap();
    fs::write(kept.join("f"), "kept").unwrap();
    let link = dir.surf().join(NEVER_OPENED);
    symlink(&kept, &link).unwrap();
    let output = run(agent(&dir).arg("open"));
    assert_eq!(output.status.code(), Some(3));
    assert!(
        stderr(&output).contains(&format!("{} is a symbolic link", link.display())),
        "{}",
        stderr(&output)
    );
    assert_eq!(fs::read_link(&link).unwrap(), kept);
    assert_eq!(names_in(&kept), ["f"]);
    assert_eq!(fs::read_to_string(kept.join("f")).unwrap(), "kept");
    assert!(dir.starts().is_empty());
}

#[test]
fn case_16_a_runtime_surf_of_mode_0755_refuses() {
    let dir = TestDir::new();
    fs::create_dir(dir.surf()).unwrap();
    fs::set_permissions(dir.surf(), fs::Permissions::from_mode(0o755)).unwrap();
    let output = run(agent(&dir).arg("open"));
    assert_eq!(output.status.code(), Some(3));
    assert!(
        stderr(&output).contains("has mode 0755"),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        fs::metadata(dir.surf()).unwrap().permissions().mode() & 0o7777,
        0o755
    );
    assert!(names_in(&dir.surf()).is_empty());
    assert!(dir.starts().is_empty());
}

#[test]
fn case_18_a_surfd_sock_that_is_a_file_refuses_and_stays() {
    let dir = TestDir::new();
    fs::create_dir(dir.surf()).unwrap();
    fs::set_permissions(dir.surf(), fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(dir.socket(), "a regular file").unwrap();
    let before = fs::metadata(dir.socket()).unwrap();
    let output = run(agent(&dir).arg("open"));
    assert_eq!(output.status.code(), Some(3));
    assert!(
        stderr(&output).contains(&format!("{} is not a socket", dir.socket().display())),
        "{}",
        stderr(&output)
    );
    let after = fs::metadata(dir.socket()).unwrap();
    assert!(after.is_file());
    assert_eq!(fs::read_to_string(dir.socket()).unwrap(), "a regular file");
    assert_eq!(after.modified().unwrap(), before.modified().unwrap());
    assert!(dir.starts().is_empty());
}

#[test]
fn case_19_a_session_socket_path_over_107_bytes_refuses_open() {
    let dir = TestDir::new();
    let long = runtime_of_length(&dir, 53);
    let output = run(agent(&dir).env("XDG_RUNTIME_DIR", &long).arg("open"));
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).contains("is 108 bytes long"),
        "{}",
        stderr(&output)
    );
    let socket = long.join("surf/surfd.sock");
    assert_eq!(socket.as_os_str().len(), 69);
    assert!(
        fs::symlink_metadata(&socket)
            .unwrap()
            .file_type()
            .is_socket()
    );
    assert_eq!(names_in(&long.join("surf")), ["surfd.lock", "surfd.sock"]);
    assert!(dir.starts().is_empty(), "a session process was started");

    let fits = runtime_of_length(&dir, 52);
    let output = run(agent(&dir).env("XDG_RUNTIME_DIR", &fits).arg("open"));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let (_, id) = dir.starts().pop().unwrap();
    let session_socket = fits.join("surf").join(id).join("session.sock");
    assert_eq!(session_socket.as_os_str().len(), 107);
}

#[test]
fn case_20_an_oversized_request_and_a_path_as_identity() {
    let dir = TestDir::new();
    let _surfd = Surfd::start(&dir, &[], |_| {});
    let before_case = names_in(&dir.path);
    let before_runtime = names_in(&dir.runtime());
    let mut big = b"surf-wire 1\ncommand look\n".to_vec();
    big.resize(wire::REQUEST_MAX + 1, b'x');
    assert!(
        raw(&dir, &big).is_empty(),
        "the oversized request was answered"
    );

    let back = raw(&dir, b"surf-wire 1\ncommand look\nsession ../x\nend\n");
    let answer = Answer::decode(&back).unwrap();
    assert_eq!(answer.status, 2);
    assert_eq!(answer.text, "");
    assert!(answer.reason.is_some());

    assert_eq!(names_in(&dir.path), before_case);
    assert_eq!(names_in(&dir.runtime()), before_runtime);
    assert_eq!(names_in(&dir.surf()), ["surfd.lock", "surfd.sock"]);
    assert!(dir.starts().is_empty());

    let back = raw(&dir, &raw_request("look", NEVER_OPENED, SOME_TOKEN));
    let answer = Answer::decode(&back).unwrap();
    assert_eq!(answer.status, 3);
    assert_eq!(
        answer.text,
        refusal(NEVER_OPENED, "unknown", "no-such-session")
    );
}

#[test]
fn case_26_an_identity_in_upper_case_is_read_in_lower_case() {
    let dir = TestDir::new();
    let _surfd = Surfd::start(&dir, &[], |_| {});
    let upper = NEVER_OPENED.to_uppercase();
    assert_ne!(upper, NEVER_OPENED);
    let back = raw(&dir, &raw_request("look", &upper, SOME_TOKEN));
    let answer = Answer::decode(&back).unwrap();
    assert_eq!(answer.status, 3);
    assert_eq!(
        answer.text,
        refusal(NEVER_OPENED, "unknown", "no-such-session")
    );

    let session = agent_open(&dir);
    let upper = session.id.to_uppercase();
    assert_ne!(upper, session.id);
    let back = raw(&dir, &raw_request("look", &upper, &session.token));
    let answer = Answer::decode(&back).unwrap();
    assert_eq!(answer.status, 0, "{answer:?}");
    assert!(
        answer
            .text
            .starts_with(&format!("surf 1 session {} open\n", session.id)),
        "{}",
        answer.text
    );
    assert!(
        dir.requests(&session.id)
            .contains(&format!("session {upper}\n")),
        "the request did not reach the session"
    );
}
