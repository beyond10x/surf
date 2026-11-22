---
format: aep.planning-md/3
id: story:tls-session-anchors
kind: story
status: active
title: 'surf-agent open --ca-file gives a session its extra anchors, and surf-agent go loads https: pages'
summary: 'OpenCall and the CLI gain --ca-file; the client sends the file bytes through surfd to the session, which fetches https: with them; end-to-end against the test server.'
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-decisions
- depends_on: story:tls-spec
- depends_on: story:tls-fetch-https
- depends_on: story:one-test-binary-per-crate
scope:
- confidence: cited
  path: cli/surf-agent.yaml
- confidence: cited
  path: crates/agent/src/cli.rs
- confidence: cited
  path: crates/agent/src/client.rs
- confidence: cited
  path: crates/agent/src/wire.rs
- confidence: cited
  path: crates/agent/tests
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: cited
  path: crates/session/src/process.rs
- confidence: cited
  path: crates/session/tests
- confidence: cited
  path: crates/surfd/src/daemon.rs
- confidence: cited
  path: crates/surfd/src/link.rs
- confidence: cited
  path: docs/agent-answer.md
- confidence: cited
  path: generated/rust
- confidence: cited
  path: spec/domains/agent.yaml
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:24:30Z", actor: "human:operator", revision: 10, decided_on: {"recorded":{"verification":1}}}
- {from: "proposed", to: "active", at: "2026-11-22T06:24:30Z", actor: "human:operator", revision: 11, decided_on: {"recorded":{"verification":1}}}
---
## Context

Split from the candidate `tls-fetch-https` of `epic:tls-https`: that story makes `surf-fetch`
and the `surf` command load `https:` addresses; this one gives a session of `surf-agent` its
extra anchors, which travel from the client through `surfd` to the session process. The two
touch different files and are checked apart (`surf` against `surf-agent`); together they were
too large for one unit of work.

U2 (`epic:tls-https`, "Decisions taken"): "plus an extra anchor file per session".
`docs/design/tls.md` item 8: `surf-agent open --ca-file <path> [<url>]`; the client reads the
file and its bytes travel in the `open` request, because a `surfd` started by an earlier `open`
does not see the environment of a later caller; at most 32 KiB and 64 certificates, every one a
CA, so that the request stays within `surf_agent::wire::REQUEST_MAX` (64 KiB,
`crates/agent/src/wire.rs:86`). The input of `open` is the struct `surf.agent.OpenCall`
(`spec/domains/agent.yaml:73-79`, field `url` only), bound by `cli/surf-agent.yaml` (`open`,
lines 37-40); the client parser is held to that document (`task cli`). The session transport
is `surf_session::fetch` (`crates/session/src/lib.rs:137`) over `surf::fetch_http`, which takes
the anchors to use since `story:tls-fetch-https`. `surfd` starts the session process in
`crates/surfd/src/daemon.rs` and talks to it over the link of `crates/surfd/src/link.rs`. The
end-to-end test of the three binaries is `crates/session/tests/e2e.rs` (`SURF_BIN_DIR`).

The epic, "Acceptance": "the end-to-end test `surf-agent go https://localhost:<port>/` against
the test server prints `page 200 "text/html" "https://localhost:<port>/"`; one test per reason
of the negative suite prints `page failed <reason>`".

Placement: the third of the four stories built as one unit (`tls-spec`, `tls-fetch-https`,
`tls-session-anchors`, `tls-release`), at a boundary between two rounds of surf, never beside a
unit that writes `spec/`, `generated/`, `crates/fetch`, `crates/session`, `crates/agent`,
`crates/surfd` or `docs/agent-answer.md` (the path rule of `story:tls-decisions`).

## Domain relations

- Session to extra anchors: zero or one set per session, owned by the session, read once at
  `open` and dropped with the session; it never outlives it and is never shared with another
  session of the same `surfd`. No `ess/1` document declares it, and the epic keeps it out of the
  model ("TLS is a port of `net` ... no new noun enters the model"). Decided by
  `epic:tls-https`, "Decisions taken", U2 (the coordinating agent, 2026-11-22, reported to the
  operator) and `docs/design/tls.md` item 8; `OpenCall` is a command-line shape, not an entity
  (`spec/domains/agent.yaml:66-72`).

## Change

- `spec/domains/agent.yaml`: `surf.agent.OpenCall` gains `ca_file: Optional<String>`, with a
  comment that the client reads the file and sends its bytes; `task generate`
  (`generated/rust/`). `cli/surf-agent.yaml`: `open` gains `--ca-file <path>`; `task cli`.
- `crates/agent/src/cli.rs` and `src/client.rs`: the client reads the file before it connects,
  refuses it with the `AnchorError` of `surf-tls` as an invalid input (the exit status and
  answer `docs/agent-answer.md` gives for one); `crates/agent/src/wire.rs`: the bytes travel in
  the `open` request.
- `crates/surfd/src/daemon.rs` and `src/link.rs`: `surfd` hands the bytes to the session
  process it starts over its link, never as an argument or an environment variable.
- `crates/session/src/lib.rs` and `src/process.rs`: the session builds its anchors from the
  bundled set and the bytes, and fetches with them for its life; an `open` without the option
  uses the bundled set.
- `docs/agent-answer.md`: `open --ca-file` and the answer to a refused file.
- Tests: `crates/session/tests/main.rs` module `https` and `crates/agent/tests/main.rs` module
  `ca_file` (both crates have one test binary since `story:one-test-binary-per-crate`).

## Acceptance

The end-to-end test runs `surf-agent open --ca-file <root.pem>` and `surf-agent go
https://localhost:<port>/` against `surf-tls-test-server` and prints `page 200 "text/html"
"https://localhost:<port>/"`, and one test per reason of the negative suite prints `page failed
<reason> "https://localhost:<port>/"`.

Each on the built `surf-agent`, `surfd` and `surf-session` (or those of `SURF_BIN_DIR`) in a
private `XDG_RUNTIME_DIR`, the server on 127.0.0.1 port 0 with the certificates of
`crates/tls-test-certs/certs/`, on the system clock (the certificates run to 2125, `expired`
ended in 2001):

1. `open --ca-file <ok-ecdsa-p256 root.pem>`, then `go https://localhost:<port>/` (the server
   answers 200 `text/html`): the answer holds `page 200 "text/html"
   "https://localhost:<port>/"`.
2. A session opened without `--ca-file`: `page failed CertificateNotTrusted
   "https://localhost:<port>/"`.
3. With the root of each case as the file: `expired`: `page failed CertificateExpired ...`;
   `wrong-host`: `page failed CertificateNameMismatch ...`; modes
   `Tls12ReadsSupportedVersions` (kind (a) of item 7) and `SelectsTls12` (kind (c)): `page
   failed TlsVersionUnsupported ...`; modes `Tls12IgnoresSupportedVersions` (kind (b)) and
   `BadFinished`: `page failed TlsHandshakeFailed ...`.
4. Two sessions of one `surfd`, A with the file and B without: A loads, B fails
   `CertificateNotTrusted` (the anchors belong to the session, not to the daemon).
5. Hostile: `--ca-file` naming a missing file, a file of 32 KiB plus one byte, a file holding a
   leaf that is no CA, a file holding a `PRIVATE KEY` block: each refused before any connection
   to `surfd`, with the exit status of an invalid input, and no session is created
   (`$XDG_RUNTIME_DIR/surf/` holds none).
6. While the session of case 1 runs, `/proc/<pid>/cmdline` and `/proc/<pid>/environ` of its
   `surf-session` process hold no byte sequence of the PEM file (its first 64 base64 characters
   searched).
7. The session log of case 2 holds a `surf.net.FetchFailed` entry with the reason
   `CertificateNotTrusted`.
8. `slot task cli` passes with the new option, and `slot task generated-check` prints that
   `generated/rust` matches `spec/`.

Budget: each test under 10 s and all of them under 20 s together (times in the report); no test
over 60 s, the test step within 120 s; 127.0.0.1 only, no network, no model call.

## Out of scope

Anchors per `go` or per daemon; trust settings beyond one extra file (pinning, allow-lists);
`surf-agent` on macOS; the `https` agent task, which `story:agent-tasks-https` owns (it depends on
this story and uses `open --ca-file` and the test server).

## Writes

`spec/domains/agent.yaml`, `cli/surf-agent.yaml`, `generated/rust/`, `crates/agent/src/cli.rs`,
`crates/agent/src/client.rs`, `crates/agent/src/wire.rs`, the module `ca_file` of the test
binary of `crates/agent`, `crates/surfd/src/daemon.rs`, `crates/surfd/src/link.rs`,
`crates/session/src/lib.rs`, `crates/session/src/process.rs`, the module `https` of the test
binary of `crates/session`, the `mod` lines of the two `tests/main.rs`, `docs/agent-answer.md`.
In the same unit before it: `spec/`, `generated/`,
`docs/agent-answer.md` (`tls-spec`) and `crates/session/src/lib.rs` (`tls-fetch-https`). Outside
the epic the same files are written by many agent and view stories (`story:agent-cookie-jar`,
`story:agent-fill-pick`, `story:agent-answer-format`, `story:view-now-panel` on `surfd`): kept
apart by the placement. Coordinator: `AGENTS.md`, `README.md`, the website pages.
