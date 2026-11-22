//! The modes of the server (story `tls-test-server`, acceptance 3): each ends the client with
//! the `Error` docs/design/tls.md item 6 names, the client reading with a timeout of 200 ms.
//! The kinds (a), (b) and (c) of a server limited to TLS 1.2 are those of items 7 and 12.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use surf_tls::connection::Connection;
use surf_tls::crypto::Scheme;
use surf_tls::error::{AlertDescription, Error, Violation};
use surf_tls_test_server::{Config, Mode, Received, Seen, TestServer};

use crate::common::{self, HOST};

/// The read timeout of the client's socket in every mode (acceptance 3).
const MODE_TIMEOUT: Duration = Duration::from_millis(200);

/// How long a test waits for the server to finish its side of a connection.
const SEEN_WITHIN: Duration = Duration::from_secs(2);

/// The bound of a mode that ends the client by its timeout (acceptance 3).
const TIMED_OUT_WITHIN: Duration = Duration::from_secs(1);

/// A stream that counts the bytes read through it.
struct Counted<S> {
    inner: S,
    read: usize,
}

impl<S: Read> Read for Counted<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.read += n;
        Ok(n)
    }
}

impl<S: Write> Write for Counted<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn server(mode: Mode, case: &str, scheme: Scheme) -> TestServer {
    let mut config = Config::new(case, scheme);
    config.mode = mode;
    TestServer::start(config).expect("the server starts")
}

fn ecdsa(mode: Mode) -> TestServer {
    server(mode, "ok-ecdsa-p256", Scheme::EcdsaSecp256r1Sha256)
}

/// The client's handshake over a socket with the 200 ms timeout, counting the bytes it reads:
/// the outcome, the bytes read, the time it took.
fn handshake(server: &TestServer, case: &str) -> (Result<(), Error>, usize, Duration) {
    let mut stream = Counted {
        inner: common::socket(server, MODE_TIMEOUT),
        read: 0,
    };
    let started = Instant::now();
    let outcome =
        Connection::new(&mut stream, HOST, &common::accepting(case), common::now()).map(drop);
    (outcome, stream.read, started.elapsed())
}

/// What the server saw on its one connection, once it ended it.
fn seen(server: &TestServer) -> Seen {
    let seen = server.wait_seen(1, SEEN_WITHIN);
    assert_eq!(seen.len(), 1, "the server ended one connection: {seen:?}");
    seen.into_iter().next().expect("one")
}

#[test]
fn tls12_reads_supported_versions_sends_protocol_version() {
    let server = ecdsa(Mode::Tls12ReadsSupportedVersions);
    let (outcome, _, _) = handshake(&server, "ok-ecdsa-p256");
    let error = outcome.expect_err("the handshake fails");
    assert_eq!(
        error,
        Error::AlertReceived {
            description: AlertDescription::PROTOCOL_VERSION,
            before_server_hello: true,
        }
    );
    assert_eq!(
        error.to_string(),
        "the server sent protocol_version before its ServerHello"
    );
    assert!(!seen(&server).server_hello_sent, "no ServerHello sent");
}

#[test]
fn tls12_ignores_supported_versions_sends_handshake_failure() {
    let server = ecdsa(Mode::Tls12IgnoresSupportedVersions);
    let (outcome, _, _) = handshake(&server, "ok-ecdsa-p256");
    let error = outcome.expect_err("the handshake fails");
    assert_eq!(
        error,
        Error::AlertReceived {
            description: AlertDescription::HANDSHAKE_FAILURE,
            before_server_hello: true,
        }
    );
    assert_eq!(
        error.to_string(),
        "the server sent handshake_failure before its ServerHello; a server limited to TLS 1.2 \
         answers this way"
    );
    assert!(!seen(&server).server_hello_sent, "no ServerHello sent");
}

#[test]
fn selects_tls12_ends_version_unsupported() {
    let server = server(Mode::SelectsTls12, "ok-rsa2048", Scheme::RsaPssRsaeSha256);
    let (outcome, _, _) = handshake(&server, "ok-rsa2048");
    assert_eq!(
        outcome,
        Err(Error::VersionUnsupported {
            detail: "the server selected TLS 1.2".to_string(),
        })
    );
    let seen = seen(&server);
    assert!(seen.server_hello_sent, "a TLS 1.2 ServerHello sent");
    assert_eq!(
        seen.alerts,
        vec![AlertDescription::PROTOCOL_VERSION],
        "the client sent protocol_version"
    );
}

#[test]
fn normal_sends_a_ticket_the_client_does_not_answer() {
    let server = ecdsa(Mode::Normal);
    let mut connection = Connection::new(
        common::socket(&server, MODE_TIMEOUT),
        HOST,
        &common::accepting("ok-ecdsa-p256"),
        common::now(),
    )
    .expect("the handshake completes");
    let (response, end) = common::get(&mut connection, "/");
    assert_eq!(end, Ok(()));
    assert_eq!(common::body(&response), b"ok");
    drop(connection);
    let seen = seen(&server);
    assert!(
        seen.ticket_sent,
        "a NewSessionTicket after the server's Finished"
    );
    // After its Finished the client sent its request and, at its close, close_notify: nothing
    // in answer to the ticket.
    assert_eq!(seen.after_handshake.len(), 2, "{:?}", seen.after_handshake);
    assert!(matches!(
        seen.after_handshake[0],
        Received::ApplicationData(n) if n > 0
    ));
    assert_eq!(
        seen.after_handshake[1],
        Received::Alert(AlertDescription::CLOSE_NOTIFY)
    );
}

#[test]
fn stall_after_server_hello_times_out() {
    let server = ecdsa(Mode::StallAfterServerHello);
    let (outcome, read, took) = handshake(&server, "ok-ecdsa-p256");
    assert_eq!(outcome, Err(Error::TimedOut));
    assert!(took < TIMED_OUT_WITHIN, "took {took:?}");
    assert!(read > 0, "the ServerHello came");
    assert!(seen(&server).server_hello_sent);
}

#[test]
fn never_answer_times_out_with_no_byte() {
    let server = ecdsa(Mode::NeverAnswer);
    let (outcome, read, took) = handshake(&server, "ok-ecdsa-p256");
    assert_eq!(outcome, Err(Error::TimedOut));
    assert!(took < TIMED_OUT_WITHIN, "took {took:?}");
    assert_eq!(read, 0, "no byte received");
    // The server accepted and read the hello: silence, not a listener that never accepts.
    assert_eq!(server.accepted(), 1);
    let seen = seen(&server);
    assert!(!seen.server_hello_sent);
    assert_eq!(
        seen.server_name.as_deref(),
        Some(HOST),
        "the hello was read"
    );
}

#[test]
fn oversized_certificate_is_refused() {
    let server = ecdsa(Mode::OversizedCertificate);
    let (outcome, _, _) = handshake(&server, "ok-ecdsa-p256");
    assert_eq!(
        outcome,
        Err(Error::AlertSent {
            description: AlertDescription::ILLEGAL_PARAMETER,
            violation: Violation::MessageTooLarge {
                declared: 0xff_ffff,
                bound: 65_535,
            },
        })
    );
    assert_eq!(
        seen(&server).alerts,
        vec![AlertDescription::ILLEGAL_PARAMETER]
    );
}

#[test]
fn bad_finished_is_refused() {
    let server = ecdsa(Mode::BadFinished);
    let (outcome, _, _) = handshake(&server, "ok-ecdsa-p256");
    assert_eq!(
        outcome,
        Err(Error::AlertSent {
            description: AlertDescription::DECRYPT_ERROR,
            violation: Violation::BadFinished,
        })
    );
    assert_eq!(seen(&server).alerts, vec![AlertDescription::DECRYPT_ERROR]);
}

#[test]
fn no_close_notify_is_truncated() {
    let server = ecdsa(Mode::NoCloseNotify);
    let mut connection: Connection<TcpStream> = Connection::new(
        common::socket(&server, MODE_TIMEOUT),
        HOST,
        &common::accepting("ok-ecdsa-p256"),
        common::now(),
    )
    .expect("the handshake completes");
    let (response, end) = common::get(&mut connection, "/");
    assert_eq!(end, Err(Error::Truncated));
    assert_eq!(common::status_line(&response), "HTTP/1.1 200 OK");
    assert_eq!(
        common::body(&response),
        b"ok",
        "the body, delimited by its end"
    );
}
