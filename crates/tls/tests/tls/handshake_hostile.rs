//! A server that deviates, each case from crafted server bytes in memory
//! (`story:tls-handshake`): every row of the tables "RFC 8446 client obligations" and
//! "Messages the client receives" that names a test of this file, each ending with the alert
//! its rule names (read back from what the client wrote) and the `Error` it returns; every
//! length field at its edges; every state against every message that must not arrive in it;
//! the bounds of what a server can make the client hold; and a seeded run of mutated messages.

#![forbid(unsafe_code)]

use std::io::{self, Read, Write};

use surf_tls::connection::Connection;
use surf_tls::crypto::Group;
use surf_tls::der::Time;
use surf_tls::error::{AlertDescription as A, Error, Violation};
use surf_tls::key_schedule::{self, CipherSuite};
use surf_tls::verify::{CertificateRejection, Check};
use surf_tls::x509::Certificate;

use crate::alloc::peak_while;
use crate::handshake::{
    Double, Server, Sh, block, certificate_message, chain, connect, hrr_random, last_alert, msg,
    outcome, outcome_to, records_raw, seal, sent, v8, v16,
};
use crate::rfc8448::Pipe;

const SUITE: CipherSuite = CipherSuite::Aes128GcmSha256;

fn ecdsa() -> Double {
    Double::accepting("ok-ecdsa-p256")
}

/// The client over `server`'s input ends with `AlertSent { description, violation }` and the
/// last record it wrote is that fatal alert.
fn refused(server: &Server, description: A, violation: Violation) {
    refused_by(server, &ecdsa(), sent(description, violation), description);
}

/// The client ends with `error` and its last record is the fatal alert `description`.
fn refused_by(server: &Server, verifier: &Double, error: Error, description: A) {
    let (result, written) = outcome(&server.input, verifier);
    assert_eq!(result, Err(error));
    assert_eq!(
        last_alert(server, &written),
        Some((2, description.0)),
        "the alert written"
    );
}

/// A server whose first answer is the ServerHello `sh`, written as given.
fn answered(edit: impl FnOnce(&mut Sh, &Server)) -> Server {
    let mut server = Server::new(SUITE);
    let mut sh = server.server_hello(Group::X25519);
    edit(&mut sh, &server);
    server.plain(22, &sh.encode());
    server
}

/// A server whose first answer is a HelloRetryRequest for secp256r1, edited.
fn retried(edit: impl FnOnce(&mut Sh)) -> Server {
    let mut server = Server::new(SUITE);
    let mut hrr = server.hello_retry_request(Some(0x0017), Some(b"cookie"));
    edit(&mut hrr);
    server.plain(22, &hrr.encode());
    server
}

/// A server after a good HelloRetryRequest for secp256r1, the client's second hello read.
fn after_retry() -> Server {
    let mut server = Server::new(SUITE);
    let hrr = server.hello_retry_request(Some(0x0017), Some(b"cookie"));
    server.send_retry(&hrr);
    server
}

/// A server after its ServerHello, its next messages pushed by `flight` and flushed.
fn flight(flight: impl FnOnce(&mut Server)) -> Server {
    let mut server = Server::new(SUITE);
    server.hello();
    flight(&mut server);
    server.flush();
    server
}

fn push_ee(server: &mut Server) {
    let ee = server.encrypted_extensions(&[]);
    server.push(&ee);
}

fn push_certificate(server: &mut Server) {
    let certificate = server.certificate();
    server.push(&certificate);
}

fn push_cv(server: &mut Server) {
    let cv = server.certificate_verify();
    server.push(&cv);
}

fn unexpected(expected: &'static str, got: u8) -> Violation {
    Violation::UnexpectedMessage { expected, got }
}

fn malformed(message: &'static str) -> Violation {
    Violation::Malformed { message }
}

// ---------------------------------------------------------------------------------------------
// ServerHello and HelloRetryRequest.

/// RFC 8446 section 4.1.3: a session id echo other than ours (another, or empty).
#[test]
fn session_id_not_echoed() {
    for id in [vec![0x23; 32], vec![]] {
        let server = answered(|sh, _| sh.session_id = id);
        refused(&server, A::ILLEGAL_PARAMETER, Violation::SessionIdMismatch);
    }
}

/// Acceptance 4.6: suite 0x1304, not offered; and 0x0000.
#[test]
fn suite_not_offered() {
    for suite in [0x1304, 0x0000] {
        let server = answered(|sh, _| sh.suite = suite);
        refused(&server, A::ILLEGAL_PARAMETER, Violation::UnofferedSuite);
    }
}

/// Acceptance 4.4: a TLS 1.3 ServerHello that selects 0xC02F, a HelloRetryRequest that selects
/// 0xCCA8, both TLS 1.2 suites surf does not offer.
#[test]
fn tls12_suite_selected() {
    let server = answered(|sh, _| sh.suite = 0xc02f);
    refused(&server, A::ILLEGAL_PARAMETER, Violation::UnofferedSuite);
    let server = retried(|hrr| hrr.suite = 0xcca8);
    refused(&server, A::ILLEGAL_PARAMETER, Violation::UnofferedSuite);
}

/// RFC 8446 sections 4.1.3 and 4.1.4: a compression method other than 0.
#[test]
fn compression_not_null() {
    let server = answered(|sh, _| sh.compression = 1);
    refused(&server, A::ILLEGAL_PARAMETER, Violation::CompressionNotNull);
    let server = retried(|hrr| hrr.compression = 1);
    refused(&server, A::ILLEGAL_PARAMETER, Violation::CompressionNotNull);
}

fn below_tls13(legacy_version: u16, tail: Option<[u8; 8]>, suite: u16) -> Server {
    answered(|sh, _| {
        sh.legacy_version = legacy_version;
        sh.suite = suite;
        sh.extensions.retain(|(t, _)| *t != 43);
        if let Some(tail) = tail {
            sh.random[24..].copy_from_slice(&tail);
        }
    })
}

const DOWNGRD: [u8; 7] = *b"DOWNGRD";

fn downgrade(last: u8) -> [u8; 8] {
    let mut out = [0u8; 8];
    out[..7].copy_from_slice(&DOWNGRD);
    out[7] = last;
    out
}

/// Acceptance 4.2: a TLS 1.2 ServerHello with `DOWNGRD` 0x01 at the end of its random.
#[test]
fn downgrade_tls12() {
    let server = below_tls13(0x0303, Some(downgrade(1)), 0x1301);
    refused(&server, A::ILLEGAL_PARAMETER, Violation::Downgrade);
}

/// Acceptance 4.2: a TLS 1.1 ServerHello with `DOWNGRD` 0x00.
#[test]
fn downgrade_tls11() {
    let server = below_tls13(0x0302, Some(downgrade(0)), 0xc02f);
    refused(&server, A::ILLEGAL_PARAMETER, Violation::Downgrade);
}

/// Acceptance 4.1, kind (c): a TLS 1.2 ServerHello without `supported_versions` and without a
/// downgrade value, with suite 0x1301 and with 0xC02F; the same with TLS 1.1; and a TLS 1.2
/// ServerHello without any extensions: `VersionUnsupported`, alert `protocol_version`.
#[test]
fn tls12_server_hello() {
    for (version, suite, words) in [
        (0x0303, 0x1301, "TLS 1.2"),
        (0x0303, 0xc02f, "TLS 1.2"),
        (0x0302, 0x1301, "TLS 1.1"),
        (0x0302, 0xc02f, "TLS 1.1"),
    ] {
        let server = below_tls13(version, None, suite);
        refused_by(
            &server,
            &ecdsa(),
            Error::VersionUnsupported {
                detail: format!("the server selected {words}"),
            },
            A::PROTOCOL_VERSION,
        );
    }
    let mut server = Server::new(SUITE);
    let mut body = vec![3, 3];
    body.extend_from_slice(&[0x77; 32]);
    body.extend_from_slice(&v8(&[0x22; 32]));
    body.extend_from_slice(&[0xc0, 0x2f, 0]);
    server.plain(22, &msg(2, &body));
    refused_by(
        &server,
        &ecdsa(),
        Error::VersionUnsupported {
            detail: "the server selected TLS 1.2".to_string(),
        },
        A::PROTOCOL_VERSION,
    );
}

/// Without `supported_versions`, a `legacy_version` above 0x0303.
#[test]
fn server_hello_version_above_tls12() {
    let server = below_tls13(0x0304, None, 0x1301);
    refused(&server, A::ILLEGAL_PARAMETER, Violation::UnofferedVersion);
}

/// Acceptance 4.3: `supported_versions` holding 0x0303, and 0x7f1c (a draft).
#[test]
fn server_hello_version_tls12() {
    for version in [[3, 3], [0x7f, 0x1c]] {
        let server = answered(|sh, _| {
            *sh = sh.clone().replace(43, version.to_vec());
        });
        refused(&server, A::ILLEGAL_PARAMETER, Violation::UnofferedVersion);
    }
}

/// A HelloRetryRequest with another session id echo.
#[test]
fn retry_session_id_not_echoed() {
    let server = retried(|hrr| hrr.session_id = vec![0x23; 32]);
    refused(&server, A::ILLEGAL_PARAMETER, Violation::SessionIdMismatch);
}

/// Acceptance 4.10: a HelloRetryRequest with neither `key_share` nor `cookie`.
#[test]
fn retry_without_change() {
    let server = retried(|hrr| *hrr = hrr.clone().without(51).without(44));
    refused(
        &server,
        A::ILLEGAL_PARAMETER,
        Violation::HelloRetryRequestNoChange,
    );
}

/// Acceptance 4.7: a second HelloRetryRequest, refused before the transcript sees it.
#[test]
fn second_retry() {
    let mut server = after_retry();
    let hrr = server.hello_retry_request(Some(0x0017), Some(b"again"));
    server.plain(22, &hrr.encode());
    refused(
        &server,
        A::UNEXPECTED_MESSAGE,
        Violation::SecondHelloRetryRequest,
    );
    // Malformed after its random, it is still a second HelloRetryRequest.
    let mut server = after_retry();
    let mut body = vec![3, 3];
    body.extend_from_slice(&hrr_random());
    server.plain(22, &msg(2, &body));
    refused(
        &server,
        A::UNEXPECTED_MESSAGE,
        Violation::SecondHelloRetryRequest,
    );
}

/// A HelloRetryRequest with an extension the hello did not offer (and that is no cookie).
#[test]
fn retry_unoffered_extension() {
    let server = retried(|hrr| *hrr = hrr.clone().with(0x1234, vec![]));
    refused(
        &server,
        A::UNSUPPORTED_EXTENSION,
        Violation::UnofferedExtension,
    );
}

/// Acceptance 4.9: after a HelloRetryRequest that selected TLS_AES_128_GCM_SHA256, a
/// ServerHello that selects TLS_AES_256_GCM_SHA384.
#[test]
fn suite_changed_after_retry() {
    let mut server = after_retry();
    let mut sh = server.server_hello(Group::Secp256r1);
    sh.suite = 0x1302;
    server.plain(22, &sh.encode());
    refused(
        &server,
        A::ILLEGAL_PARAMETER,
        Violation::SuiteChangedAfterRetry,
    );
}

/// Acceptance 4.10: a HelloRetryRequest without `supported_versions`.
#[test]
fn retry_without_supported_versions() {
    let server = retried(|hrr| *hrr = hrr.clone().without(43));
    refused(&server, A::MISSING_EXTENSION, Violation::MissingExtension);
}

/// Acceptance 4.10: a HelloRetryRequest whose `supported_versions` holds 0x0303.
#[test]
fn retry_version_tls12() {
    let server = retried(|hrr| *hrr = hrr.clone().replace(43, vec![3, 3]));
    refused(&server, A::ILLEGAL_PARAMETER, Violation::UnofferedVersion);
}

/// Acceptance 4.8: a HelloRetryRequest for x25519, already shared.
#[test]
fn retry_group_already_shared() {
    let server = retried(|hrr| *hrr = hrr.clone().replace(51, vec![0, 0x1d]));
    refused(&server, A::ILLEGAL_PARAMETER, Violation::UnofferedGroup);
}

/// Acceptance 4.8: a HelloRetryRequest for x448, not in `supported_groups`.
#[test]
fn retry_group_not_offered() {
    let server = retried(|hrr| *hrr = hrr.clone().replace(51, vec![0, 0x1e]));
    refused(&server, A::ILLEGAL_PARAMETER, Violation::UnofferedGroup);
}

/// A HelloRetryRequest whose cookie is empty: `opaque cookie<1..2^16-1>`.
#[test]
fn retry_empty_cookie() {
    let server = retried(|hrr| *hrr = hrr.clone().replace(44, vec![0, 0]));
    refused(&server, A::DECODE_ERROR, malformed("HelloRetryRequest"));
}

/// A ServerHello share in secp256r1, for which the first hello sent no share.
#[test]
fn key_share_group_not_shared() {
    let server = answered(|sh, server| *sh = server.server_hello(Group::Secp256r1));
    refused(&server, A::ILLEGAL_PARAMETER, Violation::UnofferedKeyShare);
}

/// After a HelloRetryRequest for secp256r1, a ServerHello share in x25519.
#[test]
fn key_share_group_changed_after_retry() {
    let mut server = after_retry();
    let sh = server.server_hello(Group::X25519);
    server.plain(22, &sh.encode());
    refused(
        &server,
        A::ILLEGAL_PARAMETER,
        Violation::KeyShareGroupChanged,
    );
}

/// A ServerHello without `key_share` (no PSK was offered).
#[test]
fn server_hello_without_key_share() {
    let server = answered(|sh, _| *sh = sh.clone().without(51));
    refused(&server, A::MISSING_EXTENSION, Violation::MissingExtension);
}

/// Acceptance 6.6: after a retry to secp256r1, a share that is no point of P-256; and an x25519
/// share of 32 zero bytes.
#[test]
fn key_share_rejected() {
    let mut server = after_retry();
    let mut point = vec![4];
    point.extend_from_slice(&[0x01; 64]);
    let mut share = vec![0, 0x17];
    share.extend_from_slice(&v16(&point));
    let sh = server.server_hello(Group::Secp256r1).replace(51, share);
    server.plain(22, &sh.encode());
    refused(&server, A::ILLEGAL_PARAMETER, Violation::BadKeyShare);

    let mut share = vec![0, 0x1d];
    share.extend_from_slice(&v16(&[0; 32]));
    let server = answered(|sh, _| *sh = sh.clone().replace(51, share));
    refused(&server, A::ILLEGAL_PARAMETER, Violation::BadKeyShare);
}

/// A ServerHello with `pre_shared_key` (allowed there, not offered) and with an unknown type.
#[test]
fn server_hello_unoffered_extension() {
    for extension in [41, 0x1234] {
        let server = answered(|sh, _| *sh = sh.clone().with(extension, vec![0, 0]));
        refused(
            &server,
            A::UNSUPPORTED_EXTENSION,
            Violation::UnofferedExtension,
        );
    }
}

/// RFC 8446 section 4.2: ALPN and `server_name` (offered) in a ServerHello, `supported_versions`
/// and `key_share` in EncryptedExtensions, `early_data` in a HelloRetryRequest; checked before
/// the rule of unsolicited extensions.
#[test]
fn extension_not_allowed_here() {
    for extension in [16, 0, 42] {
        let server = answered(|sh, _| *sh = sh.clone().with(extension, vec![]));
        refused(
            &server,
            A::ILLEGAL_PARAMETER,
            Violation::ExtensionNotAllowedHere,
        );
    }
    let server = retried(|hrr| *hrr = hrr.clone().with(42, vec![]));
    refused(
        &server,
        A::ILLEGAL_PARAMETER,
        Violation::ExtensionNotAllowedHere,
    );
    for extension in [43, 51] {
        let server = flight(|s| {
            let ee = s.encrypted_extensions(&[(extension, vec![3, 4])]);
            s.push(&ee);
        });
        refused(
            &server,
            A::ILLEGAL_PARAMETER,
            Violation::ExtensionNotAllowedHere,
        );
    }
}

/// Two extensions of one type in a ServerHello, a HelloRetryRequest, EncryptedExtensions.
#[test]
fn repeated_extension() {
    let server = answered(|sh, _| *sh = sh.clone().with(43, vec![3, 4]));
    refused(&server, A::ILLEGAL_PARAMETER, Violation::RepeatedExtension);
    let server = retried(|hrr| *hrr = hrr.clone().with(44, v16(b"two")));
    refused(&server, A::ILLEGAL_PARAMETER, Violation::RepeatedExtension);
    let server = flight(|s| {
        let alpn = v16(&v8(b"http/1.1"));
        let ee = s.encrypted_extensions(&[(16, alpn.clone()), (16, alpn)]);
        s.push(&ee);
    });
    refused(&server, A::ILLEGAL_PARAMETER, Violation::RepeatedExtension);
}

/// Section 6: a ServerHello that does not parse: a session id of 33 bytes, a message one byte
/// short, a byte after its extensions, an extension longer than its block; and a
/// HelloRetryRequest one byte short.
#[test]
fn malformed_server_hello() {
    let mut server = Server::new(SUITE);
    let sh = server.server_hello(Group::X25519);
    let good = sh.encode();
    let mut long_id = sh.clone();
    long_id.session_id = vec![0x22; 33];
    let mut cases = vec![long_id.encode()];
    let short = good[4..good.len() - 1].to_vec();
    cases.push(msg(2, &short));
    let mut trailing = good[4..].to_vec();
    trailing.push(0);
    cases.push(msg(2, &trailing));
    let mut overlong = good[4..].to_vec();
    let at = 2 + 32 + 1 + 32 + 3;
    overlong[at + 1] += 1;
    cases.push(msg(2, &overlong));
    for case in cases {
        let mut s = Server::new(SUITE);
        s.plain(22, &case);
        refused(&s, A::DECODE_ERROR, malformed("ServerHello"));
    }
    let hrr = server
        .hello_retry_request(Some(0x0017), Some(b"cookie"))
        .encode();
    server.plain(22, &msg(2, &hrr[4..hrr.len() - 1]));
    refused(&server, A::DECODE_ERROR, malformed("HelloRetryRequest"));
}

// ---------------------------------------------------------------------------------------------
// EncryptedExtensions, CertificateRequest, Certificate.

/// Acceptance 4.11: EncryptedExtensions with `max_fragment_length`, not offered; ALPN `h2`;
/// ALPN with two names.
#[test]
fn encrypted_extensions_unoffered() {
    let server = flight(|s| {
        let ee = s.encrypted_extensions(&[(1, vec![1])]);
        s.push(&ee);
    });
    refused(
        &server,
        A::UNSUPPORTED_EXTENSION,
        Violation::UnofferedExtension,
    );
    let mut two = v8(b"http/1.1");
    two.extend_from_slice(&v8(b"h2"));
    for alpn in [v16(&v8(b"h2")), v16(&two)] {
        let server = flight(|s| {
            let ee = s.encrypted_extensions(&[(16, alpn)]);
            s.push(&ee);
        });
        refused(&server, A::ILLEGAL_PARAMETER, Violation::UnofferedAlpn);
    }
}

/// `server_name` in EncryptedExtensions: not empty is `Malformed`; empty but not sent (the
/// host an IP literal) is `UnofferedExtension`.
#[test]
fn server_name_not_empty() {
    let server = flight(|s| {
        let ee = s.encrypted_extensions(&[(0, vec![0, 1, 2])]);
        s.push(&ee);
    });
    refused(&server, A::DECODE_ERROR, malformed("EncryptedExtensions"));
    let mut server = Server::for_host(SUITE, "ok-ecdsa-p256", "127.0.0.1");
    server.hello();
    let ee = server.encrypted_extensions(&[(0, vec![])]);
    server.push(&ee);
    server.flush();
    let (result, written) = outcome_to("127.0.0.1", &server.input, &ecdsa());
    assert_eq!(
        result,
        Err(sent(
            A::UNSUPPORTED_EXTENSION,
            Violation::UnofferedExtension
        ))
    );
    assert_eq!(last_alert(&server, &written), Some((2, 110)));
}

/// RFC 8446 section 4.3.1: a Certificate right after the ServerHello.
#[test]
fn certificate_before_encrypted_extensions() {
    let server = flight(push_certificate);
    refused(
        &server,
        A::UNEXPECTED_MESSAGE,
        unexpected("EncryptedExtensions", 11),
    );
}

fn certificate_request(context: &[u8], extensions: &[(u16, Vec<u8>)]) -> Vec<u8> {
    let mut body = v8(context);
    body.extend_from_slice(&block(extensions));
    msg(13, &body)
}

fn good_request() -> Vec<u8> {
    certificate_request(&[], &[(13, v16(&[4, 3]))])
}

/// A CertificateRequest without `signature_algorithms`.
#[test]
fn certificate_request_without_signature_algorithms() {
    let server = flight(|s| {
        push_ee(s);
        s.push(&certificate_request(&[], &[(0x1234, vec![])]));
    });
    refused(&server, A::MISSING_EXTENSION, Violation::MissingExtension);
}

/// A CertificateRequest with a context that is not empty.
#[test]
fn certificate_request_context() {
    let server = flight(|s| {
        push_ee(s);
        s.push(&certificate_request(&[1], &[(13, v16(&[4, 3]))]));
    });
    refused(
        &server,
        A::ILLEGAL_PARAMETER,
        Violation::NonEmptyRequestContext,
    );
}

/// A CertificateRequest twice, before EncryptedExtensions, and after the Certificate.
#[test]
fn certificate_request_out_of_place() {
    let server = flight(|s| {
        push_ee(s);
        s.push(&good_request());
        s.push(&good_request());
    });
    refused(
        &server,
        A::UNEXPECTED_MESSAGE,
        unexpected("Certificate", 13),
    );
    let server = flight(|s| s.push(&good_request()));
    refused(
        &server,
        A::UNEXPECTED_MESSAGE,
        unexpected("EncryptedExtensions", 13),
    );
    let server = flight(|s| {
        push_ee(s);
        push_certificate(s);
        s.push(&good_request());
    });
    refused(
        &server,
        A::UNEXPECTED_MESSAGE,
        unexpected("CertificateVerify", 13),
    );
}

/// A Certificate whose context is not empty.
#[test]
fn certificate_context() {
    let server = flight(|s| {
        push_ee(s);
        let mut certificate = s.certificate();
        certificate[4] = 1;
        certificate.insert(5, 7);
        let len = certificate.len() - 4;
        certificate[1..4].copy_from_slice(&[(len >> 16) as u8, (len >> 8) as u8, len as u8]);
        s.push(&certificate);
    });
    refused(
        &server,
        A::ILLEGAL_PARAMETER,
        Violation::NonEmptyRequestContext,
    );
}

/// Acceptance 4.13: a Certificate with an empty list.
#[test]
fn empty_certificate() {
    let server = flight(|s| {
        push_ee(s);
        s.push(&certificate_message(&[], &[]));
    });
    refused(&server, A::DECODE_ERROR, Violation::EmptyCertificate);
}

/// More than 64 entries (item 9.2) is `TooManyCertificates`; 64 reach the verifier.
#[test]
fn too_many_certificates() {
    let entries: Vec<Vec<u8>> = (0..65u8).map(|i| vec![i; 8]).collect();
    let server = flight(|s| {
        push_ee(s);
        s.push(&certificate_message(&entries, &[]));
    });
    refused(&server, A::BAD_CERTIFICATE, Violation::TooManyCertificates);

    let mut server = Server::new(SUITE);
    server.hello();
    push_ee(&mut server);
    let mut sixty_four = chain("ok-ecdsa-p256");
    sixty_four.resize(64, vec![0x30, 0]);
    server.push(&certificate_message(&sixty_four, &[]));
    push_cv(&mut server);
    server.push_finished();
    server.flush();
    let verifier = ecdsa();
    let (result, _) = outcome(&server.input, &verifier);
    assert_eq!(result, Ok(()));
    assert_eq!(verifier.seen.borrow()[0].len(), 64);
}

/// Acceptance 4.14: a Certificate message declaring 2^24 - 1 bytes (16 MiB) is refused at its
/// header, its body not read.
#[test]
fn certificate_too_large() {
    let server = flight(|s| {
        push_ee(s);
        s.push(&[11, 0xff, 0xff, 0xff]);
    });
    refused(
        &server,
        A::ILLEGAL_PARAMETER,
        Violation::MessageTooLarge {
            declared: 0xff_ffff,
            bound: 65_535,
        },
    );
}

/// RFC 8446 section 4.4.2: an extension of a server CertificateEntry the ClientHello did not
/// offer (`status_request`, `signed_certificate_timestamp`, an unknown type).
#[test]
fn certificate_entry_unoffered_extension() {
    for extension in [5, 18, 0x1234] {
        let server = flight(|s| {
            push_ee(s);
            let chain = chain("ok-ecdsa-p256");
            s.push(&certificate_message(&chain, &[(extension, vec![])]));
        });
        refused(
            &server,
            A::UNSUPPORTED_EXTENSION,
            Violation::UnofferedExtension,
        );
    }
}

/// Acceptance 6.5: the verifier's rejections send `unknown_ca` (`NoPathToAnchor`),
/// `certificate_expired` (`Expired`) and `bad_certificate` (`BadSignature`, `Unreadable` of the
/// leaf of `leaf-v1`, `WeakAlgorithm` of the case `md5`), and the error is the rejection.
#[test]
fn certificate_alerts() {
    let leaf_v1 = chain("leaf-v1");
    let unreadable = Certificate::from_der(&leaf_v1[0]).expect_err("the reader refuses a v1 leaf");
    let time = Time::from_unix(0);
    let cases = [
        (
            CertificateRejection::NotTrusted(Check::NoPathToAnchor),
            A::UNKNOWN_CA,
        ),
        (
            CertificateRejection::Expired {
                certificate: 0,
                not_before: time,
                not_after: time,
                at: time,
            },
            A::CERTIFICATE_EXPIRED,
        ),
        (
            CertificateRejection::NotTrusted(Check::BadSignature { certificate: 1 }),
            A::BAD_CERTIFICATE,
        ),
        (
            CertificateRejection::NotTrusted(Check::Unreadable {
                certificate: 0,
                error: unreadable,
                left_out: 0,
            }),
            A::BAD_CERTIFICATE,
        ),
        (
            CertificateRejection::NotTrusted(Check::WeakAlgorithm {
                certificate: 0,
                algorithm: "md5WithRSAEncryption",
            }),
            A::BAD_CERTIFICATE,
        ),
    ];
    for (rejection, alert) in cases {
        let server = flight(|s| {
            push_ee(s);
            push_certificate(s);
            push_cv(s);
        });
        let verifier = Double::answering(Err(rejection.clone()));
        refused_by(&server, &verifier, Error::Certificate(rejection), alert);
    }
}

// ---------------------------------------------------------------------------------------------
// CertificateVerify and Finished.

/// Acceptance 4.12: CertificateVerify right after EncryptedExtensions; Finished right after
/// Certificate.
#[test]
fn skipped_messages() {
    let server = flight(|s| {
        push_ee(s);
        push_cv(s);
    });
    refused(
        &server,
        A::UNEXPECTED_MESSAGE,
        unexpected("Certificate", 15),
    );
    let server = flight(|s| {
        push_ee(s);
        push_certificate(s);
        let finished = s.finished();
        s.push(&finished);
    });
    refused(
        &server,
        A::UNEXPECTED_MESSAGE,
        unexpected("CertificateVerify", 20),
    );
}

/// A CertificateVerify with `scheme` in place of the case's.
fn cv_with_scheme(case: &str, scheme: u16) -> Server {
    let mut server = Server::with_case(SUITE, case);
    server.hello();
    push_ee(&mut server);
    push_certificate(&mut server);
    let mut cv = server.certificate_verify();
    cv[4..6].copy_from_slice(&scheme.to_be_bytes());
    server.push(&cv);
    server.flush();
    server
}

/// RFC 8446 section 4.4.3: RSA PKCS#1 (offered for certificates only), SHA-1 schemes, a scheme
/// that does not fit the key, and an unknown one.
#[test]
fn certificate_verify_scheme() {
    for (case, scheme) in [
        ("ok-rsa2048", 0x0401),
        ("ok-rsa2048", 0x0201),
        ("ok-ecdsa-p256", 0x0203),
        ("ok-ecdsa-p256", 0x0807),
        ("ok-ecdsa-p256", 0x0503),
        ("ok-ecdsa-p256", 0x0804),
        ("ok-rsa2048", 0x0403),
        ("ok-ecdsa-p256", 0xfefe),
    ] {
        let server = cv_with_scheme(case, scheme);
        refused_by(
            &server,
            &Double::accepting(case),
            sent(A::ILLEGAL_PARAMETER, Violation::UnofferedSignatureScheme),
            A::ILLEGAL_PARAMETER,
        );
    }
}

/// Acceptance 4.15: a CertificateVerify with one flipped byte, for each key kind.
#[test]
fn bad_certificate_verify() {
    for case in ["ok-ecdsa-p256", "ok-rsa2048", "ok-ed25519"] {
        let mut server = Server::with_case(SUITE, case);
        server.hello();
        push_ee(&mut server);
        push_certificate(&mut server);
        let mut cv = server.certificate_verify();
        let last = cv.len() - 1;
        cv[last] ^= 1;
        server.push(&cv);
        server.flush();
        refused_by(
            &server,
            &Double::accepting(case),
            sent(A::DECRYPT_ERROR, Violation::BadCertificateVerify),
            A::DECRYPT_ERROR,
        );
    }
}

/// Acceptance 4.15: a Finished with a wrong `verify_data`, each byte position in turn.
#[test]
fn bad_finished() {
    for at in [0, 15, 31] {
        let server = flight(|s| {
            push_ee(s);
            push_certificate(s);
            push_cv(s);
            let mut finished = s.finished();
            finished[4 + at] ^= 0x80;
            s.push(&finished);
        });
        refused(&server, A::DECRYPT_ERROR, Violation::BadFinished);
    }
}

/// A Finished whose `verify_data` is not the hash's length: one short, one long, empty.
#[test]
fn finished_wrong_length() {
    for len in [31, 33, 0] {
        let server = flight(|s| {
            push_ee(s);
            push_certificate(s);
            push_cv(s);
            let mut finished = s.finished();
            finished.resize(4 + 32, 0);
            let mut body = finished[4..].to_vec();
            body.resize(len, 0);
            s.push(&msg(20, &body));
        });
        refused(&server, A::DECODE_ERROR, malformed("Finished"));
    }
}

/// A NewSessionTicket during the handshake.
#[test]
fn new_session_ticket_during_handshake() {
    let server = flight(|s| {
        push_ee(s);
        s.push(&ticket(&[0x5a; 64], &[]));
    });
    refused(&server, A::UNEXPECTED_MESSAGE, unexpected("Certificate", 4));
}

/// RFC 8446 section 4.6.3: a KeyUpdate before the server's Finished, after the ServerHello and
/// after the CertificateVerify.
#[test]
fn key_update_before_finished() {
    let server = flight(|s| s.push(&msg(24, &[0])));
    refused(
        &server,
        A::UNEXPECTED_MESSAGE,
        unexpected("EncryptedExtensions", 24),
    );
    let server = flight(|s| {
        push_ee(s);
        push_certificate(s);
        push_cv(s);
        s.push(&msg(24, &[1]));
    });
    refused(&server, A::UNEXPECTED_MESSAGE, unexpected("Finished", 24));
}

// ---------------------------------------------------------------------------------------------
// Alerts, change_cipher_spec and the end of the stream.

/// Acceptance 4.5: kind (b), `handshake_failure` as the first record, whose `Display` holds the
/// words of item 7; kind (a), `protocol_version`; nothing written after the hello.
#[test]
fn alert_before_server_hello() {
    for (description, words) in [
        (
            A::HANDSHAKE_FAILURE,
            "the server sent handshake_failure before its ServerHello; a server limited to TLS \
             1.2 answers this way",
        ),
        (
            A::PROTOCOL_VERSION,
            "the server sent protocol_version before its ServerHello",
        ),
    ] {
        let mut server = Server::new(SUITE);
        server.plain(21, &[2, description.0]);
        let (result, written) = outcome(&server.input, &ecdsa());
        let error = Error::AlertReceived {
            description,
            before_server_hello: true,
        };
        assert_eq!(result, Err(error.clone()));
        assert_eq!(error.to_string(), words);
        assert_eq!(records_raw(&written).len(), 1, "the hello alone");
    }
    // After a HelloRetryRequest it is no longer before the ServerHello.
    let mut server = after_retry();
    server.plain(21, &[2, 40]);
    let (result, _) = outcome(&server.input, &ecdsa());
    assert_eq!(
        result,
        Err(Error::AlertReceived {
            description: A::HANDSHAKE_FAILURE,
            before_server_hello: false
        })
    );
}

/// One `change_cipher_spec` 0x01 is dropped anywhere between the first hello and the server's
/// Finished; a second, or another content, is `UnexpectedChangeCipherSpec`.
#[test]
fn change_cipher_spec_bound() {
    for at in 0..3 {
        let mut server = Server::new(SUITE);
        if at == 0 {
            server.plain(20, &[1]);
        }
        server.hello();
        if at == 1 {
            server.plain(20, &[1]);
        }
        push_ee(&mut server);
        server.flush();
        if at == 2 {
            server.plain(20, &[1]);
        }
        push_certificate(&mut server);
        push_cv(&mut server);
        server.push_finished();
        server.flush();
        let (result, _) = outcome(&server.input, &ecdsa());
        assert_eq!(result, Ok(()), "one at {at}");
    }
    let mut server = Server::new(SUITE);
    server.plain(20, &[1]);
    server.hello();
    server.plain(20, &[1]);
    refused(
        &server,
        A::UNEXPECTED_MESSAGE,
        Violation::UnexpectedChangeCipherSpec,
    );
    for content in [vec![2], vec![1, 1], vec![0]] {
        let mut server = Server::new(SUITE);
        server.hello();
        server.plain(20, &content);
        refused(
            &server,
            A::UNEXPECTED_MESSAGE,
            Violation::UnexpectedChangeCipherSpec,
        );
    }
}

/// What the client did after its handshake over `server`'s input: each read's outcome, up to
/// the first error or end of stream (at most `reads`), and the bytes it wrote after its
/// Finished during those reads.
fn after_handshake(server: &Server, reads: usize) -> (Vec<Result<Vec<u8>, Error>>, Vec<u8>) {
    let mut pipe = Pipe::with_input(server.input.clone());
    let verifier = ecdsa();
    let mut connection = connect(&mut pipe, &verifier).expect("the handshake completes");
    let start = connection.get_ref().output.len();
    let mut out = Vec::new();
    for _ in 0..reads {
        let mut buf = [0u8; 256];
        let result = connection.read_data(&mut buf).map(|n| buf[..n].to_vec());
        out.push(result);
    }
    let after = connection.get_ref().output[start..].to_vec();
    connection.into_inner();
    (out, after)
}

fn ticket(ticket: &[u8], extensions: &[(u16, Vec<u8>)]) -> Vec<u8> {
    let mut body = 7200u32.to_be_bytes().to_vec();
    body.extend_from_slice(&0x0102_0304u32.to_be_bytes());
    body.extend_from_slice(&v8(&[0]));
    body.extend_from_slice(&v16(ticket));
    body.extend_from_slice(&block(extensions));
    msg(4, &body)
}

/// Acceptance 4.18: a NewSessionTicket (lifetime 7200 s, a 64-byte ticket, an unknown
/// extension and `early_data`), then `ok`, then `close_notify`: the reads return `ok` and the
/// end of the stream, and the client writes no byte after its Finished.
#[test]
fn new_session_ticket_dropped() {
    let mut server = Server::handshake(SUITE);
    server.protected(
        22,
        &ticket(&[0x5a; 64], &[(0x1234, vec![1]), (42, vec![0; 4])]),
    );
    server.data(b"ok");
    server.alert(1, 0);
    let (reads, after) = after_handshake(&server, 3);
    assert_eq!(reads, vec![Ok(b"ok".to_vec()), Ok(vec![]), Ok(vec![])]);
    assert!(after.is_empty(), "no answer to a ticket");
}

/// A NewSessionTicket whose ticket is empty: `opaque ticket<1..2^16-1>`.
#[test]
fn new_session_ticket_empty() {
    let mut server = Server::handshake(SUITE);
    server.protected(22, &ticket(&[], &[]));
    let (reads, after) = after_handshake(&server, 1);
    assert_eq!(
        reads,
        vec![Err(sent(A::DECODE_ERROR, malformed("NewSessionTicket")))]
    );
    assert_eq!(alert_after(&server, &after), Some((2, 50)));
}

/// The alert the client wrote after its Finished, opened with its application key.
fn alert_after(server: &Server, after: &[u8]) -> Option<(u8, u8)> {
    let app = server.application.as_ref()?;
    let record = records_raw(after).pop()?;
    let inner = crate::handshake::open(SUITE, &app.client, 0, &record)?;
    (inner.last() == Some(&21)).then(|| (inner[0], inner[1]))
}

/// Acceptance 4.17: after the handshake, a CertificateRequest (no `post_handshake_auth` was
/// offered) and an EncryptedExtensions, each `unexpected_message`.
#[test]
fn post_handshake_messages() {
    for (message, got) in [
        (good_request(), 13),
        (msg(8, &block(&[])), 8),
        (msg(20, &[0; 32]), 20),
    ] {
        let mut server = Server::handshake(SUITE);
        server.protected(22, &message);
        let (reads, after) = after_handshake(&server, 1);
        assert_eq!(
            reads,
            vec![Err(sent(
                A::UNEXPECTED_MESSAGE,
                unexpected("application data", got)
            ))]
        );
        assert_eq!(alert_after(&server, &after), Some((2, 10)));
    }
}

/// Every type no row allows (`client_hello`, `end_of_early_data`, `message_hash`, an
/// unassigned value) and every known type out of its place, in each state of the handshake
/// and after it: `unexpected_message`, `UnexpectedMessage` with what was expected there.
#[test]
fn unknown_handshake_type() {
    let all: [u8; 14] = [1, 2, 4, 5, 8, 11, 13, 15, 20, 24, 254, 99, 0, 255];
    type Before = fn(&mut Server);
    let states: [(&str, Before, &[u8]); 5] = [
        ("EncryptedExtensions", |_| {}, &[8]),
        ("Certificate", push_ee, &[11, 13]),
        (
            "CertificateVerify",
            |s| {
                push_ee(s);
                push_certificate(s);
            },
            &[15],
        ),
        (
            "Finished",
            |s| {
                push_ee(s);
                push_certificate(s);
                push_cv(s);
            },
            &[20],
        ),
        ("ServerHello", |_| {}, &[2]),
    ];
    let mut checked = 0;
    for (expected, before, allowed) in states {
        for t in all.iter().copied().filter(|t| !allowed.contains(t)) {
            let server = if expected == "ServerHello" {
                let mut s = Server::new(SUITE);
                s.plain(22, &msg(t, &[0; 4]));
                s
            } else {
                flight(|s| {
                    before(s);
                    s.push(&msg(t, &[0; 4]));
                })
            };
            refused(&server, A::UNEXPECTED_MESSAGE, unexpected(expected, t));
            checked += 1;
        }
    }
    for t in all.iter().copied().filter(|t| ![4, 24].contains(t)) {
        let mut server = Server::handshake(SUITE);
        server.protected(22, &msg(t, &[0; 4]));
        let (reads, _) = after_handshake(&server, 1);
        assert_eq!(
            reads,
            vec![Err(sent(
                A::UNEXPECTED_MESSAGE,
                unexpected("application data", t)
            ))]
        );
        checked += 1;
    }
    assert_eq!(checked, 13 + 12 + 13 + 13 + 13 + 12);
}

/// Application data inside the handshake, after the keys are set, is `unexpected_message`.
#[test]
fn application_data_during_handshake() {
    let mut server = Server::new(SUITE);
    server.hello();
    server.protected(23, b"early");
    refused(
        &server,
        A::UNEXPECTED_MESSAGE,
        unexpected("EncryptedExtensions", 23),
    );
}

/// Acceptance 4.16: a `change_cipher_spec` after the server's Finished.
#[test]
fn change_cipher_spec_after_finished() {
    let mut server = Server::handshake(SUITE);
    server.plain(20, &[1]);
    let (reads, after) = after_handshake(&server, 1);
    assert_eq!(
        reads,
        vec![Err(sent(
            A::UNEXPECTED_MESSAGE,
            Violation::UnexpectedChangeCipherSpec
        ))]
    );
    assert_eq!(alert_after(&server, &after), Some((2, 10)));
    // In the server's flight after its Finished, before the client's: the same.
    let mut server = Server::new(SUITE);
    server.plain(20, &[1]);
    server.hello();
    server.flight();
    server.plain(20, &[1]);
    let (reads, _) = after_handshake(&server, 1);
    assert_eq!(
        reads,
        vec![Err(sent(
            A::UNEXPECTED_MESSAGE,
            Violation::UnexpectedChangeCipherSpec
        ))]
    );
}

/// Acceptance 6.7: after `AlertReceived`, a read and a write return the same error and nothing
/// more reaches the stream, the drop included.
#[test]
fn no_data_after_alert() {
    let mut server = Server::handshake(SUITE);
    server.alert(2, 40);
    server.data(b"after");
    let mut pipe = Pipe::with_input(server.input.clone());
    let verifier = ecdsa();
    let mut connection = connect(&mut pipe, &verifier).expect("handshake");
    let written = connection.get_ref().output.len();
    let error = Error::AlertReceived {
        description: A::HANDSHAKE_FAILURE,
        before_server_hello: false,
    };
    let mut buf = [0u8; 16];
    assert_eq!(connection.read_data(&mut buf), Err(error.clone()));
    assert_eq!(connection.read_data(&mut buf), Err(error.clone()));
    assert_eq!(connection.write_data(b"x"), Err(error.clone()));
    assert_eq!(connection.close(), Err(error));
    drop(connection);
    assert_eq!(pipe.output.len(), written, "nothing after the alert");
}

/// Acceptance 6.8: after the handshake, a warning `handshake_failure`, a warning
/// `unexpected_message`, a fatal alert of the unassigned description 0xfe: each
/// `AlertReceived`, no alert sent, the next read the same error.
#[test]
fn alert_levels() {
    for (level, description) in [(1, 40), (1, 10), (2, 0xfe)] {
        let mut server = Server::handshake(SUITE);
        server.alert(level, description);
        let (reads, after) = after_handshake(&server, 2);
        let error = Error::AlertReceived {
            description: A(description),
            before_server_hello: false,
        };
        assert_eq!(reads, vec![Err(error.clone()), Err(error)]);
        assert!(after.is_empty(), "no alert sent");
    }
}

/// Acceptance 6.8: a warning `user_canceled`, then `ok`, then `close_notify`.
#[test]
fn user_canceled_ignored() {
    let mut server = Server::handshake(SUITE);
    server.alert(1, 90);
    server.data(b"ok");
    server.alert(1, 0);
    let (reads, _) = after_handshake(&server, 2);
    assert_eq!(reads, vec![Ok(b"ok".to_vec()), Ok(vec![])]);
}

/// Section 6.1: data after `close_notify` is never read.
#[test]
fn data_after_close_notify() {
    let mut server = Server::handshake(SUITE);
    server.data(b"ok");
    server.alert(1, 0);
    server.data(b"late");
    server.plain(99, b"not even a record type");
    let (reads, after) = after_handshake(&server, 3);
    assert_eq!(reads, vec![Ok(b"ok".to_vec()), Ok(vec![]), Ok(vec![])]);
    assert!(after.is_empty());
}

/// Acceptance 4.20: application data then the end of the stream without `close_notify` is
/// `Truncated`; the end of the stream inside the handshake is `ConnectionClosed`.
#[test]
fn truncated() {
    let mut server = Server::handshake(SUITE);
    server.data(b"ok");
    let (reads, _) = after_handshake(&server, 2);
    assert_eq!(reads, vec![Ok(b"ok".to_vec()), Err(Error::Truncated)]);
    let mut server = Server::new(SUITE);
    server.hello();
    let (result, _) = outcome(&server.input, &ecdsa());
    assert_eq!(result, Err(Error::ConnectionClosed));
    // Inside a record, too.
    let mut server = Server::handshake(SUITE);
    server.data(b"cut");
    server.input.pop();
    let (reads, _) = after_handshake(&server, 1);
    assert_eq!(reads, vec![Err(Error::Truncated)]);
}

/// A stream whose reads past the server's bytes time out, as a socket with a read timeout.
struct TimingOut(Pipe);

impl Read for TimingOut {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.0.read(buf)? {
            0 if !buf.is_empty() => Err(io::Error::new(io::ErrorKind::TimedOut, "timed out")),
            n => Ok(n),
        }
    }
}

impl Write for TimingOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Acceptance 4.19: a server that sends its ServerHello and then nothing, over a stream whose
/// reads time out: `TimedOut`; and after the handshake, a read that times out.
#[test]
fn timed_out() {
    let mut server = Server::new(SUITE);
    server.hello();
    let stream = TimingOut(Pipe::with_input(server.input.clone()));
    let result = Connection::with_random(
        stream,
        crate::handshake::HOST,
        &ecdsa(),
        crate::handshake::now(),
        &surf_tls::crypto::Random::fixed_sequence(&crate::handshake::DRAWS),
    );
    assert_eq!(result.map(|_| ()).err(), Some(Error::TimedOut));
    let server = Server::handshake(SUITE);
    let stream = TimingOut(Pipe::with_input(server.input.clone()));
    let mut connection = Connection::with_random(
        stream,
        crate::handshake::HOST,
        &ecdsa(),
        crate::handshake::now(),
        &surf_tls::crypto::Random::fixed_sequence(&crate::handshake::DRAWS),
    )
    .expect("handshake");
    let mut buf = [0u8; 4];
    assert_eq!(connection.read_data(&mut buf), Err(Error::TimedOut));
    let io_error = connection.read(&mut buf).expect_err("the io form");
    assert_eq!(io_error.kind(), io::ErrorKind::TimedOut);
}

// ---------------------------------------------------------------------------------------------
// KeyUpdate.

/// A KeyUpdate whose `request_update` is 2 or 255.
#[test]
fn key_update_bad_value() {
    for value in [2, 255] {
        let mut server = Server::handshake(SUITE);
        server.protected(22, &msg(24, &[value]));
        let (reads, after) = after_handshake(&server, 1);
        assert_eq!(
            reads,
            vec![Err(sent(A::ILLEGAL_PARAMETER, Violation::BadKeyUpdate))]
        );
        assert_eq!(alert_after(&server, &after), Some((2, 47)));
    }
}

/// A KeyUpdate of other than one byte.
#[test]
fn key_update_bad_length() {
    for body in [vec![], vec![0, 0]] {
        let mut server = Server::handshake(SUITE);
        server.protected(22, &msg(24, &body));
        let (reads, _) = after_handshake(&server, 1);
        assert_eq!(
            reads,
            vec![Err(sent(A::DECODE_ERROR, malformed("KeyUpdate")))]
        );
    }
}

/// Acceptance 4.21: a KeyUpdate with `update_requested` is answered with a KeyUpdate of
/// `update_not_requested` under the old key before the next data, and the next record of
/// either side is under the new keys.
#[test]
fn key_update_requested() {
    let mut server = Server::handshake(SUITE);
    server.key_update(1);
    server.data(b"ok");
    let mut pipe = Pipe::with_input(server.input.clone());
    let verifier = ecdsa();
    let mut connection = connect(&mut pipe, &verifier).expect("handshake");
    let start = connection.get_ref().output.len();
    let mut buf = [0u8; 8];
    let n = connection
        .read_data(&mut buf)
        .expect("data under the new key");
    assert_eq!(&buf[..n], b"ok");
    let answered = connection.get_ref().output.len();
    assert_eq!(
        answered, start,
        "a read writes nothing; the answer waits for a write"
    );
    connection.write_data(b"hi").expect("write");
    connection.into_inner();
    let app = server.application.as_ref().expect("app");
    let raw = records_raw(&pipe.output[start..]);
    assert_eq!(raw.len(), 2);
    let update = crate::handshake::open(SUITE, &app.client, 0, &raw[0]).expect("old key");
    assert_eq!(update, vec![24, 0, 0, 1, 0, 22]);
    let next = key_schedule::next_traffic_secret(&app.client).expect("next");
    let data = crate::handshake::open(SUITE, &next, 0, &raw[1]).expect("new key");
    assert_eq!(data, b"hi\x17");
    // update_not_requested: no answer.
    let mut server = Server::handshake(SUITE);
    server.key_update(0);
    server.data(b"ok");
    let (reads, after) = after_handshake(&server, 1);
    assert_eq!(reads, vec![Ok(b"ok".to_vec())]);
    assert!(after.is_empty());
}

/// Acceptance 6.9: after a KeyUpdate of `update_not_requested`, a record under the old server
/// key is `bad_record_mac`; the same record under the new key opens.
#[test]
fn old_key_after_key_update() {
    let mut server = Server::handshake(SUITE);
    let old = server.application.as_ref().expect("app").server.clone();
    server.key_update(0);
    server
        .input
        .extend_from_slice(&seal(SUITE, &old, 1, b"ok\x17"));
    let (reads, after) = after_handshake(&server, 1);
    assert_eq!(
        reads,
        vec![Err(sent(A::BAD_RECORD_MAC, Violation::BadRecordMac))]
    );
    assert_eq!(alert_after(&server, &after), Some((2, 20)));

    let mut server = Server::handshake(SUITE);
    server.key_update(0);
    server.data(b"ok");
    let (reads, _) = after_handshake(&server, 1);
    assert_eq!(reads, vec![Ok(b"ok".to_vec())]);
}

// ---------------------------------------------------------------------------------------------
// Length fields at their edges.

/// One case of `length_edges`: a message whose length field is set off by `delta` from the
/// true length, and the message the refusal names.
struct Edge {
    name: &'static str,
    /// Where the message is read.
    state: State,
    /// The message, from the server in that state.
    build: fn(&Server, i32) -> Vec<u8>,
    message: &'static str,
}

#[derive(Clone, Copy)]
enum State {
    Hello,
    Retry,
    EncryptedExtensions,
    Certificate,
    CertificateVerify,
    Finished,
    After,
}

/// Sets the big-endian length of `width` bytes at `at` in `bytes` to its value plus `delta`.
fn shift(bytes: &mut [u8], at: usize, width: usize, delta: i32) {
    let mut value: i64 = 0;
    for b in &bytes[at..at + width] {
        value = value << 8 | i64::from(*b);
    }
    let value = value + i64::from(delta);
    for i in 0..width {
        bytes[at + width - 1 - i] = (value >> (8 * i)) as u8;
    }
}

fn sh_edge(server: &Server, at: fn(&Sh) -> (usize, usize), delta: i32) -> Vec<u8> {
    let sh = server.server_hello(Group::X25519);
    let (offset, width) = at(&sh);
    let mut m = sh.encode();
    shift(&mut m, 4 + offset, width, delta);
    m
}

fn edges() -> Vec<Edge> {
    vec![
        Edge {
            name: "ServerHello session id",
            state: State::Hello,
            build: |s, d| sh_edge(s, |_| (34, 1), d),
            message: "ServerHello",
        },
        Edge {
            name: "ServerHello extension block",
            state: State::Hello,
            build: |s, d| sh_edge(s, |sh| (38 + sh.session_id.len(), 2), d),
            message: "ServerHello",
        },
        Edge {
            name: "ServerHello supported_versions data",
            state: State::Hello,
            build: |s, d| sh_edge(s, |sh| (38 + sh.session_id.len() + 4, 2), d),
            message: "ServerHello",
        },
        Edge {
            name: "ServerHello key_share key",
            state: State::Hello,
            build: |s, d| {
                let mut sh = s.server_hello(Group::X25519);
                let share = sh.extensions[1].1.clone();
                let mut share = share;
                shift(&mut share, 2, 2, d);
                sh.extensions[1].1 = share;
                sh.encode()
            },
            message: "ServerHello",
        },
        Edge {
            name: "HelloRetryRequest cookie",
            state: State::Retry,
            build: |s, d| {
                let mut hrr = s.hello_retry_request(Some(0x0017), Some(b"cookie"));
                let mut cookie = hrr.extensions[2].1.clone();
                shift(&mut cookie, 0, 2, d);
                hrr.extensions[2].1 = cookie;
                hrr.encode()
            },
            message: "HelloRetryRequest",
        },
        Edge {
            name: "HelloRetryRequest selected_group",
            state: State::Retry,
            build: |s, d| {
                let mut hrr = s.hello_retry_request(Some(0x0017), None);
                let mut group = hrr.extensions[1].1.clone();
                group.resize((2 + d) as usize, 0x17);
                hrr.extensions[1].1 = group;
                hrr.encode()
            },
            message: "HelloRetryRequest",
        },
        Edge {
            name: "EncryptedExtensions block",
            state: State::EncryptedExtensions,
            build: |s, d| {
                let mut m = s.encrypted_extensions(&[(16, v16(&v8(b"http/1.1")))]);
                shift(&mut m, 4, 2, d);
                m
            },
            message: "EncryptedExtensions",
        },
        Edge {
            name: "EncryptedExtensions ALPN list",
            state: State::EncryptedExtensions,
            build: |s, d| {
                let mut alpn = v16(&v8(b"http/1.1"));
                shift(&mut alpn, 0, 2, d);
                s.encrypted_extensions(&[(16, alpn)])
            },
            message: "EncryptedExtensions",
        },
        Edge {
            name: "EncryptedExtensions ALPN name",
            state: State::EncryptedExtensions,
            build: |s, d| {
                let mut alpn = v16(&v8(b"http/1.1"));
                shift(&mut alpn, 2, 1, d);
                s.encrypted_extensions(&[(16, alpn)])
            },
            message: "EncryptedExtensions",
        },
        Edge {
            name: "Certificate list",
            state: State::Certificate,
            build: |s, d| {
                let mut m = s.certificate();
                shift(&mut m, 5, 3, d);
                m
            },
            message: "Certificate",
        },
        Edge {
            name: "Certificate cert_data",
            state: State::Certificate,
            build: |s, d| {
                let mut m = s.certificate();
                shift(&mut m, 8, 3, d);
                m
            },
            message: "Certificate",
        },
        Edge {
            name: "CertificateVerify signature",
            state: State::CertificateVerify,
            build: |s, d| {
                let mut m = s.certificate_verify();
                shift(&mut m, 6, 2, d);
                m
            },
            message: "CertificateVerify",
        },
        Edge {
            name: "NewSessionTicket ticket",
            state: State::After,
            build: |_, d| {
                let mut m = ticket(&[0x5a; 64], &[]);
                shift(&mut m, 4 + 4 + 4 + 2, 2, d);
                m
            },
            message: "NewSessionTicket",
        },
        Edge {
            name: "NewSessionTicket nonce",
            state: State::After,
            build: |_, d| {
                let mut m = ticket(&[0x5a; 64], &[]);
                shift(&mut m, 4 + 8, 1, d);
                m
            },
            message: "NewSessionTicket",
        },
        Edge {
            name: "NewSessionTicket extensions",
            state: State::After,
            build: |_, d| {
                let mut m = ticket(&[0x5a; 64], &[(0x1234, vec![1])]);
                let at = m.len() - 7;
                shift(&mut m, at, 2, d);
                m
            },
            message: "NewSessionTicket",
        },
        Edge {
            name: "Finished verify_data",
            state: State::Finished,
            build: |s, d| {
                let mut f = s.finished();
                f.resize((4 + 32 + d) as usize, 0);
                let body = f[4..].to_vec();
                msg(20, &body)
            },
            message: "Finished",
        },
    ]
}

/// A server in `state`: nothing sent for a hello, the handshake done for `After`, else the
/// ServerHello and the messages before the one read there pushed.
fn server_in(state: State) -> Server {
    let mut server = Server::new(SUITE);
    match state {
        State::Hello | State::Retry => return server,
        State::After => return Server::handshake(SUITE),
        _ => {}
    }
    server.hello();
    if !matches!(state, State::EncryptedExtensions) {
        push_ee(&mut server);
    }
    if matches!(state, State::CertificateVerify | State::Finished) {
        push_certificate(&mut server);
    }
    if matches!(state, State::Finished) {
        push_cv(&mut server);
    }
    server
}

/// The client's end over `server` in `state` given `message`.
fn ending(mut server: Server, state: State, message: &[u8]) -> Result<(), Error> {
    match state {
        State::Hello | State::Retry => server.plain(22, message),
        State::After => server.protected(22, message),
        _ => {
            server.push(message);
            server.flush();
        }
    }
    match state {
        State::After => after_handshake(&server, 1).0.remove(0).map(|_| ()),
        _ => outcome(&server.input, &ecdsa()).0,
    }
}

/// Every length field of every message the server sends, one less and one more than the
/// bytes it covers: `decode_error`, `Malformed` naming the message (the Finished's
/// `verify_data` has no field of its own: its length is the message's; the context of a
/// Certificate is `certificate_context`). With the true length the same message passes the
/// point of the refusal.
#[test]
fn length_edges() {
    let mut checked = 0;
    for edge in edges() {
        for delta in [-1, 1] {
            let server = server_in(edge.state);
            let message = (edge.build)(&server, delta);
            let expected = sent(A::DECODE_ERROR, malformed(edge.message));
            let result = ending(server, edge.state, &message);
            assert_eq!(result, Err(expected), "{} {delta:+}", edge.name);
            checked += 1;
        }
        let server = server_in(edge.state);
        let message = (edge.build)(&server, 0);
        let result = ending(server, edge.state, &message);
        assert_ne!(
            result,
            Err(sent(A::DECODE_ERROR, malformed(edge.message))),
            "{} as it is",
            edge.name
        );
    }
    assert_eq!(checked, 32);
}

/// The top byte of a 3-byte length counts 65,536 each: a `cert_data` declaring 0x010110
/// (65,808 bytes) with 272 bytes present is `decode_error`, `Malformed`, though its two low
/// bytes would fit.
#[test]
fn uint24_top_byte() {
    let server = flight(|s| {
        push_ee(s);
        let mut list = vec![0x01, 0x01, 0x10];
        list.extend_from_slice(&[0x30; 272]);
        list.extend_from_slice(&[0, 0]);
        let mut body = vec![0];
        body.extend_from_slice(&crate::handshake::v24(&list));
        s.push(&msg(11, &body));
    });
    refused(&server, A::DECODE_ERROR, malformed("Certificate"));
}

// ---------------------------------------------------------------------------------------------
// What a server can make the client hold.

/// The most a handshake makes the client hold: with a Certificate message of 65,535 bytes,
/// the bound of item 9.1, the client's peak of live heap stays under 512 KiB; the message
/// declared one byte longer is refused at its header.
#[test]
fn memory_bound_certificate() {
    let build = |extra: usize| {
        let mut server = Server::new(SUITE);
        server.hello();
        push_ee(&mut server);
        let mut entries = chain("ok-ecdsa-p256");
        let used: usize = entries.iter().map(|e| e.len() + 5).sum::<usize>() + 4 + 5;
        entries.push(vec![0x30; 65_535 + extra - used]);
        let certificate = certificate_message(&entries, &[]);
        assert_eq!(certificate.len(), 4 + 65_535 + extra);
        server.push(&certificate);
        push_cv(&mut server);
        server.push_finished();
        server.flush();
        server
    };
    let server = build(0);
    let (result, peak) = peak_while(|| outcome(&server.input, &ecdsa()).0);
    assert_eq!(result, Ok(()));
    assert!(peak < 512 * 1024, "peak {peak} bytes");
    let server = build(1);
    let (result, _) = outcome(&server.input, &ecdsa());
    assert_eq!(
        result,
        Err(sent(
            A::ILLEGAL_PARAMETER,
            Violation::MessageTooLarge {
                declared: 65_536,
                bound: 65_535
            }
        ))
    );
}

/// The client holds nothing of the tickets it drops, nor of the extensions it ignores: the peak
/// of live heap after the handshake is the same for 500 and 1,000 tickets (each followed by one
/// byte of data, under the bound of records without data), and for a
/// CertificateRequest of 1,000 and of 2,000 unknown extensions in a message of one size it
/// differs by less than one byte per extension added.
#[test]
fn memory_bound_tickets_and_extensions() {
    let peak_for = |tickets: usize| {
        let mut server = Server::handshake(SUITE);
        // One byte of data after each ticket: the bound of records without data stays far.
        for _ in 0..tickets {
            server.protected(22, &ticket(&[0x5a; 64], &[(0x1234, vec![1])]));
            server.data(b"x");
        }
        let mut pipe = Pipe::with_input(server.input.clone());
        let verifier = ecdsa();
        let mut connection = connect(&mut pipe, &verifier).expect("handshake");
        let (read, peak) = peak_while(|| {
            let mut read = 0;
            let mut buf = [0u8; 4];
            while let Ok(n @ 1..) = connection.read_data(&mut buf) {
                read += n;
            }
            read
        });
        assert_eq!(read, tickets);
        connection.into_inner();
        peak
    };
    let (n, two_n) = (peak_for(500), peak_for(1_000));
    assert_eq!(n, two_n, "tickets held: {n} and {two_n} bytes");

    let peak_extensions = |count: u16, data: usize| {
        let mut server = Server::new(SUITE);
        server.hello();
        push_ee(&mut server);
        let mut extensions = vec![(13, v16(&[4, 3]))];
        extensions.extend((0..count).map(|i| (0x4000 + i, vec![0x5a; data])));
        let request = certificate_request(&[], &extensions);
        server.push(&request);
        push_certificate(&mut server);
        push_cv(&mut server);
        server.push_finished();
        server.flush();
        let (result, peak) = peak_while(|| outcome(&server.input, &ecdsa()).0);
        assert_eq!(result, Ok(()));
        (peak, request.len())
    };
    // 1,000 extensions of 4 data bytes and 2,000 empty ones: messages of one size.
    let ((n, len_n), (two_n, len_2n)) = (peak_extensions(1_000, 4), peak_extensions(2_000, 0));
    assert_eq!(len_n, len_2n);
    // Collected, 1,000 more extensions would cost at least 1,000 bytes more.
    assert!(
        n.abs_diff(two_n) < 1_000,
        "extensions held: {n} and {two_n} bytes"
    );
}

// ---------------------------------------------------------------------------------------------
// KeyUpdate answers, records without data, the cookie bound.

/// RFC 8446 section 4.6.3: a server that sends 1,000 KeyUpdates of `update_requested` (each
/// followed by one byte of data) and reads nothing makes the client write no byte inside
/// `read_data`; the next write sends one KeyUpdate of `update_not_requested` under the old key
/// and the data under the next one: two records.
#[test]
fn key_update_flood_writes_nothing_in_read() {
    let mut server = Server::handshake(SUITE);
    for _ in 0..1_000 {
        server.key_update(1);
        server.data(b"x");
    }
    let mut pipe = Pipe::with_input(server.input.clone());
    let verifier = ecdsa();
    let mut connection = connect(&mut pipe, &verifier).expect("handshake");
    let start = connection.get_ref().output.len();
    let mut read = 0;
    let mut buf = [0u8; 64];
    while read < 1_000 {
        read += connection.read_data(&mut buf).expect("data");
    }
    assert_eq!(read, 1_000);
    assert_eq!(
        connection.get_ref().output.len(),
        start,
        "nothing written by the reads"
    );
    connection.write_data(b"hi").expect("write");
    connection.into_inner();
    let app = server.application.as_ref().expect("app");
    let raw = records_raw(&pipe.output[start..]);
    assert_eq!(raw.len(), 2, "one KeyUpdate and one record of data");
    let update = crate::handshake::open(SUITE, &app.client, 0, &raw[0]).expect("old key");
    assert_eq!(update, vec![24, 0, 0, 1, 0, 22]);
    let next = key_schedule::next_traffic_secret(&app.client).expect("next");
    let data = crate::handshake::open(SUITE, &next, 0, &raw[1]).expect("new key");
    assert_eq!(data, b"hi\x17");
}

/// `close` with an answer owed: one `close_notify` under the current (old) write key, no
/// KeyUpdate.
#[test]
fn close_with_key_update_owed() {
    let mut server = Server::handshake(SUITE);
    server.key_update(1);
    server.data(b"ok");
    let mut pipe = Pipe::with_input(server.input.clone());
    let verifier = ecdsa();
    let mut connection = connect(&mut pipe, &verifier).expect("handshake");
    let start = connection.get_ref().output.len();
    let mut buf = [0u8; 8];
    assert_eq!(connection.read_data(&mut buf), Ok(2));
    connection.close().expect("close");
    connection.into_inner();
    let app = server.application.as_ref().expect("app");
    let raw = records_raw(&pipe.output[start..]);
    assert_eq!(raw.len(), 1, "close_notify alone");
    let alert = crate::handshake::open(SUITE, &app.client, 0, &raw[0]).expect("old key");
    assert_eq!(alert, vec![1, 0, 21]);
}

/// One record of the kinds that give the caller no data, by `i`: a NewSessionTicket, empty
/// application data, a warning `user_canceled`, a KeyUpdate of `update_not_requested`.
fn idle_record(server: &mut Server, i: usize) {
    match i % 4 {
        0 => server.protected(22, &ticket(&[0x5a; 16], &[])),
        1 => server.protected(23, &[]),
        2 => server.alert(1, 90),
        _ => server.key_update(0),
    }
}

/// The bound of records in a row without application data (`MAX_RECORDS_WITHOUT_DATA`, 32):
/// 32 of the four kinds and then data are read; 33 end with `unexpected_message`,
/// `TooManyRecordsWithoutData { bound: 32 }`; a byte of data starts the count again; 40
/// tickets in one record count as one record.
#[test]
fn records_without_data_bound() {
    assert_eq!(surf_tls::connection::MAX_RECORDS_WITHOUT_DATA, 32);
    let reads = |server: &Server, count: usize| after_handshake(server, count).0;
    let mut server = Server::handshake(SUITE);
    (0..32).for_each(|i| idle_record(&mut server, i));
    server.data(b"ok");
    assert_eq!(reads(&server, 1), vec![Ok(b"ok".to_vec())]);

    let mut server = Server::handshake(SUITE);
    (0..33).for_each(|i| idle_record(&mut server, i));
    server.data(b"ok");
    let (reads33, after) = after_handshake(&server, 1);
    assert_eq!(
        reads33,
        vec![Err(sent(
            A::UNEXPECTED_MESSAGE,
            Violation::TooManyRecordsWithoutData { bound: 32 }
        ))]
    );
    assert!(!after.is_empty(), "the alert is written");

    let mut server = Server::handshake(SUITE);
    (0..32).for_each(|i| idle_record(&mut server, i));
    server.data(b"a");
    (0..32).for_each(|i| idle_record(&mut server, i));
    server.data(b"b");
    assert_eq!(
        reads(&server, 2),
        vec![Ok(b"a".to_vec()), Ok(b"b".to_vec())]
    );

    let mut server = Server::handshake(SUITE);
    let mut tickets = Vec::new();
    for _ in 0..40 {
        tickets.extend_from_slice(&ticket(&[0x5a; 16], &[]));
    }
    server.protected(22, &tickets);
    (0..31).for_each(|i| idle_record(&mut server, i));
    server.data(b"ok");
    assert_eq!(reads(&server, 1), vec![Ok(b"ok".to_vec())]);
}

/// `messages::MAX_COOKIE`: 65,535 - 413 (the other extensions of a second ClientHello at
/// their largest) - 6.
const MAX_COOKIE: usize = 65_116;

/// A server for a host of 253 bytes (the longest DNS name a connection takes, so the longest
/// `server_name` sent; `messages::MAX_SERVER_NAME`, 255, bounds it from above) whose first
/// answer is a HelloRetryRequest for secp256r1 with a cookie of `len` bytes, in records of 2^14
/// bytes.
fn long_cookie(len: usize) -> (Server, String, Vec<u8>) {
    let host = format!("{0}.{0}.{0}.{1}", "a".repeat(63), "a".repeat(61));
    assert_eq!(host.len(), 253);
    let mut server = Server::for_host(SUITE, "ok-ecdsa-p256", &host);
    let cookie = vec![0x5a; len];
    let hrr = server
        .hello_retry_request(Some(0x0017), Some(&cookie))
        .encode();
    assert!(hrr.len() - 4 <= 65_535, "inside item 9.1");
    for fragment in hrr.chunks(16_384) {
        server.plain(22, fragment);
    }
    (server, host, cookie)
}

/// A HelloRetryRequest cookie at the bound fits the second ClientHello of the longest host,
/// which echoes it; one byte over is `illegal_parameter`, `MessageTooLarge` naming the bound,
/// and no second hello is written.
#[test]
fn retry_cookie_bound() {
    let (server, host, cookie) = long_cookie(MAX_COOKIE);
    let (result, written) = outcome_to(&host, &server.input, &ecdsa());
    assert_eq!(
        result,
        Err(Error::ConnectionClosed),
        "waits for the ServerHello"
    );
    let stream: Vec<u8> = crate::handshake::records(&written)
        .into_iter()
        .filter(|(t, _)| *t == 22)
        .flat_map(|(_, body)| body)
        .collect();
    let first =
        4 + (usize::from(stream[1]) << 16 | usize::from(stream[2]) << 8 | usize::from(stream[3]));
    let second = crate::handshake::Hello::parse(&stream[first..]);
    assert_eq!(
        second.extension(44),
        Some(&v16(&cookie)[..]),
        "the cookie echoed"
    );

    let (server, host, _) = long_cookie(MAX_COOKIE + 1);
    let (result, written) = outcome_to(&host, &server.input, &ecdsa());
    assert_eq!(
        result,
        Err(sent(
            A::ILLEGAL_PARAMETER,
            Violation::MessageTooLarge {
                declared: MAX_COOKIE + 1,
                bound: MAX_COOKIE
            }
        ))
    );
    let hellos = crate::handshake::records(&written)
        .into_iter()
        .filter(|(t, _)| *t == 22)
        .count();
    assert_eq!(hellos, 1, "the first hello alone");
    assert_eq!(last_alert(&server, &written), Some((2, 47)));
}

// ---------------------------------------------------------------------------------------------
// A seeded run.

/// xorshift64*, enough for a reproducible run.
pub struct Seeded(pub u64);

impl Seeded {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Changes `bytes` at one to three places: a byte flipped, set to 0 or 0xff, or one cut out.
pub fn mutate(bytes: &mut Vec<u8>, random: &mut Seeded) {
    for _ in 0..=random.below(3) {
        if bytes.is_empty() {
            return;
        }
        let at = random.below(bytes.len());
        match random.below(4) {
            0 => bytes[at] ^= 1 << random.below(8),
            1 => bytes[at] = 0,
            2 => bytes[at] = 0xff,
            _ => {
                bytes.remove(at);
            }
        }
    }
}

/// Whether `error` is an end the story allows a hostile server: an alert sent or received, a
/// certificate rejected, a version refused, or the end of the stream.
pub fn clean_end(error: &Error) -> bool {
    matches!(
        error,
        Error::AlertSent { .. }
            | Error::AlertReceived { .. }
            | Error::Certificate(_)
            | Error::VersionUnsupported { .. }
            | Error::ConnectionClosed
            | Error::Truncated
    )
}

/// The seed of the mutation runs.
pub const SEED: u64 = 0x7e57_5eed_0f11_9475;

/// 300 runs from seed [`SEED`]: one plaintext message of the server's flight (ServerHello,
/// EncryptedExtensions, Certificate, CertificateVerify, Finished, a ticket) changed at one to
/// three places and written under the right keys. No panic; each run ends with an alert, a
/// clean close or a complete handshake; the peak of live heap stays under 512 KiB.
#[test]
fn mutated_messages_end_cleanly() {
    let mut random = Seeded(SEED);
    let mut ends = std::collections::BTreeMap::new();
    let mut max_peak = 0;
    for run in 0..300 {
        let target = random.below(6);
        let mut server = Server::new(SUITE);
        let mut sh = server.server_hello(Group::X25519).encode();
        if target == 0 {
            mutate(&mut sh, &mut random);
            server.plain(22, &sh);
        } else {
            server.hello();
            let mut messages = [
                server.encrypted_extensions(&[(16, v16(&v8(b"http/1.1")))]),
                server.certificate(),
            ];
            for (i, m) in messages.iter_mut().enumerate() {
                if target == i + 1 {
                    mutate(m, &mut random);
                }
                server.push(m);
            }
            let mut cv = server.certificate_verify();
            if target == 3 {
                mutate(&mut cv, &mut random);
            }
            server.push(&cv);
            if target == 4 {
                let mut finished = server.finished();
                mutate(&mut finished, &mut random);
                server.push(&finished);
            } else {
                server.push_finished();
            }
            server.flush();
            if target == 5 {
                let mut t = ticket(&[0x5a; 64], &[(0x1234, vec![1])]);
                mutate(&mut t, &mut random);
                server.protected(22, &t);
                server.data(b"ok");
            }
        }
        let ((result, read), peak) = peak_while(|| {
            let mut pipe = Pipe::with_input(server.input.clone());
            let verifier = ecdsa();
            match connect(&mut pipe, &verifier) {
                Ok(mut connection) => {
                    let mut buf = [0u8; 8];
                    let read = connection.read_data(&mut buf).map(|_| ());
                    connection.into_inner();
                    (Ok(()), read)
                }
                Err(e) => (Err(e), Ok(())),
            }
        });
        max_peak = max_peak.max(peak);
        for outcome in [result, read] {
            let key = match outcome {
                Ok(()) => "complete".to_string(),
                Err(e) => {
                    assert!(clean_end(&e), "run {run}: {e:?}");
                    format!("{e:?}")
                        .split([' ', '{', '('])
                        .next()
                        .unwrap_or("")
                        .to_string()
                }
            };
            *ends.entry(key).or_insert(0usize) += 1;
        }
    }
    println!("mutated messages, seed {SEED:#x}, 300 runs: {ends:?}, peak {max_peak} bytes");
    assert!(max_peak < 512 * 1024);
    assert!(ends.len() >= 3, "{ends:?}");
}
