---
format: aep.planning-md/3
id: story:agent-token-hash
kind: story
status: active
title: Keep only a hash of the session token
summary: 'Deferred: the session process keeps the hash of the token and compares hashes in constant time, once the cryptographic crate is chosen.'
tags:
- deferred
relations:
- informed_by: epic:agent-cli-target
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:02:27Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-21T03:02:27Z", actor: "human:timo", revision: 3}
---
## What

The session process keeps only a hash of the token and compares the hash of a presented token in
constant time (`docs/design/agent-interface.md`, section 3).

## Why deferred

No hash function exists in this repository, and `AGENTS.md` ("Goals") takes cryptographic
primitives from an audited third-party crate that is not chosen yet and forbids writing them here.
Until then `agent-session-process` keeps the 32 token bytes in process memory only (decision D15
of `docs/plan/agent-plan-revision-1.md`).

## What would un-defer it

The operator choosing the cryptographic crate.

## Acceptance

`cargo test -p surf-session` passes with a test that finds no copy of the token bytes in the
session state after `open`, only their hash, and the token tests of `agent-session-process` still
passing.
