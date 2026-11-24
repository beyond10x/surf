//! Cases 1 to 20 of story `agent-fetch-redirects`: redirects followed inside one fetch, each
//! against servers the test starts on `127.0.0.1` port 0 and stops before it ends. Cases 21 to
//! 26, of story `security-review-fixes`: the destination rule, over a resolver and a transport
//! of the test (`TestNetwork`), which give a public address or a name the addresses the case
//! chooses and connect them to the test's servers; no real name is resolved.
//!
//! Case 10 (a `Location: https:` ended `UnsupportedScheme`) is gone since `story:tls-fetch-https`:
//! an `https:` redirect is followed, and case 11 of the module `https` is that case rewritten.
//! The module `https` also uses this module's server, `TestNetwork` and limits.
//!
//! A server answers its connections one after another and records the request target and the
//! `Host` of every request it read. A redirect it sends has a body with no declared length, and
//! the server holds the connection open until the client closes it: a client that read the body
//! of a redirect would wait for its read timeout and fail. Every other reply declares its length.

use std::collections::{HashMap, VecDeque};
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use surf_fetch::{
    fetch, send_from, send_through, Failure, Limits, Method, Network, Origin, Request, Response,
    MAX_ADDRESS_BYTES,
};

/// The longest a server waits for a client to send or to close.
const BOUND: Duration = Duration::from_secs(5);

/// The most bytes a server reads of one request head.
const REQUEST_HEAD_MAX: usize = 256 * 1024;

/// What a server read of one request.
#[derive(Debug, Clone)]
pub(crate) struct Seen {
    pub(crate) target: String,
    pub(crate) host: String,
}

/// What a server answers to one request.
pub(crate) struct Reply {
    pub(crate) status: u16,
    pub(crate) fields: Vec<(&'static str, Vec<u8>)>,
    pub(crate) body: Vec<u8>,
    /// `Content-Length` declared and the connection closed after the body; otherwise no length
    /// and the connection held until the client closes it.
    pub(crate) framed: bool,
    /// Waited before the reply is written.
    pub(crate) delay: Duration,
}

/// A redirect: `status` with one `Location`, a body of no declared length.
pub(crate) fn redirect(status: u16, location: impl Into<Vec<u8>>) -> Reply {
    Reply {
        status,
        fields: vec![("Location", location.into())],
        body: b"moved".to_vec(),
        framed: false,
        delay: Duration::ZERO,
    }
}

/// A 200 `text/html` whose body names `target`.
pub(crate) fn page(target: &str) -> Reply {
    Reply {
        status: 200,
        fields: vec![("Content-Type", b"text/html".to_vec())],
        body: format!("<p>{target}</p>").into_bytes(),
        framed: true,
        delay: Duration::ZERO,
    }
}

type Route = Box<dyn Fn(&str) -> Reply + Send>;

pub(crate) struct Server {
    pub(crate) port: u16,
    seen: Arc<Mutex<Vec<Seen>>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Server {
    pub(crate) fn start(route: impl Fn(&str) -> Reply + Send + 'static) -> Server {
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

    pub(crate) fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    /// Stop the server and return what it read.
    pub(crate) fn finish(mut self) -> Vec<Seen> {
        self.halt();
        self.seen.lock().expect("seen").clone()
    }

    pub(crate) fn targets(self) -> Vec<String> {
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

pub(crate) fn limits() -> Limits {
    Limits {
        connect_timeout: Duration::from_secs(2),
        read_timeout: Duration::from_secs(5),
        ..Limits::default()
    }
}

pub(crate) fn ok(result: Result<Response, Failure>) -> Response {
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

// --- The destination rule (story `security-review-fixes`) ------------------------------------

/// A network of the test: a name resolves to the lists it is given, one per resolution in
/// order (the last one repeats), and nothing else resolves; a connection to an address it routes
/// goes to a server of the test on 127.0.0.1, one to 127.0.0.1 itself goes there, and every
/// other address is refused. It records every name resolved and every address dialed.
pub(crate) struct TestNetwork {
    names: Mutex<HashMap<String, VecDeque<Vec<IpAddr>>>>,
    routes: HashMap<SocketAddr, u16>,
    resolved: Mutex<Vec<String>>,
    dialed: Mutex<Vec<SocketAddr>>,
}

impl TestNetwork {
    pub(crate) fn new() -> TestNetwork {
        TestNetwork {
            names: Mutex::new(HashMap::new()),
            routes: HashMap::new(),
            resolved: Mutex::new(Vec::new()),
            dialed: Mutex::new(Vec::new()),
        }
    }

    /// `name` resolves to `answers`, one list per resolution.
    pub(crate) fn name(self, name: &str, answers: &[&[&str]]) -> TestNetwork {
        let lists = answers
            .iter()
            .map(|list| list.iter().map(|ip| ip.parse().unwrap()).collect())
            .collect();
        self.names.lock().unwrap().insert(name.to_string(), lists);
        self
    }

    /// A connection to `address` goes to the server on 127.0.0.1 at `port`.
    pub(crate) fn route(mut self, address: &str, port: u16) -> TestNetwork {
        self.routes.insert(address.parse().unwrap(), port);
        self
    }

    pub(crate) fn dialed(&self) -> Vec<String> {
        self.dialed
            .lock()
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn resolved(&self) -> Vec<String> {
        self.resolved.lock().unwrap().clone()
    }
}

impl Network for TestNetwork {
    fn resolve(&self, host: &str, port: u16, _: Duration) -> Result<Vec<SocketAddr>, Failure> {
        self.resolved.lock().unwrap().push(host.to_string());
        let mut names = self.names.lock().unwrap();
        let lists = names
            .get_mut(host)
            .ok_or_else(|| Failure::Unreachable(format!("the test does not resolve {host}")))?;
        let list = if lists.len() > 1 {
            lists.pop_front().unwrap()
        } else {
            lists.front().cloned().unwrap_or_default()
        };
        Ok(list
            .into_iter()
            .map(|ip| SocketAddr::new(ip, port))
            .collect())
    }

    fn connect(&self, address: &SocketAddr, timeout: Duration) -> std::io::Result<TcpStream> {
        self.dialed.lock().unwrap().push(*address);
        let loopback = IpAddr::from([127, 0, 0, 1]);
        match self.routes.get(address) {
            Some(port) => TcpStream::connect_timeout(&SocketAddr::new(loopback, *port), timeout),
            None if address.ip() == loopback => TcpStream::connect_timeout(address, timeout),
            None => Err(std::io::Error::new(
                ErrorKind::ConnectionRefused,
                "the test routes no connection to this address",
            )),
        }
    }
}

fn get(address: &str, network: &TestNetwork) -> Result<Response, Failure> {
    let request = Request {
        address,
        method: Method::Get,
        body: None,
        media_type: None,
        anchors: None,
        jar: None,
        initiator: None,
    };
    send_through(&request, &limits(), network)
}

fn assert_forbidden(result: &Result<Response, Failure>) {
    assert!(
        matches!(result, Err(Failure::ForbiddenDestination(_))),
        "expected ForbiddenDestination, got {result:?}"
    );
}

#[test]
fn case_21_a_chain_from_a_public_address_is_refused_at_each_local_address() {
    let local = Server::start(page);
    let port = local.port;
    let locations = [
        format!("http://127.0.0.1:{port}/"),
        "http://10.0.0.1/".to_string(),
        "http://169.254.169.254/latest".to_string(),
        "http://[::1]/".to_string(),
        "http://[fc00::1]/".to_string(),
        format!("http://[::ffff:127.0.0.1]:{port}/"),
    ];
    let routed = locations.clone();
    let public = Server::start(move |target| {
        let n: usize = target.trim_start_matches('/').parse().unwrap_or(usize::MAX);
        match routed.get(n) {
            Some(location) => redirect(302, location.as_str()),
            None => page(target),
        }
    });
    let network = TestNetwork::new().route("203.0.113.5:80", public.port);
    for (n, location) in locations.iter().enumerate() {
        let result = get(&format!("http://203.0.113.5/{n}"), &network);
        assert_forbidden(&result);
        let detail = result.unwrap_err().to_string();
        assert!(
            detail.starts_with("forbidden destination: "),
            "{location}: {detail}"
        );
    }
    assert_eq!(network.dialed(), vec!["203.0.113.5:80"; locations.len()]);
    assert!(network.resolved().is_empty(), "{:?}", network.resolved());
    assert_eq!(public.targets(), ["/0", "/1", "/2", "/3", "/4", "/5"]);
    assert!(local.targets().is_empty(), "the local server was reached");
}

#[test]
fn case_22_a_name_that_resolves_public_then_local_is_refused_at_the_redirect() {
    let public = Server::start(|target| match target {
        "/a" => redirect(302, "/b"),
        other => page(other),
    });
    let network = TestNetwork::new()
        .name("rebind.test", &[&["203.0.113.5"], &["10.0.0.1"]])
        .route("203.0.113.5:80", public.port);
    let result = get("http://rebind.test/a", &network);
    assert_forbidden(&result);
    assert_eq!(network.resolved(), ["rebind.test", "rebind.test"]);
    assert_eq!(network.dialed(), ["203.0.113.5:80"]);
    assert_eq!(public.targets(), ["/a"]);
}

#[test]
fn case_23_a_name_with_a_public_and_a_local_address_after_a_redirect_is_refused() {
    let first = Server::start(|target| match target {
        "/a" => redirect(307, "http://mixed.test/b"),
        other => page(other),
    });
    let second = Server::start(page);
    let network = TestNetwork::new()
        .name("mixed.test", &[&["198.51.100.7", "127.0.0.1"]])
        .route("203.0.113.5:80", first.port)
        .route("198.51.100.7:80", second.port);
    let result = get("http://203.0.113.5/a", &network);
    assert_forbidden(&result);
    assert_eq!(network.resolved(), ["mixed.test"]);
    assert_eq!(network.dialed(), ["203.0.113.5:80"]);
    assert_eq!(first.targets(), ["/a"]);
    assert!(second.targets().is_empty(), "the second server was reached");
}

#[test]
fn case_24_a_chain_that_starts_at_127_0_0_1_is_followed_to_local_and_public_addresses() {
    let public = Server::start(page);
    let second = Server::start(move |target| match target {
        "/there" => redirect(302, "http://203.0.113.5/public"),
        other => page(other),
    });
    let port2 = second.port;
    let first = Server::start(move |target| match target {
        "/a" => redirect(302, format!("http://127.0.0.1:{port2}/there")),
        other => page(other),
    });
    let network = TestNetwork::new().route("203.0.113.5:80", public.port);
    let response = ok(get(&first.url("/a"), &network));
    assert_eq!(response.status, 200);
    assert_eq!(response.url, "http://203.0.113.5/public");
    assert_eq!(
        network.dialed(),
        [
            format!("127.0.0.1:{}", first.port),
            format!("127.0.0.1:{port2}"),
            "203.0.113.5:80".to_string(),
        ]
    );
    assert_eq!(first.targets(), ["/a"]);
    assert_eq!(second.targets(), ["/there"]);
    assert_eq!(public.targets(), ["/public"]);
}

#[test]
fn case_25_a_chain_among_public_addresses_is_followed() {
    let second = Server::start(page);
    let first = Server::start(|target| match target {
        "/a" => redirect(301, "http://elsewhere.test/b"),
        other => page(other),
    });
    let network = TestNetwork::new()
        .name("elsewhere.test", &[&["198.51.100.7"]])
        .route("203.0.113.5:80", first.port)
        .route("198.51.100.7:80", second.port);
    let response = ok(get("http://203.0.113.5/a", &network));
    assert_eq!(response.status, 200);
    assert_eq!(response.url, "http://elsewhere.test/b");
    assert_eq!(network.dialed(), ["203.0.113.5:80", "198.51.100.7:80"]);
    assert_eq!(first.targets(), ["/a"]);
    assert_eq!(second.targets(), ["/b"]);
}

/// Security review of `security-review-fixes`: the other spellings of a local address a
/// Location can carry (numeric IPv4 forms the URL parser reads, IPv4-mapped IPv6 in hexadecimal,
/// the unspecified and shared addresses, the other private ranges) are refused like the plain
/// forms, and none of them is dialed.
#[test]
fn review_other_spellings_of_a_local_address_are_refused_after_a_public_start() {
    let local = Server::start(page);
    let port = local.port;
    let locations = [
        format!("http://2130706433:{port}/"),
        format!("http://0x7f.0.0.1:{port}/"),
        format!("http://0177.0.0.1:{port}/"),
        format!("http://127.1:{port}/"),
        format!("http://[::ffff:7f00:1]:{port}/"),
        format!("http://[0:0:0:0:0:ffff:127.0.0.1]:{port}/"),
        format!("http://0:{port}/"),
        format!("http://[::]:{port}/"),
        "http://100.64.0.1/".to_string(),
        "http://172.16.0.1/".to_string(),
        "http://192.168.1.1/".to_string(),
        "http://[fe80::1]/".to_string(),
        "http://[fd00::1]/".to_string(),
        "http://[::ffff:a9fe:a9fe]/".to_string(),
    ];
    let routed = locations.clone();
    let public = Server::start(move |target| {
        let n: usize = target.trim_start_matches('/').parse().unwrap_or(usize::MAX);
        match routed.get(n) {
            Some(location) => redirect(302, location.as_str()),
            None => page(target),
        }
    });
    let network = TestNetwork::new().route("203.0.113.5:80", public.port);
    for (n, location) in locations.iter().enumerate() {
        let result = get(&format!("http://203.0.113.5/{n}"), &network);
        assert!(
            matches!(result, Err(Failure::ForbiddenDestination(_))),
            "{location}: expected ForbiddenDestination, got {result:?}"
        );
    }
    assert_eq!(network.dialed(), vec!["203.0.113.5:80"; locations.len()]);
    assert!(local.targets().is_empty(), "the local server was reached");
}

/// An IPv6 address in the NAT64 prefix judged by the IPv4 address it embeds: `64:ff9b::7f00:1`
/// is 127.0.0.1, refused after a public start, and never dialed though the test routes it.
#[test]
fn case_26_a_redirect_to_a_nat64_address_of_a_loopback_one_is_refused() {
    let local = Server::start(page);
    let public = Server::start(|target| match target {
        "/a" => redirect(302, "http://[64:ff9b::7f00:1]/"),
        other => page(other),
    });
    let network = TestNetwork::new()
        .route("203.0.113.5:80", public.port)
        .route("[64:ff9b::7f00:1]:80", local.port);
    assert_forbidden(&get("http://203.0.113.5/a", &network));
    assert_eq!(network.dialed(), ["203.0.113.5:80"]);
    assert_eq!(public.targets(), ["/a"]);
    assert!(local.targets().is_empty(), "the local server was reached");
}

/// The first request of a fetch from a page at a public address (`story:agent-press-submit`,
/// "As decided after the adversary's pass", decision 1): a name that resolves to a local
/// address, any one of those resolved, and a local IP address are refused before any
/// connection; a public one is sent. The same names given ([`Origin::Given`]) are dialed.
#[test]
fn case_27_the_first_request_from_a_public_page_goes_to_public_addresses_only() {
    let local = Server::start(page);
    let public = Server::start(page);
    let port = local.port;
    let network = TestNetwork::new()
        .name("intranet.test", &[&["127.0.0.1"]])
        .name("mixed.test", &[&["198.51.100.7", "10.0.0.1"]])
        .name("public.test", &[&["198.51.100.7"]])
        .route("198.51.100.7:80", public.port);
    let from_page = |address: &str| {
        let request = Request {
            address,
            method: Method::Post,
            body: Some(b"a=1"),
            media_type: Some("application/x-www-form-urlencoded"),
            anchors: None,
            jar: None,
            initiator: None,
        };
        send_from(&request, &limits(), &network, Origin::PublicPage)
    };
    for address in [
        format!("http://intranet.test:{port}/session"),
        "http://mixed.test/session".to_string(),
        format!("http://127.0.0.1:{port}/session"),
    ] {
        let result = from_page(&address);
        assert_forbidden(&result);
        let detail = result.unwrap_err().to_string();
        assert!(
            detail.contains("comes from a page at a public address"),
            "{detail}"
        );
    }
    assert!(network.dialed().is_empty(), "{:?}", network.dialed());
    let response = ok(from_page("http://public.test/session"));
    assert_eq!(response.status, 200);
    assert_eq!(public.targets(), ["/session"]);
    let given = ok(get(&format!("http://intranet.test:{port}/x"), &network));
    assert_eq!(given.status, 200);
    assert_eq!(local.targets(), ["/x"]);
}
