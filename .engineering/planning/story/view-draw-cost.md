---
format: aep.planning-md/3
id: story:view-draw-cost
kind: story
status: draft
title: The live view draws a session once per change, not once per load
summary: One draw of a session with 331 entries held the page's main thread for 0.3 to 2.9 s at a load of 114 to 153, and a reconnect answered by a reset runs two loads that each draw it; draft, candidates not chosen
relations:
- decomposes: epic:live-view
- informed_by: story:view-live-means-stream-open
scope:
- confidence: cited
  path: crates/view/web/view.js
revision: 2
---
## Context

Measured on 2026-11-23 on `impl/round2-view` at `505f60e`, while a red page test was diagnosed
(`story:view-live-means-stream-open`, "As found while building"): 32 executions of
`adversary-page.spec.mjs`:74 with a log planted in the page, at a load of 114 to 153.

- Each load of the selected session draws the whole timeline again (`renderSelected()` after
  `loadAll()` in `crates/view/web/view.js`). With 331 entries and 27 actions one draw took 282
  to 434 ms of the page's main thread on average per job, and up to 2,928 ms.
- After a reconnect that is answered by a `reset`, two loads run at once: the one `open()`
  starts and the one the `reset` starts after `state.forget()`. Both fetch the same detail
  (125,238 bytes each in the measurement) and both draw it.
- While a draw runs, the page answers nothing: a click, a key and a test's check wait. In one
  execution no check of the page completed for 5 s.

## Change (to be decided when the story is taken)

Candidates, not yet chosen:

- one load in flight: a load that a later one has overtaken (a `reset` after `open()`) is
  dropped or not drawn;
- a draw that writes only what changed, or at most one draw per animation frame.

## Acceptance (draft)

1. After a reconnect answered by a `reset`, the page asks `GET /api/sessions/<id>` once, or
   draws once; a page test counts it.
2. The draw of a session with 331 entries holds the main thread for at most a stated time on a
   host without load; the bound and how it is measured are decided with the change.

## Out of scope

- The words of `#live-word` (`story:view-live-means-stream-open`).
- The time budget of the page test step (`story:page-test-step-within-budget`).
