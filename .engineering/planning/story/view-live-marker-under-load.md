---
format: aep.planning-md/3
id: story:view-live-marker-under-load
kind: story
status: draft
title: The view's live marker reaches live at a load over 100
summary: two runs of the page step at a start load of 107 and 121 failed with one test each that waited 10 s for the live marker and found connecting; one wait with a diagnostic in every spec file, the cause, the fix, 10 runs at a load of 100 or more
relations:
- decomposes: epic:live-view
- informed_by: story:page-step-holds-under-load
revision: 1
---
## Why

Two runs of the page step failed on 2026-11-23 with one test each that waited 10 s for the view's
live marker (`#live-word`) to read `live` and found `connecting`:

- `controls case 6` (`view-tests/tests/controls.spec.mjs`), third run on `03f25c9`, 1-minute load
  121.49 at the start and 105.58 at the end, under `perf record -s`; the step took 163.7 s;
- `timeline of three actions` (`view-tests/tests/first-page.spec.mjs:168`, in that file's `load()`
  at line 53), first run on `14e76fd`, load 106.82 to 95.99; the step took 196.7 s of its budget
  of 120 s.

No run at a start load under 100 failed this way: 20 of 20 repeats of case 6 with traces and 11
full runs passed. A local gate at a load over 100 is red on it, and a push to `main` needs a green
local gate.

What is known:

- A `/events` request planted never to answer gives the same picture in `controls.spec.mjs`: "live
  marker: connecting; class live is-connecting; message: (empty); segments drawn: 5" (`163a995`).
- Ruled out by the code: a closed or refused stream and a failed first load (each writes another
  word), an error in drawing (it would repeat in every run), a late head of `/events` (`surfd`
  writes it at once).
- Only `controls.spec.mjs` prints what the page shows when the wait fails. The other nine spec
  files wait for `live` in their own `load()` and print the expectation alone.

Not known: whether the page's script ran in those 10 s, whether the request for `/events` left the
browser, and whether its answer's head arrived.

## Change

1. One wait for `live` in `view-tests/tests/fixture.mjs`, used by every spec file's `load()` in
   place of its own line. When it fails it prints, each read once and bounded at 2 s: the live
   marker's word and class, `#message`, the segments drawn, and for the page's `/events` request
   whether it was sent, whether its head arrived, and the milliseconds from the navigation to
   each. A passing run adds no wait and no request.
2. The cause, from that output of a failed run at a start load of 100 or more. The runs use the
   load the host has; no load is made for them.
3. The fix where the cause is: the page (`crates/view/web/`), `surfd`'s stream, the fixture, or
   the test. A longer wait in the test is a fix only when the output shows that the page's script
   had not yet run, and then the wait is bounded by the test's 60 s and named with the measured
   time.
4. `AGENTS.md`, in the page step's section: the cause, the fix, and the runs.

## Acceptance

1. With the `/events` request planted never to answer in one test of each spec file that waits
   for `live`, that test's failure text holds the live marker's word, its class, `#message`, the
   segments drawn and the state of the `/events` request. Without the plant the same tests pass.
2. The story's record names the cause and quotes the failure text of at least one failed run at
   a start load of 100 or more that shows it. If 10 consecutive runs of the page step at a start
   load of 100 or more show no such failure, the record lists the 10 runs with their loads, and
   statement 3 is met by that list.
3. After the fix, 10 consecutive runs of `task view-test` at a start load of 100 or more: no test
   fails on the wait for `live`. Each run is listed with its commit, its loads, its step line and
   its count of passed and failed tests.
4. A test fails when the cause is planted, if the cause is in the page, in `surfd` or in the
   fixture.
5. Unchanged: `--workers=2`, `retries: 0`, the 60 s bound of a test, the 120 s budget of the
   step, and every test title of the base. The step's instructions stay within the bound of
   `story:page-step-holds-under-load`.

## Out of scope

- The step's seconds at a load over 100: `story:page-step-holds-under-load`.
- The load of the host.
