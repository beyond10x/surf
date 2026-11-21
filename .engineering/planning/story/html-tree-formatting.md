---
format: aep.planning-md/3
id: story:html-tree-formatting
kind: story
status: active
title: 'Misnested formatting elements: the adoption agency algorithm'
summary: The adoption agency algorithm and Noah's Ark; every tree case tagged body or formatting passes.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-tree-body
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 5}
---
## Context

Misnested formatting elements (`<b><i></b></i>`, `<a><p></a>`) are repaired by the adoption
agency algorithm, with the "Noah's Ark" clause bounding duplicates in the list of active
formatting elements and the rule that a new `a` first closes an open one (tree construction, in
body, from memory, to be confirmed by this story). Story `html-tree-body` left the algorithm as a
hook that pops to the matching element. Cases tagged `formatting` by
`docs/design/html-parser.md` (item 8) are this story's; from memory `adoption01.dat` and
`adoption02.dat` are among them. Links are `a` elements, so a page with links parses as the
standard says only after this story.

## Change

- `crates/html/src/tree_builder/` (a `formatting` submodule): the adoption agency algorithm with
  its outer and inner loop limits, Noah's Ark in "push onto the list of active formatting
  elements", the `a`, `nobr` and formatting end tag rules of in body, and "any other end tag".
- `NOT_YET_PARSED`: the entry `misnested-formatting` removed. The entries left still match `[a-z-]+`
  (`docs/plan/agent-plan-revision-2.md`, D31), held by
  `every_not_yet_parsed_entry_matches_a_z_and_hyphen` of story `html-tree-body`.
- `crates/html-conformance/expected/tree-construction.txt`: the counts raised.

Writes: `crates/html/src/tree_builder.rs`, `crates/html/src/tree_builder/`,
`crates/html-conformance/expected/tree-construction.txt`.

## Acceptance

`slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 printing `tag body
n/n/n` and `tag formatting m/m/m` with the counts of `html-inventory`.

## Out of scope

Foster parenting inside the adoption agency algorithm when the common ancestor is a table
(story `html-tree-tables`). Scripts: nothing in this algorithm needs a script engine; custom
elements created while cloning formatting elements run no constructor.
