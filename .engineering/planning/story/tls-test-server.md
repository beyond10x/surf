---
format: aep.planning-md/3
id: story:tls-test-server
kind: story
status: draft
title: A TLS 1.3 test server on 127.0.0.1 for end-to-end tests, with the misbehaviours the negative cases need
summary: crates/tls-test-server serves HTTP over TLS 1.3 with the generated certificates, and on order answers TLS 1.2, stalls, sends an oversized Certificate, a bad Finished or no close_notify.
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-decisions
- depends_on: story:tls-handshake
- depends_on: story:tls-test-certs
- depends_on: story:tls-x509
scope:
- confidence: cited
  path: crates/tls-test-server
revision: 4
---
## Context

Split from the candidate `tls-handshake` of `epic:tls-https`: the epic plans "a TLS 1.3 test
server written here as test tooling (`crates/tls-test-server`, not shipped, sharing the record
layer and key schedule) for the end-to-end tests and agent task 5". It is a crate of its own
with its own outcome (a server the client completes a handshake with, and that misbehaves on
order), so it is a story of its own; the client is checked without it, from byte traces
(`story:tls-handshake`). `story:agent-tasks` (draft, `epic:agent-cli-target`) names it as the
server of task 5.

A defect the server and the client share passes every test of the server; the recorded flights
of an independent implementation and RFC 8448 (`story:tls-handshake`, `story:tls-record`) are
the answer the epic gives. `docs/design/tls.md` items 6 (signing through `crypto.rs` under the
feature `test-tooling`; the `Error` kinds; `LeafKey` and the reader owned by `story:tls-x509`),
7 (the error of each mode) and 12 (a TLS 1.2-only server answers our hello, which offers six TLS
1.2 suites, with a ServerHello selecting 1.2, and a hello without them with `handshake_failure`).
The certificates and keys are those of `crates/tls-test-certs/certs/` (`story:tls-test-certs`).

Placement: the last of the nine stories that write only `crates/tls*`; built alone after
`tls-handshake`, beside the rounds of surf. The four-story unit (`tls-spec` first) waits for it.

## Change

- `crates/tls-test-server` (`surf-tls-test-server`, new, test tooling, not shipped, Apache-2.0,
  `#![forbid(unsafe_code)]`, depends on `surf-tls` by path with `test-tooling`): the server side
  of RFC 8446 section 4 as far as the tests need, over the record layer, the key schedule and the
  signing of `surf-tls`. `TestServer::start(Config)` binds 127.0.0.1 port 0, serves one HTTP/1.1
  response per path from a map (status, fields, body), counts the connections it accepts, and on
  drop stops and joins its thread. `Config`: the certificate case, the suites and groups it
  accepts, whether it asks for secp256r1 by HelloRetryRequest, the server name and ALPN it saw
  (read back by the test), and a mode: `Normal`; `Tls12Only`, which answers as a real TLS
  1.2-only server does: a hello that offers one of the six TLS 1.2 suites gets a TLS 1.2
  ServerHello selecting it (without the downgrade value), a hello without them gets a
  `handshake_failure` alert; `StallAfterServerHello`; `NeverAnswer` (accepts and reads, sends
  nothing); `OversizedCertificate` (declares 16 MiB); `BadFinished`; `NoCloseNotify` (closes the
  connection after a body delimited by its end).
- Tests in `crates/tls-test-server/tests/main.rs`, against `surf_tls::Connection` with a
  verifier test double that returns the `LeafKey` of the case leaf.

## Acceptance

`cargo test -p surf-tls-test-server` passes: the `surf-tls` client completes a handshake with
the server for each suite and group, and each misbehaving mode ends the client with the `Error`
`docs/design/tls.md` item 6 names.

1. For each of the three suites, with x25519 and with a HelloRetryRequest to secp256r1: the
   client completes, sends `GET / HTTP/1.1`, reads the body `ok`; the server counted one
   connection and saw `server_name` `localhost` and ALPN `http/1.1` (six tests).
2. The certificate cases `ok-rsa2048`, `ok-ecdsa-p256`, `ok-ecdsa-p384`: the handshake
   completes.
3. Modes, each with a read timeout of 200 ms on the client stream:
   - `Tls12Only`: `VersionUnsupported`, the client sent `protocol_version`;
   - `Tls12Only` against a `GivenClientHello` without the six TLS 1.2 suites (the test mode of
     `story:tls-handshake`): `AlertReceived { handshake_failure, before_server_hello: true }`;
   - `StallAfterServerHello`: `TimedOut` within 1 s;
   - `NeverAnswer`: `TimedOut` within 1 s, no byte received;
   - `OversizedCertificate`: `AlertSent { illegal_parameter, MessageTooLarge }`;
   - `BadFinished`: `AlertSent { decrypt_error, BadFinished }`;
   - `NoCloseNotify`: `Truncated`.
4. Hostile toward the server: a client that sends 10 bytes of garbage: the server closes that
   connection and serves the next; a client that sends nothing: closed after 2 s; dropping the
   server while a connection stalls ends within 3 s with its thread joined.
5. 100 handshakes in one test in under 5 s; the time of one handshake is in the report, for the
   fetch and session stories to plan within the step.

Budget: each test under 5 s and all of them under 15 s together (times in the report); no test
over 60 s, the test step within 120 s; 127.0.0.1 only, no network, no model call.

## Out of scope

A server that ships; session tickets sent with meaning; client certificates; the fixture sites of
`story:agent-tasks`.

## Writes

`crates/tls-test-server/`. Coordinator: `Cargo.lock`, `AGENTS.md` ("Layout"). Read later by
`story:tls-fetch-https`, `story:tls-session-anchors` and `story:agent-tasks`; no other story
writes it.
