//! Runs every case of the web-platform-tests URL data (`tests/data/urltestdata.json`, its source
//! in `tests/data/SOURCE`) the way `url/url-constructor.any.js` and `url/url-origin.any.js` of
//! web-platform-tests do: the base is parsed first and its failure is the case's failure, then
//! the input against it; every attribute the case names is compared.
//!
//! A case is skipped only when parsing its input or its base ends in
//! [`ParseError::NonAsciiHost`] (domain to ASCII, IDNA, is not written), and the test checks
//! that its input or base then holds a code point outside ASCII or a percent-encoded byte
//! outside ASCII. The skipped cases are listed in `tests/data/SKIPPED`, which must match.
//!
//! The JSON reader is written here: no third-party crate, also not for a test.

use std::io::Write;

use surf_url::{form_urlencoded, Host, ParseError, Url};

const DATA: &str = include_str!("data/urltestdata.json");
const SOURCE: &str = include_str!("data/SOURCE");
const SKIPPED: &str = include_str!("data/SKIPPED");

/// The share of all cases that may be skipped, in percent (the story's acceptance).
const SKIPPED_PERCENT_MAX: usize = 15;

#[test]
fn urltestdata() {
    let data = Json::parse(DATA).expect("urltestdata.json is JSON");
    let Json::Array(entries) = data else {
        panic!("urltestdata.json is not an array");
    };
    let cases: Vec<Case> = entries.iter().filter_map(Case::from_json).collect();
    let total = cases.len();

    let mut run = 0usize;
    let mut passed = 0usize;
    let mut skipped_lines = Vec::new();
    let mut failures = Vec::new();
    for case in &cases {
        match case.run() {
            Outcome::Passed => {
                run += 1;
                passed += 1;
            }
            Outcome::Failed(why) => {
                run += 1;
                failures.push(format!("{}: {why}", case.name()));
            }
            Outcome::Skipped => {
                if !holds_non_ascii(&case.input)
                    && !case.base.as_deref().is_some_and(holds_non_ascii)
                {
                    failures.push(format!(
                        "{}: skipped as a host outside ASCII, but neither input nor base holds a \
                         code point or a percent-encoded byte outside ASCII",
                        case.name()
                    ));
                }
                skipped_lines.push(case.skipped_line());
            }
        }
    }
    let skipped = skipped_lines.len();

    let mut out = std::io::stdout().lock();
    let _ = writeln!(
        out,
        "urltestdata: {total} cases, {run} run, {passed} passed, {skipped} skipped"
    );
    drop(out);

    let expected_total = source_case_count();
    assert_eq!(
        total, expected_total,
        "urltestdata.json holds {total} cases, SOURCE says {expected_total}"
    );
    assert!(
        failures.is_empty(),
        "{} of {run} cases that ran failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(passed, run);
    assert!(
        skipped * 100 <= total * SKIPPED_PERCENT_MAX,
        "{skipped} of {total} cases skipped, more than {SKIPPED_PERCENT_MAX} percent"
    );
    let listed: Vec<&str> = SKIPPED
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    assert!(
        listed == skipped_lines,
        "tests/data/SKIPPED does not list the skipped cases; it should hold, after its comment \
         lines:\n{}",
        skipped_lines.join("\n")
    );
}

/// The `cases: <n>` line of `SOURCE`.
fn source_case_count() -> usize {
    SOURCE
        .lines()
        .find_map(|line| line.strip_prefix("cases: "))
        .and_then(|n| n.trim().parse().ok())
        .expect("SOURCE has a line `cases: <n>`")
}

/// Whether `text` holds a code point outside ASCII, or a percent-encoded byte (`%` and two hex
/// digits) above 0x7F. Either is what can make a host's domain hold a code point outside ASCII.
fn holds_non_ascii(text: &str) -> bool {
    if !text.is_ascii() {
        return true;
    }
    let bytes = text.as_bytes();
    bytes.windows(3).any(|w| {
        w[0] == b'%'
            && w[1].is_ascii_hexdigit()
            && w[2].is_ascii_hexdigit()
            && matches!(w[1], b'8'..=b'9' | b'a'..=b'f' | b'A'..=b'F')
    })
}

struct Case {
    input: String,
    base: Option<String>,
    failure: bool,
    /// The expected attributes, by their name in the data, in the data's order.
    expected: Vec<(String, String)>,
}

enum Outcome {
    Passed,
    Failed(String),
    Skipped,
}

impl Case {
    fn from_json(entry: &Json) -> Option<Case> {
        let Json::Object(members) = entry else {
            return None;
        };
        let get = |name: &str| members.iter().find(|(k, _)| k == name).map(|(_, v)| v);
        let Some(Json::String(input)) = get("input") else {
            panic!("an object without an input string: {members:?}");
        };
        let base = match get("base") {
            Some(Json::String(base)) => Some(base.clone()),
            Some(Json::Null) | None => None,
            Some(other) => panic!("base is not a string or null: {other:?}"),
        };
        let failure = matches!(get("failure"), Some(Json::Bool(true)));
        let expected = members
            .iter()
            .filter(|(k, _)| ATTRIBUTES.contains(&k.as_str()))
            .map(|(k, v)| match v {
                Json::String(s) => (k.clone(), s.clone()),
                other => panic!("{k} is not a string: {other:?}"),
            })
            .collect();
        Some(Case {
            input: input.clone(),
            base,
            failure,
            expected,
        })
    }

    fn name(&self) -> String {
        match &self.base {
            Some(base) => format!("<{}> against <{base}>", self.input),
            None => format!("<{}> without base", self.input),
        }
    }

    fn skipped_line(&self) -> String {
        match &self.base {
            Some(base) => format!("{}\t{}", json_string(&self.input), json_string(base)),
            None => format!("{}\tnull", json_string(&self.input)),
        }
    }

    fn run(&self) -> Outcome {
        let parsed = match &self.base {
            Some(base) => {
                Url::parse(base, None).and_then(|base| Url::parse(&self.input, Some(&base)))
            }
            None => Url::parse(&self.input, None),
        };
        let url = match parsed {
            Err(ParseError::NonAsciiHost) => return Outcome::Skipped,
            Err(_) if self.failure => return Outcome::Passed,
            Err(error) => return Outcome::Failed(format!("failed to parse: {error}")),
            Ok(url) if self.failure => {
                return Outcome::Failed(format!("expected failure, parsed as <{}>", url.href()))
            }
            Ok(url) => url,
        };
        let mut wrong = Vec::new();
        for (name, expected) in &self.expected {
            let actual = attribute(&url, name);
            if &actual != expected {
                wrong.push(format!("{name} is {actual:?}, expected {expected:?}"));
            }
        }
        if wrong.is_empty() {
            Outcome::Passed
        } else {
            Outcome::Failed(wrong.join("; "))
        }
    }
}

const ATTRIBUTES: [&str; 12] = [
    "href",
    "origin",
    "protocol",
    "username",
    "password",
    "host",
    "hostname",
    "port",
    "pathname",
    "search",
    "searchParams",
    "hash",
];

/// The value the `URL` interface's getter of that name returns (URL Standard, "URL class"),
/// computed from the record. `searchParams` is the `URLSearchParams` serialization of the query:
/// parsed by [`urlencoded_parse`] below, serialized by the crate.
fn attribute(url: &Url, name: &str) -> String {
    match name {
        "href" => url.href(),
        "origin" => url.origin().serialize(),
        "protocol" => format!("{}:", url.scheme()),
        "username" => url.username().to_string(),
        "password" => url.password().to_string(),
        "host" => match (url.host(), url.port()) {
            (None, _) => String::new(),
            (Some(host), None) => host.serialize(),
            (Some(host), Some(port)) => format!("{}:{port}", host.serialize()),
        },
        "hostname" => url.host().map(Host::serialize).unwrap_or_default(),
        "port" => url.port().map(|p| p.to_string()).unwrap_or_default(),
        "pathname" => url.serialize_path(),
        "search" => match url.query() {
            None | Some("") => String::new(),
            Some(query) => format!("?{query}"),
        },
        "searchParams" => form_urlencoded::serialize(&urlencoded_parse(url.query().unwrap_or(""))),
        "hash" => match url.fragment() {
            None | Some("") => String::new(),
            Some(fragment) => format!("#{fragment}"),
        },
        other => panic!("no attribute {other}"),
    }
}

/// URL Standard, "application/x-www-form-urlencoded parsing", over the UTF-8 bytes of `input`.
/// The crate does not hold this parser (out of its scope); the test needs it to compare
/// `searchParams`.
fn urlencoded_parse(input: &str) -> Vec<(String, String)> {
    let mut output = Vec::new();
    for bytes in input.as_bytes().split(|&b| b == b'&') {
        if bytes.is_empty() {
            continue;
        }
        let (name, value) = match bytes.iter().position(|&b| b == b'=') {
            Some(i) => (&bytes[..i], &bytes[i + 1..]),
            None => (bytes, &[][..]),
        };
        let decode = |part: &[u8]| {
            let plus: Vec<u8> = part
                .iter()
                .map(|&b| if b == b'+' { b' ' } else { b })
                .collect();
            String::from_utf8_lossy(&percent_decode(&plus)).into_owned()
        };
        output.push((decode(name), decode(value)));
    }
    output
}

fn percent_decode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        let byte = input[i];
        if byte == b'%' && i + 2 < input.len() {
            if let (Some(h), Some(l)) = (hex(input[i + 1]), hex(input[i + 2])) {
                output.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        output.push(byte);
        i += 1;
    }
    output
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// `text` as a JSON string, for the lines of `SKIPPED`.
fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A JSON value (RFC 8259). Numbers are kept as their text: the data holds none.
#[derive(Debug)]
enum Json {
    Null,
    Bool(bool),
    Number(#[allow(dead_code)] String),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    fn parse(text: &str) -> Result<Json, String> {
        let mut reader = Reader {
            chars: text.chars().collect(),
            at: 0,
        };
        reader.space();
        let value = reader.value()?;
        reader.space();
        if reader.at != reader.chars.len() {
            return Err(format!("text after the value at {}", reader.at));
        }
        Ok(value)
    }
}

struct Reader {
    chars: Vec<char>,
    at: usize,
}

impl Reader {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn next(&mut self) -> Result<char, String> {
        let c = self.peek().ok_or("unexpected end of the text")?;
        self.at += 1;
        Ok(c)
    }

    fn expect(&mut self, want: char) -> Result<(), String> {
        let got = self.next()?;
        if got == want {
            Ok(())
        } else {
            Err(format!(
                "expected {want:?} at {}, found {got:?}",
                self.at - 1
            ))
        }
    }

    fn space(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.at += 1;
        }
    }

    fn word(&mut self, word: &str, value: Json) -> Result<Json, String> {
        for want in word.chars() {
            self.expect(want)?;
        }
        Ok(value)
    }

    fn value(&mut self) -> Result<Json, String> {
        match self.peek() {
            Some('n') => self.word("null", Json::Null),
            Some('t') => self.word("true", Json::Bool(true)),
            Some('f') => self.word("false", Json::Bool(false)),
            Some('"') => Ok(Json::String(self.string()?)),
            Some('[') => self.array(),
            Some('{') => self.object(),
            Some(c) if c == '-' || c.is_ascii_digit() => {
                let start = self.at;
                while matches!(self.peek(), Some('0'..='9' | '-' | '+' | '.' | 'e' | 'E')) {
                    self.at += 1;
                }
                Ok(Json::Number(self.chars[start..self.at].iter().collect()))
            }
            other => Err(format!("unexpected {other:?} at {}", self.at)),
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.space();
        if self.peek() == Some(']') {
            self.at += 1;
            return Ok(Json::Array(items));
        }
        loop {
            self.space();
            items.push(self.value()?);
            self.space();
            match self.next()? {
                ',' => continue,
                ']' => return Ok(Json::Array(items)),
                other => return Err(format!("expected , or ] at {}, found {other:?}", self.at)),
            }
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.expect('{')?;
        let mut members = Vec::new();
        self.space();
        if self.peek() == Some('}') {
            self.at += 1;
            return Ok(Json::Object(members));
        }
        loop {
            self.space();
            let key = self.string()?;
            self.space();
            self.expect(':')?;
            self.space();
            members.push((key, self.value()?));
            self.space();
            match self.next()? {
                ',' => continue,
                '}' => return Ok(Json::Object(members)),
                other => return Err(format!("expected , or }} at {}, found {other:?}", self.at)),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let mut value = 0u32;
        for _ in 0..4 {
            let c = self.next()?;
            value = value * 16
                + c.to_digit(16)
                    .ok_or(format!("bad \\u escape at {}", self.at))?;
        }
        Ok(value)
    }

    /// A string. A lone surrogate escape becomes U+FFFD, as the `URL` constructor's conversion
    /// to a scalar value string does.
    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.next()? {
                '"' => return Ok(out),
                '\\' => match self.next()? {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'u' => {
                        let unit = self.hex4()?;
                        let scalar = if (0xD800..0xDC00).contains(&unit)
                            && self.chars.get(self.at) == Some(&'\\')
                            && self.chars.get(self.at + 1) == Some(&'u')
                        {
                            let save = self.at;
                            self.at += 2;
                            let low = self.hex4()?;
                            if (0xDC00..0xE000).contains(&low) {
                                0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00)
                            } else {
                                self.at = save;
                                0xFFFD
                            }
                        } else {
                            unit
                        };
                        out.push(char::from_u32(scalar).unwrap_or('\u{FFFD}'));
                    }
                    other => return Err(format!("bad escape \\{other} at {}", self.at)),
                },
                c => out.push(c),
            }
        }
    }
}
