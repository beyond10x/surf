---
format: aep.planning-md/3
id: story:tls-test-server-sends-alerts
kind: story
status: draft
title: The TLS test server sends an encrypted alert, and cases hold what a fetch reports for it
summary: two modes of the test server (an alert after the ServerHello, an alert after the response head) and the cases on them; one planted change of a reason mapping that survives every test today fails a case
relations:
- decomposes: epic:tls-https
- informed_by: story:tls-fetch-https
revision: 1
---
## Why

The adversary pass on `story:tls-fetch-https` (2026-11-24, on `cb432d2`) left one one-line change
of `crates/fetch/src/lib.rs` that no test fails on: a `protocol_version` alert after the
ServerHello reported as `TlsVersionUnsupported` (the mapping at line 986 of that commit). The
independent review of the same unit could check "an alert the server sends after the response
head" by reading the code only. Both for one reason: the TLS test server
(`crates/tls-test-server`) has no mode that sends an encrypted alert, neither inside the handshake
after the ServerHello nor after it.

## Change

1. Two modes of the test server: an encrypted alert of a named description sent after the
   ServerHello in place of the rest of the handshake, and one sent after the response head in
   place of the rest of the body.
2. Cases in `crates/fetch/tests/` on those modes: the reason a fetch reports for
   `protocol_version`, `handshake_failure` and `internal_error` after the ServerHello, and for an
   alert after the head; each reason as `spec/domains/net.yaml` and `docs/agent-answer.md` name it.

## Acceptance

1. With the mapping of the adversary's surviving change planted (a `protocol_version` alert after
   the ServerHello reported as `TlsVersionUnsupported`), at least one of the new cases fails; the
   plant is removed.
2. An alert after the response head is never a complete page: the case fails when the fetch
   returns status 200 with the bytes received so far.
3. Each new case runs under 1 s; the cases together add under 3 s to the test step of
   `surf-fetch`; the report names the step's time before and after.
4. No change of a public item of `surf-tls`; the modes live in the test server.

## Out of scope

A new failure reason. The reasons stay the five of `story:tls-spec`.
