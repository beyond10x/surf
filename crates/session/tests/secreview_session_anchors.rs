//! The trust scope of `surf-agent open --ca-file` (`story:tls-session-anchors`), checked from
//! the side of the session: the anchors of an `open` are in use only while that session
//! answers, they reach the load of the `open`'s own address and a redirect of a later `go`, no
//! later request adds or replaces them, the session holds the file to the bounds of
//! `docs/design/tls.md` item 8 whatever client sent it, and no answer, log, file or process of
//! the case holds the file's bytes. Against `surf-tls-test-server` on 127.0.0.1 with the
//! certificates of `crates/tls-test-certs/certs/`.

#![forbid(unsafe_code)]

use crate::common;

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use common::{
    TestDir, agent, agent_call, cmdline, contains, opened, regular_files_under, request, run,
    stderr, stdout, surfd_in_process, wire_call,
};
use surf_session::Session;
use surf_tls_test_server::{Config, Mode, Response, TestServer, certs_dir};
use surf_types::net::FailureReason;

const A: &str = "5e1f0c2a-7b3d-4c8e-9f10-a1b2c3d4e5f6";
const B: &str = "6f2a1d3b-8c4e-4d9f-8a21-b2c3d4e5f607";

const BODY: &[u8] = b"<!doctype html><p>ok</p>";

/// The TLS test server of `ok-ecdsa-p256`, answering `/` with 200 `text/html`.
fn server() -> TestServer {
    let mut config = Config {
        mode: Mode::Normal,
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

fn root_path(case: &str) -> PathBuf {
    certs_dir().join(case).join("root.pem")
}

fn root(case: &str) -> String {
    fs::read_to_string(root_path(case)).expect("the root of the case")
}

fn address(server: &TestServer) -> String {
    format!("https://localhost:{}/", server.port())
}

fn loaded(address: &str) -> String {
    format!("page 200 \"text/html\" \"{address}\"")
}

fn not_trusted(address: &str) -> String {
    format!("page failed CertificateNotTrusted \"{address}\"")
}

fn page_line(text: &str) -> String {
    text.lines()
        .find(|line| line.starts_with("page "))
        .unwrap_or_else(|| panic!("no page line in:\n{text}"))
        .to_string()
}

/// `open` on the session core, with `arg anchors` when `anchors` is given and `arg url` when
/// `url` is: the status and the answer text.
fn open_core<T: surf::Transport>(
    session: &mut Session<T>,
    anchors: Option<&str>,
    url: Option<&str>,
) -> (u8, String) {
    let mut args = Vec::new();
    if let Some(url) = url {
        args.push(("url", url));
    }
    if let Some(anchors) = anchors {
        args.push(("anchors", anchors));
    }
    let reply = session.answer(&request("open", None, None, &args));
    (reply.status, reply.to_link().text)
}

fn go_core<T: surf::Transport>(
    session: &mut Session<T>,
    id: &str,
    token: &str,
    url: &str,
) -> String {
    let reply = session.answer(&request("go", Some(id), Some(token), &[("url", url)]));
    assert_eq!(reply.status, 0, "go: {}", reply.to_link().text);
    reply.to_link().text
}

/// A `GET` of `url` as the session's transport sees one.
fn get(url: &str) -> surf::RequestStarted {
    surf::RequestStarted {
        request_id: surf_types::net::RequestId(surf_types::primitives::Uuid(
            "00000000-0000-4000-8000-0000000000a1".to_string(),
        )),
        url: surf_types::url::Url(url.to_string()),
        method: surf::Method::Get,
        body: None,
        body_type: None,
    }
}

/// An HTTP server on 127.0.0.1 that answers each of `count` connections with `302` to `to`.
fn redirect_server(to: String, count: usize) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port on 127.0.0.1");
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming().take(count) {
            let Ok(mut stream) = stream else { continue };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") && head.len() < 16 * 1024 {
                match stream.read(&mut byte) {
                    Ok(1) => head.push(byte[0]),
                    _ => break,
                }
            }
            let answer = format!(
                "HTTP/1.1 302 Found\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: \
                 close\r\n\r\n"
            );
            let _ = stream.write_all(answer.as_bytes());
        }
    });
    port
}

/// The anchors of a session are in use on its thread only while it answers: after its
/// answers, also after one whose transport panicked, a fetch on the same thread and a second
/// session opened there without a file authenticate against the bundled list alone. The
/// session's own later `go`, and a redirect from `http:` to `https:` inside it, use its file.
#[test]
fn the_anchors_of_a_session_are_in_use_only_while_it_answers() {
    let server = server();
    let address = address(&server);
    let redirect = format!("http://127.0.0.1:{}/", redirect_server(address.clone(), 2));
    let file = root("ok-ecdsa-p256");

    let transport = |request: &surf::RequestStarted| {
        let answer = surf_session::fetch(request);
        if request.url.0.ends_with("/panic") {
            panic!("the transport of this case panics after its fetch");
        }
        answer
    };
    let mut a = Session::new(A, transport).unwrap();
    let (status, text) = open_core(&mut a, Some(&file), None);
    assert_eq!(status, 0, "open with the file: {text}");
    let token_a = opened(&text).token;
    let text = go_core(&mut a, A, &token_a, &address);
    assert_eq!(page_line(&text), loaded(&address), "A, go: {text}");
    let text = go_core(&mut a, A, &token_a, &redirect);
    assert_eq!(
        page_line(&text),
        loaded(&address),
        "A, a redirect to https: {text}"
    );

    // Outside any answer, after A's: the bundled list alone.
    assert_eq!(
        surf_session::fetch(&get(&address)),
        surf::Answer::Failure(FailureReason::CertificateNotTrusted),
        "a fetch on A's thread after A's answer"
    );

    // An answer of A that ends in a panic.
    let panicked = panic::catch_unwind(AssertUnwindSafe(|| {
        a.answer(&request(
            "go",
            Some(A),
            Some(&token_a),
            &[("url", format!("{address}panic").as_str())],
        ))
    }));
    assert!(panicked.is_err(), "the transport panicked");
    assert_eq!(
        surf_session::fetch(&get(&address)),
        surf::Answer::Failure(FailureReason::CertificateNotTrusted),
        "a fetch on A's thread after A's answer that panicked"
    );

    let mut b = Session::new(B, surf_session::fetch).unwrap();
    let (status, text) = open_core(&mut b, None, None);
    assert_eq!(status, 0, "open without a file: {text}");
    let token_b = opened(&text).token;
    let text = go_core(&mut b, B, &token_b, &address);
    assert_eq!(page_line(&text), not_trusted(&address), "B, go: {text}");
    let text = go_core(&mut b, B, &token_b, &redirect);
    assert!(
        page_line(&text).starts_with("page failed CertificateNotTrusted "),
        "B, a redirect to https: {text}"
    );
}

/// `open <https address>` with a file loads the address through the file's anchors in the
/// answer to that `open`, not only from the next `go` on.
#[test]
fn an_open_with_an_address_loads_it_through_the_files_anchors() {
    let server = server();
    let address = address(&server);
    let mut session = Session::new(A, surf_session::fetch).unwrap();
    let (status, text) = open_core(&mut session, Some(&root("ok-ecdsa-p256")), Some(&address));
    assert_eq!(status, 0, "open: {text}");
    assert_eq!(page_line(&text), loaded(&address), "{text}");
}

/// Only the `open` that opens a session gives it anchors: a second `open` with a file on the
/// open session, and a `go` with `arg anchors`, are refused with status 2 and leave its trust
/// as it was, for a session opened without a file and for one opened with another root.
#[test]
fn no_request_after_the_open_adds_or_replaces_anchors() {
    let server = server();
    let address = address(&server);
    let right = root("ok-ecdsa-p256");
    for (id, first) in [(A, None), (B, Some(root("ok-ed25519")))] {
        let mut session = Session::new(id, surf_session::fetch).unwrap();
        let (status, text) = open_core(&mut session, first.as_deref(), None);
        assert_eq!(status, 0, "{id}: the first open: {text}");
        let token = opened(&text).token;

        let (second_open, second_text) = open_core(&mut session, Some(&right), None);
        let reply = session.answer(&request(
            "go",
            Some(id),
            Some(&token),
            &[("url", address.as_str()), ("anchors", right.as_str())],
        ));

        // The trust first, then how each request was answered.
        let text = go_core(&mut session, id, &token, &address);
        assert_eq!(page_line(&text), not_trusted(&address), "{id}: {text}");
        assert_eq!(
            second_open, 2,
            "{id}: a second open with a file: {second_text}"
        );
        assert_eq!(
            reply.status,
            2,
            "{id}: go with anchors: {}",
            reply.to_link().text
        );
    }
}

/// The base64 of `pem` without its BEGIN and END lines.
fn base64_of(pem: &str) -> String {
    pem.lines()
        .filter(|line| !line.starts_with("-----"))
        .collect()
}

/// The session holds the text of `arg anchors` to the bounds of item 8 itself, for a client
/// other than `surf-agent` that writes to the socket of `surfd`: a leaf that is no CA, a
/// `PRIVATE KEY` block, 32 KiB and one byte, 65 blocks. Each is `did refused usage`, status 2,
/// with no byte of the text in the answer, and leaves no session behind; a file at both bounds
/// (64 blocks within 32 KiB) is taken.
#[test]
fn the_session_holds_the_anchor_text_to_the_bounds_whatever_client_sent_it() {
    let dir = TestDir::new();
    let _daemon = surfd_in_process(&dir, |_| {});
    let p256 = root("ok-ecdsa-p256");
    let ed25519 = root("ok-ed25519");
    let chain = fs::read_to_string(certs_dir().join("ok-ecdsa-p256/chain.pem")).expect("the chain");
    let leaf_end = chain
        .find("-----END CERTIFICATE-----")
        .expect("an END line")
        + 26;
    let leaf = chain[..leaf_end].to_string();
    let private_key = format!(
        "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
        base64_of(&p256)
    );
    let mut too_large = p256.clone();
    while too_large.len() < 32 * 1024 + 1 {
        too_large.push('x');
    }
    too_large.truncate(32 * 1024 + 1);
    let too_many = ed25519.repeat(65);
    assert!(too_many.len() <= 32 * 1024, "65 blocks fit in 32 KiB");

    for (label, text) in [
        ("a leaf that is no CA", &leaf),
        ("a PRIVATE KEY block", &private_key),
        ("32 KiB and one byte", &too_large),
        ("65 blocks", &too_many),
    ] {
        let answer = wire_call(
            &dir,
            &request("open", None, None, &[("anchors", text.as_str())]),
        );
        assert_eq!(answer.status, 2, "{label}: {answer:?}");
        assert!(
            answer.text.contains("did refused usage"),
            "{label}: {answer:?}"
        );
        let base64 = base64_of(text);
        let shown = format!("{}{}", answer.text, answer.reason.unwrap_or_default());
        assert!(
            !contains(shown.as_bytes(), &base64.as_bytes()[..64]),
            "{label}: the answer holds bytes of the text"
        );
        let sessions: Vec<String> = fs::read_dir(dir.surf())
            .unwrap()
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| surf_agent::wire::session_id(name).is_some())
            .collect();
        assert!(sessions.is_empty(), "{label}: sessions left: {sessions:?}");
    }

    let at_bounds = ed25519.repeat(64);
    assert!(at_bounds.len() <= 32 * 1024, "64 blocks fit in 32 KiB");
    let started = Instant::now();
    let answer = wire_call(
        &dir,
        &request("open", None, None, &[("anchors", at_bounds.as_str())]),
    );
    let took = started.elapsed();
    assert_eq!(answer.status, 0, "64 blocks within 32 KiB: {answer:?}");
    eprintln!("open with 64 blocks, {} bytes: {took:?}", at_bounds.len());
}

/// The view's answer to `target`, its head included, or an empty text.
fn view(dir: &TestDir, target: &str) -> String {
    let line = fs::read_to_string(dir.surf().join("view-url")).expect("view-url");
    let rest = line
        .strip_prefix("http://127.0.0.1:")
        .expect("http://127.0.0.1:");
    let (port, token) = rest.split_once("/#").expect("/#");
    let token = token.trim_end();
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
    let _ = stream.read_to_end(&mut answer);
    String::from_utf8_lossy(&answer).into_owned()
}

/// The built programs, `surf-agent open --ca-file <root> <https address>`, `go` and `log`:
/// the first and the last 64 base64 characters of the file are in none of their outputs, the
/// live view's list and session detail, a regular file under `XDG_RUNTIME_DIR`, or the
/// arguments and environment of `surfd` and `surf-session`.
#[test]
fn no_output_file_or_process_of_the_case_holds_the_bytes_of_the_file() {
    let dir = TestDir::new();
    let server = server();
    let address = address(&server);
    let file = root_path("ok-ecdsa-p256");
    let base64 = base64_of(&root("ok-ecdsa-p256"));
    let needles = [&base64[..64], &base64[base64.len() - 64..]];

    let output = run(agent(&dir)
        .arg("open")
        .arg("--ca-file")
        .arg(&file)
        .arg(&address));
    assert_eq!(output.status.code(), Some(0), "open: {}", stderr(&output));
    let session = opened(&stdout(&output));
    assert_eq!(
        page_line(&session.text),
        loaded(&address),
        "{}",
        session.text
    );
    let mut outputs = vec![("open".to_string(), output)];
    for words in [vec!["go", address.as_str()], vec!["log"]] {
        let output = agent_call(&dir, &words, &session.id, &session.token);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{words:?}: {}",
            stderr(&output)
        );
        outputs.push((words[0].to_string(), output));
    }

    let mut places: Vec<(String, Vec<u8>)> = Vec::new();
    for (name, output) in outputs {
        places.push((format!("{name}: standard output"), output.stdout));
        places.push((format!("{name}: standard error"), output.stderr));
    }
    for target in [
        "/api/sessions".to_string(),
        format!("/api/sessions/{}", session.id),
    ] {
        let answer = view(&dir, &target);
        assert!(answer.starts_with("HTTP/1.1 200 OK"), "{target}: {answer}");
        places.push((format!("view {target}"), answer.into_bytes()));
    }
    for path in regular_files_under(&dir.runtime()) {
        let bytes = fs::read(&path).unwrap_or_default();
        places.push((path.display().to_string(), bytes));
    }
    let mut programs = 0;
    for pid in dir.processes() {
        let program = cmdline(pid).first().cloned().unwrap_or_default();
        if program.ends_with(b"/surfd") || program.ends_with(b"/surf-session") {
            programs += 1;
            for part in ["cmdline", "environ"] {
                let bytes = fs::read(format!("/proc/{pid}/{part}")).unwrap_or_default();
                places.push((format!("/proc/{pid}/{part}"), bytes));
            }
        }
    }
    assert_eq!(programs, 2, "one surfd and one surf-session run");
    let holding: Vec<&str> = places
        .iter()
        .filter(|(_, bytes)| {
            needles
                .iter()
                .any(|needle| contains(bytes, needle.as_bytes()))
        })
        .map(|(place, _)| place.as_str())
        .collect();
    assert!(holding.is_empty(), "bytes of the file in: {holding:?}");
}
