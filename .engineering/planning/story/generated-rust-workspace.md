---
format: aep.planning-md/3
id: story:generated-rust-workspace
kind: story
status: implemented
title: Commit the generated Rust workspace and hold it in the gate
summary: generated/rust holds the synthesized workspace; task check refuses drift and builds and tests it.
relations:
- decomposes: epic:headless-rust-target
- depends_on: story:net-fetch-failure-reasons
scope:
- confidence: cited
  path: .gitignore
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: inferred
  path: README.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/.gitkeep
- confidence: cited
  path: generated/rust
revision: 15
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T21:27:44Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-20T21:27:44Z", actor: "human:timo", revision: 12, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-11-20T21:42:55Z", actor: "human:timo", revision: 15, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}}
---
## Context

`ess generate synthesize --path spec --target rust --out <dir>` writes a Cargo workspace of seven
crates (`surf-types`, `shell`, `navigator`, `net`, `document`, `renderer`, `surf-system`) with
path dependencies only and no third-party crate (measured 2026-11-20, ess 0.96.0, recorded in
`AGENTS.md`, "Implementation targets"; 4,284 lines before story `net-fetch-failure-reasons`
changes the specification). `synthesize` has no `--check`. Nothing of it is in the repository
and nothing compiles it.

## Change

- `generated/rust/` holds the generated workspace, byte for byte as `ess` writes it, committed.
  No file in it is edited by hand.
- A Cargo workspace at the repository root for hand-written crates. Its members are the glob
  `crates/*`, never a list of names, so a later story adds a crate by creating its directory and
  does not edit the root `Cargo.toml`. It is set up so that a hand-written crate can depend on a generated crate by path and one
  `cargo build` and one `cargo test` from the root cover the hand-written crates and compile the
  generated ones. How the two workspaces are joined (the root workspace listing the generated
  crates as members, or excluding `generated/rust` and reaching it by path) is the implementor's
  choice; the generated files, including `generated/rust/Cargo.toml`, stay unedited either way.
  Until the first hand-written crate exists, a placeholder is not added: the build step builds
  the generated workspace.
- `Taskfile.yml`: a task `generate` that rewrites `generated/rust` from `spec/`, a task
  `generated-check` that generates into a directory under `.out/` and fails with the differing
  paths when it is not identical to `generated/rust`, and tasks `build` and `test` that run
  `cargo build` and `cargo test` with warnings denied. `check` runs `validate`, `conform`,
  `generated-check`, `build` and `test`, each step's failure failing the task (no pipe hides an
  exit status).
- `.gitignore` ignores every Cargo `target/` directory. `Cargo.lock` of the root workspace is
  committed. It gains an entry for every crate a later story adds, so two stories that each add a
  crate both change it: in a wave it is the coordinator's file, regenerated on the integration
  branch after the units merge, and no unit's change to it is merged.
- `AGENTS.md`: "Layout" and "Commands" name the new directories and tasks, and the sentence "No
  implementation exists yet" is replaced by what exists.

## Acceptance

`slot task check` exits 0 on the branch, and exits non-zero naming the file after one byte of any
file under `generated/rust` is changed.

## Out of scope

Hand-written behaviour of any kind: ports, the network, a binary.

## Scope

Derived 2026-11-20 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `generated/rust/` (new; 27 generated files: root `Cargo.toml`, `PLAN.md`, `plan.json`, and 7 crates under `generated/rust/crates/`) — cited, story Change and the sample output's own log (`27 artifact(s)`)
- **Files:** `Taskfile.yml:44` (the `check` task, today `validate` and `conform` only; gains `generate`, `generated-check`, `build`, `test`) — cited
- **Files:** `Cargo.toml` (new, repository root, members `crates/*`) and `Cargo.lock` (new, repository root) — cited, story Change
- **Files:** `.gitignore:2` (`/target/` is anchored to the root and does not cover `generated/rust/target/`) — cited
- **Symbols:** tasks `generate`, `generated-check`, `build`, `test`, `check`; command `ess generate synthesize --path spec --target rust --out <dir>` — cited, story Change and `ess generate synthesize --help`
- **Also likely:** `generated/rust/.ess-output/state.json` and a `generated/rust/Cargo.lock` need an ignore or exclusion rule, in `.gitignore` or in the `generated-check` comparison — inferred, neither is among the 27 files `ess` reports
- **Also likely:** `AGENTS.md:91` ("None is built yet") and `AGENTS.md:114-115` ("not compiled") go stale with the same change — inferred
- **Also likely:** `README.md:10` ("There is no implementation yet") and `README.md:23-32` (the `task check` transcript) — inferred, the story does not name README
- **Documents:** `AGENTS.md:59-60` (sentence "No implementation exists yet"), `AGENTS.md:63` ("Layout"), `AGENTS.md:126` ("Commands") — cited
- **Confidence:** high — the story names every path it lands on; the only inferred parts are README and the generator's state file
- **Would collide with:** any unit touching `Taskfile.yml` (above all the `check` task), `AGENTS.md` ("Rules", "Layout", "Commands"), the root `Cargo.toml` or `Cargo.lock`, or `.gitignore`; and any unit that changes `spec/`, because it must then rewrite `generated/rust/` — cited, story Change and `docs/plan/unit-invariants.md`
- **Safety fact:** the drift check is sound only over the 27 files `ess` reports, not the whole directory: `.ess-output/state.json` carries an `anchor_id` that differs in each of the five sample outputs, and `Cargo.lock` and `target/` in the sample were written by cargo 11 minutes after generation; the 27 files hold no absolute path, timestamp or UUID — cited, step 2 (`gen-rust/.ess-output/state.json`, `gen-rust.log`, file times), unproven

Coordinator's note, 2026-11-20: the scoper found that `ess` also writes
`generated/rust/.ess-output/state.json`, whose `anchor_id` differs between two generations. The
story's "byte for byte" and its drift check therefore cover every file `ess` reports as an
artifact; whether `.ess-output/` is committed or ignored is the implementor's to settle from what
`ess` documents, and the drift check names whatever it leaves out and why.

Confirmed by the implementor, wave 2, merge `209f5ff` (2026-11-20):

- Landed on: `generated/rust/` (27 artifacts and `.ess-output/state.json`), root `Cargo.toml`, root `Cargo.lock`, `crates/.gitkeep`, `Taskfile.yml`, `.gitignore`, `AGENTS.md`, `README.md`.
- `crates/.gitkeep` was not in the scope: cargo 1.99.1 refuses a member glob that matches nothing, and a tracked file under `crates/` makes the glob valid.
- Inferred line on `state.json` and `generated/rust/Cargo.lock`: partly right. `state.json` is committed; the drift check leaves it out of the fresh comparison and covers it by regenerating a copy in place. `generated/rust/Cargo.lock` is git-ignored; with the workspaces joined at the root cargo does not write it.
- Inferred lines on `AGENTS.md:91`, `:114-115` and `README.md:10`, `:23-32`: confirmed and rewritten. `README.md:27` (prerequisites) was also stale and was rewritten by the coordinator.
- Safety fact: confirmed by measurement. Two fresh generations differ only in `state.json` (`anchor_id`, `checksum`).
- The join the story left open: the root workspace lists `generated/rust/crates/*` as members beside `crates/*`. Excluding `generated/rust` was tried and discarded (a workspace with no member does not build).
