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
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/view/web/view.js
- confidence: cited
  path: docs/design/agent-interface.md
- confidence: cited
  path: view-tests/tests/adversary-detail.spec.mjs
- confidence: cited
  path: view-tests/tests/first-page.spec.mjs
revision: 8
---
## Context

Found on 2026-11-22 while a page test was diagnosed (`story:local-gate-equals-forge`, commit
`ff19f1c`). The page of the live view writes `live` into `#live-word` after its first load of
the sessions (`crates/view/web/view.js`, `start()`: `updated()` before `follow()`), before its
event stream is open. When the stream answers, the handler `open()` writes `live` again and
only then loads everything again and draws the timeline again. A log in the page showed that
drawing in 40 of 40 runs, in 3 of them 11 to 18 ms after `live` was written.

A reader sees `live` while the page does not follow yet: with a stream that never answers, the
page reads `live` for as long as it is open. And a test that waits for `live` can act on marks
that are replaced a moment later.

Seen three times in gates:

- the gate of `de08b24`: "a click on a mark beside its short name selects that event",
  `boundingBox()` null; fixed in that test and one more by `ff19f1c`;
- the gate of `4047c68` on 2026-11-23: "fixes: Enter and Space on a focused mark's button select
  its event once" counted a render after `live` as a second selection;
  `story:select-once-test-waits-for-load` holds `/events` unanswered in that one test.

Four spec files have a helper `load()` that waits for `live`
(`view-tests/tests/adversary-detail.spec.mjs:10`, `adversary-page.spec.mjs:14`,
`detail.spec.mjs:39`, `first-page.spec.mjs:50`), so every test that reads a state right after
it can meet the same render. The implementing unit of `story:select-once-test-waits-for-load`
named two in `adversary-detail.spec.mjs` that read a scroll position after `load()`: "a click on
a segment outside every mark" and "the requests list follows a new row at its end and stays
where a reader scrolled".

Placement: this story writes `crates/view/web/view.js`, which the view unit of round 2 writes on
`impl/round2-view`. It is built on that branch after `main` at `96145a2` is merged into it, and
before the gate of the round 2 wave.

## Change

- `crates/view/web/view.js`: the word `live` says the page follows. It is written when a stream
  is open and the load that `open()` starts is drawn, and not before.
  - `start()` no longer writes `live` after its first load. The word stays `connecting`, which
    `crates/view/web/index.html:14` holds from the start; the sessions of the first load are
    shown under it.
  - `open()` loads, draws, and then writes `live` with the time. A load that fails there goes to
    `failed()` as it does today.
  - While the page does not follow (before that draw, and after `closed()` until the draw after
    the next opening), nothing writes `live`: `alive()` (a keep-alive comment) and `onEvent`
    both call `updated()` today and can arrive between the opening and the draw.
  - `closed()` writes `reconnecting` as today; `offline` and a refused token are unchanged.
  - The words are four: `connecting`, `live`, `reconnecting`, `offline`. No word is added.
- `view-tests/tests/`: the four helpers `load()` keep their text; `live` now means what they
  need. A test that needs `live` without an open stream is changed to wait for the state it
  reads from the first load:
  - the test at `adversary-detail.spec.mjs:200` holds `/events` unanswered
    (`story:select-once-test-waits-for-load`): its first wait becomes the two segments, its route
    and its assertion of one `GET /api/sessions` stay;
  - the unit lists every other test that expects `live` while its `/events` is held, ended or
    routed, and what it changed in each.
- New page tests in `view-tests/tests/first-page.spec.mjs`, each named after its statement:
  1. `the word is connecting until the stream is open`: with `/events` held by a route, the
     session list holds the session of the first load and `#live-word` reads `connecting`; the
     route is continued, and the word becomes `live`;
  2. `the word is live after the load of the opening is drawn`: with the second answer of
     `GET /api/sessions` held by a route and the stream open, the word reads `connecting`; the
     answer is released, and the word becomes `live`;
  3. `after a reconnect the word is live when its load is drawn`: after `cut` and `mend` of the
     fixture (as the test "reconnect after a lost connection" does), with the answer of the
     `GET /api/sessions` that follows the new opening held, the word reads `reconnecting`;
     released, `live`.
  Each asserts the earlier word with `expect(...).toHaveText(...)` while the hold is in place,
  after a state that proves the page got that far (the listed session; the request that is
  held was seen by the route), not after a pause.
- Documents: `README.md`, section "Watching sessions live", gains the four words and what each
  says, in at most four lines; `docs/design/agent-interface.md:465` ("a "live" marker with the
  time of the last ...") is brought to it if its sentence is no longer true. The unit hands the
  CHANGELOG line over in its report (decision D7) and does not write `CHANGELOG.md`.

Surfaces written: `crates/view/web/view.js`, `view-tests/tests/first-page.spec.mjs`,
`view-tests/tests/adversary-detail.spec.mjs` (the test at `:200`), other files of
`view-tests/tests/` only where a test expects `live` without an open stream, `README.md` (that
section), `docs/design/agent-interface.md` (that sentence), and whatever test of `crates/view`
pins the bytes of `view.js`.

## Acceptance

1. Red first: the three new tests fail on the page as it is on `main` at `96145a2`, each with
   `Expected: "connecting"` or `"reconnecting"` and `Received: "live"`; the report quotes the
   three. With the change they pass.
2. No render of the timeline lands after `live`: a page test, or an assertion inside test 2,
   installs the count of the clears of `#actions` (as the test at
   `adversary-detail.spec.mjs:200` does) when it reads `live` and reads 0 clears two frames
   later and after one keep-alive comment of the stream. On `main` at `96145a2` the same
   assertion reads 1; the report quotes it.
3. `slot npx playwright test --workers=2` in `view-tests/`, all spec files, after the build
   line of `taskfiles/view-test.yml`: every test passes, three times in a row; the report gives
   the count, the time of each run, the time of each new test, the load of the host
   (`/proc/loadavg`) before each run, and the ten slowest tests of the last run. No test runs
   over 60 s; a new test over 5 s is named with its time.
4. The time of the page tests does not grow by more than 5 s against the same three runs on the
   base of the story (the branch before the change, same session, loads named): `live` comes
   later in each test, by the time of one load.
5. `git diff --stat` of the change lists only the surfaces written above; no added line of
   `view-tests/` holds `waitForTimeout`, `setTimeout`, `retries`, `retry` or `toPass`, and no
   `timeout` is raised.
6. The words: `grep -n -E 'connecting|reconnecting|offline' README.md` shows the four words in
   the section "Watching sessions live".
7. The gate: `slot --timeout 25m task check-runner` exits 0 on `impl/round2-view` at the commit
   that carries the change; the coordinating session runs it.

## Out of scope

- A change of the stream, of `crates/view/src/` or of `docs/view-stream.md`: the page asks the
  same requests in the same order.
- A wait or a retry in a helper `load()`.
- The time budget of the page test step (`story:page-test-step-within-budget`).
- A new word, a spinner, or a change of `view.css`.
