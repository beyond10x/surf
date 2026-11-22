//! The extensions of a certificate the reader knows (RFC 5280 section 4.2).

use super::name::{Place, general_name};
use super::{Error, GeneralName};
use crate::der::{self, Reader, Value, tag};

/// basicConstraints (RFC 5280 section 4.2.1.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BasicConstraints {
    /// cA.
    pub ca: bool,
    /// pathLenConstraint, if present.
    pub path_len: Option<u32>,
}

/// keyUsage (RFC 5280 section 4.2.1.3): the named bits, bit 0 (digitalSignature) the most
/// significant bit of the first byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyUsage(pub u16);

impl KeyUsage {
    /// digitalSignature (0).
    pub const DIGITAL_SIGNATURE: u8 = 0;
    /// nonRepudiation, also contentCommitment (1).
    pub const NON_REPUDIATION: u8 = 1;
    /// keyEncipherment (2).
    pub const KEY_ENCIPHERMENT: u8 = 2;
    /// dataEncipherment (3).
    pub const DATA_ENCIPHERMENT: u8 = 3;
    /// keyAgreement (4).
    pub const KEY_AGREEMENT: u8 = 4;
    /// keyCertSign (5).
    pub const KEY_CERT_SIGN: u8 = 5;
    /// cRLSign (6).
    pub const CRL_SIGN: u8 = 6;
    /// encipherOnly (7).
    pub const ENCIPHER_ONLY: u8 = 7;
    /// decipherOnly (8).
    pub const DECIPHER_ONLY: u8 = 8;

    /// Whether the bit `bit` (one of the constants) is set.
    pub fn has(self, bit: u8) -> bool {
        bit < 16 && self.0 & (0x8000 >> bit) != 0
    }
}

/// The content of the OBJECT IDENTIFIER id-kp-serverAuth, 1.3.6.1.5.5.7.3.1.
pub const SERVER_AUTH: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x01];

/// The content of the OBJECT IDENTIFIER anyExtendedKeyUsage, 2.5.29.37.0.
pub const ANY_EXTENDED_KEY_USAGE: &[u8] = &[0x55, 0x1D, 0x25, 0x00];

/// extKeyUsage (RFC 5280 section 4.2.1.12): the purposes, each the content of its OBJECT
/// IDENTIFIER, in the order written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtKeyUsage<'a> {
    /// The purposes.
    pub purposes: Vec<&'a [u8]>,
}

impl ExtKeyUsage<'_> {
    /// Whether id-kp-serverAuth is among the purposes.
    pub fn server_auth(&self) -> bool {
        self.purposes.contains(&SERVER_AUTH)
    }

    /// Whether anyExtendedKeyUsage is among the purposes.
    pub fn any(&self) -> bool {
        self.purposes.contains(&ANY_EXTENDED_KEY_USAGE)
    }
}

/// nameConstraints (RFC 5280 section 4.2.1.10): the bases of the permitted and excluded
/// subtrees. An iPAddress base is the address and its mask.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameConstraints<'a> {
    /// permittedSubtrees, if present.
    pub permitted: Option<Vec<GeneralName<'a>>>,
    /// excludedSubtrees, if present.
    pub excluded: Option<Vec<GeneralName<'a>>>,
}

/// Which of the extensions the reader knows are present and marked critical. The reader decides
/// nothing by them; path validation (`story:tls-x509-path`) does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Critical {
    /// basicConstraints.
    pub basic_constraints: bool,
    /// keyUsage.
    pub key_usage: bool,
    /// extKeyUsage.
    pub ext_key_usage: bool,
    /// subjectAltName.
    pub subject_alt_name: bool,
    /// nameConstraints.
    pub name_constraints: bool,
    /// subjectKeyIdentifier.
    pub subject_key_id: bool,
    /// authorityKeyIdentifier.
    pub authority_key_id: bool,
}

/// The extensions of a certificate.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Extensions<'a> {
    /// basicConstraints.
    pub basic_constraints: Option<BasicConstraints>,
    /// keyUsage.
    pub key_usage: Option<KeyUsage>,
    /// extKeyUsage.
    pub ext_key_usage: Option<ExtKeyUsage<'a>>,
    /// subjectAltName: its names in the order written.
    pub subject_alt_name: Option<Vec<GeneralName<'a>>>,
    /// nameConstraints.
    pub name_constraints: Option<NameConstraints<'a>>,
    /// The raw value of nameConstraints (the extnValue's content), for a trust anchor to keep.
    pub name_constraints_der: Option<&'a [u8]>,
    /// subjectKeyIdentifier.
    pub subject_key_id: Option<&'a [u8]>,
    /// The keyIdentifier of authorityKeyIdentifier.
    pub authority_key_id: Option<&'a [u8]>,
    /// Whether authorityKeyIdentifier holds `[1]` authorityCertIssuer.
    pub authority_cert_issuer: bool,
    /// Whether authorityKeyIdentifier holds `[2]` authorityCertSerialNumber.
    pub authority_cert_serial: bool,
    /// Which of the extensions above are marked critical.
    pub critical: Critical,
    /// The OBJECT IDENTIFIER (its content) of the first extension marked critical that the
    /// reader does not know; path validation rejects a certificate that has one.
    pub unknown_critical: Option<&'a [u8]>,
}

const SUBJECT_KEY_ID: &[u8] = &[0x55, 0x1D, 0x0E];
const KEY_USAGE: &[u8] = &[0x55, 0x1D, 0x0F];
const SUBJECT_ALT_NAME: &[u8] = &[0x55, 0x1D, 0x11];
const BASIC_CONSTRAINTS: &[u8] = &[0x55, 0x1D, 0x13];
const NAME_CONSTRAINTS: &[u8] = &[0x55, 0x1D, 0x1E];
const AUTHORITY_KEY_ID: &[u8] = &[0x55, 0x1D, 0x23];
const EXT_KEY_USAGE: &[u8] = &[0x55, 0x1D, 0x25];

/// One Extension: its OBJECT IDENTIFIER, whether critical, and the content of extnValue.
struct Extension<'a> {
    oid: &'a [u8],
    critical: bool,
    value: &'a [u8],
}

fn extension<'a>(reader: &mut Reader<'a>) -> Result<Extension<'a>, Error> {
    let mut ext = reader.sequence()?;
    let oid = ext.expect(tag::OID)?.oid()?;
    let critical = match ext.optional(tag::BOOLEAN)? {
        Some(b) => b.boolean()?,
        None => false,
    };
    let value = ext.expect(tag::OCTET_STRING)?.content();
    ext.finish()?;
    Ok(Extension {
        oid,
        critical,
        value,
    })
}

/// Reads the content of `[3]`: the SEQUENCE of extensions. Duplicates are found by sorting the
/// identifiers, so the work grows with n log n of the extensions present.
pub(super) fn read_extensions<'a>(explicit: &Value<'a>) -> Result<Extensions<'a>, Error> {
    let mut outer = explicit.reader();
    let list = outer.expect(tag::SEQUENCE)?;
    outer.finish()?;

    let mut oids = Vec::new();
    let mut all = list.reader();
    while !all.is_empty() {
        oids.push(extension(&mut all)?.oid);
    }
    // Extensions ::= SEQUENCE SIZE (1..MAX) OF Extension (RFC 5280 section 4.1).
    if oids.is_empty() {
        return Err(Error::BadExtension);
    }
    oids.sort_unstable();
    if oids.windows(2).any(|w| w[0] == w[1]) {
        return Err(Error::DuplicateExtension);
    }

    let mut out = Extensions::default();
    let mut all = list.reader();
    while !all.is_empty() {
        let ext = extension(&mut all)?;
        let critical = ext.critical;
        let flags = &mut out.critical;
        match ext.oid {
            BASIC_CONSTRAINTS => {
                out.basic_constraints = Some(basic_constraints(ext.value)?);
                flags.basic_constraints = critical;
            }
            KEY_USAGE => {
                out.key_usage = Some(key_usage(ext.value)?);
                flags.key_usage = critical;
            }
            EXT_KEY_USAGE => {
                out.ext_key_usage = Some(ext_key_usage(ext.value)?);
                flags.ext_key_usage = critical;
            }
            SUBJECT_ALT_NAME => {
                out.subject_alt_name = Some(subject_alt_name(ext.value)?);
                flags.subject_alt_name = critical;
            }
            NAME_CONSTRAINTS => {
                out.name_constraints = Some(read_name_constraints(ext.value)?);
                out.name_constraints_der = Some(ext.value);
                flags.name_constraints = critical;
            }
            SUBJECT_KEY_ID => {
                out.subject_key_id = Some(subject_key_id(ext.value)?);
                flags.subject_key_id = critical;
            }
            AUTHORITY_KEY_ID => {
                let aki = authority_key_id(ext.value)?;
                out.authority_key_id = aki.key_id;
                out.authority_cert_issuer = aki.cert_issuer;
                out.authority_cert_serial = aki.cert_serial;
                flags.authority_key_id = critical;
            }
            other => {
                if ext.critical && out.unknown_critical.is_none() {
                    out.unknown_critical = Some(other);
                }
            }
        }
    }
    Ok(out)
}

/// Walks the content of `[3]` to its end, reading each extension's OBJECT IDENTIFIER and
/// interpreting no value: whether one of them is nameConstraints. A list that cannot be walked
/// is an error.
pub(super) fn names_name_constraints(explicit: &Value<'_>) -> Result<bool, Error> {
    let mut outer = explicit.reader();
    let list = outer.expect(tag::SEQUENCE)?;
    outer.finish()?;
    let mut all = list.reader();
    let mut found = false;
    while !all.is_empty() {
        let mut ext = all.sequence()?;
        found |= ext.expect(tag::OID)?.oid()? == NAME_CONSTRAINTS;
    }
    Ok(found)
}

/// The value of an extension, one DER value checked whole; a shape error is
/// [`Error::BadExtension`].
fn value(bytes: &[u8]) -> Result<Value<'_>, Error> {
    Ok(der::parse(bytes)?)
}

fn shape<T>(result: Result<T, der::Error>) -> Result<T, Error> {
    result.map_err(|_| Error::BadExtension)
}

fn basic_constraints(bytes: &[u8]) -> Result<BasicConstraints, Error> {
    let v = value(bytes)?;
    if v.tag() != tag::SEQUENCE {
        return Err(Error::BadExtension);
    }
    let mut r = v.reader();
    let ca = match shape(r.optional(tag::BOOLEAN))? {
        Some(b) => shape(b.boolean())?,
        None => false,
    };
    let path_len = match shape(r.optional(tag::INTEGER))? {
        Some(i) => Some(shape(i.small_unsigned())?.ok_or(Error::BadExtension)?),
        None => None,
    };
    shape(r.finish())?;
    Ok(BasicConstraints { ca, path_len })
}

fn key_usage(bytes: &[u8]) -> Result<KeyUsage, Error> {
    let v = value(bytes)?;
    if v.tag() != tag::BIT_STRING {
        return Err(Error::BadExtension);
    }
    let bits = shape(v.bit_string())?;
    // At least one bit set (RFC 5280 section 4.2.1.3), and no trailing zero byte, which DER
    // drops from a named bit list (X.690 section 11.2.2).
    match bits.bytes {
        [a] if *a != 0 => Ok(KeyUsage(u16::from(*a) << 8)),
        [a, b] if *b != 0 => Ok(KeyUsage((u16::from(*a) << 8) | u16::from(*b))),
        _ => Err(Error::BadExtension),
    }
}

fn ext_key_usage(bytes: &[u8]) -> Result<ExtKeyUsage<'_>, Error> {
    let v = value(bytes)?;
    if v.tag() != tag::SEQUENCE {
        return Err(Error::BadExtension);
    }
    let mut r = v.reader();
    let mut purposes = Vec::new();
    while !r.is_empty() {
        purposes.push(shape(r.expect(tag::OID))?.oid()?);
    }
    if purposes.is_empty() {
        return Err(Error::BadExtension);
    }
    Ok(ExtKeyUsage { purposes })
}

fn subject_alt_name(bytes: &[u8]) -> Result<Vec<GeneralName<'_>>, Error> {
    let v = value(bytes)?;
    if v.tag() != tag::SEQUENCE {
        return Err(Error::BadExtension);
    }
    let mut r = v.reader();
    let mut names = Vec::new();
    while !r.is_empty() {
        names.push(general_name(&r.read()?, Place::AltName)?);
    }
    if names.is_empty() {
        return Err(Error::BadExtension);
    }
    Ok(names)
}

/// Reads the value of nameConstraints: at least one of the two lists, each of at least one
/// subtree, each subtree with no minimum but 0 and no maximum (RFC 5280 section 4.2.1.10).
pub(super) fn read_name_constraints(bytes: &[u8]) -> Result<NameConstraints<'_>, Error> {
    let v = value(bytes)?;
    if v.tag() != tag::SEQUENCE {
        return Err(Error::BadExtension);
    }
    let mut r = v.reader();
    let permitted = shape(r.optional(tag::context_constructed(0)))?
        .map(|s| subtrees(&s))
        .transpose()?;
    let excluded = shape(r.optional(tag::context_constructed(1)))?
        .map(|s| subtrees(&s))
        .transpose()?;
    shape(r.finish())?;
    if permitted.is_none() && excluded.is_none() {
        return Err(Error::BadExtension);
    }
    Ok(NameConstraints {
        permitted,
        excluded,
    })
}

fn subtrees<'a>(list: &Value<'a>) -> Result<Vec<GeneralName<'a>>, Error> {
    let mut r = list.reader();
    let mut bases = Vec::new();
    while !r.is_empty() {
        let mut subtree = shape(r.sequence())?;
        let base = general_name(&subtree.read()?, Place::Constraint)?;
        if let Some(min) = shape(subtree.optional(tag::context(0)))?
            && min.content() != [0x00]
        {
            return Err(Error::BadExtension);
        }
        if shape(subtree.optional(tag::context(1)))?.is_some() {
            return Err(Error::BadExtension);
        }
        shape(subtree.finish())?;
        bases.push(base);
    }
    if bases.is_empty() {
        return Err(Error::BadExtension);
    }
    Ok(bases)
}

fn subject_key_id(bytes: &[u8]) -> Result<&[u8], Error> {
    let v = value(bytes)?;
    if v.tag() != tag::OCTET_STRING {
        return Err(Error::BadExtension);
    }
    Ok(v.content())
}

/// authorityKeyIdentifier as read: the keyIdentifier, and whether authorityCertIssuer and
/// authorityCertSerialNumber are present (neither is interpreted).
struct AuthorityKeyId<'a> {
    key_id: Option<&'a [u8]>,
    cert_issuer: bool,
    cert_serial: bool,
}

fn authority_key_id(bytes: &[u8]) -> Result<AuthorityKeyId<'_>, Error> {
    let v = value(bytes)?;
    if v.tag() != tag::SEQUENCE {
        return Err(Error::BadExtension);
    }
    let mut r = v.reader();
    let key_id = shape(r.optional(tag::context(0)))?.map(|k| k.content());
    let cert_issuer = shape(r.optional(tag::context_constructed(1)))?.is_some();
    let cert_serial = shape(r.optional(tag::context(2)))?.is_some();
    shape(r.finish())?;
    Ok(AuthorityKeyId {
        key_id,
        cert_issuer,
        cert_serial,
    })
}
