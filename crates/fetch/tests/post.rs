//! Cases 1 to 13 of the story that sends a `POST` (wave 9): `surf_fetch::send` of a `Post` with a body and its
//! media type, the checks before any connection, and the method a redirect continues with, each
//! against a server the test starts on `127.0.0.1` port 0 and stops before it ends.
//!
//! A server reads the head of a request and then `Content-Length` bytes before it answers, then
//! keeps every byte that arrives until the client closes: a request that sent more than it
//! declared shows those bytes. Every reply declares its length and closes its side after it.

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use surf_fetch::{send, Failure, Limits, Method, Request, Response};

/// The longest a server waits for a client to send or to close.
const BOUND: Duration = Duration::from_secs(5);

/// The most bytes a server reads of one request head.
const REQUEST_HEAD_MAX: usize = 64 * 1024;

/// The media type of a form's body.
const FORM: &str = "application/x-www-form-urlencoded";

/// What a server read of one request.
#[derive(Debug, Clone, Default)]
struct Seen {
    /// The request line without its line terminator.
    line: String,
    /// The header fields in order, names in lower case.
    fields: Vec<(String, String)>,
    /// The `Content-Length` bytes after the head.
    body: Vec<u8>,
    /// Every byte after those, until the client closed.
    after: Vec<u8>,
}

impl Seen {
    fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    fn names(&self) -> Vec<&str> {
        self.fields.iter().map(|(n, _)| n.as_str()).collect()
    }
}

type Route = Box<dyn Fn(&str) -> Vec<u8> + Send>;

struct Server {
    port: u16,
    seen: Arc<Mutex<Vec<Seen>>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Server {
    /// A server that answers the request target with what `route` returns.
    fn start(route: impl Fn(&str) -> Vec<u8> + Send + 'static) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
        let port = listener.local_addr().expect("local address").port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let route: Route = Box::new(route);
        let handle = {
            let seen = Arc::clone(&seen);
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                for stream in listener.incoming() {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    answer(stream, &route, &seen);
                }
            })
        };
        Server {
            port,
            seen,
            stop,
            handle: Some(handle),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    /// Stop the server and return what it read.
    fn finish(mut self) -> Vec<Seen> {
        self.halt();
        self.seen.lock().expect("seen").clone()
    }

    fn halt(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.stop.store(true, Ordering::SeqCst);
            // Wake the accepting thread; it sees the flag and ends.
            let _ = TcpStream::connect(("127.0.0.1", self.port));
            handle.join().expect("server thread");
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.halt();
    }
}

fn answer(mut stream: TcpStream, route: &Route, seen: &Mutex<Vec<Seen>>) {
    let _ = stream.set_read_timeout(Some(BOUND));
    let _ = stream.set_write_timeout(Some(BOUND));
    let Some((head, mut rest)) = read_head(&mut stream) else {
        return;
    };
    let text = String::from_utf8_lossy(&head).into_owned();
    let mut lines = text.split("\r\n");
    let line = lines.next().unwrap_or_default().to_owned();
    let fields: Vec<(String, String)> = lines
        .take_while(|l| !l.is_empty())
        .filter_map(|l| l.split_once(':'))
        .map(|(n, v)| (n.to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    let length: usize = fields
        .iter()
        .find(|(n, _)| n == "content-length")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    // The declared body is read before the server answers.
    let mut buf = [0u8; 8192];
    while rest.len() < length {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => rest.extend_from_slice(&buf[..n]),
        }
    }
    let after_body = rest.split_off(length.min(rest.len()));
    let target = line.split(' ').nth(1).unwrap_or_default().to_owned();
    let reply = route(&target);
    let _ = stream.write_all(&reply);
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Write);
    // Until the client closes, or BOUND passes.
    let mut after = after_body;
    while let Ok(n) = stream.read(&mut buf) {
        if n == 0 {
            break;
        }
        after.extend_from_slice(&buf[..n]);
    }
    seen.lock().expect("seen").push(Seen {
        line,
        fields,
        body: rest,
        after,
    });
}

/// The head up to and including its empty line, and the bytes read after it.
fn read_head(stream: &mut TcpStream) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut bytes = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            let rest = bytes.split_off(end + 4);
            return Some((bytes, rest));
        }
        if bytes.len() > REQUEST_HEAD_MAX {
            return None;
        }
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return None,
            Ok(n) => bytes.extend_from_slice(&buf[..n]),
        }
    }
}

/// A reply with `status`, the fields given, and a body of `body`.
fn reply(status: u16, fields: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut bytes = format!("HTTP/1.1 {status} Status\r\n").into_bytes();
    for (name, value) in fields {
        bytes.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    bytes.extend_from_slice(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
    bytes.extend_from_slice(body);
    bytes
}

/// A 200 `text/plain` whose body is `done`.
fn done() -> Vec<u8> {
    reply(200, &[("Content-Type", "text/plain")], b"done")
}

fn limits() -> Limits {
    Limits {
        connect_timeout: Duration::from_secs(2),
        read_timeout: Duration::from_secs(5),
        ..Limits::default()
    }
}

fn post<'a>(address: &'a str, body: Option<&'a [u8]>, media_type: Option<&'a str>) -> Request<'a> {
    Request {
        address,
        method: Method::Post,
        body,
        media_type,
    }
}

fn ok(result: Result<Response, Failure>) -> Response {
    match result {
        Ok(response) => response,
        Err(failure) => panic!("expected a response, got {failure:?}"),
    }
}

/// One `Post` to a server that answers `done`: the response and the one request it read.
fn post_once(body: Option<&[u8]>, media_type: Option<&str>, limits: &Limits) -> Seen {
    let server = Server::start(|_| done());
    let address = server.url("/form");
    let result = send(&post(&address, body, media_type), limits);
    let seen = server.finish();
    let response = ok(result);
    assert_eq!(
        (response.status, response.body.as_slice()),
        (200, &b"done"[..])
    );
    assert_eq!(seen.len(), 1, "{seen:?}");
    seen.into_iter().next().unwrap_or_default()
}

/// A listener on 127.0.0.1 port 0 that never blocks: `accept` answers `WouldBlock` as long as
/// nobody connected.
fn silent_listener() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.set_nonblocking(true).expect("non-blocking");
    let address = format!("http://{}/form", listener.local_addr().expect("address"));
    (listener, address)
}

/// `request` ends `Unreachable` and nothing connected to `listener`.
fn assert_unreachable_without_connecting(listener: &TcpListener, request: &Request<'_>) {
    let result = send(request, &limits());
    assert!(
        matches!(result, Err(Failure::Unreachable(_))),
        "{request:?}: expected Unreachable, got {result:?}"
    );
    match listener.accept() {
        Err(error) => assert_eq!(error.kind(), ErrorKind::WouldBlock, "{error}"),
        Ok((_, peer)) => panic!("{request:?}: {peer} connected"),
    }
}

/// `/form` answers `status` with `Location: /next`, `/next` is `done`: the two requests read.
fn redirected_post(status: u16) -> Vec<Seen> {
    let server = Server::start(move |target| match target {
        "/form" => reply(status, &[("Location", "/next")], b""),
        _ => done(),
    });
    let address = server.url("/form");
    let result = send(&post(&address, Some(b"a=1&b=2"), Some(FORM)), &limits());
    let expected = server.url("/next");
    let seen = server.finish();
    let response = ok(result);
    assert_eq!(response.url, expected);
    assert_eq!(response.body, b"done");
    assert_eq!(seen.len(), 2, "{seen:?}");
    let first = &seen[0];
    assert_eq!(first.line, "POST /form HTTP/1.1");
    assert_eq!(first.body, b"a=1&b=2");
    seen
}

/// After a 301, 302 or 303 the second request is a `GET` with neither field and no byte after
/// its head.
fn continues_as_get(status: u16) {
    let seen = redirected_post(status);
    let second = &seen[1];
    assert_eq!(second.line, "GET /next HTTP/1.1", "{status}");
    assert_eq!(second.field("content-type"), None, "{status}: {second:?}");
    assert_eq!(second.field("content-length"), None, "{status}: {second:?}");
    assert_eq!(second.body, b"", "{status}");
    assert_eq!(
        second.after, b"",
        "{status}: bytes after the head of the GET"
    );
}

/// After a 307 or 308 the second request repeats the method, the media type and the body.
fn repeats_method_and_body(status: u16) {
    let seen = redirected_post(status);
    let second = &seen[1];
    assert_eq!(second.line, "POST /next HTTP/1.1", "{status}");
    assert_eq!(second.field("content-type"), Some(FORM), "{status}");
    assert_eq!(second.field("content-length"), Some("7"), "{status}");
    assert_eq!(second.body, b"a=1&b=2", "{status}");
    assert_eq!(second.after, b"", "{status}");
}

#[test]
fn post_sends_method_media_type_length_and_body() {
    let body = b"user=ada&password=s3cret";
    let server = Server::start(|_| done());
    let address = server.url("/session");
    let result = send(&post(&address, Some(body), Some(FORM)), &limits());
    let seen = server.finish();
    let response = ok(result);
    assert_eq!(response.status, 200);
    assert_eq!(response.url, address);
    assert_eq!(seen.len(), 1, "{seen:?}");
    let request = &seen[0];
    assert_eq!(request.line, "POST /session HTTP/1.1");
    assert_eq!(
        request.names(),
        ["host", "connection", "content-type", "content-length"]
    );
    assert_eq!(request.field("content-type"), Some(FORM));
    assert_eq!(request.field("content-length"), Some("24"));
    assert_eq!(request.body, body);
    assert_eq!(request.after, b"");
}

#[test]
fn post_with_empty_or_absent_body_sends_content_length_0() {
    for body in [Some(&b""[..]), None] {
        let request = post_once(body, Some(FORM), &limits());
        assert_eq!(request.line, "POST /form HTTP/1.1", "{body:?}");
        assert_eq!(request.field("content-length"), Some("0"), "{body:?}");
        assert_eq!(request.body, b"", "{body:?}");
        assert_eq!(request.after, b"", "{body:?}: bytes after the head");
    }
}

#[test]
fn post_without_media_type_sends_no_content_type() {
    let request = post_once(Some(b"a=1"), None, &limits());
    assert_eq!(request.line, "POST /form HTTP/1.1");
    assert_eq!(request.names(), ["host", "connection", "content-length"]);
    assert_eq!(request.field("content-length"), Some("3"));
    assert_eq!(request.body, b"a=1");
}

#[test]
fn post_body_at_the_limit_is_sent() {
    let limits = Limits {
        max_request_body_bytes: 1_000,
        ..limits()
    };
    let body: Vec<u8> = (0..1_000u32).map(|i| (i % 251) as u8).collect();
    let request = post_once(Some(&body), Some(FORM), &limits);
    assert_eq!(request.field("content-length"), Some("1000"));
    assert_eq!(request.body, body);
    assert_eq!(request.after, b"");
}

#[test]
fn post_body_over_the_limit_fails_without_connecting() {
    let limits = Limits {
        max_request_body_bytes: 1_000,
        ..limits()
    };
    let (listener, address) = silent_listener();
    let body = vec![b'x'; 1_001];
    let result = send(&post(&address, Some(&body), Some(FORM)), &limits);
    match result {
        Err(Failure::Unreachable(detail)) => assert!(detail.contains("1000"), "{detail:?}"),
        other => panic!("expected Unreachable, got {other:?}"),
    }
    match listener.accept() {
        Err(error) => assert_eq!(error.kind(), ErrorKind::WouldBlock, "{error}"),
        Ok((_, peer)) => panic!("{peer} connected"),
    }
}

#[test]
fn post_media_type_not_type_slash_subtype_fails_without_connecting() {
    let (listener, address) = silent_listener();
    let long = format!("{}/plain", "a".repeat(128));
    for media_type in [
        "text/plain\r\nX-A: 1",
        "text/plain; charset=utf-8",
        "",
        long.as_str(),
    ] {
        assert_unreachable_without_connecting(
            &listener,
            &post(&address, Some(b"a=1"), Some(media_type)),
        );
    }
}

#[test]
fn redirect_303_after_post_continues_as_get_without_body() {
    continues_as_get(303);
}

#[test]
fn redirect_302_after_post_continues_as_get_without_body() {
    continues_as_get(302);
}

#[test]
fn redirect_301_after_post_continues_as_get_without_body() {
    continues_as_get(301);
}

#[test]
fn redirect_307_after_post_repeats_method_and_body() {
    repeats_method_and_body(307);
}

#[test]
fn redirect_308_after_post_repeats_method_and_body() {
    repeats_method_and_body(308);
}

#[test]
fn redirect_307_after_a_changed_method_stays_get() {
    let server = Server::start(|target| match target {
        "/form" => reply(303, &[("Location", "/a")], b""),
        "/a" => reply(307, &[("Location", "/b")], b""),
        _ => done(),
    });
    let address = server.url("/form");
    let result = send(&post(&address, Some(b"a=1"), Some(FORM)), &limits());
    let seen = server.finish();
    assert_eq!(ok(result).body, b"done");
    let lines: Vec<&str> = seen.iter().map(|s| s.line.as_str()).collect();
    assert_eq!(
        lines,
        ["POST /form HTTP/1.1", "GET /a HTTP/1.1", "GET /b HTTP/1.1"]
    );
    let third = &seen[2];
    assert_eq!(third.field("content-type"), None, "{third:?}");
    assert_eq!(third.field("content-length"), None, "{third:?}");
    assert_eq!(
        third.after, b"",
        "bytes after the head of the third request"
    );
}

#[test]
fn post_body_outside_ascii_arrives_unchanged() {
    let body = [0x00, 0x0d, 0x0a, 0x80, 0xc3, 0xa9, 0xff];
    let request = post_once(Some(&body), Some("application/octet-stream"), &limits());
    assert_eq!(request.field("content-length"), Some("7"));
    assert_eq!(request.body, body);
    assert_eq!(request.after, b"");
}
