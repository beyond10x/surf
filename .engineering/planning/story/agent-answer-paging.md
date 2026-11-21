---
format: aep.planning-md/3
id: story:agent-answer-paging
kind: story
status: active
title: 'Page a long answer: --max-lines and look --from'
summary: 'Deferred: past a line limit an answer ends with a more line naming the handle to continue from.'
tags:
- deferred
relations:
- informed_by: epic:agent-cli-target
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:02:26Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-21T03:02:26Z", actor: "human:timo", revision: 3}
---
## What

`--max-lines <n>` on every command (default 120) and `look --from <handle>`: past the limit an answer
ends with `more <n> elements: look --from <handle>` (`docs/design/agent-interface.md`, sections 4 and
5.2).

## Why deferred

Whether to rank by visibility or keep document order is open (design section 10, question 3), and
the epic does not promise it. Decision D15 of `docs/plan/agent-plan-revision-1.md` gives it this
carrier.

## What would un-defer it

A page an agent is sent to whose `now` block costs more than an agent can read, or the operator
settling question 3.

## Acceptance

`cargo test -p surf-session` passes with a test over a page of 400 links in which `look` prints 120
element lines and the `more` line, and `look --from` the handle it names prints the next 120.
