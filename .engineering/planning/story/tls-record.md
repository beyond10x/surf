---
format: aep.planning-md/3
id: story:tls-record
kind: story
status: draft
title: surf-tls protects TLS 1.3 records and derives the key schedule, checked against RFC 8448
summary: Record layer, key schedule and transcript hash; every secret and record of RFC 8448 sections 3, 5 and 7 reproduced byte for byte; hostile records refused with their alert.
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-decisions
- depends_on: story:tls-lock-allow
scope:
- confidence: cited
  path: crates/tls/src/key_schedule.rs
- confidence: cited
  path: crates/tls/src/record.rs
- confidence: cited
  path: crates/tls/src/transcript.rs
- confidence: cited
  path: crates/tls/tests/data/rfc8448
- confidence: cited
  path: crates/tls/tests/record.rs
- confidence: cited
  path: crates/tls/tests/rfc8448.rs
revision: 4
---
## Context

Split from the candidate `tls-handshake` of `epic:tls-https`: this story is the record layer and
the key schedule, checked against the bytes of RFC 8448; `story:tls-handshake` is the client
state machine on top of it. The halves are checked apart: here every secret, key and protected
record of a published trace is reproduced; there the order and contents of messages and the
refusal of each deviating server. Together they were too large for one unit.

The epic, "What surf writes itself": "The record layer (plaintext at most 2^14 bytes, AEAD per
record, sequence numbers, alerts, `close_notify`, `KeyUpdate` received), the key schedule
(HKDF-Extract and HKDF-Expand-Label over the transcript hash)". RFC 8448 (Informational,
January 2019, https://www.rfc-editor.org/rfc/rfc8448.txt): sections 3 (a simple 1-RTT
handshake), 5 (HelloRetryRequest) and 7 (compatibility mode) are in scope; the trace prints the
client X25519 private key, and ring makes a key from given bytes through a fixed random source
(`ring-0.17.14/src/agreement.rs:106-109`; `docs/design/tls.md` item 5), so the shared secret is
derived, not supplied. Items 6 (the `Error` kinds of `error.rs`) and 9 (the bound of a
handshake message). RFC 8446 sections 5 (record protocol), 7 (key schedule) and 4.4.1
(transcript hash, `message_hash` after a HelloRetryRequest). The counting allocator of
`crates/tls/tests/alloc.rs` (`story:tls-lock-allow`) observes what a refusal allocates.

Placement: one of the nine stories that write only `crates/tls*`; built after
`tls-lock-allow`, beside `tls-x509` and `tls-test-certs`, beside the rounds of surf.

## Change

- `crates/tls/src/record.rs`: reading and writing records over any `Read + Write`; a plaintext
  fragment of at most 2^14 bytes, a ciphertext of at most 2^14 + 256; AEAD per record through
  the seam with the nonce of section 5.3 (the IV XOR the sequence number); the inner content
  type and its zero padding; a sequence number that would wrap refuses to protect another
  record; alerts read and written; a `change_cipher_spec` record handed up for the handshake to
  judge; handshake messages reassembled across records, a declared length over the bound of
  item 9 refused at the 4-byte header; the next application traffic secret for `KeyUpdate`.
- `crates/tls/src/key_schedule.rs`: HKDF-Extract and HKDF-Expand-Label and Derive-Secret over
  the seam; early, handshake and master secrets; client and server handshake and application
  traffic secrets; finished keys and `verify_data`; traffic key and IV; for SHA-256 and SHA-384.
- `crates/tls/src/transcript.rs`: the running transcript hash, with the `message_hash`
  replacement after a HelloRetryRequest.
- `crates/tls/tests/data/rfc8448/rfc8448.txt` (the RFC text, unchanged) and `SOURCE` (URL, date
  fetched, sha256, the IETF Trust Legal Provisions).
- Tests in `crates/tls/tests/main.rs`, modules `rfc8448` (reads the labelled hex blocks of
  sections 3, 5 and 7 from the RFC text) and `record`.

## Acceptance

`cargo test -p surf-tls` passes with every secret, key, IV and protected record of RFC 8448
sections 3, 5 and 7 reproduced byte for byte, and each hostile record below refused with the
`Error` named.

1. Section 3: the client X25519 key made from the private key bytes of the trace and the server
   share of the trace give the shared secret the trace prints; each derived secret, traffic key
   and IV equals the trace; the server records of the trace open into the EncryptedExtensions,
   Certificate, CertificateVerify and Finished the trace prints; our protection of the client
   Finished and of the application data records gives the bytes of the trace.
2. Section 5: the transcript after the HelloRetryRequest (with `message_hash`) and every secret
   after it equal the trace.
3. Section 7: the `change_cipher_spec` records of the trace are handed up, and the rest equals
   the trace.
4. Hostile, each from bytes in memory: a record cut after 3 of its 5 header bytes and then end of
   stream: `ConnectionClosed`; a ciphertext of 2^14 + 257 bytes: `AlertSent { record_overflow,
   RecordOverflow }`, refused at its header, the peak of the counting allocator under 64 KiB; a
   plaintext of 2^14 + 1 bytes after opening: `AlertSent { record_overflow, RecordOverflow }`;
   one flipped bit of a tag: `AlertSent { bad_record_mac, BadRecordMac }`; inner content type
   0x19: `AlertSent { unexpected_message, UnknownContentType }`; a record of zero padding only:
   `AlertSent { unexpected_message, EmptyInnerPlaintext }`; a handshake header declaring 2^24 - 1
   bytes: `AlertSent { illegal_parameter, MessageTooLarge { declared: 16777215, bound } }` after
   its 4 bytes, the peak of the counting allocator under 64 KiB; a sequence number at 2^64 - 1:
   `SequenceExhausted`, no record written.
5. `sources_match_their_digests` for `rfc8448.txt`.

Budget: each test under 1 s; no test over 60 s, the test step within 120 s, no network, no model
call.

## Out of scope

The client state machine, extensions and certificates (`story:tls-handshake`); RFC 8448 sections
4 (resumption, 0-RTT) and 6 (client authentication).

## Writes

`crates/tls/src/record.rs`, `src/key_schedule.rs`, `src/transcript.rs`, `tests/rfc8448.rs`,
`tests/record.rs`, `tests/data/rfc8448/`. Coordinator: the `mod` lines in a wave of more than one
unit. Beside it run `story:tls-x509` and `story:tls-test-certs`.
