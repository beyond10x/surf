//! Cases 11, 13 to 18 and 28: `surf-agent` against the stand-in serving `surfd.sock` under a short
//! `XDG_RUNTIME_DIR` of its own.

mod common;

use std::ffi::OsString;
use std::time::{Duration, Instant};

use common::*;
use surf_agent::cli::Environment;
use surf_agent::client::{self, Config};
use surf_agent::wire;

const ANSWER: &str = "surf 1 session 01234567-0123-4012-8012-012345678901 open\n\
did go \"http://example.org/a\\x22b\"\n\
page 200 \"text/html\" \"http://example.org/a\\x22b\" \"T\"\n";

#[test]
fn session_that_is_not_a_uuid_is_refused_and_nothing_is_sent() {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    for args in [
        vec!["look", "--session", "../x"],
        vec!["look", "--session=../x"],
    ] {
        let output = run(agent(&dir).args(&args).env("SURF_TOKEN", TOKEN));
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(
            stderr(&output).contains("not a UUID"),
            "{}",
            stderr(&output)
        );
    }
    let output = run(agent(&dir)
        .arg("look")
        .env("SURF_SESSION", "../x")
        .env("SURF_TOKEN", TOKEN));
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(dir.requests(), "");
}

#[test]
fn go_look_close_and_open_send_the_documented_request_and_print_the_answer() {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    dir.set("answer", ANSWER);
    let address = "http://example.org/a\"b";
    let cases: [(Vec<&str>, String); 4] = [
        (
            vec!["go", address],
            format!(
                "surf-wire 1\ncommand go\nsession {SESSION}\ntoken {TOKEN}\n\
                 arg url \"http://example.org/a\\x22b\"\nend\n"
            ),
        ),
        (
            vec!["look"],
            format!("surf-wire 1\ncommand look\nsession {SESSION}\ntoken {TOKEN}\nend\n"),
        ),
        (
            vec!["close"],
            format!("surf-wire 1\ncommand close\nsession {SESSION}\ntoken {TOKEN}\nend\n"),
        ),
        (
            vec!["open", address],
            "surf-wire 1\ncommand open\narg url \"http://example.org/a\\x22b\"\nend\n".to_string(),
        ),
    ];
    for (args, request) in cases {
        dir.clear_requests();
        let output = run(agent(&dir)
            .args(&args)
            .env("SURF_SESSION", SESSION.to_ascii_uppercase())
            .env("SURF_TOKEN", TOKEN));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{args:?}: {}",
            stderr(&output)
        );
        assert_eq!(output.stdout, ANSWER.as_bytes(), "{args:?}");
        assert_eq!(dir.requests(), request, "{args:?}");
    }
}

#[test]
fn answer_of_status_1_exits_1() {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    let refusal = "surf 1 session 01234567-0123-4012-8012-012345678901 open\ndid refused token\n";
    dir.set("status", "1");
    dir.set("answer", refusal);
    let output = run(agent(&dir)
        .args(["look", "--session", SESSION])
        .env("SURF_TOKEN", TOKEN));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), refusal);
}

#[test]
fn answer_of_status_3_exits_3_with_its_lines() {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    let died =
        "surf 1 session 01234567-0123-4012-8012-012345678901 died\ndid refused session-died\n";
    dir.set("status", "3");
    dir.set("answer", died);
    dir.set("reason", "the session process ended");
    let output = run(agent(&dir)
        .args(["look", "--session", SESSION])
        .env("SURF_TOKEN", TOKEN));
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(stdout(&output), died);
    assert_eq!(stderr(&output), "surf-agent: the session process ended\n");
}

#[test]
fn stand_in_that_closes_without_answering_exits_3() {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    dir.set("behaviour", "close");
    let output = run(agent(&dir)
        .args(["look", "--session", SESSION])
        .env("SURF_TOKEN", TOKEN));
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).contains("without an answer"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn stand_in_that_stalls_exits_3_within_the_timeout() {
    assert_eq!(wire::ANSWER_TIMEOUT, Duration::from_secs(75));
    assert_eq!(Config::default().answer_timeout, wire::ANSWER_TIMEOUT);
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    dir.set("behaviour", "stall");
    let env = Environment {
        runtime_dir: Some(dir.runtime().into_os_string()),
        session: Some(OsString::from(SESSION)),
        token: Some(OsString::from(TOKEN)),
        program_dir: None,
    };
    let config = Config {
        answer_timeout: Duration::from_secs(1),
        ..Config::default()
    };
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let started = Instant::now();
    let status = client::run(&[OsString::from("look")], &env, &config, &mut out, &mut err);
    let took = started.elapsed();
    assert_eq!(status, 3);
    assert!(
        took >= Duration::from_secs(1) && took < Duration::from_secs(3),
        "{took:?}"
    );
    assert_eq!(out, b"");
    assert!(String::from_utf8_lossy(&err).contains("did not answer within 1 seconds"));
}

#[test]
fn answer_over_the_limit_exits_3() {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    dir.set("behaviour", "oversize");
    let output = run(agent(&dir)
        .args(["look", "--session", SESSION])
        .env("SURF_TOKEN", TOKEN));
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).contains("longer than"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn options_win_over_the_environment() {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    let file = dir.token_file("t", TOKEN, 0o600);
    let output = run(agent(&dir)
        .args(["look", "--session", SESSION, "--token-file"])
        .arg(&file)
        .env("SURF_SESSION", OTHER_SESSION)
        .env("SURF_TOKEN", OTHER_TOKEN));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        dir.requests(),
        format!("surf-wire 1\ncommand look\nsession {SESSION}\ntoken {TOKEN}\nend\n")
    );
}
