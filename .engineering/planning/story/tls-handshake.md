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
  path: crates/tls/tests/data/client-hello-retry.golden
- confidence: cited
  path: crates/tls/tests/data/client-hello.golden
- confidence: cited
  path: crates/tls/tests/data/flights
- confidence: cited
  path: crates/tls/tests/tls/flights.rs
- confidence: cited
  path: crates/tls/tests/tls/handshake.rs
- confidence: cited
  path: crates/tls/tests/tls/handshake_hostile.rs
- confidence: cited
  path: crates/tls/tests/tls/rfc8446-client-musts.txt
- confidence: cited
  path: taskfiles/tls-flights.yml
revision: 9
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
decision 1 of rounds 1 and 2 (`docs/design/tls.md` items 7 and 12): the ClientHello also offers
the six TLS 1.2 ECDHE AEAD suites, for a server limited to TLS 1.2 that does not read
`supported_versions`; it answers with a ServerHello selecting 1.2, which surf ends with a
`protocol_version` alert and `VersionUnsupported`. A server that reads `supported_versions`
(OpenSSL 1.1.1 and later) sends a `protocol_version` alert before any ServerHello; both end
in `TlsVersionUnsupported`. A `handshake_failure` alert before any ServerHello stays
`TlsHandshakeFailed`, its detail naming the alert.

The epic, testing: "our ClientHello is held by a golden file of its own"; "recorded server
flights of an independent implementation (recorded in the `slow` lane on 127.0.0.1, replayed in
the gate with the injected randomness), so that a defect the client and the test server share is
still caught". A recording is recorded data pinned by its sha256 (item 10): the independent
server draws its own randomness, so the lane `slow` records again and compares outcomes, not
bytes. The client private key of a recording, and of the RFC 8448 trace, is made from its bytes
through a fixed random source (item 5). The RFC 8448 section 3 trace has an empty legacy session
id (rfc8448.txt:241-245) and an EncryptedExtensions with `record_size_limit` (:376-378), which
our own ClientHello neither sends nor offers: the trace runs in a test mode that takes its
ClientHellos as given bytes, over given bytes only (decision 4 of both rounds).

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
  - ServerHello, in this order: a `supported_versions` extension present that holds anything
    but 0x0304 is `illegal_parameter` and `UnofferedVersion` (section 4.2.1); with no such
    extension the server selected TLS 1.2 or below: when the last 8 bytes of its random are
    either downgrade value of section 4.1.3 (`DOWNGRD` 0x01 or 0x00) it is `illegal_parameter`
    and `Downgrade`, else `protocol_version` and `VersionUnsupported`; a TLS 1.3 ServerHello
    or HelloRetryRequest that selects one of the six TLS 1.2 suites is `illegal_parameter` and
    `Tls12SuiteInTls13`; else an offered TLS 1.3 suite, the session id echoed, compression 0.
    An alert before any ServerHello ends with `AlertReceived { description,
    before_server_hello: true }` (`protocol_version` there is the other kind of server of
    item 12).
  - HelloRetryRequest once, `cookie` echoed; the transcript as section 4.4.1. The checks of
    section 4.1.4, each `illegal_parameter` unless named otherwise: its `selected_group` must be
    in `supported_groups` and not a group the first hello sent a share for, so only secp256r1
    (`UnofferedGroup`); it must change the hello, by a `key_share` or a `cookie`
    (`HelloRetryRequestNoChange`); it must carry `supported_versions`, else `missing_extension`
    and `MissingExtension`, and that must hold 0x0304 (`UnofferedVersion`, the rule of the
    ServerHello above; so the version cannot change before the ServerHello); its suite is an
    offered TLS 1.3 suite (`UnofferedSuite`, `Tls12SuiteInTls13`), and the ServerHello that
    follows must select the same suite (`SuiteChangedAfterRetry`).
  - Then, in this order only: EncryptedExtensions (only what was offered; ALPN absent or
    `http/1.1`), a CertificateRequest at most once (below), Certificate (not empty, at most the
    entries of item 9, handed with the host and the time to the verifier), CertificateVerify
    (an offered scheme that fits the `LeafKey`, RSA only as PSS, the content of section
    4.4.3), Finished. A message out of this order (a skipped EncryptedExtensions, a skipped
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
  - The further client obligations of RFC 8446, each from the table "RFC 8446 client
    obligations" below (round 4: the class closed, not one instance more):
    - ServerHello and HelloRetryRequest: `legacy_session_id_echo` equal to ours, else
      `illegal_parameter` and `SessionIdMismatch`; compression 0, else `illegal_parameter` and
      `CompressionNotNull` (4.1.3, and for the HelloRetryRequest 4.1.4); `legacy_version` is
      ignored once `supported_versions` is present (4.2.1).
    - Key share (4.2.8): a ServerHello must carry `key_share`, else `missing_extension` and
      `MissingExtension`; its group must be one the client sent a share for, else
      `illegal_parameter` and `UnofferedKeyShare`; after a HelloRetryRequest it must be the
      group that request selected, else `illegal_parameter` and `KeyShareGroupChanged`; a share
      the agreement of the seam refuses (a P-256 point off the curve, an X25519 result of all
      zeros, a wrong length) is `illegal_parameter` and `BadKeyShare` (4.2.8.2).
    - Extensions (4.2): a recognised extension in a message the table of section 4.2 does not
      allow it in (ALPN in a ServerHello, `supported_versions` or `key_share` in
      EncryptedExtensions) is `illegal_parameter` and `ExtensionNotAllowedHere`, checked
      before the rule of unsolicited extensions; two extensions of one type in a block are
      `illegal_parameter` and `DuplicateExtension`; a `supported_groups` in
      EncryptedExtensions is read and ignored (4.2.7); an extension of a server
      CertificateEntry the ClientHello did not offer is `unsupported_extension` and
      `UnofferedExtension` (4.4.2).
    - A CertificateRequest right after EncryptedExtensions is accepted once: it must carry
      `signature_algorithms` (else `missing_extension` and `MissingExtension`), its unknown
      extensions are ignored, and the client answers it before its Finished with a Certificate
      message holding no certificates and no CertificateVerify (4.3.2, 4.4.2); surf has no
      client certificate.
    - CertificateVerify (4.4.3): a scheme not offered in `signature_algorithms`, one that does
      not fit the `LeafKey`, an RSA PKCS#1 scheme or a SHA-1 scheme is `illegal_parameter` and
      `UnofferedSignatureScheme`.
    - A `CertificateRejection` of the verifier sends a certificate alert (4.4.2.4):
      `unknown_ca` for `NotTrusted(NoPathToAnchor)`, `certificate_expired` for `Expired`,
      `bad_certificate` for every other (the seam verifies no MD5 signature, so a certificate
      signed with MD5 fails its check and ends `bad_certificate`, as 4.4.2.4 requires).
    - `KeyUpdate` (4.6.3): before the server Finished it is `unexpected_message` and
      `UnexpectedMessage`; a `request_update` other than 0 or 1 is `illegal_parameter` and
      `BadKeyUpdate`; a record under the new key before the KeyUpdate is `bad_record_mac`.
    - A message that does not parse (a length past its end, a short field) is `decode_error`
      and `Malformed { message }` (section 6).
    - Alerts (section 6): every alert of section 6.2 ends the connection whatever its level,
      and an unknown description too; after an alert sent or received the `Connection` reads
      and writes nothing more and returns that error; data after a `close_notify` is ignored;
      `Connection::close` and the drop of a `Connection` after its handshake send
      `close_notify` before the stream is shut.
- `crates/tls/src/handshake/trace.rs`, behind the feature `test-tooling` only: the test mode
  `GivenClientHello`, which takes the list of ClientHellos of a trace as bytes (RFC 8448 section
  5 has two: the first, and the second after the HelloRetryRequest with its cookie,
  renegotiation_info and `record_size_limit`, rfc8448.txt:1610-1618), the extensions each
  offered and its key shares (made from their private key bytes), and the server flight as
  bytes; it runs the state machine over those bytes only, in memory, never over a live stream,
  and sends the given hellos in their order instead of building its own. So the RFC 8448 trace
  runs with its empty session id and its `record_size_limit`. `connection.rs` has no path to
  it; no other story uses it.
- `crates/tls/src/connection.rs`: `Connection`, the client as a stream over any `Read + Write`,
  given a host, a `&dyn ServerCertVerifier` and the time; `Read` and `Write` of application data
  after the handshake; end of stream after `close_notify`; an end of stream without it after the
  handshake is `Truncated`, before its end `ConnectionClosed`; a read that times out `TimedOut`.
  Under `test-tooling`, and only then, `Connection::new` reads `TEST_TOOLING_MARKER` of item 6
  through `std::hint::black_box`, so a binary that connects carries the marker exactly when
  the feature was compiled in (`story:tls-release` searches the shipped binaries for it).
- `crates/tls/tests/data/client-hello.golden` (hex of our ClientHello with a fixed random,
  session id and key share).
- Recorded flights: `crates/tls/examples/record-flights.rs` (run only by the lane `slow`) starts
  the independent server of item 10 on 127.0.0.1 with the `ok-*` certificates of
  `crates/tls-test-certs/certs/`, and records for each configuration the bytes both ways and the
  client private key bytes: the three suites with x25519, a HelloRetryRequest to secp256r1, an
  RSA and an ECDSA certificate; `openssl s_server -tls1_2` against our hello (the
  `protocol_version` kind of item 12, nothing more is read from it); and a stack that ignores
  `supported_versions` against our hello only when item 12 names one that runs without a
  network. Output `crates/tls/tests/data/flights/<name>.bin` with `SOURCE` (the implementation
  and its version, the command line, the date, the sha256 of each recording) and `DIGEST` (the
  sha256 of the recorder sources).
- `taskfiles/tls-flights.yml`: `tls-flights-check` (the recordings against `SOURCE`, the
  recorder sources against `DIGEST`; a step of `check`) and `tls-flights-slow` (records again
  into `.out/` and requires that every new recording replays to the same outcome; in `slow`).
- Tests in `crates/tls/tests/main.rs`, modules `handshake`, `flights`, `handshake_hostile`
  (files in `tests/tls/`). The verifier in them is a test double that returns the `LeafKey` of
  the leaf of the case, read from `crates/tls-test-certs/certs/` with the reader of
  `story:tls-x509`. Each test carries the name the table below gives it.
- `crates/tls/tests/tls/rfc8446-client-musts.txt`, committed beside the tests: the table below,
  one line per row, `<section> | <obligation> | <rule> | <tests>`, where `<tests>` is a list of
  `module::function` (a trailing `*` matches every function with that prefix) or
  `not reachable: <why>`. The rows of section 5 are those of `story:tls-record`, in its file
  `rfc8446-record-musts.txt`.

## RFC 8446 client obligations

Every "MUST" of RFC 8446 (`~/.cache/rfcspec-corpus/txt/rfc8446.txt`) that binds a client, in
sections 4.1.3, 4.1.4, 4.2 (with 4.2.1, 4.2.2, 4.2.3, 4.2.7, 4.2.8 and 4.2.10 as never sent,
and the table of which extension may appear in which message), 4.3.1, 4.3.2, 4.4.2, 4.4.3,
4.4.4, 4.6.1, 4.6.2, 4.6.3, the change_cipher_spec rule of section 5, and section 6. Rows
marked "new" were missing before round 4 of the critics. `hh` is `handshake_hostile`, `hs`
`handshake`; `INDEX:<case>` is a case of `crates/tls-test-certs/certs/INDEX` (the test of
`story:tls-x509-path` for it), `wycheproof:<file>` a file of `crates/tls/tests/data/wycheproof/`
(the test of `story:tls-lock-allow` for it); both exist before this story starts.

| Section | Obligation | Rule | Test |
|---|---|---|---|
| 4.1.3 | session id echo equals ours | `illegal_parameter`, `SessionIdMismatch` (new) | `hh::session_id_not_echoed` |
| 4.1.3 | the suite was offered | `illegal_parameter`, `UnofferedSuite` | `hh::suite_not_offered` |
| 4.1.3 | a TLS 1.3 hello selects no TLS 1.2 suite | `illegal_parameter`, `Tls12SuiteInTls13` | `hh::tls12_suite_in_tls13` |
| 4.1.3 | compression method 0 | `illegal_parameter`, `CompressionNotNull` (new) | `hh::compression_not_null` |
| 4.1.3 | the random is read for the HelloRetryRequest value first | the retry branch | `hs::rfc8448_section_5` |
| 4.1.3 | neither downgrade value below TLS 1.3 | `illegal_parameter`, `Downgrade` | `hh::downgrade_tls12`, `hh::downgrade_tls11` |
| 4.1.3 | no `supported_versions`: TLS 1.2 or below | `protocol_version`, `VersionUnsupported` | `hh::tls12_server_hello` |
| 4.1.3 | renegotiation answered `protocol_version` | not reachable: surf never runs TLS 1.2 nor renegotiates | not reachable |
| 4.1.4 | the HelloRetryRequest checked as a ServerHello (version, session id, suite, compression) | the names of 4.1.3 (new for the session id) | `hh::retry_session_id_not_echoed`, `hh::tls12_suite_in_tls13` |
| 4.1.4 | the HelloRetryRequest changes the hello | `illegal_parameter`, `HelloRetryRequestNoChange` | `hh::retry_without_change` |
| 4.1.4 | a second HelloRetryRequest | `unexpected_message`, `SecondHelloRetryRequest` | `hh::second_retry` |
| 4.1.4 | HelloRetryRequest extensions offered, or `cookie` | `unsupported_extension`, `UnofferedExtension` (new case) | `hh::retry_unoffered_extension` |
| 4.1.4 | the ServerHello keeps the suite of the HelloRetryRequest | `illegal_parameter`, `SuiteChangedAfterRetry` | `hh::suite_changed_after_retry` |
| 4.1.4 | `supported_versions` in the HelloRetryRequest, kept in the ServerHello | `missing_extension`, `MissingExtension`; `UnofferedVersion` | `hh::retry_without_supported_versions`, `hh::retry_version_tls12` |
| 4.2 | no extension in an answer that was not requested | `unsupported_extension`, `UnofferedExtension` | `hh::encrypted_extensions_unoffered` |
| 4.2 | an extension only in the messages of the table | `illegal_parameter`, `ExtensionNotAllowedHere` (new) | `hh::extension_not_allowed_here` |
| 4.2 | one extension of a type per block | `illegal_parameter`, `DuplicateExtension` (new); our hello by the golden file | `hh::duplicate_extension`, `hs::client_hello_golden` |
| 4.2.1 | `supported_versions` sent with every version surf takes | 0x0304 only | `hs::client_hello_golden` |
| 4.2.1 | `supported_versions` read first, `legacy_version` then ignored | rule order (new) | `hs::legacy_version_ignored` |
| 4.2.1 | a version not offered or below TLS 1.3 | `illegal_parameter`, `UnofferedVersion` | `hh::server_hello_version_tls12` |
| 4.2.2 | the cookie copied into the second hello | the second ClientHello (new test) | `hs::retry_second_hello` |
| 4.2.2 | no cookie in a first hello | `Connection` keeps nothing between connections | `hs::client_hello_golden` |
| 4.2.3 | MD5, SHA-224, DSA and SHA-1 schemes never offered for signatures of the handshake | the schemes of the seam | `hs::client_hello_golden` |
| 4.2.3 | PSS salt as long as the digest; keys of the right type | the seam (ring), the Wycheproof vectors of `story:tls-lock-allow` | `wycheproof:rsa_pss_*` |
| 4.2.7 | the server `supported_groups` not acted on | read and ignored (new) | `hs::server_supported_groups_ignored` |
| 4.2.8 | client shares: offered groups, their order, one per group | x25519, then secp256r1 alone after a retry | `hs::client_hello_golden`, `hs::retry_second_hello` |
| 4.2.8 | the retry group offered and not already shared | `illegal_parameter`, `UnofferedGroup` | `hh::retry_group_already_shared`, `hh::retry_group_not_offered` |
| 4.2.8 | the ServerHello share in a group the client shared | `illegal_parameter`, `UnofferedKeyShare` (new) | `hh::key_share_group_not_shared` |
| 4.2.8 | after a retry, the group the retry selected | `illegal_parameter`, `KeyShareGroupChanged` (new) | `hh::key_share_group_changed_after_retry` |
| 4.2.8 | a ServerHello carries `key_share` (no PSK offered) | `missing_extension`, `MissingExtension` (new case) | `hh::server_hello_without_key_share` |
| 4.2.8.1 | FFDHE value range | not reachable: no FFDHE group is offered | not reachable |
| 4.2.8.2 | the peer point on the curve | `illegal_parameter`, `BadKeyShare` (new) | `hh::key_share_rejected` |
| 4.2.10 | 0-RTT only with `pre_shared_key` and `early_data`; not in a second hello; no resend | not reachable: surf sends neither extension (golden file) | not reachable |
| 4.3.1 | EncryptedExtensions right after the ServerHello | `unexpected_message`, `UnexpectedMessage` (new case) | `hh::certificate_before_encrypted_extensions` |
| 4.3.1 | no forbidden extension in EncryptedExtensions | `illegal_parameter`, `ExtensionNotAllowedHere` (new) | `hh::extension_not_allowed_here` |
| 4.3.2 | a CertificateRequest after EncryptedExtensions, `signature_algorithms` in it, unknown extensions ignored | accepted once; `missing_extension`, `MissingExtension` (new) | `hs::certificate_request_answered_empty`, `hh::certificate_request_without_signature_algorithms` |
| 4.3.2 | no CertificateRequest from a PSK server | not reachable: no PSK is offered | not reachable |
| 4.4.2 | a client Certificate when asked, empty without a certificate, Finished still sent | empty Certificate before Finished (new) | `hs::certificate_request_answered_empty` |
| 4.4.2 | server CertificateEntry extensions offered in the ClientHello | `unsupported_extension`, `UnofferedExtension` (new case) | `hh::certificate_entry_unoffered_extension` |
| 4.4.2 | the sender certificate first | entry 0 is the leaf for the verifier (new test) | `hs::leaf_is_first_entry` |
| 4.4.2 | RawPublicKey and OpenPGP types | not reachable: no certificate type extension is offered | not reachable |
| 4.4.2.2 | digitalSignature in the leaf keyUsage | `NotTrusted(KeyUsage)` of `story:tls-x509-path` | `INDEX:leaf-no-digital-signature` |
| 4.4.2.4 | an empty server Certificate | `decode_error`, `EmptyCertificate` | `hh::empty_certificate` |
| 4.4.2.4 | no acceptable chain: a certificate alert | `unknown_ca`, `certificate_expired`, `bad_certificate` (new) | `hh::certificate_alerts` |
| 4.4.2.4 | an MD5 signature: `bad_certificate` | the seam verifies no MD5 signature, so the check fails and `bad_certificate` is sent (new) | `hh::certificate_alerts` |
| 4.4.3 | CertificateVerify right after Certificate | `unexpected_message`, `UnexpectedMessage` | `hh::skipped_messages` |
| 4.4.3 | an offered scheme that fits the key; RSA as PSS; no SHA-1 | `illegal_parameter`, `UnofferedSignatureScheme` (new) | `hh::certificate_verify_scheme` |
| 4.4.3 | the signature verified | `decrypt_error`, `BadCertificateVerify` | `hh::bad_certificate_verify` |
| 4.4.4 | Finished verified | `decrypt_error`, `BadFinished` | `hh::bad_finished` |
| 4.4.4 | records after Finished under the application keys | the key change | `hs::rfc8448_section_3` |
| 4.6.1 | tickets: 7 days at most, unknown extensions ignored, resumption rules | not reachable for resumption: every ticket is dropped | `hh::new_session_ticket_dropped` |
| 4.6.2 | a CertificateRequest without `post_handshake_auth` | `unexpected_message`, `UnexpectedMessage` | `hh::post_handshake_messages` |
| 4.6.3 | no KeyUpdate before the server Finished | `unexpected_message`, `UnexpectedMessage` (new case) | `hh::key_update_before_finished` |
| 4.6.3 | `request_update` 0 or 1 | `illegal_parameter`, `BadKeyUpdate` (new) | `hh::key_update_bad_value` |
| 4.6.3 | answer `update_requested` before the next data; old key until the KeyUpdate | answered; a record under the new key first is `bad_record_mac` | `hh::key_update_requested` |
| 5 | one `change_cipher_spec` 0x01 dropped before the peer Finished | ignored; else `UnexpectedChangeCipherSpec` | `hs::rfc8448_section_7`, `hh::change_cipher_spec_after_finished` |
| 6 | no data after an error alert, the keys forgotten | the `Connection` ends (new case) | `hh::no_data_after_alert` |
| 6 | the alerts of 6.2 fatal whatever the level; unknown alerts are errors | `AlertReceived` (new case) | `hh::alert_levels` |
| 6 | a message that does not parse | `decode_error`, `Malformed` (new) | `hh::malformed_server_hello` |
| 6.1 | data after `close_notify` ignored | end of stream (new case) | `hh::data_after_close_notify` |
| 6.1 | `close_notify` before closing the write side | `close` and drop send it (new) | `hs::close_sends_close_notify` |
| 6.1 | `close_notify` before end of data | `Truncated` | `hh::truncated` |
| 6.2 | close at once after a fatal alert sent or received | the stream is shut | `hh::no_data_after_alert` |
  `story:tls-x509`.

## Acceptance

`cargo nextest run -p surf-tls` passes: our ClientHello equals `client-hello.golden`, RFC 8448
sections 3, 5 and 7 and every recorded flight complete or end as item 12 quotes, and each
hostile case below ends with the `Error` and the alert named.

1. ClientHello with a fixed random source: byte for byte `client-hello.golden`, which lists the
   nine suites in the order above; with a host `127.0.0.1`: no `server_name` extension.
2. RFC 8448 sections 3, 5 and 7 through `GivenClientHello`, given the ClientHellos of the trace
   as bytes (section 5: both, the first and the one after the HelloRetryRequest), the server
   flight as bytes, and the CertificateVerify check given the answer of the trace (its server
   key is RSA-1024, which the seam refuses): the client Finished equals the trace. No test runs
   the mode over a socket. `cargo tree -p surf-tls -e normal,build,features` (no
   dev-dependencies) shows no `test-tooling`.
3. Each recorded TLS 1.3 flight: the handshake completes, our ClientHello equals the recorded
   one, and the HTTP answer of the server decrypts. The recording against `openssl s_server
   -tls1_2`: what item 12 quotes for it, `AlertReceived { protocol_version,
   before_server_hello: true }` (the `protocol_version` kind). A recording from a stack that
   ignores `supported_versions`, when item 12 names one: `VersionUnsupported`, alert
   `protocol_version` sent.
4. Hostile, each from crafted server bytes in memory:
   1. a TLS 1.2 ServerHello without `supported_versions` and without a downgrade value:
      `VersionUnsupported`, alert `protocol_version` sent; the same with legacy version 0x0302
      (TLS 1.1): the same;
   2. a TLS 1.2 ServerHello with `DOWNGRD` 0x01 at the end of its random, and a TLS 1.1
      ServerHello with `DOWNGRD` 0x00: each `AlertSent { illegal_parameter, Downgrade }`;
   3. a ServerHello whose `supported_versions` holds 0x0303, and one that holds 0x7f1c (a
      draft version): each `AlertSent { illegal_parameter, UnofferedVersion }` (RFC 8446
      section 4.2.1);
   4. a TLS 1.3 ServerHello that selects 0xC02F, and a HelloRetryRequest that selects 0xCCA8,
      both offered as TLS 1.2 suites: each `AlertSent { illegal_parameter, Tls12SuiteInTls13 }`;
   5. `handshake_failure` as the first record: `AlertReceived { handshake_failure,
      before_server_hello: true }`; `protocol_version` as the first record: `AlertReceived {
      protocol_version, before_server_hello: true }`;
   6. suite 0x1304, not offered: `AlertSent { illegal_parameter, UnofferedSuite }`;
   7. a second HelloRetryRequest: `AlertSent { unexpected_message, SecondHelloRetryRequest }`;
   8. a HelloRetryRequest for x25519, already offered with a share: `AlertSent {
      illegal_parameter, UnofferedGroup }`; one for x448, not in `supported_groups`: the same;
   9. after a HelloRetryRequest that selected TLS_AES_128_GCM_SHA256, a ServerHello that
      selects TLS_AES_256_GCM_SHA384 (also offered): `AlertSent { illegal_parameter,
      SuiteChangedAfterRetry }` (RFC 8446 section 4.1.4);
   10. a HelloRetryRequest with neither `key_share` nor `cookie` (it would not change the
       hello): `AlertSent { illegal_parameter, HelloRetryRequestNoChange }`; a
       HelloRetryRequest without `supported_versions`: `AlertSent { missing_extension,
       MissingExtension }`; one whose `supported_versions` holds 0x0303: `AlertSent {
       illegal_parameter, UnofferedVersion }`;
   11. EncryptedExtensions with `max_fragment_length`, not offered: `AlertSent {
       unsupported_extension, UnofferedExtension }`; ALPN `h2`: `AlertSent { illegal_parameter,
       UnofferedAlpn }`;
   12. CertificateVerify right after EncryptedExtensions (Certificate skipped): `AlertSent {
       unexpected_message, UnexpectedMessage { expected: Certificate, got: CertificateVerify }
       }`; Finished right after Certificate (CertificateVerify skipped): the same with
       `expected: CertificateVerify, got: Finished`;
   13. a Certificate with an empty list: `AlertSent { decode_error, EmptyCertificate }`;
   14. a Certificate message declaring 16 MiB: `AlertSent { illegal_parameter, MessageTooLarge
       }` at its header, its body not read;
   15. a CertificateVerify with one flipped byte: `AlertSent { decrypt_error,
       BadCertificateVerify }`; a Finished with a wrong `verify_data`: `AlertSent {
       decrypt_error, BadFinished }`;
   16. a `change_cipher_spec` after the server Finished: `AlertSent { unexpected_message,
       UnexpectedChangeCipherSpec }`;
   17. after the handshake, an unrequested CertificateRequest: `AlertSent { unexpected_message,
       UnexpectedMessage }`; an EncryptedExtensions: the same;
   18. after the handshake, a `NewSessionTicket` (lifetime 7200 s, a 64-byte ticket), then
       application data `ok`, then `close_notify`: the reads return `ok` and then the end of
       stream, no alert is sent, and the client wrote no byte after its Finished (the ticket
       is dropped: no resumption, no answer);
   19. a server that sends its ServerHello and then nothing, over a stream whose reads time out:
       `TimedOut`;
   20. application data then end of stream without `close_notify`: `Truncated`; end of stream
       in the middle of the handshake: `ConnectionClosed`;
   21. `KeyUpdate` with `update_requested`: answered, and the next record opens with the new
       keys.
5. `tls-flights-check` passes; with one byte of a recording changed in a copy, it fails naming
   the file.
6. Every row of "RFC 8446 client obligations" marked new has its test, each from crafted bytes
   in memory, ending with the alert and the `Error` its rule column names; beyond that column:
   1. `hs::retry_second_hello`: after a HelloRetryRequest for secp256r1 with a 40-byte cookie,
      our second ClientHello holds that cookie, one secp256r1 share and the other extensions
      unchanged, in a record of version 0x0303, byte for byte `client-hello-retry.golden`;
   2. `hs::certificate_request_answered_empty`: a flight with a CertificateRequest (empty
      context, `signature_algorithms`, an unknown extension 0x1234): the client writes a
      Certificate with an empty list and its Finished, no CertificateVerify, and the handshake
      completes;
   3. `hs::legacy_version_ignored` (a ServerHello of `legacy_version` 0x0301 with
      `supported_versions` 0x0304), `hs::server_supported_groups_ignored` (EncryptedExtensions
      with `supported_groups` x448): each completes;
   4. `hs::leaf_is_first_entry`: the verifier double receives the entries in the order sent;
   5. `hh::certificate_alerts`: a verifier double that answers `NotTrusted(NoPathToAnchor)`,
      `Expired` and `NotTrusted(BadSignature)` makes the client send `unknown_ca`,
      `certificate_expired` and `bad_certificate`;
   6. `hh::key_share_rejected`: after a retry to secp256r1, a ServerHello share that is no
      point of P-256; and an x25519 share of 32 zero bytes: each `AlertSent {
      illegal_parameter, BadKeyShare }`;
   7. `hh::no_data_after_alert`: after `AlertReceived`, a read and a write return the same
      error and nothing more reaches the stream; `hs::close_sends_close_notify`: `close` writes
      one `close_notify` record under the application key and shuts the write side.
7. `hs::musts_rows_name_tests`: every line of `rfc8446-client-musts.txt` names, for each entry
   of its last column, a function `fn <name>(` in `crates/tls/tests/tls/<module>.rs`, a case of
   `INDEX`, a file of the Wycheproof data, or says `not reachable:` with a reason; a copy with
   one name changed fails naming the line. The file has the 63 rows of the table.

Budget: each test under 1 s; no test over 60 s, the test step within 120 s, no network (the
recordings are replayed from files; the independent server runs only in `tls-flights-slow`), no
model call. The time of the slow lane run is in the report.

## Out of scope

A TLS 1.2 handshake, resumption, PSK, 0-RTT, client certificates, post-quantum groups,
Encrypted Client Hello; path validation itself (`story:tls-x509-path`); the test server
(`story:tls-test-server`).

## Writes

`crates/tls/src/handshake/` (with `trace.rs`), `src/connection.rs`, `tests/tls/handshake.rs`,
`tests/tls/flights.rs`, `tests/tls/handshake_hostile.rs`, `tests/tls/rfc8446-client-musts.txt`,
`tests/data/client-hello.golden`, `tests/data/client-hello-retry.golden`, `tests/data/flights/`,
`examples/record-flights.rs`, `taskfiles/tls-flights.yml`. Coordinator: `mod` lines in a wave
of more than one unit, the root `Taskfile.yml`, `AGENTS.md`. Beside it run
`story:tls-x509-path` and `story:tls-anchors`.
