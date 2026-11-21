# Review 1: one pass over waves 1 to 4

The operator decided that no unit is attacked on its own and that the work is reviewed once
after a larger piece (approval-record:target-a-accepted-2026-11-20). This is that pass. It runs
before wave 5 of the [wave plan](waves.md).

## Subject

`8ed80f5...3aaa3aa`: everything merged in waves 1 to 4. Hand-written: `spec/domains/net.yaml`
(wave 1), the root `Cargo.toml` and the `Taskfile.yml` tasks (wave 2), `crates/fetch` and
`crates/ports` (wave 3), `crates/surf` (wave 4). `generated/rust` is written by `ess`.

## Reviewers, dispatched at once, each in its own tree

| | adversary | security reviewer |
|---|---|---|
| `subagent_type` | `aep:adversary` (model opus) | `aep:security-reviewer` (model opus) |
| lane | correctness against the stories' acceptance and the specification | memory safety, resource bounds, injection, isolation, the gate |
| branch | `impl/review-1-adversary` | `impl/review-1-security` |
| tree | `~/.local/state/worktree/trees/b10x/surf/surf-r1-adv` (id `surf-r1-adv`) | `~/.local/state/worktree/trees/b10x/surf/surf-r1-sec` (id `surf-r1-sec`) |
| scratch | `~/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/waves/review-1/adversary/` | `~/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/waves/review-1/security/` |
| stage | returned red: 4 findings, 3 red cases | returned red: 6 findings, 5 red cases |

Integration: branch `wave/review-1`, tree `~/.local/state/worktree/trees/b10x/surf/surf-r1-int`
(id `surf-r1-int`, lease `coord-b84c1d1b-86a1-4880-9b86-d11fa8d6a813`).

Both write failing tests named `review1_*` and edit no implementation. Their brief is
`waves/review-1/brief-common.md` in the coordinating session's scratch.

## Budget and routing

One pass each. A finding `introduced` by waves 1 to 4 goes to an implementor (model opus) with
the failing case; a `pre-existing` one becomes a draft in the backlog; the coordinator decides
each `undecided` one and records why. Corrections are committed on `impl/review-1-fixes`, merged
here, gated once with `slot task check`, and merged into `main`. The coordinator reads the
correction diff itself (no assertion dropped, no test relaxed); there is no second attack in
this review.

## Commits this review makes

This page, one commit for the reviewers' tests that are kept, one correction commit per
implementor round, the merges, the closing store commit, and the merge into `main`.

## Result

Both reviewers returned red on 2026-11-20. Their reports are stored as written:
`review-result:review-1-adversary` (4 findings, F1 to F4) and `review-result:review-1-security`
(6 findings, S1 to S6). All 10 are `introduced`; 3 are warnings and 7 are notes.

| id | severity | where | what | what became of it |
|---|---|---|---|---|
| F1 | warning | `Taskfile.yml`, `generated-check` | a symbolic link under `generated/rust` passed the check; cargo ran a linked `build.rs` | corrected: every entry that is neither a file nor a directory fails and is named |
| F2, S3 | note | `crates/fetch/src/lib.rs:616` | `max - body.len() + 1` overflows for a body limit of `usize::MAX` | corrected: saturating arithmetic |
| F3 | note | `crates/surf/src/main.rs:46` | exit 0 when standard output could not be written | corrected: exit 3 |
| F4 | note | `crates/surf/tests/library.rs:111` | the unit's transport-once test stays green with the `Pending` check disabled | no code change; the adversary's two cases are kept and go red on that mutant |
| S1 | warning | `crates/fetch/src/lib.rs:165` | only each read had a timeout: 4.0 s measured against a 200 ms timeout | corrected: one fetch lasts at most `connect_timeout + read_timeout`, 40 s at the defaults |
| S2 | note | `crates/fetch/src/lib.rs:400` | the body buffer grew to about twice the body limit | corrected: growth stops at a ceiling |
| S4 | note | `crates/fetch/src/lib.rs:234` | `[127.0.0.1]` and `[localhost]` were connected to | corrected: text in brackets must parse as an IPv6 address |
| S5 | warning | `crates/surf/src/lib.rs:218` | `escape` let U+2028, U+2029 and bidirectional controls through | corrected: every character outside printable ASCII is written as bytes |
| S6 | note | `crates/ports/src/random.rs:71` | the random source's error is dropped | backlog: `story:random-source-failure-cause` (draft) |

Decisions the coordinator took, each recorded in the brief of the correction unit
(`waves/review-1/fixes/brief.md` in the coordinating session's scratch):

- F3: exit status 3, the status of a run that stopped. `README.md` says the rows may be partly
  written.
- S1: no new field in `Limits`. `connect_timeout` bounds name resolution plus connecting over all
  addresses; `read_timeout` bounds the exchange from the request to the last byte of the
  response. Resolution runs on a thread that is abandoned when the time runs out. That path has
  no test: nothing on the machine stalls a resolver.
- S5: all of non-ASCII is escaped, not a list of ranges. The standard library has no table of
  general categories, a hand list would miss characters, and `surf-fetch` refuses a non-ASCII
  address.
- S6 goes to the backlog although its origin is `introduced`. The generated `UnmetObligation`
  holds static text only and the generated port keeps the ports value private, so the cause can
  reach the binary only through state shared between the ports of different components or
  through a change in what ESS generates. Both are design choices.

Correction unit: one `aep:implementor` on Opus in tree `surf-r1-fix`, branch
`impl/review-1-fixes`. Commits `f066c22` (the reviewers' 35 cases, kept as written) and `625eeff`
(the corrections and 19 new cases), merged here as `ac83e14`.

- The coordinator compared the reviewers' seven files with the archives of their worktrees: one
  line differs, `review1_library.rs:98`, whose expected value decision S5 reverses.
- The coordinator read the correction diff: no assertion dropped, no test relaxed.
- The coordinator's own run in the unit tree at `625eeff`: `cargo test --workspace --locked
  --no-fail-fast` exit 0, 37 suites, 114 passed, 0 failed, 0 ignored. On the base, the
  implementor measured 95 executed and 8 red.
- Gate, once, on merge `ac83e14`: `slot task check` exit 0. `surf v1 — 8 file(s), 2 scenario(s),
  valid`; `50 scenario(s) (2 authored), 10 refusal(s)`; `50 scenarios: 50 passed, 0 failed, 0
  error, 0 unsupported`; `generated/rust matches spec/ (27 artifact(s) and .ess-output)`; cargo
  build `Finished`; `37 test suites: 114 passed, 0 failed, 0 ignored`. Executed tests: 60 before
  the review, 114 after.

Named by the implementor and not changed:

- A host without brackets still goes to the resolver with characters RFC 3986 does not allow in
  a host name. No connection follows unless the resolver answers. It belongs to the backlog
  draft `story:invalid-address-ending`.
- The number of abandoned resolver threads is not limited. Each ends when the system resolver
  returns.
- `review1_bracketed_host_that_is_not_ipv6_is_refused_without_connecting` now takes about 20 s:
  its server waits its full 10 s for a connection that no longer comes, once per host.
- `cargo fmt --check` fails on the six reviewer files (kept as written) and on `generated/`.
  `cargo clippy` with warnings denied stops at 19 lints in the generated `surf-types`. Neither is
  in the gate.

Cost: adversary 177,912 tokens, 55 tool uses, 523 s; security reviewer 153,478 tokens, 46 tool
uses, 645 s; implementor 211,142 tokens, 95 tool uses, 719 s.

There is no second attack in this review. The second adversary pass runs after wave 5.
