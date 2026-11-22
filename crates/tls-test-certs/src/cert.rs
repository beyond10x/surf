//! The certificate builder: an X.509 v3 (or v1) certificate (RFC 5280 section 4.1) written in
//! DER and signed through the seam under the fixed random source, so that the same template
//! and issuer give the same bytes every run.

use surf_tls::crypto::{self, HashAlgorithm, PublicKeyKind, Scheme};

use crate::Error;
use crate::der;
use crate::keys::Key;

/// A day at 00:00:00Z.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Date {
    /// The year.
    pub year: u16,
    /// The month, 1 to 12.
    pub month: u8,
    /// The day, 1 to 31.
    pub day: u8,
}

impl Date {
    /// The day `year`-`month`-`day`.
    pub const fn new(year: u16, month: u8, day: u8) -> Date {
        Date { year, month, day }
    }

    /// UTCTime for the years 1950 to 2049, GeneralizedTime from 2050 (RFC 5280 section
    /// 4.1.2.5).
    pub fn der(self) -> Vec<u8> {
        if (1950..2050).contains(&self.year) {
            let text = format!(
                "{:02}{:02}{:02}000000Z",
                self.year % 100,
                self.month,
                self.day
            );
            der::tlv(der::tag::UTC_TIME, text.as_bytes())
        } else {
            let text = format!("{:04}{:02}{:02}000000Z", self.year, self.month, self.day);
            der::tlv(der::tag::GENERALIZED_TIME, text.as_bytes())
        }
    }
}

/// The validity of every certificate but those a case sets otherwise: 2025-01-01 to
/// 2125-01-01, so that both time forms are written.
pub const VALID: (Date, Date) = (Date::new(2025, 1, 1), Date::new(2125, 1, 1));

/// The bits of keyUsage (RFC 5280 section 4.2.1.3).
pub mod key_usage {
    /// digitalSignature.
    pub const DIGITAL_SIGNATURE: u8 = 0;
    /// keyEncipherment.
    pub const KEY_ENCIPHERMENT: u8 = 2;
    /// keyCertSign.
    pub const KEY_CERT_SIGN: u8 = 5;
    /// cRLSign.
    pub const CRL_SIGN: u8 = 6;
}

/// id-kp-serverAuth.
pub const SERVER_AUTH: &str = "1.3.6.1.5.5.7.3.1";
/// id-kp-clientAuth.
pub const CLIENT_AUTH: &str = "1.3.6.1.5.5.7.3.2";
/// The critical extension no reader knows: an OID of the UUID arc (ITU-T X.667), made for these
/// certificates.
pub const UNKNOWN_EXTENSION: &str = "2.25.209006924046612367016455925447005224481";

/// An entry of subjectAltName.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum San {
    /// dNSName.
    Dns(String),
    /// iPAddress, IPv4.
    Ip([u8; 4]),
}

/// What a certificate holds, but for its issuer.
#[derive(Clone, Debug)]
pub struct Template {
    /// The commonName of the subject, its one attribute.
    pub subject: String,
    /// The subject's key.
    pub key: Key,
    /// notBefore and notAfter.
    pub validity: (Date, Date),
    /// A version 1 certificate: no version field and no extensions.
    pub version1: bool,
    /// basicConstraints, critical: cA and pathLenConstraint.
    pub basic_constraints: Option<(bool, Option<u64>)>,
    /// keyUsage, critical: the bits set.
    pub key_usage: Option<Vec<u8>>,
    /// extKeyUsage: the purposes.
    pub ext_key_usage: Option<Vec<&'static str>>,
    /// subjectAltName.
    pub subject_alt_name: Option<Vec<San>>,
    /// nameConstraints, critical, with one excluded dNSName subtree.
    pub excluded_dns: Option<String>,
    /// The critical extension [`UNKNOWN_EXTENSION`].
    pub unknown_critical: bool,
    /// subjectKeyIdentifier, and authorityKeyIdentifier when the issuer has one and is another.
    pub key_ids: bool,
    /// Mixed into the serial number, which is otherwise derived from the content, so that two
    /// certificates of equal content differ.
    pub serial_label: String,
}

impl Template {
    /// A CA: basicConstraints cA, keyUsage keyCertSign and cRLSign, key identifiers.
    pub fn ca(subject: &str, key: &Key) -> Template {
        Template {
            subject: subject.to_string(),
            key: key.clone(),
            validity: VALID,
            version1: false,
            basic_constraints: Some((true, None)),
            key_usage: Some(vec![key_usage::KEY_CERT_SIGN, key_usage::CRL_SIGN]),
            ext_key_usage: None,
            subject_alt_name: None,
            excluded_dns: None,
            unknown_critical: false,
            key_ids: true,
            serial_label: String::new(),
        }
    }

    /// A server's leaf: basicConstraints without cA, keyUsage digitalSignature, extKeyUsage
    /// serverAuth, the names `sans`, key identifiers; the commonName is `common_name`.
    pub fn leaf(common_name: &str, key: &Key, sans: Vec<San>) -> Template {
        Template {
            subject: common_name.to_string(),
            key: key.clone(),
            validity: VALID,
            version1: false,
            basic_constraints: Some((false, None)),
            key_usage: Some(vec![key_usage::DIGITAL_SIGNATURE]),
            ext_key_usage: Some(vec![SERVER_AUTH]),
            subject_alt_name: Some(sans),
            excluded_dns: None,
            unknown_critical: false,
            key_ids: true,
            serial_label: String::new(),
        }
    }
}

/// Who signs: the issuer's name, key and scheme.
#[derive(Clone, Debug)]
pub struct Issuer {
    /// The commonName of the issuer.
    pub name: String,
    /// The issuer's key.
    pub key: Key,
    /// The scheme it signs in.
    pub scheme: Scheme,
    /// Whether its certificate carries a subjectKeyIdentifier, so that an
    /// authorityKeyIdentifier names it.
    pub key_id: bool,
}

impl Issuer {
    /// The issuer a template is when it signs itself, in the default scheme of its key.
    pub fn itself(template: &Template) -> Issuer {
        Issuer {
            name: template.subject.clone(),
            key: template.key.clone(),
            scheme: template.key.scheme,
            key_id: template.key_ids,
        }
    }
}

/// A Name of one commonName, a UTF8String.
pub fn name(common_name: &str) -> Vec<u8> {
    let attribute = der::sequence(&[&der::oid("2.5.4.3"), &der::utf8(common_name)]);
    der::sequence(&[&der::set(&attribute)])
}

/// The SubjectPublicKeyInfo of `key` (RFC 5280, 3279, 5480, 8410).
pub fn subject_public_key_info(key: &Key) -> Vec<u8> {
    let algorithm = match key.kind {
        PublicKeyKind::Rsa { .. } => {
            der::sequence(&[&der::oid("1.2.840.113549.1.1.1"), &der::null()])
        }
        PublicKeyKind::EcP256 => der::sequence(&[
            &der::oid("1.2.840.10045.2.1"),
            &der::oid("1.2.840.10045.3.1.7"),
        ]),
        PublicKeyKind::EcP384 => {
            der::sequence(&[&der::oid("1.2.840.10045.2.1"), &der::oid("1.3.132.0.34")])
        }
        PublicKeyKind::Ed25519 => der::sequence(&[&der::oid("1.3.101.112")]),
    };
    der::sequence(&[&algorithm, &der::bit_string(&key.public)])
}

/// The key identifier of `key`: the leftmost 160 bits of the SHA-256 of its public key bits
/// (RFC 7093 section 2, method 1).
pub fn key_identifier(key: &Key) -> Vec<u8> {
    crypto::hash(HashAlgorithm::Sha256, &key.public).as_ref()[..20].to_vec()
}

/// The AlgorithmIdentifier of a signature in `scheme`: NULL parameters for RSA PKCS#1, none for
/// ECDSA and Ed25519, and for RSASSA-PSS the parameters of RFC 4055 with MGF1 of the same hash
/// and a salt of the digest's length, as RFC 8446 section 4.2.3 has them.
pub fn algorithm_identifier(scheme: Scheme) -> Vec<u8> {
    let with_null = |oid: &str| der::sequence(&[&der::oid(oid), &der::null()]);
    let pss = |hash: &str, salt: u64| {
        let hash = with_null(hash);
        let mgf = der::sequence(&[&der::oid("1.2.840.113549.1.1.8"), &hash]);
        let params = der::sequence(&[
            &der::explicit(0, &hash),
            &der::explicit(1, &mgf),
            &der::explicit(2, &der::integer(salt)),
        ]);
        der::sequence(&[&der::oid("1.2.840.113549.1.1.10"), &params])
    };
    match scheme {
        Scheme::RsaPkcs1Sha256 => with_null("1.2.840.113549.1.1.11"),
        Scheme::RsaPkcs1Sha384 => with_null("1.2.840.113549.1.1.12"),
        Scheme::RsaPkcs1Sha512 => with_null("1.2.840.113549.1.1.13"),
        Scheme::RsaPssRsaeSha256 => pss("2.16.840.1.101.3.4.2.1", 32),
        Scheme::RsaPssRsaeSha384 => pss("2.16.840.1.101.3.4.2.2", 48),
        Scheme::RsaPssRsaeSha512 => pss("2.16.840.1.101.3.4.2.3", 64),
        Scheme::EcdsaSecp256r1Sha256 => der::sequence(&[&der::oid("1.2.840.10045.4.3.2")]),
        Scheme::EcdsaSecp384r1Sha384 => der::sequence(&[&der::oid("1.2.840.10045.4.3.3")]),
        Scheme::Ed25519 => der::sequence(&[&der::oid("1.3.101.112")]),
    }
}

fn extension(oid: &str, critical: bool, value: &[u8]) -> Vec<u8> {
    let oid = der::oid(oid);
    let value = der::octet_string(value);
    if critical {
        der::sequence(&[&oid, &der::true_(), &value])
    } else {
        der::sequence(&[&oid, &value])
    }
}

fn extensions(template: &Template, issuer: &Issuer) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    if let Some((ca, path_len)) = template.basic_constraints {
        let mut parts = Vec::new();
        if ca {
            parts.push(der::true_());
        }
        if let Some(n) = path_len {
            parts.push(der::integer(n));
        }
        let value = der::tlv(der::tag::SEQUENCE, &parts.concat());
        out.push(extension("2.5.29.19", true, &value));
    }
    if let Some(bits) = &template.key_usage {
        out.push(extension("2.5.29.15", true, &der::named_bits(bits)));
    }
    if let Some(purposes) = &template.ext_key_usage {
        let oids: Vec<Vec<u8>> = purposes.iter().map(|p| der::oid(p)).collect();
        let value = der::tlv(der::tag::SEQUENCE, &oids.concat());
        out.push(extension("2.5.29.37", false, &value));
    }
    if let Some(sans) = &template.subject_alt_name {
        let names: Vec<Vec<u8>> = sans
            .iter()
            .map(|san| match san {
                San::Dns(host) => der::implicit(2, host.as_bytes()),
                San::Ip(address) => der::implicit(7, address),
            })
            .collect();
        let value = der::tlv(der::tag::SEQUENCE, &names.concat());
        out.push(extension("2.5.29.17", false, &value));
    }
    if let Some(host) = &template.excluded_dns {
        let subtree = der::sequence(&[&der::implicit(2, host.as_bytes())]);
        let excluded = der::explicit(1, &subtree);
        out.push(extension("2.5.29.30", true, &der::sequence(&[&excluded])));
    }
    if template.key_ids {
        let id = der::octet_string(&key_identifier(&template.key));
        out.push(extension("2.5.29.14", false, &id));
        let self_signed =
            issuer.name == template.subject && issuer.key.public == template.key.public;
        if issuer.key_id && !self_signed {
            let aki = der::sequence(&[&der::implicit(0, &key_identifier(&issuer.key))]);
            out.push(extension("2.5.29.35", false, &aki));
        }
    }
    if template.unknown_critical {
        out.push(extension(UNKNOWN_EXTENSION, true, &der::null()));
    }
    out
}

/// A certificate of `template`, signed by `issuer`: the DER of the Certificate.
pub fn issue(template: &Template, issuer: &Issuer) -> Result<Vec<u8>, Error> {
    let algorithm = algorithm_identifier(issuer.scheme);
    let issuer_name = name(&issuer.name);
    let subject_name = name(&template.subject);
    let validity = der::sequence(&[&template.validity.0.der(), &template.validity.1.der()]);
    let spki = subject_public_key_info(&template.key);
    let extensions = if template.version1 {
        Vec::new()
    } else {
        extensions(template, issuer)
    };

    // The serial number: 128 bits of the SHA-256 of everything else, positive, its first byte
    // at least 0x40 so that it keeps its 16 bytes.
    let mut seed = Vec::new();
    for part in [
        template.serial_label.as_bytes(),
        &algorithm,
        &issuer_name,
        &validity,
        &subject_name,
        &spki,
        &extensions.concat(),
    ] {
        seed.extend_from_slice(&(part.len() as u32).to_be_bytes());
        seed.extend_from_slice(part);
    }
    let digest = crypto::hash(HashAlgorithm::Sha256, &seed);
    let mut serial = digest.as_ref()[..16].to_vec();
    serial[0] = (serial[0] & 0x3f) | 0x40;

    let mut tbs_parts: Vec<Vec<u8>> = Vec::new();
    if !template.version1 {
        tbs_parts.push(der::explicit(0, &der::integer(2)));
    }
    tbs_parts.push(der::unsigned(&serial));
    tbs_parts.push(algorithm.clone());
    tbs_parts.push(issuer_name);
    tbs_parts.push(validity);
    tbs_parts.push(subject_name);
    tbs_parts.push(spki);
    if !extensions.is_empty() {
        let list = der::tlv(der::tag::SEQUENCE, &extensions.concat());
        tbs_parts.push(der::explicit(3, &list));
    }
    let tbs = der::tlv(der::tag::SEQUENCE, &tbs_parts.concat());
    let signature = issuer.key.sign(issuer.scheme, &tbs)?;
    Ok(der::sequence(&[
        &tbs,
        &algorithm,
        &der::bit_string(&signature),
    ]))
}

/// `der` with the lowest bit of the last byte of its signature flipped: a certificate whose
/// signature does not verify and whose encoding is still well formed.
pub fn flip_signature_bit(mut der: Vec<u8>) -> Vec<u8> {
    let last = der.len() - 1;
    der[last] ^= 1;
    der
}
