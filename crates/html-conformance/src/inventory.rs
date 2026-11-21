//! A reader of the `html-inventory` block of `docs/design/html-parser.md` (item 9), in the
//! note's line formats: `source <name> <address> <commit>`, `file <group> <path> <cases>
//! <tag>=<n>...` (the `not-run` tag written `not-run:<reason>=<n>`), `tag <name> <n>` and
//! `total <n>`, one record per line, fields separated by one space.

use std::fmt;
use std::path::Path;

/// The group a data file counts toward, the `<group>` of a `file` line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Group {
    /// The tokenizer data, JSON.
    Tokenizer,
    /// The tree-construction data, `.dat`.
    TreeConstruction,
    /// The encoding data, `.dat`.
    Encoding,
}

impl Group {
    /// The group's name in the block.
    pub fn name(self) -> &'static str {
        match self {
            Group::Tokenizer => "tokenizer",
            Group::TreeConstruction => "tree-construction",
            Group::Encoding => "encoding",
        }
    }

    fn from_name(name: &str) -> Option<Group> {
        match name {
            "tokenizer" => Some(Group::Tokenizer),
            "tree-construction" => Some(Group::TreeConstruction),
            "encoding" => Some(Group::Encoding),
            _ => None,
        }
    }
}

/// A `source` line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    /// The name, which is also the first part of every path from it.
    pub name: String,
    /// The repository's address.
    pub address: String,
    /// The commit the data is pinned at.
    pub commit: String,
}

/// A `file` line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    /// The group.
    pub group: Group,
    /// The path relative to `crates/html-conformance/data/`.
    pub path: String,
    /// The number of cases.
    pub cases: usize,
    /// The count per tag, in the order written: the key is the tag's name, or
    /// `not-run:<reason>`.
    pub tags: Vec<(String, usize)>,
}

/// The whole block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inventory {
    /// The `source` lines.
    pub sources: Vec<Source>,
    /// The `file` lines, in the order written.
    pub files: Vec<File>,
    /// The `tag` lines, in the order written: the tag's name and its count.
    pub tags: Vec<(String, usize)>,
    /// The `total` line.
    pub total: usize,
}

/// Why the block could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// The lines of the fenced block `name` of a Markdown text: the lines between a line
/// "```name" and the next line "```".
pub fn block<'a>(text: &'a str, name: &str) -> Result<Vec<&'a str>, Error> {
    let open = format!("```{name}");
    let mut lines = text.lines();
    if !lines.by_ref().any(|line| line == open) {
        return Err(Error(format!("no block `{name}`")));
    }
    let mut out = Vec::new();
    for line in lines {
        if line == "```" {
            return Ok(out);
        }
        out.push(line);
    }
    Err(Error(format!("the block `{name}` is not closed")))
}

/// Reads the `html-inventory` block of the note at `note`.
pub fn load(note: &Path) -> Result<Inventory, Error> {
    let text =
        std::fs::read_to_string(note).map_err(|e| Error(format!("{}: {e}", note.display())))?;
    parse(&block(&text, "html-inventory")?)
        .map_err(|e| Error(format!("{}: html-inventory: {e}", note.display())))
}

fn count(field: &str, line: &str) -> Result<usize, Error> {
    if field.is_empty() || !field.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error(format!("`{field}` is not a count in `{line}`")));
    }
    field
        .parse()
        .map_err(|_| Error(format!("`{field}` is not a count in `{line}`")))
}

/// Reads the lines of an `html-inventory` block, and checks that each `file` line's counts sum
/// to its cases, that the `tag` lines sum to `total` and that the `file` lines do too.
pub fn parse(lines: &[&str]) -> Result<Inventory, Error> {
    let mut sources = Vec::new();
    let mut files = Vec::new();
    let mut tags = Vec::new();
    let mut total = None;
    for &line in lines {
        let fields: Vec<&str> = line.split(' ').collect();
        match fields.as_slice() {
            ["source", name, address, commit] => sources.push(Source {
                name: name.to_string(),
                address: address.to_string(),
                commit: commit.to_string(),
            }),
            ["file", group, path, cases, pairs @ ..] => {
                let group = Group::from_name(group)
                    .ok_or_else(|| Error(format!("unknown group in `{line}`")))?;
                let cases = count(cases, line)?;
                let mut per_tag = Vec::new();
                for pair in pairs {
                    let (key, n) = pair
                        .split_once('=')
                        .ok_or_else(|| Error(format!("`{pair}` is not <tag>=<n> in `{line}`")))?;
                    per_tag.push((key.to_string(), count(n, line)?));
                }
                let sum: usize = per_tag.iter().map(|(_, n)| n).sum();
                if sum != cases {
                    return Err(Error(format!(
                        "the tag counts sum to {sum}, not {cases}, in `{line}`"
                    )));
                }
                files.push(File {
                    group,
                    path: path.to_string(),
                    cases,
                    tags: per_tag,
                });
            }
            ["tag", name, n] => tags.push((name.to_string(), count(n, line)?)),
            ["total", n] if total.is_none() => total = Some(count(n, line)?),
            _ => return Err(Error(format!("not a record: `{line}`"))),
        }
    }
    let total = total.ok_or_else(|| Error("no `total` line".to_string()))?;
    let tag_sum: usize = tags.iter().map(|(_, n)| n).sum();
    let file_sum: usize = files.iter().map(|f| f.cases).sum();
    if tag_sum != total || file_sum != total {
        return Err(Error(format!(
            "total {total}, the `tag` lines sum to {tag_sum}, the `file` lines to {file_sum}"
        )));
    }
    for file in &files {
        for (key, _) in &file.tags {
            let name = key.split(':').next().unwrap_or(key);
            if !tags.iter().any(|(t, _)| t == name) {
                return Err(Error(format!(
                    "{}: tag `{key}` has no `tag` line",
                    file.path
                )));
            }
        }
        let source = file.path.split('/').next().unwrap_or("");
        if !sources.iter().any(|s: &Source| s.name == source) {
            return Err(Error(format!(
                "{}: no `source` line names `{source}`",
                file.path
            )));
        }
    }
    Ok(Inventory {
        sources,
        files,
        tags,
        total,
    })
}
