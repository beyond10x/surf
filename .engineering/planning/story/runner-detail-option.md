---
format: aep.planning-md/3
id: story:runner-detail-option
kind: story
status: active
title: --detail on surf-agent commands shows the detailed changes
summary: Every surf-agent command that answers with a changed block takes --detail and then lists attribute, class and text changes too.
relations:
- depends_on: story:runner-detailed-changes
- decomposes: epic:detailed-mode
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:02:28Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-21T03:02:28Z", actor: "human:timo", revision: 3}
---
## Context

The accepted design (`docs/design/agent-interface.md`, sections 4 and 5.4) gives every
`surf-agent` command an option `--detail`: under `changed`, the answer then lists every
difference of the node tree and not only of actionable elements. The operator, 2026-11-21: "when
we have some [...] detailed mode [...] which is more for recording end-to-end sessions, then we
see exactly, for example [...] a button has now gained a certain class active".

Both decomposers found that no story claims the option: `epic:agent-cli-target` leaves the
detailed mode out of scope, and this epic's `story:runner-detailed-changes` computes the
detailed lines for recordings only. The coordinating agent assigned the option to this epic,
beside that story, on 2026-11-21.

What exists when this story starts: the detailed lines of `story:runner-detailed-changes`
(`crates/scenario`), the `changed` block and the session core of `story:agent-change-and-log`,
the command-line document `cli/surf-agent.yaml` with its hand-written parser and drift test.

## Change

- `cli/surf-agent.yaml` and the parser in `crates/agent/`: a flag `--detail` on every command
  that answers with a `changed` block. The document and the parser change together, so the drift
  test never fails in between.
- The session process (`crates/session/`): with the flag, the answer's `changed` block holds the
  normal lines followed by the detailed lines for the same action, computed by the code of
  `story:runner-detailed-changes`. Without the flag the answer is byte for byte what it was.
- `docs/agent-answer.md`: the grammar of a detailed line inside `changed`, and the `note` line
  the answer prints while computed style does not exist ("detail: attributes, classes and text
  only").
- `crates/answer/`: rendering of the detailed lines, every value quoted and escaped like the
  rest of the answer.

Surfaces written: `cli/surf-agent.yaml`, `crates/agent/`, `crates/session/`, `crates/answer/`,
`docs/agent-answer.md`, `README.md`.

## Acceptance

`cargo test -p surf-session` passes with named cases: `fill` with `--detail` on a page whose
form control changes shows the normal `~` line and the attribute-level line for the same
control; the same command without `--detail` prints the same bytes as before this story (a
golden file); a page value containing a line break and a double quote appears escaped in a
detailed line; the answer with `--detail` carries the `note` line about what the detailed mode
does not see yet. The drift test between `cli/surf-agent.yaml` and the parser passes.

## Out of scope

Computed style changes (no CSS exists). Detailed lines for a new document: a navigation prints
the `changed page` line only, as in the normal mode.
