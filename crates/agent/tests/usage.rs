//! Cases 12 and 25 to 27: usage errors, and the three markers of `cli/surf-agent.yaml` the
//! client refuses.

use crate::common;

use std::fs::{self, File};
use std::io::{Seek, Write};
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

use common::*;

#[test]
fn unknown_command_exits_2_with_the_usage() {
    let dir = TestDir::new();
    for args in [vec!["peek"], vec![], vec!["--session", SESSION, "look"]] {
        let output = run(agent(&dir).args(&args));
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert_eq!(stdout(&output), "");
        assert!(
            stderr(&output).contains("usage: surf-agent open [<url>]"),
            "{}",
            stderr(&output)
        );
    }
    assert!(!dir.runtime().join("surf").exists());
}

/// A standard input that is a file the test holds open: a read by the child would move the
/// offset the two share.
fn stdin_file(dir: &TestDir) -> (File, File) {
    let path = dir.path.join("stdin");
    let mut file = File::create(&path).unwrap();
    file.write_all(format!("{TOKEN}\n").as_bytes()).unwrap();
    let file = File::open(&path).unwrap();
    let shared = file.try_clone().unwrap();
    (file, shared)
}

#[test]
fn token_stdin_exits_2_with_the_usage_and_reads_no_standard_input() {
    let dir = TestDir::new();
    let (mut file, shared) = stdin_file(&dir);
    let output = run(agent(&dir)
        .args(["look", "--session", SESSION, "--token-stdin"])
        .stdin(Stdio::from(shared)));
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("--token-stdin is not accepted"),
        "{}",
        stderr(&output)
    );
    assert!(
        stderr(&output).contains("usage: surf-agent"),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        file.stream_position().unwrap(),
        0,
        "standard input was read"
    );
}

#[test]
fn token_prompt_exits_2_with_the_usage_and_opens_no_terminal() {
    let dir = TestDir::new();
    let (mut file, shared) = stdin_file(&dir);
    let started = Instant::now();
    let output = run(agent(&dir)
        .args(["look", "--session", SESSION, "--token-prompt"])
        .stdin(Stdio::from(shared)));
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("--token-prompt is not accepted"),
        "{}",
        stderr(&output)
    );
    assert!(
        stderr(&output).contains("usage: surf-agent"),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        file.stream_position().unwrap(),
        0,
        "standard input was read"
    );
    // No code of the client can open a terminal: nothing in its sources names one.
    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in fs::read_dir(sources).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        for name in ["/dev/tty", "isatty", "IsTerminal", "is_terminal", "stdin()"] {
            assert!(!text.contains(name), "{} names {name}", path.display());
        }
    }
}

#[test]
fn session_dir_exits_2_and_touches_no_path() {
    let dir = TestDir::new();
    assert!(!Path::new("/x").exists(), "this case needs /x to be absent");
    let before = fs::metadata(dir.runtime()).unwrap().modified().unwrap();
    for args in [
        vec!["--session-dir", "/x", "look", "--session", SESSION],
        vec!["look", "--session", SESSION, "--session-dir", "/x"],
        vec!["--session-dir=/x", "open"],
    ] {
        let output = run(agent(&dir).args(&args).env("SURF_TOKEN", TOKEN));
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(
            stderr(&output).contains("--session-dir is not accepted"),
            "{}",
            stderr(&output)
        );
    }
    assert!(!Path::new("/x").exists());
    assert_eq!(fs::read_dir(dir.runtime()).unwrap().count(), 0);
    assert_eq!(
        fs::metadata(dir.runtime()).unwrap().modified().unwrap(),
        before
    );
    assert!(dir.starts().is_empty());
}
