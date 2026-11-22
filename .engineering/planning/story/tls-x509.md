---
format: aep.planning-md/3
id: story:tls-x509
kind: story
status: draft
title: surf-tls reads DER, X.509 certificates and PEM, refusing malformed input with a named error
summary: A bounded DER reader, the X.509 v3 model the epic names, PEM and base64; x509-limbo vendored and every certificate in it read or refused in held counts.
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-decisions
- depends_on: story:tls-lock-allow
scope:
- confidence: cited
  path: crates/tls/src/base64.rs
- confidence: cited
  path: crates/tls/src/der.rs
- confidence: cited
  path: crates/tls/src/error.rs
- confidence: cited
  path: crates/tls/src/pem.rs
- confidence: cited
  path: crates/tls/src/verify.rs
- confidence: cited
  path: crates/tls/src/x509
- confidence: cited
  path: crates/tls/tests/data/x509-limbo
- confidence: cited
  path: crates/tls/tests/tls/der.rs
- confidence: cited
  path: crates/tls/tests/tls/limbo_parse.rs
- confidence: cited
  path: crates/tls/tests/tls/pem.rs
- confidence: cited
  path: crates/tls/tests/tls/x509_parse.rs
revision: 7
---
## Context

Split from the candidate `tls-x509` of `epic:tls-https`: this story reads certificates and owns
the types the other TLS stories pass around; `story:tls-x509-path` validates a path and a host
name. Each half has its own observable outcome (a certificate read into fields or refused with a
named error; a path accepted or refused with a reason), so each can be checked without the
other, and together they were too large for one unit.

The epic, "What surf writes itself": "A DER reader and X.509 certificates (RFC 5280): version
3, validity (UTCTime and GeneralizedTime ...), subject public key info, the extensions
basicConstraints, keyUsage, extKeyUsage (serverAuth), subjectAltName (dNSName, iPAddress),
nameConstraints, the key identifiers ...; a critical extension it does not know rejects the
certificate", and "A PEM reader for anchor files (base64 written here)". x509-limbo (C2SP,
Apache-2.0, https://github.com/C2SP/x509-limbo): one `limbo.json`, 9,807 cases on 2026-11-22.
`docs/design/tls.md` items 6 (the owners: this story owns the reader, `LeafKey`, `TrustAnchor`
and `verify.rs`), 9 and 10 (`story:tls-decisions`). The counting allocator of
`crates/tls/tests/tls/alloc.rs` (`story:tls-lock-allow`) observes what a refusal allocates on
the thread of its test (the counts are per thread, read by `peak_while`).

Placement: one of the nine stories that write only `crates/tls*` and `taskfiles/tls-*`; built
after `tls-lock-allow`, beside `tls-test-certs` and `tls-record`, beside the rounds of surf.

## Change

- `crates/tls/src/der.rs`: a DER reader over a byte slice that never reads past it and never
  allocates by a declared length: tag, definite length in the shortest form (at most 4 length
  bytes), INTEGER in the shortest form, BOOLEAN as 0x00 or 0xFF, OBJECT IDENTIFIER, BIT STRING
  with its unused bits, OCTET STRING, UTCTime and GeneralizedTime in the forms RFC 5280
  section 4.1.2.5 allows (`Z`, seconds, no fraction), UTF8String, PrintableString, IA5String;
  nesting at most 32 deep.
- `crates/tls/src/x509/`: a certificate as read: the TBS bytes (for the signature), version
  (only 3), serial (at most 20 bytes), the signature algorithm (outer and inner equal), issuer
  and subject as DER names that compare byte for byte, validity, the subject public key, the
  extensions named above; a duplicate extension refuses the certificate; an unknown extension
  marked critical is kept as a flag that `story:tls-x509-path` rejects; the Common Name is not
  exposed as a host name. `LeafKey` (the algorithm and the key bytes the seam checks with) and
  `TrustAnchor` (subject, key, nameConstraints; made from a certificate, or from a root this
  reader refuses as name and key only) are defined here.
- `crates/tls/src/verify.rs`: the trait `ServerCertVerifier` (the certificates the server sent,
  the host, the time; answers a `LeafKey` or a `CertificateRejection`) and
  `CertificateRejection` with its kinds `NotTrusted(check)`, `Expired` and `NameMismatch`;
  `check` is an enum of the closed list of item 7 and no other name: `NoPathToAnchor`,
  `BadSignature`, `IssuerNotCa`, `PathTooLong`, `CandidateLimit`, `KeyUsage`,
  `UnknownCriticalExtension`, `NameConstraint`, `NoServerAuth`, `WeakAlgorithm`, `WeakKey`.
  `src/error.rs` gains the variant `Certificate(CertificateRejection)`.
- `crates/tls/src/pem.rs` and `src/base64.rs`: blocks between `-----BEGIN <label>-----` and
  `-----END <label>-----` with equal labels, base64 of RFC 4648 with line breaks, padding
  checked; text outside blocks ignored; each block returned with its label.
- `crates/tls/tests/data/x509-limbo/`: `limbo.json` (or the subset of item 10), `LICENSE`,
  `SOURCE` (repository, commit, date fetched, Apache-2.0, sha256 of each file, the case counts
  at that commit) and `parse-counts.txt` (below).
- Tests in `crates/tls/tests/main.rs`, modules `der`, `x509_parse`, `pem`, `limbo_parse` (files
  in `tests/tls/`). The JSON of limbo is read with the reader `story:tls-lock-allow` made
  available.

## Acceptance

`cargo nextest run -p surf-tls` passes with the cases below, and every distinct certificate of
the vendored x509-limbo data is read or refused with a named error, in the counts
`crates/tls/tests/data/x509-limbo/parse-counts.txt` holds.

1. One certificate of limbo, named by its case id in the test: version 3, its serial, issuer,
   subject, validity, key algorithm and `subjectAltName` entries equal the values written in the
   test; its `LeafKey` and the `TrustAnchor` made from it carry its key bytes.
2. Hostile DER, each a short byte string in the test, each refused with the error named:
   a SEQUENCE declaring 10 bytes with 9 present: `Truncated`; length bytes `84 FF FF FF FF`:
   `Truncated`, with the peak of the counting allocator under 64 KiB during the call;
   length `80` (indefinite): `IndefiniteLength`; `81 05`: `NonMinimalLength`; five length
   bytes: `LengthTooLong`; INTEGER `00 01`: `NonMinimalInteger`; BOOLEAN `01`: `BadBoolean`; 40
   nested SEQUENCEs: `TooDeep`; one byte after a whole certificate: `TrailingData`.
3. Hostile X.509: a version 1 certificate: `UnsupportedVersion`, while a `TrustAnchor` is still
   made from it as name and key; outer and inner signature algorithms that differ:
   `AlgorithmMismatch`; `subjectAltName` twice: `DuplicateExtension`; UTCTime `2501010000Z` (no
   seconds): `BadTime`; February 30: `BadTime`; a dNSName holding a NUL byte: `BadName`; an
   unknown critical extension: read, with the flag set.
4. PEM: a block with wrong padding: `BadBase64`; a BEGIN without its END: `UnterminatedBlock`;
   a `PRIVATE KEY` block: returned with that label; 16 MiB of `A` with no marker: no block, in
   under 1 s.
5. `limbo_parse`: every distinct certificate of the data is read or refused; the number read and
   the number refused per error kind equal `parse-counts.txt`, which the unit writes from its run
   (not a number to reach); a refusal of a certificate in a case limbo expects to succeed is
   listed by case id in the report as a finding; no panic.
6. `sources_match_their_digests` for the limbo files.
7. `check_names`: the variants of the `check` enum are the eleven names of item 7, in its order
   (`Check::ALL`, a constant of `verify.rs`, and the name each prints, compared with the list as
   item 7 prints it).

Budget: no test over 60 s, the test step within 120 s, no network, no model call.
`limbo_parse` is the one test that reads the large file: its time is in the report, and over
10 s it is split by case group. Every other test under 1 s.

## Out of scope

Path validation, the host name check and the limbo validation counts (`story:tls-x509-path`);
the trust anchors of U2 (`story:tls-anchors`); writing DER (`story:tls-test-certs`).

## Writes

`crates/tls/src/der.rs`, `src/x509/`, `src/verify.rs`, `src/pem.rs`, `src/base64.rs`, one
variant of `src/error.rs`, `tests/tls/der.rs`, `tests/tls/x509_parse.rs`, `tests/tls/pem.rs`,
`tests/tls/limbo_parse.rs`, `tests/data/x509-limbo/`. The `mod` lines of `crates/tls/src/lib.rs` and
`tests/main.rs` are handed to the coordinator in a wave of more than one unit. Beside it run
`story:tls-record` and `story:tls-test-certs` (other files). Read later by `tls-x509-path`,
`tls-anchors`, `tls-handshake` and `tls-test-server`, each with an edge to this story.
