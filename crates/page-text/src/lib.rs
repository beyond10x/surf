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
//! CommonMark with the tables of GitHub Flavored Markdown 0.29, computed from the tree without
//! CSS. Read in the GFM specification (2026-11-22): its tables extension, the fence's info
//! string (after a backtick fence it holds no backtick, so a code span with its closing
//! backticks on the line opens no fence) and the 1 to 9 digits of a list number; the rest is from
//! memory of CommonMark 0.31.2. Block or inline by the element's name: the block elements are
//! [`markdown::BLOCK_ELEMENTS`], the `display: block` and `display: list-item` rules of the HTML
//! Standard's rendering section; every other element is inline. Inline content between blocks
//! is a paragraph; blocks are parted by one empty line and nest flat; the items of one list are
//! not parted (a tight list), and a block in an item after its text is preceded by one.
//!
//! - `h1` to `h6`: `#` to `######` and the heading's content on one run (its blocks joined by a
//!   space), one line. `p` and the other blocks: a paragraph. `br`: `\` at the end of the line.
//!   `hr`: `---`. `blockquote`: its lines behind `> `. `em`, `i`: `*..*`; `strong`, `b`:
//!   `**..**`, spaces at their edges moved outside, an empty run not written. `code`: a code
//!   span of its text. `pre`, `listing`, `xmp`: a fenced code block of their text, lines as they
//!   are; when the fence would pass [`markdown::FENCE_BYTES_MAX`] (16) backticks, an indented
//!   code block (each line behind four spaces), which has no fence a run inside could close.
//! - A link (`a` or `area` with `href`): `[text](<href>) [@l<n>]`, the `href` as written, not
//!   resolved. The text is the link's content as one run (its blocks joined by a space, an `img`
//!   by its `alt`), or its label (the accessible name) when that is empty. One link, one word.
//! - `img`: `![alt]`; none without an `alt` or with a blank one. `iframe`: `[iframe](<src>)`, no
//!   handle; none without `src`.
//! - `ul`, `ol`, `menu`, `dir`, `li`: `- ` or `<n>. ` (the `start` of an `ol` read when it is
//!   0 to 999,999,999, else 1; a number past 999,999,999 stays 999,999,999, as CommonMark allows
//!   9 digits; `reversed` and `value` not read); the content of an item indented by its marker's
//!   width. An item or a quote whose prefix (the markers and indentation of the lists and quotes
//!   it is in) would pass [`markdown::PREFIX_BYTES_MAX`] (128) bytes is written at the
//!   indentation of the one it is in, without its marker; [`markdown::Markdown::deep`] says how
//!   many levels were, for the answer's `note text: deep <n>`. `dl`: `dt` a paragraph
//!   `**term**`, `dd` a paragraph.
//! - Controls (`input`, `select`, `textarea`, `button`): `[@<handle> <kind>[ disabled][
//!   required][ checked]: <label>]`, with a space on each side, the values of
//!   [`surf_page::actionable_elements`]; `: <label>` left out for an empty label. Their content
//!   (a button's text, a `select`'s options, a `textarea`'s text) and their values are not
//!   written.
//! - `nav`, `header`, `footer`, `main`, `aside`: the line `[<name> <n> lines]` before their
//!   content, `n` the lines of that content (counted by a first walk); none for an empty one.
//! - `table` (GitHub Flavored Markdown 0.29, section 4.10, "Tables (extension)"): each
//!   `caption` child as a paragraph before it, in tree order (the first is the table's caption,
//!   "forming a table"); a header row (the first row of the first `thead`, or the first row
//!   when all its cells are `th`, else a row of empty cells), then `| --- |` per column; then the
//!   other rows: those of `thead`, those of `tbody` and directly in the table in tree order,
//!   those of `tfoot`. The slots of `colspan` and `rowspan` as the HTML Standard's "forming a
//!   table" places them (`colspan` 0 or not a number is 1, at most 1,000; `rowspan` not a
//!   number is 1, at most 65,534, `0` spans to the end of its row group): a spanned slot is an
//!   empty cell, the text in the slot of the cell that spans.
//!   Columns: the widest row, at most [`markdown::COLUMNS_MAX`] (20); a wider table gets a
//!   21st column holding `[cut]`. A cell is one line: its inline content, its blocks, its `br`
//!   and the line feeds of a `pre` joined by one space, a `pre` as a code span, escaped as all
//!   page text (`|` as `\|`, in a code span too), cut at 200 characters of page text before
//!   `[cut]`; an empty cell is `|  |`. A table inside a cell is `[table <n>]` there (`n` from 1
//!   in document order over the document's nested tables), and follows its outer table after an
//!   empty line, the line `[table <n>]` and an empty line, in the same form. A table inside a
//!   heading is a run of its text.
//!
//!   Where it differs from "forming a table" (`tables.html`, read 2026-11-22): every `thead`'s
//!   rows come first, not only the first's (the story's order); a `rowspan` past the rows of its
//!   row group ends there, as a browser lays it out, where the algorithm adds empty rows;
//!   `col` and `colgroup` add no columns; hidden rows and cells take no slot, as `display:
//!   none` removes them from the layout. From GitHub Flavored Markdown: no alignment (`:`) is
//!   written.
//! - Not written: hidden nodes ([`surf_page::is_hidden`]: `head` with `title` and `meta`,
//!   `script`, `style`, `template`, `hidden`, `aria-hidden`, a closed `dialog`, a `popover` but
//!   an open `dialog`, the copy of the selected option in a `selectedcontent`, the content of a closed `details` but its `summary`, `input type=hidden`),
//!   comments, doctypes, processing instructions, `svg` and `math`; `lang`. `noscript` content is
//!   written: surf runs no script.
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
//! with its indentation and markers, wrapped at the last space that keeps it so, else between two
//! atoms or two units of text. An atom (a link with its handle, an image, an `iframe`, a code
//! span, a control) is never broken across lines; one longer than [`markdown::ATOM_BYTES_MAX`]
//! (864) bytes is cut before `[cut]` (a link's text and its target both, a cut target ending in
//! `[cut]>`; the handle stays whole). A heading is one line, cut before `[cut]` when it is
//! longer. A heading, a cell, a link's text and an `alt` keep [`markdown::TEXT_CHARS_MAX`] (200)
//! characters of page text, a label [`markdown::LABEL_CHARS_MAX`] (60), each ending in `[cut]`;
//! the escapes are added to the kept characters, then `[cut]`. In a heading or a cell an atom
//! spends the characters of page text it shows (at least 1), and a cut falls between atoms. The
//! text of a line keeps at least `LINE_BYTES_MAX - PREFIX_BYTES_MAX` (896) bytes, so the bytes
//! written are at most a constant times the page's.
//!
//! Deviations: `\\`, `\[`, `\<`, `\|` and `\x..` in code show as written in a renderer; a word
//! broken at 1,024 bytes reads as two, and so do two atoms with no space between them broken
//! there; the `href` is shown as written, not resolved; a landmark line joins the paragraph after
//! it in a renderer; a `\` of a `br` that would pass 1,024 bytes is left out, so the break reads
//! as a space; a table row is not wrapped, so it can pass 1,024 bytes (21 cells of at most 200
//! characters of page text and their escapes); an item or a quote nested past 128 bytes of
//! prefix loses its marker.
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
    /// The rows and cells this thread's table code visited.
    static TABLE_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn count_node_visit() {
    NODE_VISITS.with(|v| v.set(v.get().saturating_add(1)));
}

fn count_table_visit() {
    TABLE_VISITS.with(|v| v.set(v.get().saturating_add(1)));
}

/// The number of rows and cells the table code of this crate has visited on this thread so far:
/// each row read to place its cells or to find the end of its row group, and each cell placed,
/// counts once. For tests that bound the work of a table by a count that does not depend on the
/// machine.
#[doc(hidden)]
pub fn table_visits() -> usize {
    TABLE_VISITS.with(std::cell::Cell::get)
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
