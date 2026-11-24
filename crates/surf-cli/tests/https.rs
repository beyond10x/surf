//! Cases 18 to 21 of `story:tls-fetch-https`: the `surf` binary loads an `https:` address from
//! `surf-tls-test-server` on 127.0.0.1 port 0, with the root of the case given by `--ca-file`;
//! `--anchors` prints the trust anchors; a refused `--ca-file` makes no connection. The binary
//! is killed if it runs longer than `RUN_BOUND`. A file a case writes lies under
//! `CARGO_TARGET_TMPDIR`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use surf_tls_test_server::{Config, TestServer, certs_dir};

/// The longest the binary may run before the test kills it and fails.
const RUN_BOUND: Duration = Duration::from_secs(20);

struct Run {
    status: i32,
    stdout: String,
    stderr: String,
}

/// Run the binary with `args`, killing it after `RUN_BOUND`.
fn surf(args: &[&str]) -> Run {
    let child = Command::new(env!("CARGO_BIN_EXE_surf"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn surf");
    let pid = child.id();
    let (sender, receiver) = mpsc::channel();
    let waiter = thread::spawn(move || {
        let _ = sender.send(child.wait_with_output());
    });
    let output = match receiver.recv_timeout(RUN_BOUND) {
        Ok(output) => output.expect("surf output"),
        Err(_) => {
            let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
            let _ = waiter.join();
            panic!("surf ran longer than {RUN_BOUND:?}");
        }
    };
    waiter.join().expect("the waiting thread");
    Run {
        status: output.status.code().expect("surf exited, not signalled"),
        stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
    }
}

/// The TLS test server of the case `ok-ecdsa-p256` (its [`Config::default`]), answering `/`
/// with `ok`.
fn server() -> TestServer {
    TestServer::start(Config::default()).expect("the TLS test server starts")
}

/// The root of the case `ok-ecdsa-p256`, the one anchor its chain leads to.
fn root() -> String {
    path_text(&certs_dir().join("ok-ecdsa-p256").join("root.pem"))
}

fn path_text(path: &Path) -> String {
    path.to_str().expect("a UTF-8 path").to_string()
}

/// A directory of this test process under `CARGO_TARGET_TMPDIR`, made empty.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("surf-cli-https")
        .join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the scratch directory");
    dir
}

/// The line of `stdout` that starts with `kind` and ends with ` <rest>`, split into its identity.
fn line<'a>(stdout: &'a str, kind: &str, rest: &str) -> Option<&'a str> {
    stdout.lines().find_map(|line| {
        let after = line.strip_prefix(kind)?.strip_prefix(' ')?;
        let (id, tail) = after.split_once(' ')?;
        (tail == rest).then_some(id)
    })
}

#[test]
fn case_18_an_https_address_with_its_root_as_ca_file_commits() {
    let server = server();
    let address = format!("https://localhost:{}/", server.port());
    let run = surf(&["--ca-file", &root(), &address]);
    assert_eq!(run.status, 0, "{}{}", run.stdout, run.stderr);
    let navigation = line(&run.stdout, "navigation", &format!("Committed {address}"));
    let request = line(&run.stdout, "request", &format!("Responded {address}"));
    assert!(navigation.is_some(), "{}", run.stdout);
    assert_eq!(navigation, request, "{}", run.stdout);
}

#[test]
fn case_19_without_ca_file_the_certificate_is_not_trusted() {
    let server = server();
    let address = format!("https://localhost:{}/", server.port());
    let run = surf(&[&address]);
    assert_eq!(run.status, 1, "{}{}", run.stdout, run.stderr);
    assert!(
        line(
            &run.stdout,
            "fetch-failed",
            &format!("CertificateNotTrusted {address}")
        )
        .is_some(),
        "{}",
        run.stdout
    );
    assert!(run.stderr.contains("NoPathToAnchor"), "{}", run.stderr);
}

/// The value of the line `<key>: <value>` of the bundled list's `SOURCE`.
fn source(key: &str) -> String {
    let text = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tls/src/anchors/SOURCE"),
    )
    .expect("SOURCE of the bundled anchors");
    text.lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix(": "))
        .unwrap_or_else(|| panic!("SOURCE has no {key}"))
        .to_string()
}

#[test]
fn case_20_anchors_prints_the_bundled_list_and_the_file() {
    let date = source("date");
    let sha256 = source("sha256");
    let certificates: usize = source("count").parse().expect("a count");
    let bundled = surf_fetch::Anchors::bundled().count();
    assert!(
        bundled > 0 && bundled <= certificates,
        "{bundled} of {certificates}"
    );
    let expected = format!("anchors {bundled} bundled {date} sha256:{sha256}\n");

    let run = surf(&["--anchors"]);
    assert_eq!(run.status, 0, "{}", run.stderr);
    assert_eq!(run.stdout, expected);
    assert_eq!(run.stderr, "");

    let root = root();
    let run = surf(&["--anchors", "--ca-file", &root]);
    assert_eq!(run.status, 0, "{}", run.stderr);
    assert_eq!(run.stdout, format!("{expected}anchors 1 file {root}\n"));
}

#[test]
fn case_21_a_refused_ca_file_exits_2_naming_path_and_error_and_connects_nowhere() {
    let server = server();
    let address = format!("https://localhost:{}/", server.port());
    let dir = scratch("case-21");
    let missing = path_text(&dir.join("missing.pem"));
    let key = dir.join("key.pem");
    // A block of that label is refused before its content is decoded, so the content is a
    // placeholder, as in the PEM tests of surf-tls.
    fs::write(
        &key,
        "-----BEGIN PRIVATE KEY-----\nZm9v\n-----END PRIVATE KEY-----\n",
    )
    .expect("write the key file");
    let key = path_text(&key);
    for (path, error) in [
        (missing.as_str(), "No such file or directory"),
        (
            key.as_str(),
            "NotACertificate: block 0 is labelled PRIVATE KEY",
        ),
    ] {
        for args in [
            vec!["--ca-file", path, address.as_str()],
            vec!["--anchors", "--ca-file", path],
        ] {
            let run = surf(&args);
            assert_eq!(run.status, 2, "{args:?}: {}{}", run.stdout, run.stderr);
            assert_eq!(run.stdout, "", "{args:?}");
            assert!(run.stderr.contains(path), "{args:?}: {}", run.stderr);
            assert!(run.stderr.contains(error), "{args:?}: {}", run.stderr);
        }
    }
    assert_eq!(server.accepted(), 0, "a refused --ca-file connected");
    let _ = fs::remove_dir_all(&dir);
}
