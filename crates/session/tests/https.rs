//! Cases 1 to 4, 6 and 7 of `story:tls-session-anchors`: `surf-agent open --ca-file <root.pem>`
//! gives a session its extra trust anchors, and `surf-agent go https://localhost:<port>/` loads
//! through them. Each case runs the built `surf-agent`, `surfd` and `surf-session` (or those of
//! `SURF_BIN_DIR`) in a private `XDG_RUNTIME_DIR`, against `surf-tls-test-server` on 127.0.0.1
//! port 0 with the certificates of `crates/tls-test-certs/certs/`, on the system clock (the
//! certificates run to 2125, `expired` ended in 2001). Case 5, the files the client refuses, is
//! the module `ca_file` of the tests of `surf-agent`.
//!
//! Statements 1 to 6 of `story:session-follow-press-https` (the tests `follow_press_case_<n>_*`):
//! inside such a session `follow` and `press` load `https:` targets through the session's
//! anchors, and every scheme but `http:` and `https:` stays `unsupported-scheme`. The pages of
//! a TLS server are fixed before it binds its port, so a link or an action to the same server
//! is relative.

#![forbid(unsafe_code)]

use crate::common;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use common::{
    Opened, PageServer, TestDir, agent, agent_call, cmdline, contains, opened, run, stderr, stdout,
};
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

/// A file of the root and another root, with lines before, between and after them: the client
/// sends only the blocks, and the session loads the page through them.
#[test]
fn https_case_1_a_file_with_lines_around_its_blocks_loads_the_page() {
    let dir = TestDir::new();
    let server = server("ok-ecdsa-p256", Mode::Normal);
    let address = address(&server);
    let text = [
        "a line before the blocks\n".to_string(),
        fs::read_to_string(root("ok-ed25519")).unwrap(),
        "a line between the blocks\n".to_string(),
        fs::read_to_string(root("ok-ecdsa-p256")).unwrap(),
        "a line after the blocks\n".to_string(),
    ]
    .concat();
    let file = dir.path.join("roots.pem");
    fs::write(&file, text).unwrap();
    let session = open(&dir, Some(&file));
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
    let cmdline = fs::read(format!("/proc/{pid}/cmdline"))
        .unwrap_or_else(|error| panic!("/proc/{pid}/cmdline: {error}"));
    assert!(!cmdline.is_empty(), "/proc/{pid}/cmdline is empty");
    assert!(
        !contains(&cmdline, base64.as_bytes()),
        "/proc/{pid}/cmdline holds bytes of the file"
    );
    // `surfd` starts a session with an empty environment.
    let environ = fs::read(format!("/proc/{pid}/environ"))
        .unwrap_or_else(|error| panic!("/proc/{pid}/environ: {error}"));
    assert_eq!(environ, b"", "/proc/{pid}/environ");
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

// --- story:session-follow-press-https ---------------------------------------------------------

/// How long a case waits for a TLS server to end the connections it expects.
const SEEN_WITHIN: Duration = Duration::from_secs(10);

/// The page a link or a form of these cases leads to: HTML without a title, like [`BODY`].
const NEXT: &str = "<!doctype html><p>next</p>";

/// The TLS test server of the certificate case `case` in [`Mode::Normal`], answering each path
/// of `pages` with 200 `text/html` and its body, any other path 404.
fn tls_pages(case: &str, pages: &[(&str, &str)]) -> TestServer {
    let mut config = Config {
        case: case.to_string(),
        ..Config::default()
    };
    config.responses.clear();
    for (path, body) in pages {
        config.responses.insert(
            (*path).to_string(),
            Response {
                status: 200,
                reason: "OK".to_string(),
                fields: vec![("Content-Type".to_string(), "text/html".to_string())],
                body: body.as_bytes().to_vec(),
            },
        );
    }
    TestServer::start(config).expect("the TLS test server starts")
}

/// `surf-agent <words>` as the holder of `session`: its exit status and its answer.
fn call(dir: &TestDir, session: &Opened, words: &[&str]) -> (Option<i32>, String) {
    let output = agent_call(dir, words, &session.id, &session.token);
    (output.status.code(), stdout(&output))
}

/// A session opened with the root of `ok-ecdsa-p256` as its `--ca-file`, on `start`.
fn trusting_at(dir: &TestDir, start: &str) -> Opened {
    let session = open(dir, Some(&root("ok-ecdsa-p256")));
    let answer = go(dir, &session, start);
    assert_eq!(page_line(&answer), loaded(start), "{answer}");
    session
}

/// The `now` line of `handle` in an answer.
fn now_line<'a>(answer: &'a str, handle: &str) -> &'a str {
    let start = format!("  {handle} ");
    answer
        .lines()
        .find(|line| line.starts_with(&start))
        .unwrap_or_else(|| panic!("no now line of {handle} in:\n{answer}"))
}

/// The request lines of the connections a server ended, waiting for `count` of them.
fn request_lines(server: &TestServer, count: usize) -> Vec<String> {
    server
        .wait_seen(count, SEEN_WITHIN)
        .into_iter()
        .filter_map(|seen| seen.request)
        .collect()
}

/// Statement 1: a relative link on an `https:` page is followed through the session's anchors.
#[test]
fn follow_press_case_1_follow_loads_a_relative_https_link() {
    let dir = TestDir::new();
    let server = tls_pages(
        "ok-ecdsa-p256",
        &[
            ("/", "<!doctype html><a href=\"/next\">Next</a>"),
            ("/next", NEXT),
        ],
    );
    let start = address(&server);
    let session = trusting_at(&dir, &start);
    let (status, answer) = call(&dir, &session, &["follow", "l1"]);
    assert_eq!(status, Some(0), "{answer}");
    assert!(answer.contains("\ndid follow \"l1\"\n"), "{answer}");
    assert_eq!(
        page_line(&answer),
        loaded(&format!("{start}next")),
        "{answer}"
    );
    assert_eq!(
        request_lines(&server, 2),
        vec![
            "GET / HTTP/1.1".to_string(),
            "GET /next HTTP/1.1".to_string()
        ]
    );
}

/// Statement 2: a form of `method=post` with the relative action `/send` is sent with the value
/// filled; the server reads one `POST /send` with the body `q=blue`.
#[test]
fn follow_press_case_2_press_posts_a_form_to_a_relative_https_action() {
    let dir = TestDir::new();
    let server = tls_pages(
        "ok-ecdsa-p256",
        &[
            (
                "/",
                "<!doctype html><form method=post action=/send><input name=q>\
                 <button>Send</button></form>",
            ),
            ("/send", NEXT),
        ],
    );
    let start = address(&server);
    let session = trusting_at(&dir, &start);
    let (status, answer) = call(&dir, &session, &["fill", "f1", "blue"]);
    assert_eq!(status, Some(0), "{answer}");
    let (status, answer) = call(&dir, &session, &["press", "b1"]);
    assert_eq!(status, Some(0), "{answer}");
    assert!(answer.contains("\ndid press \"b1\"\n"), "{answer}");
    assert_eq!(
        page_line(&answer),
        loaded(&format!("{start}send")),
        "{answer}"
    );
    let seen = server.wait_seen(2, SEEN_WITHIN);
    let posts: Vec<_> = seen
        .iter()
        .filter(|seen| seen.request.as_deref() == Some("POST /send HTTP/1.1"))
        .collect();
    assert_eq!(posts.len(), 1, "one POST /send: {seen:?}");
    assert_eq!(
        String::from_utf8_lossy(&posts[0].body),
        "q=blue",
        "{seen:?}"
    );
}

/// Statement 3: a link to a server whose chain leads to no anchor of the session fails
/// `CertificateNotTrusted`, and `look` shows the page line of that last navigation.
#[test]
fn follow_press_case_3_a_link_to_an_untrusted_server_fails_certificate_not_trusted() {
    let dir = TestDir::new();
    let untrusted = server("untrusted-root", Mode::Normal);
    let other = address(&untrusted);
    let server = tls_pages(
        "ok-ecdsa-p256",
        &[(
            "/",
            &format!("<!doctype html><a href=\"{other}\">Other</a>"),
        )],
    );
    let session = trusting_at(&dir, &address(&server));
    let (status, answer) = call(&dir, &session, &["follow", "l1"]);
    assert_eq!(status, Some(0), "{answer}");
    let failed_line = failed("CertificateNotTrusted", &other);
    assert_eq!(page_line(&answer), failed_line, "{answer}");
    let (status, look) = call(&dir, &session, &["look"]);
    assert_eq!(status, Some(0), "{look}");
    assert_eq!(page_line(&look), failed_line, "{look}");
    assert_eq!(
        untrusted.accepted(),
        1,
        "the link reached the second server"
    );
}

/// Statement 4: from an `http:` page, in a session opened without `--ca-file`, a link to the
/// TLS server is followed and fails `CertificateNotTrusted` (the bundled anchors alone).
#[test]
fn follow_press_case_4_without_ca_file_an_https_link_fails_certificate_not_trusted() {
    let dir = TestDir::new();
    let tls = server("ok-ecdsa-p256", Mode::Normal);
    let target = address(&tls);
    let plain = PageServer::with_body(
        format!("<!doctype html><a href=\"{target}\">Secure</a>").into_bytes(),
    );
    let session = open(&dir, None);
    let start = plain.url("/");
    let answer = go(&dir, &session, &start);
    assert_eq!(page_line(&answer), loaded(&start), "{answer}");
    let (status, answer) = call(&dir, &session, &["follow", "l1"]);
    assert_eq!(status, Some(0), "{answer}");
    assert_eq!(
        page_line(&answer),
        failed("CertificateNotTrusted", &target),
        "{answer}"
    );
    assert_eq!(tls.accepted(), 1, "the link reached the TLS server");
}

/// Statement 5: from an `https:` page, a link to an `http:` page is followed.
#[test]
fn follow_press_case_5_from_an_https_page_an_http_link_is_followed() {
    let dir = TestDir::new();
    let plain = PageServer::start();
    let target = plain.url("/plain");
    let server = tls_pages(
        "ok-ecdsa-p256",
        &[(
            "/",
            &format!("<!doctype html><a href=\"{target}\">Plain</a>"),
        )],
    );
    let session = trusting_at(&dir, &address(&server));
    let (status, answer) = call(&dir, &session, &["follow", "l1"]);
    assert_eq!(status, Some(0), "{answer}");
    assert_eq!(page_line(&answer), loaded(&target), "{answer}");
}

/// Statement 6: on an `https:` page every other scheme stays `unsupported-scheme`, for a link
/// and for a form's action, with nothing requested.
#[test]
fn follow_press_case_6_every_scheme_but_http_and_https_is_refused_unsupported_scheme() {
    let dir = TestDir::new();
    let hrefs = [
        "javascript:void(0)",
        "mailto:a@example.test",
        "data:text/plain,x",
        "file:///etc/hostname",
        "ftp://127.0.0.1/x",
    ];
    let mut page = String::from("<!doctype html>");
    for href in hrefs {
        page.push_str(&format!("<a href=\"{href}\">x</a>"));
    }
    page.push_str(
        "<form method=post action=\"ftp://127.0.0.1/x\"><input name=q value=1>\
         <button>Send</button></form>",
    );
    let server = tls_pages("ok-ecdsa-p256", &[("/", &page)]);
    let start = address(&server);
    let session = trusting_at(&dir, &start);
    let (_, look) = call(&dir, &session, &["look"]);
    for (at, href) in hrefs.iter().enumerate() {
        let handle = format!("l{}", at + 1);
        let scheme = &href[..=href.find(':').expect("a scheme")];
        assert!(
            now_line(&look, &handle).contains(&format!("\"{scheme}")),
            "{handle} is {href}: {look}"
        );
        let (status, answer) = call(&dir, &session, &["follow", &handle]);
        assert_eq!(status, Some(1), "{href}: {answer}");
        assert!(
            answer.contains("\ndid refused unsupported-scheme\n"),
            "{href}: {answer}"
        );
        assert_eq!(page_line(&answer), loaded(&start), "{href}: {answer}");
    }
    assert!(
        now_line(&look, "b1").contains("\"ftp://127.0.0.1/x\""),
        "{look}"
    );
    let (status, answer) = call(&dir, &session, &["press", "b1"]);
    assert_eq!(status, Some(1), "{answer}");
    assert!(
        answer.contains("\ndid refused unsupported-scheme\n"),
        "{answer}"
    );
    assert_eq!(page_line(&answer), loaded(&start), "{answer}");
    assert_eq!(server.accepted(), 1, "only the page itself was requested");
}
