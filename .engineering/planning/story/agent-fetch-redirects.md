---
format: aep.planning-md/3
id: story:agent-fetch-redirects
kind: story
status: active
title: surf-fetch follows redirects and reports the response URL
summary: 301, 302, 303, 307 and 308 followed for GET up to 20 times, Location resolved with surf-url; the surf transport delivers the final URL.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-url-parser
- depends_on: story:runner-surf-cli-package
- depends_on: story:agent-redirect-spec
- depends_on: story:html-document-spec
- depends_on: story:agent-session-process
- depends_on: story:agent-session-events
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 5}
---
## Context

`crates/fetch` returns a 3xx as a response (its documentation, "What it does not do"). Story
`agent-redirect-spec` specifies a redirect as followed inside one request, the response URL on
`DeliverResponse`, and the reason `TooManyRedirects`; until this story `Browser::run` delivers the
request address as the response URL. `Location` is resolved against the URL of the response that
carried it (WHATWG Fetch "HTTP-redirect fetch" and the URL Standard parser with a base, from
memory, to be confirmed); `agent-url-parser` is the parser.

The `surf` binary lives in `crates/surf-cli` since `runner-surf-cli-package` (decision D6); its
binary tests are there.

`surf::Answer::Response` gains the response URL. `crates/session` builds answers in its tests
(`agent-session-process`, case 9), so this story updates them; the session process calls
`surf::answer`, whose signature stays. This story needs nothing from the node tree; it comes
after `html-document-spec` only because both write `crates/surf/src/lib.rs`.

## Change

- `crates/fetch`: follows 301, 302, 303, 307 and 308 for `GET`; at most 20 redirects, then
  `Failure::TooManyRedirects`; `Location` parsed with `surf-url` against the current URL; one that
  does not parse is `Failure::MalformedResponse`; a target that is not `http:` is
  `Failure::UnsupportedScheme` with no connection; a 3xx without `Location` is returned as the
  response; the connect and read limits bound the whole chain (documented); `Response` carries the
  final URL. The documentation no longer lists redirects as not done.
- `crates/surf/src/lib.rs`: `Answer::Response` carries the response URL; `answer` maps it and
  `TooManyRedirects`; `Browser::run` delivers it.
- `crates/session/tests/`: the transports built in tests give a response URL.
- `crates/fetch/tests/`, the counted cases, each against servers the test starts:
  1. a 301; 2. a 302; 3. a 303; 4. a 307; 5. a 308, each followed to a 200 with the final URL;
  6. a relative `Location`; 7. a chain of 20 followed; 8. a chain of 21: `TooManyRedirects`;
  9. a loop: `TooManyRedirects`; 10. a `Location` to `https:`: `UnsupportedScheme`, no connection;
  11. a `Location` to `file:`: `UnsupportedScheme`; 12. a 3xx without `Location`: the 3xx returned;
  13. a `Location` that does not parse: `MalformedResponse`; 14. a redirect to another port;
  15. a chain of slow hops ends within one read timeout.
- `crates/surf-cli/tests/`: 16. `surf <url>` on a server that redirects `/a` to `/b` prints the
  navigation committed at `/b`.

Surfaces written: `crates/fetch/`, `crates/surf/src/lib.rs`, `crates/session/tests/`,
`crates/surf-cli/tests/`.

## For AGENTS.md (handed over in the unit report, decision D7)

- "Layout", the `crates/fetch` entry: "... follows up to 20 redirects (301, 302, 303, 307, 308)
  and returns the address it ended at ...".

## Acceptance

`cargo test -p surf-fetch -p surf-cli` passes with the 16 numbered cases, each a test named after
its case, case 16 printing the navigation committed at `/b`, and `cargo test -p surf-session`
still passes.

## Out of scope

The method change after `POST` (`agent-fetch-post`); HTTPS; cookies across redirects.
