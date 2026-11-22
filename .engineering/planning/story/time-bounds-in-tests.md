---
format: aep.planning-md/3
id: story:time-bounds-in-tests
kind: story
status: draft
title: Limits are proved at their value without a wall time; the upper bounds left are measured
summary: The wide bounds of story:test-step-holds-under-load let a limit applied at twice its value pass; 12 assertions in 7 test files still bound a wall time from above; three gaps were there before
relations:
- decomposes: epic:headless-rust-target
- depends_on: story:one-test-binary-per-crate
revision: 3
---
## Context

`story:test-step-holds-under-load` fixed five tests that asserted an upper bound on wall time close to the time they need: two of them failed in the gate of wave 10 on a loaded machine (2026-11-22), and one failed alone 2 of 10 times beside one busy loop per processor. The repository's `AGENTS.md` now says that a test bounds a time only by a wait for the event with a limit at least 10 times the time it takes alone, and proves a cost by a count.

That story gave something up, and its section "What the change gives up" lists it: with the wide bounds, a limit of `crates/view` applied at twice its value (write limit, receive limit, heartbeat), a read timeout of `crates/fetch` applied at 30 times its value, and a walk of hidden flags per reference that compares no attribute all pass. A bound on wall time tight enough to catch them fails on a loaded machine. What catches them without a wall time: a clock the test controls (the server and the fetch read the time through a seam the test can step), or a count (a counter of node visits in `crates/page` beside `attribute_visits()`).

Gaps that were there before, found by the adversary on `01a40e2`:

- `case_32` of `crates/view/tests/http.rs` has no lower bound: a keep-alive comment sent at once passes.
- No test of `crates/fetch` sends to a server that reads slowly: a send timeout applied per write and not to the whole send (`crates/fetch/src/lib.rs:719`) passes. A server that reads 64 KiB every half read timeout would catch it: a correct send ends at the read timeout, the defect runs about 25 s.
- `case_20` has a lower bound of 900 ms for a limit of 1 s: a receive limit at 0.9 of its value passes.

Assertions that still bound a wall time from above, found with `git grep -n -E 'assert!\(.*(elapsed|took).* <' -- crates conformance` on the commit of that story (11 in 6 files; the waits for an event in `crates/view/tests/http.rs:389` and `crates/view/tests/stream_bounds.rs:183` are not of this form), and one more of another spelling:

- `crates/agent/tests/usage.rs:74`: under 5 s.
- `crates/fetch/tests/fetch.rs:273`, `:290`, `:455`: under 5 s.
- `crates/fetch/tests/redirects.rs:461`: under 2 s.
- `crates/fetch/tests/review1_security_fetch.rs:215`, `:236`: under 3 s; `:264`: under 400 ms.
- `crates/fetch/tests/review2_fetch_limits.rs:199`: under 1 s.
- `crates/surfd/tests/in_process.rs:179`, `:206`: under `ALLOWED`.
- `crates/fetch/tests/fetch_bounds.rs:108` (`assert_ends_unreachable_in_time`, the drip tests): 5 times the read timeout.

None of them failed in the gate of wave 10 or in the four runs measured after it. Whether a bound is close to the time its test needs is not measured yet.

## Change

Not decomposed. Questions the story answers before it is proposed:

1. Whether `crates/view` and `crates/fetch` get a clock the test controls, so that a limit is proved at its value without a wall time (a change of the product; where the seam sits), and whether `crates/page` gets a count of node visits.
2. For each assertion listed: the time of the test alone, three times; the ratio of the bound to it; and where the ratio is under 10, the form it takes (a wait for the event, a lower bound where the claim is "not before", a count where the claim is a cost, the clock of question 1).
3. The three gaps that were there before: one test each.

## Acceptance

Not written. It will hold, per changed test, the planted defects of "What the change gives up" turned red again, each before the runner's 60 s, and ten runs beside one busy loop per processor.

## Out of scope

- The sizes of the limit tests: `story:limit-tests-at-small-limits`.
