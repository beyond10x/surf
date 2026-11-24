---
format: aep.planning-md/3
id: story:tls-test-server-answer-timeout
kind: story
status: draft
title: The mode tests of the TLS test server wait longer than 200 ms for an answer
summary: the 200 ms read timeout of the silent modes is also the wait for an answer in the other modes, and one test missed it once at 0.232 s on a loaded host; the answering modes get a timeout of their own
relations:
- decomposes: epic:tls-https
- informed_by: story:tls-test-server
revision: 1
---
## Why

`crates/tls-test-server/tests/modes.rs` gives the client's socket a read timeout of 200 ms in
every mode (`MODE_TIMEOUT`, acceptance 3 of `story:tls-test-server`). The bound is what the
modes that send nothing are measured against. The modes in which the server answers use it
too, so an answer that takes longer than 200 ms fails the test with `TimedOut`.

Seen once, while `story:session-follow-press-https` was built on a loaded build host:
`modes::no_close_notify_is_truncated` ended with `Err(TimedOut)` at 0.232 s where it expects
`Err(Truncated)`. It passed in the rerun 22 s later (49 of 49 tests of the binary) and 5 of 5
times alone. The review of that unit read the changed code and found the order of the server's
writes unchanged.

## Change

To be made exact by the scoper and the critics; the direction:

- The tests of modes in which the server answers wait longer than 200 ms for the answer (a
  second constant); the modes that send nothing keep the 200 ms bound and their lower-bound
  assertion.
- `story:tls-test-server` acceptance 3 says which modes the 200 ms bound belongs to.

## Acceptance

Draft:

1. Each test of `modes` names which of the two timeouts it uses; the tests of the silent modes
   still assert a wait of at least 200 ms.
2. With a delay of 300 ms planted before the server's first write in `NoCloseNotify`,
   `modes::no_close_notify_is_truncated` passes; with the 200 ms constant it fails with
   `TimedOut` (the plant is tried and removed).
3. The module's tests take no longer than before on an idle host (both times in the report);
   no test over 5 s.

## Out of scope

The timeouts of `surf-tls` itself; other test modules.

## Writes

`crates/tls-test-server/tests/modes.rs`, `.engineering/planning/story/tls-test-server.md`
(coordinating role).
