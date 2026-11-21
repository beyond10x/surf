---
format: aep.planning-md/3
id: story:agent-file-open-race
kind: story
status: active
title: Open a token file and the lock file without blocking on a FIFO
summary: A file swapped for a FIFO between the check and the open blocks the open; the standard library has no O_NONBLOCK.
relations:
- decomposes: epic:agent-cli-target
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:02:26Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-21T03:02:26Z", actor: "human:timo", revision: 3}
---
## Context

Found in wave 2 (`agent-cli-client`, the token file) and wave 3 (`agent-surfd-supervisor`, the
lock file): each checks that the path is a regular file the caller owns, then opens it, and
compares device and inode afterwards. A regular file swapped for a FIFO in that window makes
the open block, because the standard library offers no non-blocking open without `libc`.
Records: `docs/plan/agent-wave-2.md`, `docs/plan/agent-wave-3.md`.

## Change

Open both files in a way that cannot block (for example a helper thread with a deadline, or
an open through `/proc/self/fd` of a directory handle), with a test that swaps a FIFO in.

## Acceptance

A test replaces the file by a FIFO between the check and the open; the command ends with its
documented refusal within a deadline the test sets, and no thread is left blocked.
