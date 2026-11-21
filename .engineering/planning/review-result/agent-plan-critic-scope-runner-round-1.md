---
format: aep.planning-md/3
id: review-result:agent-plan-critic-scope-runner-round-1
kind: review-result
status: active
title: Plan critic (scope), round 1, epic e2e-scenario-runner
relations:
- reviews: epic:e2e-scenario-runner
revision: 1
---
needs-revision

All 23 promises I traced have an owning story. Three findings remain: one story sits outside the epic's text, one story's acceptance needs a tree change that the parser epic rules out, and the recording format cannot replay a redirected page.

**What I read:** 22 artifacts. These were the epic `epic:e2e-scenario-runner` and the epics `epic:agent-cli-target` and `epic:html-parser`. I read the 11 `runner-` stories, 12 `agent-` stories (`session-process`, `change-and-log`, `session-spec`, `cli-client`, `fetch-redirects`, `fetch-post`, `links-spec`, `request-body-spec`, `form-port`, `fill-pick`, `press-submit`, `follow-links`, `back-forward`, plus parts of `page-path-spec`, `answer-format` and `forms-spec`) and the `html-` stories `document-port`, `actionable-elements`, `document-spec` and `decisions`. I also read sections 3 to 11 of `docs/design/agent-interface.md`. Commands: `aep plan artifact show`, `aep plan artifact list --kind story --status draft`, `aep plan artifact graph`, `cat -n` and `grep` on `.engineering/planning/` and `crates/surf/src/lib.rs`.

**Promises:** 20 from the epic (the 5 Outcome items split into 14 promises, 3 open questions, 2 constraints, 1 acceptance). Section 7 of the design adds 3: expected and actual `changed` lines on a failure, live or replayed chosen per run, and no process start per step. Total 23, all traced to a story. Promise 4, "each HTTP request and response", is covered only for responses without a redirect (finding 3). The epic's exclusions (scripts, HTTPS, CDP, speed target) are respected by every story.

## Findings

1. **`story:runner-detail-option` (warning, reach beyond the parent).**
   - Quoted: "The coordinating agent assigned the option to this epic, beside that story, on 2026-11-21" (`.engineering/planning/story/runner-detail-option.md:24-25`).
   - What is wrong: the epic's Outcome (`.engineering/planning/epic/e2e-scenario-runner.md:24-43`) promises `--record`, `scenario from`, `run`, determinism and secrets. It never promises `--detail` on `surf-agent` commands, and no other artifact records the assignment. So the story's outcome cannot be traced to its parent.
   - To settle it: add the assignment to the epic's Outcome, or move the story under `epic:agent-cli-target`.
   - Finding line: `story:runner-detail-option` — the story's outcome (`--detail` on every `surf-agent` command) is in no sentence of the epic's Outcome, and the assignment is recorded only in the story itself — `.engineering/planning/story/runner-detail-option.md:22-25`

2. **`story:runner-detail-option` (blocker, needs something no story delivers).**
   - Quoted: "`fill` with `--detail` on a page whose form control changes shows the normal `~` line and the attribute-level line for the same control" (`.engineering/planning/story/runner-detail-option.md:50-52`).
   - What is wrong: `html-document-port` keeps each document's tree with "No mutation path exists, so a node lives as long as its document" (`.engineering/planning/story/html-document-port.md:39-41`). `agent-form-port` keeps filled values in the control rows, "values as filled" (`agent-form-port.md:41-42`), not in the tree. No story makes `fill` change the node tree, so an attribute-level line for a fill cannot appear.
   - To settle it: change the acceptance to a tree change that can actually happen, or have a story in `epic:html-parser` or `epic:agent-cli-target` deliver one.
   - Finding line: `story:runner-detail-option` — the acceptance requires an attribute-level line after `fill`, but `html-document-port` gives the tree no mutation path and `agent-form-port` keeps filled values in rows, so no story delivers the tree change it asserts — `.engineering/planning/story/html-document-port.md:39-41`

3. **`story:runner-formats` (blocker, promise narrowed).**
   - Quoted: "`exchange <seq> during <action-seq> <method> "<address>"` blocks (request body if any, then either `status <code> <media type>` with the body, or `failed <reason>`, or `unanswered`)" (`.engineering/planning/story/runner-formats.md:41-42`).
   - What is wrong: `agent-links-spec` follows redirects inside the transport and makes "`Answer::Response` carries the response URL" (`agent-links-spec.md:66`). The agent epic's own login test answers 303 to `/home` (`agent-press-submit.md:35`). The exchange block has no place for the response URL, so a replayed redirected request commits at a different address and breaks the `expect page` line. `runner-e2e` posts to a page that does not redirect, so its acceptance would not catch this.
   - To settle it: the exchange block records the response URL and `runner-replay-transport` answers with it, with one replay case for a redirect.
   - Finding line: `story:runner-formats` — the exchange block holds no response URL although `Answer::Response` carries one after `agent-links-spec`, so a replayed redirected request (post, 303, get) commits at another address and the "each HTTP request and response" promise is narrowed to responses without a redirect — `.engineering/planning/story/runner-formats.md:41-42`

## What I could not establish

- **Stale wording (design lane):** `runner-decisions:133`, `runner-detailed-changes:58-59` and `agent-change-and-log:67-68` still say no story claims `--detail`, while `runner-detail-option` does.
- **Ordering of `crates/surf/src/main.rs` (parallel-safety lane):** `runner-surf-cli-package` removes this file. `agent-request-body-spec`, `agent-fetch-post` and `html-document-port` all write it, and nothing orders them against the move.
- **Open and close in the recording (acceptance lane):** `runner-record-session` does not say whether `open` (with its initial `Go`) and `close` are written as action blocks. Its acceptance lists only `go, follow, fill, fill, press` (`runner-record-session.md:64-66`), while `agent-page-path-spec` creates `Action` rows for `OpenSession` and `CloseSession`.
- **Where typed values come from (design lane):** `runner-record-session` says it needs "the action log (... input ...)". But `agent-page-path-spec` says "An action never stores a value the agent typed", so a recorded `fill` value has to come from the request itself. The story does not say so.
- **`--record` and the model (design lane):** whether the `record` field added to the `open` input struct (`runner-record-option.md:31-33`) becomes an ESS command input, which would conflict with D10. The story leaves this to its implementor.

```findings
- file: .engineering/planning/story/runner-detail-option.md
  line: 22
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story's outcome (`--detail` on every `surf-agent` command) is in no sentence of the epic's Outcome, and the assignment is recorded only in the story itself"
- file: .engineering/planning/story/html-document-port.md
  line: 39
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance of story:runner-detail-option requires an attribute-level line after `fill`, but `html-document-port` gives the tree no mutation path and `agent-form-port` keeps filled values in rows, so no story delivers the tree change it asserts"
- file: .engineering/planning/story/runner-formats.md
  line: 41
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the exchange block holds no response URL although `Answer::Response` carries one after `agent-links-spec`, so a replayed redirected request (post, 303, get) commits at another address and the \"each HTTP request and response\" promise is narrowed to responses without a redirect"
```
