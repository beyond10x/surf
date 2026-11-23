//! A DER writer (X.690) for the certificate builder, and a reader of one element for the tests
//! of this crate. Not the reader of `surf-tls`: this crate is built beside it and checks
//! nothing it reads.

/// The tags this crate writes.
pub mod tag {
    /// BOOLEAN.
    pub const BOOLEAN: u8 = 0x01;
    /// INTEGER.
    pub const INTEGER: u8 = 0x02;
    /// BIT STRING.
    pub const BIT_STRING: u8 = 0x03;
    /// OCTET STRING.
    pub const OCTET_STRING: u8 = 0x04;
    /// NULL.
    pub const NULL: u8 = 0x05;
    /// OBJECT IDENTIFIER.
    pub const OID: u8 = 0x06;
    /// UTF8String.
    pub const UTF8_STRING: u8 = 0x0c;
    /// UTCTime.
    pub const UTC_TIME: u8 = 0x17;
    /// GeneralizedTime.
    pub const GENERALIZED_TIME: u8 = 0x18;
    /// SEQUENCE, constructed.
    pub const SEQUENCE: u8 = 0x30;
    /// SET, constructed.
    pub const SET: u8 = 0x31;
}

/// One element: `tag`, its length and `content`.
pub fn tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(content.len() + 6);
    out.push(tag);
    let len = content.len();
    if len < 0x80 {
        out.push(len as u8);
    } else {
        let bytes = len.to_be_bytes();
        let skip = bytes.iter().take_while(|b| **b == 0).count();
        out.push(0x80 | (bytes.len() - skip) as u8);
        out.extend_from_slice(&bytes[skip..]);
    }
    out.extend_from_slice(content);
    out
}

/// A SEQUENCE of the elements `parts`, each already encoded.
pub fn sequence(parts: &[&[u8]]) -> Vec<u8> {
    tlv(tag::SEQUENCE, &parts.concat())
}

/// A SET of one element.
pub fn set(part: &[u8]) -> Vec<u8> {
    tlv(tag::SET, part)
}

/// A context-specific constructed element `[n]`, the explicit tagging of `inner`.
pub fn explicit(n: u8, inner: &[u8]) -> Vec<u8> {
    tlv(0xa0 | n, inner)
}

/// A context-specific primitive element `[n]` with `content`, the implicit tagging of a string.
pub fn implicit(n: u8, content: &[u8]) -> Vec<u8> {
    tlv(0x80 | n, content)
}

/// An OBJECT IDENTIFIER from its dotted form, such as `"1.2.840.113549.1.1.11"`.
pub fn oid(dotted: &str) -> Vec<u8> {
    let arcs: Vec<u128> = dotted
        .split('.')
        .map(|a| a.parse().expect("an arc of digits"))
        .collect();
    assert!(
        arcs.len() >= 2 && arcs[0] <= 2,
        "an OID of two arcs or more"
    );
    let mut content = Vec::new();
    let mut push = |mut arc: u128| {
        let mut groups = vec![(arc & 0x7f) as u8];
        arc >>= 7;
        while arc > 0 {
            groups.push(0x80 | (arc & 0x7f) as u8);
            arc >>= 7;
        }
        groups.reverse();
        content.extend(groups);
    };
    push(arcs[0] * 40 + arcs[1]);
    for arc in &arcs[2..] {
        push(*arc);
    }
    tlv(tag::OID, &content)
}

/// A non-negative INTEGER of the big-endian `magnitude`.
pub fn unsigned(magnitude: &[u8]) -> Vec<u8> {
    let skip = magnitude.iter().take_while(|b| **b == 0).count();
    let mut content = Vec::new();
    match magnitude.get(skip) {
        None => content.push(0),
        Some(first) => {
            if first & 0x80 != 0 {
                content.push(0);
            }
            content.extend_from_slice(&magnitude[skip..]);
        }
    }
    tlv(tag::INTEGER, &content)
}

/// A small INTEGER.
pub fn integer(value: u64) -> Vec<u8> {
    unsigned(&value.to_be_bytes())
}

/// BOOLEAN TRUE.
pub fn true_() -> Vec<u8> {
    tlv(tag::BOOLEAN, &[0xff])
}

/// NULL.
pub fn null() -> Vec<u8> {
    tlv(tag::NULL, &[])
}

/// An OCTET STRING.
pub fn octet_string(content: &[u8]) -> Vec<u8> {
    tlv(tag::OCTET_STRING, content)
}

/// A BIT STRING of whole bytes.
pub fn bit_string(bytes: &[u8]) -> Vec<u8> {
    let mut content = vec![0];
    content.extend_from_slice(bytes);
    tlv(tag::BIT_STRING, &content)
}

/// A BIT STRING of named bits (X.690 11.2.2: trailing zero bits left out), such as keyUsage.
pub fn named_bits(bits: &[u8]) -> Vec<u8> {
    let highest = bits.iter().copied().max().map_or(0, |b| b as usize + 1);
    let mut bytes = vec![0u8; highest.div_ceil(8)];
    for bit in bits {
        bytes[*bit as usize / 8] |= 0x80 >> (bit % 8);
    }
    let unused = (bytes.len() * 8 - highest) as u8;
    let mut content = vec![unused];
    content.extend(bytes);
    tlv(tag::BIT_STRING, &content)
}

/// A UTF8String.
pub fn utf8(text: &str) -> Vec<u8> {
    tlv(tag::UTF8_STRING, text.as_bytes())
}

/// One element read from the front of `input`: its tag, its content, the whole element and the
/// bytes after it. Only definite lengths of the shortest form, as DER has them.
#[derive(Clone, Copy, Debug)]
pub struct Element<'a> {
    /// The tag (one byte; this crate meets no other).
    pub tag: u8,
    /// The content.
    pub content: &'a [u8],
    /// Tag, length and content.
    pub whole: &'a [u8],
    /// What follows the element.
    pub rest: &'a [u8],
}

/// Reads the element at the front of `input`.
pub fn read(input: &[u8]) -> Result<Element<'_>, String> {
    let (&tag, after) = input.split_first().ok_or("no element")?;
    if tag & 0x1f == 0x1f {
        return Err(format!("tag {tag:#04x} of more than one byte"));
    }
    let (&first, after) = after.split_first().ok_or("no length")?;
    let (len, after) = if first < 0x80 {
        (first as usize, after)
    } else {
        let n = (first & 0x7f) as usize;
        if n == 0 || n > 4 || after.len() < n {
            return Err(format!("length of {n} bytes"));
        }
        let len = after[..n]
            .iter()
            .fold(0usize, |acc, b| acc << 8 | *b as usize);
        if after[0] == 0 || len < 0x80 {
            return Err("a length not in its shortest form".into());
        }
        (len, &after[n..])
    };
    if after.len() < len {
        return Err(format!("{len} bytes declared, {} left", after.len()));
    }
    let header = input.len() - after.len();
    Ok(Element {
        tag,
        content: &after[..len],
        whole: &input[..header + len],
        rest: &after[len..],
    })
}

/// Reads the element at the front of `input` and requires `tag`.
pub fn expect(input: &[u8], tag: u8) -> Result<Element<'_>, String> {
    let element = read(input)?;
    if element.tag != tag {
        return Err(format!(
            "tag {:#04x} where {tag:#04x} was expected",
            element.tag
        ));
    }
    Ok(element)
}

/// The elements of `content`, in order.
pub fn elements(mut content: &[u8]) -> Result<Vec<Element<'_>>, String> {
    let mut out = Vec::new();
    while !content.is_empty() {
        let element = read(content)?;
        content = element.rest;
        out.push(element);
    }
    Ok(out)
}
