//! The HTML parser of surf, written with the Rust standard library only: the encoding sniffing
//! algorithm and the decoders, the tokenizer, and tree construction of the WHATWG HTML Standard
//! (<https://html.spec.whatwg.org/multipage/parsing.html>). The decisions it follows are
//! `docs/design/html-parser.md`.
//!
//! # State
//!
//! Bytes are decoded, the decoded text is tokenized, and [`parse_document`] and
//! [`parse_document_bytes`] build a document from it with every insertion mode and the rules
//! for foreign content; [`parse_fragment`] runs the fragment parsing algorithm.
//! [`NOT_YET_PARSED`] names each part not yet parsed as the standard does. Module by module:
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
//!   serves a whole parse, so the last start tag is the one it emitted last. `<![CDATA[` opens
//!   a CDATA section when the tree builder said, before the token, that the adjusted current
//!   node is an element not in the HTML namespace; otherwise, and always in
//!   [`tokenizer::tokenize`], it is a bogus comment with the parse error
//!   `cdata-in-html-content`. A character
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
//!   [`Tree::children`], [`Tree::parent`] and [`Tree::descendants`], and its public write
//!   surface, the DOM's operations a script's bindings need (the module documentation lists
//!   them), each held to the limits of the parse ([`Tree::limits`]) and answering a
//!   [`WriteError`] instead of writing. A node is never dropped from a tree.
//! - [`tree_builder`]: written for every insertion mode of 13.2.6.4, with the stack of open
//!   elements and its scopes, the list of active formatting elements with its Noah's Ark clause
//!   and its reconstruction, the adoption agency algorithm, foster parenting, the stack of
//!   template insertion modes, the rules for foreign content with the MathML and SVG
//!   adjustments, the fragment parsing algorithm, and the `meta` rules that change the
//!   encoding. At the end of a document's parse each `select` updates its `selectedcontent`
//!   elements' copy of the selected `option`, as the DOM's queued update would. Parse errors
//!   are not collected, and no script runs here: with scripting enabled and a [`ScriptHook`]
//!   ([`parse_document_with_hook`]), the hook is called once for each script element the
//!   parser inserts, HTML or SVG, when it leaves the stack of open elements or the parse ends,
//!   to run it or to report it not run with its reason ([`NotRun`]): an SVG `script`, however
//!   it is closed, and an HTML `script` open when the parser stopped, at the end of the input
//!   or at a limit, are reported not run. A MathML `script` is no script element: no call.
//! - [`limits`]: written. [`limits::Limits`] holds the six limits of
//!   `docs/design/html-parser.md` item 7 with their defaults, [`ParseOptions::limits`] carries
//!   them into a parse, and [`Tree::stopped_by`] names the [`limits::Limit`] that stopped one.
//!   The section "Limits" below says what each bounds and where it stops.
//!
//! Entry points:
//!
//! - [`parse_document`]: the tokenizer and tree construction over decoded text. With no
//!   confidence in an encoding, a `meta` does not change it.
//! - [`parse_document_with_hook`]: [`parse_document`] with a [`ScriptHook`], called while
//!   scripting is enabled.
//! - [`parse_fragment`]: the fragment parsing algorithm (13.4) for a context element named by
//!   its namespace and local name, with no attribute and no ancestor: a document fragment that
//!   holds the children of the algorithm's `html` root.
//! - [`parse_document_bytes`]: runs [`encoding::decode`], then the tokenizer and tree
//!   construction. A `meta` met while the confidence is tentative changes the encoding as
//!   13.2.3.4 says: the same bytes are decoded again with the new encoding and the confidence
//!   certain, and parsed again from the start. [`ParsedDocument::encoding`] is the encoding in
//!   use at the end, always `Some`.
//!
//! The crate is held to the html5lib-tests and web-platform-tests data by the crate
//! `surf-html-conformance`, which counts the cases that exist, run and pass.
//!
//! # Limits
//!
//! The bytes come from an untrusted server. Every limit below ends a parse with a defined
//! result; none of them is a hang, an unbounded allocation or a panic. The standard places none
//! of them (13.2.6, before 13.2.6.1). The depth limit is the kind it expects ("practical
//! concerns will likely force user agents to impose nesting depth constraints"); the other five
//! are a deliberate deviation, and so is what reaching any of them does: parsing stops, as
//! "stop parsing" pops the open elements, without the end-of-file step, and the tree holds every
//! node inserted before that point. [`Tree::stopped_by`] names the limit, `None` for a parse
//! that ended without one; for [`parse_document_bytes`] it is `parsed.tree.stopped_by()`.
//! [`Tree::node_count`], [`Tree::stored_bytes`] and [`Tree::steps`] read the three counts the
//! limits keep. The defaults are those of [`limits::Limits::default`]:
//!
//! - **Input bytes** ([`limits::Limits::max_input_bytes`], default 16 MiB, the body limit of
//!   `surf-fetch`): a `&str` longer than the limit is cut to its longest prefix of at most that
//!   many bytes that ends on a character boundary; bytes are cut before they are decoded, so a
//!   sequence cut in two decodes as the decoder decodes an end of input. The cut input is parsed
//!   to its end, the end-of-file step included, so the tree is that of the prefix, and
//!   [`limits::Limit::InputBytes`] is named unless another limit stopped the parse of the
//!   prefix. The parse again after a `meta` that changes the encoding reads the same cut bytes.
//! - **Depth** ([`limits::Limits::max_depth`], default 512): an element that would be entry
//!   513 of the stack of open elements is not created.
//! - **Nodes** ([`limits::Limits::max_nodes`], default 1,048,576, template contents
//!   included): a node whose creation would take [`Tree::node_count`] past the limit is not
//!   created (a `template` is two nodes). The reconstruction of the active formatting elements
//!   checks before each element it creates. The adoption agency algorithm checks before each
//!   outer iteration that can create an element, for every element that iteration creates (at
//!   most four: three clones and the new formatting element), so an iteration runs whole or not
//!   at all and leaves no element out of its parent.
//! - **Attributes** ([`limits::Limits::max_attributes`], default 1,024): the tokenizer stops
//!   when a start tag token would keep one attribute more (a duplicate it drops does not
//!   count), and that tag creates no element. The `html` and `body` start tags of "in body"
//!   stop before a merge that would give the element one attribute more; none of that merge is
//!   made. The public [`tokenizer::tokenize`] runs without it.
//! - **Stored bytes** ([`limits::Limits::max_stored_bytes`], default 64 MiB): the UTF-8 bytes
//!   of every string the tree holds, clones included (element local names, attribute names,
//!   prefixes and values, text, comment data, doctype fields, processing instruction targets
//!   and data), and [`limits::ATTRIBUTE_BYTES`] (80, the size of its record) for every
//!   attribute. A node or an append that would take [`Tree::stored_bytes`] past the limit is
//!   not made. Text is appended a character at a time, so a text node holds everything up to
//!   the limit. Names count as well as values: the reconstruction clones attribute names, which
//!   would otherwise grow without a bound; records count because an attribute with a name of
//!   one byte holds 80. With the node limit it bounds the tree's heap: at most 2 ×
//!   [`Tree::stored_bytes`] + 3 × [`limits::NODE_BYTES`] (136) × [`Tree::node_count`].
//! - **Steps** ([`limits::Limits::max_steps`], default 2^27, 134,217,728): every entry visited
//!   in a scan of the stack of open elements or of the list of active formatting elements (the
//!   scopes, popping, "any other end tag", Noah's Ark, the reconstruction, the adoption agency
//!   algorithm's scans and its walk to the root), and every child visited in a parent's child
//!   list (inserting before a reference, finding the text node to append to, removing a node,
//!   moving children, looking for an element child). The count is checked after each token:
//!   the token that takes it past the limit is processed whole, then parsing stops. The
//!   `selectedcontent` update after the end-of-file token counts every node it visits and
//!   copies and is checked after each, so it stops where it passes the limit.
//!   [`Tree::steps`] reads the count a parse ended with. A scan of a child list for a reference
//!   starts at the last child, where the `table` of foster parenting stays while it is open.
//!   "Is in the stack of open elements"
//!   visits no entry (the stack keeps a count for each node), so a reconstruction with nothing
//!   to reconstruct, as before every character of a text, costs no step. The other
//!   five limits bound memory; this one bounds time, which they do not: an input inside them can
//!   cost tokens times depth stack visits. The default is twice the visits of a 16 MiB page of
//!   the kind of five large documentation pages measured (at most 3.8 visits per input byte).
//!
//! The decisions that keep the visits cheap: the special category and the scope categories of
//! an element are decided once when it is created; Noah's Ark compares two entries in O(1) when
//! their tag names, attribute counts or keys (a hash of the attributes sorted by name) differ;
//! the tokenizer's duplicate-attribute check and the `html` and `body` merge look names up in a
//! set; no rule copies a start tag token of the list of active formatting elements (the
//! adoption agency algorithm and the reconstruction read the entry's own). No clock is read: the
//! limits bound the work, and the tests of `tests/limits.rs` assert the steps, the nodes and the
//! bytes a parse allocates, not its time.
//!
//! What a parse holds besides the tree: the input as code points, 4 bytes for each character
//! (at most 64 MiB for an input of 16 MiB), the stack of open elements (at most
//! [`limits::Limits::max_depth`] entries, and a count of 4 bytes for each node), the list of
//! active formatting elements with a copy of each entry's start tag token, and the tokenizer's
//! current token and temporary buffer (a tag name of 1 byte and a buffer of 4 bytes for each
//! letter of an end tag name in RCDATA, RAWTEXT or script data, given back as character tokens
//! one at a time). [`parse_document_bytes`] holds the decoded text as well, at most 3 bytes for
//! each input byte, and two of them while it parses again after a `meta`.

#![forbid(unsafe_code)]

pub mod charref;
mod charref_table;
pub mod encoding;
pub mod input;
pub mod limits;
pub mod tokenizer;
pub mod tree;
pub mod tree_builder;

pub use tree::{Attribute, Namespace, NodeData, NodeId, Tree, WriteError};
pub use tree_builder::{NOT_YET_PARSED, NotRun, ScriptHook};

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
    /// The limits the parse stays within (the crate documentation, "Limits"); the default is
    /// [`limits::Limits::default`].
    pub limits: limits::Limits,
}

impl Default for ParseOptions {
    fn default() -> Self {
        ParseOptions {
            scripting_disabled: true,
            limits: limits::Limits::default(),
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
/// a `meta` never changes the encoding. An input longer than
/// [`limits::Limits::max_input_bytes`] is parsed as its prefix (the crate documentation,
/// "Limits").
pub fn parse_document(input: &str, options: &ParseOptions) -> Tree {
    let (input, cut) = limits::cut_str(input, options.limits.max_input_bytes);
    let mut tree = tree_builder::parse(input, options, None).tree;
    if cut {
        tree.stop_at(limits::Limit::InputBytes);
    }
    tree
}

/// [`parse_document`] with a hook for the document's scripts: while scripting is enabled
/// ([`ParseOptions::scripting_disabled`] false), `hook` is called once for each script element
/// the parser inserts, after the token that ended it or at the end of the parse, to run it
/// ([`ScriptHook::run`], which may
/// write the tree) or to report it not run with its reason ([`ScriptHook::skip`]); parsing then
/// goes on over the tree the hook left ([`tree_builder`] says when each call comes). With
/// scripting disabled no call is made, and the tree is that of [`parse_document`]; so is the
/// tree of a hook that writes nothing.
pub fn parse_document_with_hook(
    input: &str,
    options: &ParseOptions,
    hook: &mut dyn ScriptHook,
) -> Tree {
    let (input, cut) = limits::cut_str(input, options.limits.max_input_bytes);
    let mut tree = tree_builder::parse_with_hook(input, options, None, Some(hook)).tree;
    if cut {
        tree.stop_at(limits::Limit::InputBytes);
    }
    tree
}

/// Parses bytes as a document: the encoding sniffing algorithm with no transport-layer
/// encoding ([`encoding::decode`]), then the tokenizer and tree construction over the decoded
/// text, as [`parse_document`] does.
///
/// While the confidence is tentative, a `meta` that declares another encoding changes it
/// (13.2.3.4): the parse stops, the same bytes are decoded again with that encoding and the
/// confidence certain ([`encoding::decode_with`]), and the document is parsed again from the
/// start, so a parse runs at most twice. [`ParsedDocument::encoding`] is the encoding in use at
/// the end. Bytes past [`limits::Limits::max_input_bytes`] are cut before they are decoded, and
/// both parses read the same cut bytes (the crate documentation, "Limits").
pub fn parse_document_bytes(bytes: &[u8], options: &ParseOptions) -> ParsedDocument {
    let (bytes, cut) = limits::cut_bytes(bytes, options.limits.max_input_bytes);
    let mut parsed = parse_cut_bytes(bytes, options);
    if cut {
        parsed.tree.stop_at(limits::Limit::InputBytes);
    }
    parsed
}

/// [`parse_document_bytes`] over bytes already cut to the input limit.
fn parse_cut_bytes(bytes: &[u8], options: &ParseOptions) -> ParsedDocument {
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
/// The context element has no attribute and no ancestor, and its document is in no-quirks
/// mode. An input longer than [`limits::Limits::max_input_bytes`] is parsed as its prefix, as
/// [`parse_document`] does.
pub fn parse_fragment(input: &str, context: &Context, options: &ParseOptions) -> Tree {
    let (input, cut) = limits::cut_str(input, options.limits.max_input_bytes);
    let mut tree = tree_builder::parse_fragment(input, context, options);
    if cut {
        tree.stop_at(limits::Limit::InputBytes);
    }
    tree
}
