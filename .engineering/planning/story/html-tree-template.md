---
format: aep.planning-md/3
id: story:html-tree-template
kind: story
status: active
title: The template element and its insertion mode
summary: Template contents and the stack of template insertion modes; every tree case tagged up to template passes.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-tree-tables
- depends_on: story:html-actionable-elements
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 5}
---
## Context

A `template` element holds its contents in a separate document fragment, and its own insertion
mode with a stack of template insertion modes decides how the tokens inside it are treated
(tree construction: in template, and the `template` rules of in head; from memory, to be
confirmed by this story). The data writes template contents as a `content` line in the tree
format, which the harness already writes (story `html-conformance-harness`). Cases tagged
`template` by `docs/design/html-parser.md` (item 8) are this story's; by the inventory `template.dat`
holds 121 of them. Whether declarative shadow roots (the `shadowrootmode` attribute) are taken on
is item 3 of the note: proposed, not taken on, and their cases tagged `not-run`.

The test of `crates/page` that reads template parsing is here, where the parsing is
(`docs/plan/agent-plan-revision-1.md`, D12), so this story depends on
`story:html-actionable-elements`.

## Change

- `crates/html/src/tree.rs`: template contents as a fragment owned by the `template` element,
  outside the document's tree order.
- `crates/html/src/tree_builder/` (a `template` submodule): the `template` start and end tag
  rules of in head, the in template mode, the stack of template insertion modes, and every place
  the other modes consult it (reset the insertion mode appropriately, end of file, the
  appropriate place for inserting into a `template`).
- The limits keep applying; content inside template contents counts toward depth and nodes.
- `NOT_YET_PARSED`: the entry `template` removed. The entries left still match `[a-z-]+`
  (`docs/plan/agent-plan-revision-2.md`, D31), held by
  `every_not_yet_parsed_entry_matches_a_z_and_hyphen` of story `html-tree-body`.
- `crates/page/tests/tree_template.rs`: `template_contents_are_not_listed`.
- `crates/html-conformance/expected/tree-construction.txt`: the counts raised.

Writes: `crates/html/src/tree.rs`, `crates/html/src/tree_builder.rs`,
`crates/html/src/tree_builder/`, `crates/page/tests/tree_template.rs` (new),
`crates/html-conformance/expected/tree-construction.txt`.

## Acceptance

`slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 printing `tag body
476/476/476`, `tag formatting 22/22/22`, `tag tables 39/39/39` and `tag template 123/123/123` (the counts of
`html-inventory`), and `cargo test -p surf-html -p surf-page` passes, with
`template_contents_are_not_listed` among the tests run.

## Out of scope

Declarative shadow roots unless the note takes them on. Scripts: template contents are inert;
nothing in them is run. Custom elements inside a template are not upgraded.
