//! Adversary cases of `story:tls-session-anchors`, the session's half: the anchors of an `open`
//! that also names an `https:` address load that address, the session's own refusal of `arg
//! anchors` text it does not take (a client other than `surf-agent` can send any), the wire at
//! and one byte over `REQUEST_MAX` with anchors, and the anchors in use on the thread after an
//! answer. Against `surf-tls-test-server` on 127.0.0.1 port 0 with the certificates of
//! `crates/tls-test-certs/certs/`.

#![forbid(unsafe_code)]

use crate::common;

use std::fs;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use common::{
    TestDir, agent, entries_under, opened, request, run, stderr, stdout, surfd_in_process,
    wire_call,
};
use surf_agent::wire;
use surf_session::Session;
use surf_tls_test_server::{Config, Response, TestServer, certs_dir};

const ID: &str = "7a1c3e5f-2b4d-4f6a-8c0e-1d3f5a7b9c2e";

/// The most bytes of the text of `arg anchors` (`docs/design/tls.md` item 8).
const FILE_MAX: usize = 32 * 1024;

/// The TLS test server of `ok-ecdsa-p256`, answering `/` with 200 `text/html`.
fn server() -> TestServer {
    let mut config = Config::default();
    config.responses.insert(
        "/".to_string(),
        Response {
            status: 200,
            reason: "OK".to_string(),
            fields: vec![("Content-Type".to_string(), "text/html".to_string())],
            body: b"<!doctype html><p>ok</p>".to_vec(),
        },
    );
    TestServer::start(config).expect("the TLS test server starts")
}

fn address(server: &TestServer) -> String {
    format!("https://localhost:{}/", server.port())
}

fn root_path() -> PathBuf {
    certs_dir().join("ok-ecdsa-p256").join("root.pem")
}

/// The root of `ok-ecdsa-p256`, the one anchor its chain leads to.
fn root() -> String {
    fs::read_to_string(root_path()).expect("the root of the case")
}

fn page_line(text: &str) -> String {
    text.lines()
        .find(|line| line.starts_with("page "))
        .unwrap_or_else(|| panic!("no page line in:\n{text}"))
        .to_string()
}

/// Texts of `arg anchors` the session must refuse, each with its name: no PEM at all, a leaf
/// that is no CA, a `PRIVATE KEY` block, 65 blocks, and the root padded to 32 KiB and one byte.
fn refused_texts() -> Vec<(&'static str, String)> {
    let chain = fs::read_to_string(certs_dir().join("ok-ecdsa-p256").join("chain.pem")).unwrap();
    let end = "-----END CERTIFICATE-----\n";
    let leaf = chain[..chain.find(end).expect("a block") + end.len()].to_string();
    let root = root();
    let mut large = root.clone();
    large.push_str(&"#".repeat(FILE_MAX + 1 - root.len()));
    vec![
        ("not PEM", "these are not certificates".to_string()),
        ("leaf", leaf),
        (
            "private key",
            "-----BEGIN PRIVATE KEY-----\nZm9v\n-----END PRIVATE KEY-----\n".to_string(),
        ),
        (
            "65 certificate blocks",
            "-----BEGIN CERTIFICATE-----\n-----END CERTIFICATE-----\n".repeat(65),
        ),
        ("32 KiB and one byte", large),
    ]
}

/// `open <https address>` with the root as `arg anchors` loads the address in that same answer:
/// the anchors are in use for the load of the `open` itself, not only for later requests.
#[test]
fn adversary_anchors_an_open_with_an_https_address_loads_it_with_its_own_anchors() {
    let server = server();
    let address = address(&server);
    let mut session = Session::new(ID, surf_session::fetch).unwrap();
    let reply = session.answer(&request(
        "open",
        None,
        None,
        &[("url", &address), ("anchors", &root())],
    ));
    let text = reply.to_link().text;
    assert_eq!(reply.status, 0, "{text}");
    assert_eq!(
        page_line(&text),
        format!("page 200 \"text/html\" \"{address}\""),
        "{text}"
    );
}

/// The same through the built programs, the option before and after the address.
#[test]
fn adversary_anchors_surf_agent_open_ca_file_with_an_https_address_loads_it() {
    let server = server();
    let address = address(&server);
    for words in [
        ["open", "--ca-file", root_path().to_str().unwrap(), &address],
        ["open", &address, "--ca-file", root_path().to_str().unwrap()],
    ] {
        let dir = TestDir::new();
        let output = run(agent(&dir).args(words));
        let text = stdout(&output);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{words:?}: {}",
            stderr(&output)
        );
        assert_eq!(
            page_line(&text),
            format!("page 200 \"text/html\" \"{address}\""),
            "{words:?}: {text}"
        );
    }
}

/// Text of `arg anchors` the session does not take is `did refused usage`, status 2, and the
/// session is not opened: an `open` without anchors after it is still taken.
#[test]
fn adversary_anchors_the_session_refuses_anchor_text_it_does_not_take() {
    for (name, text) in refused_texts() {
        let mut session = Session::new(ID, surf_session::fetch).unwrap();
        let reply = session.answer(&request("open", None, None, &[("anchors", &text)]));
        let answer = reply.to_link().text;
        assert_eq!(reply.status, 2, "{name}: {answer}");
        assert!(answer.contains("did refused usage"), "{name}: {answer}");
        assert!(!answer.contains("token "), "{name}: {answer}");
        let reply = session.answer(&request("open", None, None, &[]));
        assert_eq!(reply.status, 0, "{name}: open after the refusal");
    }
}

/// Over the wire of an in-process `surfd` with the built `surf-session`: each refused text, two
/// `arg anchors`, and `arg anchors` on `go` are status 2 and leave no session behind; a request
/// of exactly `REQUEST_MAX` bytes whose anchors the session takes opens a session that loads
/// the server; one byte more is status 2 from `surfd` and opens none.
#[test]
fn adversary_anchors_surfd_and_the_session_on_hostile_anchor_requests() {
    let dir = TestDir::new();
    let _surfd = surfd_in_process(&dir, |_| {});
    let before = {
        let mut found = entries_under(&dir.surf());
        found.sort();
        found
    };
    let left = || {
        let mut found = entries_under(&dir.surf());
        found.sort();
        found
    };
    for (name, text) in refused_texts() {
        let answer = wire_call(&dir, &request("open", None, None, &[("anchors", &text)]));
        assert_eq!(answer.status, 2, "{name}: {answer:?}");
        assert!(
            answer.text.contains("did refused usage"),
            "{name}: {answer:?}"
        );
        assert_eq!(left(), before, "{name}: a session was left");
    }

    // Two `arg anchors`: `Request::encode` refuses them, so the bytes are written here.
    let root = root();
    let quoted = surf_answer::quote(&root);
    let twice =
        format!("surf-wire 1\ncommand open\narg anchors {quoted}\narg anchors {quoted}\nend\n");
    let answer = raw_call(&dir, twice.as_bytes()).expect("an answer to two arg anchors");
    assert_eq!(answer.status, 2, "{answer:?}");
    assert_eq!(left(), before, "two arg anchors left a session");

    // `arg anchors` on `go` of an open session.
    let server = server();
    let address = address(&server);
    let session = opened(&wire_call(&dir, &request("open", None, None, &[])).text);
    let answer = wire_call(
        &dir,
        &request(
            "go",
            Some(&session.id),
            Some(&session.token),
            &[("url", &address), ("anchors", &root)],
        ),
    );
    assert_eq!(answer.status, 2, "go with anchors: {answer:?}");
    let answer = wire_call(
        &dir,
        &request(
            "go",
            Some(&session.id),
            Some(&session.token),
            &[("url", &address)],
        ),
    );
    assert_eq!(
        page_line(&answer.text),
        format!("page failed CertificateNotTrusted \"{address}\""),
        "go after go with anchors: {answer:?}"
    );

    // A request of exactly REQUEST_MAX bytes: the root, then line feeds (4 bytes each on the
    // wire) and `#` (1 byte) outside any block.
    let base = request("open", None, None, &[("anchors", &root)])
        .encode()
        .unwrap()
        .len();
    let room = wire::REQUEST_MAX - base;
    let padded = format!("{root}{}{}", "\n".repeat(room / 4), "#".repeat(room % 4));
    assert!(padded.len() <= FILE_MAX, "{} bytes", padded.len());
    let exact = request("open", None, None, &[("anchors", &padded)])
        .encode()
        .unwrap();
    assert_eq!(exact.len(), wire::REQUEST_MAX);
    let answer = raw_call(&dir, &exact).expect("an answer at REQUEST_MAX");
    assert_eq!(answer.status, 0, "{answer:?}");
    let session = opened(&answer.text);
    let answer = wire_call(
        &dir,
        &request(
            "go",
            Some(&session.id),
            Some(&session.token),
            &[("url", &address)],
        ),
    );
    assert_eq!(
        page_line(&answer.text),
        format!("page 200 \"text/html\" \"{address}\""),
        "{answer:?}"
    );
    let sessions = left();

    // One byte more.
    let mut over = exact.clone();
    let at = over.len() - "\"\nend\n".len();
    over.insert(at, b'#');
    assert_eq!(over.len(), wire::REQUEST_MAX + 1);
    let answer = raw_call(&dir, &over).expect("an answer over REQUEST_MAX");
    assert_eq!(answer.status, 2, "{answer:?}");
    assert_eq!(
        left(),
        sessions,
        "a request over REQUEST_MAX left a session"
    );
}

/// The bytes `bytes` to the case's `surfd` on one connection, and its answer, or `None` when it
/// closes the connection without one.
fn raw_call(dir: &TestDir, bytes: &[u8]) -> Option<wire::Answer> {
    let mut stream = UnixStream::connect(dir.surf().join("surfd.sock")).expect("surfd listens");
    let _ = stream.write_all(bytes);
    let _ = stream.shutdown(std::net::Shutdown::Write);
    wire::read_answer(&mut stream, Duration::from_secs(30)).ok()
}

/// After a session with anchors answered, the thread's fetch is back to the bundled list: a
/// fetch outside any answer does not trust the server the session's anchors trust.
#[test]
fn adversary_anchors_after_an_answer_the_thread_fetches_with_the_bundled_list_alone() {
    let server = server();
    let address = address(&server);
    let mut session = Session::new(ID, surf_session::fetch).unwrap();
    let reply = session.answer(&request("open", None, None, &[("anchors", &root())]));
    assert_eq!(reply.status, 0, "{}", reply.to_link().text);
    let token = opened(&reply.to_link().text).token;
    let reply = session.answer(&request("go", Some(ID), Some(&token), &[("url", &address)]));
    let text = reply.to_link().text;
    assert_eq!(
        page_line(&text),
        format!("page 200 \"text/html\" \"{address}\""),
        "{text}"
    );
    let started = surf::RequestStarted {
        request_id: surf_types::net::RequestId(surf_types::primitives::Uuid(
            "00000000-0000-4000-8000-0000000000a1".to_string(),
        )),
        url: surf_types::url::Url(address.clone()),
        method: surf::Method::Get,
        body: None,
        body_type: None,
        initiator: None,
    };
    let answer = surf_session::fetch(&started);
    assert!(
        matches!(answer, surf::Answer::Failure(_)),
        "a fetch outside the answer used the session's anchors: {answer:?}"
    );
}
