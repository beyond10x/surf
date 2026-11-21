//! A reader of the `.dat` format of the tree-construction and encoding data, over bytes (the
//! encoding data is not UTF-8).
//!
//! The rule is the one of web-platform-tests' own reader (`html/syntax/parsing/resources/
//! test.js`, `parseDat`), as `docs/design/html-parser.md` item 3 states it: a line that starts
//! with `#` and has more after it is a section heading; `#description` starts a case, and the
//! `#data` after it belongs to the same case; otherwise `#data` starts a case. Lines before the
//! first heading belong to no case.
//!
//! A section's content is its lines joined by LF, without the final LF. The last section of a
//! case also loses the line ending of the blank line that separates it from the next case:
//! `parseDat` removes one final LF from the last section when the next case starts, and one
//! more from every section.

use std::fmt;

/// One case: its sections in the order written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatCase {
    /// The 1-based line of the case's first heading.
    pub line: usize,
    /// The sections: the heading without its `#`, and the content.
    pub sections: Vec<(String, Vec<u8>)>,
}

impl DatCase {
    /// The content of the section `name` (written without `#`); `None` when the case has none.
    pub fn section(&self, name: &str) -> Option<&[u8]> {
        self.sections
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, content)| content.as_slice())
    }
}

/// Why a `.dat` file could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    /// The 1-based line.
    pub line: usize,
    /// What was wrong.
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for Error {}

/// Reads every case of a `.dat` file. A heading that is not valid UTF-8, or one that occurs
/// twice in one case, is an error.
pub fn read(file: &[u8]) -> Result<Vec<DatCase>, Error> {
    let text = file.strip_suffix(b"\n").unwrap_or(file);
    let lines: Vec<&[u8]> = text.split(|&b| b == b'\n').collect();
    let count = lines.len();
    let mut cases = Vec::new();
    let mut current: Option<DatCase> = None;
    for (i, line) in lines.into_iter().enumerate() {
        let number = i + 1;
        let last = i + 1 == count;
        if line.len() > 1 && line[0] == b'#' {
            let heading = std::str::from_utf8(&line[1..]).map_err(|_| Error {
                line: number,
                message: "a heading that is not UTF-8".to_string(),
            })?;
            let previous = current
                .as_ref()
                .and_then(|c| c.sections.last())
                .map(|(name, _)| name.as_str());
            let starts_case =
                heading == "description" || (heading == "data" && previous != Some("description"));
            if starts_case && let Some(mut case) = current.take() {
                if let Some((_, content)) = case.sections.last_mut() {
                    content.pop();
                }
                cases.push(finish(case));
            }
            let case = current.get_or_insert_with(|| DatCase {
                line: number,
                sections: Vec::new(),
            });
            if case.sections.iter().any(|(name, _)| name == heading) {
                return Err(Error {
                    line: number,
                    message: format!("a second `#{heading}` in one case"),
                });
            }
            case.sections.push((heading.to_string(), Vec::new()));
        } else if let Some((_, content)) = current.as_mut().and_then(|c| c.sections.last_mut()) {
            content.extend_from_slice(line);
            if !last {
                content.push(b'\n');
            }
        }
    }
    if let Some(case) = current {
        cases.push(finish(case));
    }
    Ok(cases)
}

fn finish(mut case: DatCase) -> DatCase {
    for (_, content) in &mut case.sections {
        if content.last() == Some(&b'\n') {
            content.pop();
        }
    }
    case
}
