---
format: aep.planning-md/3
id: story:tls-lock-allow
kind: story
status: draft
title: The lock check allows ring for surf-tls alone, and surf-tls wraps it in one module
summary: taskfiles/lock-allow.txt pins the packages of ring by name, version, source and checksum, only surf-tls may depend on them; crates/tls starts with crypto.rs, checked against Wycheproof vectors.
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-decisions
scope:
- confidence: cited
  path: .cargo/config.toml
- confidence: cited
  path: crates/surf/tests/lock_check.rs
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
  path: crates/tls/tests/seam.rs
- confidence: cited
  path: taskfiles/lock-allow.txt
- confidence: cited
  path: taskfiles/lock-check.sh
revision: 3
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

`docs/design/tls.md` (`story:tls-decisions`): item 2 (the packages and checksums), item 4 (the
`opt-level` line, if any), item 6 (the operations of the seam, the error kinds), item 10 (the
Wycheproof files).

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
    <version> is in no package of Cargo.lock`), so the list never outgrows the lock;
  - a missing lock fails as today; the summary is `Cargo.lock: <n> package(s), <m> from outside
    this repository, each on the allow-list`, and the line of today when `<m>` is 0.
- `taskfiles/lock-allow.txt` (new): one line per package of item 2, `<name> <version> <source>
  <checksum>`, sorted, no comment lines; the format is stated at the head of the script.
- The task `lock-check` stays in the root `Taskfile.yml`, so the copies of the test keep
  working; its body becomes `sh taskfiles/lock-check.sh Cargo.lock taskfiles/lock-allow.txt`
  and its summary says what the two rules are. In a wave of more than one unit this edit is the
  coordinator (handed over in the report); alone, the unit writes it.
- `crates/surf/tests/lock_check.rs`: each copy also holds `taskfiles/lock-check.sh` and
  `taskfiles/lock-allow.txt`; the four cases stay (the registry case appends `serde 1.0.0`,
  which is on no list) and the cases below are added.
- `crates/tls` (`surf-tls`, new, Apache-2.0, `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`):
  - `Cargo.toml`: `ring = "=0.17.14"`; `surf-secret` by path; as a dev-dependency
    `surf-html-conformance` by path, for its JSON reader (`pub mod json`), so that no later
    story of the epic changes this manifest;
  - `src/crypto.rs`, the one file that names ring: the operations of item 6, AEAD seal and open
    for the three suites, SHA-256 and SHA-384 (one-shot and running), HKDF extract and expand,
    X25519 and P-256 agreement behind a trait the tests replace, signature checks for
    rsa_pkcs1_sha256/384/512, rsa_pss_rsae_sha256/384/512, ecdsa_secp256r1_sha256,
    ecdsa_secp384r1_sha384 and ed25519 behind a trait the tests replace, and signing from
    PKCS#8 for test tooling;
  - `src/error.rs`: the public error kinds of item 6; later stories add their detailed errors in
    their own modules and do not change this file (a needed change is a finding);
  - `src/lib.rs`: the crate documentation (what it is, that `crypto.rs` alone names ring, TLS
    1.2 not done) and the `mod` lines.
- `.cargo/config.toml`: the `opt-level` line of item 4, when item 4 has one.
- Tests in `crates/tls/tests/main.rs` (the one integration-test binary of the crate), module
  `seam`, over `crates/tls/tests/data/wycheproof/` (the files of item 10, unchanged, with
  `LICENSE` and `SOURCE`: `https://github.com/C2SP/wycheproof`, commit, date, Apache-2.0,
  sha256 of each file).
- For the coordinator: `README.md` principle 4 (ring 0.17.14, "reviewed as a dependency in the
  rustls audit of Cure53, 2020", https://cure53.de/pentest-report_rustls.pdf, in place of
  "planned"); `AGENTS.md` "Goals" (the crate, its version, used by `surf-tls` alone, allowed by
  `taskfiles/lock-allow.txt`; "The root `Cargo.lock` proves the rest" reworded), "Layout" (the
  `crates/tls` entry), "Commands" (`task lock-check`); the root `Cargo.lock`.

## Acceptance

`slot task lock-check` on the story branch prints `Cargo.lock: <n> package(s), <m> from outside
this repository, each on the allow-list` with `<m>` the count of item 2, and `cargo test -p
surf-tls -p surf` passes with the lock cases and the seam cases below.

Lock (`crates/surf/tests/lock_check.rs`):

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
   valid passed, <i> invalid refused, <a> acceptable, <s> skipped` with the reason of each skip
   (an AES-GCM nonce other than 96 bits; for agreement, every vector that needs a given private
   key, item 5). No invalid vector is accepted.
10. Hostile: an AES-128-GCM record with one flipped tag bit does not open; a P-256 public key
    off the curve is refused by agreement; an RSA-1024 key is refused by every RSA check.
11. `sources_match_their_digests`: the sha256 of each vendored file, through the seam, equals
    its `SOURCE` line; with one byte changed in a copy under `CARGO_TARGET_TMPDIR` it fails
    naming the file.
12. `only_crypto_names_ring`: no file under `crates/tls/src` but `crypto.rs` holds `ring::`;
    the report shows it failing once with a planted `use ring::digest;` in `src/lib.rs`.

Budget (`$HOME/beyond10x/AGENTS.md`, "Tests stay fast"): no test over 60 s, the test step
within 120 s (`task test` prints it; the report gives it before and after, and the build time
ring adds), no network and no model call in the test step. The seam tests together under 5 s,
each over 1 s named with its time.

## Out of scope

Any TLS protocol code; the X.509 reader; `release.yml` (`story:tls-release`); a second source
of primitives.

## Writes

`taskfiles/lock-check.sh`, `taskfiles/lock-allow.txt`, `crates/surf/tests/lock_check.rs`,
`crates/tls/` (`Cargo.toml`, `src/lib.rs`, `src/crypto.rs`, `src/error.rs`, `tests/main.rs`,
`tests/seam.rs`, `tests/data/wycheproof/`), `.cargo/config.toml` when item 4 asks. Coordinator:
the root `Taskfile.yml` (task body), `Cargo.lock`, `README.md`, `AGENTS.md`.
`crates/tls/Cargo.toml`, `src/crypto.rs` and `src/error.rs` are written by this story alone;
`crates/tls/src/lib.rs` and `tests/main.rs` gain `mod` lines from `tls-x509`, `tls-record`,
`tls-x509-path`, `tls-anchors`, `tls-handshake` and `tls-fetch-https`. `story:tls-spec` (beside
this one) writes `crates/surf/src/lib.rs`, not the test file here.
