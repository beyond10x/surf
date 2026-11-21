---
format: aep.planning-md/3
id: decision-blocker:agent-session-history-home
kind: decision-blocker
status: cleared
title: Nobody has decided which component holds the session history that back and forward move through
summary: 'A history entry has no home: navigator, shell or agent; back and forward of epic:agent-cli-target wait for it.'
relations:
- blocks: epic:agent-cli-target
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-11-20T23:48:51Z", actor: "human:timo", revision: 3}
---
## Question

Which component holds the session history that `back` and `forward` move through?

`epic:agent-cli-target` lists `back` and `forward` among its commands and "goes back" in its
acceptance. `docs/design/agent-interface.md` section 4 maps them to agent commands `GoBack` and
`GoForward` and to the core operation "`Navigate` to a history entry". No `ess/1` document
declares a history entry, and neither the design nor the epic says which component holds one.

## The undecided relation

A history entry and its holder: one holder has many entries, ordered, one of them current; the
holder owns them and they end with it. The candidates:

- `surf.navigation` (navigator): entries per tab, as the HTML Standard gives a traversable
  navigable its session history (from memory); `GoBack` maps to a traverse command there. The
  navigator already "decides what each tab loads" (`spec/components.yaml`).
- `surf.shell` (shell): entries on the `Tab`, which is the navigable.
- `surf.agent` (agent): entries on the `Session`, from its own `Go`, `Follow` and submissions.
  Not the standard: the history would not exist for `surf <url>` or a tab no agent holds.

Undecided with it: what traversing to an entry made by a `POST` does (fetch again with `POST`,
fetch with `GET`, refuse), since surf has no cache.

## What it stops

The `back` and `forward` commands and the clause "goes back" of the acceptance of
`epic:agent-cli-target`. No story is drafted for them. `epic:e2e-scenario-runner` lists `Back`
and `Forward` among the commands its runner executes (story `runner-run`).

## The answer the decomposer would take

The navigator, per tab, following the HTML Standard: one entry per committed navigation, the
current entry held by the navigator, a command that traverses by a delta and navigates to the
entry address with `GET`, a new navigation dropping the entries after the current one; traversing
to an entry made by `POST` refused with a reason until a cache exists. Then two stories: a
specification story (navigator entries and traversal, agent `GoBack` and `GoForward`, their
bindings) after `agent-request-body-spec`, and a session story (`back`, `forward` on the command
line) after `agent-follow-links`.

## Decision

## Decision

Taken on 2026-11-21 by the coordinating agent; the operator was told and can overrule it.

Session history lives in the navigator, per tab, as the HTML Standard gives a navigable its
session history (the section on session history and its session history entries, named from
memory, to be confirmed by story `agent-history-spec`). Traversing to an entry made by `POST` is
refused until a cache exists, as proposed above.

Stories drafted behind it: `agent-history-spec` (the specification) and `agent-back-forward`
(`back` and `forward` on the command line). The move to `cleared` is left to the coordinator or
the operator: the decomposer role moves no artifact.
