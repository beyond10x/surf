---
format: aep.planning-md/3
id: story:tls-handshake
kind: story
status: draft
title: surf-tls completes a TLS 1.3 client handshake and refuses every server that deviates
summary: The client state machine with HelloRetryRequest, compatibility mode, KeyUpdate and close_notify; ClientHello held by a golden file; RFC 8448 and recorded flights of an independent server replayed; each deviation ends with its alert.
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-decisions
- depends_on: story:tls-record
- depends_on: story:tls-test-certs
- depends_on: story:tls-x509
scope:
- confidence: cited
  path: crates/tls/examples/record-flights.rs
- confidence: cited
  path: crates/tls/src/connection.rs
- confidence: cited
  path: crates/tls/src/handshake
- confidence: cited
  path: crates/tls/tests/data/client-hello.golden
- confidence: cited
  path: crates/tls/tests/data/flights
- confidence: cited
  path: crates/tls/tests/flights.rs
- confidence: cited
  path: crates/tls/tests/handshake.rs
- confidence: cited
  path: crates/tls/tests/handshake_hostile.rs
- confidence: cited
  path: taskfiles/tls-flights.yml
revision: 4
---
## Context

The second half of the candidate `tls-handshake` of `epic:tls-https` (the first is
`story:tls-record`). The epic: "the handshake (ClientHello with `supported_versions`,
`key_share` for X25519, `supported_groups` X25519 and secp256r1, `signature_algorithms`,
`signature_algorithms_cert`, `server_name` (RFC 6066 section 3) and ALPN `http/1.1` (RFC 7301);
HelloRetryRequest with its `cookie` and a second key share for P-256; EncryptedExtensions,
Certificate, CertificateVerify, Finished), the middlebox compatibility mode of Appendix D.4 (a
legacy session id and a tolerated `change_cipher_spec`), and a `NewSessionTicket` read and
dropped". RFC 8446 sections 9.1 and 9.2. U3: no TLS 1.2 handshake. The coordinator, 2026-11-22,
decision 1 (`docs/design/tls.md` item 12): the ClientHello also offers the six TLS 1.2 ECDHE
AEAD suites, so that a TLS 1.2-only server answers with a ServerHello selecting 1.2, which surf
ends with a `protocol_version` alert and `VersionUnsupported`; a `handshake_failure` alert before
any ServerHello stays `TlsHandshakeFailed`, its detail naming the alert.

The epic, testing: "our ClientHello is held by a golden file of its own"; "recorded server
flights of an independent implementation (recorded in the `slow` lane on 127.0.0.1, replayed in
the gate with the injected randomness), so that a defect the client and the test server share is
still caught". A recording is recorded data pinned by its sha256 (item 10): the independent
server draws its own randomness, so the lane `slow` records again and compares outcomes, not
bytes. The client private key of a recording, and of the RFC 8448 trace, is made from its bytes
through a fixed random source (item 5). The RFC 8448 section 3 trace has an empty legacy session
id (rfc8448.txt:241-245) and an EncryptedExtensions with `record_size_limit` (:376-378), which
our own ClientHello neither sends nor offers: the trace runs in a test mode that takes its
ClientHello as given (decision 4).

`docs/design/tls.md` item 6: this story writes `handshake/` and `connection.rs` (the one
connection layer) and calls the trait `ServerCertVerifier` of `crates/tls/src/verify.rs`, owned
by `story:tls-x509`, with the time the caller passes (item 13); the `Error` kinds and violations
it names are those of item 6; item 9 the bounds.

Placement: one of the nine stories that write only `crates/tls*` and `taskfiles/tls-*`; built
after `tls-record`, `tls-x509` and `tls-test-certs`, beside `tls-x509-path` and `tls-anchors`,
beside the rounds of surf.

## Change

- `crates/tls/src/handshake/`: the client of RFC 8446 section 4.
  - ClientHello: legacy version 0x0303; random and a 32-byte legacy session id from the random
    source; the cipher suites TLS_AES_128_GCM_SHA256, TLS_AES_256_GCM_SHA384,
    TLS_CHACHA20_POLY1305_SHA256, then the six TLS 1.2 suites of item 12; compression null;
    `supported_versions` 0x0304 only, `supported_groups` x25519 and secp256r1, `key_share`
    x25519, `signature_algorithms` and `signature_algorithms_cert` (the schemes of the seam),
    `server_name` when the host is a name and not an IP literal, ALPN `http/1.1`.
  - ServerHello: a TLS 1.2 ServerHello (no `supported_versions` 0x0304) ends with
    `protocol_version` and `VersionUnsupported`, unless its random carries the downgrade value
    of section 4.1.3, which ends with `illegal_parameter` and `Downgrade`; else an offered TLS
    1.3 suite, the session id echoed, compression 0. An alert before any ServerHello ends with
    `AlertReceived { description, before_server_hello: true }`.
  - HelloRetryRequest once, for secp256r1 only, `cookie` echoed; the transcript as section
    4.4.1.
  - Then, in this order only: EncryptedExtensions (only what was offered; ALPN absent or
    `http/1.1`), Certificate (not empty, at most the entries of item 9, handed with the host and
    the time to the verifier), CertificateVerify (an offered scheme that fits the `LeafKey`, RSA
    only as PSS, the content of section 4.4.3), Finished. A message out of this order (a skipped
    Certificate, a skipped CertificateVerify) is `unexpected_message` and `UnexpectedMessage {
    expected, got }`; an empty Certificate is `decode_error` and `EmptyCertificate` (section
    4.4.2.4).
  - After the client Finished: application data; `NewSessionTicket` read and dropped;
    `KeyUpdate` received (answered when `update_requested`); `close_notify`; every other
    handshake message, a CertificateRequest among them (no `post_handshake_auth` was offered,
    section 4.6.2), is `unexpected_message` and `UnexpectedMessage`.
  - One `change_cipher_spec` of the single byte 0x01 is ignored between the first ClientHello and
    the server Finished; any other is `unexpected_message` and `UnexpectedChangeCipherSpec`.
  - Every failure sends the alert of section 6.2 and returns the `Error` of item 6.
- `crates/tls/src/handshake/trace.rs`, behind the feature `test-tooling` only: the test mode
  `GivenClientHello`, which takes a ClientHello as bytes, the extensions it offered and its key
  shares (made from their private key bytes) as given, so that the RFC 8448 trace runs with its
  empty session id and its `record_size_limit`. `connection.rs` has no path to it.
- `crates/tls/src/connection.rs`: `Connection`, the client as a stream over any `Read + Write`,
  given a host, a `&dyn ServerCertVerifier` and the time; `Read` and `Write` of application data
  after the handshake; end of stream after `close_notify`; an end of stream without it after the
  handshake is `Truncated`, before its end `ConnectionClosed`; a read that times out `TimedOut`.
- `crates/tls/tests/data/client-hello.golden` (hex of our ClientHello with a fixed random,
  session id and key share).
- Recorded flights: `crates/tls/examples/record-flights.rs` (run only by the lane `slow`) starts
  the independent server of item 10 on 127.0.0.1 with the `ok-*` certificates of
  `crates/tls-test-certs/certs/`, and records for each configuration the bytes both ways and the
  client private key bytes: the three suites with x25519, a HelloRetryRequest to secp256r1, an
  RSA and an ECDSA certificate, and `openssl s_server -tls1_2` against our hello and against
  the hello without the six TLS 1.2 suites. Output `crates/tls/tests/data/flights/<name>.bin`
  with `SOURCE` (the implementation and its version, the command line, the date, the sha256 of
  each recording) and `DIGEST` (the sha256 of the recorder sources).
- `taskfiles/tls-flights.yml`: `tls-flights-check` (the recordings against `SOURCE`, the
  recorder sources against `DIGEST`; a step of `check`) and `tls-flights-slow` (records again
  into `.out/` and requires that every new recording replays to the same outcome; in `slow`).
  For the coordinator: the include and the two lines.
- Tests in `crates/tls/tests/main.rs`, modules `handshake`, `flights`, `handshake_hostile`. The
  verifier in them is a test double that returns the `LeafKey` of the leaf of the case, read
  from `crates/tls-test-certs/certs/` with the reader of `story:tls-x509`.

## Acceptance

`cargo test -p surf-tls` passes: our ClientHello equals `client-hello.golden`, RFC 8448
sections 3, 5 and 7 and every recorded flight complete through the state machine, and each
hostile case below ends with the `Error` and the alert named.

1. ClientHello with a fixed random source: byte for byte `client-hello.golden`, which lists the
   nine suites in the order above; with a host `127.0.0.1`: no `server_name` extension.
2. RFC 8448 sections 3, 5 and 7 through `GivenClientHello`, the CertificateVerify check given
   the answer of the trace (its server key is RSA-1024, which the seam refuses): the client
   Finished equals the trace. `cargo tree -p surf-tls -e normal,build,features` (no
   dev-dependencies) shows no `test-tooling`.
3. Each recorded TLS 1.3 flight: the handshake completes, our ClientHello equals the recorded
   one, and the HTTP answer of the server decrypts. The TLS 1.2 flight against our hello:
   `VersionUnsupported`, alert `protocol_version` sent; against the hello without the six
   suites: what the server answered, as item 12 quotes it (`AlertReceived { handshake_failure,
   before_server_hello: true }` if it sent that).
4. Hostile, each from crafted server bytes in memory:
   1. a TLS 1.2 ServerHello: `VersionUnsupported`, alert `protocol_version`;
   2. a TLS 1.2 ServerHello with `DOWNGRD` 0x01 at the end of its random: `AlertSent {
      illegal_parameter, Downgrade }`;
   3. `handshake_failure` as the first record: `AlertReceived { handshake_failure,
      before_server_hello: true }`;
   4. suite 0x1304, not offered: `AlertSent { illegal_parameter, UnofferedSuite }`;
   5. a second HelloRetryRequest: `AlertSent { unexpected_message, SecondHelloRetryRequest }`;
   6. a HelloRetryRequest for x25519, already offered: `AlertSent { illegal_parameter,
      UnofferedGroup }`;
   7. EncryptedExtensions with `max_fragment_length`, not offered: `AlertSent {
      unsupported_extension, UnofferedExtension }`; ALPN `h2`: `AlertSent { illegal_parameter,
      UnofferedAlpn }`;
   8. CertificateVerify right after EncryptedExtensions (Certificate skipped): `AlertSent {
      unexpected_message, UnexpectedMessage { expected: Certificate, got: CertificateVerify } }`;
      Finished right after Certificate (CertificateVerify skipped): the same with `expected:
      CertificateVerify, got: Finished`;
   9. a Certificate with an empty list: `AlertSent { decode_error, EmptyCertificate }`;
   10. a Certificate message declaring 16 MiB: `AlertSent { illegal_parameter, MessageTooLarge
       }` at its header, its body not read;
   11. a CertificateVerify with one flipped byte: `AlertSent { decrypt_error,
       BadCertificateVerify }`; a Finished with a wrong `verify_data`: `AlertSent {
       decrypt_error, BadFinished }`;
   12. a `change_cipher_spec` after the server Finished: `AlertSent { unexpected_message,
       UnexpectedChangeCipherSpec }`;
   13. after the handshake, an unrequested CertificateRequest: `AlertSent { unexpected_message,
       UnexpectedMessage }`; an EncryptedExtensions: the same;
   14. a server that sends its ServerHello and then nothing, over a stream whose reads time out:
       `TimedOut`;
   15. application data then end of stream without `close_notify`: `Truncated`; end of stream
       in the middle of the handshake: `ConnectionClosed`;
   16. `KeyUpdate` with `update_requested`: answered, and the next record opens with the new
       keys.
5. `tls-flights-check` passes; with one byte of a recording changed in a copy, it fails naming
   the file.

Budget: each test under 1 s; no test over 60 s, the test step within 120 s, no network (the
recordings are replayed from files; the independent server runs only in `tls-flights-slow`), no
model call. The time of the slow lane run is in the report.

## Out of scope

A TLS 1.2 handshake, resumption, PSK, 0-RTT, client certificates, post-quantum groups,
Encrypted Client Hello; path validation itself (`story:tls-x509-path`); the test server
(`story:tls-test-server`).

## Writes

`crates/tls/src/handshake/` (with `trace.rs`), `src/connection.rs`, `tests/handshake.rs`,
`tests/flights.rs`, `tests/handshake_hostile.rs`, `tests/data/client-hello.golden`,
`tests/data/flights/`, `examples/record-flights.rs`, `taskfiles/tls-flights.yml`. Coordinator:
`mod` lines in a wave of more than one unit, the root `Taskfile.yml`, `AGENTS.md`. Beside it run
`story:tls-x509-path` and `story:tls-anchors`.
