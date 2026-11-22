---
format: aep.planning-md/3
id: story:html-tree-tables
kind: story
status: active
title: Tables, foster parenting and select
summary: The table insertion modes, foster parenting and the select rules; every tree case tagged up to tables passes.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-parser-limits
- depends_on: story:html-actionable-elements
- depends_on: story:one-test-binary-per-crate
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 5}
---
## Context

Tables have their own insertion modes, and content that is not allowed where it appears in a
table is moved in front of the table by foster parenting (tree construction: in table, in table
text, in caption, in column group, in table body, in row, in cell, and the "appropriate place
for inserting a node" with foster parenting enabled; from memory, to be confirmed by this
story). `select` has its own parsing rules, which the standard changed for the customizable
`select` element (from memory, 2025, to be confirmed against the standard and the data at the
pinned commit); this story takes whatever the standard says at the version the note pins. Cases
tagged `tables` by `docs/design/html-parser.md` (item 8) are this story's; by the inventory
`tables01.dat` (17) and `quirks01.dat` (4) hold most of them. Stories `html-tree-body`,
`html-tree-formatting` and `html-parser-limits` wrote the files this one changes.

The test of `crates/page` that reads `select` parsing is here, where the parsing is
(`docs/plan/agent-plan-revision-1.md`, D12), so this story depends on
`story:html-actionable-elements`. A `select` element is listed with no value (D11).

## Change

- `crates/html/src/tree_builder/` (a `tables` submodule): the table insertion modes, the pending
  table character tokens, clearing the stack back to a table, table, table body or row context,
  closing a cell, the table rules of in body (a `table` start tag closing a `p` outside quirks
  mode), foster parenting, also where the adoption agency algorithm reaches it, and the `select`
  rules of the pinned version.
- The limits of `crates/html/src/limits.rs` apply to everything added; `tests/limits.rs` keeps
  passing.
- `NOT_YET_PARSED`: the entry `tables-and-select` removed. The entries left still match `[a-z-]+`
  (`docs/plan/agent-plan-revision-2.md`, D31), held by
  `every_not_yet_parsed_entry_matches_a_z_and_hyphen` of story `html-tree-body`.
- `crates/page/tests/tree_tables.rs`: `a_select_in_a_table_is_listed_once_with_kind_select_and_no_value`.
- `crates/html-conformance/expected/tree-construction.txt`: the counts raised.

Writes: `crates/html/src/tree_builder.rs`, `crates/html/src/tree_builder/`,
`crates/page/tests/tree_tables.rs` (new), `crates/page/tests/main.rs` (one `mod` line), `crates/html-conformance/expected/tree-construction.txt`.

## Acceptance

`slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 printing `tag body
476/476/476`, `tag formatting 22/22/22` and `tag tables 39/39/39` with the counts of `html-inventory`, and
`cargo test -p surf-html -p surf-page` passes, with
`a_select_in_a_table_is_listed_once_with_kind_select_and_no_value` among the tests run.

## Out of scope

`template` inside tables (story `html-tree-template`), MathML and SVG inside tables (story
`html-tree-foreign`), option selectedness (`story:agent-form-port`). Scripts: a `script` or
`style` in table mode is processed by the in head rules, with what story `html-tree-body` leaves
unsupported; nothing more.

## Found before this story's wave

From wave 9 (`docs/plan/agent-wave-9.md`), for this story's own reading before its wave.

- `surf-page` (`crates/page`) lists content of a `select` that is not an `option` (a `button`
  in a `select`, an `a` in an `option`) as visible rows: the parser does not build a `select`
  by its own rules yet, and the rule for "hidden" has no line for it. When this story builds
  the `select`, say what the snapshot's rules leave inside one and whether the rule for
  "hidden" in `docs/design/html-parser.md` needs a line.
- The tests of `crates/page/tests/elements.rs` use a `select` only as
  `<select …><option>o</select>`, in five tests; by the fix unit's reading of the snapshot
  that tree is the same before and after this story. If one of them changes, the reading was
  wrong: report it.
- Step 4.5 of the adoption agency algorithm (the formatting element is on the stack and not
  in scope: return, the list keeps the entry) is reached only through `table`, `select` and
  `template` today, and a planted defect there (the entry dropped too) passes every test and
  every conformance count. A test for it belongs to the first of the three stories that
  makes the scope boundary real.
