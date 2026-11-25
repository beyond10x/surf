//! The generator of `crates/script/src/bindings.rs` (`story:webidl-bindings`): from the model
//! and the selection it writes, after a header with the WHATWG notice and the sha256 of each
//! source, [`crate::prelude`]'s text, the `Interface` enum with its brands and the table of which
//! brand implements which interface, the dictionaries and unions the arguments use, one trait per
//! slice whose default for every method answers `NotSupportedError` naming the member, one host
//! function per getter, setter, operation and interface object, and the table the realm's setup
//! reads. It refuses, naming the row, a definition the selection does not name, a member the
//! model does not hold ([`Selection::check`]), a type or a member it does not convert,
//! `[LegacyUnforgeable]` off the global interface, two arguments of one member that get one
//! Rust name, and two members that get one Rust name (`fooBar` and `foo_bar`), naming both.
//!
//! What it writes from the model and the selection is Rust: a name of the model becomes an
//! identifier by [`identifier`]'s one rule (`snake` form, `-` written `_`, a keyword raw; `self`,
//! `Self`, `super`, `crate` and the generator's own parameter names `cx` and `this` with a `_`
//! at the end), and a note of the selection goes into a one-line `///` comment with each control
//! character and each bidirectional control character written as its escape ([`doc_text`]); in a
//! string literal it is written with Rust's escapes.
//!
//! Beside the digests of its sources, the header holds the sha256 of the whole file with the 64
//! digits of that line read as zeros, which the binary `bindings` fills in after `rustfmt`
//! ([`FILE_DIGEST_LINE`]), so that `task webidl-check` notices a change of any byte of the file.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::selection::{Row, Selection, SelectionError, Slice};
use crate::{
    Argument, ConstValue, DefaultValue, DefinitionKind, Member, MemberKind, Model, ReadText, Type,
    TypeKind,
};

/// The sha256 of a source and its path from the repository root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Digest {
    /// 64 lowercase hex digits.
    pub sha256: String,
    /// The path, `/`-separated, from the repository root.
    pub path: String,
}

/// Node types the model does not define that a member may return (as `HTMLSlotElement?`): an
/// element the HTML Standard defines, wrapped as a node.
const NODE_TYPES_OUTSIDE_MODEL: &[&str] = &["HTMLSlotElement"];

/// Web IDL's table of error names with a legacy code: each name and the constant of
/// `DOMException` that holds its code.
const ERROR_NAMES: &[(&str, &str)] = &[
    ("IndexSizeError", "INDEX_SIZE_ERR"),
    ("HierarchyRequestError", "HIERARCHY_REQUEST_ERR"),
    ("WrongDocumentError", "WRONG_DOCUMENT_ERR"),
    ("InvalidCharacterError", "INVALID_CHARACTER_ERR"),
    ("NoModificationAllowedError", "NO_MODIFICATION_ALLOWED_ERR"),
    ("NotFoundError", "NOT_FOUND_ERR"),
    ("NotSupportedError", "NOT_SUPPORTED_ERR"),
    ("InUseAttributeError", "INUSE_ATTRIBUTE_ERR"),
    ("InvalidStateError", "INVALID_STATE_ERR"),
    ("SyntaxError", "SYNTAX_ERR"),
    ("InvalidModificationError", "INVALID_MODIFICATION_ERR"),
    ("NamespaceError", "NAMESPACE_ERR"),
    ("InvalidAccessError", "INVALID_ACCESS_ERR"),
    ("TypeMismatchError", "TYPE_MISMATCH_ERR"),
    ("SecurityError", "SECURITY_ERR"),
    ("NetworkError", "NETWORK_ERR"),
    ("AbortError", "ABORT_ERR"),
    ("URLMismatchError", "URL_MISMATCH_ERR"),
    ("QuotaExceededError", "QUOTA_EXCEEDED_ERR"),
    ("TimeoutError", "TIMEOUT_ERR"),
    ("InvalidNodeTypeError", "INVALID_NODE_TYPE_ERR"),
    ("DataCloneError", "DATA_CLONE_ERR"),
];

/// The slices with a trait, each with its trait's name.
const TRAITS: [(Slice, &str); 4] = [
    (Slice::Reading, "Reading"),
    (Slice::Attributes, "Attributes"),
    (Slice::Mutation, "Mutation"),
    (Slice::Selectors, "Selectors"),
];

/// The WHATWG notice of `crates/html/src/charref_table.rs:1-26`.
const NOTICE: &str = "// Copyright © WHATWG (Apple, Google, Mozilla, Microsoft).
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this
//    list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice,
//    this list of conditions and the following disclaimer in the documentation
//    and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its
//    contributors may be used to endorse or promote products derived from
//    this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS \"AS IS\"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
";

/// What `this` is for the members of an interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum This {
    /// A node: the brand check, then the node through the wrapper hook.
    Node,
    /// A `NodeList` or an `HTMLCollection`: the nodes of its snapshot.
    Collection,
    /// The global object.
    Global,
    /// `DOMException`.
    Exception,
    /// Only the brand check (`EventTarget`): its members throw.
    Brand,
}

/// A generated interface.
struct Interface {
    name: String,
    variant: String,
    parent: Option<String>,
    this: This,
}

/// How an argument is converted, before and after the copies.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Conversion {
    DomString,
    NullableDomString,
    LegacyNullString,
    Boolean,
    Unsigned { bits: u32 },
    Node { interface: String, nullable: bool },
    NodesOrStrings,
    Dictionary(String),
    StringOrDictionary(String),
    BooleanOrDictionary(String),
}

/// A dictionary the arguments use, with the members it converts.
#[derive(Clone, Debug)]
struct Dictionary {
    name: String,
    members: Vec<DictionaryField>,
}

#[derive(Clone, Debug)]
struct DictionaryField {
    name: String,
    kind: FieldKind,
}

#[derive(Clone, Debug)]
enum FieldKind {
    Boolean(Option<bool>),
    DomString(Option<String>),
    /// An interface type the bindings do not generate: only absent, or `null` when nullable.
    Absent {
        interface: String,
        nullable: bool,
    },
}

/// How a slice's answer is returned to the script.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Return {
    Undefined,
    Boolean,
    Unsigned16,
    Unsigned32,
    String,
    NullableString,
    Node,
    NullableNode,
    Collection(String),
    Strings,
    Window,
}

impl Return {
    fn rust(&self) -> &'static str {
        match self {
            Return::Undefined => "()",
            Return::Boolean => "bool",
            Return::Unsigned16 => "u16",
            Return::Unsigned32 => "u32",
            Return::String => "String",
            Return::NullableString => "Option<String>",
            Return::Node => "NodeId",
            Return::NullableNode => "Option<NodeId>",
            Return::Collection(_) => "Vec<NodeId>",
            Return::Strings => "Vec<String>",
            Return::Window => "TheWindow",
        }
    }

    fn converter(&self) -> String {
        match self {
            Return::Undefined => "|_, _, ()| Ok(Value::Undefined)".to_string(),
            Return::Boolean => "|_, _, value| Ok(Value::Boolean(value))".to_string(),
            Return::Unsigned16 | Return::Unsigned32 => {
                "|_, _, value| Ok(number(f64::from(value)))".to_string()
            }
            Return::String => "|dcx, _, value| string_value(dcx.engine(), &value)".to_string(),
            Return::NullableString => {
                "|dcx, _, value| nullable_string_value(dcx.engine(), value)".to_string()
            }
            Return::Node => "node_value".to_string(),
            Return::NullableNode => "nullable_node_value".to_string(),
            Return::Collection(variant) => format!(
                "|dcx, slices, value| collection_value(dcx, slices, Interface::{variant}, value)"
            ),
            Return::Strings => "|dcx, _, value| strings_value(dcx, value)".to_string(),
            Return::Window => "|dcx, _, TheWindow| global_value(dcx)".to_string(),
        }
    }
}

/// An argument of a member, as the generator writes it.
struct Arg {
    name: String,
    conversion: Conversion,
    optional: bool,
    default: Option<DefaultValue>,
    variadic: bool,
}

/// A method of a slice's trait.
struct Method {
    name: String,
    member: String,
    doc: String,
    this: This,
    args: Vec<Arg>,
    ret: Return,
    /// An attribute's setter: Web IDL converts its first argument, `undefined` when none was
    /// passed (3.7.6), so no count of arguments is checked.
    setter: bool,
}

/// The generator's state over one run.
struct Generator<'m> {
    model: &'m Model,
    selection: &'m Selection,
    interfaces: Vec<Interface>,
    dictionaries: BTreeMap<String, Dictionary>,
    unions: BTreeSet<Conversion>,
    methods: BTreeMap<Slice, Vec<Method>>,
    functions: String,
    /// The Rust name each member of the selection is written with (`<interface>_<member>` in
    /// `snake` form, a setter's with `set_` before it), and the member's label.
    names: BTreeMap<String, String>,
    specs: String,
}

impl PartialOrd for Conversion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Conversion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        format!("{self:?}").cmp(&format!("{other:?}"))
    }
}

/// The start of the header's last line, which ends in the sha256 of the whole file, every byte
/// of it, with the 64 digits of this line read as zeros: [`generate`] writes 64 zeros there,
/// and the binary `bindings` the digest, after `rustfmt` ([`fill_file_digest`]).
pub const FILE_DIGEST_LINE: &str =
    "//! The sha256 of this whole file, with the 64 digits of this line read as zeros: ";

/// `text` with the 64 digits of its first [`FILE_DIGEST_LINE`] replaced by what `sha256`
/// answers for all of `text` with those digits read as zeros; `None` when `text` holds no such
/// line with 64 digits.
pub fn fill_file_digest(
    text: &str,
    sha256: impl FnOnce(&[u8]) -> Result<String, String>,
) -> Option<Result<String, String>> {
    let start = if text.starts_with(FILE_DIGEST_LINE) {
        0
    } else {
        text.find(&format!("\n{FILE_DIGEST_LINE}"))? + 1
    };
    let digits = start + FILE_DIGEST_LINE.len();
    let end = digits + 64;
    if text.get(digits..end)?.len() != 64 {
        return None;
    }
    let zeroed = format!("{}{}{}", &text[..digits], "0".repeat(64), &text[end..]);
    Some(
        sha256(zeroed.as_bytes())
            .map(|digest| format!("{}{digest}{}", &text[..digits], &text[end..])),
    )
}

/// Writes `bindings.rs` from `model` and `selection`, with `digests` in its header:
/// [`generate_bindings`] with no line of what was read.
pub fn generate(
    model: &Model,
    selection: &Selection,
    digests: &[Digest],
) -> Result<String, SelectionError> {
    generate_bindings(model, selection, digests, &[])
}

/// Writes `bindings.rs` from `model` and `selection`, with `digests` in its header and, when
/// `reads` holds any, one line per page of what was read of it ([`ReadText::line`]).
pub fn generate_bindings(
    model: &Model,
    selection: &Selection,
    digests: &[Digest],
    reads: &[ReadText],
) -> Result<String, SelectionError> {
    selection.check(model)?;
    let mut generator = Generator {
        model,
        selection,
        interfaces: Vec::new(),
        dictionaries: BTreeMap::new(),
        unions: BTreeSet::new(),
        methods: BTreeMap::new(),
        functions: String::new(),
        names: BTreeMap::new(),
        specs: String::new(),
    };
    generator.order_interfaces()?;
    for index in 0..generator.interfaces.len() {
        generator.interface(index)?;
    }
    Ok(generator.write(digests, reads))
}

/// `getElementsByTagNameNS` as `get_elements_by_tag_name_ns`, `DOMException` as
/// `dom_exception`.
pub fn snake(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (index, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() {
            let previous = index.checked_sub(1).map(|at| chars[at]);
            let next = chars.get(index + 1);
            let after_lower =
                previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit());
            let ends_acronym = previous.is_some_and(|p| p.is_ascii_uppercase())
                && next.is_some_and(|n| n.is_ascii_lowercase());
            if after_lower || ends_acronym {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// `DOMException` as `DomException`, `HTMLCollection` as `HtmlCollection`.
pub fn camel(name: &str) -> String {
    snake(name)
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// The names of the parameters [`Generator::write_traits`] gives a slice's method before its
/// arguments (beside `self`).
const OWN_PARAMETERS: [&str; 2] = ["cx", "this"];

/// A Rust identifier for the Web IDL identifier `name` (`/_?[A-Za-z][0-9A-Z_a-z-]*/`): its
/// [`snake`] form with each `-` written `_`; a keyword that has a raw form is written raw
/// (`r#type`), and a name Rust or the generator keeps for itself gets a `_` at its end: the
/// keywords without a raw form (`self`, `Self`, `super`, `crate`) and [`OWN_PARAMETERS`], the
/// names of the parameters the generator writes before the arguments of a slice's method
/// (`cx_`). Two names of one member that give the same identifier are refused by the
/// generator, naming the row.
fn identifier(name: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "as", "async", "await", "break", "const", "continue", "dyn", "else", "enum", "extern",
        "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
        "pub", "ref", "return", "static", "struct", "trait", "true", "type", "unsafe", "use",
        "where", "while", "abstract", "become", "box", "do", "final", "gen", "macro", "override",
        "priv", "try", "typeof", "unsized", "virtual", "yield",
    ];
    const NOT_RAW: &[&str] = &["self", "Self", "super", "crate"];
    let snake = snake(name).replace('-', "_");
    if NOT_RAW.contains(&snake.as_str()) || OWN_PARAMETERS.contains(&snake.as_str()) {
        format!("{snake}_")
    } else if KEYWORDS.contains(&snake.as_str()) {
        format!("r#{snake}")
    } else {
        snake
    }
}

/// `text` for a one-line `///` comment: each control character (a carriage return, a tab) and
/// each bidirectional control character rustc's lint `text_direction_codepoint_in_comment`
/// refuses in a comment (U+202A to U+202E, U+2066 to U+2069) written as its Rust escape
/// (`\r`, `\u{202e}`), so that the line holds no bare CR, which rustc refuses in a doc comment,
/// no line break, and no character that lint refuses.
fn doc_text(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') {
                c.escape_default().to_string()
            } else {
                c.to_string()
            }
        })
        .collect()
}

fn has_attribute(member: &Member, name: &str) -> bool {
    member
        .extended_attributes
        .iter()
        .any(|attribute| attribute.name == name)
}

fn type_text(ty: &Type) -> String {
    let base = match &ty.kind {
        TypeKind::Any => "any".to_string(),
        TypeKind::Undefined => "undefined".to_string(),
        TypeKind::Boolean => "boolean".to_string(),
        TypeKind::Object => "object".to_string(),
        TypeKind::Integer { unsigned, size } => format!(
            "{}{}",
            if *unsigned { "unsigned " } else { "" },
            match size {
                crate::IntegerSize::Short => "short",
                crate::IntegerSize::Long => "long",
                crate::IntegerSize::LongLong => "long long",
            }
        ),
        TypeKind::Double { unrestricted } => {
            format!("{}double", if *unrestricted { "unrestricted " } else { "" })
        }
        TypeKind::DomString => "DOMString".to_string(),
        TypeKind::UsvString => "USVString".to_string(),
        TypeKind::Buffer(name) => name.clone(),
        TypeKind::Sequence(inner) => format!("sequence<{}>", type_text(inner)),
        TypeKind::Union(members) => format!(
            "({})",
            members
                .iter()
                .map(type_text)
                .collect::<Vec<_>>()
                .join(" or ")
        ),
        TypeKind::Named { name, .. } => name.clone(),
    };
    if ty.nullable {
        format!("{base}?")
    } else {
        base
    }
}

impl Generator<'_> {
    fn refuse(&self, row: &Row, what: String) -> SelectionError {
        SelectionError {
            file: self.selection.file.clone(),
            line: row.line,
            what: format!("`{} {}`: {what}", row.interface, row.member),
        }
    }

    /// Takes `rust` as the name of the member `label` of `row`; refused, naming both members,
    /// when another member of the selection is written with it. No second name is invented.
    fn claim(&mut self, row: &Row, label: &str, rust: String) -> Result<(), SelectionError> {
        if let Some(before) = self.names.get(&rust) {
            return Err(self.refuse(
                row,
                format!("the members `{before}` and `{label}` are both written `{rust}`"),
            ));
        }
        self.names.insert(rust, label.to_string());
        Ok(())
    }

    /// The parent of the interface `name` in the model.
    fn parent_of(&self, name: &str) -> Option<String> {
        self.model
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
            .flatten()
    }

    /// Whether `name` is `Node` or an interface the model makes inherit from it.
    fn is_node(&self, name: &str) -> bool {
        let mut at = Some(name.to_string());
        let mut steps = 0;
        while let Some(current) = at {
            if current == "Node" {
                return true;
            }
            steps += 1;
            if steps > 64 {
                return false;
            }
            at = self.parent_of(&current);
        }
        false
    }

    fn generated(&self, name: &str) -> Option<&Interface> {
        self.interfaces
            .iter()
            .find(|interface| interface.name == name)
    }

    /// The definitions, parents first, each with what `this` is for its members.
    fn order_interfaces(&mut self) -> Result<(), SelectionError> {
        let mut left: Vec<String> = self.selection.definitions.clone();
        while !left.is_empty() {
            let before = left.len();
            let mut index = 0;
            while index < left.len() {
                let name = left[index].clone();
                let parent = self.parent_of(&name);
                let ready = parent
                    .as_ref()
                    .is_none_or(|parent| self.generated(parent).is_some());
                if ready {
                    let has_getter = self.model.members(&name).iter().any(|member| {
                        matches!(member.kind, MemberKind::Operation { getter: true, .. })
                    });
                    let this = if name == self.selection.global {
                        This::Global
                    } else if name == "DOMException" {
                        This::Exception
                    } else if self.is_node(&name) {
                        This::Node
                    } else if has_getter {
                        This::Collection
                    } else {
                        This::Brand
                    };
                    self.interfaces.push(Interface {
                        variant: camel(&name),
                        name,
                        parent,
                        this,
                    });
                    left.remove(index);
                } else {
                    index += 1;
                }
            }
            if left.len() == before {
                return Err(SelectionError {
                    file: self.selection.file.clone(),
                    line: self.selection.definitions_line,
                    what: format!("no order of the definitions puts each parent first: {left:?}"),
                });
            }
        }
        Ok(())
    }

    /// The member a row names.
    fn member_of(&self, row: &Row) -> Option<&Member> {
        self.model
            .members(&row.interface)
            .into_iter()
            .find(|member| match &member.kind {
                MemberKind::Constructor { .. } => row.is_constructor(),
                MemberKind::Iterable { .. } => row.is_iterable(),
                MemberKind::Const { .. }
                | MemberKind::Attribute { .. }
                | MemberKind::Operation { .. }
                | MemberKind::Stringifier => member.name.as_deref() == Some(row.member.as_str()),
            })
    }

    /// The conversion of an argument or a setter's value of type `ty`.
    fn conversion(&mut self, row: &Row, ty: &Type) -> Result<Conversion, SelectionError> {
        let legacy = ty
            .extended_attributes
            .iter()
            .any(|attribute| attribute.name == "LegacyNullToEmptyString");
        let other: Vec<&str> = ty
            .extended_attributes
            .iter()
            .map(|attribute| attribute.name.as_str())
            .filter(|name| *name != "LegacyNullToEmptyString")
            .collect();
        if !other.is_empty() {
            return Err(self.refuse(row, format!("an extended attribute on a type: {other:?}")));
        }
        let refuse = |generator: &Self| {
            generator.refuse(
                row,
                format!("a type the generator does not convert: {}", type_text(ty)),
            )
        };
        Ok(match (&ty.kind, ty.nullable, legacy) {
            (TypeKind::DomString, false, false) => Conversion::DomString,
            (TypeKind::DomString, true, false) => Conversion::NullableDomString,
            (TypeKind::DomString, false, true) => Conversion::LegacyNullString,
            (TypeKind::Boolean, false, false) => Conversion::Boolean,
            (
                TypeKind::Integer {
                    unsigned: true,
                    size: crate::IntegerSize::Long,
                },
                false,
                false,
            ) => Conversion::Unsigned { bits: 32 },
            (
                TypeKind::Integer {
                    unsigned: true,
                    size: crate::IntegerSize::Short,
                },
                false,
                false,
            ) => Conversion::Unsigned { bits: 16 },
            (TypeKind::Named { name, .. }, nullable, false) if self.generated(name).is_some() => {
                if !self.is_node(name) {
                    return Err(refuse(self));
                }
                Conversion::Node {
                    interface: camel(name),
                    nullable,
                }
            }
            (TypeKind::Named { name, .. }, false, false) if self.is_dictionary(name) => {
                self.dictionary(row, name)?;
                Conversion::Dictionary(name.clone())
            }
            (TypeKind::Union(members), false, false) => {
                let names: Vec<String> = members.iter().map(type_text).collect();
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                match names.as_slice() {
                    ["TrustedType", "DOMString"] => Conversion::DomString,
                    ["Node", "DOMString"] => Conversion::NodesOrStrings,
                    ["DOMString", dictionary] if self.is_dictionary(dictionary) => {
                        self.dictionary(row, dictionary)?;
                        let union = Conversion::StringOrDictionary(dictionary.to_string());
                        self.unions.insert(union.clone());
                        union
                    }
                    ["boolean", dictionary] if self.is_dictionary(dictionary) => {
                        self.dictionary(row, dictionary)?;
                        let union = Conversion::BooleanOrDictionary(dictionary.to_string());
                        self.unions.insert(union.clone());
                        union
                    }
                    _ => return Err(refuse(self)),
                }
            }
            _ => return Err(refuse(self)),
        })
    }

    fn is_dictionary(&self, name: &str) -> bool {
        self.model.definitions().iter().any(|definition| {
            definition.name == name && matches!(definition.kind, DefinitionKind::Dictionary { .. })
        })
    }

    /// Records the dictionary `name` with the members it converts, sorted as Web IDL orders
    /// them (inherited first, then by name).
    fn dictionary(&mut self, row: &Row, name: &str) -> Result<(), SelectionError> {
        if self.dictionaries.contains_key(name) {
            return Ok(());
        }
        let Some(DefinitionKind::Dictionary { inherits, members }) = self
            .model
            .definitions()
            .iter()
            .find(|definition| definition.name == name)
            .map(|definition| &definition.kind)
        else {
            return Err(self.refuse(row, format!("no dictionary `{name}`")));
        };
        if inherits.is_some() {
            return Err(self.refuse(row, format!("the dictionary `{name}` inherits")));
        }
        let mut sorted: Vec<_> = members.iter().collect();
        sorted.sort_by(|a, b| a.name.cmp(&b.name));
        let mut fields = Vec::new();
        for member in sorted {
            let kind = match (&member.ty.kind, member.ty.nullable, &member.default) {
                (TypeKind::Boolean, false, None) => FieldKind::Boolean(None),
                (TypeKind::Boolean, false, Some(DefaultValue::Boolean(value))) => {
                    FieldKind::Boolean(Some(*value))
                }
                (TypeKind::DomString, false, None) => FieldKind::DomString(None),
                (TypeKind::DomString, false, Some(DefaultValue::String(value))) => {
                    FieldKind::DomString(Some(value.clone()))
                }
                (
                    TypeKind::Named {
                        name: interface, ..
                    },
                    nullable,
                    None,
                ) if self.generated(interface).is_none() && !self.is_dictionary(interface) => {
                    FieldKind::Absent {
                        interface: interface.clone(),
                        nullable,
                    }
                }
                _ => {
                    return Err(self.refuse(
                        row,
                        format!(
                            "a member of `{name}` the generator does not convert: {} {}",
                            type_text(&member.ty),
                            member.name
                        ),
                    ));
                }
            };
            if member.required {
                return Err(self.refuse(row, format!("a required member of `{name}`")));
            }
            fields.push(DictionaryField {
                name: member.name.clone(),
                kind,
            });
        }
        self.dictionaries.insert(
            name.to_string(),
            Dictionary {
                name: name.to_string(),
                members: fields,
            },
        );
        Ok(())
    }

    /// How a value of `ty` is returned.
    fn ret(&self, row: &Row, ty: &Type) -> Result<Return, SelectionError> {
        let refuse = || {
            self.refuse(
                row,
                format!(
                    "a return type the generator does not convert: {}",
                    type_text(ty)
                ),
            )
        };
        Ok(match (&ty.kind, ty.nullable) {
            (TypeKind::Undefined, false) => Return::Undefined,
            (TypeKind::Boolean, false) => Return::Boolean,
            (
                TypeKind::Integer {
                    unsigned: true,
                    size: crate::IntegerSize::Short,
                },
                false,
            ) => Return::Unsigned16,
            (
                TypeKind::Integer {
                    unsigned: true,
                    size: crate::IntegerSize::Long,
                },
                false,
            ) => Return::Unsigned32,
            (TypeKind::DomString | TypeKind::UsvString, false) => Return::String,
            (TypeKind::DomString | TypeKind::UsvString, true) => Return::NullableString,
            (TypeKind::Sequence(inner), false)
                if matches!(inner.kind, TypeKind::DomString) && !inner.nullable =>
            {
                Return::Strings
            }
            (TypeKind::Named { name, .. }, nullable) if name == "WindowProxy" && !nullable => {
                Return::Window
            }
            // `(Event or undefined)` of `Window.event`: an interface the bindings do not
            // generate is never returned, so the member answers `undefined`.
            (TypeKind::Union(members), false)
                if members.len() == 2
                    && matches!(members[1].kind, TypeKind::Undefined)
                    && matches!(&members[0].kind, TypeKind::Named { name, .. }
                        if self.generated(name).is_none() && !self.is_node(name))
                    && !members[0].nullable =>
            {
                Return::Undefined
            }
            (TypeKind::Named { name, .. }, nullable)
                if self.is_node(name) || NODE_TYPES_OUTSIDE_MODEL.contains(&name.as_str()) =>
            {
                if nullable {
                    Return::NullableNode
                } else {
                    Return::Node
                }
            }
            (TypeKind::Named { name, .. }, false)
                if self
                    .generated(name)
                    .is_some_and(|interface| interface.this == This::Collection) =>
            {
                Return::Collection(camel(name))
            }
            _ => return Err(refuse()),
        })
    }

    fn args(&mut self, row: &Row, arguments: &[Argument]) -> Result<Vec<Arg>, SelectionError> {
        let mut out: Vec<Arg> = Vec::new();
        for (index, argument) in arguments.iter().enumerate() {
            let name = identifier(&argument.name);
            if let Some(before) = arguments[..index]
                .iter()
                .find(|before| identifier(&before.name) == name)
            {
                return Err(self.refuse(
                    row,
                    format!(
                        "the arguments `{}` and `{}` are both written `{name}`",
                        before.name, argument.name
                    ),
                ));
            }
            let conversion = self.conversion(row, &argument.ty)?;
            if argument.variadic != (conversion == Conversion::NodesOrStrings) {
                return Err(self.refuse(
                    row,
                    format!(
                        "a variadic argument the generator does not convert: {}",
                        argument.name
                    ),
                ));
            }
            out.push(Arg {
                name,
                conversion,
                optional: argument.optional,
                default: argument.default.clone(),
                variadic: argument.variadic,
            });
        }
        Ok(out)
    }

    /// The host functions and the table entries of one interface.
    fn interface(&mut self, index: usize) -> Result<(), SelectionError> {
        let name = self.interfaces[index].name.clone();
        let variant = self.interfaces[index].variant.clone();
        let this = self.interfaces[index].this;
        let rows: Vec<Row> = self.selection.rows_of(&name).cloned().collect();
        let mut members = String::new();
        let mut unscopables = Vec::new();
        // Web IDL 3.7.9: an interface with an indexed property getter (`getter T
        // name(unsigned long index)`) gets `@@iterator`, `%Array.prototype.values%`. It follows
        // from the getter, so no row names it.
        let indexed = self.model.members(&name).iter().any(|member| {
            matches!(&member.kind, MemberKind::Operation { getter: true, arguments, .. }
            if arguments.first().is_some_and(|index| !index.ty.nullable
                && matches!(index.ty.kind, TypeKind::Integer {
                    unsigned: true,
                    size: crate::IntegerSize::Long,
                })))
        });
        let mut iterable = false;
        let mut steps = format!("interface_{}", snake(&name));
        let mut length = 0;
        let mut constructor = false;
        for row in &rows {
            let Some(member) = self.member_of(row).cloned() else {
                return Err(self.refuse(row, "the model holds no such member".to_string()));
            };
            if has_attribute(&member, "Unscopable") {
                unscopables.push(row.member.clone());
            }
            let unforgeable = has_attribute(&member, "LegacyUnforgeable");
            if unforgeable && this != This::Global {
                return Err(self.refuse(
                    row,
                    "`[LegacyUnforgeable]` off the global interface: the bindings put such a \
                     member on the global object only, and Web IDL puts it on each instance"
                        .to_string(),
                ));
            }
            let label = format!("{name}.{}", row.member);
            match &member.kind {
                MemberKind::Constructor { arguments } => {
                    constructor = true;
                    length = arguments
                        .iter()
                        .filter(|argument| !argument.optional && !argument.variadic)
                        .count();
                    match row.slice {
                        Slice::Throws => {
                            let reason = row.reason().unwrap_or_default();
                            let _ = writeln!(
                                self.functions,
                                "fn interface_{}(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {{ throwing_constructor(cx, call, {name:?}, {reason:?}) }}",
                                snake(&name)
                            );
                        }
                        Slice::Exception => {
                            if name != "DOMException" {
                                return Err(self.refuse(
                                    row,
                                    "a constructor of exception off `DOMException`".to_string(),
                                ));
                            }
                            steps = "dom_exception_constructor".to_string();
                        }
                        Slice::Reading | Slice::Attributes | Slice::Mutation | Slice::Selectors => {
                            let args = self.args(row, arguments)?;
                            let method = Method {
                                name: format!("{}_constructor", snake(&name)),
                                member: format!("{name} constructor"),
                                doc: format!("`new {name}(...)`"),
                                this: This::Global,
                                args,
                                ret: Return::Node,
                                setter: false,
                            };
                            let body = self.member_function(
                                &format!("interface_{}", snake(&name)),
                                &label,
                                &variant,
                                This::Global,
                                row.slice,
                                &method,
                                true,
                            );
                            self.functions.push_str(&body);
                            self.methods.entry(row.slice).or_default().push(method);
                        }
                    }
                }
                MemberKind::Iterable { .. } => {
                    if !indexed {
                        return Err(
                            self.refuse(row, "`iterable<T>` without an indexed getter".to_string())
                        );
                    }
                    iterable = true;
                }
                MemberKind::Const { value, .. } => {
                    let value = match value {
                        ConstValue::Integer(value) => *value,
                        ConstValue::Boolean(_) => {
                            return Err(
                                self.refuse(row, "a constant that is no integer".to_string())
                            );
                        }
                    };
                    let _ = writeln!(
                        members,
                        "MemberSpec::Constant {{ name: {:?}, value: {value}.0 }},",
                        row.member
                    );
                }
                MemberKind::Attribute { ty, readonly, .. } => {
                    let base = format!("{}_{}", snake(&name), snake(&row.member));
                    self.claim(row, &label, base.clone())?;
                    if !readonly {
                        self.claim(row, &label, format!("set_{base}"))?;
                    }
                    let getter = format!("getter_{}_{}", snake(&name), snake(&row.member));
                    let replaceable = has_attribute(&member, "Replaceable");
                    let forwards = member
                        .extended_attributes
                        .iter()
                        .any(|attribute| attribute.name == "PutForwards");
                    let has_setter = !readonly || replaceable || forwards;
                    let setter = format!("setter_{}_{}", snake(&name), snake(&row.member));
                    match row.slice {
                        Slice::Throws => {
                            self.throwing(&getter, &label, &variant, this, row);
                            if has_setter {
                                self.throwing(&setter, &label, &variant, this, row);
                            }
                        }
                        Slice::Exception => {
                            if name != "DOMException" {
                                return Err(self.refuse(
                                    row,
                                    "an attribute of exception off `DOMException`".to_string(),
                                ));
                            }
                            let function = match row.member.as_str() {
                                "name" => "dom_exception_name",
                                "message" => "dom_exception_message",
                                "code" => "dom_exception_code",
                                _ => {
                                    return Err(self.refuse(row, "an attribute of `DOMException` the generator does not know".to_string()));
                                }
                            };
                            let _ = writeln!(
                                self.functions,
                                "fn {getter}(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {{ {function}(cx, call) }}"
                            );
                        }
                        Slice::Reading | Slice::Attributes | Slice::Mutation | Slice::Selectors => {
                            let ret = self.ret(row, ty)?;
                            let method = Method {
                                name: format!("{}_{}", snake(&name), snake(&row.member)),
                                member: label.clone(),
                                doc: format!("The getter of `{label}`, `{}`", type_text(ty)),
                                this,
                                args: Vec::new(),
                                ret,
                                setter: false,
                            };
                            let body = self.member_function(
                                &getter, &label, &variant, this, row.slice, &method, false,
                            );
                            self.functions.push_str(&body);
                            self.methods.entry(row.slice).or_default().push(method);
                            if replaceable {
                                if this != This::Global {
                                    return Err(self.refuse(
                                        row,
                                        "`[Replaceable]` off the global interface".to_string(),
                                    ));
                                }
                                let _ = writeln!(
                                    self.functions,
                                    "fn {setter}(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {{ cx.charge_steps(1)?; replace_on_global(cx, &call, {:?}) }}",
                                    row.member
                                );
                            } else if forwards {
                                return Err(self.refuse(
                                    row,
                                    "`[PutForwards]` off a member of throws".to_string(),
                                ));
                            } else if !readonly {
                                let conversion = self.conversion(row, ty)?;
                                let setter_slice = row.setter_slice();
                                let method = Method {
                                    name: format!("set_{}_{}", snake(&name), snake(&row.member)),
                                    member: label.clone(),
                                    doc: format!("The setter of `{label}`, `{}`", type_text(ty)),
                                    this,
                                    args: vec![Arg {
                                        name: "value".to_string(),
                                        conversion,
                                        optional: false,
                                        default: None,
                                        variadic: false,
                                    }],
                                    ret: Return::Undefined,
                                    setter: true,
                                };
                                let body = self.member_function(
                                    &setter,
                                    &label,
                                    &variant,
                                    this,
                                    setter_slice,
                                    &method,
                                    false,
                                );
                                self.functions.push_str(&body);
                                self.methods.entry(setter_slice).or_default().push(method);
                            }
                        }
                    }
                    let setter_value = if has_setter {
                        format!("Some({setter})")
                    } else {
                        "None".to_string()
                    };
                    let _ = writeln!(
                        members,
                        "MemberSpec::Attribute {{ name: {:?}, getter: {getter}, setter: {setter_value}, unforgeable: {unforgeable} }},",
                        row.member
                    );
                }
                MemberKind::Operation {
                    return_type,
                    arguments,
                    is_static,
                    ..
                } => {
                    if *is_static {
                        return Err(self.refuse(row, "a static operation".to_string()));
                    }
                    self.claim(
                        row,
                        &label,
                        format!("{}_{}", snake(&name), snake(&row.member)),
                    )?;
                    let function = format!("operation_{}_{}", snake(&name), snake(&row.member));
                    let length = arguments
                        .iter()
                        .filter(|argument| !argument.optional && !argument.variadic)
                        .count();
                    match row.slice {
                        Slice::Throws => self.throwing(&function, &label, &variant, this, row),
                        Slice::Exception => {
                            return Err(self.refuse(row, "an operation of exception".to_string()));
                        }
                        Slice::Reading | Slice::Attributes | Slice::Mutation | Slice::Selectors => {
                            let args = self.args(row, arguments)?;
                            let ret = self.ret(row, return_type)?;
                            let signature = arguments
                                .iter()
                                .map(|argument| {
                                    format!(
                                        "{}{}{} {}",
                                        if argument.optional { "optional " } else { "" },
                                        type_text(&argument.ty),
                                        if argument.variadic { "..." } else { "" },
                                        argument.name
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(", ");
                            let method = Method {
                                name: format!("{}_{}", snake(&name), snake(&row.member)),
                                member: label.clone(),
                                doc: format!(
                                    "`{label}`: `{} {}({signature})`",
                                    type_text(return_type),
                                    row.member
                                ),
                                this,
                                args,
                                ret,
                                setter: false,
                            };
                            let body = self.member_function(
                                &function, &label, &variant, this, row.slice, &method, false,
                            );
                            self.functions.push_str(&body);
                            self.methods.entry(row.slice).or_default().push(method);
                        }
                    }
                    let _ = writeln!(
                        members,
                        "MemberSpec::Operation {{ name: {:?}, steps: {function}, length: {length}, unforgeable: {unforgeable} }},",
                        row.member
                    );
                }
                MemberKind::Stringifier => {
                    return Err(self.refuse(row, "a stringifier".to_string()));
                }
            }
        }
        if !constructor {
            let _ = writeln!(
                self.functions,
                "fn interface_{}(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {{ no_constructor(cx, call, {name:?}) }}",
                snake(&name)
            );
        }
        let unscopables = unscopables
            .iter()
            .map(|name| format!("{name:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            self.specs,
            "InterfaceSpec {{ interface: Interface::{variant}, steps: {steps}, length: {length}, members: &[{members}], unscopables: &[{unscopables}], indexed: {indexed}, iterable: {iterable} }},"
        );
        Ok(())
    }

    /// A host function of a member of `throws`: the brand check, then the throw.
    fn throwing(&mut self, function: &str, label: &str, variant: &str, this: This, row: &Row) {
        let reason = row.reason().unwrap_or_default();
        let check = match this {
            This::Global => format!("this_global(cx, call.this, {label:?})?;"),
            This::Node | This::Collection | This::Exception | This::Brand => {
                format!("this_object(cx, call.this, Interface::{variant}, {label:?})?;")
            }
        };
        let _ = writeln!(
            self.functions,
            "/// `{label}`, of `throws` ({}): the brand check of `this`, then `NotSupportedError`.\n/// Web IDL converts the arguments first; here no argument is converted, so a call with an\n/// argument whose conversion throws gets `NotSupportedError`, not that throw.\nfn {function}(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {{ cx.charge_steps(1)?; {check} throws(cx, {label:?}, {reason:?}) }}",
            doc_text(reason)
        );
    }

    /// A host function of a member of a slice with a trait: the brand check of `this`, the
    /// conversion of every argument, then, the bindings out of the host, the node of `this` and
    /// of each node argument, the copy of every String, the trait's method and the return.
    #[allow(clippy::too_many_arguments)]
    fn member_function(
        &self,
        function: &str,
        label: &str,
        variant: &str,
        this: This,
        slice: Slice,
        method: &Method,
        constructor: bool,
    ) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "fn {function}(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {{"
        );
        let _ = writeln!(out, "const MEMBER: &str = {label:?};");
        let _ = writeln!(out, "cx.charge_steps(1)?;");
        if constructor {
            let _ = writeln!(
                out,
                "if call.new_target.is_none() {{ return Err(type_error(cx, &format!(\"{{MEMBER}}: called without new\"))); }}"
            );
        } else {
            match this {
                This::Node => {
                    let _ = writeln!(
                        out,
                        "let this = this_object(cx, call.this, Interface::{variant}, MEMBER)?;"
                    );
                }
                This::Collection => {
                    let _ = writeln!(
                        out,
                        "let this = this_collection(cx, call.this, Interface::{variant}, MEMBER)?;"
                    );
                }
                This::Global => {
                    let _ = writeln!(out, "this_global(cx, call.this, MEMBER)?;");
                }
                This::Exception | This::Brand => {
                    let _ = writeln!(
                        out,
                        "this_object(cx, call.this, Interface::{variant}, MEMBER)?;"
                    );
                }
            }
        }
        let required = method
            .args
            .iter()
            .filter(|arg| !arg.optional && !arg.variadic)
            .count();
        if required > 0 && !method.setter {
            let _ = writeln!(out, "required(cx, &call, {required}, MEMBER)?;");
        }
        for (index, arg) in method.args.iter().enumerate() {
            let convert = self.convert_expression(arg, index);
            if arg.variadic {
                let _ = writeln!(
                    out,
                    "let a{index} = to_nodes_or_strings(cx, &call, {index})?;"
                );
            } else if arg.optional && !always_converted(arg) {
                let _ = writeln!(
                    out,
                    "let a{index} = if present(&call, {index}) {{ Some({convert}) }} else {{ None }};"
                );
            } else {
                let _ = writeln!(out, "let a{index} = {convert};");
            }
        }
        let _ = writeln!(out, "in_dom(cx, |dcx, slices| {{");
        let mut finishing = String::new();
        let mut call_args = Vec::new();
        if !constructor {
            match this {
                This::Node => {
                    let _ = writeln!(
                        finishing,
                        "let this = unwrap_node(dcx, slices, this, MEMBER)?;"
                    );
                    call_args.push("this".to_string());
                }
                This::Collection => call_args.push("&this".to_string()),
                This::Global | This::Exception | This::Brand => {}
            }
        }
        for (index, arg) in method.args.iter().enumerate() {
            let finish = finish_expression(&arg.conversion, "raw");
            let identity = finish == "raw";
            let line = if arg.variadic {
                format!(
                    "let a{index} = finish_nodes_or_strings(dcx, slices, &mut copies, a{index}, MEMBER)?;"
                )
            } else if arg.optional && !always_converted(arg) {
                let default = default_expression(arg);
                match (identity, &arg.default) {
                    (true, None) => String::new(),
                    (true, Some(DefaultValue::Boolean(false))) => {
                        format!("let a{index} = a{index}.unwrap_or_default();")
                    }
                    (true, Some(_)) => format!("let a{index} = a{index}.unwrap_or({default});"),
                    (false, _) => format!(
                        "let a{index} = match a{index} {{ Some(raw) => {}, None => {default} }};",
                        wrap_some(arg, &finish)
                    ),
                }
            } else if identity {
                String::new()
            } else {
                format!(
                    "let a{index} = {};",
                    finish_expression(&arg.conversion, &format!("a{index}"))
                )
            };
            if !line.is_empty() {
                let _ = writeln!(finishing, "{line}");
            }
            call_args.push(format!("a{index}"));
        }
        let copies = if finishing.contains("copies") {
            "let mut copies = Copies::new();"
        } else {
            "let copies = Copies::new();"
        };
        let _ = writeln!(out, "{copies}");
        out.push_str(&finishing);
        let field = match slice {
            Slice::Reading => "reading",
            Slice::Attributes => "attributes",
            Slice::Mutation => "mutation",
            Slice::Selectors => "selectors",
            Slice::Throws | Slice::Exception => "reading",
        };
        let mut all = vec!["dcx".to_string()];
        all.extend(call_args);
        let _ = writeln!(
            out,
            "let answer = slices.{field}.{}({});",
            method.name,
            all.join(", ")
        );
        let _ = writeln!(out, "drop(copies);");
        let _ = writeln!(
            out,
            "finish(dcx, slices, answer, {})",
            method.ret.converter()
        );
        let _ = writeln!(out, "}})");
        let _ = writeln!(out, "}}");
        out
    }

    fn convert_expression(&self, arg: &Arg, index: usize) -> String {
        let value = format!("argument(&call, {index})");
        match &arg.conversion {
            Conversion::DomString => format!("to_string(cx, {value})?"),
            Conversion::NullableDomString => format!("to_nullable_string(cx, {value})?"),
            Conversion::LegacyNullString => format!("to_legacy_null_string(cx, {value})?"),
            Conversion::Boolean => format!("to_boolean(cx, {value})?"),
            Conversion::Unsigned { bits } => format!("to_unsigned(cx, {value}, {bits})?"),
            Conversion::Node {
                interface,
                nullable: false,
            } => format!("to_node(cx, {value}, Interface::{interface}, MEMBER, {index})?"),
            Conversion::Node {
                interface,
                nullable: true,
            } => format!("to_nullable_node(cx, {value}, Interface::{interface}, MEMBER, {index})?"),
            Conversion::NodesOrStrings => format!("to_nodes_or_strings(cx, &call, {index})?"),
            Conversion::Dictionary(name) => {
                format!("convert_{}(cx, {value}, MEMBER)?", snake(name))
            }
            Conversion::StringOrDictionary(name) => format!(
                "if union_takes_dictionary({value}) {{ Raw{}::Dictionary(convert_{}(cx, {value}, MEMBER)?) }} else {{ Raw{}::String(to_string(cx, {value})?) }}",
                union_name(&arg.conversion),
                snake(name),
                union_name(&arg.conversion)
            ),
            Conversion::BooleanOrDictionary(name) => format!(
                "if union_takes_dictionary({value}) {{ Raw{}::Dictionary(convert_{}(cx, {value}, MEMBER)?) }} else {{ Raw{}::Boolean(to_boolean(cx, {value})?) }}",
                union_name(&arg.conversion),
                snake(name),
                union_name(&arg.conversion)
            ),
        }
    }

    /// The text of the file.
    fn write(&self, digests: &[Digest], reads: &[ReadText]) -> String {
        let mut out = String::new();
        out.push_str(NOTICE);
        out.push('\n');
        let definitions = self.selection.definitions.join(", ");
        // The fence of the digests' block, written so that no line of this file opens one.
        let fence = "`".repeat(3);
        let _ = writeln!(
            out,
            "//! The DOM's interfaces for the script engine swell, generated from the Web IDL of the
//! vendored DOM, Web IDL and HTML Standards (`crates/webidl/data/whatwg/`, `SOURCE`) and the
//! selection `crates/webidl/selection.txt`: {definitions}. A portion of the standards
//! incorporated into source code, under the BSD 3-Clause License above (WHATWG IPR policy,
//! <https://whatwg.org/ipr-policy>).
//!
//! Written by `cargo run -p surf-webidl --bin bindings` (`story:webidl-bindings`). Never
//! edited by hand: a member enters or leaves by a change of the selection, and gets
//! behaviour in the hand-written slice behind its trait (`crate::dom`). `task webidl-check`
//! recomputes the sha256 of each source below and of this whole file (its line at the end of
//! this header), and compares; `task webidl-slow` writes the file again and compares the bytes.
//!
//! The sha256 of each source, its path from the repository root:
//!
//! {fence}text"
        );
        for digest in digests {
            let _ = writeln!(out, "//! {}  {}", digest.sha256, digest.path);
        }
        let _ = writeln!(out, "//! {fence}");
        if !reads.is_empty() {
            let _ = writeln!(
                out,
                "//!\n//! What the generator read of each page through `surf-html`: the FNV-1a (64 bits) of the\n//! Web IDL text, its bytes and the page's path (a test of `surf-webidl` reads them again):\n//!\n//! {fence}text"
            );
            for read in reads {
                let _ = writeln!(out, "//! {}", read.line());
            }
            let _ = writeln!(out, "//! {fence}");
        }
        let _ = writeln!(out, "//!");
        let _ = writeln!(out, "{FILE_DIGEST_LINE}{}", "0".repeat(64));
        out.push_str(crate::prelude::PRELUDE);
        out.push('\n');
        self.write_interfaces(&mut out);
        self.write_dictionaries(&mut out);
        self.write_traits(&mut out);
        out.push_str(&self.functions);
        let _ = writeln!(
            out,
            "/// The interfaces, parents first, for the realm's setup.\nstatic INTERFACES: [InterfaceSpec; {}] = [{}];",
            self.interfaces.len(),
            self.specs
        );
        out
    }

    fn write_interfaces(&self, out: &mut String) {
        let _ = writeln!(
            out,
            "/// The generated interfaces, parents first. Each is the brand of the objects that \
             implement it, `Brand(index + 1)`.\n#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub enum Interface {{"
        );
        for interface in &self.interfaces {
            let _ = writeln!(out, "/// `{}`.\n{},", interface.name, interface.variant);
        }
        let _ = writeln!(out, "}}");
        let all = self
            .interfaces
            .iter()
            .map(|interface| format!("Interface::{}", interface.variant))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(out, "impl Interface {{");
        let _ = writeln!(
            out,
            "/// Every interface, parents first.\npub const ALL: [Interface; {}] = [{all}];",
            self.interfaces.len()
        );
        let _ = writeln!(
            out,
            "/// Its name.\npub fn name(self) -> &'static str {{ match self {{"
        );
        for interface in &self.interfaces {
            let _ = writeln!(
                out,
                "Interface::{} => {:?},",
                interface.variant, interface.name
            );
        }
        let _ = writeln!(out, "}} }}");
        let _ = writeln!(
            out,
            "/// Its place in [`Interface::ALL`].\npub fn index(self) -> usize {{ match self {{"
        );
        for (index, interface) in self.interfaces.iter().enumerate() {
            let _ = writeln!(out, "Interface::{} => {index},", interface.variant);
        }
        let _ = writeln!(out, "}} }}");
        let _ = writeln!(
            out,
            "/// The brand of its objects.\npub fn brand(self) -> Brand {{ match self {{"
        );
        for (index, interface) in self.interfaces.iter().enumerate() {
            let _ = writeln!(
                out,
                "Interface::{} => Brand({}),",
                interface.variant,
                index + 1
            );
        }
        let _ = writeln!(out, "}} }}");
        let _ = writeln!(
            out,
            "/// The interface of `brand`, if it is one of the generated brands.\npub fn of_brand(brand: Brand) -> Option<Interface> {{ match brand {{"
        );
        for (index, interface) in self.interfaces.iter().enumerate() {
            let _ = writeln!(
                out,
                "Brand({}) => Some(Interface::{}),",
                index + 1,
                interface.variant
            );
        }
        let _ = writeln!(out, "Brand(_) => None,\n}} }}");
        let _ = writeln!(
            out,
            "/// The interface it inherits from.\npub fn parent(self) -> Option<Interface> {{ match self {{"
        );
        for interface in &self.interfaces {
            let parent = match &interface.parent {
                Some(parent) => format!("Some(Interface::{})", camel(parent)),
                None => "None".to_string(),
            };
            let _ = writeln!(out, "Interface::{} => {parent},", interface.variant);
        }
        let _ = writeln!(out, "}} }}");
        let _ = writeln!(
            out,
            "/// The generated table of which brand inherits which members: whether an object \
             of this interface's brand implements `other` (is it, or inherits from it).\n\
             pub fn implements(self, other: Interface) -> bool {{ match self {{"
        );
        for interface in &self.interfaces {
            let mut chain = vec![format!("Interface::{}", interface.variant)];
            let mut at = interface.parent.clone();
            while let Some(parent) = at {
                chain.push(format!("Interface::{}", camel(&parent)));
                at = self.generated(&parent).and_then(|p| p.parent.clone());
            }
            let _ = writeln!(
                out,
                "Interface::{} => matches!(other, {}),",
                interface.variant,
                chain.join(" | ")
            );
        }
        let _ = writeln!(out, "}} }}");
        let _ = writeln!(out, "}}");
        let global = camel(&self.selection.global);
        let _ = writeln!(
            out,
            "/// The interface of the global object.\nconst GLOBAL: Interface = Interface::{global};"
        );
        let constants: BTreeMap<String, i64> = self
            .model
            .members("DOMException")
            .into_iter()
            .filter_map(|member| match (&member.kind, &member.name) {
                (
                    MemberKind::Const {
                        value: ConstValue::Integer(value),
                        ..
                    },
                    Some(name),
                ) => Some((name.clone(), *value)),
                _ => None,
            })
            .collect();
        let _ = writeln!(
            out,
            "/// The legacy code of an error name (Web IDL's table of error names), 0 for a name \
             without one.\nfn legacy_code(name: &str) -> u16 {{ match name {{"
        );
        for (name, constant) in ERROR_NAMES {
            if let Some(value) = constants.get(*constant) {
                let _ = writeln!(out, "{name:?} => {value},");
            }
        }
        let _ = writeln!(out, "_ => 0,\n}} }}");
    }

    fn write_dictionaries(&self, out: &mut String) {
        for dictionary in self.dictionaries.values() {
            let rust = camel(&dictionary.name);
            let _ = writeln!(
                out,
                "/// The dictionary `{}` converted; a member of an interface type the bindings do not generate is held only as absent.\n#[derive(Clone, Debug, PartialEq, Eq)]\npub struct {rust} {{",
                dictionary.name
            );
            for field in &dictionary.members {
                let ty = match &field.kind {
                    FieldKind::Boolean(Some(_)) => "bool",
                    FieldKind::Boolean(None) => "Option<bool>",
                    FieldKind::DomString(Some(_)) => "String",
                    FieldKind::DomString(None) => "Option<String>",
                    FieldKind::Absent { .. } => continue,
                };
                let _ = writeln!(
                    out,
                    "/// `{}`.\npub {}: {ty},",
                    field.name,
                    identifier(&field.name)
                );
            }
            let _ = writeln!(out, "}}");
            let _ = writeln!(out, "struct Raw{rust} {{");
            for field in &dictionary.members {
                let ty = match &field.kind {
                    FieldKind::Boolean(Some(_)) => "bool",
                    FieldKind::Boolean(None) => "Option<bool>",
                    FieldKind::DomString(_) => "Option<StringRef>",
                    FieldKind::Absent { .. } => continue,
                };
                let _ = writeln!(out, "{}: {ty},", identifier(&field.name));
            }
            let _ = writeln!(out, "}}");
            let _ = writeln!(
                out,
                "fn convert_{}(cx: &mut Cx<'_, ScriptHost>, value: Value, member: &str) -> Completion<Raw{rust}> {{",
                snake(&dictionary.name)
            );
            let _ = writeln!(out, "let object = to_dictionary(cx, value, member)?;");
            let mut fields = Vec::new();
            for field in &dictionary.members {
                let name = identifier(&field.name);
                let _ = writeln!(
                    out,
                    "let {name} = dictionary_member(cx, object, {:?})?;",
                    field.name
                );
                match &field.kind {
                    FieldKind::Boolean(Some(default)) => {
                        let _ = writeln!(
                            out,
                            "let {name} = if let Value::Undefined = {name} {{ {default} }} else {{ to_boolean(cx, {name})? }};"
                        );
                    }
                    FieldKind::Boolean(None) => {
                        let _ = writeln!(
                            out,
                            "let {name} = if let Value::Undefined = {name} {{ None }} else {{ Some(to_boolean(cx, {name})?) }};"
                        );
                    }
                    FieldKind::DomString(_) => {
                        let _ = writeln!(
                            out,
                            "let {name} = if let Value::Undefined = {name} {{ None }} else {{ Some(to_string(cx, {name})?) }};"
                        );
                    }
                    FieldKind::Absent {
                        interface,
                        nullable,
                    } => {
                        let _ = writeln!(
                            out,
                            "absent_member(cx, {name}, {nullable}, member, {interface:?})?;"
                        );
                        continue;
                    }
                }
                fields.push(name);
            }
            let _ = writeln!(out, "Ok(Raw{rust} {{ {} }})", fields.join(", "));
            let _ = writeln!(out, "}}");
            let _ = writeln!(
                out,
                "fn finish_{}(cx: &mut DomCx<'_, '_>, copies: &mut Copies, raw: Raw{rust}) -> Completion<{rust}> {{",
                snake(&dictionary.name)
            );
            let _ = writeln!(out, "let _ = (&cx, &copies);");
            let mut built = Vec::new();
            for field in &dictionary.members {
                let name = identifier(&field.name);
                match &field.kind {
                    FieldKind::Boolean(_) => built.push(format!("{name}: raw.{name}")),
                    FieldKind::DomString(Some(default)) => built.push(format!(
                        "{name}: match raw.{name} {{ Some(string) => copies.copy(cx.engine(), string)?, None => {default:?}.to_string() }}"
                    )),
                    FieldKind::DomString(None) => built.push(format!(
                        "{name}: match raw.{name} {{ Some(string) => Some(copies.copy(cx.engine(), string)?), None => None }}"
                    )),
                    FieldKind::Absent { .. } => {}
                }
            }
            let _ = writeln!(out, "Ok({rust} {{ {} }})", built.join(", "));
            let _ = writeln!(out, "}}");
        }
        for union in &self.unions {
            let name = union_name(union);
            let (first, first_raw, first_type, dictionary) = match union {
                Conversion::StringOrDictionary(dictionary) => {
                    ("String", "StringRef", "String", dictionary)
                }
                Conversion::BooleanOrDictionary(dictionary) => {
                    ("Boolean", "bool", "bool", dictionary)
                }
                _ => continue,
            };
            let rust = camel(dictionary);
            let _ = writeln!(
                out,
                "/// The union `({} or {dictionary})` converted.\n#[derive(Clone, Debug, PartialEq, Eq)]\npub enum {name} {{\n/// The {}.\n{first}({first_type}),\n/// The dictionary.\nDictionary({rust}),\n}}",
                if first == "String" {
                    "DOMString"
                } else {
                    "boolean"
                },
                first.to_lowercase()
            );
            let _ = writeln!(
                out,
                "enum Raw{name} {{ {first}({first_raw}), Dictionary(Raw{rust}) }}"
            );
            let first_finish = if first == "String" {
                "copies.copy(cx.engine(), raw)?"
            } else {
                "raw"
            };
            let _ = writeln!(
                out,
                "fn finish_{}(cx: &mut DomCx<'_, '_>, copies: &mut Copies, raw: Raw{name}) -> Completion<{name}> {{ Ok(match raw {{ Raw{name}::{first}(raw) => {name}::{first}({first_finish}), Raw{name}::Dictionary(raw) => {name}::Dictionary(finish_{}(cx, copies, raw)?) }}) }}",
                snake(&name),
                snake(dictionary)
            );
        }
    }

    fn write_traits(&self, out: &mut String) {
        for (slice, trait_name) in TRAITS {
            let _ = writeln!(
                out,
                "/// The members of the slice `{}`: each method's default answers `NotSupportedError` naming the member, until the slice's story writes it in `crate::dom::{}`.\npub trait {trait_name} {{",
                slice.word(),
                slice.word()
            );
            for method in self.methods.get(&slice).into_iter().flatten() {
                let mut params = vec![
                    "&mut self".to_string(),
                    "cx: &mut DomCx<'_, '_>".to_string(),
                ];
                let mut names = vec!["cx".to_string()];
                match method.this {
                    This::Node => {
                        params.push("this: NodeId".to_string());
                        names.push("this".to_string());
                    }
                    This::Collection => {
                        params.push("this: &[NodeId]".to_string());
                        names.push("this".to_string());
                    }
                    This::Global | This::Exception | This::Brand => {}
                }
                for arg in &method.args {
                    params.push(format!(
                        "{}: {}",
                        arg.name,
                        final_type(arg, &self.dictionaries)
                    ));
                    names.push(arg.name.clone());
                }
                let deviations = self
                    .selection
                    .rows
                    .iter()
                    .find(|row| format!("{}.{}", row.interface, row.member) == method.member)
                    .map(|row| row.deviations())
                    .unwrap_or_default();
                let mut doc = method.doc.clone();
                for deviation in deviations {
                    let _ = write!(doc, "; deviation: {}", doc_text(deviation));
                }
                let _ = writeln!(
                    out,
                    "/// {doc}.\nfn {}({}) -> DomResult<{}> {{ let _ = {}; Err(not_supported({:?})) }}",
                    method.name,
                    params.join(", "),
                    method.ret.rust(),
                    if names.len() == 1 {
                        names[0].clone()
                    } else {
                        format!("({})", names.join(", "))
                    },
                    method.member
                );
            }
            let _ = writeln!(out, "}}");
        }
    }
}

/// Whether an optional argument is converted also when it is absent: a dictionary or a union
/// with one whose default is `{}`, which is the conversion of `undefined`.
fn always_converted(arg: &Arg) -> bool {
    matches!(arg.default, Some(DefaultValue::EmptyDictionary))
        && matches!(
            arg.conversion,
            Conversion::Dictionary(_)
                | Conversion::StringOrDictionary(_)
                | Conversion::BooleanOrDictionary(_)
        )
}

fn union_name(conversion: &Conversion) -> String {
    match conversion {
        Conversion::StringOrDictionary(dictionary) => format!("StringOr{}", camel(dictionary)),
        Conversion::BooleanOrDictionary(dictionary) => format!("BooleanOr{}", camel(dictionary)),
        _ => String::new(),
    }
}

/// The Rust type a slice's method receives for `arg`.
fn final_type(arg: &Arg, dictionaries: &BTreeMap<String, Dictionary>) -> String {
    let _ = dictionaries;
    let base = match &arg.conversion {
        Conversion::DomString | Conversion::LegacyNullString => "String".to_string(),
        Conversion::NullableDomString => "Option<String>".to_string(),
        Conversion::Boolean => "bool".to_string(),
        Conversion::Unsigned { bits: 16 } => "u16".to_string(),
        Conversion::Unsigned { .. } => "u32".to_string(),
        Conversion::Node {
            nullable: false, ..
        } => "NodeId".to_string(),
        Conversion::Node { nullable: true, .. } => "Option<NodeId>".to_string(),
        Conversion::NodesOrStrings => "Vec<NodeOrString>".to_string(),
        Conversion::Dictionary(name) => camel(name),
        Conversion::StringOrDictionary(_) | Conversion::BooleanOrDictionary(_) => {
            union_name(&arg.conversion)
        }
    };
    if arg.optional && arg.default.is_none() && !arg.variadic {
        format!("Option<{base}>")
    } else {
        base
    }
}

/// The expression that turns the converted value `raw` into what the slice receives.
fn finish_expression(conversion: &Conversion, raw: &str) -> String {
    match conversion {
        Conversion::DomString => format!("copies.copy(dcx.engine(), {raw})?"),
        Conversion::NullableDomString => format!(
            "match {raw} {{ Some(string) => Some(copies.copy(dcx.engine(), string)?), None => None }}"
        ),
        Conversion::LegacyNullString => format!(
            "match {raw} {{ Some(string) => copies.copy(dcx.engine(), string)?, None => String::new() }}"
        ),
        Conversion::Boolean => raw.to_string(),
        Conversion::Unsigned { bits: 16 } => format!("u16::try_from({raw}).unwrap_or(u16::MAX)"),
        Conversion::Unsigned { .. } => raw.to_string(),
        Conversion::Node {
            nullable: false, ..
        } => format!("unwrap_node(dcx, slices, {raw}, MEMBER)?"),
        Conversion::Node { nullable: true, .. } => format!(
            "match {raw} {{ Some(object) => Some(unwrap_node(dcx, slices, object, MEMBER)?), None => None }}"
        ),
        Conversion::NodesOrStrings => {
            format!("finish_nodes_or_strings(dcx, slices, &mut copies, {raw}, MEMBER)?")
        }
        Conversion::Dictionary(name) => {
            format!("finish_{}(dcx, &mut copies, {raw})?", snake(name))
        }
        Conversion::StringOrDictionary(_) | Conversion::BooleanOrDictionary(_) => format!(
            "finish_{}(dcx, &mut copies, {raw})?",
            snake(&union_name(conversion))
        ),
    }
}

/// For an optional argument without a default the slice receives `Some(value)`.
fn wrap_some(arg: &Arg, finish: &str) -> String {
    if arg.default.is_none() {
        format!("Some({finish})")
    } else {
        finish.to_string()
    }
}

/// What the slice receives for an optional argument not passed.
fn default_expression(arg: &Arg) -> String {
    match (&arg.default, &arg.conversion) {
        (None, _) => "None".to_string(),
        (Some(DefaultValue::Boolean(value)), Conversion::Boolean) => value.to_string(),
        (Some(DefaultValue::Boolean(value)), Conversion::BooleanOrDictionary(_)) => {
            format!("{}::Boolean({value})", union_name(&arg.conversion))
        }
        (Some(DefaultValue::String(value)), Conversion::DomString) => {
            format!("{value:?}.to_string()")
        }
        (Some(DefaultValue::Null), _) => "None".to_string(),
        (Some(DefaultValue::Integer(value)), Conversion::Unsigned { .. }) => value.to_string(),
        (Some(other), _) => format!("compile_error!(\"no default {other:?}\")"),
    }
}
