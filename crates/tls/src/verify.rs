//! The verifier interface (docs/design/tls.md, items 6 and 7): the trait
//! [`ServerCertVerifier`], which takes the certificates the server sent, the host and the time
//! and answers the leaf's key or a [`CertificateRejection`]; and the closed list of [`Check`]s
//! a rejection of kind `NotTrusted` names. `story:tls-x509-path` implements the trait.
//!
//! [`LeafKey`] and [`TrustAnchor`] are defined in [`crate::x509`] and named here too.

use std::fmt;

use crate::der::Time;
use crate::error::AlertDescription;
use crate::x509;

pub use crate::x509::{LeafKey, TrustAnchor};

/// Validates the certificates a server sent for a host at a time.
pub trait ServerCertVerifier {
    /// The leaf's key when `certificates` (the leaf first, as the server sent them) are
    /// trusted for `host` at `now`; else why not.
    fn verify(
        &self,
        certificates: &[&[u8]],
        host: &str,
        now: Time,
    ) -> Result<LeafKey, CertificateRejection>;
}

/// How a path is too long (item 7, `PathTooLong`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathTooLong {
    /// More intermediates than item 9.3 allows.
    Intermediates {
        /// The intermediates of the path.
        count: usize,
        /// The most allowed.
        bound: usize,
    },
    /// A pathLenConstraint broken.
    PathLenConstraint {
        /// The certificate whose constraint is broken, the leaf 0.
        certificate: usize,
    },
}

/// Which limit of the path search of item 9.4 was reached (item 7, `CandidateLimit`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    /// The signature checks.
    SignatureChecks {
        /// The limit.
        bound: usize,
    },
    /// The candidates considered.
    CandidatesConsidered {
        /// The limit.
        bound: usize,
    },
}

/// The checks of `NotTrusted`: the closed list of item 7, in its order, with what its detail
/// column prints. `certificate` counts the certificates as the server sent them, the leaf 0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Check {
    /// A certificate the server sent that the reader refuses.
    Unreadable {
        /// The certificate.
        certificate: usize,
        /// The reader's error.
        error: x509::Error,
    },
    /// No path from the leaf to a trust anchor.
    NoPathToAnchor,
    /// A signature on the path does not verify.
    BadSignature {
        /// The certificate whose signature does not verify.
        certificate: usize,
    },
    /// An issuer without basicConstraints cA.
    IssuerNotCa {
        /// The issuer.
        certificate: usize,
    },
    /// Too many intermediates, or a pathLenConstraint broken.
    PathTooLong(PathTooLong),
    /// A limit of the path search reached.
    CandidateLimit(Limit),
    /// keyUsage without keyCertSign on an issuer, or without digitalSignature on the leaf.
    KeyUsage {
        /// The certificate.
        certificate: usize,
        /// `keyCertSign` or `digitalSignature`.
        lacks: &'static str,
    },
    /// A critical extension the reader does not know.
    UnknownCriticalExtension {
        /// The certificate.
        certificate: usize,
        /// The extension's OBJECT IDENTIFIER, dotted.
        oid: String,
    },
    /// A name outside a nameConstraints of an issuer or of the anchor.
    NameConstraint {
        /// The certificate whose constraints the name breaks.
        certificate: usize,
        /// The name.
        name: String,
    },
    /// extKeyUsage without serverAuth on the leaf.
    NoServerAuth,
    /// An MD5 or SHA-1 signature, decided before any verification.
    WeakAlgorithm {
        /// The certificate.
        certificate: usize,
        /// The algorithm's name.
        algorithm: &'static str,
    },
    /// An RSA key under 2048 bits, decided before any verification.
    WeakKey {
        /// The certificate.
        certificate: usize,
        /// The bits of its modulus.
        bits: usize,
    },
}

impl Check {
    /// One check of each kind, in the order of item 7; the fields are placeholders.
    pub const ALL: [Check; 12] = [
        Check::Unreadable {
            certificate: 0,
            error: x509::Error::Der(crate::der::Error::Truncated),
        },
        Check::NoPathToAnchor,
        Check::BadSignature { certificate: 0 },
        Check::IssuerNotCa { certificate: 0 },
        Check::PathTooLong(PathTooLong::Intermediates { count: 0, bound: 0 }),
        Check::CandidateLimit(Limit::SignatureChecks { bound: 0 }),
        Check::KeyUsage {
            certificate: 0,
            lacks: "keyCertSign",
        },
        Check::UnknownCriticalExtension {
            certificate: 0,
            oid: String::new(),
        },
        Check::NameConstraint {
            certificate: 0,
            name: String::new(),
        },
        Check::NoServerAuth,
        Check::WeakAlgorithm {
            certificate: 0,
            algorithm: "",
        },
        Check::WeakKey {
            certificate: 0,
            bits: 0,
        },
    ];

    /// The name of the check, as item 7 writes it.
    pub fn name(&self) -> &'static str {
        match self {
            Check::Unreadable { .. } => "Unreadable",
            Check::NoPathToAnchor => "NoPathToAnchor",
            Check::BadSignature { .. } => "BadSignature",
            Check::IssuerNotCa { .. } => "IssuerNotCa",
            Check::PathTooLong(_) => "PathTooLong",
            Check::CandidateLimit(_) => "CandidateLimit",
            Check::KeyUsage { .. } => "KeyUsage",
            Check::UnknownCriticalExtension { .. } => "UnknownCriticalExtension",
            Check::NameConstraint { .. } => "NameConstraint",
            Check::NoServerAuth => "NoServerAuth",
            Check::WeakAlgorithm { .. } => "WeakAlgorithm",
            Check::WeakKey { .. } => "WeakKey",
        }
    }
}

/// The detail of item 7's table.
impl fmt::Display for Check {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name();
        match self {
            Check::Unreadable { certificate, error } => {
                write!(
                    f,
                    "{name}: certificate {certificate} of the server: {error}"
                )
            }
            Check::NoPathToAnchor => write!(f, "{name}: no path to a trust anchor"),
            Check::BadSignature { certificate } => write!(
                f,
                "{name}: the signature of certificate {certificate} does not verify"
            ),
            Check::IssuerNotCa { certificate } => {
                write!(f, "{name}: certificate {certificate} issues but is no CA")
            }
            Check::PathTooLong(PathTooLong::Intermediates { count, bound }) => {
                write!(f, "{name}: {count} intermediates, at most {bound}")
            }
            Check::PathTooLong(PathTooLong::PathLenConstraint { certificate }) => write!(
                f,
                "{name}: pathLenConstraint of certificate {certificate} broken"
            ),
            Check::CandidateLimit(Limit::SignatureChecks { bound }) => {
                write!(f, "{name}: {bound} signature checks")
            }
            Check::CandidateLimit(Limit::CandidatesConsidered { bound }) => {
                write!(f, "{name}: {bound} candidates considered")
            }
            Check::KeyUsage { certificate, lacks } => {
                write!(f, "{name}: certificate {certificate} lacks {lacks}")
            }
            Check::UnknownCriticalExtension { certificate, oid } => {
                write!(f, "{name}: {oid} in certificate {certificate}")
            }
            Check::NameConstraint {
                certificate,
                name: n,
            } => write!(
                f,
                "{name}: {n} outside the constraints of certificate {certificate}"
            ),
            Check::NoServerAuth => write!(f, "{name}: the leaf is not for TLS servers"),
            Check::WeakAlgorithm {
                certificate,
                algorithm,
            } => write!(f, "{name}: {algorithm} in certificate {certificate}"),
            Check::WeakKey { certificate, bits } => {
                write!(f, "{name}: RSA {bits} bits in certificate {certificate}")
            }
        }
    }
}

/// Why the certificates of a server are not accepted (item 6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CertificateRejection {
    /// No trusted path: the check that failed. Failure reason `CertificateNotTrusted`.
    NotTrusted(Check),
    /// A certificate on the path is outside its validity. Failure reason `CertificateExpired`.
    Expired {
        /// The certificate, the leaf 0.
        certificate: usize,
        /// Its notBefore.
        not_before: Time,
        /// Its notAfter.
        not_after: Time,
        /// The time checked at.
        at: Time,
    },
    /// No subjectAltName entry of the leaf matches the host. Failure reason
    /// `CertificateNameMismatch`.
    NameMismatch {
        /// The host.
        host: String,
    },
}

impl CertificateRejection {
    /// The alert surf sends: `unknown_ca` for `NoPathToAnchor`, `certificate_expired` for
    /// `Expired`, `bad_certificate` for every other.
    pub fn alert(&self) -> AlertDescription {
        match self {
            CertificateRejection::NotTrusted(Check::NoPathToAnchor) => AlertDescription::UNKNOWN_CA,
            CertificateRejection::Expired { .. } => AlertDescription::CERTIFICATE_EXPIRED,
            _ => AlertDescription::BAD_CERTIFICATE,
        }
    }
}

/// The detail of item 7.
impl fmt::Display for CertificateRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CertificateRejection::NotTrusted(check) => write!(f, "{check}"),
            CertificateRejection::Expired {
                certificate,
                not_before,
                not_after,
                at,
            } => write!(
                f,
                "certificate {certificate} valid from {not_before} to {not_after}, checked at {at}"
            ),
            CertificateRejection::NameMismatch { host } => {
                write!(f, "no subjectAltName entry matches {host}")
            }
        }
    }
}

impl std::error::Error for CertificateRejection {}
