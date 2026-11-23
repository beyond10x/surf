---
format: aep.planning-md/3
id: story:password-printed-back-by-a-server
kind: story
status: archived
title: A typed password that a server prints back
summary: D22 covers the value the agent types; a page that prints the posted body puts it into a + text line of the changed block, which the view shows since story:view-controls-and-changes; draft, candidates not chosen
relations:
- decomposes: epic:agent-cli-target
- informed_by: story:view-controls-and-changes
- informed_by: story:agent-change-and-log
revision: 2
transitions:
- {from: "draft", to: "archived", at: "2026-11-23T10:08:28Z", actor: "human:operator", revision: 2}
---
## Why

`docs/plan/agent-plan-revision-2.md` D22: a password's value is its length in the log, as the
answer prints it. That rule covers the value the agent types into a password control. It does not
cover what a server sends back.

The adversary's pass on `story:view-controls-and-changes` (commit `851fda9`, 2026-11-23)
measured: a server whose answer page prints the posted body gives a `changed` block with the line
`+ text "Received code=abcpw=hunter2-viewpick=One"`. The agent's answer prints that line
(`crates/session/src/lib.rs`, `new_texts`), the log keeps it as the answer entry, and since
`story:view-controls-and-changes` the view shows it in "Changed by press b1" and carries it on the
stream and in `/api/sessions/<id>`. The adversary found no page of the fixtures that does this;
its page test is kept as text, not in the tree (decision D323).

A second case, read and not measured: `control_change` in `crates/session/src/lib.rs` decides by
the control's kind after the action. A control that was a password before and is a text control
after would print its earlier value. No page can do this without a script; it becomes reachable
with `epic:scripts-in-a-page`.

`docs/view-stream.md` states the limit since D324.

## Change

Draft. Not decided. Candidates:

1. Keep: text a server sends is page text. The documents say so (done, D324).
2. The session holds back a typed password in new text: it knows the values it typed into
   password controls of the document it posted from; a `+ text` line of the next `changed` block
   that contains one prints the value as its length. Cost: a short password matches by chance
   (`a` in every text); a bound on the length that is masked is part of the decision. Whether
   another block of the answer prints the same text is to be read before the choice.
3. For the second case: `control_change` prints a value only when the control was not a password
   before the action and is not one after it.

## Acceptance

Draft: for the candidate chosen, a page test and a session test with a server that prints the
posted body; the session test of a control whose type changes waits for a script engine.

## Out of scope

The value the agent types (`story:agent-fill-pick`, D22). What a script of a page reads
(`epic:scripts-in-a-page`).
