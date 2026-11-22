---
format: aep.planning-md/3
id: story:tls-tests-as-tables
kind: story
status: draft
title: The TLS tests that are rows of one claim run as table tests
summary: 'wave/tls-1 at 48f05c2: test step 130.1 s at load 110 to 127, over 120 s; opt-level 1 for surf-tls, surf-tls-test-server, surf-tls-test-certs where the rule allows, and 207 TLS tests in 20 groups become table tests (1,432 tests become 1,245); at most 110 s at a load of 100'
relations:
- decomposes: epic:tls-https
- depends_on: story:test-step-holds-with-tls
- depends_on: story:tls-handshake
- depends_on: story:tls-x509
- depends_on: story:tls-test-server
- depends_on: story:tls-test-certs
scope:
- confidence: cited
  path: .cargo/config.toml
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
revision: 11
---
## Why

The rule (`AGENTS.md` of the folder, "Tests stay fast"): the test step ends within 120 s and no
test runs over 60 s; a step over the budget is a defect that is fixed before the next feature.

D256: `wave/tls-1` does not merge to `main` before its test step is within 120 s at a load like
the one measured below. This story is that fix. It is built on `wave/tls-1` at `48f05c2`, which
holds `main` `0f0c157` (merged at `6cff209`) and the eight TLS stories, each `implemented`.

Measured on 2026-11-22 with `slot --timeout 15m task test`, `cat /proc/loadavg` before and after:

| Tree | Load | Tests | nextest `Summary` | `test step` |
|---|---|---|---|---|
| `wave/tls-1` at `48f05c2` | 110 to 127 | 1,432 passed | 108.9 s | 130.1 s, over its budget of 120 s |
| `main` at `0f0c157`, three runs | 118 to 139 | 875 | | 76.2 s, 77.5 s, 82.2 s |
| `wave/tls-1` at `6cff209` | 20 to 54 | 1,432 | | 32.0 s |

During the run on `48f05c2` a second test step ran in the pool at the same time. The wave adds
557 tests to the 875 of `main`, 545 of them in the four TLS packages: `surf-tls` 465, `surf-tls-test-server` 46, `surf-tls-test-certs`
18, `surf-tls-conformance` 16 (`AGENTS.md` on the wave, :724, :792, :769, :662).

The slowest tests of the wave under load, from the JUnit report of the run on `48f05c2`:
`surf-tls` `limbo_parse::bettertls_3` 12.4 s, `bettertls_2` 9.0 s, `bettertls_1` 8.8 s,
`bettertls_4` 7.1 s (37.2 s summed); `surf-tls-test-server`
`adversary_server::adversary_garbage_of_each_length_ends_every_connection` 6.1 s; `surf-tls`
`handshake_hostile::mutated_messages_end_cleanly` 3.3 s; `surf-tls-test-certs`
`check::a_missing_or_empty_digest_or_index_fails` 3.3 s. Each `limbo_parse` test reads one
vendored x509-limbo file of the 20 MiB whole with `surf_html_conformance::json`
(`crates/tls/tests/tls/limbo_parse.rs`:17 and :107-112) and reads each certificate with
`surf_tls::x509` and `surf_tls::pem`, in a build without optimisation. `ring` is built at
`opt-level = 3` on the wave (`.cargo/config.toml`, `[profile.dev.package.ring]`).

The tests that are rows of one claim. Counted from `git show wave/tls-1:<path>` and the wave's
`cargo nextest list`, with times from the gate on `32df67f` (2026-11-22, load 88 to 113). No
TLS test changed since: `git diff --stat 32df67f 48f05c2 -- crates/tls crates/tls-test-server
crates/tls-test-certs crates/tls-conformance` is empty. 20 groups hold 207 tests whose rows run
one claim with the same assertion shape.

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

Merged, 1,432 tests become 1,245 (the 545 TLS tests 358); 187 process starts go, 4 to 9 s summed,
1 to 2 s of wall time at 5 tests at a time. The CPU-bound tests above are the larger cost;
statement 1 takes them first.

## Change

All on `wave/tls-1`, from `48f05c2`. Statement 1 comes first, measured and decided before the
merge of statements 2 to 4, so no table merge counts for an override; then statements 2 to 7 in
any order.

1. `.cargo/config.toml` on the wave: `[profile.dev.package.<name>]` with `opt-level = 1` for each
   candidate that qualifies by the rule of `story:test-step-holds-with-tls` statement 3 (D257).
   Candidates: `surf-tls`, `surf-tls-test-server`, `surf-tls-test-certs`. Rule per candidate: two
   builds per level from `cargo clean -p <name>`, each without a hit of the shared compile cache
   for the package (`sccache --show-stats` before and after each build, in the evidence); the
   build at opt-level 1 is at most 30 s longer than at 0; and the summed test time of the tests
   counted for it is at most half. Otherwise the override is not taken. Tests counted per
   candidate:
   - `surf-tls`: its 465 tests, `limbo_parse::*` (7) among them, and the 16 tests of
     `surf-tls-conformance`, which read through it.
   - `surf-tls-test-server`: its 46 tests,
     `adversary_server::adversary_garbage_of_each_length_ends_every_connection` among them.
   - `surf-tls-test-certs`: its 18 tests, `check::a_missing_or_empty_digest_or_index_fails` among
     them.
   `[profile.dev.package.ring]` keeps `opt-level = 3`. The overrides of `main` (`surf-html`,
   `surf-url`, `surf-page-text`) stay. `AGENTS.md` on the wave, the bullet of
   `.cargo/config.toml` (:230), gains one sentence per override taken.
2. Each group becomes one test that loops over a table of its rows and runs the same assertions
   per row. Every failure text begins with `row <row name>:`; helpers that name no row today,
   such as `refused_by` and the `Err(E)` comparisons, gain it. A row that is a mutant's name in
   `adversary_x509.rs` keeps that name as its row name.
3. `rfc8446-client-musts.txt`, `rfc8446-client-fields.txt` and `rfc8446-record-musts.txt` name 61
   of the grouped tests as `tests/tls/<file>.rs::<fn>` rows. `check_named_table`
   (`handshake.rs`:1648) and `record::musts_rows_name_tests` (`record.rs`:1165) require a `#[test]
   fn` per row. A row names the table test and its row (`<file>.rs::<test>#<row>`), and both
   checkers read the rows of a table test.
4. `path.rs`: `every_case_of_index_has_its_test` (:263) compares INDEX with the table, not with
   the macro's `CASES`; `index_cases!` (:213) goes.
5. `AGENTS.md` on the wave states the new counts: `surf-tls` at :724-748 (465 tests, `seam` 48,
   `record` 29, `path` 71, `handshake_hostile` 73 and the other modules), `surf-tls-test-certs` at
   :769, `surf-tls-test-server` at :792, `surf-tls-conformance` at :662. `README.md` gets no TLS
   count.
6. The tests that each read a whole data file (`limbo_parse::*`, `wycheproof_*`), the `sh` runs
   of `anchors.rs`, and the tests named after a single mutant that are not in a group stay as
   they are.
7. `crates/surf/tests/lock_check.rs` (on the wave since `story:tls-lock-allow`, `87fd19f`) runs
   `sh taskfiles/lock-check.sh <lock> <allow-list>` in its copy per case, not `task lock-check`
   (`Command::new("task")` at :59 today), as `story:test-step-holds-with-tls` statement 13 does
   for `generated-compare` and `conform-totals` on `main`. The wave's script, its interface and
   the task (`Taskfile.yml`:154 on the wave) stay as they are; the module comment of
   `lock_check.rs` says the test runs the script.

## Acceptance

Every command runs in a tree of `wave/tls-1`; "the story's commit" is the wave's commit with
this story.

1. Step time. Three consecutive runs of `slot --timeout 15m task test` on the story's commit, each
   with `cat /proc/loadavg` before and after and both readings at least 100, each print `test
   step: N s of its budget of 120 s` with N at most 110. Every run started for this story is in
   the evidence with its load; none is dropped. If the load is under 100 while the unit works,
   the runs are reported with their load, this statement stays open, and the wave stays off
   `main` until three runs at such a load are shown.
2. Statement 1: the evidence holds one table per candidate: for each level the two `Finished`
   lines of `cargo build` after `cargo clean -p <name>`, the `sccache --show-stats` before and
   after each build, and the summed test time of the counted tests (`cargo nextest run -p <name>`
   and, for `surf-tls`, `-p surf-tls-conformance`, from the JUnit report); then "taken" or "not
   taken" by the rule. `git diff 48f05c2 -- .cargo/config.toml` adds `opt-level = 1` tables for
   the taken candidates only, and `[profile.dev.package.ring]` still reads `opt-level = 3`.
3. Per merged group, on the story's commit: one row given a wrong expectation (planted by hand,
   removed after, `git diff --stat` equal before and after) makes `cargo nextest run -p <package>
   -E 'test(=<table test>)'` fail with a line that begins `row <that row's name>:`. Twenty plants,
   each failure line in the evidence.
4. A line `tests/tls/handshake_hostile.rs::<table test>#no-such-row` added to
   `rfc8446-client-musts.txt` (planted, removed after) makes `cargo nextest run -p surf-tls -E
   'test(=handshake::musts_rows_name_tests)'` fail with a message that quotes that line. The same
   plant in `rfc8446-record-musts.txt` makes `record::musts_rows_name_tests` fail the same way.
5. `cargo nextest list --workspace` prints 1,432 tests on `48f05c2` and 1,245 on the story's
   commit (or the count reached, named). The evidence maps each merged test to its table test and
   row; each merged test's assertions are in its row.
6. `cargo nextest run -p surf-tls -E 'test(=path::every_case_of_index_has_its_test)'` passes on the
   story's commit, and `git grep -n 'index_cases!' crates/tls/tests` prints nothing.
7. `grep -n 'Tests: ' AGENTS.md` on the story's commit shows the count of `surf-tls` equal to
   `cargo nextest list -p surf-tls` on that commit; the counts of the three other TLS packages
   equal theirs.
8. `git grep -n 'Command::new("task")' crates/surf/tests/lock_check.rs` prints nothing, `git diff
   48f05c2 -- taskfiles/lock-check.sh` is empty, and a `taskfiles/lock-check.sh` that skips the
   check of the allow-list (planted, removed after) makes `cargo nextest run -p surf -E
   'test(/^lock_check::/)'` fail with the package named.

## What it does not do

- No test of `main` beyond those the merge of `main` brought: `story:test-step-holds-with-tls`.
- No change of a TLS module's code, of `ring`'s profile or of a data file.
- No override for a package that is not a candidate of statement 1.
- No higher test-thread count, no shards, no raised budget.
