//! Adversary cases of `story:view-controls-and-changes` against the session: the length the
//! `FillRequested` entry gives for a password read from standard input, which is the length the
//! view shows (`docs/view-stream.md`, "the `value` of the newest `FillRequested` entry").

#![forbid(unsafe_code)]

use std::io::Write;
use std::process::Stdio;

/// A password typed as `hunter2-view`, a carriage return and a line feed: `surf-agent` strips the
/// line feed, the session removes the carriage return from a single-line control, so the entry
/// gives the 12 bytes the control holds, and no entry or answer holds the value.
#[test]
fn adversary_controls_a_password_ending_in_cr_lf_is_its_sanitized_length() {
    let page = "<!doctype html><form method=post><label>Secret <input type=password name=pw></label>\
                </form>";
    let dir = crate::common::TestDir::new();
    let pages = crate::common::PageServer::with_body(page.as_bytes().to_vec());
    let daemon = crate::common::surfd_in_process(&dir, |_| {});
    let session = crate::common::agent_open(&dir, Some(&pages.url("/form.html")));
    let mut child = crate::common::agent(&dir)
        .args(["fill", "f1", "-"])
        .env("SURF_SESSION", &session.id)
        .env("SURF_TOKEN", &session.token)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("surf-agent runs");
    child
        .stdin
        .take()
        .expect("a piped standard input")
        .write_all(b"hunter2-view\r\n")
        .expect("the value is written");
    let output = child.wait_with_output().expect("surf-agent ends");
    assert_eq!(output.status.code(), Some(0));
    let entries = daemon
        .session_entries(&session.id)
        .expect("the session is listed")
        .entries;
    let values: Vec<&str> = entries
        .iter()
        .filter(|entry| entry.text.starts_with("event surf.agent.FillRequested "))
        .map(|entry| entry.text.as_str())
        .collect();
    assert_eq!(values.len(), 1, "{entries:#?}");
    assert!(values[0].contains(" value=\"12 bytes\""), "{}", values[0]);
    for entry in &entries {
        assert!(!entry.text.contains("hunter2"), "{}", entry.text);
    }
}
