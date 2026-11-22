---
format: aep.planning-md/3
id: story:tls-fetch-https
kind: story
status: draft
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
  path: crates/surf-cli
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: crates/surf/tests
- confidence: cited
  path: crates/tls/src/connect.rs
revision: 3
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
`fetch_http` (line 124) is the transport of `surf-cli` and `surf-session`. `docs/surf-command.md`
as `story:tls-spec` wrote it (`https:`, the reasons, `--ca-file`, `--anchors`).
`docs/design/tls.md` items 6 (the verifier interface), 7 (the reasons), 8 (`--ca-file`,
`--anchors`), 9 (the bounds).

The destination rule for redirects: `story:security-review-fixes` (branch
`impl/security-review-fixes`, not in this store) adds it to the same redirect loop of
`crates/fetch/src/lib.rs`. This story starts after that branch is merged into `main`, applies
the rule to every hop, `http:` and `https:` alike, and does not design it (epic, "Out of
scope"). The coordinator confirms the merge before dispatch.

The folder rule of one integration-test binary per crate: `crates/fetch` has 8 test files,
`crates/surf` 12, `crates/surf-cli` 6 (`story:one-test-binary-per-crate`); the new tests go into
`tests/main.rs` of each, as modules.

## Domain relations

- Request to connection: one connection per hop, owned by the fetch and closed after its
  response or its redirect head; a redirect opens the next. Inferable, inferred from
  `crates/fetch/src/lib.rs:143` ("every fetch opens and closes one connection") and the
  "Redirects" item of its documentation ("its connection is closed after its header section");
  no `ess/1` document declares this relation.
- `surf.net.FailureReason` crosses to `navigation` and `agent` unconverted
  (`spec/domains/agent.yaml:131`, `failure_reason: Optional<surf.net.FailureReason>`).

## Change

- `crates/fetch` (`Cargo.toml`: `surf-tls` by path): `https:` addresses, port 443 by default;
  after the connection, the TLS client of `surf-tls` over `Timed`, with `server_name` when the
  host is a name and ALPN `http/1.1`, handed to `exchange` unchanged; the handshake counts
  against the read timeout as the exchange does, summed over the hops. `Request` borrows the
  trust to use (`surf_tls` anchors: bundled, plus an extra file); `fetch(address, limits)` uses
  the bundled anchors. `Failure` gains `CertificateNotTrusted`, `CertificateExpired`,
  `CertificateNameMismatch`, `TlsVersionUnsupported` and `TlsHandshakeFailed`, each with a
  detail naming the check, mapped as item 7 says. Redirects from `http:` to `https:` and back
  are followed, each hop under the destination rule. The crate documentation: the first
  paragraph, "Addresses", "Limits" (the handshake inside the read timeout, the bounds of item 9),
  "Memory" (what a TLS connection adds, measured as the crate measures today), "What it does not
  do" (TLS 1.2, client certificates, resumption).
- `crates/tls/src/connect.rs` (new): joins the handshake, the path validation and the anchors
  (the verifier interface of item 6) at the time of the system clock.
- `crates/surf/src/lib.rs`: `answer` maps the five failures; `fetch_http` takes the trust to
  use.
- `crates/surf-cli`: `--ca-file <path>` and `--anchors` as `docs/surf-command.md` states them.
- `conformance/Cargo.lock`: `surf-fetch` gains `surf-tls`, and the packages of ring enter it
  (`cargo test --locked --manifest-path conformance/Cargo.toml` refuses the old lock; the lock
  check does not read this lock).
- `crates/fetch/tests/redirects.rs` case 10: rewritten as below.

## Acceptance

`cargo test -p surf-fetch -p surf -p surf-cli` passes with the cases below:
`surf --ca-file <root> https://localhost:<port>/` against the test server commits the page, and
each negative certificate case ends with its reason.

Each against `surf-tls-test-server` on 127.0.0.1 port 0, with the certificates of
`crates/tls-test-certs/certs/` and the root of the case as the extra anchor (their validity runs
to 2125, so the system clock does no harm), in `crates/fetch/tests/main.rs` module `https`:

1. `https://localhost:<port>/` (`ok-ecdsa-p256`): 200, the body, `url` equal to the address;
   the server saw `server_name` `localhost` and ALPN `http/1.1`.
2. `https://127.0.0.1:<port>/` (`ok-rsa2048`, iPAddress entry): 200; the server saw no
   `server_name`.
3. `expired`: `CertificateExpired`. 4. `wrong-host`: `CertificateNameMismatch`.
5. `ok-rsa2048` without the extra anchor: `CertificateNotTrusted`.
6. Mode `Tls12Only` (a downgrade): `TlsVersionUnsupported`.
7. Mode `BadFinished`: `TlsHandshakeFailed`.
8. Mode `StallAfterServerHello` (a server that never finishes), read timeout 1 s:
   `Unreachable` within 2 s.
9. Mode `NoCloseNotify` with a body delimited by the end of the connection: `Unreachable`;
   with `Content-Length`: 200.
10. `https://127.0.0.1:1/`: `Unreachable`, no handshake.
11. `http:` `/a` with `Location: https://localhost:<tls-port>/b`: 200 from the TLS server,
    `url` `https://localhost:<tls-port>/b` (this is case 10 of `redirects.rs`, rewritten).
12. `https:` `/a` with `Location: http://127.0.0.1:<port>/b`: 200, `url` the `http:` address.
13. A chain of 20 redirects alternating `http:` and `https:`: the 200 at the 21st request; one
    more: `TooManyRedirects`.
14. `http:` to an `https:` server of the case `wrong-host`: `CertificateNameMismatch`.
15. `Location: ftp://127.0.0.1:<port2>/`: `UnsupportedScheme`, and that listener accepted no
    connection.
16. Read timeout 1 s over an `https:` hop then an `http:` hop, each answering after 600 ms:
    `Unreachable` within 2 s.

In `crates/surf-cli/tests/main.rs` module `https`:

17. `surf --ca-file <root.pem> https://localhost:<port>/`: exit 0, lines `navigation <id>
    Committed https://localhost:<port>/` and `request <id> Responded https://localhost:<port>/`.
18. Without `--ca-file`: exit 1, `fetch-failed <id> CertificateNotTrusted
    https://localhost:<port>/`.
19. `surf --anchors`: exit 0, `anchors <n> bundled <date> sha256:<hex>` with the count and date
    of `crates/tls/src/anchors/SOURCE`; with `--ca-file <root.pem>` also `anchors 1 file
    <root.pem>`.
20. Hostile: `--ca-file` naming a missing file, and one holding a `PRIVATE KEY` block: exit 2,
    standard error naming the path and the error, no connection made.

In `crates/surf/tests/main.rs`: 21. each of the five new failures maps to the reason of the same
name.

Budget: each test under 5 s; the cases together add at most 10 s to the test step (both in the
report); no test over 60 s, the test step within 120 s; 127.0.0.1 only, no network, no model
call.

## Out of scope

The destination rule itself (`story:security-review-fixes`); cookies and `Secure`
(`story:agent-cookie-jar`); the session and `surf-agent` (`story:tls-session-anchors`); HTTP/2;
TLS 1.2.

## Writes

`crates/fetch/` (`src/lib.rs`, `Cargo.toml`, `tests/https.rs`, `tests/redirects.rs`,
`tests/main.rs`), `crates/tls/src/connect.rs` with its `mod` line, `crates/surf/src/lib.rs`, a
module of `crates/surf/tests/`, `crates/surf-cli/` (source and `tests/https.rs`),
`conformance/Cargo.lock`. `crates/fetch/src/lib.rs` is also written by
`story:security-review-fixes` (before this one, another branch), `story:fetch-early-answer` and
`story:fetch-head-memory` (drafts): never in the same wave. `crates/surf/src/lib.rs`:
`story:tls-spec`, before. `crates/*/tests/`: `story:one-test-binary-per-crate`, before.
Coordinator: `Cargo.lock`, `AGENTS.md` ("Layout" of `crates/fetch`, "Commands": only `http:`
loads), `README.md`.
