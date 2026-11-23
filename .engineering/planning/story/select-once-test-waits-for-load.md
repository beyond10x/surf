---
format: aep.planning-md/3
id: story:select-once-test-waits-for-load
kind: story
status: implemented
title: The test of one selection per key holds the event stream shut, so the page loads once
summary: 'The gate of 4047c68 ended red by one page test: the page renders its timeline again when its stream opens, and that render was counted as a second selection. The test holds /events unanswered; the page is not changed.'
relations:
- informed_by: story:view-detail-panel
- decomposes: epic:live-view
- informed_by: story:view-live-means-stream-open
- informed_by: story:page-test-step-within-budget
scope:
- confidence: cited
  path: view-tests/tests/adversary-detail.spec.mjs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T23:27:36Z", actor: "human:operator", revision: 7, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T23:27:36Z", actor: "human:operator", revision: 8, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-23T00:05:29Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"test_result":3,"verification":3}}, executor: "agent:coordinator"}
---
## Why

A gate that is red for a reason the change did not cause holds a merge. The gate of `4047c68`
(branch `fix/doc-tests-of-ports`, 2026-11-23 00:08 to 00:16 CET; the host's load read by the
coordinating session was 159 when the gate was queued and 175 at its end) ended with exit 201 by
`view-test` alone: 37 passed, `1 failed`, `tests/adversary-detail.spec.mjs:200` "fixes: Enter and
Space on a focused mark's button select its event once", message
`Enter selects once: one render of the timeline`, `Expected: 1`, `Received: 2`. The branch writes
no file under `view-tests/` or `crates/view/`.

## Context

The test holds the page to decision 1 of `story:view-detail-panel`: Enter and Space on a focused
mark's button select its event once. It counts the renders of the timeline by the clears of
`#actions` (a `MutationObserver` it installs, `view-tests/tests/adversary-detail.spec.mjs:205-210`)
and expects one more after each key.

The page renders the timeline for reasons other than a key, and each of them can land between
the test's two reads of the count (`:219` and `:224`):

- the second load: `crates/view/web/view.js`, `start()` calls `loadAll()` before it follows
  `/events` and writes `live` after it; when the stream answers, the handler `open()` calls
  `loadAll()` again, which ends with `renderSelected()`. The helper `load()` (`:10-13`) waits
  for `live`, which is written before the stream is open;
- an `entry` event of the selected session (`onEvent`, `view.js:297-301`);
- a stream that closes and opens again, which calls `open()` and loads a third time
  (`crates/view/web/stream.js`, `run()`).

No record of the failed run says which of the three it was. The page snapshot Playwright wrote
shows `reconnecting`, which is what the page shows once the fixture is stopped at the end of the
test, and says nothing about the run.

The same cause was met before. Commit `ff19f1c` (`story:local-gate-equals-forge`) changed two
other tests of this file that measured a mark in two steps; a log in the page showed the second
render in 40 of 40 runs. The draft `story:view-live-means-stream-open` changes the page so that
`live` is written when the stream is open and its load is drawn. It is not built here: it
changes what the page shows a reader and writes `crates/view/web/view.js`, which the view unit
of round 2 writes on `impl/round2-view`; it is built there, before the gate of the round 2 wave.

## Change

- `view-tests/tests/adversary-detail.spec.mjs`, the test at `:200` only: before `load()` it
  sets a route on `/events` that never answers (the handler neither fulfils nor continues the
  request). Then no stream opens for this page: `open()` is not called, so there is no second
  load, no `entry`, `session` or `reset` event and no reconnect. `start()` writes `live` after
  its first load (`view.js:333`), so `load()` passes, and both segments the test needs come from
  that load: the session's `go` has answered before the page is opened.
  The test selects a mark with a key and reads the panel; it reads nothing a stream delivers.
- A comment above the route says why it is there, in two lines at most.
- The test takes the fixture `record` and asserts at its end what the route promises:
  `record.requestsOf("/api/sessions")` (`fixture.mjs:152-154`) has length 1, with a message
  that says the page loaded once because `/events` is held.
- No wait is added: no `waitForTimeout`, no `setTimeout`, no raised `timeout`, no `retries`, no
  `toPass`, and the line `toBe(before + 1)` stays as it is.
- Nothing under `crates/view/` is written.
- The implementor reads the other 7 tests of the file and gives, for each, one line in its
  report: its name, and the state it reads after `load()` that a render after `live` could
  change (a scroll position, a focus, a count), or "none". They are reported, not changed.

Surfaces written: `view-tests/tests/adversary-detail.spec.mjs`.

When `story:view-live-means-stream-open` lands, `live` needs an open stream and this test's
`load()` no longer returns with `/events` held: that story changes this test's first wait to
the two segments, and keeps the route.

## Acceptance

1. The red is reproduced once, in a scratch copy of the unchanged test that is not committed:
   routes hold the answers of the second load (`GET /api/sessions` and
   `GET /api/sessions/<id>`), and the copy releases them after it read `before` (`:219`), so
   that the render of that load lands before the count is read at `:224`. The copy ends with
   `Enter selects once: one render of the timeline`, `Received: 2`. The report quotes the
   output and says how the copy made the render land inside the two reads.
2. The changed test can still go red on the defect it was written for. Plant, not committed: in
   `crates/view/web/timeline.js` a second listener on the mark's button that selects the event
   (beside the one listener of the mark, `:66-69`), then the build line of
   `taskfiles/view-test.yml` (`cargo build --locked --quiet -p surf-agent -p surf-session -p
   surf-view-fixture`; the page is compiled into the binary, `crates/view/src/assets.rs:14-25`).
   The changed test fails on the count. The report quotes it. The plant is taken out by
   reversing that one edit, `git diff --stat -- crates/view` prints nothing, and the build line
   runs again before acceptance 3.
3. With the tree as committed and the fixture built from it:
   `slot npx playwright test --workers=2 tests/adversary-detail.spec.mjs` in `view-tests/`
   passes, 8 of 8, three times in a row. The report gives the time of the changed test in each
   run and the load of the host (`/proc/loadavg`) before each. The test stays under 5 s, or the
   report names its time.
4. `git diff` of `view-tests/tests/adversary-detail.spec.mjs` against the commit before: every
   changed line lies in the test at `:200`; no added line holds `waitForTimeout`, `setTimeout`,
   `timeout`, `retries`, `retry` or `toPass`; the line with `toBe(before + 1)` is not in the
   diff.
5. The report holds the 7 lines the Change asks for, one per other test of the file.
6. The route is what stops the second load. With the pattern of the route changed, not
   committed, to one that matches no request, the changed test fails on the assertion of one
   `GET /api/sessions`; the report quotes it. With the pattern as committed the assertion
   holds in each run of acceptance 3.
7. `slot --timeout 25m task check-runner` exits 0 on the branch `fix/doc-tests-of-ports` at the
   commit that carries this change; that branch is what goes to `main`. The coordinating session
   runs it.

No test runs over 60 s. The page test step took 2.9 minutes in the gate named above and has no
budget on `main` yet; its budget is owned by `story:page-test-step-within-budget`, which is
built on `wave/round-2`.

## Out of scope

A change of the page (`crates/view/web/`); what `live` promises
(`story:view-live-means-stream-open`); the other tests of the file and the helper `load()`; the
tests of the other spec files; the time of the page test step
(`story:page-test-step-within-budget`, which writes other lines of this file: `:65`, `:287`,
`:319`, `:328`).
