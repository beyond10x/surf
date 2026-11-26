//! A first line that is not `ready` takes its own path in `open`, apart from a session that
//! closed its standard output with no byte: the answer says that it wrote another line and never
//! that the start failed. The cases of `adversary_2.rs` on a session that died by its own signal
//! give the same answer on both paths, so they cannot tell them apart. Each case starts one
//! `surfd` in the test process with a POSIX sh script as its session program.

use crate::common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use common::*;

/// An executable POSIX sh script `session` in the case's directory, `body` after its first line.
fn session(dir: &TestDir, body: &str) -> PathBuf {
    let path = dir.path.join("session");
    fs::write(&path, format!("#!/bin/sh\n{body}")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// The session writes a first line that is not `ready` and exits by itself, with no signal.
/// `surfd` reads a byte, so the answer is the one of another first line, with no `note ended`
/// line; a first line taken as no byte would read as a start that ended by an exit (`cannot
/// start`).
#[test]
fn another_first_line_then_an_exit_is_answered_as_another_line() {
    let dir = TestDir::new();
    let mut config = config(&dir);
    config.session_program = session(&dir, "printf 'x\\n'\nexit 3\n");
    let _daemon = start(config);
    let answer = call(&dir, &request("open", None, None));
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(
        answer.reason.as_deref(),
        Some("the session ended or wrote another line before it reported ready"),
        "{answer:?}"
    );
    assert_eq!(answer.text, "", "{answer:?}");
}
