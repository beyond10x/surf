---
format: aep.planning-md/3
id: story:tls-x509-path
kind: story
status: draft
title: surf-tls validates a server certificate path and its host name, held to x509-limbo in the gate
summary: RFC 5280 path building and validation with bounds, RFC 9525 name checks; every negative case ends with its reason; tls-x509-conformance holds the limbo counts as html-conformance holds the parser counts.
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-decisions
- depends_on: story:tls-x509
- depends_on: story:tls-test-certs
scope:
- confidence: cited
  path: crates/tls-conformance
- confidence: cited
  path: crates/tls/src/names.rs
- confidence: cited
  path: crates/tls/src/path.rs
- confidence: cited
  path: crates/tls/tests/names.rs
- confidence: cited
  path: crates/tls/tests/path.rs
- confidence: cited
  path: taskfiles/tls-x509-conformance.yml
revision: 3
---
## Context

The second half of the candidate `tls-x509` of `epic:tls-https` (the first is
`story:tls-x509`, which reads certificates). The epic: "A path from the leaf through the
intermediates the server sent to a trust anchor, with a depth bound, every signature verified;
RSA keys under 2048 bits and SHA-1 signatures refused"; "Host name checking by RFC 9525:
dNSName and iPAddress of subjectAltName only, no Common Name, a wildcard only as the whole
left-most label" (RFC 9525 sections 2, 6.3, 6.4); "x509-limbo vendored at a pinned commit, its
counts held in the gate as `surf-html-conformance --check` holds the parser counts, so that RFC
5280 sections are claimed complete only when it passes them" (`AGENTS.md`, "Goals": "as soon as
we mark it complete, it is complete as to the specs").

Inputs: the certificates and `INDEX` of `crates/tls-test-certs/certs/`
(`story:tls-test-certs`), the limbo data of `crates/tls/tests/data/x509-limbo/`
(`story:tls-x509`), the signature checks of `crates/tls/src/crypto.rs`. `docs/design/tls.md`
items 6 (the verifier interface, the error kinds), 7 (the reasons) and 9 (the bounds). The task
`html-conformance` (`taskfiles/html-conformance.yml`) is the form of the gate step: a floor per
count, compared with the expected file and with its value at a base revision (`HTML_BASE`,
`origin/main` on the forge).

## Change

- `crates/tls/src/path.rs`: build and validate a path (RFC 5280 section 6.1, the parts the
  epic names): candidates by issuer name and key identifiers, at most the intermediates and
  candidates of item 9, no certificate twice on a path; each signature checked through the
  seam; validity at the time the caller gives; basicConstraints cA and pathLenConstraint on every
  issuer; keyUsage keyCertSign on an issuer and digitalSignature on the leaf when present;
  extKeyUsage serverAuth on the leaf when present; nameConstraints of issuers and of the anchor
  over dNSName and iPAddress entries below them; an unknown critical extension refuses; RSA
  under 2048 bits and SHA-1 refuse. Each refusal names the check and maps to one kind of item 7.
- `crates/tls/src/names.rs`: RFC 9525. A dNSName matches without regard to ASCII case; `*` only
  as the whole left-most label, matching one label; an IP host matches an iPAddress entry octet
  for octet (4 or 16) and never a dNSName; the Common Name is never read.
- `crates/tls-conformance` (`surf-tls-conformance`, new, test tooling, not shipped): its binary
  runs every SERVER case of the limbo data through `path` and `names` at the validation time of
  the case and prints `x509-limbo: <exist> cases, <run> run, <pass> passed`, a line per limbo
  group, one per RFC 5280 section it maps cases to (`incomplete` unless every case of it
  passes), and the cases not run with their reason (CLIENT cases, CRL, a feature out of scope).
  `--check --base <revision>` against `crates/tls-conformance/expected/limbo.txt` with the
  rules of `surf-html-conformance --check`: a count below the expected one fails, an expected
  count below its value at the base fails, a base git cannot resolve fails, a count above passes
  and prints that the file can be raised.
- `taskfiles/tls-x509-conformance.yml`: the task `tls-x509-conformance`, run from the
  repository root, base revision `TLS_BASE` defaulting to `HTML_BASE` (so that the
  `HTML_BASE=origin/main` the forge and `check-runner` already set holds for it), log
  `.out/tls-x509-conformance.log`, its time printed. For the coordinator: the include and the
  `check` line after `html-conformance`.
- Tests in `crates/tls/tests/main.rs`, modules `path` and `names`; in
  `crates/tls-conformance/tests/main.rs`, module `check`.

## Acceptance

`cargo test -p surf-tls -p surf-tls-conformance` passes with one test per case of
`crates/tls-test-certs/certs/INDEX`, each ending as `INDEX` says, and `slot task
tls-x509-conformance` prints the `x509-limbo:` line and exits 0 against
`crates/tls-conformance/expected/limbo.txt`.

1. With the root of each case as the only anchor and the time of `INDEX`: the `ok-*` cases
   return the leaf key; `expired` and `not-yet-valid` give `CertificateExpired`; `wrong-host`,
   `wildcard-two-labels`, `common-name-only` and `ip-mismatch` give `CertificateNameMismatch`;
   `untrusted-root`, `bad-signature`, `intermediate-not-ca`, `path-too-long`,
   `unknown-critical-extension`, `rsa-1024`, `sha1`, `name-constraints-excluded` and
   `eku-client-only` give `CertificateNotTrusted`, each with the check `INDEX` names.
2. Names: `*.example.test` matches `a.example.test`, not `example.test`, not
   `a.b.example.test`; `f*.example.test` matches nothing; `EXAMPLE.test` matches `example.TEST`;
   iPAddress 127.0.0.1 matches host `127.0.0.1`, a dNSName `127.0.0.1` does not; `::1` matches
   its 16 octets.
3. `1000-sans` matches on its last entry in under 1 s (time in the report).
4. Hostile: 50 certificates sent, two CAs cross-signed in a loop: `CertificateNotTrusted` within
   the candidate bound of item 9, in under 1 s.
5. limbo: the `x509-limbo:` line with the numbers of the run, written into
   `expected/limbo.txt` from it; every case limbo expects to fail that passes (a certificate
   wrongly accepted) is listed by id in the report and in the expected file; none of them is of a
   kind the negative suite names.
6. `check`: a pass count below the expected fails; an expected count below its value at the base
   fails; an unresolvable base fails; a count above passes and prints the raise line.
7. `slot task tls-x509-conformance` ends within 60 s (time in the report).

Budget: no test over 60 s, the test step within 120 s, no network, no model call; the limbo run
is its own step, not part of `task test`; tests use the time of the case, never the wall clock.

## Out of scope

Revocation (CRL, OCSP), certificate transparency, client certificates, the anchor set itself
(`story:tls-anchors`), the handshake.

## Writes

`crates/tls/src/path.rs`, `src/names.rs`, `tests/path.rs`, `tests/names.rs`,
`crates/tls-conformance/`, `taskfiles/tls-x509-conformance.yml`. Coordinator: `mod` lines of
`crates/tls/src/lib.rs` and `tests/main.rs` in a wave of more than one unit, the root
`Taskfile.yml`, `Cargo.lock`, `AGENTS.md`. Beside it can run `story:tls-anchors` and
`story:tls-handshake` (other files of `crates/tls`).
