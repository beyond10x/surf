---
format: aep.planning-md/3
id: story:tls-session-anchors
kind: story
status: draft
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
  path: crates/agent
- confidence: cited
  path: crates/session
- confidence: cited
  path: crates/surfd
- confidence: cited
  path: docs/agent-answer.md
- confidence: cited
  path: generated/rust
- confidence: cited
  path: spec/domains/agent.yaml
revision: 3
---
## Context

Split from the candidate `tls-fetch-https` of `epic:tls-https`: that story makes `surf-fetch`
and the `surf` command load `https:` addresses; this one gives a session of `surf-agent` its
extra anchors, which travel from the client through `surfd` to the session process. The two
touch different crates and are checked apart (`surf` against `surf-agent`); together they were
too large for one unit.

U2 (`epic:tls-https`, "Decisions taken"): "plus an extra anchor file per session".
`docs/design/tls.md` item 8: `surf-agent open --ca-file <path> [<url>]`; the client reads the
file and its bytes travel in the `open` request, because a `surfd` started by an earlier `open`
does not see the environment of a later caller; at most 32 KiB and 64 certificates, every one a
CA, so that the request stays within `surf_agent::wire::REQUEST_MAX` (64 KiB,
`crates/agent/src/wire.rs:86`). The input of `open` is the struct `surf.agent.OpenCall`
(`spec/domains/agent.yaml:73-79`, field `url` only), bound by `cli/surf-agent.yaml` (`open`,
lines 37-40); the client parser is held to that document (`task cli`). The session transport
is `surf_session::fetch` (`crates/session/src/lib.rs:137`) over `surf::fetch_http`, which takes
the trust to use since `story:tls-fetch-https`. The end-to-end test of the three binaries is
`crates/session/tests/e2e.rs` (`SURF_BIN_DIR`).

The epic, "Acceptance": "the end-to-end test `surf-agent go https://localhost:<port>/` against
the test server prints `page 200 "text/html" "https://localhost:<port>/"`".

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
- `crates/agent` (the client): reads the file before it connects, refuses it with the
  `AnchorError` of `surf-tls` as an invalid input (the exit status and answer
  `docs/agent-answer.md` gives for one), and sends its bytes in the `open` request.
- `crates/surfd`: hands the bytes to the session process it starts, over its link, never on a
  command line.
- `crates/session`: builds the trust from the bundled anchors and the bytes, and fetches with it
  for the life of the session; an `open` without the option uses the bundled anchors.
- `docs/agent-answer.md`: `open --ca-file` and the answer to a refused file.
- Tests in `crates/session/tests/main.rs` and `crates/agent/tests/main.rs`, as modules (both
  crates have several files today; `story:one-test-binary-per-crate` is before this one).

## Acceptance

The end-to-end test runs `surf-agent open --ca-file <root.pem>` and `surf-agent go
https://localhost:<port>/` against `surf-tls-test-server` and prints `page 200 "text/html"
"https://localhost:<port>/"`, and a session opened without the file prints `page failed
CertificateNotTrusted "https://localhost:<port>/"`.

Each on the built `surf-agent`, `surfd` and `surf-session` (or those of `SURF_BIN_DIR`) in a
private `XDG_RUNTIME_DIR`, the server on 127.0.0.1 port 0 with the certificates of
`crates/tls-test-certs/certs/`:

1. `open --ca-file <ok-ecdsa-p256 root.pem>`, then `go https://localhost:<port>/` (the server
   answers 200 `text/html`): the answer holds `page 200 "text/html"
   "https://localhost:<port>/"`.
2. A session opened without `--ca-file`: `go` gives `page failed CertificateNotTrusted
   "https://localhost:<port>/"`.
3. Two sessions of one `surfd`, A with the file and B without: A loads, B fails
   `CertificateNotTrusted` (the anchors belong to the session, not to the daemon).
4. With the file, the server of the case `expired`: `page failed CertificateExpired ...`; mode
   `Tls12Only`: `page failed TlsVersionUnsupported ...`.
5. Hostile: `--ca-file` naming a missing file, a file of 32 KiB plus one byte, a file holding a
   leaf that is no CA, a file holding a `PRIVATE KEY` block: each refused before any connection
   to `surfd`, with the exit status of an invalid input, and no session is created
   (`$XDG_RUNTIME_DIR/surf/` holds none).
6. The session log of case 2 holds a `surf.net.FetchFailed` entry with the reason
   `CertificateNotTrusted`.
7. `slot task cli` passes with the new option, and `slot task generated-check` prints that
   `generated/rust` matches `spec/`.

Budget: each test under 10 s and all of them under 15 s together (times in the report); no test
over 60 s, the test step within 120 s; 127.0.0.1 only, no network, no model call.

## Out of scope

Anchors per `go` or per daemon; trust settings beyond one extra file (pinning, allow-lists);
`surf-agent` on macOS. Task 5 of `story:agent-tasks` (it needs that story and the page text of
round 1, neither of this epic): it can use `open --ca-file` and the test server once they exist.

## Writes

`spec/domains/agent.yaml`, `cli/surf-agent.yaml`, `generated/rust/`, `crates/agent/`,
`crates/surfd/`, `crates/session/`, `docs/agent-answer.md`. `spec/`, `generated/` and
`docs/agent-answer.md` are written by `story:tls-spec` before this one. In the rest of the plan
the agent crates, `spec/` and `docs/agent-answer.md` are written by many agent stories
(`story:agent-cookie-jar`, `story:agent-fill-pick`, `story:agent-answer-format` and others):
never in the same wave. Coordinator: `AGENTS.md`, `README.md`, the website pages.
