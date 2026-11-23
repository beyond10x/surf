---
format: aep.planning-md/3
id: story:view-large-entry-test-time
kind: story
status: draft
title: The page test of an entry over 1 MiB ends within 20 s under load
summary: an entry over 1 MiB took 27.2 s at a load of 68 to 82 and 35.0 s at 126 to 127, the slowest page test at a load over 80; where its time goes is not measured; draft, candidates not chosen
relations:
- decomposes: epic:live-view
- informed_by: story:page-test-step-within-budget
- informed_by: story:view-draw-cost
revision: 1
---
## Why

`story:page-test-step-within-budget` statement 7 bounds the page test `an entry over 1 MiB`
(`view-tests/tests/first-page.spec.mjs`) at 20 s in each of three runs and says: over 20 s in one
of them, its size is decided in a story of its own. This is that story (decision D317).

Measured, the test's time and the host's 1-minute load of the run:

- before the page-step story, on `ecfaaf8`, three runs: 12.9 s, 4.8 s and 17.3 s;
- after it, on `1555f58`, three runs at a load of 61.60 to 59.06, 59.06 to 68.47 and 68.47 to
  82.03: 10.9 s, 11.9 s and 27.2 s; 34.2 s in a run beside a planted test that slept 70 s;
- earlier the same day: 29.7 s at a load of 106 to 140, 35.0 s at 126 to 127, 16.0 s at 111 to
  117, 8.1 s at 90 falling to 53, 27.3 s at 96 to 132.

The test holds the folder's bound of 60 s in every one of these runs. It is the slowest page test
in each run at a load over 80, and the page step of the last run above took 91.2 s of 120 s.

The test's claim is an address of 60,000 bytes and an answer over 1,048,576 bytes shown by the
view; the size is the claim. Where its time goes is not measured: the fixture writing the entry,
`surfd` serving it, the page parsing and drawing it, or the assertions reading it back.
`story:view-draw-cost` (draft) measured one draw of a session of 331 entries at 0.3 to 2.9 s on
the page's main thread at a load of 114 to 153; whether the same draw is this test's cost is
open.

## Change

Draft. Not decided:

1. Measure first: the test alone, three times, with the time of each of its steps (fixture write,
   first response, the draw, each assertion), at the load of the moment, and the same at a load
   over 100 when the host has one. No load is made on purpose.
2. Candidates, to be chosen after statement 1: the page draws a large entry in bounded work per
   frame (with `story:view-draw-cost`); the view cuts what it shows of a large entry and says
   so, with the whole entry behind an action; the test asserts on the answer of `/api` and one
   drawn mark instead of the whole drawn text. A smaller entry is not a candidate while the
   claim is "over 1 MiB".

## Acceptance

Draft: `an entry over 1 MiB` takes at most 20 s in each of three consecutive runs of the page
step at a load of at least 100, with its claim unchanged, and the page step prints its budget
line within its 120 s in each.

## Out of scope

The budget of the page step and its line (`story:page-test-step-within-budget`).
