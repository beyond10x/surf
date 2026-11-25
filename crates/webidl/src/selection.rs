//! The reader of `selection.txt`, the one table of the Web IDL selection
//! (`story:webidl-bindings`, decision 1): which definitions the binary `bindings` generates,
//! which of them is the global object, and one row per member and per constructor with its
//! slice and its notes. The file's head states the grammar; [`Selection::read`] holds it and
//! [`Selection::check`] holds the rows against a [`Model`].

use std::collections::BTreeSet;
use std::fmt;

use crate::{DefinitionKind, MemberKind, Model};

/// The slice a member belongs to: the story that gives it its behaviour, or `throws` for a
/// member generated to throw `NotSupportedError` with its reason, or `exception` for
/// `DOMException`'s members, generated with their behaviour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Slice {
    Reading,
    Attributes,
    Mutation,
    Selectors,
    Throws,
    Exception,
}

impl Slice {
    /// Every slice, in the order of the grammar.
    pub const ALL: [Slice; 6] = [
        Slice::Reading,
        Slice::Attributes,
        Slice::Mutation,
        Slice::Selectors,
        Slice::Throws,
        Slice::Exception,
    ];

    /// The slice's word in the file.
    pub fn word(self) -> &'static str {
        match self {
            Slice::Reading => "reading",
            Slice::Attributes => "attributes",
            Slice::Mutation => "mutation",
            Slice::Selectors => "selectors",
            Slice::Throws => "throws",
            Slice::Exception => "exception",
        }
    }

    /// The slice of `word`, if it names one.
    pub fn from_word(word: &str) -> Option<Slice> {
        Slice::ALL.into_iter().find(|slice| slice.word() == word)
    }

    /// Whether a member of this slice gets a method of a Rust trait: `reading`, `attributes`,
    /// `mutation` and `selectors` do; `throws` and `exception` are generated whole.
    pub fn has_trait(self) -> bool {
        match self {
            Slice::Reading | Slice::Attributes | Slice::Mutation | Slice::Selectors => true,
            Slice::Throws | Slice::Exception => false,
        }
    }
}

/// A clause of a row's note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `setter: <slice>`: the setter of a writable attribute belongs to that slice.
    Setter(Slice),
    /// `reason: <words>`: why a member of `throws` throws.
    Reason(String),
    /// `deviation: <words>`: where the member deviates from its standard.
    Deviation(String),
}

/// One row: `<interface> <member> <slice> [<note>]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// The line of the file, from 1.
    pub line: usize,
    /// The interface.
    pub interface: String,
    /// The member's name, `constructor` for a constructor, `iterable` for `iterable<T>`.
    pub member: String,
    /// The slice of the member (of its getter, for an attribute).
    pub slice: Slice,
    /// The clauses of the note, in the order written.
    pub notes: Vec<Note>,
}

impl Row {
    /// The reason of a row of `throws`.
    pub fn reason(&self) -> Option<&str> {
        self.notes.iter().find_map(|note| match note {
            Note::Reason(reason) => Some(reason.as_str()),
            Note::Setter(_) | Note::Deviation(_) => None,
        })
    }

    /// The slice of the attribute's setter: the note's `setter:`, else the row's slice.
    pub fn setter_slice(&self) -> Slice {
        self.notes
            .iter()
            .find_map(|note| match note {
                Note::Setter(slice) => Some(*slice),
                Note::Reason(_) | Note::Deviation(_) => None,
            })
            .unwrap_or(self.slice)
    }

    /// The deviations the row names.
    pub fn deviations(&self) -> Vec<&str> {
        self.notes
            .iter()
            .filter_map(|note| match note {
                Note::Deviation(words) => Some(words.as_str()),
                Note::Setter(_) | Note::Reason(_) => None,
            })
            .collect()
    }

    /// Whether the row is a constructor's.
    pub fn is_constructor(&self) -> bool {
        self.member == "constructor"
    }

    /// Whether the row is `iterable<T>`'s, which is not counted as a member.
    pub fn is_iterable(&self) -> bool {
        self.member == "iterable"
    }
}

/// A line the reader refuses, or a row the model does not hold: `<file>:<line>: <what>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionError {
    /// The file named to [`Selection::read`].
    pub file: String,
    /// The line, from 1; 0 for the file as a whole.
    pub line: usize,
    /// What is wrong.
    pub what: String,
}

impl fmt::Display for SelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.file, self.line, self.what)
    }
}

impl std::error::Error for SelectionError {}

/// The selection read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    /// The file named to [`Selection::read`].
    pub file: String,
    /// The definitions generated, in the order of the `definitions:` line.
    pub definitions: Vec<String>,
    /// The line of `definitions:`.
    pub definitions_line: usize,
    /// The definition that is the global object.
    pub global: String,
    /// The rows, in the order of the file.
    pub rows: Vec<Row>,
}

impl Selection {
    /// Reads `text`, naming `file` in each error. It refuses a line that is no comment, no
    /// directive and no row of the grammar; `definitions:` or `global:` missing, twice, or after
    /// a row; a `global:` that is not among the definitions; a row whose interface is not among
    /// the definitions; an unknown slice or note; a row of `throws` without `reason:`, and a
    /// `reason:` on a row of another slice; a `setter:` of `throws` or `exception`; and a row
    /// twice.
    pub fn read(text: &str, file: &str) -> Result<Selection, SelectionError> {
        let error = |line: usize, what: String| SelectionError {
            file: file.to_string(),
            line,
            what,
        };
        let mut definitions: Option<(Vec<String>, usize)> = None;
        let mut global: Option<(String, usize)> = None;
        let mut rows: Vec<Row> = Vec::new();
        let mut seen = BTreeSet::new();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let content = raw.trim();
            if content.is_empty() || content.starts_with('#') {
                continue;
            }
            if let Some(names) = content.strip_prefix("definitions:") {
                if definitions.is_some() {
                    return Err(error(line, "a second `definitions:`".to_string()));
                }
                if !rows.is_empty() {
                    return Err(error(line, "`definitions:` after a row".to_string()));
                }
                let names: Vec<String> = names.split_whitespace().map(str::to_string).collect();
                if names.is_empty() {
                    return Err(error(
                        line,
                        "`definitions:` names no definition".to_string(),
                    ));
                }
                let mut once = BTreeSet::new();
                for name in &names {
                    if !once.insert(name) {
                        return Err(error(line, format!("`{name}` twice in `definitions:`")));
                    }
                }
                definitions = Some((names, line));
                continue;
            }
            if let Some(name) = content.strip_prefix("global:") {
                if global.is_some() {
                    return Err(error(line, "a second `global:`".to_string()));
                }
                if !rows.is_empty() {
                    return Err(error(line, "`global:` after a row".to_string()));
                }
                let mut words = name.split_whitespace();
                let (Some(name), None) = (words.next(), words.next()) else {
                    return Err(error(line, "`global:` names one definition".to_string()));
                };
                global = Some((name.to_string(), line));
                continue;
            }
            let Some((names, _)) = &definitions else {
                return Err(error(line, "a row before `definitions:`".to_string()));
            };
            if global.is_none() {
                return Err(error(line, "a row before `global:`".to_string()));
            }
            let row = read_row(content, line).map_err(|what| error(line, what))?;
            if !names.contains(&row.interface) {
                return Err(error(
                    line,
                    format!(
                        "`{} {}`: `{}` is not among the definitions generated",
                        row.interface, row.member, row.interface
                    ),
                ));
            }
            if !seen.insert((row.interface.clone(), row.member.clone())) {
                return Err(error(
                    line,
                    format!("a second row of `{} {}`", row.interface, row.member),
                ));
            }
            rows.push(row);
        }
        let Some((definitions, definitions_line)) = definitions else {
            return Err(error(0, "no `definitions:` line".to_string()));
        };
        let Some((global, global_line)) = global else {
            return Err(error(0, "no `global:` line".to_string()));
        };
        if !definitions.contains(&global) {
            return Err(error(
                global_line,
                format!("the global `{global}` is not among the definitions"),
            ));
        }
        Ok(Selection {
            file: file.to_string(),
            definitions,
            definitions_line,
            global,
            rows,
        })
    }

    /// Holds the selection against `model`: each definition is an interface the model defines
    /// (not only a partial), whose parent, if it has one, is among the definitions; each row
    /// names a member of its interface's merged view ([`Model::members`]): an attribute, an
    /// operation or a constant by its name, `constructor` a constructor, `iterable` an
    /// `iterable<T>`. A setter note names a writable attribute. The first failure is answered,
    /// naming its line.
    pub fn check(&self, model: &Model) -> Result<(), SelectionError> {
        let error = |line: usize, what: String| SelectionError {
            file: self.file.clone(),
            line,
            what,
        };
        for name in &self.definitions {
            let Some(parent) = interface_parent(model, name) else {
                return Err(error(
                    self.definitions_line,
                    format!("`{name}` is no interface the model defines"),
                ));
            };
            if let Some(parent) = parent
                && !self.definitions.contains(&parent)
            {
                return Err(error(
                    self.definitions_line,
                    format!(
                        "`{name}` inherits from `{parent}`, which is not among the definitions"
                    ),
                ));
            }
        }
        for row in &self.rows {
            let members = model.members(&row.interface);
            let found = members.iter().find(|member| match &member.kind {
                MemberKind::Constructor { .. } => row.is_constructor(),
                MemberKind::Iterable { .. } => row.is_iterable(),
                MemberKind::Const { .. }
                | MemberKind::Attribute { .. }
                | MemberKind::Operation { .. }
                | MemberKind::Stringifier => member.name.as_deref() == Some(row.member.as_str()),
            });
            let Some(member) = found else {
                return Err(error(
                    row.line,
                    format!(
                        "`{} {}`: the model holds no member `{}` of `{}`",
                        row.interface, row.member, row.member, row.interface
                    ),
                ));
            };
            let setter = row.notes.iter().any(|note| matches!(note, Note::Setter(_)));
            let writable = matches!(
                member.kind,
                MemberKind::Attribute {
                    readonly: false,
                    ..
                }
            );
            if setter && !writable {
                return Err(error(
                    row.line,
                    format!(
                        "`{} {}`: a `setter:` note on a member that is no writable attribute",
                        row.interface, row.member
                    ),
                ));
            }
        }
        Ok(())
    }

    /// The rows of `interface`, in the order of the file.
    pub fn rows_of<'s>(&'s self, interface: &'s str) -> impl Iterator<Item = &'s Row> + 's {
        self.rows
            .iter()
            .filter(move |row| row.interface == interface)
    }
}

/// The parent of the interface `name` that the model defines: `Some(None)` for one without a
/// parent, `None` when the model defines no interface `name`.
fn interface_parent(model: &Model, name: &str) -> Option<Option<String>> {
    model
        .definitions()
        .iter()
        .find_map(|definition| match &definition.kind {
            DefinitionKind::Interface {
                partial: false,
                inherits,
                ..
            } if definition.name == name => {
                Some(inherits.as_ref().map(|parent| parent.name.clone()))
            }
            _ => None,
        })
}

/// One row, or what is wrong with it.
fn read_row(content: &str, line: usize) -> Result<Row, String> {
    let mut words = content.splitn(4, char::is_whitespace);
    let (Some(interface), Some(member), Some(slice)) = (words.next(), words.next(), words.next())
    else {
        return Err(format!(
            "not a row `<interface> <member> <slice> [<note>]`: {content}"
        ));
    };
    let Some(slice) = Slice::from_word(slice) else {
        return Err(format!("`{interface} {member}`: no slice `{slice}`"));
    };
    let identifier = |word: &str| {
        word.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    if !identifier(interface) || !identifier(member) {
        return Err(format!(
            "not a row `<interface> <member> <slice> [<note>]`: {content}"
        ));
    }
    let mut notes = Vec::new();
    if let Some(rest) = words.next() {
        for clause in rest.trim().split("; ") {
            let Some((key, value)) = clause.split_once(": ") else {
                return Err(format!(
                    "`{interface} {member}`: a note clause is `<key>: <words>`: {clause}"
                ));
            };
            let value = value.trim();
            if value.is_empty() {
                return Err(format!("`{interface} {member}`: `{key}:` without words"));
            }
            let note = match key {
                "setter" => match Slice::from_word(value) {
                    Some(setter) if setter.has_trait() => Note::Setter(setter),
                    Some(_) | None => {
                        return Err(format!(
                            "`{interface} {member}`: `setter:` names no slice with a trait: {value}"
                        ));
                    }
                },
                "reason" => Note::Reason(value.to_string()),
                "deviation" => Note::Deviation(value.to_string()),
                other => {
                    return Err(format!("`{interface} {member}`: no note `{other}:`"));
                }
            };
            notes.push(note);
        }
    }
    let reasons = notes
        .iter()
        .filter(|note| matches!(note, Note::Reason(_)))
        .count();
    match (slice, reasons) {
        (Slice::Throws, 1) => {}
        (Slice::Throws, _) => {
            return Err(format!(
                "`{interface} {member}`: a row of throws names one `reason:`"
            ));
        }
        (_, 0) => {}
        (_, _) => {
            return Err(format!(
                "`{interface} {member}`: `reason:` on a row of {}",
                slice.word()
            ));
        }
    }
    Ok(Row {
        line,
        interface: interface.to_string(),
        member: member.to_string(),
        slice,
        notes,
    })
}
