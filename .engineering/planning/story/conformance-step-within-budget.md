---
format: aep.planning-md/3
id: story:conformance-step-within-budget
kind: story
status: active
title: The conformance step ends each test within 60 s and itself within 120 s
summary: 'On main (ff6431f, 196 scenarios): the conformance tests run under cargo-nextest with the 60 s end; one timed line over the tests and the suite run, own budget of 120 s; each planted defect runs 3 listed scenarios from a committed noticed.txt instead of the whole suite (about 31 runs)'
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
revision: 17
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T21:57:11Z", actor: "human:operator", revision: 13}
- {from: "proposed", to: "active", at: "2026-11-22T21:57:12Z", actor: "human:operator", revision: 14}
---
## Why

The folder's rule ("Tests stay fast"): no single test runs longer than 60 s, the runner ends a
test at 60 s and counts it as failed, the step prints its ten slowest tests, and the step fails
when it runs over its budget, locally and in CI.

Budget: the conformance step has a budget of 120 s of its own and fails over it, locally and in
CI; `task test`, `task test-tls` and the page step have one each; `task check` prints the sum of
the four (D247). Every other mention of the budget in this story reads "its budget" and takes the
number from this sentence.

The tree (D302, which moves the branch D246 named). This story is built on `main` from `ff6431f`,
where round 2 (`b4a7481`) and `wave/tls-1` are merged. Every baseline, statement and acceptance
line names that tree. Its suite holds 196 scenarios, 37 of them authored, and `task suite` writes
56 refusals (`AGENTS.md`:1626-1628 and :1637-1638, "The gate's numbers"; `Taskfile.yml`:76,
`REFUSALS_MAX: 56`). Round 2 changed `conformance/tests/planted_defect.rs`,
`review2_planted_defects.rs`, `two_of_each_entity.rs` and `conformance/src/target.rs` (`git diff
--stat 0f0c157 ff6431f -- conformance`). `ess` takes its release from `requires:` of
`spec/ess-inputs.yaml` (:2): `ess 0.97.4`; the crate pins `ess-conformance` at tag `0.97.4`
(`conformance/Cargo.toml`:18).

What the step is today (`Taskfile.yml`:105-137 on `main`):

- Step 1 (:133) runs the crate's tests with `cargo test --locked --manifest-path
  conformance/Cargo.toml`, which ends no test at 60 s, prints no ten slowest and has no budget.
  The checks at :134 and :135 grep `cargo test`'s lines for the planted-defect test and the two
  per-entity tests.
- Step 2 (:136) runs the suite against the Rust system with `cargo run`, outside every budget.
- `task test` (`Taskfile.yml`:295-372) is the pattern: it times its run after `task build`, prints
  the ten slowest through `taskfiles/slowest-tests.sh`, and prints `test step: N s, over its
  budget of 120 s` (:368) and fails, or `test step: N s of its budget of 120 s` (:371). `task
  test-tls` (:374-430) does the same with `tls test step` (:426, :429).

Measured: the crate's test binary took 169 s at a load average of about 130 on
`impl/round2-commands` on 2026-11-22, and `task conform-rust` as a whole 194 s.
`story:slow-tests-at-small-bounds` measured 56.8 s for its 15 tests on `main` in one run. In the
gate in runner form on `b00d02d` (2026-11-23, 04:55 to 05:03 CET, a load average of 110.75 at the
start and 116.83 at the end), `.out/conform-rust-test.log` shows the binary of `tests/main.rs`
ending `test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in
88.20s` (its line 531), and
`two_of_each_entity::a_view_cut_to_its_first_row_is_noticed_for_each_entity` and
`two_of_each_entity::one_identity_for_every_instance_of_an_entity_is_noticed_for_each_entity`
each `has been running for over 60 seconds` (lines 511 and 512), outside any 60 s end. The planted
run of `planted_defect.rs` printed `planted defect: 196 scenarios: 146 passed, 50 failed, 0
error, 0 unsupported` (line 127).

Where the time goes. The crate has 15 integration tests in one binary (`conformance/tests/main.rs`):
1 in `planted_defect.rs`, 11 in `review2_planted_defects.rs`, 3 in `two_of_each_entity.rs`; 13 of
them plant a defect and 2 are controls (`review2_control_the_unplanted_system_passes_every_scenario`
and `nothing_planted_fails_no_scenario`). Its library has 17 unit tests of its own (15 in
`conformance/src/target.rs`, 2 in `conformance/src/values.rs`; `running 17 tests`, line 4 of the
same log), which run no suite. Each planted defect runs the whole suite once (`run`,
`conformance/src/lib.rs`:64-70), so the cost is defects times scenarios. `two_of_each_entity.rs`
runs it 19 times in 3 tests: the control once, 7 identity defects in
`one_identity_for_every_instance_of_an_entity_is_noticed_for_each_entity`, 11 view defects in
`a_view_cut_to_its_first_row_is_noticed_for_each_entity` (`assert_each_noticed`, :386, one
`failed_with`, :328, per defect). `review2_planted_defects.rs` runs it once per test, its control
included (`suite_over`, :44). `planted_defect.rs` runs it once. That is 31 runs of the 196
scenarios (31 lines `…: 196 scenarios: …` in that log: 29 planted, 2 controls), against one run in
step 2. The two controls run the unplanted system over the whole suite, as step 2 does.

## Decisions taken

- D246: the tree above, moved by D302.
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
- D302: round 2 went to `main` when its gate was green, so this story is built on `main`
  (`ff6431f`), not on `wave/round-2`, before the next feature.
- D307: the sentences that sum the steps name four (`test step`, `tls test step`, the
  conformance step, the page-test step); D247 stays 120 s per step.
- D308: statements that described a merge order that has happened are written to the state of
  `main` (statements 4 and 6, "Overlap and order"); no remedy changes.
- D309: the digest of `conformance/tests/noticed.txt` takes `conformance/src/values.rs` as a fifth
  source, since `target.rs` uses it (`target.rs`:87).

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
   of `ess-conformance` 0.97.4) against `memory_target()` (`conformance/src/lib.rs`:45), and
   prints one line. The command is
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
3. The checks at `Taskfile.yml`:134 and :135 read nextest's log: each of
   `planted_defect::a_navigation_storage_that_drops_every_put_fails_at_least_one_scenario`,
   `two_of_each_entity::one_identity_for_every_instance_of_an_entity_is_noticed_for_each_entity`
   and `two_of_each_entity::a_view_cut_to_its_first_row_is_noticed_for_each_entity` must have its
   `PASS` line. Their messages stay: `the planted-defect test did not run` and `the per-entity
   planted-defect tests did not run`.
4. The remedy for the cost of defects times scenarios (D254): remedy A, a committed generated
   file with a digest of its sources, when the probe of statement 1 prints its expected line.
   When the probe fails, the unit stops and reports the runner's refusal text; remedy B is not
   built without a new decision.
   - A. `conformance/tests/noticed.txt` holds on its first line `sources <sha256>`, the digest of
     its five sources: `.out/suite.json`; the planted-defect test modules
     `conformance/tests/planted_defect.rs`, `review2_planted_defects.rs` and
     `two_of_each_entity.rs`; `conformance/src/target.rs`; `conformance/src/values.rs`, which
     `target.rs` uses (`target.rs`:87, D309); and the `[[package]]` entry of
     `ess-conformance` in `conformance/Cargo.lock` (:100-102). Then it holds one line per planted
     defect: its label and the ids of the first 3 scenarios, in the suite's order, that do not
     pass over it. Each planted-defect test runs its defect over those scenarios only and asserts
     that at least one does not pass, which is the claim "the suite notices the defect". A stale
     list can only turn a test red, never green.
   - The fast check, a step of `conform-rust` before the timed part, POSIX sh: it computes the
     digest of the five sources and compares it with the first line. When they differ it fails
     with the line `conform-rust: the sources of conformance/tests/noticed.txt changed; run task
     conform-slow`.
   - The task `conform-slow`, not part of `check`, runs every planted defect over the whole
     suite, writes `.out/noticed.txt` and compares its bytes with the committed file. It is a
     subtask of the lane `slow` (statement 6).
   - Who regenerates the file (D308). From this story on, the change that touches one of the
     five sources (through `spec/` for `.out/suite.json`) runs `task conform-slow` in the lane
     `slow` and commits `noticed.txt` in the same commit. The fast check's failure line names
     the task.
   - The two controls keep the whole suite. Expected: the 29 planted runs fall to 3 scenarios each;
     the step costs about 3 runs of the suite (two controls and step 2). A narrowed suite is the
     probe's question: `SuiteInput::select` of 0.97.4 (`coverage.rs`:964) needs a coverage block
     (:928-931), and the suite of `task suite` has none, so A narrows the `scenarios` map before
     admission, as the probe does.
   - B, not built without a new decision: a cache on disk under `conformance/target/` keyed by the
     digest of its sources. It saves nothing on the forge, which starts without the cache, nor
     after any change of a source.
5. `AGENTS.md`, the bullet of `task conform-rust` (:1486-1490 on `main`): its tests run under
   cargo-nextest with the 60 s end; one timed part covers the tests and the suite run; the budget
   line and the ten slowest; the fast check, `conformance/tests/noticed.txt` and `task
   conform-slow`. The bullet of `conformance/` (its head, :970-976 on `main`) names `noticed.txt`
   and its five sources.
6. The lane `slow` (D308). `main` defines the task `slow` at `Taskfile.yml`:466-470, with the
   subtasks `tls-certs-slow` and `tls-flights-slow`, never a step of `check`. The story adds
   `conform-slow` as its third subtask and extends the bullet of `task slow` in `AGENTS.md`
   (:1422-1432 on `main`) with it.

### Overlap and order

What is still open on `main` and touches the same files (the scopes of the stories not
`implemented` on `ff6431f`, round 2's eleven stories left out: their code is on `main`):

- A source of the digest: `story:tls-spec` and `story:agent-cookie-jar` change
  `conformance/src/target.rs`; `story:conformance-lock-and-lints` (draft) changes
  `conformance/src/target.rs` and `conformance/Cargo.lock`; `story:tls-fetch-https` changes
  `conformance/Cargo.lock`, whose `ess-conformance` entry alone is read. Whichever of them lands
  after this story regenerates `noticed.txt` (statement 4).
- `Taskfile.yml`: `story:test-step-holds-with-tls`, `story:agent-cookie-public-suffixes` and
  `story:conformance-lock-and-lints`; by their bodies none changes `slow` (:466-470).
  `story:conformance-lock-and-lints` adds a clippy run on `conformance/` to the gate without
  naming its task; if it goes into `conform-rust` (:105-137), it stays outside the timed line of
  statement 2, and a merge keeps both. `story:test-step-holds-with-tls` statement 3 puts
  overrides in `.cargo/config.toml`, which the conformance build reads too.
- `AGENTS.md`: `story:test-step-holds-with-tls` (active), `story:conformance-lock-and-lints`,
  `story:tls-tests-as-tables` and `story:session-argument-checks` (drafts); this story changes
  only the bullets of `task conform-rust`, `task slow` and `conformance/`.
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
     most 3, and the 2 lines of the controls with the suite's 196.
   - The digest check: with the first line of `conformance/tests/noticed.txt` edited by hand, and
     then with one byte changed in each of the five sources in turn (`.out/suite.json`,
     `conformance/tests/two_of_each_entity.rs`, `conformance/src/target.rs`,
     `conformance/src/values.rs`, the `ess-conformance` entry of `conformance/Cargo.lock`), `task
     conform-rust` fails with the line `conform-rust: the sources of conformance/tests/noticed.txt
     changed; run task conform-slow`. Six runs, each output in the evidence.
   - `task conform-slow` on the story's commit writes `.out/noticed.txt` equal byte for byte to the
     committed file, and `task --dry slow` lists `conform-slow`.
   - With one planted defect taken out of its wrapper (by hand), its test fails naming the
     defect.
   Each plant is removed after (`git diff --stat` equal before and after).
7. `cargo nextest list --manifest-path conformance/Cargo.toml` before and after gives the same 32
   tests: the 15 of `tests/main.rs` and the 17 unit tests of the library.
8. The forge: the `check` run of the story's commit on its branch or pull request prints
   `conformance step: N s of its budget of <budget> s`. The evidence names the run id and quotes
   the line.

## What it does not do

- No change of the root workspace's test step: `story:test-step-holds-with-tls`.
- No change of the suite, of a scenario, of `conformance/src/target.rs`, or of what a
  planted-defect test claims.
- No higher test-thread count, no shards, no raised budget or timeout.
- No Python: the tasks are POSIX sh; the tests and any generator stay Rust.

## Scope

The files of `aep plan artifact scope`. In `AGENTS.md` the bullets of `task conform-rust`, of `task slow` and of
`conformance/`; not the waits of `built` (`story:session-request-timeout-option`) and not the
bullets of `task view-test` and `task check` (`story:page-test-step-within-budget`).

## As decided after the adversary's pass

The adversary's pass on `4c03f2a` added four tests (`conformance/tests/adversary_noticed.rs`, kept,
commit `117b227`) and six findings; none made the step green over a defect that is not noticed.
The decisions, built in `e05e65f` and `7db71c5`, documents in `92ab623`:

- D311, over D309: the digest of `conformance/tests/noticed.txt` covers `.out/suite.json`, every
  file under `conformance/src` and `conformance/tests` except the list itself (read from the
  directories, sorted), `conformance/Cargo.toml` and `conformance/Cargo.lock` whole. Before, the
  fast check passed after `NOTICED_PER_DEFECT` went from 3 to 1. The digest does not cover the
  crates the runner is built against and not the list's own lines.
- D312: for those two cases the planted test's failure names the place: when the listed scenarios
  of a defect all pass, when a label has no line and when a line names a scenario the suite has
  not, the message says the list may be stale and names `task conform-slow WRITE=1`.
- D313: the suite is admitted once per run, not once per test. Measured at a load of 91 to 108:
  admitting the whole suite (2.8 MB, 196 scenarios) took 17.5, 8.7 and 6.0 s, a narrowed suite
  (16 KB, 3 scenarios) 7, 135 and 20 ms. `conform-rust` writes the narrowed suites of the 29
  planted defects once per digest to `conformance/target/noticed/<digest>/` before the timed
  part, and a planted test fails naming that step when its file is missing. The two controls keep
  the whole suite. The step's N went from 41.9, 33.3 and 52.0 s to 37.9, 24.0 and 25.7 s (load
  93.8 to 108.5).
- D314: the step fails when nextest reports a skipped test. Before, an `#[ignore]` on 10 of the 13
  planted tests left it green (there before this story).
- D315: the suite run against the Rust system ends after 60 s (`CONFORM_SUITE_BOUND`) with a line
  that names the step. Before, only the pool ended it.
- D316: `taskfiles/doc-tests-check.sh` reads `conformance/src` too: cargo-nextest runs no doc
  tests, and the crate has none.

Acceptance 7 after this pass: `cargo nextest list` gives 39 tests (32 before the story, 4 of the
adversary, 3 of D312). Acceptance 5 as built: an `#[ignore]` changes a source of the digest, so
each plant is made together with the matching digest line. `task --dry slow` lists `conform-slow`
only with `--verbose`, as it does the other silent tasks of the lane.

Not built: a check that `CONFORM_BUDGET` is a number. The adversary's probe found that 0, 1, -5,
`abc` and `2e3` fail the step and an empty value means 120.
