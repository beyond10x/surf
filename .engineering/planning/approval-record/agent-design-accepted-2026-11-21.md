---
format: aep.planning-md/3
id: approval-record:agent-design-accepted-2026-11-21
kind: approval-record
status: draft
title: The agent interface design is the basis for the epics
revision: 3
---
## What was put to the operator

On 2026-11-21 the coordinating agent reported that the design draft
`docs/design/agent-interface.md` was ready and asked:

> Decision: is this design the basis for the epics?
> - A: yes. I rewrite the 2 draft epics from it, add a parser epic, and break them into draft
>   stories. Recommended.
> - B: yes with changes; name the section.
> - C: wrong order; scripts or HTTPS come before the agent CLI.
> - Default on silence: A, drafts only, no implementation.

## The answer

The operator, 2026-11-21: "A"

## What it covers

- `docs/design/agent-interface.md` is the basis for `epic:agent-cli-target`,
  `epic:html-parser` and `epic:e2e-scenario-runner`.
- The epics are rewritten from it and broken down into draft stories.

## What it does not cover

- Accepting the epics or any story: they stay `draft` until he accepts them.
- Any implementation.
- The decisions the design lists in its section 11 as his to overrule: he has not been asked
  about each, and they stand as proposals he read past, not as decisions he took.

## Second statement, after reading the design

The operator, 2026-11-21, after the design was opened for him in his editor: "Alright, nice. So I mean,
I am accepting it". This accepts the design document. It still does not accept the epics or any
story, which did not exist in full when he said it.
