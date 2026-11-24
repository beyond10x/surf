//! The wire cases W1 to W14 and the secure cases X1 to X4 of `story:agent-cookie-jar`: a fetch
//! with a jar sends the `Cookie` field the jar gives for each hop and stores every `Set-Cookie`
//! field of each response. Each case has a fresh jar, a server of its own on 127.0.0.1 port 0
//! that records the head of every request and answers by path, and the module `redirects`'s
//! `TestNetwork`, in which `a.example.test`, `b.example.test`, `example.test` and `localhost` are
//! 127.0.0.1: no real name is resolved. The secure cases add the TLS test server of the case
//! `ok-ecdsa-p256`, its root as the extra anchor, its heads read back through
//! `TestServer::requests()`.
//!
//! "receives `Cookie: X`" is a head with exactly one line whose name is `Cookie` in any case,
//! and that line `Cookie: X`; "no `Cookie`" a head without such a line. Every cookie here is a
//! constant of the test.

use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use surf_cookie::Jar;
use surf_fetch::{send_through, Anchors, Failure, Method, Request, Response};
use surf_tls::crypto::Scheme;
use surf_tls_test_server::{certs_dir, Config, Response as TlsReply, TestServer};

use crate::redirects::{limits, TestNetwork};

/// The longest the server waits for a client to send.
const BOUND: Duration = Duration::from_secs(5);

/// The most bytes the server reads of one request head.
const HEAD_MAX: usize = 64 * 1024;

/// The case of the TLS test server: its leaf names `localhost`.
const TLS_CASE: &str = "ok-ecdsa-p256";

/// What the server answers to one request: a status, a `Location` when it has one, and one
/// `Set-Cookie` field for each of `cookies`; no body.
struct Answer {
    status: u16,
    location: Option<String>,
    cookies: Vec<String>,
}

/// A 200 that sets `cookies`.
fn set(cookies: &[&str]) -> Answer {
    Answer {
        status: 200,
        location: None,
        cookies: cookies.iter().map(|c| (*c).to_string()).collect(),
    }
}

/// A redirect with `status` to `location` that sets `cookies`.
fn moved(status: u16, location: &str, cookies: &[&str]) -> Answer {
    Answer {
        location: Some(location.to_string()),
        status,
        ..set(cookies)
    }
}

type Route = Box<dyn Fn(&str, u16) -> Answer + Send>;

/// A server on 127.0.0.1 that answers one request per connection by its target and the
/// server's port, and records the head of each request.
struct Recorder {
    port: u16,
    heads: Arc<Mutex<Vec<Vec<u8>>>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Recorder {
    fn start(route: impl Fn(&str, u16) -> Answer + Send + 'static) -> Recorder {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
        let port = listener.local_addr().expect("local address").port();
        let heads = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let route: Route = Box::new(route);
        let handle = {
            let heads = Arc::clone(&heads);
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                for stream in listener.incoming() {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    serve(stream, &route, port, &heads);
                }
            })
        };
        Recorder {
            port,
            heads,
            stop,
            handle: Some(handle),
        }
    }

    /// The address of `path` on this server under `host`.
    fn at(&self, host: &str, path: &str) -> String {
        format!("http://{host}:{}{path}", self.port)
    }

    /// The address of `path` on this server under 127.0.0.1.
    fn url(&self, path: &str) -> String {
        self.at("127.0.0.1", path)
    }

    /// The requests read since the last call: the target and the `Cookie` lines of each.
    fn take(&self) -> Vec<Hop> {
        let heads = std::mem::take(&mut *self.heads.lock().expect("heads"));
        heads.iter().map(|head| hop_of(head)).collect()
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.stop.store(true, Ordering::SeqCst);
            let _ = TcpStream::connect(("127.0.0.1", self.port));
            let _ = handle.join();
        }
    }
}

fn serve(mut stream: TcpStream, route: &Route, port: u16, heads: &Mutex<Vec<Vec<u8>>>) {
    let _ = stream.set_read_timeout(Some(BOUND));
    let _ = stream.set_write_timeout(Some(BOUND));
    let mut head = Vec::new();
    let mut buf = [0u8; 8192];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
        if head.len() > HEAD_MAX {
            return;
        }
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => head.extend_from_slice(&buf[..n]),
        }
    }
    let target = hop_of(&head).target;
    heads.lock().expect("heads").push(head);
    let answer = route(&target, port);
    let mut bytes = format!("HTTP/1.1 {} Status\r\n", answer.status);
    if let Some(location) = &answer.location {
        bytes.push_str(&format!("Location: {location}\r\n"));
    }
    for cookie in &answer.cookies {
        bytes.push_str(&format!("Set-Cookie: {cookie}\r\n"));
    }
    bytes.push_str("Content-Length: 0\r\n\r\n");
    let _ = stream.write_all(bytes.as_bytes());
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Write);
    // Until the client closes, or BOUND passes.
    while let Ok(n) = stream.read(&mut buf) {
        if n == 0 {
            break;
        }
    }
}

/// One request a server read: its target and its lines whose name is `Cookie` in any case.
#[derive(Debug, PartialEq, Eq)]
struct Hop {
    target: String,
    cookies: Vec<String>,
}

fn hop_of(head: &[u8]) -> Hop {
    let text = String::from_utf8_lossy(head);
    let target = text.split(' ').nth(1).unwrap_or_default().to_string();
    let cookies = text
        .split("\r\n")
        .filter(|line| {
            line.split_once(':')
                .is_some_and(|(name, _)| name.eq_ignore_ascii_case("cookie"))
        })
        .map(str::to_string)
        .collect();
    Hop { target, cookies }
}

/// `target` received `Cookie: <cookie>`, or no `Cookie` for `None`.
fn hop(target: &str, cookie: Option<&str>) -> Hop {
    Hop {
        target: target.to_string(),
        cookies: cookie.map(|c| format!("Cookie: {c}")).into_iter().collect(),
    }
}

/// The network of the cases: four names, each 127.0.0.1.
fn network() -> TestNetwork {
    TestNetwork::new()
        .name("a.example.test", &[&["127.0.0.1"]])
        .name("b.example.test", &[&["127.0.0.1"]])
        .name("example.test", &[&["127.0.0.1"]])
        .name("localhost", &[&["127.0.0.1"]])
}

/// `method` of `address` with `jar` and `initiator`, no body, verified against `anchors`.
fn send(
    jar: &Jar,
    method: Method,
    address: &str,
    initiator: Option<&str>,
    anchors: Option<&Anchors>,
) -> Result<Response, Failure> {
    let request = Request {
        address,
        method,
        body: None,
        media_type: None,
        anchors,
        jar: Some(jar),
        initiator,
    };
    send_through(&request, &limits(), &network())
}

/// A `GET` of `address` with `jar` and no initiator, which must end in a response.
fn get(jar: &Jar, address: &str) -> Response {
    match send(jar, Method::Get, address, None, None) {
        Ok(response) => response,
        Err(failure) => panic!("expected a response from {address}, got {failure:?}"),
    }
}

#[test]
fn w01_a_cookie_of_a_redirect_is_sent_on_its_next_hop() {
    let server = Recorder::start(|target, _| match target {
        "/login" => moved(302, "/home", &["s=1; Path=/"]),
        _ => set(&[]),
    });
    let jar = Jar::new();
    get(&jar, &server.url("/login"));
    assert_eq!(
        server.take(),
        [hop("/login", None), hop("/home", Some("s=1"))]
    );
}

#[test]
fn w02_each_hop_sends_the_cookies_of_its_own_address() {
    let server = Recorder::start(|target, port| match target {
        "/set-a" => set(&["a=1; Path=/"]),
        "/set-b" => set(&["b=2; Path=/"]),
        "/hop" => moved(302, &format!("http://a.example.test:{port}/end"), &[]),
        _ => set(&[]),
    });
    let jar = Jar::new();
    get(&jar, &server.url("/set-a"));
    get(&jar, &server.at("a.example.test", "/set-b"));
    server.take();
    get(&jar, &server.url("/hop"));
    assert_eq!(
        server.take(),
        [hop("/hop", Some("a=1")), hop("/end", Some("b=2"))]
    );
}

#[test]
fn w03_a_cookie_is_not_sent_to_another_host() {
    let server = Recorder::start(|target, _| match target {
        "/set-a" => set(&["a=1; Path=/"]),
        _ => set(&[]),
    });
    let jar = Jar::new();
    get(&jar, &server.url("/set-a"));
    server.take();
    get(&jar, &server.at("b.example.test", "/echo"));
    assert_eq!(server.take(), [hop("/echo", None)]);
}

#[test]
fn w04_a_domain_attribute_keeps_the_cookie_to_its_host() {
    let server = Recorder::start(|target, _| match target {
        "/set" => set(&[
            "p=1; Domain=example.test; Path=/",
            "q=1; Domain=.EXAMPLE.test; Path=/",
            "r=1; Domain=other.test; Path=/",
        ]),
        _ => set(&[]),
    });
    let jar = Jar::new();
    get(&jar, &server.at("a.example.test", "/set"));
    server.take();
    get(&jar, &server.at("a.example.test", "/echo"));
    get(&jar, &server.at("b.example.test", "/echo"));
    get(&jar, &server.at("example.test", "/echo"));
    assert_eq!(
        server.take(),
        [
            hop("/echo", Some("p=1; q=1")),
            hop("/echo", None),
            hop("/echo", None)
        ]
    );
}

#[test]
fn w05_a_domain_of_an_ip_address_matches_only_itself() {
    let server = Recorder::start(|target, _| match target {
        "/set" => set(&[
            "ip=1; Domain=127.0.0.1; Path=/",
            "ip2=1; Domain=0.0.1; Path=/",
        ]),
        _ => set(&[]),
    });
    let jar = Jar::new();
    get(&jar, &server.url("/set"));
    server.take();
    get(&jar, &server.url("/echo"));
    assert_eq!(server.take(), [hop("/echo", Some("ip=1"))]);
}

#[test]
fn w06_a_cookie_is_sent_where_its_path_matches() {
    let server = Recorder::start(|target, _| match target {
        "/p/set" => set(&["pa=1; Path=/a", "dp=1"]),
        _ => set(&[]),
    });
    let jar = Jar::new();
    get(&jar, &server.url("/p/set"));
    server.take();
    for path in ["/a", "/a/x", "/ab", "/b", "/p/q", "/pq"] {
        get(&jar, &server.url(path));
    }
    assert_eq!(
        server.take(),
        [
            hop("/a", Some("pa=1")),
            hop("/a/x", Some("pa=1")),
            hop("/ab", None),
            hop("/b", None),
            hop("/p/q", Some("dp=1")),
            hop("/pq", None),
        ]
    );
}

#[test]
fn w07_longer_paths_go_first_then_earlier_cookies() {
    let server = Recorder::start(|target, _| match target {
        "/set" => set(&["a=1; Path=/", "b=2; Path=/x", "c=3; Path=/"]),
        _ => set(&[]),
    });
    let jar = Jar::new();
    get(&jar, &server.url("/set"));
    server.take();
    get(&jar, &server.url("/x/y"));
    assert_eq!(server.take(), [hop("/x/y", Some("b=2; a=1; c=3"))]);
}

#[test]
fn w08_an_expired_cookie_deletes_the_stored_one() {
    let server = Recorder::start(|target, _| match target {
        "/set" => set(&["e=1; Path=/", "f=1; Path=/"]),
        "/del" => set(&[
            "e=1; Max-Age=0; Path=/",
            "f=1; Expires=Thu, 01 Jan 1970 00:00:01 GMT; Path=/",
        ]),
        _ => set(&[]),
    });
    let jar = Jar::new();
    get(&jar, &server.url("/set"));
    get(&jar, &server.url("/del"));
    server.take();
    get(&jar, &server.url("/echo"));
    assert_eq!(server.take(), [hop("/echo", None)]);
}

#[test]
fn w09_an_http_only_cookie_is_sent() {
    let server = Recorder::start(|target, _| match target {
        "/set" => set(&["h=1; HttpOnly; Path=/"]),
        _ => set(&[]),
    });
    let jar = Jar::new();
    get(&jar, &server.url("/set"));
    server.take();
    get(&jar, &server.url("/echo"));
    assert_eq!(server.take(), [hop("/echo", Some("h=1"))]);
}

#[test]
fn w10_a_cookie_over_4096_bytes_is_ignored() {
    let big = format!("big={}", "x".repeat(4093));
    let huge = format!("huge={}", "y".repeat(4093));
    let answer = [big.clone(), huge];
    let server = Recorder::start(move |target, _| match target {
        "/set" => set(&[answer[0].as_str(), answer[1].as_str()]),
        _ => set(&[]),
    });
    let jar = Jar::new();
    get(&jar, &server.url("/set"));
    server.take();
    get(&jar, &server.url("/echo"));
    assert_eq!(server.take(), [hop("/echo", Some(&big))]);
}

#[test]
fn w11_the_cookie_field_stays_within_8192_bytes() {
    let [a, b, c] = ["a", "b", "c"].map(|name| {
        let fill = match name {
            "a" => "x",
            "b" => "y",
            _ => "z",
        };
        format!("{name}={}", fill.repeat(4000))
    });
    let answer = [a.clone(), b.clone(), c].map(|cookie| format!("{cookie}; Path=/"));
    let server = Recorder::start(move |target, _| match target {
        "/set" => set(&[&answer[0], &answer[1], &answer[2]]),
        _ => set(&[]),
    });
    let jar = Jar::new();
    get(&jar, &server.url("/set"));
    server.take();
    get(&jar, &server.url("/echo"));
    assert_eq!(server.take(), [hop("/echo", Some(&format!("{a}; {b}")))]);
}

#[test]
fn w12_the_51st_cookie_of_a_domain_evicts_the_least_recently_used() {
    let fifty: Vec<String> = (1..=50).map(|n| format!("c{n}={n}; Path=/")).collect();
    let server = Recorder::start(move |target, _| match target {
        "/one" => set(&fifty.iter().map(String::as_str).collect::<Vec<_>>()),
        "/two" => set(&["c51=51; Path=/"]),
        _ => set(&[]),
    });
    let sent = |range: std::ops::RangeInclusive<u32>| {
        range
            .map(|n| format!("c{n}={n}"))
            .collect::<Vec<_>>()
            .join("; ")
    };
    let jar = Jar::new();
    get(&jar, &server.url("/one"));
    server.take();
    get(&jar, &server.url("/two"));
    get(&jar, &server.url("/three"));
    assert_eq!(
        server.take(),
        [
            hop("/two", Some(&sent(1..=50))),
            hop("/three", Some(&sent(2..=51)))
        ]
    );
}

#[test]
fn w13_a_cross_site_request_sends_by_same_site_and_method() {
    let server = Recorder::start(|target, _| match target {
        "/set" => set(&[
            "lax=1; SameSite=Lax; Path=/",
            "strict=1; SameSite=Strict; Path=/",
            "dflt=1; Path=/",
            "none=1; SameSite=None; Path=/",
        ]),
        "/post303" => moved(303, "/echo", &[]),
        _ => set(&[]),
    });
    let jar = Jar::new();
    get(&jar, &server.url("/set"));
    server.take();
    let cross = format!("http://localhost:{}", server.port);
    let echo = server.url("/echo");
    let post303 = server.url("/post303");
    for (method, address, initiator) in [
        (Method::Get, &echo, cross.as_str()),
        (Method::Post, &echo, cross.as_str()),
        (Method::Post, &post303, cross.as_str()),
        (Method::Get, &echo, "http://127.0.0.1:1"),
    ] {
        if let Err(failure) = send(&jar, method, address, Some(initiator), None) {
            panic!("expected a response from {address}, got {failure:?}");
        }
    }
    assert_eq!(
        server.take(),
        [
            hop("/echo", Some("lax=1; dflt=1")),
            hop("/echo", None),
            hop("/post303", None),
            hop("/echo", Some("lax=1; dflt=1")),
            hop("/echo", Some("lax=1; strict=1; dflt=1")),
        ]
    );
}

#[test]
fn w14_a_set_cookie_over_the_header_limit_ends_the_fetch_and_is_not_stored() {
    let long = format!("l={}; Path=/", "x".repeat(70_000 - 10));
    assert_eq!(long.len(), 70_000);
    let server = Recorder::start(move |target, _| match target {
        "/set" => set(&[long.as_str()]),
        _ => set(&[]),
    });
    let jar = Jar::new();
    let result = send(&jar, Method::Get, &server.url("/set"), None, None);
    assert!(
        matches!(result, Err(Failure::MalformedResponse(_))),
        "expected MalformedResponse"
    );
    server.take();
    get(&jar, &server.url("/echo"));
    assert_eq!(server.take(), [hop("/echo", None)]);
}

/// The bundled trust anchors and the root of [`TLS_CASE`].
fn anchors() -> Anchors {
    let mut anchors = Anchors::bundled();
    let root = fs::read(certs_dir().join(TLS_CASE).join("root.pem")).expect("the root.pem");
    anchors
        .with_extra_pem(&root)
        .expect("the root of the case is a CA");
    anchors
}

/// The secure cases' servers: the plain server as `localhost` port `<p>` and the TLS test
/// server as `localhost` port `<t>`, and X1 done: `https://localhost:<t>/set` stored `sec=1`
/// and `/echo` received it.
struct Secure {
    jar: Jar,
    anchors: Anchors,
    plain: Recorder,
    tls: TestServer,
}

impl Secure {
    fn start() -> Secure {
        let plain = Recorder::start(|target, _| match target {
            "/set" => set(&["s2=1; Secure; Path=/", "sec=2; Path=/"]),
            "/set-deep" => set(&["sec=3; Path=/echo"]),
            _ => set(&[]),
        });
        let mut config = Config::new(TLS_CASE, Scheme::EcdsaSecp256r1Sha256);
        let mut sets = TlsReply::ok(b"");
        sets.fields.push((
            "Set-Cookie".to_string(),
            "sec=1; Secure; Path=/".to_string(),
        ));
        let down = TlsReply {
            status: 302,
            reason: "Found".to_string(),
            fields: vec![(
                "Location".to_string(),
                plain.at("localhost", "/echo").to_string(),
            )],
            body: Vec::new(),
        };
        config.responses.extend([
            ("/set".to_string(), sets),
            ("/echo".to_string(), TlsReply::ok(b"")),
            ("/down".to_string(), down),
        ]);
        let tls = TestServer::start(config).expect("the TLS test server starts");
        let secure = Secure {
            jar: Jar::new(),
            anchors: anchors(),
            plain,
            tls,
        };
        secure.get(&secure.https("/set"));
        secure.get(&secure.https("/echo"));
        assert_eq!(
            secure.tls_hops(),
            [hop("/set", None), hop("/echo", Some("sec=1"))],
            "X1"
        );
        secure
    }

    fn https(&self, path: &str) -> String {
        format!("https://localhost:{}{path}", self.tls.port())
    }

    fn get(&self, address: &str) {
        if let Err(failure) = send(&self.jar, Method::Get, address, None, Some(&self.anchors)) {
            panic!("expected a response from {address}, got {failure:?}");
        }
    }

    /// Every request the TLS test server read, in order.
    fn tls_hops(&self) -> Vec<Hop> {
        self.tls
            .requests()
            .iter()
            .map(|head| hop_of(head))
            .collect()
    }
}

#[test]
fn x1_a_secure_cookie_from_https_is_sent_to_https() {
    Secure::start();
}

#[test]
fn x2_a_secure_cookie_is_not_sent_to_http() {
    let secure = Secure::start();
    secure.get(&secure.plain.at("localhost", "/echo"));
    assert_eq!(secure.plain.take(), [hop("/echo", None)]);
}

#[test]
fn x3_a_redirect_from_https_to_http_drops_the_secure_cookie() {
    let secure = Secure::start();
    secure.get(&secure.https("/down"));
    assert_eq!(secure.tls_hops()[2..], [hop("/down", Some("sec=1"))]);
    assert_eq!(secure.plain.take(), [hop("/echo", None)]);
}

#[test]
fn x4_http_cannot_set_a_secure_cookie_nor_shadow_one() {
    let secure = Secure::start();
    // `sec=2` on the Secure cookie's own path `/`, then `sec=3` on `/echo`, below it: both from
    // `http:`, both ignored.
    secure.get(&secure.plain.at("localhost", "/set"));
    secure.get(&secure.plain.at("localhost", "/set-deep"));
    secure.get(&secure.https("/echo"));
    assert_eq!(secure.tls_hops()[2..], [hop("/echo", Some("sec=1"))]);
}
