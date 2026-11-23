//! Conformance checks of the `https:` fetch against its own contract (the crate documentation,
//! `story:tls-fetch-https`, `docs/design/tls.md` item 7): each hop is verified for its own
//! host; an `https:` hop refuses every `Location` the `http:` path refuses, with the same
//! reason and no connection; what a `POST` carries across a redirect from `https:` to `http:`;
//! a TLS stream cut inside a record, or altered after the response head, is never a page; and
//! the read timeout is a deadline for the handshake, not a bound per read.
//!
//! Every server is on 127.0.0.1 port 0: the TLS test server, the `http:` server of the module
//! `redirects`, a one-request `http:` server of this module, and a relay of this module that
//! changes what the TLS server sends on its way to the client. No real name is resolved.

use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use surf_fetch::{send_through, Anchors, Failure, Limits, Method, Request, Response};
use surf_tls::crypto::Scheme;
use surf_tls_test_server::{certs_dir, Config, Mode, Response as TlsReply, TestServer};

use crate::redirects::{limits, ok, redirect, Server, TestNetwork};

/// The longest a helper of this module waits for a peer.
const BOUND: Duration = Duration::from_secs(5);

/// The bundled anchors and the root of each case in `cases`.
fn anchors(cases: &[&str]) -> Anchors {
    let mut anchors = Anchors::bundled();
    for case in cases {
        let root = fs::read(certs_dir().join(case).join("root.pem")).expect("root.pem of the case");
        assert_eq!(anchors.with_extra_pem(&root).expect("a CA root"), 1);
    }
    anchors
}

/// A network in which `localhost` is 127.0.0.1.
fn network() -> TestNetwork {
    TestNetwork::new().name("localhost", &[&["127.0.0.1"]])
}

fn get(address: &str, anchors: &Anchors, limits: &Limits, network: &TestNetwork) -> Result<Response, Failure> {
    let request = Request {
        address,
        method: Method::Get,
        body: None,
        media_type: None,
        anchors: Some(anchors),
    };
    send_through(&request, limits, network)
}

/// The TLS test server of `case`, signing with `scheme`, as `mode`, with `paths` beside `/`.
fn tls(case: &str, scheme: Scheme, mode: Mode, paths: Vec<(String, TlsReply)>) -> TestServer {
    let mut config = Config::new(case, scheme);
    config.mode = mode;
    config.responses.extend(paths);
    TestServer::start(config).expect("the TLS test server starts")
}

fn tls_redirect(status: u16, location: &str) -> TlsReply {
    TlsReply {
        status,
        reason: "Redirect".to_string(),
        fields: vec![("Location".to_string(), location.to_string())],
        body: Vec::new(),
    }
}

/// Whether `listener` was connected to, asked once without waiting.
fn was_connected(listener: &TcpListener) -> bool {
    listener.set_nonblocking(true).expect("nonblocking");
    match listener.accept() {
        Err(e) if e.kind() == ErrorKind::WouldBlock => false,
        Err(e) => panic!("accept: {e}"),
        Ok(_) => true,
    }
}

/// The name of a result's kind, for comparing the two paths.
fn kind(result: &Result<Response, Failure>) -> String {
    match result {
        Ok(response) => format!("Ok({})", response.status),
        Err(failure) => {
            let text = format!("{failure:?}");
            text.split('(').next().unwrap_or("").to_string()
        }
    }
}

/// Invariant: the certificate of a hop is checked for the host of that hop. Both servers are of
/// the case `ok-wildcard`, whose certificate names `*.example.test` only; the network of the
/// test gives `a.example.test` and `localhost` the address 127.0.0.1. The first hop,
/// `https://a.example.test`, redirects to `https://localhost`. A fetch that checked the second
/// certificate against the first host would load the page.
#[test]
fn secreview_tls_a_redirect_is_verified_for_the_host_of_its_own_hop() {
    let anchors = anchors(&["ok-wildcard"]);
    let network = || network().name("a.example.test", &[&["127.0.0.1"]]);
    let second = tls("ok-wildcard", Scheme::EcdsaSecp256r1Sha256, Mode::Normal, Vec::new());
    let second_other = format!("https://localhost:{}/", second.port());
    let second_named = format!("https://a.example.test:{}/", second.port());
    // The second certificate loads for its own name and is refused for `localhost` when asked
    // directly: the premise of the case.
    ok(get(&second_named, &anchors, &limits(), &network()));
    let direct = get(&second_other, &anchors, &limits(), &network());
    assert!(matches!(direct, Err(Failure::CertificateNameMismatch(_))), "{direct:?}");

    let first = tls(
        "ok-wildcard",
        Scheme::EcdsaSecp256r1Sha256,
        Mode::Normal,
        vec![("/a".to_string(), tls_redirect(302, &second_other))],
    );
    let address = format!("https://a.example.test:{}/a", first.port());
    let result = get(&address, &anchors, &limits(), &network());
    assert!(
        matches!(result, Err(Failure::CertificateNameMismatch(_))),
        "the second hop was not checked for its own host localhost: {result:?}"
    );
    // Three connections: its name, `localhost` asked directly, and the redirect's hop. The two
    // for `localhost` sent that name as server_name, and neither got a request.
    let seen = second.wait_seen(3, BOUND);
    assert_eq!(seen.len(), 3, "{seen:?}");
    let other: Vec<_> = seen
        .iter()
        .filter(|s| s.server_name.as_deref() == Some("localhost"))
        .collect();
    assert_eq!(other.len(), 2, "the hop to localhost sent another server_name: {seen:?}");
    assert!(
        other.iter().all(|s| s.request.is_none()),
        "a request was sent on a refused hop: {seen:?}"
    );
}

/// Invariant: an `https:` hop refuses every `Location` the `http:` path refuses, with the same
/// reason, and connects to none of them.
#[test]
fn secreview_tls_an_https_hop_refuses_what_the_http_path_refuses() {
    let held = TcpListener::bind("127.0.0.1:0").expect("bind");
    let p = held.local_addr().expect("address").port();
    let locations = [
        format!("https://user:secret@127.0.0.1:{p}/"),
        format!("https://user@127.0.0.1:{p}/"),
        format!("http://user:secret@127.0.0.1:{p}/"),
        "https://127.0.0.1:0/".to_string(),
        format!("https://127.0.0.1:{p}x/"),
        "https://[::1/".to_string(),
        format!("ftp://127.0.0.1:{p}/"),
        "file:///etc/passwd".to_string(),
        "data:text/html,x".to_string(),
        "javascript:alert(1)".to_string(),
        format!("wss://127.0.0.1:{p}/"),
    ];
    let paths: Vec<(String, TlsReply)> = locations
        .iter()
        .enumerate()
        .map(|(n, location)| (format!("/{n}"), tls_redirect(302, location)))
        .collect();
    let tls_server = tls("ok-ecdsa-p256", Scheme::EcdsaSecp256r1Sha256, Mode::Normal, paths);
    let routed = locations.clone();
    let http = Server::start(move |target| {
        let n: usize = target.trim_start_matches('/').parse().unwrap_or(usize::MAX);
        redirect(302, routed.get(n).cloned().unwrap_or_default())
    });
    let anchors = anchors(&["ok-ecdsa-p256"]);
    let mut differ = Vec::new();
    for (n, location) in locations.iter().enumerate() {
        let over_tls = get(
            &format!("https://localhost:{}/{n}", tls_server.port()),
            &anchors,
            &limits(),
            &network(),
        );
        let over_http = get(&http.url(&format!("/{n}")), &anchors, &limits(), &network());
        assert!(over_tls.is_err(), "{location}: {over_tls:?}");
        if kind(&over_tls) != kind(&over_http) {
            differ.push(format!("{location}: https {over_tls:?}, http {over_http:?}"));
        }
    }
    assert!(differ.is_empty(), "the two paths differ: {differ:#?}");
    assert!(!was_connected(&held), "a refused Location was connected to");
}

/// Invariant: after a first `https:` request to a public address, a `Location` to any local
/// address is `ForbiddenDestination` before a connection, as on the `http:` path. The public
/// host `a.example.test` is 203.0.113.5, routed to the TLS test server of `ok-wildcard`.
#[test]
fn secreview_tls_a_public_https_start_is_refused_at_every_local_destination() {
    let held = TcpListener::bind("127.0.0.1:0").expect("bind");
    let p = held.local_addr().expect("address").port();
    let locations = [
        format!("https://127.0.0.1:{p}/"),
        format!("http://127.0.0.1:{p}/"),
        format!("https://localhost:{p}/"),
        "https://169.254.169.254/latest".to_string(),
        "http://169.254.169.254/latest".to_string(),
        format!("https://[::ffff:127.0.0.1]:{p}/"),
        "https://[64:ff9b::a00:1]/".to_string(),
        "https://[2002:a00:1::]/".to_string(),
        "https://[fe80::1]/".to_string(),
        "https://[fc00::1]/".to_string(),
        "https://100.64.0.1/".to_string(),
        "https://0.0.0.0/".to_string(),
        "https://192.168.1.1/".to_string(),
        "https://2130706433/".to_string(),
        "https://0x7f.1/".to_string(),
    ];
    let paths: Vec<(String, TlsReply)> = locations
        .iter()
        .enumerate()
        .map(|(n, location)| (format!("/{n}"), tls_redirect(307, location)))
        .collect();
    let tls_server = tls("ok-wildcard", Scheme::EcdsaSecp256r1Sha256, Mode::Normal, paths);
    let network = TestNetwork::new()
        .name("a.example.test", &[&["203.0.113.5"]])
        .name("localhost", &[&["127.0.0.1"]])
        .route("203.0.113.5:443", tls_server.port());
    let anchors = anchors(&["ok-wildcard"]);
    let mut wrong = Vec::new();
    for (n, location) in locations.iter().enumerate() {
        let result = get(&format!("https://a.example.test/{n}"), &anchors, &limits(), &network);
        if !matches!(result, Err(Failure::ForbiddenDestination(_))) {
            wrong.push(format!("{location}: {result:?}"));
        }
    }
    assert!(wrong.is_empty(), "not ForbiddenDestination: {wrong:#?}");
    let dialed = network.dialed();
    assert_eq!(dialed.len(), locations.len(), "{dialed:?}");
    assert!(dialed.iter().all(|d| d == "203.0.113.5:443"), "{dialed:?}");
    assert!(!was_connected(&held), "a local destination was connected to");
}

/// One request an `http:` server of this module read: the head and the body it declared.
#[derive(Debug)]
struct Read1 {
    head: String,
    body: Vec<u8>,
}

impl Read1 {
    fn line(&self) -> &str {
        self.head.split("\r\n").next().unwrap_or("")
    }

    fn field_names(&self) -> Vec<String> {
        self.head
            .split("\r\n")
            .skip(1)
            .filter(|line| !line.is_empty())
            .filter_map(|line| line.split_once(':'))
            .map(|(name, _)| name.to_ascii_lowercase())
            .collect()
    }

    fn field(&self, name: &str) -> Option<String> {
        self.head.split("\r\n").skip(1).find_map(|line| {
            let (n, v) = line.split_once(':')?;
            n.eq_ignore_ascii_case(name).then(|| v.trim().to_string())
        })
    }
}

/// An `http:` server for one request: it reads the head and `Content-Length` bytes, answers
/// `200` with `ok`, and hands back what it read.
fn one_request() -> (u16, mpsc::Receiver<Read1>, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("address").port();
    listener.set_nonblocking(true).expect("nonblocking");
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let until = Instant::now() + BOUND;
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == ErrorKind::WouldBlock && Instant::now() < until => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => return,
            }
        };
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(BOUND));
        let mut bytes = Vec::new();
        let mut buf = [0u8; 4096];
        let head_end = loop {
            if let Some(at) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                break at + 4;
            }
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(n) => bytes.extend_from_slice(&buf[..n]),
            }
        };
        let head = String::from_utf8_lossy(&bytes[..head_end]).into_owned();
        let mut read = Read1 {
            head,
            body: Vec::new(),
        };
        let declared: usize = read
            .field("content-length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        read.body = bytes[head_end..].to_vec();
        while read.body.len() < declared {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => read.body.extend_from_slice(&buf[..n]),
            }
        }
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok");
        let _ = stream.shutdown(Shutdown::Write);
        let _ = sender.send(read);
    });
    (port, receiver, handle)
}

/// Invariant (the crate documentation, "A redirect changes the method"): across a redirect from
/// `https:` to `http:`, a 307 repeats the method, the body and its media type, a 302 or 303
/// continues as a `GET` without them, and no field beyond `Host`, `Connection`, `Content-Type`
/// and `Content-Length` is sent; `Host` is the authority of the new hop.
#[test]
fn secreview_tls_a_post_redirected_from_https_to_http_carries_what_the_rule_says() {
    let anchors = anchors(&["ok-ecdsa-p256"]);
    let body = b"card=4111111111111111";
    let form = "application/x-www-form-urlencoded";
    for status in [307u16, 302, 303] {
        let (port, receiver, handle) = one_request();
        let to = format!("http://127.0.0.1:{port}/b");
        let tls_server = tls(
            "ok-ecdsa-p256",
            Scheme::EcdsaSecp256r1Sha256,
            Mode::Normal,
            vec![("/a".to_string(), tls_redirect(status, &to))],
        );
        let address = format!("https://localhost:{}/a", tls_server.port());
        let request = Request {
            address: &address,
            method: Method::Post,
            body: Some(body),
            media_type: Some(form),
            anchors: Some(&anchors),
        };
        let response = ok(send_through(&request, &limits(), &network()));
        assert_eq!(response.url, to);
        let read = receiver.recv_timeout(BOUND).expect("the http: hop was reached");
        handle.join().expect("the http: server");
        assert_eq!(read.field("host").as_deref(), Some(format!("127.0.0.1:{port}").as_str()));
        let names = read.field_names();
        assert!(
            names
                .iter()
                .all(|n| ["host", "connection", "content-type", "content-length"].contains(&n.as_str())),
            "{status}: {names:?}"
        );
        if status == 307 {
            assert_eq!(read.line(), "POST /b HTTP/1.1", "{status}");
            assert_eq!(read.body, body, "{status}");
            assert_eq!(read.field("content-type").as_deref(), Some(form), "{status}");
        } else {
            assert_eq!(read.line(), "GET /b HTTP/1.1", "{status}");
            assert!(read.body.is_empty(), "{status}: {read:?}");
            assert_eq!(read.field("content-type"), None, "{status}");
            assert_eq!(read.field("content-length"), None, "{status}");
        }
    }
}

/// What a relay does to the bytes the TLS server sends to the client.
#[derive(Clone, Copy, Debug)]
enum Down {
    /// Forward the first `n` bytes, then close both connections.
    CutAt(usize),
    /// Forward every byte, the one at offset `n` with its lowest bit flipped.
    FlipAt(usize),
    /// Forward one byte at a time, each after `delay`.
    Trickle(Duration),
}

/// A relay on 127.0.0.1 port 0 to the TLS server at `port`: the client's bytes go on unchanged,
/// the server's as `down` says. It serves one connection.
struct Relay {
    port: u16,
    /// Bytes read from the server, and bytes written to the client.
    from_server: Arc<AtomicUsize>,
    to_client: Arc<AtomicUsize>,
    handle: Option<JoinHandle<()>>,
}

impl Relay {
    fn start(port: u16, down: Down) -> Relay {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let own = listener.local_addr().expect("address").port();
        listener.set_nonblocking(true).expect("nonblocking");
        let from_server = Arc::new(AtomicUsize::new(0));
        let to_client = Arc::new(AtomicUsize::new(0));
        let (from_c, to_c) = (Arc::clone(&from_server), Arc::clone(&to_client));
        let handle = thread::spawn(move || {
            let until = Instant::now() + BOUND;
            let client = loop {
                match listener.accept() {
                    Ok((client, _)) => break client,
                    Err(e) if e.kind() == ErrorKind::WouldBlock && Instant::now() < until => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => return,
                }
            };
            if client.set_nonblocking(false).is_err() {
                return;
            }
            let Ok(server) = TcpStream::connect(("127.0.0.1", port)) else {
                return;
            };
            for stream in [&client, &server] {
                let _ = stream.set_read_timeout(Some(BOUND));
                let _ = stream.set_nodelay(true);
            }
            let (mut up_from, mut up_to) = (
                client.try_clone().expect("clone"),
                server.try_clone().expect("clone"),
            );
            let up = thread::spawn(move || {
                let mut buf = [0u8; 16 * 1024];
                while let Ok(n) = up_from.read(&mut buf) {
                    if n == 0 || up_to.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
                let _ = up_to.shutdown(Shutdown::Write);
            });
            relay_down(server, client, down, &from_c, &to_c);
            let _ = up.join();
        });
        Relay {
            port: own,
            from_server,
            to_client,
            handle: Some(handle),
        }
    }

    fn join(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        self.join();
    }
}

fn relay_down(
    mut server: TcpStream,
    mut client: TcpStream,
    down: Down,
    from_server: &AtomicUsize,
    to_client: &AtomicUsize,
) {
    let mut buf = [0u8; 16 * 1024];
    let mut offset = 0usize;
    let started = Instant::now();
    'reading: while let Ok(n) = server.read(&mut buf) {
        if n == 0 {
            break;
        }
        from_server.fetch_add(n, Ordering::SeqCst);
        let chunk = &mut buf[..n];
        match down {
            Down::CutAt(at) => {
                let keep = at.saturating_sub(offset).min(n);
                if client.write_all(&chunk[..keep]).is_err() {
                    break;
                }
                to_client.fetch_add(keep, Ordering::SeqCst);
                offset += n;
                if offset >= at {
                    break;
                }
            }
            Down::FlipAt(at) => {
                if (offset..offset + n).contains(&at) {
                    chunk[at - offset] ^= 0x01;
                }
                if client.write_all(chunk).is_err() {
                    break;
                }
                to_client.fetch_add(n, Ordering::SeqCst);
                offset += n;
            }
            Down::Trickle(delay) => {
                for byte in chunk.iter() {
                    if started.elapsed() > BOUND {
                        break 'reading;
                    }
                    thread::sleep(delay);
                    if client.write_all(&[*byte]).is_err() {
                        break 'reading;
                    }
                    to_client.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
    }
    let _ = client.shutdown(Shutdown::Both);
    let _ = server.shutdown(Shutdown::Both);
}

/// The offset of the server's bytes where a relay cuts or flips: past the handshake (about 2 KB
/// for `ok-ecdsa-p256`), inside the second record of a 64 KiB body (records of 16,406 bytes),
/// so neither at a record's start nor in its header.
const INSIDE_THE_BODY: usize = 30_000;

/// A 64 KiB body, framed as `mode` frames it.
fn big(mode: Mode) -> TestServer {
    tls(
        "ok-ecdsa-p256",
        Scheme::EcdsaSecp256r1Sha256,
        mode,
        vec![("/".to_string(), TlsReply::ok(&vec![b'x'; 64 * 1024]))],
    )
}

/// Invariant: a TLS stream that ends inside a record, after the response head, is no page:
/// with a body delimited by the end of the connection and with a declared one.
#[test]
fn secreview_tls_a_stream_cut_inside_a_record_is_not_a_page() {
    let anchors = anchors(&["ok-ecdsa-p256"]);
    for mode in [Mode::NoCloseNotify, Mode::Normal] {
        let server = big(mode);
        let mut relay = Relay::start(server.port(), Down::CutAt(INSIDE_THE_BODY));
        let result = get(
            &format!("https://localhost:{}/", relay.port),
            &anchors,
            &limits(),
            &network(),
        );
        relay.join();
        let (from, to) = (
            relay.from_server.load(Ordering::SeqCst),
            relay.to_client.load(Ordering::SeqCst),
        );
        assert_eq!(to, INSIDE_THE_BODY, "{mode:?}: the relay did not cut where it should");
        assert!(from >= INSIDE_THE_BODY, "{mode:?}: the server sent only {from} bytes");
        assert!(
            matches!(result, Err(Failure::Unreachable(_))),
            "{mode:?}: a stream cut inside a record gave {}",
            match &result {
                Ok(r) => format!("a page of {} bytes, status {}", r.body.len(), r.status),
                Err(f) => format!("{f:?}"),
            }
        );
    }
}

/// Invariant: a record altered after the response head (one bit of its ciphertext) ends the
/// fetch with `TlsHandshakeFailed` (`bad_record_mac`, `docs/design/tls.md` item 7), never a
/// page.
#[test]
fn secreview_tls_a_record_altered_after_the_head_is_not_a_page() {
    let anchors = anchors(&["ok-ecdsa-p256"]);
    let server = big(Mode::Normal);
    let mut relay = Relay::start(server.port(), Down::FlipAt(INSIDE_THE_BODY));
    let result = get(
        &format!("https://localhost:{}/", relay.port),
        &anchors,
        &limits(),
        &network(),
    );
    relay.join();
    assert!(relay.to_client.load(Ordering::SeqCst) > INSIDE_THE_BODY);
    match result {
        Err(Failure::TlsHandshakeFailed(detail)) => {
            assert!(detail.contains("bad_record_mac"), "{detail}")
        }
        other => panic!("an altered record gave {:?}", other.map(|r| (r.status, r.body.len()))),
    }
}

/// Invariant (the crate documentation, "Limits", TLS): a chain at the path search's limit (the
/// case `candidate-limit`, 48 intermediates of one subject) ends `CertificateNotTrusted` naming
/// `CandidateLimit`, inside the read timeout. What the fetch's thread held at its peak is
/// printed: the documentation ("Memory") states no bound for a server's chain.
#[test]
fn secreview_tls_a_chain_at_the_search_limit_is_refused_inside_the_read_timeout() {
    let anchors = anchors(&["candidate-limit"]);
    let server = tls("candidate-limit", Scheme::EcdsaSecp256r1Sha256, Mode::Normal, Vec::new());
    let address = format!("https://localhost:{}/", server.port());
    let network = network();
    let started = Instant::now();
    let (result, held) = crate::adversary_redirect_memory::held_on_this_thread(|| {
        get(&address, &anchors, &limits(), &network)
    });
    let took = started.elapsed();
    eprintln!("a chain at the search limit: {took:?}, {held} bytes held at the peak");
    match result {
        Err(Failure::CertificateNotTrusted(detail)) => {
            assert!(detail.contains("CandidateLimit"), "{detail}")
        }
        other => panic!("{other:?}"),
    }
    assert!(took < limits().read_timeout, "took {took:?}");
}

/// Invariant (the crate documentation, "Read timeout"): the handshake shares one deadline with
/// the exchange. A server whose bytes arrive one every 10 ms never leaves a read waiting long,
/// and still ends the fetch `Unreachable` at the read timeout of 1 s.
#[test]
fn secreview_tls_a_trickling_server_is_held_to_the_read_deadline() {
    let anchors = anchors(&["ok-ecdsa-p256"]);
    let server = big(Mode::Normal);
    let mut relay = Relay::start(server.port(), Down::Trickle(Duration::from_millis(10)));
    let limits = Limits {
        read_timeout: Duration::from_secs(1),
        ..limits()
    };
    let started = Instant::now();
    let result = get(
        &format!("https://localhost:{}/", relay.port),
        &anchors,
        &limits,
        &network(),
    );
    let took = started.elapsed();
    relay.join();
    assert!(
        relay.to_client.load(Ordering::SeqCst) > 20,
        "the relay sent too little for the case to mean anything"
    );
    assert!(matches!(result, Err(Failure::Unreachable(_))), "{result:?}");
    assert!(took < Duration::from_millis(1_800), "took {took:?}");
}
