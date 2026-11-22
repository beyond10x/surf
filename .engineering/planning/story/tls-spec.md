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
- depends_on: story:tls-test-server
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
revision: 4
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

`docs/design/tls.md` (`story:tls-decisions`): item 7 (the names, when each is given, their
detail, revocation not checked), item 8 (`--ca-file` and `--anchors` of `surf`), item 12 (a
TLS 1.2 server).

Placement: the first of the four stories built as one unit, in the order `tls-spec`,
`tls-fetch-https`, `tls-session-anchors`, `tls-release`. The unit starts at the first boundary
between two rounds of surf after the nine `crates/tls*` stories are merged (the edge to
`tls-test-server` holds it there), never beside a unit that writes `spec/`, `generated/`,
`crates/session`, `crates/agent`, `crates/surfd`, `crates/surf/src/lib.rs`,
`conformance/src/target.rs` or `docs/agent-answer.md`. In round 2, `story:agent-history-spec`
writes `spec/`, `generated/rust/`, `navigation.yaml`, `conformance/src/target.rs` and
`crates/surf/src/lib.rs`, `story:agent-form-port` the last two, and the agent command stories
`docs/agent-answer.md`: the placement keeps them apart.

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
    agent tell it from `Unreachable`, as the comment of `TooManyRedirects` does; the comment of
    `CertificateNotTrusted` adds that revocation is not checked, so a revoked certificate that
    has not expired loads; that of `TlsVersionUnsupported` that a server offering at best TLS
    1.2 ends with it;
  - the comment above `entities:`: `http` and `https` are fetched, `https` over TLS 1.3 (RFC
    8446) on port 443 by default, the server authenticated against the bundled anchors and the
    extra anchors of the session, revocation not checked; redirects between `http:` and
    `https:` are followed both ways, as the Fetch Standard follows them, under the destination
    rule of `story:security-review-fixes` (named, not designed here); every other scheme stays
    `UnsupportedScheme`;
  - the marker becomes `# UNMAPPED: request headers, cookies and caching.`
- `spec/domains/navigation.yaml`, comment only: "one that leaves `http:` and `https:`".
- `spec/scenarios/page-load-fails-<reason>.yaml` (new, five), in the form of
  `page-load-fails-without-response.yaml`: `FailRequest` with that reason on a pending request
  fails the navigation and the session shows the reason. One of them also holds the hostile
  case below.
- `task generate`: `generated/rust/`.
- `crates/surf/src/lib.rs`: the reason-name function gains five arms; no other line.
  `conformance/src/target.rs`: `failure_reason` and `failure_reason_named` gain five each.
- `docs/agent-answer.md` ("Page"): the nine reasons, one line each on when it is given, and the
  sentence "surf does not check revocation: a revoked certificate that has not expired loads.";
  `docs/surf-command.md`: `https:` addresses, the same sentence, the nine reasons,
  `--ca-file <path>` and `--anchors` with their output and exit statuses as item 8 states them
  (implemented by `story:tls-fetch-https`; documented here first).
- Behaviour does not change in this story: `surf https://127.0.0.1:1/` still ends
  `UnsupportedScheme` until `story:tls-fetch-https`, the next story of the same unit.

## Acceptance

`slot task check` passes on the story branch with nine variants in `surf.net.FailureReason` and
the five new scenarios passing against the interpreter and the Rust system.

Counts are relative to the base the unit starts from (`<s>` authored scenarios, `<N>`
scenarios in the suite and `<r>` refusals there, read from `task validate`, `task conform` and
`task suite` on that base and quoted in the report):

1. `slot task validate` prints `surf v1 — 9 file(s), <s + 5> scenario(s), valid`.
2. `task conform` and `task conform-rust` each print `<M> scenarios: <M> passed, 0 failed, 0
   error, 0 unsupported` for one `<M>` of at least `<N> + 5`, and `task conform-totals` names
   one suite digest; the refusal count is `<r>` (a change is handed to the coordinator for
   `REFUSALS_MAX`, with the new lines).
3. Each new scenario: the navigation is `Failed` and the session `failure_reason` is that
   reason.
4. Hostile: `FailRequest` with `CertificateExpired` on a request already `Responded` ends in
   `wrong-state` (`surf.net.RequestStateConflict`), and the navigation stays committed.
5. `slot task generated-check` prints that `generated/rust` matches `spec/`, with the artifact
   count of the base.
6. `grep -c "UNMAPPED:.*TLS" spec/domains/net.yaml` prints 0; `slot task open` prints as many
   lines as on the base.
7. Each of the nine reasons is found by `grep -c` in `docs/agent-answer.md` and in
   `docs/surf-command.md`; `--ca-file` and `--anchors` in the second; `grep -c "does not check
   revocation"` prints at least 1 in each of the two documents and in `net.yaml`.
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
`docs/agent-answer.md`, `docs/surf-command.md`. In the same unit, later: `spec/`, `generated/`
and `docs/agent-answer.md` by `tls-session-anchors`, `crates/surf/src/lib.rs` by
`tls-fetch-https`. Outside the epic, the same files are written by `story:agent-history-spec`,
`story:agent-form-port`, the agent command stories (`docs/agent-answer.md`),
`story:agent-cookie-jar` (the same marker of `net.yaml`) and `story:agent-go-address-parsed`:
kept apart by the placement above. Coordinator: `REFUSALS_MAX` in the root `Taskfile.yml` if the
count changes, `AGENTS.md`, the section on the numbers of the gate.
