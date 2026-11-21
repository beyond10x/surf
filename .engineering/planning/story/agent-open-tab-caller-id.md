---
format: aep.planning-md/3
id: story:agent-open-tab-caller-id
kind: story
status: draft
title: OpenTab takes a caller-chosen tab identity
summary: 'Deferred: a binding from SessionOpened opens the tab of a session, so the session process no longer issues OpenWindow and OpenTab itself.'
tags:
- deferred
relations:
- informed_by: epic:agent-cli-target
revision: 1
---
## What

`surf.shell.OpenTab` takes a caller-chosen `tab_id`, as `surf.net.Fetch` takes `request_id`, so a
binding from `surf.agent.SessionOpened` can open the tab of a session and the session process no
longer issues `OpenWindow` and `OpenTab` itself (`docs/plan/agent-cli-draft-1.md`, marker 6).

## Why deferred

It changes a command of an existing component, and `agent-session-spec` works without it (the
session process issues the three commands). Decision D15 of `docs/plan/agent-plan-revision-1.md`.

## What would un-defer it

A second front on the `agent` component (the scenario runner or another protocol) that must open a
session without the session process.

## Acceptance

`slot task check` exits 0 with a passing scenario in which `OpenSession` alone leads, through a
binding, to an Open tab with the identity the session named.
