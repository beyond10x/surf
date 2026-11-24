//! Cases 1 to 4, 6 and 7 of `story:tls-session-anchors`: `surf-agent open --ca-file <root.pem>`
//! gives a session its extra trust anchors, and `surf-agent go https://localhost:<port>/` loads
//! through them. Each case runs the built `surf-agent`, `surfd` and `surf-session` (or those of
//! `SURF_BIN_DIR`) in a private `XDG_RUNTIME_DIR`, against `surf-tls-test-server` on 127.0.0.1
//! port 0 with the certificates of `crates/tls-test-certs/certs/`, on the system clock (the
//! certificates run to 2125, `expired` ended in 2001). Case 5, the files the client refuses, is
//! the module `ca_file` of the tests of `surf-agent`.

#![forbid(unsafe_code)]

use crate::common;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use common::{Opened, TestDir, agent, agent_call, cmdline, contains, opened, run, stderr, stdout};
use surf_tls_test_server::{Config, Mode, Response, TestServer, certs_dir};

/// The body every server of these cases answers `/` with: HTML without a title, so that the
/// `page` line ends at the address.
const BODY: &[u8] = b"<!doctype html><p>ok</p>";

/// The TLS test server of the certificate case `case` (each of the cases used here has a P-256
/// leaf, the scheme of [`Config::default`]), answering as `mode`, and `/` with 200 `text/html`.
fn server(case: &str, mode: Mode) -> TestServer {
    let mut config = Config {
        case: case.to_string(),
        mode,
        ..Config::default()
    };
    config.responses.insert(
        "/".to_string(),
        Response {
            status: 200,
            reason: "OK".to_string(),
            fields: vec![("Content-Type".to_string(), "text/html".to_string())],
            body: BODY.to_vec(),
        },
    );
    TestServer::start(config).expect("the TLS test server starts")
}

/// The root of the certificate case `case`, the one anchor its chain leads to.
fn root(case: &str) -> PathBuf {
    certs_dir().join(case).join("root.pem")
}

fn address(server: &TestServer) -> String {
    format!("https://localhost:{}/", server.port())
}

/// `surf-agent open`, with `--ca-file <file>` when one is given, which must exit 0.
fn open(dir: &TestDir, ca_file: Option<&PathBuf>) -> Opened {
    let mut command = agent(dir);
    command.arg("open");
    if let Some(file) = ca_file {
        command.arg("--ca-file").arg(file);
    }
    let output = run(&mut command);
    assert_eq!(output.status.code(), Some(0), "open: {}", stderr(&output));
    opened(&stdout(&output))
}

/// `surf-agent go <address>` as the holder of `session`, which must exit 0: its answer.
fn go(dir: &TestDir, session: &Opened, address: &str) -> String {
    let output = agent_call(dir, &["go", address], &session.id, &session.token);
    assert_eq!(output.status.code(), Some(0), "go: {}", stderr(&output));
    stdout(&output)
}

/// The `page` line of an answer, its line feed left out.
fn page_line(answer: &str) -> &str {
    answer
        .lines()
        .find(|line| line.starts_with("page "))
        .unwrap_or_else(|| panic!("no page line in:\n{answer}"))
}

fn loaded(address: &str) -> String {
    format!("page 200 \"text/html\" \"{address}\"")
}

fn failed(reason: &str, address: &str) -> String {
    format!("page failed {reason} \"{address}\"")
}

/// The process id of the session process of `id`, found by its identity in its arguments.
fn session_pid(dir: &TestDir, id: &str) -> u32 {
    dir.processes()
        .into_iter()
        .find(|&pid| {
            cmdline(pid)
                .iter()
                .any(|argument| contains(argument, id.as_bytes()))
        })
        .unwrap_or_else(|| panic!("no session process of {id} runs"))
}

#[test]
fn https_case_1_open_with_the_root_as_ca_file_loads_the_page() {
    let dir = TestDir::new();
    let server = server("ok-ecdsa-p256", Mode::Normal);
    let address = address(&server);
    let session = open(&dir, Some(&root("ok-ecdsa-p256")));
    let answer = go(&dir, &session, &address);
    assert_eq!(page_line(&answer), loaded(&address), "{answer}");
}

#[test]
fn https_case_2_a_session_opened_without_ca_file_does_not_trust_the_server() {
    let dir = TestDir::new();
    let server = server("ok-ecdsa-p256", Mode::Normal);
    let address = address(&server);
    let session = open(&dir, None);
    let answer = go(&dir, &session, &address);
    assert_eq!(
        page_line(&answer),
        failed("CertificateNotTrusted", &address),
        "{answer}"
    );
}

#[test]
fn https_case_3_an_expired_certificate_is_certificate_expired() {
    let dir = TestDir::new();
    let server = server("expired", Mode::Normal);
    let address = address(&server);
    let session = open(&dir, Some(&root("expired")));
    let answer = go(&dir, &session, &address);
    assert_eq!(
        page_line(&answer),
        failed("CertificateExpired", &address),
        "{answer}"
    );
}

#[test]
fn https_case_3_a_certificate_for_another_host_is_certificate_name_mismatch() {
    let dir = TestDir::new();
    let server = server("wrong-host", Mode::Normal);
    let address = address(&server);
    let session = open(&dir, Some(&root("wrong-host")));
    let answer = go(&dir, &session, &address);
    assert_eq!(
        page_line(&answer),
        failed("CertificateNameMismatch", &address),
        "{answer}"
    );
}

/// Kinds (a) and (c) of item 7 of `docs/design/tls.md`.
#[test]
fn https_case_3_a_server_limited_to_tls_1_2_is_tls_version_unsupported() {
    let dir = TestDir::new();
    let session = open(&dir, Some(&root("ok-ecdsa-p256")));
    for mode in [Mode::Tls12ReadsSupportedVersions, Mode::SelectsTls12] {
        let server = server("ok-ecdsa-p256", mode);
        let address = address(&server);
        let answer = go(&dir, &session, &address);
        assert_eq!(
            page_line(&answer),
            failed("TlsVersionUnsupported", &address),
            "{mode:?}: {answer}"
        );
    }
}

/// Kind (b) of item 7 of `docs/design/tls.md`, and a Finished that does not verify.
#[test]
fn https_case_3_a_failed_handshake_is_tls_handshake_failed() {
    let dir = TestDir::new();
    let session = open(&dir, Some(&root("ok-ecdsa-p256")));
    for mode in [Mode::Tls12IgnoresSupportedVersions, Mode::BadFinished] {
        let server = server("ok-ecdsa-p256", mode);
        let address = address(&server);
        let answer = go(&dir, &session, &address);
        assert_eq!(
            page_line(&answer),
            failed("TlsHandshakeFailed", &address),
            "{mode:?}: {answer}"
        );
    }
}

/// The anchors belong to the session, not to the daemon: A, opened with the file, and B, opened
/// after it without one, run under one `surfd`; A loads, B does not, and A still does after B.
#[test]
fn https_case_4_the_anchors_of_one_session_are_not_those_of_another() {
    let dir = TestDir::new();
    let server = server("ok-ecdsa-p256", Mode::Normal);
    let address = address(&server);
    let a = open(&dir, Some(&root("ok-ecdsa-p256")));
    let b = open(&dir, None);
    let surfds: Vec<u32> = dir
        .processes()
        .into_iter()
        .filter(|&pid| {
            cmdline(pid)
                .first()
                .is_some_and(|program| program.ends_with(b"/surfd"))
        })
        .collect();
    assert_eq!(
        surfds.len(),
        1,
        "one surfd serves both sessions: {surfds:?}"
    );
    let answer = go(&dir, &a, &address);
    assert_eq!(page_line(&answer), loaded(&address), "A: {answer}");
    let answer = go(&dir, &b, &address);
    assert_eq!(
        page_line(&answer),
        failed("CertificateNotTrusted", &address),
        "B: {answer}"
    );
    let answer = go(&dir, &a, &address);
    assert_eq!(page_line(&answer), loaded(&address), "A after B: {answer}");
}

/// The bytes of the file reach the session over its socket, not as an argument or in the
/// environment of its process: neither holds the first 64 base64 characters of the file.
#[test]
fn https_case_6_the_session_process_has_no_byte_of_the_file_in_its_arguments_or_environment() {
    let dir = TestDir::new();
    let server = server("ok-ecdsa-p256", Mode::Normal);
    let address = address(&server);
    let file = root("ok-ecdsa-p256");
    let pem = fs::read_to_string(&file).expect("the root of the case");
    let base64: String = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect::<String>()
        .chars()
        .take(64)
        .collect();
    assert_eq!(base64.len(), 64, "the file holds 64 base64 characters");
    let session = open(&dir, Some(&file));
    let answer = go(&dir, &session, &address);
    assert_eq!(page_line(&answer), loaded(&address), "{answer}");
    let pid = session_pid(&dir, &session.id);
    for part in ["cmdline", "environ"] {
        let bytes = fs::read(format!("/proc/{pid}/{part}"))
            .unwrap_or_else(|error| panic!("/proc/{pid}/{part}: {error}"));
        assert!(!bytes.is_empty(), "/proc/{pid}/{part} is empty");
        assert!(
            !contains(&bytes, base64.as_bytes()),
            "/proc/{pid}/{part} holds bytes of the file"
        );
    }
}

/// The session log of case 2, read from the live view of the built `surfd`: a
/// `surf.net.FetchFailed` entry of the address with the reason `CertificateNotTrusted`.
#[test]
fn https_case_7_the_log_of_case_2_holds_fetch_failed_certificate_not_trusted() {
    let dir = TestDir::new();
    let server = server("ok-ecdsa-p256", Mode::Normal);
    let address = address(&server);
    let session = open(&dir, None);
    let answer = go(&dir, &session, &address);
    assert_eq!(
        page_line(&answer),
        failed("CertificateNotTrusted", &address),
        "{answer}"
    );
    let detail = view_get(&dir, &format!("/api/sessions/{}", session.id));
    let name = "\"name\":\"surf.net.FetchFailed\"";
    let entries: Vec<&str> = detail
        .match_indices(name)
        .map(|(at, _)| {
            let rest = &detail[at..];
            // The entry ends where the next one starts.
            let end = rest[1..]
                .find("{\"number\":")
                .map_or(rest.len(), |end| end + 1);
            &rest[..end]
        })
        .collect();
    assert_eq!(entries.len(), 1, "one FetchFailed entry: {detail}");
    assert!(
        entries[0].contains("\"reason\":\"CertificateNotTrusted\""),
        "{}",
        entries[0]
    );
    assert!(
        entries[0].contains(&format!("\"url\":\"{address}\"")),
        "{}",
        entries[0]
    );
}

/// The body of `GET <target>` from the live view of the case's `surfd`, with the token of its
/// `view-url`; the answer must be 200.
fn view_get(dir: &TestDir, target: &str) -> String {
    let line = fs::read_to_string(dir.surf().join("view-url")).expect("view-url");
    let rest = line
        .strip_prefix("http://127.0.0.1:")
        .expect("http://127.0.0.1:");
    let (port, token) = rest.split_once("/#").expect("/#");
    let token = token.strip_suffix('\n').expect("a line feed");
    let mut stream = TcpStream::connect(("127.0.0.1", port.parse::<u16>().expect("a port")))
        .expect("the view listens");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let request = format!(
        "GET {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).unwrap();
    let answer = String::from_utf8(answer).expect("UTF-8");
    assert!(
        answer.starts_with("HTTP/1.1 200 OK\r\n"),
        "{target}: {answer}"
    );
    let at = answer.find("\r\n\r\n").expect("a head") + 4;
    answer[at..].to_string()
}
