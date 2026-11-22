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
revision: 4
---
## Context

`epic:tls-https`, "Decisions taken": U1 is `ring` 0.17.14 (the operator, 2026-11-22 05:47 CET,
relayed by `host-operator`: "A: ring 0.17.14"); the lock check allows its packages by name and
version and only for the crate `surf-tls`; this story checks the C compiler setup of the musl
release build on the forge runners before anything else is built. U2 is option B (the root list
of Mozilla as curl publishes it, compiled in, plus one extra anchor file per session). U3 is
option C (no TLS 1.2 handshake in this epic). The coordinating agent, 2026-11-22: the
ClientHello also offers six TLS 1.2 suites so that a server limited to TLS 1.2 answers with a
ServerHello and surf can name the failure `TlsVersionUnsupported` (item 12 checks it and may
overturn it with evidence).

What the forge runs today:

- `.github/workflows/release.yml`, job `package`: `dtolnay/rust-toolchain` with the target, no
  C compiler step; `x86_64-unknown-linux-musl` on `ubuntu-22.04`, `aarch64-unknown-linux-musl`
  on `ubuntu-22.04-arm`, `x86_64-apple-darwin` on `macos-15-intel`, `aarch64-apple-darwin` on
  `macos-15`; `readelf -lW` refuses an `INTERP` header on Linux; the smoke test builds `cargo
  test --release -p surf-session --test e2e --target <t>`, so ring is compiled twice per job.
- `.github/workflows/check.yml`: `ubuntu-latest`, `x86_64-unknown-linux-gnu`, installs no C
  compiler (the image has one).

Read on 2026-11-22 in ring 0.17.14 and its dependencies (the cargo registry):

- `ring::test::rand::FixedSliceRandom`, `FixedByteRandom` and `FixedSliceSequenceRandom` are
  exported, deprecated ("internal API that will be removed"; `src/lib.rs:171-177`,
  `src/deprecated_test.rs:38-46`); `FixedSliceRandom` copies its bytes into a buffer of exactly
  their length and panics on any other length (`src/testutil.rs:495-499`).
- Key generation takes the caller random source: `agreement::EphemeralPrivateKey::generate(alg,
  rng)` (`src/agreement.rs:106-109`). So a given X25519 or P-256 private key is made by a fixed
  source of its bytes.
- ECDSA: the nonce key is drawn from the caller source when the key pair is made
  (`src/ec/suite_b/ecdsa/signing.rs:163`) and each signature mixes the caller bytes with the key
  and the digest (`:172-190`); under a fixed source a signature is reproducible.
- No SHA-1 signing encoding is exported (`src/rsa/padding.rs:20-24`, the SHA-1 PKCS#1 encoding
  is `pub(super)`, for legacy verification only); RSA signing refuses a modulus under 2048 bits
  (`src/rsa/keypair.rs:283`).
- AES-128-GCM and AES-256-GCM only, no AES-192 (`src/aead/algorithm.rs:149-163`).
- The lock gets more than 8 packages: ring declares `windows-sys` 0.52 for Windows on Arm
  (`Cargo.toml:256-261`), which brings `windows-targets` 0.52 and its eight platform crates
  (`windows-targets-0.52.6/Cargo.toml:32-53`), and getrandom 0.2.17 declares `wasi` 0.11
  (`Cargo.toml:115-116`). About 19 in all by these manifests; cargo decides.

Still unknown: whether `cc` finds a compiler that targets musl on the two Linux runners without
a step; what one RSA-2048 and one P-384 verification cost at opt-level 0; what a real TLS
1.2-only server answers to the ClientHello of item 12 (RFC 5246 section 7.4.1.3 allows
`handshake_failure` when no suite is shared; OpenSSL 1.1.1 and later answer `protocol_version`,
not measured). `surf_agent::wire::REQUEST_MAX` is 64 KiB (`crates/agent/src/wire.rs:86`): an
extra anchor file that travels in an `open` request must fit in it.

## Change

First act: in a scratch workspace (the root workspace copied plus one crate with `ring =
"=0.17.14"`), `cargo generate-lockfile`; the count of packages with a `source` line goes to the
coordinator at once, and when it is not 8 (the operator was told 8) the coordinator puts it to
`host-operator` with default A: allow every package cargo writes for ring, each depended on only
by ring or another allowed package. Nothing waits on the answer but `story:tls-lock-allow`.

1. `docs/design/tls.md` (new), numbered items, each a decision with its reason or a measurement
   with how it was made:
   1. U1, U2, U3 and the decision on TLS 1.2 suites, quoted with date and who took them.
   2. The lock: every package with a `source` line of the scratch lock, with version, source,
      checksum and its dependents; these are the lines of `taskfiles/lock-allow.txt`; the
      answer of `host-operator` when there was a question.
   3. The toolchain per runner, from the probe run (below, cited by run id and commit): the C
      compiler `cc` used, the step each target needs (the exact lines for `release.yml`),
      static or not, the build time of ring; the same for `ubuntu-latest` and the workstation.
   4. Timings in the test profile (`debug = false`, opt-level 0) through `slot`, median of 5:
      RSA-2048 PKCS#1 SHA-256 and PSS verification, ECDSA P-256 and P-384, Ed25519, X25519 and
      P-256 agreement, AES-128-GCM and ChaCha20-Poly1305 seal of 16 KiB, one HKDF expand. When a
      verification is over 50 ms: the line for `.cargo/config.toml` (such as
      `[profile.dev.package.ring] opt-level = 3`, which also sets the opt-level `cc` compiles
      the C and assembly of ring with) and the times with it.
   5. Random sources: the shipped path draws from `ring::rand::SystemRandom` (getrandom over the
      operating system source, the device `surf-secret` reads); the fixed sources of
      `ring::test::rand` are named only in `crypto.rs`, under `#[allow(deprecated)]`, behind the
      cargo feature `test-tooling` (item 6), and given buffers of the exact length. What
      follows: the RFC 8448 trace and each recorded flight are replayed with their own private
      key bytes, so no test double of the shared secret is needed; every Wycheproof `x25519`
      and `ecdh_secp256r1` vector runs with its private key and its shared secret is compared;
      every certificate ring signs under a fixed source is reproducible byte for byte.
   6. The crate layout and its owners. `surf-tls` (`crates/tls`): `crypto.rs` and `error.rs`
      (`story:tls-lock-allow`); `der.rs`, `x509/`, `pem.rs`, `base64.rs`, the leaf-key type
      `LeafKey`, the trust-anchor type `TrustAnchor` and `verify.rs`, the verifier interface
      (trait `ServerCertVerifier`: the certificates the server sent, the host, the time; answers
      the `LeafKey` or a `CertificateRejection` of kind `NotTrusted(check)`, `Expired` or
      `NameMismatch`) (`story:tls-x509`); `path.rs`, the implementation of that trait over a
      set of `TrustAnchor`s, and `names.rs` (`story:tls-x509-path`); `anchors/`
      (`story:tls-anchors`); `record.rs`, `key_schedule.rs`, `transcript.rs`
      (`story:tls-record`); `handshake/` and `connection.rs`, the one connection layer: a
      client over any `Read + Write` given a host, a `&dyn ServerCertVerifier` and the time
      (`story:tls-handshake`). The public `Error` of `error.rs`: `TimedOut`, `ConnectionClosed`
      (before the handshake ended), `Truncated` (after it, without `close_notify`),
      `Io(ErrorKind)`, `VersionUnsupported { detail }`, `AlertReceived { description,
      before_server_hello }`, `AlertSent { description, violation }` with `violation` one of
      `RecordOverflow`, `BadRecordMac`, `UnknownContentType`, `EmptyInnerPlaintext`,
      `MessageTooLarge { declared, bound }`, `UnexpectedMessage { expected, got }`,
      `EmptyCertificate`, `TooManyCertificates`, `UnofferedSuite`, `UnofferedExtension`,
      `UnofferedGroup`, `UnofferedAlpn`, `SecondHelloRetryRequest`, `Downgrade`,
      `UnexpectedChangeCipherSpec`, `BadCertificateVerify`, `BadFinished`;
      `SequenceExhausted`; and `Certificate(CertificateRejection)`, which `story:tls-x509` adds.
      The seam: the operations of `crypto.rs`, the signature algorithms it refuses by name
      (`REFUSED`: every SHA-1 signature, RSA keys under 2048 bits), and behind the feature
      `test-tooling` (off by default; turned on by `surf-tls-test-certs`,
      `surf-tls-test-server` and the dev-dependency of `surf-tls` on itself) the signing from
      PKCS#8 that ring offers (ECDSA P-256 and P-384, Ed25519, RSA PKCS#1 and PSS from 2048
      bits), the fixed random sources, and the RFC 8448 trace mode of `story:tls-handshake`; a
      release build never compiles them. The test tooling crates `surf-tls-test-certs`,
      `surf-tls-conformance`, `surf-tls-test-server`, none shipped. In a wave of more than one
      unit, the `mod` lines of `crates/tls/src/lib.rs` and `crates/tls/tests/main.rs` are the
      coordinator.
   7. The failure reasons and their detail: the five names of the epic, and a table from each
      condition to its reason and to the detail text printed after it. Rows: no path to an
      anchor, a bad signature on the path, an issuer that is no CA, a path over the bound, an
      unknown critical extension, a broken name constraint, an extended key usage without
      serverAuth: `CertificateNotTrusted`; a SHA-1 signature or an RSA key under 2048 bits:
      `CertificateNotTrusted` with the kind `WeakAlgorithm` or `WeakKey`, decided before any
      verification; outside the validity, either side: `CertificateExpired`; no
      `subjectAltName` entry for the host: `CertificateNameMismatch`; a ServerHello that
      selects TLS 1.2 (no `supported_versions` 0x0304) without the downgrade value, a
      `protocol_version` alert: `TlsVersionUnsupported`; a TLS 1.2 ServerHello with the
      downgrade value of RFC 8446 section 4.1.3: `TlsHandshakeFailed` (`illegal_parameter`);
      a `handshake_failure` alert before any ServerHello: `TlsHandshakeFailed`, the detail
      naming the alert; every other alert or protocol violation: `TlsHandshakeFailed`;
      `TimedOut`, `ConnectionClosed`, `Truncated`, `Io`: `Unreachable`, as for HTTP today
      (net.yaml: "it ended before a response was complete"). Revocation is not checked: a
      revoked certificate that has not expired loads; the documents say so in those words.
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
       `crates/html-conformance/data/*/SOURCE`) and its size: the Wycheproof subset
       (`crates/tls/tests/data/wycheproof/`; the AES-192 vectors of `aes_gcm_test` are a named
       skip, their count stated), RFC 8448 (`crates/tls/tests/data/rfc8448/`), x509-limbo
       (`crates/tls/tests/data/x509-limbo/`; the size of `limbo.json` at the pinned commit, and
       when it is over 20 MiB the subset taken and the count left out), the generated
       certificates (`crates/tls-test-certs/certs/`; the cases ring cannot make, a SHA-1
       signature and an RSA-1024 key, made once by `openssl` in the lane `slow` and committed as
       bytes; the gate never runs `openssl`), the recorded flights
       (`crates/tls/tests/data/flights/`: recorded data pinned by sha256, not a generated
       product; the lane `slow` records again and compares outcomes, since the independent
       server draws its own randomness). BoGo is not used (Go, a long run). The lanes:
       `tls-certs-check` and `tls-flights-check` in `check`, `tls-certs-slow` and
       `tls-flights-slow` in `slow`.
   11. U2 in detail: the curl file read (`https://curl.se/docs/caextract.html`), its date, its
       count, its sha256 from the `.sha256`; where it lives (`crates/tls/src/anchors/`); a root
       the certificate reader refuses (a version 1 root, for one) is taken as name and key (RFC
       5280 section 6.1.1 (d)); the licence of the list and of ring, for `NOTICE`.
   12. TLS 1.2: the ClientHello offers, after the three TLS 1.3 suites, the six TLS 1.2 ECDHE
       AEAD suites TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256 (0xC02B),
       TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384 (0xC02C), TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
       (0xC02F), TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384 (0xC030) (RFC 5289),
       TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256 (0xCCA8) and
       TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256 (0xCCA9) (RFC 7905), for one purpose: a
       server limited to TLS 1.2 then selects 1.2 in its ServerHello, and surf ends with a
       `protocol_version` alert and `TlsVersionUnsupported`. surf never continues a TLS 1.2
       handshake in this epic. Checked by two recordings against `openssl s_server -tls1_2` on
       127.0.0.1, one with these suites in the hello and one without, each answer quoted; when
       the evidence shows the suites change nothing or harm a TLS 1.3 server, this item says so
       and the coordinator revises the decision.
   13. The clock: validity is checked against a time the caller passes; `surf` and the session
       pass the system clock; library tests pass a fixed time (`2026-11-22T00:00:00Z`); tests
       through a binary run on the system clock, with certificates valid until 2125 and an
       expired one whose end lies in 2001.
2. The probe, on a branch `probe/tls-ring` that is never merged into `main`:
   `.github/workflows/tls-probe.yml` (`on: push` to that branch only, `permissions: contents:
   read`, every action pinned by commit as `release.yml` pins them, the toolchain read from
   `rust-toolchain.toml`) and `probe/tls-ring/`, a crate outside the root workspace with its own
   `Cargo.lock`, whose `main` verifies one fixed ECDSA P-256 signature, runs one X25519
   agreement and prints `probe ok`. Jobs: the four release targets on their runners with the
   release `RUSTFLAGS`, `cargo build --locked --release --target <t>`, `readelf -lW` on Linux,
   the binary run; for each musl target a second job without the compiler step of item 3 (a
   negative control); and `ubuntu-latest` with `x86_64-unknown-linux-gnu`, as the gate builds.
   The unit writes these files in its worktree; the coordinator does all forge work: commits
   them on the branch, pushes it, waits for the run and gives the unit the run id and the logs.

## Acceptance

`docs/design/tls.md` is on the story branch with the thirteen items, and the probe run it cites
built and ran ring 0.17.14 on the four release targets and on the gate runner.

1. The probe run named in item 3: its five build jobs conclude `success` and each log holds
   `probe ok`; the two Linux musl jobs print no `INTERP` line.
2. The two negative-control jobs end as item 3 says: `failure` with the compiler error quoted, or
   `success`, and then item 3 says no step is needed.
3. Item 2 lists every package with a `source` line of the scratch lock with its checksum; their
   number equals `grep -c "^source = "` of that lock; when it is not 8, the report quotes the
   question put to `host-operator` and its answer, or "taken on default".
4. Item 4 gives the ten timings, and the `opt-level` line with the times under it or the
   sentence that none is over 50 ms.
5. Item 5: a scratch test, run once and quoted, makes the X25519 key of RFC 8448 section 3 from
   its bytes through a fixed source and derives the shared secret the trace prints; and signs one
   message twice with ECDSA P-256 under one fixed source and gets equal bytes.
6. Item 6 names, for each signing algorithm the cases of `story:tls-test-certs` need, the ring
   function that makes it, and lists the cases ring cannot make; each later story of the epic
   names a file of item 6 only as its owner writes it.
7. Item 7 has a row for each hostile input the later stories name (a bad certificate, a
   truncated record, a downgrade to TLS 1.2, a wrong host name, an oversized handshake message, a
   server that never finishes, a weak algorithm), each with one reason and its detail text.
8. Item 12 quotes the two recorded answers of `openssl s_server -tls1_2` (its version named):
   with the six suites a ServerHello selecting TLS 1.2, or what it was instead; without them the
   alert it sent.
9. The change to `main` is `docs/design/tls.md` alone (`git diff --stat main`), and `printf
   "docs/design/tls.md\n" | sh taskfiles/planning-only.sh` exits 0.

No test is added to the repository. The probe runs on the forge, the scratch test in the
scratch directory; neither runs in the gate.

## Out of scope

Code on `main`; the change to `release.yml` (`story:tls-release`); taking U1 to U3 again.

## Writes

`docs/design/tls.md` on `main`; on the branch `probe/tls-ring` only,
`.github/workflows/tls-probe.yml` and `probe/tls-ring/`. No other story writes
`docs/design/tls.md`: a later story that finds an item wrong says so in its report, and the
coordinator revises the note. For the coordinator: the line of `AGENTS.md`, "Commands",
"Documents", that names `docs/design/tls.md`.

## Order of the stories of the epic

Thirteen stories, each depending on this one (the coordinator, decision 8 of 2026-11-22):

- Nine write only `crates/tls*`, `taskfiles/tls-*`, `docs/design/tls.md`, the lock check and
  its test (`taskfiles/lock-check.sh`, `taskfiles/lock-allow.txt`,
  `crates/surf/tests/lock_check.rs`) and `.cargo/config.toml`, and are built beside the rounds 1
  and 2 of surf: `tls-decisions` alone first; then `tls-lock-allow`; then `tls-x509`,
  `tls-test-certs` and `tls-record` side by side; then `tls-x509-path`, `tls-anchors` and
  `tls-handshake` side by side; then `tls-test-server`.
- Four are one unit, built in this order: `tls-spec`, `tls-fetch-https`, `tls-session-anchors`,
  `tls-release`. The unit starts at the first boundary between two rounds after the nine are
  merged (after round 1 or after round 2), never beside a unit that writes `spec/`,
  `generated/`, `crates/session`, `crates/agent`, `crates/surfd`, `crates/surf/src/lib.rs`,
  `conformance/src/target.rs` or `docs/agent-answer.md`. `tls-fetch-https` also needs
  `story:one-test-binary-per-crate` and the merge of `impl/security-review-fixes`.

The longest chain is nine stories: `tls-decisions`, `tls-lock-allow`, `tls-record`,
`tls-handshake`, `tls-test-server`, `tls-spec`, `tls-fetch-https`, `tls-session-anchors`,
`tls-release`.
