//! Cases 1 to 20 of story `agent-fetch-redirects`: redirects followed inside one fetch, each
//! against servers the test starts on `127.0.0.1` port 0 and stops before it ends.
//!
//! A server answers its connections one after another and records the request target and the
//! `Host` of every request it read. A redirect it sends has a body with no declared length, and
//! the server holds the connection open until the client closes it: a client that read the body
//! of a redirect would wait for its read timeout and fail. Every other reply declares its length.

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use surf_fetch::{fetch, Failure, Limits, Response, MAX_ADDRESS_BYTES};

/// The longest a server waits for a client to send or to close.
const BOUND: Duration = Duration::from_secs(5);

/// The most bytes a server reads of one request head.
const REQUEST_HEAD_MAX: usize = 256 * 1024;

/// What a server read of one request.
#[derive(Debug, Clone)]
struct Seen {
    target: String,
    host: String,
}

/// What a server answers to one request.
struct Reply {
    status: u16,
    fields: Vec<(&'static str, Vec<u8>)>,
    body: Vec<u8>,
    /// `Content-Length` declared and the connection closed after the body; otherwise no length
    /// and the connection held until the client closes it.
    framed: bool,
    /// Waited before the reply is written.
    delay: Duration,
}

/// A redirect: `status` with one `Location`, a body of no declared length.
fn redirect(status: u16, location: impl Into<Vec<u8>>) -> Reply {
    Reply {
        status,
        fields: vec![("Location", location.into())],
        body: b"moved".to_vec(),
        framed: false,
        delay: Duration::ZERO,
    }
}

/// A 200 `text/html` whose body names `target`.
fn page(target: &str) -> Reply {
    Reply {
        status: 200,
        fields: vec![("Content-Type", b"text/html".to_vec())],
        body: format!("<p>{target}</p>").into_bytes(),
        framed: true,
        delay: Duration::ZERO,
    }
}

type Route = Box<dyn Fn(&str) -> Reply + Send>;

struct Server {
    port: u16,
    seen: Arc<Mutex<Vec<Seen>>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Server {
    fn start(route: impl Fn(&str) -> Reply + Send + 'static) -> Server {
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

    fn targets(self) -> Vec<String> {
        self.finish().into_iter().map(|seen| seen.target).collect()
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
    let Some(head) = read_head(&mut stream) else {
        return;
    };
    let text = String::from_utf8_lossy(&head).into_owned();
    let target = text
        .split(' ')
        .nth(1)
        .unwrap_or_default()
        .to_string();
    let host = text
        .split("\r\n")
        .find_map(|line| line.strip_prefix("Host: "))
        .unwrap_or_default()
        .to_string();
    let reply = route(&target);
    seen.lock().expect("seen").push(Seen { target, host });
    if !reply.delay.is_zero() {
        thread::sleep(reply.delay);
    }
    let mut bytes = format!("HTTP/1.1 {} Status\r\n", reply.status).into_bytes();
    for (name, value) in &reply.fields {
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(b": ");
        bytes.extend_from_slice(value);
        bytes.extend_from_slice(b"\r\n");
    }
    if reply.framed {
        bytes.extend_from_slice(format!("Content-Length: {}\r\n", reply.body.len()).as_bytes());
    }
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(&reply.body);
    let _ = stream.write_all(&bytes);
    let _ = stream.flush();
    if reply.framed {
        let _ = stream.shutdown(Shutdown::Write);
    }
    // Until the client closes, or BOUND passes.
    let mut buf = [0u8; 1024];
    while let Ok(n) = stream.read(&mut buf) {
        if n == 0 {
            break;
        }
    }
}

fn read_head(stream: &mut TcpStream) -> Option<Vec<u8>> {
    let mut head = Vec::new();
    let mut buf = [0u8; 8192];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
        if head.len() > REQUEST_HEAD_MAX {
            return None;
        }
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return None,
            Ok(n) => head.extend_from_slice(&buf[..n]),
        }
    }
    Some(head)
}

fn limits() -> Limits {
    Limits {
        connect_timeout: Duration::from_secs(2),
        read_timeout: Duration::from_secs(5),
        ..Limits::default()
    }
}

fn ok(result: Result<Response, Failure>) -> Response {
    match result {
        Ok(response) => response,
        Err(failure) => panic!("expected a response, got {failure:?}"),
    }
}

fn assert_malformed(result: &Result<Response, Failure>) {
    assert!(
        matches!(result, Err(Failure::MalformedResponse(_))),
        "expected MalformedResponse, got {result:?}"
    );
}

fn assert_too_many(result: &Result<Response, Failure>) {
    assert!(
        matches!(result, Err(Failure::TooManyRedirects(_))),
        "expected TooManyRedirects, got {result:?}"
    );
}

fn assert_unsupported(result: &Result<Response, Failure>) {
    assert!(
        matches!(result, Err(Failure::UnsupportedScheme(_))),
        "expected UnsupportedScheme, got {result:?}"
    );
}

/// `/a` answers `status` with `Location: /b`, `/b` a 200.
fn follows(status: u16) {
    let server = Server::start(move |target| match target {
        "/a" => redirect(status, "/b"),
        other => page(other),
    });
    let result = fetch(&server.url("/a"), &limits());
    let expected = server.url("/b");
    let targets = server.targets();
    let response = ok(result);
    assert_eq!(response.status, 200);
    assert_eq!(response.media_type, "text/html");
    assert_eq!(response.body, b"<p>/b</p>");
    assert_eq!(response.url, expected);
    assert_eq!(targets, ["/a", "/b"]);
}

#[test]
fn case_1_a_301_is_followed() {
    follows(301);
}

#[test]
fn case_2_a_302_is_followed() {
    follows(302);
}

#[test]
fn case_3_a_303_is_followed() {
    follows(303);
}

#[test]
fn case_4_a_307_is_followed() {
    follows(307);
}

#[test]
fn case_5_a_308_is_followed() {
    follows(308);
}

#[test]
fn case_6_a_relative_location_resolves_against_the_current_url() {
    let server = Server::start(|target| match target {
        "/dir/a" => redirect(302, "b?q"),
        other => page(other),
    });
    let result = fetch(&server.url("/dir/a"), &limits());
    let expected = server.url("/dir/b?q");
    let targets = server.targets();
    assert_eq!(ok(result).url, expected);
    assert_eq!(targets, ["/dir/a", "/dir/b?q"]);
}

/// `/0` to `/<last - 1>` each redirect to the next; `/<last>` answers `last_answer`.
fn chain(last: usize, last_redirects: bool) -> Server {
    Server::start(move |target| {
        let n: usize = target
            .strip_prefix('/')
            .and_then(|n| n.parse().ok())
            .unwrap_or(usize::MAX);
        if n < last || (n == last && last_redirects) {
            redirect(302, format!("/{}", n + 1))
        } else {
            page(target)
        }
    })
}

#[test]
fn case_7_a_chain_of_20_redirects_ends_at_the_21st_response() {
    let server = chain(20, false);
    let result = fetch(&server.url("/0"), &limits());
    let expected = server.url("/20");
    let targets = server.targets();
    let response = ok(result);
    assert_eq!(response.status, 200);
    assert_eq!(response.url, expected);
    assert_eq!(targets.len(), 21);
    assert_eq!(targets.last().map(String::as_str), Some("/20"));
}

#[test]
fn case_8_a_chain_of_21_redirects_is_too_many_after_21_requests() {
    let server = chain(20, true);
    let result = fetch(&server.url("/0"), &limits());
    let targets = server.targets();
    assert_too_many(&result);
    assert_eq!(targets.len(), 21);
    assert!(!targets.iter().any(|t| t == "/21"), "{targets:?}");
    let failure = result.expect_err("a failure");
    assert_eq!(failure.to_string().split(':').next(), Some("too many redirects"));
    assert!(failure.detail().contains("20"), "{failure}");
}

#[test]
fn case_9_a_loop_is_too_many_after_21_requests() {
    let server = Server::start(|target| match target {
        "/a" => redirect(301, "/a"),
        other => page(other),
    });
    let result = fetch(&server.url("/a"), &limits());
    let targets = server.targets();
    assert_too_many(&result);
    assert_eq!(targets.len(), 21);
    assert!(targets.iter().all(|t| t == "/a"), "{targets:?}");
}

#[test]
fn case_10_an_https_location_is_unsupported_and_never_connected_to() {
    let held = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port2 = held.local_addr().expect("local address").port();
    let server = Server::start(move |target| match target {
        "/a" => redirect(302, format!("https://127.0.0.1:{port2}/")),
        other => page(other),
    });
    let result = fetch(&server.url("/a"), &limits());
    let targets = server.targets();
    held.set_nonblocking(true).expect("nonblocking");
    match held.accept() {
        Err(e) if e.kind() == ErrorKind::WouldBlock => {}
        other => panic!("the https: target was connected to: {other:?}"),
    }
    assert_unsupported(&result);
    assert_eq!(targets, ["/a"]);
}

#[test]
fn case_11_a_file_location_is_unsupported() {
    let server = Server::start(|target| match target {
        "/a" => redirect(302, "file:///etc/passwd"),
        other => page(other),
    });
    let result = fetch(&server.url("/a"), &limits());
    let targets = server.targets();
    assert_unsupported(&result);
    assert_eq!(targets, ["/a"]);
}

#[test]
fn case_12_a_302_without_location_and_a_300_with_one_are_responses() {
    for (status, fields) in [
        (302, vec![]),
        (300, vec![("Location", b"/b".to_vec())]),
    ] {
        let server = Server::start(move |_| Reply {
            status,
            fields: fields.clone(),
            body: b"choose".to_vec(),
            framed: true,
            delay: Duration::ZERO,
        });
        let asked = server.url("/a");
        let result = fetch(&asked, &limits());
        let targets = server.targets();
        let response = ok(result);
        assert_eq!(response.status, status);
        assert_eq!(response.body, b"choose");
        assert_eq!(response.url, asked);
        assert_eq!(targets, ["/a"], "status {status}");
    }
}

#[test]
fn case_13_a_location_that_does_not_parse_is_malformed() {
    let server = Server::start(|target| match target {
        "/a" => redirect(302, "http://[::1/"),
        other => page(other),
    });
    let result = fetch(&server.url("/a"), &limits());
    let targets = server.targets();
    assert_malformed(&result);
    assert_eq!(targets, ["/a"]);
}

#[test]
fn case_14_a_redirect_to_another_server_is_followed_there() {
    let second = Server::start(page);
    let port2 = second.port;
    let first = Server::start(move |target| match target {
        "/a" => redirect(307, format!("http://127.0.0.1:{port2}/there")),
        other => page(other),
    });
    let result = fetch(&first.url("/a"), &limits());
    let first_targets = first.targets();
    let seen = second.finish();
    let response = ok(result);
    assert_eq!(response.status, 200);
    assert_eq!(response.url, format!("http://127.0.0.1:{port2}/there"));
    assert_eq!(first_targets, ["/a"]);
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].target, "/there");
    assert_eq!(seen[0].host, format!("127.0.0.1:{port2}"));
}

#[test]
fn case_15_the_read_timeout_bounds_the_whole_chain() {
    let server = Server::start(|target| {
        let n: usize = target
            .strip_prefix('/')
            .and_then(|n| n.parse().ok())
            .unwrap_or(usize::MAX);
        if n < 4 {
            Reply {
                delay: Duration::from_millis(400),
                ..redirect(302, format!("/{}", n + 1))
            }
        } else {
            page(target)
        }
    });
    let limits = Limits {
        read_timeout: Duration::from_secs(1),
        ..limits()
    };
    let started = Instant::now();
    let result = fetch(&server.url("/0"), &limits);
    let elapsed = started.elapsed();
    let targets = server.targets();
    assert!(
        matches!(result, Err(Failure::Unreachable(_))),
        "expected Unreachable, got {result:?}"
    );
    assert!(elapsed < Duration::from_secs(2), "took {elapsed:?}");
    assert!(!targets.iter().any(|t| t == "/4"), "{targets:?}");
}

#[test]
fn case_16_a_location_without_a_fragment_keeps_the_current_one() {
    let server = Server::start(|target| match target {
        "/a" => redirect(302, "/b"),
        "/x" => redirect(302, "/c#g"),
        other => page(other),
    });
    let kept = fetch(&server.url("/a#f"), &limits());
    let replaced = fetch(&server.url("/x#f"), &limits());
    let expected_kept = server.url("/b#f");
    let expected_replaced = server.url("/c#g");
    let targets = server.targets();
    assert_eq!(ok(kept).url, expected_kept);
    assert_eq!(ok(replaced).url, expected_replaced);
    assert_eq!(targets, ["/a", "/b", "/x", "/c"]);
    assert!(!targets.iter().any(|t| t.contains('#')), "{targets:?}");
}

#[test]
fn case_17_a_resolved_address_of_65536_bytes_is_followed_and_65537_is_malformed() {
    assert_eq!(MAX_ADDRESS_BYTES, 65_536);
    for (location, length) in [("b", 65_536), ("bb", 65_537)] {
        let server = Server::start(move |target| {
            if target.ends_with("/z") {
                redirect(302, location)
            } else {
                page("long")
            }
        });
        // The asked address ends in `/z`; `location` replaces that last segment.
        let prefix = server.url("/");
        let padding = 65_536 - prefix.len() - "/b".len();
        let asked = format!("{prefix}{}/z", "p".repeat(padding));
        assert_eq!(asked.len(), 65_536);
        let result = fetch(&asked, &limits());
        let targets = server.targets();
        if length == 65_536 {
            let response = ok(result);
            assert_eq!(response.status, 200);
            assert_eq!(response.url.len(), 65_536);
            assert!(response.url.ends_with("/b"));
            assert_eq!(targets.len(), 2);
        } else {
            assert_malformed(&result);
            assert_eq!(targets.len(), 1);
        }
    }
}

#[test]
fn case_18_a_location_that_is_not_utf8_is_malformed() {
    let server = Server::start(|target| match target {
        "/a" => redirect(302, b"/\xff".to_vec()),
        other => page(other),
    });
    let result = fetch(&server.url("/a"), &limits());
    let targets = server.targets();
    assert_malformed(&result);
    assert_eq!(targets, ["/a"]);
}

#[test]
fn case_19_two_location_fields_are_malformed() {
    let server = Server::start(|target| match target {
        "/a" => Reply {
            fields: vec![("Location", b"/b".to_vec()), ("Location", b"/c".to_vec())],
            ..redirect(302, "")
        },
        other => page(other),
    });
    let result = fetch(&server.url("/a"), &limits());
    let targets = server.targets();
    assert_malformed(&result);
    assert_eq!(targets, ["/a"]);
}

#[test]
fn case_20_without_a_redirect_the_url_is_the_asked_address_byte_for_byte() {
    let server = Server::start(page);
    let asked = format!("HTTP://127.0.0.1:{}", server.port);
    let result = fetch(&asked, &limits());
    let targets = server.targets();
    let response = ok(result);
    assert_eq!(response.status, 200);
    assert_eq!(response.url, asked);
    assert_eq!(targets, ["/"]);
}
