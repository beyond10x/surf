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
  path: crates/tls/tests/tls/record.rs
- confidence: cited
  path: crates/tls/tests/tls/rfc8446-record-musts.txt
- confidence: cited
  path: crates/tls/tests/tls/rfc8448.rs
revision: 11
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
`crates/tls/tests/tls/alloc.rs` (`story:tls-lock-allow`) observes what a refusal allocates on
the thread of its test (the counts are per thread, read by `peak_while`).

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
- `record`, `key_schedule` and `transcript` are public modules (item 6) and serve either side
  of a connection: the caller gives the keys and says which direction a record travels, and
  the key schedule returns client and server secrets alike, so `story:tls-test-server` builds
  its server from them without a change under `crates/tls/src`.
- `crates/tls/tests/data/rfc8448/rfc8448.txt` (the RFC text, unchanged) and `SOURCE` (URL, date
  fetched, sha256, the IETF Trust Legal Provisions).
- Tests in `crates/tls/tests/main.rs`, modules `rfc8448` (reads the labelled hex blocks of
  sections 3, 5 and 7 from the RFC text) and `record`, files in `tests/tls/`; each test carries
  the name the table below gives it.
- The record rules of RFC 8446 section 5 that `record.rs` also keeps (round 4 of the critics;
  names in `tls-decisions` item 6): an unknown outer or inner content type is
  `unexpected_message` and `UnknownContentType`; a record of another type between two
  fragments of one handshake message is `unexpected_message` and `InterleavedHandshake`; a key
  change asked while a handshake message is buffered is `unexpected_message` and
  `HandshakeSpansKeyChange`; a handshake or alert record with no content is
  `unexpected_message` and `EmptyContent`; an alert record that is not one alert of two bytes
  is `decode_error` and `BadAlertRecord`; a record before protection longer than 2^14 is
  `record_overflow` and `RecordOverflow`; an application data record without protection is
  `unexpected_message` and `UnprotectedRecord`; `legacy_record_version` is read and ignored,
  and every record surf writes carries 0x0303; surf writes no padding and no empty handshake
  fragment.
- `crates/tls/tests/tls/rfc8446-record-musts.txt`, committed beside the tests: the table below,
  one line per row, `<section> | <obligation> | <rule> | <tests>`, in the form of
  `rfc8446-client-musts.txt` of `story:tls-handshake`.

## RFC 8446 record obligations

Every "MUST" of RFC 8446 sections 5.1 to 5.4, and the record type rule of section 5, that binds
a client and that `record.rs` keeps. Rows marked "new" were missing before round 4 of the
critics. The `change_cipher_spec` rule of section 5 is a row of `story:tls-handshake`.

| Section | Obligation | Rule | Test |
|---|---|---|---|
| 5 | an unknown record type ends the connection | `unexpected_message`, `UnknownContentType` (new case for the outer type) | `record::unknown_content_type`, `record::unknown_outer_type` |
| 5.1 | no other record between fragments of a handshake message | `unexpected_message`, `InterleavedHandshake` (new) | `record::interleaved_handshake` |
| 5.1 | no handshake message across a key change | `unexpected_message`, `HandshakeSpansKeyChange` (new) | `record::handshake_spans_key_change` |
| 5.1 | no zero-length handshake fragment | never written; received: `unexpected_message`, `EmptyContent` (new) | `record::empty_handshake_fragment` |
| 5.1 | an alert record holds exactly one alert | `decode_error`, `BadAlertRecord` (new) | `record::alert_record_length` |
| 5.1 | `legacy_record_version` ignored; 0x0303 written | read any, write 0x0303 (new) | `record::record_version_ignored` |
| 5.1 | a record before protection at most 2^14 | `record_overflow`, `RecordOverflow` (new case) | `record::oversized_plaintext_record` |
| 5.1 | application data never unprotected | `unexpected_message`, `UnprotectedRecord` (new) | `record::unprotected_application_data` |
| 5.2 | a protected record at most 2^14 + 256 | `record_overflow`, `RecordOverflow` | `record::oversized_ciphertext` |
| 5.2 | a record that does not open | `bad_record_mac`, `BadRecordMac` | `record::flipped_tag` |
| 5.3 | sequence number 0 at each key, the nonce the IV XOR it | per key | `rfc8448::section_3`, `rfc8448::server_direction` |
| 5.3 | never wrap a sequence number | `SequenceExhausted` | `record::sequence_exhausted` |
| 5.4 | padding of zeros when sending | surf pads nothing | `rfc8448::section_3` |
| 5.4 | handshake and alert content never empty | `unexpected_message`, `EmptyContent` (new) | `record::empty_alert_content` |
| 5.4 | no non-zero octet: `unexpected_message`, only the cleartext scanned | `EmptyInnerPlaintext` | `record::padding_only` |
| 5.4 | the inner plaintext at most 2^14 + 1 | `record_overflow`, `RecordOverflow` | `record::oversized_inner_plaintext` |

## Acceptance

`cargo nextest run -p surf-tls` passes with every secret, key, IV and protected record of RFC 8448
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
4. Hostile, each from bytes in memory, each the test the table names: a record cut after 3 of
   its 5 header bytes and then end of
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
6. Server direction (`rfc8448::server_direction`), the side `story:tls-test-server` relies on:
   with the server handshake and application traffic keys of section 3, our protection of each
   server-to-client record of the trace (EncryptedExtensions, Certificate, CertificateVerify
   and Finished in their one record, the NewSessionTicket, the application data) from its
   plaintext and sequence number gives the bytes of the trace, byte for byte.
7. Every row of "RFC 8446 record obligations" marked new has its test, from bytes in memory,
   ending with the alert and the `Error` its rule column names: an outer record type 0x18;
   an alert record between two fragments of one handshake message; a key change asked while
   half a Finished is buffered; a handshake record of length 0; alert records of 1 and of 4
   bytes; a handshake record of 2^14 + 1 bytes before protection; an unprotected
   application data record after the keys are set; a protected handshake record whose
   content is empty; and a ServerHello record of version 0x0301, which is read (the version
   is ignored), while every record we write carries 0x0303.
8. `record::musts_rows_name_tests`: every line of `rfc8446-record-musts.txt` has four columns,
   and every entry of its last column is `tests/tls/<file>.rs::<function>` (the table above
   abbreviates `tests/tls/record.rs::` as `record::` and `tests/tls/rfc8448.rs::` as
   `rfc8448::`), accepted only when `crates/tls/<file path>` holds `fn <function>(` with
   `#[test]` among the attribute lines right above it, or `not reachable: <why>` with a reason;
   no entry holds a wildcard. A copy with one function name changed and one with its `#[test]`
   removed each fail naming the line. The file has the 16 rows of the table.

Budget: each test under 1 s; no test over 60 s, the test step within 120 s, no network, no model
call.

## Out of scope

The client state machine, extensions and certificates (`story:tls-handshake`); RFC 8448 sections
4 (resumption, 0-RTT) and 6 (client authentication).

## Writes

`crates/tls/src/record.rs`, `src/key_schedule.rs`, `src/transcript.rs`, `tests/tls/rfc8448.rs`,
`tests/tls/record.rs`, `tests/tls/rfc8446-record-musts.txt`, `tests/data/rfc8448/`.
Coordinator: the `mod` lines in a wave of more than one unit. Beside it run `story:tls-x509`
and `story:tls-test-certs`.
