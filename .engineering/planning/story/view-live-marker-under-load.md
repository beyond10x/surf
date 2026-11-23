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
revision: 2
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

A wait for `live` is an expectation of `#live-word` to have the text `live`, in a spec file's
`load()` or anywhere else in the page tests. A counted run is a run of `task view-test` with a
start load of 100 or more, from the commit of the diagnostic to the close of the story; every
such run is listed, none is left out, whatever its step line says. Each is listed with its
commit, its loads, its step line, its exit status and its count of passed and failed tests; a
test that fails on anything other than a wait for `live` is named with its failure. A plant is
made by hand and is not committed: `git status --short` is empty after it.

1. With the `/events` request planted never to answer in one test of each spec file that waits
   for `live`, that test's failure text holds the live marker's word, its class, `#message` and
   the segments drawn, names `/events` as sent with its milliseconds from the navigation, and
   names its head as not arrived. Without the plant the same tests pass, and the record of
   `fixture.mjs` shows no request beyond those of the base.
2. The record names the cause and quotes the failure text of at least one counted run that shows
   it. If no counted run fails on a wait for `live`, the record says "cause not found"; once 10
   counted runs in a row have passed, the title says "no failure seen in 10 runs", and the cause
   moves to a named follow-up story that keeps the diagnostic. Statement 3 is then not met by
   that list and moves with it.
3. After the fix, 10 counted runs in a row: no failure text holds a wait for `live`.
4. With the cause planted, the test named in the record fails with the text quoted in the
   record. If the cause is in the test, the record quotes the failing line before the fix and
   the passing line after it.
5. If the highest start load of the listed runs is under 100 when the story closes, the title
   says "at the measured load", and the claim at a load of 100 moves to a named follow-up story.
6. Unchanged: `--workers=2`, `retries: 0`, the 60 s bound of a test, `expect.timeout` of
   10 000 ms, the 120 s budget of the step, and every test title of the base. If the fix is a
   longer wait, it is in the one wait of `fixture.mjs`, it names the measured time, and the
   record quotes a failure text in which `/events` was not yet sent.
7. The instructions of three runs of the page step (`task page-step-cpu`) on the story's last
   commit and on its base are listed. The last commit's median is within the instruction bound
   of `story:page-step-holds-under-load`, or within 2 % of the base median if that story names
   no bound when this one closes.

## Out of scope

- The step's seconds at a load over 100: `story:page-step-holds-under-load`. That story's item on
  `controls case 6` is this story's; a longer wait is allowed here only under statement 6.
- The load of the host.
