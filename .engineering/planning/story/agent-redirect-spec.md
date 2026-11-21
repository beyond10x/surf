---
format: aep.planning-md/3
id: story:agent-redirect-spec
kind: story
status: active
title: Specify redirects inside one request and the response address
summary: surf.net delivers the address after the last redirect and fails with TooManyRedirects; the navigation commits at the response address; nothing here needs the node tree.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-page-path-spec
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 4}
---
## Context

Decision D4 of `docs/plan/agent-plan-revision-1.md` (design critic finding 7, parallel critic
finding 7): the redirect model leaves `agent-links-spec`, so that `agent-fetch-redirects`,
`agent-request-body-spec` and `agent-fetch-post` no longer wait for the node tree. This story
needs nothing from `epic:html-parser`.

`spec/domains/net.yaml` marks redirects `UNMAPPED:`; `spec/domains/navigation.yaml` says "a
redirect makes one navigation issue several requests; nothing here follows one". The epic places
redirects in `surf.net`. Decided (first in `agent-links-spec`, now here): a redirect is followed
inside one request, as the WHATWG Fetch Standard follows one inside a fetch ("HTTP-redirect
fetch", from memory, to be confirmed by this story), so one navigation keeps one request and one
identity. The HTML Standard navigates with a redirect mode of its own (from memory, to be
confirmed); where this differs, a comment beside the construct says so. The model gains a
response URL: the address after the last redirect.

`crates/surf` changes only inside `Browser::run`, which delivers the request address as the
response URL until `agent-fetch-redirects` delivers the real one. `Answer` keeps its shape, so
nothing `crates/session` calls changes here.

## Domain relations

- `surf.navigation.Navigation → surf.net.Request`: one to one through one identity
  (`spec/domains/navigation.yaml`, identity `navigation_id: surf.net.RequestId`; binding
  `fetch-on-navigation-started`, `request_id: event.navigation_id`). It stays one to one.

## Change

- `spec/domains/net.yaml`: `DeliverResponse` and `ResponseReceived` carry the response `url`; a
  failure reason `TooManyRedirects`; the redirect marker replaced by the decision with `refs:`,
  and what the transport does stated beside it: 301, 302, 303, 307, 308 followed, at most 20, a
  `Location` that is not `http:` ends the request `UnsupportedScheme`, a 3xx without `Location`
  is a response.
- `spec/domains/navigation.yaml`: `CommitNavigation` takes the response `url`;
  `NavigationCommitted` carries it; the redirect marker replaced.
- `spec/components.yaml`: `commit-on-response` maps the response URL; the agent note binding copies
  it to the session; the document is parsed at it.
- Authored scenarios: `spec/scenarios/navigation-commits-at-response-url.yaml` (a response at
  another URL commits there and the document has that URL) and
  `agent-session-shows-response-url.yaml`.
- `task generate`; `crates/surf/src/lib.rs` (`Browser::run` only); `conformance/src/`.

Surfaces written: `spec/domains/net.yaml`, `spec/domains/navigation.yaml`, `spec/components.yaml`,
`spec/scenarios/`, `spec/ess-inputs.yaml`, `generated/rust/`, `crates/surf/src/lib.rs`,
`conformance/src/`. Not `Taskfile.yml` (decision D30); no binding is added and the `refused:` count does not
rise. `epic:html-parser`: `html-document-spec` writes `spec/components.yaml`, `generated/rust/`,
`conformance/src/` and `crates/surf/src/lib.rs` too and must come after this story.

## For AGENTS.md (handed over in the unit report, decision D7)

- "Components": the page-load paragraph says a response names the address it came from and the
  navigation commits there.
- "The gate's numbers": the new totals.

## Acceptance

`slot task check REFUSALS_MAX=<n>` exits 0 with the scenario total at least 4 above the total recorded before it
and equal on both runners with `0 failed, 0 error, 0 unsupported`, the two authored files of
Change and a synthesized `FailRequest` scenario with `TooManyRedirects` among the passed, and the
`refused:` count unchanged, `<n>` the count recorded before it (decision D30).

## Out of scope

Following redirects on the wire (`agent-fetch-redirects`); the method change after `POST`
(`agent-request-body-spec`); `Follow` (`agent-links-spec`).
