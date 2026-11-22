---
format: aep.planning-md/3
id: story:tls-spec
kind: story
status: draft
title: 'Specify https: fetching, redirects between schemes and the five TLS failure reasons'
summary: net.yaml fetches https and gains CertificateNotTrusted, CertificateExpired, CertificateNameMismatch, TlsVersionUnsupported, TlsHandshakeFailed with scenarios; the answer and surf command documents list them.
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-decisions
scope:
- confidence: cited
  path: conformance/src/target.rs
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: docs/agent-answer.md
- confidence: cited
  path: docs/surf-command.md
- confidence: cited
  path: generated/rust
- confidence: cited
  path: spec/domains/navigation.yaml
- confidence: cited
  path: spec/domains/net.yaml
- confidence: cited
  path: spec/scenarios
revision: 3
---
## Context

`spec/domains/net.yaml`: `surf.net.FailureReason` (lines 29-45) has four variants,
`Unreachable`, `UnsupportedScheme`, `MalformedResponse` and `TooManyRedirects`, the last with a
comment because the Fetch Standard gives that network error no name. The comment above
`entities:` (lines 55-76) says `http` is the only scheme fetched, that a `Location` whose scheme
is not `http:` ends the request with `UnsupportedScheme`, and ends `# UNMAPPED: request
headers, cookies, caching and TLS.` `spec/domains/navigation.yaml` lines 19-20: "one that
leaves `http:` fails it with UnsupportedScheme".

Two hand-written functions match `FailureReason` exhaustively and stop compiling once
`task generate` adds variants: `crates/surf/src/lib.rs:316-319` (the reason names) and
`conformance/src/target.rs:613-630` (`failure_reason`, `failure_reason_named`).
`docs/agent-answer.md` lines 169-170 list the reasons of `page failed`, and its grammar holds
any of them (`failure = 1*ALPHA`, line 232); `docs/surf-command.md` lines 36-38 list four.

`docs/design/tls.md` (`story:tls-decisions`): item 7 (the names and when each is given), item 8
(`--ca-file` and `--anchors` of `surf`), item 12 (TLS 1.2).

## Domain relations

- `surf.net.FailureReason` is carried unconverted from `net` to `navigation` and `agent`:
  fields of that type in `spec/domains/navigation.yaml:195` and `:301` and
  `spec/domains/agent.yaml:131`, `:176`, `:506`, `:724`, `:761` (`failure_reason`, `reason`).
  Inferable from those `ess/1` documents; no `relations:` entry is involved, and new variants
  need no change there.
- No new entity: the epic keeps TLS a port of `net` ("nothing of it is observable but the
  reason; so no new noun enters the model").

## Change

- `spec/domains/net.yaml`:
  - `FailureReason` gains the five reasons of item 7, each with a `summary` and a comment
    stating that the Fetch Standard returns a network error there and that the name lets an
    agent tell it from `Unreachable`, as the comment of `TooManyRedirects` does;
  - the comment above `entities:`: `http` and `https` are fetched, `https` over TLS 1.3 (RFC
    8446) on port 443 by default, the server authenticated against the bundled anchors and the
    extra anchors of the session; redirects between `http:` and `https:` are followed both
    ways, as the Fetch Standard follows them, under the destination rule of
    `story:security-review-fixes` (named, not designed here); every other scheme stays
    `UnsupportedScheme`; TLS 1.2 is `TlsVersionUnsupported`;
  - the marker becomes `# UNMAPPED: request headers, cookies and caching.`
- `spec/domains/navigation.yaml`, comment only: "one that leaves `http:` and `https:`".
- `spec/scenarios/page-load-fails-<reason>.yaml` (new, five), in the form of
  `page-load-fails-without-response.yaml`: `FailRequest` with that reason on a pending request
  fails the navigation and the session shows the reason. One of them also holds the hostile
  case below.
- `task generate`: `generated/rust/`.
- `crates/surf/src/lib.rs`: the reason-name function gains five arms; no other line.
  `conformance/src/target.rs`: `failure_reason` and `failure_reason_named` gain five each.
- `docs/agent-answer.md` ("Page"): the nine reasons, one line each on when it is given;
  `docs/surf-command.md`: `https:` addresses, the nine reasons, `--ca-file <path>` and
  `--anchors` with their output and exit statuses as item 8 states them (implemented by
  `story:tls-fetch-https`; documented here first).
- Behaviour does not change in this story: `surf https://127.0.0.1:1/` still ends
  `UnsupportedScheme` until `story:tls-fetch-https`.

## Acceptance

`slot task check` passes on the story branch with nine variants in `surf.net.FailureReason` and
the five new scenarios passing against the interpreter and the Rust system.

1. `slot task validate` prints `surf v1 — 9 file(s), 19 scenario(s), valid` (14 authored today
   plus 5).
2. `task conform` and `task conform-rust` print `<N> scenarios: <N> passed, 0 failed, 0 error,
   0 unsupported` for the same `<N>`, and `task conform-totals` names one suite digest; `<N>` and
   the refusal count are in the report (a refusal count other than 28 is handed to the
   coordinator for `REFUSALS_MAX`).
3. Each new scenario: the navigation is `Failed` and the session `failure_reason` is that
   reason.
4. Hostile: `FailRequest` with `CertificateExpired` on a request already `Responded` ends in
   `wrong-state` (`surf.net.RequestStateConflict`), and the navigation stays committed.
5. `slot task generated-check` prints `generated/rust matches spec/ (30 artifact(s) and
   .ess-output)`.
6. `grep -c "UNMAPPED:.*TLS" spec/domains/net.yaml` prints 0; `slot task open` prints as many
   lines as on `main`.
7. Each of the nine reasons is found by `grep -c` in `docs/agent-answer.md` and in
   `docs/surf-command.md`; `--ca-file` and `--anchors` in the second.
8. `cargo build --locked --manifest-path conformance/Cargo.toml` exits 0; `cargo test -p surf`
   passes unchanged.

Budget: no test of the root workspace is added; the scenarios run in the specification steps.
No network, no model call.

## Out of scope

Any transport code; the `open --ca-file` option and its specification
(`story:tls-session-anchors`); the destination rule.

## Writes

`spec/domains/net.yaml`, `spec/domains/navigation.yaml`, `spec/scenarios/page-load-fails-*.yaml`
(new), `generated/rust/`, `crates/surf/src/lib.rs` (one function), `conformance/src/target.rs`,
`docs/agent-answer.md`, `docs/surf-command.md`. Later in this epic `spec/`, `generated/` and
`docs/agent-answer.md` are written again by `story:tls-session-anchors` and
`crates/surf/src/lib.rs` by `story:tls-fetch-https`, both after this one. In the rest of the plan
`spec/` and `generated/` are written by every specification story (`story:agent-cookie-jar` edits
the same marker of `net.yaml`, `story:agent-go-address-parsed`): never in the same wave.
Coordinator: `REFUSALS_MAX` in the root `Taskfile.yml` if the count changes, `AGENTS.md` "The
gate numbers".
