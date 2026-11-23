---
format: aep.planning-md/3
id: story:test-step-scripts-and-binaries
kind: story
status: draft
title: The test step's scripts start fewer programs and it lists no empty test binary
summary: generated-compare starts at most 25 programs per run (72 today), lock_check runs its script without task (4.2 s per call in the pool), 26 empty test binaries leave the step, and the doc step runs beside nextest if measured
relations:
- decomposes: epic:headless-rust-target
- informed_by: story:test-step-holds-with-tls
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/agent-tasks/Cargo.toml
- confidence: cited
  path: crates/agent/Cargo.toml
- confidence: cited
  path: crates/answer/Cargo.toml
- confidence: cited
  path: crates/html-conformance/Cargo.toml
- confidence: cited
  path: crates/page-text/Cargo.toml
- confidence: cited
  path: crates/page/Cargo.toml
- confidence: cited
  path: crates/ports/Cargo.toml
- confidence: cited
  path: crates/secret/Cargo.toml
- confidence: cited
  path: crates/session/Cargo.toml
- confidence: cited
  path: crates/surf-cli/Cargo.toml
- confidence: cited
  path: crates/surf/Cargo.toml
- confidence: cited
  path: crates/surf/tests/common/mod.rs
- confidence: cited
  path: crates/surf/tests/lock_check.rs
- confidence: cited
  path: crates/surfd/Cargo.toml
- confidence: cited
  path: crates/url/Cargo.toml
- confidence: cited
  path: crates/view/Cargo.toml
- confidence: cited
  path: taskfiles/generated-compare.sh
- confidence: cited
  path: taskfiles/test-layout.sh
- confidence: cited
  path: taskfiles/test-layout.yml
revision: 22
---
## Why

The rule of the folder this repository is worked in ("Tests stay fast"): the test step ends within
120 s of wall time and an expensive step does not run inside each test. On `main` at `96145a2`
four runs of `slot --timeout 15m task test` (2026-11-23, 01:07 to 01:19 CET) printed
`test step:` 131.6 s, 71.4 s, 132.6 s and 102.9 s, at a 1-minute load of 205 to 127, 127 to 106,
106 to 137 and 137 to 153. `story:test-step-holds-with-tls` asks for at most 90 s at a load of
100 or more; this story removes four costs that are not the tests' own subject.

How the step spends its time, measured on `96145a2`:

- Five tests run at once. The pool's daemon starts with `--test-threads 5` (its user unit) and
  passes that to nextest; neither `.config/nextest.toml` nor `Taskfile.yml` sets a number. The
  summed test time over nextest's `Summary` is 4.94, 4.92, 4.87 and 4.90 in the four runs (their
  JUnit reports), and fewer than five tests ran for only 3.1, 7.1 and 5.5 s of runs 2 to 4. So
  one second of summed test time is about 0.2 s of wall time.
- In the pool a started program costs 20 to 30 times its time outside it: the pool's jobs share a
  CPU quota of 1800 % at CPU weight 20 (`systemctl --user show slot.service`), and the host's
  CPU pressure ("some", 10 s) read 85 %. One `task lock-check` takes 171 to 346 ms outside the
  pool (five runs in a copy, load 137) and about 4.2 s inside it (below).
- The step minus nextest's `Summary` was 14.3, 9.0, 20.6 and 11.9 s in the four runs: cargo's
  build check (`Finished ... in 0.18s` to `0.25s` in `.out/test.log`), nextest's listing of the
  test binaries, and the doc step.

The four costs:

1. `generated-compare`. Sixteen cases of four modules of `surf` (`review1_generated_check` 6
   runs of the script, `review2_generated_check` 2, `generated_check_links` 6,
   `generated_check_modes` 2) run `sh taskfiles/generated-compare.sh <fresh tree>` in a copy
   (`crates/surf/tests/common/mod.rs:64-74`). That is the subject of the tests, but one run
   starts 72 programs when step 1 fails and 77 when both steps pass (strace -f outside the pool,
   load 153): `cmp` 30 (the loop per file, `taskfiles/generated-compare.sh:64-66`), `sed` 13,
   `comm` 8, `sort` 7, `find` 7. Eleven of them list the fresh tree (`:39-41`, `:50-51`), which is
   the same tree in every case. Outside the pool a run takes 175 to 199 ms with step 1 failing
   and 320 to 409 ms with both steps (three runs each, load 137). Alone in the pool,
   `generated_check_links` took 32.8 s summed for 6 cases (Summary 8.8 s, load 173 to 178) and
   `review1_generated_check` 29.4 s for 4 tests with 6 runs (load 138 to 142). In the third run
   of the step (132.6 s) the four modules held 51.6 s summed (23.9, 10.7, 11.5 and 5.5 s).
2. The doc step. `cargo test --doc -p surf-ports` runs after nextest (`Taskfile.yml:330`):
   "all doctests ran in 3.70s; merged doctests compilation took 2.38s" in the fourth run's
   `.out/test-doc.log`; alone at load 160 to 172 it printed 9.91 s and 9.30 s, compilation
   7.42 s and 8.20 s. Most of it is rustdoc compiling, which needs no test slot of nextest.
3. `lock_check`. Its 16 tests run `task lock-check` in a copy 18 times
   (`crates/surf/tests/lock_check.rs:57-67`; `a_line_cargo_does_not_write_fails_naming_it` runs
   it 3 times, `:250-264`). `task` parses `Taskfile.yml` and runs one `sh` line: 3 programs
   against 2 for `sh taskfiles/lock-check.sh` (strace). Outside the pool: 171 to 346 ms against
   19 to 27 ms. Alone in the pool the module took 75.6 s summed, about 4.2 s per call (Summary
   15.5 s, load 156 to 173); in the third run of the step 27.9 s. The cases are about the
   script; `conform_totals`, which runs its script with `sh` in a copy, took 0.17 to 0.45 s per
   test in that run. Statement 7 of `story:tls-tests-as-tables` (draft) plans this change; this
   story takes it.
4. Empty test binaries. nextest lists 47 binaries of the step and 21 hold tests
   (`cargo nextest list --workspace` without the TLS packages, load 160 to 162). The 26 others:
   the libraries of the 8 generated packages (`agent`, `document`, `navigator`, `net`,
   `renderer`, `shell`, `surf-system`, `surf-types`), 12 hand-written libraries without a unit
   test (`surf`, `surf-agent`, `surf-agent-tasks`, `surf-answer`, `surf-html-conformance`,
   `surf-page`, `surf-page-text`, `surf-ports`, `surf-secret`, `surf-session`, `surf-url`,
   `surf-view`) and 6 binaries (`surf-agent`, `surf`, `entities-table`,
   `surf-html-conformance`, `surf-session`, `surfd`). nextest runs each binary twice to list it.
   Listing all 47 one after the other outside the pool took 1.84 s and 1.46 s, the 26 empty
   ones 0.87 s and 0.62 s (load 164). In the pool the listing was not measured apart; the fourth
   run leaves about 8 s of the step outside `Summary` once the doc step is taken off. The
   repository already sets `test = false` where a target holds no test
   (`crates/test-process/Cargo.toml:11`, `crates/view-fixture/Cargo.toml:14` and `:19`,
   `crates/agent-tasks/Cargo.toml:16`), and the folder's rule asks for few test binaries.

Expected, as summed test time of the third run of the step: statement 1 about 29 s (the script
is about 4 of the 5.5 s of one case; about 72 programs become at most 25), statement 3 about 22 s
(18 calls of about 1.5 s in that run become calls of about 0.3 s). As wall time: about 6 s and
4.5 s, plus about 3 to 4 s of listing (statement 4, an estimate) and up to the doc step's time
(statement 2, kept only when measured). The changed tests are bound by acceptance, not by these
estimates.

## Change

All on `main`, from `96145a2`. One unit; story `test-harness-fewer-processes` is built beside it
and shares no code file with it.

1. `taskfiles/generated-compare.sh` and `task generated-fresh` (`Taskfile.yml:239-258`).
   - The listings of the fresh tree are made once per run of task: `generated-fresh` writes
     `fresh.list`, `fresh.all` and `fresh.executable` beside the tree, by the script
     (`sh taskfiles/generated-compare.sh --lists <fresh tree>`), the same lines that `:39`,
     `:50` and `:51` write today. The compare reads them; when they are missing it makes them
     itself, so `task generated-compare FRESH=<tree>` keeps working on any tree.
   - The loop of `cmp` (`:64-66`) becomes one `diff -rq --no-dereference` of the fresh tree and
     `generated/rust`, whose "Files ... differ" lines one `awk` keeps for the paths both
     listings hold and prints as today: `generated/rust/<path>: differs from what ess
     generates`.
   - The four listings of the committed side (`:53-56`) become one `find generated/rust ...
     -printf` and one `awk`, which write `committed.list`, `committed.all` and
     `committed.executable` and the lines `generated/rust/<path>: not a regular file`.
   - Every line the script prints, its exit status and step 2 stay as they are. The head of the
     script says the three listings and who writes them. `find -printf` and `diff
     --no-dereference` are GNU; this host has both, and the unit checks the forge's runner by
     its first green forge run of the changed script.
   - `crates/surf/tests/common/mod.rs` `fresh()` (`:42-60`) also fails, saying so, when the
     three listings are missing beside the fresh tree.
   - Target: at most 25 programs per run with step 1 failing (72 today).
2. The doc step beside nextest (`Taskfile.yml:328-330`), kept only when measured. `cargo test
   --doc -p surf-ports` starts in the background before `cargo nextest run`, and the step waits
   for both; each writes its own log as today, and the step fails when either fails.
   - It is kept only when all hold: three alternating pairs of `slot --timeout 15m task test`
     with and without it, the load before and after each, give a median gain of at least 3 s;
     and inside `slot task test` the second cargo takes no second slot and the two do not wait
     on each other for good (`slot status` read during the step shows one job for it, and the
     step ends in all six runs).
   - Otherwise the unit removes it again and this statement says so, with the six times.
   - The step's printed lines stay: the ten slowest, nextest's `Summary`, `895 tests: 895
     passed, 0 failed, 0 ignored (cargo-nextest 893, doc tests 2)`, and `test step: N s of its
     budget of 120 s`.
3. `crates/surf/tests/lock_check.rs`: `check()` (`:57-67`) runs `sh taskfiles/lock-check.sh
   Cargo.lock taskfiles/lock-allow.txt` in the copy, and `copy_with()` (`:32-48`) copies no
   `Taskfile.yml`. `the_unchanged_lock_passes` (`:113-128`) also runs `task --dry lock-check`
   in the repository and asserts that the command it prints is that line, so the task stays
   tied to the script; no test is added. Every assertion of the
   16 tests stays. This is statement 7 of `story:tls-tests-as-tables`; that story drops its
   statement 7 and its acceptance 8 when this one lands (a change of that story's file, outside
   this scope).
4. No empty test binary in the step.
   - `test = false` for the library of the 12 hand-written packages named in Why 4 (`[lib]`;
     `doctest` stays as it is, so `surf-ports` keeps its two doc tests) and for the 6 binaries
     (`[[bin]]`; `surf-html-conformance`'s two binaries get a `[[bin]]` entry each, as cargo now
     finds them by path). The manifests: `crates/{agent,agent-tasks,answer,html-conformance,
     page,page-text,ports,secret,session,surf,surf-cli,surfd,url,view}/Cargo.toml`.
   - The 8 generated packages are excluded in the step's command, because `ess` writes their
     manifests: `Taskfile.yml` gains `GENERATED_PACKAGES` beside `TLS_PACKAGES` (`:77`), and
     `task test` adds `--exclude` for each name (`:328`).
   - `taskfiles/test-layout.sh` gains one rule: it fails, naming the file, when a target whose
     manifest entry sets `test = false`, or a package under `generated/rust/crates`, holds
     `#[test]` or `#[cfg(test)]` in a file under its `src/` (read as the script reads a line
     today, without literals and comments). `task test-layout-test` (`taskfiles/test-layout.yml`)
     copies `src/bin/*.rs` and the generated packages' manifests and `src/*.rs` too, and gains
     five cases: three fail (`#[test]` in `src/lib.rs` of a library with `test = false`,
     `#[cfg(test)]` in the file of a binary with `test = false`, `#[test]` in a generated
     package) and two pass (`#[test]` in `crates/html/src` without `test = false`, and the word
     in a comment and a string of a library with `test = false`): 37 cases.
5. `AGENTS.md`, read at `96145a2`: `:131-134` (no test copies `Taskfile.yml` now; the includes
   stay optional), `:165-169` (`lock-check` among the tasks whose tests run their script with
   `sh`), `:247-266` (Layout: a target without tests has `test = false`, and the guard),
   `:1191-1224` (`test-layout`, `test-layout-test`), `:1302-1319` (`lock-check`) and
   `:1323-1359` (`generated-check`, `build`, `test`: the listings, the generated packages left
   out, the doc step if kept). Story `test-harness-fewer-processes` changes only `:1546-1557`.

Machine rules for every command this story names: builds and tests go through the pool (`cargo`
by its shim, any other command that compiles or tests with the `slot` prefix); a script that
measures, loops or waits starts without the prefix; a pool job over 10 minutes names its time
(`slot --timeout <t>`, at most 30 minutes); no Python, also not in a probe; no network and no
model call in a test.

### Merge with other branches

`impl/round2-commands` and `impl/round2-view` add dependency lines to `crates/page/Cargo.toml`,
`crates/session/Cargo.toml` and `crates/surf/Cargo.toml` (`git diff --stat 96145a2 <branch>`);
statement 4 adds a `[lib]` or `[[bin]]` entry in other lines of them; a merge keeps both. Neither
branch changes `Taskfile.yml`, `taskfiles/`, `crates/surf/tests/common/mod.rs` or
`crates/surf/tests/lock_check.rs`. Their hunks in `AGENTS.md` start at `:283` and do not reach
the ranges of statement 5.

## What it does not do

- No cache of `ess synthesize` in `task generated-fresh` keyed by a digest of `spec/`: it runs
  before the timed part of the step (5.52 s outside the pool, load 175).
- No change of the number of tests at once, of `.config/nextest.toml`, of the budget, of a timeout,
  or of the pool.
- No change of what a test asserts but the added `task --dry` assertion, and no test removed,
  merged or added.
- No file of story `test-harness-fewer-processes`: `crates/html-conformance/src` and `tests`,
  `crates/agent-tasks/tests`, `taskfiles/agent-tasks-model.sh`, the test helpers of `surfd` and
  `surf-session`, `crates/view*/tests`.
- No TLS package and no step of `task test-tls`.
- No edit of `.engineering/`; the change of `story:tls-tests-as-tables` is made by the planning
  session.

## Acceptance

1. Programs per run, counted with `strace -f -e trace=execve` outside the pool, before and after,
   quoted with the counts per program: `taskfiles/generated-compare.sh` in a copy with step 1
   failing (72 before) at most 25 after, and with both steps passing (77 before) quoted;
   `lock_check`'s vehicle, 3 before (`task`, `sh`, `awk`) and 2 after.
2. Each changed module alone, `cargo nextest run -p surf -E 'test(/^<module>::/)'` for
   `lock_check`, `review1_generated_check`, `review2_generated_check`, `generated_check_links` and
   `generated_check_modes`: its summed time before (at `96145a2`) and after, in three alternating
   pairs, with `/proc/loadavg` before and after each run. Reported; `lock_check` and
   `generated_check_links` each at most half their before time in each pair.
3. One plant per changed script or test, removed after (`git diff --stat` the same before and
   after), the failure text quoted:
   - `generated-compare.sh` with the byte comparison skipped: `review1_one_changed_byte_in_any_
     generated_file_fails_naming_it` fails naming the files it missed;
   - `lock-check.sh` that accepts a package from outside: `a_registry_package_fails_naming_it`
     fails;
   - `Taskfile.yml` whose `lock-check` runs another line: `the_unchanged_lock_passes` fails;
   - `test-layout.sh` without the new rule: the three new failing cases of `task
     test-layout-test` fail it; `#[test]` planted in `crates/surf/src/lib.rs`: `task test-layout`
     fails naming the file;
   - statement 2, if kept: a failing doc test planted in `crates/ports/src/lib.rs` fails the step,
     and so does a failing nextest test planted in one module, each run once.
4. `cargo nextest list --workspace` without the TLS packages, before and after: the same 893
   test names (the sorted lists compared, not only the count), and 47 binaries before against 21
   after. `cargo test --doc -p surf-ports` runs 2 doc tests before and after.
5. No test over 60 s. Every changed test is timed alone after the change; one over 5 s is named
   with its time. No timeout is raised and no retry is added (`git diff` shows none).
6. Three runs of `slot --timeout 15m task test` on the story's commit, each with
   `/proc/loadavg` before and after and its line `test step: N s`. Reported, not bounded here; the
   bound is acceptance 1 of `story:test-step-holds-with-tls`.
7. `task test-layout` passes, and `task test-layout-test` prints `test-layout-test: 37 case(s),
   each decided as expected by taskfiles/test-layout.sh`.
8. `git diff --stat 96145a2` lists only the files of Scope.
9. The gate `slot --timeout 25m task check-runner` on the story's commit, run by the coordinating
   session, passes; its line `test step: N s of its budget of 120 s` is quoted.

## Scope

`Taskfile.yml`; `taskfiles/generated-compare.sh`, `taskfiles/test-layout.sh`,
`taskfiles/test-layout.yml`; `crates/surf/tests/common/mod.rs`, `crates/surf/tests/lock_check.rs`;
the manifests `crates/agent/Cargo.toml`, `crates/agent-tasks/Cargo.toml`,
`crates/answer/Cargo.toml`, `crates/html-conformance/Cargo.toml`, `crates/page/Cargo.toml`,
`crates/page-text/Cargo.toml`, `crates/ports/Cargo.toml`, `crates/secret/Cargo.toml`,
`crates/session/Cargo.toml`, `crates/surf/Cargo.toml`, `crates/surf-cli/Cargo.toml`,
`crates/surfd/Cargo.toml`, `crates/url/Cargo.toml`, `crates/view/Cargo.toml`; `AGENTS.md` at
`:131-134`, `:165-169`, `:247-266`, `:1191-1224`, `:1302-1319` and `:1323-1359` (line numbers of
`96145a2`).
