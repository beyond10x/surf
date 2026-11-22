//! Path building and validation (`surf_tls::path::PathVerifier`, `story:tls-x509-path`).
//!
//! The first part runs each case of `crates/tls-test-certs/certs/INDEX` with its root as the
//! only anchor, at the time and host of its line, and compares the outcome with the line, read
//! from the file (acceptance 1, 2, 5 and 10). The second part builds small chains with a DER
//! writer of this file and checks them with [`KeyEcho`], a signature verifier of the test under
//! which a signature verifies exactly when its bytes are the signer's public key: so a chain of
//! any shape is made without a key or a real signature, and the bounds of docs/design/tls.md
//! item 9 are reached at small sizes. Costs are counts ([`Work`]), never wall times.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use surf_tls::crypto::{CryptoError, PublicKeyKind, Scheme, SignatureVerifier};
use surf_tls::der::Time;
use surf_tls::path::{
    CANDIDATES_MAX, INTERMEDIATES_MAX, Limits, NAME_CONSTRAINT_COMPARISONS_MAX, PathVerifier,
    SIGNATURE_CHECKS_MAX, Work,
};
use surf_tls::pem;
use surf_tls::verify::{CertificateRejection, Check, Limit, PathTooLong, ServerCertVerifier};
use surf_tls::x509::{self, Certificate, LeafKey, TrustAnchor};

use crate::x509_parse::{seq, tlv};

// ---------------------------------------------------------------------------------------------
// The cases of INDEX.

fn certs_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tls-test-certs/certs")
}

/// One line of INDEX: host, time and outcome as written.
struct IndexLine {
    host: String,
    time: Time,
    outcome: String,
}

fn index_lines() -> Vec<(String, IndexLine)> {
    let text = fs::read_to_string(certs_dir().join("INDEX")).expect("read INDEX");
    text.lines()
        .map(|line| {
            let fields: Vec<&str> = line.split(' ').collect();
            assert!(fields.len() >= 4, "an INDEX line: {line}");
            (
                fields[0].to_string(),
                IndexLine {
                    host: fields[1].to_string(),
                    time: parse_time(fields[2]),
                    outcome: fields[3..].join(" "),
                },
            )
        })
        .collect()
}

/// `YYYY-MM-DDTHH:MM:SSZ`.
fn parse_time(text: &str) -> Time {
    let n = |range: std::ops::Range<usize>| text[range].parse::<u32>().expect("a number");
    assert_eq!(text.len(), 20, "{text}");
    Time::from_utc(n(0..4), n(5..7), n(8..10), n(11..13), n(14..16), n(17..19)).expect("a time")
}

fn pem_file(case: &str, file: &str) -> Vec<Vec<u8>> {
    let bytes = fs::read(certs_dir().join(case).join(file)).expect("read a PEM file");
    pem::parse(&bytes)
        .expect("PEM")
        .into_iter()
        .map(|b| b.data)
        .collect()
}

/// The outcome as a line of INDEX writes it.
fn outcome_text(result: &Result<LeafKey, CertificateRejection>) -> String {
    match result {
        Ok(_) => "ok".into(),
        Err(CertificateRejection::Expired { .. }) => "CertificateExpired".into(),
        Err(CertificateRejection::NameMismatch { .. }) => "CertificateNameMismatch".into(),
        Err(CertificateRejection::NotTrusted(check)) => {
            format!("CertificateNotTrusted {}", check.name())
        }
    }
}

/// The case run with its root as the only anchor, through both ways of making the anchor:
/// from the root's certificate (its extensions checked too) and from a [`TrustAnchor`] alone.
/// Both give the same outcome; the first is returned with its work.
fn run(case: &str, chain: &[Vec<u8>]) -> (Result<LeafKey, CertificateRejection>, Work) {
    let lines = index_lines();
    let (_, line) = lines
        .iter()
        .find(|(name, _)| name == case)
        .unwrap_or_else(|| panic!("{case} is a case of INDEX"));
    let root = pem_file(case, "root.pem");
    assert_eq!(root.len(), 1, "{case}: one root");
    let refs: Vec<&[u8]> = chain.iter().map(Vec::as_slice).collect();
    let mut with_root = PathVerifier::new(Vec::new());
    with_root
        .add_root(&root[0])
        .expect("the root gives an anchor");
    let (result, work) = with_root.verify_counting(&refs, &line.host, line.time);
    let anchor = TrustAnchor::from_der(&root[0]).expect("the root gives an anchor");
    let by_anchor = PathVerifier::new(vec![anchor]).verify(&refs, &line.host, line.time);
    assert_eq!(
        by_anchor, result,
        "{case}: the same through a TrustAnchor alone"
    );
    (result, work)
}

/// Runs `case` and compares its outcome with its line of INDEX, then the details acceptance 1
/// names for it.
fn index_case(case: &str) {
    let lines = index_lines();
    let (_, line) = lines.iter().find(|(name, _)| name == case).expect("a line");
    let chain = pem_file(case, "chain.pem");
    let started = Instant::now();
    let (result, work) = run(case, &chain);
    let took = started.elapsed();
    println!("{case}: {} in {took:?}, {work:?}", outcome_text(&result));
    assert_eq!(outcome_text(&result), line.outcome, "{case}: {result:?}");
    let rejected = |check: Check| Err(CertificateRejection::NotTrusted(check));
    match case {
        name if name.starts_with("ok-") || name == "1000-sans" => {
            let leaf = Certificate::from_der(&chain[0]).expect("the leaf reads");
            assert_eq!(result, Ok(LeafKey::from_certificate(&leaf)), "{case}");
        }
        "pathlen-exceeded" => assert_eq!(
            result,
            rejected(Check::PathTooLong(PathTooLong::PathLenConstraint {
                certificate: 2
            }))
        ),
        "path-too-long" => assert_eq!(
            result,
            rejected(Check::PathTooLong(PathTooLong::Intermediates {
                count: 7,
                bound: 6
            }))
        ),
        "leaf-no-digital-signature" => assert_eq!(
            result,
            rejected(Check::KeyUsage {
                certificate: 0,
                lacks: "digitalSignature"
            })
        ),
        "issuer-no-key-cert-sign" => assert_eq!(
            result,
            rejected(Check::KeyUsage {
                certificate: 1,
                lacks: "keyCertSign"
            })
        ),
        "leaf-v1" => assert_eq!(
            result,
            rejected(Check::Unreadable {
                certificate: 0,
                error: x509::Error::UnsupportedVersion
            })
        ),
        "intermediate-not-ca" => {
            assert_eq!(result, rejected(Check::IssuerNotCa { certificate: 1 }))
        }
        "bad-signature" => assert_eq!(result, rejected(Check::BadSignature { certificate: 0 })),
        "untrusted-root" => assert_eq!(result, rejected(Check::NoPathToAnchor)),
        "rsa-1024" => assert_eq!(
            result,
            rejected(Check::WeakKey {
                certificate: 0,
                bits: 1024
            })
        ),
        "sha1" | "md5" => {
            assert!(
                matches!(
                    result,
                    Err(CertificateRejection::NotTrusted(Check::WeakAlgorithm {
                        certificate: 0,
                        ..
                    }))
                ),
                "{case}: {result:?}"
            );
            assert_eq!(work.signature_checks, 0, "{case}: refused before any check");
        }
        "candidate-limit" | "candidate-limit-no-anchor" => {
            assert_eq!(
                result,
                rejected(Check::CandidateLimit(Limit::SignatureChecks { bound: 100 }))
            );
            assert_eq!(work.signature_checks, 100, "{case}: the limit, reached");
        }
        "expired" | "not-yet-valid" => {
            assert!(
                matches!(
                    result,
                    Err(CertificateRejection::Expired { certificate: 0, .. })
                ),
                "{case}: {result:?}"
            );
        }
        _ => {}
    }
}

macro_rules! index_cases {
    ($($test:ident => $case:literal,)*) => {
        $(
            #[test]
            fn $test() {
                index_case($case);
            }
        )*
        /// The cases with a test of their own above.
        const CASES: &[&str] = &[$($case),*];
    };
}

index_cases! {
    index_ok_rsa2048 => "ok-rsa2048",
    index_ok_ecdsa_p256 => "ok-ecdsa-p256",
    index_ok_ecdsa_p384 => "ok-ecdsa-p384",
    index_ok_ed25519 => "ok-ed25519",
    index_ok_wildcard => "ok-wildcard",
    index_expired => "expired",
    index_not_yet_valid => "not-yet-valid",
    index_wrong_host => "wrong-host",
    index_untrusted_root => "untrusted-root",
    index_bad_signature => "bad-signature",
    index_intermediate_not_ca => "intermediate-not-ca",
    index_path_too_long => "path-too-long",
    index_pathlen_exceeded => "pathlen-exceeded",
    index_leaf_no_digital_signature => "leaf-no-digital-signature",
    index_issuer_no_key_cert_sign => "issuer-no-key-cert-sign",
    index_candidate_limit => "candidate-limit",
    index_candidate_limit_no_anchor => "candidate-limit-no-anchor",
    index_unknown_critical_extension => "unknown-critical-extension",
    index_wildcard_two_labels => "wildcard-two-labels",
    index_common_name_only => "common-name-only",
    index_ip_mismatch => "ip-mismatch",
    index_name_constraints_excluded => "name-constraints-excluded",
    index_eku_client_only => "eku-client-only",
    index_1000_sans => "1000-sans",
    index_ok_rsa_pss => "ok-rsa-pss",
    index_ok_rsa_pss_sha384 => "ok-rsa-pss-sha384",
    index_ok_rsa_pss_sha512 => "ok-rsa-pss-sha512",
    index_ok_rsa_pkcs1_sha384 => "ok-rsa-pkcs1-sha384",
    index_ok_rsa_pkcs1_sha512 => "ok-rsa-pkcs1-sha512",
    index_leaf_v1 => "leaf-v1",
    index_sha1 => "sha1",
    index_md5 => "md5",
    index_rsa_1024 => "rsa-1024",
}

#[test]
fn every_case_of_index_has_its_test() {
    let named: BTreeSet<String> = index_lines().into_iter().map(|(n, _)| n).collect();
    let tested: BTreeSet<String> = CASES.iter().map(|c| c.to_string()).collect();
    assert_eq!(named, tested);
    assert_eq!(CASES.len(), 33);
}

/// Acceptance 10: each of the twelve checks is the expected check of a case of INDEX, and
/// that case gives it. The names come from `Check::ALL`.
#[test]
fn every_check_has_a_case() {
    let lines = index_lines();
    let mut missing = Vec::new();
    for check in &Check::ALL {
        let want = format!("CertificateNotTrusted {}", check.name());
        let cases: Vec<&String> = lines
            .iter()
            .filter(|(_, l)| l.outcome == want)
            .map(|(n, _)| n)
            .collect();
        if cases.is_empty() {
            missing.push(check.name());
        }
        for case in cases {
            let (result, _) = run(case, &pem_file(case, "chain.pem"));
            assert_eq!(outcome_text(&result), want, "{case}");
        }
    }
    assert!(missing.is_empty(), "checks without a case: {missing:?}");
}

/// Acceptance 2: the policy comes before any verification: the `sha1` leaf with one byte of
/// its signature flipped still gives `WeakAlgorithm`, and `bad-signature` gives `BadSignature`.
#[test]
fn a_weak_algorithm_is_refused_before_its_signature_is_verified() {
    let mut chain = pem_file("sha1", "chain.pem");
    let last = chain[0].len() - 1;
    chain[0][last] ^= 0x01;
    let (result, work) = run("sha1", &chain);
    assert!(
        matches!(
            result,
            Err(CertificateRejection::NotTrusted(
                Check::WeakAlgorithm { .. }
            ))
        ),
        "{result:?}"
    );
    assert_eq!(work.signature_checks, 0);
    let (bad, _) = run("bad-signature", &pem_file("bad-signature", "chain.pem"));
    assert_ne!(outcome_text(&bad), outcome_text(&result));
}

/// Acceptance 5: `candidate-limit` ends at the limit of 100 signature checks, the work a count
/// and the time printed for the report.
#[test]
fn candidate_limit_ends_at_the_signature_limit() {
    let chain = pem_file("candidate-limit", "chain.pem");
    assert_eq!(chain.len(), 50);
    let started = Instant::now();
    let (result, work) = run("candidate-limit", &chain);
    println!(
        "candidate-limit: {result:?} in {:?}, {work:?}",
        started.elapsed()
    );
    assert_eq!(
        result,
        Err(CertificateRejection::NotTrusted(Check::CandidateLimit(
            Limit::SignatureChecks { bound: 100 }
        )))
    );
    assert_eq!(work.signature_checks, 100);
    // 48 name comparisons and checks for the leaf's issuer, 47 for I0's, 5 at the level of I1,
    // each level also comparing the anchor's name.
    assert!(work.considered <= 3 * 51, "{work:?}");
}

// ---------------------------------------------------------------------------------------------
// Chains built here, checked with KeyEcho.

/// A signature verifies when its bytes are the public key: the signer's key is the
/// signature.
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
const OID_SHA1_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x05];
const OID_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01];
const OID_BASIC: &[u8] = &[0x55, 0x1D, 0x13];
const OID_KEY_USAGE: &[u8] = &[0x55, 0x1D, 0x0F];
const OID_EKU: &[u8] = &[0x55, 0x1D, 0x25];
const OID_SAN: &[u8] = &[0x55, 0x1D, 0x11];
const OID_NC: &[u8] = &[0x55, 0x1D, 0x1E];
const OID_SKI: &[u8] = &[0x55, 0x1D, 0x0E];
const OID_AKI: &[u8] = &[0x55, 0x1D, 0x23];
const OID_SERVER_AUTH: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x01];
const OID_UNKNOWN: &[u8] = &[0x2A, 0x03, 0x04];

fn oid(content: &[u8]) -> Vec<u8> {
    tlv(0x06, content)
}

fn name(cn: &str) -> Vec<u8> {
    seq(&[&tlv(0x31, &seq(&[&oid(OID_CN), &tlv(0x0C, cn.as_bytes())]))])
}

fn extension(oid_content: &[u8], critical: bool, value: &[u8]) -> Vec<u8> {
    if critical {
        seq(&[&oid(oid_content), &[0x01, 0x01, 0xFF], &tlv(0x04, value)])
    } else {
        seq(&[&oid(oid_content), &tlv(0x04, value)])
    }
}

/// The 32 bytes of the Ed25519 key numbered `k`.
fn key(k: u16) -> Vec<u8> {
    let mut out = vec![0x5A; 32];
    out[..2].copy_from_slice(&k.to_be_bytes());
    out
}

/// keyUsage bits as the content of the BIT STRING value (unused bits, then the bytes).
const KU_DIGITAL_SIGNATURE: &[u8] = &[0x07, 0x80];
const KU_KEY_CERT_SIGN: &[u8] = &[0x02, 0x04];

/// A dNSName, an iPAddress and an rfc822Name as GeneralNames.
fn dns_name(name: &str) -> Vec<u8> {
    tlv(0x82, name.as_bytes())
}
fn ip_name(octets: &[u8]) -> Vec<u8> {
    tlv(0x87, octets)
}
fn email_name(name: &str) -> Vec<u8> {
    tlv(0x81, name.as_bytes())
}

/// The value of nameConstraints with the bases of `permitted` and `excluded`.
fn name_constraints(permitted: &[Vec<u8>], excluded: &[Vec<u8>]) -> Vec<u8> {
    let subtrees = |tag: u8, bases: &[Vec<u8>]| {
        let subtrees: Vec<Vec<u8>> = bases.iter().map(|b| seq(&[b])).collect();
        let refs: Vec<&[u8]> = subtrees.iter().map(Vec::as_slice).collect();
        tlv(tag, &refs.concat())
    };
    let mut parts = Vec::new();
    if !permitted.is_empty() {
        parts.push(subtrees(0xA0, permitted));
    }
    if !excluded.is_empty() {
        parts.push(subtrees(0xA1, excluded));
    }
    let refs: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
    seq(&refs)
}

/// One certificate to build.
#[derive(Clone)]
struct Cert {
    subject: String,
    issuer: String,
    key: u16,
    signer: u16,
    /// basicConstraints: `None` leaves it out; cA and pathLenConstraint.
    basic: Option<(bool, Option<u8>)>,
    basic_critical: bool,
    key_usage: Option<&'static [u8]>,
    eku: Option<Vec<&'static [u8]>>,
    sans: Vec<Vec<u8>>,
    nc: Option<Vec<u8>>,
    nc_critical: bool,
    ski: Option<Vec<u8>>,
    aki: Option<Vec<u8>>,
    not_after: &'static str,
    unknown_critical: bool,
    sha1: bool,
    rsa_bits_1024: bool,
    serial: u16,
}

impl Cert {
    fn ca(subject: &str, key: u16, issuer: &str, signer: u16) -> Cert {
        Cert {
            subject: subject.into(),
            issuer: issuer.into(),
            key,
            signer,
            basic: Some((true, None)),
            basic_critical: true,
            key_usage: None,
            eku: None,
            sans: Vec::new(),
            nc: None,
            nc_critical: true,
            ski: None,
            aki: None,
            not_after: "21250101000000Z",
            unknown_critical: false,
            sha1: false,
            rsa_bits_1024: false,
            serial: 1,
        }
    }

    fn leaf(host: &str, issuer: &str, signer: u16) -> Cert {
        Cert {
            basic: None,
            sans: vec![dns_name(host)],
            ..Cert::ca(&format!("leaf {host}"), 9999, issuer, signer)
        }
    }

    fn der(&self) -> Vec<u8> {
        let alg = if self.sha1 {
            seq(&[&oid(OID_SHA1_RSA), &[0x05, 0x00]])
        } else {
            seq(&[&oid(OID_ED25519)])
        };
        let spki = if self.rsa_bits_1024 {
            let mut modulus = vec![0x00, 0xC0];
            modulus.extend(std::iter::repeat_n(0x11, 127));
            let rsa_key = seq(&[&tlv(0x02, &modulus), &tlv(0x02, &[0x01, 0x00, 0x01])]);
            seq(&[
                &seq(&[&oid(OID_RSA), &[0x05, 0x00]]),
                &tlv(0x03, &[&[0x00][..], &rsa_key].concat()),
            ])
        } else {
            seq(&[
                &seq(&[&oid(OID_ED25519)]),
                &tlv(0x03, &[&[0x00][..], &key(self.key)].concat()),
            ])
        };
        let mut extensions = Vec::new();
        if let Some((ca, path_len)) = self.basic {
            let mut value = Vec::new();
            if ca {
                value.extend([0x01, 0x01, 0xFF]);
            }
            if let Some(n) = path_len {
                value.extend(tlv(0x02, &[n]));
            }
            extensions.push(extension(
                OID_BASIC,
                self.basic_critical,
                &tlv(0x30, &value),
            ));
        }
        if let Some(bits) = self.key_usage {
            extensions.push(extension(OID_KEY_USAGE, true, &tlv(0x03, bits)));
        }
        if let Some(purposes) = &self.eku {
            let oids: Vec<Vec<u8>> = purposes.iter().map(|p| oid(p)).collect();
            let refs: Vec<&[u8]> = oids.iter().map(Vec::as_slice).collect();
            extensions.push(extension(OID_EKU, false, &seq(&refs)));
        }
        if !self.sans.is_empty() {
            let refs: Vec<&[u8]> = self.sans.iter().map(Vec::as_slice).collect();
            extensions.push(extension(OID_SAN, false, &seq(&refs)));
        }
        if let Some(nc) = &self.nc {
            extensions.push(extension(OID_NC, self.nc_critical, nc));
        }
        if let Some(ski) = &self.ski {
            extensions.push(extension(OID_SKI, false, &tlv(0x04, ski)));
        }
        if let Some(aki) = &self.aki {
            extensions.push(extension(OID_AKI, false, &seq(&[&tlv(0x80, aki)])));
        }
        if self.unknown_critical {
            extensions.push(extension(OID_UNKNOWN, true, &[0x05, 0x00]));
        }
        let mut tbs: Vec<Vec<u8>> = vec![
            tlv(0xA0, &tlv(0x02, &[0x02])),
            tlv(0x02, &serial_bytes(self.serial)),
            alg.clone(),
            name(&self.issuer),
            seq(&[
                &tlv(0x17, b"250101000000Z"),
                &tlv(0x18, self.not_after.as_bytes()),
            ]),
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

/// 2026-11-22T00:00:00Z, the time of the library's tests (item 13).
fn now() -> Time {
    parse_time("2026-11-22T00:00:00Z")
}

/// The root `Root` with key 1, self-signed.
fn root() -> Cert {
    Cert::ca("Root", 1, "Root", 1)
}

fn verifier(roots: &[&Cert], limits: Limits) -> PathVerifier<KeyEcho> {
    let mut v = PathVerifier::new(Vec::new())
        .with_limits(limits)
        .with_verifier(KeyEcho);
    for r in roots {
        v.add_root(&r.der()).expect("a root");
    }
    v
}

fn check(
    roots: &[&Cert],
    sent: &[&Cert],
    host: &str,
    limits: Limits,
) -> (Result<LeafKey, CertificateRejection>, Work) {
    let ders: Vec<Vec<u8>> = sent.iter().map(|c| c.der()).collect();
    let refs: Vec<&[u8]> = ders.iter().map(Vec::as_slice).collect();
    verifier(roots, limits).verify_counting(&refs, host, now())
}

fn not_trusted(check: Check) -> Result<LeafKey, CertificateRejection> {
    Err(CertificateRejection::NotTrusted(check))
}

/// A chain of `n` intermediates `I1` (under the root) to `In` (the leaf's issuer), sent leaf
/// first.
fn straight(n: u16) -> Vec<Cert> {
    let mut out = vec![Cert::leaf("a.test", &format!("I{n}"), 100 + n)];
    for i in (1..=n).rev() {
        let (issuer, signer) = if i == 1 {
            ("Root".to_string(), 1)
        } else {
            (format!("I{}", i - 1), 100 + i - 1)
        };
        out.push(Cert::ca(&format!("I{i}"), 100 + i, &issuer, signer));
    }
    out
}

#[test]
fn the_bounds_of_item_9_have_their_values() {
    assert_eq!(INTERMEDIATES_MAX, 6);
    assert_eq!(SIGNATURE_CHECKS_MAX, 100);
    assert_eq!(CANDIDATES_MAX, 25_000);
    assert_eq!(NAME_CONSTRAINT_COMPARISONS_MAX, 250_000);
    assert_eq!(
        Limits::default(),
        Limits {
            intermediates: 6,
            signature_checks: 100,
            candidates: 25_000,
            name_constraint_comparisons: 250_000,
        }
    );
}

#[test]
fn a_straight_chain_returns_the_leaf_key() {
    let chain = straight(1);
    let sent: Vec<&Cert> = chain.iter().collect();
    let (result, work) = check(&[&root()], &sent, "a.test", Limits::default());
    assert_eq!(
        result,
        Ok(LeafKey {
            kind: PublicKeyKind::Ed25519,
            bytes: key(9999)
        })
    );
    assert_eq!(work.signature_checks, 2);
}

/// Item 9.3, the path depth: six intermediates pass, the seventh is not descended into; with
/// the bound set to 2, two pass and three do not.
#[test]
fn the_intermediates_bound_is_reached() {
    for (n, limit, ok) in [(6, 6, true), (7, 6, false), (2, 2, true), (3, 2, false)] {
        let chain = straight(n);
        let sent: Vec<&Cert> = chain.iter().collect();
        let limits = Limits {
            intermediates: limit,
            ..Limits::default()
        };
        let (result, work) = check(&[&root()], &sent, "a.test", limits);
        if ok {
            assert!(result.is_ok(), "{n} under {limit}: {result:?}");
        } else {
            assert_eq!(
                result,
                not_trusted(Check::PathTooLong(PathTooLong::Intermediates {
                    count: usize::from(n),
                    bound: limit
                })),
                "{n} under {limit}"
            );
            assert_eq!(work.signature_checks, limit, "the cut costs no check");
        }
    }
}

/// Item 9.4: the limit of signature checks, set to 3.
#[test]
fn the_signature_check_limit_is_reached() {
    let chain = straight(5);
    let sent: Vec<&Cert> = chain.iter().collect();
    let limits = Limits {
        signature_checks: 3,
        ..Limits::default()
    };
    let (result, work) = check(&[&root()], &sent, "a.test", limits);
    assert_eq!(
        result,
        not_trusted(Check::CandidateLimit(Limit::SignatureChecks { bound: 3 }))
    );
    assert_eq!(work.signature_checks, 3);
    let limits = Limits {
        signature_checks: 6,
        ..Limits::default()
    };
    assert!(check(&[&root()], &sent, "a.test", limits).0.is_ok());
}

/// Item 9.4: the limit of candidates considered, set to 10.
#[test]
fn the_candidate_limit_is_reached() {
    let chain = straight(5);
    let sent: Vec<&Cert> = chain.iter().collect();
    let limits = Limits {
        candidates: 10,
        ..Limits::default()
    };
    let (result, work) = check(&[&root()], &sent, "a.test", limits);
    assert_eq!(
        result,
        not_trusted(Check::CandidateLimit(Limit::CandidatesConsidered {
            bound: 10
        }))
    );
    assert_eq!(work.considered, 10);
}

fn serial_bytes(n: u16) -> Vec<u8> {
    if n < 0x80 {
        vec![n as u8]
    } else {
        n.to_be_bytes().to_vec()
    }
}

/// The leaf and `n` intermediates of one subject and one key identifier, each with its own
/// serial; intermediate `i` has the key `key_of(i)` and is signed by `signer_of(i)`, and the
/// leaf is signed by key `100`.
fn same_subject(n: u16, key_of: &dyn Fn(u16) -> u16, signer_of: &dyn Fn(u16) -> u16) -> Vec<Cert> {
    let mut leaf = Cert::leaf("a.test", "Same", 100);
    leaf.aki = Some(vec![0xAB; 20]);
    let mut out = vec![leaf];
    for i in 0..n {
        let mut ca = Cert::ca("Same", key_of(i), "Same", signer_of(i));
        ca.ski = Some(vec![0xAB; 20]);
        ca.aki = Some(vec![0xAB; 20]);
        ca.serial = 1000 + i;
        out.push(ca);
    }
    out
}

/// Hostile chains (item 9.4): many candidates of one subject and key identifier, none of which
/// leads to the anchor. When only the first intermediate verifies the leaf and no signature of
/// an intermediate verifies, every other candidate is checked once below the first and once at
/// the leaf's level: 2n - 1 checks, 39 at n = 20 and 79 at 2n = 40, and the limit of 100 caps
/// it at 60 and 120. When every intermediate verifies every other (one key for all), the
/// search is capped at 100 checks at either size and the names compared grow with n.
#[test]
fn many_candidates_of_one_subject_cost_work_linear_in_their_number() {
    let anchors = [&root()];
    let work_at = |n: u16, key_of: &dyn Fn(u16) -> u16, signer: &dyn Fn(u16) -> u16| {
        let chain = same_subject(n, key_of, signer);
        let sent: Vec<&Cert> = chain.iter().collect();
        check(&anchors, &sent, "a.test", Limits::default())
    };
    // Intermediate i has key 100 + i, so the first verifies the leaf; each is signed by a key
    // nobody holds.
    let own = |i: u16| 100 + i;
    let dead = |_: u16| 5000;
    let (r20, w20) = work_at(20, &own, &dead);
    let (r40, w40) = work_at(40, &own, &dead);
    println!("dead ends: n=20 {w20:?}, n=40 {w40:?}");
    assert_eq!(r20, not_trusted(Check::BadSignature { certificate: 1 }));
    assert_eq!(r40, not_trusted(Check::BadSignature { certificate: 1 }));
    assert_eq!((w20.signature_checks, w40.signature_checks), (39, 79));
    let limit = not_trusted(Check::CandidateLimit(Limit::SignatureChecks { bound: 100 }));
    let (r60, w60) = work_at(60, &own, &dead);
    let (r120, w120) = work_at(120, &own, &dead);
    assert_eq!((&r60, &r120), (&limit, &limit));
    assert_eq!((w60.signature_checks, w120.signature_checks), (100, 100));

    // One key for all, each signed by it: each verifies every other.
    let one = |_: u16| 100;
    let (r10, w10) = work_at(10, &one, &one);
    let (r20, w20) = work_at(20, &one, &one);
    println!("every one verifies: n=10 {w10:?}, n=20 {w20:?}");
    for r in [&r10, &r20] {
        assert_eq!(
            *r,
            not_trusted(Check::CandidateLimit(Limit::SignatureChecks { bound: 100 }))
        );
    }
    assert_eq!((w10.signature_checks, w20.signature_checks), (100, 100));
    // A step follows each check that verifies, and the first: at most 101 steps, each
    // comparing the anchor and the n sent but the leaf, so at most 101 (n + 1) names.
    assert!(w10.considered <= 101 * 11, "{w10:?}");
    assert!(w20.considered <= 101 * 21, "{w20:?}");
}

/// A cycle of intermediates that sign each other in turn, the leaf under the first and none of
/// them under the anchor: the search goes round the cycle until the path holds six
/// intermediates, the same work at n = 8 and at 2n = 16 but for the names compared, and two
/// that cross-sign end when the next is already on the path.
#[test]
fn a_cycle_of_cross_signed_intermediates_ends_at_the_bound() {
    let cycle = |n: u16| {
        let mut out = vec![Cert::leaf("a.test", "C0", 200)];
        for i in 0..n {
            let next = (i + 1) % n;
            out.push(Cert::ca(
                &format!("C{i}"),
                200 + i,
                &format!("C{next}"),
                200 + next,
            ));
        }
        out
    };
    let run = |n: u16| {
        let chain = cycle(n);
        let sent: Vec<&Cert> = chain.iter().collect();
        check(&[&root()], &sent, "a.test", Limits::default())
    };
    let (r2, w2) = run(2);
    assert_eq!(r2, not_trusted(Check::NoPathToAnchor));
    assert_eq!(w2.signature_checks, 2);
    let (r8, w8) = run(8);
    let (r16, w16) = run(16);
    println!("cycle: n=2 {w2:?}, n=8 {w8:?}, n=16 {w16:?}");
    let cut = not_trusted(Check::PathTooLong(PathTooLong::Intermediates {
        count: 7,
        bound: 6,
    }));
    assert_eq!(r8, cut);
    assert_eq!(r16, cut);
    assert_eq!((w8.signature_checks, w16.signature_checks), (6, 6));
    // Seven steps, the path holding p = 1 to 7 certificates; each compares the anchor and
    // the n + 1 - p sent ones not on the path: 7 (n - 2) in all.
    assert_eq!((w8.considered, w16.considered), (42, 98));
}

/// No certificate twice on a path: a chain that names itself as its issuer and one sent twice
/// do not loop.
#[test]
fn no_certificate_is_twice_on_a_path() {
    let me = Cert::ca("Loop", 300, "Loop", 300);
    let leaf = Cert::leaf("a.test", "Loop", 300);
    let (result, work) = check(
        &[&root()],
        &[&leaf, &me, &me, &me],
        "a.test",
        Limits::default(),
    );
    assert_eq!(result, not_trusted(Check::NoPathToAnchor));
    assert_eq!(work.signature_checks, 1, "{work:?}");
}

/// The anchors are candidates before the certificates sent: a sent copy of the root is not
/// needed and not used.
#[test]
fn an_anchor_ends_the_path_before_a_sent_copy_of_it() {
    let chain = straight(1);
    let r = root();
    let (result, work) = check(
        &[&r],
        &[&chain[0], &chain[1], &r],
        "a.test",
        Limits::default(),
    );
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(work.signature_checks, 2);
}

#[test]
fn an_issuer_without_ca_or_with_a_non_critical_basic_constraints_is_no_ca() {
    let mut chain = straight(1);
    chain[1].basic = Some((false, None));
    let sent: Vec<&Cert> = chain.iter().collect();
    let (result, _) = check(&[&root()], &sent, "a.test", Limits::default());
    assert_eq!(result, not_trusted(Check::IssuerNotCa { certificate: 1 }));
    chain[1].basic = None;
    let sent: Vec<&Cert> = chain.iter().collect();
    let (result, _) = check(&[&root()], &sent, "a.test", Limits::default());
    assert_eq!(result, not_trusted(Check::IssuerNotCa { certificate: 1 }));
    chain[1].basic = Some((true, None));
    chain[1].basic_critical = false;
    let sent: Vec<&Cert> = chain.iter().collect();
    let (result, _) = check(&[&root()], &sent, "a.test", Limits::default());
    assert_eq!(result, not_trusted(Check::IssuerNotCa { certificate: 1 }));
}

/// The root's own certificate is an issuer too, numbered after the certificates sent.
#[test]
fn the_root_is_checked_as_an_issuer_when_its_certificate_is_known() {
    let chain = straight(1);
    let sent: Vec<&Cert> = chain.iter().collect();
    let mut r = root();
    r.basic = None;
    let (result, _) = check(&[&r], &sent, "a.test", Limits::default());
    assert_eq!(result, not_trusted(Check::IssuerNotCa { certificate: 2 }));
    let mut r = root();
    r.not_after = "20260101000000Z";
    let (result, _) = check(&[&r], &sent, "a.test", Limits::default());
    assert!(
        matches!(
            result,
            Err(CertificateRejection::Expired { certificate: 2, .. })
        ),
        "{result:?}"
    );
    let mut r = root();
    r.key_usage = Some(KU_DIGITAL_SIGNATURE);
    let (result, _) = check(&[&r], &sent, "a.test", Limits::default());
    assert_eq!(
        result,
        not_trusted(Check::KeyUsage {
            certificate: 2,
            lacks: "keyCertSign"
        })
    );
    // Through a TrustAnchor alone, the root is its name and key: no check of its extensions.
    let anchor = TrustAnchor::from_der(&r.der()).expect("an anchor");
    let ders: Vec<Vec<u8>> = sent.iter().map(|c| c.der()).collect();
    let refs: Vec<&[u8]> = ders.iter().map(Vec::as_slice).collect();
    let result = PathVerifier::new(vec![anchor])
        .with_verifier(KeyEcho)
        .verify(&refs, "a.test", now());
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn a_path_len_constraint_counts_the_intermediates_below_but_not_self_issued_ones() {
    let mut chain = straight(3);
    // I1 (sent last) with pathLenConstraint 1: I2 and I3 below it are two.
    chain[3].basic = Some((true, Some(1)));
    let sent: Vec<&Cert> = chain.iter().collect();
    let (result, _) = check(&[&root()], &sent, "a.test", Limits::default());
    assert_eq!(
        result,
        not_trusted(Check::PathTooLong(PathTooLong::PathLenConstraint {
            certificate: 3
        }))
    );
    chain[3].basic = Some((true, Some(2)));
    let sent: Vec<&Cert> = chain.iter().collect();
    assert!(
        check(&[&root()], &sent, "a.test", Limits::default())
            .0
            .is_ok()
    );
    // A self-issued intermediate below a pathLenConstraint of 0 is not counted.
    let mut upper = Cert::ca("Upper", 400, "Root", 1);
    upper.basic = Some((true, Some(0)));
    let rollover = Cert::ca("Upper", 401, "Upper", 400);
    let leaf = Cert::leaf("a.test", "Upper", 401);
    let (result, _) = check(
        &[&root()],
        &[&leaf, &rollover, &upper],
        "a.test",
        Limits::default(),
    );
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn the_leaf_key_usage_and_extended_key_usage_are_checked_when_present() {
    let mut chain = straight(1);
    chain[0].key_usage = Some(KU_KEY_CERT_SIGN);
    let sent: Vec<&Cert> = chain.iter().collect();
    let (result, _) = check(&[&root()], &sent, "a.test", Limits::default());
    assert_eq!(
        result,
        not_trusted(Check::KeyUsage {
            certificate: 0,
            lacks: "digitalSignature"
        })
    );
    chain[0].key_usage = Some(KU_DIGITAL_SIGNATURE);
    chain[0].eku = Some(vec![&[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x02]]);
    let sent: Vec<&Cert> = chain.iter().collect();
    let (result, _) = check(&[&root()], &sent, "a.test", Limits::default());
    assert_eq!(result, not_trusted(Check::NoServerAuth));
    chain[0].eku = Some(vec![OID_SERVER_AUTH]);
    let sent: Vec<&Cert> = chain.iter().collect();
    assert!(
        check(&[&root()], &sent, "a.test", Limits::default())
            .0
            .is_ok()
    );
}

#[test]
fn an_unknown_critical_extension_on_any_certificate_of_the_path_refuses() {
    for at in [0, 1] {
        let mut chain = straight(1);
        chain[at].unknown_critical = true;
        let sent: Vec<&Cert> = chain.iter().collect();
        let (result, _) = check(&[&root()], &sent, "a.test", Limits::default());
        assert_eq!(
            result,
            not_trusted(Check::UnknownCriticalExtension {
                certificate: at,
                oid: "1.2.3.4".into()
            })
        );
    }
}

#[test]
fn a_weak_key_refuses_on_the_leaf_and_on_an_anchor() {
    let mut chain = straight(1);
    chain[0].rsa_bits_1024 = true;
    let sent: Vec<&Cert> = chain.iter().collect();
    let (result, work) = check(&[&root()], &sent, "a.test", Limits::default());
    assert_eq!(
        result,
        not_trusted(Check::WeakKey {
            certificate: 0,
            bits: 1024
        })
    );
    assert_eq!(work.signature_checks, 0);
    let mut weak_root = root();
    weak_root.rsa_bits_1024 = true;
    let leaf = Cert::leaf("a.test", "Root", 1);
    let (result, work) = check(&[&weak_root], &[&leaf], "a.test", Limits::default());
    assert_eq!(
        result,
        not_trusted(Check::WeakKey {
            certificate: 1,
            bits: 1024
        })
    );
    assert_eq!(work.signature_checks, 0);
    let mut sha1_leaf = Cert::leaf("a.test", "Root", 1);
    sha1_leaf.sha1 = true;
    let (result, work) = check(&[&root()], &[&sha1_leaf], "a.test", Limits::default());
    assert_eq!(
        result,
        not_trusted(Check::WeakAlgorithm {
            certificate: 0,
            algorithm: "sha1WithRSAEncryption"
        })
    );
    assert_eq!(work.signature_checks, 0);
}

/// Validity holds both its ends (RFC 5280 section 4.1.2.5: "the validity period ... includes
/// both notBefore and notAfter"): the leaf is good at its notBefore (2025-01-01T00:00:00Z)
/// and at its notAfter, and expired one second outside either.
#[test]
fn validity_includes_both_its_ends() {
    let mut leaf = Cert::leaf("a.test", "Root", 1);
    leaf.not_after = "20260101000000Z";
    let der = leaf.der();
    let at =
        |t: &str| verifier(&[&root()], Limits::default()).verify(&[&der], "a.test", parse_time(t));
    for good in ["2025-01-01T00:00:00Z", "2026-01-01T00:00:00Z"] {
        assert!(at(good).is_ok(), "{good}");
    }
    for bad in ["2024-12-31T23:59:59Z", "2026-01-01T00:00:01Z"] {
        assert!(
            matches!(
                at(bad),
                Err(CertificateRejection::Expired { certificate: 0, .. })
            ),
            "{bad}"
        );
    }
}

#[test]
fn the_host_is_checked_after_the_path() {
    let chain = straight(1);
    let sent: Vec<&Cert> = chain.iter().collect();
    let (result, _) = check(&[&root()], &sent, "b.test", Limits::default());
    assert_eq!(
        result,
        Err(CertificateRejection::NameMismatch {
            host: "b.test".into()
        })
    );
}

#[test]
fn an_unreadable_certificate_sent_refuses_with_the_reader_error() {
    let chain = straight(1);
    let leaf = chain[0].der();
    let junk = vec![0x30, 0x03, 0x02, 0x01];
    let refs: Vec<&[u8]> = vec![&leaf, &junk];
    let result = verifier(&[&root()], Limits::default()).verify(&refs, "a.test", now());
    assert!(
        matches!(
            result,
            Err(CertificateRejection::NotTrusted(Check::Unreadable {
                certificate: 1,
                ..
            }))
        ),
        "{result:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Name constraints.

/// A leaf with `sans` under an intermediate with the nameConstraints `nc`, checked for the host
/// `a.example.test`.
fn constrained(nc: Vec<u8>, sans: Vec<Vec<u8>>) -> Result<LeafKey, CertificateRejection> {
    constrained_for(nc, sans, "a.example.test")
}

fn constrained_for(
    nc: Vec<u8>,
    sans: Vec<Vec<u8>>,
    host: &str,
) -> Result<LeafKey, CertificateRejection> {
    let mut ica = Cert::ca("I1", 101, "Root", 1);
    ica.nc = Some(nc);
    let mut leaf = Cert::leaf("x", "I1", 101);
    leaf.sans = sans;
    check(&[&root()], &[&leaf, &ica], host, Limits::default()).0
}

fn nc_refused(name: &str) -> Result<LeafKey, CertificateRejection> {
    not_trusted(Check::NameConstraint {
        certificate: 1,
        name: name.into(),
    })
}

#[test]
fn dns_name_constraints_permit_and_exclude_whole_labels() {
    let permitted = |c: &str| name_constraints(&[dns_name(c)], &[]);
    let excluded = |c: &str| name_constraints(&[], &[dns_name(c)]);
    let san = vec![dns_name("a.example.test")];
    assert!(constrained(permitted("example.test"), san.clone()).is_ok());
    assert!(constrained(permitted("a.example.test"), san.clone()).is_ok());
    assert!(constrained(permitted("EXAMPLE.TEST"), san.clone()).is_ok());
    assert_eq!(
        constrained(permitted("ample.test"), san.clone()),
        nc_refused("a.example.test")
    );
    assert_eq!(
        constrained(excluded("example.test"), san.clone()),
        nc_refused("a.example.test")
    );
    assert!(constrained(excluded("b.example.test"), san.clone()).is_ok());
    // A constraint of another form leaves dNSNames free.
    let ip_only = name_constraints(&[ip_name(&[10, 0, 0, 0, 255, 0, 0, 0])], &[]);
    assert!(constrained(ip_only, san).is_ok());
}

#[test]
fn a_wildcard_name_is_held_to_every_name_it_matches() {
    let wildcard = vec![dns_name("*.example.test")];
    // Excluding one name the wildcard matches refuses it; one it cannot match does not.
    let excluded = |c: &str| name_constraints(&[], &[dns_name(c)]);
    assert_eq!(
        constrained(excluded("bar.example.test"), wildcard.clone()),
        nc_refused("*.example.test")
    );
    assert!(constrained(excluded("a.bar.example.test"), wildcard.clone()).is_ok());
    // Permitting one name the wildcard matches does not permit the wildcard.
    let permitted = |c: &str| name_constraints(&[dns_name(c)], &[]);
    assert_eq!(
        constrained(permitted("a.example.test"), wildcard.clone()),
        nc_refused("*.example.test")
    );
    assert!(constrained(permitted("example.test"), wildcard).is_ok());
}

#[test]
fn a_dns_constraint_outside_the_preferred_syntax_refuses() {
    for bad in [".example.test", "*.example.test"] {
        let nc = name_constraints(&[dns_name(bad)], &[]);
        assert_eq!(
            constrained(nc, vec![dns_name("a.example.test")]),
            nc_refused("a.example.test"),
            "{bad}"
        );
    }
}

#[test]
fn ip_constraints_compare_the_masked_octets() {
    let permitted = name_constraints(&[ip_name(&[192, 0, 2, 0, 255, 255, 255, 0])], &[]);
    assert!(
        constrained_for(
            permitted.clone(),
            vec![ip_name(&[192, 0, 2, 9])],
            "192.0.2.9"
        )
        .is_ok()
    );
    assert_eq!(
        constrained_for(
            permitted.clone(),
            vec![ip_name(&[192, 0, 3, 9])],
            "192.0.3.9"
        ),
        nc_refused("192.0.3.9")
    );
    // A permitted list of the iPAddress form holds every iPAddress name to it, an IPv6 one
    // under an IPv4 range too: no range of the list holds it.
    let mut v6 = [0u8; 16];
    v6[15] = 1;
    assert_eq!(
        constrained_for(permitted, vec![ip_name(&v6)], "::1"),
        nc_refused("::1")
    );
    let excluded = name_constraints(&[], &[ip_name(&[192, 0, 2, 0, 255, 255, 255, 0])]);
    assert_eq!(
        constrained_for(
            excluded.clone(),
            vec![ip_name(&[192, 0, 2, 1])],
            "192.0.2.1"
        ),
        nc_refused("192.0.2.1")
    );
    assert!(constrained_for(excluded, vec![ip_name(&[192, 0, 3, 1])], "192.0.3.1").is_ok());
    // A mask that is not ones then zeros (0xA0 = 1010 0000) is no range: a permitted one
    // holds no address, an excluded one holds every address (fail closed both ways).
    let gappy = [10, 0, 0, 0, 255, 0xA0, 0, 0];
    for nc in [
        name_constraints(&[ip_name(&gappy)], &[]),
        name_constraints(&[], &[ip_name(&gappy)]),
    ] {
        assert_eq!(
            constrained_for(nc, vec![ip_name(&[10, 0, 0, 1])], "10.0.0.1"),
            nc_refused("10.0.0.1")
        );
    }
}

/// Names of a form surf does not check (rfc822Name here) under a constraint of that form
/// refuse: the check fails closed.
#[test]
fn a_constraint_of_a_form_surf_does_not_check_refuses_a_name_of_that_form() {
    let nc = name_constraints(&[email_name("example.test")], &[]);
    let refused = constrained(
        nc.clone(),
        vec![dns_name("a.example.test"), email_name("a@example.test")],
    );
    assert!(
        matches!(
            refused,
            Err(CertificateRejection::NotTrusted(Check::NameConstraint {
                certificate: 1,
                ..
            }))
        ),
        "{refused:?}"
    );
    assert!(constrained(nc, vec![dns_name("a.example.test")]).is_ok());
}

/// The anchor's nameConstraints hold too, also through a TrustAnchor alone.
#[test]
fn the_anchor_name_constraints_hold() {
    let mut r = root();
    r.nc = Some(name_constraints(&[], &[dns_name("example.test")]));
    let leaf = Cert::leaf("a.example.test", "Root", 1);
    let (result, _) = check(&[&r], &[&leaf], "a.example.test", Limits::default());
    assert_eq!(
        result,
        not_trusted(Check::NameConstraint {
            certificate: 1,
            name: "a.example.test".into()
        })
    );
    let anchor = TrustAnchor::from_der(&r.der()).expect("an anchor");
    let der = leaf.der();
    let result = PathVerifier::new(vec![anchor])
        .with_verifier(KeyEcho)
        .verify(&[&der], "a.example.test", now());
    assert_eq!(
        result,
        not_trusted(Check::NameConstraint {
            certificate: 1,
            name: "a.example.test".into()
        })
    );
}

/// RFC 5280 section 6.1.3 (b) and (c): a self-issued intermediate below a constraint is not
/// held to it, the leaf always is, self-issued or not.
#[test]
fn a_self_issued_leaf_is_held_to_name_constraints_and_a_self_issued_intermediate_is_not() {
    let mut ica = Cert::ca("I1", 101, "Root", 1);
    ica.nc = Some(name_constraints(&[], &[dns_name("example.test")]));
    let mut leaf = Cert::leaf("a.example.test", "I1", 101);
    leaf.subject = "I1".into();
    let (result, _) = check(
        &[&root()],
        &[&leaf, &ica],
        "a.example.test",
        Limits::default(),
    );
    assert_eq!(result, nc_refused("a.example.test"));

    let mut rollover = Cert::ca("I1", 102, "I1", 101);
    rollover.sans = vec![dns_name("x.example.test")];
    let leaf = Cert::leaf("a.other.test", "I1", 102);
    let (result, _) = check(
        &[&root()],
        &[&leaf, &rollover, &ica],
        "a.other.test",
        Limits::default(),
    );
    assert!(result.is_ok(), "{result:?}");
}

/// RFC 5280 section 4.2.1.10: nameConstraints "MUST be marked critical" and are "used only in
/// a CA certificate". Not critical in an intermediate or in the anchor's certificate, or in a
/// leaf that is no CA, they refuse, even where every name lies inside them.
#[test]
fn name_constraints_not_critical_or_in_a_leaf_refuse() {
    let permitted = name_constraints(&[dns_name("example.test")], &[]);
    let refused = |certificate| {
        not_trusted(Check::NameConstraint {
            certificate,
            name: if certificate == 0 {
                "nameConstraints in an end-entity certificate".into()
            } else {
                "nameConstraints not marked critical".into()
            },
        })
    };
    let leaf = Cert::leaf("a.example.test", "I1", 101);
    let mut ica = Cert::ca("I1", 101, "Root", 1);
    ica.nc = Some(permitted.clone());
    let run = |r: &Cert, sent: &[&Cert]| check(&[r], sent, "a.example.test", Limits::default()).0;
    assert!(run(&root(), &[&leaf, &ica]).is_ok());
    ica.nc_critical = false;
    assert_eq!(run(&root(), &[&leaf, &ica]), refused(1));
    let mut r = root();
    r.nc = Some(permitted.clone());
    r.nc_critical = false;
    let direct = Cert::leaf("a.example.test", "Root", 1);
    assert_eq!(run(&r, &[&direct]), refused(1));
    let mut in_leaf = Cert::leaf("a.example.test", "Root", 1);
    in_leaf.nc = Some(permitted);
    assert_eq!(run(&root(), &[&in_leaf]), refused(0));
    // A leaf that is a CA may carry them.
    in_leaf.basic = Some((true, None));
    assert!(run(&root(), &[&in_leaf]).is_ok());
}

/// Item 9.4: when both carry one, a candidate's subjectKeyIdentifier must equal the
/// authorityKeyIdentifier of the certificate below, the anchor's certificate as well as a sent
/// one; a candidate whose identifier differs costs no signature check. Without one on either
/// side the names alone choose.
#[test]
fn key_identifiers_that_differ_rule_out_a_candidate() {
    let mut r = root();
    r.ski = Some(vec![0x01; 20]);
    let mut ica = Cert::ca("I1", 101, "Root", 1);
    ica.ski = Some(vec![0x02; 20]);
    ica.aki = Some(vec![0x01; 20]);
    let mut leaf = Cert::leaf("a.test", "I1", 101);
    leaf.aki = Some(vec![0x02; 20]);
    let run =
        |r: &Cert, leaf: &Cert, ica: &Cert| check(&[r], &[leaf, ica], "a.test", Limits::default());
    let (result, work) = run(&r, &leaf, &ica);
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(work.signature_checks, 2);
    let mut other = leaf.clone();
    other.aki = Some(vec![0x08; 20]);
    let (result, work) = run(&r, &other, &ica);
    assert_eq!(result, not_trusted(Check::NoPathToAnchor));
    assert_eq!(work.signature_checks, 0);
    let mut ica_other = ica.clone();
    ica_other.aki = Some(vec![0x09; 20]);
    let (result, work) = run(&r, &leaf, &ica_other);
    assert_eq!(result, not_trusted(Check::NoPathToAnchor));
    assert_eq!(work.signature_checks, 1);
    other.aki = None;
    let (result, _) = run(&r, &other, &ica);
    assert!(result.is_ok(), "{result:?}");
}

/// The work of name constraints is bounded (item 9): n names against m subtrees is n × m
/// comparisons, and the limit, set to 24 here, ends it.
#[test]
fn the_name_constraint_work_is_bounded() {
    let bases: Vec<Vec<u8>> = (0..5).map(|i| dns_name(&format!("d{i}.test"))).collect();
    let nc = name_constraints(&bases, &[]);
    let sans: Vec<Vec<u8>> = (0..5).map(|i| dns_name(&format!("h{i}.d4.test"))).collect();
    let mut ica = Cert::ca("I1", 101, "Root", 1);
    ica.nc = Some(nc);
    let mut leaf = Cert::leaf("h0.d4.test", "I1", 101);
    leaf.sans = sans;
    let run = |limit: usize| {
        let limits = Limits {
            name_constraint_comparisons: limit,
            ..Limits::default()
        };
        check(&[&root()], &[&leaf, &ica], "h0.d4.test", limits)
    };
    let (ok, work) = run(25);
    assert!(ok.is_ok(), "{ok:?}");
    assert_eq!(
        work.name_constraint_comparisons, 25,
        "5 names, each 5 subtrees"
    );
    let (refused, work) = run(24);
    assert_eq!(
        refused,
        not_trusted(Check::NameConstraint {
            certificate: 1,
            name: "the work of 24 name comparisons".into()
        })
    );
    assert_eq!(work.name_constraint_comparisons, 24);
}
