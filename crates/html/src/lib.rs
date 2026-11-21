//! The HTML parser of surf, written with the Rust standard library only: the encoding sniffing
//! algorithm and the decoders, the tokenizer, and tree construction of the WHATWG HTML Standard
//! (<https://html.spec.whatwg.org/multipage/parsing.html>). The decisions it follows are
//! `docs/design/html-parser.md`.
//!
//! # State
//!
//! Bytes are decoded and the decoded text can be tokenized, but no entry point builds a tree
//! from it yet. [`parse_document`] and [`parse_fragment`] return a tree that holds only its
//! root, and neither calls the tokenizer. Module by module:
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
//! - [`tokenizer`]: written but for the character reference states. [`tokenizer::tokenize`]
//!   runs the data, RCDATA, RAWTEXT, script data and PLAINTEXT states, the less-than sign, end
//!   tag and script data escape states they lead to, and the tag, attribute, comment, DOCTYPE,
//!   CDATA section and processing instruction states, started in any [`tokenizer::State`], and
//!   returns tokens and parse errors. It reads `last_start_tag` as the last start tag emitted,
//!   which each start tag it emits replaces; an end tag in the RCDATA, RAWTEXT or script data
//!   states ends them only when its name is that tag's. It runs from one state to the end of
//!   the input: no tree builder switches its state between tokens. `<![CDATA[` in the data
//!   state is a bogus comment with the parse error `cdata-in-html-content`, because there is no
//!   tree builder to say that the adjusted current node is in a foreign namespace. The
//!   character reference state is a stub: `&` is emitted as a character, or appended to the
//!   attribute value, and the characters after it are read in the state it came from.
//! - [`charref`]: nothing yet. The module holds no item: no named reference table, no
//!   character reference states.
//! - [`tree`]: the types of the node arena and the accessors [`Tree::root`], [`Tree::data`] and
//!   [`Tree::children`]. Nothing in the crate adds a node to a tree.
//! - [`tree_builder`]: nothing yet. The module holds no item: no insertion mode.
//! - [`limits`]: nothing yet. The module holds no item: none of the limits of
//!   `docs/design/html-parser.md` item 7 is enforced.
//!
//! Entry points:
//!
//! - [`parse_document`]: returns a document with no child, whatever the input and the options.
//! - [`parse_fragment`]: returns a document fragment with no child, whatever the input, the
//!   context and the options.
//! - [`parse_document_bytes`]: runs [`encoding::decode`] and passes the text to
//!   [`parse_document`], so the tree has no child. [`ParsedDocument::encoding`] is the
//!   encoding [`encoding::decode`] chose, always `Some`. Changing the encoding while parsing
//!   is not written.
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
