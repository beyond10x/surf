---
format: aep.planning-md/3
id: story:view-stream-string-example
kind: story
status: active
title: Correct the string example in docs/view-stream.md
summary: The example at docs/view-stream.md:138 leaves characters raw that the rule above it escapes.
relations:
- decomposes: epic:agent-cli-target
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:02:28Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-21T03:02:28Z", actor: "human:timo", revision: 3}
---
## Context

Found in wave 3 (`view-page-prototype`, `docs/plan/agent-wave-3.md`). The code follows the rule;
the example and case 8 of `story:view-page-prototype` showed the escapes lost.

## Change

The example shows the escaped form the rule gives.

## Acceptance

A test of `surf-view` reads the example from the document and compares it with what the
server writes for the same value.
