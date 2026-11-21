---
format: aep.planning-md/3
id: approval-record:runner-decisions-accepted-2026-11-21
kind: approval-record
status: draft
title: The scenario runner's decisions, accepted under the operator's word on the plan
relations:
- decides: story:runner-decisions
revision: 1
---
## What was decided

`docs/design/scenario-runner.md` (unit commit `23f2069`) answers Q1 to Q3 and D4 to D13 of
`story:runner-decisions`.

## The operator's word

On 2026-11-21 the operator accepted the plan and its designs for implementation: "I'm just
accepting these new designs, accept all of them, put it into stories or epics, whatever, and
implement it" (`approval-record:agent-plan-accepted-2026-11-21`). The note did not exist when he
said it. The coordinating agent reads that word as covering the decision notes this plan's
decision stories write (`docs/plan/agent-waves.md`, "Rules of a wave in this plan"); this
reading is the agent's, not a second statement of his.

## What he was shown

The same day the coordinating agent reported the note's main decisions to him in the session,
as decisions he can overrule: recordings and scenarios are surf's own line-based text formats;
a step finds its element by kind, accessible name and tree path; a request the recording does
not hold fails the navigation; a typed password is stored as a numbered variable and read from
an environment variable with the prefix `SURF_VAR_`; files are created with mode 0600 and
never overwritten. He did not object and gave further instructions afterwards.

## Standing

Reversible. If he overrules a decision, the note is amended and the stories built on it are
revised before their waves.
