//! A JSON reader (RFC 8259) for the tokenizer data, with the standard library only.
//!
//! A string is read into code points, not into a `String`: `\uXXXX` escapes that form a
//! surrogate pair are joined, and a lone surrogate escape is kept as the surrogate code point,
//! which the tokenizer data uses. The other strict rules of the grammar hold: no trailing comma,
//! no leading zero, no raw control character in a string, nothing after the value.

use std::fmt;

/// How deep arrays and objects may nest before the reader refuses the input.
pub const DEPTH_MAX: usize = 256;

/// A JSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number, kept as written.
    Number(String),
    /// A string, as code points.
    String(Vec<u32>),
    /// An array.
    Array(Vec<Value>),
    /// An object: its members in the order written.
    Object(Vec<(Vec<u32>, Value)>),
}

impl Value {
    /// The member `key` of an object; `None` for another value or a missing member.
    pub fn get(&self, key: &str) -> Option<&Value> {
        let Value::Object(members) = self else {
            return None;
        };
        let key: Vec<u32> = key.chars().map(u32::from).collect();
        members.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    /// The JSON type, for messages.
    pub fn kind(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "a boolean",
            Value::Number(_) => "a number",
            Value::String(_) => "a string",
            Value::Array(_) => "an array",
            Value::Object(_) => "an object",
        }
    }
}

/// Why the input is not JSON, with the byte offset where the reader stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    /// The byte offset into the input.
    pub offset: usize,
    /// What was wrong there.
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON at byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for Error {}

/// Reads one JSON value that is the whole of `input`, whitespace around it allowed.
pub fn parse(input: &str) -> Result<Value, Error> {
    let mut reader = Reader {
        bytes: input.as_bytes(),
        input,
        pos: 0,
    };
    reader.whitespace();
    let value = reader.value(0)?;
    reader.whitespace();
    if reader.pos != reader.bytes.len() {
        return Err(reader.error("text after the value"));
    }
    Ok(value)
}

struct Reader<'a> {
    bytes: &'a [u8],
    input: &'a str,
    pos: usize,
}

impl Reader<'_> {
    fn error(&self, message: &str) -> Error {
        Error {
            offset: self.pos,
            message: message.to_string(),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn whitespace(&mut self) {
        while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.peek() {
            self.pos += 1;
        }
    }

    fn expect(&mut self, byte: u8, message: &str) -> Result<(), Error> {
        if self.peek() == Some(byte) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, Error> {
        match self.peek() {
            None => Err(self.error("end of input where a value was expected")),
            Some(b'{') => self.object(depth + 1),
            Some(b'[') => self.array(depth + 1),
            Some(b'"') => self.string().map(Value::String),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'n') => self.literal("null", Value::Null),
            Some(_) => Err(self.error("not the start of a value")),
        }
    }

    fn literal(&mut self, word: &str, value: Value) -> Result<Value, Error> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(value)
        } else {
            Err(self.error("not a literal"))
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.pos;
        while let Some(b'0'..=b'9') = self.peek() {
            self.pos += 1;
        }
        self.pos - start
    }

    fn number(&mut self) -> Result<Value, Error> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.pos += 1;
                if let Some(b'0'..=b'9') = self.peek() {
                    return Err(self.error("a leading zero"));
                }
            }
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return Err(self.error("no digit in a number")),
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            if self.digits() == 0 {
                return Err(self.error("no digit after the decimal point"));
            }
        }
        if let Some(b'e' | b'E') = self.peek() {
            self.pos += 1;
            if let Some(b'+' | b'-') = self.peek() {
                self.pos += 1;
            }
            if self.digits() == 0 {
                return Err(self.error("no digit in the exponent"));
            }
        }
        Ok(Value::Number(self.input[start..self.pos].to_string()))
    }

    fn hex4(&mut self) -> Result<u32, Error> {
        let Some(digits) = self.bytes.get(self.pos..self.pos + 4) else {
            return Err(self.error("fewer than four hex digits after \\u"));
        };
        let mut value = 0;
        for &b in digits {
            let digit = (b as char)
                .to_digit(16)
                .ok_or_else(|| self.error("not a hex digit after \\u"))?;
            value = value * 16 + digit;
        }
        self.pos += 4;
        Ok(value)
    }

    fn string(&mut self) -> Result<Vec<u32>, Error> {
        self.pos += 1;
        let mut out = Vec::new();
        loop {
            let Some(b) = self.peek() else {
                return Err(self.error("unterminated string"));
            };
            match b {
                b'"' => {
                    self.pos += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.pos += 1;
                    let Some(e) = self.peek() else {
                        return Err(self.error("unterminated escape"));
                    };
                    self.pos += 1;
                    let unit = match e {
                        b'"' => u32::from('"'),
                        b'\\' => u32::from('\\'),
                        b'/' => u32::from('/'),
                        b'b' => 0x08,
                        b'f' => 0x0C,
                        b'n' => 0x0A,
                        b'r' => 0x0D,
                        b't' => 0x09,
                        b'u' => self.hex4()?,
                        _ => {
                            self.pos -= 1;
                            return Err(self.error("not an escape"));
                        }
                    };
                    out.push(unit);
                    join_surrogate_pair(&mut out);
                }
                0x00..=0x1F => return Err(self.error("a raw control character in a string")),
                _ => {
                    let c = self.input[self.pos..]
                        .chars()
                        .next()
                        .ok_or_else(|| self.error("not UTF-8"))?;
                    self.pos += c.len_utf8();
                    out.push(u32::from(c));
                }
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, Error> {
        if depth > DEPTH_MAX {
            return Err(self.error("nested too deep"));
        }
        self.pos += 1;
        let mut items = Vec::new();
        self.whitespace();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.whitespace();
            items.push(self.value(depth)?);
            self.whitespace();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(self.error("neither `,` nor `]` after an array item")),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, Error> {
        if depth > DEPTH_MAX {
            return Err(self.error("nested too deep"));
        }
        self.pos += 1;
        let mut members = Vec::new();
        self.whitespace();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Object(members));
        }
        loop {
            self.whitespace();
            if self.peek() != Some(b'"') {
                return Err(self.error("an object key that is not a string"));
            }
            let key = self.string()?;
            self.whitespace();
            self.expect(b':', "no `:` after an object key")?;
            self.whitespace();
            let value = self.value(depth)?;
            members.push((key, value));
            self.whitespace();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Object(members));
                }
                _ => return Err(self.error("neither `,` nor `}` after an object member")),
            }
        }
    }
}

/// Joins the last two code points of `out` into one when they are a high and a low surrogate.
pub fn join_surrogate_pair(out: &mut Vec<u32>) {
    if let [.., high, low] = out[..]
        && (0xD800..=0xDBFF).contains(&high)
        && (0xDC00..=0xDFFF).contains(&low)
    {
        out.truncate(out.len() - 2);
        out.push(0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00));
    }
}
