---
format: aep.planning-md/3
id: story:agent-request-body-spec
kind: story
status: active
title: Specify the request method and body from a navigation to its fetch
summary: Navigate, NavigationStarted, Fetch and RequestStarted carry a method (GET or POST), a body and its media type; the transport receives them.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-fetch-redirects
- depends_on: story:runner-surf-cli-package
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 5}
---
## Context

Step 4 of design section 9 needs `POST` and request bodies in `surf.net`. `spec/domains/net.yaml`
marks "request methods and bodies" `UNMAPPED:`, and `crates/fetch` sends `GET` only. A form
submission (`agent-forms-spec`) navigates with a method and a body.

Re-chained by decision D4: this story stands on `agent-redirect-spec` and `agent-fetch-redirects`,
not on `agent-links-spec`, and needs nothing from the node tree. It comes after
`html-document-spec` only because both write the specification files, `generated/rust/`,
`conformance/src/` and `crates/surf/src/lib.rs`.

`surf::Transport::fetch` gains the method, the body and its media type. `crates/session` calls it
through its transport (`agent-session-process`), so this story changes `crates/session` too
(parallel critic, round 1, finding 7). The binary is in `crates/surf-cli` (decision D6).

Standards, from memory, to be confirmed: WHATWG Fetch, a request has a method, a body and a header
list, and a redirect changes the method (`POST` to `GET` on 301 and 302; any method but `HEAD` to
`GET` on 303); HTML Standard form submission navigates with a POST resource.

## Domain relations

- `surf.navigation.Navigation → surf.net.Request`: one to one through one identity
  (`spec/domains/navigation.yaml`, identity `navigation_id: surf.net.RequestId`; binding
  `fetch-on-navigation-started`). Method and body travel along it.

## Change

- `spec/domains/net.yaml`: type `surf.net.Method` (`Get`, `Post`); `Fetch` and `RequestStarted`
  gain `method`, an optional `body: Bytes` and an optional `body_type: surf.net.MediaType`;
  `Request` stores its method; the marker narrowed to headers, cookies, caching and TLS; the
  redirect method rule as a comment with `refs:`.
- `spec/domains/navigation.yaml`: `Navigate` and `NavigationStarted` gain the same three.
- `spec/components.yaml`: `fetch-on-navigation-started` maps them; the agent binding to `Navigate`
  navigates with `Get` and no body.
- Authored scenarios: `spec/scenarios/navigation-post-reaches-request.yaml` (a `Navigate` with
  `Post`, a body and its media type publishes a `RequestStarted` with the same three) and
  `navigation-get-has-no-body.yaml`.
- `task generate`; `crates/surf/src/lib.rs`: `Transport::fetch` takes method, body and body type;
  `Browser::start` and `load` navigate with `Get`; `crates/surf-cli/src/main.rs`: until
  `agent-fetch-post` the transport ends a request that is not `GET` with
  `Failure(UnsupportedScheme)` and a diagnostic line; `crates/session/src/`: its transport takes
  the new arguments and passes `GET` on; `conformance/src/`.

Surfaces written: `spec/domains/net.yaml`, `spec/domains/navigation.yaml`, `spec/components.yaml`,
`spec/scenarios/`, `spec/ess-inputs.yaml`, `generated/rust/`, `crates/surf/src/lib.rs`,
`crates/surf-cli/src/main.rs`, `crates/session/src/`, `conformance/src/`. Not `Taskfile.yml`
(decision D30).

## For AGENTS.md (handed over in the unit report, decision D7)

- "Components": `Fetch` and `Navigate` carry a method and a body.
- "The gate's numbers": the new totals.

## Acceptance

`slot task check REFUSALS_MAX=<n>` exits 0 with the scenario total at least 6 above the total recorded before it
and equal on both runners with `0 failed, 0 error, 0 unsupported`, the two authored files of
Change among the passed, and the `refused:` count at most 2 above the count recorded before it,
`<n>` equal to it, the unit report giving `<n>` and each new refusal with its cause (decision
D30).

## Out of scope

Sending the body (`agent-fetch-post`); forms (`agent-forms-spec`); other methods; request headers.
