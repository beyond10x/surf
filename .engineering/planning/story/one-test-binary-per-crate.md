---
format: aep.planning-md/3
id: story:one-test-binary-per-crate
kind: story
status: draft
title: Each crate has one integration-test binary
summary: 76 files under crates/*/tests/ are 76 test executables in 15 crates. Each crate with more than one gets tests/main.rs with the files as modules; no test is removed or changed in what it asserts; the build time, the size of the test executables and the step test are measured before and after.
relations:
- decomposes: epic:headless-rust-target
revision: 1
---
## Context

The rules of the folder this repository is worked in say that test binaries are few: one
integration-test binary per crate (`tests/main.rs` with modules). Each file directly under a
crate's `tests/` is linked as its own executable and, under cargo-nextest, listed by its own
process before a test runs.

Counted at `e47b1d1`: 76 files under `crates/*/tests/*.rs` in 15 crates (`surf` 12, `session`
9, `fetch` 8, `view` 6, `surf-cli` 6, `answer` 6, `surfd` 5, `html-conformance` 5, `agent` 5,
`html` 4, `url` 3, `ports` 3, `view-fixture` 2, `secret` 1, `page` 1), and 3 under
`conformance/tests/`. The gate counts 126 suites with the unit-test binaries and the doc tests.
Wave 10 of the agent interface plan adds two more files (`crates/page/tests/label.rs`,
`crates/html/tests/limits.rs`).

Not measured: the link time and the size of the 76 executables, and what the step gains.

## Change

- Measure first: the time of `cargo build --workspace --all-targets` from a clean `target/`,
  the size of `target/debug/deps`, and the step `test`, before the change.
- Each crate with more than one file under `tests/` gets `tests/main.rs` that declares the
  files as modules, moved into a directory under `tests/` so that cargo finds one target; a
  `common` module is shared by the modules as it is today. No test is removed, renamed or
  changed in what it asserts; a test's name gains its module as a prefix. Helpers that two
  files define under the same name stay private to their modules.
- A test that relies on being alone in its process (a process-wide setting, an environment
  variable, the working directory, a count of threads or of file descriptors, memory
  measured for the process) is listed, and keeps its own binary with the reason written at
  its head, or is changed so that it does not rely on it. Under cargo-nextest each test is its
  own process, so the list is made by reading, and checked by one run of `cargo test` (one
  process per binary) that must also pass.
- Every place that names a test file by its path follows: `AGENTS.md` ("Layout", "The gate's
  numbers"), the Taskfile's `grep` lines for test names, `docs/`, the stories' scopes in the
  store where a later wave still writes the file.
- The same measurement after the change.

## Acceptance

The gate exits 0 with the same number of tests as before (the unit states both numbers and
the number of test executables before and after); each crate has at most one integration
test executable beside those kept apart with a written reason; the step `test` is not slower
than before, with both times; `cargo test --workspace --locked` passes as well as the
nextest run.

## Out of scope

Unit tests inside `src/`. The generated workspace under `generated/rust/`. Changing what a
test checks.
