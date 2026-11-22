---
format: aep.planning-md/3
id: story:tls-decisions
kind: story
status: draft
title: Decide the TLS design and check that ring builds for the four release targets
summary: docs/design/tls.md records U1 to U3, the reasons, the extra anchor option, the crate layout, the bounds and the test design; a probe workflow on an unmerged branch builds ring 0.17.14 on the forge runners.
relations:
- decomposes: epic:tls-https
scope:
- confidence: cited
  path: docs/design/tls.md
revision: 3
---
## Context

`epic:tls-https`, "Decisions taken": U1 is `ring` 0.17.14 (the operator, 2026-11-22 05:47 CET,
relayed by `host-operator`: "A: ring 0.17.14"); the lock check allows its packages by name and
version and only for the crate `surf-tls`; this story checks the C compiler setup of the musl
release build on the forge runners before anything else is built. U2 is option B (the root list
of Mozilla as curl publishes it, compiled in, plus one extra anchor file per session). U3 is
option C (no TLS 1.2 in this epic; such a server fails with `TlsVersionUnsupported`). The epic
leaves to this story: the failure reasons, the extra anchor option, the crate layout, the test
design and the measurements below.

What the forge runs today:

- `.github/workflows/release.yml`, job `package`: `dtolnay/rust-toolchain` with the target, no
  C compiler step; `x86_64-unknown-linux-musl` on `ubuntu-22.04`, `aarch64-unknown-linux-musl`
  on `ubuntu-22.04-arm`, `x86_64-apple-darwin` on `macos-15-intel`, `aarch64-apple-darwin` on
  `macos-15`; `RUSTFLAGS=-D warnings --remap-path-prefix=... -C strip=symbols`; `readelf -lW`
  refuses an `INTERP` header on Linux; the smoke test builds `cargo test --release -p
  surf-session --test e2e --target <t>`, so ring is compiled twice per Linux job.
- `.github/workflows/check.yml`: `ubuntu-latest`, `x86_64-unknown-linux-gnu`, installs no C
  compiler (the image has one).

Not known, and why this story exists before any code:

- Whether the build script of ring (through `cc`) finds a compiler that targets musl on the two
  Linux runners without a step. From memory, not checked: for `x86_64-unknown-linux-musl` `cc`
  looks for `musl-gcc`, which Ubuntu ships in `musl-tools`.
- The count "8 packages" of U1 was resolved for x86_64 Linux. `Cargo.lock` lists the
  dependencies of every target; from memory, ring 0.17 has a target dependency for Windows and
  getrandom 0.2 one for WASI, so the lock may hold more than 8.
- Whether ring makes an ephemeral X25519 or P-256 key from given bytes, signs ECDSA
  deterministically, or takes a random source written outside it (the epic: "not checked (story 1
  checks it)"). From memory, none of the three; the test design depends on the answer.
- What one RSA-2048 and one P-384 verification cost in the test profile (opt-level 0).
- `surf_agent::wire::REQUEST_MAX` is 64 KiB (`crates/agent/src/wire.rs:86`): an extra anchor
  file that travels in an `open` request must fit in it.

## Change

1. `docs/design/tls.md` (new), numbered items, each a decision with its reason or a measurement
   with how it was made:
   1. U1, U2, U3 as taken, quoted with date and who took them.
   2. The lock: every package with a `source` line that `cargo generate-lockfile` writes for a
      scratch copy of the root workspace plus a crate with `ring = "=0.17.14"` (default
      features), with version, source, checksum and its dependents. These are the lines of
      `taskfiles/lock-allow.txt` (`story:tls-lock-allow`). When the count is not 8, the
      report puts a question to `host-operator` through the coordinator (default A: allow every
      package cargo writes for ring, each depended on only by ring or another allowed package).
   3. The toolchain per runner, from the probe run (below, cited by run id and commit): the C
      compiler `cc` used, the step each target needs (the exact lines for `release.yml`),
      static or not, the build time of ring; the same for `ubuntu-latest` (the gate) and for the
      workstation.
   4. Timings in the test profile (`debug = false`, opt-level 0) on the workstation through
      `slot`, median of 5: RSA-2048 PKCS#1 SHA-256 and PSS verification, ECDSA P-256 and P-384,
      Ed25519, X25519 and P-256 agreement, AES-128-GCM and ChaCha20-Poly1305 seal of 16 KiB,
      one HKDF expand. When a verification is over 50 ms: the line for `.cargo/config.toml`
      (such as `[profile.dev.package.ring] opt-level = 3`, which also sets the opt-level `cc`
      compiles the C and assembly of ring with) and the times with it.
   5. What ring 0.17.14 takes from its caller (key agreement from given bytes, deterministic
      ECDSA signing, a random source of the caller), each with the file and line of ring read,
      and the consequence: where the RFC 8448 trace and a recorded flight need a given key share,
      the key agreement of the seam is a test double that answers the recorded shared secret;
      Wycheproof `x25519_test` and `ecdh_secp256r1_test` then check only that an invalid public
      key is refused; a certificate signed with ECDSA is not byte-reproducible.
   6. The crate layout and its interfaces: `surf-tls` (`crates/tls`) with the modules the
      stories of this epic write (`crypto.rs`, `error.rs`, `der.rs`, `x509/`, `pem.rs`,
      `base64.rs`, `path.rs`, `names.rs`, `anchors/`, `record.rs`, `key_schedule.rs`,
      `transcript.rs`, `handshake/`, `connect.rs`); the operations `crypto.rs` offers, signing
      for test tooling included (no RSA key generation: ring has none); the public error kinds of
      `error.rs`; the verifier interface between the handshake and path validation (the
      handshake hands over the certificates and the host, and gets the leaf key or one of the
      three certificate kinds of item 7); the key-agreement, signature-check, random and clock
      seams the tests inject; the test tooling crates `surf-tls-test-certs`
      (`crates/tls-test-certs`), `surf-tls-conformance` (`crates/tls-conformance`) and
      `surf-tls-test-server` (`crates/tls-test-server`), none shipped. In a wave of more than one
      unit, the `mod` lines of `crates/tls/src/lib.rs` and `crates/tls/tests/main.rs` are the
      coordinator, written from what each unit hands over.
   7. The failure reasons: the five names of the epic (`CertificateNotTrusted`,
      `CertificateExpired`, `CertificateNameMismatch`, `TlsVersionUnsupported`,
      `TlsHandshakeFailed`), or a replacement with its reason, and a table from each condition to
      its reason. At least these rows: no path to an anchor, a bad signature on the path, an
      issuer that is no CA, a path over the bound, an unknown critical extension, an RSA key
      under 2048 bits, a SHA-1 signature, a name constraint broken: `CertificateNotTrusted`;
      outside the validity, either side: `CertificateExpired`; no `subjectAltName` entry for the
      host: `CertificateNameMismatch`; a ServerHello without `supported_versions` 0x0304, a
      `protocol_version` alert: `TlsVersionUnsupported`; every other alert or protocol error, a
      bad `CertificateVerify`, a bad `Finished`, an oversized handshake message:
      `TlsHandshakeFailed`; the read timeout running out during the handshake, and a connection
      closed before the handshake ends: `Unreachable`, as for HTTP today; a body read until the
      connection ends that ends without `close_notify` (truncation): `Unreachable` (net.yaml:
      "it ended before a response was complete").
   8. The extra anchors: `surf --ca-file <path> <url>` and `surf-agent open --ca-file <path>
      [<url>]`. The client reads the file and its bytes travel in the `open` request; not an
      environment variable, because a `surfd` started by an earlier `open` does not see the
      environment of a later caller. At most 32 KiB and 64 certificates, so that an `open`
      request stays within `REQUEST_MAX`; every block a `CERTIFICATE`, every certificate a CA.
      `surf --anchors [--ca-file <path>]` prints `anchors <n> bundled <yyyy-mm-dd>
      sha256:<64 hex>` and with a file `anchors <m> file <path>`, exit 0.
   9. Bounds, each with its reason: the largest handshake message, the most certificates in a
      Certificate message, the most intermediates on a path, the most path candidates tried, one
      HelloRetryRequest.
   10. The test design, per material: where it is vendored, its `SOURCE` (the form of
       `crates/html-conformance/data/*/SOURCE`: repository, commit, date fetched, licence,
       sha256 of each file) and its size: the Wycheproof subset
       (`crates/tls/tests/data/wycheproof/`), RFC 8448 (`crates/tls/tests/data/rfc8448/`),
       x509-limbo (`crates/tls/tests/data/x509-limbo/`; the size of `limbo.json` at the pinned
       commit, and when it is over 20 MiB the subset taken and the count left out, so that no
       RFC 5280 section is ever claimed complete over a subset), the generated certificates
       (`crates/tls-test-certs/certs/` with `DIGEST`), the recorded flights
       (`crates/tls/tests/data/flights/`; the independent server, default `openssl s_server` of
       the host, its version in `SOURCE`; recorded data pinned by its sha256, since a server that
       draws its own randomness never records the same bytes twice). BoGo is not used (Go, a long
       run). The lanes: `tls-certs-check` and `tls-flights-check` in `check`, `tls-certs-slow`
       and `tls-flights-slow` in `slow`.
   11. U2 in detail: the curl file read (`https://curl.se/docs/caextract.html`), its date, its
       count, its sha256 from the `.sha256`; where it lives (`crates/tls/src/anchors/`); how a
       root the X.509 reader refuses (a version 1 root, for one) is taken (as name and key, RFC
       5280 section 6.1.1 (d)); the licence of the list (MPL-2.0) and of ring for `NOTICE`.
   12. U3 in detail: what the client sees of a TLS 1.2 server and which reason each gives.
2. The probe, on a branch `probe/tls-ring` that is never merged into `main`:
   `.github/workflows/tls-probe.yml` (`on: push` to that branch only, `permissions: contents:
   read`, every action pinned by commit as `release.yml` pins them, the toolchain read from
   `rust-toolchain.toml`) and `probe/tls-ring/`, a crate outside the root workspace with its own
   `Cargo.lock`, whose `main` verifies one fixed ECDSA P-256 signature, runs one X25519
   agreement and prints `probe ok`. Jobs: the four release targets on their runners with the
   release `RUSTFLAGS`, `cargo build --locked --release --target <t>`, `readelf -lW` on Linux,
   then the binary run; for each musl target a second job without the compiler step of item 3
   (a negative control: it shows whether the step is needed); and `ubuntu-latest` with
   `x86_64-unknown-linux-gnu`, as the gate builds. The unit writes these files in its worktree.
   The coordinator does all forge work: commits them on the branch, pushes it, waits for the run
   and gives the unit the run id and the logs.

## Acceptance

`docs/design/tls.md` is on the story branch with the twelve items, and the probe run it cites
built and ran ring 0.17.14 on the four release targets and on the gate runner.

1. The probe run named in item 3: its five build jobs conclude `success` and each log holds
   `probe ok`; the two Linux musl jobs print no `INTERP` line.
2. The two negative-control jobs end as item 3 says: `failure` with the compiler error quoted in
   item 3, or `success`, and then item 3 says no step is needed.
3. Item 2 lists every package with a `source` line of the scratch lock with its checksum; the
   number of lines equals `grep -c "^source = "` of that lock. When it is not 8, the report holds
   the question for `host-operator` with its default.
4. Item 4 gives the ten timings, and either the `opt-level` line with the times under it or the
   sentence that none is over 50 ms.
5. Item 5 answers its three questions, each with the file and line of ring 0.17.14 read.
6. Item 7 has a row for each hostile input the later stories name: a bad certificate, a
   truncated record, a downgrade to TLS 1.2, a wrong host name, an oversized handshake message,
   a server that never finishes the handshake; each with exactly one reason.
7. The change to `main` is `docs/design/tls.md` alone (`git diff --stat main`), and
   `printf "docs/design/tls.md\n" | sh taskfiles/planning-only.sh` exits 0, so its pull request
   skips the build and test jobs.

No test is added. The probe runs on the forge, never in the gate or its test step.

## Out of scope

Code on `main`; the change to `release.yml` (`story:tls-release`); taking U1 to U3 again.

## Writes

`docs/design/tls.md` on `main`; on the branch `probe/tls-ring` only,
`.github/workflows/tls-probe.yml` and `probe/tls-ring/`. No other story writes
`docs/design/tls.md`: a later story that finds an item wrong says so in its report, and the
coordinator revises the note. Every later story of this epic depends on this one. For the
coordinator: the line of `AGENTS.md`, "Commands", "Documents", that names `docs/design/tls.md`.

## Order of the stories of the epic

Thirteen stories, each depending on this one. Waves by their `depends_on` edges, with the `mod`
lines of `crates/tls` in the hands of the coordinator:

1. `tls-decisions`.
2. `tls-lock-allow` beside `tls-spec`.
3. `tls-x509`, `tls-test-certs` and `tls-record` beside each other (and `tls-spec` if it waits).
4. `tls-x509-path`, `tls-anchors` and `tls-handshake` beside each other.
5. `tls-test-server`.
6. `tls-fetch-https` (also after `story:one-test-binary-per-crate` and the merge of
   `impl/security-review-fixes`).
7. `tls-session-anchors`.
8. `tls-release`.

The longest chain is eight stories: `tls-decisions`, `tls-lock-allow`, `tls-record`,
`tls-handshake`, `tls-test-server`, `tls-fetch-https`, `tls-session-anchors`, `tls-release`.
