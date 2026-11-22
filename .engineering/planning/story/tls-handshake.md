---
format: aep.planning-md/3
id: story:tls-handshake
kind: story
status: active
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
  path: crates/tls/tests/tls/rfc8446-client-fields.txt
- confidence: cited
  path: crates/tls/tests/tls/rfc8446-client-musts.txt
- confidence: cited
  path: taskfiles/tls-flights.yml
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:24:28Z", actor: "human:operator", revision: 13, decided_on: {"recorded":{"verification":1}}}
- {from: "proposed", to: "active", at: "2026-11-22T06:24:29Z", actor: "human:operator", revision: 14, decided_on: {"recorded":{"verification":1}}}
---
## Context

The second half of the candidate `tls-handshake` of `epic:tls-https` (the first is
`story:tls-record`). The epic: "the handshake (ClientHello with `supported_versions`,
`key_share` for X25519, `supported_groups` X25519 and secp256r1, `signature_algorithms`,
`signature_algorithms_cert`, `server_name` (RFC 6066 section 3) and ALPN `http/1.1` (RFC 7301);
HelloRetryRequest with its `cookie` and a second key share for P-256; EncryptedExtensions,
Certificate, CertificateVerify, Finished), the middlebox compatibility mode of Appendix D.4 (a
legacy session id and a tolerated `change_cipher_spec`), and a `NewSessionTicket` read and
dropped". RFC 8446 sections 9.1 and 9.2. U3: no TLS 1.2 handshake. The ClientHello offers the
three TLS 1.3 suites and nothing else: the six TLS 1.2 suites the coordinator took in on
2026-11-22 were taken out the same day after the fifth critic round, because RFC 8446 section
9.3 says a client supports every parameter it advertises (`docs/design/tls.md` items 7 and
12). A server limited to TLS 1.2 is reported by one of three kinds: (a) it reads
`supported_versions` and sends `protocol_version` before any ServerHello:
`TlsVersionUnsupported`; (b) it ignores the extension, finds no suite and sends
`handshake_failure` before any ServerHello: `TlsHandshakeFailed`, with the fixed detail of
item 7; (c) it sends a ServerHello below TLS 1.3: surf sends `protocol_version`,
`TlsVersionUnsupported`.

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
    TLS_CHACHA20_POLY1305_SHA256 and no other (section 9.3, item 12); compression null;
    `supported_versions` 0x0304 only, `supported_groups` x25519 and secp256r1, `key_share`
    x25519, `signature_algorithms` and `signature_algorithms_cert` with the schemes of item 14,
    `server_name` when the host is a name and not an IP literal, ALPN `http/1.1`.
  - ServerHello, in this order: a `supported_versions` extension present that holds anything
    but 0x0304 is `illegal_parameter` and `UnofferedVersion` (section 4.2.1); with no such
    extension and a `legacy_version` of 0x0303 or below the server selected TLS 1.2 or below
    (kind (c) of item 7, whatever its suite): when the last 8 bytes of its random are either
    downgrade value of section 4.1.3 (`DOWNGRD` 0x01 or 0x00) it is `illegal_parameter` and
    `Downgrade`, else `protocol_version` and `VersionUnsupported`; with no such extension and a
    `legacy_version` above 0x0303 it is `illegal_parameter` and `UnofferedVersion`; else an
    offered TLS 1.3 suite (any other suite, a TLS 1.2 one among them, is `illegal_parameter`
    and `UnofferedSuite`), the session id echoed, compression 0. An alert before any
    ServerHello ends with `AlertReceived { description, before_server_hello: true }`:
    `protocol_version` there is kind (a), `handshake_failure` kind (b), with the fixed detail
    of item 7.
  - HelloRetryRequest once, `cookie` echoed; the transcript as section 4.4.1. The checks of
    section 4.1.4, each `illegal_parameter` unless named otherwise: its `selected_group` must be
    in `supported_groups` and not a group the first hello sent a share for, so only secp256r1
    (`UnofferedGroup`); it must change the hello, by a `key_share` or a `cookie`
    (`HelloRetryRequestNoChange`); it must carry `supported_versions`, else `missing_extension`
    and `MissingExtension`, and that must hold 0x0304 (`UnofferedVersion`, the rule of the
    ServerHello above; so the version cannot change before the ServerHello); its suite is an
    offered TLS 1.3 suite (`UnofferedSuite`), and the ServerHello that follows must select the
    same suite (`SuiteChangedAfterRetry`). The second ClientHello is the first unchanged but
    for one secp256r1 share in `key_share` and the `cookie` (section 4.1.2).
  - Then, in this order only: EncryptedExtensions (only what was offered; ALPN absent or
    `http/1.1`), a CertificateRequest at most once (below), Certificate (not empty, at most the
    entries of item 9, handed with the host and the time to the verifier), CertificateVerify
    (an offered scheme that fits the `LeafKey`, RSA only as PSS, the content of section
    4.4.3), Finished. A message out of this order (a skipped EncryptedExtensions, a skipped
    Certificate, a skipped CertificateVerify) is `unexpected_message` and `UnexpectedMessage {
    expected, got }`; an empty Certificate is `decode_error` and `EmptyCertificate` (section
    4.4.2.4), one with more entries than item 9 allows `bad_certificate` and
    `TooManyCertificates`; every field of every message as the table "Messages the client
    receives" below.
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
      `illegal_parameter` and `RepeatedExtension`; a `supported_groups` in
      EncryptedExtensions is read and ignored (4.2.7); an extension of a server
      CertificateEntry the ClientHello did not offer is `unsupported_extension` and
      `UnofferedExtension` (4.4.2).
    - A CertificateRequest right after EncryptedExtensions is accepted once: it must carry
      `signature_algorithms` (else `missing_extension` and `MissingExtension`), its
      `certificate_request_context` must be empty (else `illegal_parameter` and
      `NonEmptyRequestContext`, as for the context of the server Certificate), its unknown
      extensions are ignored, and the client answers it before its Finished with a Certificate
      message holding no certificates and no CertificateVerify (4.3.2, 4.4.2); surf has no
      client certificate.
    - CertificateVerify (4.4.3): a scheme not offered in `signature_algorithms`, one that does
      not fit the `LeafKey`, an RSA PKCS#1 scheme or a SHA-1 scheme is `illegal_parameter` and
      `UnofferedSignatureScheme`.
    - A `CertificateRejection` of the verifier sends a certificate alert (4.4.2.4):
      `unknown_ca` for `NotTrusted(NoPathToAnchor)`, `certificate_expired` for `Expired`,
      `bad_certificate` for every other, `NotTrusted(Unreadable)` and an MD5 signature
      (`NotTrusted(WeakAlgorithm)`, as 4.4.2.4 requires) among them.
    - `KeyUpdate` (4.6.3): before the server Finished it is `unexpected_message` and
      `UnexpectedMessage`; a `request_update` other than 0 or 1 is `illegal_parameter` and
      `BadKeyUpdate`; after a KeyUpdate the old key is gone, so a record still protected under
      it is `bad_record_mac` and `BadRecordMac`.
    - A message that does not parse (a length past its end, a short field) is `decode_error`
      and `Malformed { message }` (section 6).
    - Alerts (section 6): every alert of section 6.2 ends the connection whatever its level
      (a warning-level `handshake_failure` too), and an unknown description too; `close_notify`
      ends the stream and `user_canceled` (section 6.1) is read and ignored; after an alert
      sent or received the `Connection` reads and writes nothing more and returns that error;
      data after a `close_notify` is ignored; `Connection::close` and the drop of a
      `Connection` after its handshake send `close_notify` before the stream is shut.
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
  RSA and an ECDSA certificate; and `openssl s_server -tls1_2` against our hello (kind (a) of
  item 12, nothing more is read from it). Output `crates/tls/tests/data/flights/<name>.bin`
  with `SOURCE` (the implementation and its version, the command line, the date, the sha256 of
  each recording) and `DIGEST` (the sha256 of the recorder sources).
- `taskfiles/tls-flights.yml`: `tls-flights-check` (the recordings against `SOURCE`, the
  recorder sources against `DIGEST`; a step of `check`) and `tls-flights-slow` (records again
  into `.out/` and requires that every new recording replays to the same outcome; in `slow`).
- Tests in `crates/tls/tests/main.rs`, modules `handshake`, `flights`, `handshake_hostile`
  (files in `tests/tls/`). The verifier in them is a test double that returns the `LeafKey` of
  the leaf of the case, read from `crates/tls-test-certs/certs/` with the reader of
  `story:tls-x509`. Each test carries the name the two tables below give it.
- Two files committed beside the tests, one line per row of the two tables below, `<section or
  message> | <what> | <rule> | <tests>`: `crates/tls/tests/tls/rfc8446-client-musts.txt` and
  `crates/tls/tests/tls/rfc8446-client-fields.txt`. In them `<tests>` is a list whose every
  entry is one of: `tests/tls/<file>.rs::<function>`, the module file written out (in the
  story the tables abbreviate `handshake_hostile.rs` as `hh` and `handshake.rs` as `hs`), the
  function named whole, no wildcard; `INDEX:<case>`, a case of
  `crates/tls-test-certs/certs/INDEX`; `wycheproof:<file name>`, a file of
  `crates/tls/tests/data/wycheproof/` named whole; or `not reachable: <why>`. The rows of
  section 5 are those of `story:tls-record`, in its file `rfc8446-record-musts.txt`.

## RFC 8446 client obligations

Every "MUST" of RFC 8446 (`~/.cache/rfcspec-corpus/txt/rfc8446.txt`) that binds a client, in
sections 4.1.2, 4.1.3, 4.1.4, 4.2 (with 4.2.1, 4.2.2, 4.2.3, 4.2.7, 4.2.8 and 4.2.10 as never
sent, and the table of which extension may appear in which message), 4.3.1, 4.3.2, 4.4.2, 4.4.3,
4.4.4, 4.6.1, 4.6.2, 4.6.3, the change_cipher_spec rule of section 5, section 6, and 9.1 to 9.3
as they bind a client (what surf offers against each: `docs/design/tls.md` item 14, no
deviation). Rows marked "new" were added in round 4 of the critics, rows marked "round 5" in
round 5. `hh` is `handshake_hostile.rs`, `hs` `handshake.rs` (the committed file writes
`tests/tls/handshake_hostile.rs::` and `tests/tls/handshake.rs::`). `INDEX:<case>` is a case of
`crates/tls-test-certs/certs/INDEX`, which `story:tls-test-certs` writes before this story
starts: the checker looks the case up in that file; the test of the case is the one of
`story:tls-x509-path`, built beside this story and landing in the same group. A
`wycheproof:<file>` is a file of `crates/tls/tests/data/wycheproof/`, which
`story:tls-lock-allow` writes with its test before this story starts.

| Section | Obligation | Rule | Test |
|---|---|---|---|
| 4.1.3 | session id echo equals ours | `illegal_parameter`, `SessionIdMismatch` (new) | `hh::session_id_not_echoed` |
| 4.1.3 | the suite was offered (a TLS 1.2 suite in a TLS 1.3 ServerHello or HelloRetryRequest among the others) | `illegal_parameter`, `UnofferedSuite` | `hh::suite_not_offered`, `hh::tls12_suite_selected` |
| 4.1.3 | compression method 0 | `illegal_parameter`, `CompressionNotNull` (new) | `hh::compression_not_null` |
| 4.1.3 | the random is read for the HelloRetryRequest value first | the retry branch | `hs::rfc8448_section_5` |
| 4.1.3 | neither downgrade value below TLS 1.3 | `illegal_parameter`, `Downgrade` | `hh::downgrade_tls12`, `hh::downgrade_tls11` |
| 4.1.3 | no `supported_versions`: TLS 1.2 or below | `protocol_version`, `VersionUnsupported` | `hh::tls12_server_hello` |
| 4.1.3 | renegotiation answered `protocol_version` | none | not reachable: surf never runs TLS 1.2 nor renegotiates |
| 4.1.4 | the HelloRetryRequest checked as a ServerHello (version, session id, suite, compression) | the names of 4.1.3 (new for the session id) | `hh::retry_session_id_not_echoed`, `hh::tls12_suite_selected` |
| 4.1.4 | the HelloRetryRequest changes the hello | `illegal_parameter`, `HelloRetryRequestNoChange` | `hh::retry_without_change` |
| 4.1.4 | a second HelloRetryRequest | `unexpected_message`, `SecondHelloRetryRequest` | `hh::second_retry` |
| 4.1.4 | HelloRetryRequest extensions offered, or `cookie` | `unsupported_extension`, `UnofferedExtension` (new case) | `hh::retry_unoffered_extension` |
| 4.1.4 | the ServerHello keeps the suite of the HelloRetryRequest | `illegal_parameter`, `SuiteChangedAfterRetry` | `hh::suite_changed_after_retry` |
| 4.1.4 | `supported_versions` in the HelloRetryRequest, kept in the ServerHello | `missing_extension`, `MissingExtension`; `UnofferedVersion` | `hh::retry_without_supported_versions`, `hh::retry_version_tls12` |
| 4.2 | no extension in an answer that was not requested | `unsupported_extension`, `UnofferedExtension` | `hh::encrypted_extensions_unoffered` |
| 4.2 | an extension only in the messages of the table | `illegal_parameter`, `ExtensionNotAllowedHere` (new) | `hh::extension_not_allowed_here` |
| 4.2 | one extension of a type per block | `illegal_parameter`, `RepeatedExtension` (new; renamed in round 5); our hello by the golden file | `hh::repeated_extension`, `hs::client_hello_golden` |
| 4.2.1 | `supported_versions` sent with every version surf takes | 0x0304 only | `hs::client_hello_golden` |
| 4.2.1 | `supported_versions` read first, `legacy_version` then ignored | rule order (new) | `hs::legacy_version_ignored` |
| 4.2.1 | a version not offered or below TLS 1.3 | `illegal_parameter`, `UnofferedVersion` | `hh::server_hello_version_tls12` |
| 4.2.2 | the cookie copied into the second hello | the second ClientHello (new test) | `hs::retry_second_hello` |
| 4.2.2 | no cookie in a first hello | `Connection` keeps nothing between connections | `hs::client_hello_golden` |
| 4.2.3 | MD5, SHA-224, DSA and SHA-1 schemes never offered for signatures of the handshake | the schemes of the seam | `hs::client_hello_golden` |
| 4.2.3 | PSS salt as long as the digest; keys of the right type | the seam (ring), the Wycheproof vectors of `story:tls-lock-allow` | `wycheproof:rsa_pss_2048_sha256_mgf1_32_test.json` |
| 4.2.7 | the server `supported_groups` not acted on | read and ignored (new) | `hs::server_supported_groups_ignored` |
| 4.2.8 | client shares: offered groups, their order, one per group | x25519, then secp256r1 alone after a retry | `hs::client_hello_golden`, `hs::retry_second_hello` |
| 4.2.8 | the retry group offered and not already shared | `illegal_parameter`, `UnofferedGroup` | `hh::retry_group_already_shared`, `hh::retry_group_not_offered` |
| 4.2.8 | the ServerHello share in a group the client shared | `illegal_parameter`, `UnofferedKeyShare` (new) | `hh::key_share_group_not_shared` |
| 4.2.8 | after a retry, the group the retry selected | `illegal_parameter`, `KeyShareGroupChanged` (new) | `hh::key_share_group_changed_after_retry` |
| 4.2.8 | a ServerHello carries `key_share` (no PSK offered) | `missing_extension`, `MissingExtension` (new case) | `hh::server_hello_without_key_share` |
| 4.2.8.1 | FFDHE value range | none | not reachable: no FFDHE group is offered |
| 4.2.8.2 | the peer point on the curve | `illegal_parameter`, `BadKeyShare` (new) | `hh::key_share_rejected` |
| 4.2.10 | 0-RTT only with `pre_shared_key` and `early_data`; not in a second hello; no resend | none | not reachable: surf sends neither extension (golden file) |
| 4.3.1 | EncryptedExtensions right after the ServerHello | `unexpected_message`, `UnexpectedMessage` (new case) | `hh::certificate_before_encrypted_extensions` |
| 4.3.1 | no forbidden extension in EncryptedExtensions | `illegal_parameter`, `ExtensionNotAllowedHere` (new) | `hh::extension_not_allowed_here` |
| 4.3.2 | a CertificateRequest after EncryptedExtensions, `signature_algorithms` in it, unknown extensions ignored | accepted once; `missing_extension`, `MissingExtension` (new) | `hs::certificate_request_answered_empty`, `hh::certificate_request_without_signature_algorithms` |
| 4.3.2 | no CertificateRequest from a PSK server | none | not reachable: no PSK is offered |
| 4.4.2 | a client Certificate when asked, empty without a certificate, Finished still sent | empty Certificate before Finished (new) | `hs::certificate_request_answered_empty` |
| 4.4.2 | server CertificateEntry extensions offered in the ClientHello | `unsupported_extension`, `UnofferedExtension` (new case) | `hh::certificate_entry_unoffered_extension` |
| 4.4.2 | the sender certificate first | entry 0 is the leaf for the verifier (new test) | `hs::leaf_is_first_entry` |
| 4.4.2 | RawPublicKey and OpenPGP types | none | not reachable: no certificate type extension is offered |
| 4.4.2.2 | digitalSignature in the leaf keyUsage | `NotTrusted(KeyUsage)` of `story:tls-x509-path` | `INDEX:leaf-no-digital-signature` |
| 4.4.2.4 | an empty server Certificate | `decode_error`, `EmptyCertificate` | `hh::empty_certificate` |
| 4.4.2.4 | no acceptable chain: a certificate alert | `unknown_ca`, `certificate_expired`, `bad_certificate` (new) | `hh::certificate_alerts` |
| 4.4.2.4 | an MD5 signature: `bad_certificate` | `REFUSED` names MD5: `NotTrusted(WeakAlgorithm)`, `bad_certificate` (round 5) | `INDEX:md5`, `hh::certificate_alerts` |
| 4.4.3 | CertificateVerify right after Certificate | `unexpected_message`, `UnexpectedMessage` | `hh::skipped_messages` |
| 4.4.3 | an offered scheme that fits the key; RSA as PSS; no SHA-1 | `illegal_parameter`, `UnofferedSignatureScheme` (new) | `hh::certificate_verify_scheme` |
| 4.4.3 | the signature verified | `decrypt_error`, `BadCertificateVerify` | `hh::bad_certificate_verify` |
| 4.4.4 | Finished verified | `decrypt_error`, `BadFinished` | `hh::bad_finished` |
| 4.4.4 | records after Finished under the application keys | the key change | `hs::rfc8448_section_3` |
| 4.6.1 | tickets: 7 days at most, unknown extensions ignored, resumption rules | not reachable for resumption: every ticket is dropped | `hh::new_session_ticket_dropped` |
| 4.6.2 | a CertificateRequest without `post_handshake_auth` | `unexpected_message`, `UnexpectedMessage` | `hh::post_handshake_messages` |
| 4.6.3 | no KeyUpdate before the server Finished | `unexpected_message`, `UnexpectedMessage` (new case) | `hh::key_update_before_finished` |
| 4.6.3 | `request_update` 0 or 1 | `illegal_parameter`, `BadKeyUpdate` (new) | `hh::key_update_bad_value` |
| 4.6.3 | answer `update_requested` before the next data; the old key ends with the KeyUpdate | answered; a record under the old key after the KeyUpdate is `bad_record_mac`, `BadRecordMac` (round 5) | `hh::key_update_requested`, `hh::old_key_after_key_update` |
| 5 | one `change_cipher_spec` 0x01 dropped before the peer Finished | ignored; else `UnexpectedChangeCipherSpec` | `hs::rfc8448_section_7`, `hh::change_cipher_spec_after_finished` |
| 6 | no data after an error alert, the keys forgotten | the `Connection` ends (new case) | `hh::no_data_after_alert` |
| 6 | the alerts of 6.2 fatal whatever the level; unknown alerts are errors | `AlertReceived`: a warning-level `handshake_failure` (not `close_notify` nor `user_canceled`) and a fatal alert of the unknown description 0xfe (round 5: inputs named) | `hh::alert_levels` |
| 6 | a message that does not parse | `decode_error`, `Malformed` (new) | `hh::malformed_server_hello` |
| 6.1 | data after `close_notify` ignored | end of stream (new case) | `hh::data_after_close_notify` |
| 6.1 | `close_notify` before closing the write side | `close` and drop send it (new) | `hs::close_sends_close_notify` |
| 6.1 | `close_notify` before end of data | `Truncated` | `hh::truncated` |
| 6.2 | close at once after a fatal alert sent or received | the stream is shut | `hh::no_data_after_alert` |
| 4.1.2 | the second ClientHello is the first but for one share of the retry group and the cookie | (round 5) | `hs::retry_second_hello` |
| 4.1.2 | `legacy_version` 0x0303 | the golden file (round 5) | `hs::client_hello_golden` |
| 4.1.2 | a new 32-byte `legacy_session_id` in compatibility mode | from the random source (round 5) | `hs::client_hello_golden` |
| 4.1.2 | `legacy_compression_methods` exactly one byte 0 | the golden file (round 5) | `hs::client_hello_golden` |
| 4.1.2 | `supported_versions` in every TLS 1.3 ClientHello | in both hellos (round 5) | `hs::client_hello_golden`, `hs::retry_second_hello` |
| 4.4.2.4 | a certificate the reader refuses ends the handshake with a certificate alert | `NotTrusted(Unreadable)`, `bad_certificate` (round 5) | `INDEX:leaf-v1`, `hh::certificate_alerts` |
| 9.1 | TLS_AES_128_GCM_SHA256 implemented (the other two SHOULD) | all three offered and run (round 5) | `hs::client_hello_golden`, `hs::rfc8448_section_3` |
| 9.1 | rsa_pkcs1_sha256 for certificates, rsa_pss_rsae_sha256 for CertificateVerify and certificates, ecdsa_secp256r1_sha256 | verified by the seam and offered (round 5) | `INDEX:ok-rsa2048`, `INDEX:ok-rsa-pss`, `INDEX:ok-ecdsa-p256`, `wycheproof:rsa_pss_2048_sha256_mgf1_32_test.json`, `hs::client_hello_golden` |
| 9.1 | key exchange with secp256r1 (X25519 SHOULD) | x25519 shared first, secp256r1 after a retry (round 5) | `hs::retry_second_hello`, `hs::rfc8448_section_5` |
| 9.2 | `supported_versions`, `cookie`, `signature_algorithms`, `signature_algorithms_cert`, `supported_groups`, `key_share`, `server_name` implemented and sent when applicable | the golden file (round 5) | `hs::client_hello_golden` |
| 9.2 | no `pre_shared_key`: `signature_algorithms` and `supported_groups` sent, `key_share` with `supported_groups` | the golden file (round 5) | `hs::client_hello_golden` |
| 9.2 | `server_name` used | sent for a host name, never for an IP literal, as the host `127.0.0.1` case of the test shows (round 5) | `hs::client_hello_golden` |
| 9.3 | a client supports every parameter of its ClientHello | the three TLS 1.3 suites only, the six TLS 1.2 suites taken out (round 5) | `hs::client_hello_golden` |
| 9.3 | unknown extensions of a CertificateRequest or NewSessionTicket ignored | ignored (round 5) | `hs::certificate_request_answered_empty`, `hh::new_session_ticket_dropped` |

## Messages the client receives

Every message the client can receive, field by field (round 5): the values accepted, and for
every other value the violation and the alert sent (all of `docs/design/tls.md` item 6). A
row whose rule is a row of the table above names the same tests. `record` is `record.rs` of
`story:tls-record` (the file writes `tests/tls/record.rs::`). Committed as
`rfc8446-client-fields.txt`, checked as the table above.

| Message | Field | Accepted; every other value | Test |
|---|---|---|---|
| ServerHello | place | the first message after a ClientHello; a second ServerHello is the last row | `hs::rfc8448_section_3` |
| ServerHello | `legacy_version` | any value with `supported_versions` (ignored); without it 0x0303 or below is kind (c): `protocol_version`, `VersionUnsupported`, or with a downgrade value in the random `illegal_parameter`, `Downgrade`; above 0x0303: `illegal_parameter`, `UnofferedVersion` | `hs::legacy_version_ignored`, `hh::tls12_server_hello`, `hh::downgrade_tls12`, `hh::downgrade_tls11`, `hh::server_hello_version_above_tls12` |
| ServerHello | `random` | any 32 bytes; the value of section 4.1.3 makes it a HelloRetryRequest (rows below) | `hs::rfc8448_section_5` |
| ServerHello | `legacy_session_id_echo` | the 32 bytes sent; other bytes: `illegal_parameter`, `SessionIdMismatch`; a length over 32: `decode_error`, `Malformed` | `hh::session_id_not_echoed`, `hh::malformed_server_hello` |
| ServerHello | `cipher_suite` | one of the three offered; any other, a TLS 1.2 suite among them: `illegal_parameter`, `UnofferedSuite` | `hh::suite_not_offered`, `hh::tls12_suite_selected` |
| ServerHello | `legacy_compression_method` | 0; other: `illegal_parameter`, `CompressionNotNull` | `hh::compression_not_null` |
| ServerHello | `supported_versions` | 0x0304; other: `illegal_parameter`, `UnofferedVersion`; absent: the `legacy_version` row | `hh::server_hello_version_tls12` |
| ServerHello | `key_share` | one valid share of a group the client shared, after a retry the group selected; absent: `missing_extension`, `MissingExtension`; another group: `illegal_parameter`, `UnofferedKeyShare` or after a retry `KeyShareGroupChanged`; a share the seam refuses: `illegal_parameter`, `BadKeyShare` | `hh::server_hello_without_key_share`, `hh::key_share_group_not_shared`, `hh::key_share_group_changed_after_retry`, `hh::key_share_rejected` |
| ServerHello | every other extension | none; one not offered (`pre_shared_key` among them): `unsupported_extension`, `UnofferedExtension`; one offered that section 4.2 does not allow here (ALPN, `server_name`): `illegal_parameter`, `ExtensionNotAllowedHere`; a type twice: `illegal_parameter`, `RepeatedExtension` | `hh::server_hello_unoffered_extension`, `hh::extension_not_allowed_here`, `hh::repeated_extension` |
| ServerHello | lengths | every field ends inside the message and the message ends with its last field; else `decode_error`, `Malformed` | `hh::malformed_server_hello` |
| HelloRetryRequest | place | once, as the first message after the first ClientHello; a second: `unexpected_message`, `SecondHelloRetryRequest` | `hh::second_retry` |
| HelloRetryRequest | `legacy_session_id_echo`, `legacy_compression_method`, lengths | as the ServerHello | `hh::retry_session_id_not_echoed`, `hh::compression_not_null`, `hh::malformed_server_hello` |
| HelloRetryRequest | `cipher_suite` | one of the three offered, else `illegal_parameter`, `UnofferedSuite`; the ServerHello after it the same suite, else `illegal_parameter`, `SuiteChangedAfterRetry` | `hh::tls12_suite_selected`, `hh::suite_changed_after_retry` |
| HelloRetryRequest | `supported_versions` | 0x0304; absent: `missing_extension`, `MissingExtension`; other: `illegal_parameter`, `UnofferedVersion` | `hh::retry_without_supported_versions`, `hh::retry_version_tls12` |
| HelloRetryRequest | `key_share` (`selected_group`) | secp256r1; x25519 (shared already) or a group not offered: `illegal_parameter`, `UnofferedGroup` | `hh::retry_group_already_shared`, `hh::retry_group_not_offered` |
| HelloRetryRequest | `cookie` | 1 byte or more, copied into the second hello; empty: `decode_error`, `Malformed` | `hs::retry_second_hello`, `hh::retry_empty_cookie` |
| HelloRetryRequest | `key_share` and `cookie` together | at least one; neither: `illegal_parameter`, `HelloRetryRequestNoChange` | `hh::retry_without_change` |
| HelloRetryRequest | every other extension | none; not offered: `unsupported_extension`, `UnofferedExtension`; a type twice: `illegal_parameter`, `RepeatedExtension` | `hh::retry_unoffered_extension`, `hh::repeated_extension` |
| EncryptedExtensions | place | right after the ServerHello; any other message there: `unexpected_message`, `UnexpectedMessage` | `hh::certificate_before_encrypted_extensions` |
| EncryptedExtensions | `server_name` | empty, only when sent; not sent: `unsupported_extension`, `UnofferedExtension`; not empty: `decode_error`, `Malformed` | `hh::server_name_not_empty` |
| EncryptedExtensions | ALPN | absent, or one name `http/1.1`; other: `illegal_parameter`, `UnofferedAlpn` | `hh::encrypted_extensions_unoffered` |
| EncryptedExtensions | `supported_groups` | read and ignored | `hs::server_supported_groups_ignored` |
| EncryptedExtensions | every other extension | none; not offered (`max_fragment_length`): `unsupported_extension`, `UnofferedExtension`; `supported_versions` or `key_share`: `illegal_parameter`, `ExtensionNotAllowedHere`; a type twice: `illegal_parameter`, `RepeatedExtension` | `hh::encrypted_extensions_unoffered`, `hh::extension_not_allowed_here`, `hh::repeated_extension` |
| CertificateRequest | place | once, right after EncryptedExtensions; elsewhere, twice or after the handshake: `unexpected_message`, `UnexpectedMessage` | `hh::certificate_request_out_of_place`, `hh::post_handshake_messages` |
| CertificateRequest | `certificate_request_context` | empty; else `illegal_parameter`, `NonEmptyRequestContext` | `hh::certificate_request_context` |
| CertificateRequest | `signature_algorithms` | present, any schemes; absent: `missing_extension`, `MissingExtension` | `hh::certificate_request_without_signature_algorithms` |
| CertificateRequest | every other extension | read and ignored, unknown ones among them | `hs::certificate_request_answered_empty` |
| Certificate | place | after EncryptedExtensions or the CertificateRequest; else `unexpected_message`, `UnexpectedMessage` | `hh::skipped_messages` |
| Certificate | `certificate_request_context` | empty; else `illegal_parameter`, `NonEmptyRequestContext` | `hh::certificate_context` |
| Certificate | `certificate_list` | 1 to the entries of item 9; none: `decode_error`, `EmptyCertificate`; more: `bad_certificate`, `TooManyCertificates`; a declared length over item 9: `illegal_parameter`, `MessageTooLarge` | `hh::empty_certificate`, `hh::too_many_certificates`, `hh::certificate_too_large` |
| Certificate | `cert_data` | DER the reader of `story:tls-x509` reads, first the leaf; refused: `bad_certificate`, `NotTrusted(Unreadable)`; a verifier rejection: the alerts of 4.4.2.4 | `hs::leaf_is_first_entry`, `INDEX:leaf-v1`, `hh::certificate_alerts` |
| Certificate | entry `extensions` | none (surf offers neither `status_request` nor `signed_certificate_timestamp`); any: `unsupported_extension`, `UnofferedExtension` | `hh::certificate_entry_unoffered_extension` |
| CertificateVerify | `algorithm` | offered in `signature_algorithms`, fits the `LeafKey`, RSA only as PSS, no SHA-1; else `illegal_parameter`, `UnofferedSignatureScheme` | `hh::certificate_verify_scheme` |
| CertificateVerify | `signature` | verifies over the content of 4.4.3; else `decrypt_error`, `BadCertificateVerify` | `hh::bad_certificate_verify` |
| Finished | `verify_data` length | the hash length of the suite; else `decode_error`, `Malformed` | `hh::finished_wrong_length` |
| Finished | `verify_data` | the value of 4.4.4; else `decrypt_error`, `BadFinished` | `hh::bad_finished` |
| NewSessionTicket | place | after the client Finished; before: `unexpected_message`, `UnexpectedMessage` | `hh::new_session_ticket_during_handshake` |
| NewSessionTicket | `ticket_lifetime`, `ticket_age_add`, `ticket_nonce` | any; dropped | `hh::new_session_ticket_dropped` |
| NewSessionTicket | `ticket` | 1 byte or more, dropped; empty: `decode_error`, `Malformed` | `hh::new_session_ticket_empty` |
| NewSessionTicket | `extensions` | read and ignored, unknown ones among them | `hh::new_session_ticket_dropped` |
| KeyUpdate | place | after the server Finished; before: `unexpected_message`, `UnexpectedMessage` | `hh::key_update_before_finished` |
| KeyUpdate | length | one byte; else `decode_error`, `Malformed` | `hh::key_update_bad_length` |
| KeyUpdate | `request_update` | 0, or 1 answered before the next data; else `illegal_parameter`, `BadKeyUpdate` | `hh::key_update_bad_value`, `hh::key_update_requested` |
| KeyUpdate | the key after it | the new key only; a record under the old: `bad_record_mac`, `BadRecordMac` | `hh::old_key_after_key_update` |
| alert | record | one alert of two bytes; else `decode_error`, `BadAlertRecord` | `record::alert_record_length` |
| alert | `level` | read; it decides nothing | `hh::alert_levels` |
| alert | `description` | `close_notify`: end of stream; `user_canceled`: ignored; every other, unknown ones among them: `AlertReceived`, nothing sent; before any ServerHello with `before_server_hello: true` (kinds (a) and (b)) | `hh::data_after_close_notify`, `hh::user_canceled_ignored`, `hh::alert_levels`, `hh::alert_before_server_hello` |
| any other type | handshake type | a type of no row above (`client_hello`, `end_of_early_data`, `message_hash`, an unassigned value), or a known one out of its place: `unexpected_message`, `UnexpectedMessage` | `hh::unknown_handshake_type`, `hh::post_handshake_messages` |
| any other type | record content type | `change_cipher_spec` as the row of section 5 above; any other: `unexpected_message`, `UnknownContentType` | `hh::change_cipher_spec_after_finished`, `record::unknown_content_type` |

## Acceptance

`cargo nextest run -p surf-tls` passes: our ClientHello equals `client-hello.golden`, RFC 8448
sections 3, 5 and 7 and every recorded flight complete or end as item 12 quotes, and each
hostile case below ends with the `Error` and the alert named.

1. ClientHello with a fixed random source: byte for byte `client-hello.golden`, which lists the
   three TLS 1.3 suites in the order above and no other suite; with a host `127.0.0.1`: no
   `server_name` extension.
2. RFC 8448 sections 3, 5 and 7 through `GivenClientHello`, given the ClientHellos of the trace
   as bytes (section 5: both, the first and the one after the HelloRetryRequest), the server
   flight as bytes, and the CertificateVerify check given the answer of the trace (its server
   key is RSA-1024, which the seam refuses): the client Finished equals the trace. No test runs
   the mode over a socket. `cargo tree -p surf-tls -e normal,build,features` (no
   dev-dependencies) shows no `test-tooling`.
3. Each recorded TLS 1.3 flight: the handshake completes, our ClientHello equals the recorded
   one, and the HTTP answer of the server decrypts. The recording against `openssl s_server
   -tls1_2`: what item 12 quotes for it, `AlertReceived { protocol_version,
   before_server_hello: true }`, kind (a).
4. Hostile, each from crafted server bytes in memory:
   1. kind (c): a TLS 1.2 ServerHello without `supported_versions` and without a downgrade
      value, once with suite 0x1301 and once with 0xC02F: each `VersionUnsupported`, alert
      `protocol_version` sent; the same with legacy version 0x0302 (TLS 1.1): the same
      (`hh::tls12_server_hello`);
   2. a TLS 1.2 ServerHello with `DOWNGRD` 0x01 at the end of its random, and a TLS 1.1
      ServerHello with `DOWNGRD` 0x00: each `AlertSent { illegal_parameter, Downgrade }`;
   3. a ServerHello whose `supported_versions` holds 0x0303, and one that holds 0x7f1c (a
      draft version): each `AlertSent { illegal_parameter, UnofferedVersion }` (RFC 8446
      section 4.2.1);
   4. a TLS 1.3 ServerHello that selects 0xC02F, and a HelloRetryRequest that selects 0xCCA8,
      both TLS 1.2 suites surf does not offer: each `AlertSent { illegal_parameter,
      UnofferedSuite }` (`hh::tls12_suite_selected`);
   5. kind (b): `handshake_failure` as the first record: `AlertReceived { handshake_failure,
      before_server_hello: true }`, whose `Display` holds the fixed words of item 7; kind (a):
      `protocol_version` as the first record: `AlertReceived { protocol_version,
      before_server_hello: true }` (`hh::alert_before_server_hello`);
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
6. Every row of "RFC 8446 client obligations" marked new or round 5, and every row of
   "Messages the client receives", has its tests, each from crafted bytes in memory, ending
   with the alert and the `Error` its rule column names; beyond that column:
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
      `Expired`, `NotTrusted(BadSignature)`, `NotTrusted(Unreadable)` (the reader refused the
      leaf of `leaf-v1`) and `NotTrusted(WeakAlgorithm)` (the answer of `story:tls-x509-path`
      for the case `md5`) makes the client send `unknown_ca`, `certificate_expired` and three
      times `bad_certificate`;
   6. `hh::key_share_rejected`: after a retry to secp256r1, a ServerHello share that is no
      point of P-256; and an x25519 share of 32 zero bytes: each `AlertSent {
      illegal_parameter, BadKeyShare }`;
   7. `hh::no_data_after_alert`: after `AlertReceived`, a read and a write return the same
      error and nothing more reaches the stream; `hs::close_sends_close_notify`: `close` writes
      one `close_notify` record under the application key and shuts the write side;
   8. `hh::alert_levels`: after the handshake, a warning-level (1) `handshake_failure`, a
      warning-level `unexpected_message`, and a fatal-level alert of the unassigned
      description 0xfe, each in a connection of its own: each `AlertReceived` with that
      description, no alert sent, and the next read returns the same error;
      `hh::user_canceled_ignored`: a warning-level `user_canceled`, then application data `ok`,
      then `close_notify`: the reads return `ok` and then the end of stream;
   9. `hh::old_key_after_key_update`: after a server `KeyUpdate` with `update_not_requested`, a
      record of application data protected under the old server key: `AlertSent {
      bad_record_mac, BadRecordMac }`; the same record under the new key opens.
7. `hs::musts_rows_name_tests`: every line of `rfc8446-client-musts.txt` has four columns, and
   every entry of its last column is one of: `tests/tls/<file>.rs::<function>`, accepted only
   when `crates/tls/<file path>` holds `fn <function>(` with `#[test]` among the attribute
   lines right above it; `INDEX:<case>`, accepted only when `crates/tls-test-certs/certs/INDEX`
   has a line for that case; `wycheproof:<file name>`, accepted only when that file exists in
   `crates/tls/tests/data/wycheproof/`; `not reachable: <why>` with a reason. No entry holds a
   wildcard. A copy with one function name changed, one with its `#[test]` removed, and one
   with a case name changed each fail naming the line. The file has the 76 rows of the table.
8. `hs::fields_rows_name_tests`: the same check over `rfc8446-client-fields.txt`, which has the
   49 rows of the table "Messages the client receives"; its entries
   `tests/tls/record.rs::<function>` are checked in the file of `story:tls-record`.

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
`tests/tls/rfc8446-client-fields.txt`, `tests/data/client-hello.golden`,
`tests/data/client-hello-retry.golden`, `tests/data/flights/`, `examples/record-flights.rs`,
`taskfiles/tls-flights.yml`. Coordinator: `mod` lines in a wave of more than one unit, the root
`Taskfile.yml`, `AGENTS.md`. Beside it run `story:tls-x509-path` and `story:tls-anchors`.
