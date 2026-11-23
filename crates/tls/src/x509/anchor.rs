//! `LeafKey` and `TrustAnchor` (docs/design/tls.md, item 6): what path validation answers and
//! what it validates against.

use std::sync::Arc;

use super::{Certificate, Error, Name, NameConstraints};
use crate::crypto::{PublicKey, PublicKeyKind};

/// A public key as the seam checks signatures with it: its kind and its bytes, owned. What a
/// [`crate::verify::ServerCertVerifier`] answers for the leaf, and the key of a
/// [`TrustAnchor`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafKey {
    /// The kind of the key.
    pub kind: PublicKeyKind,
    /// The bytes, in the form [`PublicKeyKind`] names.
    pub bytes: Vec<u8>,
}

impl LeafKey {
    /// The subject public key of `cert`.
    pub fn from_certificate(cert: &Certificate<'_>) -> Self {
        LeafKey {
            kind: cert.public_key.kind,
            bytes: cert.public_key.bytes.to_vec(),
        }
    }

    /// The key as the seam takes it.
    pub fn public_key(&self) -> PublicKey<'_> {
        PublicKey {
            kind: self.kind,
            bytes: &self.bytes,
        }
    }
}

/// A trust anchor (RFC 5280 section 6.1.1 (d)): a subject name, a key and, when made from a
/// certificate the reader reads, its nameConstraints and the certificate itself, so that path
/// validation holds the anchor to the checks of an issuer (docs/design/tls.md item 9.4:
/// validity, basicConstraints and pathLenConstraint, keyUsage, nameConstraints), however the
/// anchor was made. Owned, so that it outlives the bytes it was read from; the certificate is
/// shared between clones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustAnchor {
    subject: Vec<u8>,
    /// The key.
    pub key: LeafKey,
    name_constraints: Option<Vec<u8>>,
    certificate: Option<Arc<[u8]>>,
    /// The reader's refusal, when the anchor was made from a certificate it refuses (a version
    /// 1 root, for one) that holds no nameConstraints, and so holds the name and the key only.
    pub refused: Option<Error>,
}

impl TrustAnchor {
    /// The anchor of a certificate the reader read: its subject, key, nameConstraints and the
    /// certificate.
    pub fn from_certificate(cert: &Certificate<'_>) -> Self {
        TrustAnchor {
            subject: cert.subject.der().to_vec(),
            key: LeafKey::from_certificate(cert),
            name_constraints: cert.extensions.name_constraints_der.map(<[u8]>::to_vec),
            certificate: Some(Arc::from(cert.der)),
            refused: None,
        }
    }

    /// The certificate the anchor was made from, when the reader read it; `None` for an anchor
    /// of name and key alone ([`TrustAnchor::refused`]).
    pub fn certificate(&self) -> Option<&[u8]> {
        self.certificate.as_deref()
    }

    /// The anchor of the certificate `der`: from the whole certificate when the reader reads
    /// it, else from its subject and subject public key alone, with the refusal in
    /// [`TrustAnchor::refused`], but only when a walk over the certificate shows that it holds
    /// no nameConstraints (D164: an anchor of name and key alone would drop them). The reader's
    /// error when the certificate names nameConstraints, when its fields after the key cannot
    /// be walked, or when its subject or key do not read.
    pub fn from_der(der: &[u8]) -> Result<Self, Error> {
        let refused = match Certificate::from_der(der) {
            Ok(cert) => return Ok(TrustAnchor::from_certificate(&cert)),
            Err(e) => e,
        };
        let (subject, key) = super::subject_and_key(der).map_err(|_| refused)?;
        Ok(TrustAnchor {
            subject: subject.der().to_vec(),
            key: LeafKey {
                kind: key.kind,
                bytes: key.bytes.to_vec(),
            },
            name_constraints: None,
            certificate: None,
            refused: Some(refused),
        })
    }

    /// The subject.
    pub fn subject(&self) -> Name<'_> {
        Name::new(&self.subject)
    }

    /// The nameConstraints, if the anchor has them.
    pub fn name_constraints(&self) -> Result<Option<NameConstraints<'_>>, Error> {
        self.name_constraints
            .as_deref()
            .map(super::extensions::read_name_constraints)
            .transpose()
    }
}
