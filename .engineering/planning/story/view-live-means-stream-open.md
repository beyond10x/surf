---
format: aep.planning-md/3
id: story:view-live-means-stream-open
kind: story
status: active
title: The live view says live when its stream is open
summary: The page of the live view writes live after its first load, before its event stream is open, and draws the timeline again when the stream opens. It writes live when the stream is open and that load is drawn, and the page tests wait for that state.
relations:
- decomposes: epic:live-view
- informed_by: story:local-gate-equals-forge
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/view/web/view.js
- confidence: cited
  path: view-tests/tests/adversary-detail.spec.mjs
- confidence: cited
  path: view-tests/tests/first-page.spec.mjs
- confidence: cited
  path: website/docs/how-it-is-built.md
- confidence: cited
  path: website/docs/status.md
revision: 18
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T00:29:33Z", actor: "human:operator", revision: 17}
- {from: "proposed", to: "active", at: "2026-11-23T00:29:33Z", actor: "human:operator", revision: 18}
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

On `impl/round2-view` seven spec files have a helper `load()` that waits for `live`
(`view-tests/tests/`: `acceptance.spec.mjs:19`, `adversary-detail.spec.mjs:10`,
`adversary-now.spec.mjs:14`, `adversary-page.spec.mjs:14`, `detail.spec.mjs:39`,
`first-page.spec.mjs:50`, `now.spec.mjs:15`), so every test that reads a state right after it
can meet the same render. The implementing unit of `story:select-once-test-waits-for-load` named
two in `adversary-detail.spec.mjs` that read a scroll position after `load()`: "a click on a
segment outside every mark" and "the requests list follows a new row at its end and stays where
a reader scrolled".

Placement: this story writes `crates/view/web/view.js`, which the view unit of round 2 writes on
`impl/round2-view`. It is built on that branch at `1716d8d` (`main` at `96145a2` merged), on top
of `story:view-now-panel` and `story:view-controls-and-changes`, and before the gate of the
round 2 wave. `story:page-test-step-within-budget`, built on `wave/round-2` after the wave
merge, depends on it (edge): its baseline of 54 page tests, 18 of them in
`first-page.spec.mjs`, becomes 59 and 23, new test 3 uses `cut` and `mend`, new test 1 reads the
session list, and the lines it cites in the spec files shift.

## Change

- `crates/view/web/view.js`: the word `live` says the page follows. It is written when a stream
  is open and the load that its `open()` started is drawn, and not before.
  - `start()` no longer writes `live` after its first load. The word stays `connecting`, which
    `crates/view/web/index.html:14` holds from the start; the sessions of the first load are
    shown under it.
  - `open()` loads, draws, and then writes `live` with the time, and only if the stream that
    called it is still the open one: a load whose stream closed before its draw, or that a
    later opening has overtaken, writes no word.
  - While the page does not follow (before that draw, and after `closed()` until the draw after
    the next opening), nothing writes `live`: `alive()` (a keep-alive comment) and `onEvent`
    both call `updated()` today and can arrive between the opening and the draw. Events that
    arrive then are still kept and drawn as today; only the word waits.
  - A load that fails in `open()` goes to `failed()` as it does today (a message; `offline`
    after a refused token). The word stays the one it was, `connecting` or `reconnecting`: the
    page does not say `live` over a list it could not load. The load is not asked again until
    the next opening; a retry is `story:view-first-load-retry`'s question, not this story's.
  - `closed()` writes `reconnecting` as today; `offline` and a refused token are unchanged.
  - The words are four: `connecting`, `live`, `reconnecting`, `offline`. No word is added.
- `view-tests/tests/`: the seven helpers `load()` keep their text; `live` now means what they
  need. One existing test expects `live` without an open stream (read by the design critic on
  `1716d8d`): the test at `adversary-detail.spec.mjs:200`, which holds `/events` unanswered
  (`story:select-once-test-waits-for-load`). Its first wait becomes the two segments; its route
  (`:205`) and its assertion of one `GET /api/sessions` (`:234`) stay. If the unit finds
  another such test, it names it and what it changed.
- New page tests in `view-tests/tests/first-page.spec.mjs`, each named after its statement.
  A route holds a request with a promise the test resolves; no test pauses.
  1. `the word is connecting until the stream is open`: with `/events` held by a route, the
     session list holds the session of the first load and `#live-word` reads `connecting`; the
     route is continued, and the word becomes `live`.
  2. `the word is live after the load of the opening is drawn`: with the second answer of
     `GET /api/sessions` held and the stream open, the session acts (`surf.agent(id, "go", ...)`)
     and the timeline shows the new action (one segment more), which proves an event arrived
     and was drawn; the word still reads `connecting`. The answer is released, and the word
     becomes `live`.
  3. `after a reconnect the word is live when its load is drawn`: after `cut` and `mend` of the
     fixture (as the test "reconnect after a lost connection" does), with the answer of the
     `GET /api/sessions` that follows the new opening held, the word reads `reconnecting`;
     released, `live`.
  4. `no render of the timeline follows live`: a script added before the page runs
     (`page.addInitScript`) counts, in the page, the clears of `#actions` that happen after
     `#live-word` first reads `live`. "After" is the order of the mutation records: `open()`
     draws and then writes `live` in one task, so the script uses one observer over the
     document and counts a clear only when the record that makes `#live-word` read `live` comes
     before it in that order; it does not read the word's text when its callback runs. The test
     waits for `live`, for the end of the second answer of `GET /api/sessions/<id>` and two
     frames, and reads 0.
  5. `no word is written while the page does not follow`: one session is open. A script added
     before the page runs records every word `#live-word` takes, from the nodes each mutation
     record adds (`written()` in `dom.js:39` puts a new text node on each write), not from the
     word's text when its callback runs. A route answers the first
     `/events` with `route.fulfill` (status 200, `text/event-stream`, a body of one comment
     block, `: keep-alive` and an empty line): the stream opens, gives one comment and ends,
     and the API stays reachable (no `cut`, which would refuse the requests of the load too).
     The route holds the second answer of `GET /api/sessions`, the load of that opening, and
     the next `/events`. Two assertions, each with its message:
     - "a keep-alive before the draw writes no word": the word reads `reconnecting` and the
       recorded words hold no `live`;
     - "a load whose stream closed writes no word": the script records the clears of
       `#actions` too, in the same list and in the order of the mutation records. The test
       reads the length of the list, releases the held answer (`route.continue()`) and waits
       until a clear is recorded after that length: the draw of the released load, which the
       page does in a task of its own after the answers are parsed. Then the word reads
       `reconnecting` and the list holds no `live`; a `live` written by the continuation of
       `open()` that drew is in the list by then, because it is written in that task.
     Then the held `/events` is continued, and the word becomes `live`.
- Documents, written by the unit as the units before it on this branch did (`ba9d465`,
  `936d45d`):
  - `README.md`, section "Watching sessions live": the four words and what each says, in at
    most four lines;
  - `CHANGELOG.md`, `## [Unreleased]`, `### Changed`: one bullet;
  - the count of page tests, 54 on `1716d8d`, in the nine lines that state it. Four are a
    count: `AGENTS.md:827` (with "18 in `tests/first-page.spec.mjs`"), `AGENTS.md:1496`,
    `README.md:214`, `website/docs/status.md:23`; they get the new count. Five are a recorded
    run, `54 passed (2.7m)` with its load: `AGENTS.md:1625`, `README.md:481` and `:492`,
    `website/docs/how-it-is-built.md:57` and `:69`; each is replaced by the line, the task
    time and the load of the last run of acceptance 4, the same run in all five. `40 passed, 54
    skipped` (`README.md:479`, `how-it-is-built.md:55`) is another count and stays;
  - `docs/design/agent-interface.md:465` ("a "live" marker with the time of the last update")
    stays true and is not written.

Surfaces written: `crates/view/web/view.js`, `view-tests/tests/first-page.spec.mjs`,
`view-tests/tests/adversary-detail.spec.mjs` (the test at `:200`), `README.md`, `CHANGELOG.md`,
`AGENTS.md` (the count), `website/docs/how-it-is-built.md` and `website/docs/status.md` (the
count).

## Acceptance

The page is compiled into `surf-view-fixture` (`crates/view/src/assets.rs:14-25`). Every run of
acceptances 1 to 3 on the base page or on a planted `view.js`, and the run after a plant is
removed, comes after the build line of `taskfiles/view-test.yml`; the report shows that line's
output before each quoted result.

1. Red first: on the page as it is on `impl/round2-view` at `1716d8d`, new tests 1, 2 and 3
   fail where they expect `connecting` or `reconnecting`, with `live` received, new test 4
   reads 1, and new test 5 fails at its first assertion with `live` among the recorded words;
   the report quotes the five failures. With the change the five pass.
2. The rule on `onEvent` and `alive()` is held by two plants, each by hand, removed after,
   `git diff --stat` equal before and after:
   - with `setLive("live", Date.now())` called directly as the first line of `onEvent`, test 2
     fails with `live` received;
   - with `setLive("live", Date.now())` called directly in `alive()`, test 5 fails at "a
     keep-alive before the draw writes no word" with `live` among the recorded words.
   A plant is named by what it writes, not by a call of `updated()`, because the unit may put
   the rule inside `updated()`.
   The report quotes both.
3. The rule on the opening is held by test 5: with the check that the stream of the load is
   still the open one taken out (planted, removed after), test 5 passes its first assertion and
   fails at "a load whose stream closed writes no word" with `live` received; the report
   quotes it.
4. `slot task --taskfile taskfiles/view-test.yml view-test`, the whole task with its build
   line, three times in a row, each timed: every test passes each time; the report gives the
   count (59), Playwright's line and the task's seconds of each run, the time of each new test,
   the load of the host (`/proc/loadavg`) before and after each run, and the ten slowest tests
   of the last run. The last run is the one the five recorded lines of the documents take:
   its line, its task seconds, its load and its slowest test, also in the sentences beside
   those lines that name the old run's slowest test (`an entry over 1 MiB` 29.7 s). No test runs over 60 s; a new test over 5 s is named with its time. The time of the
   whole step is reported and not bounded here (`story:page-test-step-within-budget`).
5. `git diff --stat` of the change lists only the surfaces written above. No added line of
   `view-tests/` holds `waitForTimeout`, `setTimeout`, `retries`, `retry` or `toPass`, and no
   `timeout` is raised. In the test at `adversary-detail.spec.mjs:200` the route on `/events`
   and the assertion of one `GET /api/sessions` are not in the diff.
6. The words: `grep -n -E 'connecting|reconnecting|offline' README.md` shows the four words in
   the section "Watching sessions live"; `git diff` of `CHANGELOG.md` shows one added bullet
   under `### Changed` of `## [Unreleased]`; `git grep -n -w 54 -- AGENTS.md README.md
   website/docs` shows only the two lines `40 passed, 54 skipped`, and the report gives the
   nine lines before and after.
7. The report names every existing test whose first wait or expectation the unit changed, with
   file, line and reason; the list the design critic read has one entry.
8. The gate: `slot --timeout 25m task check-runner` exits 0 on `impl/round2-view` at the commit
   that carries the change; the coordinating session runs it.

## Out of scope

- A change of the stream, of `crates/view/src/` or of `docs/view-stream.md`: the page asks the
  same requests in the same order.
- A retry of a load that failed in `open()` (`story:view-first-load-retry` decides the words
  and the retry of a failed first load).
- A wait or a retry in a helper `load()`.
- The time budget of the page test step (`story:page-test-step-within-budget`).
- A new word, a spinner, or a change of `view.css`.
