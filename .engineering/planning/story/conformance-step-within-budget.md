---
format: aep.planning-md/3
id: story:conformance-step-within-budget
kind: story
status: draft
title: The conformance step ends each test within 60 s and itself within 120 s
summary: task conform-rust runs the conformance tests with cargo test, which ends no test at 60 s; the binary took 169 s at load 130. They run under nextest with the 60 s end, a budget line and 120 s
relations:
- decomposes: epic:headless-rust-target
- informed_by: story:test-step-holds-with-tls
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: conformance/tests/main.rs
- confidence: cited
  path: taskfiles/slowest-tests.sh
revision: 5
---
## Why

The folder's rule ("Tests stay fast"): no single test runs longer than 60 s, the runner ends a
test at 60 s and counts it as failed, and the step fails when it runs over 120 s. `task
conform-rust` (`Taskfile.yml`:83 and on) runs the conformance runner's own tests with `cargo test
--locked --manifest-path conformance/Cargo.toml` (`Taskfile.yml`:111), which ends no test at 60 s
and has no budget of its own, then the suite against the Rust system. Measured: the conformance
test binary took 169 s at a load average of about 130 on `impl/round2-commands` (2026-11-22);
`story:slow-tests-at-small-bounds` measured 56.8 s for its 15 tests in one run. `task test`
(`Taskfile.yml`:360 to :394) is the pattern: cargo-nextest with `.config/nextest.toml`, the ten
slowest printed, and `test step: N s, over its budget of 120 s` (:391) failing the step.

## Change

1. Measure first: three runs of `task conform-rust` (`ESS_TOOLCHAIN=0.97.3 slot --timeout 15m task
   conform-rust`), each with `cat /proc/loadavg` before and after; the per-test times of the
   conformance tests and the time of the suite run apart.
2. `Taskfile.yml`, task `conform-rust`: the conformance tests run under cargo-nextest with the
   root's `.config/nextest.toml` (`--config-file`), so a test over 60 s is ended and fails; the
   task prints the ten slowest (`taskfiles/slowest-tests.sh`) and a line `conformance tests: N s
   of its budget of 120 s`, and fails over 120 s with `over its budget of 120 s`, as `task test`
   does. The two `grep -q` checks for the planted-defect tests read nextest's output.
3. From the measurement, the largest costs of the conformance tests, each as a statement with its
   time now and expected, so that the step holds 120 s at a load of 100.
4. `AGENTS.md` :1075-1079 (the description of `task conform-rust`: "Its tests, ... then the suite")
   says how its tests run and the budget.

`impl/ess-0-97-4` changes `AGENTS.md` at :1078 (the tag of `ess` that cargo fetches, hunk
`@@ -1075,7 +1075,7 @@`), inside the range of statement 4; the merge keeps both sides.

## Acceptance

1. The three runs of statement 1 and their loads are in the evidence before any change.
2. A conformance test made to sleep 70 s (planted, removed after) is ended at 60 s, counts as
   failed, and fails `task conform-rust`.
3. A conformance step made to run over 120 s (planted, removed after) prints `over its budget of
   120 s` and fails `task conform-rust`.
4. Three runs of `task conform-rust` after the change, each with both load readings at least
   100, pass and print the budget line with N at most 120.
5. The two planted-defect tests are still named and found by the task's checks
   (`task --summary conform-rust` and the log in the evidence).

## What it does not do

- No change of the root workspace's test step: `story:test-step-holds-with-tls`.
- No change of the suite, of a scenario or of the conformance runner's code beyond statement 3.
- No higher test-thread count, no shards, no raised budget.
