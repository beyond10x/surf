---
format: aep.planning-md/3
id: story:agent-close-window
kind: story
status: active
title: Close a window, and close the window of a closed session
summary: 'Deferred: a CloseWindow command and a binding, so a closed session leaves no Open window.'
tags:
- deferred
relations:
- informed_by: epic:agent-cli-target
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:02:26Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-21T03:02:26Z", actor: "human:timo", revision: 3}
---
## What

A command that closes a window, saying what happens to its tabs and their navigations
(`spec/domains/shell.yaml`, the `UNMAPPED:` marker on `Window`), and a binding so a closed session
closes the window it opened (`docs/plan/agent-cli-draft-1.md`, marker 10).

## Why deferred

A closed session leaves an empty Open window in a process that exits anyway, so nothing an agent
sees depends on it. Decision D15 of `docs/plan/agent-plan-revision-1.md`.

## What would un-defer it

A session that outlives its first window (more than one window per session), or a front that keeps
one process for many sessions.

## Acceptance

`slot task check` exits 0 with a passing scenario in which `CloseSession` leaves the window of the
session Closed and its tab Closed.
