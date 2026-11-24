//! The adversary of `story:agent-skill-definition`: what `help_for_agent` does not hold.
//!
//! Sections 1 and 2 of the definition are fixed lines of the binary, and no test of
//! `help_for_agent` reads them beyond their bytes: a line dropped, or the lines of section 2
//! printed under section 1, leaves it green. Here they are held to the story's Change and to the
//! sources they speak of: the parser's commands, `cli::USAGE`, the golden `open.txt` and the
//! "Page" block of `docs/agent-answer.md`, each read when the test runs.
//!
//! And the words: `help --for-agent`, exactly, prints the definition whatever the environment
//! holds; every other form near it is a usage error; none of them starts `surfd`, sends a
//! request or writes to the runtime directory.

use crate::common;

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use common::*;
use surf_agent::cli::{COMMANDS, USAGE};

/// The six section lines, in their order (the story's Change).
const SECTION_LINES: [&str; 6] = [
    "## 1. surf",
    "## 2. the session",
    "## 3. commands",
    "## 4. the answer",
    "## 5. untrusted text",
    "## 6. worked calls",
];

/// The file at `path` from the repository root, read now.
fn read(path: &str) -> String {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    fs::read_to_string(&file).unwrap_or_else(|error| panic!("{}: {error}", file.display()))
}

/// The standard output of `surf-agent help --for-agent` with an empty environment.
fn printed() -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_surf-agent"))
        .env_clear()
        .args(["help", "--for-agent"])
        .output()
        .expect("surf-agent runs");
    assert_eq!(output.status.code(), Some(0));
    String::from_utf8(output.stdout).expect("the definition is UTF-8")
}

/// The lines of section `number` (from 1): after its section line, up to the next section line,
/// or for section 6 up to the last line of the definition.
fn section(text: &str, number: usize) -> Vec<&str> {
    let all: Vec<&str> = text.lines().collect();
    let at = |line: &str| {
        all.iter()
            .position(|printed| *printed == line)
            .unwrap_or_else(|| panic!("no section line {line:?}"))
    };
    let start = at(SECTION_LINES[number - 1]) + 1;
    let end = match SECTION_LINES.get(number) {
        Some(next) => at(next),
        None => all.len() - 1,
    };
    all[start..end].to_vec()
}

/// Section 2: `open` first, the two exports, the token never an argument word, `close` at the
/// end (the story's Change); the head line and the token line it quotes are those of the golden
/// `open.txt`; every command it names is a command of the parser, every option and variable one
/// of `cli::USAGE`. Section 1: the line of a scheme surf does not fetch is the form of a failed
/// load of the "Page" block, with a reason of its table.
#[test]
fn sections_1_and_2_hold_what_the_story_names() {
    let text = printed();

    let two = section(&text, 2);
    assert!(!two.is_empty(), "section 2 is empty");
    assert!(
        two[0].contains("surf-agent open"),
        "section 2, line 1 does not start the session with surf-agent open: {:?}",
        two[0]
    );
    let last = two[two.len() - 1];
    assert!(
        last.contains("surf-agent close"),
        "section 2 does not end with surf-agent close: {last:?}"
    );
    let joined = two.join("\n");
    for export in ["export SURF_SESSION=", "export SURF_TOKEN="] {
        assert!(joined.contains(export), "section 2 does not say {export:?}");
    }
    assert!(
        two.iter()
            .any(|line| line.contains("never an argument word")),
        "section 2 does not say that the token is never an argument word"
    );
    let open = read("crates/session/tests/golden/open.txt");
    for golden in open.lines().take(2) {
        assert!(
            joined.contains(golden),
            "section 2 does not show the line {golden:?} of open.txt"
        );
    }
    for line in &two {
        for (at, _) in line.match_indices("surf-agent ") {
            let word: String = line[at + "surf-agent ".len()..]
                .chars()
                .take_while(char::is_ascii_lowercase)
                .collect();
            assert!(
                COMMANDS.iter().any(|command| command.name == word),
                "section 2 names surf-agent {word:?}, which is no command of the parser"
            );
        }
    }
    for word in joined.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_')) {
        if word.starts_with("--") || word.starts_with("SURF_") {
            assert!(
                USAGE.contains(word),
                "section 2 names {word}, the usage does not"
            );
        }
    }

    let one = section(&text, 1);
    let document = read("docs/agent-answer.md");
    let form = document
        .lines()
        .find(|line| line.starts_with("page failed "))
        .expect("the form page failed of docs/agent-answer.md");
    assert!(
        document.contains("| `UnsupportedScheme` |"),
        "UnsupportedScheme is no reason of the table of the Page block"
    );
    let expected = form.replace("<reason>", "UnsupportedScheme");
    assert!(
        one.iter().any(|line| line.contains(&expected)),
        "section 1 does not name the line {expected:?}"
    );
}

/// `help --for-agent` with a session, a token and a runtime directory in the environment prints
/// the same bytes as with none, and so it does with `XDG_RUNTIME_DIR` unset or naming a file. Every
/// other form near it exits 2 with the usage and prints nothing. No form starts `surfd`, sends a
/// request or makes anything in the runtime directory.
#[test]
fn only_the_exact_words_print_the_definition() {
    let reference = printed();
    let dir = TestDir::new();
    let file = dir.token_file("t", TOKEN, 0o600);

    let with_session = |args: &[&str]| -> Output {
        run(agent(&dir)
            .env("SURF_SESSION", SESSION)
            .env("SURF_TOKEN", TOKEN)
            .args(args))
    };

    let output = with_session(&["help", "--for-agent"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_eq!(stdout(&output), reference, "with a session and a token set");

    for runtime in [None, Some(file.as_os_str())] {
        let mut command = Command::new(bin_dir().join("surf-agent"));
        command.env_clear().args(["help", "--for-agent"]);
        if let Some(runtime) = runtime {
            command.env("XDG_RUNTIME_DIR", runtime);
        }
        let output = run(&mut command);
        assert_eq!(output.status.code(), Some(0), "XDG_RUNTIME_DIR {runtime:?}");
        assert_eq!(stdout(&output), reference, "XDG_RUNTIME_DIR {runtime:?}");
    }

    for args in [
        vec!["help", "--For-Agent"],
        vec!["help", "--FOR-AGENT"],
        vec!["HELP", "--for-agent"],
        vec!["--for-agent", "help"],
        vec!["--for-agent"],
        vec!["--help"],
        vec!["-h"],
        vec!["help", "for-agent"],
        vec!["help", "--for-agent="],
        vec!["help", "--", "--for-agent"],
        vec!["help", "--for-agent", "--session", SESSION],
        vec!["--session", SESSION, "help", "--for-agent"],
        vec!["help", "--for-agent", ""],
    ] {
        let output = with_session(&args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            stdout(&output)
        );
        assert_eq!(stdout(&output), "", "{args:?}");
        assert!(
            stderr(&output).contains(USAGE),
            "{args:?}: {}",
            stderr(&output)
        );
    }

    assert_eq!(dir.starts(), Vec::<u32>::new(), "a surfd was started");
    assert_eq!(dir.requests(), "", "a request was sent");
    let made: Vec<_> = fs::read_dir(dir.runtime())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert!(made.is_empty(), "the runtime directory holds {made:?}");
}
