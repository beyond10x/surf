---
format: aep.planning-md/3
id: story:one-test-binary-per-crate
kind: story
status: draft
title: Each crate has one integration-test binary
summary: 77 files under crates/*/tests/ are 77 test executables in 16 crates. Every crate with a tests/ directory, and conformance/, gets autotests = false and one [[test]] main over tests/main.rs with one mod line per file; no file moves and no test is removed; a POSIX sh gate check keeps the form; the executables and the test step are measured before and after.
relations:
- decomposes: epic:headless-rust-target
- depends_on: story:html-accessible-name
- depends_on: story:html-parser-limits
scope:
- confidence: cited
  path: .github/workflows/release.yml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: conformance/Cargo.toml
- confidence: cited
  path: conformance/tests
- confidence: cited
  path: crates/agent/Cargo.toml
- confidence: cited
  path: crates/agent/tests
- confidence: cited
  path: crates/answer/Cargo.toml
- confidence: cited
  path: crates/answer/tests
- confidence: cited
  path: crates/fetch/Cargo.toml
- confidence: cited
  path: crates/fetch/tests
- confidence: cited
  path: crates/html-conformance/Cargo.toml
- confidence: cited
  path: crates/html-conformance/tests
- confidence: cited
  path: crates/html/Cargo.toml
- confidence: cited
  path: crates/html/tests
- confidence: cited
  path: crates/page/Cargo.toml
- confidence: cited
  path: crates/page/tests
- confidence: cited
  path: crates/ports/Cargo.toml
- confidence: cited
  path: crates/ports/tests
- confidence: cited
  path: crates/secret/Cargo.toml
- confidence: cited
  path: crates/secret/tests
- confidence: cited
  path: crates/session/Cargo.toml
- confidence: cited
  path: crates/session/tests
- confidence: cited
  path: crates/surf-cli/Cargo.toml
- confidence: cited
  path: crates/surf-cli/tests
- confidence: cited
  path: crates/surf/Cargo.toml
- confidence: cited
  path: crates/surf/tests
- confidence: cited
  path: crates/surfd/Cargo.toml
- confidence: cited
  path: crates/surfd/tests
- confidence: cited
  path: crates/test-process/Cargo.toml
- confidence: cited
  path: crates/test-process/tests
- confidence: cited
  path: crates/url/Cargo.toml
- confidence: cited
  path: crates/url/tests
- confidence: cited
  path: crates/view-fixture/Cargo.toml
- confidence: cited
  path: crates/view-fixture/tests
- confidence: cited
  path: crates/view/Cargo.toml
- confidence: cited
  path: crates/view/tests
- confidence: cited
  path: taskfiles/release-smoke.sh
- confidence: cited
  path: taskfiles/test-layout.sh
- confidence: cited
  path: taskfiles/test-layout.yml
revision: 15
---
## Context

The folder rule 'Tests stay fast' says that test binaries are few: one integration-test binary per crate (`tests/main.rs` with modules). Each `.rs` file
directly under a crate's `tests/` is linked as its own executable, and cargo-nextest lists each executable in its own process before a test runs.

Counted at `5c0eb6d` (branch `plan/useful-for-agents`; its code is `main` at `0721558`): 77 files under `crates/*/tests/*.rs` in 16 crates (`surf` 12,
`session` 9, `fetch` 8, `answer` 6, `surf-cli` 6, `view` 6, `agent` 5, `html-conformance` 5, `surfd` 5, `html` 4, `ports` 3, `url` 3, `view-fixture` 2,
`page` 1, `secret` 1, `test-process` 1), so 77 integration-test executables. `crates/test-process/tests/main.rs` is the only file of its crate; it has the
file name already and holds its tests in two inline modules (Change 8 moves them into files). 3 files under `conformance/tests/` (its own workspace). 4 crates have `tests/common/` (`agent`, `session`, `surf`, `surfd`), declared by 21
`mod common;` lines. No manifest has `autotests`, `[[test]]`, `harness = false` or `required-features`. Wave 10 (`wave/a10-label-limits`, `a4fa398`) adds
`crates/html/tests/limits.rs`, `crates/html/tests/adversary_limits.rs`, `crates/html/tests/common/` (2 more `mod common;` lines) and
`crates/page/tests/label.rs`. The unit counts again on its base (Placement) and states the counts in its report; its cases compare against those.

What names a test target today:

- `.github/workflows/release.yml:152`: `cargo test --locked --release --package "$PACKAGE" --target "$TARGET" --test cli`, on two Linux and two macOS
  targets. The workflow also runs on `workflow_dispatch` (line 12) and publishes nothing on a manual run.
- `taskfiles/release-smoke.sh:96`: `set -- --locked --release -p surf-session --test e2e`, run with `SURF_BIN_DIR`, which
  `crates/session/tests/common/mod.rs:229` and `:267` read.
- The comment `crates/view/tests/prototype.rs:68` (`--test prototype`).
- The two `grep` lines of `task conform-rust` (`Taskfile.yml:100-101`): they match the names of three tests of `conformance/tests/planted_defect.rs` and
  `conformance/tests/two_of_each_entity.rs` without a module prefix.

Paths of test files (not targets) are named in `taskfiles/release.yml:15`, `taskfiles/release-smoke.sh:20`, `docs/releasing.md:41`, `AGENTS.md`, `docs/`,
the comments of `crates/session/src/lib.rs`, `crates/url/src/lib.rs`, `crates/secret/src/compare.rs`, `crates/view-fixture/src/main.rs`,
`crates/view-fixture/src/bin/surf-view-foreign.rs` and `crates/html-conformance/src/entities_table.rs`, and in the Writes lists of stories. No file moves, so
these stay true.

What holds in one binary, read at `5c0eb6d`:

- `#[path = "../../../agent/tests/common/base.rs"]` (`crates/session/tests/common/mod.rs:8`, `crates/surfd/tests/common/mod.rs:8`),
  `#[path = "../src/compare.rs"]` (`crates/secret/tests/secret.rs:12`) and `include_str!("data/…")` (`crates/url/tests/urltestdata.rs:17-19`) resolve
  against the directory of the file that holds them, which does not change. No test file declares another module from a file (`mod x;`) but these and
  `mod common;`, and none uses a `crate::` or `super::` path.
- `#![forbid(unsafe_code)]` heads 22 test files; as an inner attribute of a module it stays legal and forbids `unsafe` in that module.
- `crates/fetch/tests/adversary_redirect_memory.rs:77` is a `#[global_allocator]` that counts every thread (its head, lines 14-15); in one binary it is the
  allocator of every test of `surf-fetch`. Wave 10's `crates/html/tests/common/mod.rs` counts per thread and becomes the allocator of every test of
  `surf-html`.
- `env!("CARGO_BIN_EXE_*")` (12 uses) and `CARGO_TARGET_TMPDIR` work the same in a `[[test]]` target.
- `.cargo/config.toml` sets `debug = false` for the profiles `dev` and `test`.
- `task test` (`Taskfile.yml:322`) prints the ten slowest tests by JUnit class name, which becomes `<package>::main`, and as its last line
  `test step: N s of its budget of 120 s` (line 380). `.config/nextest.toml` ends a test at 60 s (`slow-timeout = { period = "60s", terminate-after = 1 }`).
  `task check` runs `task test`; `task check-runner` runs `task check` in a clean export of `HEAD`.
- libtest matches a test filter as a substring of the full name. `cli::` also matches `review1_cli::` (`crates/surf-cli`); `view::` also matches
  `adversary_view::` and `events::` also matches `adversary_events::` (`crates/session`); `fetch::` also matches `review1_fetch::` and `review1_security_fetch::` (`crates/fetch`).
  `e2e::` (`crates/session`), `json::` and `prototype::` (`crates/view`) match one module each.

## Change

Bounds for every command of this story. Every build and test runs through the build pool: `cargo` is wrapped by the pool's shim, every other command that
builds or tests gets the `slot` prefix (`slot task test`). The pool ends a job after 10 minutes; a command that needs longer names its time with
`slot --timeout <t>` or, for `cargo`, `SLOT_TIMEOUT=<t>`, at most 1 hour; a `timeout` wrapped around the command extends nothing. A script that measures or
waits (the three timed runs of statement 1) starts without the `slot` prefix, and the commands in it that build or test carry it. The test step ends within
120 s and no test runs longer than 60 s. Nothing that stays in the repository is written in Python. Every `cargo` command of this story, in Change and in
Acceptance, runs with `RUSTFLAGS='-D warnings'`, the flags `task test` sets, so that they share one build; the first `cargo` command after a change of a
manifest or a test file, and the first on the base, carries `SLOT_TIMEOUT=30m`.

1. Measure first, on the base, before any change (decision C7). Lists go under `.out/otb/`, which is not committed.
   - The test executables: `SLOT_TIMEOUT=30m RUSTFLAGS='-D warnings' cargo nextest list --workspace --locked --list-type binaries-only --message-format json
     > .out/otb/binaries-before.json`; per crate the number of entries of `kind` `test`
     (`jq -r '."rust-binaries"[] | select(.kind == "test") | ."binary-id"' .out/otb/binaries-before.json | sed 's/::.*//' | LC_ALL=C sort | uniq -c`), and
     the summed bytes of their `binary-path` files (`stat -c %s`, summed by `awk`).
   - The tests: `cargo nextest list --workspace --locked --message-format json`, written as one line `<binary-id> <test name>` per test with
     `jq -r '."rust-suites" | to_entries[] | .key as $b | .value.testcases | keys[] | "\($b) \(.)"'`, sorted, into `.out/otb/tests-before.txt`; the same with
     `--manifest-path conformance/Cargo.toml` into `.out/otb/conformance-before.txt`. Before it reads a field, the unit prints one record of each JSON
     (`jq '."rust-binaries" | to_entries[0]'`, `jq '."rust-suites" | to_entries[0]'`) and names in its report the field names it used.
   - The selections: `cargo test --locked -p surf-cli --test cli -- --list` into `.out/otb/cli-before.txt` and
     `cargo test --locked -p surf-session --test e2e -- --list` into `.out/otb/e2e-before.txt`.
   - The step: one `SLOT_TIMEOUT=30m RUSTFLAGS='-D warnings' cargo nextest run --workspace --locked --no-run`, so that the timed runs do not compile, then
     three runs of `slot task test`, each after `cat /proc/loadavg`, keeping the line `test step: N s of its budget of 120 s` and the count line
     `N tests: P passed, F failed, I ignored (cargo-nextest n, doc tests d)`.
   No build from a clean `target/`; no `cargo test --workspace`.
2. The form is proved on one crate first (decision C1): the unit applies statement 3 to `crates/agent` alone (5 files and `tests/common/`), shows case 2,
   and only then changes the other crates.
3. Every crate directory that has a `tests/` directory (the 16 of Context with those wave 10 adds, `crates/test-process` and `crates/fetch` included,
   decision C2) and `conformance/` (decision C3) get, in `Cargo.toml`, `autotests = false` under `[package]` and a target
   `[[test]]` with `name = "main"` and `path = "tests/main.rs"`. `tests/main.rs` (new, except in `crates/test-process`) holds a head comment, one
   `mod <file>;` line per `tests/<file>.rs`, sorted, and `mod common;` where `tests/common/` exists. It holds `#![forbid(unsafe_code)]`, except in
   `crates/fetch` and `crates/html`, whose counting allocators are `unsafe`. No file moves and none is renamed, so no `#[path]` or `include_str!` changes.
   `mod core;` (`crates/session/tests/core.rs`) sits at the root of the binary beside the crate `core`; the unit reports whether it builds. If it does
   not, `main.rs` declares it `#[path = "core.rs"]` on one line and `mod session_core;` on the next, and its tests are named `session_core::<name>`.
4. In each test file, `mod common;` (21 lines at `5c0eb6d`, 23 with wave 10) becomes `use crate::common;`. No other line of a test file changes but those
   of statements 5 and 7. No test is removed, renamed or changed in what it asserts; its name gains its module as a prefix (`e2e::<name>`).
5. `crates/fetch` keeps no binary of its own (decision C2). The sentence of `crates/fetch/tests/adversary_redirect_memory.rs:14-15` becomes: "The allocator
   is the global allocator of the one test binary of `surf-fetch` and counts every thread of it. cargo-nextest, which the gate runs, starts each test in
   its own process, so a case counts only its own allocations; the cases still hold `SERIAL`. Without cargo-nextest, run
   `cargo test -p surf-fetch -- --test-threads=1`."
6. `Taskfile.yml:100-101` (`task conform-rust`) match `planted_defect::a_navigation_storage_that_drops_every_put_fails_at_least_one_scenario`,
   `two_of_each_entity::one_identity_for_every_instance_of_an_entity_is_noticed_for_each_entity` and
   `two_of_each_entity::a_view_cut_to_its_first_row_is_noticed_for_each_entity` (decision C3).
7. Every selection of a test target names `main` and a module, and selects the same tests as before:
   - `.github/workflows/release.yml:152`: `cargo test --locked --release --package "$PACKAGE" --target "$TARGET" --test main cli:: -- --skip review1_cli::`
     (`cli::` alone would also run `review1_cli::`). macOS now compiles the whole test binary of `surf-cli`, where it compiled `tests/cli.rs` alone.
   - `taskfiles/release-smoke.sh:96`: `set -- --locked --release -p surf-session --test main e2e::` (without the filter the smoke run would run every test
     of `surf-session` against `SURF_BIN_DIR`).
   - The comment `crates/view/tests/prototype.rs:68`: `cargo test -p surf-view --test main prototype::`.
8. A gate check keeps the form (decision C4): `taskfiles/test-layout.sh <root>`, POSIX `sh`, no cargo call, run by the task `test-layout` of the new
   `taskfiles/test-layout.yml`, included from the root `Taskfile.yml` as `planning-only` is and run by `task check` after `lock-check`. For each directory
   `crates/*/tests` and `conformance/tests` under `<root>` it fails, naming the path:
   - when `main.rs` is missing;
   - when the `Cargo.toml` beside the directory has no line `autotests = false`, or has not exactly one `[[test]]` entry, or that entry does not hold
     exactly the two keys `name = "main"` and `path = "tests/main.rs"` (a `test = false`, `required-features` or `harness = false` key fails) (decision C9);
   - when a line of `main.rs` is not one of: blank, a `//` comment (`//!` included), an inner attribute `#![…]`, a `use …;` line, or a declaration
     `mod <name>;`, `#[path = "<stem>.rs"]` followed by `mod <name>;` on the next line or on the same line, or `#[path = "<dir>/<stem>.rs"]` likewise;
     so `#[cfg…]`, `/*` and an inline `mod x {` fail (decision C10). This fits statement 3: the head comment is `//!`, `#![forbid(unsafe_code)]` is an inner
     attribute; `crates/fetch` and `crates/html` put nothing for their global allocators in `main.rs` (each stays in its module file), they only leave out
     `#![forbid(unsafe_code)]`. `crates/test-process/tests/main.rs` holds its tests in two inline modules (`mod lines {`, `mod children {`); the unit
     moves each body, unchanged, into `tests/lines.rs` and `tests/children.rs`, declared `mod lines;` and `mod children;`, so its test names stay;
   - when a `*.rs` file directly in it other than `main.rs` is not declared in `main.rs` by one of those forms (the whole line `mod <stem>;`, `grep -qx`);
   - when a `*.rs` file in a subdirectory one level down (`tests/<dir>/<stem>.rs`, `common/` and `data/` skipped) is not declared: the whole line
     `#[path = "<dir>/<stem>.rs"]` followed by the whole line `mod <stem>;`, or the one line `#[path = "<dir>/<stem>.rs"] mod <stem>;`, the form of
     `story:tls-decisions` for `crates/tls/tests/tls/` (decision C11);
   - when a selection drifts (decision C12): the script holds the list of selections, one line each of crate directory, selected module and skipped
     modules (`crates/surf-cli cli review1_cli`, `crates/session e2e`, `crates/view prototype`, the packages `surf-cli`, `surf-session`, `surf-view`), and
     fails naming the module when another module name on a `mod` line of that crate's `main.rs` (either level) ends with the selected name and is not in
     the skipped list. It compares module names, not file stems: a file `a.rs` declared `mod zz_cli;` joins the selection `cli::`.
   Else it prints `test-layout: <d> test directories, <f> test files, one test binary each`. The task `test-layout-test` of the same file, run by
   `task check` after `planning-only-test`, copies the `Cargo.toml` files, the `tests/*.rs` files and the `tests/<dir>/*.rs` files of `crates/*` and
   `conformance` into `.out/test-layout/` and runs the script there twelve times: the copy as it is passes; it fails naming the path with a planted
   `crates/page/tests/planted.rs`; with `autotests = false` taken out of `crates/page/Cargo.toml`; with `crates/page/tests/main.rs` removed; with the
   `[[test]]` entry taken out of `crates/page/Cargo.toml`; with a second `[[test]]` entry in it; with a third key `harness = false` in its entry; with
   `// mod elements;` in place of `mod elements;` in `crates/page/tests/main.rs`; with `#[cfg(any())]` on the line before `mod elements;`; with
   `#[cfg(any())]` on the line before a `#[path = "elements.rs"]` line that declares `mod elements;`; with a planted `crates/page/tests/tls/planted.rs`
   that no `#[path]` line names; with a planted `crates/surf-cli/tests/a.rs` declared `mod zz_cli;` (a module name that ends with the selected `cli`, not
   skipped).
   `SCRIPT=<path>` runs a copy of the script.
9. `AGENTS.md` follows: "Layout" states the form of statement 3, that a new test file gets its `mod` line in `tests/main.rs`, that a module may also lie
   one directory down, declared by `#[path = "<dir>/<stem>.rs"]` and `mod <stem>;` (the form of `crates/tls/tests/tls/`), that a new crate starts with
   `autotests = false`, `[[test]] name = "main"` and `tests/main.rs`, and that one file's tests are selected with `--test main <file>::` (with
   `-- --skip <other>::` where another module's name ends in `<file>`, Context); that the selections are listed in `taskfiles/test-layout.sh`, and a stem
   added to the skipped list of a selection gets its `--skip <stem>::` in that selection; "Commands" names `task test-layout` and `task test-layout-test`;
   "The gate's numbers" gives the count line and the `test step` line of the last run of statement 11, and the test suites of the conformance crate.
10. No debug info in a test binary: `.cargo/config.toml` keeps `debug = false` for `dev` and `test`; the story does not change it.
11. The same measurement as statement 1, on the branch, on the same base. The numbers of 1 and 11 (executables per crate, summed bytes, three `test step`
    lines each with the load average, the count lines, the line counts and `sha256sum` of the `.out/otb/` lists) are recorded in the story's evidence
    (`aep plan artifact evidence story:one-test-binary-per-crate`, from the unit's report). The bound on time is the budget, not a comparison.

## Acceptance

One case per statement of Change, with the same number. The bounds of Change hold for every command. `<base>` is the commit the unit started on;
`<d>` and `<f>` are its counts on that base: `<d>` the directories `crates/*/tests` and `conformance/tests` (17 at `5c0eb6d`), `<f>` the `*.rs` files
directly in them other than `main.rs` (79 at `5c0eb6d`, 82 with wave 10).

1. Before: the report gives, from `<base>`, the output of the `uniq -c` of statement 1 (one line per crate, 77 executables at `5c0eb6d`), the summed bytes,
   the three `test step: N s of its budget of 120 s` lines with their load averages, the count line, and `wc -l` and `sha256sum` of
   `.out/otb/tests-before.txt`, `conformance-before.txt`, `cli-before.txt` and `e2e-before.txt`. `wc -l < .out/otb/tests-before.txt` equals
   `jq '."test-count"'` of the JSON it was written from, and that number equals the cargo-nextest count `n` of the gate's count line; `n` counts the tests
   nextest skipped, ignored ones included, so it equals `test-count` of a list made without a filter. For `conformance-before.txt`, its own workspace,
   which the gate's count line does not count (`task conform-rust` runs it), only `wc -l` equals `test-count` of its JSON. `cli-before.txt` and
   `e2e-before.txt` each hold at least one line ending `: test`.
2. First crate: after statement 3 in `crates/agent` alone,
   `SLOT_TIMEOUT=30m RUSTFLAGS='-D warnings' cargo nextest list -p surf-agent --locked --list-type binaries-only --message-format json | jq -r '."rust-binaries"[] | select(.kind == "test") | ."binary-id"'`
   prints exactly `surf-agent::main`. With a planted `crates/agent/tests/planted.rs` that holds `#[test] fn planted() {}` and no `mod` line, the same
   command prints exactly `surf-agent::main`, `RUSTFLAGS='-D warnings' cargo nextest list -p surf-agent --locked` prints no line that holds `planted`, and
   `sh taskfiles/test-layout.sh .` exits 1 naming `crates/agent/tests/planted.rs`. The planted file is then removed. The report shows the four outputs.
3. Every crate: the command of case 2 (with its `SLOT_TIMEOUT=30m`) with `--workspace` in place of `-p surf-agent`, piped through `LC_ALL=C sort`, prints exactly these 16 lines and
   one line `<package>::main` for each test directory the base adds: `surf-agent::main`, `surf-answer::main`, `surf-cli::main`, `surf-fetch::main`,
   `surf-html-conformance::main`, `surf-html::main`, `surf-page::main`, `surf-ports::main`, `surf-secret::main`, `surf-session::main`,
   `surf-test-process::main`, `surf-url::main`, `surf-view-fixture::main`, `surf-view::main`, `surf::main`, `surfd::main`. With
   `--manifest-path conformance/Cargo.toml` in place of `--workspace` it prints exactly `surf-conformance::main`.
4. No test removed: `sed '/^[^ ]*::main /!s/^\([^ :]*\)::\([^ /]*\) /\1::main \2::/' .out/otb/tests-before.txt | LC_ALL=C sort > .out/otb/tests-renamed.txt`
   turns `surf-session::e2e <name>` into `surf-session::main e2e::<name>` and leaves unit-test and binary lines as they are; the list of statement 1 taken
   on the branch into `.out/otb/tests-after.txt`; `diff .out/otb/tests-renamed.txt .out/otb/tests-after.txt` prints nothing and exits 0, and
   `wc -l` of both files equals the `wc -l` of case 1 and `jq '."test-count"'` of the JSON of the branch. The same for `conformance-before.txt` and
   `conformance-after.txt`. If statement 3 declared `session_core`, the `sed` also turns `surf-session::main core::` into
   `surf-session::main session_core::`. And
   `git diff --numstat <base> -- 'crates/*/tests/*.rs' 'conformance/tests/*.rs' ':(exclude)crates/*/tests/main.rs' ':(exclude)conformance/tests/main.rs'`
   prints `1	1	<file>` for each file that held `mod common;` (21 at `5c0eb6d`, 23 with wave 10), one line for
   `crates/fetch/tests/adversary_redirect_memory.rs` and one for `crates/view/tests/prototype.rs`, one each for the new `crates/test-process/tests/lines.rs`
   and `children.rs` (statement 8, their bodies as they were in its `main.rs`), and no other line.
5. Fetch: `grep -c -- 'cargo test -p surf-fetch -- --test-threads=1' crates/fetch/tests/adversary_redirect_memory.rs` prints `1`;
   `RUSTFLAGS='-D warnings' cargo nextest run -p surf-fetch --locked` exits 0; `grep -c '^surf-fetch::main adversary_redirect_memory::' .out/otb/tests-after.txt`
   and `grep -c '^surf-fetch::adversary_redirect_memory ' .out/otb/tests-before.txt` print the same number, and it is not 0.
6. Conformance: `slot task conform-rust` exits 0, and
   `grep -c '^test \(planted_defect::a_navigation_storage_that_drops_every_put_fails_at_least_one_scenario\|two_of_each_entity::one_identity_for_every_instance_of_an_entity_is_noticed_for_each_entity\|two_of_each_entity::a_view_cut_to_its_first_row_is_noticed_for_each_entity\) \.\.\. ok$' .out/conform-rust-test.log`
   prints `3`.
7. Selections: `grep ': test$' .out/otb/cli-before.txt | sed 's/^/cli::/'` equals
   `RUSTFLAGS='-D warnings' cargo test --locked -p surf-cli --test main cli:: -- --skip review1_cli:: --list | grep ': test$'`, and
   `grep ': test$' .out/otb/e2e-before.txt | sed 's/^/e2e::/'` equals
   `RUSTFLAGS='-D warnings' cargo test --locked -p surf-session --test main e2e:: -- --list | grep ': test$'` (each pair compared with `diff`, empty). The
   release workflow runs once on the story branch by `workflow_dispatch` (`gh workflow run release.yml --ref <branch>`; a manual run publishes nothing,
   `release.yml:4-5`, it leaves the archives as the run's artifacts); its four package jobs, which run
   the `--test main cli::` step and `taskfiles/release-smoke.sh`, end `success`, and the evidence holds the run's URL and conclusion.
8. Gate check: `slot task test-layout` on the branch prints `test-layout: <d> test directories, <f> test files, one test binary each` and exits 0;
   `slot task test-layout-test` prints `test-layout-test: 12 case(s), each decided as expected by taskfiles/test-layout.sh` and exits 0 (the cases of
   statement 8, among them those of decisions C9 to C12); with `SCRIPT=<copy>` where the copy always exits 0, it exits 1 and names the eleven cases that
   should fail. Both take under 1 s; neither calls cargo.
9. References: `git grep -n -- '--test ' -- ':!.engineering'` prints, beside lines that hold `--test main `, only the four lines that name no target of a
   crate of this repository: the note "`cargo test -p <package> --test <name>` does not build the package's examples" of `AGENTS.md` (line 900 at
   `5c0eb6d`), `docs/plan/agent-wave-3.md:85` (a wave record), and `docs/design/tls.md:248` and `:351` (records of scratch tests). `AGENTS.md`
   holds the text of statement 9: `grep -c 'autotests = false' AGENTS.md` prints at least 1 more than on `<base>`; `grep -c -- '--test main ' AGENTS.md`
   at least 1; `grep -c 'task test-layout-test' AGENTS.md` at least 1; `grep -c 'test step: [0-9.]* s of its budget of 120 s' AGENTS.md` at least 1 more
   than on `<base>`, the line of the last run of statement 11.
10. No debug info: with `bin=$(jq -r '."rust-binaries"."surf-session::main"."binary-path"' .out/otb/binaries-after.json)`, `test -f "$bin"` exits 0,
    `readelf -S "$bin"` exits 0 and lists a `.text` section, and `readelf -S "$bin" | grep -c '\.debug_'` prints `0`.
11. Budget: each of the three `slot task test` runs on the branch exits 0 and prints `test step: N s of its budget of 120 s` with N at most 120;
    `grep -c TIMEOUT .out/test.log` prints `0` after each; the count line has the same total N as in case 1; the `uniq -c` of statement 11 prints one
    executable per crate. The evidence holds the numbers of statements 1 and 11.

## Out of scope

- Unit tests inside `src/`: they stay in the test binary of each library and binary target.
- A build of the whole workspace from a clean `target/`, and any run of `cargo test --workspace`: neither has the 60 s bound nor the 120 s budget; the gate
  runs cargo-nextest (`task test`).
- Moving test files into a subdirectory, and the form of the test binary of `crates/tls` (`story:tls-lock-allow`).
- The generated workspace under `generated/rust/`, which holds no tests.
- Changing what a test checks, and making a test faster.
- `docs/releasing.md`: it names `crates/session/tests/e2e.rs` by path (line 41), which stays true.
- The planning store: the stories that follow this one were changed on `plan/useful-for-agents` before it starts (Placement).

## Placement

Its base is `main` after three merges: release 0.3.0 (the security fixes), `fix/tests-hold-under-load` (`story:tests-hold-under-load`, which writes
`crates/surf/tests/common/mod.rs`, `crates/surf/tests/review1_generated_check.rs` and `crates/view/tests/stream_bounds.rs`; the story is in the store of
`release/0.3.0`, merged there, not of this branch, and reaches `main` with release 0.3.0, so no edge to it can be recorded here: the base merge is the
order), and wave 10
(`wave/a10-label-limits`: `story:html-accessible-name` and `story:html-parser-limits`, each an edge; they add the test files counted in Context).

It runs alone: no other unit that writes under `crates/*/tests`, `conformance/tests`, the `Cargo.toml` of a crate with tests, `Taskfile.yml`,
`taskfiles/`, `.github/` or `AGENTS.md` runs beside it. One unit, one implementor; alone in its wave, the unit writes the root `Taskfile.yml` itself.
Round 1 (the rounds after wave 10) starts after its merge; `docs/plan/agent-waves.md` places it on the row of wave 10.

The stories that follow it were changed on `plan/useful-for-agents`, the branch that holds every planning story of surf, before it starts (decision C6).
Each story that adds a file to a crate's `tests/` names `crates/<c>/tests/main.rs` (one `mod` line) in its Writes; each story that creates a crate
with tests creates it in the form of Change statement 3; each `--test <file>` became `--test main <file>::`. Direct `depends_on` edges to this story:
`story:agent-tasks`, `story:tls-lock-allow`, `story:tls-fetch-https`, `story:tls-session-anchors`, `story:html-document-port`, `story:html-tree-tables`
and `story:page-text-answer-block`, and the drafts `story:fetch-head-memory`, `story:view-surfd-start-cases` and `story:page-text-on-other-commands`;
every other such story reaches it through one of them.

## Writes

`crates/<c>/Cargo.toml` and `crates/<c>/tests/` of each crate with tests (`agent`, `answer`, `fetch`, `html`, `html-conformance`, `page`, `ports`,
`secret`, `session`, `surf`, `surf-cli`, `surfd`, `test-process`, `url`, `view`, `view-fixture`): `tests/main.rs` (new), one line in each file that held
`mod common;`, the head comment of `crates/fetch/tests/adversary_redirect_memory.rs`, the comment `crates/view/tests/prototype.rs:68`.
`conformance/Cargo.toml`, `conformance/tests/main.rs` (new). `Taskfile.yml` (the two `grep` lines of `conform-rust`, the include of
`taskfiles/test-layout.yml`, two lines of `check`), `taskfiles/test-layout.sh` (new), `taskfiles/test-layout.yml` (new), `taskfiles/release-smoke.sh`
(line 96), `.github/workflows/release.yml` (line 152), `AGENTS.md` ("Layout", "Commands", "The gate's numbers").
