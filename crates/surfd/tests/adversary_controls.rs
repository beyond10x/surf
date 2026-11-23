//! Adversary cases of `story:view-controls-and-changes` against `surfd`'s stand-in session, which
//! case 2 runs against: what it records for a `fill` must be what the session records, or a case
//! on the stand-in passes on behaviour the session does not have.

use crate::common;

use std::io::{ErrorKind, Write};
use std::process::{Output, Stdio};
use std::thread;
use std::time::Duration;

use common::*;

/// `surf-agent <words>` as `session`, with `input` on its standard input.
fn agent_with_input(dir: &TestDir, session: &Opened, input: &[u8], words: &[&str]) -> Output {
    let mut command = agent(dir);
    command
        .args(words)
        .env("SURF_SESSION", &session.id)
        .env("SURF_TOKEN", &session.token)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for _ in 0..50 {
        match command.spawn() {
            Err(error) if error.kind() == ErrorKind::ExecutableFileBusy => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => panic!("surf-agent: {error}"),
            Ok(mut child) => {
                let mut stdin = child.stdin.take().expect("a pipe");
                stdin.write_all(input).unwrap();
                drop(stdin);
                return child.wait_with_output().unwrap();
            }
        }
    }
    panic!("the copied program stayed busy");
}

/// The stand-in's comment: "As the session does (`crates/session/src/entries.rs`): the value as
/// its length". The session writes the length of the value it set, a password's carriage returns
/// and line feeds removed (`crates/session`, `adversary_controls`: 12 bytes for `hunter2-view`, a
/// carriage return and a line feed). The stand-in measures the request's value as it came.
#[test]
fn adversary_controls_the_stand_in_writes_the_length_the_session_writes_for_a_password_ending_in_cr() {
    let dir = TestDir::new();
    let daemon = start(config(&dir));
    let session = open(&dir);
    let fill = agent_with_input(&dir, &session, b"hunter2-view\r\n", &["fill", "f2", "-"]);
    assert_eq!(fill.status.code(), Some(0), "{}", stderr(&fill));
    let entries = daemon
        .session_entries(&session.id)
        .expect("the session is listed")
        .entries;
    let value = entries
        .iter()
        .find(|entry| entry.text.starts_with("event surf.agent.FillRequested "))
        .map(|entry| entry.text.clone())
        .expect("the FillRequested entry");
    assert!(value.contains(" value=\"12 bytes\""), "{value}");
}
