---
format: aep.planning-md/3
id: story:agent-surfd-session-limit
kind: story
status: active
title: Bound the number of open sessions in surfd
summary: surfd bounds connections, requests and answers, and has no limit on open sessions; each is a process and a thread.
relations:
- decomposes: epic:agent-cli-target
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:02:27Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-21T03:02:27Z", actor: "human:timo", revision: 3}
---
## Context

Found in wave 3 (`agent-surfd-supervisor`, `docs/plan/agent-wave-3.md`). A caller that can
connect is the same user, who can start processes anyway, so this is a resource bound and not
a privilege boundary.

## Change

A limit on open sessions, with the refusal reason `docs/agent-answer.md` gives or gains for it.

## Acceptance

A test opens sessions up to the limit against a stand-in session; the next `open` is refused
with the documented reason and exit status, and closing one session makes room for one.
