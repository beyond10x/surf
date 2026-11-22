---
format: aep.planning-md/3
id: story:fetch-head-memory
kind: story
status: draft
title: surf-fetch states and holds a bound on memory while a head is read
summary: Each parsed header field takes 48 bytes besides its bytes, so a section of short fields held 24.59 times the header limit; measured in wave 7, no bound stated.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:one-test-binary-per-crate
revision: 1
---
## Context

Found in wave 7 by the unit that fixed the adversary's findings in the redirects
(`docs/plan/agent-wave-7.md`), with a counting allocator; older than the redirects.

`surf-fetch` bounds the bytes of a header section (`Limits::max_header_bytes`, 64 KiB by
default). While a head is read, each parsed field is kept with 48 bytes besides its name and
value (`Head` in `crates/fetch/src/lib.rs`). A header section of many short fields therefore
holds many times the header limit. Measured at the defaults: 1,611,203 bytes (24.59 times the
header limit) for a section of fields of three bytes; 3,445,951 bytes (52.58 times) when that
head answers a request after a redirect to a URL of 64 KiB of empty path segments, which the
fetch still holds parsed. The crate documentation ("Memory") says that no bound is stated for
this moment.

No body is held at that moment, so the peak of a whole fetch stays below what it holds while
a body at the body limit is read (16,917,505 bytes at the defaults). With a caller's small
`max_body_bytes` the head is the peak.

## Change

Decided in this story before code changes, one of:

- a bound on the number of header fields of one response (a section over it is
  `MalformedResponse`), with the number argued from real servers;
- a head kept as the bytes read with offsets into them, so that a field costs a fixed small
  number of bytes besides its own;
- only the fields the crate reads are kept (`Location`, `Content-Type`, `Content-Length`,
  `Transfer-Encoding`) and every other field is counted and dropped.

The URL held parsed while the next head is read is part of the same moment: say what of it
is needed there.

## Acceptance

- The crate documentation states a bound for the moment a head is read, as a sum of named
  terms with a number at the defaults.
- A case of `crates/fetch/tests/adversary_redirect_memory.rs` measures a header section of
  fields of three bytes at the header limit, alone and after a redirect to a URL of 64 KiB,
  and compares what was held with that sum written out.
- The existing suites of `surf-fetch` pass without a change to their cases.
