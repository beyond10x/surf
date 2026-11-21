//! The cases of one data file, as `docs/design/html-parser.md` item 3 defines a case: a
//! tokenizer test object in one initial state, a tree-construction case of the `.dat` rule, an
//! encoding `#data` section with its `#encoding` (and `encoding/chardet/test_big5.txt`, which
//! has neither, as one case). Cases are numbered from 1 in file order; a tokenizer case's number
//! counts each initial state of each test object.

use std::fmt;
use std::path::Path;

use surf_html::tokenizer::State;
use surf_html::{Context, Namespace};

use crate::inventory::Group;
use crate::json::{self, Value};
use crate::tags::{self, Tag};
use crate::{dat, unescape};

/// A token in the form the tokenizer data writes it, every string as code points. Adjacent
/// `Character` tokens are coalesced and attributes sorted, so that two lists compare equal
/// exactly when the data would call them equal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tok {
    /// `["DOCTYPE", name, public id, system id, correctness]`; correctness is the negation of
    /// the force-quirks flag.
    Doctype {
        /// The name.
        name: Option<Vec<u32>>,
        /// The public identifier.
        public_id: Option<Vec<u32>>,
        /// The system identifier.
        system_id: Option<Vec<u32>>,
        /// The correctness.
        correct: bool,
    },
    /// `["StartTag", name, {attributes}]`, with `true` after them when self-closing.
    StartTag {
        /// The name.
        name: Vec<u32>,
        /// The attributes, sorted.
        attributes: Vec<(Vec<u32>, Vec<u32>)>,
        /// The self-closing flag.
        self_closing: bool,
    },
    /// `["EndTag", name]`.
    EndTag {
        /// The name.
        name: Vec<u32>,
    },
    /// `["Comment", data]`.
    Comment(Vec<u32>),
    /// `["Character", data]`.
    Character(Vec<u32>),
    /// `["ProcessingInstruction", target, data]`.
    ProcessingInstruction {
        /// The target.
        target: Vec<u32>,
        /// The data.
        data: Vec<u32>,
    },
    /// An end-of-file token before the last token, which the data never expects.
    EndOfFile,
}

/// Appends a token, coalescing it with a `Character` token before it.
pub fn push_tok(tokens: &mut Vec<Tok>, tok: Tok) {
    if let Tok::Character(more) = &tok
        && let Some(Tok::Character(text)) = tokens.last_mut()
    {
        text.extend_from_slice(more);
        return;
    }
    tokens.push(tok);
}

/// What a case gives the parser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    /// A tokenizer case.
    Tokenizer {
        /// The input, `doubleEscaped` unescaped.
        input: Vec<u32>,
        /// The initial state.
        state: State,
        /// `lastStartTag`.
        last_start_tag: Option<String>,
    },
    /// A tree-construction case.
    Tree {
        /// The `#data` section.
        input: String,
        /// The context element of `#document-fragment`; `None` for a document.
        fragment: Option<Context>,
        /// Whether the case has `#script-on`.
        script_on: bool,
    },
    /// An encoding case: the bytes of `#data`, or of the whole file for `chardet`.
    Encoding(Vec<u8>),
}

/// What a case expects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expected {
    /// The tokens, coalesced, and the parse error codes in order.
    Tokens {
        /// The tokens.
        tokens: Vec<Tok>,
        /// The error codes.
        errors: Vec<String>,
    },
    /// The `#document` section.
    Tree {
        /// The tree in the `| ` format.
        document: String,
    },
    /// The `#encoding` label; `None` for the `chardet` file, which names none.
    Encoding(Option<String>),
}

/// One case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Case {
    /// The data file's path relative to `crates/html-conformance/data/`.
    pub path: String,
    /// The case's number in its file, from 1.
    pub number: usize,
    /// Its tag.
    pub tag: Tag,
    /// What it gives the parser.
    pub input: Input,
    /// What it expects.
    pub expected: Expected,
}

/// Why a data file could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    /// The data file's path.
    pub path: String,
    /// What was wrong.
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for Error {}

/// Reads every case of the data file `path` (relative to `data`) of `group`.
pub fn read(data: &Path, group: Group, path: &str) -> Result<Vec<Case>, Error> {
    let fail = |message: String| Error {
        path: path.to_string(),
        message,
    };
    let bytes = std::fs::read(data.join(path)).map_err(|e| fail(e.to_string()))?;
    match group {
        Group::Tokenizer => {
            let text = std::str::from_utf8(&bytes).map_err(|e| fail(e.to_string()))?;
            tokenizer_cases(path, text).map_err(fail)
        }
        Group::TreeConstruction => tree_cases(path, &bytes).map_err(fail),
        Group::Encoding => encoding_cases(path, &bytes).map_err(fail),
    }
}

fn string(value: &Value, what: &str) -> Result<Vec<u32>, String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        other => Err(format!("{what} is {}, not a string", other.kind())),
    }
}

fn to_string(code_points: &[u32], what: &str) -> Result<String, String> {
    code_points
        .iter()
        .map(|&c| char::from_u32(c).ok_or_else(|| format!("{what} holds U+{c:04X}")))
        .collect()
}

fn state(name: &str) -> Result<State, String> {
    Ok(match name {
        "Data state" => State::Data,
        "PLAINTEXT state" => State::Plaintext,
        "RCDATA state" => State::Rcdata,
        "RAWTEXT state" => State::Rawtext,
        "Script data state" => State::ScriptData,
        "CDATA section state" => State::CdataSection,
        other => return Err(format!("unknown initial state `{other}`")),
    })
}

const TEST_KEYS: [&str; 7] = [
    "description",
    "doubleEscaped",
    "errors",
    "initialStates",
    "input",
    "lastStartTag",
    "output",
];

fn tokenizer_cases(path: &str, text: &str) -> Result<Vec<Case>, String> {
    let root = json::parse(text).map_err(|e| e.to_string())?;
    let Value::Object(members) = &root else {
        return Err("the file is not a JSON object".to_string());
    };
    let [(key, Value::Array(tests))] = members.as_slice() else {
        return Err("the file is not one member holding an array".to_string());
    };
    let key = to_string(key, "the top-level key")?;
    if key != "tests" && key != "xmlViolationTests" {
        return Err(format!("unknown top-level key `{key}`"));
    }
    let not_run = tags::not_run_file(path);
    let mut cases = Vec::new();
    for (index, test) in tests.iter().enumerate() {
        let at = |message: String| format!("test {}: {message}", index + 1);
        let Value::Object(members) = test else {
            return Err(at("not an object".to_string()));
        };
        for (k, _) in members {
            let k = to_string(k, "a key").map_err(at)?;
            if !TEST_KEYS.contains(&k.as_str()) {
                return Err(at(format!("unknown key `{k}`")));
            }
        }
        let double_escaped = match test.get("doubleEscaped") {
            None => false,
            Some(Value::Bool(b)) => *b,
            Some(other) => return Err(at(format!("doubleEscaped is {}", other.kind()))),
        };
        let unescaped = |s: Vec<u32>| -> Result<Vec<u32>, String> {
            if double_escaped { unescape(&s) } else { Ok(s) }
        };
        let input = unescaped(string(
            test.get("input")
                .ok_or_else(|| at("no input".to_string()))?,
            "input",
        )?)
        .map_err(at)?;
        let tokens = expected_tokens(
            test.get("output")
                .ok_or_else(|| at("no output".to_string()))?,
            &unescaped,
        )
        .map_err(at)?;
        let errors = match test.get("errors") {
            None => Vec::new(),
            Some(Value::Array(errors)) => errors
                .iter()
                .map(|e| {
                    let code = e.get("code").ok_or("an error without a code")?;
                    to_string(&string(code, "an error code")?, "an error code")
                })
                .collect::<Result<_, String>>()
                .map_err(at)?,
            Some(other) => return Err(at(format!("errors is {}", other.kind()))),
        };
        let last_start_tag = match test.get("lastStartTag") {
            None => None,
            Some(v) => Some(to_string(&string(v, "lastStartTag")?, "lastStartTag").map_err(at)?),
        };
        let states = match test.get("initialStates") {
            None => vec![State::Data],
            Some(Value::Array(names)) => names
                .iter()
                .map(|n| state(&to_string(&string(n, "a state")?, "a state")?))
                .collect::<Result<_, String>>()
                .map_err(at)?,
            Some(other) => return Err(at(format!("initialStates is {}", other.kind()))),
        };
        for state in states {
            let tag = match not_run {
                Some(reason) => Tag::NotRun(reason),
                None => tags::tokenizer(&input, state == State::Data),
            };
            cases.push(Case {
                path: path.to_string(),
                number: cases.len() + 1,
                tag,
                input: Input::Tokenizer {
                    input: input.clone(),
                    state,
                    last_start_tag: last_start_tag.clone(),
                },
                expected: Expected::Tokens {
                    tokens: tokens.clone(),
                    errors: errors.clone(),
                },
            });
        }
    }
    Ok(cases)
}

fn expected_tokens(
    output: &Value,
    unescaped: &dyn Fn(Vec<u32>) -> Result<Vec<u32>, String>,
) -> Result<Vec<Tok>, String> {
    let Value::Array(items) = output else {
        return Err(format!("output is {}", output.kind()));
    };
    let text = |v: &Value, what: &str| -> Result<Vec<u32>, String> { unescaped(string(v, what)?) };
    let optional = |v: &Value, what: &str| -> Result<Option<Vec<u32>>, String> {
        match v {
            Value::Null => Ok(None),
            other => text(other, what).map(Some),
        }
    };
    let mut tokens = Vec::new();
    for item in items {
        let Value::Array(fields) = item else {
            return Err(format!("a token is {}", item.kind()));
        };
        let Some(Value::String(kind)) = fields.first() else {
            return Err("a token without its kind".to_string());
        };
        let kind = to_string(kind, "a token kind")?;
        let tok = match (kind.as_str(), &fields[1..]) {
            ("DOCTYPE", [name, public_id, system_id, Value::Bool(correct)]) => Tok::Doctype {
                name: optional(name, "a doctype name")?,
                public_id: optional(public_id, "a public id")?,
                system_id: optional(system_id, "a system id")?,
                correct: *correct,
            },
            ("StartTag", [name, Value::Object(attributes), rest @ ..]) => {
                let self_closing = match rest {
                    [] => false,
                    [Value::Bool(b)] => *b,
                    _ => return Err("a start tag with more than a self-closing flag".to_string()),
                };
                let mut attributes = attributes
                    .iter()
                    .map(|(k, v)| Ok((unescaped(k.clone())?, text(v, "an attribute value")?)))
                    .collect::<Result<Vec<_>, String>>()?;
                attributes.sort();
                Tok::StartTag {
                    name: text(name, "a tag name")?,
                    attributes,
                    self_closing,
                }
            }
            ("EndTag", [name]) => Tok::EndTag {
                name: text(name, "a tag name")?,
            },
            ("Comment", [data]) => Tok::Comment(text(data, "comment data")?),
            ("Character", [data]) => Tok::Character(text(data, "character data")?),
            ("ProcessingInstruction", [target, data]) => Tok::ProcessingInstruction {
                target: text(target, "a target")?,
                data: text(data, "processing instruction data")?,
            },
            (other, _) => return Err(format!("a `{other}` token of an unknown form")),
        };
        push_tok(&mut tokens, tok);
    }
    Ok(tokens)
}

fn utf8(bytes: &[u8], what: &str) -> Result<String, String> {
    String::from_utf8(bytes.to_vec()).map_err(|_| format!("{what} is not UTF-8"))
}

fn context(line: &str) -> Context {
    let (namespace, local_name) = if let Some(name) = line.strip_prefix("svg ") {
        (Namespace::Svg, name)
    } else if let Some(name) = line.strip_prefix("math ") {
        (Namespace::MathMl, name)
    } else {
        (Namespace::Html, line)
    };
    Context {
        namespace,
        local_name: local_name.to_string(),
    }
}

const TREE_SECTIONS: [&str; 8] = [
    "description",
    "data",
    "errors",
    "new-errors",
    "document",
    "document-fragment",
    "script-on",
    "script-off",
];

fn tree_cases(path: &str, bytes: &[u8]) -> Result<Vec<Case>, String> {
    let not_run = tags::not_run_file(path);
    let mixed = tags::is_mixed(path);
    let mut cases = Vec::new();
    for case in dat::read(bytes).map_err(|e| e.to_string())? {
        let at = |message: String| format!("the case at line {}: {message}", case.line);
        for (name, _) in &case.sections {
            if !TREE_SECTIONS.contains(&name.as_str()) {
                return Err(at(format!("unknown section `#{name}`")));
            }
        }
        if case.section("script-on").is_some() && case.section("script-off").is_some() {
            return Err(at("both `#script-on` and `#script-off`".to_string()));
        }
        let input = utf8(
            case.section("data")
                .ok_or_else(|| at("no `#data`".to_string()))?,
            "`#data`",
        )
        .map_err(at)?;
        let document = utf8(
            case.section("document")
                .ok_or_else(|| at("no `#document`".to_string()))?,
            "`#document`",
        )
        .map_err(at)?;
        let fragment = match case.section("document-fragment") {
            None => None,
            Some(line) => Some(context(&utf8(line, "`#document-fragment`").map_err(at)?)),
        };
        let tag = match not_run {
            Some(reason) => Tag::NotRun(reason),
            None if mixed => Tag::Mixed,
            None => tags::thematic(&input, fragment.is_some()),
        };
        cases.push(Case {
            path: path.to_string(),
            number: cases.len() + 1,
            tag,
            input: Input::Tree {
                input,
                fragment,
                script_on: case.section("script-on").is_some(),
            },
            expected: Expected::Tree { document },
        });
    }
    Ok(cases)
}

fn encoding_cases(path: &str, bytes: &[u8]) -> Result<Vec<Case>, String> {
    let not_run = tags::not_run_file(path);
    if !path.ends_with(".dat") {
        let Some(reason) = not_run else {
            return Err("an encoding file that is not `.dat` and not a not-run file".to_string());
        };
        return Ok(vec![Case {
            path: path.to_string(),
            number: 1,
            tag: Tag::NotRun(reason),
            input: Input::Encoding(bytes.to_vec()),
            expected: Expected::Encoding(None),
        }]);
    }
    let mut cases = Vec::new();
    for case in dat::read(bytes).map_err(|e| e.to_string())? {
        let at = |message: String| format!("the case at line {}: {message}", case.line);
        let names: Vec<&str> = case.sections.iter().map(|(n, _)| n.as_str()).collect();
        if names != ["data", "encoding"] {
            return Err(at(format!(
                "sections {names:?}, not `#data` and `#encoding`"
            )));
        }
        let data = case.section("data").unwrap_or_default();
        let label =
            utf8(case.section("encoding").unwrap_or_default(), "`#encoding`").map_err(at)?;
        if label.trim().is_empty() {
            return Err(at("an empty `#encoding`".to_string()));
        }
        cases.push(Case {
            path: path.to_string(),
            number: cases.len() + 1,
            tag: not_run.map_or(Tag::Encoding, Tag::NotRun),
            input: Input::Encoding(data.to_vec()),
            expected: Expected::Encoding(Some(label)),
        });
    }
    Ok(cases)
}
