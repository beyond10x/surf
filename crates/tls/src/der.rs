//! A DER reader (ITU-T X.690, the distinguished encoding) over a byte slice, for the
//! certificates a server sends (docs/design/tls.md, item 6; `story:tls-x509`).
//!
//! The input comes from any server on the open web, so the reader never reads past its slice,
//! never allocates by a declared length (a length is compared with the bytes present before
//! anything else is done with it), never panics, and descends at most [`DEPTH_MAX`] values deep.
//! What it takes: a tag of one byte (tag numbers up to 30); a definite length in the shortest
//! form, in at most [`LENGTH_BYTES_MAX`] bytes; INTEGER in the shortest form; BOOLEAN as 0x00 or
//! 0xFF; OBJECT IDENTIFIER with each arc in the shortest form; BIT STRING with its unused bits,
//! which are zero; OCTET STRING; NULL; UTCTime and GeneralizedTime in the forms RFC 5280 section
//! 4.1.2.5 allows (`Z`, with seconds, no fraction); UTF8String, PrintableString and IA5String.

use std::fmt;

/// How deep values may nest: the outermost value is at depth 1, a value inside it at depth 2.
/// A value deeper than this is [`Error::TooDeep`].
pub const DEPTH_MAX: usize = 32;

/// The most bytes the long form of a length may have. More is [`Error::LengthTooLong`].
pub const LENGTH_BYTES_MAX: usize = 4;

/// The tags of the universal types the reader knows, and of SEQUENCE and SET.
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
    pub const UTF8_STRING: u8 = 0x0C;
    /// PrintableString.
    pub const PRINTABLE_STRING: u8 = 0x13;
    /// IA5String.
    pub const IA5_STRING: u8 = 0x16;
    /// UTCTime.
    pub const UTC_TIME: u8 = 0x17;
    /// GeneralizedTime.
    pub const GENERALIZED_TIME: u8 = 0x18;
    /// SEQUENCE and SEQUENCE OF.
    pub const SEQUENCE: u8 = 0x30;
    /// SET and SET OF.
    pub const SET: u8 = 0x31;

    /// The tag `[n]` of the context-specific class, primitive (an IMPLICIT tag of a primitive
    /// type).
    pub const fn context(n: u8) -> u8 {
        0x80 | n
    }

    /// The tag `[n]` of the context-specific class, constructed (an EXPLICIT tag, or an
    /// IMPLICIT tag of a constructed type).
    pub const fn context_constructed(n: u8) -> u8 {
        0xA0 | n
    }
}

/// Why the reader refused its input. Each kind has the name [`Error::name`] gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Error {
    /// A value declares more bytes than are present, or the input ends inside a header.
    Truncated,
    /// The length 0x80: the indefinite form, which DER forbids.
    IndefiniteLength,
    /// A length in the long form that fits a shorter form.
    NonMinimalLength,
    /// A length of more than [`LENGTH_BYTES_MAX`] bytes.
    LengthTooLong,
    /// A tag number over 30 (the multi-byte form), or a universal type in a form DER forbids.
    UnsupportedTag,
    /// Bytes after the value that should be the whole input.
    TrailingData,
    /// A value deeper than [`DEPTH_MAX`].
    TooDeep,
    /// A value of another tag than the structure has there.
    UnexpectedTag {
        /// The tag the structure has there.
        expected: u8,
        /// The tag found.
        got: u8,
    },
    /// The structure has a value where its input ended.
    Missing {
        /// The tag the structure has there.
        expected: u8,
    },
    /// An INTEGER with a leading byte that a shorter form would leave out.
    NonMinimalInteger,
    /// An INTEGER with no content.
    EmptyInteger,
    /// A BOOLEAN that is not one byte of 0x00 or 0xFF.
    BadBoolean,
    /// A NULL with content.
    BadNull,
    /// An OBJECT IDENTIFIER that is empty, ends inside an arc, or has an arc not in the
    /// shortest form.
    BadOid,
    /// A BIT STRING with no unused-bits byte, more than 7 unused bits, or unused bits that are
    /// not zero.
    BadBitString,
    /// A time not in the form RFC 5280 section 4.1.2.5 allows, or a date that does not exist.
    BadTime,
    /// A string with a character its type does not allow.
    BadString,
}

impl Error {
    /// The name of the kind, as the variant is written.
    pub fn name(self) -> &'static str {
        match self {
            Error::Truncated => "Truncated",
            Error::IndefiniteLength => "IndefiniteLength",
            Error::NonMinimalLength => "NonMinimalLength",
            Error::LengthTooLong => "LengthTooLong",
            Error::UnsupportedTag => "UnsupportedTag",
            Error::TrailingData => "TrailingData",
            Error::TooDeep => "TooDeep",
            Error::UnexpectedTag { .. } => "UnexpectedTag",
            Error::Missing { .. } => "Missing",
            Error::NonMinimalInteger => "NonMinimalInteger",
            Error::EmptyInteger => "EmptyInteger",
            Error::BadBoolean => "BadBoolean",
            Error::BadNull => "BadNull",
            Error::BadOid => "BadOid",
            Error::BadBitString => "BadBitString",
            Error::BadTime => "BadTime",
            Error::BadString => "BadString",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnexpectedTag { expected, got } => {
                write!(
                    f,
                    "UnexpectedTag: 0x{got:02x} where 0x{expected:02x} belongs"
                )
            }
            Error::Missing { expected } => write!(f, "Missing: no value 0x{expected:02x}"),
            other => f.write_str(other.name()),
        }
    }
}

impl std::error::Error for Error {}

/// The header of a value: its tag, the bytes the tag and length take, and the length it
/// declares, which [`header`] does not compare with the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    /// The tag byte.
    pub tag: u8,
    /// The bytes of the tag and the length.
    pub header_len: usize,
    /// The length the header declares.
    pub length: u64,
}

/// Reads the tag and length at the start of `input`, in DER's form, without looking at the
/// content.
pub fn header(input: &[u8]) -> Result<Header, Error> {
    let (&tag, rest) = input.split_first().ok_or(Error::Truncated)?;
    if tag & 0x1F == 0x1F {
        return Err(Error::UnsupportedTag);
    }
    let (&first, rest) = rest.split_first().ok_or(Error::Truncated)?;
    if first < 0x80 {
        return Ok(Header {
            tag,
            header_len: 2,
            length: u64::from(first),
        });
    }
    if first == 0x80 {
        return Err(Error::IndefiniteLength);
    }
    let count = usize::from(first & 0x7F);
    if count > LENGTH_BYTES_MAX {
        return Err(Error::LengthTooLong);
    }
    let bytes = rest.get(..count).ok_or(Error::Truncated)?;
    if bytes[0] == 0 {
        return Err(Error::NonMinimalLength);
    }
    let length = bytes.iter().fold(0u64, |n, &b| (n << 8) | u64::from(b));
    if length < 0x80 {
        return Err(Error::NonMinimalLength);
    }
    Ok(Header {
        tag,
        header_len: 2 + count,
        length,
    })
}

/// One value: its tag, its content and the bytes of the whole value, borrowed from the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Value<'a> {
    tag: u8,
    content: &'a [u8],
    raw: &'a [u8],
    depth: usize,
}

/// Reads the values of a slice one after another: the whole input, or the content of a
/// constructed value.
#[derive(Clone, Copy, Debug)]
pub struct Reader<'a> {
    input: &'a [u8],
    depth: usize,
}

impl<'a> Reader<'a> {
    /// A reader of `input`, whose values are at depth 1.
    pub fn new(input: &'a [u8]) -> Self {
        Reader { input, depth: 1 }
    }

    /// Whether every byte was read.
    pub fn is_empty(&self) -> bool {
        self.input.is_empty()
    }

    /// The tag of the next value, if there is one.
    pub fn peek_tag(&self) -> Option<u8> {
        self.input.first().copied()
    }

    /// The next value, whatever its tag.
    pub fn read(&mut self) -> Result<Value<'a>, Error> {
        if self.depth > DEPTH_MAX {
            return Err(Error::TooDeep);
        }
        let h = header(self.input)?;
        let present = (self.input.len() - h.header_len) as u64;
        if h.length > present {
            return Err(Error::Truncated);
        }
        // The length fits in what is present, so it fits in a usize.
        let end = h.header_len + h.length as usize;
        let raw = &self.input[..end];
        let value = Value {
            tag: h.tag,
            content: &raw[h.header_len..],
            raw,
            depth: self.depth,
        };
        self.input = &self.input[end..];
        Ok(value)
    }

    /// The next value, which must have the tag `expected`.
    pub fn expect(&mut self, expected: u8) -> Result<Value<'a>, Error> {
        match self.peek_tag() {
            None => Err(Error::Missing { expected }),
            Some(got) if got != expected => Err(Error::UnexpectedTag { expected, got }),
            Some(_) => self.read(),
        }
    }

    /// The next value if it has the tag `wanted`; else nothing is read.
    pub fn optional(&mut self, wanted: u8) -> Result<Option<Value<'a>>, Error> {
        if self.peek_tag() == Some(wanted) {
            self.read().map(Some)
        } else {
            Ok(None)
        }
    }

    /// Enters the next value, which must be a SEQUENCE: a reader of its content.
    pub fn sequence(&mut self) -> Result<Reader<'a>, Error> {
        Ok(self.expect(tag::SEQUENCE)?.reader())
    }

    /// Ends the reader: [`Error::TrailingData`] if a byte is left.
    pub fn finish(self) -> Result<(), Error> {
        if self.input.is_empty() {
            Ok(())
        } else {
            Err(Error::TrailingData)
        }
    }
}

impl<'a> Value<'a> {
    /// The tag byte.
    pub fn tag(&self) -> u8 {
        self.tag
    }

    /// The content, without tag and length.
    pub fn content(&self) -> &'a [u8] {
        self.content
    }

    /// The whole value: tag, length and content.
    pub fn raw(&self) -> &'a [u8] {
        self.raw
    }

    /// Whether the tag has the constructed bit.
    pub fn is_constructed(&self) -> bool {
        self.tag & 0x20 != 0
    }

    /// A reader of the content, whose values are one level deeper than this one.
    pub fn reader(&self) -> Reader<'a> {
        Reader {
            input: self.content,
            depth: self.depth + 1,
        }
    }

    /// The content of an INTEGER in the shortest form, two's complement, big-endian.
    pub fn integer(&self) -> Result<&'a [u8], Error> {
        integer(self.content)
    }

    /// The INTEGER as an unsigned number of at most 32 bits; `None` for a negative or a larger
    /// one.
    pub fn small_unsigned(&self) -> Result<Option<u32>, Error> {
        let bytes = self.integer()?;
        if bytes[0] & 0x80 != 0 {
            return Ok(None);
        }
        let bytes = if bytes[0] == 0 { &bytes[1..] } else { bytes };
        if bytes.len() > 4 {
            return Ok(None);
        }
        Ok(Some(
            bytes.iter().fold(0u32, |n, &b| (n << 8) | u32::from(b)),
        ))
    }

    /// The BOOLEAN.
    pub fn boolean(&self) -> Result<bool, Error> {
        match self.content {
            [0x00] => Ok(false),
            [0xFF] => Ok(true),
            _ => Err(Error::BadBoolean),
        }
    }

    /// The content of an OBJECT IDENTIFIER, checked.
    pub fn oid(&self) -> Result<&'a [u8], Error> {
        oid(self.content)
    }

    /// The BIT STRING.
    pub fn bit_string(&self) -> Result<BitString<'a>, Error> {
        bit_string(self.content)
    }

    /// The time of a UTCTime or a GeneralizedTime.
    pub fn time(&self) -> Result<Time, Error> {
        match self.tag {
            tag::UTC_TIME => utc_time(self.content),
            tag::GENERALIZED_TIME => generalized_time(self.content),
            got => Err(Error::UnexpectedTag {
                expected: tag::UTC_TIME,
                got,
            }),
        }
    }

    /// The text of a UTF8String, a PrintableString or an IA5String.
    pub fn string(&self) -> Result<&'a str, Error> {
        let text = std::str::from_utf8(self.content).map_err(|_| Error::BadString)?;
        let allowed = match self.tag {
            tag::UTF8_STRING => true,
            tag::PRINTABLE_STRING => text.bytes().all(printable),
            tag::IA5_STRING => text.is_ascii(),
            got => {
                return Err(Error::UnexpectedTag {
                    expected: tag::UTF8_STRING,
                    got,
                });
            }
        };
        if allowed {
            Ok(text)
        } else {
            Err(Error::BadString)
        }
    }
}

fn printable(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b" '()+,-./:=?".contains(&b)
}

/// A BIT STRING: the count of unused bits in its last byte, and its bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitString<'a> {
    /// The unused bits at the end of the last byte, 0 to 7.
    pub unused_bits: u8,
    /// The bytes, the last one's unused bits zero.
    pub bytes: &'a [u8],
}

fn integer(content: &[u8]) -> Result<&[u8], Error> {
    match content {
        [] => Err(Error::EmptyInteger),
        [0x00, next, ..] if next & 0x80 == 0 => Err(Error::NonMinimalInteger),
        [0xFF, next, ..] if next & 0x80 != 0 => Err(Error::NonMinimalInteger),
        _ => Ok(content),
    }
}

fn oid(content: &[u8]) -> Result<&[u8], Error> {
    let last = content.last().ok_or(Error::BadOid)?;
    if last & 0x80 != 0 {
        return Err(Error::BadOid);
    }
    let mut start_of_arc = true;
    for &b in content {
        if start_of_arc && b == 0x80 {
            return Err(Error::BadOid);
        }
        start_of_arc = b & 0x80 == 0;
    }
    Ok(content)
}

fn bit_string(content: &[u8]) -> Result<BitString<'_>, Error> {
    let (&unused_bits, bytes) = content.split_first().ok_or(Error::BadBitString)?;
    if unused_bits > 7 {
        return Err(Error::BadBitString);
    }
    match bytes.last() {
        None if unused_bits != 0 => Err(Error::BadBitString),
        Some(&last) if last & ((1u8 << unused_bits) - 1) != 0 => Err(Error::BadBitString),
        _ => Ok(BitString { unused_bits, bytes }),
    }
}

/// The dotted form of an OBJECT IDENTIFIER's content (`2.5.29.17`), for messages. An arc that
/// fits 128 bits is written in decimal, a longer one as `0x` and its bits in hex, so that the
/// work grows with the length of the content and not with its square.
pub fn oid_to_string(content: &[u8]) -> String {
    oid_to_string_counting(content).0
}

/// [`oid_to_string`] and the count of the steps it took (each byte looked at, each hex digit
/// made), for the test of its bound.
#[doc(hidden)]
pub fn oid_to_string_counting(content: &[u8]) -> (String, usize) {
    let mut work = 0usize;
    let mut out = String::new();
    let mut start = 0;
    for (i, &b) in content.iter().enumerate() {
        work += 1;
        if b & 0x80 != 0 {
            continue;
        }
        let arc = &content[start..=i];
        let first = start == 0;
        start = i + 1;
        if !first {
            out.push('.');
        }
        match (arc_value(arc, &mut work), first) {
            (Some(v), false) => out.push_str(&v.to_string()),
            // The first subidentifier is 40 × the first arc (0, 1 or 2) + the second arc.
            (Some(v), true) if v < 40 => out.push_str(&format!("0.{v}")),
            (Some(v), true) if v < 80 => out.push_str(&format!("1.{}", v - 40)),
            (Some(v), true) => out.push_str(&format!("2.{}", v - 80)),
            (None, first) => {
                let mut digits = arc_hex(arc, &mut work);
                if first {
                    out.push_str("2.");
                    subtract_hex(&mut digits, 0x50, &mut work);
                }
                write_big(&mut out, &digits);
            }
        }
    }
    (out, work)
}

/// The value of one arc, if it fits 128 bits.
fn arc_value(arc: &[u8], work: &mut usize) -> Option<u128> {
    let mut v: u128 = 0;
    for &b in arc {
        *work += 1;
        if v >> 121 != 0 {
            return None;
        }
        v = (v << 7) | u128::from(b & 0x7F);
    }
    Some(v)
}

/// The hex digits (0 to 15) of one arc, most significant first, in one pass over its bytes.
fn arc_hex(arc: &[u8], work: &mut usize) -> Vec<u8> {
    let bits = arc.len() * 7;
    // Zero bits in front, so that the bits fill whole hex digits.
    let mut held = ((4 - bits % 4) % 4) as u32;
    let mut acc: u32 = 0;
    let mut digits = Vec::with_capacity(bits.div_ceil(4));
    for &b in arc {
        *work += 1;
        acc = (acc << 7) | u32::from(b & 0x7F);
        held += 7;
        while held >= 4 {
            held -= 4;
            digits.push(((acc >> held) & 0xF) as u8);
            *work += 1;
        }
        acc &= (1 << held) - 1;
    }
    digits
}

/// Subtracts `n` (under 16 × 16) from the hex digits of a number at least 2^128.
fn subtract_hex(digits: &mut [u8], n: u32, work: &mut usize) {
    let mut borrow = n;
    for d in digits.iter_mut().rev() {
        *work += 1;
        if borrow == 0 {
            break;
        }
        let low = borrow & 0xF;
        borrow >>= 4;
        if u32::from(*d) >= low {
            *d -= low as u8;
        } else {
            *d = (u32::from(*d) + 16 - low) as u8;
            borrow += 1;
        }
    }
}

/// Writes a number given by its hex digits: in decimal if it fits 128 bits, else as `0x` and
/// its digits.
fn write_big(out: &mut String, digits: &[u8]) {
    let skip = digits.iter().take_while(|&&d| d == 0).count();
    let digits = &digits[skip..];
    if digits.len() <= 32 {
        let v = digits.iter().fold(0u128, |v, &d| (v << 4) | u128::from(d));
        out.push_str(&v.to_string());
    } else {
        out.push_str("0x");
        out.extend(
            digits
                .iter()
                .map(|&d| char::from_digit(u32::from(d), 16).unwrap_or('?')),
        );
    }
}

/// Reads the one value that is the whole of `input` and checks every value inside it, down to
/// [`DEPTH_MAX`]: the lengths, the tags, and the content of every BOOLEAN, INTEGER, NULL,
/// OBJECT IDENTIFIER and BIT STRING of the universal class. A byte after the value is
/// [`Error::TrailingData`].
pub fn parse(input: &[u8]) -> Result<Value<'_>, Error> {
    let mut reader = Reader::new(input);
    let value = reader.read()?;
    check(&value)?;
    reader.finish()?;
    Ok(value)
}

/// Checks one value and, if it is constructed, every value inside it. Recursion is bounded:
/// a reader below [`DEPTH_MAX`] refuses to read.
fn check(value: &Value<'_>) -> Result<(), Error> {
    if value.tag & 0xC0 == 0 {
        match value.tag {
            tag::SEQUENCE | tag::SET => {}
            t if t & 0x20 != 0 => return Err(Error::UnsupportedTag),
            0x10 | 0x11 => return Err(Error::UnsupportedTag),
            tag::BOOLEAN => {
                value.boolean()?;
            }
            tag::INTEGER => {
                value.integer()?;
            }
            tag::NULL if !value.content.is_empty() => return Err(Error::BadNull),
            tag::OID => {
                value.oid()?;
            }
            tag::BIT_STRING => {
                value.bit_string()?;
            }
            _ => {}
        }
    }
    if value.is_constructed() {
        let mut inner = value.reader();
        while !inner.is_empty() {
            check(&inner.read()?)?;
        }
    }
    Ok(())
}

/// A point in time, in whole seconds since 1970-01-01T00:00:00Z, as a certificate states it
/// and as a caller passes the time to check against (docs/design/tls.md, item 13).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time(i64);

impl Time {
    /// The time `seconds` after 1970-01-01T00:00:00Z.
    pub const fn from_unix(seconds: i64) -> Self {
        Time(seconds)
    }

    /// The seconds since 1970-01-01T00:00:00Z.
    pub const fn unix(self) -> i64 {
        self.0
    }

    /// The time of a date and a time of day in UTC; `None` for one that does not exist (a
    /// February 30, an hour 24, a second 60, a year over 9999).
    pub fn from_utc(
        year: u32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> Option<Self> {
        if year > 9999
            || !(1..=12).contains(&month)
            || day == 0
            || day > days_in_month(year, month)
            || hour > 23
            || minute > 59
            || second > 59
        {
            return None;
        }
        let days = days_from_civil(i64::from(year), i64::from(month), i64::from(day));
        Some(Time(
            days * 86_400 + i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second),
        ))
    }

    /// The date and time of day in UTC: year, month, day, hour, minute, second.
    pub fn to_utc(self) -> (i64, u32, u32, u32, u32, u32) {
        let days = self.0.div_euclid(86_400);
        let secs = self.0.rem_euclid(86_400);
        let (y, m, d) = civil_from_days(days);
        (
            y,
            m,
            d,
            (secs / 3_600) as u32,
            (secs % 3_600 / 60) as u32,
            (secs % 60) as u32,
        )
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (y, mo, d, h, mi, s) = self.to_utc();
        write!(f, "{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
    }
}

fn leap(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Days since 1970-01-01 of a proleptic Gregorian date (H. Hinnant's `days_from_civil`).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// The number written by the ASCII digits `digits`; `None` if one is not a digit.
fn digits(digits: &[u8]) -> Option<u32> {
    digits.iter().try_fold(0u32, |n, &b| {
        b.is_ascii_digit().then(|| n * 10 + u32::from(b - b'0'))
    })
}

/// UTCTime as RFC 5280 section 4.1.2.5.1 allows it: `YYMMDDHHMMSSZ`; YY of 50 and over is
/// 19YY, under 50 is 20YY.
fn utc_time(content: &[u8]) -> Result<Time, Error> {
    let [body @ .., b'Z'] = content else {
        return Err(Error::BadTime);
    };
    if body.len() != 12 {
        return Err(Error::BadTime);
    }
    let yy = digits(&body[0..2]).ok_or(Error::BadTime)?;
    let year = if yy >= 50 { 1900 + yy } else { 2000 + yy };
    date_time(year, &body[2..])
}

/// GeneralizedTime as RFC 5280 section 4.1.2.5.2 allows it: `YYYYMMDDHHMMSSZ`.
fn generalized_time(content: &[u8]) -> Result<Time, Error> {
    let [body @ .., b'Z'] = content else {
        return Err(Error::BadTime);
    };
    if body.len() != 14 {
        return Err(Error::BadTime);
    }
    let year = digits(&body[0..4]).ok_or(Error::BadTime)?;
    date_time(year, &body[4..])
}

/// `MMDDHHMMSS` after the year.
fn date_time(year: u32, rest: &[u8]) -> Result<Time, Error> {
    let field = |i: usize| digits(&rest[i..i + 2]).ok_or(Error::BadTime);
    Time::from_utc(year, field(0)?, field(2)?, field(4)?, field(6)?, field(8)?)
        .ok_or(Error::BadTime)
}
