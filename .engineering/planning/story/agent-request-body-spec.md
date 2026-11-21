---
format: aep.planning-md/3
id: story:agent-request-body-spec
kind: story
status: implemented
title: Specify the request method and body from a navigation to its fetch
summary: Navigate, NavigationStarted, Fetch and RequestStarted carry a method (GET or POST), a body and its media type; the transport receives them.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-fetch-redirects
- depends_on: story:runner-surf-cli-package
scope:
- confidence: cited
  path: conformance/src/target.rs
- confidence: cited
  path: conformance/src/values.rs
- confidence: cited
  path: crates/ports/src/lib.rs
- confidence: cited
  path: crates/ports/tests/ports.rs
- confidence: cited
  path: crates/session/src/entries.rs
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: cited
  path: crates/session/tests/core.rs
- confidence: cited
  path: crates/session/tests/events.rs
- confidence: cited
  path: crates/surf-cli/src/main.rs
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: crates/surf/tests/forget_delivered.rs
- confidence: cited
  path: crates/surf/tests/held.rs
- confidence: cited
  path: crates/surf/tests/library.rs
- confidence: cited
  path: crates/surf/tests/request_method.rs
- confidence: cited
  path: crates/surf/tests/review1_library.rs
- confidence: cited
  path: generated/rust/
- confidence: cited
  path: spec/components.yaml
- confidence: cited
  path: spec/domains/navigation.yaml
- confidence: cited
  path: spec/domains/net.yaml
- confidence: cited
  path: spec/ess-inputs.yaml
- confidence: cited
  path: spec/scenarios/agent-session-reads-committed-navigation.yaml
- confidence: cited
  path: spec/scenarios/agent-session-shows-failure-reason.yaml
- confidence: cited
  path: spec/scenarios/agent-session-shows-response-url.yaml
- confidence: cited
  path: spec/scenarios/navigation-commits-at-response-url.yaml
- confidence: cited
  path: spec/scenarios/navigation-get-has-no-body.yaml
- confidence: cited
  path: spec/scenarios/navigation-post-reaches-request.yaml
- confidence: cited
  path: spec/scenarios/page-load-commits-and-parses.yaml
- confidence: cited
  path: spec/scenarios/page-load-fails-without-response.yaml
- confidence: cited
  path: spec/scenarios/two-of-each-entity.yaml
revision: 36
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-21T18:58:48Z", actor: "human:operator", revision: 36, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

Step 4 of design section 9 needs `POST` and request bodies in `surf.net`. `spec/domains/net.yaml`
marks "request methods and bodies" `UNMAPPED:` (the comment above `entities:`), and `crates/fetch`
sends `GET` only. A form submission (`agent-forms-spec`) navigates with a method and a body.

Re-chained by decision D4: this story stands on `agent-redirect-spec` and `agent-fetch-redirects`,
not on `agent-links-spec`, and needs nothing from the node tree. It comes after
`html-document-spec` only because both write the specification files, `generated/rust/`,
`conformance/src/` and `crates/surf/src/lib.rs`.

`surf::Transport::fetch` gains the method, the body and its media type. `crates/session` calls it
through its transport (`agent-session-process`), so this story changes `crates/session` too
(parallel critic, round 1, finding 7). The binary is in `crates/surf-cli` (decision D6). Under
decision D27 a story that adds an event field maps it in `crates/session` with a named case: the
three new fields of `NavigationStarted` and `RequestStarted` are mapped here.

Standards, to be confirmed by the unit in the published text: WHATWG Fetch, a request has a
method, a body and a header list; "HTTP-redirect fetch" sets the method to `GET` and the body to
null when the status is 301 or 302 and the method is `POST`, or the status is 303 and the method
is neither `GET` nor `HEAD`. HTML Standard form submission navigates with a POST resource (a
request body and its content type).

Measured on `main` at `01547cd` with ess 0.96.0, on a draft of the change in a scratch copy of
`spec/` (`ess specify validate`, `ess verify conform synthesize`, `ess verify conform run
--target interpreted`, `ess generate synthesize --target rust`, the generated workspace checked
with warnings denied):

- The change adds no synthesized scenario and no refusal: 93 synthesized and authored scenarios
  become 95 with the two authored files below, the refusals stay 28 (the same 28 lines), the Rust
  generation reports `168 capabilities: 160 generated, 4 obligation(s), 4 refused` (was 167 and
  159) and still writes 30 artifacts.
- A required `method` on `Navigate` makes every authored step that issues `Navigate` refused by
  validation (`ESS-AUTHOR-014`, "requires `method`, and nothing supplies it"): 9 steps in 7 files.
  Each gets `method: Get`.
- Without an `example:` on the optional `body` of `Navigate`, the synthesized scenario
  `fetch-on-navigation-started/binding/mapping` errors on the interpreter
  (`ESS-CF-SUITE: surf.navigation.NavigationStarted carried no field body`): synthesis sends no
  optional input, and an absent field cannot be followed into the mapping. With an `example:` on
  each optional input of `Navigate` and `Fetch`, all 95 pass.
- An authored step claims only the events of its own command: a `surf.net.RequestStarted` claimed
  under a `Navigate` step fails (`event surf.net.RequestStarted published` expected, only
  `NavigationStarted` observed). An absent optional field cannot be claimed `null` in an event
  payload on the interpreter ("body was not carried"); in a view row it can. So the request's
  method and media type are stored and shown by `Requests`, and the body travels only in events
  and invocations (a row is kept for the browser's life; `Browser::forget_delivered` does not
  touch rows, `crates/surf/src/lib.rs:495`).
- The generated binding `navigate-on-navigation-requested` with `method: Get` fills
  `method: surf_types::net::Method::Get, body: None, body_type: None`.

## Domain relations

- `surf.navigation.Navigation → surf.net.Request`: one to one through one identity
  (`spec/domains/navigation.yaml`, identity `navigation_id: surf.net.RequestId`; binding
  `fetch-on-navigation-started`). Method, body and media type travel along it in
  `NavigationStarted` and the `Fetch` the binding invokes. The `Navigation` row does not store the
  method; `agent-history-spec`, which needs it to recognise a `POST` entry, adds it there.

## Change

- `spec/domains/net.yaml`:
  - type `surf.net.Method`, an enum with the variants `Get` and `Post`;
  - `Fetch` gains the inputs `method: surf.net.Method`, `body: Optional<Bytes>` (example
    `"YT0x"`) and `body_type: Optional<surf.net.MediaType>` (example
    `"application/x-www-form-urlencoded"`), and `refs: ["whatwg-fetch:concept-request-method",
    "whatwg-fetch:concept-request-body", "whatwg-fetch:http-redirect-fetch"]`; its outcome
    `started` sets `method` and `body_type` on the request and its `RequestStarted` carries all
    three;
  - `Request` gains the fields `method` and `body_type: Optional<surf.net.MediaType>`; the view
    `Requests` shows both, between `url` and `state`; the body is not stored;
  - the comment above `entities:` states the redirect method rule of Context (301 and 302 after
    `POST`, 303 after any method continue as `GET` without a body; 307 and 308 keep both), and the
    marker reads `UNMAPPED: request headers, cookies, caching and TLS.`
- `spec/domains/navigation.yaml`: `Navigate` gains the same three inputs, with the same examples,
  and `NavigationStarted` the same three fields, passed from the input.
- `spec/components.yaml`: `fetch-on-navigation-started` maps `method`, `body` and `body_type` from
  the event; `navigate-on-navigation-requested` maps `method: Get` and maps no body.
- `spec/scenarios/`: the 9 `Navigate` steps of `page-load-commits-and-parses`,
  `page-load-fails-without-response`, `two-of-each-entity` (2), `navigation-commits-at-response-url`,
  `agent-session-reads-committed-navigation`, `agent-session-shows-failure-reason` (2) and
  `agent-session-shows-response-url` gain `method: Get`. Two new files, listed in
  `spec/ess-inputs.yaml`, domain `surf.navigation`:
  - `navigation-post-reaches-request.yaml`: a `Navigate` with `Post`, the body `YT0x` and
    `application/x-www-form-urlencoded`; its `NavigationStarted` carries the three, and `Requests`
    holds the request with `method: Post`, that media type and `state: Pending`;
  - `navigation-get-has-no-body.yaml`: a `Navigate` with `Get` and no body; its
    `NavigationStarted` carries `method: Get`, and `Requests` holds the request with
    `method: Get`, `body_type: null` and `state: Pending`.
- `task generate`.
- `crates/surf/src/lib.rs`:
  - `pub use surf_types::net::{Method, RequestStarted}`;
  - `Transport::fetch(&mut self, request: &RequestStarted) -> Answer`, and the closure impl for
    `FnMut(&RequestStarted) -> Answer`; `Browser::run` hands the started event itself;
  - `pub fn fetch_http(request: &RequestStarted, limits: &surf_fetch::Limits) ->
    Result<surf_fetch::Response, surf_fetch::Failure>`: a `Get` without a body and without a
    media type goes to `surf_fetch::fetch`; any other request ends with
    `Failure::UnsupportedScheme` whose detail names the method, with no connection attempted,
    until `agent-fetch-post`;
  - `Browser::start` and `Browser::load` keep their signatures and navigate with `Get`, no body;
  - `Browser::held` counts the request bodies `NavigationStarted`, `RequestStarted` and the
    invocation `FetchOnNavigationStarted` carry, beside the response bodies.
- `crates/surf-cli/src/main.rs`: its transport takes the started request and calls
  `surf::fetch_http`; its diagnostic line is unchanged. The binary navigates with `Get` only, so
  no test of the binary reaches the refusal.
- `crates/session/src/lib.rs`: `fetch(request: &RequestStarted) -> Answer` is
  `surf::answer(surf::fetch_http(request, &Limits::default()))`. `crates/session/src/entries.rs`:
  the event lines of `NavigationStarted` and `RequestStarted` write `method`, `body` (its length,
  `<n> bytes`, never the bytes, D22) and `body_type`, in the order of `spec/`, an absent one as the
  field with no value.
- `crates/ports/src/lib.rs`: the two documentation examples that build a `Navigate`.
- `conformance/src/target.rs` and `conformance/src/values.rs`: the new inputs of `Navigate` and
  `Fetch` (an absent optional and one sent as `null` read the same), the new fields of the
  `Requests` row, of the two events and of the invocations `FetchOnNavigationStarted` and
  `NavigateOnNavigationRequested`, and the unit tests of `target.rs` that build them.
- Tests that build the changed types or implement `Transport`: `crates/surf/tests/library.rs`,
  `review1_library.rs`, `forget_delivered.rs`, `held.rs`; `crates/session/tests/core.rs`,
  `events.rs`.
- New cases:
  - `crates/surf/tests/request_method.rs`: 1. `load_hands_get_without_body_to_the_transport`
     (`Browser::load` with a recording transport: `Get`, no body, no media type);
     2. `post_navigation_reaches_the_transport` (a `Navigate` on `navigator_mut()` with `Post`,
     `a=1` and its media type, then `run`: the transport receives the three unchanged);
     3. `fetch_http_refuses_post_without_connecting` and 4. `fetch_http_refuses_get_with_body`
     (a listener the test binds on 127.0.0.1 port 0, non-blocking: `Err(UnsupportedScheme(_))`
     and `accept()` answers `WouldBlock`); 5. `held_counts_a_request_body` (the post of case 2 over a
     transport that answers `Unanswered`: `held().body_bytes` is 9, three bytes each in
     `NavigationStarted`, `RequestStarted` and the `Fetch` invocation, before `forget_delivered`,
     and 0 after).
  - `crates/session/tests/core.rs`: `fetch_refuses_post` (`surf_session::fetch` of a `Post`
    answers `Answer::Failure(FailureReason::UnsupportedScheme)`).
  - `crates/session/tests/events.rs`: `case_17_every_event_maps_with_all_its_fields` covers the
    three new fields of both events, and `byte_fields_are_lengths` counts `Optional<Bytes>` too
    and lists 5 fields.

Surfaces written: `spec/domains/net.yaml`, `spec/domains/navigation.yaml`, `spec/components.yaml`,
`spec/scenarios/` (2 new files, 7 changed), `spec/ess-inputs.yaml`, `generated/rust/`,
`crates/surf/src/lib.rs`, `crates/surf/tests/`, `crates/surf-cli/src/main.rs`,
`crates/session/src/lib.rs`, `crates/session/src/entries.rs`, `crates/session/tests/core.rs`,
`crates/session/tests/events.rs`, `crates/ports/src/lib.rs`, `conformance/src/`. Not
`Taskfile.yml` (decision D30; the refusals do not change), not `Cargo.lock` (no dependency
changes).

## For AGENTS.md (handed over in the unit report, decision D7)

- "Components": `Navigate` and `Fetch` carry a method (`Get`, `Post`), an optional body and its
  media type; `Requests` shows the method and the media type; the agent's `Go` navigates with
  `Get`; until `agent-fetch-post` the transports of `surf` and `surf-session` end any other
  request with `UnsupportedScheme`.
- "Layout": fourteen authored scenario files, with the two of this story; `crates/surf`: the
  transport takes the started request (`surf::RequestStarted`), and `surf::fetch_http`.
- "The gate's numbers": 95 scenarios (14 authored), 28 refusals, the new suite digest, the
  number the planted defect fails, the test suites and tests of the root workspace.

## Acceptance

- `slot task check REFUSALS_MAX=28` exits 0 and prints `surf v1 — 9 file(s), 14 scenario(s),
  valid`, `95 scenario(s) (14 authored), 28 refusal(s)`, `95 scenarios: 95 passed, 0 failed,
  0 error, 0 unsupported` for the interpreter and for `conform-rust`, `the interpreter and the Rust
  system ran 95 scenario(s) each`, `generated/rust matches spec/ (30 artifact(s) and .ess-output)`
  and `Cargo.lock: 22 package(s), none from outside this repository`.
- `.out/conform.log` and `.out/conform-rust.log` list
  `surf.navigation/authored/navigation-post-reaches-request` and
  `surf.navigation/authored/navigation-get-has-no-body` among the passed.
- `grep -c '^refused:' .out/synthesize.log` prints 28; the unit report gives `<n>` = 28 and names
  any refusal that differs from the list in `AGENTS.md`.
- `task open` lists no marker for request methods or bodies.
- `cargo test -p surf --test request_method` passes its 5 cases; `cargo test -p surf-session
  --test core` passes `fetch_refuses_post`; `cargo test -p surf-session --test events` passes
  `case_17_every_event_maps_with_all_its_fields` and `byte_fields_are_lengths`.

## Out of scope

Sending the body (`agent-fetch-post`); a request body limit (`agent-fetch-post`; nothing in this
story lets a caller outside the process supply a body); forms (`agent-forms-spec`); storing the
method on the navigation (`agent-history-spec`); other methods; request headers; the example
events of `crates/view/examples/prototype.rs` and the example JSON of `docs/view-stream.md`, which
show `NavigationStarted` and `RequestStarted` without the new fields and are held only to their
own tests.
