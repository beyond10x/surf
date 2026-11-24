---
format: aep.planning-md/3
id: story:tls-fetch-memory-at-bounds
kind: story
status: draft
title: The memory a fetch holds for a certificate chain at the TLS bounds is measured and bounded
summary: a fixture chain of 65,535 bytes and one of 64 certificates, a case that measures the peak of a fetch on each, and the bound in the fetch documentation in place of the sentence that says it was not measured
relations:
- decomposes: epic:tls-https
- informed_by: story:tls-fetch-https
revision: 1
---
## Why

The fetch documentation's "Memory" section (`crates/fetch/src/lib.rs`, written in
`story:tls-fetch-https`) says that the memory a fetch holds for a server's certificate chain at
the TLS bounds was not measured: a Certificate message of 65,535 bytes, up to 64 certificates.
No fixture of that size exists. The independent review of the unit (2026-11-24) could not decide
its bound on memory for that reason, and the adversary pass did not get to it. What is measured:
the 50-certificate `candidate-limit` chain is refused with `CandidateLimit` in 7.5 ms and held
139,137 bytes at its peak.

A server chooses its chain, so this is memory a hostile server controls.

## Change

1. A fixture chain at the bounds in `crates/tls-test-certs`: a Certificate message as close to
   65,535 bytes as the encoding allows, and one of 64 certificates. Made once and committed with a
   digest of its sources, or built once per test run; never per test.
2. A case that fetches from the test server with each fixture and measures the peak of the
   bytes allocated during the fetch.
3. The measured numbers and a bound in the "Memory" section and in `AGENTS.md`, replacing the
   sentence that says it was not measured.

## Acceptance

1. The two cases print the peak in bytes; the bound written in the "Memory" section is at most
   twice the larger peak, and a case fails when a fetch holds more than the bound.
2. With a plant that keeps every received handshake message for the life of the connection, a
   case fails; the plant is removed.
3. A Certificate message over the bound of `surf-tls` is refused before its bytes are held; the
   case names the reason reported.
4. Each case runs under 5 s, timed in the report; the generator of the fixtures is timed before
   and after, and no test runs it.

## Out of scope

A change of the bounds of `surf-tls`. If a measured peak shows that a bound is too high, that is
a finding for a story of its own.
