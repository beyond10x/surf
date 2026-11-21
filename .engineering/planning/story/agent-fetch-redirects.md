---
format: aep.planning-md/3
id: story:agent-fetch-redirects
kind: story
status: implemented
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
scope:
- confidence: cited
  path: conformance/Cargo.lock
- confidence: cited
  path: crates/fetch/
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: cited
  path: crates/session/tests/core.rs
- confidence: cited
  path: crates/session/tests/redirect.rs
- confidence: cited
  path: crates/surf-cli/tests/redirects.rs
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: crates/surf/tests/forget_delivered.rs
- confidence: cited
  path: crates/surf/tests/held.rs
- confidence: cited
  path: crates/surf/tests/library.rs
- confidence: cited
  path: crates/surf/tests/review1_library.rs
- confidence: cited
  path: docs/surf-command.md
- confidence: cited
  path: spec/domains/net.yaml
revision: 21
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-21T16:32:39Z", actor: "human:operator", revision: 21, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

`crates/fetch` returns a 3xx as a response (its documentation, "What it does not do",
`crates/fetch/src/lib.rs:56-64`). Story `agent-redirect-spec` (wave 4) specified a redirect as
followed inside one request: `surf.net.DeliverResponse.url` is the response URL, "the address
after the last redirect, which is the request's own address when nothing redirected"
(`spec/domains/net.yaml:144-145`); `ResponseReceived.url` is that input; `CommitNavigation` sets
the navigation's `url` to it, so `Navigations`, `Documents` and the session's `page` line show
it, while `Requests` and the `go` action keep the address asked for; the reason
`surf.net.FailureReason::TooManyRedirects` exists (`net.yaml:42`). The names are unchanged on
`main` after wave 6. Until this story `Browser::run` delivers the request's own address as the
response URL (`crates/surf/src/lib.rs:599-603`).

`spec/domains/net.yaml:48-58` states what the transport does: 301, 302, 303, 307 and 308 with a
`Location` are followed inside one request, at most 20 times, a redirect after the twentieth is
`TooManyRedirects`, a `Location` whose scheme is not `http:` is `UnsupportedScheme`, a redirect
status without `Location` is a response. Its marker `UNMAPPED:` at lines 57-58 (a `Location`
that does not parse) names this story as the one that settles it.

Standards, from memory, to be confirmed by the unit against the published texts: WHATWG Fetch
"HTTP-redirect fetch" and "location URL" (resolution against the response's URL; a `Location`
without a fragment takes the fragment of the request's current URL; more than one `Location`
value, or one that does not parse, is a network error); the URL Standard's parser with a base.
`agent-url-parser` gave the parser: `surf_url::Url::parse(input, Some(&base))` and `href()`.

The `surf` binary lives in `crates/surf-cli` since `runner-surf-cli-package` (decision D6).

`surf::Answer::Response` gains the response URL. Every constructor of it in the tree must change
or the package does not compile: `crates/surf/src/lib.rs:107`, `crates/surf/tests/library.rs:57`
and `:166`, `crates/surf/tests/review1_library.rs:22`, `crates/surf/tests/forget_delivered.rs:30`,
`crates/surf/tests/held.rs:17` and `crates/session/tests/core.rs:23` (`git grep -n
"Answer::Response"`). `conformance/src/target.rs:79-80` answers `Answer::Unanswered` only and
compiles unchanged. The session process calls `surf::answer` (`crates/session/src/lib.rs:115-116`),
whose signature stays.

A redirect adds no event and no entry: `Browser::answer` (`crates/surf/src/lib.rs:585-618`)
calls the transport once per `RequestStarted` and issues one `DeliverResponse`; the hops happen
inside `surf_fetch::fetch`. What changes for the session and the live view is the address: the
page line, the `ResponseReceived`, `NavigationCommitted` and document entries and the answer
entry hold an address the server chose, not one the agent sent. Today every address in a
session is bounded by the wire's request (`surf_agent::wire::REQUEST_MAX`, 64 KiB), and
`crates/session/src/lib.rs:49-53` and `:76-80` derive the answer's and the entries' bounds from
it. Measured with `surf-url` on 2026-11-21 (a scratch binary over the tree's crate): a relative
`Location` of 30,000 bytes against a base path of 40,000 bytes resolves to 70,023 bytes, and a
`Location` of 60,001 bytes holding 20,000 spaces resolves to 100,022 bytes (each space written
`%20`). One hop can so add up to three times the header limit (64 KiB) to the address, and 20
hops about 3.8 MiB, against an answer of 4 MiB (`wire::ANSWER_TEXT_MAX`) and entries of 8 MiB
(`surfd::link::ENTRIES_MAX`) that carry the address several times. This story bounds the
response URL at 64 KiB, so that every bound derived from the wire still holds.

## Change

- `crates/fetch` (`surf-fetch`, path dependency on `surf-url` in `crates/fetch/Cargo.toml`):
  - Follows 301, 302, 303, 307 and 308 for `GET` when the response has a `Location`; any other
    status (300 and 304 included), or one of the five without `Location`, is the response.
  - At most 20 redirects: the response to the 21st request, if it is a redirect, ends the fetch
    `Failure::TooManyRedirects` (new variant; `Display` word `too many redirects`; its detail
    names the 20). So a chain of 20 makes 21 requests and ends at the 21st response, and a chain
    of 21 or a loop makes 21 requests and fails.
  - `Location`: the field's value as UTF-8, resolved with `surf_url::Url::parse(location,
    Some(&current))`, where `current` is the URL of the response that carried it (the asked
    address parsed with `surf-url` for the first hop). A `Location` that does not parse, one
    that is not UTF-8, a host `surf-url` refuses (`ParseError::NonAsciiHost`, no IDNA) and more
    than one `Location` field each end the fetch `Failure::MalformedResponse`, with no further
    connection. An empty value is a `Location` (it resolves to the current address).
  - A resolved URL without a fragment takes the fragment of the current URL (Fetch). The next
    address is the resolved URL's `href()`; it goes through the same `Target::parse` as an asked
    address, so a scheme other than `http:` is `Failure::UnsupportedScheme` with no connection,
    and user information is `Failure::Unreachable`, as for an asked address. The fragment is
    never sent.
  - `pub const MAX_ADDRESS_BYTES: usize = 65_536`: a resolved address longer than that ends the
    fetch `Failure::MalformedResponse` with no further connection. Equal to
    `surf_agent::wire::REQUEST_MAX`, so a response URL is never longer than an address an agent
    can send. The asked address is not checked by it.
  - The body of a followed redirect is not read: the connection is closed after its header
    section. Each response gets its own header budget (`Limits::max_header_bytes`).
  - Timeouts over the whole chain: the time spent resolving and connecting, summed over every
    hop, is bounded by `Limits::connect_timeout`, and the time spent in exchanges after a
    connection is made, summed over every hop, by `Limits::read_timeout`. So one fetch still
    lasts at most the connect timeout plus the read timeout, 40 s at the defaults, whatever the
    number of hops.
  - `Response` gains `url: String`: the asked address byte for byte when nothing redirected
    (`net.yaml:144-145`), else the `href()` of the last resolved URL.
  - The crate documentation: "Limits and how each is enforced" gains a "Redirects" item (the
    20, `MAX_ADDRESS_BYTES`, the header budget per response, the timeouts over the chain); "What
    it does not do" no longer lists redirects; the reasons in the first paragraph are four.
- `crates/surf/src/lib.rs`: `Answer::Response` gains `url: String`; `answer` maps
  `Response.url` and `Failure::TooManyRedirects` to `FailureReason::TooManyRedirects`;
  `Browser::run` issues `DeliverResponse` with `url: Url(url)` from the answer, and the comment at
  lines 599-600 goes.
- `crates/surf/tests/library.rs`, `review1_library.rs`, `forget_delivered.rs`, `held.rs` and
  `crates/session/tests/core.rs`: each transport answers `url` = the address it was asked for, so
  no assertion of those files changes.
- `crates/session/src/lib.rs`, documentation only: "What it keeps" and "The entries" say an
  address comes from a request of at most `REQUEST_MAX` bytes or from a response URL of at most
  `surf_fetch::MAX_ADDRESS_BYTES`, the same 64 KiB.
- `spec/domains/net.yaml`, comment only: the marker `UNMAPPED:` at lines 57-58 is replaced by the
  decision (a `Location` that does not parse, is not UTF-8 or comes twice, and a resolved
  address over 64 KiB, end the request `MalformedResponse`; a `Location` without a fragment takes
  the current one). Measured 2026-11-21 on a scratch copy with that replacement, ess 0.96.0:
  `surf v1 — 9 file(s), 12 scenario(s), valid`, `30 artifact(s)`, and the generated workspace
  identical to `generated/rust/` (`diff -rq` empty); `task open` goes from 20 lines to 19.
- `docs/surf-command.md`: `<reason>` (line 36-37) gains `TooManyRedirects`; one sentence: after
  redirects the `navigation` and `document` lines hold the address of the last response, the
  `request` and `fetch-failed` lines the address asked for.
- `conformance/Cargo.lock`: `surf-fetch` gains the dependency `surf-url` and the package
  `surf-url` is added; no other line changes (`cargo test --locked --manifest-path
  conformance/Cargo.toml` refuses the old lock). The root `Cargo.lock` is the coordinator's.
- `crates/fetch/tests/redirects.rs`, the counted cases, each against servers the test starts on
  127.0.0.1 port 0, each server recording the request targets it read:
  1. a 301; 2. a 302; 3. a 303; 4. a 307; 5. a 308: `/a` answers the status with `Location: /b`,
     `/b` a 200; the result is the 200 with `url` `http://127.0.0.1:<port>/b`, the server read
     `/a` then `/b`;
  6. a relative `Location: b?q` from `/dir/a`: `url` `http://127.0.0.1:<port>/dir/b?q`;
  7. a chain of 20 (`/0` to `/19` each redirect to the next, `/20` a 200): the 200 at `/20`, 21
     requests read;
  8. a chain of 21 (`/20` redirects to `/21` too): `TooManyRedirects`, 21 requests read, `/21`
     never;
  9. a loop (`/a` redirects to `/a`): `TooManyRedirects`, 21 requests read;
  10. `Location: https://127.0.0.1:<port2>/` with a listener the test holds on `<port2>`:
      `UnsupportedScheme`, and that listener accepted no connection;
  11. `Location: file:///etc/passwd`: `UnsupportedScheme`;
  12. a 302 without `Location`, and a 300 with one: each returned as the response, `url` the
      asked address, one request read;
  13. `Location: http://[::1/` (does not parse): `MalformedResponse`, one request read;
  14. a redirect to a second server on another port: its 200, `url` naming that port, and it read
      `Host: 127.0.0.1:<port2>`;
  15. slow hops: `read_timeout` 1 s, four hops each answering its 302 after 400 ms, the last a
      200: `Unreachable` within 2 s (a deadline per hop would end in the 200 after 1.6 s);
  16. fragments: from `/a#f`, `Location: /b` gives `url` ending `/b#f` and `Location: /c#g` gives
      `/c#g`; no request target read holds `#`;
  17. the address bound: a relative `Location` from a long base path resolving to exactly
      `MAX_ADDRESS_BYTES` bytes is followed; one resolving to one byte more is
      `MalformedResponse` with one request read;
  18. `Location: /\xff` (not UTF-8): `MalformedResponse`, one request read;
  19. two `Location` fields: `MalformedResponse`, one request read;
  20. no redirect: `HTTP://127.0.0.1:<port>` (upper-case scheme, no path) answered 200 gives `url`
      equal to that text byte for byte.
- `crates/surf-cli/tests/redirects.rs`:
  21. `surf http://127.0.0.1:<port>/a` against a server where `/a` answers 302 to `/b` and `/b`
      200 `text/html`: exit 0 and exactly three lines, `navigation <id> Committed
      http://127.0.0.1:<port>/b`, `request <id> Responded http://127.0.0.1:<port>/a`, `document
      <document-id> http://127.0.0.1:<port>/b`;
  22. `surf` on a loop: exit 1, `navigation <id> Failed …/a`, `request <id> Failed …/a`,
      `fetch-failed <id> TooManyRedirects …/a`, and standard error starting `surf:
      http://127.0.0.1:<port>/a: too many redirects`.
- `crates/session/tests/redirect.rs`:
  23. a session over `surf_session::fetch` (`Session::new`, in process) opens and goes to `/a`,
      which redirects to `/b` (200 `text/html`): status 0; the answer holds the line `page 200
      "text/html" "http://127.0.0.1:<port>/b"` and the path line `  2 go
      "http://127.0.0.1:<port>/a"`; the reply's entries are as many as those of a second session's
      `go` straight to `/b`; the `surf.net.RequestStarted` entry names `/a` and the
      `surf.net.ResponseReceived` entry `/b`.

Surfaces written: `crates/fetch/`, `crates/surf/src/lib.rs`, `crates/surf/tests/library.rs`,
`crates/surf/tests/review1_library.rs`, `crates/surf/tests/forget_delivered.rs`,
`crates/surf/tests/held.rs`, `crates/surf-cli/tests/redirects.rs`,
`crates/session/tests/core.rs`, `crates/session/tests/redirect.rs`, `crates/session/src/lib.rs`
(documentation only), `spec/domains/net.yaml` (a comment only), `docs/surf-command.md`,
`conformance/Cargo.lock`. No other story of wave 7 writes any of them
(`html-character-references`: `crates/html/`, `crates/html-conformance/`; `view-first-page`:
`crates/view/web/`, `crates/view/examples/prototype.rs`, `crates/view-fixture/`, `view-tests/`,
`taskfiles/view-test.yml`). The root `Cargo.lock` changes in this unit and in `view-first-page`
(a new package): the coordinator's, regenerated on the wave branch. No existing test serves a
3xx (`git grep` for the five statuses and `Location` under `crates/*/tests` and `conformance/`),
so no test outside this list turns red: not `crates/session/tests/view.rs` or
`adversary_view.rs`, not `crates/surfd/tests/`, `crates/agent/`, `crates/view/tests/`, and not the
conformance suite.

## For AGENTS.md, README.md and the website (handed over in the unit report, decision D7)

- `AGENTS.md` "Layout", the `crates/fetch` entry: "... follows up to 20 redirects (301, 302,
  303, 307, 308), returns the address it ended at (at most 64 KiB) and `TooManyRedirects` ...
  Depends on `surf-url`." (today "No dependency").
- `AGENTS.md` "Components" (lines 349-355): the sentences "This is model text only ... with the
  address asked for" go; the transport follows redirects.
- `AGENTS.md` "Commands" (lines 645-646): "a redirect is not followed (a redirect status is
  delivered as the response)" goes; the test counts of `surf-session` (line 250) gain
  `redirect` 1.
- `README.md` lines 87 and 190-193, `website/docs/what-works.md` lines 61-64 and 120,
  `website/docs/drive-from-an-agent.md` line 130: each says redirects are not followed.

## Acceptance

`cargo test -p surf-fetch -p surf-cli -p surf-session -p surf` passes, with the 23 numbered
cases each a test named `case_<n>_…` (1 to 20 in `crates/fetch/tests/redirects.rs`, 21 and 22 in
`crates/surf-cli/tests/redirects.rs`, 23 in `crates/session/tests/redirect.rs`), case 21 printing
`navigation <id> Committed http://127.0.0.1:<port>/b` and `request <id> Responded
http://127.0.0.1:<port>/a`; every test of those four packages that passed on `main` still passes.
`cargo build --locked --manifest-path conformance/Cargo.toml` exits 0. `slot task open` prints
19 lines and none names `Location`; `slot task generated-check` prints `generated/rust matches
spec/ (30 artifact(s) and .ess-output)`. `grep -c TooManyRedirects docs/surf-command.md` prints
at least 1.

## Out of scope

The method change after `POST` (`agent-fetch-post`); HTTPS; cookies across redirects; a bound on
the asked address; `Refresh` and HTML meta refresh.

## Revised 2026-11-21, before wave 7

- Added to the surfaces: the four test files of `crates/surf/tests/` that construct
  `Answer::Response` (the package does not compile otherwise) and `-p surf` in the acceptance;
  `conformance/Cargo.lock` (refused by `--locked` once `surf-fetch` depends on `surf-url`);
  `spec/domains/net.yaml` (the marker that names this story, a comment, measured: no generated
  change); `docs/surf-command.md` (the `<reason>` list, handed here by `agent-redirect-spec`);
  `crates/session/src/lib.rs` (its bounds argue from the wire's request alone).
- Decided: the response URL is the asked address byte for byte when nothing redirected
  (`net.yaml:144-145`); a resolved address over 64 KiB, a `Location` not UTF-8 or given twice
  are `MalformedResponse`; the fragment rule of Fetch; a redirect's body is not read; the two
  timeouts each bound their phase summed over the chain.
- Cases: 10 observes "no connection" with a listener; 7 to 9 count the requests; 16 to 20, 22 and
  23 added; case 16 of the old body is 21, with the exact lines; the `crates/surf-cli/tests/`
  directory narrowed to one new file.
- The AGENTS.md hand-over names the paragraphs that say redirects are not followed.
