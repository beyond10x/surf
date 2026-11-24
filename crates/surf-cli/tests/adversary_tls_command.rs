//! The adversary's cases of `story:tls-fetch-https` against the `surf` binary, held to
//! `docs/surf-command.md` ("`https:` and the trust anchors", "Output", "Exit statuses"): each
//! TLS failure reason on the `fetch-failed` line with exit 1, a redirect from `http:` to
//! `https:` with `--ca-file`, and the files `--ca-file` refuses or takes at the size bound. Each
//! server runs on 127.0.0.1 port 0; the binary is killed after `RUN_BOUND`. A file a case
//! writes lies under `CARGO_TARGET_TMPDIR`.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use surf_tls_test_server::{Config, Mode, TestServer, certs_dir};

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

fn path_text(path: &Path) -> String {
    path.to_str().expect("a UTF-8 path").to_string()
}

/// The `root.pem` of a case of `crates/tls-test-certs/certs/`.
fn root(case: &str) -> String {
    path_text(&certs_dir().join(case).join("root.pem"))
}

/// The TLS test server of the P-256 case `case`, answering as `mode`.
fn server(case: &str, mode: Mode) -> TestServer {
    let mut config = Config::default();
    config.case = case.to_string();
    config.mode = mode;
    TestServer::start(config).expect("the TLS test server starts")
}

/// A directory of this test process under `CARGO_TARGET_TMPDIR`, made empty.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("surf-cli-adversary-tls-command")
        .join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the scratch directory");
    dir
}

/// `docs/surf-command.md`, "Output": `fetch-failed <request-id> <reason> <address>`, `<reason>`
/// one of the five of TLS; "Exit statuses": 1 when the navigation is `Failed`. Each failure of
/// a server that `--ca-file` names the root of: the binary prints its own reason, and the
/// `surf:` line on standard error the reason's words.
#[test]
fn adversary_tls_each_tls_failure_is_its_reason_on_the_fetch_failed_line_with_exit_1() {
    for (case, mode, reason, words) in [
        (
            "expired",
            Mode::Normal,
            "CertificateExpired",
            "certificate expired",
        ),
        (
            "not-yet-valid",
            Mode::Normal,
            "CertificateExpired",
            "certificate expired",
        ),
        (
            "wrong-host",
            Mode::Normal,
            "CertificateNameMismatch",
            "certificate name mismatch",
        ),
        (
            "untrusted-root",
            Mode::Normal,
            "CertificateNotTrusted",
            "certificate not trusted",
        ),
        (
            "ok-ecdsa-p256",
            Mode::Tls12ReadsSupportedVersions,
            "TlsVersionUnsupported",
            "TLS version unsupported",
        ),
        (
            "ok-ecdsa-p256",
            Mode::BadFinished,
            "TlsHandshakeFailed",
            "TLS handshake failed",
        ),
    ] {
        let server = server(case, mode);
        let address = format!("https://localhost:{}/", server.port());
        let run = surf(&["--ca-file", &root(case), &address]);
        let label = format!("{case} {mode:?}");
        assert_eq!(run.status, 1, "{label}: {}{}", run.stdout, run.stderr);
        assert!(
            run.stdout
                .lines()
                .any(|line| line.starts_with("fetch-failed ")
                    && line.ends_with(&format!(" {reason} {address}"))),
            "{label}: {}",
            run.stdout
        );
        assert!(
            run.stdout
                .lines()
                .any(|line| line.starts_with("navigation ")
                    && line.ends_with(&format!(" Failed {address}"))),
            "{label}: {}",
            run.stdout
        );
        assert!(
            run.stderr.contains(&format!("{address}: {words}: ")),
            "{label}: {}",
            run.stderr
        );
    }
}

/// An `http:` server whose every answer is a 302 to `location`.
fn redirecting(location: String) -> (u16, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local address").port();
    let handle = thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut head = Vec::new();
        let mut buf = [0u8; 1024];
        while !head.windows(4).any(|w| w == b"\r\n\r\n") {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(n) => head.extend_from_slice(&buf[..n]),
            }
        }
        let reply = format!(
            "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let _ = stream.write_all(reply.as_bytes());
    });
    (port, handle)
}

/// `docs/surf-command.md`: "Redirects between `http:` and `https:` are followed both ways", the
/// server "authenticated against the bundled trust anchors and the anchors of `--ca-file`";
/// "After redirects, the `navigation` and `document` lines hold the address of the last
/// response, and the `request` and `fetch-failed` lines the address asked for".
#[test]
fn adversary_tls_an_http_redirect_to_https_commits_with_the_anchors_of_ca_file() {
    let server = server("ok-ecdsa-p256", Mode::Normal);
    let to = format!("https://localhost:{}/", server.port());
    let (port, handle) = redirecting(to.clone());
    let from = format!("http://127.0.0.1:{port}/a");
    let run = surf(&["--ca-file", &root("ok-ecdsa-p256"), &from]);
    let _ = handle.join();
    assert_eq!(run.status, 0, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout
            .lines()
            .any(|line| line.starts_with("navigation ")
                && line.ends_with(&format!(" Committed {to}"))),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout
            .lines()
            .any(|line| line.starts_with("request ")
                && line.ends_with(&format!(" Responded {from}"))),
        "{}",
        run.stdout
    );
}

/// `docs/surf-command.md`: "The file holds at most 32 KiB (32,768 bytes)"; a file is refused
/// "when it is over 32,768 bytes (before any block is read)". A root padded with line feeds to
/// exactly 32,768 bytes is taken; one byte more is refused as too large, exit 2, naming the
/// path, also when its first 32,768 bytes alone would be taken.
#[test]
fn adversary_tls_ca_file_takes_32768_bytes_and_refuses_one_more() {
    let dir = scratch("size");
    let pem = fs::read(certs_dir().join("ok-ecdsa-p256").join("root.pem")).expect("root.pem");
    let at = dir.join("at-bound.pem");
    let over = dir.join("over-bound.pem");
    let mut bytes = pem.clone();
    bytes.resize(32_768, b'\n');
    fs::write(&at, &bytes).expect("write");
    bytes.push(b'\n');
    fs::write(&over, &bytes).expect("write");
    let (at, over) = (path_text(&at), path_text(&over));

    let run = surf(&["--anchors", "--ca-file", &at]);
    assert_eq!(run.status, 0, "{}", run.stderr);
    assert!(
        run.stdout.ends_with(&format!("anchors 1 file {at}\n")),
        "{}",
        run.stdout
    );

    let run = surf(&["--anchors", "--ca-file", &over]);
    assert_eq!(run.status, 2, "{}{}", run.stdout, run.stderr);
    assert_eq!(run.stdout, "");
    assert!(run.stderr.contains(&over), "{}", run.stderr);
    assert!(run.stderr.contains("TooLarge"), "{}", run.stderr);
    let _ = fs::remove_dir_all(&dir);
}

/// `docs/surf-command.md`: a file is refused whole "when it holds no PEM block", or holds "one
/// [certificate that] is no CA"; a path that is a directory cannot be read. Each: exit 2, the
/// path and the error on standard error, nothing on standard output.
#[test]
fn adversary_tls_ca_file_empty_a_directory_or_a_leaf_exits_2() {
    let dir = scratch("refused");
    let empty = dir.join("empty.pem");
    fs::write(&empty, b"").expect("write");
    let leaf = certs_dir().join("ok-ecdsa-p256").join("chain.pem");
    for (path, error) in [
        (path_text(&empty), "NoCertificate"),
        (path_text(&dir), "Is a directory"),
        (path_text(&leaf), "NotACa"),
    ] {
        let run = surf(&["--anchors", "--ca-file", &path]);
        assert_eq!(run.status, 2, "{path}: {}{}", run.stdout, run.stderr);
        assert_eq!(run.stdout, "", "{path}");
        assert!(run.stderr.contains(&path), "{path}: {}", run.stderr);
        assert!(run.stderr.contains(error), "{path}: {}", run.stderr);
    }
    let _ = fs::remove_dir_all(&dir);
}
