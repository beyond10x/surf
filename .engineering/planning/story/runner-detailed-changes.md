---
format: aep.planning-md/3
id: story:runner-detailed-changes
kind: story
status: active
title: Compute the detailed changed lines over the node tree
summary: Attribute, class, text and node changes between two states of one document, in the recording's grammar.
relations:
- depends_on: story:runner-formats
- depends_on: story:runner-replay-transport
- decomposes: epic:detailed-mode
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:02:28Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-21T03:02:28Z", actor: "human:timo", revision: 3}
---
## Context

A recording holds each action "in the detailed mode as far as that mode exists (attributes,
classes and text from the node tree; computed style later)" (epic, Outcome). The design defines
the lines (section 5.4): attribute set or removed, class added or removed, text changed, node
added, removed or moved, `~ b1 class +active`; a new document gives `changed page` and no
per-node lines (section 5.3); before computed style a `note` line says what is missing.
`epic:agent-cli-target` computes the normal-mode `changed` block and does not list the detailed
mode in its outcome, and `epic:html-parser` only makes the tree reachable "so that a later step
can compute what changed". So the detailed lines are computed here.

Before scripts, a page changes within one document only through control state (`fill`, `pick`),
so a real session produces few detailed lines; the tests build the two states directly.

Needs from `epic:html-parser`: the node tree type, reachable in process (element names,
attributes, text nodes, child order), and a node identity that stays the same for a node across
changes within one document, so two states can be compared node by node.

Needs from `epic:agent-cli-target`: the pair of page states the session compares for the
`changed` block, and the handle of each actionable node, so detailed lines name a handle where
the node has one and a tree path where it has not.

## Change

- In `crates/scenario` (package `surf-scenario`), which gains a path dependency on the
  html-parser's tree crate: a function from two states of one document to the detailed lines,
  in document order of the later state, in the grammar `story:runner-formats` gives them, with a
  closing `note detail: computed style not available` line until style is computed.
- Tests in `crates/scenario/tests/detail.rs`, one per line kind: attribute set, attribute
  removed, class added, class removed, text changed, node added, node removed, node moved; one for
  a node with a handle and one without; one for two states of different documents (no per-node
  lines); one for identical states (no line but the note).

Surfaces: `crates/scenario/src/` (a new module and `lib.rs`), `crates/scenario/Cargo.toml`,
`crates/scenario/tests/detail.rs`. Shares `lib.rs` and `Cargo.toml` with
`runner-replay-transport`, so it follows it.

## Acceptance

`cargo test -p surf-scenario` passes with the twelve named detail cases.

## Out of scope

Computed style changes (design section 9, step 8). The `--detail` option of `surf-agent`
(design section 4), which neither this epic nor `epic:agent-cli-target` claims. Writing the lines
into a recording (`runner-record-session`).

## Domain relations

A detailed line and a node of the tree, many to one: rests on the node identity
`epic:html-parser` declares, which no `ess/1` document holds yet (`spec/domains/document.yaml`,
the `UNMAPPED:` marker on the tree of nodes).
