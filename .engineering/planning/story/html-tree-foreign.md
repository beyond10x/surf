---
format: aep.planning-md/3
id: story:html-tree-foreign
kind: story
status: implemented
title: Foreign content and the fragment parsing algorithm
summary: MathML and SVG in the tree, integration points, CDATA, and fragments for a context element; every tree case the note runs is tagged and attempted.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-tree-template
scope:
- confidence: cited
  path: crates/html-conformance/expected/tree-construction.txt
- confidence: cited
  path: crates/html/src/lib.rs
- confidence: cited
  path: crates/html/src/tokenizer.rs
- confidence: cited
  path: crates/html/src/tree_builder
- confidence: cited
  path: crates/html/src/tree_builder.rs
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-22T16:29:15Z", actor: "human:operator", revision: 10, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

MathML and SVG elements are parsed by the rules for foreign content: their own namespace, the
SVG tag name and attribute case adjustments, the foreign attributes (`xlink:`, `xml:`,
`xmlns`), the MathML text and HTML integration points, the breakout of HTML elements, and
CDATA sections allowed only inside foreign content (tree construction: the rules for parsing
tokens in foreign content; from memory, to be confirmed by this story). The fragment parsing
algorithm parses markup for a context element, and the data uses it in every
`#document-fragment` case. Both are tagged `foreign` by `docs/design/html-parser.md` (item 8):
by the inventory `math.dat` (8), `svg.dat` (8), `namespace-sensitivity.dat` (1),
`foreign-fragment.dat` (66) and `tests_innerHTML_1.dat` (81) are all `foreign`. surf never parses a fragment at run time; the algorithm
is taken because the standard's data tests the tree builder through it.

With this story every tag of the thematic files is claimed. The mixed files (tag `mixed`) are
story `html-tree-complete`'s.

## Change

- `crates/html/src/tree_builder/` (a `foreign` submodule): the rules for foreign content and
  their dispatch from the tree construction dispatcher, the adjusted current node, the
  adjustments of tag names and attributes, the integration points, and the tokenizer hook for
  CDATA answered from the adjusted current node.
- The fragment parsing algorithm in `crates/html/src/lib.rs` and the tree builder: the context
  element, the tokenizer state it picks, the `html` root, reset the insertion mode
  appropriately for a fragment, and the form element pointer.
- The limits keep applying.
- `NOT_YET_PARSED`: the entry `foreign-content` removed. The entries left still match `[a-z-]+`
  (`docs/plan/agent-plan-revision-2.md`, D31), held by
  `every_not_yet_parsed_entry_matches_a_z_and_hyphen` of story `html-tree-body`.
- `crates/html-conformance/expected/tree-construction.txt`: the counts raised.

Writes: `crates/html/src/lib.rs`, `crates/html/src/tree_builder.rs`,
`crates/html/src/tree_builder/`, `crates/html/src/tokenizer.rs` (the CDATA hook only),
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
476/476/476`, `tag formatting 22/22/22`, `tag tables 39/39/39`, `tag template 123/123/123` and `tag foreign
222/222/222` with the counts of `html-inventory`, and `cargo test -p surf-html` passes.

## Out of scope

The cases tagged `mixed` and the section marks (story `html-tree-complete`). Scripts: an SVG
`script` end tag in foreign content is where the standard processes the SVG script element;
surf pops it and runs nothing. The fragment algorithm's use by `innerHTML` and
`insertAdjacentHTML` needs a DOM interface surf does not have.

## As built

What differs from the text above, as of 280e8a7:

- Writes, "`crates/html/src/tokenizer.rs` (the CDATA hook only)": the hook also changes
  `crates/html/src/tokenizer/machine.rs`, where the markup declaration open state reads the
  flag the tree builder sets before each token (`set_cdata_allowed`). `tokenize` keeps the
  bogus comment.
- Change, the fragment parsing algorithm: the context element has no attribute and no
  ancestor, because `Context` in `crates/html/src/lib.rs` names only a namespace and a local
  name. A MathML `annotation-xml` context is therefore never an HTML integration point, and
  the form element pointer is set only when the context is itself a `form`
  (`crates/html/src/tree_builder/foreign.rs`, `parse_fragment` in
  `crates/html/src/tree_builder.rs`).
- Left open: the SVG `script` end tag pops the element and nothing runs (`open` line
  `svg-script-processing`).
