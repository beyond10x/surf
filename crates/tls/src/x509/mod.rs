//! X.509 version 3 certificates as RFC 5280 defines them, read from DER (docs/design/tls.md,
//! item 6; `story:tls-x509`): the parts a TLS client needs and nothing else.
//!
//! A certificate is read into its TBS bytes (what the signature covers), its version (only 3),
//! its serial (at most [`SERIAL_BYTES_MAX`] bytes), its signature algorithm (the outer one and
//! the one inside the TBS must be the same bytes), issuer and subject as DER names compared byte
//! for byte, its validity, its subject public key, and the extensions basicConstraints,
//! keyUsage, extKeyUsage, subjectAltName (dNSName, iPAddress), nameConstraints and the two key
//! identifiers. A duplicate extension refuses the certificate. An extension the reader does not
//! know, marked critical, is kept as a flag ([`Extensions::unknown_critical`]) that path
//! validation (`story:tls-x509-path`) rejects. The Common Name is not read: no host name comes
//! from it.
//!
//! The reader checks no signature and no time: [`crate::crypto`] and path validation do.
//! MD5 and SHA-1 signature algorithms are read and named, so that [`crate::crypto::REFUSED`]
//! refuses them by name.

mod algorithm;
mod anchor;
mod extensions;
mod name;

use std::fmt;

use crate::crypto::{PublicKey, SignatureAlgorithm};
use crate::der::{self, Reader, Time, tag};

pub use anchor::{LeafKey, TrustAnchor};
pub use extensions::{BasicConstraints, ExtKeyUsage, Extensions, KeyUsage, NameConstraints};
pub use name::{GeneralName, Name};

/// The most bytes the content of a serial number may have (RFC 5280 section 4.1.2.2: "Conforming
/// CAs MUST NOT use serialNumber values longer than 20 octets"). More is
/// [`Error::SerialTooLong`].
pub const SERIAL_BYTES_MAX: usize = 20;

/// Why the reader refused a certificate. [`Error::name`] gives each kind its name; the error of
/// the DER reader keeps the name of its own kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Error {
    /// The DER is malformed or has another structure than a certificate.
    Der(der::Error),
    /// A version other than 3 (a version 1 or 2 certificate).
    UnsupportedVersion,
    /// A serial of more than [`SERIAL_BYTES_MAX`] bytes.
    SerialTooLong,
    /// A signature algorithm the reader does not know, or RSASSA-PSS with other parameters than
    /// those of RFC 4055 for SHA-256, SHA-384 or SHA-512.
    UnsupportedAlgorithm,
    /// The signature algorithm outside the TBS is not the same bytes as the one inside.
    AlgorithmMismatch,
    /// A subject public key of an algorithm or curve the reader does not know.
    UnsupportedKey,
    /// A subject public key whose bytes do not have the form of its algorithm.
    BadKey,
    /// An extension present twice.
    DuplicateExtension,
    /// A known extension whose value does not have its structure.
    BadExtension,
    /// A name that breaks its rules: a dNSName with a byte outside `!` to `~` (a NUL among
    /// them), an iPAddress of another length, a relative distinguished name with no attribute.
    BadName,
}

impl Error {
    /// The name of the kind, as the variant is written; for [`Error::Der`] the DER kind's name.
    pub fn name(self) -> &'static str {
        match self {
            Error::Der(e) => e.name(),
            Error::UnsupportedVersion => "UnsupportedVersion",
            Error::SerialTooLong => "SerialTooLong",
            Error::UnsupportedAlgorithm => "UnsupportedAlgorithm",
            Error::AlgorithmMismatch => "AlgorithmMismatch",
            Error::UnsupportedKey => "UnsupportedKey",
            Error::BadKey => "BadKey",
            Error::DuplicateExtension => "DuplicateExtension",
            Error::BadExtension => "BadExtension",
            Error::BadName => "BadName",
        }
    }
}

impl From<der::Error> for Error {
    fn from(e: der::Error) -> Self {
        Error::Der(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Der(e) => write!(f, "{e}"),
            other => f.write_str(other.name()),
        }
    }
}

impl std::error::Error for Error {}

/// The validity of a certificate: from `not_before` to `not_after`, both included (RFC 5280
/// section 4.1.2.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Validity {
    /// The first second the certificate is valid.
    pub not_before: Time,
    /// The last second the certificate is valid.
    pub not_after: Time,
}

/// A certificate as read, borrowing the bytes it was read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Certificate<'a> {
    /// The whole certificate.
    pub der: &'a [u8],
    /// The TBSCertificate, tag and length included: the bytes the signature covers.
    pub tbs: &'a [u8],
    /// The version: always 3.
    pub version: u8,
    /// The content of the serial number's INTEGER.
    pub serial: &'a [u8],
    /// The signature algorithm.
    pub signature_algorithm: SignatureAlgorithm,
    /// The bytes of the signature (the BIT STRING's bytes).
    pub signature: &'a [u8],
    /// The issuer's name.
    pub issuer: Name<'a>,
    /// The validity.
    pub validity: Validity,
    /// The subject's name.
    pub subject: Name<'a>,
    /// The subject public key, in the form the seam checks signatures with.
    pub public_key: PublicKey<'a>,
    /// The SubjectPublicKeyInfo, tag and length included.
    pub spki: &'a [u8],
    /// The extensions.
    pub extensions: Extensions<'a>,
}

impl<'a> Certificate<'a> {
    /// Reads a certificate that is the whole of `der`.
    /// Every value of it is checked as DER first ([`der::parse`]), so a byte after the
    /// certificate is [`der::Error::TrailingData`].
    pub fn from_der(der: &'a [u8]) -> Result<Self, Error> {
        der::parse(der)?;
        let mut cert = Reader::new(der).sequence()?;
        let tbs_value = cert.expect(tag::SEQUENCE)?;
        let outer_algorithm = cert.expect(tag::SEQUENCE)?;
        let signature = cert.expect(tag::BIT_STRING)?.bit_string()?;
        cert.finish()?;
        if signature.unused_bits != 0 {
            return Err(Error::Der(der::Error::BadBitString));
        }

        let mut tbs = tbs_value.reader();
        let version = match tbs.optional(tag::context_constructed(0))? {
            None => None,
            Some(explicit) => {
                let mut inner = explicit.reader();
                let number = inner.expect(tag::INTEGER)?.small_unsigned()?;
                inner.finish()?;
                number
            }
        };
        // The INTEGER 2 is version 3; an absent version is version 1.
        if version != Some(2) {
            return Err(Error::UnsupportedVersion);
        }
        let serial = tbs.expect(tag::INTEGER)?.integer()?;
        if serial.len() > SERIAL_BYTES_MAX {
            return Err(Error::SerialTooLong);
        }
        let inner_algorithm = tbs.expect(tag::SEQUENCE)?;
        if inner_algorithm.raw() != outer_algorithm.raw() {
            return Err(Error::AlgorithmMismatch);
        }
        let signature_algorithm = algorithm::signature_algorithm(inner_algorithm.raw())?;
        let issuer = tbs.expect(tag::SEQUENCE)?;
        name::check_name(&issuer)?;
        let mut validity = tbs.sequence()?;
        let not_before = validity.read()?.time()?;
        let not_after = validity.read()?.time()?;
        validity.finish()?;
        let subject = tbs.expect(tag::SEQUENCE)?;
        name::check_name(&subject)?;
        let spki = tbs.expect(tag::SEQUENCE)?;
        let public_key = algorithm::public_key(spki.raw())?;
        // issuerUniqueID and subjectUniqueID, kept by no one.
        tbs.optional(tag::context(1))?;
        tbs.optional(tag::context(2))?;
        let extensions = match tbs.optional(tag::context_constructed(3))? {
            Some(explicit) => extensions::read_extensions(&explicit)?,
            None => Extensions::default(),
        };
        tbs.finish()?;

        Ok(Certificate {
            der,
            tbs: tbs_value.raw(),
            version: 3,
            serial,
            signature_algorithm,
            signature: signature.bytes,
            issuer: Name::new(issuer.raw()),
            validity: Validity {
                not_before,
                not_after,
            },
            subject: Name::new(subject.raw()),
            public_key,
            spki: spki.raw(),
            extensions,
        })
    }
}

/// The subject and the subject public key of a certificate, every other field skipped
/// unread: what a trust anchor keeps of a root the reader refuses (RFC 5280 section 6.1.1 (d)).
fn subject_and_key(der: &[u8]) -> Result<(Name<'_>, PublicKey<'_>), Error> {
    let mut outer = Reader::new(der);
    let mut cert = outer.sequence()?;
    outer.finish()?;
    let mut tbs = cert.sequence()?;
    tbs.optional(tag::context_constructed(0))?;
    tbs.expect(tag::INTEGER)?;
    tbs.expect(tag::SEQUENCE)?;
    tbs.expect(tag::SEQUENCE)?;
    tbs.expect(tag::SEQUENCE)?;
    let subject = tbs.expect(tag::SEQUENCE)?;
    name::check_name(&subject)?;
    let spki = tbs.expect(tag::SEQUENCE)?;
    let key = algorithm::public_key(spki.raw())?;
    Ok((Name::new(subject.raw()), key))
}
