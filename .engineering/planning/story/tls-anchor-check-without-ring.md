---
format: aep.planning-md/3
id: story:tls-anchor-check-without-ring
kind: story
status: draft
title: surf-agent checks a --ca-file file without linking ring
summary: the client's check of an anchor file uses no ring code, yet surf-agent and surfd link ring through surf-tls since story:tls-session-anchors; the check builds without ring and without reading the bundled list
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-session-anchors
- informed_by: story:tls-session-anchors
revision: 2
---
## Why

`story:tls-session-anchors` lets the client refuse a `--ca-file` file before it connects, with
the anchor check of `surf-tls`. For that, `crates/agent/Cargo.toml` depends on `surf-tls`, and
`surf-tls` depends on `ring` with no feature gate. Measured
on `77c4b83` (`cargo tree --offline -p <package> -i ring -e normal`): `ring` reaches `surf-agent`
(through `surf-tls`) and `surfd` (through `surf-agent`); before the unit it reached
`surf-session` and `surf-cli` alone. The check itself calls no `ring` item: in `crates/tls/src`,
`ring::` is named in `crypto.rs` only, and the client uses the PEM, DER, X.509
and anchors modules. The client also builds the whole bundled list (`Anchors::bundled()`, the
roots of `crates/tls/src/anchors/cacert.pem`) only to check one file.

`task lock-check` still passes and still means what `AGENTS.md` says: `ring` is a direct
dependency of `surf-tls` alone. What changed is which binaries link it.

## Change

To be made exact by the scoper and the critics; the direction:

- `surf-tls` gets a way to check an anchor file that builds without `ring`: a feature that
  leaves the handshake, the record layer and signature checks out, or a small crate of the
  reader modules that `surf-tls` and `surf-agent` both use.
- `surf-agent` uses that, and checks the file against the bounds and the CA rules without
  reading the bundled list.
- `AGENTS.md`: which packages link `ring`.

## Acceptance

Draft:

1. `cargo tree -p surf-agent -i ring -e normal` and `cargo tree -p surfd -i ring -e normal` each
   print that no package named `ring` is in the graph; for `surf-session` and `surf-cli` they
   print a path.
2. Every test of the modules `ca_file` and `adversary_ca_file` of `surf-agent` passes unchanged.
3. `slot task lock-check` prints the allow-list line as before.
4. The release smoke test's marker search finds no `ring` symbol in `surf-agent` and `surfd` of
   a release build (the search the story names, with its time).

## Out of scope

Removing `ring` from `surf-session` or `surf-cli`; a second implementation of the checks.

## Writes

`crates/tls/Cargo.toml`, `crates/tls/src/lib.rs`, `crates/agent/Cargo.toml`,
`crates/agent/src/client.rs`, `Cargo.lock`, `AGENTS.md` (coordinating role).
