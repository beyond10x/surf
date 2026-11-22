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
revision: 3
---
## Context

The second half of the candidate `tls-handshake` of `epic:tls-https` (the first is
`story:tls-record`). The epic: "the handshake (ClientHello with `supported_versions`,
`key_share` for X25519, `supported_groups` X25519 and secp256r1, `signature_algorithms`,
`signature_algorithms_cert`, `server_name` (RFC 6066 section 3) and ALPN `http/1.1` (RFC 7301);
HelloRetryRequest with its `cookie` and a second key share for P-256; EncryptedExtensions,
Certificate, CertificateVerify, Finished), the middlebox compatibility mode of Appendix D.4 (a
legacy session id and a tolerated `change_cipher_spec`), and a `NewSessionTicket` read and
dropped". RFC 8446 section 9.1 and 9.2 (mandatory suites, signatures, groups, extensions).
U3: TLS 1.2 is out; such a server gives `TlsVersionUnsupported`.

The epic, testing: "our ClientHello is held by a golden file of its own"; "recorded server
flights of an independent implementation (recorded in the `slow` lane on 127.0.0.1, replayed in
the gate with the injected randomness), so that a defect the client and the test server share is
still caught"; the replay runs the key agreement through a test double that answers the
recorded shared secret (`docs/design/tls.md` item 5). Item 6: the verifier interface (this
story calls it; `story:tls-x509-path` implements the real one, and `story:tls-fetch-https`
joins them), the seams for random, key agreement and signature check. Item 7: the error kind of
each case. Item 9: the bounds. Item 10: the independent server (default `openssl s_server` of
the host) and how recordings are pinned.

## Change

- `crates/tls/src/handshake/`: the client of RFC 8446 section 4. ClientHello: legacy version
  0x0303, random and a 32-byte legacy session id from the random seam (`surf-secret` by
  default), the three suites, `supported_versions` 0x0304 only, `supported_groups` x25519 and
  secp256r1, `key_share` x25519, `signature_algorithms` and `signature_algorithms_cert` (the
  schemes of the seam), `server_name` when the host is a name and not an IP literal, ALPN
  `http/1.1`. HelloRetryRequest once, for secp256r1 only, `cookie` echoed. ServerHello: 0x0304
  in `supported_versions` (else `TlsVersionUnsupported`), an offered suite, the session id
  echoed, the downgrade values of section 4.1.3 refused. EncryptedExtensions: only what was
  offered; ALPN absent or `http/1.1`. Certificate: at most the entries of item 9, handed with
  the host to the verifier. CertificateVerify: an offered scheme that fits the key, RSA only as
  PSS, the content of section 4.4.3. Finished checked, client Finished sent. After it:
  application data, `NewSessionTicket` read and dropped, `KeyUpdate` received (answered when
  `update_requested`), `close_notify`. One `change_cipher_spec` of the single byte 0x01 is
  ignored between the first ClientHello and the server Finished; any other is
  `unexpected_message`. Every failure sends the alert of section 6.2.
- `crates/tls/src/connection.rs`: the client as a stream over any `Read + Write`, `Read` and
  `Write` of application data after the handshake; end of stream after `close_notify`; an end
  of stream without it is the truncation error of item 7.
- `crates/tls/tests/data/client-hello.golden` (hex of our ClientHello with the injected random,
  session id and key share).
- Recorded flights: `crates/tls/examples/record-flights.rs` (run only by the slow lane) starts
  the independent server of item 10 on 127.0.0.1 with the `ok-*` certificates of
  `crates/tls-test-certs/certs/`, and records for each configuration the bytes both ways and the
  shared secret: the three suites with x25519, a HelloRetryRequest to secp256r1, an RSA and an
  ECDSA certificate, and a server limited to TLS 1.2. Output
  `crates/tls/tests/data/flights/<name>.bin` with `SOURCE` (the implementation and its version,
  the command line, the date, the sha256 of each recording) and `DIGEST` (the sha256 of the
  recorder sources). A recording is recorded data, pinned by its sha256: the independent server
  draws its own randomness, so a new recording never equals the old one byte for byte; the slow
  lane records again and requires that every new recording replays to the same outcome.
- `taskfiles/tls-flights.yml`: `tls-flights-check` (the recordings against `SOURCE`, the
  recorder sources against `DIGEST`; a step of `check`) and `tls-flights-slow` (in `slow`). For
  the coordinator: the include and the two lines.
- Tests in `crates/tls/tests/main.rs`, modules `handshake`, `flights`, `handshake_hostile`. The
  verifier in them is a test double that returns the leaf key of the case, read from
  `crates/tls-test-certs/certs/`.

## Acceptance

`cargo test -p surf-tls` passes: our ClientHello equals `client-hello.golden`, RFC 8448
sections 3, 5 and 7 and every recorded flight complete through the state machine, and each
hostile case below ends with the error and the alert named.

1. ClientHello with the injected values: byte for byte `client-hello.golden`; with a host
   `127.0.0.1`: no `server_name` extension.
2. RFC 8448 sections 3, 5 and 7 through the state machine, the CertificateVerify check given the
   answer of the trace (its server key is RSA-1024, which the seam refuses): the client Finished
   equals the trace.
3. Each recorded flight: the handshake completes, our ClientHello equals the recorded one, and
   the HTTP answer of the server decrypts; the TLS 1.2 recording ends `TlsVersionUnsupported`.
4. Hostile, each from crafted server bytes in memory: a ServerHello without
   `supported_versions` (TLS 1.2 form): `TlsVersionUnsupported`, alert `protocol_version`;
   0x0304 with the downgrade value `DOWNGRD` 0x01 in its random: `illegal_parameter`; suite
   0x1304, not offered: `illegal_parameter`; a second HelloRetryRequest: `unexpected_message`;
   a HelloRetryRequest for x25519, already offered: `illegal_parameter`; an EncryptedExtensions
   holding `max_fragment_length`, not offered: `unsupported_extension`; ALPN `h2`:
   `illegal_parameter`; a Certificate message declaring 16 MiB: refused at its header,
   `TlsHandshakeFailed`, its body not read; a CertificateVerify with one flipped byte:
   `decrypt_error`; a Finished with a wrong `verify_data`: `decrypt_error`; a
   `change_cipher_spec` after the server Finished: `unexpected_message`; a server that sends its
   ServerHello and then nothing, over a stream whose reads time out: the timeout error, which
   item 7 maps to `Unreachable`; application data then end of stream without `close_notify`:
   the truncation error; `KeyUpdate` with `update_requested`: answered, and the next record
   opens with the new keys.
5. `tls-flights-check` passes; with one byte of a recording changed in a copy, it fails naming
   the file.

Budget: each test under 1 s; no test over 60 s, the test step within 120 s, no network (the
recordings are replayed from files; the independent server runs only in `tls-flights-slow`), no
model call. The time of the slow lane run is in the report.

## Out of scope

TLS 1.2, resumption, PSK, 0-RTT, client certificates, post-quantum groups, Encrypted Client
Hello; path validation itself (`story:tls-x509-path`); the test server
(`story:tls-test-server`).

## Writes

`crates/tls/src/handshake/`, `src/connection.rs`, `tests/handshake.rs`, `tests/flights.rs`,
`tests/handshake_hostile.rs`, `tests/data/client-hello.golden`, `tests/data/flights/`,
`examples/record-flights.rs`, `taskfiles/tls-flights.yml`. Coordinator: `mod` lines in a wave of
more than one unit, the root `Taskfile.yml`, `AGENTS.md`. Beside it can run
`story:tls-x509-path` and `story:tls-anchors`.
