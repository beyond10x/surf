# Wave 5 of 5: the suite runs against the Rust system in the gate

One wave of the [wave plan](waves.md). N is one.

## Unit

| | |
|---|---|
| story | `story:rust-conformance` |
| objective | epic `headless-rust-target`: `task check` runs the synthesized conformance suite against the Rust system, and a planted defect is shown failing |
| scope | cited: `conformance`, `Taskfile.yml`, `AGENTS.md`; inferred: `crates/surf/src/lib.rs` |
| acceptance | `slot task conform-rust` exits 0 with every scenario of the suite executed against the Rust system and passed (zero failed, zero error, zero unsupported, the total equal to the interpreter run's in the same `task check`), the crate's planted-defect test passes, and the root `Cargo.lock` lists no package from outside this repository |
| move to `implemented` needs | no required evidence; `test_result` and `verification` are recorded |

## The check

1. Implementable in this tree: yes, with one thing the first four waves did not need. The crate
   takes `ess-conformance` as a git dependency at tag `0.96.0`
   (`git ls-remote --tags https://github.com/beyond10x/ess 0.96.0` answered `b294146` on
   2026-11-20), so cargo fetches from GitHub once, and the unit builds third-party crates for
   the first time in this repository.
2. Surfaces that overlap: none inside the wave (one unit). `conformance/Cargo.lock` is new and is
   the unit's. The root `Cargo.lock` must not change.
3. Blast radius: one new directory outside the root workspace, one task, two project files, and
   the smallest change to `crates/surf/src/lib.rs` if the assembly cannot yet take a foreign
   transport or a foreign `Navigation` storage.

Dependencies, all `implemented`: `story:headless-cli` (wave 4) and through it waves 1 to 3.
Review 1 is merged (`77dd4c5`): the wave starts from the corrected code. `aep plan artifact
waves --status active` on 2026-11-20: `1 wave(s), 0 collision(s), 0 unassessed`.

## Pre-flight numbers (2026-11-20)

- free disk on `/`: 117 G (`df -h /`). The build pool evicts below 40 G. The unit stops and
  reports if free disk falls below 70 G: the size of a build directory with the runner's
  third-party crates is not known. The ESS repository's own build directory reached 56 to 59 G
  in this session; this crate builds 8 of its crates, by the scoper's count.
- the whole gate on `main` at `77dd4c5`: exit 0, 37 test suites, 114 passed (review 1)
- model budget: not asked; N is 1

## After this wave: the second adversary pass

By the operator's decision the adversary attacks once more after this wave. Its page is
review-2.md, written when it runs.

## Commits this wave makes

The opening store commit (this page), one unit commit, the merge of the unit into the wave
branch, the closing store commit, and the merge into `main`.

## Record

Kept current while the wave runs.

| | |
|---|---|
| wave branch | `wave/5-rust-conformance` |
| unit branch | `impl/rust-conformance` |
| integration tree | `~/.local/state/worktree/trees/b10x/surf/surf-w5-int` (managed id `surf-w5-int`, lease `coord-b84c1d1b-86a1-4880-9b86-d11fa8d6a813`) |
| unit tree | `~/.local/state/worktree/trees/b10x/surf/surf-w5-conf` (managed id `surf-w5-conf`) |
| build directory | `target/` inside each tree; `conformance/target/` for the conformance workspace |
| scratch | `~/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/waves/wave-5/rust-conformance/` |
| stage | delivered: unit `4fa980c`, merge `f7442d9`, gate exit 0 |

## Result

- Unit: green on the first return. Claim VERIFIED by the coordinator in the unit tree at
  `4fa980c`: `slot task conform-rust` exit 0, `50 scenarios: 50 passed, 0 failed, 0 error, 0
  unsupported`; the planted-defect test `ok`, its run over the broken storage `50 scenarios: 29
  passed, 21 failed, 0 error, 0 unsupported`; `git diff --quiet -- Cargo.lock` exit 0. The task
  does not exist on the base. The implementor showed the planted-defect assertion failing with
  the defect removed (`no scenario failed over a navigation storage that drops every put`).
- Gate, once, on merge `f7442d9`: `slot task check` exit 0. Six steps, none skipped: `surf v1 —
  8 file(s), 2 scenario(s), valid`; `50 scenario(s) (2 authored), 10 refusal(s)`; interpreter
  `50 scenarios: 50 passed, 0 failed, 0 error, 0 unsupported`; Rust system `50 scenarios: 50
  passed, 0 failed, 0 error, 0 unsupported`; `generated/rust matches spec/ (27 artifact(s) and
  .ess-output)`; cargo build `Finished`; `37 test suites: 114 passed, 0 failed, 0 ignored`. The
  suite is synthesized twice in one gate, because `conform` and `conform-rust` both depend on
  `suite`.
- No change under `crates/`: `Browser::new` over `System::new` already takes a foreign storage.
- `conformance/Cargo.lock`: 57 packages, 46 from outside this repository (8 ESS crates at the
  commit of tag `0.96.0`, 38 from crates.io). The implementor pinned 10 transitive crates back to
  the versions ESS's own lock holds. Direct dependencies: four path crates of this repository and
  `ess-conformance`, `ess-primitives`, `ess-domain`.
- Both reports say `execution_status: passed` and `conformance_status: inconclusive`: the suite
  carries no coverage inventory, and ess 0.96.0 does not qualify a run over unknown coverage. The
  gate reads the counts.
- Named by the implementor about the generated code: no dispatch keyed by name and no name
  accessor on `BindingInvocation`, so the adapter (`conformance/src/target.rs`, 862 lines) spells
  every command, view, event, error and outcome name by hand. A name the specification gains or
  changes has to be added there by hand. `redeliver_event` answers unsupported; no scenario asks
  for it while every binding is `at_most_once`.
- Disk: 117 G free at dispatch, 112 G after the unit's gate; `conformance/target/` 1.2 G.
- Cost: one `aep:implementor` on Opus, 299,138 tokens, 110 tool uses, 787 s.
- Filed to the backlog: none.
- Also committed with this wave's closing commit, because they were written while it ran:
  [agent-cli-draft-1.md](agent-cli-draft-1.md) and the "Commit everything" rule in `AGENTS.md`.
