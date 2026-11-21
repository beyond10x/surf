# Review 2: the adversary pass after wave 5

The second of the two adversary passes the operator decided on
(approval-record:target-a-accepted-2026-11-20). It closes the [wave plan](waves.md).

## Subject

Two things nobody has attacked:

- `77dd4c5...2bcfe46`: wave 5. The crate `conformance/` (the adapter between the ESS runner and
  the Rust system, the planted-defect test), the task `conform-rust`, and its place in `check`.
- `3aaa3aa...77dd4c5`: the corrections of [review 1](review-1.md) (`625eeff`). Review 1 had no
  second attack: the two fetch timeouts changed meaning, buffer growth got ceilings, `escape`
  covers all of non-ASCII, the binary exits 3 when standard output fails, `generated-check`
  refuses symbolic links.

## Reviewer

| | adversary |
|---|---|
| `subagent_type` | `aep:adversary` (model opus) |
| branch | `impl/review-2-adversary` |
| tree | `~/.local/state/worktree/trees/b10x/surf/surf-r2-adv` (id `surf-r2-adv`) |
| scratch | `~/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/waves/review-2/adversary/` |
| stage | returned red: 8 findings, 5 red cases |

Integration: branch `wave/review-2`, tree `~/.local/state/worktree/trees/b10x/surf/surf-r2-int`
(id `surf-r2-int`, lease `coord-b84c1d1b-86a1-4880-9b86-d11fa8d6a813`).

The reviewer writes failing tests named `review2_*` and edits no implementation. Its brief is
`waves/review-2/brief.md` in the coordinating session's scratch.

## Budget and routing

One pass. As in review 1: a finding `introduced` goes to an implementor (model opus) with the
failing case, a `pre-existing` one becomes a draft in the backlog, the coordinator decides each
`undecided` one and records why. Corrections are committed on `impl/review-2-fixes`, merged
here, gated once with `slot task check`, and merged into `main`. No further attack follows; the
epic is then moved to `implemented`.

## Commits this review makes

This page, one commit for the reviewer's tests that are kept, one correction commit per
implementor round, the merges, the closing store commit, and the merge into `main`.

## Result

The adversary returned red on 2026-11-21. Its report is stored as written:
`review-result:review-2-adversary`, 8 findings (3 `introduced`, 5 `pre-existing`), 3 warnings
and 5 notes.

Its answer to the question of part A: a green `task conform-rust` did not mean the Rust system
conforms. Of 17 system defects it planted, 6 passed all 50 scenarios.

| id | severity | origin | what | what became of it |
|---|---|---|---|---|
| F1 | warning | pre-existing (the suite ESS synthesizes) | every instance given the same identity passes 50 of 50 | corrected here: authored scenario `two-of-each-entity` |
| F2 | warning | pre-existing (suite) | every view cut to its first row passes 50 of 50 | corrected here: the same scenario asserts exactly two rows per view |
| F3 | note | introduced (`conformance/src/target.rs`) | the adapter took a command's events from the typed outcome, so no-event checks could not fail | corrected: events come from the system's log; a disagreement is an error |
| F4 | note | pre-existing (generated pump, suite, `spec/scenarios`) | the binding invocation record is trusted | half corrected: the three authored scenarios assert the request's address. A binding that delivers twice still passes: `expect_invocation` has no count |
| F5 | warning | pre-existing (gate since wave 2) | no gate step read the root `Cargo.lock` for outside packages | corrected: task `lock-check` |
| F6 | note | introduced (wave 5 gate) | nothing compared the Rust total with the interpreter's | corrected: task `conform-totals`, by suite digest and total |
| F7 | note | introduced (`README.md`) | exit 3 claimed for a closed standard output | corrected in `README.md`: undetectable, the runtime reopens descriptor 1 on `/dev/null` |
| F8 | note | pre-existing (check since wave 2) | `generated-check` ignored file modes | corrected: executable bits are compared |

Decisions the coordinator took (brief: `waves/review-2/fixes/brief.md` in the coordinating
session's scratch):

- F1, F2 and F4 are `pre-existing` and would go to the backlog by the routing above. They were
  corrected in this round instead, as far as this repository can: the suite is ESS's, the
  authored scenarios under `spec/scenarios/` are this repository's, and the adversary's cases
  for them were red. The gaps in what ESS synthesizes remain and are candidates for ESS issues
  (below).
- F7: documented, not detected. The adversary's two cases for it were rewritten to assert the
  documented behaviour; that is the one test change of this review.

Correction unit: one `aep:implementor` on Opus in tree `surf-r2-fix`, branch
`impl/review-2-fixes`. Commits `84307d8` (the adversary's 24 cases) and `5dd9346` (the
corrections and 29 new cases), merged here as `a3ffda4`.

- The coordinator compared the adversary's four files with the archive of its worktree: three
  identical, `review2_output.rs` differing in the two F7 cases.
- The coordinator's own run in the unit tree at `5dd9346`: `slot task conform-rust` exit 0,
  `51 scenarios: 51 passed, 0 failed, 0 error, 0 unsupported`; the root suite exit 0, 43 suites,
  138 passed.
- Gate, once, on merge `a3ffda4`: `slot task check` exit 0. Eight steps, none skipped: `surf v1
  — 8 file(s), 3 scenario(s), valid`; `51 scenario(s) (3 authored), 10 refusal(s)`; interpreter
  `51 scenarios: 51 passed, 0 failed, 0 error, 0 unsupported`; Rust system the same line; `the
  interpreter and the Rust system ran 51 scenario(s) each, of suite sha256:1778f074…`;
  `generated/rust matches spec/ (27 artifact(s) and .ess-output)`; `Cargo.lock: 10 package(s),
  none from outside this repository`; cargo build `Finished`; `43 test suites: 138 passed, 0
  failed, 0 ignored`. The conformance crate's own tests: 24. The planted defect fails 22 of 51.
- The corrections of review 1 in `crates/fetch` held under the second attack: each limit at its
  exact value and one past it, the buffer ceilings, the IPv6 forms in brackets.

What stays open, and where it is written:

- A binding that delivers an event twice to an `at_most_once` binding passes the suite. No
  construct of ESS 0.96.0 asserts a count of invocations.
- `lock-check` does not see a path dependency on a directory outside the repository (no
  `source` line). Its summary and `AGENTS.md` say so.
- `cargo fmt --check` fails on reviewer files kept as written; `cargo clippy` with warnings
  denied stops at 19 lints in generated `surf-types`. Neither is in the gate.
- Candidates for ESS issues from this review, not filed (filing waits for the operator's word,
  with the nine of `docs/plan/agent-cli-draft-1.md`): synthesis makes no scenario with two
  instances of one entity; view checks are `contains` over one row; `expect_invocation` has no
  count and the invoked input is not compared with the recorded one; the generated code has no
  dispatch keyed by name, so an adapter spells every name by hand.

Cost: adversary 289,267 tokens (over two runs, cut once by a rate limit); implementor 242,764
tokens (the same).

With this review the wave plan is delivered: five waves and two adversary passes.
`epic:headless-rust-target` is moved to `implemented` on the gate record of `a3ffda4`.
