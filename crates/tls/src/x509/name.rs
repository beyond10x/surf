//! Names: the issuer and subject as DER, and the GeneralName of subjectAltName and
//! nameConstraints (RFC 5280 sections 4.1.2.4, 4.2.1.6 and 4.2.1.10).

use super::Error;
use crate::der::{Reader, Value, tag};

/// A distinguished name as DER (the RDNSequence, tag and length included). Two names are equal
/// when their bytes are; nothing is normalised and no attribute is interpreted, the Common Name
/// included.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Name<'a> {
    der: &'a [u8],
}

impl<'a> Name<'a> {
    pub(crate) fn new(der: &'a [u8]) -> Self {
        Name { der }
    }

    /// The DER of the name.
    pub fn der(&self) -> &'a [u8] {
        self.der
    }
}

/// One GeneralName.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneralName<'a> {
    /// A dNSName: bytes from `!` to `~` only; in a subjectAltName also not empty.
    Dns(&'a str),
    /// An iPAddress: in a subjectAltName 4 or 16 bytes; in nameConstraints the address and its
    /// mask, 8 or 32 bytes.
    Ip(&'a [u8]),
    /// Any other kind (otherName, rfc822Name, x400Address, directoryName, ediPartyName,
    /// uniformResourceIdentifier, registeredID): its tag and content, not interpreted.
    Other {
        /// The context-specific tag, 0x80 to 0x88 or 0xA0 to 0xA8.
        tag: u8,
        /// The content.
        content: &'a [u8],
    },
}

/// Where a GeneralName stands, which decides the lengths its kinds may have.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Place {
    /// In subjectAltName.
    AltName,
    /// As the base of a subtree of nameConstraints.
    Constraint,
}

/// Reads one GeneralName.
pub(super) fn general_name<'a>(value: &Value<'a>, place: Place) -> Result<GeneralName<'a>, Error> {
    let content = value.content();
    match value.tag() {
        // dNSName [2] IMPLICIT IA5String.
        0x82 => {
            if place == Place::AltName && content.is_empty() {
                return Err(Error::BadName);
            }
            if !content.iter().all(|&b| (0x21..=0x7E).contains(&b)) {
                return Err(Error::BadName);
            }
            let text = std::str::from_utf8(content).map_err(|_| Error::BadName)?;
            Ok(GeneralName::Dns(text))
        }
        // iPAddress [7] IMPLICIT OCTET STRING.
        0x87 => {
            let allowed = match place {
                Place::AltName => [4, 16],
                Place::Constraint => [8, 32],
            };
            if allowed.contains(&content.len()) {
                Ok(GeneralName::Ip(content))
            } else {
                Err(Error::BadName)
            }
        }
        // rfc822Name, uniformResourceIdentifier, registeredID: primitive.
        t @ (0x81 | 0x86 | 0x88) => Ok(GeneralName::Other { tag: t, content }),
        // otherName, x400Address, directoryName, ediPartyName: constructed.
        t @ (0xA0 | 0xA3 | 0xA4 | 0xA5) => {
            if t == 0xA4 {
                // directoryName [4] EXPLICIT Name.
                let mut inner = value.reader();
                let name = inner.expect(tag::SEQUENCE)?;
                inner.finish()?;
                check_name(&name)?;
            }
            Ok(GeneralName::Other { tag: t, content })
        }
        _ => Err(Error::BadName),
    }
}

/// Checks that `name` is a Name: a SEQUENCE of SETs, each holding at least one SEQUENCE of an
/// OBJECT IDENTIFIER and one value.
pub(super) fn check_name(name: &Value<'_>) -> Result<(), Error> {
    if name.tag() != tag::SEQUENCE {
        return Err(Error::Der(crate::der::Error::UnexpectedTag {
            expected: tag::SEQUENCE,
            got: name.tag(),
        }));
    }
    let mut rdns = name.reader();
    while !rdns.is_empty() {
        let mut rdn = rdns.expect(tag::SET)?.reader();
        if rdn.is_empty() {
            return Err(Error::BadName);
        }
        while !rdn.is_empty() {
            let mut atv: Reader<'_> = rdn.sequence()?;
            atv.expect(tag::OID)?.oid()?;
            atv.read()?;
            atv.finish()?;
        }
    }
    Ok(())
}
