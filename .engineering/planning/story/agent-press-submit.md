---
format: aep.planning-md/3
id: story:agent-press-submit
kind: story
status: active
title: press submits a form and the session lands on the answer
summary: press <handle> on a submit button submits through the document and navigates with GET or POST; the test server receives the encoded entries.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-fill-pick
- depends_on: story:agent-session-events
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: README.md
- confidence: cited
  path: cli/surf-agent.yaml
- confidence: inferred
  path: crates/agent-tasks/status.txt
- confidence: cited
  path: crates/agent/src/cli.rs
- confidence: inferred
  path: crates/agent/src/wire.rs
- confidence: inferred
  path: crates/agent/tests/drift.rs
- confidence: cited
  path: crates/answer/src/lib.rs
- confidence: inferred
  path: crates/answer/tests
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: cited
  path: crates/session/tests
- confidence: cited
  path: crates/surfd/src/daemon.rs
- confidence: cited
  path: docs/agent-answer.md
- confidence: cited
  path: docs/design/agent-interface.md
- confidence: inferred
  path: generated/rust
- confidence: inferred
  path: spec/domains/agent.yaml
- confidence: inferred
  path: website/docs/drive-from-an-agent.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 6}
---
## Context

Step 4 of design section 9: `press` activates a button; with no script engine the only activation
is form submission (design section 4). `agent-forms-spec` specifies `Press` and the submission;
`agent-form-port` computes its body; `agent-fetch-post` sends `POST` and turns a 303 into `GET`.
Design section 5.1 shows a button line naming what it submits: `submits F1 post /session`.

## Change

- `crates/session`: `press <handle>` on a submit button issues `Press`; the answer shows the page the
  navigation committed at. Forms get handles `F<n>` in document order, printed on the line of each
  submit button with the method and the resolved action, quoted. A button that submits nothing
  (`type=button`, `reset`, no form owner) is refused `no-default-action` until a script engine
  exists; a submission that ends without a body is refused with its reason.
- `crates/answer`, `docs/agent-answer.md`: the `submits` part of a button line, with its `parse`
  beside its rendering (decision D20 of `docs/plan/agent-plan-revision-2.md`).
- `crates/session`: the event entries of `agent-session-events` for the events of `Press` and the
  submission, in its mapping function and its mapping list (decision D27); a request body is
  written as `<n> bytes`, never the body (decision D22).
- The `did` and `path` lines of `press`, as `docs/agent-answer.md` and `crates/answer` write
  them: `did press "<handle>"` and `<n> press "<handle>"`. The reasons `no-default-action`,
  `disabled` (if `agent-fill-pick` has not added it) and the reasons a submission ends without a
  body go into the reason table with exit status 1, into `RefusalReason` and into a test.
- `cli/surf-agent.yaml`, `crates/agent`: `press`, together, so the drift test holds.
- `crates/session/tests/`, the counted cases, against a server the test starts:
  1. a login form filled and pressed: the server reads `POST /session HTTP/1.1` with
     `Content-Type: application/x-www-form-urlencoded` and the body `user=ada&password=s3cret`,
     answers 303 to `/home`, and the answer shows the page at `/home`;
  2. a `GET` form lands on its action with the entries in the query;
  3. the button line of a form reads `submits F1 post "/session"`;
  4. `press` on a `type=button`: `no-default-action`; 5. on a `reset`: `no-default-action`;
  6. on a disabled submit button: `disabled`; 7. a `multipart/form-data` form: refused with its
     reason; 8. an action `javascript:x`: refused with its reason;
  9. forging: an action holding `"`, a line feed and a bidirectional control is printed quoted and
     escaped on the button line, with no extra line;
  10. `press_events_map_with_their_fields`, with `surfd` run in the test process (decision D29):
      after case 1 the session log holds the `Press` and submission event entries with their
      fields, none `unmapped`, the request body as `<n> bytes`, and `s3cret` in no entry;
  11. round trip: `surf_answer::parse` of the answer of case 3 and of case 9 gives the typed
      answer, and rendering that gives the same lines byte for byte.

Surfaces written: `crates/session/`, `crates/answer/`, `docs/agent-answer.md`,
`cli/surf-agent.yaml`, `crates/agent/`.

## For README.md (handed over in the unit report, decision D7)

- "Driving surf from an agent": "`press b1` submits the form of a submit button; the answer shows
  the page the server answered with. A button that only a script would act on is refused."

## Acceptance

`cargo test -p surf-session` passes with the 11 numbered cases, each a test named after its case,
case 1 reading the request line `POST /session HTTP/1.1` and the body `user=ada&password=s3cret` and
answering with the page at `/home`.

## Out of scope

Scripts and elements a script made clickable; `formaction` and the other submitter attributes;
`changed` (`agent-change-and-log`).
