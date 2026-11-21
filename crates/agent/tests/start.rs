//! Cases 19 to 24: the runtime directory, and `open` starting `surfd`, with a copy of the built
//! `surf-agent` in a directory of the test and the stand-in beside it as `surfd`.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::*;
use surf_agent::wire;

const OPENED: &str = "surf 1 session 01234567-0123-4012-8012-012345678901 open\n\
token fedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedc\n\
did open \"http://example.org/\"\npage none\n";

#[test]
fn no_runtime_dir_exits_3_and_starts_nothing() {
    let dir = TestDir::new();
    let output = run(agent(&dir)
        .env_remove("XDG_RUNTIME_DIR")
        .args(["open", "http://example.org/"]));
    assert_eq!(output.status.code(), Some(3));
    assert!(
        stderr(&output).contains("XDG_RUNTIME_DIR is not set"),
        "{}",
        stderr(&output)
    );
    assert!(dir.starts().is_empty(), "surfd was started");
    assert_eq!(fs::read_dir(dir.runtime()).unwrap().count(), 0);
    let output = run(agent(&dir).env("XDG_RUNTIME_DIR", "").arg("open"));
    assert_eq!(output.status.code(), Some(3));
    assert!(dir.starts().is_empty(), "surfd was started");
}

#[test]
fn socket_path_over_107_bytes_exits_3() {
    let dir = TestDir::new();
    let suffix = "/surf/surfd.sock".len();
    let base = dir.path.join("l");
    let pad = |total: usize| -> PathBuf {
        let length = total - suffix - base.as_os_str().len();
        PathBuf::from(format!("{}{}", base.display(), "x".repeat(length)))
    };
    assert_eq!(wire::socket_path(&pad(107)).unwrap().as_os_str().len(), 107);
    assert_eq!(
        wire::socket_path(&pad(108)),
        Err(wire::SocketPathError::TooLong(108))
    );
    let long = pad(108);
    fs::create_dir_all(&long).unwrap();
    let output = run(agent(&dir).env("XDG_RUNTIME_DIR", &long).arg("open"));
    assert_eq!(output.status.code(), Some(3));
    assert!(
        stderr(&output).contains("108 bytes long"),
        "{}",
        stderr(&output)
    );
    assert!(
        stderr(&output).contains("at most 107"),
        "{}",
        stderr(&output)
    );
    assert_eq!(fs::read_dir(&long).unwrap().count(), 0);
    assert!(dir.starts().is_empty(), "surfd was started");
}

#[test]
fn open_with_no_listener_starts_surfd_once() {
    let dir = TestDir::new();
    dir.set("answer", OPENED);
    let output = run(agent(&dir).args(["open", "http://example.org/"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), OPENED);
    assert_eq!(dir.starts().len(), 1, "surfd started once");
    assert_eq!(
        dir.requests(),
        "surf-wire 1\ncommand open\narg url \"http://example.org/\"\nend\n"
    );
    // surf-agent did not wait for surfd to end: it still runs.
    assert!(alive(dir.starts()[0]));
}

#[test]
fn open_with_surfd_listening_starts_nothing() {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    dir.set("answer", OPENED);
    assert_eq!(dir.starts().len(), 1);
    let output = run(agent(&dir).arg("open"));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), OPENED);
    assert_eq!(dir.starts().len(), 1, "a second surfd was started");
    assert_eq!(dir.requests(), "surf-wire 1\ncommand open\nend\n");
}

#[test]
fn surfd_that_refuses_exits_3_with_its_reason() {
    let dir = TestDir::new();
    dir.set("start", "refused surf is a symbolic link\n");
    let output = run(agent(&dir).arg("open"));
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "surf-agent: surfd refused to start: surf is a symbolic link\n"
    );
    assert_eq!(dir.starts().len(), 1);
}

#[test]
fn surfd_that_writes_nothing_exits_3_within_6_seconds() {
    let dir = TestDir::new();
    dir.set("start", "silent");
    let started = Instant::now();
    let output = run(agent(&dir).arg("open"));
    assert!(
        started.elapsed() < Duration::from_secs(6),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(output.status.code(), Some(3));
    assert!(
        stderr(&output).contains("did not report ready within 5 seconds"),
        "{}",
        stderr(&output)
    );
    let starts = dir.starts();
    assert_eq!(starts.len(), 1);
    assert!(!alive(starts[0]), "the silent surfd was left running");
    assert!(!Path::new(&dir.runtime().join("surf/surfd.sock")).exists());
}
