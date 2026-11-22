---
format: aep.planning-md/3
id: story:tls-tests-as-tables
kind: story
status: draft
title: The TLS tests that are rows of one claim run as table tests
summary: 207 TLS tests in 20 groups each run one claim per row; each group becomes one test over its table with the row in every failure, the rfc8446 tables name rows; 1,432 tests become 1,245 on the wave
relations:
- decomposes: epic:tls-https
- depends_on: story:test-step-holds-with-tls
- depends_on: story:tls-handshake
- depends_on: story:tls-x509
- depends_on: story:tls-test-server
- depends_on: story:tls-test-certs
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/surf/tests/lock_check.rs
- confidence: cited
  path: crates/tls-conformance/tests
- confidence: cited
  path: crates/tls-test-certs/tests
- confidence: cited
  path: crates/tls-test-server/tests
- confidence: cited
  path: crates/tls/tests/tls
revision: 8
---
## Why

The rule (`AGENTS.md` of the folder, "Tests stay fast"): the test step ends within 120 s and no
test runs over 60 s. The gate on `wave/tls-1` at `32df67f` (2026-11-22, 19:51 to 19:58 CET, load
average 88 to 113 on 20 CPUs) ended red on time alone: `Summary [ 107.519s] 1432 tests run: 1432
passed`, `test step: 126.3 s, over its budget of 120 s`. Most of that is the tests of `main`
(`story:test-step-holds-with-tls`). The wave adds 557 tests: `surf-tls` 465 (75.8 s summed),
`surf-tls-test-server` 46 (9.8 s), `surf-tls-test-certs` 18 (6.9 s), `surf-tls-conformance` 16
(1.5 s). None is over 2.8 s; each costs one process start, 0.02 to 0.05 s at that load, and the
step runs 5 tests at a time.

Counted from `git show wave/tls-1:<path>` and the wave's `cargo nextest list`, with times from
that gate: 20 groups hold 207 tests whose rows run one claim with the same assertion shape.

| Group | Tests | Summed |
|---|---|---|
| `crates/tls/tests/tls/handshake_hostile.rs`: tests that end in `refused(..)` / `refused_by(..)` | 46 | 5.0 s |
| `path.rs`: `index_cases!`, one test per row of INDEX | 33 | 2.2 s |
| `x509_parse.rs` (9) and `adversary_x509.rs` (11): `Certificate::from_der` is `Err(E)` | 20 | 1.3 s |
| `der.rs` (15) and `adversary_x509.rs` (1): `der::parse` is `Err(E)` | 16 | 1.0 s |
| `surf-tls-test-server` `handshake.rs`: `suite_and_group` (6) and `scheme` (6) | 12 | 0.3 s |
| `record.rs`: `reader(..)` then `refused(..)` | 9 | 0.6 s |
| `surf-tls-test-certs` `check.rs` (7) and `adversary.rs` (2): a changed copy fails the check | 9 | 4.3 s |
| `surf-tls-conformance` `check.rs`: `compare(..)` failures | 8 | 0.6 s |
| `names.rs` (7), `adversary_path.rs` (7), `pem.rs` and `adversary_x509.rs` (6), `adversary_handshake.rs` (5), `seam.rs` sign (5) and ring search (4), `tls-test-server` `modes.rs` (5) and `adversary_server.rs` (4 + 2), `handshake.rs` traces (3) and outcomes (3), `rfc8448.rs` replays (3) | 54 | 4.0 s |

Merged, 1,432 tests become 1,245 (545 TLS tests 358); 187 process starts go, 4 to 9 s summed,
1 to 2 s of wall time. The gain is small; the story is worth it for the count of tests the wave
keeps adding, and it is ordered after the story that takes the large costs.

## Change

On `wave/tls-1`, after `story:test-step-holds-with-tls` is on `main` and merged into the wave.

1. Each group becomes one test that loops over a table of its rows and runs the same assertions
   per row. Every failure text names the row (helpers that name none today, such as `refused_by`
   and the `Err(E)` comparisons, gain `{row}`). A row that is a mutant's name in
   `adversary_x509.rs` keeps that name as its row name.
2. `rfc8446-client-musts.txt`, `rfc8446-client-fields.txt` and `rfc8446-record-musts.txt` name 61
   of the grouped tests as `tests/tls/<file>.rs::<fn>` rows, and `check_named_table`
   (`handshake.rs`) requires a `#[test] fn` per row. A row names the table test and its row
   (`<file>.rs::<test>#<row>`), and the checker reads the rows of a table test.
3. `path.rs`: `every_case_of_index_has_its_test` compares INDEX with the table, not with the
   macro's `CASES`; `index_cases!` goes.
4. `AGENTS.md` on the wave, :713-730 (the per-module TLS counts: 465, `seam` 48, `record` 29),
   states the new counts. `README.md` gets no TLS count.
5. The tests that each read a whole data file (`limbo_parse::*`, `wycheproof_*`), the `sh` runs of
   `anchors.rs`, and the tests named after a single mutant that are not in a group stay as they
   are.
6. `crates/surf/tests/lock_check.rs` (on the wave since `story:tls-lock-allow`, `87fd19f`) runs
   `sh taskfiles/lock-check.sh <lock> <allow-list>` in its copy per case, not `task lock-check`,
   as `story:test-step-holds-with-tls` statement 13 does for `generated-compare` and
   `conform-totals` on `main`. The wave's script, its interface and the task (`Taskfile.yml`:190
   there) stay as they are; the module comment of `lock_check.rs` says the test runs the script.
   On `main` (4 tests, 4.1 s summed in the baseline) the test is not changed.

## Acceptance

1. Step time on the wave: three consecutive runs of `task test` on the wave with this story,
   each with `cat /proc/loadavg` before and after and both readings at least 100, print `test
   step: N s` with N at most 110. Every run started for this story is in the evidence; none is
   dropped. If the load is under 100 while the unit works, the runs are reported with their load
   and this statement stays open.
2. Per merged group: one row given a wrong expectation (planted, removed after) turns its test
   red, and the failure text names that row.
3. A line of `rfc8446-client-musts.txt` whose test has no such row (planted) makes
   `check_named_table` fail and name the line.
4. `cargo nextest list` on the wave before and after: 1,432 and 1,245 (or the count reached), with
   the list of merged tests and the table test and row each went into; no assertion is dropped.
5. `git grep -n 'Command::new("task")' crates/surf/tests/lock_check.rs` prints nothing,
   `git diff <base> -- taskfiles/lock-check.sh` is empty, and a `taskfiles/lock-check.sh` that
   skips the check of the allow-list (planted, removed after) turns `lock_check`'s tests red with
   the package named.

## What it does not do

- No test of `main`: `story:test-step-holds-with-tls`.
- No change of a TLS module's code, of `ring`'s profile or of a data file.
- No higher test-thread count, no shards, no raised budget.
