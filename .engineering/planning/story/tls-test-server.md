---
format: aep.planning-md/3
id: story:tls-test-server
kind: story
status: active
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
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:24:29Z", actor: "human:operator", revision: 8, decided_on: {"recorded":{"verification":1}}}
- {from: "proposed", to: "active", at: "2026-11-22T06:24:29Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"verification":1}}}
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
feature `test-tooling`; the `Error` kinds; the public modules; `LeafKey` and the reader owned by
`story:tls-x509`), 7 (the error of each mode, and the three kinds of a server limited to TLS
1.2) and 12 (our hello offers the three TLS 1.3 suites only; the six TLS 1.2 suites were taken
out on 2026-11-22 after the fifth critic round, RFC 8446 section 9.3). Kind (a), a server that
reads `supported_versions`, answers `protocol_version` before any ServerHello; kind (b), one
that ignores it, finds no suite it knows in our hello and answers `handshake_failure` before
any ServerHello; kind (c), a ServerHello below TLS 1.3, is what no real server sends to our
hello and is a hostile mode. All three are tested end to end here and in
`story:tls-fetch-https`.
The certificates and keys are those of `crates/tls-test-certs/certs/` (`story:tls-test-certs`).

Placement: the last of the nine stories that write only `crates/tls*`; built alone after
`tls-handshake`, beside the rounds of surf. The four-story unit (`tls-spec` first) waits for it.

## Change

- `crates/tls-test-server` (`surf-tls-test-server`, new, test tooling, not shipped, Apache-2.0,
  `#![forbid(unsafe_code)]`, depends on `surf-tls` by path with `test-tooling`): the server side
  of RFC 8446 section 4 as far as the tests need, built from the public modules of `surf-tls`
  that item 6 lists (`record`, `key_schedule`, `transcript`, which serve either side, and the
  signing of `crypto`); the server messages (ServerHello, EncryptedExtensions, Certificate,
  CertificateVerify, Finished, NewSessionTicket, alerts) are written in this crate. Nothing
  under `crates/tls/src` is made public or changed for it (item 6, `tls-decisions` acceptance
  6); a need found there is a finding for the coordinator. `TestServer::start(Config)` binds
  127.0.0.1 port 0, serves one HTTP/1.1 response per path from a map (status, fields, body),
  counts the connections it accepts, and on drop stops and joins its thread. `Config`: the
  certificate case, the scheme it signs its CertificateVerify with (one the client offered in
  `signature_algorithms`), the suites and groups it accepts, whether it asks for secp256r1 by
  HelloRetryRequest, the server name and ALPN it saw (read back by the test), and a mode:
  `Normal` (sends one `NewSessionTicket` after its Finished, as servers commonly do);
  the two kinds of a real server limited to TLS 1.2 of item 7, answering as such a server does:
  `Tls12ReadsSupportedVersions` (kind (a): a `protocol_version` alert before any ServerHello,
  as OpenSSL 1.1.1 and later limited to 1.2 answer) and `Tls12IgnoresSupportedVersions` (kind
  (b): it chooses by `legacy_version` and holds TLS 1.2 suites only, so our hello, which offers
  none, gets a `handshake_failure` alert before any ServerHello); the hostile mode
  `SelectsTls12` (kind (c): whatever the hello, a TLS 1.2 ServerHello selecting 0xC02F without
  a downgrade value); `StallAfterServerHello`; `NeverAnswer` (accepts and reads, sends
  nothing); `OversizedCertificate` (declares 16 MiB); `BadFinished`; `NoCloseNotify` (closes
  the connection after a body delimited by its end).
- Tests in `crates/tls-test-server/tests/main.rs`, against `surf_tls::Connection` with a
  verifier test double that returns the `LeafKey` of the case leaf. No test uses the trace mode
  `GivenClientHello` (it runs over given bytes only, `story:tls-handshake`).

## Acceptance

`cargo nextest run -p surf-tls-test-server` passes: the `surf-tls` client completes a handshake with
the server for each suite and group, and each misbehaving mode ends the client with the `Error`
`docs/design/tls.md` item 6 names.

1. For each of the three suites, with x25519 and with a HelloRetryRequest to secp256r1: the
   client completes, sends `GET / HTTP/1.1`, reads the body `ok`; the server counted one
   connection and saw `server_name` `localhost` and ALPN `http/1.1` (six tests).
2. Every scheme of our `signature_algorithms` completes a handshake, the server signing its
   CertificateVerify with it, one test per scheme, each under 1 s (`docs/design/tls.md` item
   14, RFC 8446 section 9.3): ecdsa_secp256r1_sha256 (`ok-ecdsa-p256`),
   ecdsa_secp384r1_sha384 (`ok-ecdsa-p384`), ed25519 (`ok-ed25519`), rsa_pss_rsae_sha256,
   rsa_pss_rsae_sha384 and rsa_pss_rsae_sha512 (each `ok-rsa2048`): six tests; in each the
   client reads the body `ok` and the test reads back the scheme the server used. The schemes
   of `signature_algorithms_cert` are certificate signatures, checked by path validation and
   not by this verifier double: `story:tls-x509-path` acceptance 1.
3. Modes, each with a read timeout of 200 ms on the client stream:
   - `Tls12ReadsSupportedVersions` (kind (a)): `AlertReceived { protocol_version,
     before_server_hello: true }`, no ServerHello sent;
   - `Tls12IgnoresSupportedVersions` (kind (b)): `AlertReceived { handshake_failure,
     before_server_hello: true }`, no ServerHello sent, and the `Display` of the error holds the
     fixed words of item 7;
   - `SelectsTls12` (kind (c)): a TLS 1.2 ServerHello selecting 0xC02F (the `ok-rsa2048` case)
     and the client ends `VersionUnsupported`, having sent `protocol_version`;
   - `Normal` sends a `NewSessionTicket` after its Finished: the client reads the body `ok`
     and sends no byte in answer to the ticket;
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

`crates/tls-test-server/` (its `Cargo.toml` with `autotests = false` and `[[test]] name = "main"`, tests in `tests/main.rs`). Coordinator: `Cargo.lock`, `AGENTS.md` ("Layout"). Read later by
`story:tls-fetch-https`, `story:tls-session-anchors`, `story:agent-tasks-https` and
`story:agent-cookie-jar`; `story:agent-cookie-jar` also writes it after this story merged (it adds
`TestServer::requests()`).
