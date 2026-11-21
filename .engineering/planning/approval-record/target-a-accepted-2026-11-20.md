---
format: aep.planning-md/3
id: approval-record:target-a-accepted-2026-11-20
kind: approval-record
status: draft
title: Operator accepted the first implementation target and its waves
relations:
- decides: epic:headless-rust-target
- decides: story:net-fetch-failure-reasons
- decides: story:generated-rust-workspace
- decides: story:http-fetch-library
- decides: story:memory-ports
- decides: story:headless-cli
- decides: story:rust-conformance
revision: 2
---
## Which stop

Two stops, taken on the operator's words in this session and not on a question:

1. the move of epic `headless-rust-target` and its six stories from `draft` to `active`;
2. the stage-1 stop of every wave of the plan derived from those six stories.

The operator was present. This is not a non-interactive bypass.

## The operator's words (The operator, 2026-11-20, this session)

- On the first implementation target, answering options A to D: "A (for B - file an issue - via
  bot token)". A is "Rust, all 5 components in one process, headless command line".
- "allright, lets start the implementation"
- "use opus agents for it using /implementing except that you only review once after a bigger
  junk of work, not for every single story/sub-item"

## What was decided

- The epic and the six stories are the decomposition of target A. They were drafted by the
  coordinating agent, argued by the four plan critics in two rounds (round 1: one approve, three
  needs-revision with seven findings, all seven recorded `fixed`; round 2: four approve), and are
  moved to `active` as the operator's act, run by the coordinator.
- The waves of the plan run without a stop between them. The wave pages are still written.
- Two deviations from the wave procedure, both the operator's:
  - implementors are dispatched on the Opus model;
  - the adversary does not attack each unit as it returns. It attacks once per larger piece of
    work: once after the wave that makes `surf <url>` run (story `headless-cli`), over everything
    merged up to there, and once after story `rust-conformance`. A unit still merges only on a
    green package-scoped gate and a verified claim.

## The commits this covers

The operator's standing rule admits "an AEP wave's own commits". For each wave: one commit per
unit, the merges into the integration branch, the opening and closing store commits, and the
merge into `main` once the gate is green. Before the first wave: one opening commit on `main`
that holds what the repository had uncommitted at that point (the specification, the project
files, the planning store and the plan pages), because the repository had no commit to branch
from. No push, no tag, no release, no remote.

## What the operator was not asked

Whether the opening commit on `main` may be made. It is local, the repository has no remote, and
it is reported in the closing report of the first wave.
