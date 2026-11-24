//! A reader of Web IDL for the generated DOM bindings: it reads the IDL Index of the vendored
//! DOM Standard and Web IDL Standard into a model, and refuses every construct it does not know,
//! naming the file and line, so that a newer index fails instead of being read wrong.
//!
//! `data/whatwg/` holds the two pages unchanged, each a commit snapshot, with `SOURCE` (address,
//! commit, date, licence, the line count of each index and the sha256 of each file).
//!
//! # Reading
//!
//! [`read_page`] parses a page with `surf-html` and takes the text content of each `pre`
//! element after the `h2` element whose `id` is `idl-index` and before the next `h2`
//! ([`extract_index`]); [`read`] reads such a text. A line of an error or of the model is a line
//! of that text, counted from 1. [`Model`] holds the definitions of both pages: the merged view
//! of an interface ([`Model::members`], [`Model::counts`]), the extended attributes of a file
//! ([`Model::extended_attribute_counts`]) and the names no definition defines
//! ([`Model::unresolved`]). No name is resolved outside the model.
//!
//! # The constructs read
//!
//! - Definitions: `interface` with inheritance, `partial interface`, `interface mixin` and
//!   `includes`, `callback interface`, `callback`, `dictionary` with inheritance, `required`
//!   members and default values, `enum`, `typedef`.
//! - Members: `const`, `attribute` and `readonly attribute`, `stringifier` (bare and on an
//!   attribute), regular and `static` operations, `constructor`, special `getter` operations,
//!   `iterable<T>`.
//! - Arguments: `optional` with a default (`{}`, `null`, `true`, `false`, a string, an integer),
//!   and variadic (`T... name`); named by an identifier or, from `ADDED`, by an argument name
//!   keyword.
//! - Types: `any`, `undefined`, `boolean`, `object`, `unsigned short`, `unsigned long`,
//!   `unsigned long long`, `double`, `unrestricted double`, `DOMString`, `USVString`,
//!   `sequence<T>`, unions, nullable `T?`, a name, and extended attributes on a type; and, from
//!   `ADDED`, `short` and the buffer related types.
//! - Extended attributes: the names of [`EXTENDED_ATTRIBUTES`], with no value, an identifier,
//!   `*` or a list of identifiers.
//! - Identifiers escaped with a leading `_` (`_any`), read without it; integers in decimal,
//!   hexadecimal and octal; `//` and `/* */` comments.
//!
//! Added after the first runs over the vendored pages named them as refused, each part of the
//! grammar of the vendored Web IDL Standard ([`ADDED`]):
//!
//! - "argument name keyword": an argument named by a keyword of the grammar's
//!   `ArgumentNameKeyword` (`DOMString interface`, `EventListener? callback`,
//!   `DOMString? namespace`), read as written (`dom.html`, line 61 of its index);
//! - "short": the type `short` (`dom.html`, line 524);
//! - "buffer related type": the grammar's `BufferRelatedType`s, `ArrayBuffer`,
//!   `SharedArrayBuffer`, `DataView` and the twelve typed arrays (`webidl.html`, line 14);
//! - "[Serializable]": the extended attribute with no value (`webidl.html`, line 1). The HTML
//!   Standard defines it, not the Web IDL Standard; only its form, `ExtendedAttributeNoArgs`, is
//!   Web IDL's grammar;
//! - "[AllowShared]": the extended attribute with no value, on a union member type
//!   (`webidl.html`, line 20).
//!
//! # Refused
//!
//! Every other construct, each with `<file>:<line>: refused: <construct>`, among them
//! `Promise<T>`, an operation name twice in one interface, mixin or callback interface (an
//! overload, refused by [`read`]) or twice in the merged view of an interface, its partials and
//! the mixins it includes (refused by [`Model::new`], naming the second), `namespace`,
//! `maplike`, `setlike`, `async iterable`, `legacycaller`, an extended attribute not in
//! [`EXTENDED_ATTRIBUTES`] or with an argument list, `setter`, `deleter`, `static` attributes,
//! `record`, `FrozenArray`, `long`, `float`, `byte`, `octet`, `bigint`, `ByteString`, floating
//! point constants and the defaults `[]` and `undefined`.
//!
//! [`Model::new`] also refuses a name that two definitions define (a `partial interface` and an
//! `includes` define none), naming both, and `A includes B;` where `B` is defined and is no
//! interface mixin or `A` is defined and is no interface.
//!
//! A type that nests deeper than [`MAX_TYPE_DEPTH`] levels is refused too, so a hostile text
//! cannot drive the recursion of the type parser past that many calls.

#![forbid(unsafe_code)]

mod extract;
mod lexer;
mod model;
mod parser;

pub use extract::extract_index;
pub use model::{
    Argument, ConstValue, Counts, DefaultValue, Definition, DefinitionKind, DictionaryMember,
    ExtendedAttribute, ExtendedAttributeValue, IntegerSize, Member, MemberKind, Model, Reference,
    Type, TypeKind, Unresolved,
};
pub use parser::MAX_TYPE_DEPTH;

/// The names of the extended attributes the reader knows; every other one is refused.
pub const EXTENDED_ATTRIBUTES: &[&str] = &[
    "AllowShared",
    "CEReactions",
    "EnforceRange",
    "Exposed",
    "LegacyNullToEmptyString",
    "LegacyUnenumerableNamedProperties",
    "LegacyUnforgeable",
    "NewObject",
    "PutForwards",
    "Replaceable",
    "SameObject",
    "Serializable",
    "Unscopable",
];

/// The names of [`EXTENDED_ATTRIBUTES`] that apply to a type: the Web IDL Standard, section
/// 2.13.33 "Annotated types", lists `[AllowResizable]`, `[AllowShared]`, `[Clamp]`,
/// `[EnforceRange]` and `[LegacyNullToEmptyString]`; the two the indexes do not use,
/// `[AllowResizable]` and `[Clamp]`, are refused like every name outside
/// [`EXTENDED_ATTRIBUTES`]. As that section says, one of these applies to a type when it is
/// written on the type itself (`attribute [EnforceRange] long x`), or before an argument or a
/// dictionary member: there it is read onto its type ([`Type::extended_attributes`]), and every
/// other name stays on the argument or the member. Written before an attribute, an operation, a
/// constant, any other member or a definition, it is refused.
pub const TYPE_EXTENDED_ATTRIBUTES: &[&str] =
    &["AllowShared", "EnforceRange", "LegacyNullToEmptyString"];

/// The constructs added after the first run over the vendored pages named them as refused; each
/// is part of the grammar of the vendored Web IDL Standard.
pub const ADDED: &[&str] = &[
    "argument name keyword",
    "short",
    "buffer related type",
    "[Serializable]",
    "[AllowShared]",
];

/// A construct the reader refuses, or a text it cannot read, with the file and line it starts on.
/// It prints as `<file>:<line>: refused: <construct>`, where the line is a line of the extracted
/// IDL index (the text [`read`] is given), counted from 1, not a line of the page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadError {
    /// The file named to [`read`] or [`read_page`].
    pub file: String,
    /// A line of the extracted IDL index (the text [`read`] is given), counted from 1, not a
    /// line of the page; 0 for a page without an IDL Index.
    pub line: usize,
    /// What was refused.
    pub construct: String,
}

impl ReadError {
    pub(crate) fn new(file: &str, line: usize, construct: impl Into<String>) -> ReadError {
        ReadError {
            file: file.to_string(),
            line,
            construct: construct.into(),
        }
    }
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}: refused: {}",
            self.file, self.line, self.construct
        )
    }
}

impl std::error::Error for ReadError {}

/// The definitions of the IDL `text`, each naming `file`.
pub fn read(text: &str, file: &str) -> Result<Vec<Definition>, ReadError> {
    parser::parse(lexer::lex(text, file)?, file)
}

/// A page read: the text of its IDL Index and its definitions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    /// The text of the IDL Index ([`extract_index`]).
    pub index: String,
    /// Its definitions ([`read`]).
    pub definitions: Vec<Definition>,
}

/// The IDL Index of the page `bytes` ([`extract_index`]) and its definitions, each naming
/// `file`.
pub fn read_page(bytes: &[u8], file: &str) -> Result<Page, ReadError> {
    let index = extract_index(bytes, file)?;
    let definitions = read(&index, file)?;
    Ok(Page { index, definitions })
}
