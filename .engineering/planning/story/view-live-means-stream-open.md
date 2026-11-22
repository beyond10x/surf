---
format: aep.planning-md/3
id: story:view-live-means-stream-open
kind: story
status: draft
title: The live view says live when its stream is open
summary: The page of the live view writes live after its first load, before its event stream is open, and draws the timeline again when the stream opens. It writes live when the stream is open and that load is drawn, and the page tests wait for that state.
relations:
- decomposes: epic:live-view
- informed_by: story:local-gate-equals-forge
revision: 2
---
## Context

Found on 2026-11-22 while a page test was diagnosed (`story:local-gate-equals-forge`, commit ff19f1c). The page of the live view writes "live" into `#live-word` after its first load of the sessions (`crates/view/web/view.js`, `updated()` before `follow()`), before its event stream is open. When the stream opens, the page loads everything again and draws the timeline again. A log in the page showed that drawing in 40 of 40 runs, in 3 of them 11 to 18 ms after "live" was written.

A reader sees "live" while the page does not follow yet, and a test that waits for "live" can act on marks that are replaced a moment later.

Seen a third time on 2026-11-23: the gate of `4047c68` ended red by the test "fixes: Enter and Space on a focused mark's button select its event once" of `view-tests/tests/adversary-detail.spec.mjs`, which counted a render after "live" as a second selection (`story:select-once-test-waits-for-load`, which holds `/events` unanswered in that one test and does not change the page). Four spec files have a helper `load()` that waits for "live" (`adversary-detail.spec.mjs:10`, `adversary-page.spec.mjs:14`, `detail.spec.mjs:39`, `first-page.spec.mjs:50`), so every test that reads a state right after it can meet the same render.

Placement: this story writes `crates/view/web/view.js`, which the view unit of round 2 writes on `impl/round2-view`. It is built on that branch, before the gate of the round 2 wave.

## Change

- The page writes "live" when its stream is open and the load after the opening is drawn, and another word before that.
- `docs/view-stream.md` or the design of the view states what "live" promises.
- The helper `load()` of `view-tests/` waits for that state; the two tests that measure in one step of the page keep their form.
- A test that holds `/events` unanswered (the test at `adversary-detail.spec.mjs:200` after `story:select-once-test-waits-for-load`) never reads "live" then: its first wait becomes the state it needs from the first load (two segments), and its route stays.

## Acceptance

- A page test opens the page against a stream whose opening is held back and reads a word other than "live" until the stream is open; when the status is draft, the page test fails on the page as it is at ff19f1c.

## Out of scope

- The reconnect after a lost connection, which has its own tests.
