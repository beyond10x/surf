---
format: aep.planning-md/3
id: story:agent-surfd-idle-exit-race
kind: story
status: active
title: A client that meets surfd at its idle exit starts a fresh one
summary: A client that connects after surfd decided to exit and before it removed its socket gets end of file and exits 3.
relations:
- decomposes: epic:agent-cli-target
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:02:27Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-21T03:02:27Z", actor: "human:timo", revision: 3}
---
## Context

Found in wave 3 (`agent-surfd-supervisor`, `docs/plan/agent-wave-3.md`).

## Change

The client treats end of file before any answer byte on a connection it did not start as
"no daemon" and goes through its start path once.

## Acceptance

A test holds a stand-in daemon at the point between its decision to exit and the removal of
the socket, runs the client, and gets the answer of a freshly started daemon, exit status 0.
