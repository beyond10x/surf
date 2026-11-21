---
format: aep.planning-md/3
id: approval-record:agent-plan-accepted-2026-11-21
kind: approval-record
status: draft
title: Operator accepted the agent interface plan, surfd and the live view, for implementation
revision: 2
---
## What was put to the operator

Open since 2026-11-21, asked twice by the coordinating agent:

> Decision: may I implement once the critics approve?
> - A: yes, all 3 epics. Waves run without a stop, Opus implementors, one review per larger
>   piece, as in the first plan. You get a "to try" list when `surf-agent open/go/look/close`
>   works. Recommended.
> - B: yes, but only the first piece [...]. Then I stop and show you.
> - C: no; you read the stories first.

After that the operator proposed a daemon and a live view ("What if there is a surfd [...] and
there would also be some web UI showing all the active sessions on that daemon [...]"), and the
coordinating agent amended the design (`docs/design/agent-interface.md`, sections 3 and 12) and
told him it would adjust the stories and draft `epic:live-view` unless he said no.

## The answer

The operator, 2026-11-21:

> So yeah, I'm just accepting these new designs, accept all of them, put it into stories or
> epics, whatever, and implement it. And as soon as there is a first UI, a prototype of it or
> something, just open that UI in my Brave browser

## What it covers

- The design as amended: the agent command line, `surfd`, the live view, recording and the
  scenario runner.
- The epics written from it: `epic:agent-cli-target`, `epic:html-parser`,
  `epic:e2e-scenario-runner`, `epic:live-view`, and their stories. They are accepted for
  implementation, which is option A of the question above: waves without a stop between them.
- An instruction for the first view: when a first version of the live view runs, the
  coordinating agent opens it in the operator's Brave browser.

## What it does not cover, as the coordinating agent reads it

- `epic:detailed-mode` and the four backlog drafts tagged `deferred`: they are not designs he
  was shown as work to do now, and they wait for things that do not exist.
- Anything that leaves the machine: pushing, a pull request, filing issues. Those questions
  were asked separately and are not answered by this sentence.
- Skipping the critic panel's second round or the gate. He accepted the plan; the stories are
  moved to `active` when round 2 has read them, and each still merges only on its gate.
