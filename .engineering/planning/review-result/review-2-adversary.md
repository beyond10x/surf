---
format: aep.planning-md/3
id: review-result:review-2-adversary
kind: review-result
status: active
title: Adversary, review 2, wave 5 and the corrections of review 1
relations:
- reviews: story:rust-conformance
- reviews: story:http-fetch-library
- reviews: story:headless-cli
- reviews: story:generated-rust-workspace
revision: 2
---
```
unit: review-2 (adversary) — wave 5 (77dd4c5...2bcfe46) and the review-1 corrections (625eeff), worktree surf-r2-adv at main 2bcfe46 plus 4 uncommitted review2_* files
verdict: red
cases: executed 117→141, red 5
origin: introduced 3, pre-existing 5, undecided 0
wrote-outside-worktree: 1 directory, $HOME/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/waves/review-2/adversary/ (scratch copy copy1 with its target/ dirs deleted, logs, 5 scripts)
needs-coordinator: yes. Three judgement calls: whether F1 and F2 (gaps in the suite ESS synthesized) hold wave 5's claim, whether to add a gate check of the root Cargo.lock (F5), and whether to file F1, F2 and F4 against ESS
```

**Short answer to part A.** A green `task conform-rust` does not mean the Rust system conforms. Three planted system defects pass all 50 scenarios, plus two more that live only in generated code:
- Every assigned identity is the same.
- Every view returns only its first row.
- The navigator never publishes `NavigationCancelled`.
- A refused command publishes an event anyway.
- A binding invokes its command with input other than what it records, or delivers an event twice.

The adapter itself reads views, the event log and invocations from the system. Its one weak point is that it builds a command's direct events from the typed outcome and never from the log (F3). Most of the other gaps are in the suite ESS synthesized. Part B's corrections held, apart from a closed standard output and a file's executable bit.

## 1. `git --no-pager diff --stat`

Empty: no tracked file changed. `git status --short` shows only four new, uncommitted test files:
```
?? conformance/tests/review2_planted_defects.rs
?? crates/fetch/tests/review2_fetch_limits.rs
?? crates/surf/tests/review2_generated_check.rs
?? crates/surf/tests/review2_output.rs
```
Charter check: no implementation, Taskfile, manifest, lock, `spec/` or `generated/` file was touched in the tree. Every mutant lived in the scratch copy or inside my own test file.

One deviation: the eight mutant runs started the built binary `copy1/conformance/target/debug/surf-conformance` directly, not through `slot`, under one second each. The build before it went through the pool.

## 2. Cases added, each run alone first

| file | what it asserts | now |
|---|---|---|
| `conformance/tests/review2_planted_defects.rs` | a control, plus 10 planted system defects (in storage ports and random sources behind the real `SurfTarget`); each asserts at least one scenario fails | 9 green, 2 red |
| `crates/surf/tests/review2_output.rs` | exit 3 when standard output is closed, for a committed and a failed load; exit 1 with standard error closed; exit 3 on a broken pipe | 2 green, 2 red |
| `crates/surf/tests/review2_generated_check.rs` | a hand-written file in `.ess-output/` fails the check; a generated file made executable fails the check | 1 green, 1 red |
| `crates/fetch/tests/review2_fetch_limits.rs` | each limit at its exact value and one past it; a chunked body's buffer stays within the body limit; IPv6 forms in brackets | 7 green |

Red output from the first runs:
```
review2 [every assigned identity is the same]: 50 scenarios: 50 passed, 0 failed, 0 error, 0 unsupported
thread 'review2_every_assigned_identity_the_same_is_noticed' panicked at tests/review2_planted_defects.rs:68:5:
[every assigned identity is the same] the suite passed all 50 scenarios over a system with this defect
review2 [every view answers its first row only]: 50 scenarios: 50 passed, 0 failed, 0 error, 0 unsupported
[every view answers its first row only] the suite passed all 50 scenarios over a system with this defect
review2 [control]: 50 scenarios: 50 passed, 0 failed, 0 error, 0 unsupported
```
```
assertion `left == right` failed: standard output closed, committed load; stderr ""
  left: Some(0)
 right: Some(3)
assertion `left == right` failed: standard output closed, failed load; stderr "surf: http://127.0.0.1:34807/: unreachable: connecting to 127.0.0.1:34807 failed: Connection refused (os error 111)\n"
  left: Some(1)
 right: Some(3)
```
```
assertion `left != right` failed: crates/surf-types/src/lib.rs with mode 100755 passed:
generated/rust matches spec/ (27 artifact(s) and .ess-output)
```

## 3. The suite, run after the cases existed

- **Before:** 114 root workspace tests (AGENTS.md and the review-1 record) plus 3 in `conformance/` (2 unit tests and the planted-defect test, counted from this run's log) = 117.
- **`slot task conform-rust`:** exit 201, `task: Failed to run task "conform-rust": exit status 1`. In the crate's tests the existing `planted_defect.rs` passed (`1 passed`) and mine gave `test result: FAILED. 9 passed; 2 failed`. That is 14 executed, 2 red.
- **`slot task test`:** exit 201. It stops at the first failing test binary, so it ran only 17 suites, `37 passed, 1 failed`.
- **`cargo test --workspace --locked --no-fail-fast`:** exit 101, `40 test suites: 124 passed, 3 failed, 0 ignored`. That is 127 executed, 3 red.
- **Total:** 141 executed, 5 red.

### Defects planted against the suite

| defect | where | scenarios failed | verdict |
|---|---|---|---|
| net storage drops every `put` | test | 13 | noticed |
| document storage drops every `put` | test | 2 | noticed |
| `Navigations` view one row short (last row dropped) | test | 18 | noticed |
| **every view answers only its first row** | test | **0 / 50** | **missed: suite (F2)** |
| **every assigned identity the same (zero random source)** | test | **0 / 50** | **missed: suite (F1)** |
| navigation left `Started` after `committed` | test | 6 | noticed |
| refusal answered as acceptance (ended navigation reads back `Started`) | test | 9 | noticed |
| event field wrong (stored URL rewritten) | test | 8 | noticed |
| store keeps superseded rows (navigation listed `Started` and `Committed`) | test | 9 (by the complete-subject snapshots) | noticed |
| `OpenTab` into a window never opened is accepted | test | 1 | noticed |
| `Browser::run` skipped (no binding delivered) | scratch copy | 8 | noticed |
| binding transform drops `body` | scratch copy, generated code | 1 | noticed |
| `parse-on-navigation-committed` recorded but its command not invoked | scratch copy, generated code | 2 | noticed |
| **navigator port never publishes `NavigationCancelled`** | scratch copy, generated code | **0 / 50** | **missed: adapter (F3)** |
| **a refusal publishes `NavigationCancelled`** | scratch copy, generated code | **0 / 50** | **missed: adapter (F3)** |
| **`Fetch` invoked with a URL other than the recorded one** | scratch copy, generated code | **0 / 50** | **missed: suite and specification (F4)** |
| **`NavigationStarted` delivered twice to an `at_most_once` binding** | scratch copy, generated code | **0 / 50** | **missed: suite (F4)** |

## 4. Findings

| id | severity | origin | `file:line` | what was measured | what reaches it | test |
|---|---|---|---|---|---|---|
| F1 | warning | pre-existing (suite ESS synthesized) | `conformance/tests/review2_planted_defects.rs:303` | all 50 scenarios pass when every window, tab, navigation, document and frame gets the same identity. No scenario makes two instances of one entity and reads both back | a random source or context that repeats identities; the shipped `OsRandom` does not | `review2_every_assigned_identity_the_same_is_noticed` |
| F2 | warning | pre-existing (suite) | `conformance/tests/review2_planted_defects.rs:276` | all 50 pass when every view returns only its first row. Every view check is `contains` over at most one relevant row | any storage `list` or query bug that shows only when a view holds two rows (a second tab or navigation) | `review2_every_view_cut_to_its_first_row_is_noticed` |
| F3 | note | introduced (wave 5 adapter) | `conformance/src/target.rs:117` (also `:133`, `:770`) | the adapter builds a command's direct events from the outcome variant (`vec![event]`, or `Vec::new()` for a refusal), never from `System::published`. The runner's `expect_no_event` and `expect_no_events` read only those direct events (ess-conformance `runner.rs:1785`, `:2365`). Both scratch mutants pass 50/50. So the roughly 13 no-event checks in each scenario cannot fail behind the generated ports | a change in what `ess` generates for a port. `generated-check` blocks hand edits; nothing in the tree reaches it today | scratch mutants `REVIEW2_NO_CANCELLED_EVENT` and `REVIEW2_EVENT_ON_REFUSAL` |
| F4 | note | pre-existing (ESS generated pump and suite; `spec/scenarios`) | `generated/rust/crates/surf-system/src/lib.rs:418` | the invocation record is trusted. `Fetch` invoked with another URL, or delivered twice, passes 50/50. No `expect_invocation` has a count, and neither authored scenario asserts the `Requests` URL after the flow | an ESS generator change | scratch mutants `REVIEW2_FETCH_OTHER_URL` and `REVIEW2_DELIVER_TWICE` |
| F5 | warning | pre-existing (gate since wave 2) | `Taskfile.yml:162` (and `AGENTS.md:38`) | `cfg-if = "1"` added to `crates/surf`, then `cargo generate-lockfile --offline` exit 0. The root lock then lists `source = "registry+https://github.com/rust-lang/crates.io-index"`, and `cargo build --workspace --all-targets --locked` exit 0 (`Compiling cfg-if v1.0.5`). No step of `task check` reads the root lock for a `source` line. The acceptance's "root Cargo.lock lists no outside package" was checked once by hand with `git diff` | a contributor's dependency plus a regenerated lock, in one commit | scratch `third-party.sh` and `third-party.out` |
| F6 | note | introduced (wave 5 gate) | `Taskfile.yml:61` | the acceptance names "Rust total equal to the interpreter's total in the same gate". No command compares the two summary lines; only a person reading them does | a runner or suite change that drops scenarios on one side only | none (read) |
| F7 | note, INFEASIBLE | introduced (README sentence of 625eeff) | `README.md:101` | with standard output closed (`>&-`), a committed load exits 0 with nothing on standard error, and a failed load exits 1. strace shows the Rust runtime reopening the closed descriptor: `poll(... {fd=1, revents=POLLNVAL})`, then `openat(AT_FDCWD, "/dev/null", O_RDWR) = 1`, then the rows written to `/dev/null`. The binary cannot tell this from `>/dev/null` without unsafe code. The fix is a README line | a user running `surf url >&-` | `review2_output.rs:77`, `:91` |
| F8 | note | pre-existing (check since wave 2) | `Taskfile.yml:136` | `generated-check` compares bytes with `cmp -s`, not modes. A generated `.rs` file at mode 100755 passes: `generated/rust matches spec/`. Nothing cargo does reads the mode | a `chmod`, committed | `review2_generated_check.rs:85` |

## 5. What I attacked and found sound

- **Adapter sources:** view rows come from the component queries, eventual events from `System::published`, invocations from `System::invocations`, errors from the typed error fields.
- **Adapter answers built from the request:** the correlation stamped on every event comes from the request, but the runner keeps one correlation per scenario and the adapter makes a fresh `Browser` per scenario, so stamping it is equivalent. Consistency tokens are a counter, which is correct for a synchronous system. The variant-name tables are keyed by typed variants.
- **The task:**
  - `conform-rust` cannot exit 0 on zero scenarios: `main.rs:55` requires `total > 0`, and the planted test requires at least one failure.
  - The suite cannot be stale: `deps: [suite]` with no `sources:`, so it is regenerated on every run.
  - `check` stops at a failing `task:` step, as seen in this run.
  - The root workspace does not reach `conformance/`: it is outside the member globs and has its own `[workspace]`.
- **fetch:**
  - Each exact limit is accepted and one past it refused (header 300/301, Content-Length 1000/1001, close-delimited 1000/1001, chunk line 1024/1025).
  - A chunked body's capacity stays at most the body limit.
  - `[::ffff:127.0.0.1]` and `[::FFFF:7F00:1]` are admitted; `[]`, `[fe80::1%25lo]` and `[::1%lo]` are refused without connecting.
  - By reading: every read and write, the `Interrupted` retries included, rechecks the deadline. Zero and overflowing durations behave as documented.
- **Escaping:** the escaped form is unambiguous. A broken pipe exits 3. A closed standard error keeps exit 1. I found no unescaped path to either stream.
- **`generated-check`:** an extra file in `.ess-output/` fails it. Entries left out of the check (`generated/rust/target/`, a hard link with identical bytes) are not read by any cargo run from the root.

## 6. What I did not get to

- A resolver that stalls (no local means to make one).
- Paths with a newline or a leading dash, FIFOs and sockets in `generated-check`. Read only: the find and comm logic flags them, though a path with a newline is reported under the wrong name.
- A panic in the `surf` binary with server-controlled text: none found by reading, none constructed.

## 7. Paths written outside the worktree

`$HOME/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/waves/review-2/adversary/` holds:
- `copy1/`: one scratch copy. Its `target/` (290 M) and `conformance/target/` (806 M) are deleted; 1.3 M is left.
- Logs: `*.log`, `*.out`, `strace-closed-stdout.txt`.
- Scripts: `run-mutant.sh`, `run-mutants.sh`, `third-party.sh`, `suite-run.sh`.

The worktree's own `conformance/target/` (1.2 G) and `target/` are left in place for the coordinator, who owns cleanup. No process of mine is running. The lease `adv-r2` was released with `session-end`. Free disk on `/` was 94 G at the last check.

```findings
- file: conformance/tests/review2_planted_defects.rs
  line: 303
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: a system that gives every window, tab, navigation, document and frame the same identity passes all 50 synthesized scenarios, because no scenario creates two instances of one entity and reads both back
- file: conformance/tests/review2_planted_defects.rs
  line: 276
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: a system whose every view returns only its first row passes all 50 scenarios, because every view check is contains over at most one relevant row
- file: conformance/src/target.rs
  line: 117
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the adapter reports a command's direct events from the typed outcome variant and never from the system log, so expect_no_event and expect_no_events cannot fail, and a port that drops an event or publishes one on a refusal passes 50/50
- file: generated/rust/crates/surf-system/src/lib.rs
  line: 418
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the suite trusts the binding invocation record, so Fetch invoked with another URL than recorded, or delivered twice to an at_most_once binding, passes 50/50
- file: Taskfile.yml
  line: 162
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: no gate step checks the root Cargo.lock for packages from outside the repository; a crates.io dependency added to crates/surf with a regenerated lock passes cargo build --locked
- file: Taskfile.yml
  line: 61
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the acceptance's equality of the Rust and interpreter scenario totals is checked by no command, only by a person reading two summary lines
- file: README.md
  line: 101
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: with standard output closed the Rust runtime reopens it on /dev/null, so surf exits 0 or 1 with the rows lost, not 3 as the README states; the fix is a README line
- file: Taskfile.yml
  line: 136
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: generated-check compares bytes only, so a generated file whose mode was changed to 100755 passes
```
