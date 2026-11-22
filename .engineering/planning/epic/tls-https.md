---
format: aep.planning-md/3
id: epic:tls-https
kind: epic
status: draft
title: 'surf loads https: addresses'
summary: 'TLS 1.3 client written here over one audited source of primitives; X.509 path validation, host names, trust anchors; https: and redirects between schemes in surf-fetch, with TLS failure reasons in the answer.'
revision: 18
---
## Context

The operator, 2026-11-21: "of course we do need SSL soon" (`AGENTS.md`, "Implementation
targets"); 2026-11-22: "I wonder what is needed to actually make [surf] useful for agents,
because right now I think it isn't." The coordinator's order of the remaining work (D85) puts
"reach the web: TLS 1.3 and `https:`" second, after reading a page. An answer of release 0.3.0
fails every `https:` address with `page failed UnsupportedScheme`, and a redirect to `https:`
fails the same way (`spec/domains/net.yaml`, the comment above `entities:`; `crates/fetch`,
"What it does not do"). `docs/design/agent-interface.md` section 10, item 7: "A site an agent is
sent to test is served over HTTPS in practice."

What binds this epic:

- `AGENTS.md`, "Goals" (decision C of 2026-11-20): no third-party crate in anything that ships,
  with one exception, "audited cryptographic primitives (ciphers, hashes, key exchange, signature
  verification) come from a third-party crate, because hand-written ones leak keys through timing.
  Which crate is not chosen yet. The TLS protocol code on top of them is written here." README
  principle 4 says the same; principle 5: no unsafe code in a hand-written crate.
- The lock check (`Taskfile.yml`, task `lock-check`, a step of `check`) fails on every package of
  the root `Cargo.lock` that has a `source =` line and prints `Cargo.lock: <n> package(s), none
  from outside this repository`. `conformance/` has its own lock and depends on `surf` by path, so
  it takes the primitives too.
- The release builds four targets on the forge's own runners (`.github/workflows/release.yml`):
  `x86_64-unknown-linux-musl` on `ubuntu-22.04`, `aarch64-unknown-linux-musl` on
  `ubuntu-22.04-arm`, `x86_64-apple-darwin` on `macos-15-intel`, `aarch64-apple-darwin` on
  `macos-15`; the Linux binaries are static.
- The seam: `crates/fetch` writes and reads through `Timed`, a `Read + Write` over the
  `TcpStream` that holds the read timeout (`crates/fetch/src/lib.rs`, `struct Timed`); `send`
  connects, calls `exchange` with the stream and follows redirects in one loop. A TLS stream
  wraps `Timed` and is handed to `exchange` unchanged, so the HTTP code stays one.
- The folder's test budget (`$HOME/beyond10x/AGENTS.md`, "Tests stay fast"): no test over 60 s,
  the gate's test step within 120 s (22 to 29 s on 2026-11-22, `AGENTS.md`, "The gate's
  numbers"), no network request and no model call in the test step; an expensive product
  (a generated certificate set) is made once and committed with the digest of its sources, and
  regenerated in a lane named `slow` that compares bytes; a recorded handshake is recorded data,
  pinned by its sha256, and the lane `slow` records it again and compares outcomes, since the
  independent server draws its own randomness.
- RFC 8446 section 9.1, read 2026-11-22 at https://www.rfc-editor.org/rfc/rfc8446.html: a
  compliant application MUST implement TLS_AES_128_GCM_SHA256 and SHOULD implement
  TLS_AES_256_GCM_SHA384 and TLS_CHACHA20_POLY1305_SHA256; MUST support the signatures
  rsa_pkcs1_sha256 (for certificates), rsa_pss_rsae_sha256 and ecdsa_secp256r1_sha256; MUST
  support key exchange with secp256r1 and SHOULD support X25519. Section 9.2: the extensions
  supported_versions, cookie, signature_algorithms, signature_algorithms_cert, supported_groups,
  key_share and server_name are mandatory to implement.
- RFC 9525 (November 2023, obsoletes RFC 6125), read 2026-11-22 at
  https://www.rfc-editor.org/rfc/rfc9525.html: section 2, "The Common Name RDN MUST NOT be used
  to identify a service"; section 6.3, a wildcard is the complete content of the left-most label
  and matches one label; section 6.4, an IP address matches an `iPAddress` entry of the
  subjectAltName octet for octet.

## Outcome

- `surf https://…` and `surf-agent open|go https://…` load a page over TLS 1.3, with the server
  authenticated against the trust anchors of decision U2, and show it as an `http:` page is shown.
- A TLS failure ends the request with a reason of its own, shown as `page failed <reason>
  "<address>"` (below), so that an agent can tell a bad certificate from a host that cannot be
  reached. A server that speaks only TLS 1.2 is told apart from both when it reads
  `supported_versions` or answers with a TLS 1.2 ServerHello (`TlsVersionUnsupported`); one
  that ignores the extension ends `TlsHandshakeFailed` with a detail that says so (Risks).
- A redirect from `http:` to `https:` and back is followed inside one request, as the Fetch
  Standard follows it, under the destination rule of `story:security-review-fixes`.
- The cryptographic primitives come from the one source decision U1 names. The lock check allows
  exactly that source's packages, pinned by name, version and checksum, and only as dependencies
  of the one crate that wraps them; every other outside package still fails it.

## What surf writes itself, whatever the primitives

- `crates/tls` (`surf-tls`, new; `#![forbid(unsafe_code)]`): the client side of TLS 1.3 (RFC
  8446) only. The record layer (plaintext at most 2^14 bytes, AEAD per record, sequence numbers,
  alerts, `close_notify`, `KeyUpdate` received), the key schedule (HKDF-Extract and
  HKDF-Expand-Label over the transcript hash), the handshake (ClientHello with
  `supported_versions`, `key_share` for X25519, `supported_groups` X25519 and secp256r1,
  `signature_algorithms`, `signature_algorithms_cert`, `server_name` (RFC 6066 section 3) and ALPN
  `http/1.1` (RFC 7301); HelloRetryRequest with its `cookie` and a second key share for P-256;
  EncryptedExtensions, Certificate, CertificateVerify, Finished), the middlebox compatibility
  mode of Appendix D.4 (a legacy session id and a tolerated `change_cipher_spec`), and a
  `NewSessionTicket` read and dropped.
- A DER reader and X.509 certificates and path validation (RFC 5280): version 3, validity
  (UTCTime and GeneralizedTime against the clock), subject public key info, the extensions
  basicConstraints, keyUsage, extKeyUsage (serverAuth), subjectAltName (dNSName, iPAddress),
  nameConstraints, the key identifiers for building the path; a critical extension it does not
  know rejects the certificate. A path from the leaf through the intermediates the server sent to
  a trust anchor, with a depth bound, every signature verified; RSA keys under 2048 bits and
  SHA-1 signatures refused.
- Host name checking by RFC 9525 (above): dNSName and iPAddress of subjectAltName only, no
  Common Name, a wildcard only as the whole left-most label.
- A PEM reader for anchor files (base64 written here), the trust anchors of decision U2, and an
  extra anchor file for a session (`SURF_CA_FILE` or an `open` option, decided by the first
  story), which the tests and a local development CA need whatever U2 decides.
- The seam to the primitives: one module of `surf-tls` (`crypto.rs`) and nothing else names the
  outside crate: AEAD seal and open for the three suites, SHA-256 and SHA-384, HKDF, X25519 and
  P-256 key agreement, and signature verification for the schemes of section 9.1 plus
  ecdsa_secp384r1_sha384, ed25519 and RSA-PSS and PKCS#1 v1.5 with SHA-384 and SHA-512. A
  different source later replaces that module only.
- The client random and the ephemeral keys come from `surf-secret` (the operating system's
  source) where the primitive crate takes caller bytes, else from the crate's own source over the
  same device; tests inject both, and the clock, through the seam.

## Out of scope

- TLS 1.2 and older (decision U3; default out, with a reason of its own so the gap is seen).
- Client certificates, session resumption, PSK and 0-RTT, HTTP/2, OCSP and CRLs, certificate
  transparency, Encrypted Client Hello, a post-quantum key exchange, a server side that ships.
- Trust decisions the operator of an agent configures beyond one extra anchor file (pinning,
  allow-lists).
- The destination rule for redirects itself: `story:security-review-fixes`
  (`impl/security-review-fixes`) adds it to the same redirect loop of `crates/fetch/src/lib.rs`;
  this epic's fetch story depends on it and does not design it.
- Cookies and the `Secure` attribute: `story:agent-cookie-jar`.

## Changes outside `crates/fetch` and `crates/tls`

- `spec/domains/net.yaml`: `https` is fetched (the comment above `entities:` and the scheme
  rule); `surf.net.FailureReason` gains the TLS reasons, each with a comment where the Fetch
  Standard has only "network error" (as `TooManyRedirects` has one). Proposed, settled by the
  first story: `CertificateNotTrusted` (no path to an anchor, a bad signature on the path),
  `CertificateExpired` (outside its validity), `CertificateNameMismatch`, `TlsVersionUnsupported`
  (the server reads `supported_versions` and refuses, or answers with a TLS 1.2 ServerHello),
  `TlsHandshakeFailed` (every other alert or protocol error, a server limited to TLS 1.2 that
  ignores `supported_versions` among them). The
  certificate itself is not an entity: TLS is a port of `net`, as HTTP is, and nothing of it is
  observable but the reason; so no new noun enters the model.
- The answer: `docs/agent-answer.md` ("Page") lists the new reasons; `failure = 1*ALPHA` holds
  them unchanged. `docs/surf-command.md` the same for `surf`. `surf::fetch_http` and the session's
  transport stop refusing `https:`.
- Redirects between schemes: `net.yaml` and `crates/fetch` follow `http:` to `https:` and
  `https:` to `http:`; the test of `story:agent-fetch-redirects` case 10 (`Location: https:` is
  `UnsupportedScheme`) changes to a followed redirect, against the TLS test server.
- The release: `taskfiles/release-smoke.sh` checks that `surf https://127.0.0.1:1/` ends
  `Unreachable` (the scheme is accepted) and that the binary finds its anchors (a line such as
  `surf --anchors` printing their count and source, decided by the first story); the release
  workflow gains what the chosen source needs to build on the four runners; `docs/releasing.md`
  gains the anchor list's age check if U2 bundles one.
- README principle 4 and `AGENTS.md` "Goals": the exception is stated with its crate, version and
  audit link, in place of "planned"; "What works", "What surf is not" and the website follow.
- The lock check: an allow-list file (`taskfiles/lock-allow.txt`, one line per package: name,
  version, source, checksum) and a second rule, that no package of this repository but
  `surf-tls` depends on an allowed package; the summary line becomes `Cargo.lock: <n>
  package(s), <m> from outside this repository, each on the allow-list`. A test of the task with
  a lock that holds one package off the list, and one with an allowed package used by another
  crate, each failing (`taskfiles/` has tests of this form for `planning-only`).
- `story:agent-token-hash` (deferred until the cryptographic crate is chosen) can use the SHA-256
  of the seam once U1 is decided.

## Testing within the budget

No test reaches the network; every handshake runs against bytes in the tree or a server the test
starts on 127.0.0.1. What exists, read 2026-11-22:

- RFC 8448 (Informational, January 2019, https://www.rfc-editor.org/rfc/rfc8448.txt): section 2
  the private keys, 3 a simple 1-RTT handshake, 4 a resumed 0-RTT handshake, 5 a
  HelloRetryRequest, 6 client authentication, 7 compatibility mode; an ephemeral X25519 key and
  an RSA server certificate. The server's RSA modulus is 128 bytes (1024 bits), so a verifier that
  refuses keys under 2048 bits, ring among them, cannot check the trace's CertificateVerify
  (counted from the trace, not stated by it): the trace tests the key schedule, the record layer
  and the parsing, with the signature check given the trace's answer. Sections 3, 5 and 7 are in
  scope; 4 and 6 are not (no resumption, no client certificates).
- x509-limbo (C2SP, Apache-2.0, https://github.com/C2SP/x509-limbo): one `limbo.json` held to
  `limbo-schema.json`; each case names the peer certificate, trusted and untrusted certificates,
  the expected peer name, the validation time and the expected result. 9,807 cases on
  2026-11-22 (935 success, 8,872 failure; bettertls 9,572, rfc5280 109, webpki 56, crl 22;
  9,797 of kind SERVER).
- Wycheproof (C2SP, Apache-2.0, https://github.com/C2SP/wycheproof): JSON vectors, among them
  `aes_gcm_test`, `chacha20_poly1305_test`, `x25519_test`, `ecdh_secp256r1_test`,
  `ecdsa_secp256r1_sha256_test`, `ecdsa_secp384r1_sha384_test`, `ed25519_test`,
  `hkdf_sha256_test`, `hkdf_sha384_test` and the RSA-PSS and PKCS#1 signature files for 2048 bits.
- BoGo, BoringSSL's TLS suite: a Go runner drives a shim binary over a port and a JSON
  configuration of disabled tests and an error map; rustls runs it with a Rust shim and needs Go
  (https://raw.githubusercontent.com/google/boringssl/main/ssl/test/PORTING.md,
  https://raw.githubusercontent.com/rustls/rustls/main/bogo/README.md). Not for the gate (Go, a
  long run); a candidate for the `slow` lane after story 8.
- badssl.com (Apache-2.0, https://github.com/chromium/badssl.com): the cases expired,
  wrong.host, self-signed, untrusted-root, revoked, incomplete-chain, sha256/384/512, the rsa and
  ecc sizes, 1000-sans and more; the repository builds its certificates and serves them locally,
  so its cases can be generated or recorded rather than fetched. The negative suite below takes
  their names.

The plan, default of this draft: known-answer tests of the seam from Wycheproof vectors (a
vendored subset with its `SOURCE` file, as `crates/html-conformance/data/` holds its data; the
AES-192 vectors are a named skip, ring has no AES-192); the RFC 8448 traces for the key schedule,
the record layer and the parsing of every server message (in a test mode that takes the trace
ClientHello as given; our ClientHello is held by a golden file of its own, since it differs from
the trace); x509-limbo vendored at a pinned commit, its counts held in the gate as
`surf-html-conformance --check` holds the parser counts, so that RFC 5280 sections are claimed
complete only when it passes them; a negative suite of certificates (expired, not yet valid,
wrong host, untrusted root, bad signature, an intermediate that is no CA, a path too long, an
unknown critical extension, RSA 1024, SHA-1, a wildcard across two labels), generated once in
the `slow` lane and committed with the digest of the generator, the SHA-1 and RSA-1024 cases made
by `openssl` there because ring cannot sign them, the gate never running `openssl`; a TLS 1.3
test server written here as test tooling (`crates/tls-test-server`, not shipped, sharing the
record layer and key schedule) for the end-to-end tests and agent task 5; and recorded server
flights of an independent implementation (recorded in the `slow` lane on 127.0.0.1, replayed in
the gate), so that a defect the client and the test server share is still caught. The
recordings are recorded data pinned by their sha256; the `slow` lane records again and compares
outcomes, not bytes.

Read on 2026-11-22 in ring 0.17.14 (`story:tls-decisions`, item 5): key generation takes the
caller random source (`src/agreement.rs:106-109`), and ring exports fixed sources
(`ring::test::rand::FixedSliceRandom`, deprecated, `src/lib.rs:171-177`); so a replay and the
RFC 8448 trace make the client key from its recorded bytes, and the Wycheproof agreement vectors
compare shared secrets. ECDSA signing is reproducible under a fixed source
(`src/ec/suite_b/ecdsa/signing.rs:163`, `:172-190`), so every generated certificate is compared
byte for byte. Every one of these runs in milliseconds; the first story measures one RSA-2048
and one P-384 verification in the test profile (opt-level 0) and, if one is over 50 ms, sets an
`opt-level` for the primitive packages in `.cargo/config.toml`.

## Decisions (the first story settles them, or records the operator's answer)

### U1. The source of the primitives (for the operator)

Read 2026-11-22 from each project's own documentation and source; packages counted by resolving
the dependency tree of the newest release through the crates.io API (normal and build
dependencies, default features, x86_64 Linux; an inline resolver, not cargo, so plus or minus a
few). Every candidate below contains unsafe code or assembly, which principle 5 allows in a
dependency.

| | A. `ring` 0.17.14 | B. `aws-lc-rs` 1.19.0 | C. RustCrypto and dalek | D. written here |
|---|---|---|---|---|
| audit | none of ring alone (its README: "It is an experiment"); reviewed as a dependency in Cure53's rustls audit of May to June 2020, work package WP2 "ring, webpki, sct.rs, rustls-native-certs", 4 minor findings, no vulnerability (https://cure53.de/pentest-report_rustls.pdf); RUSTSEC-2025-0007 ("unmaintained") withdrawn on 2025-02-22, the rustls team maintains it | no audit named in its README; AWS-LC holds FIPS 140-3 level 1 certificates (#4631 v1.0, #4816 and #5429 v2.0, #5298 and #5314 v3.1; v4.0 "Modules in Process", https://raw.githubusercontent.com/aws/aws-lc/main/crypto/fipsmodule/FIPS.md); the validated module is the `fips` feature (`aws-lc-fips-sys`, AWS-LC-FIPS 4.x for the newest), not the default build | aes-gcm and chacha20poly1305: NCC Group, report v1.0 of 2020-02-13, "no significant findings", of commit `a15698fd` of RustCrypto/AEADs, no crate version named (https://web.archive.org/web/20240108154854/https://research.nccgroup.com/wp-content/uploads/2020/02/NCC_Group_MobileCoin_RustCrypto_AESGCM_ChaCha20Poly1305_Implementation_Review_2020-02-12_v1.0.pdf); curve25519, x25519 and ed25519-dalek: Quarkslab, 2019-08-26, no critical vulnerability, no versions named (https://blog.quarkslab.com/security-audit-of-dalek-libraries.html); rsa: "previously audited", "diverged significantly since then" (its README), and RUSTSEC-2023-0071 (Marvin, a timing leak of the private key, fixed in 0.10.0; it concerns private-key operations, which a client that only verifies does not run: inferred); p256, p384 and ecdsa: "never been independently audited" (their READMEs); sha2, hkdf: no audit stated | none; RUSTSEC-2024-0344 shows LLVM turning constant-time Rust into a branch (https://api.osv.dev/v1/vulns/RUSTSEC-2024-0344), the reason decision C took a crate |
| toolchain | a C compiler that builds `.S` assembly; no Perl for the crates.io package (https://raw.githubusercontent.com/briansmith/ring/main/BUILDING.md); its CI builds the four targets surf releases | a C and C++ compiler; "CMake is never required", no bindgen, no Go for the default build; the `fips` feature needs CMake and Go (https://aws.github.io/aws-lc-rs/requirements/index.html); musl x86_64 and aarch64 and aarch64-apple-darwin supported | plain Rust toolchain only | plain Rust |
| packages added | 8 with itself: cc, cfg-if, find-msvc-tools, getrandom 0.2, libc, ring, shlex, untrusted 0.9 (cc, shlex, find-msvc-tools build only) | 13: aws-lc-rs, aws-lc-sys, cc, cmake (declared, not run), dunce, find-msvc-tools, fs_extra, jobserver, libc, pkg-config, shlex, untrusted 0.7, zeroize | 65 for the whole set (aes-gcm 0.11.1, chacha20poly1305 0.11.0, x25519-dalek 3.0.0, p256 and p384 0.14.0, sha2 0.11.1, hkdf 0.13.0, rsa 0.10.0, ecdsa 0.17.0, ed25519-dalek 3.0.0), proc-macro2, quote and syn among them | 0 |
| licence | Apache-2.0 AND ISC | Apache-2.0 OR ISC; aws-lc-sys adds MIT and BSD-3-Clause parts | Apache-2.0 OR MIT; dalek BSD-3-Clause | Apache-2.0 |
| TLS 1.3 needs | all: AES-128-GCM, AES-256-GCM, ChaCha20-Poly1305; HKDF SHA-256 and SHA-384; X25519, P-256 (and P-384) agreement; RSA-PSS and PKCS#1 v1.5 with SHA-256/384/512 from 2048 bits, ECDSA P-256 and P-384, Ed25519 (https://docs.rs/ring/0.17.14/ring/); no post-quantum | all of A's, and ML-KEM (https://docs.rs/aws-lc-rs/1.19.0/aws_lc_rs/) | all, across the crates (inferred from their features, not run) | all, written and kept here |

Cost and default:

- **A (default).** The fewest packages (8, three of them build-only), every need covered,
  BoringSSL's primitives, built by its own CI for surf's four targets, the provider rustls calls
  "easier to build". Costs: a C compiler in the gate and on the four release runners (the musl
  targets need one that targets musl: measured by story 1); principle 4's word "audited" is
  reworded to what exists, a review within the rustls audit of 2020.
- **B.** The only source with a certification of its module, but only with the `fips` feature,
  which adds CMake and Go to every build; without it, 13 packages and a C++ compiler for an
  uncertified build of the same code. Choose it when "audited" must mean a certificate.
- **C.** No C toolchain at all, the simplest release; but 65 packages, proc-macro crates in the
  shipped tree, and the elliptic curves and ECDSA that every TLS server's certificate uses state
  that they were never audited.
- **D.** No outside package; the timing risk decision C was taken to avoid, with no audit.

### U2. Trust anchors (for the operator)

- **A. The operating system's store.** Linux: the files Go reads
  (https://github.com/golang/go/blob/master/src/crypto/x509/root_linux.go):
  `/etc/ssl/certs/ca-certificates.crt`, `/etc/pki/tls/certs/ca-bundle.crt`,
  `/etc/ssl/ca-bundle.pem`, `/etc/pki/tls/cacert.pem`,
  `/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem`, `/etc/ssl/cert.pem`, and the directories
  `/etc/ssl/certs`, `/etc/pki/tls/certs`. macOS: "macOS has no default SSL_CERT_{FILE,DIR}
  paths"; Go verifies through Security.framework (`root_darwin.go`), and so do
  rustls-native-certs and rustls-platform-verifier: from Rust that is foreign calls, unsafe code
  principle 5 forbids in a hand-written crate. `/etc/ssl/cert.pem` exists on macOS; whether it is
  kept current is not established (one tutorial says it is "never updated"; Apple's forum declined
  to say). Running `/usr/bin/security` to export the system roots is not checked. A static binary
  in an empty container finds no store.
- **B. A bundled list (default).** Mozilla's store, as curl publishes it
  (https://curl.se/docs/caextract.html: PEM, checked daily, converted when the source changes;
  2026-11-05, 118 certificates, with a `.sha256`), compiled into `surf-tls`, refreshed by a task
  outside the gate that writes the file and a `SOURCE` with its checksum, as the vendored test
  data has. The release procedure gains a step: a list older than 90 days is refreshed before a
  release. The same anchors on all four targets, no unsafe code, testable in the gate; the
  extra anchor file covers a local or company CA. Cost: a CA Mozilla removes stays trusted until
  the next release; webpki-roots, which ships the same list for rustls, warns that it suits only
  applications that are rebuilt and redeployed (https://github.com/rustls/webpki-roots).
- **C. Both:** the system's file on Linux when one is found, the bundled list otherwise and on
  macOS. Two behaviours to test and document; a host's added CA works without the extra file.

### U3. TLS 1.2 (for the operator)

Qualys SSL Pulse, scan of 2025-06-01 (the newest data file,
https://www.ssllabs.com/assets/project/ssl-pulse/data/pulse-2025-06-01.json): of 134,380 trusted
sites, TLS 1.3 is the best protocol of 101,141 (75.3 %) and TLS 1.2 of 33,183 (24.7 %). HTTP
Archive's Web Almanac 2025 (https://almanac.httparchive.org/en/2025/security): TLS 1.3 on
"roughly 76% of all web pages". So about one site in four needs TLS 1.2.

- **A (default). TLS 1.3 only in this epic**, `TlsVersionUnsupported` for a server without it,
  so the gap is counted. The goals name RFC 8446, not RFC 5246.
- **B. A TLS 1.2 client in this epic**: ECDHE with AEAD suites only, the extended master secret
  (RFC 7627), no renegotiation; a second handshake and key derivation, roughly a third more work,
  and its own negative cases.
- **C. TLS 1.2 as the next epic**, started when the model lane of `story:agent-tasks` or the
  operator meets sites it blocks.

### Settled in this draft (reversible)

The failure reasons above; the extra anchor file; the crate layout (`surf-tls`, the seam module,
`tls-test-server` as test tooling); the test plan above.

## Candidate stories, in the order a unit could build them

Drafted on 2026-11-22 as thirteen stories (`decomposes` this epic); the eight candidates of the
first draft, with `tls-x509` split into `tls-x509`, `tls-test-certs` and `tls-x509-path`,
`tls-handshake` into `tls-record`, `tls-handshake` and `tls-test-server`, and `tls-fetch-https`
into `tls-fetch-https` and `tls-session-anchors`. Order, by the coordinator (2026-11-22, round 1
decision 8 and round 2 decision 7):

1. Nine stories write only `crates/tls*`, `taskfiles/tls-*`, `docs/design/tls.md`, the lock
   check and its test (`taskfiles/lock-check.sh`, `taskfiles/lock-allow.txt`, the module
   `lock_check` of the test binary of `crates/surf`) and `.cargo/config.toml`; for them the
   coordinator writes the root `Taskfile.yml`, `Cargo.lock`, `README.md` and `AGENTS.md`. They
   are built beside the rounds 1 and 2 of surf: `tls-decisions` alone first; then
   `tls-lock-allow`, after `story:one-test-binary-per-crate` (which moves
   `crates/surf/tests/lock_check.rs`); then `tls-x509`, `tls-test-certs` and `tls-record` side
   by side; then `tls-x509-path`, `tls-anchors` and `tls-handshake` side by side; then
   `tls-test-server`.
2. Four are one unit, in the order `tls-spec`, `tls-fetch-https`, `tls-session-anchors`,
   `tls-release`. It starts at the first boundary between two rounds after the nine are merged,
   never beside a unit that writes `spec/`, `generated/`, `crates/fetch`, `crates/session`,
   `crates/agent`, `crates/surfd`, `crates/surf/src/lib.rs`, `conformance/src/target.rs` or
   `docs/agent-answer.md`. `tls-fetch-https` also needs `story:one-test-binary-per-crate` and
   the merge of `impl/security-review-fixes` (the destination rule,
   `story:security-review-fixes`).

`crates/tls` is born with one integration-test binary: `tests/main.rs`, its modules in
`tests/tls/`, no other `.rs` file beside `main.rs`. The `mod` lines of `crates/tls/src/lib.rs`
and `crates/tls/tests/main.rs` are written by the coordinator in a wave of more than one unit,
so the stories of one step can share the crate.

## Risks

- The chosen source does not build static for musl or on a macOS runner without a toolchain step
  (story 1 measures it on the forge before anything depends on it).
- A defect the client and its own test server share passes every test of the server; the
  recorded flights of an independent implementation and x509-limbo are the answer.
- RSA and P-384 verification in an unoptimised test build may cost tens of milliseconds per call
  (not measured); many handshakes per test file add up against the 120 s step.
- A stale bundled anchor list trusts a removed CA until the next release.
- A server limited to TLS 1.2 ends `TlsVersionUnsupported` when it reads `supported_versions`
  (it sends `protocol_version`) or answers with a TLS 1.2 ServerHello, and `TlsHandshakeFailed`
  when it ignores the extension and answers `handshake_failure`; the detail of that reason says
  a server limited to TLS 1.2 answers this way, but telling kind (b) apart from other handshake
  failures needs TLS 1.2 itself, which is the next epic (U3). Until then the outcome "a server
  that speaks only TLS 1.2" holds for kinds (a) and (c), and for (b) only as that detail.
- Revocation is not checked: a revoked certificate that has not expired loads. The documents say
  so where `https:` is described.
- The clock: validity is checked against a time the caller passes. `surf` and the session pass the
  system clock. Library tests pass a fixed time (2026-11-22) and never read the wall clock; tests
  through a binary run on the system clock, with certificates valid until 2125 and an expired one
  whose end lies in 2001, so they hold until 2125.

## Acceptance

`slot task check` exits 0 on `main` after `story:tls-release` and, in that run: the lock check
prints the allow-list line and fails a planted outside package (its test); the end-to-end test
`surf-agent go https://localhost:<port>/` against the test server prints `page 200 "text/html"
"https://localhost:<port>/"`; one test per reason of the negative suite prints `page failed
<reason>` with the reason the suite names (`story:tls-session-anchors`); the x509-limbo counts
of `crates/tls-conformance/expected/limbo.txt` are held; the bytes of RFC 8448 sections 3, 5
and 7 are reproduced (every secret, key, IV and protected record, `story:tls-record`, and the
client Finished, `story:tls-handshake`); the test step stays within 120 s. The release smoke
test of the next release passes on the four targets, and no shipped binary of it holds the
`test-tooling` marker (`story:tls-release`).

Agent task 5 of `story:agent-tasks` turning `pass` needs `story:agent-tasks` (draft,
`epic:agent-cli-target`) and the page text of round 1 (`look --as markdown`), neither of this
epic. That line belongs to whichever of the two lands last; this epic gives it the test server
and `surf-agent open --ca-file`.

## Decisions taken

- U1, the source of the cryptographic primitives: option A, `ring` 0.17.14. The operator, 2026-11-22 05:47 CET, relayed by the session `host-operator`: "A: ring 0.17.14". It adds 8 packages to `Cargo.lock` and needs a C compiler that also builds assembly on every build; the first story checks that setup for the musl release build on the forge's runners before anything else is built. It is the one exception principle 4 of the README plans; the lock check allows these packages by name and version and only for the crate `surf-tls`.

Taken by the coordinating agent on 2026-11-22, each reported to the operator, who can overrule it:

- U2, trust anchors: option B, the bundled list of Mozilla as curl publishes it, built into the binary and refreshed before each release, plus an extra anchor file per session. Reason: the same list on Linux and macOS, and reading the macOS keychain needs unsafe calls, which principle 5 forbids.
- U3, TLS 1.2: option C, out of this epic and drafted as the epic after it. Reason: this epic stays the size of one protocol version, and about one trusted site in four offers TLS 1.2 as its best protocol (SSL Pulse, 2025-06-01: 24.7 %), so the gap is closed next and not left open. Until then such a server fails with `TlsVersionUnsupported` when it reads `supported_versions` or answers with a TLS 1.2 ServerHello, and with `TlsHandshakeFailed` and a detail that says so when it ignores the extension.

- TLS 1.2 suites in the ClientHello: taken by the coordinating agent on 2026-11-22 (six TLS 1.2 ECDHE AEAD suites beside the three TLS 1.3 suites), reversed the same day after the fifth critic round because RFC 8446 section 9.3 says "A client sending a ClientHello MUST support all parameters advertised in it" (rfc8446.txt:5834-5836), and surf does not support TLS 1.2. The ClientHello offers the three TLS 1.3 suites and nothing else. A server limited to TLS 1.2 is reported by what it does: (a) it reads `supported_versions` and sends `protocol_version` before any ServerHello: `TlsVersionUnsupported`; (b) it ignores `supported_versions`, finds no suite in common and sends `handshake_failure` before any ServerHello: `TlsHandshakeFailed`, with a detail that names the alert and says in fixed words that a server limited to TLS 1.2 answers this way; (c) it sends a ServerHello that selects a version below 1.3: surf sends `protocol_version`, `TlsVersionUnsupported`, the downgrade values of RFC 8446 section 4.1.3 checked as before.

## Measured: the packages ring adds

Measured by the coordinating agent on 2026-11-22 with `cargo generate-lockfile` and `cargo tree` in a scratch package whose one dependency is `ring = "=0.17.14"`:

- `Cargo.lock` gains 19 entries: cc, cfg-if, find-msvc-tools, getrandom, libc, ring, shlex, untrusted, wasi, windows-sys, windows-targets and the eight `windows_*` crates of windows-targets.
- 8 of them are compiled for each of x86_64 and aarch64 Linux (gnu and musl) and macOS: cc, cfg-if, find-msvc-tools, getrandom, libc, ring, shlex, untrusted. The other 11 belong to Windows and WASI targets; cargo records them in the lock and never downloads or builds them for surf's targets.
- The count of 8 given with U1 was the compiled set for one target. The comparison of U1 counted every candidate that way, so the order of the candidates is the one the operator saw. The lock check of `tls-lock-allow` names all 19.
