---
format: aep.planning-md/3
id: story:page-test-step-within-budget
kind: story
status: active
title: The page-test step ends each test within 60 s
summary: 'On wave/round-2 after round 2 merges (54 page tests; impl/round2-view: 54 passed (2.7m), task 189 s at load 55 to 81): test timeout 60 s, own budget and ten slowest, npm ci out of the timed part, a shared fixture if fixture starts sum over 10 s, largest costs cut'
relations:
- decomposes: epic:live-view
- informed_by: story:test-step-holds-with-tls
- depends_on: story:conformance-step-within-budget
- depends_on: story:view-live-means-stream-open
scope:
- confidence: cited
  path: .github/workflows/check.yml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/view-fixture
- confidence: cited
  path: taskfiles/view-test.yml
- confidence: cited
  path: view-tests/playwright.config.mjs
- confidence: cited
  path: view-tests/tests
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T21:48:50Z", actor: "human:operator", revision: 11}
- {from: "proposed", to: "active", at: "2026-11-22T21:48:51Z", actor: "human:operator", revision: 12}
---
## Why

The folder's rule ("Tests stay fast"): no single test runs longer than 60 s, the runner ends a
test at 60 s, the step prints its ten slowest tests, and the step fails over its budget, locally
and in CI. The page step is `task view-test` (`taskfiles/view-test.yml`), run by `task check`
after `view-stream-check`.

Budget: the page step has a budget of 120 s of its own and fails over it, locally and in CI; `task
test` and the conformance step have one each; `task check` prints the sum of the three (D247).
Every other mention of the budget in this story reads "its budget" and takes the number from this
sentence.

The tree (D246). This story is built on the branch `wave/round-2` after `impl/round2-commands` and
`impl/round2-view` are merged into it, and before that wave merges to `main`. Every baseline,
statement and acceptance line names that tree. The page tests on it are the 54 of
`impl/round2-view` (`20ab15b`): 18 in `first-page.spec.mjs`, 8 in `adversary-detail.spec.mjs`, 7
in `adversary-now.spec.mjs`, 6 in `adversary-page.spec.mjs`, 6 in `detail.spec.mjs`, 5 in
`now.spec.mjs`, 4 in `acceptance.spec.mjs`. `main` (`0f0c157`) has 38 of them.

Measured so far:

- `main` (`0f0c157`), the forge's check 45069730446: `38 passed (16.5s)`; the forge's load is not
  known.
- `impl/round2-view`, recorded in its `AGENTS.md` (the paragraph after the waits of `built`):
  `54 passed (2.7m)`, the task 189 s with its build and `npm ci`, at a load average of 55 to 81 on
  2026-11-22. At a load average of 106 to 140 the three slowest were `an entry over 1 MiB` 29.7 s,
  `adversary: a refused token on the stream stops the page` 15.5 s and `dropped entries are a
  gap` 9.7 s.

What the step does today, on `main` and on `impl/round2-view` alike (neither round-2 branch
changes `taskfiles/view-test.yml`, `view-tests/playwright.config.mjs`,
`view-tests/tests/fixture.mjs` or `crates/view-fixture/src`):

- `taskfiles/view-test.yml`:25 builds `surf-agent`, `surf-session` and `surf-view-fixture`; :26
  runs `npm --prefix view-tests ci --prefer-offline`, which reaches the registry when the npm
  cache does not hold the pin; :27 runs `npx playwright test --workers=2`.
- The step prints no budget line and no ten slowest tests. Its reporter is `list`
  (`view-tests/playwright.config.mjs`:12), which writes no file of per-test times.
- `view-tests/playwright.config.mjs`:13 sets `timeout: 90_000`, over the rule's 60 s. Six tests
  set `test.setTimeout(120_000)`: `adversary-detail.spec.mjs`:287, `adversary-page.spec.mjs`:75 and
  :104, `first-page.spec.mjs`:405, :431 and :474. One sets 60,000 (`adversary-page.spec.mjs`:148).
- Each test starts its own `surf-view-fixture` (`view-tests/tests/fixture.mjs`:220-227, the
  fixture `surf`), a page server, `surfd` in process and sessions; `startFixture` (:208-218) starts
  more in one test.
- `adversary: a refused token on the stream stops the page` waits a fixed 12 s
  (`adversary-page.spec.mjs`:166, `page.waitForTimeout(12_000)`).
- `view-tests/tests` holds 12 assertions with `timeout: 2000`: 6 in `first-page.spec.mjs`, 3 in
  `detail.spec.mjs`, 3 in `adversary-detail.spec.mjs` (`git grep -c "timeout: 2000"
  view-tests/tests`, the same on both trees). One of them failed once at a load of 110
  (`story:slow-tests-at-small-bounds`).

## Decisions taken

- D246: the tree above.
- D247: the budget sentence above.
- D253 (replaces D248): the start of one `surf-view-fixture` is measured first. If its median
  times the 54 page tests is over 10 s, the tests that use no process-wide part of the fixture
  share one fixture process per run; otherwise each test keeps its own, and `AGENTS.md` records
  why. Statement 6.
- D249: `npm ci` leaves the timed step. A setup task runs it only when
  `view-tests/package-lock.json` differs from the installed tree. The timed step makes no network
  request.
- D251: every time target here is read on three consecutive runs that print `/proc/loadavg`
  before and after. The story closes on those three runs at the load that is present, and names
  that load. A later run over the budget at a higher load is a new defect with its own story.

## Change

Statement 1 comes first; the others follow in any order.

1. Measure first, on the tree, before any change: three consecutive runs of `slot --timeout 15m
   task --taskfile taskfiles/view-test.yml view-test`, each with `cat /proc/loadavg` before and
   after. Each run passes `--reporter=list,junit` with `PLAYWRIGHT_JUNIT_OUTPUT_NAME` set to
   `.out/view-test/junit.xml` on the command line, so no file changes for the measurement. Apart
   from the time inside tests, it records the build (`cargo build`), `npm ci` and the Playwright
   run as a whole. Then a script, not committed and started without the `slot` prefix, starts
   `target/debug/surf-view-fixture` 10 times in a row, each from its spawn to its ready line (its
   first line of JSON) and then closes its input. It prints `cat /proc/loadavg` before and after,
   the median of the 10 starts, and the sum for the tree: the median times 54, the count of page
   tests. That sum decides statement 6.
2. `view-tests/playwright.config.mjs`: `timeout: 60_000`. The six `test.setTimeout(120_000)`
   become at most `60_000`; the one at 60,000 stays.
3. `view-tests/playwright.config.mjs`: `reporter: [["list"], ["junit", { outputFile:
   "../.out/view-test/junit.xml" }]]`. Playwright's JUnit reporter writes one `<testcase
   name="…" classname="…" time="…">` per test, time in seconds, the shape
   `taskfiles/slowest-tests.sh` reads.
4. `taskfiles/view-test.yml` gains the task `view-test-setup`, POSIX sh. It compares the
   `sha256sum` of `view-tests/package-lock.json` with the digest stored in
   `view-tests/node_modules/.surf-lock.sha256`. When they are equal it prints `view-tests:
   node_modules matches package-lock.json` and runs nothing else. Otherwise it runs `npm --prefix
   view-tests ci --no-audit --no-fund` and then writes the digest. `view-test` runs it before the
   timed part. `.github/workflows/check.yml`:133 runs `task --taskfile taskfiles/view-test.yml
   view-test-setup` in place of its own `npm ci`; the browser install at :134 stays.
5. `taskfiles/view-test.yml`, task `view-test`: the build and `view-test-setup` run first, outside
   the timed part, as `task test` times its run after `task build`. The timed part is
   `./node_modules/.bin/playwright test --workers=2` in `view-tests/`: no `npx` and no `npm`, so no
   network request. After it the step prints `the ten slowest tests:` and `sh
   taskfiles/slowest-tests.sh .out/view-test/junit.xml 10`, then `page step: N s of its budget of
   <budget> s`, or `page step: N s, over its budget of <budget> s` and exit 1 when N is over the
   budget. The budget is the variable `PAGE_BUDGET`, its default the number of the budget
   sentence. The step removes `.out/page-step.seconds` at its start and writes N there at its end.
6. The fixture per test (D253). The sum of statement 1 decides between two branches, and the
   evidence names the branch taken.

   The process-wide parts of a fixture. Commands come over its standard input
   (`crates/view-fixture/src/main.rs`:184-231) from the process that spawned it (`fixture.mjs`:21
   and :67). One `surfd` lists every session it holds. `stop-surfd` (`main.rs`:215-218) drops the
   one daemon. `cut` and `mend` (`main.rs`:219-226) act on the one pipe. The log bounds are
   options of the process (`--entries-per-session`, `--bytes-total`, `main.rs`:117-131). A test
   uses a process-wide part when it calls `stop-surfd`, `cut` or `mend`, starts a fixture with
   other log bounds, or reads the list of sessions as a whole: its count, a session by its place
   in it, or a locator over every entry of it.

   The tests that use one, on the tree (21 of 54):
   - `acceptance.spec.mjs`:40, :48, :67 and :82: a count of the listed sessions, through
     `twoSessions` (:30-35).
   - `adversary-now.spec.mjs`:158: a locator over every title of the list (:185).
   - `adversary-page.spec.mjs`:52 (a count, :56), :74 (`cut` and `mend`, :79-85), :103 (other log
     bounds, :105; `cut` and `mend`, :123-125) and :147 (`cut` and `mend`, :159-163).
   - `first-page.spec.mjs`:66, :78, :94 and :106 (counts, :70 to :117), :120 (a count, :126), :256
     (`stop-surfd`, :260), :264 (a place in the list, :281), :293 (a place in the list, :331),
     :364 (`cut` and `mend`, :377-380), :404 (`cut` and `mend`, :409-415) and :430 (other log
     bounds, :433 and :453).
   - `now.spec.mjs`:107: a locator over every title of the list (:112).

   Branch "shared", when the sum is over 10 s. The 33 other tests share one `surf-view-fixture`
   per run; the 21 tests above keep a fixture of their own each, as today.
   - `crates/view-fixture` (Rust): an option `--listen` binds a listener on 127.0.0.1, port 0, and
     the ready line gains `"control":"127.0.0.1:<port>"`. Each connection to it takes the
     commands `open`, `agent` and `kill` in the protocol of standard input, one JSON answer per
     line, served in a thread of its own; the tokens of opened sessions are shared under a lock.
     `stop-surfd`, `cut`, `mend` and every other line answer `{"error":"command"}` there.
     Standard input keeps every command, and its end still stops everything.
   - `view-tests/tests/shared-fixture.mjs`, Playwright's `globalSetup` and `globalTeardown` in
     `view-tests/playwright.config.mjs`: the setup starts the shared process with `--listen` and
     hands its ready line to the workers in an environment variable; the teardown closes its input
     and waits for it to exit.
   - `view-tests/tests/fixture.mjs` gets a second way to reach a fixture: the fixture `surf`
     connects to the shared process's listener. The 21 tests above take the fixture `ownSurf`,
     today's start of a process per test, or `startFixture`.
   - `fixture.mjs` appends one line per start of a `surf-view-fixture` to
     `.out/view-test/fixture-starts.txt`, which the step removes at its start.

   Branch "own", when the sum is 10 s or less. Each test keeps its own fixture. `AGENTS.md`, in the
   paragraph of the page tests' times, records the reason in one sentence: the median start, the
   sum for the 54 tests, the load before and after, and the command that printed them.
7. The largest costs, each with its time now and expected:
   - `adversary: a refused token on the stream stops the page` (`adversary-page.spec.mjs`:147-171,
     15.5 s at a load of 106 to 140). Its `page.waitForTimeout(12_000)` (:166) waits to see that no
     request follows the refusal. The page's stream asks again after `FIRST_DELAY`, 500 ms,
     doubling to `LAST_DELAY`, 5,000 ms (`crates/view/web/stream.js`:9-10 and :138-141), so a page
     that asks again does so within 500 ms. The wait becomes 2,000 ms, four times `FIRST_DELAY`,
     and its comment says so. Expected: under 5 s.
   - `an entry over 1 MiB` (`first-page.spec.mjs`:473-490, 29.7 s at a load of 106 to 140). The
     address of 60,000 bytes and the answer over 1,048,576 bytes are its claim, so the size stays.
     Statement 1 shows where its time goes. The page step's `cargo build` reads the overrides
     `story:test-step-holds-with-tls` statement 3 puts in `.cargo/config.toml`. If it is over 20 s
     in one of the three runs of acceptance 8, the unit stops and reports, and its size is decided
     in a story of its own.
   - Waits. A test that waits for the view to show an entry waits on that entry under the
     expect timeout of 10 s (`playwright.config.mjs`:14) or the test's own timeout, never a fixed
     sleep. A `timeout: 2000` stays only where a sentence of a story or a document states 2 s,
     and a comment beside it names that sentence as path:line. Two such sentences are known:
     `.engineering/planning/story/view-first-page.md`:226-227 ("latest follows", `first-page.spec.mjs`:173-176)
     and `.engineering/planning/story/view-detail-panel.md`:157-158 ("a go adds a row live",
     `detail.spec.mjs`:164). Every other `timeout: 2000` goes.
8. The sum (D247). `task test` (`Taskfile.yml`:325-331 on `main`) also writes its seconds to
   `.out/test-step.seconds`; the conformance step writes `.out/conform-step.seconds`
   (`story:conformance-step-within-budget`); the page step writes `.out/page-step.seconds`. A task
   `step-budgets`, the last of `check`, POSIX sh, prints `three steps: N s (test step A s,
   conformance step B s, page step C s)` and fails naming the file when one of the three is
   missing. It fails on no budget of its own.
9. `AGENTS.md`: the bullet of `task view-test` (:1055-1062 on `main`) says the setup task, the
   timed part, the budget line, the ten slowest and the JUnit file; the paragraph of the page
   tests' times (:1319-1320 on `main`, rewritten by `impl/round2-view`) gains the three runs of
   acceptance 8 with their loads; the description of `task check` (:964-974 on `main`) names `step-budgets`.

### Overlap and order

- `impl/round2-view` changes `AGENTS.md` at the page tests' count (:631-633 on `main`) and their
  times (:1319-1320 on `main`); this story writes over the merged text.
- `wave/tls-1` changes `Taskfile.yml` in the list of `check` (:338-359 on `main`) and adds the
  task `slow:` after :359. `step-budgets` goes in as the last item of `check`; the merge keeps
  both.
- `wave/tls-1` changes `AGENTS.md` in the paragraph of `task check` (:966-968 and :977-980 on
  `main`), which statement 9 also changes, and inserts bullets before the bullet of `task
  view-test` (after :1054 on `main`), next to statement 9's edit of that bullet. The merge keeps
  both sides of each.
- `story:test-step-holds-with-tls` changes `Taskfile.yml` at `generated-compare` and
  `conform-totals`, not at `test`'s budget lines (:327 and :330).
- `story:conformance-step-within-budget` lands on the same tree first: statement 8 reads its
  seconds file.

## Acceptance

1. Statement 1's three runs, each with its two load readings, the JUnit file and the fixed costs,
   are in the evidence before any change.
2. `grep -n timeout view-tests/playwright.config.mjs` shows `timeout: 60_000`, and `git grep -nE
   "test\.setTimeout\(" view-tests/tests` shows no value over `60_000`.
3. A test that sleeps 70 s (`await page.waitForTimeout(70_000)`, planted by hand and not
   committed) is ended at 60 s: Playwright prints `Test timeout of 60000ms exceeded`, and `task
   --taskfile taskfiles/view-test.yml view-test` exits non-zero. The output is in the evidence;
   `git status --short view-tests` is empty after.
4. The step prints `page step: N s of its budget of <budget> s` and ten lines read from
   `.out/view-test/junit.xml` by `taskfiles/slowest-tests.sh`. Run with `PAGE_BUDGET=1` it prints
   `page step: N s, over its budget of 1 s` and exits non-zero.
5. With `view-tests/node_modules` installed and its digest equal, `view-test-setup` prints
   `view-tests: node_modules matches package-lock.json` and starts no `npm`. With the stored digest
   changed by hand it runs `npm ci` and writes the digest again. `task --dry --taskfile
   taskfiles/view-test.yml view-test` shows no `npm` and no `npx` in the timed part.
6. Statement 7, refused token: the test passes in under 5 s in each run of acceptance 8. With a
   page that sends one more request to `/events` after the refusal (planted in
   `crates/view/web/stream.js` by hand, removed after, `git diff --stat` equal before and after)
   it fails on its assertion that no request followed; the failure text is in the evidence.
7. Statement 7, waits: `git grep -c "timeout: 2000" view-tests/tests` before (6, 3, 3) and after
   is in the evidence; each `timeout: 2000` left has a comment naming its sentence as path:line;
   `git grep -n waitForTimeout view-tests/tests` shows only the wait of 2,000 ms.
8. Three consecutive runs of `slot --timeout 15m task --taskfile taskfiles/view-test.yml
   view-test` on the story's commit, each with `cat /proc/loadavg` before and after: all 54 tests
   pass, each prints its budget line with N within the budget, and no test takes over 20 s in the
   JUnit file. The evidence names the load of the three runs (D251).
9. `slot --timeout 30m task check > .out/check.log 2>&1` on the story's commit on `wave/round-2`
   passes, and `.out/check.log` holds the line `three steps: N s (test step A s, conformance step
   B s, page step C s)`. A, B and C equal the numbers of the lines `test step: …`, `conformance
   step: …` and `page step: …` in the same log, and N is their sum. The evidence names the commit
   and quotes the four lines.
10. Statement 6: the evidence holds statement 1's output of the 10 starts (median, sum, both load
    readings) and names the branch taken, "shared" or "own".
    - "shared": after each run of acceptance 8, `wc -l < .out/view-test/fixture-starts.txt`
      prints 23: 1 shared process, 1 for each of the 21 tests of statement 6, and 1 more for the
      second fixture of `first-page.spec.mjs`:430. On the listener, `stop-surfd`, `cut` and `mend`
      are answered `{"error":"command"}`, shown by a test of `crates/view-fixture/tests`.
    - "own": `grep -n` on `AGENTS.md` shows the sentence of statement 6 with its median, its sum
      and its load, and the command that printed them.
11. The forge: the `check` run of the story's commit on its branch or pull request prints `page
    step: N s of its budget of <budget> s`. The evidence names the run id and quotes the line.

## What it does not do

- No more workers than 2, no retries, no raised timeout or budget.
- No change of the view's product code; the plant of acceptance 6 is removed.
- No change of the root workspace's test step beyond writing its seconds file:
  `story:test-step-holds-with-tls`.
- No browser install in the step; the forge's step at `.github/workflows/check.yml`:134 stays.
- No Python: the setup task and `step-budgets` are POSIX sh; the tests stay JavaScript.

## Scope

The files of `aep plan artifact scope`. In `AGENTS.md` the bullets of `task view-test` and `task
check` and the paragraph of the page tests' times; not the bullet of `task conform-rust`
(`story:conformance-step-within-budget`) and not the waits of `built`
(`story:session-request-timeout-option`).
