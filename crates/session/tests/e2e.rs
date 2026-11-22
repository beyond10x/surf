//! Case 1 of `story:agent-session-process`, alone: the built `surf-agent`, `surfd` and
//! `surf-session` against a page server on `127.0.0.1`, each answer compared with its golden
//! text. With `SURF_BIN_DIR` set it builds nothing and runs the three programs of that
//! directory, so the release workflow runs the same case on the packed binaries. Since
//! `story:page-text-look`, its case 11: `go <url> --as markdown` prints page text in UTF-8.

#![forbid(unsafe_code)]

use crate::common;

use std::time::Duration;

use common::{
    PageServer, TestDir, agent, agent_call, cmdline, contains, entries_under, golden, opened,
    placeholders, run, stderr, stdout,
};
use surf_test_process::ended_within;

/// The process id of the session process of `id`, found by its identity in its arguments.
fn session_pid(dir: &TestDir, id: &str) -> u32 {
    dir.processes()
        .into_iter()
        .find(|&pid| {
            cmdline(pid)
                .iter()
                .any(|argument| contains(argument, id.as_bytes()))
        })
        .unwrap_or_else(|| panic!("no session process of {id} runs"))
}

#[test]
fn e2e_open_go_look_close() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let a = pages.url("/a");
    let b = pages.url("/b");

    let output = run(agent(&dir).args(["open", &a]));
    assert_eq!(output.status.code(), Some(0), "open: {}", stderr(&output));
    let session = opened(&stdout(&output));
    let (id, token) = (session.id.as_str(), session.token.as_str());
    let pid = session_pid(&dir, id);
    let mut answers = vec![("open", stdout(&output))];

    for (name, words) in [
        ("go", vec!["go", b.as_str()]),
        ("look", vec!["look"]),
        ("close", vec!["close"]),
    ] {
        let output = agent_call(&dir, &words, id, token);
        assert_eq!(output.status.code(), Some(0), "{name}: {}", stderr(&output));
        answers.push((name, stdout(&output)));
    }

    for (name, text) in &answers {
        assert_eq!(
            placeholders(text, id, token, pages.port),
            golden(&format!("{name}.txt")),
            "the answer of {name}"
        );
        surf_answer::parse(text)
            .unwrap_or_else(|error| panic!("the answer of {name} does not parse: {error:?}"));
    }

    let output = agent_call(&dir, &["look"], id, token);
    assert_eq!(output.status.code(), Some(1), "look after close");
    assert_eq!(
        stdout(&output),
        format!("surf 1 session {id} closed\ndid refused closed\n")
    );
    // 5 s: the session process ends after its answer to `close` and surfd reaps it after that;
    // no answer the test read waits for either.
    if let Err(error) = ended_within(pid, Duration::from_secs(5)) {
        panic!("the session process {pid} still runs: {error}");
    }
    let left = entries_under(&dir.surf().join(id));
    assert!(
        left.is_empty(),
        "left under the session's directory: {left:?}"
    );
}

/// Case 11 of `story:page-text-look` (decision D1 B): page text outside ASCII reaches the agent
/// as UTF-8 through `surf-session`, `surfd` and `surf-agent`.
#[test]
fn e2e_go_as_markdown_prints_utf_8() {
    let dir = TestDir::new();
    let pages = PageServer::with_body(b"<p>K&auml;se</p>".to_vec());
    let session = common::agent_open(&dir, None);
    let (id, token) = (session.id.as_str(), session.token.as_str());
    let url = pages.url("/k");
    let output = agent_call(&dir, &["go", &url, "--as", "markdown"], id, token);
    assert_eq!(output.status.code(), Some(0), "go: {}", stderr(&output));
    let text = stdout(&output);
    assert!(
        text.ends_with(
            "note text: no-css\ntext markdown 1 of 1 lines from 1, 8 of 8 bytes\n> K\u{e4}se\n"
        ),
        "{text}"
    );
    assert!(contains(&output.stdout, b"> K\xc3\xa4se\n"));
    let output = agent_call(&dir, &["close"], id, token);
    assert_eq!(output.status.code(), Some(0), "close: {}", stderr(&output));
}
