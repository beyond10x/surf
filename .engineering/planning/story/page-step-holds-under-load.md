---
format: aep.planning-md/3
id: story:page-step-holds-under-load
kind: story
status: draft
title: The page step holds its 120 s at a load over 100
summary: the page step took 124.0 s of 120 s with 71 tests at a load of 70 to 107, 19.0 s on the forge; a page test costs about 3 worker-seconds under load; measure where it goes, then cut the CPU seconds of the step by 40 %
relations:
- decomposes: epic:live-view
- informed_by: story:page-test-step-within-budget
- informed_by: story:view-large-entry-test-time
revision: 1
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

## Change

1. Measure first, before any change, on the story's base commit: the CPU seconds (user and
   system, of the whole process tree) and the wall seconds of the page step, three runs, each
   with the load before and after; and for five tests (the slowest, two of
   `controls.spec.mjs`, one of `first-page.spec.mjs`, one of `now.spec.mjs`) the wall time of each
   phase: fixture and `surfd` start, session open, each `surf` command, `page.goto`, each
   expectation. No load is made on purpose. CPU seconds are the measure that is compared between
   commits, since wall time follows the host's load.
2. Cut the largest costs the measurement shows, largest first, until statement 3 holds. Candidates,
   none chosen before statement 1: a build setting for the crates the page tests run
   (`.cargo/config.toml` holds such settings for four packages already), measured with its compile
   time before and after; fewer processes per test (one `surfd` per worker with a session per
   test, if the tests' claims allow it); a cheaper page test of the entry over 1 MiB
   (`story:view-large-entry-test-time`, whose statement 1 this measurement serves).
   Not candidates: a higher budget, more workers, shards, a retry, a wait for a lower load, and
   removing a test's claim.
3. Target: the CPU seconds of the page step fall by at least 40 % against statement 1 with the
   same 71 tests, and the step prints its line within 120 s in a run that starts at a load of
   100 or more when the host has one.

## Acceptance

1. Statement 1's numbers are in the evidence, with the commands.
2. Three consecutive runs of the page step after the change: CPU seconds at most 60 % of the
   median of statement 1; each within 120 s; 71 or more tests passed, none skipped.
3. Every claim of a test that was changed is shown red against a plant after the change.
4. The gate in runner form passes on the story's commit, with the compile times of its build
   steps before and after when a build setting changed.
5. `AGENTS.md` records the measurement and what was cut.

## Out of scope

The bound per test and the step's line (`story:page-test-step-within-budget`). The test step of
the Rust tests under load (`story:test-step-holds-with-tls`).
