# Wave 2 of 5: the generated Rust workspace, held in the gate

One wave of the [wave plan](waves.md). N is one.

## Unit

| | |
|---|---|
| story | `story:generated-rust-workspace` |
| objective | epic `headless-rust-target`: the implementation is generated from the specification, committed, and refused by the gate when it drifts or stops compiling |
| scope | cited: `generated/rust`, `Cargo.toml`, `Cargo.lock`, `Taskfile.yml`, `.gitignore`, `AGENTS.md`; inferred: `README.md` |
| acceptance | `slot task check` exits 0 on the branch, and exits non-zero naming the file after one byte of any file under `generated/rust` is changed |
| what its move to `implemented` needs | no required evidence; the wave records the gate run as `test_result` and the claim check as `verification` |

## The check

1. Implementable in this tree: yes. `ess` 0.96.0 and cargo 1.99.1 are installed; the generated
   workspace has path dependencies only, so nothing is fetched.
2. Surfaces that overlap: none inside the wave (one unit). Its dependency,
   `story:net-fetch-failure-reasons`, is `implemented` (wave 1, merge `ee4c775`).
3. Blast radius: one new directory, one new root manifest, and the gate file.

Check path: computed by `aep plan artifact waves`; this story's lines are in
[waves.md](waves.md). `aep plan artifact blocked` on 2026-11-20: "nothing is blocked".

## Pre-flight numbers (2026-11-20)

- free disk on `/`: 86 G (`df -h /`), down from 95 G at wave 1; the build pool evicts below 40 G
- one measured build of the generated workspace (scratch copy, before wave 1 changed the
  specification): 68 M build directory, rebuild under one second
- model budget: not asked; N is 1

## Commits this wave makes

The opening store commit (this page), one unit commit, the merge of the unit into the wave
branch, the closing store commit, and the merge into `main`. Nothing else.

## Record

Kept current while the wave runs.

| | |
|---|---|
| wave branch | `wave/2-generated-workspace` |
| unit branch | `impl/generated-rust-workspace` |
| integration tree | `~/.local/state/worktree/trees/b10x/surf/surf-w2-int` (managed id `surf-w2-int`, lease `coord-b84c1d1b-86a1-4880-9b86-d11fa8d6a813`) |
| unit tree | `~/.local/state/worktree/trees/b10x/surf/surf-w2-gen` (managed id `surf-w2-gen`) |
| build directory | `target/` inside each tree |
| scratch | `~/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/waves/wave-2/generated-rust-workspace/` |
| stage | delivered: unit `7d2bb5d`, merge `209f5ff`, gate exit 0 |

## Result

- Unit: green on the first return. Claim VERIFIED by the coordinator in the unit tree: `slot task
  check` exit 0; after one byte of `generated/rust/crates/surf-types/src/net.rs` was changed, exit
  201 with `generated/rust/crates/surf-types/src/net.rs: differs from what ess generates`; the
  file was restored to the same SHA-256. The implementor ran the same change over all 28 files
  under `generated/rust` in a sandbox copy; all 28 failed naming the file.
- Gate, once, on merge `209f5ff`, in a tree with no build directory: `slot task check` exit 0.
  Five steps, none skipped: `surf v1 — 8 file(s), 2 scenario(s), valid`; `50 scenario(s) (2
  authored), 10 refusal(s)`; `50 scenarios: 50 passed, 0 failed, 0 error, 0 unsupported`;
  `generated/rust matches spec/ (27 artifact(s) and .ess-output)`; cargo build `Finished` in
  0.52s; `14 test suites: 0 passed, 0 failed, 0 ignored`.
- The generated crates hold no test, so the `test` step runs 0 tests. It shows only that test
  targets compile with warnings denied.
- The coordinator changed one line the unit was held off: `README.md:27` now names cargo.
- No adversary pass (operator decision; the adversary runs after wave 4).
- Cost: one `aep:implementor` on Opus, 163,459 tokens, 81 tool uses, 548 s.
- Findings filed to the backlog: none.
