---
format: aep.planning-md/3
id: story:fetch-early-answer
kind: story
status: draft
title: surf-fetch returns an answer a server sent before it read the request body
summary: A server that answers and closes on a large request body ends the fetch Unreachable and its answer is lost; one that answers and holds the connection blocks until the read timeout. Found in wave 9, measured.
relations:
- decomposes: epic:agent-cli-target
revision: 1
---
## Context

Found in wave 9 by the unit of `story:agent-fetch-post` and by the adversary's pass over it
(`docs/plan/agent-wave-9.md`). `surf-fetch` writes the whole request, head and body, before it
reads anything of the answer (`exchange` in `crates/fetch/src/lib.rs`).

- A server that answers and closes before it read the request body: with a body that fits
  the socket's buffers the write succeeds and the answer is returned (a 413 in 50 of 50 runs
  with 7 bytes); with a large body the write fails and the fetch ends `Unreachable`, so the
  answer the server sent is lost (10 of 10 runs with 4 MiB). RFC 9112 section 9.5 says a
  client that sends a body SHOULD watch the connection for an error response while it
  sends. The crate documentation states both outcomes and that a caller may rely on neither.
- A server that answers and keeps the connection open without reading a 16 MiB body blocks
  the fetch until the read timeout (2 s in the unit's probe), and the fetch then ends
  `Unreachable`.

An agent that submits a large form to a server that refuses it early therefore reads
"unreachable" where a browser shows the server's answer.

## Change

Measured first, against a server of the test's own: bodies of 64 KiB, 1 MiB and 16 MiB
against a server that answers 413 and closes, one that answers and holds the connection, and
one that answers 100 Continue and then reads; what the fetch returns and after how long.

Decided in this story before code changes:

- whether the fetch reads while it writes (a second thread, or non-blocking sockets with the
  standard library only: the crate has no dependency), or sends `Expect: 100-continue` for a
  body over a size and waits a bounded time;
- what a fetch returns when the write failed and an answer was read: the answer, or
  `Unreachable` as now;
- what bounds the time of a write to a server that does not read (a write timeout beside the
  read timeout), and whether `Limits` gains it.

## Acceptance

- A case sends a `Post` of 4 MiB to a server that answers 413 and closes without reading,
  and reads what the story decided, the same in 20 of 20 runs.
- A case sends a `Post` of 16 MiB to a server that answers and holds the connection, and
  ends within the bound the story decided.
- The crate documentation's sentence on such a server states the one outcome.
- The cases of `crates/fetch` pass without a change to their assertions, or each changed
  assertion is named with its reason; the memory cases hold their bounds.
