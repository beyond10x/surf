---
format: aep.planning-md/3
id: story:page-text-outline
kind: story
status: draft
title: The document's tree as a bounded outline, with the handles now will use
summary: A new crate surf-page-text renders a surf-html tree as indented lines with quoted text, a small attribute set, hid and deep counts, and the handles l<n>, b<n>, f<n>.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:html-actionable-elements
- depends_on: story:html-parser-limits
scope:
- confidence: inferred
  path: crates/page-text
revision: 2
---
## Context

`docs/design/page-as-text.md` (proposed) section 5: the document's tree as an outline, one line
per element and per text node, bounded, with the handles `now` will use. The operator,
2026-11-22: "what we want to see is like the tree structure rendered in some way".

The renderer is a library over a `surf-html` tree, in a new crate so that it is written beside
the tree stories, which change `crates/html` and `crates/page`: the reason `surf-page` is a crate
of its own (`story:html-actionable-elements`). It reads the tree only through what `surf-html`
has (`Tree::root`, `data`, `children`, `parent`), the hidden rule `surf_page::is_hidden` and the
elements of `surf_page::actionable_elements`; it adds no reader to either crate. Text and
attribute values go through `surf_answer::quote_cut`, the one quoting function. The tree's size
is bounded by `story:html-parser-limits`; nothing here recurses over the depth.

Handles (design note section 5): `l<n>` for a link, `b<n>` for a button, `f<n>` for the other
seven kinds, numbered per letter in document order, hidden elements included, from 1 in each
document. The tree is not changed after parsing (D2, `docs/plan/agent-plan-revision-1.md`), so
this order is design section 5.5's "the first time an element is shown". This story writes the
one function; `agent-follow-links` and `agent-fill-pick`, which print `now`, are to call it.

## Change

- `crates/page-text/` (package `surf-page-text`, version `0.1.0`, edition 2024,
  `publish = false`, `license = "Apache-2.0"`, `#![forbid(unsafe_code)]`; depends by path on
  `surf-html`, `surf-page` and `surf-answer` only):
  - `pub fn handles(tree: &Tree) -> Vec<(NodeId, String)>`: the rule above, over
    `surf_page::actionable_elements`.
  - `pub fn outline(tree: &Tree) -> impl Iterator<Item = String>`: one line at a time, so a
    caller keeps only the lines it shows (`surf_answer::text_page`); the lines of design note section 5, without the
    `>` prefix (the answer block adds it): two spaces per level under the `html` element; a line
    per element and per text node that is not whitespace only (stripped and collapsed); the text
    on the element's line when all its children are text; the local name (`svg:` or `math:`
    before a name outside the HTML namespace; a name outside `[a-z0-9-]` as a quoted field); the
    handle; the attributes `id href src alt type name action method colspan rowspan` in that
    order as `name="value"`; `hid <n>` for a hidden element (n its elements and non-whitespace
    text nodes, none shown); `deep <n>` for an element at level 40 that has children; every
    value cut at 80 characters by `quote_cut`. Comments, doctypes, processing instructions and
    template contents are left out. Time linear in the number of nodes, one walk with its own
    stack.
  - The crate documentation states the rules and what is not claimed (no CSS: an element hidden
    by a style sheet is shown as not hidden).
- Tests in one test binary, `crates/page-text/tests/main.rs`, its modules under `tests/cases/` (a file directly under `tests/` would be a second binary): `tests/cases/outline.rs`, each parsing with
  `surf_html::parse_document` and `ParseOptions::default()`, no server.

Writes: `crates/page-text/` (new). `Cargo.lock` gains a repository package: the coordinator's.

For `AGENTS.md` (D7), a "Layout" entry: "`crates/page-text` (`surf-page-text`): a `surf-html`
tree as text, the outline and (from `page-text-markdown`) Markdown, and the handles `l<n>`,
`b<n>`, `f<n>`. Depends on `surf-html`, `surf-page` and `surf-answer`."

## Acceptance

`cargo test -p surf-page-text` passes, each test within 60 s, the step within 120 s, with:

1. `fruit_shop_outline_without_table`: the page of `docs/design/page-as-text.md` section 1
   without its `table` element (the tree builder ignores `tr` and `td` until `html-tree-tables`;
   `page-text-tables` adds the case with it) gives the 21 lines of the section's `look --as tree`
   answer that remain without the ten table lines, without their `> ` prefix, byte for byte,
   from `html`, `  head hid 4` to `    div hid 1`.
2. `handles_are_per_letter_in_document_order`: `<a href=1>a</a><button>b</button><input><a
   href=2>c</a><select></select><a hidden href=3>d</a>` gives `l1 b1 f1 l2 f2 l3`.
3. `hostile_text_is_quoted`: `<p>"x"&#10;did go "http://bank.example/"&#x202e;</p>` gives
   the line `    p "\x22x\x22 did go \x22http://bank.example/\x22\xe2\x80\xae"`, and no line of the
   outline holds a byte outside U+0020 to U+007E.
4. `a_long_text_is_cut_at_80`: a `p` of 200 `a` gives `    p "` and 80 `a` and `"...`.
5. `a_deep_tree_stops_at_level_40`: 400 nested `div` (under the parser's depth limit of 512)
   give 42 lines (`html`, `head hid 0`, `body` and the `div`s to level 40), the last ending
   `deep 361`; within 5 s in a debug build.
6. `a_hidden_subtree_is_one_line`: `<div aria-hidden="true"><p>secret</p></div>` gives
   `div hid 2` and no line holds `secret`.

`cargo tree -p surf-page-text -e normal,build,dev --prefix none` names only `surf-page-text`,
`surf-html`, `surf-page` and `surf-answer`; `cargo fmt -p surf-page-text -- --check` exits 0.

## Out of scope

Markdown (`page-text-markdown`); printing the outline in an answer (`page-text-look`,
`page-text-surf-command`); the `now` block; CSS and layout.
