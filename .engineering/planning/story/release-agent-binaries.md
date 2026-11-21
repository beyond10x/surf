---
format: aep.planning-md/3
id: story:release-agent-binaries
kind: story
status: implemented
title: Release 0.2.0 with the agent binaries for Linux
summary: The release workflow builds, smoke-tests and packs surf, surf-agent, surfd and surf-session into each Linux archive and surf alone for macOS; the four binary packages move to 0.2.0.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-session-process
- depends_on: story:runner-surf-cli-package
scope:
- confidence: cited
  path: .github/workflows/release.yml
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/agent/Cargo.toml
- confidence: inferred
  path: crates/session/Cargo.toml
- confidence: cited
  path: crates/surf-cli/Cargo.toml
- confidence: cited
  path: crates/surfd/Cargo.toml
- confidence: inferred
  path: docs/agent-answer.md
- confidence: cited
  path: docs/releasing.md
- confidence: inferred
  path: website/docs/drive-from-an-agent.md
- confidence: cited
  path: website/docs/try-it.md
- confidence: cited
  path: website/sidebars.ts
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:02:28Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T03:02:28Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-21T04:59:15Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

Decided by the operator on 2026-11-21: surf reaches a checkpoint after wave 4 of
`docs/plan/agent-waves.md` with a release `0.2.0` that ships the agent binaries. Release `0.1.0`
shipped `surf` only (`story:release-binaries`, `CHANGELOG.md`). After wave 4 the agent command
line works end to end with four binaries: `surf` (package `surf-cli`), `surf-agent` (package
`surf-agent`, `crates/agent`), `surfd` (package `surfd`, `crates/surfd`) and `surf-session`
(package `surf-session`, `crates/session`, `story:agent-session-process`). They find each other
only in one directory: `surf-agent open` starts `surfd` from the directory of its own executable
(`crates/agent/src/client.rs`, `start_surfd`), and `surfd` starts `surf-session` from its own
(`crates/surfd/src/daemon.rs`, `SESSION_PROGRAM` and `Config::new`). So an archive and an install
keep the four together.

What `.github/workflows/release.yml` does today: the job `version` reads the version of `PACKAGE:
surf-cli` with `cargo pkgid` and fails a tag that differs or is not annotated; the job `package`,
per target of four (`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`,
`x86_64-apple-darwin`, `aarch64-apple-darwin`), builds `--bin surf` with warnings denied, runs
`cargo test --release --package surf-cli --target <target> --test cli` (whose cases start their
own servers in the test), checks a Linux binary for an `INTERP` header and every binary for a
home path, packs `surf-<version>-<target>/` with `surf`, `README.md`, `LICENSE`, `NOTICE`, and
runs the unpacked `surf` twice: without an argument (exit 2) and on `http://127.0.0.1:1/` (exit
1, `fetch-failed ... Unreachable`). The workflow starts no HTTP server of its own, and it has no
`XDG_RUNTIME_DIR` step (`.github/workflows/check.yml` has one).

macOS: the agent binaries build there and cannot run there.
- `surf_agent::wire::caller_uid` reads the owner of `/proc/self` (`crates/agent/src/wire.rs:574`),
  which macOS does not have; `surfd` refuses its start when it cannot read it
  (`crates/surfd/src/daemon.rs:401`), and the client's token-file owner check needs it.
- The client and `surfd` require `XDG_RUNTIME_DIR` (`crates/agent/src/cli.rs:214`,
  `crates/surfd/src/daemon.rs:381`), which macOS does not set.
- They compile: `cargo check --target aarch64-apple-darwin` and `--target x86_64-apple-darwin`
  of `surf-agent` and `surfd`, all targets, `-D warnings`, exit 0 (measured 2026-11-21 in a
  scratch copy of `crates/agent`, `surfd`, `answer`, `secret`). Nothing in them is behind a
  `cfg` for Linux; `std::os::unix` only.
So `0.2.0` packs them for Linux only: a binary that compiles and refuses every start is not
shipped.

## Change

- `.github/workflows/release.yml`:
  - `version`: unchanged; the tag is still checked against the version of `surf-cli`.
  - `package`: on the two Linux targets it builds `surf-cli`, `surf-agent`, `surfd` and
    `surf-session` (`cargo build --locked --release -p surf-cli -p surf-agent -p surfd -p
    surf-session --bins --target <target>`), applies the `INTERP` and home-path checks to each of
    the four, and packs all four into `surf-<version>-<target>/` with `README.md`, `LICENSE`,
    `NOTICE`. On the two macOS targets it builds and packs `surf` only, as now, and a comment
    says why (the two lines of Context).
  - The smoke test per Linux target, on the unpacked directory, after the existing two runs of
    `surf`: a step that exports a private `XDG_RUNTIME_DIR` as `check.yml` does, then
    `SURF_BIN_DIR="$check/$package" cargo test --locked --release -p surf-session --target
    "$TARGET" --test e2e`. That is the end-to-end case of `story:agent-session-process` (case 1,
    `crates/session/tests/e2e.rs`), run on the packed binaries: a page server on
    `std::net::TcpListener` inside the test, `surf-agent open <address>`, `go`, `look` and
    `close`, each answer compared with its golden text written from `docs/agent-answer.md` and
    read back by `surf_answer::parse`, `look` after `close` answered `did refused closed`; its
    guard stops the case's `surfd`. Then the step fails when a process whose arguments start with
    `$check/$package/` still runs (`ps -eo args`). The case itself asserts that nothing of the
    session is left under its `$XDG_RUNTIME_DIR/surf/<id>/` after `close`.
  - `checksums`: unchanged (four archives, `SHA256SUMS`).
- Versions: `version = "0.2.0"` in `crates/surf-cli/Cargo.toml`, `crates/agent/Cargo.toml`,
  `crates/surfd/Cargo.toml` and `crates/session/Cargo.toml`, the four packages that ship a
  binary. `Cargo.lock` is the coordinator's (`cargo generate-lockfile --offline`).
- `docs/releasing.md`: "What a release holds": a Linux archive holds `surf`, `surf-agent`,
  `surfd`, `surf-session`, `README.md`, `LICENSE`, `NOTICE`; a macOS archive `surf` and the three
  files, and why; the four binaries stay in one directory. "Cutting a release": step 1 sets the
  version in the four manifests; step 7 runs `scan-artifact` on the four ELF binaries of each
  Linux archive (eight commands).
- `CHANGELOG.md`, `## [Unreleased]`, `### Added`, written for a user: `surf-agent open [<url>]`,
  `go <url>`, `look`, `close` drive a browser session from a command line, one session process
  per session under the daemon `surfd`; the session and its token are passed in `SURF_SESSION`
  and `SURF_TOKEN`; the answer format is [`docs/agent-answer.md`](docs/agent-answer.md); Linux
  only, and why macOS waits; the four binaries are installed into one directory.
- `website/docs/try-it.md`, "Install": the Linux archives hold four binaries, installed together
  (`sudo install surf-$version-$target/surf* /usr/local/bin/`); the macOS archives hold `surf`.
- `website/docs/drive-from-an-agent.md` (new) and its entry in `website/sidebars.ts` ("Start
  here", after `try-it`): open, export the two values, go, look, close, one answer shown, a link
  to `docs/agent-answer.md` on GitHub at `main`, "Quoting lowers the risk of page text steering
  an agent; it does not remove it.", Linux only.
- `docs/agent-answer.md`: only a line the built binaries contradict, each named in the report.
- For `README.md`, handed to the coordinator as lines: the install commands for four binaries
  on Linux, and a link to the new website page.

## Acceptance

Without a tag: a manual run of the release workflow on the branch's commit succeeds, its two
Linux smoke tests included; the Linux x86_64 archive downloaded from that run lists `surf`,
`surf-agent`, `surfd` and `surf-session` (`tar -tzf`), the macOS archives `surf` only; and run
here, `SURF_BIN_DIR=<that archive, unpacked> slot cargo test --locked -p surf-session --test e2e`
passes: against a local page server it runs that archive's `surf-agent open <address>`, `go`,
`look` and `close`, each exits 0 and prints the lines `docs/agent-answer.md` gives, and
afterwards no process from that directory runs (`ps -eo args`) and nothing of the session is
left under `$XDG_RUNTIME_DIR/surf/<id>/`. The tag and the release are cut by the coordinator as `docs/releasing.md`
says.

## Out of scope

The agent binaries on macOS: a caller user id and a runtime directory that work there, backlog
draft "Run the agent binaries on macOS: a caller user id without /proc and a runtime directory
without XDG_RUNTIME_DIR". Windows. Signing and notarisation. Package managers. The tag and the
release.

Surfaces written: `.github/workflows/release.yml`, `docs/releasing.md`, `CHANGELOG.md`,
`crates/surf-cli/Cargo.toml`, `crates/agent/Cargo.toml`, `crates/surfd/Cargo.toml`,
`crates/session/Cargo.toml`, `website/docs/try-it.md`, `website/docs/drive-from-an-agent.md`,
`website/sidebars.ts`, `docs/agent-answer.md` (only if wrong). `story:agent-session-process`
writes `crates/session/` and a sentence of `docs/agent-answer.md` before it: an order, not a
collision; no other wave 4 story writes these.
