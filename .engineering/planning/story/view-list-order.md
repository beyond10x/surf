---
format: aep.planning-md/3
id: story:view-list-order
kind: story
status: draft
title: The session list's order and opened_at agree under two opens at the same moment
summary: surfd orders the list by spawn order and reads the time of opening before the lock; the contract says oldest opened_at first.
relations:
- decomposes: epic:live-view
revision: 1
---
## Context

Found by the adversary's pass over `story:view-surfd-serves` in wave 6
(`docs/plan/agent-wave-6.md`), by reading; no test shows it.

`docs/view-stream.md` states that the session list is ordered "oldest `opened_at` first".
`surfd` orders the list by the order in which the sessions were spawned, and reads the time of
opening before it takes the lock under which the order is assigned
(`crates/surfd/src/daemon.rs`). Two `open`s at the same moment can therefore be listed with the
later `opened_at` first.

## Change

One of the two, decided in this story before code changes:

- the time of opening is read under the lock that assigns the order, so that both orders
  agree; or
- the contract states the order as the order of opening as `surfd` assigned it, and
  `opened_at` as a time that can be equal or out of order by the resolution of the clock.

## Acceptance

- A test opens sessions from several threads at once, many times, and the list is in the
  order the contract states every time.
- `docs/view-stream.md` and the code state the same order.
