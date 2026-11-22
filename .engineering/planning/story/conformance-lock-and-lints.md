---
format: aep.planning-md/3
id: story:conformance-lock-and-lints
kind: story
status: draft
title: The gate checks the conformance runner's lock and lints
summary: lock-check reads conformance/Cargo.lock, the gate runs clippy on conformance/, the AGENTS.md sentence on that lock says what holds, and two texts stop naming ESS 0.96.0 as the release in use.
relations:
- decomposes: epic:headless-rust-target
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: conformance/Cargo.lock
- confidence: cited
  path: conformance/src/target.rs
- confidence: cited
  path: crates/surf/tests
- confidence: cited
  path: taskfiles/lock-check.sh
- confidence: cited
  path: website/docs/status.md
revision: 8
---
## Why

Story `ess-0-97-3` moved every pin of ESS to 0.97.3 and found four things about `conformance/`
and about texts that name a release, none of them in its paths:

1. No step of the gate reads `conformance/Cargo.lock`. `lock-check` reads the workspace's
   `Cargo.lock` only, so a package from outside this repository can enter the conformance
   runner's lock without a line on the allow-list.
2. `AGENTS.md` says the conformance lock pins the third-party packages to the versions ESS's
   own lock holds at the tag. The unit of `ess-0-97-3` counted ten packages that differ.
3. `cargo clippy` in `conformance/` fails at `conformance/src/target.rs:583`. The gate runs
   clippy on the root workspace only.
4. `website/docs/status.md` and the doc comments of the generated-check tests in
   `crates/surf/tests/` name ESS 0.96.0 as the release in use.

## Change

- `lock-check` reads `conformance/Cargo.lock` with the same rule as the workspace's lock:
  every package from outside this repository is on the allow-list by name, version and
  checksum, and a git dependency by its repository and commit.
- The gate runs clippy on `conformance/` with the lints of the root workspace; the finding at
  `conformance/src/target.rs:583` is fixed in the code after its cause is read.
- The sentence of `AGENTS.md` says what the lock is: written by cargo for the three git
  dependencies at their tag, checked by `lock-check`.
- The two texts of point 4 name the release that `spec/ess-inputs.yaml` selects, or name none.
  A line that records what a named release did when it was measured keeps its release.

## Acceptance

When the story is implemented:

1. A package added to `conformance/Cargo.lock` that is not on the allow-list makes
   `lock-check` exit non-zero and name the package (a test case with a planted lock).
2. `cargo clippy` in `conformance/` exits 0 inside the gate, and a planted lint there makes
   the gate's step exit non-zero.
3. `git grep -n '0\.96\.0' -- website/docs/status.md crates/surf/tests` prints only lines that
   record a measurement on that release, each listed in the report.
4. The gate's added steps are timed before and after; the test step stays within 120 s and
   the report names both times.
