---
format: aep.planning-md/3
id: story:time-bounds-in-tests
kind: story
status: draft
title: The eleven upper bounds on wall time left in the tests are measured
summary: After story:test-step-holds-under-load, 11 assertions in 6 test files still bound a wall time from above; none failed in the gate, and the ratio of bound to time is not measured
relations:
- decomposes: epic:headless-rust-target
- depends_on: story:one-test-binary-per-crate
revision: 1
---
## Context

`story:test-step-holds-under-load` fixed five tests that asserted an upper bound on wall time close to the time they need: two of them failed in the gate of wave 10 on a loaded machine (2026-11-22), and one failed alone 2 of 10 times beside one busy loop per processor. The repository's `AGENTS.md` now says that a test bounds a time only by a wait for the event with a limit at least 10 times the time it takes alone, and proves a cost by a count.

Assertions of the same form that are left, found with `git grep -n -E 'assert!\(.*(elapsed|took).* <' -- crates conformance` on the commit of that story (11 in 6 files; the three waits for an event in `crates/view/tests/http.rs:389` and `crates/view/tests/stream_bounds.rs:183` are not of this form):

- `crates/agent/tests/usage.rs:74`: under 5 s.
- `crates/fetch/tests/fetch.rs:273`, `:290`, `:455`: under 5 s.
- `crates/fetch/tests/redirects.rs:461`: under 2 s.
- `crates/fetch/tests/review1_security_fetch.rs:215`, `:236`: under 3 s; `:264`: under 400 ms.
- `crates/fetch/tests/review2_fetch_limits.rs:199`: under 1 s.
- `crates/surfd/tests/in_process.rs:179`, `:206`: under `ALLOWED`.

None of them failed in the gate of wave 10 or in the four runs measured after it. Whether a bound is close to the time its test needs is not measured yet.

## Change

Not decomposed. For each assertion: the time of the test alone, three times; the ratio of the bound to it; and where the ratio is under 10, the form of `story:test-step-holds-under-load` (a wait for the event, a lower bound where the claim is "not before", a count where the claim is a cost).

## Acceptance

Not written. It will hold, per changed test, a planted defect that still turns it red before the runner's 60 s, and ten runs beside one busy loop per processor.

## Out of scope

- The sizes of the limit tests: `story:limit-tests-at-small-limits`.
