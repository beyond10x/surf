# Wave plan: first implementation target

Derived on 2026-11-20 from the six accepted stories of `epic:headless-rust-target`, with
`aep plan artifact waves --kind story --status active` (aep 0.80.0, implementing skill 0.26.0).
The waves run in this order. A wave takes no story from another wave or from the backlog.

| wave | stories | what it leaves on `main` | page | state |
|---|---|---|---|---|
| 1 | `story:net-fetch-failure-reasons` | the specification names the three ways a fetch ends without a response | [wave-1.md](wave-1.md) | delivered 2026-11-20, merge `ee4c775` |
| 2 | `story:generated-rust-workspace` | `generated/rust`, the root Cargo workspace, and a gate that refuses drift and builds | [wave-2.md](wave-2.md) | delivered 2026-11-20, merge `209f5ff` |
| 3 | `story:http-fetch-library`, `story:memory-ports` | `crates/fetch` and `crates/ports` | [wave-3.md](wave-3.md) | delivered 2026-11-20, gated at `aa52c47` |
| 4 | `story:headless-cli` | `surf <url>` runs the page load | [wave-4.md](wave-4.md) | delivered 2026-11-20, merge `8867e2a` |
| 5 | `story:rust-conformance` | the suite runs against the Rust system in the gate | [wave-5.md](wave-5.md) | delivered 2026-11-21, merge `f7442d9` |

## The verb's answer, verbatim

```
wave 1
  story:net-fetch-failure-reasons (inferred)
wave 2
  story:generated-rust-workspace (inferred)
wave 3
  story:http-fetch-library
  story:memory-ports
wave 4
  story:headless-cli
wave 5
  story:rust-conformance (inferred)
collision: story:generated-rust-workspace story:headless-cli AGENTS.md
collision: story:generated-rust-workspace story:headless-cli README.md (inferred)
collision: story:generated-rust-workspace story:net-fetch-failure-reasons AGENTS.md
collision: story:generated-rust-workspace story:net-fetch-failure-reasons README.md (inferred)
collision: story:generated-rust-workspace story:net-fetch-failure-reasons Taskfile.yml
collision: story:generated-rust-workspace story:rust-conformance AGENTS.md
collision: story:generated-rust-workspace story:rust-conformance Taskfile.yml
collision: story:headless-cli story:net-fetch-failure-reasons AGENTS.md
collision: story:headless-cli story:net-fetch-failure-reasons README.md (inferred)
collision: story:headless-cli story:rust-conformance AGENTS.md
collision: story:headless-cli story:rust-conformance crates/surf/src/lib.rs (inferred)
collision: story:net-fetch-failure-reasons story:rust-conformance AGENTS.md
collision: story:net-fetch-failure-reasons story:rust-conformance Taskfile.yml
5 wave(s), 13 collision(s), 0 unassessed
```

Every collision is between stories of different waves. The one pair that runs at once, wave 3,
shares no typed scope entry. Both of its stories add an entry to the root `Cargo.lock`; that file
is the coordinator's in a wave (story bodies, and the Scope note of
`story:http-fetch-library`), so it is not a typed entry.

## What the operator decided (approval-record:target-a-accepted-2026-11-20)

- The epic and the six stories are accepted, and the waves run without a stop between them.
- Implementors run on the Opus model (`subagent_type: aep:implementor`, `model: opus`).
- The adversary (`subagent_type: aep:adversary`) does not attack each unit. It attacks twice in
  this plan: after wave 4, over everything merged in waves 1 to 4, and after wave 5. A unit
  still merges only on its green package-scoped gate and a verified claim, and each wave closes
  on the whole gate.
- The commits: one opening commit on `main` before wave 1; per wave, an opening store commit, one
  commit per unit, the merges, a closing store commit and the merge into `main`. No push, no tag,
  no release.

## Conventions

- Branches: `wave/<n>-<name>` for a wave, `impl/<story-slug>` for a unit, forked from the wave
  branch.
- Trees: every checkout is a managed worktree (`worktree create`). The build directory is
  `target/` inside the tree. Scratch is `~/.cache/scratch/<coordinating session>/waves/<wave>/
  <unit>/`, assigned in the unit's brief.
- The brief of a unit is a file in its scratch directory. It starts from
  [unit-invariants.md](unit-invariants.md).
- `Cargo.lock` is the coordinator's: regenerated on the wave branch after the units merge.

## Backlog

Stories filed from findings while this plan runs are listed here by id. They stay `draft` and
enter no wave of this plan.

- `story:invalid-address-ending` (draft, filed in wave 3 from the implementor report of `story:http-fetch-library`): an `http:` address that cannot be parsed ends as `Unreachable`; the specification has no reason for it.
- `story:random-source-failure-cause` (draft, filed in review 1 from finding S6 of the security reviewer): a failing `/dev/urandom` ends the run with exit 3, and the operating system's error is dropped at `crates/ports/src/random.rs:71`.

## Delivered

All five waves and both adversary passes are done. `main` after review 2 passes `slot task check`
with 51 scenarios on the interpreter and on the Rust system and 138 tests in 43 suites
([review-2.md](review-2.md)). `epic:headless-rust-target` is `implemented`. The two backlog
drafts above stay drafts. What comes next is planned from `docs/design/agent-interface.md`.
