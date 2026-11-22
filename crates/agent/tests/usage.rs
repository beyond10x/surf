//! Cases 12 and 25 to 27: usage errors, and the three markers of `cli/surf-agent.yaml` the
//! client refuses. Since `story:page-text-look`, its case 7: `--as` and `--from-line`.

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

/// Case 7 of `story:page-text-look`: `--as` and `--from-line` used wrongly are usage, status 2,
/// and nothing is sent (no `surfd` is started; the session and token are valid).
#[test]
fn as_and_from_line_used_wrongly_exit_2_and_send_nothing() {
    let dir = TestDir::new();
    for (args, says) in [
        (vec!["look", "--as", "pdf"], "--as takes tree or markdown"),
        (
            vec!["look", "--from-line", "3"],
            "--from-line is taken only beside --as",
        ),
        (
            vec!["look", "--as", "tree", "--from-line", "0"],
            "--from-line takes a line number from 1",
        ),
        (
            vec!["look", "--as", "tree", "--from-line", "x"],
            "--from-line takes a line number from 1",
        ),
        (
            vec!["open", "--as", "tree"],
            "open takes --as only with an address",
        ),
        (vec!["close", "--as", "tree"], "is not an option of close"),
        (
            vec!["look", "--as", "tree", "--as", "markdown"],
            "--as is given twice",
        ),
    ] {
        let output = run(agent(&dir)
            .args(&args)
            .env("SURF_SESSION", SESSION)
            .env("SURF_TOKEN", TOKEN));
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert_eq!(stdout(&output), "", "{args:?}");
        assert!(
            stderr(&output).contains(says),
            "{args:?}: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).contains("[--as tree|markdown [--from-line <n>]]"),
            "{}",
            stderr(&output)
        );
    }
    assert!(!dir.runtime().join("surf").exists(), "nothing was sent");
    assert!(dir.starts().is_empty());
}

/// `story:agent-follow-links`: `follow` takes one handle; without it, with two, or with an
/// option it does not take it is usage, status 2, and nothing is sent. With one handle the
/// parser sends `arg handle` and nothing else.
#[test]
fn follow_takes_one_handle() {
    let dir = TestDir::new();
    for (args, says) in [
        (vec!["follow"], "follow needs a handle"),
        (
            vec!["follow", "l1", "l2"],
            "follow takes at most 1 positional word",
        ),
        (
            vec!["follow", "l1", "--as", "tree"],
            "is not an option of follow",
        ),
    ] {
        let output = run(agent(&dir)
            .args(&args)
            .env("SURF_SESSION", SESSION)
            .env("SURF_TOKEN", TOKEN));
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert_eq!(stdout(&output), "", "{args:?}");
        let printed = stderr(&output);
        assert!(printed.contains(says), "{args:?}: {printed}");
        assert!(printed.contains("surf-agent follow <handle>"), "{printed}");
    }
    assert!(!dir.runtime().join("surf").exists(), "nothing was sent");

    let env = surf_agent::cli::Environment {
        runtime_dir: Some(dir.runtime().into()),
        session: Some(SESSION.into()),
        token: Some(TOKEN.into()),
        program_dir: None,
    };
    let call = surf_agent::cli::parse(&["follow".into(), "l3".into()], &env).expect("a call");
    let request = call.request();
    assert_eq!(request.command, "follow");
    assert_eq!(request.session.as_deref(), Some(SESSION));
    let args: Vec<(&str, &str)> = request
        .args
        .iter()
        .map(|arg| (arg.name.as_str(), arg.value.as_str()))
        .collect();
    assert_eq!(args, vec![("handle", "l3")]);
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
