//! Case 5 of `story:tls-session-anchors`, and the request of `open --ca-file`: the client reads
//! the file of `--ca-file` before it connects, refuses one `surf-tls` refuses as an invalid input
//! (exit 2 and the usage, as `docs/agent-answer.md` gives a usage error) with nothing connected
//! and no session made, and sends the bytes of a file it takes as `arg anchors` of the `open`
//! request. With a copy of the built `surf-agent` and the stand-in beside it as `surfd`. The
//! certificates are those of `crates/tls-test-certs/certs/`.

use crate::common;

use std::fs;
use std::path::{Path, PathBuf};

use common::*;

/// The certificate case `case` of `crates/tls-test-certs/certs/`.
fn case(case: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tls-test-certs/certs")
        .join(case)
}

/// The most bytes a file of extra anchors may hold (`docs/design/tls.md` item 8).
const FILE_MAX: usize = 32 * 1024;

/// The four hostile files of case 5, each with what the refusal names: a path to no file, a
/// file of 32 KiB and one byte, a file holding a leaf that is no CA (the first block of the
/// chain of `ok-ecdsa-p256`), and a file holding a `PRIVATE KEY` block (refused by its label
/// before its content is decoded, so the content is a placeholder).
fn hostile(dir: &TestDir) -> Vec<(PathBuf, &'static str)> {
    let missing = dir.path.join("missing.pem");
    let large = dir.path.join("large.pem");
    let root = fs::read(case("ok-ecdsa-p256").join("root.pem")).unwrap();
    let mut bytes = root.repeat(FILE_MAX / root.len() + 1);
    bytes.truncate(FILE_MAX + 1);
    fs::write(&large, &bytes).unwrap();
    let leaf = dir.path.join("leaf.pem");
    let chain = fs::read_to_string(case("ok-ecdsa-p256").join("chain.pem")).unwrap();
    let end = "-----END CERTIFICATE-----\n";
    let first = &chain[..chain.find(end).expect("a certificate block") + end.len()];
    fs::write(&leaf, first).unwrap();
    let key = dir.path.join("key.pem");
    fs::write(
        &key,
        "-----BEGIN PRIVATE KEY-----\nZm9v\n-----END PRIVATE KEY-----\n",
    )
    .unwrap();
    vec![
        (missing, "the --ca-file file cannot be read"),
        (large, "TooLarge"),
        (leaf, "NotACa: certificate 0 is not a CA"),
        (key, "NotACertificate: block 0 is not a CERTIFICATE"),
    ]
}

/// `surf-agent open --ca-file <file> https://localhost:1/` for each hostile file: exit 2, the
/// refusal and the usage on standard error, without the path, and nothing on standard output.
fn refuse_each(dir: &TestDir) {
    for (file, names) in hostile(dir) {
        let output = run(agent(dir)
            .arg("open")
            .arg("--ca-file")
            .arg(&file)
            .arg("https://localhost:1/"));
        let err = stderr(&output);
        assert_eq!(output.status.code(), Some(2), "{}: {err}", file.display());
        assert_eq!(stdout(&output), "", "{}", file.display());
        assert!(err.contains(names), "{}: {err}", file.display());
        assert!(err.contains("usage: surf-agent open"), "{err}");
        assert!(
            !err.contains(file.to_str().unwrap()),
            "the message repeats the path: {err}"
        );
    }
}

#[test]
fn ca_file_case_5_a_refused_file_starts_no_surfd_and_makes_no_session() {
    let dir = TestDir::new();
    refuse_each(&dir);
    assert!(dir.starts().is_empty(), "surfd was started");
    assert!(
        !dir.runtime().join("surf").exists(),
        "$XDG_RUNTIME_DIR/surf/ was made"
    );
}

#[test]
fn ca_file_case_5_a_refused_file_sends_nothing_to_a_running_surfd() {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    refuse_each(&dir);
    assert_eq!(dir.requests(), "", "a request reached surfd");
    let left: Vec<_> = fs::read_dir(dir.runtime().join("surf"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(left, ["surfd.sock"], "a session was made");
}

/// A file the client takes travels in the `open` request as `arg anchors`, its bytes quoted;
/// the path does not.
#[test]
fn ca_file_open_sends_the_bytes_of_the_file_as_arg_anchors() {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    let file = case("ok-ecdsa-p256").join("root.pem");
    let pem = fs::read_to_string(&file).unwrap();
    let _ = run(agent(&dir)
        .arg("open")
        .arg("--ca-file")
        .arg(&file)
        .arg("https://localhost:1/"));
    let requests = dir.requests();
    let line = format!("arg anchors {}\n", surf_answer::quote(&pem));
    assert!(requests.contains(&line), "{requests}");
    assert!(
        requests.starts_with("surf-wire 1\ncommand open\n"),
        "{requests}"
    );
    assert!(
        !requests.contains(file.to_str().unwrap()),
        "the path travels: {requests}"
    );
}
