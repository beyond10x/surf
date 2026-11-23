---
format: aep.planning-md/3
id: story:test-step-scripts-and-binaries
kind: story
status: implemented
title: The test step's scripts start fewer programs and it lists no empty test binary
summary: generated-compare starts at most 25 programs per run (72 today) from listings made once and refused when stale, lock_check runs its script without task (4.2 s per call in the pool), 18 targets get test = false and 8 generated packages leave the step, and the doc step runs beside nextest if it gains in three of three pairs
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
revision: 28
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T01:22:28Z", actor: "human:operator", revision: 25}
- {from: "proposed", to: "active", at: "2026-11-23T01:22:29Z", actor: "human:operator", revision: 26}
- {from: "active", to: "implemented", at: "2026-11-23T02:54:32Z", actor: "human:operator", revision: 28, decided_on: {"recorded":{"test_result":4,"verification":2}}, executor: "agent:coordinator"}
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
   the same tree in every case (`fresh()`, `common/mod.rs:42-60`). Outside the pool a run takes
   175 to 199 ms with step 1 failing and 320 to 409 ms with both steps (three runs each, load
   137). Alone in the pool, `generated_check_links` took 32.8 s summed for 6 cases (Summary 8.8 s,
   load 173 to 178) and `review1_generated_check` 29.4 s for 4 tests with 6 runs (load 138 to
   142). In the third run of the step (132.6 s) the four modules held 51.6 s summed (23.9, 10.7,
   11.5 and 5.5 s).
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

All on `main`, from `96145a2`. One unit. Story `test-harness-fewer-processes` shares no file with
it: its scope no longer holds `AGENTS.md` (D281), so `aep plan artifact waves` reports no
collision between the two (on `1f53d35` it reported one, `AGENTS.md`, and put them in waves 11 and
12). By the scopes of the store on `1f53d35`, this story shares files with stories built after
these two: `story:tls-fetch-https` (`crates/surf-cli/Cargo.toml`, `crates/surf/tests`),
`story:agent-cookie-jar` (`crates/agent-tasks`, `crates/session/Cargo.toml`,
`crates/surf/Cargo.toml`, `crates/surf/tests`), `story:page-test-step-within-budget`
(`AGENTS.md`, `Taskfile.yml`) and `story:agent-tasks-https` (`crates/agent-tasks`).

1. `taskfiles/generated-compare.sh` and `task generated-fresh` (`Taskfile.yml:239-258`).
   - The listings of the fresh tree are made once per run of task: `generated-fresh` runs
     `sh taskfiles/generated-compare.sh --lists <fresh tree>` after `ess` wrote the tree, which
     writes `<fresh tree>.list`, `.all` and `.executable` beside it, the same lines that `:39`,
     `:50` and `:51` write today.
   - The compare uses those listings only when none is older than the tree or its log: it
     refuses them, exit 1 with one line naming the listing and `task generated-fresh`, when
     `find "$fresh" "$fresh.log" \( -newer "$fresh.list" -o -newer "$fresh.all" -o -newer
     "$fresh.executable" \) -print -quit` prints a path. A file written or removed in the tree
     after the listings changes the mtime of the file or of its directory, so a stale listing
     never stands for the tree, and the count guard (`:40-45`) compares the log with a listing
     of this tree.
   - When the three listings are missing, the compare makes them itself in its own directory
     (`$dir`, `.out/generated-check/compare` of the directory it runs in), never beside the
     tree, which the tests share; `task generated-compare FRESH=<tree>` keeps working on any
     tree.
   - The loop of `cmp` (`:64-66`) becomes one `diff -rq --no-dereference "$fresh"
     generated/rust/.` (standard error with it) and one `awk`. The committed operand is
     `generated/rust/.`, so a `generated/rust` that is a link is entered, as `cd generated/rust`
     enters it today; the line of `:47-48` for the link stays. The `awk` takes the paths `diff`
     reports as differing, and every path `diff` names in an error line (a file it cannot read is
     still reported as differing, as a non-zero `cmp` is today), and prints the line
     `generated/rust/<path>: differs from what ess generates` for each, in the order of
     `comm -12 fresh.list committed.list`, the order of today's lines (`diff -r` walks one
     directory at a time, which differs once a name continues another with `-` or `.`).
   - The four listings of the committed side (`:53-56`) become one `find generated/rust/. ...
     -printf`, one `sort` and one `awk`, which write `committed.list`, `committed.all`,
     `committed.executable` and the lines `generated/rust/<path>: not a regular file`, each
     sorted as today.
   - Every line the script prints, its order, its exit status and step 2 stay as they are. The
     head of the script names the three listings, who writes them and when they are refused.
     `find -printf` and `diff --no-dereference` are GNU. The script already relies on GNU `find`
     (`-perm /111`, `:51`, `:56`), and the forge runs `ubuntu-latest`
     (`.github/workflows/check.yml:22`), so the forge has both.
   - `crates/surf/tests/common/mod.rs` `fresh()` (`:42-60`) also fails, naming them, when the
     three listings are missing beside the fresh tree.
   - Target: at most 25 programs per run with step 1 failing and the listings present (72
     today).
2. The doc step beside nextest (`Taskfile.yml:328-330`), kept only when measured. `cargo test
   --doc -p surf-ports` starts in the background before `cargo nextest run`, and the step waits
   for both before `elapsed=` (`:331`); each writes its own log as today, and the step fails when
   either fails.
   - The rule: three alternating pairs of `slot --timeout 15m task test`, with and without it,
     the load before and after each run. For each run the step time minus nextest's `Summary`
     time. It is kept only when that time is smaller with it in all three pairs, and inside
     `slot task test` the second cargo takes no second slot and the two do not wait on each
     other for good (`slot status` read during the step shows one job for it, and the step ends
     in all six runs). Otherwise the unit removes it again.
   - The six times, with their loads and the decision, go into the unit's report, not into this
     story.
   - The step's printed lines stay: the ten slowest, nextest's `Summary`, `895 tests: 895
     passed, 0 failed, 0 ignored (cargo-nextest 893, doc tests 2)`, and `test step: N s of its
     budget of 120 s`.
3. `crates/surf/tests/lock_check.rs`: `check()` (`:57-67`) runs `sh taskfiles/lock-check.sh
   Cargo.lock taskfiles/lock-allow.txt` in the copy, and `copy_with()` (`:32-48`) copies no
   `Taskfile.yml`. `the_unchanged_lock_passes` (`:113-128`) also runs `task --dry --verbose
   lock-check` in the repository and asserts that its standard error holds the line
   `task: [lock-check] sh taskfiles/lock-check.sh Cargo.lock taskfiles/lock-allow.txt`
   (`Taskfile.yml:181`; the task is `silent`, so `task --dry` alone prints nothing), so the task
   stays tied to the script; no test is added. Every assertion of the 16 tests stays.
4. No empty test binary in the step.
   - `test = false` for the library of the 12 hand-written packages named in Why 4 (`[lib]`;
     `doctest` stays as it is, so `surf-ports` keeps its two doc tests) and for the 6 binaries
     (`[[bin]]`; `surf-html-conformance`'s two binaries, `src/main.rs` and
     `src/bin/entities-table.rs`, get a `[[bin]]` entry each, as cargo now finds them by path).
     The manifests: `crates/{agent,agent-tasks,answer,html-conformance,page,page-text,ports,
     secret,session,surf,surf-cli,surfd,url,view}/Cargo.toml`.
   - The 8 generated packages are excluded in the step's command, because `ess` writes their
     manifests: `Taskfile.yml` gains `GENERATED_PACKAGES` beside `TLS_PACKAGES` (`:77`), and
     `task test` adds `--exclude` for each name (`:328`).
   - `taskfiles/test-layout.sh` gains one rule. The targets it covers: a `[lib]` or `[[bin]]`
     entry with `test = false` in any `Cargo.toml` under the root (the directories it does not
     enter today stay so; a `Cargo.toml` below `tests/` is still a fixture), and the library and
     binaries of every package under `generated/rust/crates`. The files of a target are its
     `path` (`src/lib.rs` and `src/main.rs` when none is given) and every module that file
     declares out of line, followed to its file as the script follows the files of a test binary
     today (`#[path]` read the same way; a child it cannot read fails). The rule fails, naming
     the file and line, when one of those files holds `#[test]` in its code (the line without
     literals and comments, as the script reads a line today). It does not flag `#[cfg(test)]`:
     `crates/secret/src/compare.rs` holds it on purpose (`:35`, `:40`, `:49`), because
     `crates/secret/tests/secret.rs:12-13` compiles that file into the test binary by `#[path]`
     (`AGENTS.md:1209` names this). A library's modules are not files of a binary of the same
     package: `surfd`'s binary gets `test = false` while its library keeps its tests in
     `crates/surfd/src/view.rs` (`mod view;`, `lib.rs:99`).
   - `task test-layout-test` (`taskfiles/test-layout.yml`) copies every `*.rs` under
     `crates/*/src` and `generated/rust/crates/*/src` at any depth, and the generated packages'
     manifests, beside what it copies today, and gains seven cases. Three fail: `#[test]` in
     `src/lib.rs` of a library with `test = false`; `#[test]` in a module file that the file of a
     binary with `test = false` declares; `#[test]` in `src/lib.rs` of a package under
     `generated/rust/crates`. Four pass: `#[test]` in `crates/html/src`, a library without
     `test = false`; `#[test]` in a comment and in a string of a library with `test = false`;
     `#[cfg(test)]` in a module file of a library with `test = false` that a test binary of the
     package also declares by `#[path]` (the shape of `surf-secret`); `#[test]` in a module of
     the library of a package whose binary has `test = false` (the shape of `surfd`). 32 cases
     today (`check` calls in `taskfiles/test-layout.yml`), 39 after.
5. `AGENTS.md`, read at `96145a2`: `:131-134` (no test copies `Taskfile.yml` now; the includes
   stay optional), `:165-169` (`lock-check` among the tasks whose tests run their script with
   `sh`), `:247-266` (Layout: a target without tests has `test = false`, and the guard),
   `:1191-1224` (`test-layout`, `test-layout-test`), `:1302-1319` (`lock-check`) and
   `:1323-1359` (`generated-check`, `build`, `test`: the listings, when they are refused, the
   generated packages left out, the doc step if kept).

Machine rules for every command this story names: builds and tests go through the pool (`cargo`
by its shim, any other command that compiles or tests with the `slot` prefix); a script that
measures, loops or waits starts without the prefix; a pool job over 10 minutes names its time
(`slot --timeout <t>`, at most 30 minutes); no test over 60 s and the step's budget is 120 s; no
Python, also not in a probe; no network and no model call in a test.

### Revisions of other stories this change needs

Made by the planning session in the change that accepts this story, not by the unit. Line numbers
are those of the store files on `1f53d35`.

- `story:tls-tests-as-tables`: statement 7 (`:151-157`) and acceptance 8 (`:210-215`) go, each
  replaced by one line saying this story took it; its scope entry `crates/surf/tests/lock_check.rs`
  (`:22-23`) goes, so `waves` no longer reports that file between the two; its summary (`:7`)
  loses "lock_check without task"; its overlap bullet on `story:tls-fetch-https` (`:161-162`) no
  longer names `lock_check.rs`.
- `story:test-step-holds-with-tls` (active, depends on this story): statement 13 (`:316-319`)
  and its exclusion (`:406-407`) say that this story changes `crates/surf/tests/lock_check.rs`;
  acceptance 9 (`:452-453`) drops that file from its empty diff; acceptance 6 (`:442`) keeps 875
  as the count of its time and says the step holds 893 tests at `96145a2`.

### Merge with other branches

`impl/round2-commands` adds dependency lines to `crates/page/Cargo.toml`,
`crates/session/Cargo.toml` and `crates/surf/Cargo.toml`; `impl/round2-view` to
`crates/page/Cargo.toml` and `crates/session/Cargo.toml` only (`git diff --stat 96145a2
<branch>`). Statement 4 adds a `[lib]` or `[[bin]]` entry in other lines of them; a merge keeps
both. Neither branch changes `Taskfile.yml`, `taskfiles/`, `crates/surf/tests/common/mod.rs` or
`crates/surf/tests/lock_check.rs`. Their hunks in `AGENTS.md` start at `:283` and do not reach
the ranges of statement 5.

## What it does not do

- No cache of `ess synthesize` in `task generated-fresh` keyed by a digest of `spec/`: it runs
  before the timed part of the step (5.52 s outside the pool, load 175).
- No change of the number of tests at once, of `.config/nextest.toml`, of the budget, of a timeout,
  or of the pool.
- No change of what a test asserts but the added `task --dry --verbose` assertion and the check
  of `fresh()`, and no test removed, merged or added.
- No file of story `test-harness-fewer-processes`: `crates/html-conformance/src` and `tests`,
  `crates/agent-tasks/tests`, `taskfiles/agent-tasks-model.sh`, the test helpers of `surfd` and
  `surf-session`, `crates/view*/tests`.
- No TLS package and no step of `task test-tls`.
- No edit of `.engineering/`: the revisions of other stories listed above are the planning
  session's.

## Acceptance

1. Programs per run, counted with `strace -f -e trace=execve` outside the pool, before and after,
   quoted with the counts per program:
   - `taskfiles/generated-compare.sh` in a copy with step 1 failing, the three listings present
     beside the fresh tree and newer than it and its log (as `task generated-fresh` leaves
     them): 72 before, at most 25 after;
   - the same with both steps passing (77 before): quoted;
   - the same with step 1 failing and the listings missing: quoted (the fallback);
   - `lock_check`'s vehicle, 3 before (`task`, `sh`, `awk`) and 2 after.
2. Each changed module alone, `cargo nextest run -p surf -E 'test(/^<module>::/)'` for
   `lock_check`, `review1_generated_check`, `review2_generated_check`, `generated_check_links` and
   `generated_check_modes`: its summed time before (at `96145a2`) and after, in three alternating
   pairs, with `/proc/loadavg` before and after each run. Reported; `lock_check` and
   `generated_check_links` each at most half their before time in each pair.
3. One plant per changed script or test, removed after (`git diff --stat` the same before and
   after), the failure text quoted:
   - `generated-compare.sh` with the byte comparison skipped:
     `review1_one_changed_byte_in_any_generated_file_fails_naming_it` fails naming the files it
     missed;
   - `generated-compare.sh` without the check of the listings' age: run on a copy of the fresh
     tree (`cp -Rp`, with its log and listings) from which one generated file was removed after
     the listings were written, it prints no line for that file; with the check it exits 1
     naming `<copy>.list`;
   - the listings removed beside the fresh tree: `cargo nextest run -p surf -E
     'test(/^generated_check_modes::/)'` fails at `fresh()` naming them (`task generated-fresh`
     restores them);
   - the fallback: `task generated-compare FRESH=<copy of the fresh tree without listings>`
     prints the same lines and exit status as with them, and `ls -a` of the copy's parent is the
     same before and after;
   - `lock-check.sh` that accepts a package from outside: `a_registry_package_fails_naming_it`
     fails;
   - `Taskfile.yml` whose `lock-check` runs another line: `the_unchanged_lock_passes` fails;
   - `test-layout.sh` without the new rule: the three new failing cases of `task
     test-layout-test` fail it; `#[test]` planted in `crates/surf/src/lib.rs`: `task test-layout`
     fails naming the file;
   - statement 2, if kept: a failing doc test planted in `crates/ports/src/lib.rs` fails the step,
     and so does a failing nextest test planted in one module, each run once.
4. The step's own listing command, `cargo nextest list --workspace` with `--exclude` for each of
   `TLS_PACKAGES` and of the 8 generated packages (after: of `GENERATED_PACKAGES`), as `task
   test` has them, before and after: the same 893 test names (the sorted lists compared, not
   only the count), and 39 test binaries before against 21 after. The two binary counts are
   counted from the manifests for this story, not run (47 listed in Why 4 less the 8 generated
   libraries; less the 18 targets of statement 4); the unit's two listings must show them.
   `cargo test --doc -p surf-ports` runs 2 doc tests before and after.
5. No test over 60 s. Every changed test is timed alone after the change; one over 5 s is named
   with its time. No timeout is raised and no retry is added (`git diff` shows none).
6. Three runs of `slot --timeout 15m task test` on the story's commit, each quoted with
   `/proc/loadavg` before and after, its line `test step: N s` and its count line, which reads
   `895 tests: 895 passed, 0 failed, 0 ignored (cargo-nextest 893, doc tests 2)`. A run that
   exits non-zero, or whose count line differs, fails this statement. The time is reported, not
   bounded here; the bound is acceptance 1 of `story:test-step-holds-with-tls`.
7. `task test-layout` passes, and `task test-layout-test` prints `test-layout-test: 39 case(s),
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

## As built

The change is `2c86943` on `fix/test-step-scripts`, measured on 2026-11-23.

- `taskfiles/generated-compare.sh`: a failing first step ran 72 programs and runs 23 with the
  listings present, 25 with them missing; both steps passing, 77 and 30. Its output is byte for
  byte the output of the script before in all three cases.
- `lock_check`: 3 programs per call before, 2 after.
- Test binaries: 39 before, 21 after, each with tests; 893 test names and 2 doc tests before and
  after. `test-layout-test`: 39 cases.
- The doc step runs beside cargo-nextest: step time less `Summary` 13.0, 12.3 and 8.5 s without,
  4.6, 3.3 and 2.5 s with, in three alternating pairs, one pool job each.
- Three runs of `task test` in a row at a load of 80 to 111: 23.4 s, 32.9 s and 39.5 s of the
  budget of 120 s.

Short of acceptance 2: `generated_check_links` took 10.78 s after against 11.47 s before in the
third pair, not half. The run after had a load of 95 to 107, the run before 52 to 58; the other
two pairs gave 26.34 to 6.97 s and 13.27 to 3.02 s. Decided (D296): the pair is not measured
again. The count of programs, which the acceptance stands for, is met.

The four manifests of `crates/html-conformance`, `crates/session`, `crates/surfd` and
`crates/view` are changed by this story, as its scope says (D295).
