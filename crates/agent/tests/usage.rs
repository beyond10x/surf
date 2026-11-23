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

/// `story:agent-press-submit`: `press` takes one handle, as `follow` does; without it, with
/// two, or with an option it does not take it is usage, status 2, and nothing is sent.
#[test]
fn press_takes_one_handle() {
    let dir = TestDir::new();
    for (args, says) in [
        (vec!["press"], "press needs a handle"),
        (
            vec!["press", "b1", "b2"],
            "press takes at most 1 positional word",
        ),
        (
            vec!["press", "b1", "--as", "tree"],
            "is not an option of press",
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
        assert!(printed.contains("surf-agent press <handle>"), "{printed}");
    }
    assert!(!dir.runtime().join("surf").exists(), "nothing was sent");

    let env = surf_agent::cli::Environment {
        runtime_dir: Some(dir.runtime().into()),
        session: Some(SESSION.into()),
        token: Some(TOKEN.into()),
        program_dir: None,
    };
    let call = surf_agent::cli::parse(&["press".into(), "b1".into()], &env).expect("a call");
    let request = call.request();
    assert_eq!(request.command, "press");
    let args: Vec<(&str, &str)> = request
        .args
        .iter()
        .map(|arg| (arg.name.as_str(), arg.value.as_str()))
        .collect();
    assert_eq!(args, vec![("handle", "b1")]);
}

/// `story:agent-back-forward`: `back` and `forward` take no word and no option but the session's;
/// with one it is usage, status 2, and nothing is sent. Without, the parser sends no `arg`.
#[test]
fn back_and_forward_take_no_argument() {
    let dir = TestDir::new();
    for command in ["back", "forward"] {
        for (args, says) in [
            (
                vec![command, "l1"],
                format!("{command} takes at most 0 positional word"),
            ),
            (
                vec![command, "--as", "tree"],
                format!("is not an option of {command}"),
            ),
        ] {
            let output = run(agent(&dir)
                .args(&args)
                .env("SURF_SESSION", SESSION)
                .env("SURF_TOKEN", TOKEN));
            assert_eq!(output.status.code(), Some(2), "{args:?}");
            assert_eq!(stdout(&output), "", "{args:?}");
            let printed = stderr(&output);
            assert!(printed.contains(&says), "{args:?}: {printed}");
            assert!(
                printed.contains(&format!("surf-agent {command} [--session <id>]")),
                "{printed}"
            );
        }
        let env = surf_agent::cli::Environment {
            runtime_dir: Some(dir.runtime().into()),
            session: Some(SESSION.into()),
            token: Some(TOKEN.into()),
            program_dir: None,
        };
        let call = surf_agent::cli::parse(&[command.into()], &env).expect("a call");
        let request = call.request();
        assert_eq!(request.command, command);
        assert_eq!(request.session.as_deref(), Some(SESSION));
        assert!(request.args.is_empty(), "{:?}", request.args);
    }
    assert!(!dir.runtime().join("surf").exists(), "nothing was sent");
}

/// `story:agent-change-and-log`: `log` takes at most one word, a number of actions from 1; a
/// word that is not one, two words or an option it does not take is usage, status 2, and nothing
/// is sent. `log` sends no `arg`, `log 2` sends `arg count "2"`.
#[test]
fn log_takes_a_number_from_1() {
    let dir = TestDir::new();
    for (args, says) in [
        (vec!["log", "0"], "log takes a number of actions from 1"),
        (vec!["log", "x"], "log takes a number of actions from 1"),
        (vec!["log", "-1"], "is not an option of surf-agent"),
        (vec!["log", "1", "2"], "log takes at most 1 positional word"),
        (vec!["log", "--as", "tree"], "is not an option of log"),
    ] {
        let output = run(agent(&dir)
            .args(&args)
            .env("SURF_SESSION", SESSION)
            .env("SURF_TOKEN", TOKEN));
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert_eq!(stdout(&output), "", "{args:?}");
        let printed = stderr(&output);
        assert!(printed.contains(says), "{args:?}: {printed}");
        assert!(printed.contains("surf-agent log [<n>]"), "{printed}");
    }
    assert!(!dir.runtime().join("surf").exists(), "nothing was sent");

    let env = surf_agent::cli::Environment {
        runtime_dir: Some(dir.runtime().into()),
        session: Some(SESSION.into()),
        token: Some(TOKEN.into()),
        program_dir: None,
    };
    for (words, sent) in [
        (vec!["log"], vec![]),
        (vec!["log", "2"], vec![("count", "2")]),
    ] {
        let words: Vec<std::ffi::OsString> = words.into_iter().map(Into::into).collect();
        let request = surf_agent::cli::parse(&words, &env)
            .expect("a call")
            .request();
        assert_eq!(request.command, "log");
        assert_eq!(request.session.as_deref(), Some(SESSION));
        let args: Vec<(&str, &str)> = request
            .args
            .iter()
            .map(|arg| (arg.name.as_str(), arg.value.as_str()))
            .collect();
        assert_eq!(args, sent);
    }
}

/// `story:agent-fill-pick`: `fill` and `pick` take a handle and one word; without the word it
/// is usage, status 2, and nothing is sent. `fill <handle> -` sends the value read from standard
/// input without its last line feed; a value over the bound, or not UTF-8, is usage and no
/// message repeats it. The `Debug` of a call writes the value's length only.
#[test]
fn fill_and_pick_take_a_handle_and_a_word() {
    let dir = TestDir::new();
    for (args, says) in [
        (
            vec!["fill", "f1"],
            "fill needs a value, or - for standard input",
        ),
        (vec!["pick", "f3"], "pick needs an option, on or off"),
        (vec!["fill"], "fill needs a handle"),
        (
            vec!["pick", "f3", "on", "off"],
            "pick takes at most 2 positional word",
        ),
    ] {
        let output = run(agent(&dir)
            .args(&args)
            .env("SURF_SESSION", SESSION)
            .env("SURF_TOKEN", TOKEN));
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        let printed = stderr(&output);
        assert!(printed.contains(says), "{args:?}: {printed}");
        assert!(
            printed.contains("surf-agent fill <handle> <value>|-"),
            "{printed}"
        );
    }
    assert!(!dir.runtime().join("surf").exists(), "nothing was sent");

    let env = surf_agent::cli::Environment {
        runtime_dir: Some(dir.runtime().into()),
        session: Some(SESSION.into()),
        token: Some(TOKEN.into()),
        program_dir: None,
    };
    let args = |call: &surf_agent::cli::Call| -> Vec<(String, String)> {
        call.request()
            .args
            .iter()
            .map(|arg| (arg.name.clone(), arg.value.clone()))
            .collect()
    };
    let fill = surf_agent::cli::parse(&["fill".into(), "f1".into(), "ada lovelace".into()], &env)
        .expect("a call");
    assert_eq!(
        args(&fill),
        vec![
            ("handle".to_string(), "f1".to_string()),
            ("value".to_string(), "ada lovelace".to_string())
        ]
    );
    let debug = format!("{fill:?}");
    assert!(!debug.contains("lovelace"), "{debug}");
    assert!(debug.contains("12 bytes"), "{debug}");
    let pick =
        surf_agent::cli::parse(&["pick".into(), "f6".into(), "Red".into()], &env).expect("a call");
    assert_eq!(
        args(&pick),
        vec![
            ("handle".to_string(), "f6".to_string()),
            ("option".to_string(), "Red".to_string())
        ]
    );

    let config = surf_agent::client::Config::default();
    let too_long = vec![b'Q'; surf_agent::client::STDIN_VALUE_MAX + 1];
    for (input, says) in [
        (too_long, "is longer than"),
        (vec![b'Q', b'Q', 0xff, b'\n'], "is not UTF-8"),
    ] {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let status = surf_agent::client::run_with_input(
            &["fill".into(), "f2".into(), "-".into()],
            &env,
            &config,
            &mut input.as_slice(),
            &mut out,
            &mut err,
        );
        let printed = String::from_utf8_lossy(&err);
        assert_eq!(status, 2, "{printed}");
        assert!(printed.contains(says), "{printed}");
        assert!(
            !printed.contains("QQ"),
            "the value is not repeated: {printed}"
        );
        assert!(out.is_empty());
    }
    assert!(!dir.runtime().join("surf").exists(), "nothing was sent");
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
    // No code of the client can open a terminal: nothing in its sources names one. Standard
    // input is named once, in `main.rs`, which hands it to `run_with_input` for the value of
    // `fill -` (`story:agent-fill-pick`); the read above shows a token is never read from it.
    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in fs::read_dir(sources).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        for name in ["/dev/tty", "isatty", "IsTerminal", "is_terminal"] {
            assert!(!text.contains(name), "{} names {name}", path.display());
        }
        let named = text.matches("stdin()").count();
        let allowed = usize::from(path.file_name().is_some_and(|name| name == "main.rs"));
        assert_eq!(named, allowed, "{} names stdin()", path.display());
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
