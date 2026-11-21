---
format: aep.planning-md/3
id: story:runner-surf-cli-package
kind: story
status: implemented
title: Move the surf binary into a package of its own
summary: crates/surf-cli holds the surf binary and its tests, so it can depend on crates that depend on the surf library.
relations:
- depends_on: story:runner-decisions
- decomposes: epic:agent-cli-target
- depends_on: story:agent-cli-spec
- depends_on: story:agent-session-spec
scope:
- confidence: cited
  path: crates/surf-cli
- confidence: cited
  path: crates/surf/Cargo.toml
- confidence: cited
  path: crates/surf/src/main.rs
- confidence: cited
  path: crates/surf/tests/cli.rs
- confidence: cited
  path: crates/surf/tests/output_streams.rs
- confidence: cited
  path: crates/surf/tests/review1_cli.rs
- confidence: cited
  path: crates/surf/tests/review1_output.rs
- confidence: cited
  path: crates/surf/tests/review2_output.rs
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-21T02:59:46Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

`surf run` and `surf scenario from` are subcommands of the `surf` binary
(`story:runner-decisions`, D11). `surf run` drives the session core, which story
`agent-session-process` puts in `crates/session` (package `surf-session`), a crate that depends
on the `surf` library. Today the binary and the library are one package, `crates/surf`
(`crates/surf/Cargo.toml`, `crates/surf/src/main.rs`, 71 lines): a binary in that package cannot
depend on a crate that depends on its own library, because cargo refuses the cycle. So the binary
moves into a package of its own, with its behaviour unchanged.

It moves first (`docs/plan/agent-plan-revision-1.md`, D6): it depends on `runner-decisions`
only, and every later story that changes the binary or its tests writes `crates/surf-cli` and
depends on this one: `agent-fetch-redirects`, `agent-request-body-spec`, `agent-fetch-post`,
`agent-form-port`, `html-document-port`, and in `epic:e2e-scenario-runner`
`runner-scenario-from`, `runner-run` and `runner-e2e` (the last through `runner-run`).

The tests that run the binary (`env!("CARGO_BIN_EXE_surf")`) are, on `main` at `1c979b4`,
`crates/surf/tests/cli.rs`, `output_streams.rs`, `review1_cli.rs`, `review1_output.rs` and
`review2_output.rs` (`grep -l CARGO_BIN_EXE_surf crates/surf/tests/*.rs`); a test of another
package cannot name that binary, so they move with it. A test file that runs the binary and lands
on `main` before this story moves with them; the implementor runs the same `grep` at the start
and reports the list.

The output format and the exit statuses of `surf` are documented in `docs/surf-command.md` since
wave 1; `README.md` is a page for people. The binary and the moved tests still name `README.md`
for them in comments, so the move corrects those comments.

`story:agent-session-spec` may add the generated `agent` crate to `crates/surf/Cargo.toml`,
which this story edits too; this story depends on it, so the order is stated.

## Change

- A new package `crates/surf-cli` (package `surf-cli`), holding the binary `surf` (`[[bin]]
  name = "surf"`), its `src/main.rs` moved from `crates/surf/src/main.rs` unchanged apart from
  paths and comments, and the five binary tests above moved to `crates/surf-cli/tests/` under the
  same file and test names. Every comment of the moved files that names `README.md` for the output
  format or an exit status names `docs/surf-command.md` instead; on the wave branch these are
  `main.rs:3`, `output_streams.rs:1` and `:105`, `review1_cli.rs:1` and `:77`, and
  `review2_output.rs:2`, `:101` and `:121` (`grep -n README` over the five files and `main.rs`;
  the implementor runs it again and reports the lines). After the move that `grep` over
  `crates/surf-cli/` finds no line. It depends on `surf`, `surf-fetch` and `surf-ports` by path
  (`main.rs` imports `surf_ports::ComponentPorts`, `main:crates/surf/src/main.rs:14`).
- `crates/surf`: `src/main.rs` removed, the package description no longer names a command line,
  and the library tests (on `main`: `conform_totals.rs`, `escape.rs`, `generated_check_links.rs`,
  `generated_check_modes.rs`, `library.rs`, `lock_check.rs`, `review1_generated_check.rs`,
  `review1_library.rs`, `review2_generated_check.rs`) stay. `surf-fetch` stays a
  dependency, because `answer` in `crates/surf/src/lib.rs` uses it.
- The root `Cargo.lock` gains `surf-cli`; in a wave it is the coordinator's file.

Surfaces: `crates/surf-cli/` (new), `crates/surf/src/main.rs` (removed), `crates/surf/Cargo.toml`,
`crates/surf/tests/` (five files moved out). `AGENTS.md` and `README.md` through the coordinator
(revision D7), below.

## For AGENTS.md and README.md

Handed over in the unit's report under "For AGENTS.md" and "For README.md" when the wave has
more than one unit; written by the unit itself otherwise.

- `AGENTS.md`, "Layout": the `crates/surf` entry ends at the library ("`crates/surf` (`surf`): a
  library. It builds the generated `System` ... The binary is in `crates/surf-cli`."), and a new
  entry: "`crates/surf-cli` (`surf-cli`): the binary `surf`. Depends on `surf`, `surf-fetch` and `surf-ports`.
  It is a package of its own so that it can depend on crates that depend on the `surf` library."
- `README.md`, the paragraph naming `crates/surf` as the command line: the command line is
  `crates/surf-cli`, the assembly `crates/surf`.

## Acceptance

`slot task check` exits 0, `cargo test -p surf-cli` runs and passes the five test binaries `cli`,
`output_streams`, `review1_cli`, `review1_output` and `review2_output`, `crates/surf/tests/` holds
none of those five files, `grep -rn README crates/surf-cli/` prints no line, and `cargo metadata --no-deps
--format-version 1` lists the binary target `surf` under package `surf-cli` and no binary target
under package `surf`.

## Out of scope

Any new subcommand (stories `runner-scenario-from` and `runner-run`). Any change to what `surf
<url>` prints or its exit statuses.

## Scope

Derived 2026-11-21 by `story-scoper`.

- **Primary surface:** `crates/surf-cli/` (new package) — cited, the story's Change names "A new package `crates/surf-cli` (package `surf-cli`)".
- **Files written:**
  - `crates/surf-cli/Cargo.toml` — cited, package `surf-cli` with `[[bin]] name = "surf"`, depending on `surf` and `surf-fetch` by path.
  - `crates/surf-cli/src/main.rs` — cited, moved from `crates/surf/src/main.rs` (71 lines) "unchanged apart from paths".
  - `crates/surf-cli/tests/cli.rs`, `output_streams.rs`, `review1_cli.rs`, `review1_output.rs`, `review2_output.rs` — cited, the five files with `CARGO_BIN_EXE_surf` on `main` at `1c979b4`. I confirmed the same five with `git grep` on `main`.
  - `crates/surf/src/main.rs` — cited, removed.
  - `crates/surf/Cargo.toml` — cited, only the `description` changes: it no longer says "and the surf command line" (`main:crates/surf/Cargo.toml:3`). The binary target comes from auto-discovery of `src/main.rs`, so no `[[bin]]` is removed. inferred
  - `crates/surf/tests/cli.rs`, `output_streams.rs`, `review1_cli.rs`, `review1_output.rs`, `review2_output.rs` — cited, removed from `crates/surf/tests/`.
  - A binary-running test file that lands on `main` before this story starts also moves (the story says the implementor re-runs the `grep` and reports the list). — cited
- **Files read, not written:**
  - `crates/surf/src/lib.rs`: the items the binary imports, `Answer`, `EXIT_STOPPED`, `EXIT_USAGE`, `MemoryBrowser`, `answer`, `escape` and `exit_status`. — cited, `main:crates/surf/src/main.rs:10`
  - `crates/fetch`: `Limits` and `fetch`. — cited, `main:crates/surf/src/main.rs:11`
  - `crates/ports`: `ComponentPorts`. — cited, `main:crates/surf/src/main.rs:12`
  - `crates/surf/tests/` (the nine library tests) and `crates/surf/src/lib.rs` stay in place. — cited
  - The five moved tests use only `std`, no `surf_*` crate and no `../` path. A `grep` over `main` found no `CARGO_MANIFEST_DIR` use in them, so moving them needs no path edit. — cited
  - `conformance/Cargo.toml` depends on `surf` by path (`main:conformance/Cargo.toml:12`). `surf`'s dependencies do not change, so `conformance/Cargo.lock` does not change. — inferred
- **Handed to the coordinator:**
  - `Cargo.lock` (root): gains `surf-cli` (the story's Change says so). `task build` and `task test` run `--locked` (`main:Taskfile.yml:243,252`), so the unit's gate needs the regenerated lock in its tree. The coordinator commits it. — cited for the story's statement, inferred for the `--locked` consequence
  - `AGENTS.md`, "Layout", the `crates/surf` entry (`main:AGENTS.md:111-116`): it ends at the library and says "The binary is in `crates/surf-cli`". Add the entry `crates/surf-cli` (`surf-cli`): the binary `surf`, depending on `surf` and `surf-fetch` (and `surf-ports`, see the discrepancy below), a package of its own so it can depend on crates that depend on the `surf` library. — cited, the text is in the story
  - `AGENTS.md:159`, "the crate `surf` assembles them into the `surf` command line": it names the old crate. The story does not list this line. — inferred
  - `README.md`, the paragraph naming `crates/surf` as the command line (`main:README.md:14`): the command line is `crates/surf-cli`, the assembly `crates/surf`. — cited
  - Root `Taskfile.yml`: nothing. It builds and tests with `--workspace` and `crates/*` is a glob member (`main:Cargo.toml`), so no line mentions the package. — cited
- **Symbols:** no public item is added or changed. Package `surf-cli` and binary target `surf` are new, and `surf` loses its binary target. — cited, the Acceptance asks `cargo metadata` to list it that way. The moved `main` and `diagnose` are private. — inferred
- **Would collide with:**
  - `story:agent-cli-spec` (runs first) shares one file, `crates/surf/Cargo.toml`. It adds the generated `agent` crate as a dependency (its body says so, and that "`runner-surf-cli-package` edits `crates/surf/Cargo.toml` too"). This story edits only the `description`, a different line, and starts after it. — cited
  - `story:agent-cli-spec` also lists `crates/surf/tests/library.rs`, `crates/surf/tests/review1_library.rs` and `crates/surf/src/lib.rs`. This story does not write them, so no collision. — cited
  - `story:agent-cli-spec` may need `crates/surf/src/main.rs` if `ComponentPorts::new()` stops being a whole-set constructor, which would put `main.rs` in both. The story lists `crates/surf/src/lib.rs` and `crates/ports`, not `main.rs`. `main:crates/surf/src/main.rs:48` calls `MemoryBrowser::from_ports(ComponentPorts::new(), transport)`, and `ports` is where `AgentPorts` lands. I read that as `main.rs` staying untouched. — inferred
  - `story:agent-session-spec`: no shared file. It writes `crates/surf/src/lib.rs` (only where forced), `crates/ports/` and `conformance/src/`, plus `spec/` and `generated/`. — cited
  - `story:agent-cli-client`: no shared file. It writes `crates/agent/` only. — cited
  - Both of those, like this story, hand `Cargo.lock` and `AGENTS.md` to the coordinator. — cited
  - Later stories that write `crates/surf-cli` depend on this one: `agent-fetch-redirects`, `agent-request-body-spec`, `agent-fetch-post`, `agent-form-port`, `html-document-port`, `runner-scenario-from`, `runner-run` and `runner-e2e`. — cited
- **Not yet existing when it starts:**
  - From `story:agent-cli-spec`: the six-component `Browser` and `MemoryBrowser` in `crates/surf/src/lib.rs`, `AgentPorts` in `crates/ports`, and the generated `agent` dependency in `crates/surf/Cargo.toml`. This story moves the binary over whatever `crates/surf` is by then. — cited
  - From `story:runner-decisions`: the D11 subcommand decisions. They are context only, since no new subcommand lands here. — cited
- **Confidence:** high. The story names every path, symbol and test file, and I matched them against `main` (five test files, 71-line `main.rs`, the `surf-fetch` use in `answer`). Two readings are inferred: the `ComponentPorts::new()` point above, and the single-line `description` edit.

Two defects in the story text:
1. Its Change says `surf-cli` "depends on `surf` and `surf-fetch` by path", but `main.rs` also imports `surf_ports::ComponentPorts` (`main:crates/surf/src/main.rs:12`). The manifest needs `surf-ports = { path = "../ports" }` too, or the build fails.
2. After `agent-cli-spec` the generated `agent` crate may also be imported by `main.rs`. I cannot confirm that from the plan branch.

