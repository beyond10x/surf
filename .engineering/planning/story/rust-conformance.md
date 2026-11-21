---
format: aep.planning-md/3
id: story:rust-conformance
kind: story
status: implemented
title: Run the conformance suite against the Rust system in the gate
summary: task check runs the synthesized suite against the Rust system and a planted defect is shown failing.
relations:
- decomposes: epic:headless-rust-target
- depends_on: story:headless-cli
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: conformance
- confidence: inferred
  path: crates/surf/src/lib.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T21:27:44Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-20T21:27:44Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-11-20T23:06:58Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}}
---
## Context

`task check` runs the synthesized conformance suite (50 scenarios on 2026-11-20) against the
interpreter built into `ess`. That holds the specification to itself. It says nothing about the
Rust system: the generated behaviours over `surf-ports`, assembled by the library of story
`headless-cli`, have never answered a scenario.

The route ESS 0.96.0 offers for a Rust system, as the scoper read it at tag `0.96.0` on
2026-11-20 (nothing compiled): the `ess-conformance` crate, taken as a git dependency
(`ess-conformance = { git = "https://github.com/beyond10x/ess", tag = "0.96.0" }`; it is not on
crates.io). The test implements `ess_conformance::target::ConformanceTarget`, admits the suite
with `AdmittedSuite::from_json`, runs it with `Runner::for_suite(..).run_admitted(..)` and writes
an `ess-conformance-report/2` with `CountReport::from_run` (ESS, `website/docs/start/runners/
rust.md` and `crates/verify/ess-conformance/src/counts.rs`). `ess verify conform run --target`
takes no user implementation, and `synthesize --target` writes packages for Go and TypeScript
only. The runner brings eight ESS crates and, by the scoper's count from ESS's lock file, 39
third-party packages. The suite has `expect_invocation` and `expect_no_invocation` steps, so the
target implements `observe_invocations` from the generated `System::invocations()`. The adapter
between the runner's values and the generated typed commands, views and events is written by
hand: the generated crates have no name-keyed dispatch and no serialization.

The no-third-party-crate rule covers what ships. The runner is test tooling. To keep that
checkable, the root workspace's `Cargo.lock` stays free of third-party packages: the conformance
crate is a Cargo workspace of its own, outside the root workspace.

Known cost: with a git dependency, `task check` fetches `github.com/beyond10x/ess` when cargo's
cache does not hold the tag.

## Change

- A test crate at `conformance/` (package `surf-conformance`), its own Cargo workspace with its
  own committed `Cargo.lock`, excluded from the root workspace. It depends on the `surf` library
  by path, and `surf` does not depend on it. It runs the suite `task suite` writes against the
  Rust system, in process, with the network transport replaced by one that never answers,
  because the suite drives the page load with its own `DeliverResponse` and `FailRequest`
  commands. Where the `surf` library's assembly cannot yet take a foreign transport or a foreign
  `Navigation` storage, the smallest change to `crates/surf/src/lib.rs` that allows it is part of
  this story.
- `Taskfile.yml`: a task `conform-rust` that depends on `suite`, runs the crate and writes its
  report under `.out/`; `check` runs it after `conform` and fails when any scenario fails, errors
  or is unsupported. The root `build` and `test` tasks do not reach `conformance/`.
- The planted defect is a test that stays: the same crate runs the same suite against a system
  whose `Navigation` storage drops every `put`, and asserts that the run reports at least one
  failed scenario. A suite that would pass over a broken system fails this test.
- `AGENTS.md`: "The gate's numbers" gains the Rust run's scenario counts beside the
  interpreter's; "Layout" and "Commands" name `conformance/` and `conform-rust`; the no-third-
  party rule names `conformance/` as the one place third-party crates are allowed and says the
  root `Cargo.lock` proves the rest.

## Acceptance

`slot task conform-rust` exits 0 with every scenario of the suite executed against the Rust
system and passed (zero failed, zero error, zero unsupported, the total equal to the interpreter
run's in the same `task check`), the crate's planted-defect test, which runs the suite over a
`Navigation` storage that drops every `put` and asserts at least one failed scenario, passes, and
the root `Cargo.lock` lists no package from outside this repository.

## Out of scope

Changing the specification to make a scenario pass. A scenario the Rust system fails is a finding
that goes back to the story that owns the failing code, or to ESS as an issue when the generated
code is at fault.

## Scope

Derived 2026-11-20 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** a new test crate `crates/conformance` (does not exist yet) — inferred, the body says only "a test crate under `crates/`" and gives no directory name
- **Files:** `Taskfile.yml:37-41` (the `check` command list; new task `conform-rust` beside `conform` at `:25-30`) — cited
- **Files:** `AGENTS.md:135-137` ("The gate's numbers") — cited
- **Symbols:** `ess_conformance::target::ConformanceTarget`, `AdmittedSuite`, `Runner`, `CountReport::from_run` (ess tag 0.96.0, `website/docs/start/runners/rust.md:30-52,126-136`, `crates/verify/ess-conformance/src/counts.rs:213`) — cited
- **Symbols:** generated `System::invocations`, `System::published`, `NavigationStorage::put` (`generated/rust/crates/surf-system/src/lib.rs:232,250` and `generated/rust/crates/surf-types/src/behaviour.rs:48`, lines from the scratch sample, tree not committed yet) — cited
- **Also likely:** `crates/surf/src/lib.rs` (does not exist yet) — inferred, the planted defect needs the assembly to accept a foreign `NavigationStorage`, and the run needs the transport step withheld; story `headless-cli` makes only the transport a parameter
- **Also likely:** root `Cargo.lock` (does not exist yet) — inferred, gains this crate plus 8 ESS crates and 39 third-party packages; story `generated-rust-workspace` makes it the coordinator's file
- **Also likely:** `Taskfile.yml` task `test` (does not exist yet) — inferred, a root `cargo test` runs this crate's tests, which read `.out/suite.json`, so `test` needs `suite` first
- **Also likely:** `AGENTS.md:126-133` ("Commands") — inferred, the new task would be listed there
- **Documents:** `AGENTS.md` only — cited
- **Confidence:** medium — the ESS route, the Taskfile and the AGENTS.md lines are read; the crate's directory name is not given, and `crates/surf` and `generated/rust` do not exist yet — inferred
- **Would collide with:** any unit editing the `check` or `test` task of `Taskfile.yml`, `AGENTS.md` "Commands" or "The gate's numbers", the public assembly API of the `surf` library, or the root `Cargo.lock` — inferred
- **Safety fact:** the runner's third-party crates are reached only from the new test crate, and `surf` does not depend on it, so `cargo tree -p surf --edges normal` stays repository-only — step 1 (stated in the story's Change; `crates/surf/Cargo.toml` does not exist), unproven — inferred

Coordinator's note, 2026-11-20: this section was derived from the body at revision 2. The body
was then rewritten from what the scoper read in ESS: the crate is at `conformance/`, a Cargo
workspace of its own with its own `Cargo.lock`, outside the root workspace, so the lines above
naming `crates/conformance`, the root `Cargo.lock` and the root `test` task no longer apply. The
typed scope entries follow the rewritten body.
