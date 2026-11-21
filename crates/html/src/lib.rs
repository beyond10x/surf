//! The HTML parser of surf, written with the Rust standard library only: the encoding sniffing
//! algorithm and the decoders, the tokenizer, and tree construction of the WHATWG HTML Standard
//! (<https://html.spec.whatwg.org/multipage/parsing.html>). The decisions it follows are
//! `docs/design/html-parser.md`.
//!
//! # State: an interface that parses nothing yet
//!
//! Every entry point returns an empty result: [`parse_document`] and [`parse_fragment`] a tree
//! that holds only its root, [`parse_document_bytes`] that tree and no encoding,
//! [`encoding::decode`] no text and no encoding, [`tokenizer::tokenize`] no token and no parse
//! error. The stories of `epic:html-parser` fill one module each:
//!
//! - [`encoding`]: the label table, the decoders, BOM sniffing and the prescan.
//! - [`input`]: preprocessing of the input stream.
//! - [`tokenizer`]: the tokenizer states and the token types.
//! - [`charref`]: the character reference states and the named reference table.
//! - [`tree`]: the node arena.
//! - [`tree_builder`]: the insertion modes.
//! - [`limits`]: the limits of `docs/design/html-parser.md` item 7.
//!
//! The crate is held to the html5lib-tests and web-platform-tests data by the crate
//! `surf-html-conformance`, which counts the cases that exist, run and pass.

#![forbid(unsafe_code)]

pub mod charref;
pub mod encoding;
pub mod input;
pub mod limits;
pub mod tokenizer;
pub mod tree;
pub mod tree_builder;

pub use tree::{Attribute, Namespace, NodeData, NodeId, Tree};

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
/// Not written yet: returns a document with no child.
pub fn parse_document(input: &str, options: &ParseOptions) -> Tree {
    let _ = (input, options);
    Tree::document()
}

/// Parses bytes as a document: the encoding sniffing algorithm with no transport-layer
/// encoding ([`encoding::decode`]), then [`parse_document`] over the decoded text.
///
/// The encoding is the one [`encoding::decode`] chose. Changing the encoding while parsing
/// (13.2.3.4) is not written yet.
pub fn parse_document_bytes(bytes: &[u8], options: &ParseOptions) -> ParsedDocument {
    let decoded = encoding::decode(bytes);
    ParsedDocument {
        tree: parse_document(&decoded.text, options),
        encoding: decoded.encoding,
    }
}

/// Runs the fragment parsing algorithm (13.4) over an already decoded input for a context
/// element, with no sanitizer and no declarative shadow roots: a tree whose root is a
/// [`NodeData::DocumentFragment`] holding the nodes the algorithm returns.
///
/// Not written yet: returns a fragment with no child.
pub fn parse_fragment(input: &str, context: &Context, options: &ParseOptions) -> Tree {
    let _ = (input, context, options);
    Tree::fragment()
}
