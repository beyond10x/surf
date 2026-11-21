//! URL Standard, "Hosts (domains and IP addresses)": the host parser, the IPv4 and IPv6
//! parsers, the opaque-host parser and the host serializer.
//!
//! Domain to ASCII (IDNA, UTS #46) is not written. The standard's domain parser returns an ASCII
//! domain lower-cased without running it (step 5, "due to web compatibility"), so only a domain
//! that holds a code point outside ASCII after percent-decoding needs it; that domain is
//! [`ParseError::NonAsciiHost`].

use crate::percent::{self, EncodeSet};
use crate::{ParseError, ValidationError};

/// A host: a domain, an IP address, an opaque host or the empty host.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Host {
    /// A domain, ASCII and lower-cased (only special URLs have one).
    Domain(String),
    /// An IPv4 address, as its 32-bit value.
    Ipv4(u32),
    /// An IPv6 address, as its eight 16-bit pieces.
    Ipv6([u16; 8]),
    /// An opaque host (a URL that is not special), percent-encoded and ASCII.
    Opaque(String),
    /// The empty host.
    Empty,
}

impl Host {
    /// The host serializer.
    pub fn serialize(&self) -> String {
        match self {
            Host::Domain(text) | Host::Opaque(text) => text.clone(),
            Host::Ipv4(address) => serialize_ipv4(*address),
            Host::Ipv6(pieces) => format!("[{}]", serialize_ipv6(pieces)),
            Host::Empty => String::new(),
        }
    }
}

/// The forbidden host code points.
fn is_forbidden_host(c: char) -> bool {
    matches!(
        c,
        '\0' | '\t'
            | '\n'
            | '\r'
            | ' '
            | '#'
            | '/'
            | ':'
            | '<'
            | '>'
            | '?'
            | '@'
            | '['
            | '\\'
            | ']'
            | '^'
            | '|'
    )
}

/// The forbidden domain code points: the forbidden host code points, C0 controls, `%` and
/// U+007F.
fn is_forbidden_domain(c: char) -> bool {
    is_forbidden_host(c) || c <= '\u{1F}' || c == '%' || c == '\u{7F}'
}

/// The host parser, over a non-empty `input` (`is_opaque` when the URL is not special).
pub(crate) fn parse_host(input: &str, is_opaque: bool) -> Result<Host, ParseError> {
    if let Some(inner) = input.strip_prefix('[') {
        let Some(inner) = inner.strip_suffix(']') else {
            return Err(ParseError::Invalid(ValidationError::Ipv6Unclosed));
        };
        return parse_ipv6(inner).map(Host::Ipv6);
    }
    if is_opaque {
        return parse_opaque_host(input);
    }
    let decoded = percent::percent_decode(input.as_bytes());
    let domain = String::from_utf8_lossy(&decoded);
    let ascii_domain = parse_domain(&domain)?;
    if ends_in_a_number(&ascii_domain) {
        return parse_ipv4(&ascii_domain).map(Host::Ipv4);
    }
    Ok(Host::Domain(ascii_domain))
}

/// The domain parser with beStrict false, for what this crate does: an ASCII domain is
/// lower-cased; any other domain needs domain to ASCII and is [`ParseError::NonAsciiHost`].
fn parse_domain(domain: &str) -> Result<String, ParseError> {
    if !domain.is_ascii() {
        return Err(ParseError::NonAsciiHost);
    }
    let result = domain.to_ascii_lowercase();
    if result.is_empty() {
        return Err(ParseError::Invalid(ValidationError::DomainInvalidCodePoint));
    }
    if result.chars().any(is_forbidden_domain) {
        return Err(ParseError::Invalid(ValidationError::DomainInvalidCodePoint));
    }
    Ok(result)
}

/// The ends-in-a-number checker.
fn ends_in_a_number(input: &str) -> bool {
    let mut parts: Vec<&str> = input.split('.').collect();
    if parts.last() == Some(&"") && parts.len() > 1 {
        parts.pop();
    }
    let Some(last) = parts.last() else {
        return false;
    };
    if !last.is_empty() && last.bytes().all(|b| b.is_ascii_digit()) {
        return true;
    }
    parse_ipv4_number(last).is_some()
}

/// The IPv4 parser.
fn parse_ipv4(input: &str) -> Result<u32, ParseError> {
    let mut parts: Vec<&str> = input.split('.').collect();
    if parts.last() == Some(&"") && parts.len() > 1 {
        parts.pop();
    }
    if parts.len() > 4 {
        return Err(ParseError::Invalid(ValidationError::Ipv4TooManyParts));
    }
    let mut numbers = Vec::with_capacity(4);
    for part in parts {
        let Some(number) = parse_ipv4_number(part) else {
            return Err(ParseError::Invalid(ValidationError::Ipv4NonNumericPart));
        };
        numbers.push(number);
    }
    let Some((&last, rest)) = numbers.split_last() else {
        return Err(ParseError::Invalid(ValidationError::Ipv4NonNumericPart));
    };
    if rest.iter().any(|&n| n > 255) {
        return Err(ParseError::Invalid(ValidationError::Ipv4OutOfRangePart));
    }
    // 256 ^ (5 - size), for a size of 1 to 4.
    let limit: u64 = 1 << (8 * (5 - numbers.len()));
    if last >= limit {
        return Err(ParseError::Invalid(ValidationError::Ipv4OutOfRangePart));
    }
    let mut ipv4 = last;
    for (counter, &n) in rest.iter().enumerate() {
        ipv4 += n << (8 * (3 - counter));
    }
    u32::try_from(ipv4).map_err(|_| ParseError::Invalid(ValidationError::Ipv4OutOfRangePart))
}

/// The IPv4 number parser, without its validation-error flag. A value above `u64::MAX` is
/// returned as `u64::MAX`: every use compares it against a limit far below.
fn parse_ipv4_number(input: &str) -> Option<u64> {
    if input.is_empty() {
        return None;
    }
    let bytes = input.as_bytes();
    let (digits, radix) = if bytes.len() >= 2 && (bytes[..2] == *b"0x" || bytes[..2] == *b"0X") {
        (&bytes[2..], 16)
    } else if bytes.len() >= 2 && bytes[0] == b'0' {
        (&bytes[1..], 8)
    } else {
        (bytes, 10)
    };
    let mut value: u64 = 0;
    for &byte in digits {
        let digit = u64::from(percent::hex_value(byte)?);
        if digit >= radix {
            return None;
        }
        value = value.saturating_mul(radix).saturating_add(digit);
    }
    Some(value)
}

/// The IPv6 parser.
fn parse_ipv6(input: &str) -> Result<[u16; 8], ParseError> {
    use ValidationError as V;
    let fail = |error| Err(ParseError::Invalid(error));
    let chars: Vec<char> = input.chars().collect();
    let c = |pointer: usize| chars.get(pointer).copied();
    let mut address = [0u16; 8];
    let mut piece_index = 0usize;
    let mut compress: Option<usize> = None;
    let mut pointer = 0usize;

    if c(pointer) == Some(':') {
        if c(pointer + 1) != Some(':') {
            return fail(V::Ipv6InvalidCompression);
        }
        pointer += 2;
        piece_index += 1;
        compress = Some(piece_index);
    }
    while c(pointer).is_some() {
        if piece_index == 8 {
            return fail(V::Ipv6TooManyPieces);
        }
        if c(pointer) == Some(':') {
            if compress.is_some() {
                return fail(V::Ipv6MultipleCompression);
            }
            pointer += 1;
            piece_index += 1;
            compress = Some(piece_index);
            continue;
        }
        let mut value: u32 = 0;
        let mut length = 0usize;
        while length < 4 {
            let Some(digit) = c(pointer).and_then(|ch| ch.to_digit(16)) else {
                break;
            };
            value = value * 0x10 + digit;
            pointer += 1;
            length += 1;
        }
        if c(pointer) == Some('.') {
            if length == 0 {
                return fail(V::Ipv4InIpv6InvalidCodePoint);
            }
            pointer -= length;
            if piece_index > 6 {
                return fail(V::Ipv4InIpv6TooManyPieces);
            }
            let mut numbers_seen = 0;
            while c(pointer).is_some() {
                let mut ipv4_piece: Option<u32> = None;
                if numbers_seen > 0 {
                    if c(pointer) == Some('.') && numbers_seen < 4 {
                        pointer += 1;
                    } else {
                        return fail(V::Ipv4InIpv6InvalidCodePoint);
                    }
                }
                if !c(pointer).is_some_and(|ch| ch.is_ascii_digit()) {
                    return fail(V::Ipv4InIpv6InvalidCodePoint);
                }
                while let Some(number) = c(pointer).and_then(|ch| ch.to_digit(10)) {
                    match ipv4_piece {
                        None => ipv4_piece = Some(number),
                        Some(0) => return fail(V::Ipv4InIpv6InvalidCodePoint),
                        Some(piece) => ipv4_piece = Some(piece * 10 + number),
                    }
                    if ipv4_piece.is_some_and(|piece| piece > 255) {
                        return fail(V::Ipv4InIpv6OutOfRangePart);
                    }
                    pointer += 1;
                }
                let piece = ipv4_piece.unwrap_or(0);
                let slot = &mut address[piece_index];
                *slot = (u32::from(*slot) * 0x100 + piece) as u16;
                numbers_seen += 1;
                if numbers_seen == 2 || numbers_seen == 4 {
                    piece_index += 1;
                }
            }
            if numbers_seen != 4 {
                return fail(V::Ipv4InIpv6TooFewParts);
            }
            break;
        } else if c(pointer) == Some(':') {
            pointer += 1;
            if c(pointer).is_none() {
                return fail(V::Ipv6InvalidCodePoint);
            }
        } else if c(pointer).is_some() {
            return fail(V::Ipv6InvalidCodePoint);
        }
        address[piece_index] = value as u16;
        piece_index += 1;
    }
    if let Some(compress) = compress {
        let mut swaps = piece_index - compress;
        piece_index = 7;
        while piece_index != 0 && swaps > 0 {
            address.swap(piece_index, compress + swaps - 1);
            piece_index -= 1;
            swaps -= 1;
        }
    } else if piece_index != 8 {
        return fail(V::Ipv6TooFewPieces);
    }
    Ok(address)
}

/// The opaque-host parser.
fn parse_opaque_host(input: &str) -> Result<Host, ParseError> {
    if input.chars().any(is_forbidden_host) {
        return Err(ParseError::Invalid(ValidationError::HostInvalidCodePoint));
    }
    let mut output = String::with_capacity(input.len());
    percent::utf8_percent_encode_into(&mut output, input, EncodeSet::C0Control);
    Ok(Host::Opaque(output))
}

/// The IPv4 serializer.
fn serialize_ipv4(address: u32) -> String {
    let [a, b, c, d] = address.to_be_bytes();
    format!("{a}.{b}.{c}.{d}")
}

/// The IPv6 serializer, without the brackets.
fn serialize_ipv6(address: &[u16; 8]) -> String {
    let compress = compressed_piece_index(address);
    let mut output = String::new();
    let mut ignore0 = false;
    for (piece_index, &piece) in address.iter().enumerate() {
        if ignore0 && piece == 0 {
            continue;
        }
        ignore0 = false;
        if compress == Some(piece_index) {
            output.push_str(if piece_index == 0 { "::" } else { ":" });
            ignore0 = true;
            continue;
        }
        output.push_str(&format!("{piece:x}"));
        if piece_index != 7 {
            output.push(':');
        }
    }
    output
}

/// Find the IPv6 address compressed piece index: the first longest run of two or more zero
/// pieces.
fn compressed_piece_index(address: &[u16; 8]) -> Option<usize> {
    let mut longest_index = None;
    let mut longest_size = 1;
    let mut found_index = None;
    let mut found_size = 0;
    for (piece_index, &piece) in address.iter().enumerate() {
        if piece != 0 {
            if found_size > longest_size {
                longest_index = found_index;
                longest_size = found_size;
            }
            found_index = None;
            found_size = 0;
        } else {
            if found_index.is_none() {
                found_index = Some(piece_index);
            }
            found_size += 1;
        }
    }
    if found_size > longest_size {
        return found_index;
    }
    longest_index
}
