---
format: aep.planning-md/3
id: story:agent-fetch-post
kind: story
status: implemented
title: surf-fetch sends POST with a body and changes the method on redirect
summary: surf-fetch sends a request with a method, a body and its media type; POST sends Content-Type and Content-Length; 301 and 302 after POST, and 303 after any method but GET, continue as GET without a body.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-request-body-spec
- depends_on: story:runner-surf-cli-package
scope:
- confidence: cited
  path: crates/fetch/src/lib.rs
- confidence: cited
  path: crates/fetch/tests/adversary_redirect_memory.rs
- confidence: cited
  path: crates/fetch/tests/fetch_bounds.rs
- confidence: cited
  path: crates/fetch/tests/post.rs
- confidence: cited
  path: crates/fetch/tests/review1_fetch.rs
- confidence: cited
  path: crates/fetch/tests/review1_security_fetch.rs
- confidence: cited
  path: crates/fetch/tests/review2_fetch_limits.rs
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: cited
  path: crates/session/tests/core.rs
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: crates/surf/tests/request_method.rs
revision: 18
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-21T20:39:46Z", actor: "human:operator", revision: 18, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

`crates/fetch` (`surf-fetch`) sends `GET` only: `exchange` writes `GET <target> HTTP/1.1`, `Host`
and `Connection: close` (`crates/fetch/src/lib.rs:322-325`), and its documentation lists
"methods other than `GET`, request bodies" as not done (`lib.rs:110-111`). Wave 8
(`agent-request-body-spec`) gave a request a method (`surf.net.Method`: `Get`, `Post`), an
optional body and its media type, and handed the whole `RequestStarted` to the transport.
`surf::fetch_http` (`crates/surf/src/lib.rs:121-134`) is the transport step of `surf-cli` and
`surf-session`: it fetches a `Get` without body and media type and ends every other request with
`UnsupportedScheme` before any connection. Two tests pin that refusal:
`fetch_http_refuses_post_without_connecting` (`crates/surf/tests/request_method.rs:139`) and
`fetch_refuses_post` (`crates/session/tests/core.rs:111`).

The redirect rule is already specified, in the comment of `spec/domains/net.yaml:70-75`, as the
Fetch Standard has it ("HTTP-redirect fetch", section 4.5, step 12, read on the Living Standard
last updated 20 November 2026): a 301 or 302 after a `Post`, and a 303 after any method other
than `Get` (or `HEAD`), continue as `Get` with the body set to null and the request-body-header
names (`Content-Encoding`, `Content-Language`, `Content-Location`, `Content-Type`, section 2.2.5)
deleted; every other redirect, 307 and 308 among them, keeps the method and the body. With the
two methods surf has, a 303 changes only a `Post`; a 303 after a `Get` stays a `Get`, as
`crates/fetch/tests/redirects.rs` already holds. Content-Length follows "HTTP-network-or-cache
fetch" (section 4.6, steps 5 to 9): the body's length when there is a body; `0` when the body is
null and the method is `POST`; no field for a `GET` without a body.

No command sends a `Post` yet: `surf <url>` and an agent's `go` and `open` navigate with `Get`.
Nothing of this story is reachable by a user of the commands; its cases reach it through the
libraries (`surf_fetch`, `surf::fetch_http`, `surf_session::fetch`) and a server the test starts.
`story:agent-press-submit` is the first command that sends a `Post`.

## Change

- `crates/fetch/src/lib.rs`:
  - A request type, borrowed from the caller: `pub struct Request<'a> { address: &'a str, method:
    Method, body: Option<&'a [u8]>, media_type: Option<&'a str> }` and `pub enum Method { Get,
    Post }` of its own (the crate depends on `surf-url` only, not on the generated types), and
    `pub fn send(request: &Request<'_>, limits: &Limits) -> Result<Response, Failure>`.
    `fetch(address, limits)` stays, as `send` of a `Get` without a body, so the 51 calls of
    `fetch(&…)` in six test files and the one in `adversary_redirect_memory.rs` do not change.
  - The request: `POST <target> HTTP/1.1`, `Host`, `Connection: close`, then `Content-Type:
    <media type>` when the request has one, then `Content-Length: <n>` for every `Post` (`0` when
    the body is absent or empty), then the body bytes unchanged. A `Get` sends neither field. The
    body is written from the caller's slice (head and body as two writes); the fetch allocates no
    copy of it.
  - Checks before resolving the host, after the address checks, each ending the fetch with
    `Failure::Unreachable` and a detail that names the check, with no connection (the crate's
    precedent for a request it cannot send, `lib.rs:121-125`; the specification has no reason
    for it): a `Get` with a body or a media type; a media type that is not `type "/" subtype`,
    each a token (RFC 9110 section 5.6.2) of at most 127 bytes (RFC 6838 section 4.2), which
    refuses CR, LF and `;` and so any header injection; a body longer than
    `Limits::max_request_body_bytes`.
  - `Limits::max_request_body_bytes`, default 16 MiB (16,777,216 bytes, the response body
    limit's value). The four `Limits` literals without `..Limits::default()` gain the field:
    `tests/fetch_bounds.rs:77`, `tests/review1_fetch.rs:70`, `tests/review1_security_fetch.rs:70`,
    `tests/review2_fetch_limits.rs:57`.
  - Redirects: after a 301 or 302 to a `Post`, and after a 303 to a `Post`, the next request is
    a `Get` without body, `Content-Type` or `Content-Length`, and stays so for the rest of the
    chain; a 307 or 308 repeats the method, the body and the media type. Sending the body again
    counts against the read timeout as every request of a fetch does (`lib.rs:30-38`).
  - Documentation: "What it does not do" drops `GET`-only and request bodies and names what
    stays out (below); "Limits" gains the request body limit and the media type check; "Memory"
    says the request body is the caller's and is not copied, so the sum while a response body is
    read stays 16,917,505 bytes at the defaults; a write that fails (a server that answers and
    closes before reading the body) is `Unreachable`, as a send failure is today (`lib.rs:329-331`).
- `crates/surf/src/lib.rs`: `fetch_http` maps `surf::Method` to `surf_fetch::Method` and calls
  `surf_fetch::send` with `request.body.as_deref()` and the media type; a `Get` with a body or a
  media type is still refused before connecting, as `UnsupportedScheme` (wave 8's reason, kept so
  that `fetch_http_refuses_get_with_body` stands unchanged). The module documentation
  (`lib.rs:30-32`) and the function's (`lib.rs:115-120`) say what it sends now.
- `crates/session/src/lib.rs`: the documentation of `fetch` (`lib.rs:132-137`) drops "until
  `story:agent-fetch-post`". No code change.
- `crates/surf-cli/src/main.rs` needs no change: it calls `fetch_http` unchanged and navigates
  with `Get` only (`main.rs:39-41`).
- Tests, each against a server the test starts on `127.0.0.1` port 0. A server that reads a
  `Post` reads the head and then `Content-Length` bytes before it answers: a server that answers
  first can block with the fetch on full socket buffers until the read timeout. "Without
  connecting" is shown as in `request_method.rs:98-101`: a non-blocking listener whose `accept`
  answers `WouldBlock` afterwards. In `crates/fetch/tests/post.rs` (new):
  1. `post_sends_method_media_type_length_and_body`: the server reads the request line
     `POST /session HTTP/1.1`, `Content-Type: application/x-www-form-urlencoded`,
     `Content-Length: 24` and the body `user=ada&password=s3cret`;
  2. `post_with_empty_or_absent_body_sends_content_length_0`: `Some(b"")` and `None` each send
     `Content-Length: 0` and no byte after the head;
  3. `post_without_media_type_sends_no_content_type`;
  4. `post_body_at_the_limit_is_sent`: limit 1,000, a body of 1,000 bytes arrives whole;
  5. `post_body_over_the_limit_fails_without_connecting`: 1,001 bytes, `Unreachable`, no
     connection;
  6. `post_media_type_not_type_slash_subtype_fails_without_connecting`: `text/plain\r\nX-A: 1`,
     `text/plain; charset=utf-8`, the empty text and a type of 128 bytes each end `Unreachable`
     with no connection;
  7. `redirect_303_after_post_continues_as_get_without_body`: the second request line is
     `GET /next HTTP/1.1`, its head has no `Content-Type` and no `Content-Length`, and no byte
     follows the head before the fetch closes the connection;
  8. `redirect_302_after_post_continues_as_get_without_body`, the same;
  9. `redirect_301_after_post_continues_as_get_without_body`, the same;
  10. `redirect_307_after_post_repeats_method_and_body`: the second request is `POST` with the
      same `Content-Type`, `Content-Length` and body;
  11. `redirect_308_after_post_repeats_method_and_body`, the same;
  12. `redirect_307_after_a_changed_method_stays_get`: `POST`, 303, then 307: the third request
      is `GET` without body;
  13. `post_body_outside_ascii_arrives_unchanged`: the bytes `00 0d 0a 80 c3 a9 ff` arrive as
      sent and `Content-Length: 7`.
- `crates/fetch/tests/adversary_redirect_memory.rs`: 14. `a_post_body_is_not_copied`: a `Post`
  of 1 MiB through one 307, then a response body at the default limit, holds at most 16,917,505
  bytes beyond what was held before the fetch (the body is allocated before the measurement).
  Its server reads the request body by `Content-Length` with a buffer on the stack.
- `crates/surf/tests/request_method.rs`: 15. `fetch_http_refuses_post_without_connecting`
  becomes `post_navigation_is_sent_by_fetch_http`: a `Browser` whose transport is
  `answer(fetch_http(request, &short()))` runs `Navigate` with `Post`, `a=1` and the form media
  type; the server reads `POST /form HTTP/1.1`, the media type, `Content-Length: 3` and `a=1`,
  and the `Requests` row is `Responded`. The other four cases stay; the file keeps 5.
- `crates/session/tests/core.rs`: 16. `fetch_refuses_post` becomes `fetch_sends_post`:
  `surf_session::fetch` of a `Post` reaches a server that reads `POST /form HTTP/1.1` and the
  body, and answers `Answer::Response` with status 200. The file keeps 10.

Formatting: `cargo fmt -p surf-fetch` rewrites five test files the unit does not own
(`fetch.rs`, `redirects.rs`, `review1_fetch.rs`, `review1_security_fetch.rs`,
`review2_fetch_limits.rs`; `cargo fmt -p surf-fetch -- --check`, read on `main` at `5129b50`), and
`cargo fmt -p surf` one (`review1_generated_check.rs`). Format with `rustfmt --edition 2021` on
the unit's own files, or restore the others.

## Acceptance

`cargo test -p surf-fetch -p surf -p surf-session` passes with warnings denied. `surf-fetch` has
105 tests (91 before: 13 unit tests and 78 in seven files), the 13 of `tests/post.rs` and case 14
named as above; case 1 reads `POST /session HTTP/1.1` and the body the test sent; cases 5 and 6
end `Unreachable` with no connection. `request_method.rs` has 5 tests and `core.rs` 10, with
`post_navigation_is_sent_by_fetch_http` and `fetch_sends_post` in place of the two refusals, and
`grep -rn "agent-fetch-post" crates/` prints nothing. Each of cases 1, 7, 10, 12 and 14 fails
against `main` at `5129b50` with the test written first (1 and 10 cannot send, 7 and 12 cannot
redirect a `Post`, 14 has no `send`).

## Out of scope

Other methods (`HEAD`, `PUT`); `Expect: 100-continue`; chunked request bodies; a media type with
parameters (`multipart/form-data; boundary=…` is `story:agent-forms-spec`'s: `surf.net.MediaType`
is specified without parameters, and the forms of this plan send
`application/x-www-form-urlencoded`, which has none); `Origin`, `Referer` and every other request
header (`spec/domains/net.yaml`: `UNMAPPED: request headers, cookies, caching and TLS.`); form
encoding (`agent-form-port`); a command that sends a `Post` (`agent-press-submit`). The
documents that say a `Post` is refused (`README.md:185-186`, `website/docs/what-works.md:28-29`,
`AGENTS.md:219-222` and `:455-456`) are the coordinator's, in the wave's documents commit.
