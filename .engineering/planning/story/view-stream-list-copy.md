---
format: aep.planning-md/3
id: story:view-stream-list-copy
kind: story
status: draft
title: Measure, and if needed remove, the copy of the session list under the daemon's lock at each read of a stream
summary: Every read of a view stream copies the session list under surfd's table lock; found by the adversary of wave 6, not measured.
relations:
- decomposes: epic:live-view
revision: 1
---
## Context

Found by the adversary's pass over `story:view-surfd-serves` in wave 6
(`docs/plan/agent-wave-6.md`), by reading; no test shows it.

A stream of the live view learns of a change of the session list by comparing the list at
each wait: that was decided before wave 6, so that `crates/surfd/src/daemon.rs` gained no
channel. The reader of a stream therefore takes the daemon's table lock and copies every listed
session on each call of `Reader::next` (`crates/surfd/src/daemon.rs`, the snapshot used by
`crates/surfd/src/view.rs`): once per entry, and every 100 ms while the stream is idle, for each
open stream. A listed session carries its address and title, and ended sessions stay listed.
The copy is made while the lock that `open`, `go` and every other request of an agent need is
held.

Not measured: how large the copy is for real sessions, and how long the lock is held.

## Change

- Measure first: the bytes copied and the time the lock is held per read, with the largest
  list the bounds allow, and state both in this story before any code changes.
- If the measurement shows a hold that an agent's request can feel: a reader copies the list
  only when the list changed (a counter of changes read without the copy, or the copy made
  outside the lock from a shared value). The stream's events for the list stay as
  `docs/view-stream.md` states them.

## Acceptance

- The measurement is written into this story, with the command that produced it.
- If code changes: a test counts the copies made while a stream is idle and no session
  changes, and the count stays at the number the story states; the cases of
  `crates/surfd/tests/view.rs` and `crates/session/tests/view.rs` pass unchanged.
