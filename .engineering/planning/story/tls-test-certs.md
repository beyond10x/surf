---
format: aep.planning-md/3
id: story:tls-test-certs
kind: story
status: draft
title: A generated set of test certificates, made once in the slow lane and held by a digest
summary: crates/tls-test-certs writes the positive cases and the negative suite (badssl names) into committed files with a DIGEST; tls-certs-check in the gate, tls-certs-slow regenerates and compares.
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-decisions
- depends_on: story:tls-lock-allow
scope:
- confidence: cited
  path: crates/tls-test-certs
- confidence: cited
  path: taskfiles/tls-certs-check.sh
- confidence: cited
  path: taskfiles/tls-certs.yml
revision: 5
---
## Context

Split from the candidate `tls-x509` of `epic:tls-https`: the generator of the negative suite and
its lane `slow` are checked by a digest and a byte comparison, apart from path validation, which
`story:tls-x509-path` checks against what this story writes. The test server
(`story:tls-test-server`), the handshake flights (`story:tls-handshake`) and the fetch tests
(`story:tls-fetch-https`, `story:tls-session-anchors`) read the same certificates.

The epic, "Testing within the budget": "a negative suite of certificates (expired, not yet
valid, wrong host, untrusted root, bad signature, an intermediate that is no CA, a path too
long, an unknown critical extension, RSA 1024, SHA-1, a wildcard across two labels), generated
once in the `slow` lane and committed with the digest of the generator"; the badssl.com names
(Apache-2.0, https://github.com/chromium/badssl.com). The folder rule
(`$HOME/beyond10x/AGENTS.md`, "Tests stay fast"): an expensive product is made once, committed
with a digest of its sources, checked by a fast digest comparison, and regenerated in a lane
named `slow` that compares bytes.

What ring 0.17.14 can sign (`docs/design/tls.md` items 5 and 6, read in its source): ECDSA
P-256 and P-384, Ed25519, RSA PKCS#1 and PSS with SHA-256/384/512 from 2048 bits; ECDSA is
reproducible under a fixed random source (`src/ec/suite_b/ecdsa/signing.rs:163`, `:172-190`).
It cannot sign with SHA-1 (`src/rsa/padding.rs:20-24`) nor with an RSA key under 2048 bits
(`src/rsa/keypair.rs:283`). The coordinator, 2026-11-22, decision 2: those certificates are made
once with a named external tool in the lane `slow` and committed as bytes with a digest; the gate
never runs that tool. Signing goes through `crates/tls/src/crypto.rs` with the feature
`test-tooling`; no crate but `surf-tls` depends on ring (the lock check).

Placement: one of the nine stories that write only `crates/tls*` and `taskfiles/tls-*`; built
after `tls-lock-allow`, beside `tls-x509` and `tls-record`, beside the rounds of surf.

## Change

- `crates/tls-test-certs` (`surf-tls-test-certs`, new, test tooling, not shipped, Apache-2.0,
  `#![forbid(unsafe_code)]`, depends on `surf-tls` by path with `test-tooling`):
  - a DER writer of its own and a certificate builder; every signature through the seam under
    a fixed random source, so every certificate it makes is reproducible byte for byte;
  - `keys/`: the private keys as PKCS#8 (RSA 1024, 2048 and 4096 bits, ECDSA P-256 and P-384,
    Ed25519), sources made once by the unit with `openssl genpkey` (the commands in
    `keys/SOURCE`) and never regenerated; `openssl genpkey` writes Ed25519 as PKCS#8 v1, which
    the seam reads with `Ed25519KeyPair::from_pkcs8_maybe_unchecked` (item 6; ring
    `from_pkcs8` takes v2 only, `ed25519/signing.rs:79-111`);
  - `src/cases.rs`, the cases, each a root, intermediates, a leaf, the host checked, and the
    expected outcome: `ok-rsa2048` (leaf `localhost` with `subjectAltName` dNSName `localhost`
    and iPAddress 127.0.0.1), `ok-ecdsa-p256`, `ok-ecdsa-p384`, `ok-ed25519`, `ok-wildcard`
    (`*.example.test` for `a.example.test`), `expired` (notAfter 2001-01-01), `not-yet-valid`
    (notBefore 2090), `wrong-host` (`other.test` for `localhost`), `untrusted-root`,
    `bad-signature` (one bit of the leaf signature flipped), `intermediate-not-ca`,
    `path-too-long` (one intermediate more than item 9 allows), `unknown-critical-extension`,
    `wildcard-two-labels` (`*.example.test` for `a.b.example.test`), `common-name-only` (no
    `subjectAltName`, CN `localhost`), `ip-mismatch`, `name-constraints-excluded`,
    `eku-client-only`, `1000-sans` (the match is the last entry);
  - the positive cases valid from 2025-01-01 to 2125-01-01, so that both UTCTime and
    GeneralizedTime are written (RFC 5280 section 4.1.2.5);
  - the binary `surf-tls-test-certs --out <dir>` writes per case `<case>/chain.pem` (leaf
    first), `<case>/root.pem` and, for the cases the test server uses, `<case>/leaf.pk8`; and
    `INDEX`, one line per case: name, host, validation time (`2026-11-22T00:00:00Z`), expected
    outcome (`ok`, or the reason and, for `CertificateNotTrusted`, one check name of the closed
    list of item 7: `untrusted-root` `NoPathToAnchor`, `bad-signature` `BadSignature`,
    `intermediate-not-ca` `IssuerNotCa`, `path-too-long` `PathTooLong`,
    `unknown-critical-extension` `UnknownCriticalExtension`, `name-constraints-excluded`
    `NameConstraint`, `eku-client-only` `NoServerAuth`, `sha1` `WeakAlgorithm`, `rsa-1024`
    `WeakKey`). This crate is built beside `story:tls-x509`, so it writes the names as text,
    not through the enum of `verify.rs`.
- `crates/tls-test-certs/openssl/`: `make.sh` (POSIX `sh`; run only by the lane `slow`), which
  makes the two cases ring cannot sign, `sha1` (a leaf signed sha1WithRSAEncryption by the RSA
  2048 intermediate) and `rsa-1024` (a leaf with the RSA 1024 key, signed by the intermediate
  with SHA-256), from the committed keys with a fixed serial, fixed validity dates and a fixed
  subject, so that a run makes the same bytes (RSA PKCS#1 v1.5 signing is deterministic); and
  `SOURCE` (the `openssl version` used, the date, the sha256 of each output). Its output goes
  into `certs/` and `INDEX` beside the other cases.
- `crates/tls-test-certs/certs/`: that output, committed, with `DIGEST`: one line per source
  (`src/`, `keys/`, `openssl/`, `Cargo.toml`) with its sha256, then the sha256 of those lines.
- `taskfiles/tls-certs-check.sh` (fast, no build, no `openssl`): recomputes `DIGEST` with
  `sha256sum` and fails `tls-certs: the sources changed since certs/ was generated; run task
  tls-certs-slow`.
- `taskfiles/tls-certs.yml`: `tls-certs-check` (the script; a step of `check`) and
  `tls-certs-slow` (generate into `.out/tls-certs/`, run `openssl/make.sh` there, compare every
  file with `certs/` byte for byte; with `WRITE=1` it writes `certs/` and `DIGEST`). For the
  coordinator: the include, the `check` line and the task `slow` with its first line, in the
  root `Taskfile.yml`.
- Tests in `crates/tls-test-certs/tests/main.rs`.

## Acceptance

`cargo nextest run -p surf-tls-test-certs` passes and `slot task tls-certs-check` prints `tls-certs:
<n> cases, certs/ matches its sources`; `slot task tls-certs-slow` (outside the gate) ends with
no file that differs.

1. `index`: `INDEX` names every case of `src/cases.rs` and the two of `openssl/` once, and every
   file it names exists; every check name in it is one of the eleven of item 7 (a constant of
   the test, copied from item 7), and a copy of `INDEX` with `NotACa` in place of
   `IssuerNotCa` fails naming the line.
2. `signatures`: each certificate the builder made verifies through the seam under its issuer
   key, except the leaf of `bad-signature`, which does not.
3. `reproducible`: the builder run twice in the test makes the same bytes for every case it
   makes, equal to the committed files.
4. `openssl_cases`: the leaf of `sha1` holds the OID 1.2.840.113549.1.1.5
   (sha1WithRSAEncryption) as its signature algorithm, the leaf of `rsa-1024` a modulus of 128
   bytes (both found by a byte search in the test, not by the reader under test); the
   seam names them `WeakAlgorithm` and `WeakKey`.
5. `rsa-1024` holds a 1024-bit key and `1000-sans` 1000 dNSName entries.
6. Hostile: a test copies the crate under `CARGO_TARGET_TMPDIR`, changes one byte of
   `src/cases.rs` and runs `sh taskfiles/tls-certs-check.sh` on it: exit 1 with the line above;
   the same with one byte of `openssl/make.sh` changed.
7. The slow lane run is in the report with its time and the `openssl version`; a second run
   right after it also ends with no file that differs.

Budget: generation and `openssl` never run in the test step; tests read committed files, except
case 3, which builds in memory (its time in the report, under 5 s); each other test under 1 s;
no network, no model call; no test reads the wall clock.

## Out of scope

Validating the certificates (`story:tls-x509-path`); certificates for client authentication.

## Writes

`crates/tls-test-certs/` (`src/`, `keys/`, `openssl/`, `certs/`, `tests/`),
`taskfiles/tls-certs.yml`, `taskfiles/tls-certs-check.sh`. Coordinator: the root `Taskfile.yml`
(include, `check` line, the task `slow`), `Cargo.lock`, `AGENTS.md`. No other story writes
`crates/tls-test-certs/`; the stories named in Context read `certs/` by path. Beside it run
`story:tls-x509` and `story:tls-record`.
