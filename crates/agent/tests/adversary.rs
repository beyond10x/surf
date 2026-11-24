//! Adversary of `story:scripts-spec`, decision 3 and acceptance 5: the binding declares
//! `open --scripts`, and the client refuses the option, in every spelling, with status 2 and the
//! usage, and starts no session, until `story:scripts-document-port`. Neither does any other
//! command, a prefix of the option, nor the option before the command take it.

use crate::common::*;

/// Every spelling of the option on `open`: refused as the option, status 2, the usage, nothing
/// on standard output, no `surfd` started and nothing made under the runtime directory.
#[test]
fn adversary_every_spelling_of_scripts_on_open_is_refused_and_starts_nothing() {
    let dir = TestDir::new();
    for args in [
        vec!["open", "--scripts"],
        vec!["open", "--scripts", "true"],
        vec!["open", "--scripts=true"],
        vec!["open", "--scripts", "false"],
        vec!["open", "--scripts=false"],
        vec!["open", "--scripts="],
        vec!["open", "--scripts", "TRUE"],
        vec!["open", "--scripts", "1"],
        vec!["open", "--scripts", "true", "--scripts", "true"],
        vec!["open", "--scripts", "false", "--scripts", "true"],
        vec!["open", "http://127.0.0.1:9/", "--scripts", "true"],
        vec![
            "open",
            "--ca-file",
            "/nonexistent",
            "--scripts",
            "true",
            "http://127.0.0.1:9/",
        ],
    ] {
        let output = run(agent(&dir).args(&args));
        let stderr = stderr(&output);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {stderr}");
        assert_eq!(stdout(&output), "", "{args:?}");
        assert!(
            stderr.contains("--scripts is not accepted"),
            "{args:?}: {stderr}"
        );
        assert!(
            stderr.contains("usage: surf-agent open [<url>]"),
            "{args:?}: {stderr}"
        );
    }
    assert!(!dir.runtime().join("surf").exists(), "nothing was sent");
    assert!(dir.starts().is_empty(), "no surfd was started");
}

/// The option anywhere else: after `--` (an argument, not the option), on another command, as
/// a prefix or in another case of its name, and before the command. Each is a usage error,
/// status 2, and starts nothing.
#[test]
fn adversary_scripts_outside_open_or_misspelled_is_usage_and_starts_nothing() {
    let dir = TestDir::new();
    for (args, says) in [
        (vec!["open", "--", "--scripts", "true"], "usage: surf-agent"),
        (vec!["go", "--scripts", "true"], "is not an option of go"),
        (
            vec!["look", "--scripts", "true"],
            "is not an option of look",
        ),
        (vec!["close", "--scripts=true"], "is not an option of close"),
        (vec!["open", "--script", "true"], "is not an option of open"),
        (
            vec!["open", "--scripts-on", "true"],
            "is not an option of open",
        ),
        (
            vec!["open", "--Scripts", "true"],
            "is not an option of open",
        ),
        (
            vec!["--scripts", "true", "open"],
            "is an option before the command",
        ),
    ] {
        let output = run(agent(&dir).args(&args));
        let stderr = stderr(&output);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {stderr}");
        assert_eq!(stdout(&output), "", "{args:?}");
        assert!(stderr.contains(says), "{args:?}: {stderr}");
        assert!(
            stderr.contains("usage: surf-agent open [<url>]"),
            "{args:?}: {stderr}"
        );
    }
    assert!(!dir.runtime().join("surf").exists(), "nothing was sent");
    assert!(dir.starts().is_empty(), "no surfd was started");
}
