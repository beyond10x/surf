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
revision: 3
---
## Context

Split from the candidate `tls-x509` of `epic:tls-https`: the generator of the negative suite and
its `slow` lane are checked by a digest and a byte comparison, apart from path validation, which
`story:tls-x509-path` checks against what this story writes. The test server
(`story:tls-test-server`), the handshake flights (`story:tls-handshake`) and the fetch tests
(`story:tls-fetch-https`, `story:tls-session-anchors`) read the same certificates.

The epic, "Testing within the budget": "a negative suite of certificates (expired, not yet
valid, wrong host, untrusted root, bad signature, an intermediate that is no CA, a path too
long, an unknown critical extension, RSA 1024, SHA-1, a wildcard across two labels), generated
once in the `slow` lane and committed with the digest of the generator"; the badssl.com names
(Apache-2.0, https://github.com/chromium/badssl.com) for the cases; validity "far in the future
and checked against an injected clock, never the wall clock". The folder rule
(`$HOME/beyond10x/AGENTS.md`, "Tests stay fast"): an expensive product is made once, committed
with a digest of its sources, checked by a fast digest comparison, and regenerated in a lane
named `slow` that compares bytes. `docs/design/tls.md` item 5: ring signs ECDSA with its own
randomness (to be confirmed there), so an ECDSA signature is not reproducible; RSA PKCS#1 v1.5
and Ed25519 signatures are. Item 6: signing goes through `crates/tls/src/crypto.rs`; no crate
but `surf-tls` depends on ring (the lock check).

## Change

- `crates/tls-test-certs` (`surf-tls-test-certs`, new, test tooling, not shipped, Apache-2.0,
  `#![forbid(unsafe_code)]`, depends on `surf-tls` by path):
  - a DER writer of its own and a certificate builder;
  - `keys/`: the private keys as PKCS#8 (RSA 1024, 2048 and 4096 bits, ECDSA P-256 and P-384,
    Ed25519), sources made once by the unit with `openssl genpkey` (the commands in
    `keys/SOURCE`) and never regenerated;
  - `src/cases.rs`, the cases, each a root, intermediates, a leaf, the host checked, and the
    expected outcome: `ok-rsa2048` (leaf `localhost` with `subjectAltName` dNSName `localhost`
    and iPAddress 127.0.0.1), `ok-ecdsa-p256`, `ok-ecdsa-p384`, `ok-ed25519`, `ok-wildcard`
    (`*.example.test` for `a.example.test`), `expired` (notAfter 2001), `not-yet-valid`
    (notBefore 2090), `wrong-host` (`other.test` for `localhost`), `untrusted-root`,
    `bad-signature` (one bit of the leaf signature flipped), `intermediate-not-ca`,
    `path-too-long` (one intermediate more than item 9 allows), `unknown-critical-extension`,
    `rsa-1024`, `sha1`, `wildcard-two-labels` (`*.example.test` for `a.b.example.test`),
    `common-name-only` (no `subjectAltName`, CN `localhost`), `ip-mismatch`,
    `name-constraints-excluded`, `eku-client-only`, `1000-sans` (the match is the last entry);
  - the positive cases valid from 2025-01-01 to 2125-01-01, so that both UTCTime and
    GeneralizedTime are written (RFC 5280 section 4.1.2.5: from 2050 GeneralizedTime);
  - the binary `surf-tls-test-certs --out <dir>` writes per case `<case>/chain.pem` (leaf
    first), `<case>/root.pem` and, for the cases the test server uses, `<case>/leaf.pk8`; and
    `INDEX`, one line per case: name, host, validation time (`2026-11-22T00:00:00Z`), expected
    outcome (`ok` or the reason of item 7 and the check that fails).
- `crates/tls-test-certs/certs/`: that output, committed, with `DIGEST`: one line per source
  (`src/`, `keys/`, `Cargo.toml`) with its sha256, then the sha256 of those lines.
- `taskfiles/tls-certs-check.sh` (fast, no build): recomputes `DIGEST` with `sha256sum` and
  fails `tls-certs: the sources changed since certs/ was generated; run task tls-certs-slow`.
- `taskfiles/tls-certs.yml`: `tls-certs-check` (the script; a step of `check`) and
  `tls-certs-slow` (generate into `.out/tls-certs/`, compare with `certs/`: byte for byte, and
  for an ECDSA signature the TBS bytes and a verification under the issuer key instead; with
  `WRITE=1` it writes `certs/` and `DIGEST`). For the coordinator: the include, the `check`
  line and the task `slow` with its first line, in the root `Taskfile.yml`.
- Tests in `crates/tls-test-certs/tests/main.rs`.

## Acceptance

`cargo test -p surf-tls-test-certs` passes and `slot task tls-certs-check` prints `tls-certs:
<n> cases, certs/ matches its sources`; `slot task tls-certs-slow` (outside the gate) ends with
no difference.

1. `index`: `INDEX` names every case of `src/cases.rs` once, and every file it names exists.
2. `signatures`: each certificate of every chain verifies through the seam under its issuer
   key, except the leaf of `bad-signature`, which does not.
3. `tbs`: for each certificate the TBS bytes the builder makes equal those in the committed
   file.
4. `rsa-1024` holds a 1024-bit key, `sha1` a sha1WithRSAEncryption signature, `1000-sans` 1000
   dNSName entries (counted by the builder, not by the reader under test).
5. Hostile: a test copies the crate under `CARGO_TARGET_TMPDIR`, changes one byte of
   `src/cases.rs` and runs `sh taskfiles/tls-certs-check.sh` on it: exit 1 with the line above.
6. The slow lane run is in the report with its time; a second run right after it also ends with
   no difference.

Budget: generation never runs in the test step; tests read committed files; each test under
1 s; no network, no model call; no test reads the wall clock.

## Out of scope

Validating the certificates (`story:tls-x509-path`); certificates for client authentication.

## Writes

`crates/tls-test-certs/` (`src/`, `keys/`, `certs/`, `tests/`), `taskfiles/tls-certs.yml`,
`taskfiles/tls-certs-check.sh`. Coordinator: the root `Taskfile.yml` (include, `check` line, the
task `slow`), `Cargo.lock`, `AGENTS.md`. No other story writes `crates/tls-test-certs/`; the
stories named in Context read `certs/` by path. Beside it can run `story:tls-x509` and
`story:tls-record`.
