---
format: aep.planning-md/3
id: story:local-gate-equals-forge
kind: story
status: implemented
title: The local gate runs as the forge's runner does, and a red forge run fails the watch script
summary: On 2026-11-21 the forge's check went red twice after a green local gate, both in a test that decides from one read whether another process has ended. One pinned rustc read by both, a task that runs the gate in the runner's form on a clean export, one tested helper with a bounded wait for an ended process, and a watch script in the repository that exits non-zero on a red run.
relations:
- decomposes: epic:headless-rust-target
scope:
- confidence: cited
  path: .github/workflows/check.yml
- confidence: cited
  path: .github/workflows/release.yml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/agent/Cargo.toml
- confidence: cited
  path: crates/agent/tests
- confidence: cited
  path: crates/session/Cargo.toml
- confidence: cited
  path: crates/session/tests
- confidence: cited
  path: crates/surfd/Cargo.toml
- confidence: cited
  path: crates/surfd/tests
- confidence: cited
  path: crates/surfd/tests/log.rs
- confidence: cited
  path: crates/test-process
- confidence: cited
  path: crates/view-fixture/Cargo.toml
- confidence: cited
  path: crates/view-fixture/src/main.rs
- confidence: cited
  path: crates/view-fixture/tests
- confidence: cited
  path: docs/badges/make.sh
- confidence: cited
  path: docs/badges/tests.svg
- confidence: cited
  path: docs/releasing.md
- confidence: cited
  path: rust-toolchain.toml
- confidence: cited
  path: taskfiles/forge-watch.sh
- confidence: cited
  path: taskfiles/planning-only.sh
- confidence: cited
  path: view-tests/tests/adversary-detail.spec.mjs
- confidence: cited
  path: website/docs/how-it-is-built.md
- confidence: cited
  path: website/docs/status.md
- confidence: cited
  path: website/docs/what-works.md
revision: 33
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T23:35:07Z", actor: "human:operator", revision: 21}
- {from: "proposed", to: "active", at: "2026-11-21T23:35:08Z", actor: "human:operator", revision: 22}
- {from: "active", to: "implemented", at: "2026-11-22T03:26:39Z", actor: "human:operator", revision: 33, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

The rules of the folder this repository is worked in say that the local gate equals the forge:
one pinned rustc read by both, what the runner does differently written in the repository and
used by the local gate, a red forge run makes the push or watch script exit non-zero, and when
the forge goes red after a green local gate the fix includes the change that makes the local
gate catch the same failure.

On 2026-11-21 the forge's `check` went red twice after a green local gate, both times in
`crates/view-fixture/tests/fixture.rs`, the test that kills a session and reads its process:

- run 44925207287 at `ff8d8fa`: "the foreign process did not start". The test started the
  system's `tail` under the name `surf-session`; on the runner's image `tail` is one name of a
  program that chooses by its name and exits. Fixed in `f8898db` (a program of the repository).
- run 44947513891 at `61fb7a9`: "the fixture's own session ... is killed". The helper `alive`
  counted the `/proc/<pid>/stat` state `X` as running; a single read after the kill met it.
  Fixed in `36e0dfb`: five copies of `alive` count `Z`, `X` and `x` as ended, and two tests
  give the decision lines without a process (a planted defect goes red on any machine).

Both are one class: a test decides from one read whether another process has ended, or relies
on what a program of the machine does. What the repository has today:

- No `rust-toolchain.toml`. The workflow names `1.99.1` in `.github/workflows/check.yml`; a
  local gate uses whatever rustc the machine has (1.99.1 on the coordinator's machine today).
- The runner's environment (`CI`, `GITHUB_ACTIONS`, `CARGO_TERM_COLOR=always`,
  `CARGO_INCREMENTAL=0`, no `RUSTC_WRAPPER`, `ess` 0.96.0) is set by a script of the
  coordinator outside the repository.
- The three `tests/common/mod.rs` copies of `alive` (`surf-agent`, `surfd`, `surf-session`)
  have no test of their own; single reads after an end remain (`crates/agent/tests/start.rs`,
  `crates/surfd/tests/in_process.rs`, `crates/session/tests/e2e.rs`), and loops on
  `/proc/<pid>` existing.
- The script that publishes `main` and waits for the forge's runs is outside the repository;
  until 2026-11-21 it ended 0 when a run concluded `failure`.

## Change

- `rust-toolchain.toml` pins the rustc; the workflows `check` and `release` read the file and
  name no version of their own.
- A task of the repository runs the gate as the runner does (the environment above, the `ess`
  of the pin, a clean export of the committed tree for it to run in, so that a file that
  exists only in a working tree cannot make it pass), and `AGENTS.md` names it as the gate to
  run before a merge.
- The decision "has this process ended" exists once, in a crate of the workspace that only
  tests depend on, with line tests for every state `proc_pid_stat(5)` lists and a bounded
  wait (`ended_within(pid, bound)`) that the tests use in place of a single read after a kill,
  a close or an end of input. The unit lists every such read and loop in `crates/*/tests/` and
  changes each or says why it stays.
- The unit lists every program of the machine a test starts (`kill`, `task`, `sleep` and
  what else it finds) and, for each, whether the runner's image provides the same program
  under that name; one that differs is replaced by a program of the repository or by the
  standard library.
- A run that provokes the class on the local machine: the test binaries that start processes
  run repeatedly on one processor in a lane named `slow`, outside the gate's test step. The
  unit measures first whether that reproduces the read of `X` with the helper as it was at
  `61fb7a9`, and reports the count; if it does not, the lane is not added and the line tests
  are the catch.
- A script of the repository waits for the forge's runs of a commit, reading through the
  bot's command with the policy path taken from the environment, prints one line per run and
  exits non-zero when a run does not conclude `success`. No path of a machine in it. Its
  decision on a file of runs is checked by a task of the gate, on recorded files, without the
  network.
- A pull request whose changed files are all planning or prose files (`.engineering/`,
  `docs/plan/`, `*.md` outside `website/`) passes the workflow `check` without building or
  testing: the job runs, finds only such paths, and ends green. One path list, in a script
  the workflow calls, with a task of the gate that gives it both sides. A push to `main`
  always runs the whole gate.

## Acceptance

`rustc --version` in the tree and in the forge's log are the same release, from one file. The
runner-form task exits 0 on a clean tree and exits non-zero when a test reads a file that is
not committed (shown once with a planted file, taken back). With `X` taken out of the ended
states in the shared helper a test goes red on the unit's machine; every test that asserted
on one read after a process ended uses the bounded wait. The watch script exits non-zero
for `61fb7a9` (its `check` run failed) and 0 for `e47b1d1`. The gate's test step stays within
its budget, with the unit's measured time. The path list's task passes for a list of planning
files only and fails for a list that holds one file of `crates/` or of `website/`.

## Out of scope

The forge's rulesets and branch protection. The other workflows, but for the rustc release
`release` names. A second runner image.

## Added while the unit ran

Two tests failed while the story was built, each once and each on the loaded machine, and both were test faults.

- `case_11_the_bound_over_all_sessions_drops_an_ended_log_whole` (`crates/surfd/tests/log.rs`) asserted that the log of a closed session is still kept; `surfd` keeps the entries of the `close` answer before or after it lists the session ended, and after it they drop the ended log under the bound of the test. 10 of 100 runs on one processor were red before, 0 of 100 after (e7c2abf).
- The page test "a click on a mark beside its short name selects that event" read a box in two steps while the page drew its timeline again at the opening of its stream. 1 of 40 runs under four busy processes was red before, 0 of 40 after (ff19f1c).
- `taskfiles/check-runner.sh` first emptied `RUSTC_WRAPPER`; it now leaves the compiler cache of the machine in use (a758451).
- The planning-only path list excludes `*.md` under the code directories and two documents the gate reads.

Found and not changed: the page of the live view writes "live" after its first load, before its stream is open (`crates/view/web/view.js`); `story:view-live-means-stream-open` is its draft.
