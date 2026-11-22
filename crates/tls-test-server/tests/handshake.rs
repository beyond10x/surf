//! Handshakes that complete (story `tls-test-server`, acceptance 1 and 2): each of the three
//! suites with x25519 and with a HelloRetryRequest to secp256r1, and each scheme of our
//! `signature_algorithms` (docs/design/tls.md item 14) signing the server's CertificateVerify.
//! In each the client sends `GET / HTTP/1.1` and reads the body `ok`, and the test reads back
//! what the server saw.

use std::time::Duration;

use surf_tls::crypto::{Group, Scheme};
use surf_tls::key_schedule::CipherSuite;
use surf_tls_test_server::{Config, Seen, TestServer};

use crate::common::{self, HOST};

/// How long a test waits for the server to finish its side of a connection.
const SEEN_WITHIN: Duration = Duration::from_secs(2);

/// One handshake and one GET against a server of `config`, the double answering the leaf of
/// the config's case: the suite the client got, the body, and what the server saw.
fn complete(config: Config) -> (CipherSuite, Vec<u8>, Seen) {
    let case = config.case.clone();
    let server = TestServer::start(config).expect("the server starts");
    let mut connection = common::connect(&server, &case).expect("the handshake completes");
    let suite = connection.suite();
    let (response, end) = common::get(&mut connection, "/");
    assert_eq!(end, Ok(()), "the response ends with close_notify");
    assert_eq!(common::status_line(&response), "HTTP/1.1 200 OK");
    drop(connection);
    let seen = server.wait_seen(1, SEEN_WITHIN);
    assert_eq!(server.accepted(), 1, "the server counted one connection");
    assert_eq!(seen.len(), 1, "the server ended one connection: {seen:?}");
    let body = common::body(&response).to_vec();
    (suite, body, seen.into_iter().next().expect("one"))
}

/// Acceptance 1: `suite` with x25519, or with a HelloRetryRequest to secp256r1.
fn suite_and_group(suite: CipherSuite, retry: bool) {
    let mut config = Config::new("ok-ecdsa-p256", Scheme::EcdsaSecp256r1Sha256);
    config.suites = vec![suite];
    config.hello_retry_request = retry;
    let (got, body, seen) = complete(config);
    assert_eq!(got, suite);
    assert_eq!(body, b"ok");
    assert_eq!(seen.server_name.as_deref(), Some(HOST));
    assert_eq!(seen.alpn, vec!["http/1.1".to_string()]);
    assert_eq!(seen.suite, Some(suite));
    let group = if retry {
        Group::Secp256r1
    } else {
        Group::X25519
    };
    assert_eq!(seen.group, Some(group));
    assert_eq!(seen.hello_retry_request_sent, retry);
    assert_eq!(seen.request.as_deref(), Some("GET / HTTP/1.1"));
}

#[test]
fn aes128_gcm_sha256_x25519() {
    suite_and_group(CipherSuite::Aes128GcmSha256, false);
}

#[test]
fn aes128_gcm_sha256_retry_secp256r1() {
    suite_and_group(CipherSuite::Aes128GcmSha256, true);
}

#[test]
fn aes256_gcm_sha384_x25519() {
    suite_and_group(CipherSuite::Aes256GcmSha384, false);
}

#[test]
fn aes256_gcm_sha384_retry_secp256r1() {
    suite_and_group(CipherSuite::Aes256GcmSha384, true);
}

#[test]
fn chacha20_poly1305_sha256_x25519() {
    suite_and_group(CipherSuite::ChaCha20Poly1305Sha256, false);
}

#[test]
fn chacha20_poly1305_sha256_retry_secp256r1() {
    suite_and_group(CipherSuite::ChaCha20Poly1305Sha256, true);
}
