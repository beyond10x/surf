//! The HTML parser of surf, written with the Rust standard library only: the encoding sniffing
//! algorithm and the decoders, the tokenizer, and tree construction of the WHATWG HTML Standard
//! (<https://html.spec.whatwg.org/multipage/parsing.html>). The decisions it follows are
//! `docs/design/html-parser.md`.
//!
//! # State
//!
//! Bytes are decoded, the decoded text is tokenized, and [`parse_document`] and
//! [`parse_document_bytes`] build a document from it with the insertion modes outside tables,
//! templates and foreign content. [`NOT_YET_PARSED`] names each part not yet parsed as the
//! standard does; [`parse_fragment`] returns a fragment that holds only its root. Module by
//! module:
//!
//! - [`encoding`]: written. [`encoding::decode`] runs the encoding sniffing algorithm with no
//!   transport-layer encoding: a byte order mark (confidence certain), else the prescan of the
//!   first 1,024 bytes ([`encoding::PRESCAN_BYTES`]) for a `meta` element and an XML
//!   declaration, else windows-1252 (confidence tentative); it then decodes the bytes. The
//!   decoders are those of UTF-8, UTF-16LE, UTF-16BE, windows-1252 and replacement. The other
//!   35 of the 40 encodings in the label table [`encoding::ENCODINGS`] are decoded as
//!   windows-1252: [`encoding::Decoded::encoding`] names the encoding chosen and
//!   [`encoding::Decoded::decoded_with`] the one that decoded. [`encoding::get_an_encoding`]
//!   looks a label up. [`encoding::decode_with`] decodes again with a named encoding, with
//!   confidence certain, and returns `None` for a name that is no encoding's. The user's
//!   override, the container document, an earlier visit and autodetection are not taken.
//! - [`input`]: written. [`input::InputStream`] holds the code points of the input with every
//!   CR LF and every CR on its own turned into one LF, and [`input::parse_error`] names the
//!   three parse errors of the input stream (surrogate, noncharacter, control character).
//! - [`tokenizer`]: written. [`tokenizer::tokenize`] runs the data, RCDATA, RAWTEXT, script
//!   data and PLAINTEXT states, the less-than sign, end tag and script data escape states they
//!   lead to, the tag, attribute, comment, DOCTYPE, CDATA section and processing instruction
//!   states, and the character reference states, started in any [`tokenizer::State`], and
//!   returns tokens and parse errors. It reads `last_start_tag` as the last start tag emitted,
//!   which each start tag it emits replaces; an end tag in the RCDATA, RAWTEXT or script data
//!   states ends them only when its name is that tag's. The tree builder drives the same state
//!   machine one token at a time and switches its state between two tokens; one tokenizer
//!   serves a whole parse, so the last start tag is the one it emitted last. `<![CDATA[` in the
//!   data state is a bogus comment with the parse error `cdata-in-html-content`: no element in
//!   a foreign namespace is created yet, so the adjusted current node is never one. A character
//!   reference in data, in RCDATA and in an attribute value is read as 13.2.5.77 to 13.2.5.84
//!   say: a named one from the table of 13.5, a numeric one with the replacements of the
//!   numeric character reference end state.
//! - [`charref`]: written, with no public item. It holds the named reference lookup, longest
//!   name first and at most 32 code points after `&`, and the numeric replacement table that
//!   the tokenizer's character reference states use. The named table is a private module
//!   written from the vendored `entities.json` by the binary `entities-table` of
//!   `surf-html-conformance` and never edited by hand; it carries the BSD 3-Clause notice of
//!   the WHATWG in its header.
//! - [`tree`]: the node arena and its readers [`Tree::root`], [`Tree::data`],
//!   [`Tree::children`], [`Tree::parent`] and [`Tree::descendants`]. Only tree construction
//!   changes a tree; a node is never dropped from it.
//! - [`tree_builder`]: written for the insertion modes initial, before html, before head, in
//!   head, in head noscript, after head, in body, text, after body, in frameset, after
//!   frameset, after after body and after after frameset, with the stack of open elements and
//!   its scopes, the list of active formatting elements with its Noah's Ark clause and its
//!   reconstruction, the adoption agency algorithm, and the `meta` rules that change the
//!   encoding. Not yet: tables and `select`, `template` and foreign content
//!   ([`NOT_YET_PARSED`]); their tokens go to in body's "any other start tag" and "any other end
//!   tag". Parse errors are not collected, and no script runs.
//! - [`limits`]: nothing yet. The module holds no item: none of the limits of
//!   `docs/design/html-parser.md` item 7 is enforced, so the depth of a tree, the number of its
//!   nodes and the length of its texts have no bound.
//!
//! Entry points:
//!
//! - [`parse_document`]: the tokenizer and tree construction over decoded text. With no
//!   confidence in an encoding, a `meta` does not change it.
//! - [`parse_fragment`]: returns a document fragment with no child, whatever the input, the
//!   context and the options (story `html-tree-foreign`).
//! - [`parse_document_bytes`]: runs [`encoding::decode`], then the tokenizer and tree
//!   construction. A `meta` met while the confidence is tentative changes the encoding as
//!   13.2.3.4 says: the same bytes are decoded again with the new encoding and the confidence
//!   certain, and parsed again from the start. [`ParsedDocument::encoding`] is the encoding in
//!   use at the end, always `Some`.
//!
//! The crate is held to the html5lib-tests and web-platform-tests data by the crate
//! `surf-html-conformance`, which counts the cases that exist, run and pass.

#![forbid(unsafe_code)]

pub mod charref;
mod charref_table;
pub mod encoding;
pub mod input;
pub mod limits;
pub mod tokenizer;
pub mod tree;
pub mod tree_builder;

pub use tree::{Attribute, Namespace, NodeData, NodeId, Tree};
pub use tree_builder::NOT_YET_PARSED;

/// How a document or a fragment is parsed.
///
/// Build it with `ParseOptions { scripting_disabled: …, ..ParseOptions::default() }`, so that a
/// field added later keeps its default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseOptions {
    /// "Parser scripting disabled" (13.2.4.5): `true` unless a script engine runs the document.
    /// surf has none, so the default is `true`; the tree-construction data sets it `false` for
    /// its `#script-on` cases.
    pub scripting_disabled: bool,
}

impl Default for ParseOptions {
    fn default() -> Self {
        ParseOptions {
            scripting_disabled: true,
        }
    }
}

/// The context element of the fragment parsing algorithm (13.4), named by its namespace and
/// local name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    /// The context element's namespace: [`Namespace::Html`], [`Namespace::Svg`] or
    /// [`Namespace::MathMl`].
    pub namespace: Namespace,
    /// The context element's local name.
    pub local_name: String,
}

/// What [`parse_document_bytes`] returns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedDocument {
    /// The document.
    pub tree: Tree,
    /// The name, as the Encoding Standard writes it, of the encoding chosen for the document at
    /// the end of parsing: the one the encoding sniffing algorithm chose
    /// ([`encoding::Decoded::encoding`]), also when it has no decoder and the text was decoded
    /// as windows-1252 ([`encoding::Decoded::decoded_with`]); `None` when none was chosen.
    pub encoding: Option<&'static str>,
}

/// Parses an already decoded input as a document: a tree whose root is a
/// [`NodeData::Document`].
///
/// The tokenizer and tree construction run as [`tree_builder`] says, with the parts
/// [`NOT_YET_PARSED`] names not yet parsed as the standard does. The text has no confidence, so
/// a `meta` never changes the encoding.
pub fn parse_document(input: &str, options: &ParseOptions) -> Tree {
    tree_builder::parse(input, options, None).tree
}

/// Parses bytes as a document: the encoding sniffing algorithm with no transport-layer
/// encoding ([`encoding::decode`]), then the tokenizer and tree construction over the decoded
/// text, as [`parse_document`] does.
///
/// While the confidence is tentative, a `meta` that declares another encoding changes it
/// (13.2.3.4): the parse stops, the same bytes are decoded again with that encoding and the
/// confidence certain ([`encoding::decode_with`]), and the document is parsed again from the
/// start, so a parse runs at most twice. [`ParsedDocument::encoding`] is the encoding in use at
/// the end.
pub fn parse_document_bytes(bytes: &[u8], options: &ParseOptions) -> ParsedDocument {
    let decoded = encoding::decode(bytes);
    let in_use = decoded.encoding.map(|name| tree_builder::InputEncoding {
        name,
        confidence: decoded.confidence,
    });
    let parsed = tree_builder::parse(&decoded.text, options, in_use);
    let Some(new) = parsed.reparse_with else {
        return ParsedDocument {
            tree: parsed.tree,
            encoding: decoded.encoding,
        };
    };
    // The new encoding is a name of the label table, which `decode_with` always decodes.
    let (text, encoding) = match encoding::decode_with(bytes, new) {
        Some(again) => (again.text, again.encoding),
        None => (decoded.text, decoded.encoding),
    };
    let certain = encoding.map(|name| tree_builder::InputEncoding {
        name,
        confidence: encoding::Confidence::Certain,
    });
    ParsedDocument {
        tree: tree_builder::parse(&text, options, certain).tree,
        encoding,
    }
}

/// Runs the fragment parsing algorithm (13.4) over an already decoded input for a context
/// element, with no sanitizer and no declarative shadow roots: a tree whose root is a
/// [`NodeData::DocumentFragment`] holding the nodes the algorithm returns.
///
/// Not written yet (story `html-tree-foreign`): returns a fragment with no child.
pub fn parse_fragment(input: &str, context: &Context, options: &ParseOptions) -> Tree {
    let _ = (input, context, options);
    Tree::fragment()
}
