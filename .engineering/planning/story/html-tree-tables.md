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
revision: 5
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
tagged `tables` by `docs/design/html-parser.md` (item 8) are this story's; from memory
`tables01.dat` and `quirks01.dat` are among them. Stories `html-tree-body`,
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
`crates/page/tests/tree_tables.rs` (new), `crates/html-conformance/expected/tree-construction.txt`.

## Acceptance

`slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 printing `tag body
n/n/n`, `tag formatting m/m/m` and `tag tables t/t/t` with the counts of `html-inventory`, and
`cargo test -p surf-html -p surf-page` passes, with
`a_select_in_a_table_is_listed_once_with_kind_select_and_no_value` among the tests run.

## Out of scope

`template` inside tables (story `html-tree-template`), MathML and SVG inside tables (story
`html-tree-foreign`), option selectedness (`story:agent-form-port`). Scripts: a `script` or
`style` in table mode is processed by the in head rules, with what story `html-tree-body` leaves
unsupported; nothing more.
