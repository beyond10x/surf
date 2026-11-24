---
format: aep.planning-md/3
id: story:view-live-marker-under-load
kind: story
status: implemented
title: 'The view''s live marker reaches live at the measured load: the page tests run in a network namespace of their own'
summary: two runs of the page step at a start load of 107 and 121 failed with one test each that waited 10 s for the live marker and found connecting; one wait with a diagnostic in every spec file, the cause, the fix, 10 runs at a load of 100 or more
relations:
- decomposes: epic:live-view
- informed_by: story:page-step-holds-under-load
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T17:19:28Z", actor: "human:operator", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-23T17:19:29Z", actor: "human:operator", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-24T02:11:18Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"test_result":3,"verification":1}}, executor: "agent:coordinator"}
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

## Found so far

First unit, 2026-11-23, branch `fix/view-live-marker` from `3b0fc83`:

- Change 1 is built in `7f646a5` and `941f7fa`: `expectLive(page)` in
  `view-tests/tests/fixture.mjs` is the one wait for `live`, used by every spec file. A failed
  wait prints the live marker's word and class, `#message`, the segments drawn, whether `/events`
  was sent and its head arrived with the milliseconds from the navigation, the page's other
  fetches, the browser's requests with their status, page errors, console errors, and a `GET /`
  of the view made by the test. Acceptance 1: with `/events` planted never to answer, one test
  per spec file fails with that text (10 on `7f646a5`, 9 of 10 on `941f7fa`: `controls case 6`
  was not selected in that run); on the base the same plant prints `Received "connecting"` only.
- Acceptance 7: base median 117.47 G instructions (120.93, 117.47, 116.16), `941f7fa` median
  117.37 G (117.37, 116.79, 119.11), all runs within the budget, 71 passed each.
- No counted run: the host was restarted at 20:30 CET and its load stayed under 100 afterwards
  (highest seen 92.32) through 3,600 s of waiting.
- A failure at a low load, not counted: on `7f646a5`, load 21.76 to 37.66, `page lists both
  sessions` waited 10 s and printed: live marker `connecting`, segments drawn 0, `/events` not
  sent, no other fetch of the page, 12 files loaded by the browser, the view's `/` from the test
  200 in 54 ms. The page's script made no request in 10 s. So the failure is not bound to a load
  of 100. Decision D337: the second unit looks for the cause by repeating page starts at the
  load the host has.
- A probe of `crates/view/src/http.rs` (a plant, removed): in one full run at most 9 connections
  were open, no 503, no connection closed without an answer, a head within 71 ms.
- A failure that names a cause, 2026-11-23: the gate on `941f7fa` (load 44.00 to 68.02, page step
  113.4 s) failed `controls case 5` and passed the other 70 page tests. The diagnostic printed:
  live marker `connecting`, segments drawn 0, `/events` not sent, `/now.js`, `/changes.js` and
  `/timeline.js` with no answer while the nine other files answered 200, no page error, and three
  console errors `Failed to load resource: net::ERR_NETWORK_CHANGED`. The page's module graph did
  not finish, so `view.js` never ran. Lead, not yet confirmed: the browser cancels requests in
  flight when an address is added to or removed from any interface of the host, a loopback target
  included, and other work on the host adds and removes container interfaces. The second unit
  confirms it with a planted address change inside a network namespace of its own and then
  chooses the fix; a retry of the page load and a longer wait stay out.

## As found and as closed

Second unit, 2026-11-23 and 24, branch `fix/view-live-marker-cause` from `941f7fa`, commits
`c40365c` and `ee57cbc`, on `main` as the merge `4386cad`.

The cause is not the load. The browser cancels every request in flight with
`net::ERR_NETWORK_CHANGED` when an address of any interface in its network namespace is added or
removed, a request to loopback included. On the host, containers of other work start and stop;
each adds or removes an interface with a link-local address. A page whose module files were
cancelled never ran its script, so the marker stayed `connecting`.

- The host, read only: 0 address changes in 60 s, 10 in 20 minutes, in two bursts, all on
  container interfaces. Both failures with a diagnostic have such an interface coming up in the
  kernel log within 2 s of the failed test.
- Planted inside a network namespace of its own, never on the host: an address added and
  deleted every 200 ms. Without the fix 25 page starts failed of at least 62, with the gate's
  text (module files with no answer, `net::ERR_NETWORK_CHANGED`, 0 segments, `/events` not
  sent). With no plant, 0 of 196. With the plant and the fix, 0 of 227.
- The fix: `taskfiles/page-netns.sh` runs the page step, the browser and every fixture in a user
  and network namespace of its own with only loopback up; without one the step fails with one
  line and runs no test. `view-tests/global-setup.mjs` refuses to start a page test outside such
  a namespace. `.github/workflows/check.yml` allows unprivileged user namespaces on the runner.

Against the statements:

1. Met in the first unit (see "Found so far").
2. The cause is named, and the failure text that shows it is quoted above under "Found so far".
   That run started at a load of 44.00, so it is not a counted run. No counted run exists: no
   run of `task view-test` started at a load of 100 or more between the diagnostic and the
   close, and no load was made for one.
3. Not shown as written, for the same reason: 0 counted runs. Shown in its place: 0 of 227 page
   starts failed under the plant with the fix; 252 page starts on the host with the fix, 0
   failed; three runs of `task page-step-cpu` and two gates on `ee57cbc`, 71 passed each.
4. Met by the plant above, and held from now on by a committed check: outside a namespace of
   their own the page tests fail in `global-setup.mjs` before a browser starts.
5. The highest start load of a listed run of the whole page step is 88.21. Runs of a part of it
   started higher (the plant runs at 128.07, 121.42 and 98.60, the cold starts of the first unit
   at 98.48); they are no runs of `task view-test`. The title says "at the measured load", and
   the claim at a load of 100 or more moves to `story:view-live-marker-at-load-100`.
6. Unchanged: `--workers=2`, `retries: 0`, the 60 s bound, `expect.timeout` of 10 000 ms, the
   120 s budget, every test title. The fix is no longer wait.
7. Base median 117.47 G instructions (120.93, 117.47, 116.16). `ee57cbc`: median 119.14 G
   (119.14, 119.32, 118.58), each run within 120 s, 71 passed each. The bound of
   `story:page-step-holds-under-load` is 122 G.

One local gate on `ee57cbc` passed every test and took 131.6 s for the page step, at a load of
65.77 to 99.08 beside a second gate; the next, alone at a load of 78.52 to 49.51, took 21.9 s.
The step's seconds under load are `story:page-step-holds-under-load`'s.
