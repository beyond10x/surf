---
format: aep.planning-md/3
id: story:doc-tests-of-one-package
kind: story
status: active
title: The test step runs the doc tests of surf-ports only, and a check holds that no other package has one
summary: The doc command of task test took about 21 s of a step of 121.9 s at a load of 142 to 177; alone it takes 20.0 to 47.2 s for the workspace and 3.9 to 13.4 s for surf-ports, where both doc tests are; run -p surf-ports and fail the gate on a doc test elsewhere
relations:
- decomposes: epic:headless-rust-target
- informed_by: story:test-step-holds-with-tls
- depends_on: story:session-request-timeout-option
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: taskfiles/doc-tests-check.sh
- confidence: cited
  path: taskfiles/doc-tests.yml
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T23:00:33Z", actor: "human:operator", revision: 10}
- {from: "proposed", to: "active", at: "2026-11-22T23:00:33Z", actor: "human:operator", revision: 11}
---
## Why

The rule (`AGENTS.md` of the folder, "Tests stay fast"): the test step ends within its budget of
120 s, and a step over its budget is a defect that is fixed before the next feature.

The gate on `impl/session-request-timeout` at `9f83444` (2026-11-22, 23:29 to 23:53 CET, load
142 when it started, 177 at its end) ended red on time alone: `test step: 121.9 s, over its
budget of 120 s`, 893 of 893 tests passed, nextest's `Summary` 98.3 s. In that step
`.out/test.log` was last written at 23:53:11 and `.out/test-doc.log` at 23:53:32: the doc-test
command took about 21 s.

`task test` runs `cargo test --doc --workspace --locked` after `cargo nextest run`
(`Taskfile.yml`:324 on `main` at `7d15e06`). The workspace has 2 doc tests, both in
`crates/ports/src/lib.rs` (package `surf-ports`): the block that opens at line 40, and the
`compile_fail,E0277` block that opens at line 61. The workspace has 30 packages (`cargo metadata
--no-deps`); the command starts rustdoc for each that has a library.

The doc command alone, measured on the tree of `cb22b79` (equal to `main` `7d15e06`) on
2026-11-22 from 23:53 to 23:56 CET, with `RUSTFLAGS="-D warnings"` as the task sets it, in
alternating runs, each through the pool:

| Pair | `--workspace` | `-p surf-ports` | Load before the pair, after it |
|---|---|---|---|
| 1 | 47.2 s | 3.9 s | 169.6, 175.8 |
| 2 | 20.0 s | 9.8 s | 175.8, 158.1 |
| 3 | 34.1 s | 13.4 s | 158.1, 146.7 |

Each run passed 2 doc tests. The gain per pair is 43.3 s, 10.2 s and 20.7 s; its median is
20.7 s. An earlier measurement of one run each at a load of 107 to 119
(`story:test-step-holds-with-tls`: 8.2 s and 10.4 s against 5.3 s and 13.0 s) found none; the
three pairs above replace it.

Stop rule (D265): a median gain under 10 s over the three pairs would leave the doc step as it
is and archive this story. The median is 20.7 s and each pair shows a gain, so the story is
built.

The whole step before, on `main` `7d15e06` (tree of `cb22b79`), three consecutive runs of
`slot task test`: 95.8 s, 98.1 s and 80.8 s at a load of 25 to 159, of which 32.5 s, 26.5 s and
20.1 s were outside nextest's `Summary`. That difference also holds nextest's build check and
the listing of its test binaries, so it is not the number of the gain.

Doc comments in `crates/` write their code blocks as fenced blocks in `///` or `//!` lines. On
`main` at `7d15e06`, the TLS packages included, the fences that open a block are:
`crates/agent/src/wire.rs`:14 and :44, `crates/surf/src/lib.rs`:396, `crates/surfd/src/link.rs`:18
and :56, `crates/view-fixture/src/main.rs`:12, each marked `text`, which rustdoc does not run; and
`crates/ports/src/lib.rs`:40 (no mark) and :61 (`compile_fail,E0277`), the two doc tests. No
source uses `#[doc = …]`, no fence is written with tildes, and `generated/` holds no fence.

The tree: this story is built on the branch `fix/doc-tests-of-ports` from
`impl/session-request-timeout` at `9f83444`, which holds `main` `7d15e06`, and lands on `main`
together with that branch after one gate. `9f83444` is the base of every "before" number above
that names it. The check of statement 2 reads `crates/` and `generated/`, which
`impl/round2-commands` and `impl/round2-view` write: at their tips of 2026-11-22 neither adds a
fence with a mark other than `text` nor a `#[doc =` attribute (`git grep` on each branch).

The 90 s that `story:test-step-holds-with-tls` asks for at a load of at least 100 stays that
story's: it measures again after this story is on `main`. This story claims the gain of the doc
command and a green gate of its own commit.

## Change

1. `Taskfile.yml`, task `test`: the doc step runs `cargo test --doc -p surf-ports --locked --color
   never` into `.out/test-doc.log`. The count line stays: `N tests: … (cargo-nextest n, doc tests
   2)`. The task's summary says it.
2. `taskfiles/doc-tests.yml`, included by the root `Taskfile.yml` like the other files of
   `taskfiles/` (`flatten: true`, `optional: true`), holds two tasks over
   `taskfiles/doc-tests-check.sh`, POSIX sh:
   - `doc-tests-check`, in the list of `check` before `build`. The script reads every `*.rs`
     under `crates/` and `generated/` but those under `crates/ports/` (`target` and dot
     directories not entered). In the `///` and `//!` lines of a file it follows the fences: a
     line whose text after the comment mark and blanks starts with three backticks or three
     tildes opens a block when none is open and closes the open one otherwise. An opening fence
     whose mark is not `text` fails the script with `doc-tests-check: <file>:<line> opens a code
     block that rustdoc would run; mark it text or move the doc test to crates/ports`. A line
     that holds `#[doc =` or `#![doc =` fails it with `doc-tests-check: <file>:<line> sets a doc
     attribute; the gate runs the doc tests of surf-ports only`. Every failing line is printed
     before it exits 1. Otherwise it prints `doc-tests-check: no doc test outside crates/ports`.
     It calls no cargo.
   - `doc-tests-check-test`, in the list of `check` beside `test-layout-test` and
     `sh-echo-test`, in the form of those two: it writes its cases into a directory under
     `.out/`, runs the script on each and compares the exit status and the line printed. Cases:
     a fence without a mark fails; `rust,no_run` fails; a fence of tildes without a mark fails;
     `text` passes; a `text` block followed by a second block without a mark fails at the second
     block's line (the closing fence of the first is not taken as an opening); `#[doc = "…"]`
     fails; any of them under `crates/ports/` passes. It prints `doc-tests-check-test: N
     case(s), each decided as expected by taskfiles/doc-tests-check.sh`.
3. A code block written by indentation in a doc comment is outside the script's reach: rustdoc
   would run it and the gate does not. `AGENTS.md` says so and asks for fences.
4. `AGENTS.md`: the bullet of `task test` (:1310 on `main`) says that the doc step runs
   `surf-ports` only; the paragraph of `task check` (:1127-1145) names the two new steps; the
   list of `taskfiles/` (:131-150) names `taskfiles/doc-tests.yml` with its two tasks; a bullet
   of `task doc-tests-check` with the rule for doc comments outside `crates/ports`: a code block
   is fenced and marked `text`.
5. The planning store: `story:test-step-holds-with-tls` says "No change of the doc step
   (measured: no gain)"; that sentence names this story and its measurement instead, and the
   story gains the edge `depends_on story:doc-tests-of-one-package`. The coordinator writes
   both through `aep plan artifact`.

No task of the lane `slow` is added: `slow` makes committed products again and compares bytes,
and a count of doc tests is neither.

### Overlap

- `Taskfile.yml`: this story changes the doc command of `test` (:324), its summary (:294-295),
  the includes (:46-69) and the list of `check` (:413-438), where it adds one step before
  `build` and one beside `sh-echo-test`. `story:page-test-step-within-budget` (on
  `wave/round-2`) makes `test` write `.out/test-step.seconds` (near :342-347) and adds
  `step-budgets` as the last item of `check`; the merge keeps each side.
  `story:tls-step-within-budget` is on `main`; its nextest command and `test-tls` stay as they
  are. `crates/surf/tests/slowest_tests.rs` reads the two `slowest-tests.sh` calls and the two
  `junit=` lines of `Taskfile.yml`, which this story does not touch.
- `AGENTS.md`: the bullet of `task test`, the paragraph of `task check` and the list of
  `taskfiles/`. `story:page-test-step-within-budget` (statement 9) changes the paragraph of
  `task check` too; the merge keeps both sentences. `impl/round2-view` changes the page tests'
  times at :1532 ("The gate's numbers"), and `impl/session-request-timeout` the waits of
  `built` at :1517-1522; neither is a region of this story.
- `story:conformance-step-within-budget` changes the task `slow` and its bullet, which this
  story does not touch.

## Acceptance

Every command runs in a tree of `fix/doc-tests-of-ports` at the story's commit, unless named
otherwise.

1. The gain, measured before the build and held in the evidence: the three pairs of "Why" with
   their loads, median gain 20.7 s, at least the 10 s of the stop rule.
2. The gate: `slot --timeout 25m task check-runner` on the story's commit, with `cat
   /proc/loadavg` before and after, exits 0 and its log holds `test step: N s of its budget of
   120 s`, a count line that ends `doc tests 2)` and `tls test step: N s of its budget of
   120 s`. The evidence names N and the load beside the 121.9 s of the base `9f83444`.
3. `task doc-tests-check` prints `doc-tests-check: no doc test outside crates/ports` and exits
   0. `task doc-tests-check-test` prints `doc-tests-check-test: 13 case(s), each decided as
   expected by taskfiles/doc-tests-check.sh`. As built: 13 cases, the 7 of statement 2 and six
   more (`#![doc =`; a `//!` fence under `generated/`; a failing form under a `target`
   directory, under a dot directory and in a `////` comment, each of which passes; a root
   without `crates/` and `generated/`, which exits 2). With the script planted to accept every mark (by hand, removed after, `git
   diff --stat` equal before and after) `task doc-tests-check-test` exits non-zero and names the
   cases it expected to fail.
4. A doc test of `surf-ports` planted to fail (`assert!(false)` in the block at
   `crates/ports/src/lib.rs`:40, by hand, removed after) makes `slot task test` exit non-zero,
   and `.out/test-doc.log` shows `FAILED`.
5. `task --dry --verbose check` lists `doc-tests-check` before `build`, and
   `doc-tests-check-test`. As built: both tasks are `silent` like the other checks, and `task
   --dry check` alone does not list a silent task.
6. `grep -n 'cargo test --doc' Taskfile.yml` shows `-p surf-ports` and no `--workspace`.
7. `grep -n 'doc-tests' AGENTS.md` shows the bullet of `task doc-tests-check`, the paragraph of
   `task check` and the list of `taskfiles/`; `grep -n 'surf-ports' AGENTS.md` shows the bullet
   of `task test`; `grep -n 'indentation' AGENTS.md` shows the sentence of statement 3 (a code
   block written by indentation is not checked); `grep -n 'marked .text.' AGENTS.md` shows the
   rule of statement 4 for doc comments outside `crates/ports`.
8. Statement 5: `aep plan artifact show story:test-step-holds-with-tls` names this story at the
   sentence of the doc step and lists the edge.
9. The forge: the `check` run of the merge on `main` prints `test step: N s of its budget of
   120 s` with a count line that ends `doc tests 2)`; the evidence names the run id.

## What it does not do

- No change of a doc test or of nextest's run.
- No task in the lane `slow`.
- No Python: the check and its test are POSIX sh.
