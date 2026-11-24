//! The adversary's cases of `story:tls-fetch-https` against `surf-fetch`: what a server, a
//! redirect or the clock can make an `https:` fetch answer, beside the cases of the module
//! `https`. Each runs against `surf-tls-test-server` on 127.0.0.1 port 0, or against a listener
//! or a forwarder of this module on 127.0.0.1; `TestNetwork` of the module `redirects` names
//! every host, so no real name is resolved.
//!
//! - A redirect from one `https:` host to another is verified for the host it leads to.
//! - The read timeout is summed over the hops also when the `https:` hop comes second.
//! - A host no certificate can match is `CertificateNameMismatch` before a byte is written.
//! - Every case of `crates/tls-test-certs/certs/INDEX` ends, through a fetch, with the reason
//!   and check `INDEX` names (`docs/design/tls.md` item 7).
//! - An oversized handshake message, a connection that ends during the handshake, a record of
//!   the body that is changed or cut, and a body past its limit over TLS.

use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use surf_fetch::{send_through, Anchors, Failure, Limits, Method, Request, Response};
use surf_tls::crypto::Scheme;
use surf_tls_test_server::{certs_dir, Config, Mode, Response as TlsReply, TestServer};

use crate::redirects::{limits, ok, page, redirect, Reply, Server, TestNetwork};

/// The longest a helper of this module waits for a peer.
const BOUND: Duration = Duration::from_secs(5);

/// The bundled trust anchors and the root of `case`.
fn anchors(case: &str) -> Anchors {
    let mut anchors = Anchors::bundled();
    let root = fs::read(certs_dir().join(case).join("root.pem")).expect("the root.pem of the case");
    anchors
        .with_extra_pem(&root)
        .expect("the root of the case is a CA");
    anchors
}

/// `localhost`, `other.test`, `a.example.test`, `a.b.example.test` and `bad_host.test` are
/// 127.0.0.1.
fn network() -> TestNetwork {
    TestNetwork::new()
        .name("localhost", &[&["127.0.0.1"]])
        .name("other.test", &[&["127.0.0.1"]])
        .name("a.example.test", &[&["127.0.0.1"]])
        .name("a.b.example.test", &[&["127.0.0.1"]])
        .name("bad_host.test", &[&["127.0.0.1"]])
}

fn get_with(address: &str, anchors: &Anchors, limits: &Limits) -> Result<Response, Failure> {
    let request = Request {
        address,
        method: Method::Get,
        body: None,
        media_type: None,
        anchors: Some(anchors),
        jar: None,
        initiator: None,
    };
    send_through(&request, limits, &network())
}

fn get(address: &str, anchors: &Anchors) -> Result<Response, Failure> {
    get_with(address, anchors, &limits())
}

fn start(config: Config) -> TestServer {
    TestServer::start(config).expect("the TLS test server starts")
}

fn p256(case: &str) -> Config {
    Config::new(case, Scheme::EcdsaSecp256r1Sha256)
}

fn tls_redirect(location: &str) -> TlsReply {
    TlsReply {
        status: 302,
        reason: "Found".to_string(),
        fields: vec![("Location".to_string(), location.to_string())],
        body: Vec::new(),
    }
}

/// A redirect from `https://localhost` to `https://other.test` on a server of the same case,
/// whose certificate names `localhost` only: the second hop is verified for `other.test`, the host it
/// connects to, not for the host the fetch started at. A client that checked the first host on
/// every hop would load a page of a host the certificate does not name.
#[test]
fn adversary_tls_a_redirect_to_another_host_is_verified_for_that_host() {
    // Two servers of the same case: the second, reached as `other.test`, would answer `/b`.
    let mut config = p256("ok-ecdsa-p256");
    config
        .responses
        .insert("/b".to_string(), TlsReply::ok(b"other host"));
    let second = start(config);
    let mut first = p256("ok-ecdsa-p256");
    first.responses.insert(
        "/a".to_string(),
        tls_redirect(&format!("https://other.test:{}/b", second.port())),
    );
    let first = start(first);
    let result = get(
        &format!("https://localhost:{}/a", first.port()),
        &anchors("ok-ecdsa-p256"),
    );
    match result {
        Err(Failure::CertificateNameMismatch(detail)) => {
            assert!(detail.contains("other.test"), "{detail}")
        }
        other => panic!("expected CertificateNameMismatch naming other.test, got {other:?}"),
    }
    let seen = second.wait_seen(1, BOUND);
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert_eq!(seen[0].server_name.as_deref(), Some("other.test"));
    assert_eq!(seen[0].request, None, "a request was sent to other.test");
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
        let handle = thread::spawn(move || {
            let Some(client) = accept_within(&listener, BOUND) else {
                return;
            };
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

/// The first connection to `listener` within `bound`, blocking afterwards.
fn accept_within(listener: &TcpListener, bound: Duration) -> Option<TcpStream> {
    listener.set_nonblocking(true).expect("nonblocking");
    let until = Instant::now() + bound;
    loop {
        match listener.accept() {
            Ok((client, _)) => {
                client.set_nonblocking(false).ok()?;
                return Some(client);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock && Instant::now() < until => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(_) => return None,
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

/// The crate documentation, "Limits": the read timeout bounds "the whole exchange after the
/// connection is made ... summed over every request of the fetch", and for `https:` "the TLS
/// handshake is inside it". Case 16 of the module `https` puts the `https:` hop first, where
/// the time left is still the whole timeout; here an `http:` hop of 600 ms comes first and the
/// TLS handshake of the second hop takes 600 ms more: with a read timeout of 1 s the fetch must
/// end `Unreachable`, within 2 s.
#[test]
fn adversary_tls_the_handshake_of_a_second_hop_gets_only_the_read_time_left() {
    let mut config = p256("ok-ecdsa-p256");
    config
        .responses
        .insert("/b".to_string(), TlsReply::ok(b"late"));
    let tls_server = start(config);
    let slow = Slow::start(tls_server.port(), Duration::from_millis(600));
    let slow_port = slow.port;
    let http = Server::start(move |target| match target {
        "/a" => Reply {
            delay: Duration::from_millis(600),
            ..redirect(302, format!("https://localhost:{slow_port}/b"))
        },
        other => page(other),
    });
    let limits = Limits {
        read_timeout: Duration::from_secs(1),
        ..limits()
    };
    let started = Instant::now();
    let result = get_with(&http.url("/a"), &anchors("ok-ecdsa-p256"), &limits);
    let took = started.elapsed();
    assert_eq!(http.targets(), ["/a"]);
    assert!(
        matches!(result, Err(Failure::Unreachable(_))),
        "two hops of 600 ms each under a read timeout of 1 s: {result:?} after {took:?}"
    );
    assert!(took < Duration::from_secs(2), "took {took:?}");
}

/// `docs/design/tls.md` item 7: "a host no certificate can match, refused before the first
/// byte (`Error::Host`, item 6)" is `CertificateNameMismatch`, detail `the host "<host>"
/// matches no certificate: <reason>`. `_` is no byte of a DNS name; the URL Standard and the
/// fetch's own address check let it through.
#[test]
fn adversary_tls_a_host_no_certificate_can_match_is_a_name_mismatch_before_any_byte() {
    let server = start(p256("ok-ecdsa-p256"));
    let result = get(
        &format!("https://bad_host.test:{}/", server.port()),
        &anchors("ok-ecdsa-p256"),
    );
    match result {
        Err(Failure::CertificateNameMismatch(detail)) => {
            assert!(detail.contains("matches no certificate"), "{detail}")
        }
        other => panic!("expected CertificateNameMismatch, got {other:?}"),
    }
    let seen = server.wait_seen(1, BOUND);
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert!(
        seen[0].alpn.is_empty() && !seen[0].server_hello_sent,
        "a ClientHello was written: {seen:?}"
    );
}

/// The scheme the test server signs with for the leaf of `case`.
fn scheme_of(case: &str) -> Scheme {
    match case {
        "ok-ecdsa-p384" => Scheme::EcdsaSecp384r1Sha384,
        "ok-ed25519" => Scheme::Ed25519,
        "ok-rsa2048"
        | "ok-rsa-pss"
        | "ok-rsa-pss-sha384"
        | "ok-rsa-pss-sha512"
        | "ok-rsa-pkcs1-sha384"
        | "ok-rsa-pkcs1-sha512"
        | "md5"
        | "sha1" => Scheme::RsaPssRsaeSha256,
        _ => Scheme::EcdsaSecp256r1Sha256,
    }
}

/// The reason of a fetch's result, as `INDEX` writes an outcome.
fn outcome(result: &Result<Response, Failure>) -> (&'static str, String) {
    match result {
        Ok(response) if response.status == 200 => ("ok", String::new()),
        Ok(response) => ("status", response.status.to_string()),
        Err(failure) => {
            let reason = match failure {
                Failure::Unreachable(_) => "Unreachable",
                Failure::UnsupportedScheme(_) => "UnsupportedScheme",
                Failure::MalformedResponse(_) => "MalformedResponse",
                Failure::TooManyRedirects(_) => "TooManyRedirects",
                Failure::ForbiddenDestination(_) => "ForbiddenDestination",
                Failure::CertificateNotTrusted(_) => "CertificateNotTrusted",
                Failure::CertificateExpired(_) => "CertificateExpired",
                Failure::CertificateNameMismatch(_) => "CertificateNameMismatch",
                Failure::TlsVersionUnsupported(_) => "TlsVersionUnsupported",
                Failure::TlsHandshakeFailed(_) => "TlsHandshakeFailed",
            };
            (reason, failure.detail().to_string())
        }
    }
}

/// Every case of `crates/tls-test-certs/certs/INDEX` (`<case> <host> <time> <outcome>
/// [<check>]`), fetched from the TLS test server at its host with its root as the extra anchor,
/// ends with the outcome `INDEX` names, and a `CertificateNotTrusted` with a detail that starts
/// with its check (`docs/design/tls.md` item 7). `rsa-1024` is left out: ring does not sign
/// with a key under 2048 bits, so the test server cannot serve it.
#[test]
fn adversary_tls_every_index_case_ends_with_its_reason_and_check() {
    let index = fs::read_to_string(certs_dir().join("INDEX")).expect("INDEX");
    let mut wrong = Vec::new();
    let mut slowest = (Duration::ZERO, String::new());
    for line in index.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [case, host, _time, expected, rest @ ..] = fields.as_slice() else {
            panic!("an INDEX line of fewer than four fields: {line:?}");
        };
        if *case == "rsa-1024" {
            continue;
        }
        let server = start(Config::new(case, scheme_of(case)));
        let address = format!("https://{host}:{}/", server.port());
        let started = Instant::now();
        let result = get(&address, &anchors(case));
        let took = started.elapsed();
        if took > slowest.0 {
            slowest = (took, (*case).to_string());
        }
        let (reason, detail) = outcome(&result);
        let check_ok = rest
            .first()
            .is_none_or(|check| detail.starts_with(&format!("{check}:")));
        if reason != *expected || !check_ok {
            wrong.push(format!(
                "{case}: INDEX {expected} {rest:?}, the fetch {reason}: {detail}"
            ));
        }
    }
    eprintln!("slowest case: {} in {:?}", slowest.1, slowest.0);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// `docs/agent-answer.md`, `TlsHandshakeFailed`: "an oversized handshake message"; item 7:
/// `illegal_parameter: a <message> of <declared> bytes, at most <bound>`. The server sends only
/// the header of a Certificate of 2^24 - 1 bytes: refused at the header, long before the read
/// timeout.
#[test]
fn adversary_tls_an_oversized_certificate_message_is_a_handshake_failure_at_its_header() {
    let mut config = p256("ok-ecdsa-p256");
    config.mode = Mode::OversizedCertificate;
    let server = start(config);
    let started = Instant::now();
    let result = get(
        &format!("https://localhost:{}/", server.port()),
        &anchors("ok-ecdsa-p256"),
    );
    let took = started.elapsed();
    match result {
        Err(Failure::TlsHandshakeFailed(detail)) => {
            assert!(detail.contains("illegal_parameter"), "{detail}");
            assert!(detail.contains("65535"), "{detail}");
        }
        other => panic!("expected TlsHandshakeFailed, got {other:?}"),
    }
    assert!(took < Duration::from_secs(2), "took {took:?}");
}

/// A listener that reads what the client sends first, writes `answer`, and closes.
fn closing(answer: &'static [u8]) -> (u16, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local address").port();
    let handle = thread::spawn(move || {
        let Some(mut client) = accept_within(&listener, BOUND) else {
            return;
        };
        let _ = client.set_read_timeout(Some(BOUND));
        let mut buf = [0u8; 4096];
        let _ = client.read(&mut buf);
        let _ = client.write_all(answer);
        let _ = client.shutdown(Shutdown::Both);
    });
    (port, handle)
}

/// `docs/design/tls.md` item 7: "the connection closed before the handshake ended
/// (`ConnectionClosed`), a truncated record among them" is `Unreachable`, not a reason of TLS.
/// Two servers: one closes after reading the ClientHello, one after the header of a handshake
/// record that promises 100 bytes.
#[test]
fn adversary_tls_a_connection_that_ends_in_the_handshake_is_unreachable() {
    for answer in [&b""[..], &[22, 3, 3, 0, 100][..]] {
        let (port, handle) = closing(answer);
        let result = get(
            &format!("https://localhost:{port}/"),
            &anchors("ok-ecdsa-p256"),
        );
        let _ = handle.join();
        assert!(
            matches!(result, Err(Failure::Unreachable(_))),
            "after {answer:?}: {result:?}"
        );
    }
}

/// What the forwarder does with one record the server sends.
#[derive(Clone, Copy, Debug)]
enum Tamper {
    /// One bit of the record's last byte (its authentication tag) flipped.
    Flip,
    /// The record's header and half of its payload, then both connections closed.
    Cut,
}

/// A forwarder on 127.0.0.1 port 0 to `port` that passes the client's bytes as they come and
/// the server's record by record, and applies `tamper` to the second record of 16,401 bytes (a
/// full record of 2^14 bytes of data with its type and tag: the first body record after the
/// one that carries the head of the response).
fn tampering(port: u16, tamper: Tamper) -> (u16, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let own = listener.local_addr().expect("local address").port();
    let handle = thread::spawn(move || {
        let Some(client) = accept_within(&listener, BOUND) else {
            return;
        };
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
        let (mut from, mut to) = (server, client);
        let mut full = 0;
        loop {
            let mut header = [0u8; 5];
            if from.read_exact(&mut header).is_err() {
                break;
            }
            let len = usize::from(u16::from_be_bytes([header[3], header[4]]));
            let mut payload = vec![0u8; len];
            if from.read_exact(&mut payload).is_err() {
                break;
            }
            if header[0] == 23 && len == 16_401 {
                full += 1;
            }
            if full == 2 && header[0] == 23 && len == 16_401 {
                match tamper {
                    Tamper::Flip => {
                        if let Some(last) = payload.last_mut() {
                            *last ^= 1;
                        }
                    }
                    Tamper::Cut => {
                        let _ = to.write_all(&header);
                        let _ = to.write_all(&payload[..len / 2]);
                        let _ = to.shutdown(Shutdown::Both);
                        let _ = from.shutdown(Shutdown::Both);
                        break;
                    }
                }
            }
            if to
                .write_all(&header)
                .and_then(|()| to.write_all(&payload))
                .is_err()
            {
                break;
            }
        }
        let _ = to.shutdown(Shutdown::Write);
        let _ = up.join();
    });
    (own, handle)
}

/// A body of 40,000 bytes with its `Content-Length`, whose second record of data the forwarder
/// changes or cuts after the head of the response arrived in the first. A changed record is a
/// violation after the handshake (`bad_record_mac`), `TlsHandshakeFailed` by item 7 ("every
/// other alert received or protocol violation") and the crate's own `Failure` documentation
/// ("or the TLS connection after it"); a record cut by the end of the connection is
/// `Unreachable` (item 7, "the connection closed after the handshake without
/// `close_notify`"). Neither is a response.
#[test]
fn adversary_tls_a_body_record_changed_or_cut_after_the_head_is_no_response() {
    for tamper in [Tamper::Flip, Tamper::Cut] {
        let mut config = p256("ok-ecdsa-p256");
        config
            .responses
            .insert("/".to_string(), TlsReply::ok(&vec![b'x'; 40_000]));
        let server = start(config);
        let (port, handle) = tampering(server.port(), tamper);
        let result = get(
            &format!("https://localhost:{port}/"),
            &anchors("ok-ecdsa-p256"),
        );
        let _ = handle.join();
        match (tamper, &result) {
            (Tamper::Flip, Err(Failure::TlsHandshakeFailed(detail))) => {
                assert!(detail.contains("bad_record_mac"), "{detail}")
            }
            (Tamper::Cut, Err(Failure::Unreachable(_))) => {}
            _ => panic!("{tamper:?}: {result:?}"),
        }
    }
}

/// The crate documentation, "Limits": "a body read until the connection closes fails as soon
/// as it holds one byte more than the limit", `MalformedResponse`; over TLS the same, also when
/// the server then ends without `close_notify` (which alone is `Unreachable`).
#[test]
fn adversary_tls_a_body_past_its_limit_is_malformed_over_tls() {
    let mut config = p256("ok-ecdsa-p256");
    config.mode = Mode::NoCloseNotify;
    config
        .responses
        .insert("/".to_string(), TlsReply::ok(&vec![b'x'; 600]));
    let server = start(config);
    let limits = Limits {
        max_body_bytes: 512,
        ..limits()
    };
    let result = get_with(
        &format!("https://localhost:{}/", server.port()),
        &anchors("ok-ecdsa-p256"),
        &limits,
    );
    assert!(
        matches!(result, Err(Failure::MalformedResponse(_))),
        "{result:?}"
    );
}

/// A server that asks for another key share by HelloRetryRequest: the fetch loads, and the
/// handshake's second flight runs under the same deadline.
#[test]
fn adversary_tls_a_hello_retry_request_loads() {
    let mut config = p256("ok-ecdsa-p256");
    config.hello_retry_request = true;
    let server = start(config);
    let response = ok(get(
        &format!("https://localhost:{}/", server.port()),
        &anchors("ok-ecdsa-p256"),
    ));
    assert_eq!(response.body, b"ok");
    let seen = server.wait_seen(1, BOUND);
    assert!(seen[0].hello_retry_request_sent, "{seen:?}");
}
