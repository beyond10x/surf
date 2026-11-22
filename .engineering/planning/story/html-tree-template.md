---
format: aep.planning-md/3
id: story:html-tree-template
kind: story
status: implemented
title: The template element and its insertion mode
summary: Template contents and the stack of template insertion modes; every tree case tagged up to template passes.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-tree-tables
- depends_on: story:html-actionable-elements
scope:
- confidence: cited
  path: crates/html-conformance/expected/tree-construction.txt
- confidence: cited
  path: crates/html/src/tree.rs
- confidence: cited
  path: crates/html/src/tree_builder
- confidence: cited
  path: crates/html/src/tree_builder.rs
- confidence: cited
  path: crates/page/tests/main.rs
- confidence: cited
  path: crates/page/tests/tree_template.rs
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-22T16:29:13Z", actor: "human:operator", revision: 12, decided_on: {"recorded":{"test_result":1,"verification":1}}}
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
`crates/html/src/tree_builder/`, `crates/page/tests/tree_template.rs` (new), `crates/page/tests/main.rs` (one `mod` line),
`crates/html-conformance/expected/tree-construction.txt`.

Bounds for every command of this story. Every build and test runs through the build pool:
`cargo` is wrapped by the pool's shim, every other command that builds or tests gets the `slot`
prefix. The pool ends a job after 10 minutes; a command that needs longer names its time with
`slot --timeout <t>` or `SLOT_TIMEOUT=<t>`, at most 30 minutes; a `timeout` wrapped around the
command extends nothing. A script that measures or waits starts without the `slot` prefix; the
builds and test runs inside it carry it. No test runs over 60 s and the test step ends within
120 s. No Python.

## Acceptance

`slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 printing `tag body
476/476/476`, `tag formatting 22/22/22`, `tag tables 39/39/39` and `tag template 123/123/123` (the counts of
`html-inventory`), and `cargo test -p surf-html -p surf-page` passes, with
`template_contents_are_not_listed` among the tests run.

## Out of scope

Declarative shadow roots unless the note takes them on. Scripts: template contents are inert;
nothing in them is run. Custom elements inside a template are not upgraded.

## Found before this story's wave

Two findings of waves 8 and 9 that this story's reading takes up
(`docs/plan/agent-wave-8.md`, and the reading of `story:html-actionable-elements` before
wave 9):

- A `template` start tag in the head goes through the generic rules, and the rule that
  leaves the head then pops the current node, which is the `template` and not the `head`
  (`crates/html/src/tree_builder/modes.rs`, the in head rules). `body` becomes a child of
  `head`. Measured by a probe on the tree after wave 8. For `surf-page` every element of
  such a page is hidden by the `head` rule, until this story.
- `tag template` was 30 half-way through `story:html-tree-body` and 29 after it; inferred,
  not proven: `template.dat` case 6, whose end tag `template` goes to "any other end tag",
  which stops at a special element where the standard's own rule pops until the `template`.
  The reading of `story:html-tree-formatting` measured that the adoption agency algorithm
  brings the tag to 30 again (`template.dat` case 84).

## As built

What differs from the text above, as of 280e8a7:

- Change, "`crates/html/src/tree.rs`: template contents as a fragment": the tree held template
  contents already before this story (`Tree::create_element` gives an HTML `template` its
  contents), so the story changed nothing in `tree.rs`.
- Change, "The limits keep applying": the step limit is now also checked after the end-of-file
  token, whose in template rule scans the stack of open elements (`run` in
  `crates/html/src/tree_builder.rs`, held by
  `adversary_tree::the_end_of_file_token_is_held_to_the_step_limit`).
- Change, "The limits keep applying": the seeded test
  `limits::seeded_1_200_inputs_end_within_bounds` in `crates/html/tests/limits.rs`, a file
  outside this story's Writes, runs with a node limit of 25 instead of 30. With template
  contents parsed as the standard says, the same inputs build fewer nodes, and 30 stopped 4 of
  the 1,200 inputs, fewer than the 5 the test asserts; 25 stops 8 (the comment beside the
  limit).
- Left open: the snapshot's content patching, a `template` start tag with a `for` attribute, is
  not taken. The standard moves the template's content into the element it marks, and surf
  keeps it in the template contents (`open` line `template-content-patching`).
- Left open: declarative shadow roots are not attached (`open` line `declarative-shadow-roots`,
  and the comment of `crates/html/src/tree_builder/template.rs`).
