//! A document parsed by `surf-html` as text for an agent: the outline of its tree
//! ([`outline()`]), the form `surf-agent look --as tree` prints (`docs/design/page-as-text.md`).
//! Every function gives the lines without the `> ` prefix of the answer's `text` block, one at a
//! time, so that a caller keeps only the lines it shows (`surf_answer::text_page`).
//!
//! The tree is read only through what `surf-html` has ([`Tree::root`](surf_html::Tree::root),
//! `data`, `children`), the hidden rule of `surf-page` ([`surf_page::is_hidden`], read for a
//! whole tree in one walk by [`surf_page::hidden_roots`]) and the handles of
//! [`surf_page::handles`]. Page text is written only through `surf_answer::quote_text` and
//! `quote_text_cut`, so its characters follow rule 2 of section 3 of the design note. No function
//! recurses over the depth of the tree: each walks with its own stack, and the size of a tree is
//! bounded by the limits of `surf-html`.
//!
//! # The outline
//!
//! - One line per element and per text node that is not ASCII whitespace only, in tree order,
//!   from the children of the root: the `html` element at level 0, two spaces per level below
//!   it. A text is stripped and collapsed (ASCII whitespace). An element whose children are all
//!   text nodes carries their text on its own line (`h1 "Fruit shop"`).
//! - An element's line: its local name, after `svg:` or `math:` for an element in the SVG or the
//!   MathML namespace, quoted by `quote_text` when it is not of the form `[a-z][a-z0-9-]*` or is
//!   of the form of a handle, `[lbf][0-9]+` (so `@l1` is surf's and `"l1"` the page's); its
//!   handle as `@<handle>`; the attributes in no namespace of [`outline::ATTRIBUTES`] (`id lang
//!   href src alt type name action method colspan rowspan`), in that order, as `name="value"`;
//!   then `hid <n>` for a hidden element, or `deep <n>` for an element at level
//!   [`outline::LEVEL_MAX`] (40) that has children; then its text. Every text and attribute value
//!   goes through `quote_text_cut` at [`outline::VALUE_CHARS_MAX`] (80) characters.
//! - `hid <n>` and `deep <n>`: `n` is the number of elements and of text nodes that are not
//!   whitespace only below the element; none of them is shown.
//! - Left out: comments, doctypes, processing instructions, template contents (they are not
//!   children of their `template`), and a text node hidden while its parent is not (the content
//!   of a closed `details` that is a text child of it).
//!
//! # Not claimed
//!
//! - No CSS: an element a style sheet or a `style` attribute hides is shown as not hidden;
//!   "hidden" is the rule of [`surf_page::is_hidden`].
//! - No script: the tree is the parser's, as the page was served.

#![forbid(unsafe_code)]

pub mod outline;

pub use outline::outline;

thread_local! {
    /// The nodes this thread's walks of this crate visited.
    static NODE_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn count_node_visit() {
    NODE_VISITS.with(|v| v.set(v.get().saturating_add(1)));
}

/// The number of nodes the walks of this crate have visited on this thread so far: each node
/// taken from a walk's stack counts once. For tests that bound the work of a rendering by a
/// count that does not depend on the machine.
#[doc(hidden)]
pub fn node_visits() -> usize {
    NODE_VISITS.with(std::cell::Cell::get)
}

/// Infra, "strip and collapse ASCII whitespace".
fn collapse(text: &str) -> String {
    text.split_ascii_whitespace().collect::<Vec<_>>().join(" ")
}
