//! Reads back what the builder wrote, for the tests of this crate: the parts of a certificate a
//! signature check needs, the public key and the dNSNames. It checks no more than it must to
//! find them; it is not the X.509 reader of `surf-tls`, which this crate is built beside.

use surf_tls::crypto::{PublicKeyKind, SignatureAlgorithm, SignatureHash};

use crate::der::{self, tag};

/// The parts of a certificate.
#[derive(Clone, Copy, Debug)]
pub struct Certificate<'a> {
    /// The TBSCertificate, tag and length included: what the signature covers.
    pub tbs: &'a [u8],
    /// The content of the `[0]` version field, if there is one.
    pub version: Option<&'a [u8]>,
    /// The issuer's Name, tag and length included.
    pub issuer: &'a [u8],
    /// The subject's Name, tag and length included.
    pub subject: &'a [u8],
    /// The SubjectPublicKeyInfo, tag and length included.
    pub spki: &'a [u8],
    /// The content of the SEQUENCE of extensions, if there is one.
    pub extensions: Option<&'a [u8]>,
    /// The outer signatureAlgorithm, tag and length included.
    pub signature_algorithm: &'a [u8],
    /// The signature, without the BIT STRING's count of unused bits.
    pub signature: &'a [u8],
}

/// The parts of the certificate `der`.
pub fn certificate(der: &[u8]) -> Result<Certificate<'_>, String> {
    let outer = der::expect(der, tag::SEQUENCE)?;
    if !outer.rest.is_empty() {
        return Err("bytes after the certificate".into());
    }
    let tbs = der::expect(outer.content, tag::SEQUENCE)?;
    let algorithm = der::expect(tbs.rest, tag::SEQUENCE)?;
    let signature = der::expect(algorithm.rest, tag::BIT_STRING)?;
    if !signature.rest.is_empty() {
        return Err("bytes after the signature".into());
    }
    let (&unused, signature_bytes) = signature.content.split_first().ok_or("empty signature")?;
    if unused != 0 {
        return Err("a signature of part of a byte".into());
    }

    let mut fields = der::elements(tbs.content)?.into_iter().peekable();
    let version = match fields.peek() {
        Some(e) if e.tag == 0xa0 => Some(fields.next().expect("peeked").content),
        _ => None,
    };
    let mut next = |what: &str, want: u8| {
        let e = fields.next().ok_or_else(|| format!("no {what}"))?;
        if e.tag != want {
            return Err(format!("{what}: tag {:#04x}", e.tag));
        }
        Ok::<_, String>(e)
    };
    next("serialNumber", tag::INTEGER)?;
    next("signature", tag::SEQUENCE)?;
    let issuer = next("issuer", tag::SEQUENCE)?.whole;
    next("validity", tag::SEQUENCE)?;
    let subject = next("subject", tag::SEQUENCE)?.whole;
    let spki = next("subjectPublicKeyInfo", tag::SEQUENCE)?.whole;
    let mut extensions = None;
    for e in fields {
        if e.tag == 0xa3 {
            extensions = Some(der::expect(e.content, tag::SEQUENCE)?.content);
        }
    }
    Ok(Certificate {
        tbs: tbs.whole,
        version,
        issuer,
        subject,
        spki,
        extensions,
        signature_algorithm: algorithm.whole,
        signature: signature_bytes,
    })
}

/// The kind and the bytes of the public key of a SubjectPublicKeyInfo, in the form
/// `surf_tls::crypto::PublicKey` takes.
pub fn public_key(spki: &[u8]) -> Result<(PublicKeyKind, &[u8]), String> {
    let spki = der::expect(spki, tag::SEQUENCE)?;
    let algorithm = der::expect(spki.content, tag::SEQUENCE)?;
    let bits = der::expect(algorithm.rest, tag::BIT_STRING)?;
    let (&unused, key) = bits.content.split_first().ok_or("empty key")?;
    if unused != 0 {
        return Err("a key of part of a byte".into());
    }
    let parts = der::elements(algorithm.content)?;
    let oid = parts.first().ok_or("no algorithm")?.whole;
    let parameter = parts.get(1).map(|p| p.whole);
    let kind = if oid == der::oid("1.2.840.113549.1.1.1") {
        let rsa = der::expect(key, tag::SEQUENCE)?;
        let modulus = der::expect(rsa.content, tag::INTEGER)?.content;
        let skip = modulus.iter().take_while(|b| **b == 0).count();
        let modulus = &modulus[skip..];
        let first = *modulus.first().ok_or("a modulus of zero")?;
        PublicKeyKind::Rsa {
            bits: modulus.len() * 8 - first.leading_zeros() as usize,
        }
    } else if oid == der::oid("1.2.840.10045.2.1") {
        match parameter {
            Some(p) if p == der::oid("1.2.840.10045.3.1.7") => PublicKeyKind::EcP256,
            Some(p) if p == der::oid("1.3.132.0.34") => PublicKeyKind::EcP384,
            _ => return Err("a curve other than P-256 and P-384".into()),
        }
    } else if oid == der::oid("1.3.101.112") {
        PublicKeyKind::Ed25519
    } else {
        return Err("an unknown key algorithm".into());
    };
    Ok((kind, key))
}

/// The signature algorithm of an AlgorithmIdentifier.
pub fn signature_algorithm(identifier: &[u8]) -> Result<SignatureAlgorithm, String> {
    use SignatureHash::{Md5, Sha1, Sha256, Sha384, Sha512};
    let identifier = der::expect(identifier, tag::SEQUENCE)?;
    let oid = der::read(identifier.content)?.whole;
    let pkcs1 = |last: u8| der::oid(&format!("1.2.840.113549.1.1.{last}"));
    let algorithm = if oid == pkcs1(4) {
        SignatureAlgorithm::RsaPkcs1(Md5)
    } else if oid == pkcs1(5) {
        SignatureAlgorithm::RsaPkcs1(Sha1)
    } else if oid == pkcs1(11) {
        SignatureAlgorithm::RsaPkcs1(Sha256)
    } else if oid == pkcs1(12) {
        SignatureAlgorithm::RsaPkcs1(Sha384)
    } else if oid == pkcs1(13) {
        SignatureAlgorithm::RsaPkcs1(Sha512)
    } else if oid == pkcs1(10) {
        let params = der::expect(der::read(identifier.content)?.rest, tag::SEQUENCE)?;
        let hash = der::expect(params.content, 0xa0)?;
        let hash = der::expect(hash.content, tag::SEQUENCE)?;
        let hash = der::read(hash.content)?.whole;
        let sha2 = |last: u8| der::oid(&format!("2.16.840.1.101.3.4.2.{last}"));
        SignatureAlgorithm::RsaPss(if hash == sha2(1) {
            Sha256
        } else if hash == sha2(2) {
            Sha384
        } else if hash == sha2(3) {
            Sha512
        } else {
            return Err("an RSASSA-PSS hash other than SHA-2".into());
        })
    } else if oid == der::oid("1.2.840.10045.4.3.2") {
        SignatureAlgorithm::Ecdsa(Sha256)
    } else if oid == der::oid("1.2.840.10045.4.3.3") {
        SignatureAlgorithm::Ecdsa(Sha384)
    } else if oid == der::oid("1.3.101.112") {
        SignatureAlgorithm::Ed25519
    } else {
        return Err("an unknown signature algorithm".into());
    };
    Ok(algorithm)
}

/// The value of the extension `oid` (dotted), if the certificate has it.
pub fn extension<'a>(certificate: &Certificate<'a>, oid: &str) -> Result<Option<&'a [u8]>, String> {
    let wanted = der::oid(oid);
    for extension in der::elements(certificate.extensions.unwrap_or(&[]))? {
        let parts = der::elements(extension.content)?;
        if parts.first().map(|p| p.whole) == Some(&wanted[..]) {
            let value = parts.last().ok_or("an extension without a value")?;
            return Ok(Some(value.content));
        }
    }
    Ok(None)
}

/// The dNSName entries of subjectAltName, in order.
pub fn dns_names<'a>(certificate: &Certificate<'a>) -> Result<Vec<&'a [u8]>, String> {
    let Some(value) = extension(certificate, "2.5.29.17")? else {
        return Ok(Vec::new());
    };
    let names = der::expect(value, tag::SEQUENCE)?;
    Ok(der::elements(names.content)?
        .into_iter()
        .filter(|e| e.tag == 0x82)
        .map(|e| e.content)
        .collect())
}
