---
format: aep.planning-md/3
id: story:agent-cookie-jar
kind: story
status: draft
title: 'A cookie jar per session: Set-Cookie and Cookie, never printed'
summary: 'One jar per session in memory by draft 22 of the RFC 6265 revision: host-only Domain rule, Secure tested against the TLS test server, SameSite by the navigation initiator, never printed; look shows cookies <n>.'
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-follow-links
- depends_on: story:agent-press-submit
- depends_on: story:tls-fetch-https
- depends_on: story:tls-session-anchors
- depends_on: story:agent-tasks
- depends_on: story:page-text-answer-block
scope:
- confidence: inferred
  path: conformance/src/target.rs
- confidence: cited
  path: crates/agent-tasks
- confidence: cited
  path: crates/answer/src/lib.rs
- confidence: cited
  path: crates/answer/tests
- confidence: cited
  path: crates/cookie
- confidence: cited
  path: crates/fetch
- confidence: cited
  path: crates/session/src
- confidence: cited
  path: crates/session/tests
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: crates/surf/tests
- confidence: cited
  path: crates/tls-test-server
- confidence: cited
  path: docs/agent-answer.md
- confidence: cited
  path: generated/rust
- confidence: cited
  path: spec/domains/agent.yaml
- confidence: cited
  path: spec/domains/navigation.yaml
- confidence: cited
  path: spec/domains/net.yaml
- confidence: cited
  path: spec/scenarios
revision: 6
---
## Context

The coordinator's order of the remaining work (D85,
`wave/a10-label-limits:docs/plan/agent-waves.md:142-143`, a file of that branch until it
merges), item 4: "Stay signed in. Cookies: not planned (`spec/domains/net.yaml` lists them as
unmapped). A story after the forms." Without cookies an agent cannot stay logged in: a site that
answers a login with `Set-Cookie` and a redirect forgets the agent at the next request.
`crates/fetch/src/lib.rs:146` lists "cookies" under "What it does not do"; `spec/domains/net.yaml:76`
is `# UNMAPPED: request headers, cookies, caching and TLS.`, which `story:tls-spec`
(tls-spec.md:91) turns into `# UNMAPPED: request headers, cookies and caching.`

What binds the design, read in the tree and on the branches named:

- A session is one process (`crates/session`, started by `surfd`); its transport calls
  `surf::fetch_http` (`crates/session/src/lib.rs:138`), below the `Transport` trait of
  `crates/surf/src/lib.rs:105-108`, whose `fetch` takes a `RequestStarted` and returns an
  `Answer`. Everything a session publishes goes to `surfd` as entries and from there to the live
  view (`crates/session/src/entries.rs`, `docs/view-stream.md`). A cookie value is a credential.
- The request loop is rewritten by `story:security-review-fixes` (branch
  `impl/security-review-fixes`, `6b900a5:crates/fetch/src/lib.rs:389-520`): `send` is
  `send_through(request, limits, &OsNetwork)`, and a `Network` (`resolve`, `connect`) is the
  seam where a test gives names the addresses it chooses. `story:tls-fetch-https` builds on it
  after its merge (tls-fetch-https.md:60-64) and puts the anchors on `Request` and on
  `fetch_http` (:93-104): one per-session request context. The jar goes where the anchors went.
- `story:runner-replay-transport` wraps the session's `Transport` to record each `RequestStarted`
  and its `Answer` (runner-replay-transport.md:15-45); neither type has a header field.
- `epic:scripts-in-a-page` (`plan/scripts-in-a-page`) runs scripts in a process of their own:
  `document.cookie` will ask the session process, and `HttpOnly` cookies stay out of it. Named
  here, designed there.

The standards, read 2026-11-22: RFC 6265 (Standards Track, April 2011) and its revision
draft-ietf-httpbis-rfc6265bis-22 of 2025-12-01 (intended Proposed Standard, in the RFC Editor's
queue, no RFC number yet; text https://www.ietf.org/archive/id/draft-ietf-httpbis-rfc6265bis-22.txt,
read from a copy fetched that day). surf follows draft 22 and cites its sections below; the story
that follows the published RFC checks the numbers.

## Decisions taken

Taken by the coordinating agent on 2026-11-22, reversible, reported to the operator, who can
overrule each:

1. U4, cookies across sites: A stays (bundle the public suffix list). This story builds the
   host-only rule, option B: every accepted `Domain` attribute gives a host-only cookie (rule R4).
   The list, with its date and digest, and the widening of R4 and of "same site" are
   `story:agent-cookie-public-suffixes` (draft, depends on this story).
2. "Same site" without the list (rule R9): same scheme and same host.
3. Placement: after round 2 and after `story:tls-fetch-https` and `story:tls-session-anchors`
   (two specification changes never start from one base: every `task generate` rewrites all of
   `generated/` with one digest); depends on `story:tls-fetch-https`, `story:tls-session-anchors`,
   `story:agent-tasks` and `story:page-text-answer-block`, besides `story:agent-follow-links` and
   `story:agent-press-submit`. The jar is carried where `tls-fetch-https` puts the anchors, on
   top of `send_through`. `Secure` is tested against the TLS test server.
4. A recording never sees a cookie (rule R17), stated and tested, not designed in the runner.
5. What an agent sees: a count, `cookies <n>`, after the `path` block (rule R18); no name, no
   value. Names are chosen by servers and some carry identifiers; an agent that needs more is
   debugging a site, which is the live view's job, and the live view gets nothing of cookies
   either.
6. Not an entity: the jar is port state of the `net` component, as the HTTP client is. The one
   field the model gains is what `SameSite` needs: the initiator of a navigation, drafted in the
   specification first, by this story's first commit.
7. Tests go into each crate's one integration-test binary (modules named under Acceptance).

## Placement

Starts after these merges: round 2 (`follow`, `press`;
`wave/a10-label-limits:docs/plan/agent-waves.md:66`), the TLS unit's `tls-fetch-https` and
`tls-session-anchors` (and so `story:security-review-fixes`, which `tls-fetch-https` starts
after), `story:agent-tasks`, and `story:page-text-answer-block` (round 1). Before round 3's runner
unit, which writes `crates/session` and `crates/surf` and wraps the `Transport` (agent-waves.md:159
moves round 3 behind items 2 and 4).

It runs beside no unit that writes `spec/`, `generated/`, `crates/fetch`, `crates/surf/src/lib.rs`,
`crates/session`, `crates/answer`, `docs/agent-answer.md`, `crates/tls-test-server` or
`crates/agent-tasks`. Writers of the same files, all before it: `story:tls-spec` (the `UNMAPPED:`
line of `net.yaml`, tls-spec.md:91, and `navigation.yaml`), `story:agent-links-spec` and
`story:agent-history-spec` (`spec/`, `generated/`), `story:tls-fetch-https` (`crates/fetch`,
`crates/surf/src/lib.rs`, `crates/session/src/lib.rs`, tls-fetch-https.md:196-211),
`story:tls-session-anchors` (`crates/session`, `docs/agent-answer.md`), `story:page-text-answer-block`
(`crates/answer`, `docs/agent-answer.md`, its "Writes"), `story:tls-test-server` (its crate,
tls-test-server.md:124-128: this story adds one accessor after it), `story:agent-tasks`
(`crates/agent-tasks`). `story:agent-skill-definition` writes none of these files and may run
beside it; it reads `docs/agent-answer.md` when its tests run.

## Change

The rules, each with the case that holds it (Acceptance):

- R1 Parsing (draft 22 §5.6): a `Set-Cookie` string with a control character other than HTAB is
  ignored (step 1); a string without `=` is a cookie with an empty name (step 3), serialised as
  its value alone (§5.8.3 step 6); an empty name whose value starts with `__Secure-` or `__Host-`
  is ignored (§5.7 step 22). Cases U6, U7.
- R2 Size: a name and value over 4096 octets together make the cookie ignored (§5.6 step 5, §5.7
  step 4); an attribute value over 1024 octets makes that attribute ignored (§5.6, attribute step
  6). Cases U5, W10.
- R3 Lifetime: dates by §5.1.1; `Max-Age` wins over `Expires` (§5.7 step 6); a value of 0 or
  less expires the cookie (§5.6.2 step 7), which deletes a stored one of the same name, domain and
  path (§5.7 step 23 and the eviction of expired cookies); a lifetime over 400 days is cut to 400
  days (§5.5, §5.6.1 step 4, §5.6.2 step 6); an `Expires` that does not parse is ignored and the
  cookie lasts as long as the jar (§5.6.1 step 2). The jar's methods take the time as an argument
  (`now: SystemTime`); the session passes the system clock, the unit tests a time they set.
  Cases U1 to U4, W8.
- R4 Domain, host-only rule (option B): the attribute loses a leading `.` and is lower-cased
  (§5.6.3). When the request host domain-matches it (§5.1.3: equal, or a suffix after a `.`, and
  for an IP address host only when equal), the cookie is stored host-only for the request host;
  otherwise the cookie is ignored (§5.7 step 10). So a cookie that `a.example.test` sets with
  `Domain=example.test` is sent to `a.example.test` only: not to the sibling `b.example.test`, not
  to `example.test`. Cases W4, W5.
- R5 Path: a missing attribute, or one that does not start with `/`, gives the default-path of the
  request address (§5.6.4, §5.1.4); a cookie is sent where its path path-matches (§5.1.4). Cases
  U5, W6.
- R6 Secure: surf counts only the scheme `https:` as a "secure" connection (§5.7 step 13, §5.8.3
  step 3); `localhost` is not trusted for it. A `Secure` cookie from an `http:` response is
  ignored; it is sent only to `https:` addresses; a cookie without `Secure` from an `http:`
  response is ignored when a `Secure` cookie of the same name, a domain-matching domain and a
  path its path matches exists (§5.7 step 16). Cases X1 to X4.
- R7 HttpOnly is stored and sent in `Cookie` (§5.6.6, §5.8.3 step 3 excludes it only from a
  "non-HTTP" retrieval, which surf has none of). Case W9.
- R8 SameSite (§5.6.7, §5.7 steps 17 to 19, §5.8.3 step 3): `Strict`, `Lax` and `None` matched
  without regard to case; absent or any other value gives the enforcement "Default". A `None`
  cookie without `Secure` is ignored (step 19). Storing: every navigation of surf is top-level
  (one tab, no frames), so step 18 never ignores a cookie a navigation receives. Sending: a
  same-site request sends every cookie that matches; a cross-site request sends `None` cookies
  always, `Lax` and "Default" cookies only on a hop whose method is `GET` (§5.6.7.1: the method
  of the current hop), and `Strict` cookies never. surf does not apply "Lax-allowing-unsafe"
  (§5.6.7.2). Cases U9, W13, S4.
- R9 Same site and the initiator (§5.2: the request's URL against its client's "site for
  cookies"): a request is same-site when its navigation has no initiator (`go`, `open`, `back`,
  `forward`: a person typing or using the browser's controls, the request "has no client") or when
  the hop's address has the same scheme and the same host as the initiator's origin, ports
  ignored. `follow` and `press` carry the origin of the document they act on. Every hop of a
  redirect keeps the navigation's initiator; its own address and method decide. Cases U10, W13,
  S4.
- R10 Prefixes (§5.4, §5.7 steps 20 and 21), matched without regard to case: `__Secure-` needs
  `Secure`; `__Host-` needs `Secure`, a `Path` attribute of `/` and no `Domain` attribute (under
  R4 the check reads whether a `Domain` attribute was given, as the draft's host-only flag would
  be false for one). Case U8.
- R11 Replacement (§5.7 step 23): a cookie of the same name, domain and path replaces the stored
  one and keeps its creation time. Case U11.
- R12 The `Cookie` field (§5.8.3 steps 4 and 6): longer paths first, then earlier creation;
  `name=value` pairs joined by `; `. One `Cookie` field per request, none when no cookie matches.
  Case W7.
- R13 Bounds and eviction (§5.7, "remove excess cookies", §6.1): at most 50 cookies per domain
  and 3,000 per session (the draft's minimums; with R2 at most about 12 MiB of names and values
  per session, the memory bound the crate documentation states). Expired cookies are evicted when
  the jar is used. Over a bound, the draft's priorities: expired cookies; cookies without `Secure`
  of a domain that holds more than 50; any cookie of such a domain; any cookie. Within one
  priority the earliest last-access time goes first, then (surf's choice where the draft says
  nothing) the earliest creation. Creation and last-access times are ticks of the jar's own
  counter, one per stored cookie and one per retrieval, so the order is total. Cases U12, U13,
  W12.
- R14 Field bounds: the `Cookie` field value sent is at most 8,192 bytes; cookies are dropped from
  the end of R12's order until it fits, and no field says so. A `Set-Cookie` field over the header
  limit ends the fetch as every over-long field does (`MalformedResponse`), and nothing of that
  response is stored. Cases W11, W14.
- R15 One jar per session, in memory: dropped at `close`, when the process ends and when it dies;
  never written to a file, never shared between sessions. Cases S3, S5.
- R16 Never printed: no answer, entry, event, view frame, file or `Debug` text holds a cookie's
  name or value; `Debug` of the jar is `Jar { cookies: <n> }`. Cases U14, S6.
- R17 Below the `Transport`: the jar is used inside `fetch_http`, so the `RequestStarted` and the
  `Answer` that pass the `Transport` (which `RecordingTransport` records) never hold a cookie.
  Case T1.
- R18 `cookies <n>` in the answer: one line after the `path` block and before the `note` lines,
  `<n>` the cookies the jar holds when the answer is made, expired ones evicted first; no line when
  it holds none. Cases A1, A2, S2, S3.
- R19 Redirects: the `Set-Cookie` fields of every response of a chain are stored for that
  response's address before the next hop is sent; each hop sends the cookies of its own address.
  Cases W1, W2, X3, S1.

Where it is written:

- `spec/domains/navigation.yaml` (`Navigate`, line 85; `NavigationStarted`, line 261) and
  `spec/domains/net.yaml` (`Fetch`, line 125; `RequestStarted`, line 260): the field `initiator:
  Optional<String>`, the serialised origin (`http://127.0.0.1:8080`) of the document that started
  the navigation; `spec/domains/agent.yaml`: the bindings of `Follow` and `Press` pass the current
  document's origin, those of `Go`, `OpenSession`, `GoBack` and `GoForward` none; `net.yaml` states
  R1 to R19 in a comment, as it states the redirect rules, and its marker becomes `# UNMAPPED:
  request headers and caching.`; new scenarios `spec/scenarios/navigation-initiator-*.yaml`;
  `task generate` (`generated/rust/`) and what the conformance target needs
  (`conformance/src/target.rs`).
- `crates/cookie` (new, `surf-cookie`, shipped, standard library and `surf-url` only, Apache-2.0,
  `#![forbid(unsafe_code)]`): `Jar` (one `Mutex` inside, so a `&Jar` is shared), `Jar::receive(url,
  set_cookie_field, now)`, `Jar::cookie_field(url, site, method, now) -> Option<String>`,
  `Jar::count(now)`, `site(initiator, url) -> Site`, `parse_date`; the crate documentation names
  each rule's section and the memory bound.
- `crates/fetch` (`Cargo.toml`: `surf-cookie` by path): `Request` gains `jar: Option<&Jar>` and
  `initiator: Option<&str>` beside the anchors of `tls-fetch-https`; its `Debug` shows the jar as
  `Jar { cookies: <n> }`. In `send_through`, per hop: the `Cookie` field from the jar (R9, R12,
  R14) written by `exchange`; every `Set-Cookie` field of the head kept (within the header limit)
  and given to the jar before the next hop (R19). Existing tests change only by the two new fields
  in their `Request` values.
- `crates/surf/src/lib.rs`: `fetch_http` takes one per-session context, `RequestContext { anchors,
  jar }`, in place of the anchors argument `tls-fetch-https` gave it, and passes
  `RequestStarted::initiator` on.
- `crates/session/src`: one `Jar` per session process, passed in that context; the `cookies` line
  (R18).
- `crates/answer/src/lib.rs` and `docs/agent-answer.md`, by "Adding a block"
  (docs/agent-answer.md:278-293): the line `cookies <n>` in a section "Cookies"; its place in
  "Lines" (after `path`, before the `note` lines); in "Grammar" `cookies-line = "cookies" SP
  number LF ; number from 1` and `answer = head-line [token-line] [did-line] [page-line]
  [path-block] [cookies-line] *note-line ...` (the rest as it stands then); `Answer.cookies:
  Option<u64>`, `render_cookies`, `parse_cookies` in `BLOCKS`.
- `crates/tls-test-server`: `TestServer::requests()`, the head bytes of each request it read, in
  order, so a test reads the `Cookie` field it received.
- `crates/agent-tasks`: the task `login` (below), its site `sites/login/`, its block in
  `tasks.txt`, its line `login pass` in `status.txt`, the scripted agent's steps for it in module
  `tasks` (`open`; `look` for the fields labelled `"Name"` and `"Password"` and the button `"Sign
  in"`; `fill <handle> -` with `ada`, then with `lovelace`, on standard input; `press`; `look --as
  markdown`; `close`), and the fixture server's two routes: `POST
  /login` with the body `name=ada&password=lovelace` answers `303`, `Location: /account.html` and
  `Set-Cookie: sid=4f1c2a9e7b3d5f6081a2c3d4e5f60718; Path=/; HttpOnly; SameSite=Lax`; `GET
  /account.html` answers the account page only with `Cookie: sid=4f1c2a9e7b3d5f6081a2c3d4e5f60718`,
  and `303` to `/login.html` otherwise. The block: `task login`, `server http`, `start
  /login.html`, `target /account.html`, `connections 3`, `commands open look fill fill press look
  close`, `question On which day is the next delivery?`, `answer Tuesday`.

## Acceptance

`slot task check` exits 0. The folder's bounds hold: the test step ends within 120 s, no test runs
over 60 s, no model call and no network request in the test step (every server is the test's own
on 127.0.0.1 or 127.0.0.2; names resolve through the test's `Network`), nothing expensive is made
per test, no Python. The new tests add at most 10 s to the test step; each over 5 s is named with
its time in the report. T is 1795305600 (2026-11-22T00:00:00Z). "receives `Cookie: X`" means the
request head the server read holds exactly the line `Cookie: X`; "no `Cookie`" means it holds no
line whose name is `Cookie` in any case.

Unit cases, `crates/cookie/tests/main.rs`, module `jar` (`Jar` called directly, the time given):

- U1 `parse_date`: `Sun, 06 Nov 1994 08:49:37 GMT`, `Sunday, 06-Nov-94 08:49:37 GMT` and `Sun Nov
  6 08:49:37 1994` give 784111777; `06 Nov 1600 08:49:37` (year below 1601), `31 Feb 2030
  08:49:37` (no such date) and `06 Nov 1994 24:00:00` fail.
- U2 From `http://h.test/` at T, `m=1; Max-Age=60; Expires=Thu, 01 Jan 1970 00:00:01 GMT`:
  `cookie_field(http://h.test/)` is `m=1` at T+59 and none at T+61. `n=1; Max-Age=-1`: none at T.
  `o=1; Max-Age=1x`: the attribute is ignored, `o=1` at T+10^9.
- U3 At T, `l=1; Max-Age=40000000` and `k=1; Expires=Fri, 01 Jan 2100 00:00:00 GMT`: both sent at
  1829865599, neither at 1829865601 (T plus 400 days is 1829865600).
- U4 At T, `p=1; Expires=someday`: `p=1` at T+10^9.
- U5 From `http://h.test/d/e/set`, `l=1; Path=/` followed by 1,024 `a`: sent to `/d/e/x`, not to
  `/`. `big` with a value of 4,093 bytes is stored; `huge` with 4,093 bytes is not (4,097 octets).
- U6 `Set-Cookie: abc` gives `cookie_field` `abc`; `=__Host-x` is not stored.
- U7 `a=b` U+0001 `c` is not stored.
- U8 From `https://h.test/x`: `__Secure-a=1; Secure`, `__Secure-b=1`, `__Host-d=1; Secure; Path=/`,
  `__Host-e=1; Secure; Path=/; Domain=h.test`, `__Host-f=1; Secure`, `__HOST-g=1; Secure; Path=/`,
  and from `http://h.test/` `__secure-c=1; Secure`: `cookie_field(https://h.test/x)` is
  `__Secure-a=1; __Host-d=1; __HOST-g=1`.
- U9 From `https://h.test/`: `st=1; SameSite=Strict; Secure`, `lx=1; SameSite=Lax; Secure`, `df=1;
  Secure`, `no=1; SameSite=None; Secure`, `bd=1; SameSite=Bogus; Secure`, `nn=1; SameSite=None`.
  `cookie_field(https://h.test/, Same, Get)` and `(Same, Post)`: `st=1; lx=1; df=1; no=1; bd=1`;
  `(Cross, Get)`: `lx=1; df=1; no=1; bd=1`; `(Cross, Post)`: `no=1`.
- U10 `site(None, http://a.test/)` is `Same`; `site(http://a.test:1, http://a.test:2/)` is `Same`;
  `site(http://a.test, https://a.test/)`, `site(http://a.test, http://b.a.test/)` and
  `site(http://127.0.0.1, http://127.0.0.2/)` are `Cross`.
- U11 `a=1; Path=/`, `b=1; Path=/`, then `a=2; Path=/`: `a=2; b=1`.
- U12 50 cookies `c1=1` to `c50=1` on each of `h1.test` to `h60.test`, then `x=1` on `h61.test`:
  `count` is 3000 and `cookie_field(http://h1.test/)` starts with `c2=1`.
- U13 From `https://h.test/`: `s1=1; Secure`, then `n1=1` to `n50=1`: `cookie_field` starts with
  `s1=1; n2=1` (the 51st evicts `n1`, not the older `s1`).
- U14 `format!("{jar:?}")` of a jar holding `s=1` is `Jar { cookies: 1 }`; after `m=1; Max-Age=1`
  at T, `count(T+2)` is the count without it.

Wire cases, `crates/fetch/tests/main.rs`, module `cookies`: `send_through` with a fresh jar per
case, a test `Network` that resolves `a.example.test`, `b.example.test`, `example.test` and
`localhost` to 127.0.0.1, and the module's server on 127.0.0.1 port `<p>`, which records every
request head and answers by path:

- W1 `GET http://127.0.0.1:<p>/login` answered `302`, `Location: /home`, `Set-Cookie: s=1; Path=/`:
  `/home` receives `Cookie: s=1`.
- W2 After `/set-a` (`a=1; Path=/`) and `http://a.example.test:<p>/set-b` (`b=2; Path=/`): `GET
  http://127.0.0.1:<p>/hop` answered `302` to `http://a.example.test:<p>/end`: `/hop` receives
  `Cookie: a=1`, `/end` receives `Cookie: b=2`.
- W3 After `/set-a`: `GET http://b.example.test:<p>/echo` has no `Cookie`.
- W4 `http://a.example.test:<p>/set` answers `p=1; Domain=example.test; Path=/`, `q=1;
  Domain=.EXAMPLE.test; Path=/`, `r=1; Domain=other.test; Path=/`: `http://a.example.test:<p>/echo`
  receives `Cookie: p=1; q=1`; `http://b.example.test:<p>/echo` and `http://example.test:<p>/echo`
  have no `Cookie`.
- W5 `/set` answers `ip=1; Domain=127.0.0.1; Path=/` and `ip2=1; Domain=0.0.1; Path=/`: `/echo`
  receives `Cookie: ip=1`.
- W6 `/p/set` answers `pa=1; Path=/a` and `dp=1`: `/a` and `/a/x` receive `Cookie: pa=1`; `/ab`
  and `/b` have no `Cookie`; `/p/q` receives `Cookie: dp=1`; `/pq` has none.
- W7 `/set` answers `a=1; Path=/`, `b=2; Path=/x`, `c=3; Path=/`: `/x/y` receives `Cookie: b=2;
  a=1; c=3`.
- W8 After `e=1; Path=/` and `f=1; Path=/`, `/del` answers `e=1; Max-Age=0; Path=/` and `f=1;
  Expires=Thu, 01 Jan 1970 00:00:01 GMT; Path=/`: `/echo` has no `Cookie`.
- W9 `/set` answers `h=1; HttpOnly; Path=/`: `/echo` receives `Cookie: h=1`.
- W10 `/set` answers `big=` and 4,093 `x`, `huge=` and 4,093 `y`: `/echo` receives `Cookie: big=`
  and the 4,093 `x`.
- W11 `/set` answers `a=`, `b=` and `c=`, each with 4,000 bytes (`x`, `y`, `z`) and `Path=/`:
  `/echo` receives `Cookie: a=<4,000 x>; b=<4,000 y>` (8,006 bytes of value).
- W12 `/one` answers `c1=1` to `c50=50`, each `Path=/`: `/two` receives `Cookie: c1=1; c2=2; ...;
  c50=50` and answers `c51=51; Path=/`; `/three` receives `Cookie: c2=2; ...; c50=50; c51=51`.
- W13 After `/set` (no initiator) answered `lax=1; SameSite=Lax; Path=/`, `strict=1;
  SameSite=Strict; Path=/`, `dflt=1; Path=/`, `none=1; SameSite=None; Path=/`: `GET /echo` with
  initiator `http://localhost:<p>` receives `Cookie: lax=1; dflt=1`; `POST /echo` with that
  initiator has no `Cookie`; `POST /post303` with it, answered `303` to `/echo`: the `POST` hop has
  no `Cookie`, the `GET` hop receives `Cookie: lax=1; dflt=1`; `GET /echo` with initiator
  `http://127.0.0.1:1` receives `Cookie: lax=1; strict=1; dflt=1`.
- W14 `/set` answers one `Set-Cookie` field of 70,000 bytes: `Failure::MalformedResponse`; `/echo`
  has no `Cookie`.

Secure cases, same module, against the TLS test server of `story:tls-test-server` (`localhost`
port `<t>`, the test root of `crates/tls-test-certs/certs/` as the extra anchors, the heads read
back through `TestServer::requests()`) and the plain server as `localhost` port `<p>`:

- X1 `https://localhost:<t>/set` answers `sec=1; Secure; Path=/`: `https://localhost:<t>/echo`
  receives `Cookie: sec=1`.
- X2 Then `http://localhost:<p>/echo` has no `Cookie`.
- X3 `https://localhost:<t>/down` answered `302` to `http://localhost:<p>/echo`: `/down` receives
  `Cookie: sec=1`, the `http:` hop has no `Cookie`.
- X4 `http://localhost:<p>/set` answers `s2=1; Secure; Path=/` and `sec=2; Path=/`:
  `https://localhost:<t>/echo` receives `Cookie: sec=1`.

Below the transport, `crates/surf/tests/main.rs`, module `cookies`:

- T1 `a_transport_sees_no_cookie`: a closure `Transport` calls `fetch_http` with a jar for two
  requests, `/set` (answering `s=` and 32 random hexadecimal digits) and `/echo`; `/echo` receives
  `Cookie: s=<the digits>`; the `Debug` text of both `RequestStarted` values and both `Answer`
  values holds neither `s=` nor the digits.

The answer, `crates/answer/tests/main.rs`, module `golden`:

- A1 `crates/answer/tests/golden/cookies.txt` (`surf 1 session 7f3a open`, `did look`, `page 200
  "text/html" "http://shop.example/"`, `path 1 of 1`, `  1 go "http://shop.example/"`, `cookies
  2`) renders from typed values and parses back byte for byte.
- A2 `parse` refuses `cookies 0`, `cookies 02` and a `cookies 1` line after a `note` line, each
  naming its line.

The session, `crates/session/tests/main.rs`, module `cookies`, one test
`a_session_keeps_its_cookies_to_itself`: one `surfd` in the test process (`surfd_in_process`), the
binaries of `SURF_BIN_DIR`, a page server on 127.0.0.1 port `<p>` and one on 127.0.0.2 port `<q>`
(Linux routes 127.0.0.0/8 to the loopback; the agent binaries run on Linux only):

- S1 Session A: `go http://127.0.0.1:<p>/login`, answered `302`, `Location: /home`, `Set-Cookie:
  s=<32 random hexadecimal digits>; Path=/`: `/home` receives `Cookie: s=<digits>`; exit 0.
- S2 A: `look` equals `crates/session/tests/golden/look-cookies.txt` (the `look` answer of the
  page `/home` with `cookies 1` after its `path` block, `<id>` and `<port>` in place).
- S3 Session B: `go http://127.0.0.1:<p>/home` has no `Cookie`; B's `look` has no line starting
  with `cookies`.
- S4 A: `go http://127.0.0.1:<p>/set-site` (answering `lax=1; SameSite=Lax; Path=/`, `strict=1;
  SameSite=Strict; Path=/`, `dflt=1; Path=/`), `go http://127.0.0.2:<q>/from` (a link `Echo` to
  `http://127.0.0.1:<p>/echo` and a `POST` form to it with a button `Send`), `follow` the link
  found by its label: `/echo` receives `Cookie: s=<digits>; lax=1; dflt=1`; `go` to `/from` again
  and `press` `Send`: the `POST` has no `Cookie`; `go http://127.0.0.1:<p>/echo`: `Cookie:
  s=<digits>; lax=1; strict=1; dflt=1`; `look` prints `cookies 4`.
- S5 `close` A, `open` C, C: `go http://127.0.0.1:<p>/echo` has no `Cookie`.
- S6 The digits occur nowhere in: the standard output and error of every `surf-agent` call; the
  bytes of the view's event stream from before the first `open` to the last answer and the bodies
  of `/api/sessions` and `/api/sessions/<id>` for each session (as case 16 of
  `crates/session/tests/view.rs:455-486` reads them); `format!("{:?}", daemon.session_entries(id))`
  for each session; every regular file under the test's `XDG_RUNTIME_DIR` after the last call.
  S1 shows the server received them.
- S7 No golden file of `crates/session/tests/golden/` but the new one changes.

The specification and the tasks:

- `task generate` and `task check` pass; the scenarios `navigation-initiator-follow` (a `Navigate`
  with initiator `http://a.test` gives `NavigationStarted` and `RequestStarted` with it) and
  `navigation-initiator-none` (without it, neither carries one) pass in both conformance reports;
  `task open` prints `spec/domains/net.yaml:<line>:# UNMAPPED: request headers and caching.`
- `agent-tasks-report` prints `task login pass 7 calls 3 connections` and no `raise:` line for it.

## Out of scope

- The public suffix list, domain cookies sent to sibling hosts, and "same site" by registrable
  domain: `story:agent-cookie-public-suffixes`.
- A cookie file, cookies kept after `close` or shared between sessions, a cookie command
  (`surf-agent cookies`), third-party cookies of subresources (surf loads none), `document.cookie`
  (`epic:scripts-in-a-page`), cookie partitioning (CHIPS), "Lax-allowing-unsafe".
- What a recording holds beyond R17 (`epic:e2e-scenario-runner`).

## Why `epic:agent-cli-target`

The jar is one per agent session, lives in the session process and shows in the agent answer
(`cookies <n>`), which that epic owns; cookies are item 4 of the order the operator's question led
to (D85, the branch file above). It needs `https:` only for its `Secure` cases, which use the
test server of `epic:tls-https`.
