//! Adversary cases of `story:tls-session-anchors`, the client's half: the empty path of
//! `--ca-file`, a path that is not a regular file (a FIFO among them), and the size bound of the
//! file at 32 KiB and at one byte more, with a file that is valid PEM on both sides of the bound.
//! With a copy of the built `surf-agent` and the stand-in beside it as `surfd`.

use crate::common;

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use common::*;

/// The most bytes a file of extra anchors may hold (`docs/design/tls.md` item 8).
const FILE_MAX: usize = 32 * 1024;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tls-test-certs/certs/ok-ecdsa-p256/root.pem")
}

/// `surf-agent <words>`: exit 2, nothing on standard output, `names` and the usage on standard
/// error, nothing started and nothing sent to the stand-in.
fn refused(dir: &TestDir, words: &[&str], names: &str) {
    let output = run(agent(dir).args(words));
    let err = stderr(&output);
    assert_eq!(output.status.code(), Some(2), "{words:?}: {err}");
    assert_eq!(stdout(&output), "", "{words:?}");
    assert!(err.contains(names), "{words:?}: {err}");
    assert!(err.contains("usage: surf-agent open"), "{words:?}: {err}");
    assert!(dir.starts().is_empty(), "{words:?}: surfd was started");
    assert_eq!(dir.requests(), "", "{words:?}: a request was sent");
}

#[test]
fn adversary_ca_file_an_empty_path_is_refused_by_name() {
    let dir = TestDir::new();
    refused(&dir, &["open", "--ca-file", ""], "--ca-file needs a path");
    refused(&dir, &["open", "--ca-file="], "--ca-file needs a path");
}

#[test]
fn adversary_ca_file_a_path_that_is_no_regular_file_is_refused_without_reading_it() {
    let dir = TestDir::new();
    let directory = dir.path.join("a-directory");
    fs::create_dir(&directory).unwrap();
    for path in [Path::new("/dev/zero"), directory.as_path()] {
        refused(
            &dir,
            &["open", "--ca-file", path.to_str().unwrap()],
            "the --ca-file file is not a regular file",
        );
    }
}

/// A FIFO that nobody writes to: opening it for reading would wait for a writer forever, so the
/// client looks at the path before it opens it. It must end within 5 s with exit 2, the line of a
/// path that is not a regular file, nothing started and nothing sent.
#[test]
fn adversary_ca_file_a_fifo_is_refused_and_does_not_hang() {
    let dir = TestDir::new();
    let fifo = dir.path.join("anchors.fifo");
    let made = Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo runs");
    assert!(made.success());
    let mut child = agent(&dir)
        .arg("open")
        .arg("--ca-file")
        .arg(&fifo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("surf-agent starts");
    let deadline = Instant::now() + Duration::from_secs(5);
    let ended = loop {
        if child.try_wait().unwrap().is_some() {
            break true;
        }
        if Instant::now() >= deadline {
            break false;
        }
        thread::sleep(Duration::from_millis(10));
    };
    if !ended {
        let _ = child.kill();
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        ended,
        "surf-agent open --ca-file <fifo> did not end within 5 s"
    );
    let err = stderr(&output);
    assert_eq!(output.status.code(), Some(2), "{err}");
    assert!(
        err.contains("the --ca-file file is not a regular file"),
        "{err}"
    );
    assert!(dir.starts().is_empty(), "surfd was started");
    assert_eq!(dir.requests(), "", "a request was sent");
}

/// The root followed by `#` outside any block, `size` bytes in all: valid PEM of one CA.
fn padded_root(dir: &TestDir, name: &str, size: usize) -> PathBuf {
    let mut text = fs::read_to_string(root()).unwrap();
    text.push_str(&"#".repeat(size - text.len()));
    let path = dir.path.join(name);
    fs::write(&path, &text).unwrap();
    path
}

/// A file of exactly 32 KiB is taken and sent whole, also through a symbolic link; a file of
/// one byte more is refused `TooLarge` although its first 32 KiB are a file that is taken.
#[test]
fn adversary_ca_file_the_bound_is_32_kib_exactly() {
    let dir = TestDir::new();
    let at = padded_root(&dir, "at.pem", FILE_MAX);
    let link = dir.path.join("link.pem");
    symlink(&at, &link).unwrap();
    let over = padded_root(&dir, "over.pem", FILE_MAX + 1);
    refused(
        &dir,
        &["open", "--ca-file", over.to_str().unwrap()],
        "TooLarge",
    );
    let _surfd = StandIn::start(&dir);
    let text = fs::read_to_string(&at).unwrap();
    let line = format!("arg anchors {}\n", surf_answer::quote(&text));
    for file in [&at, &link] {
        dir.clear_requests();
        let _ = run(agent(&dir).arg("open").arg("--ca-file").arg(file));
        let requests = dir.requests();
        assert!(requests.contains(&line), "{}: {requests}", file.display());
    }
}
