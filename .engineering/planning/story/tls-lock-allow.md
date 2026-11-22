---
format: aep.planning-md/3
id: story:tls-lock-allow
kind: story
status: implemented
title: The lock check allows ring for surf-tls alone, and surf-tls wraps it in one module
summary: taskfiles/lock-allow.txt pins the packages of ring by name, version, source and checksum, only surf-tls may depend on them; crates/tls starts with crypto.rs, checked against Wycheproof vectors.
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-decisions
- depends_on: story:one-test-binary-per-crate
scope:
- confidence: cited
  path: .cargo/config.toml
- confidence: cited
  path: crates/surf/tests
- confidence: cited
  path: crates/tls/Cargo.toml
- confidence: cited
  path: crates/tls/src/crypto.rs
- confidence: cited
  path: crates/tls/src/error.rs
- confidence: cited
  path: crates/tls/src/lib.rs
- confidence: cited
  path: crates/tls/tests/data/wycheproof
- confidence: cited
  path: crates/tls/tests/main.rs
- confidence: cited
  path: crates/tls/tests/tls/alloc.rs
- confidence: cited
  path: crates/tls/tests/tls/seam.rs
- confidence: cited
  path: taskfiles/lock-allow.txt
- confidence: cited
  path: taskfiles/lock-check.sh
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:24:24Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"verification":1}}}
- {from: "proposed", to: "active", at: "2026-11-22T06:24:24Z", actor: "human:operator", revision: 10, decided_on: {"recorded":{"verification":1}}}
- {from: "active", to: "implemented", at: "2026-11-22T21:13:20Z", actor: "human:operator", revision: 16, decided_on: {"recorded":{"test_result":1,"verification":3}}, executor: "agent:coordinator"}
---
## Context

The lock check is the task `lock-check` of the root `Taskfile.yml` (lines 140-166): an `awk`
over the root `Cargo.lock` that names each package with a `source` line and fails, else prints
`Cargo.lock: <n> package(s), none from outside this repository`. Its test is
`crates/surf/tests/lock_check.rs`: each case copies `Taskfile.yml` and `Cargo.lock` into its own
directory under `CARGO_TARGET_TMPDIR` and runs `task lock-check` there (four cases:
`the_unchanged_lock_passes`, `a_registry_package_fails_naming_it`,
`a_git_package_fails_naming_it`, `a_missing_lock_fails`).

U1 (`epic:tls-https`, "Decisions taken"): ring 0.17.14; the lock check allows its packages by
name and version and only for `surf-tls`. The outcome of the epic adds the checksum, and that no
other crate of this repository depends on an allowed package. `AGENTS.md`, "Goals": "Which crate
is not chosen yet." `README.md` line 154, principle 4: "planned: audited cryptographic
primitives for TLS." Principle 5: no unsafe code in a hand-written crate.

`docs/design/tls.md` (`story:tls-decisions`): item 2 (the 19 packages and their checksums, 8
compiled per target; epic section "Measured: the packages ring adds"), item 4 (the `opt-level`
line, if any), item 5 (random sources), item 6 (the operations of the seam, the refused
algorithms, the feature `test-tooling`, the marker, the public modules, the `Error` kinds), item
10 (the Wycheproof files, the AES-192 skip). ring has no AES-192
(`ring-0.17.14/src/aead/algorithm.rs:149-163`).

`crates/fetch/tests/adversary_redirect_memory.rs` holds a counting allocator over all threads,
its cases serialised by a lock of their own; the parser-limits work made its allocator count per
thread (`crates/html/tests/common/mod.rs` at 3a8e882, branch `impl/html-parser-limits`). The
hostile cases of `story:tls-x509` and `story:tls-record` need one in the test binary of
`surf-tls`, where every module runs on parallel threads under `cargo test`; a per-thread count
keeps their peaks apart.

`story:one-test-binary-per-crate` makes `crates/surf/tests/lock_check.rs`, which stays in place, the module `lock_check` of the one test
binary of `crates/surf`; this story rewrites that file, so it depends on that story and starts
after it (round 2, decision 7).

Placement: the second of the nine stories that write only `crates/tls*`, `taskfiles/tls-*`, the
lock check and its test; built alone, after `tls-decisions` and
`story:one-test-binary-per-crate`, beside the rounds of surf.

## Change

- `taskfiles/lock-check.sh` (new, POSIX `sh` and `awk`; `sh taskfiles/lock-check.sh <lock>
  <allow-list>`):
  - every package with a `source` line matches a line of the allow-list on name, version,
    source and checksum, else `Cargo.lock: <name> <version> from <source>: not on the
    allow-list` (a known name with another version or checksum also names the listed line);
  - a package of this repository (no `source` line) that lists an allowed package among its
    `dependencies` is `surf-tls`, else `Cargo.lock: <package> depends on <allowed>; only
    surf-tls may`; an allowed package may depend on another allowed package;
  - a line of the allow-list that no package matches fails (`taskfiles/lock-allow.txt: <name>
    <version> is in no package of Cargo.lock`);
  - a missing lock fails as today; the summary is `Cargo.lock: <n> package(s), <m> from outside
    this repository, each on the allow-list`, and the line of today when `<m>` is 0.
- `taskfiles/lock-allow.txt` (new): one line per package of item 2, `<name> <version> <source>
  <checksum>`, sorted, no comment lines; the format is stated at the head of the script.
- The task `lock-check` stays in the root `Taskfile.yml`, so the copies of the test keep
  working; its body becomes `sh taskfiles/lock-check.sh Cargo.lock taskfiles/lock-allow.txt`
  and its summary says what the two rules are. The coordinator writes this edit when the wave
  has more than one unit; alone, the unit writes it.
- the module `lock_check` of the test binary of `crates/surf` (the file `crates/surf/tests/lock_check.rs`,
  which stays in place): each copy also holds
  `taskfiles/lock-check.sh` and `taskfiles/lock-allow.txt`; the four cases stay (the registry
  case appends `serde 1.0.0`, which is on no list) and the cases below are added.
- `crates/tls` (`surf-tls`, new, Apache-2.0, `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`):
  - `Cargo.toml`: `ring = "=0.17.14"` and no other dependency (the shipped random source is
    `ring::rand::SystemRandom`, item 5; `surf-secret` is not a dependency, no story of the epic
    uses it); the feature `test-tooling` (off by default); as dev-dependencies `surf-tls`
    itself by path with `test-tooling`, and `surf-html-conformance` by path for its JSON reader
    (`pub mod json`), so that no later story of the epic changes this manifest;
  - `src/crypto.rs`, the one file that names ring: AEAD seal and open for AES-128-GCM,
    AES-256-GCM and ChaCha20-Poly1305; SHA-256 and SHA-384, one-shot and running; HKDF extract
    and expand; X25519 and P-256 agreement with a key made from a random source; signature
    checks for rsa_pkcs1_sha256/384/512, rsa_pss_rsae_sha256/384/512, ecdsa_secp256r1_sha256,
    ecdsa_secp384r1_sha384 and ed25519, each behind a trait the tests can replace; `REFUSED`,
    the algorithms it refuses by name (every MD5 or SHA-1 signature, RSA keys under 2048 bits),
    with a function that names the refusal for a signature algorithm and key before any
    verification;
    the shipped random source `ring::rand::SystemRandom`; behind `test-tooling` the signing
    from PKCS#8 (ECDSA P-256 and P-384; Ed25519 by `Ed25519KeyPair::from_pkcs8_maybe_unchecked`,
    which reads the PKCS#8 v1 that `openssl genpkey` writes; RSA PKCS#1 and PSS from 2048 bits)
    and the fixed random sources of `ring::test::rand`, given buffers of their exact length,
    under `#[allow(deprecated)]`;
  - `src/error.rs`: the public `Error` of item 6 without its `Certificate` variant, which
    `story:tls-x509` adds; no other story changes this file (a needed change is a finding);
  - `src/lib.rs`: the crate documentation (what it is, that `crypto.rs` alone names ring, TLS
    1.2 not done, revocation not checked), the `mod` lines (the public modules of item 6), and
    under `test-tooling` only `pub const TEST_TOOLING_MARKER: &str = "surf-tls test-tooling,
    never in a release";` (item 6; `story:tls-handshake` makes `Connection` read it).
- `.cargo/config.toml`: the `opt-level` line of item 4, when item 4 has one.
- Tests in `crates/tls/tests/main.rs`, the one integration-test binary of the crate (the
  layout of `story:tls-decisions`: each module declared `#[path = "tls/<name>.rs"] mod
  <name>;`, its file in `crates/tls/tests/tls/`, which holds no `main.rs`, so cargo finds no
  second target): module `alloc`, the counting allocator in the form of
  `crates/html/tests/common/mod.rs` (commit
  3a8e882 on `impl/html-parser-limits`): a `GlobalAlloc` over `System` whose live, peak and
  allocated counts are kept per thread in `thread_local!` cells, read by `peak_while` and
  `allocated_while`, so a test reads only its own allocations also when tests run side by side
  in one process; the only `unsafe` under `crates/tls`, test only; and module `seam`, over
  `crates/tls/tests/data/wycheproof/` (the files of item 10, unchanged, with `LICENSE` and
  `SOURCE`: `https://github.com/C2SP/wycheproof`, commit, date, Apache-2.0, sha256 of each
  file, the AES-192 count).
- For the coordinator: `README.md` principle 4 (ring 0.17.14, "reviewed as a dependency in the
  rustls audit of Cure53, 2020", https://cure53.de/pentest-report_rustls.pdf, in place of
  "planned"); `AGENTS.md` "Goals" (the crate, its version, used by `surf-tls` alone, allowed by
  `taskfiles/lock-allow.txt`; "The root `Cargo.lock` proves the rest" reworded), "Layout" (the
  `crates/tls` entry, and that the counting allocator of `crates/tls/tests` is the second
  `unsafe` of the repository), "Commands" (`task lock-check`); the root `Cargo.lock`.

## Acceptance

`slot task lock-check` on the story branch prints `Cargo.lock: <n> package(s), 19 from outside
this repository, each on the allow-list`, and `cargo nextest run -p surf-tls -p surf` passes
with the lock cases and the seam cases below.

Lock (module `lock_check` of the test binary of `crates/surf`):

1. The lock of the branch passes and prints that summary line.
2. `serde 1.0.0` from crates.io appended: fails, `Cargo.lock: serde 1.0.0 from
   registry+https://github.com/rust-lang/crates.io-index: not on the allow-list`.
3. `ring 0.17.13` in place of 0.17.14: fails naming it and the listed 0.17.14.
4. ring 0.17.14 with one hex digit of its checksum changed: fails naming it.
5. ring 0.17.14 with a `git+https://` source: fails naming it.
6. `surf-fetch` with `ring` among its dependencies: fails, `Cargo.lock: surf-fetch depends on
   ring; only surf-tls may`.
7. An allow-list line for a package the lock does not hold: fails naming the line.
8. No lock: fails, as today.

Seam (`crates/tls/tests/main.rs`, module `seam`):

9. One test per Wycheproof file: every vector of result `valid` passes, every `invalid` one is
   refused, `acceptable` ones are counted; each prints `wycheproof <file>: <n> vectors, <v>
   valid passed, <i> invalid refused, <a> acceptable, <s> skipped`, and the only skips are the
   AES-192 vectors of `aes_gcm_test` and AES-GCM nonces other than 96 bits, each counted by
   name and the AES-192 count equal to `SOURCE`. The `x25519` and `ecdh_secp256r1` vectors run
   with their private keys through a fixed source and compare the shared secret. No invalid
   vector is accepted.
10. SHA-384 of `abc` equals the FIPS 180-4 example value; a running SHA-256 and SHA-384 fed
    in three uneven parts equals the one-shot hash.
11. Signing under `test-tooling`: for each of ECDSA P-256, P-384, Ed25519, RSA PKCS#1 SHA-256
    and PSS SHA-256, a key from PKCS#8 signs a message and the check of the seam accepts it
    and refuses it with one byte flipped; the Ed25519 key is a PKCS#8 v1 file as `openssl
    genpkey` writes it (a constant of the test, its command in `SOURCE`); ECDSA under one fixed
    source signs one message twice to equal bytes.
12. Refused by name: the seam names an RSA-1024 key `WeakKey`, and a sha1WithRSAEncryption and
    an md5WithRSAEncryption signature each `WeakAlgorithm`, before any verification; an
    AES-128-GCM record with one flipped tag bit does not open; a P-256 public key off the curve
    is refused by agreement.
13. `sources_match_their_digests`: the sha256 of each vendored file, through the seam, equals
    its `SOURCE` line; with one byte changed in a copy under `CARGO_TARGET_TMPDIR` it fails
    naming the file.
14. `only_crypto_names_ring`: no file under `crates/tls/src` but `crypto.rs` holds `ring::`;
    the report shows it failing once with a planted `use ring::digest;` in `src/lib.rs`.
15. `alloc`: a test that allocates 1 MiB reads a peak of at least 1 MiB, and one that
    allocates nothing reads a peak under 64 KiB while another thread of the same test
    allocates and holds 4 MiB (the counts are per thread).

Budget (`$HOME/beyond10x/AGENTS.md`, "Tests stay fast"): no test over 60 s, the test step
within 120 s (`task test` prints it; the report gives it before and after, and the build time
ring adds), no network and no model call in the test step. The seam tests together under 5 s,
each over 1 s named with its time.

## Out of scope

Any TLS protocol code; the X.509 reader; `release.yml` (`story:tls-release`); a second source
of primitives.

## Writes

`taskfiles/lock-check.sh`, `taskfiles/lock-allow.txt`, the module `lock_check` of the test
binary of `crates/surf` (the file
`crates/surf/tests/lock_check.rs`, a module of `tests/main.rs` since `story:one-test-binary-per-crate`; this story starts after that one, by its `depends_on` edge,
so that the two never rewrite the file side by side), `crates/tls/` (`Cargo.toml` with `autotests = false` and `[[test]] name = "main"`,
`src/lib.rs`, `src/crypto.rs`, `src/error.rs`, `tests/main.rs`, `tests/tls/alloc.rs`,
`tests/tls/seam.rs`, `tests/data/wycheproof/`), `.cargo/config.toml` when item 4 asks.
Coordinator: the root `Taskfile.yml` (task body), `Cargo.lock`, `README.md`, `AGENTS.md`.
`crates/tls/Cargo.toml` and `src/crypto.rs` are written by this story alone; `src/error.rs`
gains one variant from `tls-x509`; `crates/tls/src/lib.rs` and `tests/main.rs` gain `mod` lines
from `tls-x509`, `tls-record`, `tls-x509-path`, `tls-anchors` and `tls-handshake`. Nothing runs
beside it in the epic.

## As built

What differs from the text above, as of 6cff209 (branch `wave/tls-1`):

- `crypto::Secret` and `crypto::Digest` overwrite their bytes when dropped, with a plain
  `fill(0)` (`crates/tls/src/crypto.rs:223-227` for `Digest`, `:463-467` for `Secret`): a
  finding of the security review of the seams, fixed in d299d0d; the Change above does not name
  it. `security_seams` holds at compile time that each of the four secret-holding types has a
  `Drop` of its own.
- "no other story changes this file" (`src/error.rs`): `story:tls-record` (e3bfaa7,
  `KeyLimitReached`), `story:tls-handshake` (b082959, 25546b3) and the review of the seams
  (ee6c101: `Error::Host`, `error::excerpt`, `EXCERPT_BYTES_MAX` 64; 2047347) changed it; its
  module documentation (`crates/tls/src/error.rs:4-8`) names who added what.
- Under `test-tooling` `src/lib.rs` also makes the module `handshake` public
  (`crates/tls/src/lib.rs:25-28`), for the trace mode of `story:tls-handshake`.
- `lock_check` holds cases beyond the eight above: `a_line_cargo_does_not_write_fails_naming_it`
  and six `adversary_*` cases.

Not shown by a test or a run:

- Acceptance 14, the failure with a planted `use ring::digest;` in `src/lib.rs`: held in memory
  by `seam::the_ring_search_sees_every_spelling`, not as a file planted under `src/`; a run
  with the line planted would show it.
- That the bytes of `Secret` and `Digest` read zero after a drop (it needs `unsafe` to read
  them).
- Budget, "the seam tests together under 5 s": in the gate run on 32df67f (load 105 to 143)
  they took 6.8 s together, the longest 0.9 s; on 6cff209 none is among the ten slowest (the
  tenth took 1.6 s). The sum at normal load would show it; the gate's JUnit report of
  6cff209 was overwritten by a later step.
