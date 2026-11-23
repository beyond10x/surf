//! Cases 1 to 17 of `story:tls-fetch-https`: `https:` addresses fetched over TLS 1.3, and
//! redirects between `http:` and `https:`. Each runs against `surf-tls-test-server` on
//! 127.0.0.1 port 0, with the certificates of `crates/tls-test-certs/certs/` and the root of the
//! case as the extra trust anchor beside the bundled ones, on the system clock (the certificates
//! run to 2125, `expired` ended in 2001). The `http:` servers are those of the module
//! `redirects`, and `localhost` is 127.0.0.1 through its `TestNetwork`: no real name is
//! resolved.
//!
//! Besides the cases: an empty set of trust anchors is refused before any connection, and what a
//! TLS connection adds to what a fetch holds (the crate documentation, "Memory"), counted on the
//! fetch's thread by the allocator of the module `adversary_redirect_memory`.

use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use surf_fetch::{send_through, Anchors, Failure, Limits, Method, Request, Response};
use surf_tls::crypto::Scheme;
use surf_tls_test_server::{certs_dir, Config, Mode, Response as TlsReply, TestServer};

use crate::adversary_redirect_memory::held_on_this_thread;
use crate::redirects::{limits, ok, page, redirect, Reply, Server, TestNetwork};

/// The longest a case waits for a server to report what it saw.
const BOUND: Duration = Duration::from_secs(5);

/// The bundled trust anchors and the root of `case` as the extra anchor.
fn anchors(case: &str) -> Anchors {
    let mut anchors = Anchors::bundled();
    let root = fs::read(certs_dir().join(case).join("root.pem")).expect("the root.pem of the case");
    let added = anchors
        .with_extra_pem(&root)
        .expect("the root of the case is a CA");
    assert_eq!(added, 1);
    anchors
}

/// A network of the test in which `localhost` is 127.0.0.1.
fn network() -> TestNetwork {
    TestNetwork::new().name("localhost", &[&["127.0.0.1"]])
}

/// A `GET` of `address` verified against `anchors`, over `network`.
fn get_through(
    address: &str,
    anchors: &Anchors,
    limits: &Limits,
    network: &TestNetwork,
) -> Result<Response, Failure> {
    let request = Request {
        address,
        method: Method::Get,
        body: None,
        media_type: None,
        anchors: Some(anchors),
    };
    send_through(&request, limits, network)
}

fn get(address: &str, anchors: &Anchors) -> Result<Response, Failure> {
    get_through(address, anchors, &limits(), &network())
}

/// The TLS test server of `case`, signing with `scheme`, answering as `mode`.
fn tls(case: &str, scheme: Scheme, mode: Mode) -> TestServer {
    let mut config = Config::new(case, scheme);
    config.mode = mode;
    TestServer::start(config).expect("the TLS test server starts")
}

/// The TLS test server of a case whose leaf has a P-256 key.
fn p256(case: &str) -> TestServer {
    tls(case, Scheme::EcdsaSecp256r1Sha256, Mode::Normal)
}

/// The TLS test server of a case with the responses `paths` beside `/`.
fn tls_with(case: &str, paths: Vec<(String, TlsReply)>) -> TestServer {
    let mut config = Config::new(case, Scheme::EcdsaSecp256r1Sha256);
    config.responses.extend(paths);
    TestServer::start(config).expect("the TLS test server starts")
}

/// A 302 of the TLS test server to `location`.
fn tls_redirect(location: &str) -> TlsReply {
    TlsReply {
        status: 302,
        reason: "Found".to_string(),
        fields: vec![("Location".to_string(), location.to_string())],
        body: Vec::new(),
    }
}

fn failure(result: Result<Response, Failure>) -> Failure {
    match result {
        Ok(response) => panic!(
            "expected a failure, got status {} from {}",
            response.status, response.url
        ),
        Err(failure) => failure,
    }
}

/// Whether `listener` was connected to: it is asked once, without waiting.
fn was_connected(listener: &TcpListener) -> bool {
    listener.set_nonblocking(true).expect("nonblocking");
    match listener.accept() {
        Err(e) if e.kind() == ErrorKind::WouldBlock => false,
        Err(e) => panic!("accept: {e}"),
        Ok(_) => true,
    }
}

#[test]
fn case_1_an_https_address_loads_with_server_name_and_alpn() {
    let server = p256("ok-ecdsa-p256");
    let address = format!("https://localhost:{}/", server.port());
    let response = ok(get(&address, &anchors("ok-ecdsa-p256")));
    assert_eq!(response.status, 200);
    assert_eq!(response.media_type, "text/plain");
    assert_eq!(response.body, b"ok");
    assert_eq!(response.url, address);
    let seen = server.wait_seen(1, BOUND);
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert_eq!(seen[0].server_name.as_deref(), Some("localhost"));
    assert_eq!(seen[0].alpn, ["http/1.1"]);
    assert_eq!(seen[0].request.as_deref(), Some("GET / HTTP/1.1"));
}

#[test]
fn case_2_an_ip_address_matches_an_ip_entry_and_sends_no_server_name() {
    let server = tls("ok-rsa2048", Scheme::RsaPssRsaeSha256, Mode::Normal);
    let address = format!("https://127.0.0.1:{}/", server.port());
    let response = ok(get(&address, &anchors("ok-rsa2048")));
    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"ok");
    let seen = server.wait_seen(1, BOUND);
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert_eq!(seen[0].server_name, None);
}

#[test]
fn case_3_an_expired_certificate_is_certificate_expired_naming_its_not_after() {
    let server = p256("expired");
    let address = format!("https://localhost:{}/", server.port());
    let failure = failure(get(&address, &anchors("expired")));
    assert!(
        matches!(failure, Failure::CertificateExpired(_)),
        "{failure:?}"
    );
    let text = failure.to_string();
    assert!(text.starts_with("certificate expired: "), "{text}");
    assert!(text.contains("to 2001-01-01"), "{text}");
}

#[test]
fn case_4_a_certificate_for_another_host_is_a_name_mismatch_naming_the_host() {
    let server = p256("wrong-host");
    let address = format!("https://localhost:{}/", server.port());
    let failure = failure(get(&address, &anchors("wrong-host")));
    assert!(
        matches!(failure, Failure::CertificateNameMismatch(_)),
        "{failure:?}"
    );
    assert!(failure.to_string().contains("localhost"), "{failure}");
}

#[test]
fn case_5_without_the_extra_anchor_the_certificate_is_not_trusted_naming_the_check() {
    let server = tls("ok-rsa2048", Scheme::RsaPssRsaeSha256, Mode::Normal);
    let address = format!("https://localhost:{}/", server.port());
    let failure = failure(get(&address, &Anchors::bundled()));
    assert!(
        matches!(failure, Failure::CertificateNotTrusted(_)),
        "{failure:?}"
    );
    let text = failure.to_string();
    assert!(text.contains("NoPathToAnchor"), "{text}");
    assert!(text.contains("no path to a trust anchor"), "{text}");
}

#[test]
fn case_6_the_three_kinds_of_a_server_limited_to_tls_1_2() {
    let anchors = anchors("ok-ecdsa-p256");
    let fetch = |mode: Mode| {
        let server = tls("ok-ecdsa-p256", Scheme::EcdsaSecp256r1Sha256, mode);
        failure(get(
            &format!("https://localhost:{}/", server.port()),
            &anchors,
        ))
    };
    let a = fetch(Mode::Tls12ReadsSupportedVersions);
    assert!(matches!(a, Failure::TlsVersionUnsupported(_)), "{a:?}");
    let b = fetch(Mode::Tls12IgnoresSupportedVersions);
    assert!(matches!(b, Failure::TlsHandshakeFailed(_)), "{b:?}");
    assert!(
        b.to_string().contains(
            "the server sent handshake_failure before its ServerHello; a server limited to TLS \
             1.2 answers this way"
        ),
        "{b}"
    );
    let c = fetch(Mode::SelectsTls12);
    assert!(matches!(c, Failure::TlsVersionUnsupported(_)), "{c:?}");
}

#[test]
fn case_7_a_bad_finished_is_a_handshake_failure_naming_decrypt_error() {
    let server = tls(
        "ok-ecdsa-p256",
        Scheme::EcdsaSecp256r1Sha256,
        Mode::BadFinished,
    );
    let address = format!("https://localhost:{}/", server.port());
    let failure = failure(get(&address, &anchors("ok-ecdsa-p256")));
    assert!(
        matches!(failure, Failure::TlsHandshakeFailed(_)),
        "{failure:?}"
    );
    assert!(failure.to_string().contains("decrypt_error"), "{failure}");
}

#[test]
fn case_8_a_server_that_never_finishes_is_unreachable_within_the_read_timeout() {
    let anchors = anchors("ok-ecdsa-p256");
    let limits = Limits {
        read_timeout: Duration::from_secs(1),
        ..limits()
    };
    for mode in [Mode::StallAfterServerHello, Mode::NeverAnswer] {
        let server = tls("ok-ecdsa-p256", Scheme::EcdsaSecp256r1Sha256, mode);
        let address = format!("https://localhost:{}/", server.port());
        let started = Instant::now();
        let result = get_through(&address, &anchors, &limits, &network());
        let took = started.elapsed();
        assert!(
            matches!(result, Err(Failure::Unreachable(_))),
            "{mode:?}: {result:?}"
        );
        assert!(took < Duration::from_secs(2), "{mode:?}: took {took:?}");
    }
}

#[test]
fn case_9_no_close_notify_fails_a_body_delimited_by_the_end_and_not_a_declared_one() {
    let anchors = anchors("ok-ecdsa-p256");
    let until_close = tls(
        "ok-ecdsa-p256",
        Scheme::EcdsaSecp256r1Sha256,
        Mode::NoCloseNotify,
    );
    let result = get(
        &format!("https://localhost:{}/", until_close.port()),
        &anchors,
    );
    assert!(matches!(result, Err(Failure::Unreachable(_))), "{result:?}");
    let mut config = Config::new("ok-ecdsa-p256", Scheme::EcdsaSecp256r1Sha256);
    config.mode = Mode::NoCloseNotify;
    let mut declared = TlsReply::ok(b"ok");
    declared
        .fields
        .push(("Content-Length".to_string(), "2".to_string()));
    config.responses.insert("/".to_string(), declared);
    let declared = TestServer::start(config).expect("the TLS test server starts");
    let response = ok(get(
        &format!("https://localhost:{}/", declared.port()),
        &anchors,
    ));
    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"ok");
}

#[test]
fn case_10_a_refused_connection_is_unreachable_with_no_handshake() {
    let result = get("https://127.0.0.1:1/", &anchors("ok-ecdsa-p256"));
    match result {
        Err(Failure::Unreachable(detail)) => {
            assert!(detail.contains("connecting to 127.0.0.1:1"), "{detail}")
        }
        other => panic!("expected Unreachable, got {other:?}"),
    }
}

#[test]
fn case_11_an_http_redirect_to_https_is_followed() {
    let tls_server = tls_with(
        "ok-ecdsa-p256",
        vec![("/b".to_string(), TlsReply::ok(b"from tls"))],
    );
    let tls_port = tls_server.port();
    let http = Server::start(move |target| match target {
        "/a" => redirect(302, format!("https://localhost:{tls_port}/b")),
        other => page(other),
    });
    let response = ok(get(&http.url("/a"), &anchors("ok-ecdsa-p256")));
    let targets = http.targets();
    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"from tls");
    assert_eq!(response.url, format!("https://localhost:{tls_port}/b"));
    assert_eq!(targets, ["/a"]);
    let seen = tls_server.wait_seen(1, BOUND);
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert_eq!(seen[0].request.as_deref(), Some("GET /b HTTP/1.1"));
}

#[test]
fn case_12_an_https_redirect_to_http_is_followed() {
    let http = Server::start(page);
    let to = http.url("/b");
    let tls_server = tls_with("ok-ecdsa-p256", vec![("/a".to_string(), tls_redirect(&to))]);
    let address = format!("https://localhost:{}/a", tls_server.port());
    let response = ok(get(&address, &anchors("ok-ecdsa-p256")));
    let targets = http.targets();
    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"<p>/b</p>");
    assert_eq!(response.url, to);
    assert_eq!(targets, ["/b"]);
}

/// `/0` to `/20` alternate: an even number is the `http:` server, an odd one the TLS server, and
/// each redirects to the next on the other server; `/20` answers a page, or with `one_more` a
/// 21st redirect.
fn alternating(one_more: bool) -> (Result<Response, Failure>, Vec<String>, usize, String) {
    let tls_port = Arc::new(AtomicU16::new(0));
    let port = Arc::clone(&tls_port);
    let http = Server::start(move |target| {
        let n: usize = target.trim_start_matches('/').parse().unwrap_or(usize::MAX);
        let tls_port = port.load(Ordering::SeqCst);
        if n < 20 || (n == 20 && one_more) {
            redirect(302, format!("https://localhost:{tls_port}/{}", n + 1))
        } else {
            page(target)
        }
    });
    let paths = (1..20)
        .step_by(2)
        .map(|n| {
            let to = http.url(&format!("/{}", n + 1));
            (format!("/{n}"), tls_redirect(&to))
        })
        .collect();
    let tls_server = tls_with("ok-ecdsa-p256", paths);
    tls_port.store(tls_server.port(), Ordering::SeqCst);
    let result = get(&http.url("/0"), &anchors("ok-ecdsa-p256"));
    let last = http.url("/20");
    let targets = http.targets();
    let handshakes = tls_server
        .wait_seen(10, BOUND)
        .iter()
        .filter(|seen| seen.request.is_some())
        .count();
    (result, targets, handshakes, last)
}

#[test]
fn case_13_a_chain_of_20_redirects_alternating_schemes_ends_at_the_21st_and_one_more_is_too_many() {
    let (result, targets, requests, last) = alternating(false);
    let response = ok(result);
    assert_eq!(response.status, 200);
    assert_eq!(response.url, last);
    assert_eq!(targets.len(), 11, "{targets:?}");
    assert_eq!(requests, 10);
    let (result, targets, requests, _) = alternating(true);
    assert!(
        matches!(result, Err(Failure::TooManyRedirects(_))),
        "{result:?}"
    );
    assert_eq!(targets.len() + requests, 21, "{targets:?}");
}

#[test]
fn case_14_an_http_redirect_to_an_https_server_of_another_host_is_a_name_mismatch() {
    let tls_server = p256("wrong-host");
    let tls_port = tls_server.port();
    let http = Server::start(move |target| match target {
        "/a" => redirect(302, format!("https://localhost:{tls_port}/")),
        other => page(other),
    });
    let failure = failure(get(&http.url("/a"), &anchors("wrong-host")));
    assert!(
        matches!(failure, Failure::CertificateNameMismatch(_)),
        "{failure:?}"
    );
}

#[test]
fn case_15_an_ftp_location_is_unsupported_and_never_connected_to() {
    let held = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port2 = held.local_addr().expect("local address").port();
    let tls_server = tls_with(
        "ok-ecdsa-p256",
        vec![(
            "/a".to_string(),
            tls_redirect(&format!("ftp://127.0.0.1:{port2}/")),
        )],
    );
    let address = format!("https://localhost:{}/a", tls_server.port());
    let result = get(&address, &anchors("ok-ecdsa-p256"));
    assert!(
        matches!(result, Err(Failure::UnsupportedScheme(_))),
        "{result:?}"
    );
    assert!(!was_connected(&held), "the ftp: target was connected to");
}

/// A forwarder on 127.0.0.1 port 0 to `port`: what the client sends goes on at once, what the
/// server sends first waits `delay`, so a TLS handshake through it takes at least `delay`.
struct Slow {
    port: u16,
    handle: Option<JoinHandle<()>>,
}

impl Slow {
    fn start(port: u16, delay: Duration) -> Slow {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
        let own = listener.local_addr().expect("local address").port();
        listener.set_nonblocking(true).expect("nonblocking");
        let handle = thread::spawn(move || {
            // Accepting gives up after `BOUND`, so a fetch that never connects ends the case.
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
            }
            let (mut up_from, mut up_to) = (
                client.try_clone().expect("clone"),
                server.try_clone().expect("clone"),
            );
            let up = thread::spawn(move || pump(&mut up_from, &mut up_to, Duration::ZERO));
            let (mut down_from, mut down_to) = (server, client);
            pump(&mut down_from, &mut down_to, delay);
            let _ = up.join();
        });
        Slow {
            port: own,
            handle: Some(handle),
        }
    }
}

impl Drop for Slow {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Copies `from` to `to` until either ends, the first bytes after `delay`.
fn pump(from: &mut TcpStream, to: &mut TcpStream, delay: Duration) {
    let mut buf = [0u8; 16 * 1024];
    let mut first = true;
    while let Ok(n) = from.read(&mut buf) {
        if n == 0 {
            break;
        }
        if first {
            thread::sleep(delay);
            first = false;
        }
        if to.write_all(&buf[..n]).is_err() {
            break;
        }
    }
    let _ = to.shutdown(Shutdown::Write);
}

#[test]
fn case_16_the_read_timeout_bounds_the_handshake_and_the_exchange_summed_over_the_hops() {
    let http = Server::start(|target| Reply {
        delay: Duration::from_millis(600),
        ..page(target)
    });
    let to = http.url("/b");
    let tls_server = tls_with("ok-ecdsa-p256", vec![("/a".to_string(), tls_redirect(&to))]);
    let slow = Slow::start(tls_server.port(), Duration::from_millis(600));
    let limits = Limits {
        read_timeout: Duration::from_secs(1),
        ..limits()
    };
    let address = format!("https://localhost:{}/a", slow.port);
    let started = Instant::now();
    let result = get_through(&address, &anchors("ok-ecdsa-p256"), &limits, &network());
    let took = started.elapsed();
    let targets = http.targets();
    assert!(matches!(result, Err(Failure::Unreachable(_))), "{result:?}");
    assert!(took < Duration::from_secs(2), "took {took:?}");
    assert_eq!(targets, ["/b"], "the http: hop was not reached");
}

#[test]
fn case_17_the_destination_rule_holds_on_https_hops() {
    // A public https: start whose redirect leads to a private address.
    let tls_server = tls_with(
        "ok-wildcard",
        vec![("/a".to_string(), tls_redirect("https://10.0.0.1/b"))],
    );
    let network = TestNetwork::new()
        .name("a.example.test", &[&["203.0.113.5"]])
        .route("203.0.113.5:443", tls_server.port());
    let result = get_through(
        "https://a.example.test/a",
        &anchors("ok-wildcard"),
        &limits(),
        &network,
    );
    assert!(
        matches!(result, Err(Failure::ForbiddenDestination(_))),
        "{result:?}"
    );
    assert_eq!(network.dialed(), ["203.0.113.5:443"]);
    assert_eq!(tls_server.accepted(), 1);
    let seen = tls_server.wait_seen(1, BOUND);
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert!(seen[0].suite.is_some(), "no handshake: {seen:?}");

    // A public http: start whose redirect leads to the TLS server on 127.0.0.1.
    let local = p256("ok-ecdsa-p256");
    let local_port = local.port();
    let public = Server::start(move |target| match target {
        "/a" => redirect(302, format!("https://127.0.0.1:{local_port}/")),
        other => page(other),
    });
    let network = TestNetwork::new().route("203.0.113.5:80", public.port);
    let result = get_through(
        "http://203.0.113.5/a",
        &anchors("ok-ecdsa-p256"),
        &limits(),
        &network,
    );
    assert!(
        matches!(result, Err(Failure::ForbiddenDestination(_))),
        "{result:?}"
    );
    assert_eq!(network.dialed(), ["203.0.113.5:80"]);
    assert_eq!(public.targets(), ["/a"]);
    assert_eq!(local.accepted(), 0, "the TLS server was connected to");
}

/// `docs/design/tls.md` item 6 and `surf_tls::anchors`: the place that builds a verifier refuses
/// an empty set of anchors, here before any connection.
#[test]
fn an_empty_set_of_anchors_is_refused_before_any_connection() {
    let server = p256("ok-ecdsa-p256");
    let empty = Anchors::from_list(b"");
    assert_eq!(empty.count(), 0);
    let network = network();
    let result = get_through(
        &format!("https://localhost:{}/", server.port()),
        &empty,
        &limits(),
        &network,
    );
    assert!(
        matches!(result, Err(Failure::CertificateNotTrusted(_))),
        "{result:?}"
    );
    assert!(network.dialed().is_empty(), "{:?}", network.dialed());
    assert_eq!(server.accepted(), 0);
}

/// The crate documentation, "Memory": what an `https:` fetch holds at its peak beside what the
/// same fetch over `http:` holds, counted on the fetch's thread (the servers run on threads of
/// their own). The anchors are made before the count starts, as a caller makes them once.
#[test]
fn a_tls_connection_adds_at_most_the_documented_bytes() {
    let body = vec![b'x'; 64 * 1024];
    let mut config = Config::new("ok-ecdsa-p256", Scheme::EcdsaSecp256r1Sha256);
    config
        .responses
        .insert("/".to_string(), TlsReply::ok(&body));
    let tls_server = TestServer::start(config).expect("the TLS test server starts");
    let served = body.clone();
    let http = Server::start(move |_| Reply {
        status: 200,
        fields: vec![("Content-Type", b"text/plain".to_vec())],
        body: served.clone(),
        framed: true,
        delay: Duration::ZERO,
    });
    let anchors = anchors("ok-ecdsa-p256");
    let network = network();
    let tls_address = format!("https://localhost:{}/", tls_server.port());
    let http_address = http.url("/");
    // Each once before the count, so that what is made once per process is made.
    ok(get_through(&tls_address, &anchors, &limits(), &network));
    ok(get_through(&http_address, &anchors, &limits(), &network));

    let (over_tls, tls_held) =
        held_on_this_thread(|| get_through(&tls_address, &anchors, &limits(), &network));
    let (over_http, http_held) =
        held_on_this_thread(|| get_through(&http_address, &anchors, &limits(), &network));

    assert_eq!(ok(over_tls).body, body);
    assert_eq!(ok(over_http).body, body);
    let added = tls_held.saturating_sub(http_held);
    eprintln!(
        "an https: fetch of a 64 KiB body held {tls_held} bytes, over http: {http_held}, \
         {added} more"
    );
    assert!(
        added <= TLS_ADDS_MAX,
        "crate documentation of surf-fetch, \"Memory\": a TLS connection added {added} bytes, \
         more than {TLS_ADDS_MAX}"
    );
}

/// What the crate documentation, "Memory", states a TLS connection adds at most.
const TLS_ADDS_MAX: usize = 1 << 20;
