//! Conformance checks of `surf --ca-file` against `docs/surf-command.md` ("`https:` and the
//! trust anchors"): a file the reader refuses is refused whole, with exit 2 and no connection,
//! never a fall back to the bundled list alone; a refused load prints nothing of the file.
//! Against `surf-tls-test-server` on 127.0.0.1 port 0; files lie under `CARGO_TARGET_TMPDIR`.

use std::fs;
use std::path::PathBuf;
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
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("surf-cli-secreview-tls-command")
        .join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the scratch directory");
    dir
}

fn text(path: &std::path::Path) -> String {
    path.to_str().expect("a UTF-8 path").to_string()
}

/// Invariant: a `--ca-file` the reader refuses ends the call with exit 2 and no connection,
/// for each way the document names (no PEM block, a certificate that is no CA, a file over
/// 32,768 bytes, a file that is a directory) and for a file whose good CA root is followed by
/// a refused block: the good root is not taken alone, and the bundled list is not used alone.
#[test]
fn secreview_tls_every_refused_ca_file_exits_2_and_connects_nowhere() {
    let server = TestServer::start(Config::default()).expect("the TLS test server starts");
    let address = format!("https://localhost:{}/", server.port());
    let dir = scratch("refused");
    let case = certs_dir().join("ok-ecdsa-p256");
    let root = fs::read_to_string(case.join("root.pem")).expect("root.pem");
    let chain = fs::read_to_string(case.join("chain.pem")).expect("chain.pem");

    let empty = dir.join("empty.pem");
    fs::write(&empty, "").expect("write");
    let leaf_first = dir.join("leaf.pem");
    fs::write(&leaf_first, &chain).expect("write");
    let too_large = dir.join("large.pem");
    let mut large = root.clone();
    while large.len() <= 32 * 1024 {
        large.push_str(&root);
    }
    fs::write(&too_large, &large).expect("write");
    let root_then_bad = dir.join("root-then-bad.pem");
    fs::write(
        &root_then_bad,
        format!("{root}-----BEGIN CERTIFICATE-----\n!!!!\n-----END CERTIFICATE-----\n"),
    )
    .expect("write");
    let directory = dir.join("a-directory");
    fs::create_dir_all(&directory).expect("mkdir");

    for (path, why) in [
        (&empty, "NoCertificate"),
        (&leaf_first, "NotACa"),
        (&too_large, "TooLarge"),
        (&root_then_bad, "BadPem: block 1"),
        (&directory, ""),
    ] {
        let path = text(path);
        let run = surf(&["--ca-file", &path, &address]);
        assert_eq!(run.status, 2, "{path}: {}{}", run.stdout, run.stderr);
        assert_eq!(run.stdout, "", "{path}");
        assert!(run.stderr.contains(&path), "{path}: {}", run.stderr);
        assert!(run.stderr.contains(why), "{path}: {}", run.stderr);
    }
    assert_eq!(server.accepted(), 0, "a refused --ca-file connected");
    let _ = fs::remove_dir_all(&dir);
}

/// Invariant: what `surf` prints of a failed `https:` load holds no byte of the anchors file
/// beyond its path: a `--ca-file` that is a good CA of another case, against a server it does
/// not anchor, ends `CertificateNotTrusted` and standard error and output hold no PEM text.
#[test]
fn secreview_tls_a_refused_load_prints_nothing_of_the_anchors_file() {
    let server = TestServer::start(Config::default()).expect("the TLS test server starts");
    let address = format!("https://localhost:{}/", server.port());
    let other = certs_dir().join("ok-rsa2048").join("root.pem");
    let pem = fs::read_to_string(&other).expect("root.pem");
    let run = surf(&["--ca-file", &text(&other), &address]);
    assert_eq!(run.status, 1, "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.contains("CertificateNotTrusted"), "{}", run.stdout);
    for line in pem.lines().filter(|l| l.len() >= 16) {
        assert!(!run.stderr.contains(line), "standard error holds {line:?}");
        assert!(!run.stdout.contains(line), "standard output holds {line:?}");
    }
}
