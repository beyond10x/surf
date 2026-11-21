---
format: aep.planning-md/3
id: story:view-http-server
kind: story
status: implemented
title: 'The view''s HTTP server: GET only, fixed routes, Host and token checks, bounded server-sent events'
summary: crates/view serves the fixed route table of docs/view-stream.md over a source trait, standard library and surf-secret only, with its limits, the order of its checks, the Host check, the view token, the head lines and refusal bodies byte for byte, and bounded server-sent events; 33 named cases.
relations:
- decomposes: epic:live-view
- depends_on: story:view-decisions
- depends_on: story:agent-secret-primitives
scope:
- confidence: cited
  path: crates/view
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-21T02:10:31Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

`epic:live-view`: `surfd` serves the view "with a server written here: standard library only,
HTTP/1.1, `GET` only, a fixed set of routes", on 127.0.0.1, refusing another `Host`, requiring the
view token, under a content security policy. The server is the part of the view an untrusted local
client talks to: any program of any local user can connect to the port, and a page in the user's
browser on another origin can make the browser send requests to it (DNS rebinding included).

This story writes that server as a library and holds it to `docs/view-stream.md` and items 6 and 8
of `docs/design/live-view.md` (`story:view-decisions`). It is tested against a source the test
supplies; serving real sessions is `story:view-surfd-serves`. Nothing exists of it yet.

Two decisions of `docs/plan/agent-plan-revision-2.md` bind it. D25: the comparison in constant
time is `surf-secret`'s (`story:agent-secret-primitives`, crate `crates/secret`); this crate
writes none. D23: `surfd` stamps every entry with a number that increases across all sessions and
offers a reader from a number: entries after it, or the statement that the number is older than
what is kept; `/events` and `Last-Event-ID` use that number. The source this server reads from has
that shape.

Standards, all from memory, the sections relied on confirmed by this story: RFC 9112 (message
syntax), RFC 9110 (semantics: `Host`, `405` and `Allow`, `431`, `505`), the HTML Standard's
server-sent events section (the event stream format, `Last-Event-ID`), Content Security Policy
Level 3, Referrer Policy. surf claims no section complete.

`docs/view-stream.md` is now written (wave 1). Where this story and that document differ, the
document holds: the order of the head's lines, `Date` on every answer and `WWW-Authenticate:
Bearer` on a 401, the refusal body `{"error":"<word>"}`, the order of the checks, `HEAD` answered
405, a stream whose body ends when the connection closes, `session` events live only, and `reset`
for a `Last-Event-ID` that is older than what is kept, above the newest number or not a number.

## Change

`crates/view` (package `surf-view`): standard library only, `#![forbid(unsafe_code)]`; it depends
on `surf-secret` and on nothing else.

- `src/http.rs`: `serve(listener, config, source)`. The caller binds the listener; the server
  refuses to start on an address other than 127.0.0.1. `config` holds the view token, the `Host`
  value it accepts (`127.0.0.1:<port>`), the page's files as a list of path, content type and
  bytes, and the limits of `docs/view-stream.md` with their defaults. One thread per connection,
  at most the connection limit. One request per connection; every answer carries
  `Connection: close`, the stream included.
- The route table of `docs/view-stream.md`, "The route table": `/` (the configured
  `index.html`), each other page file by its exact path, `/api/sessions`, `/api/sessions/<id>`
  (`<id>` 36 characters, lower-case hexadecimal digits in groups of 8, 4, 4, 4 and 12 joined by
  `-`) and `/events`. A target is matched byte for byte: no percent-decoding, no `.` or `..`
  resolution, and no route takes a query.
- `source`: a trait the caller implements: the session list as bytes; one session as bytes or
  none; the newest number across all sessions (D23); and a reader from a number that yields every
  kept entry above it in number order and then, live, every new entry and every `session` item,
  each as `(number, event name, data)`, or says that the number is older than what is kept.
  `session` items come only live, never from the replay; the server keeps no history of its own.
- The checks, in the order of `docs/view-stream.md`, "Status codes and their order", the first
  that fails answering and nothing after it running: 1 connection limit (503, the request not
  read); 2 receive limit (closed, no answer); 3 size (431); 4 syntax (400, or 505 for a version
  `HTTP/<digit>.<digit>` other than `HTTP/1.1`): a line ending in a bare LF, a header line without
  `:` or starting with a space or a tab, a target not starting with `/`, `Content-Length` or
  `Transfer-Encoding` present, `Host` absent or twice, `Authorization` twice; 5 method (anything
  but `GET`, `HEAD` included: 405 with `Allow: GET`); 6 `Host` not exactly `127.0.0.1:<port>`
  (403); 7 route (404); 8 token, on `/api/` and `/events`, from `Authorization: Bearer <64
  lower-case hexadecimal digits>` only, compared by `surf-secret`'s comparison in constant time
  (401 with `WWW-Authenticate: Bearer`); 9 session, `/api/sessions/<id>` naming no listed session
  (404); 10 stream limit (503).
- Every refusal has the body `{"error":"<word>"}` with `Content-Type: application/json`, the word
  of the document's table: `bad-request`, `unauthorized`, `host`, `not-found`, `method`,
  `too-large`, `busy`, `version`.
- The head of every answer: the status line with the document's reason, `Content-Type`,
  `Content-Length` (absent on the stream only), `Allow: GET` on a 405 only, `WWW-Authenticate:
  Bearer` on a 401 only, the five security lines exactly as the document writes them,
  `Date: <IMF-fixdate>`, `Connection: close`; in that order, each ending in CR LF, and no other
  line: no `Access-Control-`, `Set-Cookie` or `Server` line.
- `src/sse.rs`: the stream of `docs/view-stream.md`, "The stream". No `Content-Length` and no
  chunked coding: the body ends when the connection closes. Each event the three lines `id:`,
  `event:`, `data:` ending in LF and an empty line; `: keep-alive` and an empty line at the
  heartbeat interval. Without `Last-Event-ID`: live after the newest number. With
  `Last-Event-ID: <n>`, `<n>` decimal digits of at most 20, not above the newest number, and the
  reader not saying it is older than what is kept: the replay above `<n>`, then live. Any other
  `Last-Event-ID` (older than kept, above the newest, not a number): `reset` with
  `data: {"newest":<n>}` and `id: <n>` first, then live after the newest number. A bounded queue
  per reader that closes that reader alone when full; at most the stream limit at once.

`crates/view/tests/http.rs`, the counted cases, each against a server the test starts on
127.0.0.1 port 0 with a source the test supplies, spoken to over `std::net::TcpStream`:

Routes and methods:
1. `GET /` answers 200 with the configured `index.html` and `text/html; charset=utf-8`; a
   configured `/view.js` answers `text/javascript; charset=utf-8` and a `/view.css`
   `text/css; charset=utf-8`; `/index.html` answers 404;
2. an unknown path answers 404 with the body `{"error":"not-found"}`;
3. `POST /`, `PUT /api/sessions`, `DELETE /api/sessions/<id>` and `HEAD /` each answer 405 with
   `Allow: GET` and the body `{"error":"method"}`, and the source sees no call;
4. `/api/sessions/../sessions`, `/api/%73essions`, `/api/sessions?x=1` and `/api/sessions/<id>`
   with the identity in upper case answer 404;
5. a request target in absolute form, and `*`, answer 400;
6. `HTTP/1.0` and `HTTP/2.0` answer 505 with `{"error":"version"}`; `HTTP/1` and `HTTPS/1.1`
   answer 400;
7. a `GET` with `Content-Length: 5`, and one with `Transfer-Encoding: chunked`, answer 400;
8. a header line without `:`, a header line starting with a space, a request whose lines end in a
   bare LF, and `Authorization` given twice: each 400 with `{"error":"bad-request"}`.

Token:
9. `/api/sessions` without `Authorization` answers 401 with `WWW-Authenticate: Bearer` and
   `{"error":"unauthorized"}`, and the body holds no byte of the source's answer;
10. a token of the right length differing in the first byte, one differing in the last byte, and
    the right token in upper-case hexadecimal digits: 401;
11. the right token in the query (`/api/sessions?token=…`) and not in the header: 404, because no
    route takes a query;
12. `/api/sessions/<id>` and `/events` without the token: 401;
13. the right token: 200 with the source's bytes and `application/json`; and `/` without a
    token: 200 (D18).

The order of the checks:
14. `/api/nope` without the token: 404, not 401 (route before token); `/api/sessions/<id>` of a
    session the source does not list: 401 without the token and 404 with it (session after
    token); `POST /` with `Host: evil.example`: 405 (method before `Host`); `GET /nope` with
    `Host: evil.example`: 403 (`Host` before route).

Host:
15. `Host: evil.example:<port>`: 403 with `{"error":"host"}`;
16. `Host: localhost:<port>` and `Host: 127.0.0.1:<other port>`: 403;
17. no `Host`, and two `Host` lines: 400.

Limits:
18. a request line and headers over 8,192 bytes: 431 with `{"error":"too-large"}`, then the
    connection closes;
19. 51 header lines: 431;
20. a client that sends half a request line and stops: closed with no answer within the receive
    limit (set to one second in the test) plus one second;
21. with the connection limit set to 4, a fifth connection is answered 503 with
    `{"error":"busy"}` without its request being read, then closed, and the four others are still
    answered;
22. a listener on an address other than 127.0.0.1 (0.0.0.0): `serve` refuses to start;
23. the write limit: with it set to one second, a client that requests `/api/sessions` from a
    source answering 8 MiB and reads nothing is closed within two seconds, and the open-connection
    count returns to what it was before; a stream reader that stops reading while an entry is
    being written is closed the same way.

The head:
24. with the page files and the source of the examples of `docs/view-stream.md`, the answers to
    `GET /`, `GET /view.js`, `GET /api/sessions` and `GET /api/sessions` without the token equal
    the document's examples in every byte except the 29 bytes of the `Date` value, which is an
    IMF-fixdate; a 404 and a 405 have the same lines in the same order (the 405 with `Allow: GET`
    after `Content-Length`), and no answer of this file holds an `Access-Control-`, `Set-Cookie`
    or `Server` line.

Stream:
25. `/events` answers a head of the status line, `Content-Type: text/event-stream`, the five
    security lines, `Date` and `Connection: close`, with no `Content-Length` and no
    `Transfer-Encoding`; three entries the source yields live arrive in order with their `id:`
    lines; when the server ends the stream it closes the connection;
26. `Last-Event-ID: <number of the second>` delivers the third entry and not the first two: the
    server asks the source's reader for the entries above that number;
27. a `Last-Event-ID` the source's reader says is older than what is kept delivers
    `reset` with `data: {"newest":<n>}` and `id: <n>` first, `<n>` the source's newest number,
    then live entries;
28. `Last-Event-ID` above the newest number (`99` with the newest at 3), `abc`, and 21 digits: each
    delivers `reset` first, then live entries;
29. a `session` item the source yields live is written with `event: session` and the `id:` the
    source gives it (the newest number); a reader that reconnects with `Last-Event-ID` below that
    number receives the entries the source replays and no `session` event;
30. a reader that never reads while the source yields past the queue bound is closed, a second
    reader receives every entry, and the server's queued bytes for the first never exceed the
    bound (a counter the server exposes to tests);
31. with the stream limit set to 2, a third stream is answered 503 with `{"error":"busy"}`;
32. with the heartbeat set to one second, an idle stream receives `: keep-alive` and an empty line
    within two seconds;
33. a stream whose reader is gone ends its thread: the open-stream count returns to 0.

Surfaces written: `crates/view/Cargo.toml`, `crates/view/src/lib.rs`, `crates/view/src/http.rs`,
`crates/view/src/sse.rs`, `crates/view/tests/http.rs`. The root `Cargo.lock` gains the crate; in a
wave it is the coordinator's.

## For AGENTS.md (decision D7)

- "Layout": "`crates/view` (`surf-view`): the live view's HTTP server, standard library only:
  `GET` only, a fixed route table, the `Host` and view-token checks, server-sent events with a
  bound per reader. `docs/view-stream.md` is what it answers."

## Acceptance

`cargo test -p surf-view --test http` passes with the 33 numbered cases, each a test named after
its case, and `cargo tree -p surf-view -e normal` prints `surf-view` and `surf-secret` and no
other package.

## Out of scope

What the JSON says and where it comes from (`story:view-surfd-serves`); the page's files
(`story:view-first-page`); TLS, IPv6, answering `HEAD`, compression, ranges, persistent
connections; any route that changes something.

## Scope

Derived 2026-11-21 by `story-scoper`.

- **Primary surface:** `crates/view` — cited, the story's Change opens "`crates/view` (package `surf-view`)" and its "Surfaces written" line lists five files, all under it. The crate is new (`crates/` holds `fetch`, `ports` and `surf` only), and the root `Cargo.toml` members glob `crates/*` picks it up without an edit.
- **Files written:** `crates/view/Cargo.toml` — cited, "Surfaces written". The story says it depends on `surf-secret` and nothing else. The path form `../secret` is inferred from `AGENTS.md` Layout, where hand-written crates depend on siblings by relative path.
- **Files written:** `crates/view/src/lib.rs` — cited, "Surfaces written". It declares the modules `http` and `sse`. `story:view-surfd-serves` also writes it later (module lines).
- **Files written:** `crates/view/src/http.rs` — cited, Change item `src/http.rs`: `serve`, the config, the fixed route table, the source trait, the check order and the headers.
- **Files written:** `crates/view/src/sse.rs` — cited, Change item `src/sse.rs`: the event-stream framing, heartbeat, `Last-Event-ID` resume, `reset`, the bounded per-reader queue and the stream limit.
- **Files written:** `crates/view/tests/http.rs` — cited, "Surfaces written": the 33 named cases, spoken over `std::net::TcpStream`.
- **Files read, not written:** `docs/view-stream.md` — cited. The status codes, the five headers and the content security policy are held to it. It does not exist on this branch; `story:view-decisions` writes it.
- **Files read, not written:** `docs/design/live-view.md` items 6 and 8 — cited. They give the token path and the limits (8 KiB, 50 lines, 5 s receive, 16 connections, 8 streams, 256 entries or 1 MiB per stream queue, 15 s heartbeat, 10 s write). The file does not exist on this branch; `story:view-decisions` writes it.
- **Files read, not written:** `surf-secret` in `crates/secret` — cited. It provides `Token`, `Token::from_hex`, `Token::matches` and `equal`. The crate does not exist on this branch; `story:agent-secret-primitives` writes it. Which of `Token::matches` or `equal` the server calls is inferred.
- **Files read, not written:** `docs/plan/agent-plan-revision-2.md` D23 and D25, `docs/plan/agent-plan-revision-1.md` D7 and D16 — cited. D23 gives the shape of the source's reader and D25 puts the constant-time comparison in `surf-secret`.
- **Handed to the coordinator:** `Cargo.lock` — cited, the story says "The root `Cargo.lock` gains the crate; in a wave it is the coordinator's." The `surf-view` package, with its one dependency `surf-secret`, appears in the lock.
- **Handed to the coordinator:** `AGENTS.md` "Layout" — cited, the story's "For AGENTS.md (decision D7)" section: "`crates/view` (`surf-view`): the live view's HTTP server, standard library only: `GET` only, a fixed route table, the `Host` and view-token checks, server-sent events with a bound per reader. `docs/view-stream.md` is what it answers."
- **Handed to the coordinator:** `README.md` and the root `Taskfile.yml`: nothing — cited. The story names no README section and adds no task. `task build` and `task test` already run it through the `crates/*` glob, and the story's own check is `cargo test -p surf-view --test http`.
- **Symbols:** `surf_view::serve(listener, config, source)` — cited. The config type is inferred as a struct with these fields: view token, accepted `Host`, page files as (path, content type, bytes), and the limits with defaults.
- **Symbols:** the limits value, with the item 8 defaults, and the source trait (session list as bytes, one session as bytes or none, a reader from a number yielding `(number, event name, data)` or reporting that the number is too old) — cited. The story names both. The type names are inferred, because the story gives none.
- **Symbols:** the stream's reader type and the test counters (queued bytes per reader, open-stream count, open-connection count) in `sse.rs` and `http.rs` — cited, from cases 23, 30 and 33: "a counter the server exposes to tests". The names, and whether they are public or behind `cfg(test)`, are inferred.
- **Would collide with:** `crates/view/src/lib.rs`, which `story:view-surfd-serves` also writes (its "Surfaces written" lists "`crates/view/src/lib.rs` (module lines)") — cited. That story depends on this one, so the two cannot share a wave.
- **Would collide with:** `crates/view/src` and `crates/view/tests/`, written wholesale by `story:view-controls-and-changes` ("`crates/view/src/`, `crates/view/tests/`") — cited. It sits at the bottom of the chain `view-http-server` → `view-surfd-serves` → `view-first-page` → `view-detail-panel` → `view-now-panel` → `view-controls-and-changes`, so it cannot share a wave either.
- **Would collide with:** `story:view-first-page` (`crates/view/src/lib.rs`, module line), `story:view-now-panel` (`crates/view/src/assets.rs`, `model.rs`, `json.rs`, `tests/`) and `story:view-detail-panel` (`crates/view/src/assets.rs`) — inferred. Each writes under `crates/view`, and each is downstream of this story.
- **Would collide with:** no story outside the view chain — inferred, from scanning every story body for `crates/view/`, `surf-view`, `src/http`, `src/sse` and `tests/http`. `story:agent-secret-primitives` mentions `surf-view` only as a consumer and writes only `crates/secret/`.
- **Not yet existing when it starts:** `crates/secret` with `Token`, `equal` and `Token::from_hex`, delivered by `story:agent-secret-primitives`.
- **Not yet existing when it starts:** `docs/view-stream.md` and `docs/design/live-view.md`, delivered by `story:view-decisions`.
- **Not yet existing when it starts:** the session-identity form, which the `/api/sessions/<id>` route checks ("an identity in the form the session identity has, otherwise 404"). `docs/view-stream.md` or the surfd stories deliver it; I did not trace which. This is inferred.
- **Not yet existing when it starts:** the real source behind the source trait. `story:view-surfd-serves`, with `story:agent-session-events` for D23's numbers, delivers it. This story's tests supply their own source.
- **Confidence:** high — the story lists its five files and the crate is new, so nothing else in the tree is written. The weak spots are the exact names of the public items, and that the dependencies' interfaces are read from their stories rather than from code.
