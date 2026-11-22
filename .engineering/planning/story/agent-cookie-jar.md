---
format: aep.planning-md/3
id: story:agent-cookie-jar
kind: story
status: draft
title: 'A cookie jar per session: Set-Cookie and Cookie, never printed'
summary: The session's transport stores Set-Cookie and sends Cookie by RFC 6265 and draft 22 of its revision, one jar per session in memory, never shared or printed; look shows a count.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-follow-links
- depends_on: story:agent-press-submit
revision: 2
---
## Context

The coordinator's order of the remaining work (D85), item 4: cookies, not planned. Without them
an agent cannot stay logged in: a site that answers a login with `Set-Cookie` and a redirect
forgets the agent at the next request. `crates/fetch` sends no request field but `Host`,
`Connection`, `Content-Type` and `Content-Length` and reads no `Set-Cookie` ("What it does not
do": "cookies"); `spec/domains/net.yaml` ends its rules with `UNMAPPED: request headers, cookies,
caching and TLS.`

What binds the design:

- A session is one process (`crates/session`, started by `surfd`), and its transport is
  `surf::fetch_http` over `surf-fetch`. Everything a session publishes goes to `surfd` as entries
  and from there to the live view (`crates/session/src/entries.rs`, `docs/view-stream.md`). A
  cookie value is a credential, so it must be in no event, no view row, no entry, no answer and no
  `Debug` output.
- Redirects are followed inside one fetch (`crates/fetch/src/lib.rs`, the loop of `send`): a
  `Set-Cookie` on a redirect must be stored before the next hop, and each hop sends the cookies of
  its own address.
- `story:runner-record-session` records every exchange with the network; a recording must hold a
  `Cookie` or `Set-Cookie` value only as a reference, as it holds a password.
- `epic:scripts-in-a-page` (`plan/scripts-in-a-page`) runs scripts in a process of their own
  (its S2): `document.cookie` will have to ask the session process, and `HttpOnly` cookies stay
  out of it. Named here, designed there.

The standards, read 2026-11-22:

- RFC 6265, Standards Track, April 2011 (https://www.rfc-editor.org/rfc/rfc6265).
- Its revision, draft-ietf-httpbis-rfc6265bis-22 of 2025-12-01, intended Proposed Standard,
  submitted to the IESG and in the RFC Editor's queue, "Final Approval Pending", with no RFC
  number yet (https://datatracker.ietf.org/doc/draft-ietf-httpbis-rfc6265bis/; text:
  https://www.ietf.org/archive/id/draft-ietf-httpbis-rfc6265bis-22.txt). surf follows draft 22
  and names its sections; when the RFC is published, the story that follows it checks the
  numbers.
  - `SameSite`: section 4.1.2.7 and 5.6.7 (values `None`, `Strict`, `Lax`; an absent attribute
    gives the enforcement "Default"); its security discussion in 8.8.
  - Prefixes: 4.1.3.1 `__Secure-` (needs `Secure`), 4.1.3.2 `__Host-` (`Secure`, `Path=/`, no
    `Domain`); 5.4, matched without regard to case.
  - Size: a name and value over 4096 octets make the cookie ignored (5.6 step 5, 5.7 step 4); an
    attribute value over 1024 octets is ignored.
  - Lifetime: 5.5, the limit "SHOULD NOT be greater than 400 days (34560000 seconds)"; a longer
    `Expires` or `Max-Age` "MUST be reduced to the limit".
  - Public suffixes: 5.7 step 9, "If the user agent is configured to reject 'public suffixes' and
    the domain-attribute is a public suffix": the cookie becomes host-only when the attribute
    equals the host, and is ignored otherwise. Rejecting them is so a configuration, which needs
    a list.
  - Limits: 6.1, at least 50 cookies per domain and 3000 in total.

## The decisions of this story

- **Where the jar lives**: in the session process, in the transport, one jar per session, in
  memory only. It is dropped at `close`, when the process ends and when it dies; it is never
  written to a file and never shared between sessions. A new crate `crates/cookie`
  (`surf-cookie`, standard library and `surf-url` only) holds the parser, the storage model, the
  date parser and the `Cookie` field's serialisation; `surf-fetch` takes the jar as a parameter of
  `send` through a small trait of its own (cookies for an address; store the `Set-Cookie` values of
  a response for an address), so that `surf-fetch` stays without state and every test of it that
  passes no jar is unchanged.
- **Not an entity**: the jar is port state of the `net` component, as the HTTP client is a port,
  because the model's events are published to `surfd`. `net.yaml` states the rules as a comment, as
  it states the redirect rules, and replaces `cookies` in its `UNMAPPED:` marker. The one field the
  model gains is what `SameSite` needs: the initiator of a navigation (none for `go` and `open`,
  the document's origin for `follow` and `press`), carried by `Navigate` and `Fetch`. That field is
  drafted in the specification first (guardrail of the planning store: a new typed field is
  specified before code), by this story's first commit.
- **Attributes**: `Expires`, `Max-Age`, `Domain`, `Path`, `Secure`, `HttpOnly`, `SameSite`, and
  the `__Secure-` and `__Host-` prefixes, as the current revision draft states them (above).
  `Secure` cookies are stored only from and sent only to `https:` addresses, so they are inert
  until `epic:tls-https`; a non-secure origin cannot overwrite a `Secure` cookie. `HttpOnly` is
  stored and has no effect until scripts exist. `SameSite`: `go` and `open` are a person typing an
  address (same-site); `follow` and a `GET` submission from another site send `Lax` and `None`
  cookies; a cross-site `POST` sends `None` cookies only; a cookie without the attribute has the
  enforcement "Default" of section 5.6.7, and the unit writes down from the draft's retrieval
  algorithm what "Default" sends in each of these cases before coding it.
- **Host-only and domain cookies, and the public suffix problem**: a cookie without `Domain` is
  host-only. A `Domain` attribute is accepted only when the request host domain-matches it and,
  for an IP address host, only when it equals the host. surf has no public suffix list, so a
  server at `a.example.co.uk` could set a cookie for `co.uk` that the same session then sends to
  every `co.uk` site it visits. Options, decision U4 of the plan: A bundle the Public Suffix List
  (data, refreshed with the trust anchors if U2 bundles them); B treat every `Domain` attribute
  as host-only (safe; breaks a login on `login.example.org` that sets a cookie for
  `app.example.org`); C refuse a `Domain` of one label and accept the rest (blocks `com`, not
  `co.uk`). Default A. Until it is decided, the story builds B, which A extends.
- **Bounds** (the draft's limits, above, as the lower bounds, and surf's own caps): a cookie whose
  name and value exceed 4096 bytes, or an attribute value over 1024 bytes, is ignored; at most 50
  cookies per domain and 3,000 per session (the draft's minimums, so at most about 12 MiB of
  names and values per session, which the crate documentation states as its memory bound),
  evicted as the draft says (expired first, then the least recently used); at most 400 days of lifetime; a `Set-Cookie` field over the header limit
  ends as every over-long field does today (`MalformedResponse`); the `Cookie` field sent is at
  most 8 KiB, the most recently set cookies dropped first, and the request says so in no field.
- **What an agent sees**: options were nothing, a count, or names without values. Decided
  (reversible): a count. `look` prints a line `cookies <n>` after `page` when the session holds
  any, counted over the whole jar; no name and no value. Names are chosen by servers and some
  carry identifiers, and an agent that needs to know more is debugging a site, which is the live
  view's job, and the live view gets nothing of cookies either. `docs/agent-answer.md` gains the
  block by its section "Adding a block".

## Change

- `spec/domains/net.yaml`: the cookie rules as a comment, the initiator field on `Navigate`,
  `NavigationStarted`, `Fetch` and `RequestStarted` with scenarios, `task generate`.
- `crates/cookie` (new), `crates/fetch` (the jar trait, `Cookie` sent, `Set-Cookie` read on every
  response of a chain), `crates/surf` (`fetch_http` takes the jar), `crates/session` (one jar per
  session, the `cookies` line), `crates/answer` and `docs/agent-answer.md` (the block),
  `crates/agent` (nothing but the golden answers).
- A sixth task for `story:agent-tasks`: a login form, a redirect that sets a cookie, a page that
  answers only with it; the scripted agent logs in and reads the page.

## Acceptance

- `cargo test -p surf-cookie` passes the parsing and storage cases of the current revision
  draft, each named after its section: dates, `Max-Age` over `Expires`, `Domain` with a leading
  dot, path defaulting and matching, `Secure` from `http:` refused, the two prefixes, the bounds
  above, eviction order.
- Against a server the test starts on 127.0.0.1: a `go` to `/login` that answers `302` to
  `/home` with `Set-Cookie: s=1; Path=/` reaches `/home` with `Cookie: s=1`; a second session's
  `go` to `/home` sends no `Cookie`; after `close` and a new `open` nothing is sent.
- No answer, entry, event, view stream frame or `Debug` output of a session that holds a cookie
  holds its value: a test sets a cookie with a value of 32 random hexadecimal digits and searches
  every byte the session wrote to `surfd` and every answer for it.
- `look` prints `cookies 1` for that session and no `cookies` line for a session without one.
- The test step stays within 120 s; no test of this story runs over 5 s.

## Out of scope

- A cookie file, cookies shared between sessions or kept after `close`, a cookie command
  (`surf-agent cookies`), third-party cookies of subresources (surf loads none), `document.cookie`
  (`epic:scripts-in-a-page`), cookie partitioning (CHIPS).

## Why `epic:agent-cli-target`

The jar is one per agent session, lives in the session process and shows in the agent answer
(`cookies <n>`), which that epic owns. It is not TLS: `Secure` only waits for `https:`, so it
needs nothing of `epic:tls-https` to be built.

## Decisions taken

Taken by the coordinating agent on 2026-11-22, reported to the operator, who can overrule it:

- U4, cookies across sites: option A, the public suffix list is bundled as data, with its date and digest, refreshed before each release. The story builds the host-only rule first (option B), which the list then extends. Reason: without the list a cookie set for a public suffix is either accepted for every site under it or sign-in across subdomains breaks.
