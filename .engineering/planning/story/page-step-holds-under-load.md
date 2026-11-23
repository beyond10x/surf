---
format: aep.planning-md/3
id: story:page-step-holds-under-load
kind: story
status: draft
title: The page step holds its 120 s at a load over 100
summary: the page step took 124.0 s of 120 s with 71 tests at a load of 70 to 107 and costs a median of 76.8 CPU seconds; four cuts bring it to at most 53.8 s (70 %), target 46.1 s; a flaky test is fixed
relations:
- decomposes: epic:live-view
- informed_by: story:page-test-step-within-budget
- informed_by: story:view-large-entry-test-time
revision: 3
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

CPU seconds here: the user plus system time of the Playwright call of `task view-test` with all
its descendants, measured after a build that compiled nothing, by the script this story commits
(`taskfiles/page-step-cpu.sh`, POSIX sh): the call runs in a cgroup of its own and the script
reads `usage_usec` of its `cpu.stat`, if one run agrees with the `perf` sum within 3 %; else the
`perf` sum. The script is not a step of `task check`.

1. The cuts, each a commit with the CPU seconds before and after, largest first:
   a. `an entry over 1 MiB`: one `go` instead of three; the size of the answer and the four
      actions are asserted in a Rust test of the detail in `crates/surfd/tests/view.rs`; the page
      test keeps the claim that the view draws an entry over 1,048,576 bytes with an address of
      60,000 bytes.
   b. `crates/view-fixture/src/main.rs`, `kill_left`: the fixture ends the processes it started,
      found without reading `environ` of every process of the host. Its claim (no process of the
      test is left) stays and is shown red against a plant.
   c. The Playwright workers: one run with `NODE_OPTIONS=--cpu-prof`, then the largest cost it
      shows. A named candidate: the record fixture of `view-tests/tests/fixture.mjs` asks every
      response for its body; it takes a body only when a test reads it.
   d. Chromium launch arguments in `view-tests/playwright.config.mjs`, each kept only when it
      saves 2 CPU seconds or more and no test changes: `--disable-gpu`; a limit of renderer
      processes so that pages of a worker share one.
2. `controls case 6`: the cause of the one failure is found (the trace of the failed run, the
   page's stream code) and fixed in the page or the test; a longer timeout is not the fix.
3. Not part of this story and not a fix: a higher budget, more workers, shards, a retry, a wait
   for a lower load, removing a test's claim, one page shared by the tests of a worker.
4. `AGENTS.md` records the measurement, the method and what was cut.

## Acceptance

1. The base measurement above is in the evidence with its commands and the base commit's hash,
   recorded before the first commit of the change.
2. CPU: in each of three consecutive runs after the last cut the CPU seconds are at most 53.8 s
   (70 % of the base median of 76.8 s). The target of the cuts is 46.1 s (60 %); what was reached
   is stated.
3. Line: each of these three runs prints `page step: N s of its budget of 120 s` and exits 0, and
   the evidence records its 1-minute load at start and end.
4. Tests: every test title of the base commit's `.out/view-test/junit.xml` passes in each of the
   three runs, none skipped and none flaky; a title that statement 1a changes is named.
5. Every run of the page step on the story's branch after the last cut, to the close, is listed
   with start load, end load and wall seconds. None is over 120 s, and the list names its highest
   start load. If that load is under 100, the story's title says "at the measured load" and the
   claim at a load of 100 moves to a named follow-up story.
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
the page (`story:view-draw-cost`, `story:view-large-entry-test-time` for what 1a leaves).
