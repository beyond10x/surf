# Wave 3 of 5: the fetch library and the in-memory ports

One wave of the [wave plan](waves.md). N is two, run at once.

## Units

| | `story:http-fetch-library` | `story:memory-ports` |
|---|---|---|
| objective (epic `headless-rust-target`) | the `net` transport can fetch an `http:` address with the standard library only, bounded in time and size | the generated components get storage and identities, one store per component |
| scope | cited: `crates/fetch` | cited: `crates/ports` |
| acceptance | `cargo test -p surf-fetch` passes with a test for each of the 14 numbered cases, and `cargo tree -p surf-fetch` shows no dependency | `cargo test -p surf-ports` passes, documentation tests included, with the five numbered tests present |
| move to `implemented` needs | no required evidence; `test_result` and `verification` are recorded | the same |

## The check

1. Implementable in this tree: yes for both. Standard library only; the tests use
   `127.0.0.1`; nothing is fetched.
2. Surfaces that overlap: none between the two typed scopes (`crates/fetch`, `crates/ports`).
   The verb places both in wave 3 and prints no collision between them (see
   [waves.md](waves.md)). Three files both would otherwise touch are the coordinator's in this
   wave and are held off both units in their briefs: the root `Cargo.lock` (regenerated on the
   wave branch after the merges), `crates/.gitkeep` (removed by the coordinator once a crate
   exists) and `AGENTS.md`.
3. Blast radius: one new crate each. `surf-fetch` depends on nothing. `surf-ports` depends on
   generated crates and changes none.

Dependencies: both depend on `story:generated-rust-workspace`, `implemented` in wave 2 (merge
`209f5ff`). `aep plan artifact blocked` on 2026-11-20: "nothing is blocked".

Dry run of the merge before the first unit lands: not needed by the split-file rule (no shared
file is assigned to either unit); the coordinator still runs `git merge-tree` over the two unit
branches before merging the second.

## Pre-flight numbers (2026-11-20)

- free disk on `/`: 67 G (`df -h /`); it was 95 G at wave 1 and 86 G at wave 2. This plan's own
  trees held 68 M of build output each. The build pool evicts below 40 G.
- one measured build of the root workspace from nothing: under 2 s (wave 2 gate)
- model budget: not asked; N is 2, under the default of 4

## Commits this wave makes

The opening store commit (this page), two unit commits, two merges into the wave branch, one
coordinator commit on the wave branch for the files held off both units (`Cargo.lock`,
`crates/.gitkeep`, `AGENTS.md`), the closing store commit, and the merge into `main`.

## Record

Kept current while the wave runs.

| | `http-fetch-library` | `memory-ports` |
|---|---|---|
| unit branch | `impl/http-fetch-library` | `impl/memory-ports` |
| unit tree | `~/.local/state/worktree/trees/b10x/surf/surf-w3-fetch` (id `surf-w3-fetch`) | `~/.local/state/worktree/trees/b10x/surf/surf-w3-ports` (id `surf-w3-ports`) |
| build directory | `target/` inside the tree | `target/` inside the tree |
| scratch | `~/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/waves/wave-3/http-fetch-library/` | `~/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/waves/wave-3/memory-ports/` |
| stage | delivered: unit `2652165`, merge `d357621` | delivered: unit `b7686c3`, merge `537c32b` |

Wave branch `wave/3-fetch-and-ports`; integration tree
`~/.local/state/worktree/trees/b10x/surf/surf-w3-int` (id `surf-w3-int`, lease
`coord-b84c1d1b-86a1-4880-9b86-d11fa8d6a813`).

## Result

- Both units green on the first return. Claims VERIFIED by the coordinator in each unit tree:
  `cargo test -p surf-fetch` exit 0 with 12 + 24 tests and a test for each of cases 01 to 14,
  `cargo tree -p surf-fetch` printing the crate alone; `cargo test -p surf-ports` exit 0 with 10
  integration tests and 2 documentation tests. Neither package exists on the base.
- Merge dry run before the second unit landed: `git merge-tree --write-tree
  --merge-base=f86ef25 wave/3-fetch-and-ports impl/http-fetch-library` exit 0.
- Coordinator commit `aa52c47`: `Cargo.lock` regenerated (nine packages, no `source =` line),
  `crates/.gitkeep` removed, `AGENTS.md` and `README.md` updated.
- Gate, once, on that tree: `slot task check` exit 0. Five steps, none skipped: `surf v1 — 8
  file(s), 2 scenario(s), valid`; `50 scenario(s) (2 authored), 10 refusal(s)`; `50 scenarios: 50
  passed, 0 failed, 0 error, 0 unsupported`; `generated/rust matches spec/ (27 artifact(s) and
  .ess-output)`; cargo build `Finished`; `21 test suites: 48 passed, 0 failed, 0 ignored`.
  Executed tests: 0 before the wave, 48 after.
- Free disk when the units returned: 64 G.
- Observed and not acted on: `cargo clippy` over the workspace fails on `needless_return` in the
  generated `surf-types/src/behaviour.rs` (reported by the `memory-ports` implementor). The gate
  runs no clippy step. `--no-deps` on a hand-written crate passes.
- The `http-fetch-library` implementor fetched the texts of RFC 9110 and RFC 9112 from
  rfc-editor.org to confirm the section numbers it cites. No test reaches outside the machine.
- No adversary pass (operator decision; the adversary runs after wave 4).
- Cost: `http-fetch-library` 121,857 tokens, 43 tool uses, 570 s; `memory-ports` 125,114 tokens,
  52 tool uses, 526 s; both `aep:implementor` on Opus.
- Filed to the backlog: `story:invalid-address-ending` (draft).
