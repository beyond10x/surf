//! Adversary cases for `story:tls-x509`: inputs the unit's own tests do not hold, each asserting
//! what the story, RFC 5280 or the design note says the reader does with them, and a seeded
//! mutation run over certificates of x509-limbo that asserts no panic and the reader's own
//! invariants.

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use surf_html_conformance::json::{self, Value};
use surf_tls::der;
use surf_tls::pem;
use surf_tls::x509::{Certificate, Error, TrustAnchor};

use crate::x509_parse::{LIMBO_LEAF, Spec, seq, tlv};

fn oid(content: &[u8]) -> Vec<u8> {
    tlv(0x06, content)
}

fn extension(oid_content: &[u8], critical: bool, value: &[u8]) -> Vec<u8> {
    if critical {
        seq(&[&oid(oid_content), &[0x01, 0x01, 0xFF], &tlv(0x04, value)])
    } else {
        seq(&[&oid(oid_content), &tlv(0x04, value)])
    }
}

const OID_NC: &[u8] = &[0x55, 0x1D, 0x1E];
const OID_KEY_USAGE: &[u8] = &[0x55, 0x1D, 0x0F];
const OID_ECDSA_SHA512: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x04];
/// id-ml-dsa-44, 2.16.840.1.101.3.4.3.17 (RFC 9881).
const OID_ML_DSA_44: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x03, 0x11];

/// The certificate of `spec` with `tail` (raw bytes) in place of its extensions: what follows
/// the subject public key inside the TBS.
fn build_with_tail(spec: &Spec, tail: &[u8]) -> Vec<u8> {
    let version = tlv(0xA0, &tlv(0x02, spec.version.as_deref().unwrap_or(&[0x02])));
    let validity = seq(&[&spec.not_before, &spec.not_after]);
    let tbs = seq(&[
        &version,
        &spec.serial,
        &spec.inner_alg,
        &spec.issuer,
        &validity,
        &spec.subject,
        &spec.spki,
        tail,
    ]);
    let signature = tlv(
        0x03,
        &[0x00, 0x30, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 0x01],
    );
    seq(&[&tbs, &spec.outer_alg, &signature])
}

/// nameConstraints permitting the dNSName subtree `dns` only.
fn permitting(dns: &[u8]) -> Vec<u8> {
    seq(&[&tlv(0xA0, &seq(&[&tlv(0x82, dns)]))])
}

// ---------------------------------------------------------------------------------------------
// Meaning.

/// A root that carries nameConstraints must not become an anchor without them. The reader
/// refuses the three roots below (a nameConstraints with a `maximum`, nameConstraints twice, a
/// signature algorithm it does not know), and `TrustAnchor::from_der` then makes an anchor of
/// name and key alone: the constraints the root states are gone, and every name under it
/// passes.
#[test]
fn a_refused_root_with_name_constraints_is_no_unconstrained_anchor() {
    let mut bad_nc = Spec::p256();
    bad_nc.extensions.push(extension(
        OID_NC,
        true,
        &seq(&[&tlv(
            0xA0,
            &seq(&[&tlv(0x82, b"example.com"), &tlv(0x81, &[0x01])]),
        )]),
    ));
    let mut twice = Spec::p256();
    twice
        .extensions
        .push(extension(OID_NC, true, &permitting(b"example.com")));
    twice
        .extensions
        .push(extension(OID_NC, true, &permitting(b"example.org")));
    let mut unknown_alg = Spec::p256().signed_with(seq(&[&oid(OID_ML_DSA_44)]));
    unknown_alg
        .extensions
        .push(extension(OID_NC, true, &permitting(b"example.com")));

    let mut unconstrained = Vec::new();
    for (what, spec) in [
        ("nameConstraints with a maximum", bad_nc),
        ("nameConstraints twice", twice),
        ("an unknown signature algorithm", unknown_alg),
    ] {
        let der_bytes = spec.build();
        let refused = Certificate::from_der(&der_bytes).err();
        assert!(refused.is_some(), "{what}: the reader refuses the root");
        if let Ok(anchor) = TrustAnchor::from_der(&der_bytes)
            && anchor.name_constraints() == Ok(None)
        {
            unconstrained.push(format!("{what} ({:?})", anchor.refused));
        }
    }
    assert!(
        unconstrained.is_empty(),
        "anchors made without the nameConstraints their root carries: {unconstrained:?}"
    );
}

/// The story admits ECDSA with SHA-256 or SHA-384 (and names MD5 and SHA-1 to refuse); item 14
/// of the design has no ECDSA scheme with SHA-512. Any other signature algorithm is
/// `UnsupportedAlgorithm`.
#[test]
fn ecdsa_with_sha_512_is_an_unsupported_algorithm() {
    let der_bytes = Spec::p256()
        .signed_with(seq(&[&oid(OID_ECDSA_SHA512)]))
        .build();
    assert_eq!(
        Certificate::from_der(&der_bytes).map(|c| c.signature_algorithm),
        Err(Error::UnsupportedAlgorithm)
    );
}

/// What path validation needs to refuse the vendored limbo cases `rfc5280::ski::critical-ski`,
/// `rfc5280::aki::critical-aki`, `rfc5280::root-non-critical-basic-constraints`,
/// `rfc5280::nc::permitted-dns-match-noncritical` and
/// `rfc5280::san::noncritical-with-empty-subject` (all FAILURE): whether a known extension is
/// critical. The reader reads each known extension the same with and without the flag, so
/// `story:tls-x509-path` cannot tell them apart.
#[test]
fn the_critical_flag_of_a_known_extension_is_not_lost() {
    let known: [(&str, &[u8], Vec<u8>); 5] = [
        (
            "subjectKeyIdentifier",
            &[0x55, 0x1D, 0x0E],
            tlv(0x04, &[1, 2, 3, 4]),
        ),
        (
            "authorityKeyIdentifier",
            &[0x55, 0x1D, 0x23],
            seq(&[&tlv(0x80, &[1, 2, 3, 4])]),
        ),
        (
            "basicConstraints",
            &[0x55, 0x1D, 0x13],
            seq(&[&[0x01, 0x01, 0xFF]]),
        ),
        ("nameConstraints", OID_NC, permitting(b"example.com")),
        (
            "subjectAltName",
            &[0x55, 0x1D, 0x11],
            seq(&[&tlv(0x82, b"example.com")]),
        ),
    ];
    let mut lost = Vec::new();
    for (what, oid_content, value) in known {
        let read = |critical: bool| {
            let mut spec = Spec::p256();
            spec.extensions
                .retain(|e| !e.windows(oid_content.len()).any(|w| w == oid_content));
            spec.extensions
                .push(extension(oid_content, critical, &value));
            let der_bytes = spec.build();
            let cert = Certificate::from_der(&der_bytes).expect("reads");
            format!("{:?}", cert.extensions)
        };
        if read(true) == read(false) {
            lost.push(what);
        }
    }
    assert!(
        lost.is_empty(),
        "read the same critical and not critical: {lost:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// DER and RFC 5280 strictness.

/// `Extensions ::= SEQUENCE SIZE (1..MAX) OF Extension` (RFC 5280 section 4.1): `[3]` holding
/// an empty SEQUENCE is not a certificate.
#[test]
fn an_empty_extensions_sequence_is_refused() {
    let der_bytes = build_with_tail(&Spec::p256(), &tlv(0xA3, &seq(&[])));
    assert!(
        Certificate::from_der(&der_bytes).is_err(),
        "[3] with no extension read"
    );
}

/// issuerUniqueID is `[1] IMPLICIT BIT STRING`: an unused-bits byte of 9 is no BIT STRING, as
/// `der::Error::BadBitString` says for every other BIT STRING.
#[test]
fn a_unique_identifier_that_is_no_bit_string_is_refused() {
    let der_bytes = build_with_tail(&Spec::p256(), &tlv(0x81, &[0x09, 0xFF]));
    assert_eq!(
        Certificate::from_der(&der_bytes).map(|_| ()),
        Err(Error::Der(der::Error::BadBitString))
    );
}

/// RFC 5280 section 4.2.1.3: "When the keyUsage extension appears in a certificate, at least
/// one of the bits MUST be set to 1"; and X.690 section 11.2.2 drops the trailing zero bits of a
/// named bit list, so `03 03 07 80 00` is not DER.
#[test]
fn a_key_usage_with_no_bit_or_a_trailing_zero_byte_is_a_bad_extension() {
    for (what, value) in [
        ("no bit set", &[0x03, 0x01, 0x00][..]),
        ("a trailing zero byte", &[0x03, 0x03, 0x07, 0x80, 0x00][..]),
    ] {
        let mut spec = Spec::p256();
        spec.extensions[1] = extension(OID_KEY_USAGE, true, value);
        assert_eq!(
            Certificate::from_der(&spec.build()).map(|c| c.extensions.key_usage),
            Err(Error::BadExtension),
            "{what}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Lines no test of the unit holds: each case below fails on a one-line mutant of the reader
// that the unit's 121 tests pass (the mutant is named in each).

/// der.rs:194 `length < 0x80` as `length < 0x7F`: `81 7F` fits the short form.
#[test]
fn length_81_7f_is_not_minimal() {
    let mut input = vec![0x04, 0x81, 0x7F];
    input.extend_from_slice(&[0u8; 0x7F]);
    assert_eq!(der::parse(&input), Err(der::Error::NonMinimalLength));
}

/// der.rs:638, the rule of 400 as a rule of 200: 2200 is no leap year, 2400 is.
#[test]
fn february_29_of_2200_is_bad_and_of_2400_reads() {
    let time = |text: &[u8]| der::parse(&tlv(0x18, text)).expect("a value").time();
    assert_eq!(time(b"22000229000000Z"), Err(der::Error::BadTime));
    assert!(time(b"24000229000000Z").is_ok());
}

/// x509/mod.rs:150 dropped: a signature whose BIT STRING has unused bits is refused, though the
/// unused bits are zero.
#[test]
fn a_signature_with_unused_bits_is_a_bad_bit_string() {
    let spec = Spec::p256();
    let mut tbs: Vec<Vec<u8>> = vec![tlv(0xA0, &tlv(0x02, &[0x02]))];
    tbs.extend([
        spec.serial.clone(),
        spec.inner_alg.clone(),
        spec.issuer.clone(),
        seq(&[&spec.not_before, &spec.not_after]),
        spec.subject.clone(),
        spec.spki.clone(),
    ]);
    let refs: Vec<&[u8]> = tbs.iter().map(Vec::as_slice).collect();
    let signature = tlv(
        0x03,
        &[0x01, 0x30, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 0x02],
    );
    let der_bytes = seq(&[&seq(&refs), &spec.outer_alg, &signature]);
    assert_eq!(
        Certificate::from_der(&der_bytes).map(|_| ()),
        Err(Error::Der(der::Error::BadBitString))
    );
}

/// x509/extensions.rs:171 without `out.unknown_critical.is_none()`: of two unknown critical
/// extensions the first is named, as the field's documentation says.
#[test]
fn the_first_unknown_critical_extension_is_named() {
    let first = [0x2B, 0x06, 0x01, 0x04, 0x01, 0x82, 0x37, 0x01];
    let second = [0x2B, 0x06, 0x01, 0x04, 0x01, 0x82, 0x37, 0x02];
    let mut spec = Spec::p256();
    spec.extensions.push(extension(&first, true, &[0x05, 0x00]));
    spec.extensions
        .push(extension(&second, true, &[0x05, 0x00]));
    let der_bytes = spec.build();
    let cert = Certificate::from_der(&der_bytes).expect("reads");
    assert_eq!(cert.extensions.unknown_critical, Some(&first[..]));
}

/// x509/extensions.rs:201 with a negative pathLenConstraint read as absent: `pathLenConstraint
/// INTEGER (0..MAX)`, so -1 is a bad extension, not "no limit".
#[test]
fn a_negative_path_len_constraint_is_a_bad_extension() {
    let mut spec = Spec::p256();
    spec.extensions[0] = extension(
        &[0x55, 0x1D, 0x13],
        true,
        &seq(&[&[0x01, 0x01, 0xFF], &[0x02, 0x01, 0xFF]]),
    );
    assert_eq!(
        Certificate::from_der(&spec.build()).map(|c| c.extensions.basic_constraints),
        Err(Error::BadExtension)
    );
}

/// x509/extensions.rs:285 with any minimum but 1 taken: a subtree's minimum other than 0 is a
/// bad extension (RFC 5280 section 4.2.1.10), 2 as well as 1.
#[test]
fn a_subtree_minimum_other_than_zero_is_a_bad_extension() {
    for min in [0x01, 0x02] {
        let nc = seq(&[&tlv(
            0xA0,
            &seq(&[&tlv(0x82, b"example.com"), &tlv(0x80, &[min])]),
        )]);
        let mut spec = Spec::p256();
        spec.extensions.push(extension(OID_NC, true, &nc));
        assert_eq!(
            Certificate::from_der(&spec.build()).map(|_| ()),
            Err(Error::BadExtension),
            "minimum {min}"
        );
    }
}

/// x509/name.rs:59 with the empty dNSName refused everywhere: as a base of nameConstraints the
/// empty dNSName reads (only a subjectAltName refuses it, as `GeneralName::Dns` says).
#[test]
fn an_empty_dns_name_reads_as_a_name_constraint() {
    let mut spec = Spec::p256();
    spec.extensions
        .push(extension(OID_NC, true, &permitting(b"")));
    let der_bytes = spec.build();
    let cert = Certificate::from_der(&der_bytes).expect("reads");
    let nc = cert.extensions.name_constraints.expect("nameConstraints");
    assert_eq!(
        nc.permitted,
        Some(vec![surf_tls::x509::GeneralName::Dns("")])
    );
}

/// x509/name.rs:109 dropped: a relative distinguished name with no attribute is a bad name.
#[test]
fn an_empty_relative_distinguished_name_is_a_bad_name() {
    let spec = Spec {
        subject: seq(&[&tlv(0x31, &[])]),
        ..Spec::p256()
    };
    assert_eq!(
        Certificate::from_der(&spec.build()).map(|_| ()),
        Err(Error::BadName)
    );
}

/// x509/algorithm.rs:167 without `bytes[0] != 0x04`: a P-256 key of 65 bytes that is not an
/// uncompressed point (first byte 0x02) is a bad key.
#[test]
fn a_p256_key_that_is_no_uncompressed_point_is_bad() {
    let mut point = vec![0x02];
    point.extend(1..=64u8);
    let spec = Spec {
        spki: seq(&[
            &seq(&[
                &oid(&[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01]),
                &oid(&[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07]),
            ]),
            &tlv(0x03, &[&[0x00][..], &point].concat()),
        ]),
        ..Spec::p256()
    };
    assert_eq!(
        Certificate::from_der(&spec.build()).map(|_| ()),
        Err(Error::BadKey)
    );
}

/// The SubjectPublicKeyInfo of an RSA key with the modulus `modulus` (an INTEGER's content).
fn rsa_spki(modulus: &[u8]) -> Vec<u8> {
    let key = seq(&[&tlv(0x02, modulus), &tlv(0x02, &[0x01, 0x00, 0x01])]);
    seq(&[
        &seq(&[
            &oid(&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01]),
            &[0x05, 0x00],
        ]),
        &tlv(0x03, &[&[0x00][..], &key].concat()),
    ])
}

/// x509/algorithm.rs:213 without `- top.leading_zeros()`: a modulus of 256 bytes whose top
/// byte is 0x40 has 2047 bits, under the 2048 of `REFUSED`; counted by its bytes it would pass
/// as 2048.
#[test]
fn an_rsa_modulus_of_2047_bits_is_read_as_2047() {
    let mut modulus = vec![0x40];
    modulus.extend((0..255u32).map(|i| (i % 251) as u8 | 1));
    let spec = Spec {
        spki: rsa_spki(&modulus),
        ..Spec::rsa()
    };
    let der_bytes = spec.build();
    let cert = Certificate::from_der(&der_bytes).expect("reads");
    assert_eq!(
        cert.public_key.kind,
        surf_tls::crypto::PublicKeyKind::Rsa { bits: 2047 }
    );
    assert!(
        surf_tls::crypto::refusal(cert.signature_algorithm, cert.public_key.kind).is_some(),
        "under 2048 bits is refused"
    );
}

/// x509/mod.rs:188, issuerUniqueID not skipped: a version 3 certificate may carry
/// issuerUniqueID and subjectUniqueID before its extensions (RFC 5280 section 4.1).
#[test]
fn unique_identifiers_before_the_extensions_are_read_past() {
    let spec = Spec::p256();
    let refs: Vec<&[u8]> = spec.extensions.iter().map(Vec::as_slice).collect();
    let tail = [
        tlv(0x81, &[0x00, 0x01, 0x02]),
        tlv(0x82, &[0x00, 0x03, 0x04]),
        tlv(0xA3, &seq(&refs)),
    ]
    .concat();
    let der_bytes = build_with_tail(&spec, &tail);
    let cert = Certificate::from_der(&der_bytes).expect("reads");
    assert!(cert.extensions.subject_alt_name.is_some());
}

/// x509/algorithm.rs:49 with any parameters taken: sha256WithRSAEncryption takes a NULL or no
/// parameters (RFC 4055 section 5); others are an unsupported algorithm.
#[test]
fn rsa_pkcs1_with_parameters_other_than_null_is_unsupported() {
    let sha256_rsa = oid(&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B]);
    for params in [seq(&[]), vec![0x02, 0x01, 0x00]] {
        let spec = Spec::rsa().signed_with(seq(&[&sha256_rsa, &params]));
        assert_eq!(
            Certificate::from_der(&spec.build()).map(|_| ()),
            Err(Error::UnsupportedAlgorithm),
            "{params:02x?}"
        );
    }
}

/// pem.rs:115 dropped: a BEGIN inside an open block is an unterminated block, not content.
#[test]
fn a_begin_inside_an_open_block_is_unterminated() {
    let text = b"-----BEGIN A-----\nQUFB\n-----BEGIN B-----\nQUFB\n-----END B-----\n";
    assert_eq!(pem::parse(text), Err(pem::Error::UnterminatedBlock));
}

/// pem.rs:93 without the tab: a tab after the dashes of a BEGIN or END line is white space at
/// the end of the line, as a space and a CR are.
#[test]
fn a_tab_after_the_begin_and_end_lines_is_skipped() {
    let text = b"-----BEGIN A-----\t\nQUFB\n-----END A-----\t\n";
    assert_eq!(
        pem::parse(text).map(|blocks| blocks
            .into_iter()
            .map(|b| (b.label, b.data))
            .collect::<Vec<_>>()),
        Ok(vec![("A".to_string(), b"AAA".to_vec())])
    );
}

/// base64.rs:54, :59 and :40: the bits a padded group leaves over must be zero, each of them
/// (`QI==` sets the highest of the four, `QUC=` the higher of the two), and no character
/// follows a pad (`QQ=Q`).
#[test]
fn base64_with_a_left_over_bit_or_a_character_after_a_pad_is_bad() {
    for body in ["QI==", "QUC=", "QQ=Q"] {
        let text = format!("-----BEGIN A-----\n{body}\n-----END A-----\n");
        assert_eq!(
            pem::parse(text.as_bytes()),
            Err(pem::Error::BadBase64),
            "{body}"
        );
    }
}

/// x509/algorithm.rs:203, the exponent's sign not checked: a negative public exponent is a bad
/// key.
#[test]
fn a_negative_rsa_exponent_is_a_bad_key() {
    let mut modulus = vec![0x00, 0xC0];
    modulus.extend((0..255u32).map(|i| (i % 251) as u8 | 1));
    let key = seq(&[&tlv(0x02, &modulus), &tlv(0x02, &[0x81, 0x00, 0x01])]);
    let spki = seq(&[
        &seq(&[
            &oid(&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01]),
            &[0x05, 0x00],
        ]),
        &tlv(0x03, &[&[0x00][..], &key].concat()),
    ]);
    let spec = Spec {
        spki,
        ..Spec::rsa()
    };
    assert_eq!(
        Certificate::from_der(&spec.build()).map(|_| ()),
        Err(Error::BadKey)
    );
}

// ---------------------------------------------------------------------------------------------
// A seeded mutation run: no panic, and the reader's invariants.

/// xorshift64*, seeded: the same inputs on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

const EDGE_BYTES: [u8; 12] = [
    0x00, 0x01, 0x7F, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x88, 0xFE, 0xFF,
];

fn mutate(rng: &mut Rng, input: &[u8]) -> Vec<u8> {
    let mut out = input.to_vec();
    for _ in 0..=rng.below(3) {
        if out.is_empty() {
            out.push(0x30);
        }
        let i = rng.below(out.len());
        match rng.below(8) {
            0 => out[i] ^= 1 << rng.below(8),
            1 => out[i] = rng.next() as u8,
            2 => out[i] = EDGE_BYTES[rng.below(EDGE_BYTES.len())],
            3 => out.insert(i, rng.next() as u8),
            4 => {
                out.remove(i);
            }
            5 => out.truncate(i),
            6 => {
                let end = (i + 1 + rng.below(64)).min(out.len());
                let slice = out[i..end].to_vec();
                let at = rng.below(out.len());
                out.splice(at..at, slice);
            }
            _ => {
                let j = rng.below(out.len());
                out.swap(i, j);
            }
        }
    }
    out
}

/// Reads `bytes` every way the crate offers and checks what holds for every input.
fn exercise(bytes: &[u8]) -> bool {
    let parsed = der::parse(bytes);
    let anchor = TrustAnchor::from_der(bytes);
    match Certificate::from_der(bytes) {
        Ok(cert) => {
            assert!(parsed.is_ok(), "a certificate read is DER");
            assert_eq!(cert.der, bytes);
            assert!(der::parse(cert.tbs).is_ok(), "its TBS is one DER value");
            assert_eq!(cert.version, 3);
            assert!(cert.serial.len() <= surf_tls::x509::SERIAL_BYTES_MAX);
            let anchor = anchor.expect("a certificate read is an anchor");
            assert_eq!(anchor.refused, None);
            assert_eq!(anchor.subject(), cert.subject);
            let _ = anchor.name_constraints();
            if let Some(unknown) = cert.extensions.unknown_critical
                && unknown.len() <= 64
            {
                let _ = der::oid_to_string(unknown);
            }
            true
        }
        Err(refused) => {
            if let Ok(anchor) = anchor {
                assert_eq!(anchor.refused, Some(refused));
                let _ = anchor.name_constraints();
            }
            false
        }
    }
}

/// The certificates of `limbo-other.json`, each distinct one once.
fn limbo_certificates() -> Vec<Vec<u8>> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/x509-limbo/limbo-other.json");
    let data = fs::read_to_string(path).expect("read limbo-other.json");
    let root = json::parse(&data).expect("JSON");
    let Some(Value::Array(cases)) = root.get("testcases") else {
        panic!("no testcases");
    };
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mut seen: HashSet<Vec<u8>> = HashSet::new();
    let mut texts: Vec<&Value> = Vec::new();
    for case in cases {
        for key in ["trusted_certs", "untrusted_intermediates"] {
            if let Some(Value::Array(items)) = case.get(key) {
                texts.extend(items);
            }
        }
        texts.extend(case.get("peer_certificate"));
    }
    for text in texts {
        let Value::String(points) = text else {
            continue;
        };
        let text: String = points.iter().filter_map(|&p| char::from_u32(p)).collect();
        for block in pem::parse(text.as_bytes()).unwrap_or_default() {
            if seen.insert(block.data.clone()) {
                out.push(block.data);
            }
        }
    }
    out
}

/// The seed and the count of the run in the test step; `ADVERSARY_SEED` and
/// `ADVERSARY_MUTATIONS` change them for a longer run by hand.
const SEED: u64 = 0x005E_ED0F_A15E_CE27;
const MUTATIONS: usize = 2_000;

#[test]
fn seeded_mutations_of_limbo_certificates_read_or_are_refused_without_panic() {
    let mut corpus = limbo_certificates();
    assert!(corpus.len() > 900, "{} certificates", corpus.len());
    corpus.push(Spec::p256().build());
    corpus.push(Spec::rsa().build());
    let count = std::env::var("ADVERSARY_MUTATIONS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(MUTATIONS);
    let seed = std::env::var("ADVERSARY_SEED")
        .ok()
        .and_then(|n| u64::from_str_radix(n.trim_start_matches("0x"), 16).ok())
        .unwrap_or(SEED)
        | 1;
    let mut rng = Rng(seed);
    let mut read = 0usize;
    for _ in 0..count {
        let base = &corpus[rng.below(corpus.len())];
        let input = mutate(&mut rng, base);
        read += usize::from(exercise(&input));
    }
    // PEM: mutations of the text of the limbo leaf.
    for _ in 0..count / 10 {
        let text = mutate(&mut rng, LIMBO_LEAF.as_bytes());
        if let Ok(blocks) = pem::parse(&text) {
            for block in blocks {
                let _ = exercise(&block.data);
            }
        }
    }
    println!(
        "adversary: seed {seed:#x}, {count} mutations of {} certificates, {read} read",
        corpus.len()
    );
}

// ---------------------------------------------------------------------------------------------
// The vendored data, read as the specification it is.

/// The trusted certificates of the limbo case `id` in `limbo-other.json`.
fn limbo_trusted(id: &str) -> Vec<Vec<u8>> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/x509-limbo/limbo-other.json");
    let data = fs::read_to_string(path).expect("read limbo-other.json");
    let root = json::parse(&data).expect("JSON");
    let Some(Value::Array(cases)) = root.get("testcases") else {
        panic!("no testcases");
    };
    let text = |v: &Value| match v {
        Value::String(p) => p.iter().filter_map(|&c| char::from_u32(c)).collect(),
        _ => String::new(),
    };
    let case = cases
        .iter()
        .find(|c| c.get("id").map(text).as_deref() == Some(id))
        .expect("the case");
    let Some(Value::Array(trusted)) = case.get("trusted_certs") else {
        panic!("{id} has no trusted_certs");
    };
    trusted
        .iter()
        .flat_map(|t| pem::parse(text(t).as_bytes()).expect("PEM"))
        .map(|b| b.data)
        .collect()
}

/// limbo `rfc5280::nc::invalid-ipv4-address` and `invalid-ipv6-address` (FAILURE): the root's
/// nameConstraints hold an iPAddress of a wrong length. The reader refuses the root (`BadName`),
/// and `TrustAnchor::from_der` takes it as name and key: an anchor with no constraint at all,
/// so the leaf under it is trusted for any name and the case limbo expects to fail passes.
#[test]
fn the_limbo_roots_with_a_malformed_ip_constraint_give_no_unconstrained_anchor() {
    let mut unconstrained = Vec::new();
    for id in [
        "rfc5280::nc::invalid-ipv4-address",
        "rfc5280::nc::invalid-ipv6-address",
    ] {
        for der_bytes in limbo_trusted(id) {
            if let Ok(anchor) = TrustAnchor::from_der(&der_bytes)
                && anchor.name_constraints() == Ok(None)
            {
                unconstrained.push(format!("{id}: refused {:?}", anchor.refused));
            }
        }
    }
    assert!(
        unconstrained.is_empty(),
        "unconstrained anchors from constrained roots: {unconstrained:?}"
    );
}
