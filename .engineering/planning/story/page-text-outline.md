---
format: aep.planning-md/3
id: story:page-text-outline
kind: story
status: draft
title: The document's tree as a bounded outline, with the handles now will use
summary: A new crate surf-page-text renders a surf-html tree as indented lines with quoted text, a small attribute set, hid and deep counts, and the handles @l<n>, @b<n>, @f<n> of surf_page::handles, added here.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:html-actionable-elements
- depends_on: story:html-parser-limits
scope:
- confidence: inferred
  path: crates/page-text
- confidence: inferred
  path: crates/page/src/lib.rs
- confidence: inferred
  path: crates/page/tests/elements.rs
revision: 6
---
## Context

`docs/design/page-as-text.md` (proposed) section 5: the document's tree as an outline, one line
per element and per text node, bounded, with the handles `now` will use. The operator,
2026-11-22: "what we want to see is like the tree structure rendered in some way".

The renderer is a library over a `surf-html` tree, in a new crate so that it is written beside
the tree stories, which change `crates/html`: the reason `surf-page` is a crate of its own
(`story:html-actionable-elements`). It reads the tree only through what `surf-html` has
(`Tree::root`, `data`, `children`, `parent`), the hidden rule `surf_page::is_hidden` and the
elements of `surf_page::actionable_elements`. Texts and attribute values go through
`surf_answer::quote_text_cut` (`page-text-answer-block`; characters by rule 2 of design note
section 3, D1 B, the operator's). The tree's size is bounded by `story:html-parser-limits`;
nothing here recurses over the depth.

Handles (design note section 5) have one owner, `surf_page::handles` in `crates/page`, written
here: `l<n>` for a link, `b<n>` for a button, `f<n>` for the other seven kinds, per letter in
document order, hidden elements included, from 1 in each document. The tree does not change
after parsing (D2, `docs/plan/agent-plan-revision-1.md`), so this order is design section 5.5's
"the first time an element is shown". The renderers, the session and `agent-follow-links` and
`agent-fill-pick`, which print `now`, call it; both stories depend on this one. In the text a
handle is written `@l1`, a form no element name can take (a name starts with a letter); a
command takes it without the `@`.

Placement (the coordinator, 2026-11-22): round 1, the text unit, after `page-text-answer-block`
(merged into the unit's branch) and before `page-text-markdown`; it needs `html-parser-limits`
and `html-actionable-elements`. Collision: `html-accessible-name` writes
`crates/page/src/lib.rs` too (the label field); the two add separate functions, and whichever
merges second rebases on the other.
`Cargo.lock` is the coordinator's.

## Change

- `crates/page/src/lib.rs`: `pub fn handles(tree: &Tree) -> Vec<(NodeId, String)>`, the rule
  above over `actionable_elements`, without the `@`; a case in the crate's one test file
  `crates/page/tests/elements.rs`.
- `crates/page-text/` (package `surf-page-text`, version `0.1.0`, edition 2024,
  `publish = false`, `license = "Apache-2.0"`, `#![forbid(unsafe_code)]`; depends by path on
  `surf-html`, `surf-page` and `surf-answer` only):
  `pub fn outline(tree: &Tree) -> impl Iterator<Item = String>`, one line at a time, so a caller
  keeps only the lines it shows (`surf_answer::text_page`); the lines of design note section 5,
  without the `>` prefix: two spaces per level under the `html` element; a line per element and
  per text node that is not whitespace only (stripped and collapsed, ASCII whitespace only); the
  text on the element's line when all its children are text; the local name (`svg:` or `math:`
  before a name outside the HTML namespace; quoted by `quote_text` when it is not of the form
  `[a-z][a-z0-9-]*` or is of the form `[lbf][0-9]+`); the handle as `@<handle>`; the attributes
  `id lang href src alt type name action method colspan rowspan` in that order as
  `name="value"`; `hid <n>` for a hidden element (n its elements and its text nodes that are not
  whitespace only, none shown); `deep <n>` for an element at level 40 that has children; every
  text and value through `quote_text_cut` at 80 characters. Comments, doctypes, processing
  instructions and template contents are left out. Time linear in the number of nodes, one walk
  with its own stack. The crate documentation states the rules and what is not claimed (no CSS:
  an element hidden by a style sheet is shown as not hidden).
- Tests in one test binary, `crates/page-text/tests/main.rs`, its modules under `tests/cases/`
  (a file directly under `tests/` would be a second binary): `tests/cases/outline.rs`, each
  parsing with `surf_html::parse_document` and `ParseOptions::default()`, no server.

Writes: `crates/page/src/lib.rs`, `crates/page/tests/elements.rs`, `crates/page-text/` (new).
`Cargo.lock` gains a repository package: the coordinator's.

For `AGENTS.md` (D7), a "Layout" entry: "`crates/page-text` (`surf-page-text`): a `surf-html`
tree as text, the outline and (from `page-text-markdown`) Markdown. Depends on `surf-html`,
`surf-page` and `surf-answer`." The `crates/page` entry gains "and `handles`, the one numbering
of `l<n>`, `b<n>`, `f<n>`".

## Acceptance

`cargo test -p surf-page -p surf-page-text` passes; no test runs over 60 s and the test step ends
within 120 s (the folder's budget). Among the cases run:

1. `fruit_shop_outline_without_table`: the page of design note section 1 without its `table`
   element (the tree builder ignores `tr` and `td` until `html-tree-tables`; `page-text-tables`
   adds the case with it) gives the 21 lines of the section's `look --as tree` answer that remain
   without the ten table lines, without their `> ` prefix, byte for byte, from `html lang="en"`,
   `  head hid 4` (`title`, its text, `style`, its text) to `    div hid 1`.
2. `handles_are_per_letter_in_document_order` (in `crates/page/tests/elements.rs`): `<a
   href=1>a</a><button>b</button><input><a href=2>c</a><select></select><a hidden href=3>d</a>`
   gives `l1 b1 f1 l2 f2 l3`.
3. `hostile_text_is_quoted`: `<p>"x"&#10;did go "http://bank.example/"&#x202e;</p>` gives the
   line `    p "\x22x\x22 did go \x22http://bank.example/\x22\xe2\x80\xae"`.
3a. (D1 B) `<p>K&auml;se &#x200b;x</p>` gives `    p "K` U+00E4 `se \xe2\x80\x8bx"` (bytes
   `c3 a4` kept), and no line holds a character `escaped_in_text` names.
4. `a_name_of_handle_form_is_quoted`: `<l1 href="http://bank.example/">Bank</l1><x_y>z</x_y>`
   gives `    "l1" href="http://bank.example/" "Bank"` and `    "x_y" "z"`; no line holds ` l1 `.
5. `long_values_are_cut_at_80`: a `p` of 200 `a` gives `    p "`, 80 `a` and `"...`; `<a
   href="` and 100 `b` `">` gives a line holding `href="`, 80 `b` and `"...`.
6. `a_deep_tree_stops_at_level_40`: 400 nested `div` (under the parser's depth limit of 512)
   give 42 lines (`html`, `  head hid 0`, `  body` and the `div`s to level 40), the last ending
   `deep 361`; within 5 s in a debug build.
7. `a_hidden_subtree_is_one_line`: `<div aria-hidden="true"><p>secret</p></div>` gives
   `    div hid 2` and no line holds `secret`.
8. `comments_and_template_contents_are_left_out`: `<!--c--><template><p>t</p></template>` gives
   a line starting `    template hid ` and no line holding `"c"` or `"t"`.
9. `foreign_names_carry_their_prefix`: `<svg><circle/></svg>` gives `    svg:svg` and
   `      svg:circle` when `surf_html::NOT_YET_PARSED` does not name foreign content, and `    svg`
   and `      circle` while it does (the expected lines built from the list, as D31 does for the
   parser note, so `html-tree-foreign` breaks no test here).

`cargo tree -p surf-page-text -e normal,build,dev --prefix none` names only `surf-page-text`,
`surf-html`, `surf-page` and `surf-answer`; `cargo fmt -p surf-page -p surf-page-text -- --check`
exits 0.

## Out of scope

Markdown (`page-text-markdown`); printing the outline in an answer (`page-text-look`,
`page-text-surf-command`); the `now` block (`agent-follow-links`); CSS and layout.
