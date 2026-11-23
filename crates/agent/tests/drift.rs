//! Cases 1 and 2: the parser of `surf-agent` against `.out/cli/binding.json`, which `task cli`
//! writes from `cli/surf-agent.yaml`. The binding is read with a JSON reader written here.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use surf_agent::cli::{self, Source};

/// What the binding declares and the parser refuses: the markers `UNMAPPED:` (b) and (c) of
/// `cli/surf-agent.yaml`.
const BINDING_ONLY: &[&str] = &[
    "back token protected hidden_tty --token-prompt",
    "back token protected stdin --token-stdin",
    "close token protected hidden_tty --token-prompt",
    "close token protected stdin --token-stdin",
    "fill token protected hidden_tty --token-prompt",
    "fill token protected stdin --token-stdin",
    "follow token protected hidden_tty --token-prompt",
    "follow token protected stdin --token-stdin",
    "forward token protected hidden_tty --token-prompt",
    "forward token protected stdin --token-stdin",
    "global state --session-dir",
    "go token protected hidden_tty --token-prompt",
    "go token protected stdin --token-stdin",
    "look token protected hidden_tty --token-prompt",
    "look token protected stdin --token-stdin",
    "pick token protected hidden_tty --token-prompt",
    "pick token protected stdin --token-stdin",
    "press token protected hidden_tty --token-prompt",
    "press token protected stdin --token-stdin",
];

/// What the parser reads and the binding cannot declare: the marker `UNMAPPED:` (a), the
/// environment sources `SURF_SESSION` and `SURF_TOKEN`.
const PARSER_ONLY: &[&str] = &[
    "back session_id environment SURF_SESSION",
    "back token environment SURF_TOKEN",
    "close session_id environment SURF_SESSION",
    "close token environment SURF_TOKEN",
    "fill session_id environment SURF_SESSION",
    "fill token environment SURF_TOKEN",
    "follow session_id environment SURF_SESSION",
    "follow token environment SURF_TOKEN",
    "forward session_id environment SURF_SESSION",
    "forward token environment SURF_TOKEN",
    "go session_id environment SURF_SESSION",
    "go token environment SURF_TOKEN",
    "look session_id environment SURF_SESSION",
    "look token environment SURF_TOKEN",
    "pick session_id environment SURF_SESSION",
    "pick token environment SURF_TOKEN",
    "press session_id environment SURF_SESSION",
    "press token environment SURF_TOKEN",
];

#[test]
fn drift() {
    let binding = read_binding();
    let (accepted, refused) = parser_reading();
    compare(&binding_reading(&binding), &accepted, &refused)
        .unwrap_or_else(|difference| panic!("the parser drifted from the binding:\n{difference}"));
}

#[test]
fn planted_drift() {
    let mut binding = read_binding();
    let (accepted, refused) = parser_reading();
    assert!(compare(&binding_reading(&binding), &accepted, &refused).is_ok());
    let commands = match &mut binding {
        Json::Object(fields) => fields.get_mut("commands").expect("commands"),
        _ => panic!("the binding is not an object"),
    };
    let Json::Array(commands) = commands else {
        panic!("commands is not an array")
    };
    let mut renamed = 0;
    for command in commands.iter_mut() {
        let Json::Object(fields) = command else {
            panic!("a command is not an object")
        };
        if let Some(Json::Array(path)) = fields.get_mut("path")
            && path == &vec![Json::String("look".into())]
        {
            path[0] = Json::String("peek".into());
            renamed += 1;
        }
    }
    assert_eq!(renamed, 1, "the binding has one command look");
    let difference = compare(&binding_reading(&binding), &accepted, &refused)
        .expect_err("a renamed command must fail the comparison");
    assert!(difference.contains("peek"), "{difference}");
    assert!(difference.contains("look"), "{difference}");
}

fn read_binding() -> Json {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.out/cli/binding.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}; run `slot task cli` first", path.display()));
    parse_json(&text).expect("binding.json is JSON")
}

/// Every command, positional argument, option, protected source and global of the binding, one
/// line each.
fn binding_reading(binding: &Json) -> BTreeSet<String> {
    let mut lines = BTreeSet::new();
    if let Some(Json::Object(globals)) = binding.get("globals") {
        for (role, long) in globals {
            lines.insert(format!("global {role} --{}", long.as_str()));
        }
    }
    let callables = binding.get("callables").expect("callables");
    for command in binding.get("commands").expect("commands").as_array() {
        let path: Vec<&str> = command
            .get("path")
            .expect("path")
            .as_array()
            .iter()
            .map(Json::as_str)
            .collect();
        assert_eq!(path.len(), 1, "a command path of one word");
        let name = path[0];
        lines.insert(format!("command {name}"));
        let callable = command.get("callable").expect("callable").as_str();
        let input = callables
            .get(callable)
            .and_then(|c| c.get("input"))
            .expect("input");
        for argument in command.get("arguments").expect("arguments").as_array() {
            let field = argument.get("field").expect("field").as_str();
            let source = argument.get("source").expect("source");
            match source.get("kind").expect("kind").as_str() {
                "positional" => {
                    let index = source.get("index").expect("index").as_number();
                    let kind = input
                        .get("shape")
                        .and_then(|s| s.get("fields"))
                        .and_then(|f| f.get(field))
                        .and_then(|f| f.get("kind"))
                        .expect("the positional's field in the input shape")
                        .as_str();
                    let required = if kind == "optional" {
                        "optional"
                    } else {
                        "required"
                    };
                    lines.insert(format!("{name} {field} positional {index} {required}"));
                }
                "option" => {
                    let long = source.get("long").expect("long").as_str();
                    lines.insert(format!("{name} {field} option --{long}"));
                }
                "protected" => {
                    for flag in ["file", "stdin", "hidden_tty"] {
                        if let Some(long) = source.get(flag) {
                            lines.insert(format!(
                                "{name} {field} protected {flag} --{}",
                                long.as_str()
                            ));
                        }
                    }
                }
                other => panic!("a source kind the test does not know: {other}"),
            }
        }
    }
    lines
}

/// The parser's table in the same lines: those it accepts, and those it declares refused.
fn parser_reading() -> (BTreeSet<String>, BTreeSet<String>) {
    let mut accepted = BTreeSet::new();
    let mut refused = BTreeSet::new();
    for global in cli::GLOBALS {
        let line = format!("global {} --{}", global.role, global.long);
        if global.accepted {
            accepted.insert(line)
        } else {
            refused.insert(line)
        };
    }
    for command in cli::COMMANDS {
        accepted.insert(format!("command {}", command.name));
        for argument in command.arguments {
            for source in argument.sources {
                let what = match source {
                    Source::Positional { index, required } => format!(
                        "positional {index} {}",
                        if *required { "required" } else { "optional" }
                    ),
                    Source::Option { long } => format!("option --{long}"),
                    Source::TokenFile { long } => format!("protected file --{long}"),
                    Source::TokenStdin { long } => format!("protected stdin --{long}"),
                    Source::TokenPrompt { long } => format!("protected hidden_tty --{long}"),
                    Source::Environment { name } => format!("environment {name}"),
                };
                let line = format!("{} {} {what}", command.name, argument.field);
                if source.accepted() {
                    accepted.insert(line)
                } else {
                    refused.insert(line)
                };
            }
        }
    }
    (accepted, refused)
}

/// Both ways: the binding less what the parser accepts is exactly [`BINDING_ONLY`], which the
/// parser refuses and nothing else; what the parser accepts less the binding is exactly
/// [`PARSER_ONLY`].
fn compare(
    binding: &BTreeSet<String>,
    accepted: &BTreeSet<String>,
    refused: &BTreeSet<String>,
) -> Result<(), String> {
    let expected_binding_only: BTreeSet<String> =
        BINDING_ONLY.iter().map(|s| s.to_string()).collect();
    let expected_parser_only: BTreeSet<String> =
        PARSER_ONLY.iter().map(|s| s.to_string()).collect();
    let binding_only: BTreeSet<String> = binding.difference(accepted).cloned().collect();
    let parser_only: BTreeSet<String> = accepted.difference(binding).cloned().collect();
    let mut report = String::new();
    for line in binding_only.difference(&expected_binding_only) {
        report.push_str(&format!("in the binding, not in the parser: {line}\n"));
    }
    for line in expected_binding_only.difference(&binding_only) {
        report.push_str(&format!("a listed difference that is not one: {line}\n"));
    }
    for line in parser_only.difference(&expected_parser_only) {
        report.push_str(&format!("in the parser, not in the binding: {line}\n"));
    }
    for line in expected_parser_only.difference(&parser_only) {
        report.push_str(&format!(
            "a listed parser-only source that is missing: {line}\n"
        ));
    }
    if refused != &expected_binding_only {
        for line in refused.symmetric_difference(&expected_binding_only) {
            report.push_str(&format!(
                "refused by the parser and listed differ on: {line}\n"
            ));
        }
    }
    if report.is_empty() {
        Ok(())
    } else {
        Err(report)
    }
}

// --- A JSON reader, enough for binding.json -------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(fields) => fields.get(key),
            _ => None,
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Json::String(text) => text,
            other => panic!("not a string: {other:?}"),
        }
    }

    fn as_number(&self) -> &str {
        match self {
            Json::Number(text) => text,
            other => panic!("not a number: {other:?}"),
        }
    }

    fn as_array(&self) -> &[Json] {
        match self {
            Json::Array(items) => items,
            other => panic!("not an array: {other:?}"),
        }
    }
}

fn parse_json(text: &str) -> Result<Json, String> {
    let mut reader = JsonReader {
        bytes: text.as_bytes(),
        at: 0,
    };
    let value = reader.value()?;
    reader.space();
    if reader.at != reader.bytes.len() {
        return Err(format!("text after the value at byte {}", reader.at));
    }
    Ok(value)
}

struct JsonReader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl JsonReader<'_> {
    fn space(&mut self) {
        while matches!(self.bytes.get(self.at), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), String> {
        self.space();
        if self.bytes.get(self.at) == Some(&byte) {
            self.at += 1;
            Ok(())
        } else {
            Err(format!("expected {:?} at byte {}", byte as char, self.at))
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Result<Json, String> {
        if self.bytes[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            Ok(value)
        } else {
            Err(format!("unknown literal at byte {}", self.at))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.space();
        match self.bytes.get(self.at) {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Json::String(self.string()?)),
            Some(b't') => self.literal("true", Json::Bool(true)),
            Some(b'f') => self.literal("false", Json::Bool(false)),
            Some(b'n') => self.literal("null", Json::Null),
            Some(b'-' | b'0'..=b'9') => {
                let start = self.at;
                while matches!(
                    self.bytes.get(self.at),
                    Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                ) {
                    self.at += 1;
                }
                Ok(Json::Number(
                    String::from_utf8(self.bytes[start..self.at].to_vec()).unwrap(),
                ))
            }
            _ => Err(format!("no value at byte {}", self.at)),
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.expect(b'{')?;
        let mut fields = BTreeMap::new();
        self.space();
        if self.bytes.get(self.at) == Some(&b'}') {
            self.at += 1;
            return Ok(Json::Object(fields));
        }
        loop {
            self.space();
            let key = self.string()?;
            self.expect(b':')?;
            let value = self.value()?;
            if fields.insert(key.clone(), value).is_some() {
                return Err(format!("the key {key:?} twice"));
            }
            self.space();
            match self.bytes.get(self.at) {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(Json::Object(fields));
                }
                _ => return Err(format!("expected , or }} at byte {}", self.at)),
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.space();
        if self.bytes.get(self.at) == Some(&b']') {
            self.at += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.space();
            match self.bytes.get(self.at) {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(format!("expected , or ] at byte {}", self.at)),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let Some(&byte) = self.bytes.get(self.at) else {
                return Err("an unterminated string".into());
            };
            self.at += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let escape = *self.bytes.get(self.at).ok_or("an unterminated escape")?;
                    self.at += 1;
                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hex = std::str::from_utf8(&self.bytes[self.at..self.at + 4])
                                .map_err(|_| "a \\u escape")?;
                            let unit = u32::from_str_radix(hex, 16).map_err(|_| "a \\u escape")?;
                            self.at += 4;
                            out.push(
                                char::from_u32(unit)
                                    .ok_or("a \\u escape outside the BMP scalars")?,
                            );
                        }
                        _ => return Err(format!("an unknown escape at byte {}", self.at)),
                    }
                }
                _ => {
                    let start = self.at - 1;
                    let mut end = self.at;
                    while end < self.bytes.len() && !matches!(self.bytes[end], b'"' | b'\\') {
                        end += 1;
                    }
                    out.push_str(
                        std::str::from_utf8(&self.bytes[start..end]).map_err(|_| "not UTF-8")?,
                    );
                    self.at = end;
                }
            }
        }
    }
}
