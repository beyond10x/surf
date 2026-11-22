---
format: aep.planning-md/3
id: story:page-test-step-within-budget
kind: story
status: draft
title: The page-test step ends each test within 60 s
summary: task view-test took 2.7 min for 43 tests at load 140, slowest 29.9 s, and Playwright's timeout is 90 s; measure the fixed costs, set the timeout to at most 60 s, and take the largest costs
relations:
- decomposes: epic:live-view
- informed_by: story:test-step-holds-with-tls
scope:
- confidence: cited
  path: taskfiles/view-test.yml
- confidence: cited
  path: view-tests/playwright.config.mjs
- confidence: cited
  path: view-tests/tests
revision: 4
---
## Why

The folder's rule ("Tests stay fast"): no single test runs longer than 60 s, and the runner
enforces it (Playwright `timeout`). The page-test step, `task view-test`
(`taskfiles/view-test.yml`, run by `task check` after `view-stream-check`, `Taskfile.yml`:418),
took 2.7 min for 43 tests at a load average of about 140 on 2026-11-22, its slowest test 29.9 s;
`view-tests/playwright.config.mjs`:13 sets `timeout: 90_000`, over the rule's 60 s. Each test
starts its own `surf-view-fixture` (a page server, `surfd` in process and sessions).
`story:slow-tests-at-small-bounds`, which this step's work comes from, also counted 12
assertions with `timeout: 2000` in `view-tests/tests/` (6 in `first-page.spec.mjs`, 3 in
`detail.spec.mjs`, 3 in `adversary-detail.spec.mjs`), one of which failed once at a load of 110.

## Change

1. Measure first: three runs of `task view-test` (`slot --timeout 15m task --taskfile
   taskfiles/view-test.yml view-test`), each with `cat /proc/loadavg` before and after; per-test
   times from Playwright's JSON reporter; and the fixed costs apart from the time inside tests:
   the browser's start, the build of the fixture and binaries, `npm ci`, and the fixture each test
   starts.
2. `view-tests/playwright.config.mjs`: `timeout` at most 60,000.
3. From the measurement, the largest costs each as a statement with its time now and expected
   (for example one fixture per file where a test does not need its own, or a smaller page); a
   test that waits for the view to show an entry waits on the event it follows under the test's
   own timeout, and a `timeout: 2000` stays only where a sentence of the documents states that
   time, with its reason.

## Acceptance

1. The three measured runs and their loads are in the evidence before any change.
2. `grep -n timeout view-tests/playwright.config.mjs` shows a test timeout of at most 60,000.
3. Three runs of `task view-test` after the change, each with its loads (both at least 100),
   pass, and no test takes over 20 s in Playwright's report.
4. The count of `timeout: 2000` left is stated, each with the sentence it proves.

## What it does not do

- No change of the root workspace's test step: `story:test-step-holds-with-tls`.
- No more workers than the 2 of `task view-test`, no retries.
