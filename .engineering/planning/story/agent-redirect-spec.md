---
format: aep.planning-md/3
id: story:agent-redirect-spec
kind: story
status: implemented
title: Specify redirects inside one request and the response address
summary: surf.net delivers the address after the last redirect and fails with TooManyRedirects; the navigation commits at the response address; nothing here needs the node tree.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-page-path-spec
scope:
- confidence: cited
  path: conformance/src
- confidence: cited
  path: crates/ports/src/ports.rs
- confidence: cited
  path: crates/ports/tests/agent.rs
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: crates/surf/tests/review1_library.rs
- confidence: cited
  path: generated/rust
- confidence: cited
  path: spec/components.yaml
- confidence: cited
  path: spec/domains/agent.yaml
- confidence: cited
  path: spec/domains/navigation.yaml
- confidence: cited
  path: spec/domains/net.yaml
- confidence: cited
  path: spec/ess-inputs.yaml
- confidence: cited
  path: spec/scenarios
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-21T04:50:54Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

Decision D4 of `docs/plan/agent-plan-revision-1.md`: the redirect model leaves `agent-links-spec`,
so that `agent-fetch-redirects`, `agent-request-body-spec` and `agent-fetch-post` no longer wait
for the node tree. This story needs nothing from `epic:html-parser`.

`spec/domains/net.yaml:42` marks redirects `UNMAPPED:` (with methods, bodies, headers, cookies,
caching and TLS); `spec/domains/navigation.yaml:15` says "a redirect makes one navigation issue
several requests; nothing here follows one". Decided: a redirect is followed inside one request,
as the WHATWG Fetch Standard follows one inside a fetch ("HTTP-redirect fetch"), so one
navigation keeps one request and one identity. The HTML Standard navigates with redirect mode
"manual" and follows each hop itself, and hands a redirect to a scheme it does not fetch on
instead of failing it; a comment beside the construct says so. The model gains a response URL:
the address after the last redirect. The draft's author checked these against the published
standards (the five redirect statuses, at most 20 redirects, a non-HTTP(S) scheme is a network
error, a 3xx without `Location` is returned as the response, the response URL is the last URL,
the document is created at the response URL).

Today `ResponseReceived.url` and `NavigationCommitted.url` are the stored address of the request
and of the navigation (`{subject: url}`, `net.yaml:149`, `navigation.yaml:140`), and
`note-on-navigation-committed` maps no address (`spec/components.yaml:214`), so the session's
`url` is the address asked for (`NoteNavigationStarted`).

### Measured on a draft of the model

The change below was drafted in a scratch copy of `spec/`, `Taskfile.yml`, `taskfiles/` and
`cli/` on 2026-11-21 with ess 0.96.0; the coordinator hands the draft to the unit.

| | base (this branch) | draft |
|---|---|---|
| `slot task validate` | `surf v1 — 9 file(s), 7 scenario(s), valid` | `surf v1 — 9 file(s), 9 scenario(s), valid` |
| `slot task conform REFUSALS_MAX=27`, synthesis | `81 scenario(s) (7 authored), 27 refusal(s)` | `83 scenario(s) (9 authored), 27 refusal(s)` |
| the same, interpreter | `81 scenarios: 81 passed, 0 failed, 0 error, 0 unsupported` | `83 scenarios: 83 passed, 0 failed, 0 error, 0 unsupported` |
| `grep -c '^refused:' .out/synthesize.log` | 27 | 27, the same lines (no refusal new, none gone) |
| `ess generate synthesize --strict-requires --path spec --target rust --out <dir>` | `151 capabilities: 143 generated, 4 obligation(s), 4 refused`, `30 artifact(s)` | the same two lines, no `ESS-` error |

The 27 refusals: 22 `ESS-SYNTH-010` (10 delivery, 10 on-failure, 2 flow), 4 `ESS-SYNTH-001`
(the four `Note*/noted`), 1 `ESS-SYNTH-014` (`RecordAction/recorded`). The total rises by 2, the
two authored files: synthesis adds no scenario for a new input or a new enum variant. It takes
the first variant of an enum for a value it sends, so the synthesized
`surf.net.FailRequest/outcome/failed` sends `Unreachable`; `TooManyRedirects` appears in the
suite only in the `variants` lists of shapes (18 times).

## Domain relations

- `surf.navigation.Navigation → surf.net.Request`: one to one through one identity
  (`spec/domains/navigation.yaml`, identity `navigation_id: surf.net.RequestId`; binding
  `fetch-on-navigation-started`, `request_id: event.navigation_id`, `spec/components.yaml:127`). It
  stays one to one.

## Change

- `spec/domains/net.yaml`: the failure reason `TooManyRedirects`; `DeliverResponse` takes the
  response `url` (`surf.url.Url`), with `refs:` to Fetch "HTTP-redirect fetch" and the response
  URL; `ResponseReceived.url` is that input (it was the request's stored address); `FailRequest`
  `failed` with `refs:` to "HTTP-redirect fetch". The redirect marker is replaced by the decision
  and what the transport does: 301, 302, 303, 307, 308 followed inside one request, at most 20,
  a `Location` that is not `http:` ends the request `UnsupportedScheme` (Fetch also follows
  `https:`; a comment says so), a 3xx without `Location` is a response. Two markers remain there:
  a `Location` that does not parse as a URL (Fetch: a network error; which reason is open until
  `agent-fetch-redirects`), and methods, bodies (the method change after `POST` included),
  headers, cookies, caching and TLS. `Request.url` stays the address asked for.
- `spec/domains/navigation.yaml`: `CommitNavigation` takes the response `url`, with `refs:` to
  the HTML Standard; `NavigationCommitted.url` is that input; the marker at line 15 replaced, with
  the comment on HTML's own redirect handling. `CommitNavigation` `committed` also sets the
  navigation's `url` to the response address (`sets: url: input.url` beside `moves:`): after
  redirects the document's address is the final response's (the HTML Standard commits the
  navigation with the final response URL), so `Navigations`, `NavigationCommitted`, `Documents`
  and the session's `page` line all show it. The address that was asked for stays visible as the
  `go` action's argument in the session's path, and as the request's address in `Requests`.
- `spec/domains/agent.yaml`: `NoteNavigationCommitted` takes `url` (required, as
  `NoteNavigationStarted` does) and sets the session's `url`, so `look` shows the response
  address.
- `spec/components.yaml`: `commit-on-response` and `note-on-navigation-committed` map `url:
  event.url`; `parse-on-navigation-committed` keeps `url: event.url`, which is now the response
  address (its summary and comment say so). No binding is added.
- Authored scenarios: `spec/scenarios/navigation-commits-at-response-url.yaml` (a response at
  another address commits there: `Requests.url` the address asked for, `Navigations.url` and
  `Documents.url` the response's) and `spec/scenarios/agent-session-shows-response-url.yaml`, both listed in
  `spec/ess-inputs.yaml`. The four authored files with `DeliverResponse` steps
  (`page-load-commits-and-parses.yaml`, `two-of-each-entity.yaml`,
  `agent-session-reads-committed-navigation.yaml`, `agent-session-shows-failure-reason.yaml`)
  gain `url:` on each of their five such steps, the request's own address: without it validate
  refuses each with `ESS-AUTHOR-014`.
- `task generate`. The generated types that change: `FailureReason` gains `TooManyRedirects`;
  `DeliverResponse`, `CommitNavigation` and `NoteNavigationCommitted` gain `url: Url`;
  `ResponseReceived` and `NavigationCommitted` keep their fields (their `url` now from the input).
- The hand-written code those changes reach, each found with `grep -rn` on this branch:
  - `crates/surf/src/lib.rs`: `Browser::run` (`answer`, line 528) delivers the request's own
    address as the response `url` until `agent-fetch-redirects`; `failure_reason` (line 254)
    gains the arm `TooManyRedirects`. `Answer` and `Transport` keep their shape.
  - `crates/ports/src/ports.rs`: the hand-written `NoteNavigationCommittedBehavior` (line 413)
    also sets `session.url`; `crates/ports/tests/agent.rs` (lines 260, 284): the literals gain
    `url`.
  - `crates/surf/tests/review1_library.rs:54`: the `DeliverResponse` literal gains `url`.
  - `conformance/src/target.rs`: `failure_reason` and `failure_reason_named` (lines 557 to 570)
    gain `TooManyRedirects`; the `NoteNavigationCommitted` (924), `CommitNavigation` (1113,
    1744) and `DeliverResponse` (1188) literals gain `url`, read from the suite's named values.
  `surf-fetch` returns no `TooManyRedirects` yet, so `docs/surf-command.md` (the `<reason>` list
  of what `surf` prints) is not changed here; `agent-fetch-redirects` adds it.

Surfaces written: `spec/domains/net.yaml`, `spec/domains/navigation.yaml`,
`spec/domains/agent.yaml`, `spec/components.yaml`, `spec/scenarios/`, `spec/ess-inputs.yaml`,
`generated/rust/`, `crates/surf/src/lib.rs`, `crates/surf/tests/review1_library.rs`,
`crates/ports/src/ports.rs`, `crates/ports/tests/agent.rs`, `conformance/src/`. No other wave 4
story writes any of them. Not `Taskfile.yml` (decision D30); the `refused:` count does not rise.
`crates/session` (`agent-session-process`, same wave) constructs none of the changed types and
writes the failure reason without a `match`, so either may merge first.
`epic:html-parser`: `html-document-spec` writes `spec/components.yaml`, `generated/rust/`,
`conformance/src/` and `crates/surf/src/lib.rs` too and comes after this story (its
`depends_on`).

## For AGENTS.md (handed over in the unit report, decision D7)

- "Components": the page-load paragraph says a response names the address it came from, the
  navigation commits there, the document is parsed at it and the session shows it.
- "Layout": `spec/scenarios/` holds nine authored files; the two new ones named.
- "The gate's numbers": the new totals, and `REFUSALS_MAX` unchanged at 27.

## Acceptance

`slot task check REFUSALS_MAX=27` exits 0 and prints `surf v1 — 9 file(s), 9 scenario(s),
valid` and `83 scenario(s) (9 authored), 27 refusal(s)` (base: 7 scenario files, `81
scenario(s) (7 authored), 27 refusal(s)`); against the interpreter and against the Rust system
each `83 scenarios: 83 passed, 0 failed, 0 error, 0 unsupported`, the two new authored files
among the passed; `conform-totals` reports 83 for both; `generated-check` reports `30
artifact(s)`; `grep '^refused:' .out/synthesize.log` prints the same 27 lines as on the base;
and `jq -r '.scenarios["surf.net.FailRequest/outcome/failed"].steps[] |
select(.step=="expect_event_values") | .shape.reason.variants | join(",")' .out/suite.json`
prints `Unreachable,UnsupportedScheme,MalformedResponse,TooManyRedirects`. A different count is
a finding to report, not a number to reach by adding scenarios.

## Out of scope

Following redirects on the wire and the reason for a `Location` that does not parse
(`agent-fetch-redirects`); the method change after `POST` (`agent-request-body-spec`); `Follow`
(`agent-links-spec`).

## Revised 2026-11-21, before wave 4

- Acceptance: absolute numbers measured on a draft (83 scenarios, 9 authored, 27 refusals;
  base 81, 7, 27), replacing "at least 4 above": the two authored files add 2 and synthesis adds
  none.
- "A synthesized `FailRequest` scenario with `TooManyRedirects` among the passed" replaced: no
  synthesized scenario sends it (synthesis takes the first variant); the acceptance checks the
  variant in the scenario's shape instead.
- `spec/domains/agent.yaml` is written (`NoteNavigationCommitted` gains `url`): the binding
  cannot map into an input that does not exist.
- "`crates/surf/src/lib.rs` (`Browser::run` only)" corrected: `failure_reason` gains an arm.
  Added `crates/ports/src/ports.rs` (the hand-written note obligation), and the test literals in
  `crates/ports/tests/agent.rs` and `crates/surf/tests/review1_library.rs`, which would not
  compile.
- Four existing authored files gain `url:` on their `DeliverResponse` steps (`ESS-AUTHOR-014`).
- The navigation's stored `url` becomes the response address at commit (decided by the
  coordinator, measured: the same 83, 9, 27 and Rust output as the variant that kept it); the
  request's stays the address asked for, and the path keeps it as the `go` argument.
- The standards' wording, "from memory, to be confirmed", was checked by the draft's author;
  the HTML deviation is named.
- The marker for an unparsable `Location` is new; the old marker keeps methods to TLS.
