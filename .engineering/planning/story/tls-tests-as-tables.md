---
format: aep.planning-md/3
id: story:tls-tests-as-tables
kind: story
status: draft
title: The TLS tests that are rows of one claim run as table tests
summary: 207 TLS tests in 20 groups become 20 table tests (545 TLS tests become 358), opt-level 1 for surf-tls, surf-tls-test-server, surf-tls-test-certs where the rule allows; lowers test-tls, not a precondition of the wave's merge
relations:
- decomposes: epic:tls-https
- depends_on: story:test-step-holds-with-tls
- depends_on: story:tls-handshake
- depends_on: story:tls-x509
- depends_on: story:tls-test-server
- depends_on: story:tls-test-certs
- depends_on: story:tls-step-within-budget
- depends_on: story:tls-fetch-https
scope:
- confidence: cited
  path: .cargo/config.toml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/tls-conformance/tests
- confidence: cited
  path: crates/tls-test-certs/tests
- confidence: cited
  path: crates/tls-test-server/tests
- confidence: cited
  path: crates/tls/tests/tls
- confidence: cited
  path: crates/tls/tests/tls/path.rs
revision: 17
---
## Why

The rule (`AGENTS.md` of the folder, "Tests stay fast"): a test step ends within its budget of
120 s and no test runs over 60 s; an expensive step runs once, not per test.

The TLS tests run in a gate step of their own, `test-tls`, with its own budget of 120 s
(`story:tls-step-within-budget`, which this story depends on). The wave merges to `main` when its
gate is green with both steps within their budgets; this story is not a precondition of that
merge. It lowers the time and the count of `test-tls`. It is built on `wave/tls-1` from
`48f05c2`, after `story:tls-step-within-budget`.

Measured on 2026-11-22 with `slot --timeout 15m task test`, `cat /proc/loadavg` before and after:

| Tree | Load | Tests | nextest `Summary` | `test step` |
|---|---|---|---|---|
| `wave/tls-1` at `48f05c2` | 110 to 127 | 1,432 passed | 108.9 s | 130.1 s, over its budget of 120 s |
| `main` at `0f0c157`, three runs | 118 to 139 | 875 | | 76.2 s, 77.5 s, 82.2 s |
| `wave/tls-1` at `6cff209` | 20 to 54 | 1,432 | | 32.0 s |

During the run on `48f05c2` a second test step ran in the pool at the same time. The wave adds
557 tests to the 875 of `main`, 545 of them in the four TLS packages that `test-tls` runs:
`surf-tls` 465, `surf-tls-test-server` 46, `surf-tls-test-certs` 18, `surf-tls-conformance` 16
(`AGENTS.md` on the wave, :724, :792, :769, :662).

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
one claim with the same assertion shape; statement 2 lists them with their table tests. Merged,
the 545 TLS tests become 358 and the workspace's 1,432 become 1,245; 187 process starts go, 4 to
9 s summed, 1 to 2 s of wall time at 5 tests at a time. The CPU-bound tests above are the larger
cost; statement 1 takes them first.

## Change

All on `wave/tls-1`, after `story:tls-step-within-budget`. Statement 1 comes first, measured and
decided before the merge of statement 2, so no table merge counts for an override; then
statements 2 to 6 in any order.

1. `.cargo/config.toml` on the wave: `[profile.dev.package.<name>]` with `opt-level = 1` for each
   candidate that qualifies by the rule of `story:test-step-holds-with-tls` statement 3 (D257).
   Candidates: `surf-tls`, `surf-tls-test-server`, `surf-tls-test-certs`. Rule per candidate: two
   builds per level from `cargo clean -p <name>`, each without a hit of the shared compile cache
   for the package (`sccache --show-stats` before and after each build); the build at opt-level
   1 is at most 30 s longer than at 0; and the summed test time of the tests counted for it is at
   most half. Otherwise the override is not taken. Each build and each test run is read with
   `cat /proc/loadavg` before and after it. Tests counted per candidate:
   - `surf-tls`: its 465 tests, `limbo_parse::*` (7) among them, and the 16 tests of
     `surf-tls-conformance`, which read through it.
   - `surf-tls-test-server`: its 46 tests,
     `adversary_server::adversary_garbage_of_each_length_ends_every_connection` among them.
   - `surf-tls-test-certs`: its 18 tests, `check::a_missing_or_empty_digest_or_index_fails` among
     them.
   `[profile.dev.package.ring]` keeps `opt-level = 3`. The overrides of `main` (`surf-html`,
   `surf-url`, `surf-page-text`) stay. `AGENTS.md` on the wave, the bullet of
   `.cargo/config.toml` (:230), gains one sentence per override taken.
2. Each group becomes one table test that runs every row, prints `row <row name>` on standard
   error before each row (so a row ended at 60 s is the last one printed), and fails once at the
   end with one line `row <row name>: <failure>` per failing row. Helpers that name no row today,
   such as `refused_by` and the `Err(E)` comparisons, return their failure text instead of
   panicking. A row that is a mutant's name in `adversary_x509.rs` keeps that name as its row
   name. The 20 groups and their table tests (package `surf-tls` unless named):
   1. `handshake_hostile.rs`, the 46 tests that end in `refused(..)` or `refused_by(..)`:
      `handshake_hostile::refusals`.
   2. `path.rs`, the 33 tests of `index_cases!`: `path::index_rows`.
   3. `x509_parse.rs` (9) and `adversary_x509.rs` (11), `Certificate::from_der` is `Err(E)`:
      `x509_parse::from_der_refusals`.
   4. `der.rs` (15) and `adversary_x509.rs` (1), `der::parse` is `Err(E)`:
      `der::parse_refusals`.
   5. `surf-tls-test-server` `handshake.rs`, `suite_and_group` (6):
      `handshake::suite_and_group_rows`.
   6. `surf-tls-test-server` `handshake.rs`, `scheme_*` (6): `handshake::scheme_rows`.
   7. `record.rs`, `reader(..)` then `refused(..)` (9): `record::refusals`.
   8. `surf-tls-test-certs` `check.rs` (7) and `adversary.rs` (2), a changed copy fails the check:
      `check::changed_copies_fail`.
   9. `surf-tls-conformance` `check.rs`, `compare(..)` failures (8): `check::compare_failures`.
   10. `names.rs` (7): `names::rows`.
   11. `adversary_path.rs` (7): `adversary_path::rows`.
   12. `pem.rs` and `adversary_x509.rs` (6): `pem::refusals`.
   13. `adversary_handshake.rs` (5): `adversary_handshake::rows`.
   14. `seam.rs`, sign (5): `seam::sign_rows`.
   15. `seam.rs`, ring search (4): `seam::ring_search_rows`.
   16. `surf-tls-test-server` `modes.rs` (5): `modes::rows`.
   17. `surf-tls-test-server` `adversary_server.rs` (4 and 2): `adversary_server::rows`.
   18. `handshake.rs`, traces (3): `handshake::trace_rows`.
   19. `handshake.rs`, outcomes (3): `handshake::outcome_rows`.
   20. `rfc8448.rs`, replays (3): `rfc8448::replay_rows`.
   The unit writes the mapping of the 207 tests, one line `<package> <test> -> <table
   test>#<row>`, into the evidence.
3. `rfc8446-client-musts.txt`, `rfc8446-client-fields.txt` and `rfc8446-record-musts.txt` name 61
   of the grouped tests as `tests/tls/<file>.rs::<fn>` rows. `check_named_table`
   (`handshake.rs`:1648, called by `musts_rows_name_tests` at :1719 and `fields_rows_name_tests`
   at :1729) and the check of `record::musts_rows_name_tests` (`record.rs`:1165) require a
   `#[test] fn` per row. A row names the table test and its row (`<file>.rs::<test>#<row>`), and
   both checkers read the rows of a table test.
4. `path.rs`: `every_case_of_index_has_its_test` (:263) compares INDEX with the table, not with
   the macro's `CASES`; `index_cases!` (:213) goes.
5. `AGENTS.md` on the wave states the new counts: `surf-tls` at :724-748 (`Tests: 465 in one
   binary`, `seam` 48, `record` 29, `path` 71, `handshake_hostile` 73 and the other modules),
   `surf-tls-conformance` at :662 (`16 tests (`), `surf-tls-test-certs` at :769 (`18 tests (`),
   `surf-tls-test-server` at :792 (`46 tests (`). `README.md` gets no TLS count.
6. The tests that each read a whole data file (`limbo_parse::*`, `wycheproof_*`), the `sh` runs
   of `anchors.rs`, and the tests named after a single mutant that are not in a group stay as
   they are.
7. (Taken by `story:test-step-scripts-and-binaries`, statement 3: `crates/surf/tests/lock_check.rs`
   runs `sh taskfiles/lock-check.sh` instead of `task lock-check`, on `main`. This story no
   longer changes that file.)

### Overlap and order

- `story:tls-fetch-https` scopes `crates/surf/tests`; this story depends on it and changes no
  file there (`lock_check.rs` went to `story:test-step-scripts-and-binaries` with statement 7).
- `story:tls-release` depends on this story.
- `AGENTS.md` on the wave: this story changes the bullet of `.cargo/config.toml` (:230) and the
  TLS counts (:662, :724-748, :769, :792). `story:tls-step-within-budget` changes the bullets of
  `task test` and `task test-tls` and the paragraph of `task check`;
  `story:conformance-step-within-budget` the bullets of `task conform-rust`, `task slow` and
  `conformance/`; `story:page-test-step-within-budget` the bullets of `task view-test` and `task
  check` and the page tests' times; `story:doc-tests-of-one-package` the bullet of `task test`
  and of `task slow`; the round-2 branches the sections of their own stories. None of them
  changes the lines of this story.

## Acceptance

Every command runs in a tree of `wave/tls-1`; "before" is the commit of
`story:tls-step-within-budget`, "after" the story's commit.

1. `test-tls`: three consecutive runs of `slot --timeout 15m task test-tls` before and three
   after, each with `cat /proc/loadavg` before and after, each exits 0 and prints `tls test step:
   N s of its budget of 120 s`. The evidence holds the six times with their loads. Stop: if a run
   after every statement is built, at a load of 100 or more, is over 120 s, the unit reports the
   ten largest remaining costs from `target/nextest/tls/junit.xml` of that run, and the story
   stays `active`.
2. Statement 1: the evidence holds one table per candidate: for each level the two `Finished`
   lines of `cargo build` after `cargo clean -p <name>`, the `sccache --show-stats` before and
   after each build, and the summed test time of the counted tests (`cargo nextest run -p <name>`
   and, for `surf-tls`, `-p surf-tls-conformance`, from the JUnit report); each build and each
   test run with its two load readings; then "taken" or "not taken" by the rule. `git diff
   48f05c2 -- .cargo/config.toml` adds `opt-level = 1` tables for the taken candidates only, and
   `[profile.dev.package.ring]` still reads `opt-level = 3`.
3. Statement 2, per table test of the list: one row given a wrong expectation (planted by hand,
   removed after, `git diff --stat` equal before and after) makes `cargo nextest run -p
   <package> -E 'test(=<table test>)'` fail with the line `row <that row's name>: …`, and the
   test's output shows `row <name>` for every row. Twenty plants, each failure in the evidence.
4. Statement 3, one plant per name file, each a line `<file>.rs::<table test>#no-such-row` added
   by hand and removed after: in `rfc8446-client-musts.txt` it makes `cargo nextest run -p surf-tls
   -E 'test(=handshake::musts_rows_name_tests)'` fail; in `rfc8446-client-fields.txt`,
   `handshake::fields_rows_name_tests`; in `rfc8446-record-musts.txt`,
   `record::musts_rows_name_tests`. Each failure quotes the planted line.
5. The counts: `cargo nextest list --workspace` prints 1,432 tests on `48f05c2` and 1,245 after;
   `task test-tls` prints `358 tests: 358 passed, 0 failed, 0 ignored` after. The mapping of
   statement 2 is checked against the two lists: the list after equals the list on `48f05c2`
   without the 207 mapped tests and with the 20 table tests, compared with `sort` and `comm` in
   a command quoted in the evidence that prints nothing.
6. `cargo nextest run -p surf-tls -E 'test(=path::every_case_of_index_has_its_test)'` passes after,
   and `git grep -n 'index_cases!' crates/tls/tests` prints nothing.
7. `grep -nE 'Tests: [0-9]+ in one binary|[0-9]+ tests \(' AGENTS.md` after shows the counts of
   `surf-tls`, `surf-tls-conformance`, `surf-tls-test-certs` and `surf-tls-test-server`, each
   equal to `cargo nextest list -p <package>` on the story's commit.
8. (Taken by `story:test-step-scripts-and-binaries` with statement 7.)

## What it does not do

- No test of `main` beyond those the merge of `main` brought: `story:test-step-holds-with-tls`.
- No change of a TLS module's code, of `ring`'s profile or of a data file.
- No override for a package that is not a candidate of statement 1.
- No change of `test-tls` or of `task test`: `story:tls-step-within-budget`.
- No higher test-thread count, no shards, no raised budget.
