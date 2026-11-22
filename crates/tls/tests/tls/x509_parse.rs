//! The X.509 reader, `surf_tls::x509`, and the types of `surf_tls::verify` (`story:tls-x509`):
//! one certificate of x509-limbo read into its fields (acceptance 1), hostile certificates
//! refused with their named error (acceptance 3), and the closed list of checks of item 7
//! (acceptance 7). The hostile certificates are built here by a small DER writer of the test;
//! their signatures are not real, since the reader verifies none.

#![forbid(unsafe_code)]

use std::fs;
use std::path::PathBuf;

use surf_tls::crypto::{self, PublicKeyKind, Refusal, Scheme, SignatureAlgorithm, SignatureHash};
use surf_tls::der::{self, Time};
use surf_tls::error::{AlertDescription, Error as TlsError};
use surf_tls::pem;
use surf_tls::verify::{CertificateRejection, Check, ServerCertVerifier};
use surf_tls::x509::{
    BasicConstraints, Certificate, Critical, Error, GeneralName, KeyUsage, LeafKey,
    SERIAL_BYTES_MAX, TrustAnchor,
};

// ---------------------------------------------------------------------------------------------
// A DER writer for the tests.

/// One value of tag `tag` around `content`, its length in the shortest form.
pub fn tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    let n = content.len();
    if n < 0x80 {
        out.push(n as u8);
    } else {
        let bytes = n.to_be_bytes();
        let skip = bytes.iter().take_while(|&&b| b == 0).count();
        out.push(0x80 | (bytes.len() - skip) as u8);
        out.extend_from_slice(&bytes[skip..]);
    }
    out.extend_from_slice(content);
    out
}

/// A SEQUENCE of `parts`.
pub fn seq(parts: &[&[u8]]) -> Vec<u8> {
    tlv(0x30, &parts.concat())
}

fn oid(content: &[u8]) -> Vec<u8> {
    tlv(0x06, content)
}

const NULL: [u8; 2] = [0x05, 0x00];

const OID_ECDSA_SHA256: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x02];
const OID_SHA256_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B];
const OID_MD5_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x04];
const OID_RSA_PSS: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0A];
const OID_MGF1: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x08];
const OID_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01];
const OID_SHA256: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
const OID_SHA384: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02];
const OID_EC_PUBLIC: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01];
const OID_P256: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07];
const OID_P521: &[u8] = &[0x2B, 0x81, 0x04, 0x00, 0x23];
const OID_CN: &[u8] = &[0x55, 0x04, 0x03];
const OID_SAN: &[u8] = &[0x55, 0x1D, 0x11];
const OID_BASIC: &[u8] = &[0x55, 0x1D, 0x13];
const OID_KEY_USAGE: &[u8] = &[0x55, 0x1D, 0x0F];
const OID_EKU: &[u8] = &[0x55, 0x1D, 0x25];
const OID_NC: &[u8] = &[0x55, 0x1D, 0x1E];
const OID_SKI: &[u8] = &[0x55, 0x1D, 0x0E];
const OID_AKI: &[u8] = &[0x55, 0x1D, 0x23];
const OID_POLICIES: &[u8] = &[0x55, 0x1D, 0x20];
const OID_SERVER_AUTH: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x01];

fn alg(oid_content: &[u8], params: Option<&[u8]>) -> Vec<u8> {
    match params {
        Some(p) => seq(&[&oid(oid_content), p]),
        None => seq(&[&oid(oid_content)]),
    }
}

/// The AlgorithmIdentifier of RSASSA-PSS of RFC 4055 with `hash`, MGF1 of `mgf_hash`, `salt`
/// and, if given, `trailer`.
fn pss(hash: &[u8], mgf_hash: &[u8], salt: u8, trailer: Option<u8>) -> Vec<u8> {
    let hash_alg = alg(hash, Some(&NULL));
    let mgf = seq(&[&oid(OID_MGF1), &alg(mgf_hash, Some(&NULL))]);
    let mut params = vec![
        tlv(0xA0, &hash_alg),
        tlv(0xA1, &mgf),
        tlv(0xA2, &tlv(0x02, &[salt])),
    ];
    if let Some(t) = trailer {
        params.push(tlv(0xA3, &tlv(0x02, &[t])));
    }
    let refs: Vec<&[u8]> = params.iter().map(Vec::as_slice).collect();
    alg(OID_RSA_PSS, Some(&seq(&refs)))
}

fn name_cn(cn: &str) -> Vec<u8> {
    seq(&[&tlv(0x31, &seq(&[&oid(OID_CN), &tlv(0x0C, cn.as_bytes())]))])
}

fn extension(oid_content: &[u8], critical: bool, value: &[u8]) -> Vec<u8> {
    if critical {
        seq(&[&oid(oid_content), &[0x01, 0x01, 0xFF], &tlv(0x04, value)])
    } else {
        seq(&[&oid(oid_content), &tlv(0x04, value)])
    }
}

fn san(names: &[Vec<u8>]) -> Vec<u8> {
    let refs: Vec<&[u8]> = names.iter().map(Vec::as_slice).collect();
    extension(OID_SAN, false, &seq(&refs))
}

/// A P-256 point: 0x04, then 64 bytes. Not on the curve; the reader does not check that.
fn p256_point() -> Vec<u8> {
    let mut point = vec![0x04];
    point.extend(1..=64u8);
    point
}

/// The modulus of an RSA key of 2048 bits as an INTEGER's content: 0x00, then 256 bytes.
fn rsa_modulus() -> Vec<u8> {
    let mut n = vec![0x00, 0xC0];
    n.extend((0..255u32).map(|i| (i % 251) as u8 | 1));
    n
}

/// The parts of a certificate, each as the bytes of its whole value.
#[derive(Clone)]
pub struct Spec {
    /// The content of the INTEGER inside `[0]`; `None` leaves `[0]` out (version 1).
    pub version: Option<Vec<u8>>,
    pub serial: Vec<u8>,
    pub inner_alg: Vec<u8>,
    pub outer_alg: Vec<u8>,
    pub issuer: Vec<u8>,
    pub not_before: Vec<u8>,
    pub not_after: Vec<u8>,
    pub subject: Vec<u8>,
    pub spki: Vec<u8>,
    /// Each extension's SEQUENCE; empty leaves `[3]` out.
    pub extensions: Vec<Vec<u8>>,
}

impl Spec {
    /// A version 3 certificate with a P-256 key, signed with ecdsa-with-SHA256, with
    /// basicConstraints, keyUsage, extKeyUsage and subjectAltName.
    pub fn p256() -> Spec {
        let ecdsa = alg(OID_ECDSA_SHA256, None);
        Spec {
            version: Some(vec![0x02]),
            serial: tlv(0x02, &[0x01, 0x02, 0x03]),
            inner_alg: ecdsa.clone(),
            outer_alg: ecdsa,
            issuer: name_cn("Example CA"),
            not_before: tlv(0x17, b"250101000000Z"),
            not_after: tlv(0x18, b"20991231235959Z"),
            subject: name_cn("example.com"),
            spki: seq(&[
                &seq(&[&oid(OID_EC_PUBLIC), &oid(OID_P256)]),
                &tlv(0x03, &[&[0x00][..], &p256_point()].concat()),
            ]),
            extensions: vec![
                extension(OID_BASIC, true, &seq(&[])),
                extension(OID_KEY_USAGE, true, &[0x03, 0x02, 0x07, 0x80]),
                extension(OID_EKU, false, &seq(&[&oid(OID_SERVER_AUTH)])),
                san(&[tlv(0x82, b"example.com"), tlv(0x87, &[192, 0, 2, 1])]),
            ],
        }
    }

    /// The same with an RSA key of 2048 bits, signed with sha256WithRSAEncryption.
    pub fn rsa() -> Spec {
        let rsa_key = seq(&[&tlv(0x02, &rsa_modulus()), &tlv(0x02, &[0x01, 0x00, 0x01])]);
        Spec {
            spki: seq(&[
                &alg(OID_RSA, Some(&NULL)),
                &tlv(0x03, &[&[0x00][..], &rsa_key].concat()),
            ]),
            ..Spec::p256()
        }
        .signed_with(alg(OID_SHA256_RSA, Some(&NULL)))
    }

    /// The same, inner and outer algorithm both `algorithm`.
    pub fn signed_with(self, algorithm: Vec<u8>) -> Spec {
        Spec {
            inner_alg: algorithm.clone(),
            outer_alg: algorithm,
            ..self
        }
    }

    /// The DER of the certificate.
    pub fn build(&self) -> Vec<u8> {
        let tail = if self.extensions.is_empty() {
            Vec::new()
        } else {
            let refs: Vec<&[u8]> = self.extensions.iter().map(Vec::as_slice).collect();
            tlv(0xA3, &seq(&refs))
        };
        self.build_with_tail(&tail)
    }

    /// The DER of the certificate with `tail` (raw bytes) after the subject public key, in place
    /// of the extensions.
    pub fn build_with_tail(&self, tail: &[u8]) -> Vec<u8> {
        let mut tbs: Vec<Vec<u8>> = Vec::new();
        if let Some(v) = &self.version {
            tbs.push(tlv(0xA0, &tlv(0x02, v)));
        }
        tbs.push(self.serial.clone());
        tbs.push(self.inner_alg.clone());
        tbs.push(self.issuer.clone());
        tbs.push(seq(&[&self.not_before, &self.not_after]));
        tbs.push(self.subject.clone());
        tbs.push(self.spki.clone());
        tbs.push(tail.to_vec());
        let refs: Vec<&[u8]> = tbs.iter().map(Vec::as_slice).collect();
        let tbs = seq(&refs);
        let signature = tlv(
            0x03,
            &[0x00, 0x30, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 0x01],
        );
        seq(&[&tbs, &self.outer_alg, &signature])
    }
}

fn read(spec: &Spec) -> Result<(), Error> {
    Certificate::from_der(&spec.build()).map(|_| ())
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Acceptance 1: one certificate of limbo.

/// The case of x509-limbo whose leaf acceptance 1 reads.
pub const LIMBO_CASE: &str = "webpki::san::exact-dns-san";

/// The `peer_certificate` of [`LIMBO_CASE`], as `tests/data/x509-limbo/limbo.json` holds it
/// (`limbo_parse` checks that it is the same text).
pub const LIMBO_LEAF: &str = "-----BEGIN CERTIFICATE-----
MIIBsDCCAVagAwIBAgIUEPGmJ7j5NpF66USuYwk06q9ykz4wCgYIKoZIzj0EAwIw
GjEYMBYGA1UEAwwPeDUwOS1saW1iby1yb290MCAXDTcwMDEwMTAwMDAwMVoYDzI5
NjkwNTAzMDAwMDAxWjAWMRQwEgYDVQQDDAtleGFtcGxlLmNvbTBZMBMGByqGSM49
AgEGCCqGSM49AwEHA0IABHSRybbu7OqX362aRe4ZXj6Y1JyUbafihYDBQwWLVn/9
8SyBs0sU6R/emceloUn4HzKlAwYGZ3DFLTJgcP0Up5OjfDB6MB0GA1UdDgQWBBTM
AYItPbkFvZUGctBk3zL2buz+kDAfBgNVHSMEGDAWgBSTvupyclexN8yBtzgbP06c
VDW7WzALBgNVHQ8EBAMCB4AwEwYDVR0lBAwwCgYIKwYBBQUHAwEwFgYDVR0RBA8w
DYILZXhhbXBsZS5jb20wCgYIKoZIzj0EAwIDSAAwRQIge6n4mH04yTX66gW9wvMS
nXoJ6l8c3qwNkqaZZIQ1Lh0CIQCqrbXUNqlHEX/cZqdqq+FD95vo+z5yf7lr/v29
3fW8gQ==
-----END CERTIFICATE-----
";

const LEAF_SERIAL: &str = "10f1a627b8f936917ae944ae630934eaaf72933e";
const LEAF_POINT: &str = "047491c9b6eeecea97dfad9a45ee195e3e98d49c946da7e28580c143058b567ffd\
                          f12c81b34b14e91fde99c7a5a149f81f32a50306066770c52d326070fd14a793";

#[test]
fn a_certificate_of_limbo_reads_into_its_fields() {
    let blocks = pem::parse(LIMBO_LEAF.as_bytes()).expect("PEM");
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].label, "CERTIFICATE");
    let cert = Certificate::from_der(&blocks[0].data)
        .unwrap_or_else(|e| panic!("the leaf of {LIMBO_CASE}: {e}"));
    assert_eq!(cert.version, 3);
    assert_eq!(cert.serial, hex(LEAF_SERIAL));
    // CN=x509-limbo-root and CN=example.com, each one UTF8String.
    let mut issuer = hex("301a3118301606035504030c0f");
    issuer.extend_from_slice(b"x509-limbo-root");
    assert_eq!(cert.issuer.der(), issuer);
    let mut subject = hex("30163114301206035504030c0b");
    subject.extend_from_slice(b"example.com");
    assert_eq!(cert.subject.der(), subject);
    assert_eq!(cert.validity.not_before.to_string(), "1970-01-01T00:00:01Z");
    assert_eq!(cert.validity.not_after.to_string(), "2969-05-03T00:00:01Z");
    assert_eq!(
        cert.signature_algorithm,
        SignatureAlgorithm::Ecdsa(SignatureHash::Sha256)
    );
    assert_eq!(cert.public_key.kind, PublicKeyKind::EcP256);
    assert_eq!(cert.public_key.bytes, hex(LEAF_POINT));
    assert_eq!(
        cert.extensions.subject_alt_name,
        Some(vec![GeneralName::Dns("example.com")])
    );
    assert_eq!(
        cert.extensions
            .key_usage
            .map(|k| k.has(KeyUsage::DIGITAL_SIGNATURE)),
        Some(true)
    );
    assert_eq!(
        cert.extensions
            .ext_key_usage
            .as_ref()
            .map(|e| e.server_auth()),
        Some(true)
    );
    assert_eq!(
        cert.extensions.subject_key_id,
        Some(&hex("cc01822d3db905bd950672d064df32f66eecfe90")[..])
    );
    assert_eq!(
        cert.extensions.authority_key_id,
        Some(&hex("93beea727257b137cc81b7381b3f4e9c5435bb5b")[..])
    );
    assert_eq!(cert.extensions.unknown_critical, None);
    // The TBS is the first value inside the certificate, from its fourth byte.
    assert_eq!(cert.tbs, &blocks[0].data[4..4 + 4 + 342]);

    let leaf = LeafKey::from_certificate(&cert);
    assert_eq!(leaf.kind, PublicKeyKind::EcP256);
    assert_eq!(leaf.bytes, hex(LEAF_POINT));
    assert_eq!(leaf.public_key(), cert.public_key);
    let anchor = TrustAnchor::from_certificate(&cert);
    assert_eq!(anchor.key.bytes, hex(LEAF_POINT));
    assert_eq!(anchor.subject().der(), subject);
    assert_eq!(anchor.refused, None);
    assert_eq!(anchor.name_constraints(), Ok(None));
    assert_eq!(TrustAnchor::from_der(&blocks[0].data), Ok(anchor));
}

// ---------------------------------------------------------------------------------------------
// The certificates the test writes read.

#[test]
fn the_written_certificate_reads_with_its_extensions() {
    let der_bytes = Spec::p256().build();
    let cert = Certificate::from_der(&der_bytes).expect("reads");
    assert_eq!(cert.version, 3);
    assert_eq!(cert.serial, &[0x01, 0x02, 0x03]);
    assert_eq!(
        cert.extensions.basic_constraints,
        Some(BasicConstraints {
            ca: false,
            path_len: None
        })
    );
    assert_eq!(
        cert.extensions.subject_alt_name,
        Some(vec![
            GeneralName::Dns("example.com"),
            GeneralName::Ip(&[192, 0, 2, 1])
        ])
    );
    assert_eq!(cert.public_key.bytes, p256_point());
    assert_eq!(cert.der, &der_bytes[..]);
}

#[test]
fn an_rsa_key_reads_with_its_bits() {
    let cert_der = Spec::rsa().build();
    let cert = Certificate::from_der(&cert_der).expect("reads");
    assert_eq!(cert.public_key.kind, PublicKeyKind::Rsa { bits: 2048 });
    assert_eq!(
        cert.signature_algorithm,
        SignatureAlgorithm::RsaPkcs1(SignatureHash::Sha256)
    );
}

#[test]
fn basic_constraints_key_usage_and_name_constraints_read() {
    let mut spec = Spec::p256();
    let permitted = tlv(
        0xA0,
        &[
            seq(&[&tlv(0x82, b"example.com")]),
            seq(&[&tlv(0x87, &[10, 0, 0, 0, 255, 0, 0, 0])]),
        ]
        .concat(),
    );
    let excluded = tlv(0xA1, &seq(&[&tlv(0x82, b"bad.example.com")]));
    spec.extensions = vec![
        extension(
            OID_BASIC,
            true,
            &seq(&[&[0x01, 0x01, 0xFF], &[0x02, 0x01, 0x02]]),
        ),
        extension(OID_KEY_USAGE, true, &[0x03, 0x02, 0x01, 0x06]),
        extension(OID_NC, true, &seq(&[&permitted, &excluded])),
        extension(OID_SKI, false, &tlv(0x04, &[0xAA; 20])),
        extension(OID_AKI, false, &seq(&[&tlv(0x80, &[0xBB; 20])])),
    ];
    let der_bytes = spec.build();
    let cert = Certificate::from_der(&der_bytes).expect("reads");
    let e = &cert.extensions;
    assert_eq!(
        e.basic_constraints,
        Some(BasicConstraints {
            ca: true,
            path_len: Some(2)
        })
    );
    let usage = e.key_usage.expect("keyUsage");
    assert!(usage.has(KeyUsage::KEY_CERT_SIGN));
    assert!(usage.has(KeyUsage::CRL_SIGN));
    assert!(!usage.has(KeyUsage::DIGITAL_SIGNATURE));
    let nc = e.name_constraints.as_ref().expect("nameConstraints");
    assert_eq!(
        nc.permitted,
        Some(vec![
            GeneralName::Dns("example.com"),
            GeneralName::Ip(&[10, 0, 0, 0, 255, 0, 0, 0])
        ])
    );
    assert_eq!(nc.excluded, Some(vec![GeneralName::Dns("bad.example.com")]));
    assert_eq!(e.subject_key_id, Some(&[0xAA; 20][..]));
    assert_eq!(e.authority_key_id, Some(&[0xBB; 20][..]));
    assert_eq!(e.ext_key_usage, None);
    assert_eq!(e.subject_alt_name, None);
    let anchor = TrustAnchor::from_certificate(&cert);
    let anchor_nc = anchor.name_constraints().expect("reads").expect("present");
    assert_eq!(&anchor_nc, nc);
}

// ---------------------------------------------------------------------------------------------
// Acceptance 3: hostile X.509.

#[test]
fn a_version_1_certificate_is_unsupported_and_still_an_anchor_by_name_and_key() {
    let spec = Spec {
        version: None,
        extensions: Vec::new(),
        ..Spec::p256()
    };
    let der_bytes = spec.build();
    assert_eq!(read(&spec), Err(Error::UnsupportedVersion));
    let anchor = TrustAnchor::from_der(&der_bytes).expect("an anchor as name and key");
    assert_eq!(anchor.refused, Some(Error::UnsupportedVersion));
    assert_eq!(anchor.subject().der(), name_cn("example.com"));
    assert_eq!(anchor.key.kind, PublicKeyKind::EcP256);
    assert_eq!(anchor.key.bytes, p256_point());
    assert_eq!(anchor.name_constraints(), Ok(None));
}

/// id-ml-dsa-44, 2.16.840.1.101.3.4.3.17 (RFC 9881): a signature algorithm the reader does not
/// know.
const OID_ML_DSA_44: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x03, 0x11];

/// D164: a refused version 3 root whose extensions name no nameConstraints is an anchor of name
/// and key.
#[test]
fn a_refused_version_3_root_without_name_constraints_is_still_an_anchor() {
    let spec = Spec::p256().signed_with(alg(OID_ML_DSA_44, None));
    assert_eq!(read(&spec), Err(Error::UnsupportedAlgorithm));
    let anchor = TrustAnchor::from_der(&spec.build()).expect("an anchor as name and key");
    assert_eq!(anchor.refused, Some(Error::UnsupportedAlgorithm));
    assert_eq!(anchor.subject().der(), name_cn("example.com"));
    assert_eq!(anchor.name_constraints(), Ok(None));
}

/// D164: a refused root whose extension list cannot be walked to its end gives no anchor, and
/// `TrustAnchor::from_der` returns the reader's error.
#[test]
fn a_refused_root_whose_extension_list_is_cut_short_gives_no_anchor() {
    let spec = Spec::p256();
    let first = &spec.extensions[0];
    // A SEQUENCE that declares the first extension whole and holds its first 5 bytes.
    let mut list = vec![0x30, first.len() as u8];
    list.extend_from_slice(&first[..5]);
    let der_bytes = spec.build_with_tail(&tlv(0xA3, &list));
    let refused = Certificate::from_der(&der_bytes).expect_err("refused");
    assert_eq!(TrustAnchor::from_der(&der_bytes), Err(refused));
}

/// A version 1 root carries no unique identifier and no extensions, a version 2 root no
/// extensions, and a unique identifier is a BIT STRING: else there is no anchor.
#[test]
fn a_root_with_fields_its_version_does_not_have_gives_no_anchor() {
    let v1 = Spec {
        version: None,
        extensions: Vec::new(),
        ..Spec::p256()
    };
    let v2 = Spec {
        version: Some(vec![0x01]),
        ..v1.clone()
    };
    let extensions = tlv(0xA3, &seq(&[&Spec::p256().extensions[0]]));
    for (what, spec, tail) in [
        ("v1 issuerUniqueID", &v1, tlv(0x81, &[0x00, 0x01])),
        ("v1 subjectUniqueID", &v1, tlv(0x82, &[0x00, 0x01])),
        ("v1 extensions", &v1, extensions.clone()),
        ("v2 extensions", &v2, extensions),
        (
            "v2 issuerUniqueID no BIT STRING",
            &v2,
            tlv(0x81, &[0x09, 0xFF]),
        ),
    ] {
        let der_bytes = spec.build_with_tail(&tail);
        assert!(Certificate::from_der(&der_bytes).is_err(), "{what}");
        assert!(TrustAnchor::from_der(&der_bytes).is_err(), "{what}");
    }
    // A version 2 root with both unique identifiers is an anchor of name and key.
    let tail = [tlv(0x81, &[0x00, 0x01]), tlv(0x82, &[0x00, 0x02])].concat();
    let anchor = TrustAnchor::from_der(&v2.build_with_tail(&tail)).expect("an anchor");
    assert_eq!(anchor.refused, Some(Error::UnsupportedVersion));
}

/// F2: the critical flag of each known extension, and the presence of authorityCertIssuer and
/// authorityCertSerialNumber, read for path validation.
#[test]
fn the_critical_flags_and_the_authority_cert_fields_read() {
    let mut spec = Spec::p256();
    spec.extensions
        .push(extension(OID_SKI, true, &tlv(0x04, &[0x01, 0x02])));
    spec.extensions.push(extension(
        OID_AKI,
        false,
        &seq(&[
            &tlv(0x80, &[0x01, 0x02]),
            &tlv(0xA1, &tlv(0xA4, &name_cn("Example CA"))),
            &tlv(0x82, &[0x01]),
        ]),
    ));
    spec.extensions.push(extension(
        OID_NC,
        true,
        &seq(&[&tlv(0xA0, &seq(&[&tlv(0x82, b"example.com")]))]),
    ));
    let der_bytes = spec.build();
    let cert = Certificate::from_der(&der_bytes).expect("reads");
    let e = &cert.extensions;
    assert_eq!(
        e.critical,
        Critical {
            basic_constraints: true,
            key_usage: true,
            ext_key_usage: false,
            subject_alt_name: false,
            name_constraints: true,
            subject_key_id: true,
            authority_key_id: false,
        }
    );
    assert_eq!(e.authority_key_id, Some(&[0x01, 0x02][..]));
    assert!(e.authority_cert_issuer);
    assert!(e.authority_cert_serial);
    // With the keyIdentifier alone, neither.
    let mut spec = Spec::p256();
    spec.extensions
        .push(extension(OID_AKI, true, &seq(&[&tlv(0x80, &[0x01])])));
    let der_bytes = spec.build();
    let cert = Certificate::from_der(&der_bytes).expect("reads");
    assert!(cert.extensions.critical.authority_key_id);
    assert!(!cert.extensions.authority_cert_issuer);
    assert!(!cert.extensions.authority_cert_serial);
}

#[test]
fn versions_other_than_3_are_unsupported() {
    for v in [vec![0x00], vec![0x01], vec![0x03], vec![0x7F]] {
        let spec = Spec {
            version: Some(v.clone()),
            ..Spec::p256()
        };
        assert_eq!(read(&spec), Err(Error::UnsupportedVersion), "{v:?}");
    }
}

#[test]
fn outer_and_inner_algorithms_that_differ_are_a_mismatch() {
    let spec = Spec {
        outer_alg: alg(OID_SHA256_RSA, Some(&NULL)),
        ..Spec::p256()
    };
    assert_eq!(read(&spec), Err(Error::AlgorithmMismatch));
    // The same algorithm, written with and without its NULL, differs too.
    let spec = Spec {
        inner_alg: alg(OID_SHA256_RSA, None),
        ..Spec::rsa()
    };
    assert_eq!(read(&spec), Err(Error::AlgorithmMismatch));
}

#[test]
fn subject_alt_name_twice_is_a_duplicate_extension() {
    let mut spec = Spec::p256();
    spec.extensions.push(san(&[tlv(0x82, b"other.example")]));
    assert_eq!(read(&spec), Err(Error::DuplicateExtension));
    let mut spec = Spec::p256();
    let first = spec.extensions[0].clone();
    spec.extensions.push(first);
    assert_eq!(read(&spec), Err(Error::DuplicateExtension));
}

#[test]
fn utc_time_without_seconds_is_bad_time() {
    let spec = Spec {
        not_before: tlv(0x17, b"2501010000Z"),
        ..Spec::p256()
    };
    assert_eq!(read(&spec), Err(Error::Der(der::Error::BadTime)));
    assert_eq!(Error::Der(der::Error::BadTime).name(), "BadTime");
}

#[test]
fn february_30_is_bad_time() {
    let spec = Spec {
        not_after: tlv(0x17, b"250230000000Z"),
        ..Spec::p256()
    };
    assert_eq!(read(&spec), Err(Error::Der(der::Error::BadTime)));
}

#[test]
fn a_dns_name_holding_a_nul_byte_is_a_bad_name() {
    let mut spec = Spec::p256();
    spec.extensions[3] = san(&[tlv(0x82, b"example.com\0.evil.example")]);
    assert_eq!(read(&spec), Err(Error::BadName));
    for bad in [&b""[..], b"exa mple.com", "exämple.com".as_bytes(), b"\x7f"] {
        let mut spec = Spec::p256();
        spec.extensions[3] = san(&[tlv(0x82, bad)]);
        assert_eq!(read(&spec), Err(Error::BadName), "{bad:?}");
    }
    let mut spec = Spec::p256();
    spec.extensions[3] = san(&[tlv(0x87, &[127, 0, 0])]);
    assert_eq!(read(&spec), Err(Error::BadName), "an address of 3 bytes");
}

#[test]
fn other_kinds_of_general_name_are_kept_without_their_meaning() {
    let mut spec = Spec::p256();
    spec.extensions[3] = san(&[
        tlv(0x81, b"someone@example.com"),
        tlv(0x86, b"https://example.com/"),
        tlv(0xA4, &name_cn("Directory")),
    ]);
    let der_bytes = spec.build();
    let cert = Certificate::from_der(&der_bytes).expect("reads");
    let names = cert.extensions.subject_alt_name.expect("SAN");
    assert_eq!(names.len(), 3);
    assert_eq!(
        names[0],
        GeneralName::Other {
            tag: 0x81,
            content: b"someone@example.com"
        }
    );
}

#[test]
fn an_unknown_critical_extension_is_read_with_its_flag() {
    let unknown = [0x2B, 0x06, 0x01, 0x04, 0x01, 0x82, 0x37, 0x01];
    let mut spec = Spec::p256();
    spec.extensions
        .push(extension(&unknown, true, &[0x05, 0x00]));
    let der_bytes = spec.build();
    let cert = Certificate::from_der(&der_bytes).expect("read, not refused");
    assert_eq!(cert.extensions.unknown_critical, Some(&unknown[..]));
    // Not critical: no flag. A known one the reader does not model, critical: the flag.
    let mut spec = Spec::p256();
    spec.extensions
        .push(extension(&unknown, false, &[0x05, 0x00]));
    let der_bytes = spec.build();
    let cert = Certificate::from_der(&der_bytes).expect("reads");
    assert_eq!(cert.extensions.unknown_critical, None);
    let mut spec = Spec::p256();
    spec.extensions
        .push(extension(OID_POLICIES, true, &seq(&[])));
    let der_bytes = spec.build();
    let cert = Certificate::from_der(&der_bytes).expect("reads");
    assert_eq!(cert.extensions.unknown_critical, Some(OID_POLICIES));
}

#[test]
fn rsassa_pss_with_sha_256_and_a_salt_of_20_is_unsupported() {
    let spec = Spec::rsa().signed_with(pss(OID_SHA256, OID_SHA256, 20, None));
    assert_eq!(read(&spec), Err(Error::UnsupportedAlgorithm));
}

#[test]
fn rsassa_pss_with_sha_256_and_a_salt_of_32_reads_as_rsa_pss_rsae_sha256() {
    let der_bytes = Spec::rsa()
        .signed_with(pss(OID_SHA256, OID_SHA256, 32, None))
        .build();
    let cert = Certificate::from_der(&der_bytes).expect("reads");
    assert_eq!(
        cert.signature_algorithm,
        SignatureAlgorithm::RsaPss(SignatureHash::Sha256)
    );
    let scheme = crypto::scheme(cert.signature_algorithm, cert.public_key.kind);
    assert_eq!(scheme.map(Scheme::name), Some("rsa_pss_rsae_sha256"));
    // trailerField 1 written out reads too; SHA-384 with a salt of 48.
    assert!(read(&Spec::rsa().signed_with(pss(OID_SHA256, OID_SHA256, 32, Some(1)))).is_ok());
    let der_bytes = Spec::rsa()
        .signed_with(pss(OID_SHA384, OID_SHA384, 48, None))
        .build();
    let cert = Certificate::from_der(&der_bytes).expect("reads");
    assert_eq!(
        cert.signature_algorithm,
        SignatureAlgorithm::RsaPss(SignatureHash::Sha384)
    );
}

#[test]
fn other_rsassa_pss_parameters_are_unsupported() {
    for (what, algorithm) in [
        (
            "MGF1 of another hash",
            pss(OID_SHA256, OID_SHA384, 32, None),
        ),
        ("trailer 2", pss(OID_SHA256, OID_SHA256, 32, Some(2))),
        (
            "a salt of 48 with SHA-256",
            pss(OID_SHA256, OID_SHA256, 48, None),
        ),
        ("no parameters", alg(OID_RSA_PSS, None)),
        (
            "empty parameters, the SHA-1 defaults",
            alg(OID_RSA_PSS, Some(&seq(&[]))),
        ),
    ] {
        let spec = Spec::rsa().signed_with(algorithm);
        assert_eq!(read(&spec), Err(Error::UnsupportedAlgorithm), "{what}");
    }
}

#[test]
fn md5_with_rsa_encryption_reads_and_is_named_md5() {
    let der_bytes = Spec::rsa()
        .signed_with(alg(OID_MD5_RSA, Some(&NULL)))
        .build();
    let cert = Certificate::from_der(&der_bytes).expect("read, for REFUSED to refuse");
    assert_eq!(
        cert.signature_algorithm,
        SignatureAlgorithm::RsaPkcs1(SignatureHash::Md5)
    );
    assert_eq!(cert.signature_algorithm.name(), "md5WithRSAEncryption");
    assert_eq!(
        crypto::refusal(cert.signature_algorithm, cert.public_key.kind),
        Some(Refusal::WeakAlgorithm {
            algorithm: "md5WithRSAEncryption"
        })
    );
}

#[test]
fn an_unknown_signature_algorithm_is_unsupported() {
    let spec = Spec::p256().signed_with(alg(&[0x2B, 0x65, 0x71], None)); // Ed448
    assert_eq!(read(&spec), Err(Error::UnsupportedAlgorithm));
    let spec = Spec::p256().signed_with(alg(OID_ECDSA_SHA256, Some(&NULL)));
    assert_eq!(
        read(&spec),
        Err(Error::UnsupportedAlgorithm),
        "ECDSA with a NULL"
    );
}

#[test]
fn an_unknown_key_is_unsupported_and_a_malformed_one_is_bad() {
    let p521 = Spec {
        spki: seq(&[
            &seq(&[&oid(OID_EC_PUBLIC), &oid(OID_P521)]),
            &tlv(0x03, &[0x00, 0x04, 0x01]),
        ]),
        ..Spec::p256()
    };
    assert_eq!(read(&p521), Err(Error::UnsupportedKey));
    let short_point = Spec {
        spki: seq(&[
            &seq(&[&oid(OID_EC_PUBLIC), &oid(OID_P256)]),
            &tlv(0x03, &[0x00, 0x04, 0x01, 0x02]),
        ]),
        ..Spec::p256()
    };
    assert_eq!(read(&short_point), Err(Error::BadKey));
}

// ---------------------------------------------------------------------------------------------
// The serial's limit, at one below, at and one above.

#[test]
fn serial_bytes_max_is_20() {
    assert_eq!(SERIAL_BYTES_MAX, 20);
}

#[test]
fn a_serial_of_19_and_of_20_bytes_reads_and_one_of_21_is_too_long() {
    for (len, expected) in [
        (SERIAL_BYTES_MAX - 1, Ok(())),
        (SERIAL_BYTES_MAX, Ok(())),
        (SERIAL_BYTES_MAX + 1, Err(Error::SerialTooLong)),
    ] {
        let spec = Spec {
            serial: tlv(0x02, &vec![0x11; len]),
            ..Spec::p256()
        };
        assert_eq!(read(&spec), expected, "{len} bytes");
    }
}

// ---------------------------------------------------------------------------------------------
// Errors and their names.

#[test]
fn every_error_has_its_name() {
    for (error, name) in [
        (Error::Der(der::Error::Truncated), "Truncated"),
        (Error::UnsupportedVersion, "UnsupportedVersion"),
        (Error::SerialTooLong, "SerialTooLong"),
        (Error::UnsupportedAlgorithm, "UnsupportedAlgorithm"),
        (Error::AlgorithmMismatch, "AlgorithmMismatch"),
        (Error::UnsupportedKey, "UnsupportedKey"),
        (Error::BadKey, "BadKey"),
        (Error::DuplicateExtension, "DuplicateExtension"),
        (Error::BadExtension, "BadExtension"),
        (Error::BadName, "BadName"),
    ] {
        assert_eq!(error.name(), name);
        assert_eq!(error.to_string(), name);
    }
}

// ---------------------------------------------------------------------------------------------
// Acceptance 7: the checks of item 7, and `verify`.

/// The name of each check, by a match with no wildcard: a variant added to `Check` does not
/// compile here until it is named.
fn named(check: &Check) -> &'static str {
    match check {
        Check::Unreadable { .. } => "Unreadable",
        Check::NoPathToAnchor => "NoPathToAnchor",
        Check::BadSignature { .. } => "BadSignature",
        Check::IssuerNotCa { .. } => "IssuerNotCa",
        Check::PathTooLong(_) => "PathTooLong",
        Check::CandidateLimit(_) => "CandidateLimit",
        Check::KeyUsage { .. } => "KeyUsage",
        Check::UnknownCriticalExtension { .. } => "UnknownCriticalExtension",
        Check::NameConstraint { .. } => "NameConstraint",
        Check::NoServerAuth { .. } => "NoServerAuth",
        Check::WeakAlgorithm { .. } => "WeakAlgorithm",
        Check::WeakKey { .. } => "WeakKey",
    }
}

/// The first column of the table of checks in item 7 of `docs/design/tls.md`.
fn item_7_checks() -> Vec<String> {
    let note = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/design/tls.md");
    let text = fs::read_to_string(note).expect("read the design note");
    let item = text
        .split("\n## 7. ")
        .nth(1)
        .expect("item 7")
        .split("\n## ")
        .next()
        .expect("its text");
    let table = item
        .split("| check | condition |")
        .nth(1)
        .expect("the table of checks");
    table
        .lines()
        .skip(2)
        .take_while(|line| line.starts_with('|'))
        .map(|line| {
            let cell = line.split('|').nth(1).expect("a first cell").trim();
            cell.trim_matches('`').to_string()
        })
        .collect()
}

#[test]
fn check_names() {
    let expected = item_7_checks();
    assert_eq!(expected.len(), 12, "{expected:?}");
    let names: Vec<&str> = Check::ALL.iter().map(Check::name).collect();
    assert_eq!(names, expected);
    for check in &Check::ALL {
        assert_eq!(check.name(), named(check));
        assert!(check.to_string().starts_with(check.name()), "{check}");
    }
}

#[test]
fn each_check_prints_the_detail_of_item_7() {
    use surf_tls::verify::{Limit, PathTooLong};
    let cases = [
        (
            Check::Unreadable {
                certificate: 1,
                error: Error::Der(der::Error::Truncated),
            },
            "Unreadable: certificate 1 of the server: Truncated",
        ),
        (
            Check::NoPathToAnchor,
            "NoPathToAnchor: no path to a trust anchor",
        ),
        (
            Check::BadSignature { certificate: 0 },
            "BadSignature: the signature of certificate 0 does not verify",
        ),
        (
            Check::IssuerNotCa { certificate: 2 },
            "IssuerNotCa: certificate 2 issues but is no CA",
        ),
        (
            Check::PathTooLong(PathTooLong::Intermediates { count: 7, bound: 6 }),
            "PathTooLong: 7 intermediates, at most 6",
        ),
        (
            Check::PathTooLong(PathTooLong::PathLenConstraint { certificate: 3 }),
            "PathTooLong: pathLenConstraint of certificate 3 broken",
        ),
        (
            Check::CandidateLimit(Limit::SignatureChecks { bound: 100 }),
            "CandidateLimit: 100 signature checks",
        ),
        (
            Check::CandidateLimit(Limit::CandidatesConsidered { bound: 25_000 }),
            "CandidateLimit: 25000 candidates considered",
        ),
        (
            Check::KeyUsage {
                certificate: 1,
                lacks: "keyCertSign",
            },
            "KeyUsage: certificate 1 lacks keyCertSign",
        ),
        (
            Check::UnknownCriticalExtension {
                certificate: 0,
                oid: "1.3.6.1.4.1.311.1".to_string(),
            },
            "UnknownCriticalExtension: 1.3.6.1.4.1.311.1 in certificate 0",
        ),
        (
            Check::NameConstraint {
                certificate: 1,
                name: "evil.example".to_string(),
            },
            "NameConstraint: evil.example outside the constraints of certificate 1",
        ),
        (
            Check::NoServerAuth { certificate: 0 },
            "NoServerAuth: the leaf is not for TLS servers",
        ),
        (
            Check::WeakAlgorithm {
                certificate: 0,
                algorithm: "sha1WithRSAEncryption",
            },
            "WeakAlgorithm: sha1WithRSAEncryption in certificate 0",
        ),
        (
            Check::WeakKey {
                certificate: 1,
                bits: 1024,
            },
            "WeakKey: RSA 1024 bits in certificate 1",
        ),
    ];
    for (check, text) in cases {
        assert_eq!(check.to_string(), text);
    }
}

#[test]
fn a_rejection_prints_its_detail_and_sends_its_alert() {
    let expired = CertificateRejection::Expired {
        certificate: 0,
        not_before: Time::from_utc(2025, 1, 1, 0, 0, 0).unwrap(),
        not_after: Time::from_utc(2026, 1, 1, 0, 0, 0).unwrap(),
        at: Time::from_utc(2026, 11, 22, 0, 0, 0).unwrap(),
    };
    assert_eq!(
        expired.to_string(),
        "certificate 0 valid from 2025-01-01T00:00:00Z to 2026-01-01T00:00:00Z, \
         checked at 2026-11-22T00:00:00Z"
    );
    assert_eq!(expired.alert(), AlertDescription::CERTIFICATE_EXPIRED);
    let mismatch = CertificateRejection::NameMismatch {
        host: "example.org".to_string(),
    };
    assert_eq!(
        mismatch.to_string(),
        "no subjectAltName entry matches example.org"
    );
    assert_eq!(mismatch.alert(), AlertDescription::BAD_CERTIFICATE);
    let no_path = CertificateRejection::NotTrusted(Check::NoPathToAnchor);
    assert_eq!(no_path.alert(), AlertDescription::UNKNOWN_CA);
    for check in Check::ALL {
        let alert = CertificateRejection::NotTrusted(check.clone()).alert();
        if check == Check::NoPathToAnchor {
            assert_eq!(alert, AlertDescription::UNKNOWN_CA);
        } else {
            assert_eq!(alert, AlertDescription::BAD_CERTIFICATE, "{check}");
        }
    }
    let error = TlsError::Certificate(no_path);
    assert_eq!(
        error.to_string(),
        "NoPathToAnchor: no path to a trust anchor"
    );
}

/// A verifier of the test that trusts any leaf it can read: the trait's shape.
struct ReadsTheLeaf;

impl ServerCertVerifier for ReadsTheLeaf {
    fn verify(
        &self,
        certificates: &[&[u8]],
        _host: &str,
        _now: Time,
    ) -> Result<LeafKey, CertificateRejection> {
        let leaf = certificates
            .first()
            .ok_or(CertificateRejection::NotTrusted(Check::NoPathToAnchor))?;
        Certificate::from_der(leaf)
            .map(|cert| LeafKey::from_certificate(&cert))
            .map_err(|error| {
                CertificateRejection::NotTrusted(Check::Unreadable {
                    certificate: 0,
                    error,
                })
            })
    }
}

#[test]
fn a_server_cert_verifier_answers_a_leaf_key_or_a_rejection() {
    let verifier: &dyn ServerCertVerifier = &ReadsTheLeaf;
    let now = Time::from_utc(2026, 11, 22, 0, 0, 0).unwrap();
    let good = Spec::p256().build();
    let key = verifier
        .verify(&[&good], "example.com", now)
        .expect("a key");
    assert_eq!(key.public_key().bytes, p256_point());
    let rejection = verifier
        .verify(&[&good[..10]], "example.com", now)
        .expect_err("refused");
    assert_eq!(
        rejection,
        CertificateRejection::NotTrusted(Check::Unreadable {
            certificate: 0,
            error: Error::Der(der::Error::Truncated)
        })
    );
}
