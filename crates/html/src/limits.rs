//! The limits of `docs/design/html-parser.md` item 7, which bound the work and the memory of a
//! parse of untrusted input, and the [`Limit`] that stopped one. Written by
//! `story:html-parser-limits`.
//!
//! The standard places none of them (13.2.6, before 13.2.6.1: "practical concerns will likely
//! force user agents to impose nesting depth constraints"). The depth limit is the kind it
//! expects; the input, node, attribute, stored-byte and step limits are a deliberate deviation,
//! and so is what reaching any of them does: parsing stops, the tree holds what was built before
//! that point, and [`crate::Tree::stopped_by`] names the limit. The crate documentation, section
//! "Limits", says where each one stops.

/// The bounds a parse stays within. Every field is public so a caller, or a test, can lower or
/// raise one and keep the [`Default`] of the others:
/// `Limits { max_depth: 64, ..Limits::default() }`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The most bytes of input parsed. A longer `&str` is cut to its longest prefix of at most
    /// this many bytes that ends on a character boundary, and bytes are cut before they are
    /// decoded; the cut input is parsed to its end. Default 16 MiB (16,777,216), the body limit
    /// of `surf-fetch`.
    pub max_input_bytes: usize,
    /// The most elements on the stack of open elements: an element that would be the next one
    /// is not created. Default 512.
    pub max_depth: usize,
    /// The most nodes the tree holds ([`crate::Tree::node_count`]: the document, every node
    /// under it, template contents and nodes removed from their parent). Default 1,048,576.
    pub max_nodes: usize,
    /// The most attributes one element holds: a start tag that would keep one more creates no
    /// element. Default 1,024.
    pub max_attributes: usize,
    /// The most bytes of strings the tree holds ([`crate::Tree::stored_bytes`]: the UTF-8 bytes
    /// of element names, attribute names, prefixes and values, text, comment data, doctype
    /// fields, processing instruction targets and data; clones included). Default 64 MiB
    /// (67,108,864).
    pub max_stored_bytes: usize,
    /// The most entries tree construction visits in its scans of the stack of open elements, of
    /// the list of active formatting elements and of a node's children, checked after each
    /// token. Default 134,217,728 (2^27).
    pub max_steps: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_input_bytes: 16 * 1024 * 1024,
            max_depth: 512,
            max_nodes: 1024 * 1024,
            max_attributes: 1024,
            max_stored_bytes: 64 * 1024 * 1024,
            max_steps: 1 << 27,
        }
    }
}

/// The limit that stopped a parse ([`crate::Tree::stopped_by`]), named as
/// `surf.document.ParseLimit` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Limit {
    /// The input was longer than [`Limits::max_input_bytes`]; the parse ran over its prefix.
    InputBytes,
    /// An element would have been one more than [`Limits::max_depth`] on the stack of open
    /// elements.
    Depth,
    /// A node would have taken the tree past [`Limits::max_nodes`].
    Nodes,
    /// An element would have held one more attribute than [`Limits::max_attributes`].
    Attributes,
    /// A string would have taken the tree past [`Limits::max_stored_bytes`].
    StoredBytes,
    /// Tree construction visited more entries than [`Limits::max_steps`].
    Steps,
}

/// The longest prefix of `input` of at most `max` bytes that ends on a character boundary, and
/// whether it is shorter than `input`.
pub(crate) fn cut_str(input: &str, max: usize) -> (&str, bool) {
    if input.len() <= max {
        return (input, false);
    }
    let mut end = max;
    while !input.is_char_boundary(end) {
        end -= 1;
    }
    (&input[..end], true)
}

/// The first `max` bytes of `input`, and whether that is shorter than `input`.
pub(crate) fn cut_bytes(input: &[u8], max: usize) -> (&[u8], bool) {
    if input.len() <= max {
        (input, false)
    } else {
        (&input[..max], true)
    }
}
