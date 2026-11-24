//! The lexical grammar of Web IDL (section 2.11 of the Web IDL Standard): integer, decimal,
//! identifier, string, whitespace, comment and other, the longest match first. Whitespace and
//! comments are dropped; `...` is one token.

use crate::ReadError;

/// The kind of a token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// `-?([1-9][0-9]*|0[Xx][0-9A-Fa-f]+|0[0-7]*)`.
    Integer,
    /// A number with a fraction or an exponent.
    Decimal,
    /// `[_-]?[A-Za-z][0-9A-Z_a-z-]*`, keywords included.
    Identifier,
    /// `"[^"]*"`, quotes included.
    String,
    /// One character that starts no other token, or `...`.
    Other,
}

/// A token and the line it starts on, counted from 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Token {
    pub(crate) kind: Kind,
    pub(crate) text: String,
    pub(crate) line: usize,
}

/// The tokens of `text`, or the line of a comment or string that does not end.
pub(crate) fn lex(text: &str, file: &str) -> Result<Vec<Token>, ReadError> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut line = 1;
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes[at];
        if matches!(byte, b'\t' | b'\n' | b'\r' | b' ') {
            if byte == b'\n' {
                line += 1;
            }
            at += 1;
            continue;
        }
        if bytes[at..].starts_with(b"//") {
            while at < bytes.len() && bytes[at] != b'\n' {
                at += 1;
            }
            continue;
        }
        if bytes[at..].starts_with(b"/*") {
            let start = line;
            let Some(end) = text[at + 2..].find("*/") else {
                return Err(ReadError::new(file, start, "a comment that does not end"));
            };
            line += text[at..at + 2 + end].matches('\n').count();
            at += end + 4;
            continue;
        }
        if byte == b'"' {
            let Some(end) = text[at + 1..].find('"') else {
                return Err(ReadError::new(file, line, "a string that does not end"));
            };
            let token = &text[at..at + end + 2];
            tokens.push(Token {
                kind: Kind::String,
                text: token.to_string(),
                line,
            });
            line += token.matches('\n').count();
            at += end + 2;
            continue;
        }
        let integer = integer_length(&bytes[at..]);
        let decimal = decimal_length(&bytes[at..]);
        let identifier = identifier_length(&bytes[at..]);
        let longest = integer.max(decimal).max(identifier);
        let (kind, length) = if longest == 0 {
            if bytes[at..].starts_with(b"...") {
                (Kind::Other, 3)
            } else {
                let width = text[at..].chars().next().map_or(1, char::len_utf8);
                (Kind::Other, width)
            }
        } else if decimal == longest {
            (Kind::Decimal, decimal)
        } else if integer == longest {
            (Kind::Integer, integer)
        } else {
            (Kind::Identifier, identifier)
        };
        tokens.push(Token {
            kind,
            text: text[at..at + length].to_string(),
            line,
        });
        at += length;
    }
    Ok(tokens)
}

fn digits(bytes: &[u8], from: usize, accept: fn(u8) -> bool) -> usize {
    let mut at = from;
    while at < bytes.len() && accept(bytes[at]) {
        at += 1;
    }
    at
}

/// The length of the integer token at the start of `bytes`, 0 for none.
fn integer_length(bytes: &[u8]) -> usize {
    let sign = usize::from(bytes.first() == Some(&b'-'));
    match bytes.get(sign) {
        Some(b'1'..=b'9') => digits(bytes, sign + 1, |b| b.is_ascii_digit()),
        Some(b'0') => {
            let hex = matches!(bytes.get(sign + 1), Some(b'x' | b'X'));
            if hex && bytes.get(sign + 2).is_some_and(u8::is_ascii_hexdigit) {
                digits(bytes, sign + 2, |b| b.is_ascii_hexdigit())
            } else {
                digits(bytes, sign + 1, |b| matches!(b, b'0'..=b'7'))
            }
        }
        _ => 0,
    }
}

/// The length of the decimal token at the start of `bytes`, 0 for none.
fn decimal_length(bytes: &[u8]) -> usize {
    let sign = usize::from(bytes.first() == Some(&b'-'));
    let whole = digits(bytes, sign, |b| b.is_ascii_digit());
    let mut end = 0;
    if bytes.get(whole) == Some(&b'.') {
        let fraction = digits(bytes, whole + 1, |b| b.is_ascii_digit());
        if whole > sign || fraction > whole + 1 {
            end = fraction;
        }
    } else if whole > sign {
        end = whole;
    }
    if end == 0 {
        return 0;
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let mut exponent = end + 1;
        if matches!(bytes.get(exponent), Some(b'+' | b'-')) {
            exponent += 1;
        }
        let after = digits(bytes, exponent, |b| b.is_ascii_digit());
        if after > exponent {
            return after;
        }
    }
    // Digits alone are an integer, not a decimal.
    if end == whole { 0 } else { end }
}

/// The length of the identifier token at the start of `bytes`, 0 for none.
fn identifier_length(bytes: &[u8]) -> usize {
    let lead = usize::from(matches!(bytes.first(), Some(b'_' | b'-')));
    if !bytes.get(lead).is_some_and(u8::is_ascii_alphabetic) {
        return 0;
    }
    digits(bytes, lead + 1, |b| {
        b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
    })
}
