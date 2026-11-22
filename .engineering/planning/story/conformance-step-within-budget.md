---
format: aep.planning-md/3
id: story:conformance-step-within-budget
kind: story
status: draft
title: The conformance step ends each test within 60 s and itself within 120 s
summary: 'On wave/round-2 (170 scenarios): the conformance tests run under cargo-nextest with the 60 s end; one timed line over the tests and the suite run, own budget of 120 s; each planted defect runs 3 listed scenarios from a committed noticed.txt instead of the whole suite (about 31 runs)'
relations:
- decomposes: epic:headless-rust-target
- informed_by: story:test-step-holds-with-tls
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: conformance/src/lib.rs
- confidence: cited
  path: conformance/tests
- confidence: cited
  path: conformance/tests/main.rs
- confidence: cited
  path: taskfiles/slowest-tests.sh
revision: 10
---
## Why

The folder's rule ("Tests stay fast"): no single test runs longer than 60 s, the runner ends a
test at 60 s and counts it as failed, the step prints its ten slowest tests, and the step fails
when it runs over its budget, locally and in CI.

Budget: the conformance step has a budget of 120 s of its own and fails over it, locally and in
CI; `task test` and the page step have one each; `task check` prints the sum of the three (D247).
Every other mention of the budget in this story reads "its budget" and takes the number from this
sentence.

The tree (D246). This story is built on the branch `wave/round-2` after `impl/round2-commands` and
`impl/round2-view` are merged into it, and before that wave merges to `main`. Every baseline,
statement and acceptance line names that tree. Its suite holds the 170 scenarios of
`impl/round2-commands`. Both round-2 branches change `conformance/tests/planted_defect.rs`,
`review2_planted_defects.rs` and `two_of_each_entity.rs`, and `impl/round2-commands` changes
`conformance/src/target.rs`; building after their merge leaves no conflict in those files. `ess`
takes its release from `requires:` of `spec/ess-inputs.yaml`: `ess 0.97.4` on `main` and on
`impl/round2-commands`.

What the step is today (`Taskfile.yml`:83-115 on `main`; no round-2 branch and not `wave/tls-1`
changes these lines):

- Step 1 (:111) runs the crate's tests with `cargo test --locked --manifest-path
  conformance/Cargo.toml`, which ends no test at 60 s, prints no ten slowest and has no budget.
  The checks at :112 and :113 grep `cargo test`'s lines for the planted-defect test and the two
  per-entity tests.
- Step 2 (:114) runs the suite against the Rust system with `cargo run`, outside every budget.
- `task test` (`Taskfile.yml`:270-331) is the pattern: it times its run after `task build`, prints
  the ten slowest through `taskfiles/slowest-tests.sh`, and prints `test step: N s, over its
  budget of 120 s` (:327) and fails, or `test step: N s of its budget of 120 s` (:330).

Measured: the crate's test binary took 169 s at a load average of about 130 on
`impl/round2-commands` on 2026-11-22, and `task conform-rust` as a whole 194 s.
`story:slow-tests-at-small-bounds` measured 56.8 s for its 15 tests on `main` in one run.

Where the time goes. The crate has 15 tests in one binary (`conformance/tests/main.rs`): 1 in
`planted_defect.rs`, 11 in `review2_planted_defects.rs`, 3 in `two_of_each_entity.rs`. Each
planted defect runs the whole suite once (`run`, `conformance/src/lib.rs`:64-70), so the cost is
defects times scenarios. `two_of_each_entity.rs` runs it 19 times in 3 tests: the control once,
7 identity defects in `one_identity_for_every_instance_of_an_entity_is_noticed_for_each_entity`,
11 view defects in `a_view_cut_to_its_first_row_is_noticed_for_each_entity`
(`assert_each_noticed`, one `failed_with` per defect). `review2_planted_defects.rs` runs it once
per test, its control included (`suite_over`). `planted_defect.rs` runs it once. That is about 31
runs of the 170 scenarios, against one run in step 2. The two controls run the unplanted system
over the whole suite, as step 2 does.

## Decisions taken

- D246: the tree above.
- D247: the budget sentence above.
- D250: the step's line covers the run of the conformance tests and the run of the suite against
  the Rust system (step 2), builds excluded, the same way `task test` times its run after the
  build. One line, `conformance step: N s of its budget of <budget> s`, and a failing form over
  the budget. The step prints its ten slowest tests through `taskfiles/slowest-tests.sh`.
- D254: remedy A of statement 4 when the probe of statement 1 passes; when it fails, the unit
  stops and reports the refusal, and B waits for a new decision.
- D251: every time target here is read on three consecutive runs that print `/proc/loadavg`
  before and after. The story closes on those three runs at the load that is present, and names
  that load. A later run over the budget at a higher load is a new defect with its own story.

## Change

Statement 1 comes first; the others follow in any order.

1. Measure first, on the tree, before any change: three consecutive runs of `slot --timeout 15m
   task conform-rust`, each with `cat /proc/loadavg` before and after. Per-test times come from
   one run of `cargo nextest run --locked --manifest-path conformance/Cargo.toml --config-file
   .config/nextest.toml` and its JUnit file; the time of step 2 alone from the built binary.
   Then the probe of statement 4, a local test that is not committed:
   `conformance/tests/probe.rs` with its `mod` line in `tests/main.rs`. Its one test,
   `probe::a_narrowed_suite_is_admitted`, keeps the first 3 scenarios of the `scenarios` map of
   `.out/suite.json`, runs that suite through `surf_conformance::run` (`AdmittedSuite::from_json`
   of `ess-conformance` 0.97.4) against `memory_target()`, and prints one line. The command is
   `cargo nextest run --locked --manifest-path conformance/Cargo.toml --config-file
   .config/nextest.toml --no-capture -E 'test(=probe::a_narrowed_suite_is_admitted)'`. The
   expected line is `probe: narrowed suite admitted, 3 scenarios run`. On a refusal the test prints
   `probe: refused: <the runner's text>` and fails.
2. `Taskfile.yml`, task `conform-rust`. First, outside the timed part: `cargo nextest run --no-run`
   and `cargo build`, both `--locked --manifest-path conformance/Cargo.toml`, the first with
   `--config-file .config/nextest.toml`. Then, timed: `cargo nextest run --locked --manifest-path
   conformance/Cargo.toml --config-file .config/nextest.toml --color never` into
   `.out/conform-rust-test.log`, so a test over 60 s is ended and fails (`slow-timeout` with
   `terminate-after` of the root's `.config/nextest.toml`); the checks of statement 3; and step 2,
   run as the built binary `conformance/target/debug/surf-conformance .out/suite.json
   .out/conform-rust.json`. After it the step prints `the ten slowest tests:` and `sh
   taskfiles/slowest-tests.sh conformance/target/nextest/default/junit.xml 10`, then `conformance
   step: N s of its budget of <budget> s`, or `conformance step: N s, over its budget of <budget>
   s` and exit 1 when N is over the budget. The budget is the variable `CONFORM_BUDGET`, its
   default the number of the budget sentence. The step removes `.out/conform-step.seconds` at its
   start and writes N there at its end, for `step-budgets` of
   `story:page-test-step-within-budget`. `RUSTFLAGS: -D warnings` stays.
3. The checks at `Taskfile.yml`:112 and :113 read nextest's log: each of
   `planted_defect::a_navigation_storage_that_drops_every_put_fails_at_least_one_scenario`,
   `two_of_each_entity::one_identity_for_every_instance_of_an_entity_is_noticed_for_each_entity`
   and `two_of_each_entity::a_view_cut_to_its_first_row_is_noticed_for_each_entity` must have its
   `PASS` line. Their messages stay: `the planted-defect test did not run` and `the per-entity
   planted-defect tests did not run`.
4. The remedy for the cost of defects times scenarios (D254): remedy A, a committed generated
   file with a digest of its sources, when the probe of statement 1 prints its expected line.
   When the probe fails, the unit stops and reports the runner's refusal text; remedy B is not
   built without a new decision.
   - A. `conformance/tests/noticed.txt`
     holds the sha256 of `.out/suite.json` on its first line and then one line per planted
     defect: its label and the ids of the first 3 scenarios, in the suite's order, that do not
     pass over it. Each planted-defect test runs its defect over those scenarios only and
     asserts that at least one does not pass, which is the claim "the suite notices the defect".
     A stale list can only turn a test red, never green. A fast check in `conform-rust` compares
     the digest with `.out/suite.json` and, when they differ, fails naming `task slow`. The task
     `slow` (the lane the rule names), not part of `check`, runs every planted defect over the
     whole suite, writes `.out/noticed.txt` and compares its bytes with the committed file. The
     two controls keep the whole suite. Expected: the 29 planted runs fall to 3 scenarios each;
     the step costs about 3 runs of the suite (two controls and step 2). A narrowed suite is the
     probe's question: `SuiteInput::select` of 0.97.4 (`coverage.rs`:964) needs a coverage block
     (:928-931), and the suite of `task suite` has none, so A narrows the `scenarios` map before
     admission, as the probe does.
   - B, not built without a new decision: a cache on disk under `conformance/target/` keyed by the
     digest of its sources. It saves nothing on the forge, which starts without the cache, nor
     after any change of a source.
5. `AGENTS.md`, the bullet of `task conform-rust` (:1087-1091 on `main`): its tests run under
   cargo-nextest with the 60 s end; one timed part covers the tests and the suite run; the budget
   line and the ten slowest; under remedy A, `conformance/tests/noticed.txt` and `task slow`. The
   bullet of `conformance/` (:654-661 on `main`) names `noticed.txt` under remedy A.

### Overlap and order

- `wave/tls-1` changes `Taskfile.yml` at :15 and :43 (includes and tasks of its own), `lock-check` (:136-167)
  and the list of `check` (:346-374), not `conform-rust` (:83-115). It changes `AGENTS.md` from
  :1092, the bullet of `task lock-check` right after this story's bullet; the merge keeps both.
- `story:session-request-timeout-option` changes `AGENTS.md` at the waits of `built` (:1306-1307
  on `main`); this story changes the bullet of `task conform-rust` and the bullet of
  `conformance/`. The sections do not overlap.
- `story:tls-spec` and round 2 change `conformance/src/target.rs`; this story does not.
- `story:page-test-step-within-budget` reads `.out/conform-step.seconds` and lands after this
  story on the same tree.

## Acceptance

1. Statement 1's three runs, each with its two load readings, the per-test times, the time of
   step 2 alone and the probe's result, are in the evidence before any change.
2. A conformance test made to sleep 70 s (planted by hand, removed after) is ended at 60 s:
   nextest prints its `TIMEOUT` line for that test and a summary with `1 timed out`, and `task
   conform-rust` exits non-zero. The output is in the evidence.
3. Run with `CONFORM_BUDGET=1`, the step prints `conformance step: N s, over its budget of 1 s`
   and exits non-zero.
4. Three consecutive runs of `slot --timeout 15m task conform-rust` on the story's commit, each
   with `cat /proc/loadavg` before and after, pass, print ten slowest lines read from
   `conformance/target/nextest/default/junit.xml`, and print the budget line with N within the
   budget. N spans the nextest run and step 2 and not the builds. The evidence names the load of
   the three runs and the commit (D251).
5. With `planted_defect::a_navigation_storage_that_drops_every_put_fails_at_least_one_scenario`
   marked `#[ignore]` (planted by hand, removed after), the task prints `the planted-defect test
   did not run` and exits non-zero. The same plant on one per-entity test prints `the per-entity
   planted-defect tests did not run` and exits non-zero.
6. Statement 4, remedy A. The evidence holds the probe's command and its output line.
   - Planted tests: `cargo nextest list --manifest-path conformance/Cargo.toml` lists the 13
     planted-defect tests and the 2 controls of today (acceptance 7).
   - Scenarios per planted run: one run of the crate's tests with `--no-capture` prints the count
     line each run already writes (`planted defect: N scenarios`, `review2 [<label>]: N
     scenarios`, `<label>: N scenarios`): 29 lines for the 29 planted defects, each with N at
     most 3, and the 2 lines of the controls with the suite's 170.
   - The digest check: with the first line of `conformance/tests/noticed.txt` edited by hand,
     `task conform-rust` fails naming `task slow`. `task slow` on the story's commit writes
     `.out/noticed.txt` equal byte for byte to the committed file.
   - With one planted defect taken out of its wrapper (by hand), its test fails naming the
     defect.
   Each plant is removed after (`git diff --stat` equal before and after).
7. `cargo nextest list --manifest-path conformance/Cargo.toml` before and after gives the same 15
   tests.

## What it does not do

- No change of the root workspace's test step: `story:test-step-holds-with-tls`.
- No change of the suite, of a scenario, of `conformance/src/target.rs`, or of what a
  planted-defect test claims.
- No higher test-thread count, no shards, no raised budget or timeout.
- No Python: the tasks are POSIX sh; the tests and any generator stay Rust.

## Scope

The files of `aep plan artifact scope`. In `AGENTS.md` the bullets of `task conform-rust` and of
`conformance/`; not the waits of `built` (`story:session-request-timeout-option`) and not the
bullets of `task view-test` and `task check` (`story:page-test-step-within-budget`).
