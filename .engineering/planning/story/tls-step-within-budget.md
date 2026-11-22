---
format: aep.planning-md/3
id: story:tls-step-within-budget
kind: story
status: draft
title: The TLS tests run in a gate step of their own within 120 s
summary: 'wave/tls-1 at 48f05c2: task test 130.1 s at load 110 to 127, over 120 s, 545 of its 1,432 tests in the four TLS packages; task test-tls runs those under the 60 s end with its own budget of 120 s; the wave merges when both steps hold'
relations:
- decomposes: epic:tls-https
- informed_by: story:test-step-holds-with-tls
scope:
- confidence: cited
  path: .config/nextest.toml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
revision: 2
---
## Why

The rule (`AGENTS.md` of the folder, "Tests stay fast"): a test step ends within its budget of
120 s, no test runs over 60 s, the step prints its ten slowest tests, and a step over its budget
is a defect fixed before the next feature.

Measured on 2026-11-22 with `slot --timeout 15m task test`, `cat /proc/loadavg` before and after:

| Tree | Load | nextest `Summary` | `test step` |
|---|---|---|---|
| `wave/tls-1` at `48f05c2` | 110 to 127 | 1,432 passed, 108.9 s | 130.1 s, over its budget of 120 s |
| `main` at `0f0c157`, three runs | 118 to 139 | 875 | 76.2 s, 77.5 s, 82.2 s |
| `wave/tls-1` at `6cff209` | 20 to 54 | 1,432 | 32.0 s |

During the run on `48f05c2` a second test step ran in the pool at the same time. The wave adds
557 nextest tests to the 875 of `main`. 545 of them are in the four TLS packages (`AGENTS.md` on
the wave): `surf-tls` 465 (:724), `surf-tls-test-server` 46 (:792), `surf-tls-test-certs` 18
(:769), `surf-tls-conformance` 16 (:662). The other 887 tests on `48f05c2` are in the packages of
`main`, `crates/surf/tests/lock_check.rs` among them.

Decision (by the operator's order, 2026-11-22 22:49 CET, replacing D256): the TLS tests get a gate
step of their own with its own budget of 120 s, so they do not add to the 120 s of `task test`.
The wave merges to `main` when its gate is green with both steps within their budgets. The table
rewrite and the opt-level work of `story:tls-tests-as-tables` are not a precondition of the
merge.

The tree: this story is built on `wave/tls-1` at `48f05c2`, which holds `main` `0f0c157` (merged
at `6cff209`) and the eight TLS stories, each `implemented`. The tests that `story:tls-spec`,
`story:tls-fetch-https`, `story:tls-session-anchors` and `story:tls-release` add to the four TLS
packages run in `test-tls`; their tests in other packages run in `task test`.

## Change

1. `.config/nextest.toml` gains a profile `tls`, which inherits `default` (`fail-fast = false`,
   `slow-timeout` of 60 s with `terminate-after = 1`), with `[profile.tls.junit] path =
   "junit.xml"`. Its JUnit report is `target/nextest/tls/junit.xml`; the report of `task test`
   stays `target/nextest/default/junit.xml`, so neither run overwrites the other.
2. `Taskfile.yml`, task `test` (:282-344 on the wave): `cargo nextest run --workspace --exclude
   surf-tls --exclude surf-tls-test-server --exclude surf-tls-test-certs --exclude
   surf-tls-conformance`, the rest as today: the doc tests, the ten slowest, the `Summary`, the
   count line and `test step: N s of its budget of 120 s` or the `over its budget` form.
3. `Taskfile.yml`, a task `test-tls`, POSIX sh, after `task build`: `cargo nextest run --profile
   tls --locked --color never -p surf-tls -p surf-tls-test-server -p surf-tls-test-certs -p
   surf-tls-conformance` into `.out/test-tls.log`, timed alone. It prints `the ten slowest tests:`
   and `sh taskfiles/slowest-tests.sh target/nextest/tls/junit.xml 10`, nextest's `Summary` line,
   the count line `N tests: P passed, F failed, I ignored`, then `tls test step: N s of its budget
   of 120 s`, or `tls test step: N s, over its budget of 120 s` and exit 1. The budget is the
   variable `TLS_TEST_BUDGET`, default 120. It fails when a test failed. It removes
   `.out/tls-test-step.seconds` at its start and writes N there at its end.
4. The list of `check` (:350-372 on the wave) runs `test-tls` right after `test`. The forge runs
   `task check`, so it runs both steps.
5. The sum line. Where `step-budgets` of `story:page-test-step-within-budget` exists (on
   `wave/round-2` once that story is built), it counts four steps: the merge of `wave/tls-1` and
   `wave/round-2`, whichever reaches `main` second, extends `step-budgets` to read
   `.out/tls-test-step.seconds` too and to print `four steps: N s (test step A s, tls test step
   T s, conformance step B s, page step C s)`.
6. `AGENTS.md` on the wave: the bullet of `task test` (:1308) says which four packages it leaves
   out; a new bullet of `task test-tls` after it names the step, its profile, its JUnit file, its
   line and its budget; the paragraph of `task check` (:1127) names the step.

### Overlap

- `AGENTS.md`: this story changes the bullets of `task test` and `task test-tls` and the paragraph
  of `task check` on the wave. `story:tls-tests-as-tables` changes the TLS counts (:662,
  :724-748, :769, :792) and the bullet of `.cargo/config.toml` (:230).
  `story:conformance-step-within-budget` changes the bullets of `task conform-rust`, `task slow`
  and `conformance/`; `story:page-test-step-within-budget` the bullets of `task view-test` and the
  page tests' times, and also the paragraph of `task check`; the merge keeps both sides there.
  `story:doc-tests-of-one-package` changes the bullet of `task test` (the doc tests); the merge
  keeps both sides.
- `Taskfile.yml`: `story:doc-tests-of-one-package` changes the doc-test command of `test`, this
  story its nextest command; the merge keeps both.

## Acceptance

Every command runs in a tree of `wave/tls-1` at the story's commit, unless named otherwise.

1. `slot --timeout 30m task check > .out/check.log 2>&1`, with `cat /proc/loadavg` before and
   after, exits 0, and `.out/check.log` holds `test step: N s of its budget of 120 s` and `tls
   test step: N s of its budget of 120 s`. The evidence quotes both lines with the load.
2. The counts: `test` prints `(cargo-nextest 887, doc tests 2)` in its count line and `test-tls`
   prints `545 tests: 545 passed, 0 failed, 0 ignored`; 887 and 545 sum to the 1,432 of
   `48f05c2`. `target/nextest/default/junit.xml` and `target/nextest/tls/junit.xml` both exist
   after the run.
3. `TLS_TEST_BUDGET=1 slot task test-tls` prints `tls test step: N s, over its budget of 1 s` and
   exits non-zero.
4. A TLS test planted to fail (`assert!(false)` in one test of `crates/tls/tests/tls/der.rs`, by
   hand, removed after, `git diff --stat` equal before and after): `slot task test-tls` exits
   non-zero with that test's `FAIL` line, and `slot task test` exits 0.
5. The forge: the `check` run of the story's commit on its branch or pull request prints `test
   step: N s of its budget of 120 s` and `tls test step: N s of its budget of 120 s`. The evidence
   names the run id and quotes both lines.
6. `grep -n 'task test-tls' AGENTS.md` shows the new bullet and the paragraph of `task check`.

## What it does not do

- No change of a test, of a TLS module's code or of `.cargo/config.toml`:
  `story:tls-tests-as-tables`.
- No higher test-thread count, no shards, no raised budget.
- No Python: the task is POSIX sh.
