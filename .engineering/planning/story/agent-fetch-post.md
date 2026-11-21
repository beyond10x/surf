---
format: aep.planning-md/3
id: story:agent-fetch-post
kind: story
status: active
title: surf-fetch sends POST with a body and changes the method on redirect
summary: fetch takes a method, a body and its media type; POST sends Content-Type and Content-Length; 301 and 302 after POST and any 303 continue as GET.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-request-body-spec
- depends_on: story:runner-surf-cli-package
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 4}
---
## Context

`crates/fetch` sends `GET` only (its documentation). Story `agent-request-body-spec` specifies a
method, a body and its media type on a request and the redirect method rule, and leaves the binary
transport refusing anything but `GET`; `agent-fetch-redirects` follows redirects. RFC 9110
(methods, `Content-Length`), RFC 9112 and WHATWG Fetch "HTTP-redirect fetch" (the method change),
from memory, to be confirmed by this story.

The surf-fetch entry point changes; `crates/session` calls it through its transport, so this
story changes `crates/session` too (parallel critic, round 1, finding 7). The binary is in
`crates/surf-cli` (decision D6).

## Change

- `crates/fetch`: the entry point takes a request (address, method, optional body and its media
  type) and the limits; `POST` sends `Content-Type` and `Content-Length`; a request body limit,
  default documented, refused before connecting; after `POST`, 301 and 302 continue as `GET`
  without a body; 303 after any method continues as `GET` without a body; 307 and 308 repeat
  method and body. The documentation no longer lists methods and bodies as not done.
- `crates/surf/src/lib.rs`, `crates/surf-cli/src/main.rs`, `crates/session/src/`: the transports
  pass method and body to `fetch`; the interim refusal of `agent-request-body-spec` goes.
- `crates/fetch/tests/`, the counted cases, against servers the test starts:
  1. the server reads `POST /session HTTP/1.1`, `Content-Type`, `Content-Length` and the body sent;
  2. an empty body sends `Content-Length: 0`; 3. a body at the limit is sent;
  4. a body one byte over the limit fails with no connection; 5. 303 after `POST` continues as
  `GET` without a body; 6. 307 after `POST` repeats method and body; 7. 308 likewise;
  8. 302 after `POST` becomes `GET`; 9. 301 after `POST` becomes `GET`;
  10. a body with bytes outside ASCII arrives unchanged.
- `crates/surf-cli/tests/`: 11. the binary transport no longer refuses a `POST` (a library test
  issues a `Post` navigation through the binary transport and the server reads it).

Surfaces written: `crates/fetch/`, `crates/surf/src/lib.rs`, `crates/surf-cli/`, `crates/session/src/`.

## Acceptance

`cargo test -p surf-fetch -p surf-cli` passes with the 11 numbered cases, each a test named after
its case, case 1 reading the request line `POST /session HTTP/1.1` and the body the test sent, and
`cargo test -p surf-session` still passes.

## Out of scope

Other methods; `Expect: 100-continue`; chunked request bodies; form encoding (`agent-form-port`).
