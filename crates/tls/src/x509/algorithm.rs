//! Signature algorithms and subject public keys (RFC 3279, 4055, 5480, 5758, 8410).

use super::Error;
use crate::crypto::{PublicKey, PublicKeyKind, SignatureAlgorithm, SignatureHash};
use crate::der::{self, Reader, tag};

const MD5_WITH_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x04];
const SHA1_WITH_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x05];
const SHA256_WITH_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B];
const SHA384_WITH_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0C];
const SHA512_WITH_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0D];
const RSASSA_PSS: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0A];
const MGF1: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x08];
const RSA_ENCRYPTION: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01];
const ECDSA_WITH_SHA1: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x01];
const ECDSA_WITH_SHA256: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x02];
const ECDSA_WITH_SHA384: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x03];
const ECDSA_WITH_SHA512: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x04];
const ED25519: &[u8] = &[0x2B, 0x65, 0x70];
const EC_PUBLIC_KEY: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01];
const P256: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07];
const P384: &[u8] = &[0x2B, 0x81, 0x04, 0x00, 0x22];
const SHA256: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
const SHA384: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02];
const SHA512: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03];

/// An AlgorithmIdentifier: the content of its OBJECT IDENTIFIER and its parameters, if any.
fn algorithm_identifier(raw: &[u8]) -> Result<(&[u8], Option<der::Value<'_>>), Error> {
    let mut outer = Reader::new(raw);
    let mut r = outer.sequence()?;
    outer.finish()?;
    let oid = r.expect(tag::OID)?.oid()?;
    let params = if r.is_empty() { None } else { Some(r.read()?) };
    r.finish()?;
    Ok((oid, params))
}

fn is_null(params: &Option<der::Value<'_>>) -> bool {
    matches!(params, Some(p) if p.tag() == tag::NULL && p.content().is_empty())
}

/// Reads an AlgorithmIdentifier of a signature (its whole value). RSA PKCS#1 v1.5 takes a
/// NULL or no parameters; ECDSA and Ed25519 none; RSASSA-PSS exactly those of RFC 4055 for
/// SHA-256, SHA-384 or SHA-512 (MGF1 of the same hash, a salt of the digest length, trailer 1).
pub(super) fn signature_algorithm(raw: &[u8]) -> Result<SignatureAlgorithm, Error> {
    use SignatureHash::{Md5, Sha1, Sha256, Sha384, Sha512};
    let (oid, params) = algorithm_identifier(raw)?;
    let rsa = |hash| {
        if params.is_none() || is_null(&params) {
            Ok(SignatureAlgorithm::RsaPkcs1(hash))
        } else {
            Err(Error::UnsupportedAlgorithm)
        }
    };
    let bare = |algorithm| {
        if params.is_none() {
            Ok(algorithm)
        } else {
            Err(Error::UnsupportedAlgorithm)
        }
    };
    match oid {
        MD5_WITH_RSA => rsa(Md5),
        SHA1_WITH_RSA => rsa(Sha1),
        SHA256_WITH_RSA => rsa(Sha256),
        SHA384_WITH_RSA => rsa(Sha384),
        SHA512_WITH_RSA => rsa(Sha512),
        ECDSA_WITH_SHA1 => bare(SignatureAlgorithm::Ecdsa(Sha1)),
        ECDSA_WITH_SHA256 => bare(SignatureAlgorithm::Ecdsa(Sha256)),
        ECDSA_WITH_SHA384 => bare(SignatureAlgorithm::Ecdsa(Sha384)),
        ECDSA_WITH_SHA512 => bare(SignatureAlgorithm::Ecdsa(Sha512)),
        ED25519 => bare(SignatureAlgorithm::Ed25519),
        RSASSA_PSS => match params {
            Some(p) => pss(&p)
                .ok_or(Error::UnsupportedAlgorithm)
                .map(SignatureAlgorithm::RsaPss),
            None => Err(Error::UnsupportedAlgorithm),
        },
        _ => Err(Error::UnsupportedAlgorithm),
    }
}

/// The hash of a hash AlgorithmIdentifier of SHA-2 with a NULL or no parameters.
fn sha2(value: &der::Value<'_>) -> Option<(SignatureHash, u32)> {
    let (oid, params) = algorithm_identifier(value.raw()).ok()?;
    if params.is_some() && !is_null(&params) {
        return None;
    }
    match oid {
        SHA256 => Some((SignatureHash::Sha256, 32)),
        SHA384 => Some((SignatureHash::Sha384, 48)),
        SHA512 => Some((SignatureHash::Sha512, 64)),
        _ => None,
    }
}

/// The one value inside `[n]` EXPLICIT, if the reader is at it.
fn explicit<'a>(r: &mut Reader<'a>, n: u8) -> Option<der::Value<'a>> {
    let v = r.optional(tag::context_constructed(n)).ok()??;
    let mut inner = v.reader();
    let value = inner.read().ok()?;
    inner.finish().ok()?;
    Some(value)
}

/// The hash of RSASSA-PSS-params when they are those of RFC 4055 for one SHA-2 hash; `None`
/// for any other parameters (the defaults, SHA-1, are other parameters).
fn pss(params: &der::Value<'_>) -> Option<SignatureHash> {
    if params.tag() != tag::SEQUENCE {
        return None;
    }
    let mut r = params.reader();
    let (hash, len) = sha2(&explicit(&mut r, 0)?)?;
    let mgf = explicit(&mut r, 1)?;
    let (mgf_oid, mgf_params) = algorithm_identifier(mgf.raw()).ok()?;
    if mgf_oid != MGF1 || sha2(&mgf_params?)?.0 != hash {
        return None;
    }
    let salt = explicit(&mut r, 2)?;
    if salt.tag() != tag::INTEGER || salt.small_unsigned().ok()?? != len {
        return None;
    }
    if r.peek_tag() == Some(tag::context_constructed(3)) {
        let trailer = explicit(&mut r, 3)?;
        if trailer.tag() != tag::INTEGER || trailer.small_unsigned().ok()?? != 1 {
            return None;
        }
    }
    r.finish().ok()?;
    Some(hash)
}

/// Reads a SubjectPublicKeyInfo (its whole value): an RSA key (rsaEncryption with a NULL; the
/// key bytes the RSAPublicKey), an EC key on P-256 or P-384 (the uncompressed point), or an
/// Ed25519 key (32 bytes).
pub(super) fn public_key(raw: &[u8]) -> Result<PublicKey<'_>, Error> {
    let mut outer = Reader::new(raw);
    let mut r = outer.sequence()?;
    outer.finish()?;
    let algorithm = r.expect(tag::SEQUENCE)?;
    let bits = r.expect(tag::BIT_STRING)?.bit_string()?;
    r.finish()?;
    let (oid, params) = algorithm_identifier(algorithm.raw())?;
    if bits.unused_bits != 0 {
        return Err(Error::BadKey);
    }
    let bytes = bits.bytes;
    let kind = match oid {
        RSA_ENCRYPTION => {
            if !is_null(&params) {
                return Err(Error::BadKey);
            }
            PublicKeyKind::Rsa {
                bits: rsa_bits(bytes)?,
            }
        }
        EC_PUBLIC_KEY => {
            let curve = match &params {
                Some(p) if p.tag() == tag::OID => p.content(),
                _ => return Err(Error::BadKey),
            };
            let (kind, len) = match curve {
                P256 => (PublicKeyKind::EcP256, 65),
                P384 => (PublicKeyKind::EcP384, 97),
                _ => return Err(Error::UnsupportedKey),
            };
            if bytes.len() != len || bytes[0] != 0x04 {
                return Err(Error::BadKey);
            }
            kind
        }
        ED25519 => {
            if params.is_some() || bytes.len() != 32 {
                return Err(Error::BadKey);
            }
            PublicKeyKind::Ed25519
        }
        _ => return Err(Error::UnsupportedKey),
    };
    Ok(PublicKey { kind, bytes })
}

/// The bits of the modulus of an RSAPublicKey (RFC 8017 appendix A.1.1): a SEQUENCE of two
/// positive INTEGERs.
fn rsa_bits(key: &[u8]) -> Result<usize, Error> {
    let bad = |_| Error::BadKey;
    let value = der::parse(key).map_err(bad)?;
    if value.tag() != tag::SEQUENCE {
        return Err(Error::BadKey);
    }
    let mut r = value.reader();
    let modulus = r
        .expect(tag::INTEGER)
        .map_err(bad)?
        .integer()
        .map_err(bad)?;
    let exponent = r
        .expect(tag::INTEGER)
        .map_err(bad)?
        .integer()
        .map_err(bad)?;
    r.finish().map_err(bad)?;
    if modulus[0] & 0x80 != 0 || exponent[0] & 0x80 != 0 {
        return Err(Error::BadKey);
    }
    let modulus = if modulus[0] == 0 {
        &modulus[1..]
    } else {
        modulus
    };
    match modulus.first() {
        None => Err(Error::BadKey),
        Some(&top) => Ok(modulus.len() * 8 - top.leading_zeros() as usize),
    }
}
