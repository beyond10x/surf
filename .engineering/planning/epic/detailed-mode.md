---
format: aep.planning-md/3
id: epic:detailed-mode
kind: epic
status: active
title: 'The detailed mode: every change of the node tree and of computed style'
summary: --detail and the detailed lines of a recording; waits for a script engine, because nothing mutates a document's tree before one exists.
relations:
- depends_on: epic:scripts-in-a-page
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:02:25Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T03:02:26Z", actor: "human:timo", revision: 4}
---
## Context

The operator, 2026-11-21: "We will also, when we have some [...] detailed mode or something like that,
which is more for recording end-to-end sessions, then we see exactly, for example [...] a font
has now changed, or a button has now gained a certain class active or whatever". The accepted
design (`docs/design/agent-interface.md`, sections 4, 5.4 and step 8 of section 9) gives every
`surf-agent` command an option `--detail` and makes the detailed lines what a recording holds.

Critic round 1 of the agent interface plan (`docs/plan/agent-plan-revision-1.md`, decisions D2
and D3) found that nothing can produce such a change yet. A document's tree is built by parsing
and is not mutated before a script engine exists; a class gained or an attribute set needs a
script, and a style changed needs CSS. What an agent changes on a page before that is control
state, which the normal `changed` block already shows.

So the detailed mode is its own epic, after scripts. Two draft stories written for
`epic:e2e-scenario-runner` are its first material: `story:runner-detailed-changes` (detailed
lines over two states of a tree) and `story:runner-detail-option` (`--detail` on `surf-agent`
commands). Both were written before D2 and need rewriting when this epic is taken up.

## Outcome

- `--detail` on every `surf-agent` command that answers with a `changed` block: under the
  normal lines, every difference of the node tree (attribute set or removed, class added or
  removed, text changed, node added, removed or moved) and, once style is computed, every
  computed style property that changed.
- A recording made with `--record` holds the detailed lines for each action, in the block kind
  the recording format reserves for them.
- A scenario asserts a detailed line only where someone marked it.

## Needs first

A script engine that mutates the tree (its own design and epic; none exists), a node identity
that is stable across mutations, and for style changes the CSS cascade.

## Acceptance

Not written. It is written when the script engine's epic exists and says what a session can
change on a page.
