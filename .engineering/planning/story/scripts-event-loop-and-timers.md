---
format: aep.planning-md/3
id: story:scripts-event-loop-and-timers
kind: story
status: draft
title: 'Scripts run after the page has loaded: a task queue and timers'
summary: 'surf gains an event loop per document: tasks, setTimeout and setInterval over the session''s clock, and the microtask checkpoint after each task; D2 (no mutation after parsing) ends.'
relations:
- decomposes: epic:app-journeys-in-surf
- depends_on: story:scripts-document-port
- depends_on: story:swell-0-9-0
revision: 2
---
## Context

"Timers and the event loop: no task queue exists in surf [...] Scripts run only while parsing,
so D2 (`docs/plan/agent-plan-revision-1.md`: no mutation after parsing) still holds afterwards"
(`.engineering/planning/epic/scripts-in-a-page.md:120-122`). `story:swell-0-9-0` runs the
microtask checkpoint after each script; nothing runs a job after the last script. swell makes
no timeout job by any engine path at `0.9.0` (HOST-85 and HOST-86 `pending slice 14`,
`crates/swell/src/host/hooks.rs:7-9 @ 0.9.0`), so timers are host functions of surf.

## What it holds

- A task queue per document and the rule for when the session's process runs it: after load,
  and between an agent's calls, each under the document's limits (steps, memory, wall clock:
  `docs/design/scripts.md`, Q4), with a bound on timers and on tasks per answer.
- `setTimeout`, `setInterval`, `clearTimeout`, `clearInterval`, `queueMicrotask` over the
  session's clock; what an agent's `look` waits for (a quiet queue or a bound). For
  `queueMicrotask` its unit takes `HostError::MemoryBudget` out of `surf_defect!()` in
  `crates/script/src/runner.rs` and handles it at the call (`story:swell-0-9-0`, decision 5), and
  reads again the queue clearing after a limit that `story:swell-0-9-0` sets in its decision 6.
- The end of D2: every reader of the tree that assumed it does not change after parsing.

## Waits for

`story:scripts-document-port`, `story:swell-0-9-0`; a decision on the wait rule of `look`.
