//! URL Standard, "Percent-encoded bytes": the percent-encode sets, UTF-8 percent-encode and
//! percent-decode. Only UTF-8 is an output encoding here.

/// A percent-encode set. Every set of the standard holds every code point above U+007E, so a
/// set is told by the ASCII code points it holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum EncodeSet {
    /// C0 controls and every code point above U+007E.
    C0Control,
    /// C0 control set and space, `"`, `<`, `>`, `` ` ``.
    Fragment,
    /// C0 control set and space, `"`, `#`, `<`, `>`.
    Query,
    /// Query set and `'`.
    SpecialQuery,
    /// Query set and `?`, `^`, `` ` ``, `{`, `}`.
    Path,
    /// Path set and `/`, `:`, `;`, `=`, `@`, `[` to `]`, `|`.
    Userinfo,
    /// The component set (userinfo set and `$` to `&`, `+`, `,`) and `!`, `'` to `)`, `~`:
    /// everything but ASCII alphanumerics, `*`, `-`, `.` and `_`. The component set itself is
    /// not used here.
    FormUrlencoded,
}

impl EncodeSet {
    /// Whether the set holds `byte`, read as the code point of the same value.
    pub(crate) fn contains(self, byte: u8) -> bool {
        if !(0x20..=0x7E).contains(&byte) {
            return true;
        }
        let c = byte;
        match self {
            EncodeSet::C0Control => false,
            EncodeSet::Fragment => matches!(c, b' ' | b'"' | b'<' | b'>' | b'`'),
            EncodeSet::Query => matches!(c, b' ' | b'"' | b'#' | b'<' | b'>'),
            EncodeSet::SpecialQuery => EncodeSet::Query.contains(c) || c == b'\'',
            EncodeSet::Path => {
                EncodeSet::Query.contains(c) || matches!(c, b'?' | b'^' | b'`' | b'{' | b'}')
            }
            EncodeSet::Userinfo => {
                EncodeSet::Path.contains(c)
                    || matches!(c, b'/' | b':' | b';' | b'=' | b'@' | b'['..=b']' | b'|')
            }
            EncodeSet::FormUrlencoded => {
                !(c.is_ascii_alphanumeric() || matches!(c, b'*' | b'-' | b'.' | b'_'))
            }
        }
    }
}

const UPPER_HEX: &[u8; 16] = b"0123456789ABCDEF";

/// Percent-encode a byte: `%` and two upper-case hex digits.
pub(crate) fn push_percent_encoded(out: &mut String, byte: u8) {
    out.push('%');
    out.push(char::from(UPPER_HEX[usize::from(byte >> 4)]));
    out.push(char::from(UPPER_HEX[usize::from(byte & 0x0F)]));
}

/// "Percent-encode after encoding" with UTF-8, appended to `out`. With the form-urlencoded set,
/// a space becomes `+` (spaceAsPlus).
pub(crate) fn utf8_percent_encode_into(out: &mut String, input: &str, set: EncodeSet) {
    let space_as_plus = set == EncodeSet::FormUrlencoded;
    for &byte in input.as_bytes() {
        if space_as_plus && byte == b' ' {
            out.push('+');
        } else if set.contains(byte) {
            push_percent_encoded(out, byte);
        } else {
            out.push(char::from(byte));
        }
    }
}

/// UTF-8 percent-encode one code point.
pub(crate) fn utf8_percent_encode_char(out: &mut String, c: char, set: EncodeSet) {
    let mut buffer = [0u8; 4];
    utf8_percent_encode_into(out, c.encode_utf8(&mut buffer), set);
}

/// Percent-decode a byte sequence: `%` and two hex digits become the byte they name; anything
/// else stays.
pub(crate) fn percent_decode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len());
    let mut i = 0;
    while let Some(&byte) = input.get(i) {
        if byte == b'%' {
            let high = input.get(i + 1).copied().and_then(hex_value);
            let low = input.get(i + 2).copied().and_then(hex_value);
            if let (Some(high), Some(low)) = (high, low) {
                output.push(high << 4 | low);
                i += 3;
                continue;
            }
        }
        output.push(byte);
        i += 1;
    }
    output
}

pub(crate) fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
