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
  path: crates/tls/tests/tls/names.rs
- confidence: cited
  path: crates/tls/tests/tls/path.rs
- confidence: cited
  path: taskfiles/tls-x509-conformance.yml
revision: 8
---
## Context

The second half of the candidate `tls-x509` of `epic:tls-https` (the first is
`story:tls-x509`, which reads certificates and owns `LeafKey`, `TrustAnchor` and the trait
`ServerCertVerifier` of `crates/tls/src/verify.rs`). The epic: "A path from the leaf through the
intermediates the server sent to a trust anchor, with a depth bound, every signature verified;
RSA keys under 2048 bits and SHA-1 signatures refused"; "Host name checking by RFC 9525:
dNSName and iPAddress of subjectAltName only, no Common Name, a wildcard only as the whole
left-most label" (RFC 9525 sections 2, 6.3, 6.4); "x509-limbo vendored at a pinned commit, its
counts held in the gate as `surf-html-conformance --check` holds the parser counts, so that RFC
5280 sections are claimed complete only when it passes them" (`AGENTS.md`, "Goals").

Inputs: the certificates and `INDEX` of `crates/tls-test-certs/certs/`
(`story:tls-test-certs`), the limbo data of `crates/tls/tests/data/x509-limbo/`
(`story:tls-x509`), the signature checks and `REFUSED` of `crates/tls/src/crypto.rs`.
`docs/design/tls.md` items 6 (owners, the verifier interface), 7 (the reasons and the closed
list of check names, `WeakAlgorithm` and `WeakKey` decided before any verification), 9 (the
bounds) and 13 (the time is passed by the caller; `LIMBO_TIME`). The task `html-conformance`
(`taskfiles/html-conformance.yml`) is the form of the gate step.

Placement: one of the nine stories that write only `crates/tls*` and `taskfiles/tls-*`; built
after `tls-x509` and `tls-test-certs`, beside `tls-anchors` and `tls-handshake`, beside the
rounds of surf.

## Change

- `crates/tls/src/path.rs`: `PathVerifier`, the implementation of `ServerCertVerifier` over a
  set of `TrustAnchor`s, at the time the caller passes. It builds and validates a path (RFC 5280
  section 6.1, the parts the epic names): candidates by issuer name and key identifiers, at most
  the intermediates and candidates of item 9, no certificate twice on a path; for each
  certificate first the policy of `REFUSED` (`WeakAlgorithm`, `WeakKey`), then the signature
  through the seam (`BadSignature`); validity; basicConstraints cA and pathLenConstraint on every
  issuer; keyUsage keyCertSign on an issuer and digitalSignature on the leaf when present;
  extKeyUsage serverAuth on the leaf when present; nameConstraints of issuers and of the anchor
  over dNSName and iPAddress entries below them; an unknown critical extension refuses. A
  certificate the server sent that the reader of `story:tls-x509` refuses is
  `NotTrusted(Unreadable)` with the reader error. Each refusal is a `CertificateRejection`
  naming its check.
- `crates/tls/src/names.rs`: RFC 9525. A dNSName matches without regard to ASCII case; `*` only
  as the whole left-most label, matching one label; an IP host matches an iPAddress entry octet
  for octet (4 or 16) and never a dNSName; the Common Name is never read.
- `crates/tls-conformance` (`surf-tls-conformance`, new, test tooling, not shipped): its binary
  runs every SERVER case of the limbo data through `PathVerifier` at the validation time of the
  case; a case without `validation_time` runs at `LIMBO_TIME`, a constant of
  `crates/tls-conformance/src/lib.rs` equal to the `commit-date` that
  `crates/tls/tests/data/x509-limbo/SOURCE` records for the pinned limbo commit
  (`story:tls-x509`, item 13), never at the wall clock, so
  `expected/limbo.txt` does not move with the clock. It prints `x509-limbo: <exist> cases,
  <run> run, <pass> passed`, a line per limbo group,
  one per RFC 5280 section it maps cases to (`incomplete` unless every case of it passes), and
  the cases not run with their reason (CLIENT cases, CRL, a feature out of scope).
  `crates/tls-conformance/expected/limbo-kinds.txt`, committed: one line per pattern of limbo
  case ids (such as `rfc5280::nc::*`, `webpki::eku::*`, `*::expired*`) with the kind and check
  a failure of such a case is expected to end with; every FAILURE case matches exactly one line.
  `--check --base <revision>` against `crates/tls-conformance/expected/limbo.txt` with the
  rules of `surf-html-conformance --check`: a count below the expected one fails, an expected
  count below its value at the base fails, a base git cannot resolve fails, a count above passes
  and prints that the file can be raised; a FAILURE case that ends with another kind than its
  line of `limbo-kinds.txt` is printed and counted as not passed.
- `taskfiles/tls-x509-conformance.yml`: the task `tls-x509-conformance`, run from the
  repository root, base revision `TLS_BASE` defaulting to `HTML_BASE` (so that the
  `HTML_BASE=origin/main` the forge and `check-runner` already set holds for it), log
  `.out/tls-x509-conformance.log`, its time printed. For the coordinator: the include and the
  `check` line after `html-conformance`.
- Tests in `crates/tls/tests/main.rs`, modules `path` and `names` (files in `tests/tls/`); in
  `crates/tls-conformance/tests/main.rs`, module `check`.

## Acceptance

`cargo nextest run -p surf-tls -p surf-tls-conformance` passes with one test per case of
`crates/tls-test-certs/certs/INDEX`, each ending as `INDEX` says, and `slot task
tls-x509-conformance` prints the `x509-limbo:` line and exits 0 against
`crates/tls-conformance/expected/limbo.txt`.

1. With the root of each case as the only anchor and the time of `INDEX`: the `ok-*` cases
   return the leaf key; `expired` and `not-yet-valid` give `Expired`; `wrong-host`,
   `wildcard-two-labels`, `common-name-only` and `ip-mismatch` give `NameMismatch`;
   `untrusted-root`, `bad-signature`, `intermediate-not-ca`, `path-too-long`,
   `pathlen-exceeded`, `leaf-no-digital-signature`, `issuer-no-key-cert-sign`,
   `unknown-critical-extension`, `name-constraints-excluded` and `eku-client-only` give
   `NotTrusted` with the check `INDEX` names, which is one of the closed list of item 7 and
   equal to the name `Check::ALL` prints for it: among them `pathlen-exceeded` gives
   `NotTrusted(PathTooLong)` (a pathLenConstraint of 0 broken, within the intermediates item 9
   allows), and `leaf-no-digital-signature` and `issuer-no-key-cert-sign` give
   `NotTrusted(KeyUsage)`; `leaf-v1` gives `NotTrusted(Unreadable)` with the reader error
   `UnsupportedVersion`; `ok-rsa-pss` returns the leaf key (RFC 8446 section 9.1).
2. Weak algorithms by policy: `sha1` and `md5` give `NotTrusted(WeakAlgorithm)` and `rsa-1024`
   `NotTrusted(WeakKey)`, each different from the `NotTrusted(BadSignature)` of
   `bad-signature`; a copy of the `sha1` leaf with one signature byte flipped still gives
   `WeakAlgorithm` (the policy comes before any verification).
3. Names: `*.example.test` matches `a.example.test`, not `example.test`, not
   `a.b.example.test`; `f*.example.test` matches nothing; `EXAMPLE.test` matches `example.TEST`;
   iPAddress 127.0.0.1 matches host `127.0.0.1`, a dNSName `127.0.0.1` does not; `::1` matches
   its 16 octets.
4. `1000-sans` matches on its last entry in under 1 s (time in the report).
5. Hostile: `candidate-limit` (50 certificates sent, 48 of them intermediates with one subject
   and different keys, two of those cross-signed in a loop): `NotTrusted(CandidateLimit)` when
   the candidate bound of item 9 is reached, in under 1 s.
6. limbo: the `x509-limbo:` line with the numbers of the run, written into `expected/limbo.txt`
   from it; every FAILURE case maps to one line of `limbo-kinds.txt`; every case limbo expects
   to fail that passes (a certificate wrongly accepted) is listed by id and kind in the report
   and in the expected file, and none of them has a kind or check the negative suite of
   `INDEX` names.
7. `check`: a pass count below the expected fails; an expected count below its value at the base
   fails; an unresolvable base fails; a count above passes and prints the raise line; a case id
   matched by no line of `limbo-kinds.txt` fails naming it.
8. `slot task tls-x509-conformance` ends within 60 s (time in the report).
9. `limbo_time`: `LIMBO_TIME` equals the commit date of the pinned limbo commit as `SOURCE`
   records it (`commit-date`, `story:tls-x509`); the runner takes the time of each case from the
   case or from `LIMBO_TIME` and reads no system clock (the test finds no `SystemTime` under
   `crates/tls-conformance/src`).
10. `every_check_has_a_case`: each of the twelve check names of item 7 is the expected check of
    at least one case of `INDEX`, and each such case gives `NotTrusted` with that check (the
    test reads the names from `Check::ALL` and fails naming a check without a case).

Budget: no test over 60 s, the test step within 120 s, no network, no model call; the limbo run
is its own step, not part of `task test`; tests pass the time of the case, never the wall clock.

## Out of scope

Revocation (CRL, OCSP), certificate transparency, client certificates, the anchor set itself
(`story:tls-anchors`), the handshake.

## Writes

`crates/tls/src/path.rs`, `src/names.rs`, `tests/tls/path.rs`, `tests/tls/names.rs`,
`crates/tls-conformance/`, `taskfiles/tls-x509-conformance.yml`. Coordinator: `mod` lines of
`crates/tls/src/lib.rs` and `tests/main.rs` in a wave of more than one unit, the root
`Taskfile.yml`, `Cargo.lock`, `AGENTS.md`. Beside it run `story:tls-anchors` and
`story:tls-handshake` (other files of `crates/tls`).
