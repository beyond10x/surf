# Wave 1 of 5: the specification names how a fetch fails

One wave of the [wave plan](waves.md). N is one.

## Unit

| | |
|---|---|
| story | `story:net-fetch-failure-reasons` |
| objective | epic `headless-rust-target`: the transport of the first target can report an unsupported scheme and a malformed response, and the specification says so before any code does |
| scope | cited: `spec/domains/net.yaml`, `AGENTS.md`, `Taskfile.yml`; inferred: `README.md` |
| acceptance | `slot task check` exits 0, and the compiled specification shows `surf.net.FailureReason` with exactly `Unreachable`, `UnsupportedScheme` and `MalformedResponse`, each with a summary |
| what its move to `implemented` needs | the story lifecycle declares no required evidence; the wave records the gate run as `test_result` and the claim check as `verification` |

## The check

1. Implementable in this tree: yes. It edits one specification file and two project files and
   needs `ess` 0.96.0, which is installed and pinned.
2. Surfaces that overlap: none inside the wave (one unit). The verb's lines for this story are
   in [waves.md](waves.md); every collision it names is with a story of a later wave.
3. Blast radius: one domain file. Nothing in `spec/` branches on a variant of
   `surf.net.FailureReason` (Scope of the story, safety fact).

Check path: computed by `aep plan artifact waves`, not judged by reading pairs.

## Pre-flight numbers (2026-11-20)

- free disk on `/`: 95 G (`df -h /`); build pool: 14 of 20 slots free (`slot status`)
- one measured build of the generated workspace in scratch: 68 M build directory, a rebuild in
  under one second; this wave compiles nothing
- model budget: not asked; N is 1

## Commits this wave makes

The opening store commit (this page), one unit commit, the merge of the unit into the wave
branch, the closing store commit, and the merge into `main`. Nothing else.

## Record

Kept current while the wave runs.

| | |
|---|---|
| wave branch | `wave/1-fetch-failure-reasons` |
| unit branch | `impl/net-fetch-failure-reasons` |
| integration tree | `~/.local/state/worktree/trees/b10x/surf/surf-w1-int` (managed id `surf-w1-int`, lease `coord-b84c1d1b-86a1-4880-9b86-d11fa8d6a813`) |
| unit tree | `~/.local/state/worktree/trees/b10x/surf/surf-w1-net` (managed id `surf-w1-net`), base `8ed80f5` |
| build directory | `target/` inside each tree; this wave compiles nothing |
| scratch | `~/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/waves/wave-1/net-fetch-failure-reasons/` |
| stage | delivered: unit `cb18a34`, merge `ee4c775`, gate exit 0 |

## Result

- Unit: green on the first return. Claim VERIFIED: `ess specify inspect --path spec
  surf.net.FailureReason` shows 1 variant on the wave branch at `6e11dcd` and 3 variants with 3
  summaries on the unit branch at `cb18a34`.
- Gate, once, on merge `ee4c775`: `slot task check` exit 0. Its three lines:
  `surf v1 — 8 file(s), 2 scenario(s), valid`;
  `50 scenario(s) (2 authored), 10 refusal(s), written to .out/suite.json`;
  `50 scenarios: 50 passed, 0 failed, 0 error, 0 unsupported`. Steps run: validate, suite,
  conform. None skipped.
- Executed cases: 50 before, 50 after. The change adds enum variants no scenario reads, so the
  count is not expected to move; the claim check above is what distinguishes before from after.
- No adversary pass (operator decision; the adversary runs after wave 4).
- Cost: one `aep:implementor` on Opus, 39,042 tokens, 23 tool uses, 64 s.
- Findings filed to the backlog: none.
