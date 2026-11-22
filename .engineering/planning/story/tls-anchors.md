---
format: aep.planning-md/3
id: story:tls-anchors
kind: story
status: draft
title: surf-tls carries the root list of Mozilla as curl publishes it, and takes one extra anchor file
summary: cacert.pem compiled in with its SOURCE, an extra PEM file checked against bounds, anchors-refresh outside the gate and anchors-age for the release.
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-decisions
- depends_on: story:tls-x509
- depends_on: story:tls-test-certs
scope:
- confidence: cited
  path: crates/tls/src/anchors
- confidence: cited
  path: crates/tls/tests/anchors.rs
- confidence: cited
  path: taskfiles/tls-anchors.sh
- confidence: cited
  path: taskfiles/tls-anchors.yml
revision: 3
---
## Context

U2 (`epic:tls-https`, "Decisions taken"): option B, "the bundled list of Mozilla as curl
publishes it, built into the binary and refreshed before each release, plus an extra anchor
file per session". The epic: curl publishes it as PEM (https://curl.se/docs/caextract.html,
2026-11-05, 118 certificates, with a `.sha256`); a task outside the gate refreshes it and writes
a `SOURCE` with its checksum, as the vendored test data has; the release procedure refreshes a
list older than 90 days. `docs/design/tls.md` items 8 (the extra file: at most 32 KiB and 64
certificates, every one a CA) and 11 (the file read, its licence, how a root the X.509 reader
refuses is taken). The PEM reader and the certificate reader are those of `story:tls-x509`.

## Change

- `crates/tls/src/anchors/cacert.pem` (the curl file, unchanged) and `SOURCE` (its URL, the date
  of the list, the date fetched, the sha256 from the `.sha256`, the count, the licence of the
  list).
- `crates/tls/src/anchors/mod.rs`: `Anchors::bundled()` reads the included file once into trust
  anchors (subject, public key, nameConstraints); a root the certificate reader refuses is taken
  as subject and key only, as item 11 says; `count()`, `bundled_date()` and `bundled_sha256()`
  for `surf --anchors`. `Anchors::with_extra_pem(&[u8])` adds the extra file, or refuses it with
  an `AnchorError` that names the block: `NoCertificate`, `NotACertificate { index, label }`,
  `BadPem { index }`, `BadCertificate { index, error }`, `NotACa { index }`, `TooLarge`,
  `TooMany`.
- `taskfiles/tls-anchors.sh` and `taskfiles/tls-anchors.yml`: `anchors-refresh` fetches
  `cacert.pem` and its `.sha256` from curl.se with `curl`, refuses a file whose sha256 differs,
  and writes `cacert.pem` and `SOURCE`; `FILE=<pem> SHA256=<file>` takes local files instead (the
  test uses these); it never runs in `check`, `check-runner` or a workflow. `anchors-age` (no
  network) prints `anchors: <n> certificates of <date>, <d> days old` and fails when the list is
  older than 90 days at `DATE` (default today). For the coordinator: the include.
- Tests in `crates/tls/tests/main.rs`, module `anchors`.

## Acceptance

`cargo test -p surf-tls` passes with the cases below, `Anchors::bundled().count()` equals the
count in `crates/tls/src/anchors/SOURCE`, and `slot task anchors-age` prints the age of the list
and exits 0.

1. The bundled count equals `SOURCE`, and the sha256 of the included bytes, through the seam,
   equals `SOURCE`.
2. The extra file `crates/tls-test-certs/certs/ok-rsa2048/root.pem`: accepted, count plus 1.
3. Hostile extra files, each refused with the error named: 1 KiB of random bytes:
   `NoCertificate`; a `PRIVATE KEY` block: `NotACertificate { index: 0, label: "PRIVATE KEY" }`
   (the key is not kept); the leaf of `ok-rsa2048` (no CA): `NotACa { index: 0 }`; base64 cut
   in the middle: `BadPem { index: 0 }`; 32 KiB plus one byte: `TooLarge`, before any block is
   read; 65 small certificates: `TooMany`.
4. `anchors-refresh` with `FILE` and a `SHA256` file that does not match (a test running the
   script under `CARGO_TARGET_TMPDIR`): exit 1, `cacert.pem` and `SOURCE` unchanged; with a
   matching pair: both written, `SOURCE` naming the new sha256.
5. `anchors-age` with `DATE` 91 days after the list date: exit 1 naming the age; with `DATE` 90
   days after: exit 0.

Budget: reading the bundled list in a test takes under 1 s (time in the report); no test over
60 s, the test step within 120 s, no network (the refresh task is outside the gate and its test
uses local files), no model call.

## Out of scope

The system store of the host (U2 option A, not taken); pinning and allow-lists; how the extra
file reaches a fetch (`story:tls-fetch-https`, `story:tls-session-anchors`); the release step
(`story:tls-release`).

## Writes

`crates/tls/src/anchors/`, `crates/tls/tests/anchors.rs`, `taskfiles/tls-anchors.sh`,
`taskfiles/tls-anchors.yml`. Coordinator: `mod` lines in a wave of more than one unit, the root
`Taskfile.yml` include, `AGENTS.md`. Beside it can run `story:tls-x509-path` and
`story:tls-handshake`. `story:tls-release` adds the age check to `docs/releasing.md` and the
licence of the list to `NOTICE`.
