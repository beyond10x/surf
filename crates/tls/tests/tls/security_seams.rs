//! Security review of the seams of `surf-tls` (docs/design/tls.md items 6 to 9 and 13): the
//! handshake and the verifier, the host name, the time, the signature schemes, the alerts, the
//! anchors, the costs across the seam, the state after a fatal error and the secrets. Every
//! cost is a count of bytes, never a wall time.

#![forbid(unsafe_code)]

use std::fs;

use surf_tls::anchors::{AnchorError, Anchors, EXTRA_BYTES_MAX};
use surf_tls::connection::Connection;
use surf_tls::crypto::{CryptoError, Random, Scheme, SignatureVerifier, SigningKey};
use surf_tls::der::Time;
use surf_tls::error::{AlertDescription as A, Error, Violation};
use surf_tls::key_schedule::CipherSuite;
use surf_tls::names::{self, Host, HostReason};
use surf_tls::path::PathVerifier;
use surf_tls::pem;
use surf_tls::verify::{CertificateRejection, Check, ServerCertVerifier};

use crate::alloc::peak_while;
use crate::handshake::{
    DRAWS, Double, HOST, Hello, Server, certificate_message, certs_dir, chain, client_written_to,
    connect, connect_to, hellos_in, last_alert, msg, now, outcome, outcome_to, sent, to_hex, v16,
};
use crate::rfc8448::Pipe;
use crate::x509_parse::{seq, tlv};

const SUITE: CipherSuite = CipherSuite::Aes128GcmSha256;

// ---------------------------------------------------------------------------------------------
// Helpers.

fn root_of(case: &str) -> Vec<u8> {
    let text = fs::read(certs_dir().join(case).join("root.pem")).expect("root.pem");
    pem::parse(&text).expect("PEM").remove(0).data
}

/// The shipped verifier whose one anchor is the test root of `case`.
fn path_verifier(case: &str) -> PathVerifier {
    let mut verifier = PathVerifier::new(Vec::new());
    verifier
        .add_root(&root_of(case))
        .expect("the test root is an anchor");
    verifier
}

/// A signature verifies when its bytes are the signer's public key.
#[derive(Clone, Copy, Debug, Default)]
struct KeyEcho;

impl SignatureVerifier for KeyEcho {
    fn verify(
        &self,
        _scheme: Scheme,
        public_key: &[u8],
        _message: &[u8],
        signature: &[u8],
    ) -> Result<(), CryptoError> {
        if signature == public_key {
            Ok(())
        } else {
            Err(CryptoError::BadSignature)
        }
    }
}

const OID_CN: &[u8] = &[0x55, 0x04, 0x03];
const OID_ED25519: &[u8] = &[0x2B, 0x65, 0x70];
const OID_BASIC: &[u8] = &[0x55, 0x1D, 0x13];
const OID_KEY_USAGE: &[u8] = &[0x55, 0x1D, 0x0F];
const OID_SAN: &[u8] = &[0x55, 0x1D, 0x11];
const KU_DIGITAL_SIGNATURE: &[u8] = &[0x07, 0x80];

fn name(cn: &str) -> Vec<u8> {
    seq(&[&tlv(
        0x31,
        &seq(&[&tlv(0x06, OID_CN), &tlv(0x0C, cn.as_bytes())]),
    )])
}

fn extension(oid: &[u8], critical: bool, value: &[u8]) -> Vec<u8> {
    if critical {
        seq(&[&tlv(0x06, oid), &[0x01, 0x01, 0xFF], &tlv(0x04, value)])
    } else {
        seq(&[&tlv(0x06, oid), &tlv(0x04, value)])
    }
}

/// The 32 bytes of the Ed25519 key numbered `k`.
fn key(k: u16) -> Vec<u8> {
    let mut out = vec![0x5A; 32];
    out[..2].copy_from_slice(&k.to_be_bytes());
    out
}

/// One certificate to build: Ed25519 keys, signatures that [`KeyEcho`] reads.
#[derive(Clone)]
struct Built {
    subject: String,
    issuer: String,
    key: u16,
    signer: u16,
    ca: bool,
    key_usage: Option<&'static [u8]>,
    sans: Vec<Vec<u8>>,
    not_before: Vec<u8>,
    not_after: Vec<u8>,
    unknown_critical: Option<Vec<u8>>,
    serial: u8,
}

impl Built {
    /// A self-signed CA valid from 2025 to 2125.
    fn ca(subject: &str, key: u16) -> Built {
        Built {
            subject: subject.into(),
            issuer: subject.into(),
            key,
            signer: key,
            ca: true,
            key_usage: None,
            sans: Vec::new(),
            not_before: tlv(0x17, b"250101000000Z"),
            not_after: tlv(0x18, b"21250101000000Z"),
            unknown_critical: None,
            serial: 1,
        }
    }

    /// A leaf for `host` under `issuer` signed by the key `signer`.
    fn leaf(host: &str, key: u16, issuer: &str, signer: u16) -> Built {
        Built {
            subject: format!("leaf {host}"),
            issuer: issuer.into(),
            signer,
            ca: false,
            sans: vec![tlv(0x82, host.as_bytes())],
            ..Built::ca("unused", key)
        }
    }

    fn der(&self) -> Vec<u8> {
        assert!(self.serial < 0x80, "a one-byte serial");
        let alg = seq(&[&tlv(0x06, OID_ED25519)]);
        let spki = seq(&[
            &seq(&[&tlv(0x06, OID_ED25519)]),
            &tlv(0x03, &[&[0x00][..], &key(self.key)].concat()),
        ]);
        let mut extensions = Vec::new();
        if self.ca {
            extensions.push(extension(OID_BASIC, true, &tlv(0x30, &[0x01, 0x01, 0xFF])));
        }
        if let Some(bits) = self.key_usage {
            extensions.push(extension(OID_KEY_USAGE, true, &tlv(0x03, bits)));
        }
        if !self.sans.is_empty() {
            let refs: Vec<&[u8]> = self.sans.iter().map(Vec::as_slice).collect();
            extensions.push(extension(OID_SAN, false, &seq(&refs)));
        }
        if let Some(oid) = &self.unknown_critical {
            extensions.push(extension(oid, true, &[0x05, 0x00]));
        }
        let mut tbs: Vec<Vec<u8>> = vec![
            tlv(0xA0, &tlv(0x02, &[0x02])),
            tlv(0x02, &[self.serial]),
            alg.clone(),
            name(&self.issuer),
            seq(&[&self.not_before, &self.not_after]),
            name(&self.subject),
            spki,
        ];
        if !extensions.is_empty() {
            let refs: Vec<&[u8]> = extensions.iter().map(Vec::as_slice).collect();
            tbs.push(tlv(0xA3, &seq(&refs)));
        }
        let refs: Vec<&[u8]> = tbs.iter().map(Vec::as_slice).collect();
        let signature = [&[0x00][..], &key(self.signer)].concat();
        seq(&[&seq(&refs), &alg, &tlv(0x03, &signature)])
    }
}

/// `der` as one PEM block labelled CERTIFICATE.
fn pem_of(der: &[u8]) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = Vec::new();
    for chunk in der.chunks(3) {
        let byte = |i: usize| u32::from(chunk.get(i).copied().unwrap_or(0));
        let v = byte(0) << 16 | byte(1) << 8 | byte(2);
        for i in 0..4 {
            if i <= chunk.len() {
                text.push(ALPHABET[(v >> (18 - 6 * i)) as usize & 63]);
            } else {
                text.push(b'=');
            }
        }
    }
    let mut out = b"-----BEGIN CERTIFICATE-----\n".to_vec();
    for line in text.chunks(64) {
        out.extend_from_slice(line);
        out.push(b'\n');
    }
    out.extend_from_slice(b"-----END CERTIFICATE-----\n");
    out
}

/// The `server_name` of the first ClientHello written for `host`: `None` when nothing was
/// written, `Some(None)` for a hello without the extension.
fn sni_of(host: &str) -> Option<Option<Vec<u8>>> {
    let written = client_written_to(host, &[]);
    let hellos = hellos_in(&written);
    let hello = Hello::parse(hellos.first()?);
    Some(hello.extension(0).map(|data| data[5..].to_vec()))
}

/// A host as a failure line shows it: escaped, cut after 24 characters.
fn shown(host: &str) -> String {
    let escaped: String = host.escape_debug().collect();
    if escaped.len() > 24 {
        format!("{}... ({} bytes)", &escaped[..24], host.len())
    } else {
        escaped
    }
}

// ---------------------------------------------------------------------------------------------
// 1. The verifier and CertificateVerify.

/// The key CertificateVerify is checked with is the key of the first certificate, the one the
/// verifier judged: a server that appends another certificate (the P-384 leaf; every P-256
/// leaf of the test certificates but `ok-wildcard`'s holds the same key, see
/// `cv_by_the_shared_p256_key_is_refused_for_a_leaf_with_its_own`) and signs with that one's
/// key ends with
/// `illegal_parameter`, `UnofferedSignatureScheme`, though its path is trusted.
#[test]
fn cv_is_checked_with_the_key_of_the_first_certificate() {
    let mut server = Server::new(SUITE);
    server.hello();
    let ee = server.encrypted_extensions(&[]);
    server.push(&ee);
    let mut certificates = chain("ok-ecdsa-p256");
    certificates.push(chain("ok-ecdsa-p384")[0].clone());
    server.push(&certificate_message(&certificates, &[]));
    let hash = server.transcript.current().expect("hash");
    let mut content = vec![0x20; 64];
    content.extend_from_slice(b"TLS 1.3, server CertificateVerify");
    content.push(0);
    content.extend_from_slice(hash.as_ref());
    let pk8 = fs::read(certs_dir().join("ok-ecdsa-p384").join("leaf.pk8")).expect("leaf.pk8");
    let random = Random::fixed_byte(0x5a);
    let other = SigningKey::from_pkcs8(Scheme::EcdsaSecp384r1Sha384, &pk8, &random)
        .expect("the appended certificate's P-384 key");
    let signature = other.sign(&content, &random).expect("sign");
    let mut body = Scheme::EcdsaSecp384r1Sha384.code().to_be_bytes().to_vec();
    body.extend_from_slice(&v16(&signature));
    server.push(&msg(15, &body));
    server.push_finished();
    server.flush();

    let verifier = path_verifier("ok-ecdsa-p256");
    let refs: Vec<&[u8]> = certificates.iter().map(Vec::as_slice).collect();
    assert!(
        verifier.verify(&refs, HOST, now()).is_ok(),
        "the path of the first certificate is trusted for the host"
    );
    let (result, written) = outcome(&server.input, &verifier);
    assert_eq!(
        result,
        Err(sent(
            A::ILLEGAL_PARAMETER,
            Violation::UnofferedSignatureScheme
        ))
    );
    assert_eq!(last_alert(&server, &written), Some((2, 47)));
}

/// CertificateVerify is bound to the key of the leaf the verifier judged, also between two
/// P-256 keys: `ok-wildcard`'s leaf holds a P-256 key of its own (`keys/p256-60.pk8`), so a
/// signature over the right transcript by the key the other P-256 leaves share (that of
/// `ok-ecdsa-p256`) ends with `decrypt_error`, `BadCertificateVerify`, while the leaf's own key
/// completes the handshake.
#[test]
fn cv_by_the_shared_p256_key_is_refused_for_a_leaf_with_its_own() {
    let host = "a.example.test";
    let case = "ok-wildcard";
    let own = fs::read(certs_dir().join(case).join("leaf.pk8")).expect("leaf.pk8");
    let shared = fs::read(certs_dir().join("ok-ecdsa-p256").join("leaf.pk8")).expect("leaf.pk8");
    assert!(own != shared, "{case} holds a key of its own");
    let verifier = path_verifier(case);
    for (key, expected) in [
        (&own, Ok(())),
        (
            &shared,
            Err(sent(A::DECRYPT_ERROR, Violation::BadCertificateVerify)),
        ),
    ] {
        let mut server = Server::for_host(SUITE, case, host);
        server.hello();
        let ee = server.encrypted_extensions(&[]);
        server.push(&ee);
        let certificate = server.certificate();
        server.push(&certificate);
        let hash = server.transcript.current().expect("hash");
        let mut content = vec![0x20; 64];
        content.extend_from_slice(b"TLS 1.3, server CertificateVerify");
        content.push(0);
        content.extend_from_slice(hash.as_ref());
        let random = Random::fixed_byte(0x5a);
        let signer = SigningKey::from_pkcs8(Scheme::EcdsaSecp256r1Sha256, key, &random)
            .expect("a P-256 key");
        let signature = signer.sign(&content, &random).expect("sign");
        let mut body = Scheme::EcdsaSecp256r1Sha256.code().to_be_bytes().to_vec();
        body.extend_from_slice(&v16(&signature));
        server.push(&msg(15, &body));
        server.push_finished();
        server.flush();
        let (result, written) = outcome_to(host, &server.input, &verifier);
        assert_eq!(result, expected);
        if expected.is_err() {
            assert_eq!(last_alert(&server, &written), Some((2, 51)));
        }
    }
}

/// An empty list and an entry with empty `cert_data` (`opaque cert_data<1..2^24-1>`) end with
/// `decode_error` and never reach the verifier.
#[test]
fn empty_certificate_entries_end_before_the_verifier() {
    let leaf = chain("ok-ecdsa-p256")[0].clone();
    for (label, certificates) in [
        ("an empty list", vec![]),
        ("one empty entry", vec![Vec::new()]),
        (
            "the leaf and an empty entry",
            vec![leaf.clone(), Vec::new()],
        ),
        (
            "an empty entry and the leaf",
            vec![Vec::new(), leaf.clone()],
        ),
    ] {
        let mut server = Server::new(SUITE);
        server.hello();
        let ee = server.encrypted_extensions(&[]);
        server.push(&ee);
        server.push(&certificate_message(&certificates, &[]));
        server.flush();
        let verifier = Double::accepting("ok-ecdsa-p256");
        let (result, written) = outcome(&server.input, &verifier);
        let violation = if certificates.is_empty() {
            Violation::EmptyCertificate
        } else {
            Violation::Malformed {
                message: "Certificate",
            }
        };
        assert_eq!(result, Err(sent(A::DECODE_ERROR, violation)), "{label}");
        assert_eq!(last_alert(&server, &written), Some((2, 50)), "{label}");
        assert!(
            verifier.seen.borrow().is_empty(),
            "{label}: the verifier is not called"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// 2. The host name.

/// The name sent in `server_name` and the name the verifier is given are one value, the host
/// the caller asked for: for an IP host neither carries a name.
#[test]
fn sni_and_verifier_host_are_one_value() {
    let mut split = Vec::new();
    for host in [
        "localhost",
        "LOCALHOST",
        "localhost.",
        "127.0.0.1",
        "127.0.0.1.",
        "[::1]",
    ] {
        let Some(sni) = sni_of(host) else {
            continue;
        };
        let mut server = Server::for_host(SUITE, "ok-ecdsa-p256", host);
        server.hello();
        server.flight();
        let verifier = Double::accepting("ok-ecdsa-p256");
        let _ = outcome_to(host, &server.input, &verifier);
        let given = verifier
            .hosts
            .borrow()
            .first()
            .cloned()
            .expect("the verifier is called");
        let same = match &sni {
            Some(name) => name.as_slice() == given.as_bytes(),
            None => matches!(names::host(&given), Host::Ip(_)),
        };
        if !same {
            let sni = sni.map(|n| String::from_utf8_lossy(&n).into_owned());
            split.push(format!(
                "{}: server_name {sni:?}, verifier {given:?}",
                shown(host)
            ));
        }
    }
    assert!(
        split.is_empty(),
        "the name sent and the name verified differ: {split:?}"
    );
}

/// The host is normalised once where the connection is made (one trailing dot removed, ASCII
/// lower case): `server_name`, the verifier's host and `Connection::host` are that one value.
#[test]
fn the_connection_reports_the_host_it_sent_and_verified() {
    for (host, normalised) in [
        ("LocalHost.", "localhost"),
        ("localhost", "localhost"),
        ("127.0.0.1.", "127.0.0.1"),
        ("[::1]", "[::1]"),
    ] {
        let mut server = Server::for_host(SUITE, "ok-ecdsa-p256", host);
        server.hello();
        server.flight();
        let mut pipe = Pipe::with_input(server.input.clone());
        let verifier = Double::accepting("ok-ecdsa-p256");
        let connection = connect_to(&mut pipe, host, &verifier).expect("the handshake completes");
        assert_eq!(connection.host(), normalised, "{host}");
        connection.into_inner();
        assert_eq!(verifier.hosts.borrow().as_slice(), [normalised], "{host}");
        let sni = sni_of(host).expect("a ClientHello");
        let expected =
            matches!(names::host(normalised), Host::Dns(_)).then(|| normalised.as_bytes().to_vec());
        assert_eq!(sni, expected, "{host}");
    }
}

/// A host that no certificate can match (the verifier's reference identifier is a DNS name that
/// is not in the preferred name syntax, `names::dns_name_matches`) is refused before the first
/// byte: no ClientHello carries it.
#[test]
fn a_host_no_certificate_can_match_sends_nothing() {
    let long = format!(
        "{}.{}.{}.{}",
        "a".repeat(63),
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(62)
    );
    assert_eq!(long.len(), 254);
    let mut written_anyway = Vec::new();
    for host in [
        "",
        "local\0host",
        "b\u{fc}cher.test",
        long.as_str(),
        "0x7f000001",
        "a..test",
    ] {
        assert!(
            matches!(names::host(host), Host::Dns(_)) && !names::is_dns_name(host),
            "{} matches no certificate",
            shown(host)
        );
        let written = client_written_to(host, &[]);
        if !written.is_empty() {
            written_anyway.push(shown(host));
        }
    }
    assert!(
        written_anyway.is_empty(),
        "a ClientHello was written for hosts no certificate can match: {written_anyway:?}"
    );
}

/// A refused host ends with `Error::Host` naming the rule it breaks, nothing written and the
/// verifier not called; the host in the text is a bounded, escaped excerpt.
#[test]
fn a_refused_host_names_its_reason_in_a_bounded_text() {
    let long_label = format!("{}.test", "a".repeat(64));
    let long_name = format!("{0}.{0}.{0}.{1}", "a".repeat(63), "a".repeat(62));
    let huge = format!("\u{1b}[31m{}", "\u{7f}".repeat(100_000));
    for (host, reason) in [
        ("", HostReason::Empty),
        (".", HostReason::Empty),
        ("local\0host", HostReason::Byte(0)),
        ("b\u{fc}cher.test", HostReason::Byte(0xc3)),
        ("a b", HostReason::Byte(b' ')),
        (long_name.as_str(), HostReason::NameTooLong { bytes: 254 }),
        (long_label.as_str(), HostReason::LabelTooLong { bytes: 64 }),
        ("a..test", HostReason::EmptyLabel),
        (".test", HostReason::EmptyLabel),
        ("test..", HostReason::EmptyLabel),
        ("-a.test", HostReason::HyphenAtLabelEdge),
        ("a-.test", HostReason::HyphenAtLabelEdge),
        ("0x7f000001", HostReason::NumberLastLabel),
        ("127.0.0.0x1", HostReason::NumberLastLabel),
        ("1.2.3.4.5", HostReason::NumberLastLabel),
        (huge.as_str(), HostReason::NameTooLong { bytes: huge.len() }),
        ("\u{1b}[31m", HostReason::Byte(0x1b)),
    ] {
        let mut pipe = Pipe::default();
        let verifier = Double::accepting("ok-ecdsa-p256");
        let error = connect_to(&mut pipe, host, &verifier)
            .map(|c| {
                c.into_inner();
            })
            .expect_err("a host no certificate can match");
        let Error::Host(refusal) = &error else {
            panic!("{}: {error:?}", shown(host));
        };
        assert_eq!(refusal.reason, reason, "{}", shown(host));
        assert!(pipe.output.is_empty(), "{}: nothing written", shown(host));
        assert!(verifier.hosts.borrow().is_empty(), "{}", shown(host));
        let text = error.to_string();
        assert!(text.len() <= 512, "{} bytes: {}", text.len(), shown(&text));
        assert!(
            text.bytes().all(|b| (0x20..0x7f).contains(&b)),
            "printable ASCII only: {}",
            shown(&text)
        );
        if host.len() > 64 {
            assert!(
                text.contains(&format!("({} bytes, cut after 64)", host.len())),
                "{text}"
            );
        }
    }
    assert_eq!(
        Error::Host(names::connection_host("a\\b\n").expect_err("refused")).to_string(),
        "the host \"a\\\\b\\x0a\" matches no certificate: the byte 0x5c is none of the letters, \
         digits, hyphen and dot of a DNS name"
    );
}

// ---------------------------------------------------------------------------------------------
// 3. Time.

/// A clock before 1970 or at either end of `Time` ends the handshake with `Expired` at that
/// time and `certificate_expired`; its text is printed without a panic.
#[test]
fn extreme_clocks_end_in_certificate_expired() {
    let verifier = path_verifier("ok-ecdsa-p256");
    for at in [
        Time::from_unix(i64::MIN),
        Time::from_unix(-1),
        Time::from_unix(i64::MAX),
    ] {
        let server = Server::handshake(SUITE);
        let mut pipe = Pipe::with_input(server.input.clone());
        let random = Random::fixed_sequence(&DRAWS);
        let result = Connection::with_random(&mut pipe, HOST, &verifier, at, &random).map(|c| {
            c.into_inner();
        });
        let error = result.expect_err("no certificate is valid then");
        match &error {
            Error::Certificate(CertificateRejection::Expired { at: checked, .. }) => {
                assert_eq!(*checked, at);
            }
            other => panic!("{other:?} at {}", at.unix()),
        }
        assert!(!error.to_string().is_empty());
        assert_eq!(last_alert(&server, &pipe.output), Some((2, 45)));
    }
}

// ---------------------------------------------------------------------------------------------
// 4. Signature schemes.

/// A CertificateVerify scheme offered but of another key kind or curve than the leaf's:
/// `illegal_parameter`, `UnofferedSignatureScheme`, with the shipped path verifier.
#[test]
fn cv_scheme_fits_the_leaf_key_of_each_kind() {
    for (case, scheme) in [
        ("ok-ecdsa-p384", 0x0403u16),
        ("ok-ecdsa-p384", 0x0804),
        ("ok-ed25519", 0x0403),
        ("ok-ed25519", 0x0804),
        ("ok-rsa2048", 0x0807),
        ("ok-rsa2048", 0x0503),
    ] {
        let mut server = Server::with_case(SUITE, case);
        server.hello();
        let ee = server.encrypted_extensions(&[]);
        server.push(&ee);
        let certificate = server.certificate();
        server.push(&certificate);
        let mut cv = server.certificate_verify();
        cv[4..6].copy_from_slice(&scheme.to_be_bytes());
        server.push(&cv);
        server.flush();
        let (result, written) = outcome(&server.input, &path_verifier(case));
        assert_eq!(
            result,
            Err(sent(
                A::ILLEGAL_PARAMETER,
                Violation::UnofferedSignatureScheme
            )),
            "{case} {scheme:#06x}"
        );
        assert_eq!(
            last_alert(&server, &written),
            Some((2, 47)),
            "{case} {scheme:#06x}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// 5. Alerts and refusal texts.

/// Every check of item 7, `Expired` and `NameMismatch`: one alert each (`unknown_ca`,
/// `certificate_expired`, `bad_certificate`), written as the client's last record, and nothing
/// else after the ClientHello: no Finished, no data.
#[test]
fn every_rejection_sends_its_one_alert_and_nothing_after() {
    let time = Time::from_unix(0);
    let mut rejections: Vec<CertificateRejection> = Check::ALL
        .iter()
        .cloned()
        .map(CertificateRejection::NotTrusted)
        .collect();
    rejections.push(CertificateRejection::Expired {
        certificate: 0,
        not_before: time,
        not_after: time,
        at: time,
    });
    rejections.push(CertificateRejection::NameMismatch {
        host: HOST.to_string(),
    });
    assert_eq!(rejections.len(), 14);
    for rejection in rejections {
        let expected = match &rejection {
            CertificateRejection::NotTrusted(Check::NoPathToAnchor) => A::UNKNOWN_CA,
            CertificateRejection::Expired { .. } => A::CERTIFICATE_EXPIRED,
            _ => A::BAD_CERTIFICATE,
        };
        assert_eq!(rejection.alert(), expected, "{rejection}");
        let server = Server::handshake(SUITE);
        let verifier = Double::answering(Err(rejection.clone()));
        let (result, written) = outcome(&server.input, &verifier);
        assert_eq!(result, Err(Error::Certificate(rejection.clone())));
        let records = server.client_records(&written);
        assert_eq!(records.len(), 2, "the hello and the alert: {rejection}");
        assert_eq!(records[1].content_type, 21, "{rejection}");
        assert_eq!(records[1].content, vec![2, expected.0], "{rejection}");
        assert!(records[1].protected, "{rejection}");
    }
}

/// A refusal text carries a bounded excerpt of the peer's bytes: a leaf with a critical
/// extension of an OBJECT IDENTIFIER of 20,001 bytes gives a text of at most 1 KiB.
#[test]
fn a_refusal_text_holds_a_bounded_excerpt_of_the_peer() {
    let root = Built::ca("Seam Root", 1).der();
    let mut long_oid = vec![0x2A];
    long_oid.extend(std::iter::repeat_n(0x01, 20_000));
    let leaf = Built {
        unknown_critical: Some(long_oid),
        ..Built::leaf("localhost", 9, "Seam Root", 1)
    }
    .der();
    let mut verifier = PathVerifier::new(Vec::new()).with_verifier(KeyEcho);
    verifier.add_root(&root).expect("a root");
    let rejection = verifier
        .verify(&[leaf.as_slice()], HOST, now())
        .expect_err("an unknown critical extension");
    assert!(
        matches!(
            rejection,
            CertificateRejection::NotTrusted(Check::UnknownCriticalExtension { .. })
        ),
        "{}",
        rejection.to_string().chars().take(80).collect::<String>()
    );
    let text = rejection.to_string();
    assert!(
        text.len() <= 1024,
        "the refusal text holds {} bytes: {}...",
        text.len(),
        text.chars().take(60).collect::<String>()
    );
}

/// 40,000 bytes from outside the crate, with control characters and a backslash.
fn hostile() -> String {
    format!("\u{1b}]0;x\u{7}\\{}", "1.".repeat(20_000))
}

/// `check` with every field that holds bytes from outside the crate set to [`hostile`], and
/// whether it has one. The match names every variant: a new one does not compile until it is
/// sorted here.
fn hostile_check(check: &Check) -> (Check, bool) {
    match check {
        Check::UnknownCriticalExtension { certificate, .. } => (
            Check::UnknownCriticalExtension {
                certificate: *certificate,
                oid: hostile(),
            },
            true,
        ),
        Check::NameConstraint { certificate, .. } => (
            Check::NameConstraint {
                certificate: *certificate,
                name: hostile(),
            },
            true,
        ),
        Check::Unreadable { .. }
        | Check::NoPathToAnchor
        | Check::BadSignature { .. }
        | Check::IssuerNotCa { .. }
        | Check::PathTooLong(_)
        | Check::CandidateLimit(_)
        | Check::KeyUsage { .. }
        | Check::NoServerAuth { .. }
        | Check::WeakAlgorithm { .. }
        | Check::WeakKey { .. } => (check.clone(), false),
    }
}

/// Every text of a refusal (each `Check` of item 7 inside `NotTrusted`, `Expired`,
/// `NameMismatch`, each `AnchorError`, `Error::Host`) with the longest bytes from outside the
/// crate: at most 512 bytes, printable ASCII only, and where it holds such bytes, the excerpt
/// says it was cut (`error::excerpt`, 64 bytes).
#[test]
fn every_refusal_text_shows_outside_bytes_as_a_cut_excerpt() {
    let time = Time::from_unix(0);
    let mut texts: Vec<(String, bool)> = Vec::new();
    for check in Check::ALL.iter() {
        let (check, outside) = hostile_check(check);
        texts.push((CertificateRejection::NotTrusted(check).to_string(), outside));
    }
    for rejection in [
        CertificateRejection::Expired {
            certificate: 0,
            not_before: time,
            not_after: time,
            at: time,
        },
        CertificateRejection::NameMismatch { host: hostile() },
    ] {
        let outside = match &rejection {
            CertificateRejection::NotTrusted(_) | CertificateRejection::Expired { .. } => false,
            CertificateRejection::NameMismatch { .. } => true,
        };
        texts.push((Error::Certificate(rejection).to_string(), outside));
    }
    let label = "A".repeat(EXTRA_BYTES_MAX);
    for error in [
        AnchorError::NoCertificate,
        AnchorError::NotACertificate { index: 0, label },
        AnchorError::BadPem { index: 0 },
        AnchorError::BadCertificate {
            index: 0,
            error: surf_tls::x509::Error::BadName,
        },
        AnchorError::NotACa { index: 0 },
        AnchorError::TooLarge,
        AnchorError::TooMany,
    ] {
        let outside = match &error {
            AnchorError::NotACertificate { .. } => true,
            AnchorError::NoCertificate
            | AnchorError::BadPem { .. }
            | AnchorError::BadCertificate { .. }
            | AnchorError::NotACa { .. }
            | AnchorError::TooLarge
            | AnchorError::TooMany => false,
        };
        texts.push((error.to_string(), outside));
    }
    let refusal = names::connection_host(&hostile()).expect_err("no DNS name");
    texts.push((Error::Host(refusal).to_string(), true));
    assert_eq!(texts.len(), 12 + 2 + 7 + 1);

    for (text, outside) in &texts {
        let head: String = text.chars().take(80).collect();
        assert!(text.len() <= 512, "{} bytes: {}", text.len(), shown(&head));
        assert!(
            text.bytes().all(|b| (0x20..0x7f).contains(&b)),
            "printable ASCII only: {}",
            shown(&head)
        );
        if *outside {
            assert!(text.contains("bytes, cut after 64)"), "{}", shown(text));
        }
    }
}

/// A dNSName of the peer in a nameConstraints refusal is cut at 64 bytes and says so; a host
/// in a name mismatch is escaped; short printable values print as they are.
#[test]
fn a_name_constraint_refusal_cuts_the_peer_name() {
    let refusal = CertificateRejection::NotTrusted(Check::NameConstraint {
        certificate: 1,
        name: format!("{}\n.example", "a".repeat(70)),
    });
    assert_eq!(
        refusal.to_string(),
        format!(
            "NameConstraint: {} ... (79 bytes, cut after 64) outside the constraints of \
             certificate 1",
            "a".repeat(64)
        )
    );
    let escaped = CertificateRejection::NameMismatch {
        host: "a\u{0}b\\".into(),
    };
    assert_eq!(
        escaped.to_string(),
        "no subjectAltName entry matches a\\x00b\\\\"
    );
}

// ---------------------------------------------------------------------------------------------
// 6. Anchors.

/// A verifier over an anchor set whose `count()` is 0 refuses every path with
/// `NoPathToAnchor`, and the handshake sends `unknown_ca`.
#[test]
fn an_empty_anchor_set_refuses_every_path() {
    let anchors = Anchors::from_list(b"no PEM block here");
    assert_eq!(anchors.count(), 0);
    let verifier = PathVerifier::new(anchors.as_slice().to_vec());
    for case in ["ok-ecdsa-p256", "ok-rsa2048", "ok-ed25519", "ok-ecdsa-p384"] {
        let certificates = chain(case);
        let refs: Vec<&[u8]> = certificates.iter().map(Vec::as_slice).collect();
        assert_eq!(
            verifier.verify(&refs, HOST, now()),
            Err(CertificateRejection::NotTrusted(Check::NoPathToAnchor)),
            "{case}"
        );
    }
    let server = Server::handshake(SUITE);
    let (result, written) = outcome(&server.input, &verifier);
    assert_eq!(
        result,
        Err(Error::Certificate(CertificateRejection::NotTrusted(
            Check::NoPathToAnchor
        )))
    );
    assert_eq!(last_alert(&server, &written), Some((2, 48)));
}

/// Item 9.4: "A candidate that is an anchor and verifies ends the path: the other checks of item
/// 7 run on it (validity, basicConstraints and pathLenConstraint, keyUsage, extKeyUsage,
/// nameConstraints)". A root added with `add_root` is held to them; the same root read from an
/// extra anchor file (`Anchors::with_extra_pem`, then `PathVerifier::new`) must be too.
#[test]
fn extra_anchors_are_held_to_the_issuer_checks_of_item_9_4() {
    let leaf = Built::leaf("localhost", 9, "Seam Root", 1).der();
    let expired = Built {
        not_before: tlv(0x17, b"000101000000Z"),
        not_after: tlv(0x17, b"010101000000Z"),
        ..Built::ca("Seam Root", 1)
    }
    .der();
    let no_cert_sign = Built {
        key_usage: Some(KU_DIGITAL_SIGNATURE),
        ..Built::ca("Seam Root", 1)
    }
    .der();
    let mut drift = Vec::new();
    for (label, root) in [
        ("an expired root", expired),
        ("a root without keyCertSign", no_cert_sign),
    ] {
        let mut by_certificate = PathVerifier::new(Vec::new()).with_verifier(KeyEcho);
        by_certificate.add_root(&root).expect("a root");
        let checked = by_certificate.verify(&[leaf.as_slice()], HOST, now());
        assert!(
            checked.is_err(),
            "{label}: add_root holds the root to item 9.4"
        );

        let mut anchors = Anchors::from_list(b"");
        assert_eq!(
            anchors.with_extra_pem(&pem_of(&root)),
            Ok(1),
            "{label}: the extra file takes the root"
        );
        let extra = PathVerifier::new(anchors.as_slice().to_vec()).with_verifier(KeyEcho);
        let got = extra.verify(&[leaf.as_slice()], HOST, now());
        if got != checked {
            drift.push(format!(
                "{label}: extra file {}, add_root {:?}",
                if got.is_ok() { "accepts" } else { "refuses" },
                checked.as_ref().err()
            ));
        }
    }
    assert!(
        drift.is_empty(),
        "an anchor of the extra file skips the checks of item 9.4: {drift:?}"
    );
}

/// Item 9.4 for the bundled list: every anchor of `Anchors::bundled()` holds the certificate it
/// was read from, and a root read the way the bundle is read (`Anchors::from_list`) is held to
/// the issuer checks as through `add_root`: an expired root, one without keyCertSign, and one
/// that is no CA.
#[test]
fn bundled_anchors_keep_their_certificate_and_are_checked() {
    let bundled = Anchors::bundled();
    let without: Vec<usize> = (0..bundled.count())
        .filter(|&i| bundled.as_slice()[i].certificate().is_none())
        .collect();
    assert!(
        without.is_empty(),
        "anchors without certificate: {without:?}"
    );
    let held: usize = bundled
        .as_slice()
        .iter()
        .filter_map(|a| a.certificate())
        .map(<[u8]>::len)
        .sum();
    eprintln!(
        "{} bundled anchors hold {held} bytes of certificates",
        bundled.count()
    );

    let leaf = Built::leaf("localhost", 9, "Seam Root", 1).der();
    let roots = [
        Built {
            not_before: tlv(0x17, b"000101000000Z"),
            not_after: tlv(0x17, b"010101000000Z"),
            ..Built::ca("Seam Root", 1)
        },
        Built {
            key_usage: Some(KU_DIGITAL_SIGNATURE),
            ..Built::ca("Seam Root", 1)
        },
        Built {
            ca: false,
            ..Built::ca("Seam Root", 1)
        },
    ];
    for root in roots {
        let root = root.der();
        let mut by_certificate = PathVerifier::new(Vec::new()).with_verifier(KeyEcho);
        by_certificate.add_root(&root).expect("a root");
        let checked = by_certificate.verify(&[leaf.as_slice()], HOST, now());
        assert!(checked.is_err(), "add_root refuses: {checked:?}");
        let list = Anchors::from_list(&pem_of(&root));
        assert_eq!(list.count(), 1);
        let got = PathVerifier::new(list.as_slice().to_vec())
            .with_verifier(KeyEcho)
            .verify(&[leaf.as_slice()], HOST, now());
        assert_eq!(got, checked);
    }
}

// ---------------------------------------------------------------------------------------------
// 7. Costs across the seam.

/// The largest Certificate message the handshake lets through (64 entries, under 65,535 bytes),
/// each certificate readable and full of the shortest dNSNames: the verifier's peak memory stays
/// linear in the bytes sent, at most 32 times them.
#[test]
fn the_verifier_memory_is_linear_in_a_full_certificate_message() {
    let sans: Vec<Vec<u8>> = (0..250).map(|_| tlv(0x82, b"a")).collect();
    let certificates: Vec<Vec<u8>> = (0..64u8)
        .map(|i| {
            Built {
                subject: format!("s{i}"),
                issuer: "nobody".into(),
                sans: sans.clone(),
                serial: i + 1,
                ..Built::leaf("a", 9, "nobody", 1)
            }
            .der()
        })
        .collect();
    let total: usize = certificates.iter().map(Vec::len).sum();
    assert!(
        total + 4 + 64 * 5 <= 65_535,
        "the message fits item 9.1: {total} bytes of certificates"
    );
    let refs: Vec<&[u8]> = certificates.iter().map(Vec::as_slice).collect();
    let verifier = PathVerifier::new(Vec::new());
    let (result, peak) = peak_while(|| verifier.verify(&refs, "a.test", now()));
    assert_eq!(
        result,
        Err(CertificateRejection::NotTrusted(Check::NoPathToAnchor))
    );
    eprintln!("verifier peak {peak} bytes for {total} bytes of certificates");
    assert!(
        peak <= 32 * total,
        "peak {peak} bytes for {total} bytes of certificates"
    );
}

// ---------------------------------------------------------------------------------------------
// 8. After a fatal error.

/// A record that does not open ends the connection with `bad_record_mac`; after that alert no
/// read, write, flush, close or drop writes a byte, and each returns the same error.
#[test]
fn after_a_bad_record_nothing_more_is_written() {
    let mut server = Server::handshake(SUITE);
    server.data(b"ok");
    let last = server.input.len() - 1;
    server.input[last] ^= 1;
    server.data(b"after");
    let mut pipe = Pipe::with_input(server.input.clone());
    let verifier = Double::accepting("ok-ecdsa-p256");
    let mut connection = connect(&mut pipe, &verifier).expect("the handshake completes");
    let before = connection.get_ref().output.len();
    let mut buf = [0u8; 16];
    let error = connection
        .read_data(&mut buf)
        .expect_err("a record that does not open");
    assert_eq!(error, sent(A::BAD_RECORD_MAC, Violation::BadRecordMac));
    let after_alert = connection.get_ref().output.len();
    assert!(after_alert > before, "the alert is written");
    assert_eq!(connection.read_data(&mut buf), Err(error.clone()));
    assert_eq!(connection.read_data(&mut []), Err(error.clone()));
    assert_eq!(connection.write_data(b"x"), Err(error.clone()));
    assert_eq!(connection.write_data(b""), Err(error.clone()));
    assert!(std::io::Write::flush(&mut connection).is_err());
    assert_eq!(connection.close(), Err(error));
    drop(connection);
    assert_eq!(
        pipe.output.len(),
        after_alert,
        "nothing after the alert, the drop included"
    );
    assert_eq!(last_alert(&server, &pipe.output), Some((2, 20)));
}

// ---------------------------------------------------------------------------------------------
// 9. Secrets.

/// Fails to compile unless `T` has a `Drop` of its own (a field's drop glue does not count):
/// the types of docs/design/tls.md item 6 that hold secret bytes and overwrite them with
/// zeros when dropped. That the bytes are zeros after the drop cannot be read without
/// `unsafe`; this holds that the wipe is there.
#[allow(drop_bounds)]
const fn wipes_on_drop<T: Drop>() {}

const _: () = {
    wipes_on_drop::<surf_tls::crypto::Secret>();
    wipes_on_drop::<surf_tls::crypto::Digest>();
    wipes_on_drop::<surf_tls::key_schedule::Secret>();
    wipes_on_drop::<surf_tls::key_schedule::TrafficKey>();
};

/// `Debug` of a connection and of the traffic secrets prints no byte of a secret.
#[test]
fn debug_prints_no_secret() {
    let server = Server::handshake(SUITE);
    let mut pipe = Pipe::with_input(server.input.clone());
    let verifier = Double::accepting("ok-ecdsa-p256");
    let connection = connect(&mut pipe, &verifier).expect("the handshake completes");
    let handshake = server.handshake.as_ref().expect("handshake secrets");
    let application = server.application.as_ref().expect("application secrets");
    let texts = [
        format!("{connection:?}"),
        format!("{handshake:?}"),
        format!("{application:?}"),
    ];
    for secret in [
        &handshake.client,
        &handshake.server,
        &application.client,
        &application.server,
    ] {
        let bytes = secret.as_ref();
        let listed = format!("{:?}", &bytes[..4]);
        let listed = listed.trim_end_matches(']');
        for text in &texts {
            assert!(!text.contains(&to_hex(bytes)), "{text}");
            assert!(!text.contains(&to_hex(&bytes[..8])), "{text}");
            assert!(!text.contains(listed), "{text}");
        }
    }
    connection.into_inner();
}
