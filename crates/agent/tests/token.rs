//! Cases 3 to 10 and 29: where the token comes from, what is refused, and that it is never
//! printed.

use crate::common;

use std::ffi::OsString;
use std::os::unix::fs::symlink;

use common::*;
use surf_agent::cli::{self, Environment};

fn look_with_token_file(dir: &TestDir, path: &std::path::Path) -> std::process::Output {
    run(agent(dir)
        .args(["look", "--session", SESSION, "--token-file"])
        .arg(path))
}

fn look_with_surf_token(dir: &TestDir, token: &str) -> std::process::Output {
    run(agent(dir)
        .args(["look", "--session", SESSION])
        .env("SURF_TOKEN", token))
}

#[test]
fn token_from_surf_token() {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    let output = look_with_surf_token(&dir, TOKEN);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(
        dir.requests().contains(&format!("\ntoken {TOKEN}\n")),
        "{}",
        dir.requests()
    );
}

#[test]
fn token_from_a_file_of_mode_0600() {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    for (name, contents) in [("a", TOKEN.to_string()), ("b", format!("{TOKEN}\n"))] {
        dir.clear_requests();
        let file = dir.token_file(name, &contents, 0o600);
        let output = look_with_token_file(&dir, &file);
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        assert!(
            dir.requests().contains(&format!("\ntoken {TOKEN}\n")),
            "{}",
            dir.requests()
        );
    }
}

#[test]
fn token_file_of_mode_0644_is_refused() {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    let file = dir.token_file("t", TOKEN, 0o644);
    let output = look_with_token_file(&dir, &file);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("mode 0644"), "{}", stderr(&output));
    assert_eq!(dir.requests(), "");
    assert!(token_windows_in(&stderr(&output)).is_empty());
}

#[test]
fn token_file_that_is_a_symbolic_link_is_refused() {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    let target = dir.token_file("target", TOKEN, 0o600);
    let link = dir.path.join("link");
    symlink(&target, &link).unwrap();
    let output = look_with_token_file(&dir, &link);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("symbolic link"),
        "{}",
        stderr(&output)
    );
    assert_eq!(dir.requests(), "");
}

/// A malformed token, given by `SURF_TOKEN` and by a file of mode 0600: exit 2, nothing sent.
fn refused_both_ways(token: &str) {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    let output = look_with_surf_token(&dir, token);
    assert_eq!(
        output.status.code(),
        Some(2),
        "SURF_TOKEN: {}",
        stderr(&output)
    );
    let file = dir.token_file("t", token, 0o600);
    let output = look_with_token_file(&dir, &file);
    assert_eq!(
        output.status.code(),
        Some(2),
        "token file: {}",
        stderr(&output)
    );
    assert_eq!(dir.requests(), "");
}

#[test]
fn token_of_63_digits_is_refused() {
    refused_both_ways(&TOKEN[..63]);
}

#[test]
fn token_of_65_digits_is_refused() {
    refused_both_ways(&format!("{TOKEN}a"));
}

#[test]
fn token_in_upper_case_is_refused() {
    refused_both_ways(&TOKEN.to_ascii_uppercase());
}

#[test]
fn token_as_an_argument_word_is_refused() {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    for args in [
        vec!["look", "--session", SESSION, "--token", TOKEN],
        vec!["look", "--session", SESSION, &format!("--token={TOKEN}")],
        vec!["look", "--session", SESSION, TOKEN],
        vec![TOKEN, "look"],
    ] {
        let output = run(agent(&dir).args(&args));
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        let printed = format!("{}{}", stdout(&output), stderr(&output));
        assert!(printed.contains("usage: surf-agent"), "{printed}");
        assert_eq!(
            token_windows_in(&printed),
            Vec::<String>::new(),
            "{printed}"
        );
    }
    assert_eq!(dir.requests(), "");
}

#[test]
fn token_not_in_debug_output() {
    let env = Environment {
        token: Some(OsString::from(TOKEN)),
        session: Some(OsString::from(SESSION)),
        ..Environment::default()
    };
    let call = cli::parse(&[OsString::from("look")], &env).expect("a call");
    let call_debug = format!("{call:?}");
    let request_debug = format!("{:?}", call.request());
    let env_debug = format!("{env:?}");
    for debug in [&call_debug, &request_debug, &env_debug] {
        assert_eq!(token_windows_in(debug), Vec::<String>::new(), "{debug}");
    }
    assert!(call_debug.contains("Token(..)"), "{call_debug}");

    let dir = TestDir::new();
    let output = run(agent(&dir)
        .args(["look", "--session", "../x"])
        .env("SURF_TOKEN", TOKEN));
    assert_eq!(output.status.code(), Some(2));
    let printed = format!("{}{}", stdout(&output), stderr(&output));
    assert!(printed.contains("not a UUID"), "{printed}");
    assert_eq!(
        token_windows_in(&printed),
        Vec::<String>::new(),
        "{printed}"
    );
}
