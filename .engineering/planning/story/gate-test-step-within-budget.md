---
format: aep.planning-md/3
id: story:gate-test-step-within-budget
kind: story
status: implemented
title: The gate's test step ends within 2 minutes and no test runs over 60 seconds
summary: The step test took 153 seconds at 36e0dfb and one test 75 seconds, because the tests of task generated-check generated the whole workspace once per case. The generation is made once per gate run, the step runs cargo-nextest, a test over 60 seconds is ended and fails, the step prints its ten slowest tests and fails over 120 seconds, and test builds carry no debug information.
relations:
- decomposes: epic:headless-rust-target
scope:
- confidence: cited
  path: .cargo/config.toml
- confidence: cited
  path: .config/nextest.toml
- confidence: cited
  path: .github/workflows/check.yml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: README.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/agent/tests/common/mod.rs
- confidence: cited
  path: crates/fetch/tests/fetch_bounds.rs
- confidence: cited
  path: crates/fetch/tests/review1_security_fetch.rs
- confidence: cited
  path: crates/surf/tests/common/mod.rs
- confidence: cited
  path: crates/surf/tests/generated_check_links.rs
- confidence: cited
  path: crates/surf/tests/generated_check_modes.rs
- confidence: cited
  path: crates/surf/tests/review1_generated_check.rs
- confidence: cited
  path: crates/surf/tests/review2_generated_check.rs
- confidence: cited
  path: crates/surfd/tests/common/mod.rs
- confidence: cited
  path: website/docs/how-it-is-built.md
revision: 22
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T23:34:19Z", actor: "human:operator", revision: 20, decided_on: {"recorded":{"test_result":1,"verification":1}}}
- {from: "proposed", to: "active", at: "2026-11-21T23:34:19Z", actor: "human:operator", revision: 21, decided_on: {"recorded":{"test_result":1,"verification":1}}}
- {from: "active", to: "implemented", at: "2026-11-21T23:34:19Z", actor: "human:operator", revision: 22, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

The rules of the folder this repository is worked in set a budget for a gate's test step: it
ends within 2 minutes of wall time, no single test runs longer than 60 seconds, an expensive
product is made once and never once per test, and the runner enforces the budget. A step over
the budget is a defect with a story, fixed before the next feature.

Measured on 2026-11-21 at `36e0dfb`, `slot task check` in the runner's form on a shared and
loaded machine (the lines stamped with the wall time): the step `test` (`cargo test
--workspace --locked`, after `build` had compiled every target) ran from 22:31:18 to 22:33:51,
153 seconds, for 126 suites and 555 tests. The suites' own times sum to 150.9 seconds; the
binaries of 3 seconds or more:

| binary | time | tests |
|---|---|---|
| `crates/surf/tests/review1_generated_check.rs` | 75.36 s | 4 |
| `crates/fetch/tests/review1_security_fetch.rs` | 20.01 s | 13 |
| `crates/session/tests/built.rs` | 10.27 s | 9 |
| `crates/surf/tests/generated_check_links.rs` | 7.82 s | 6 |
| `crates/session/tests/adversary_events.rs` | 7.06 s | 1 |
| `crates/agent/tests/start.rs` | 5.01 s | 6 |
| `crates/surf/tests/review2_generated_check.rs` | 3.54 s | 2 |
| `crates/surf/tests/generated_check_modes.rs` | 3.04 s | 2 |

One test is over 60 seconds: `review1_one_changed_byte_in_any_generated_file_fails_naming_it`
("has been running for over 60 seconds" in the log). It copies `spec/` and `generated/rust`
once per committed generated file (30 artifacts and the record), changes one byte, and runs
`task generated-check` in the copy, which generates the whole workspace with `ess` again
before it compares. The generation is the expensive product, and it is made once per file:
the shape the folder's rule names. The other three `generated_check` binaries run the same
task per case.

Not measured: the same step on the forge's runner (the run of `61fb7a9` failed after 72
seconds of the step, before its end), and the step on an idle machine. The other steps of the
gate are not this story's; in the same run `conform-rust` took 194 seconds.

## Change

- Measure first, in the unit: the step `test` three times on the unit's machine with the
  time of each binary and of each test of the binaries above, and the test step of the last
  three successful `check` runs on the forge (from their logs).
- The tests of `task generated-check` make the generation once per test binary and share it
  (`OnceLock`): the unit decides how the task takes a fresh generation that already exists
  (a variable that names the directory, or the comparison as its own task that `generated-check`
  calls after generating), keeps `task generated-check` as the gate runs it unchanged in what
  it refuses, and keeps every assertion: each committed file is still changed by one byte and
  must be named. A planted defect per kind of refusal still goes red.
- The other binaries over 5 seconds: for each, where the time goes (a timeout that is waited
  out, a server started per test, a large input), and the change that brings it under 5
  seconds without losing an assertion, or the reason it stays with its measured time.
- The runner enforces the budget: a test over 60 seconds fails the step, and the step prints
  its ten slowest tests. The unit decides the means under the repository's rule that the root
  workspace has no third-party crate (an installed runner such as cargo-nextest pinned by
  version and digest in the forge's workflow, or a check of the test log's own times in the
  Taskfile), and says what the forge's workflow needs.
- `AGENTS.md`: "Commands" and "The gate's numbers" state the budget, the measured time of the
  step and the ten slowest tests.

## Acceptance

`slot task check` in the runner's form exits 0 with the counts of before (126 suites or more,
no test removed), its step `test` ends within 120 seconds of wall time on the unit's machine in
three runs of three, no test runs over 60 seconds, and the step prints the ten slowest tests.
A test made to sleep 61 seconds fails the step (shown once, taken back). The forge's `check`
run of the merge succeeds and its test step is within 120 seconds, read from the log.

## Out of scope

The other steps of the gate (`conform-rust`, `build`, `view-test`, `html-conformance`), which
get their own story if one is over a budget; a full regeneration lane named `slow` unless the
unit finds that the generation itself has to leave the test step.

## Added while the unit ran

Added on 2026-11-21 while the unit ran, from two lines the folder's rules gained that evening:

- The step itself fails when it runs over 120 seconds, on a developer's machine and on the
  forge: the task measures the wall time of its test commands, prints it, and exits non-zero
  naming the time when it is over the budget.
- Test builds carry no debug information: `.cargo/config.toml` of the repository sets it for
  the `dev` and `test` profiles (a setting in `Cargo.toml` loses against a cargo config file
  of the machine), shown with `readelf -S` on one test binary.

The other half of that second line, one integration-test binary per crate, is
`story:one-test-binary-per-crate`.
