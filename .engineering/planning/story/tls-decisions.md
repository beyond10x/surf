---
format: aep.planning-md/3
id: story:tls-decisions
kind: story
status: active
title: Decide the TLS design and check that ring builds for the four release targets
summary: docs/design/tls.md records U1 to U3, the reasons, the extra anchor option, the crate layout, the bounds and the test design; a probe workflow on an unmerged branch builds ring 0.17.14 on the forge runners.
relations:
- decomposes: epic:tls-https
scope:
- confidence: cited
  path: docs/design/tls.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:24:23Z", actor: "human:operator", revision: 10, decided_on: {"recorded":{"verification":1}}}
- {from: "proposed", to: "active", at: "2026-11-22T06:24:24Z", actor: "human:operator", revision: 11, decided_on: {"recorded":{"verification":1}}}
---
## Context

`epic:tls-https`, "Decisions taken": U1 is `ring` 0.17.14 (the operator, 2026-11-22 05:47 CET,
relayed by `host-operator`: "A: ring 0.17.14"); the lock check allows its packages by name and
version and only for the crate `surf-tls`; this story checks the C compiler setup of the musl
release build on the forge runners before anything else is built. U2 is option B (the root list
of Mozilla as curl publishes it, compiled in, plus one extra anchor file per session). U3 is
option C (no TLS 1.2 handshake in this epic). The coordinating agent took six TLS 1.2 suites into
the ClientHello on 2026-11-22 and reversed it the same day after the fifth critic round: RFC 8446
section 9.3, "A client sending a ClientHello MUST support all parameters advertised in it". The
ClientHello offers the three TLS 1.3 suites alone; a server limited to TLS 1.2 is reported by
one of three kinds (item 7).

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
- The lock gets 19 packages, 8 of them compiled on each release target: measured by the
  coordinator (epic section "Measured: the packages ring adds"). ring declares `windows-sys`
  0.52 for Windows (`Cargo.toml:256-261`), which brings `windows-targets` 0.52 and its eight
  platform crates, and getrandom 0.2.17 declares `wasi` 0.11 (`Cargo.toml:115-116`); cargo
  records them and never builds them for the targets of surf.
- `SecureRandom` is sealed (`src/rand.rs:23`): no random source of our own can be written;
  every draw goes through `SystemRandom` or, in tests, the fixed sources of `ring::test::rand`.

Still unknown: whether `cc` finds a compiler that targets musl on the two Linux runners without
a step; what one RSA-2048 and one P-384 verification cost at opt-level 0; whether a stack that
ignores `supported_versions` can be run without a network (item 12). `surf_agent::wire::REQUEST_MAX`
is 64 KiB (`crates/agent/src/wire.rs:86`): an extra anchor file that travels in an `open`
request must fit in it.

## Change

First act: in a scratch workspace (the root workspace copied plus one crate with `ring =
"=0.17.14"`), `cargo generate-lockfile`, to read the version, source and checksum of the 19
packages the coordinator measured; a count other than 19 is reported to the coordinator. The
count is settled (19 in the lock, 8 compiled per target); no question goes to `host-operator`
and `story:tls-lock-allow` waits for nothing but this story.

1. `docs/design/tls.md` (new), numbered items, each a decision with its reason or a measurement
   with how it was made:
   1. U1, U2, U3 and the decision on TLS 1.2 suites with its reversal, quoted with date and who
      took them.
   2. The lock: the 19 packages of the epic section "Measured: the packages ring adds", each
      with version, source, checksum and its dependents, and which 8 of them are compiled on
      each of the four release targets (the other 11 belong to Windows and WASI targets and are
      recorded in the lock only); these are the lines of `taskfiles/lock-allow.txt`.
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
      the `LeafKey` or a `CertificateRejection` of kind `NotTrusted(check)` with a check of item
      7, `Expired` or `NameMismatch`) (`story:tls-x509`); `path.rs`, the implementation of that
      trait over a set of `TrustAnchor`s, and `names.rs` (`story:tls-x509-path`); `anchors/`
      (`story:tls-anchors`); `record.rs`, `key_schedule.rs`, `transcript.rs`
      (`story:tls-record`); `handshake/` and `connection.rs`, the one connection layer: a
      client over any `Read + Write` given a host, a `&dyn ServerCertVerifier` and the time
      (`story:tls-handshake`). The public `Error` of `error.rs`: `TimedOut`, `ConnectionClosed`
      (before the handshake ended), `Truncated` (after it, without `close_notify`),
      `Io(ErrorKind)`, `VersionUnsupported { detail }`, `AlertReceived { description,
      before_server_hello }`, `AlertSent { description, violation }` with `violation` one of
      `RecordOverflow`, `BadRecordMac`, `UnknownContentType`, `EmptyInnerPlaintext`,
      `MessageTooLarge { declared, bound }`, `UnexpectedMessage { expected, got }`,
      `EmptyCertificate`, `TooManyCertificates`, `UnofferedSuite` (also a TLS 1.2 suite in a
      TLS 1.3 ServerHello or HelloRetryRequest: surf offers none),
      `UnofferedVersion` (a `supported_versions` in a ServerHello or HelloRetryRequest that
      holds anything but 0x0304), `UnofferedExtension`, `UnofferedGroup`, `UnofferedAlpn`,
      `SecondHelloRetryRequest`, `SuiteChangedAfterRetry` (a ServerHello after a
      HelloRetryRequest that selects another suite than it did, RFC 8446 section 4.1.4),
      `HelloRetryRequestNoChange` (a HelloRetryRequest that would not change the hello, the
      same section), `MissingExtension` (a HelloRetryRequest without `supported_versions`, a
      ServerHello without `key_share`, a CertificateRequest without `signature_algorithms`;
      alert `missing_extension`; a ServerHello without `supported_versions` is TLS 1.2),
      `SessionIdMismatch` (a session id echo other than ours), `CompressionNotNull`,
      `UnofferedKeyShare` (a ServerHello share in a group the client sent no share for),
      `KeyShareGroupChanged` (after a HelloRetryRequest, a ServerHello share in another group
      than it selected), `BadKeyShare` (a share the agreement refuses), `ExtensionNotAllowedHere`
      (an extension in a message the table of RFC 8446 section 4.2 does not allow it in),
      `RepeatedExtension` (two extensions of one type in a block; `DuplicateExtension` is the
      error of the X.509 reader), `NonEmptyRequestContext` (a `certificate_request_context` that
      is not empty in a CertificateRequest or Certificate of the handshake),
      `UnofferedSignatureScheme` (a CertificateVerify scheme not offered,
      not fitting the key, RSA PKCS#1 or SHA-1), `BadKeyUpdate` (a `request_update` other than
      0 or 1), `Malformed { message }` (alert `decode_error`), and from `story:tls-record`
      `InterleavedHandshake`, `HandshakeSpansKeyChange`, `EmptyContent` (a handshake or alert
      record with no content), `BadAlertRecord` (an alert record that is not one alert of two
      bytes), `UnprotectedRecord` (application data without protection), `Downgrade`,
      `UnexpectedChangeCipherSpec`, `BadCertificateVerify`, `BadFinished`; `SequenceExhausted`;
      and `Certificate(CertificateRejection)`, which `story:tls-x509` adds; a
      `CertificateRejection` sends `unknown_ca` (`NoPathToAnchor`), `certificate_expired`
      (`Expired`) or `bad_certificate` (every other). The list is closed; it follows the tables
      "RFC 8446 client obligations" and "Messages the client receives" of `story:tls-handshake`
      and "RFC 8446 record obligations" of `story:tls-record`.
      The seam: the operations of `crypto.rs`, the signature algorithms it refuses by name
      (`REFUSED`: every MD5 or SHA-1 signature, RSA keys under 2048 bits), and behind the feature
      `test-tooling` (off by default; turned on by `surf-tls-test-certs`,
      `surf-tls-test-server` and the dev-dependency of `surf-tls` on itself) the signing from
      PKCS#8 that ring offers (ECDSA P-256 and P-384 by `EcdsaKeyPair::from_pkcs8`, Ed25519 by
      `Ed25519KeyPair::from_pkcs8_maybe_unchecked`, which reads the PKCS#8 v1 that `openssl
      genpkey` writes where `from_pkcs8` wants v2 (`ed25519/signing.rs:79-111`), RSA PKCS#1
      and PSS from 2048 bits by `RsaKeyPair::from_pkcs8`), the fixed random sources, the RFC
      8448 trace mode of `story:tls-handshake`, and `TEST_TOOLING_MARKER`, a fixed string
      (`surf-tls test-tooling, never in a release`) that `Connection` reads through
      `std::hint::black_box` when the feature is on, so that a binary that connects carries it
      exactly then; a release build never compiles them. The public modules of `surf-tls`:
      `crypto`, `error`, `der`, `x509`, `pem`, `verify`, `path`, `anchors`, `record`,
      `key_schedule`, `transcript` and `connection`; `handshake` is private but for
      `handshake::trace` under `test-tooling`. `record.rs`, `key_schedule.rs` and
      `transcript.rs` serve either side of a connection (keys and direction given by the
      caller), so `crates/tls-test-server` builds the server from them and from `crypto` and
      writes its own server messages; it writes nothing under `crates/tls/src`, and a need it
      finds there is a finding for the owner of the file. The test tooling crates
      `surf-tls-test-certs`, `surf-tls-conformance`, `surf-tls-test-server`, none shipped. In
      a wave of more than one unit, the coordinator writes the `mod` lines of
      `crates/tls/src/lib.rs` and `crates/tls/tests/main.rs`.
   7. The failure reasons and their detail: the five names of the epic, and a table from each
      condition to its reason and to the detail text printed after it. The checks of
      `NotTrusted(check)`, a closed list; `INDEX` of `story:tls-test-certs` and
      `CertificateRejection` of `story:tls-x509` take these names and no other:
      `Unreadable` (a certificate the server sent that the X.509 reader refuses: bad DER, a
      version 1 leaf, `AlgorithmMismatch` and the other errors of the reader, the error named
      in the detail; alert `bad_certificate`), `NoPathToAnchor` (no path to an anchor; alert
      `unknown_ca`), `BadSignature` (a signature on the path does not
      verify), `IssuerNotCa` (an issuer without basicConstraints cA), `PathTooLong` (more
      intermediates than item 9 allows, or a pathLenConstraint broken), `CandidateLimit` (the
      most path candidates of item 9 tried), `KeyUsage` (keyUsage without keyCertSign on an
      issuer or digitalSignature on the leaf), `UnknownCriticalExtension`, `NameConstraint`
      (a name outside a nameConstraints of an issuer or the anchor), `NoServerAuth`
      (extKeyUsage without serverAuth), `WeakAlgorithm` (an MD5 or SHA-1 signature) and
      `WeakKey` (an RSA key under 2048 bits), the last two decided before any verification;
      every check but `NoPathToAnchor` sends `bad_certificate`. All give
      `CertificateNotTrusted`. Further rows: outside the validity, either side:
      `CertificateExpired`; no `subjectAltName` entry for the host: `CertificateNameMismatch`.
      A server limited to TLS 1.2, three kinds (item 12): (a) a `protocol_version` alert
      before any ServerHello: `TlsVersionUnsupported`; (b) a `handshake_failure` alert before
      any ServerHello: `TlsHandshakeFailed`, detail `the server sent handshake_failure before
      its ServerHello; a server limited to TLS 1.2 answers this way`; (c) a ServerHello that
      selects TLS 1.2 or below (no `supported_versions`) without a downgrade value, any suite:
      surf sends `protocol_version`, `TlsVersionUnsupported`. A ServerHello of TLS 1.2 or
      below with either downgrade value of RFC 8446 section 4.1.3 (`DOWNGRD` 0x01 or 0x00):
      `TlsHandshakeFailed` (`illegal_parameter`); a ServerHello whose `supported_versions`
      holds anything but 0x0304: `TlsHandshakeFailed` (`illegal_parameter`); every other
      alert or protocol violation: `TlsHandshakeFailed`, the detail naming the alert;
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
       (`crates/tls/tests/data/wycheproof/`, `rsa_pss_2048_sha256_mgf1_32_test.json` among its
       files; the AES-192 vectors of `aes_gcm_test` are a named skip, their count stated), RFC
       8448 (`crates/tls/tests/data/rfc8448/`), x509-limbo (`crates/tls/tests/data/x509-limbo/`;
       the size of `limbo.json` at the pinned commit, and when it is over 20 MiB the subset
       taken and the count left out), the generated certificates (`crates/tls-test-certs/certs/`;
       the cases ring cannot make, a SHA-1 signature, an MD5 signature and an RSA-1024 key, made
       once by `openssl` in the lane `slow` and committed as bytes; the gate never runs
       `openssl`), the recorded flights
       (`crates/tls/tests/data/flights/`: recorded data pinned by sha256, not a generated
       product; the lane `slow` records again and compares outcomes, since the independent
       server draws its own randomness). BoGo is not used (Go, a long run). The lanes:
       `tls-certs-check` and `tls-flights-check` in `check`, `tls-certs-slow` and
       `tls-flights-slow` in `slow`.
   11. U2 in detail: the curl file read (`https://curl.se/docs/caextract.html`), its date, its
       count, its sha256 from the `.sha256`; where it lives (`crates/tls/src/anchors/`); a root
       the certificate reader refuses (a version 1 root, for one) is taken as name and key (RFC
       5280 section 6.1.1 (d)); the licence of the list and of ring, for `NOTICE`.
   12. TLS 1.2: the ClientHello offers the three TLS 1.3 suites and nothing else (RFC 8446
       section 9.3: a client supports every parameter it advertises). The coordinator took six
       TLS 1.2 suites in on 2026-11-22 and reversed it the same day after the fifth critic
       round; both are quoted with their date. A server limited to TLS 1.2 is reported by the
       three kinds of item 7: (a) `protocol_version` before any ServerHello, (b)
       `handshake_failure` before any ServerHello, (c) a ServerHello below TLS 1.3. The
       recording against `openssl s_server -tls1_2` (its version named, 3.6.5 on the
       workstation) on 127.0.0.1 is quoted as kind (a) and shows nothing else. Telling kind
       (b) apart from other handshake failures needs TLS 1.2 itself, the next epic (U3). surf
       never continues a TLS 1.2 handshake in this epic.
   13. The clock: validity is checked against a time the caller passes; `surf` and the session
       pass the system clock; library tests pass a fixed time (`2026-11-22T00:00:00Z`); a
       limbo case without `validation_time` runs at `LIMBO_TIME`, the `commit-date` of the
       pinned limbo commit as its `SOURCE` records it (`story:tls-x509`, `story:tls-x509-path`);
       tests through a binary run on the system clock, with
       certificates valid until 2125 and an expired one whose end lies in 2001.
   14. RFC 8446 section 9, what surf offers against each requirement that binds a client:
       - 9.1 TLS_AES_128_GCM_SHA256 (MUST): offered; TLS_AES_256_GCM_SHA384 and
         TLS_CHACHA20_POLY1305_SHA256 (SHOULD): offered. Met.
       - 9.1 signatures (MUST): rsa_pkcs1_sha256 for certificates, rsa_pss_rsae_sha256 for
         CertificateVerify and for certificates (the reader of `story:tls-x509` takes the
         RSASSA-PSS AlgorithmIdentifier of RFC 4055 with SHA-256, SHA-384 or SHA-512, MGF1 of the
         same hash and a salt of the digest length, case `ok-rsa-pss` of
         `story:tls-test-certs`), ecdsa_secp256r1_sha256: all verified by the seam and offered.
         `signature_algorithms` (CertificateVerify): ecdsa_secp256r1_sha256,
         ecdsa_secp384r1_sha384, ed25519, rsa_pss_rsae_sha256, rsa_pss_rsae_sha384,
         rsa_pss_rsae_sha512; `signature_algorithms_cert`: the same plus rsa_pkcs1_sha256,
         rsa_pkcs1_sha384, rsa_pkcs1_sha512. Met.
       - 9.1 key exchange: secp256r1 (MUST) in `supported_groups`, reached by a
         HelloRetryRequest; X25519 (SHOULD) with a share in the first hello. Met.
       - 9.2 extensions (MUST implement, MUST send when applicable): `supported_versions`,
         `cookie` (echoed), `signature_algorithms`, `signature_algorithms_cert`,
         `supported_groups`, `key_share`, `server_name` (sent when the host is a name; RFC 6066
         allows no IP literal there). No `pre_shared_key` or `psk_key_exchange_modes`, as no PSK
         is offered. The ClientHello holds both `signature_algorithms` and `supported_groups`,
         and `key_share` with `supported_groups`. Met.
       - 9.3 a client supports every parameter it advertises. Shown for each: the three suites
         and both groups complete a handshake with the test server, x25519 directly and
         secp256r1 after a HelloRetryRequest, with `server_name` and ALPN `http/1.1`
         (`story:tls-test-server` acceptance 1); each of the six schemes of
         `signature_algorithms` completes a handshake with the server signing its
         CertificateVerify with it (acceptance 2 there); each of the nine schemes of
         `signature_algorithms_cert` signs the leaf of a case that path validation accepts
         (`ok-rsa2048`, `ok-rsa-pkcs1-sha384`, `ok-rsa-pkcs1-sha512`, `ok-rsa-pss`,
         `ok-rsa-pss-sha384`, `ok-rsa-pss-sha512`, `ok-ecdsa-p256`, `ok-ecdsa-p384`,
         `ok-ed25519` of `story:tls-test-certs`, `story:tls-x509-path` acceptance 1); the
         `cookie` is copied (`story:tls-handshake`). Met since the six TLS 1.2 suites were
         taken out (item 12).
       - 9.3 a client ignores unknown extensions in a CertificateRequest or NewSessionTicket:
         met (`story:tls-handshake`).
       No requirement of sections 9.1 to 9.3 is deviated from. A deviation found later is a
       line here, named, and goes to the coordinator for `host-operator`.
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
3. Item 2 lists the 19 packages of the epic section "Measured: the packages ring adds", each with
   version, source and checksum, and names the 8 compiled on each release target; their number
   equals `grep -c "^source = "` of the scratch lock. No question goes to `host-operator`.
4. Item 4 gives the ten timings, and the `opt-level` line with the times under it or the
   sentence that none is over 50 ms.
5. Item 5: a scratch test, run once and quoted, makes the X25519 key of RFC 8448 section 3 from
   its bytes through a fixed source and derives the shared secret the trace prints; signs one
   message twice with ECDSA P-256 under one fixed source and gets equal bytes; and loads an
   Ed25519 key written by `openssl genpkey` with `Ed25519KeyPair::from_pkcs8_maybe_unchecked`
   (the output of `from_pkcs8` on the same key quoted beside it).
6. Item 6 names, for each signing algorithm the cases of `story:tls-test-certs` need, the ring
   function that makes it, and lists the cases ring cannot make; it lists the public modules of
   `surf-tls` and says that `crates/tls-test-server` reads them and writes nothing under
   `crates/tls/src`; each later story of the epic names a file of item 6 only as its owner
   writes it.
7. Item 7 has a row for each hostile input the later stories name (a bad certificate, a
   certificate the reader refuses, a truncated record, a downgrade to TLS 1.2, a wrong host
   name, an oversized handshake message, a server that never finishes, a weak algorithm, each
   of the three kinds (a), (b), (c) of a server limited to TLS 1.2), each with one reason and
   its detail text, the fixed words of kind (b) quoted; and the closed list of the twelve names
   of the `NotTrusted` checks, each with its condition.
8. Item 12 quotes the recording against `openssl s_server -tls1_2` (its version named) as kind
   (a) and nothing more; item 14 has a line for each requirement of RFC 8446 sections 9.1 and
   9.2 and the client invariant of 9.3, with what surf offers and "met" or the deviation.
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

Thirteen stories, each depending on this one (the coordinator, decision 8 of 2026-11-22, and
decision 7 of round 2):

- Nine write only `crates/tls*`, `taskfiles/tls-*`, `docs/design/tls.md`, the lock check and
  its test (`taskfiles/lock-check.sh`, `taskfiles/lock-allow.txt`, the module `lock_check` of
  the test binary of `crates/surf`) and `.cargo/config.toml`; for them the coordinator writes
  the root `Taskfile.yml`, `Cargo.lock`, `README.md` and `AGENTS.md`. They are built beside the
  rounds 1 and 2 of surf: `tls-decisions` alone first; then `tls-lock-allow` (after
  `story:one-test-binary-per-crate`, which moves `crates/surf/tests/lock_check.rs`); then
  `tls-x509`, `tls-test-certs` and `tls-record` side by side; then `tls-x509-path`,
  `tls-anchors` and `tls-handshake` side by side; then `tls-test-server`.
- Four are one unit, built in this order: `tls-spec`, `tls-fetch-https`, `tls-session-anchors`,
  `tls-release`. The unit starts at the first boundary between two rounds after the nine are
  merged (after round 1 or after round 2), never beside a unit that writes `spec/`,
  `generated/`, `crates/fetch`, `crates/session`, `crates/agent`, `crates/surfd`,
  `crates/surf/src/lib.rs`, `conformance/src/target.rs` or `docs/agent-answer.md`.
  `tls-fetch-https` also needs `story:one-test-binary-per-crate` and the merge of
  `impl/security-review-fixes`.

`crates/tls` is born with one integration-test binary: `crates/tls/tests/main.rs`, which
declares each module as `#[path = "tls/<name>.rs"] mod <name>;`, the modules in
`crates/tls/tests/tls/`, the data in `crates/tls/tests/data/`; no `.rs` file lies beside
`main.rs`. The other new crates of the epic have `tests/main.rs` alone.

The longest chain is nine stories: `tls-decisions`, `tls-lock-allow`, `tls-record`,
`tls-handshake`, `tls-test-server`, `tls-spec`, `tls-fetch-https`, `tls-session-anchors`,
`tls-release`.
