//! Adversary cases for `story:tls-x509-path`: inputs the unit's own tests do not hold, each
//! asserting what RFC 5280, the CA/Browser Forum Baseline Requirements, docs/design/tls.md or a
//! doc comment of `path.rs` / `names.rs` says the verifier does with them. Chains are built with
//! a small DER writer and checked with [`KeyEcho`]: a signature verifies exactly when its bytes
//! are the signer's public key, as in `path.rs`. Costs are counts, never wall times.

#![forbid(unsafe_code)]

use surf_tls::crypto::{CryptoError, Scheme, SignatureVerifier};
use surf_tls::der::Time;
use surf_tls::names;
use surf_tls::path::{Limits, PathVerifier, Work};
use surf_tls::verify::{CertificateRejection, Check};
use surf_tls::x509::LeafKey;

use crate::x509_parse::{seq, tlv};

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
const OID_EKU: &[u8] = &[0x55, 0x1D, 0x25];
const OID_SAN: &[u8] = &[0x55, 0x1D, 0x11];
const OID_NC: &[u8] = &[0x55, 0x1D, 0x1E];
const OID_SKI: &[u8] = &[0x55, 0x1D, 0x0E];
const OID_AKI: &[u8] = &[0x55, 0x1D, 0x23];
const OID_SERVER_AUTH: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x01];
const OID_EMAIL_PROTECTION: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x04];

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

fn key(k: u16) -> Vec<u8> {
    let mut out = vec![0x5A; 32];
    out[..2].copy_from_slice(&k.to_be_bytes());
    out
}

fn dns_name(name: &str) -> Vec<u8> {
    tlv(0x82, name.as_bytes())
}

fn ip_name(octets: &[u8]) -> Vec<u8> {
    tlv(0x87, octets)
}

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

#[derive(Clone)]
struct Cert {
    subject: String,
    issuer: String,
    key: u16,
    signer: u16,
    ca: bool,
    eku: Option<Vec<&'static [u8]>>,
    sans: Vec<Vec<u8>>,
    nc: Option<Vec<u8>>,
    ski: Option<Vec<u8>>,
    ski_critical: bool,
    aki: Option<Vec<u8>>,
    /// The DER of the subject or the issuer instead of a Name of one Common Name.
    subject_der: Option<Vec<u8>>,
    issuer_der: Option<Vec<u8>>,
    serial: u8,
}

impl Cert {
    fn ca(subject: &str, key: u16, issuer: &str, signer: u16) -> Cert {
        Cert {
            subject: subject.into(),
            issuer: issuer.into(),
            key,
            signer,
            ca: true,
            eku: None,
            sans: Vec::new(),
            nc: None,
            ski: None,
            ski_critical: true,
            aki: None,
            subject_der: None,
            issuer_der: None,
            serial: 1,
        }
    }

    fn leaf(host: &str, issuer: &str, signer: u16) -> Cert {
        Cert {
            ca: false,
            sans: vec![dns_name(host)],
            ..Cert::ca(&format!("leaf {host}"), 9999, issuer, signer)
        }
    }

    fn der(&self) -> Vec<u8> {
        let alg = seq(&[&oid(OID_ED25519)]);
        let spki = seq(&[
            &seq(&[&oid(OID_ED25519)]),
            &tlv(0x03, &[&[0x00][..], &key(self.key)].concat()),
        ]);
        let mut extensions = Vec::new();
        if self.ca {
            extensions.push(extension(OID_BASIC, true, &seq(&[&[0x01, 0x01, 0xFF]])));
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
            extensions.push(extension(OID_NC, true, nc));
        }
        if let Some(ski) = &self.ski {
            extensions.push(extension(OID_SKI, self.ski_critical, &tlv(0x04, ski)));
        }
        if let Some(aki) = &self.aki {
            extensions.push(extension(OID_AKI, false, &seq(&[&tlv(0x80, aki)])));
        }
        let mut tbs: Vec<Vec<u8>> = vec![
            tlv(0xA0, &tlv(0x02, &[0x02])),
            tlv(0x02, &[self.serial]),
            alg.clone(),
            self.issuer_der
                .clone()
                .unwrap_or_else(|| name(&self.issuer)),
            seq(&[&tlv(0x17, b"250101000000Z"), &tlv(0x18, b"21250101000000Z")]),
            self.subject_der
                .clone()
                .unwrap_or_else(|| name(&self.subject)),
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

fn now() -> Time {
    Time::from_utc(2026, 11, 22, 0, 0, 0).expect("a time")
}

fn root() -> Cert {
    Cert::ca("Root", 1, "Root", 1)
}

fn check(
    roots: &[&Cert],
    sent: &[&Cert],
    host: &str,
    limits: Limits,
) -> (Result<LeafKey, CertificateRejection>, Work) {
    let mut v = PathVerifier::new(Vec::new())
        .with_limits(limits)
        .with_verifier(KeyEcho);
    for r in roots {
        v.add_root(&r.der()).expect("a root");
    }
    let ders: Vec<Vec<u8>> = sent.iter().map(|c| c.der()).collect();
    let refs: Vec<&[u8]> = ders.iter().map(Vec::as_slice).collect();
    v.verify_counting(&refs, host, now())
}

/// An intermediate whose extKeyUsage holds emailProtection and not serverAuth is a CA for
/// S/MIME only: the CA/Browser Forum Baseline Requirements section 7.1.2.3 ("Technically
/// Constrained Non-TLS Subordinate CA") keep such a CA outside the TLS audit precisely because
/// its extKeyUsage keeps it from issuing for TLS, and Chrome, Firefox and rustls-webpki
/// (`check_issuer_independent_properties`, `check_eku` on every certificate of the path) refuse
/// a TLS leaf under it. surf holds extKeyUsage on the leaf only, so the holder of an S/MIME CA
/// under a trusted root makes a server certificate for any host that surf accepts.
#[test]
fn an_intermediate_for_email_only_does_not_issue_a_tls_server_certificate() {
    let leaf = Cert::leaf("bank.test", "Mail CA", 101);
    let mut ica = Cert::ca("Mail CA", 101, "Root", 1);
    ica.eku = Some(vec![OID_SERVER_AUTH]);
    let (control, _) = check(&[&root()], &[&leaf, &ica], "bank.test", Limits::default());
    assert!(
        control.is_ok(),
        "serverAuth on the intermediate: {control:?}"
    );
    ica.eku = Some(vec![OID_EMAIL_PROTECTION]);
    let (result, _) = check(&[&root()], &[&leaf, &ica], "bank.test", Limits::default());
    assert!(
        result.is_err(),
        "a TLS leaf under an emailProtection-only intermediate was accepted: {result:?}"
    );
}

/// docs/design/tls.md item 7: "`<n>` counts the certificates as the server sent them, the leaf
/// 0". A certificate sent twice is dropped before the search, so every certificate after it is
/// numbered one too low: the issuer that is no CA, sent fourth (3), is named as the duplicate
/// in front of it (2).
#[test]
fn a_duplicate_does_not_shift_the_number_of_a_later_certificate() {
    let leaf = Cert::leaf("a.test", "I1", 101);
    let other = Cert::ca("Other", 500, "Root", 1);
    let mut ica = Cert::ca("I1", 101, "Root", 1);
    ica.ca = false;
    let (result, _) = check(
        &[&root()],
        &[&leaf, &other, &other, &ica],
        "a.test",
        Limits::default(),
    );
    assert_eq!(
        result,
        Err(CertificateRejection::NotTrusted(Check::IssuerNotCa {
            certificate: 3
        }))
    );
}

/// The bound of name-constraint work (item 9.4 as the hand-over writes it, "the number
/// rustls-webpki keeps") charges a comparison only for a subtree of the name's own form. Every
/// subtree of another form is still visited for every name (`has_form` twice and the `filter`
/// of `constrain`), uncharged: n dNSNames under m iPAddress subtrees cost 2 n m visits and charge
/// 0, so the limit does not bound the work. rustls-webpki 0.103.17 charges each subtree visited
/// (`subject_name/mod.rs`, `consume_name_constraint_comparison` before the form is looked at).
/// Here 100 names under 100 subtrees, 10,000 visits, against a limit of 1,000.
#[test]
fn name_constraint_work_is_charged_for_every_subtree_visited() {
    let excluded: Vec<Vec<u8>> = (0..100u8)
        .map(|i| ip_name(&[10, i, 0, 0, 255, 255, 0, 0]))
        .collect();
    let mut ica = Cert::ca("I1", 101, "Root", 1);
    ica.nc = Some(name_constraints(&[], &excluded));
    let mut leaf = Cert::leaf("h0.a.test", "I1", 101);
    leaf.sans = (0..100)
        .map(|i| dns_name(&format!("h{i}.a.test")))
        .collect();
    let limits = Limits {
        name_constraint_comparisons: 1_000,
        ..Limits::default()
    };
    let (result, work) = check(&[&root()], &[&leaf, &ica], "h0.a.test", limits);
    assert!(
        matches!(
            result,
            Err(CertificateRejection::NotTrusted(
                Check::NameConstraint { .. }
            ))
        ),
        "10,000 subtree visits under a limit of 1,000 ended {result:?} with {work:?}"
    );
}

/// `names::is_dns_name` says its last-label rule is there "so that no IPv4 address in text is a
/// DNS name". These are IPv4 addresses in text as inet_aton(3) and the WHATWG URL host parser
/// read them (127.0.0.1), and each passes as a DNS name.
#[test]
fn no_ipv4_address_in_text_is_a_dns_name() {
    for text in ["0x7f000001", "0x7f.0x1", "127.0.0.0x1", "0x7F.0.0.0X1"] {
        assert!(
            !names::is_dns_name(text),
            "{text} is an IPv4 address in text and reads as a DNS name"
        );
    }
}

/// rfc5280::ski::critical-ski is `open` because the key-identifier filter rules the root out
/// first. Without an authorityKeyIdentifier in the leaf the filter lets the root through, and
/// its critical subjectKeyIdentifier must still refuse: the filter is not the only guard.
#[test]
fn a_root_with_a_critical_subject_key_identifier_refuses_without_the_filter() {
    let mut r = root();
    r.ski = Some(vec![0x01; 20]);
    let leaf = Cert::leaf("a.test", "Root", 1);
    let (result, _) = check(&[&r], &[&leaf], "a.test", Limits::default());
    assert_eq!(
        result,
        Err(CertificateRejection::NotTrusted(
            Check::UnknownCriticalExtension {
                certificate: 1,
                oid: "2.5.29.14".into()
            }
        ))
    );
}

/// path.rs, the checks of a complete path: the anchor's authorityKeyIdentifier "with a
/// keyIdentifier other than its subjectKeyIdentifier" is `Unreadable` with `BadExtension`,
/// numbered after the certificates sent.
#[test]
fn a_root_whose_key_identifiers_differ_is_unreadable() {
    let mut r = root();
    r.ski = Some(vec![0x01; 20]);
    r.ski_critical = false;
    r.aki = Some(vec![0x02; 20]);
    let leaf = Cert::leaf("a.test", "Root", 1);
    let (result, _) = check(&[&r], &[&leaf], "a.test", Limits::default());
    assert_eq!(
        result,
        Err(CertificateRejection::NotTrusted(Check::Unreadable {
            certificate: 1,
            error: surf_tls::x509::Error::BadExtension
        }))
    );
}

/// RFC 5280 section 4.2.1.6: a leaf with an empty subject must mark its subjectAltName
/// critical; not critical, it is `Unreadable` with `BadExtension` (path.rs).
#[test]
fn a_leaf_with_an_empty_subject_and_a_non_critical_san_is_unreadable() {
    let mut leaf = Cert::leaf("a.test", "Root", 1);
    leaf.subject_der = Some(seq(&[]));
    let (result, _) = check(&[&root()], &[&leaf], "a.test", Limits::default());
    assert_eq!(
        result,
        Err(CertificateRejection::NotTrusted(Check::Unreadable {
            certificate: 0,
            error: surf_tls::x509::Error::BadExtension
        }))
    );
}

/// RFC 5280 section 4.1.2.6: an issuer has a non-empty subject; one with an empty subject is
/// `IssuerNotCa` (path.rs), even when the name of the certificate below it is empty too.
#[test]
fn an_issuer_with_an_empty_subject_is_no_ca() {
    let mut ica = Cert::ca("unused", 101, "Root", 1);
    ica.subject_der = Some(seq(&[]));
    let mut leaf = Cert::leaf("a.test", "unused", 101);
    leaf.issuer_der = Some(seq(&[]));
    let (result, _) = check(&[&root()], &[&leaf, &ica], "a.test", Limits::default());
    assert_eq!(
        result,
        Err(CertificateRejection::NotTrusted(Check::IssuerNotCa {
            certificate: 1
        }))
    );
}

/// path.rs: directoryName constraints are not checked, so they fail closed over a certificate
/// with a subject, permitted or excluded alike.
#[test]
fn a_directory_name_constraint_fails_closed_over_a_subject() {
    for nc in [
        name_constraints(&[tlv(0xA4, &name("leaf a.test"))], &[]),
        name_constraints(&[], &[tlv(0xA4, &name("Elsewhere"))]),
    ] {
        let mut ica = Cert::ca("I1", 101, "Root", 1);
        ica.nc = Some(nc);
        let leaf = Cert::leaf("a.test", "I1", 101);
        let (result, _) = check(&[&root()], &[&leaf, &ica], "a.test", Limits::default());
        assert_eq!(
            result,
            Err(CertificateRejection::NotTrusted(Check::NameConstraint {
                certificate: 1,
                name: "the subject of certificate 0".into()
            }))
        );
    }
}

/// path.rs, `meets_excluded`: a dNSName outside the preferred syntax counts as met by an
/// excluded subtree, though it does not lie in it.
#[test]
fn a_dns_name_outside_the_preferred_syntax_meets_every_excluded_subtree() {
    let mut ica = Cert::ca("I1", 101, "Root", 1);
    ica.nc = Some(name_constraints(&[], &[dns_name("other.test")]));
    let mut leaf = Cert::leaf("a.example.test", "I1", 101);
    leaf.sans.push(dns_name("a_b.example.test"));
    let (result, _) = check(
        &[&root()],
        &[&leaf, &ica],
        "a.example.test",
        Limits::default(),
    );
    assert_eq!(
        result,
        Err(CertificateRejection::NotTrusted(Check::NameConstraint {
            certificate: 1,
            name: "a_b.example.test".into()
        }))
    );
}

/// An IP host never matches a wildcard or a dNSName spelling of it, and an IP literal in odd
/// spellings is no IP host and matches no iPAddress.
#[test]
fn ip_hosts_match_only_their_octets() {
    let sans = [
        surf_tls::x509::GeneralName::Dns("*.0.0.1"),
        surf_tls::x509::GeneralName::Dns("127.0.0.1"),
    ];
    assert!(!names::matches(&sans, "127.0.0.1"));
    let ip = [surf_tls::x509::GeneralName::Ip(&[127, 0, 0, 1])];
    for host in [
        "127.1",
        "0x7f.0.0.1",
        "127.000.0.1",
        "0177.0.0.1",
        "[127.0.0.1]",
    ] {
        assert!(!names::matches(&ip, host), "{host}");
    }
}
