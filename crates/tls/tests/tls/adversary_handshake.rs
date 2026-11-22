//! The adversary's cases against the client handshake and the connection of
//! `story:tls-handshake`: extensions the table of RFC 8446 section 4.2 does not allow in a
//! CertificateRequest or a NewSessionTicket, a flood of KeyUpdates that ask for an answer, a
//! HelloRetryRequest whose cookie cannot fit into a second ClientHello, and handshake messages
//! that span a key change. Each counts records or reads outcomes; none bounds wall time.

#![forbid(unsafe_code)]

use surf_tls::crypto::Group;
use surf_tls::error::{AlertDescription as A, Error, Violation};
use surf_tls::key_schedule::CipherSuite;

use crate::handshake::{
    Double, Server, block, connect, last_alert, msg, outcome, records_raw, sent, v8, v16,
};
use crate::rfc8448::Pipe;

const SUITE: CipherSuite = CipherSuite::Aes128GcmSha256;

fn ecdsa() -> Double {
    Double::accepting("ok-ecdsa-p256")
}

/// The client over `server`'s input ends with `AlertSent { description, violation }` and the
/// last record it wrote is that fatal alert.
fn refused(server: &Server, description: A, violation: Violation) {
    let (result, written) = outcome(&server.input, &ecdsa());
    assert_eq!(result, Err(sent(description, violation)));
    assert_eq!(
        last_alert(server, &written),
        Some((2, description.0)),
        "the alert written"
    );
}

/// A NewSessionTicket of a 64-byte ticket with `extensions`.
fn ticket(extensions: &[(u16, Vec<u8>)]) -> Vec<u8> {
    let mut body = 7200u32.to_be_bytes().to_vec();
    body.extend_from_slice(&0x0102_0304u32.to_be_bytes());
    body.extend_from_slice(&v8(&[0]));
    body.extend_from_slice(&v16(&[0x5a; 64]));
    body.extend_from_slice(&block(extensions));
    msg(4, &body)
}

/// RFC 8446 section 4.2: "If an implementation receives an extension which it recognizes and
/// which is not specified for the message in which it appears, it MUST abort the handshake with
/// an "illegal_parameter" alert." `supported_versions` and `key_share` are not specified for a
/// CertificateRequest; the story's rule "Extensions (4.2)" names the same refusal.
#[test]
fn adversary_certificate_request_extension_not_allowed_here() {
    for (extension_type, data) in [(43u16, vec![3, 4]), (51, vec![0, 0x1d])] {
        let mut server = Server::new(SUITE);
        server.hello();
        let ee = server.encrypted_extensions(&[]);
        server.push(&ee);
        let mut body = v8(&[]);
        body.extend_from_slice(&block(&[(13, v16(&[4, 3])), (extension_type, data)]));
        server.push(&msg(13, &body));
        let certificate = server.certificate();
        server.push(&certificate);
        let cv = server.certificate_verify();
        server.push(&cv);
        server.push_finished();
        server.flush();
        refused(
            &server,
            A::ILLEGAL_PARAMETER,
            Violation::ExtensionNotAllowedHere,
        );
    }
}

/// A CertificateRequest whose `signature_algorithms` does not parse as
/// `supported_signature_algorithms<2..2^16-2>` (RFC 8446 section 4.2.3): one byte. The story:
/// "A message that does not parse (a length past its end, a short field) is `decode_error` and
/// `Malformed { message }` (section 6)".
#[test]
fn adversary_certificate_request_signature_algorithms_malformed() {
    let mut server = Server::new(SUITE);
    server.hello();
    let ee = server.encrypted_extensions(&[]);
    server.push(&ee);
    let mut body = v8(&[]);
    body.extend_from_slice(&block(&[(13, vec![0xff])]));
    server.push(&msg(13, &body));
    let certificate = server.certificate();
    server.push(&certificate);
    let cv = server.certificate_verify();
    server.push(&cv);
    server.push_finished();
    server.flush();
    refused(
        &server,
        A::DECODE_ERROR,
        Violation::Malformed {
            message: "CertificateRequest",
        },
    );
}

/// The same rule of section 4.2 for a NewSessionTicket, which the table allows `early_data`
/// alone of the extensions it names: a `key_share` in a ticket is `illegal_parameter`.
#[test]
fn adversary_new_session_ticket_extension_not_allowed_here() {
    let mut server = Server::handshake(SUITE);
    server.protected(22, &ticket(&[(51, vec![0, 0x1d])]));
    server.data(b"ok");
    let mut pipe = Pipe::with_input(server.input.clone());
    let verifier = ecdsa();
    let mut connection = connect(&mut pipe, &verifier).expect("the handshake completes");
    let mut buf = [0u8; 8];
    let read = connection.read_data(&mut buf).map(|n| buf[..n].to_vec());
    connection.into_inner();
    assert_eq!(
        read,
        Err(sent(
            A::ILLEGAL_PARAMETER,
            Violation::ExtensionNotAllowedHere
        ))
    );
}

/// RFC 8446 section 4.6.3: an `update_requested` is answered "prior to sending its next
/// Application Data record", which "causes an implementation which receives multiple
/// KeyUpdates while it is silent to respond with a single update"; `connection.rs` says the
/// same ("answered before the next data"). Eight requests while the client is silent, then
/// data, then one write of the client: one KeyUpdate and one record of data, counted.
#[test]
fn adversary_key_update_requests_answered_once() {
    let mut server = Server::handshake(SUITE);
    for _ in 0..8 {
        server.key_update(1);
    }
    server.data(b"ok");
    let mut pipe = Pipe::with_input(server.input.clone());
    let verifier = ecdsa();
    let mut connection = connect(&mut pipe, &verifier).expect("the handshake completes");
    let start = connection.get_ref().output.len();
    let mut buf = [0u8; 8];
    let n = connection.read_data(&mut buf).expect("the data");
    assert_eq!(&buf[..n], b"ok");
    connection.write_data(b"hi").expect("the write");
    let written = records_raw(&connection.get_ref().output[start..]).len();
    connection.into_inner();
    assert_eq!(
        written, 2,
        "records written for 8 requests and one write: {written}"
    );
}

/// A HelloRetryRequest of 65,530 bytes (inside the bound of item 9.1) whose cookie of 65,440
/// bytes does not fit with the other extensions into the `extensions<8..2^16-1>` of a second
/// ClientHello. The story: "Every failure sends the alert of section 6.2 and returns the
/// `Error` of item 6"; the client ends with a fatal alert, not with an error of its stream.
#[test]
fn adversary_retry_cookie_too_long_for_second_hello() {
    let mut server = Server::new(SUITE);
    let cookie = vec![0x5a; 65_440];
    let hrr = server
        .hello_retry_request(Some(0x0017), Some(&cookie))
        .encode();
    assert!(hrr.len() - 4 <= 65_535, "the message is inside item 9.1");
    for fragment in hrr.chunks(16_384) {
        server.plain(22, fragment);
    }
    let (result, written) = outcome(&server.input, &ecdsa());
    assert!(
        matches!(result, Err(Error::AlertSent { .. })),
        "the client ended with {result:?}"
    );
    let records = records_raw(&written);
    let last = records.last().expect("a record");
    assert_eq!((last[0], last[5]), (21, 2), "a fatal alert, last");
}

/// RFC 8446 section 4.1.4: after a HelloRetryRequest that selected TLS_AES_128_GCM_SHA256, a
/// ServerHello that selects TLS_CHACHA20_POLY1305_SHA256, a suite of the same hash, so the
/// transcript's own refusal of another hash does not catch it: `illegal_parameter`,
/// `SuiteChangedAfterRetry`.
#[test]
fn adversary_suite_changed_to_same_hash_after_retry() {
    let mut server = Server::new(SUITE);
    let hrr = server.hello_retry_request(Some(0x0017), Some(b"cookie"));
    server.send_retry(&hrr);
    let mut sh = server.server_hello(Group::Secp256r1);
    sh.suite = CipherSuite::ChaCha20Poly1305Sha256.code();
    server.plain(22, &sh.encode());
    refused(
        &server,
        A::ILLEGAL_PARAMETER,
        Violation::SuiteChangedAfterRetry,
    );
}

/// RFC 8446 section 5.1: the ServerHello and the EncryptedExtensions in one record without
/// protection span the change to the handshake keys: `unexpected_message`,
/// `HandshakeSpansKeyChange`.
#[test]
fn adversary_server_hello_and_encrypted_extensions_in_one_record() {
    let mut server = Server::new(SUITE);
    let mut content = server.server_hello(Group::X25519).encode();
    content.extend_from_slice(&msg(8, &block(&[])));
    server.plain(22, &content);
    refused(
        &server,
        A::UNEXPECTED_MESSAGE,
        Violation::HandshakeSpansKeyChange,
    );
}

/// RFC 8446 section 5.1: the server's Finished and a NewSessionTicket in one record span the
/// change to the application keys: `unexpected_message`, `HandshakeSpansKeyChange`, and the
/// client writes no Finished.
#[test]
fn adversary_finished_and_ticket_in_one_record() {
    let mut server = Server::new(SUITE);
    server.hello();
    let ee = server.encrypted_extensions(&[]);
    server.push(&ee);
    let certificate = server.certificate();
    server.push(&certificate);
    let cv = server.certificate_verify();
    server.push(&cv);
    server.push_finished();
    server.push(&ticket(&[]));
    server.flush();
    refused(
        &server,
        A::UNEXPECTED_MESSAGE,
        Violation::HandshakeSpansKeyChange,
    );
}
