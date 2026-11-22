---
format: aep.planning-md/3
id: story:tls-fetch-https
kind: story
status: active
title: 'surf-fetch and the surf command load https: addresses and follow redirects between http: and https:'
summary: The TLS client over Timed inside surf-fetch, the five failures mapped to their reasons, redirects across schemes, surf --ca-file and --anchors; end-to-end against the test server.
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-decisions
- depends_on: story:tls-spec
- depends_on: story:tls-x509-path
- depends_on: story:tls-anchors
- depends_on: story:tls-test-server
- depends_on: story:one-test-binary-per-crate
scope:
- confidence: cited
  path: conformance/Cargo.lock
- confidence: cited
  path: crates/fetch
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: cited
  path: crates/surf-cli/Cargo.toml
- confidence: cited
  path: crates/surf-cli/src/main.rs
- confidence: cited
  path: crates/surf-cli/tests
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: crates/surf/tests
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:24:30Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"verification":1}}}
- {from: "proposed", to: "active", at: "2026-11-22T06:24:30Z", actor: "human:operator", revision: 10, decided_on: {"recorded":{"verification":1}}}
---
## Context

`crates/fetch/src/lib.rs`: "It does not do HTTPS or any scheme other than `http:` (a redirect
to one is `Failure::UnsupportedScheme`)" ("What it does not do"); "An address that does not
start with `http:` is `Failure::UnsupportedScheme` and no connection is attempted"
("Addresses"). `send` (line 357) connects (`connect`, line 750) and calls `exchange` (line 493)
with a `Timed` (line 600), the `Read + Write` over the `TcpStream` that holds the deadlines,
and follows redirects in one loop. The connect timeout bounds resolving and connecting, the read
timeout everything after the connection is made, each summed over the hops. The epic: "A TLS
stream wraps `Timed` and is handed to `exchange` unchanged, so the HTTP code stays one."

`story:agent-fetch-redirects`, case 10 (`crates/fetch/tests/redirects.rs`): `Location:
https://...` ends `UnsupportedScheme` and the listener accepts no connection; the epic turns it
into a followed redirect against the TLS test server. `crates/surf/src/lib.rs` `answer` (lines
152-172) maps each `surf_fetch::Failure` to the `FailureReason` of the same name;
`fetch_http` (line 124) is the transport of `surf-cli` (`crates/surf-cli/src/main.rs:41`) and
`surf-session` (`crates/session/src/lib.rs:138`), so a new parameter changes both callers.
`docs/surf-command.md` as `story:tls-spec` wrote it. `docs/design/tls.md` items 6 (`Connection`
of `connection.rs`, `PathVerifier`, `Anchors`), 7 (the reasons and their detail text,
revocation), 8 (`--ca-file`, `--anchors`), 9 (the bounds), 13 (the caller passes the time:
`surf` and the session pass the system clock).

The destination rule for redirects: `story:security-review-fixes` (branch
`impl/security-review-fixes`, not in this store) adds it to the same redirect loop of
`crates/fetch/src/lib.rs`. This story starts after that branch is merged into `main`, applies
the rule to every hop, `http:` and `https:` alike, and does not design it (epic, "Out of
scope"). The coordinator confirms the merge before the unit starts.

The folder rule of one integration-test binary per crate: `crates/fetch` has 8 test files,
`crates/surf` 12, `crates/surf-cli` 6 (`story:one-test-binary-per-crate`, an edge); the new
tests go into `tests/main.rs` of each, as modules.

Placement: the second of the four stories built as one unit (`tls-spec`, `tls-fetch-https`,
`tls-session-anchors`, `tls-release`), at a boundary between two rounds of surf, never beside a
unit that writes `crates/fetch`, `crates/session`, `crates/surf/src/lib.rs` or the other paths
of that rule (`story:tls-decisions`, "Order of the stories of the epic").

## Domain relations

- Request to connection: one connection per hop, owned by the fetch and closed after its
  response or its redirect head; a redirect opens the next. Inferable, inferred from
  `crates/fetch/src/lib.rs:143` ("every fetch opens and closes one connection") and `:74`
  ("its connection is closed after its header section"); no `ess/1` document declares this
  relation.
- `surf.net.FailureReason` crosses to `navigation` and `agent` unconverted
  (`spec/domains/agent.yaml:131`, `failure_reason: Optional<surf.net.FailureReason>`).

## Change

- `crates/fetch` (`Cargo.toml`: `surf-tls` by path, without `test-tooling`): `https:`
  addresses, port 443 by default; after the connection, made through the `Network` of
  `story:security-review-fixes` with its destination rule checked before it, as for `http:`,
  `surf_tls::Connection` over `Timed`,
  with the host, a `PathVerifier` over the anchors the request borrows, and the system clock;
  the handshake counts against the read timeout as the exchange does, summed over the hops; the
  stream is handed to `exchange` unchanged. `Request` borrows the anchors to use
  (`surf_tls::Anchors`, bundled plus an extra file); `fetch(address, limits)` uses the bundled
  ones. `Failure` gains `CertificateNotTrusted`, `CertificateExpired`,
  `CertificateNameMismatch`, `TlsVersionUnsupported` and `TlsHandshakeFailed`, mapped from
  `surf_tls::Error` as item 7 says, each with the detail text of item 7. Redirects from `http:`
  to `https:` and back are followed, each hop under the destination rule. The crate
  documentation: the first paragraph, "Addresses", "Limits" (the handshake inside the read
  timeout, the bounds of item 9), "Memory" (what a TLS connection adds, measured with the
  counting allocator of the module `adversary_redirect_memory` as the crate measures today), "What
  it does not do" (TLS 1.2, client certificates, resumption, and: "It does not check
  revocation: a revoked certificate that has not expired loads.").
- `crates/surf/src/lib.rs`: `answer` maps the five failures; `fetch_http` takes the anchors to
  use. Its callers `crates/surf-cli/src/main.rs` and `crates/session/src/lib.rs` pass them (the
  session the bundled set; `tls-session-anchors` adds the extra file).
- `crates/surf-cli/src/main.rs`: `--ca-file <path>` and `--anchors` as `docs/surf-command.md`
  states them.
- `crates/surf-cli/Cargo.toml`: `surf-tls-test-server` and `surf-tls-test-certs` as
  dev-dependencies (both with `surf-tls` and its `test-tooling`); so a `cargo test` of
  `surf-cli` builds the binary `surf` with `test-tooling`, which `story:tls-release` keeps out
  of the shipped binaries. `crates/fetch/Cargo.toml` gains the same two dev-dependencies.
- `conformance/Cargo.lock`: `surf-fetch` gains `surf-tls`, and the packages of ring enter it;
  the lock check does not read this lock.
- The module `redirects` of the test binary of `crates/fetch` (from
  `crates/fetch/tests/redirects.rs`), case 10: rewritten as case 11 below.

## Acceptance

`cargo nextest run -p surf-fetch -p surf -p surf-cli -p surf-session` passes with the cases below:
`surf --ca-file <root> https://localhost:<port>/` against the test server commits the page, and
each negative certificate case ends with its reason.

Each against `surf-tls-test-server` on 127.0.0.1 port 0, with the certificates of
`crates/tls-test-certs/certs/` and the root of the case as the extra anchor, on the system clock
(the certificates run to 2125, `expired` ended in 2001), in `crates/fetch/tests/main.rs` module
`https`:

1. `https://localhost:<port>/` (`ok-ecdsa-p256`): 200, the body, `url` equal to the address;
   the server saw `server_name` `localhost` and ALPN `http/1.1`.
2. `https://127.0.0.1:<port>/` (`ok-rsa2048`, iPAddress entry): 200; the server saw no
   `server_name`.
3. `expired`: `CertificateExpired`, its `Display` naming notAfter 2001-01-01.
4. `wrong-host`: `CertificateNameMismatch`, its `Display` naming `localhost`.
5. `ok-rsa2048` without the extra anchor: `CertificateNotTrusted`, its `Display` naming the
   check (no path to an anchor).
6. The three kinds of a server limited to TLS 1.2 (item 7): mode `Tls12ReadsSupportedVersions`
   (kind (a), a `protocol_version` alert before any ServerHello): `TlsVersionUnsupported`;
   mode `Tls12IgnoresSupportedVersions` (kind (b), a `handshake_failure` alert before any
   ServerHello): `TlsHandshakeFailed`, its `Display` holding the fixed words of item 7 (`the
   server sent handshake_failure before its ServerHello; a server limited to TLS 1.2 answers
   this way`); mode `SelectsTls12` (kind (c), a TLS 1.2 ServerHello): `TlsVersionUnsupported`.
7. Mode `BadFinished`: `TlsHandshakeFailed`, its `Display` naming `decrypt_error`.
8. Mode `StallAfterServerHello` (a server that never finishes), read timeout 1 s:
   `Unreachable` within 2 s; mode `NeverAnswer`: the same.
9. Mode `NoCloseNotify` with a body delimited by the end of the connection: `Unreachable`;
   with `Content-Length`: 200.
10. `https://127.0.0.1:1/`: `Unreachable`, no handshake.
11. `http:` `/a` with `Location: https://localhost:<tls-port>/b`: 200 from the TLS server,
    `url` `https://localhost:<tls-port>/b` (case 10 of `redirects.rs`, rewritten).
12. `https:` `/a` with `Location: http://127.0.0.1:<port>/b`: 200, `url` the `http:` address.
13. A chain of 20 redirects alternating `http:` and `https:`: the 200 at the 21st request; one
    more: `TooManyRedirects`.
14. `http:` to an `https:` server of the case `wrong-host`: `CertificateNameMismatch`.
15. `Location: ftp://127.0.0.1:<port2>/`: `UnsupportedScheme`, and that listener accepted no
    connection.
16. Read timeout 1 s over an `https:` hop then an `http:` hop, each answering after 600 ms:
    `Unreachable` within 2 s.
17. The destination rule on an `https:` hop (`story:security-review-fixes`, through its
    `Network` trait and the `TestNetwork` of the `redirects` module): `https://a.example.test/a`
    resolved to 203.0.113.5 and routed to the TLS test server (case `ok-wildcard`), answering
    `Location: https://10.0.0.1/b`: `ForbiddenDestination`; the test network dialed only
    `203.0.113.5:443` and the server counted one connection and one handshake. A public
    `http:` start redirected to `https://127.0.0.1:<tls-port>/`: `ForbiddenDestination`, and
    the TLS server accepted no connection, so no handshake began.

In `crates/surf-cli/tests/main.rs` module `https`:

18. `surf --ca-file <root.pem> https://localhost:<port>/`: exit 0, lines `navigation <id>
    Committed https://localhost:<port>/` and `request <id> Responded https://localhost:<port>/`.
19. Without `--ca-file`: exit 1, `fetch-failed <id> CertificateNotTrusted
    https://localhost:<port>/`, standard error naming the check.
20. `surf --anchors`: exit 0, `anchors <n> bundled <date> sha256:<hex>` with the count and date
    of `crates/tls/src/anchors/SOURCE`; with `--ca-file <root.pem>` also `anchors 1 file
    <root.pem>`.
21. Hostile: `--ca-file` naming a missing file, and one holding a `PRIVATE KEY` block: exit 2,
    standard error naming the path and the error, no connection made.

Elsewhere:

22. `crates/surf/tests/main.rs`: each of the five new failures maps to the reason of the same
    name.
23. `cargo test --locked --manifest-path conformance/Cargo.toml` exits 0 with the new
    `conformance/Cargo.lock`.
24. `cargo tree -p surf-cli -e normal,build,features` lists `surf-tls` without `test-tooling`;
    `cargo tree -p surf-cli -e dev,features` lists it with `test-tooling` (the reason for the
    order of steps in `story:tls-release`).
25. `grep -c "does not check revocation" crates/fetch/src/lib.rs` prints at least 1.

Budget: each test under 5 s; the cases together add at most 10 s to the test step (both in the
report); no test over 60 s, the test step within 120 s; 127.0.0.1 only, no network, no model
call.

## Out of scope

The destination rule itself (`story:security-review-fixes`); cookies and `Secure`
(`story:agent-cookie-jar`); the extra file of an agent session (`story:tls-session-anchors`);
HTTP/2; a TLS 1.2 handshake.

## Writes

`crates/fetch/` (`src/lib.rs`, `Cargo.toml`, the modules `https` (new) and `redirects` of its
one test binary, the `mod` line in `tests/main.rs`), `crates/surf/src/lib.rs`, a module of the
test binary of `crates/surf`, `crates/surf-cli/src/main.rs`, `crates/surf-cli/Cargo.toml`
(dev-dependencies), the module `https` of the test binary of `crates/surf-cli`,
`crates/session/src/lib.rs` (the call of `fetch_http`), `conformance/Cargo.lock`.
`crates/fetch/src/lib.rs` is also written by `story:security-review-fixes` (before this one,
another branch), `story:fetch-early-answer` and `story:fetch-head-memory` (drafts);
`crates/surf/src/lib.rs` and `crates/session/src/lib.rs` by `story:agent-form-port`,
`story:agent-history-spec` and other round-2 stories: kept apart by the placement, whose path
list names `crates/fetch`. `crates/*/tests/`: `story:one-test-binary-per-crate`, before.
Coordinator: `Cargo.lock`, `AGENTS.md` ("Layout" of `crates/fetch`, "Commands": only `http:`
loads), `README.md`.
