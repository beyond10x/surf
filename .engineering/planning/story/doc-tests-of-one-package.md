---
format: aep.planning-md/3
id: story:doc-tests-of-one-package
kind: story
status: draft
title: The test step runs the doc tests of surf-ports only
summary: cargo test --doc --workspace starts rustdoc for 24 packages (28 on the wave) for 2 doc tests, both in surf-ports; step minus Summary 14.4 to 32.5 s at load 110 to 153; run -p surf-ports, a guard against doc tests elsewhere, the workspace count in the slow lane
relations:
- decomposes: epic:headless-rust-target
- informed_by: story:test-step-holds-with-tls
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: taskfiles/doc-tests-check.sh
revision: 3
---
## Why

The rule (`AGENTS.md` of the folder, "Tests stay fast"): the test step ends within its budget of
120 s; an expensive step runs once, not per package that has nothing to run.

`task test` runs `cargo test --doc --workspace --locked` after `cargo nextest run` (`Taskfile.yml`
:324 on `main` at `7d15e06`, which holds the first TLS wave). The workspace has 2 doc tests,
both in `crates/ports/src/lib.rs` (package `surf-ports`): the block that opens at line 40, and
the `compile_fail,E0277` block that opens at line 61. The workspace has 30 packages (`cargo
metadata --no-deps`); the command starts rustdoc for each that has a library.

Measured on 2026-11-22: the step time minus nextest's `Summary` time, which is the doc step,
nextest's start and listing, and the step's own lines.

| Tree | Load | `test step` | step minus `Summary` |
|---|---|---|---|
| `main` `0f0c157`, three runs | 118 to 139 | 76.2 s, 77.5 s, 82.2 s | 15.2 s, 14.4 s, 19.2 s |
| `wave/tls-1` `48f05c2`, three runs | 110 to 153 | 130.1 s, 141.8 s, 147.4 s | 21.2 s, 32.5 s, 25.5 s |
| `wave/tls-1` `48f05c2`, one run | 148 to 120 | 83.9 s (`Summary` 67.9 s) | 16.0 s |
| `main` `7d15e06` (tree of `cb22b79`), three runs | 25 to 159 | 95.8 s, 98.1 s, 80.8 s | 32.5 s, 26.5 s, 20.1 s |

The last row is `task test` without the TLS packages, which run in `task test-tls` since
`story:tls-step-within-budget`: 888 tests, each run green. Two of the three are over the 90 s
that `story:test-step-holds-with-tls` asks for at such a load, and the time outside nextest is
the largest single cost left in them (the slowest test took 10.8 s to 11.0 s, and
`story:session-request-timeout-option` brings it to about 2 s).

`story:test-step-holds-with-tls` measured earlier the same day that `cargo test --doc
--workspace` alone took 8.2 s and 10.4 s, and `cargo test --doc -p surf-ports` 5.3 s and 13.0 s,
one run of each at a load of 107 to 119, and left the doc step as it was. D259 decides to run the
doc tests of `surf-ports` only; this story measures the gain.

Doc comments in `crates/` write their code blocks as fenced blocks in `///` or `//!` lines. On
`main` at `7d15e06`, the TLS packages included, the fences that open a block are: `crates/agent/src/wire.rs`:14 and :44,
`crates/surf/src/lib.rs`:396, `crates/surfd/src/link.rs`:18 and :56,
`crates/view-fixture/src/main.rs`:12, each marked `text`, which rustdoc does not run; and
`crates/ports/src/lib.rs`:40 (no mark) and :61 (`compile_fail,E0277`), the two doc tests. No
source uses `#[doc = …]`.

The tree: this story is built on a branch from `main` at `7d15e06` or later and lands on `main`.
It writes no file under `crates/`, so it runs beside the round 2 units.

## Change

1. `Taskfile.yml`, task `test`: the doc step runs `cargo test --doc -p surf-ports --locked --color
   never` into `.out/test-doc.log`. The count line stays: `N tests: … (cargo-nextest n, doc tests
   2)`. The task's summary says it.
2. `taskfiles/doc-tests-check.sh`, POSIX sh, and a task `doc-tests-check` in the list of `check`
   before `build`. The script reads every `*.rs` under `crates/` and `generated/`, follows the
   fences of `///` and `//!` lines, and counts an opening fence as a doc test when its mark is
   empty or each comma-separated word of it is one of `rust`, `compile_fail`, `no_run`,
   `should_panic`, `ignore`, `edition2015`, `edition2018`, `edition2021`, `edition2024`,
   `standalone_crate` or an error code `E` and four digits. A fence marked otherwise (`text`) is
   not a doc test. A doc test outside `crates/ports/` fails the script with one line per test:
   `doc-tests: <file>:<line> holds a doc test; only surf-ports runs doc tests`. Otherwise it prints
   `doc-tests: 2 doc tests, all in crates/ports`. An indented code block or a `#[doc]` attribute is
   outside its reach; the slow lane catches those by the count.
3. A task `doc-tests-slow` runs `cargo test --doc --workspace --locked` into
   `.out/doc-tests-slow.log` and compares the summed count of its `test result:` lines with 2:
   `doc-tests-slow: 2 doc tests in the workspace, as expected`, or `doc-tests-slow: N doc tests in
   the workspace, expected 2` and exit 1. It is a subtask of the lane `slow`, never a step of
   `check`. `Taskfile.yml` on `main` holds the task `slow` (:440); this story adds
   `doc-tests-slow` to its list. `story:conformance-step-within-budget` adds `conform-slow` to
   the same list on `wave/round-2`; the merge keeps both subtasks.
4. `AGENTS.md`: the bullet of `task test` (:1310 on `main`) says that the doc step runs
   `surf-ports` only; a bullet of `task doc-tests-check`; the bullet of `task slow` (:1227) names
   `doc-tests-slow`.

### Overlap

- `Taskfile.yml`: `story:tls-step-within-budget` is on `main`; this story changes the doc command
  of `test` and leaves its nextest command and `test-tls` as they are.
  `story:page-test-step-within-budget` (on `wave/round-2`) makes `test` write
  `.out/test-step.seconds`, and `story:conformance-step-within-budget` extends `slow`; the merge
  keeps each side.
- `AGENTS.md`: the bullets of `task test` and `task slow`; `story:conformance-step-within-budget`
  changes the bullet of `task slow` too, and `impl/round2-view` the page tests' times after the
  bullet of `task test`; the merge keeps each side.
- `story:session-request-timeout-option` changes `crates/session` and one sentence of
  `AGENTS.md` (the waits of `built`), no line of this story.

## Acceptance

Every command runs in a tree of this story's branch from `main`, unless named otherwise.

1. Before: three consecutive runs of `slot --timeout 15m task test` on the branch's base, each
   with `cat /proc/loadavg` before and after; the evidence holds each `test step` line, each
   `Summary` line and their difference.
2. After: the same three runs on the story's commit, each exits 0 and prints `test step: N s of
   its budget of 120 s` and a count line that ends `doc tests 2)`; the evidence holds the same
   three numbers per run with the loads, and names the gain of the difference against
   acceptance 1.
3. `sh taskfiles/doc-tests-check.sh` prints `doc-tests: 2 doc tests, all in crates/ports` and exits
   0. With a doc-comment code block opened by a fence of three backticks and no mark, planted by
   hand in `crates/url/src/lib.rs`, it exits 1 and
   prints `doc-tests: crates/url/src/lib.rs:<line> holds a doc test; only surf-ports runs doc
   tests` with the line of the planted fence. The same block marked `text` passes. Each plant is
   removed after (`git diff --stat` equal before and after).
4. A doc test of `surf-ports` planted to fail (`assert!(false)` in the block at
   `crates/ports/src/lib.rs`:40, by hand, removed after) makes `slot task test` exit non-zero, and
   `.out/test-doc.log` shows `FAILED`.
5. `task doc-tests-slow` prints `doc-tests-slow: 2 doc tests in the workspace, as expected`, and
   `task --dry slow` lists `doc-tests-slow`.
6. `grep -n 'cargo test --doc' Taskfile.yml` shows `-p surf-ports` in `test` and `--workspace`
   only in `doc-tests-slow`.

## What it does not do

- No change of a doc test or of nextest's run.
- No Python: the check is POSIX sh.
