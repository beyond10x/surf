//! A document parsed by `surf-html` as text for an agent: the outline of its tree
//! ([`outline()`]), the form `surf-agent look --as tree` prints, and its content as Markdown
//! ([`markdown::markdown`], `look --as markdown`), as `docs/design/page-as-text.md` has them.
//! Every function gives the lines without the `> ` prefix of the answer's `text` block, one at a
//! time, so that a caller keeps only the lines it shows (`surf_answer::text_page`).
//!
//! The tree is read only through what `surf-html` has ([`Tree::root`](surf_html::Tree::root),
//! `data`, `children`), the hidden rule of `surf-page` ([`surf_page::is_hidden`], read for a
//! whole tree in one walk by [`surf_page::hidden_roots`]), the rows of
//! [`surf_page::actionable_elements`] and the handles of [`surf_page::handles`]. Page text in the
//! outline is written only through `surf_answer::quote_text` and `quote_text_cut`, and in the
//! Markdown through the escapes below, so its characters follow rule 2 of section 3 of the design
//! note (`surf_answer::escaped_in_text`, `escaped_at_run_start`). No function recurses over the
//! depth of the tree: each walks with its own stack, and the size of a tree is bounded by the
//! limits of `surf-html`.
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
//! # The Markdown
//!
//! CommonMark 0.31.2 (from memory; the specification was not read for this crate), computed from
//! the tree without CSS. Block or inline by the element's name: the block elements are
//! [`markdown::BLOCK_ELEMENTS`], the `display: block` and `display: list-item` rules of the HTML
//! Standard's rendering section; every other element is inline. Inline content between blocks
//! is a paragraph; blocks are parted by one empty line and nest flat; the items of one list are
//! not parted (a tight list), and a block in an item after its text is preceded by one.
//!
//! - `h1` to `h6`: `#` to `######` and the heading's content on one run (its blocks joined by a
//!   space). `p` and the other blocks: a paragraph. `br`: `\` at the end of the line. `hr`:
//!   `---`. `blockquote`: its lines behind `> `. `em`, `i`: `*..*`; `strong`, `b`: `**..**`,
//!   spaces at their edges moved outside, an empty run not written. `code`: a code span of its
//!   text. `pre`, `listing`, `xmp`: a fenced code block of their text, lines as they are.
//! - A link (`a` or `area` with `href`): `[text](<href>) [@l<n>]`, the `href` as written, not
//!   resolved. The text is the link's content as one run (its blocks joined by a space, an `img`
//!   by its `alt`), or its label (the accessible name) when that is empty. One link, one word.
//! - `img`: `![alt]`; none without an `alt` or with a blank one. `iframe`: `[iframe](<src>)`, no
//!   handle; none without `src`.
//! - `ul`, `ol`, `menu`, `dir`, `li`: `- ` or `<n>. ` (the `start` of an `ol` read when it is
//!   0 to 999,999,999, else 1; `reversed` and `value` not read); the content of an item indented
//!   by its marker's width. `dl`: `dt` a paragraph `**term**`, `dd` a paragraph.
//! - Controls (`input`, `select`, `textarea`, `button`): `[@<handle> <kind>[ disabled][
//!   required][ checked]: <label>]`, with a space on each side, the values of
//!   [`surf_page::actionable_elements`]; `: <label>` left out for an empty label. Their content
//!   (a button's text, a `select`'s options, a `textarea`'s text) and their values are not
//!   written.
//! - `nav`, `header`, `footer`, `main`, `aside`: the line `[<name> <n> lines]` before their
//!   content, `n` the lines of that content (counted by a first walk); none for an empty one.
//! - `table`: one paragraph of its text, its blocks joined by a space (until
//!   `story:page-text-tables`).
//! - Not written: hidden nodes ([`surf_page::is_hidden`]: `head` with `title` and `meta`,
//!   `script`, `style`, `template`, `hidden`, `aria-hidden`, a closed `dialog`, the content of a
//!   closed `details` but its `summary`, `input type=hidden`), comments, doctypes, processing
//!   instructions, `svg` and `math`; `lang`. `noscript` content is written: surf runs no script.
//! - Whitespace: ASCII whitespace stripped and collapsed per block; U+00A0 kept.
//!
//! The escapes (design note section 3, rules 2 to 4):
//!
//! - In text, link text and a link destination, one set: `\` `` ` `` `*` `_` `[` `]` `<` `>`
//!   `&` `|` `~` `#` get a backslash. At the start of every line, after wrapping, also `-`, `+`,
//!   `=` and the `.` or `)` after leading digits (`1\.`). In code only `\`, `[`, `]`, `<` and
//!   `|`; a fence or a code span is one backtick longer than the longest run inside, a fence at
//!   least three. So an unescaped `[`, `]` or `<` is surf's.
//! - The characters `surf_answer::escaped_in_text` names are written as `\x` and two hexadecimal
//!   digits per UTF-8 byte; so is a combining mark `escaped_at_run_start` names at the start of a
//!   run (after a character surf wrote, and at the start of a line).
//! - An escape unit is one character, a `\` with the character it escapes, or the `\x` bytes of
//!   one character; every wrap and cut falls between units.
//!
//! Bounds (design note section 6): a line has at most [`markdown::LINE_BYTES_MAX`] (1,024) bytes
//! with its indentation and markers, wrapped at the last space that keeps it so, a longer word
//! broken between units. A heading, a link's text and an `alt` are cut at
//! [`markdown::TEXT_CHARS_MAX`] (200) characters of page text, a label at
//! [`markdown::LABEL_CHARS_MAX`] (60), each ending in `[cut]`; the escapes are added to the kept
//! characters, then `[cut]`.
//!
//! Deviations: `\\`, `\[`, `\<`, `\|` and `\x..` in code show as written in a renderer; a word
//! broken at 1,024 bytes reads as two, and a heading wrapped there continues as a paragraph; the
//! `href` is shown as written, not resolved; a landmark line joins the paragraph after it in a
//! renderer; a `\` of a `br` that would pass 1,024 bytes is left out, so the break reads as a
//! space.
//!
//! # Not claimed
//!
//! - No CSS: an element a style sheet or a `style` attribute hides is shown as not hidden;
//!   "hidden" is the rule of [`surf_page::is_hidden`]. Text is in source order.
//! - No script: the tree is the parser's, as the page was served.
//! - No model reader: the escapes protect a reader that parses Markdown; whether a language model
//!   reads `\[@l1\]` as different from `[@l1]` is not established (design note section 3).

#![forbid(unsafe_code)]

pub mod markdown;
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
