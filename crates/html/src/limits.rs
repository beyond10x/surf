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
    /// The deepest node of the tree: the elements on its path from the root, itself included,
    /// a template counted for the nodes of its contents; for a subtree out of the tree (a
    /// hook's writes make them), from its own top. It bounds the tree as it is when the parser
    /// would insert or move a node, not the stack of open elements: an element that would be
    /// deeper is not created, and a move of the adoption agency algorithm or a
    /// `selectedcontent` copy that would put a node deeper is not made (the crate
    /// documentation, "Limits"). Default 512.
    pub max_depth: usize,
    /// The most nodes the tree holds ([`crate::Tree::node_count`]: the document, every node
    /// under it, template contents and nodes removed from their parent). Default 1,048,576.
    pub max_nodes: usize,
    /// The most attributes one element holds: a start tag that would keep one more creates no
    /// element. Default 1,024.
    pub max_attributes: usize,
    /// The most stored bytes the tree holds ([`crate::Tree::stored_bytes`]): the UTF-8 bytes of
    /// its strings (element names, attribute names, prefixes and values, text, comment data,
    /// doctype fields, processing instruction targets and data; clones included), and
    /// [`ATTRIBUTE_BYTES`] (80) for every attribute. Default 64 MiB (67,108,864).
    ///
    /// What it promises: the heap a tree holds is at most 2 × `stored_bytes()` + 3 ×
    /// [`NODE_BYTES`] (136) × `node_count()`. The factor 2 is the room a growing list or text
    /// keeps (it at most doubles; an attribute list that was empty grows exactly); the node term
    /// is the node's record in the arena and its entry in its parent's child list, each in a
    /// list that at most doubles, a child list's first room of 4 entries and a text's first
    /// 8 bytes. At the defaults that is at most 2 × 64 MiB + 3 × 136 × 1,048,576 bytes, about
    /// 536 MiB (inferred from the bound, not measured).
    pub max_stored_bytes: usize,
    /// The most entries tree construction visits in its scans of the stack of open elements, of
    /// the list of active formatting elements and of a node's children, and the nodes its walks
    /// for the depth limit visit (a walk up to read a depth, the depths of the stack's entries
    /// read again after a hook's write or an adoption agency iteration, and the walk of the
    /// nodes an adoption agency iteration would move deeper), checked after each token
    /// ([`crate::Tree::steps`] reads them). Default 134,217,728 (2^27).
    pub max_steps: usize,
}

/// The stored bytes an attribute counts beside its strings ([`Limits::max_stored_bytes`]): the
/// size of its record, `size_of::<Attribute>()`, 80 bytes on a 64-bit target.
pub const ATTRIBUTE_BYTES: usize = std::mem::size_of::<crate::tree::Attribute>();

/// The bytes of one node's record in the arena and of its entry in its parent's child list,
/// `size_of` of the record and of a [`crate::NodeId`]: 136 bytes on a 64-bit target. Not
/// counted in the stored bytes (the node limit bounds the nodes); it is the node term of the
/// heap bound of [`Limits::max_stored_bytes`].
pub const NODE_BYTES: usize = crate::tree::NODE_RECORD_BYTES;

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
    /// A node the parser would have made or moved would have been deeper than
    /// [`Limits::max_depth`] in the tree as it was then.
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
