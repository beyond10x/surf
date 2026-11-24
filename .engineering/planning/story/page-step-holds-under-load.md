---
format: aep.planning-md/3
id: story:page-step-holds-under-load
kind: story
status: implemented
title: The page step costs 36 % fewer instructions
summary: 'three cuts take the page step from a median of 185.61 G to 119.58 G instructions (35.6 % fewer); bound: a median of at most 122 G over three runs within the 120 s budget; the claim at a load over 100 is withdrawn (49.2 s at 110, 196.7 s at 107, 209.1 s at 143)'
relations:
- decomposes: epic:live-view
- informed_by: story:page-test-step-within-budget
- informed_by: story:view-large-entry-test-time
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T09:26:33Z", actor: "human:operator", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-23T09:26:33Z", actor: "human:operator", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-23T17:17:35Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"test_result":5,"verification":1}}, executor: "agent:coordinator"}
---
## Why

The folder's budget: a test step ends within 120 s. The page step (`task view-test`, two Playwright
workers) holds it at a low load and not at a high one. Measured in runner form on 2026-11-23, the
1-minute load at the start and the end of the gate:

- 60 tests on `ba0c584`: 60.6 s at 66.62 to 87.70; 28.0 s on `2a4fe3c` at 62.92 to 53.93;
- 65 tests on `851fda9`: 63.7 s at 64.14 to 76.62;
- 71 tests on `03f25c9` (`story:view-controls-and-changes` with the adversary's tests): 124.0 s at
  70.02 to 106.67, over the budget, gate exit 201. The sum of the 71 test times was 217.8 s on two
  workers; the slowest were `an entry over 1 MiB` 17.7 s and nine tests of 4.4 to 6.4 s.
- On the forge the same step takes 19.0 s (60 tests, check run 45135338000).

So a page test costs about 0.6 worker-seconds on the forge and about 3 at a load over 100 on this
host, and each new test moves the step by that amount. `story:page-test-step-within-budget` put
the bound and the line in place; it did not make a test cheaper. Where a test's time goes is not
measured: the start of `surfd`, of the fixture and of the session, each `surf` command (a process
of a debug build), the browser's context and page, the draw, the assertions.

## Measured on the base commit

On `03f25c9`, before any change (2026-11-23, tree clean, the commands of `task view-test`
unchanged; evidence under the story):

- Three runs of the page step, 1-minute load before and after: 115.6 s (97.85 to 106.78),
  161.2 s (106.78 to 121.49, over the budget) and 163.7 s (121.49 to 105.58, over the budget; one
  test, `controls case 6`, failed once: `#live-word` read "connecting" after 10 s).
- CPU seconds of the Playwright call with all its descendants (user and system; `perf record -s
  -e task-clock`, the per-thread counters summed): 68.9, 78.4 and 76.8 s; the median is 76.8 s.
  `time` counts reaped children only and showed about 57 % of that.
- By program in the third run: Chromium renderers 28.8 s (71 processes, a median of 0.31 s per
  test; 5.2 s for `an entry over 1 MiB`), the two Playwright workers and their runner 21.9 s,
  Chromium's browser, GPU, zygote and utility processes 16.8 s, `surf-view-fixture` with `surfd`
  inside 7.9 s (72 processes), `surf-session` 1.0 s, `surf-agent` 0.3 s (245 processes).
- The fixture's exit reads `/proc/<pid>/environ` of every process of the host (`kill_left`),
  about 4,000 at this load: 4.6 s in one run, 112 to 174 ms per fixture.
- `an entry over 1 MiB` alone: 6.0 s, of it three `go` of about 250 ms, 2.0 s for the detail's
  JSON in Node and 2.9 s for two count expectations.

So the step spends 77 CPU seconds in 116 to 164 s of wall time on two workers: at a load of 100
on 20 CPUs it waits for a CPU more than it computes. The cuts are in CPU seconds; the wall time
follows the load.

## Change

The measure, as decided after the measurement (D332, D333): the instructions of the Playwright
call of `task view-test` with all its descendants, user only, after a build that compiled
nothing, as `task page-step-cpu` prints them (`taskfiles/page-step-cpu.sh`, POSIX sh,
`perf stat`). The script also prints the CPU seconds (user plus system time). It is not a step
of `task check`. The story first bounded the CPU seconds; "As built" below says why that bound
was replaced.

1. The cuts, each a commit with the CPU seconds before and after, largest first:
   a. `an entry over 1 MiB`: one `go` instead of three; the size of the answer and the four
      actions are asserted in a Rust test of the detail in `crates/session/tests/view.rs`; the
      page test keeps the claim that the view draws an entry over 1,048,576 bytes with an address
      of 60,000 bytes.
   b. `crates/view-fixture/src/main.rs`, `kill_left`: the fixture ends the processes it started,
      found without reading `environ` of every process of the host. Its claim (no process of the
      test is left) stays and is shown red against a plant.
   c. The Playwright workers: one run with `NODE_OPTIONS=--cpu-prof`, then the largest cost it
      shows. A named candidate: the record fixture of `view-tests/tests/fixture.mjs` asks every
      response for its body; it takes a body only when a test reads it.
   d. Chromium launch arguments in `view-tests/playwright.config.mjs`, each kept only when it
      saves 2 CPU seconds or more and no test changes: `--disable-gpu`; a limit of renderer
      processes so that pages of a worker share one.
2. `controls case 6`: its failure text says what the page shows. The cause and the fix are
   `story:view-live-marker-under-load`'s.
3. Not part of this story and not a fix: a higher budget, more workers, shards, a retry, removing
   a test's claim, one page shared by the tests of a worker.
4. `AGENTS.md` records the measurement, the method and what was cut.

## Acceptance

1. The base measurement above is in the evidence with its commands and the base commit's hash,
   recorded before the first commit of the change.
2. Instructions: three passing runs on the base commit and three on the story's last commit are
   listed, each with its commit, its loads, its wall seconds, its CPU seconds, its instructions
   and its count of passed and failed tests; a run with a failed test is listed and repeated.
   The medians and the saving in percent are stated.
3. Bound: the median of three consecutive runs of `task page-step-cpu` whose page step ends
   within its 120 s is at most 122 G instructions. A run whose step goes over 120 s is listed
   and not counted. `AGENTS.md` names the bound and how to check it.
4. Tests: every test title of the base commit's `.out/view-test/junit.xml` passes in each
   counted run of the last commit, none skipped; a title that statement 1a changes is named.
5. Every run of the page step on the story's branches after the last cut, to the close, is
   listed with start load, end load and wall seconds, the runs over 120 s among them. The story
   makes no claim at a load over 100; "As built" says what was measured there.
6. Claims: for 1a the page test and the Rust test are each shown red against a plant; for 1b the
   fixture's test of a left process is shown red against a plant; when
   `view-tests/tests/fixture.mjs` changed, every test that reads what changed counts as changed
   and one test per reader is shown red against a plant. Plants are made by hand, not committed,
   and `git status --short` is empty after.
7. `git diff <base> -- taskfiles/view-test.yml view-tests/playwright.config.mjs` shows
   `--workers=2`, no `retries`, `timeout: 60_000` and the budget of 120 s unchanged.
8. The gate in runner form (`task check-runner`) passes on the story's last commit.
9. When a build setting changed, the evidence holds the compile time of each build step before
   and after, each with its load.

## Out of scope

The bound per test and the step's line (`story:page-test-step-within-budget`). The test step of
the Rust tests under load (`story:test-step-holds-with-tls`). The draw cost of a large entry in
the page (`story:view-draw-cost`, `story:view-large-entry-test-time` for what 1a leaves). The
live marker that stays `connecting` at a load over 100 (`story:view-live-marker-under-load`).
The load of the host.

## As built, and as decided after the measurement

Base `03f25c9`. Cuts kept: 1a in `81f4520`, 1b in `e794f71`, 1c in `c301a84`. The script is
`d09b6e4`, corrected in `14e76fd` (it counted the wait in the pool's queue as the step's time)
and extended in `97237e2` (instructions).

- Method: the cgroup form read 11 to 12 % below `perf` and was dropped; CPU seconds are
  `perf stat -e task-clock`. Instructions are counted in user mode only (the host's
  `perf_event_paranoid` is 2).
- 1d was not kept: `--disable-gpu` saved 0.9 CPU s, a limit of one renderer process nothing.
- D331, one more cut tried in the Playwright workers, none kept: Playwright maps every captured
  stack through its source maps and has no supported setting for it, and the unsupported one
  saves about 1 CPU s; the record taking answers from the page's response events saved nothing;
  `NODE_COMPILE_CACHE` saves about 0.65 s.
- CPU seconds: 68.9, 78.4 and 76.8 s on the base at loads of 98 to 121; 55.5, 55.6 and 56.5 s
  on `c301a84` at loads of 78 to 94, over the bound of 53.8 s the story had then. Runs of the
  same work (114.7 to 117.8 G instructions) cost 38.1 CPU s at a load of 36 and 62.1 CPU s at
  153 to 252, so CPU seconds measure the host's load with the step's work.
- D332 (the operator session, 2026-11-23): the bound is set in instructions, from three runs on
  the base and three on the tip. Passing runs on the base: 193.23, 185.61 and 185.20 G, median
  185.61 G. On the tip: 127.08 G (step 209.1 s, load 142.57 to 111.08), 117.93 G (49.2 s,
  109.83 to 97.81) and 119.58 G (74.7 s, 69.12 to 68.48), median 119.58 G: 35.6 % fewer. In CPU
  seconds the medians are 67.6 and 55.5 s.
- D333: the count grows with the step's wall time, so the bound is on runs whose step ends
  within 120 s: a median of at most 122 G.
- D334 (the operator session, 2026-11-23): the claim of the first title, "holds its 120 s at a
  load over 100", is withdrawn. After the cuts the step took 49.2 s at a start load of 110,
  196.7 s at 107 and 209.1 s at 143, on 20 CPUs; no cut of the step's own work changes that. Runs
  under a load of 90 took 20.2 to 96.7 s. The local gate waits for a load under 90 before it
  starts; both gates of 2026-11-23 after the cuts were green that way (page step 84.5 s at 89.87
  to 86.57 and 96.7 s at 87.30 to 106.28).
- `an entry over 1 MiB`: 14.5 to 33.0 s on the base, 1.3 to 9.1 s after 1a.
- `controls case 6`: the cause was not found; 20 of 20 repeats and 11 full runs passed. Its
  failure now prints the live marker's word and class, `#message` and the segments drawn
  (`163a995`). The same failure appeared in `timeline of three actions` at a start load of 107.
