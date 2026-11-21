//! Runs the cases against `surf-html` and counts them.
//!
//! - Tokenizer: [`surf_html::tokenizer::tokenize`] from the case's initial state with its last
//!   start tag. Compared: the tokens, adjacent `Character` tokens coalesced, one final
//!   end-of-file token dropped (the data writes none), and the parse error codes in order.
//! - Tree construction: [`surf_html::parse_fragment`] for the `#document-fragment` context, else
//!   [`surf_html::parse_document`], parser scripting disabled unless the case has `#script-on`.
//!   Compared: the tree in the `| ` format, byte for byte, with `#document`.
//! - Encoding: [`surf_html::parse_document_bytes`]. Compared: the name of the encoding used at
//!   the end with the `#encoding` label, both trimmed and lowered in ASCII. Every label the data
//!   holds (`utf-8`, `windows-1252`, `iso-8859-2`, `euc-jp`, in any case) is its encoding's name
//!   in lower case; a label that is not would never pass, and no case passes without an
//!   encoding.

use std::path::Path;

use surf_html::tokenizer::{self, Token};
use surf_html::{ParseOptions, parse_document, parse_document_bytes, parse_fragment};

use crate::cases::{self, Case, Expected, Input, Tok, push_tok};
use crate::dump;
use crate::inventory::Inventory;

/// What running a case gave.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Run, and the result is the expected one.
    Pass,
    /// Run, and the result differs.
    Fail,
    /// Not run, with the reason.
    NotRun(&'static str),
}

fn code_points(s: &str) -> Vec<u32> {
    s.chars().map(u32::from).collect()
}

/// The tokenizer's tokens in the data's form.
pub fn tokens(tokens: &[Token]) -> Vec<Tok> {
    let mut out = Vec::new();
    let count = tokens.len();
    for (i, token) in tokens.iter().enumerate() {
        let tok = match token {
            Token::Doctype {
                name,
                public_id,
                system_id,
                force_quirks,
            } => Tok::Doctype {
                name: name.as_deref().map(code_points),
                public_id: public_id.as_deref().map(code_points),
                system_id: system_id.as_deref().map(code_points),
                correct: !force_quirks,
            },
            Token::StartTag {
                name,
                attributes,
                self_closing,
            } => {
                let mut attributes: Vec<(Vec<u32>, Vec<u32>)> = attributes
                    .iter()
                    .map(|a| (code_points(&a.name), code_points(&a.value)))
                    .collect();
                attributes.sort();
                Tok::StartTag {
                    name: code_points(name),
                    attributes,
                    self_closing: *self_closing,
                }
            }
            Token::EndTag { name } => Tok::EndTag {
                name: code_points(name),
            },
            Token::Comment { data } => Tok::Comment(code_points(data)),
            Token::ProcessingInstruction { target, data } => Tok::ProcessingInstruction {
                target: code_points(target),
                data: code_points(data),
            },
            Token::Character(c) => Tok::Character(vec![*c]),
            Token::EndOfFile if i + 1 == count => continue,
            Token::EndOfFile => Tok::EndOfFile,
        };
        push_tok(&mut out, tok);
    }
    out
}

/// Runs one case.
pub fn run(case: &Case) -> Outcome {
    if let Some(reason) = case.tag.reason() {
        return Outcome::NotRun(reason);
    }
    let pass = match (&case.input, &case.expected) {
        (
            Input::Tokenizer {
                input,
                state,
                last_start_tag,
            },
            Expected::Tokens {
                tokens: expected_tokens,
                errors,
            },
        ) => {
            let result = tokenizer::tokenize(input, *state, last_start_tag.as_deref());
            let codes: Vec<&str> = result.errors.iter().map(|e| e.code).collect();
            tokens(&result.tokens) == *expected_tokens && codes == *errors
        }
        (
            Input::Tree {
                input,
                fragment,
                script_on,
            },
            Expected::Tree { document },
        ) => {
            // The struct update keeps the default of a field a later story adds.
            #[allow(clippy::needless_update)]
            let options = ParseOptions {
                scripting_disabled: !script_on,
                ..ParseOptions::default()
            };
            let tree = match fragment {
                Some(context) => parse_fragment(input, context, &options),
                None => parse_document(input, &options),
            };
            dump::from_tree(&tree).is_ok_and(|nodes| dump::write(&nodes) == *document)
        }
        (Input::Encoding(bytes), Expected::Encoding(Some(label))) => {
            let parsed = parse_document_bytes(bytes, &ParseOptions::default());
            parsed
                .encoding
                .is_some_and(|name| name.eq_ignore_ascii_case(label.trim()))
        }
        _ => false,
    };
    if pass { Outcome::Pass } else { Outcome::Fail }
}

/// Cases that exist, run and pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Count {
    /// Cases that exist.
    pub exist: usize,
    /// Cases that run.
    pub run: usize,
    /// Cases that pass.
    pub pass: usize,
}

impl Count {
    fn add(&mut self, outcome: Outcome) {
        self.exist += 1;
        match outcome {
            Outcome::Pass => {
                self.run += 1;
                self.pass += 1;
            }
            Outcome::Fail => self.run += 1,
            Outcome::NotRun(_) => {}
        }
    }

    /// `<pass>/<run>/<exist>`.
    pub fn fraction(&self) -> String {
        format!("{}/{}/{}", self.pass, self.run, self.exist)
    }
}

/// The counts of one data file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileReport {
    /// The path relative to `data/`.
    pub path: String,
    /// Its counts.
    pub count: Count,
    /// Each reason cases of it are not run, with their number, in first-seen order.
    pub not_run: Vec<(&'static str, usize)>,
}

/// The counts of a whole run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Per data file, in the order of the inventory.
    pub files: Vec<FileReport>,
    /// Per tag, in the order of the inventory's `tag` lines; a tag no line names after them.
    pub tags: Vec<(String, Count)>,
    /// Over every case.
    pub total: Count,
    /// Every case that runs and does not pass: path, case number, tag name.
    pub failing: Vec<(String, usize, &'static str)>,
}

impl Report {
    /// The lines the binary prints: one per file, one per tag, then the total.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for file in &self.files {
            let mut line = format!("file {} {}", file.path, file.count.fraction());
            for (reason, n) in &file.not_run {
                line.push_str(&format!(" not-run:{reason}={n}"));
            }
            lines.push(line);
        }
        for (name, count) in &self.tags {
            lines.push(format!("tag {name} {}", count.fraction()));
        }
        lines.push(format!(
            "html-conformance: {} cases, {} run, {} passed",
            self.total.exist, self.total.run, self.total.pass
        ));
        lines
    }
}

/// Reads and runs every file of the inventory.
pub fn run_all(data: &Path, inventory: &Inventory) -> Result<Report, cases::Error> {
    let mut tags: Vec<(String, Count)> = inventory
        .tags
        .iter()
        .map(|(name, _)| (name.clone(), Count::default()))
        .collect();
    let mut report = Report {
        files: Vec::new(),
        tags: Vec::new(),
        total: Count::default(),
        failing: Vec::new(),
    };
    for file in &inventory.files {
        let mut file_report = FileReport {
            path: file.path.clone(),
            count: Count::default(),
            not_run: Vec::new(),
        };
        for case in cases::read(data, file.group, &file.path)? {
            let outcome = run(&case);
            file_report.count.add(outcome);
            report.total.add(outcome);
            let name = case.tag.name();
            match tags.iter_mut().find(|(n, _)| n == name) {
                Some((_, count)) => count.add(outcome),
                None => {
                    let mut count = Count::default();
                    count.add(outcome);
                    tags.push((name.to_string(), count));
                }
            }
            match outcome {
                Outcome::NotRun(reason) => {
                    match file_report.not_run.iter_mut().find(|(r, _)| *r == reason) {
                        Some((_, n)) => *n += 1,
                        None => file_report.not_run.push((reason, 1)),
                    }
                }
                Outcome::Fail => report.failing.push((case.path.clone(), case.number, name)),
                Outcome::Pass => {}
            }
        }
        report.files.push(file_report);
    }
    report.tags = tags;
    Ok(report)
}
