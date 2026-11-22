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
- confidence: cited
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
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 6}
---
## Context

Step 4 of design section 9: `press` activates a button; with no script engine the only activation
is form submission (design section 4). `agent-forms-spec` specifies `Press` and the submission;
`agent-form-port` computes its body, ends a submission with one of the seven reasons of the
specification and gives forms their handles `F<n>`; `agent-fetch-post` sends `POST` and turns a
303 into `GET`. Design section 5.1 shows a button line naming what it submits, with the action
unquoted (`submits F1 post /session`, `docs/design/agent-interface.md:231`); this story quotes
it, like every page-derived field, and corrects that line.

## Change

- `crates/session`: `press <handle>` on a submit button issues `Press`; the answer shows the page
  the navigation committed at. The line of each submit button names what it submits:
  `submits F1 post "/session"`, the form's handle as `surf_page::form_handles` gives it (`F<n>`,
  a function `story:agent-form-port` adds beside `surf_page::handles`, which stays unchanged;
  this story only prints it, and page text prints no `@F<n>`), the method, and the resolved action
  quoted like every page-derived field. `docs/design/agent-interface.md:231` writes the action
  unquoted; this story corrects that line to `submits F1 post "/session"`.
- `no-default-action` is the session's refusal, before any command is issued, for a button of
  `type=button` and of `type=reset` (no reset is built: a reset button is refused by name, as a
  plain one is), until a script engine exists. A submit button goes to `Press`, and a submission
  that ends without a navigation ends with the specification's reason
  (`SubmissionAbandoned.reason`, `spec/domains/document.yaml:105-121`), refused with its word,
  exit status 1:
  - `NoFormOwner`: `no-form-owner`, case 12;
  - `NotSubmitButton`: cannot be reached through the session, which refuses `type=button` and
    `type=reset` as `no-default-action` before `Press`; no word and no case;
  - `InvalidControls`: `invalid-controls`, case 13;
  - `InvalidAction`: `invalid-action`, case 14;
  - `UnsupportedScheme`: `unsupported-scheme`, case 8;
  - `UnsupportedEnctype`: `unsupported-enctype`, case 7;
  - `DialogMethod`: `dialog-method`, case 15.
- `crates/answer`, `docs/agent-answer.md`: the `submits` part of a button line, with its `parse`
  beside its rendering (decision D20 of `docs/plan/agent-plan-revision-2.md`).
- The event entries exist already: `PressRequested`, `SubmissionStarted`, `FormSubmitted` (the
  body as `<n> bytes`, decision D22) and `SubmissionAbandoned` are mapped
  (`crates/session/src/entries.rs:250`, `:268`, `:271`, `:281`). This story adds no mapping;
  case 10 tests them.
- The `did` and `path` lines of `press`. `did press "<handle>"`: the handle quoted, as the caller
  typed it. The `path` entry takes the form `story:agent-follow-links` gives an action on an
  element (`spec/domains/agent.yaml:302-304`): `<n> press b1`, the handle bare (the design shows
  no `press` entry; `docs/design/agent-interface.md:223` shows `3 fill f1`, and this follows it).
  The reasons `no-default-action`, `disabled` (if `agent-fill-pick` has not added it) and the six
  submission words above go into the reason table of `docs/agent-answer.md` with exit status 1,
  into `RefusalReason` and its `RefusalReason::ALL` (`crates/answer/src/lib.rs:150`), into the
  exhaustive `match` of `crates/surfd/src/daemon.rs:815-821`, and into a test.
- `cli/surf-agent.yaml`, `crates/agent`: `press`, together, so the drift test holds.
- `crates/session/tests/`, the counted cases, against a server the test starts:
  1. a login form filled and pressed: the server reads `POST /session HTTP/1.1` with
     `Content-Type: application/x-www-form-urlencoded` and the body `user=ada&password=s3cret`,
     answers 303 to `/home`, and the answer shows the page at `/home`;
  2. the `GET` form of the `search` site (`crates/agent-tasks/sites/search/search.html:10-13`:
     `action="results"`, a field `q`, a button `Find`), `q` filled with `lighthouse` and pressed:
     the server reads `GET /results?q=lighthouse HTTP/1.1`, and the answer's `page` line ends in
     `/results?q=lighthouse"`;
  3. the button line of a form reads `submits F1 post "/session"`;
  4. `press` on a `type=button`: `no-default-action`; 5. on a `reset`: `no-default-action`;
  6. on a disabled submit button: `disabled`; 7. a `multipart/form-data` form:
     `unsupported-enctype`; 8. an action `javascript:x`: `unsupported-scheme`;
  9. forging: an action holding `"`, a line feed and a bidirectional control is printed quoted and
     escaped on the button line, with no extra line;
  10. `press_events_map_with_their_fields`, with `surfd` run in the test process (decision D29):
      after case 1 the session log holds the `PressRequested`, `SubmissionStarted` and
      `FormSubmitted` entries with their fields, none `unmapped`, the request body as
      `<n> bytes`, and `s3cret` in no entry;
  11. round trip: `surf_answer::parse` of the answer of case 3 and of case 9 gives the typed
      answer, and rendering that gives the same lines byte for byte;
  12. a submit button with no form owner: `no-form-owner`; 13. a `required` field left empty:
      `invalid-controls`; 14. an action that does not parse as a URL: `invalid-action`;
  15. a form with `method="dialog"`: `dialog-method`.

- `crates/agent-tasks/status.txt`, the round's measure: `search pass` (line 4). A task listed
  `pending` that passes makes the report print `raise: search` (`crates/agent-tasks/src/report.rs:9-14`),
  so the line is changed in this story.

Surfaces written: `crates/session/`, `crates/answer/`, `docs/agent-answer.md`,
`cli/surf-agent.yaml`, `crates/agent/`, `crates/surfd/src/daemon.rs`,
`docs/design/agent-interface.md` (line 231), `crates/agent-tasks/status.txt`.

## For README.md (handed over in the unit report, decision D7)

- "Driving surf from an agent": "`press b1` submits the form of a submit button; the answer shows
  the page the server answered with. A button that only a script would act on is refused."

## Acceptance

`cargo nextest run -p surf-session` passes with the 15 numbered cases, each a test named after its
case, case 1 reading the request line `POST /session HTTP/1.1` and the body
`user=ada&password=s3cret` and answering with the page at `/home`. The round's measure:
`crates/agent-tasks/status.txt` reads `search pass` (line 4 reads `search pending
story:agent-fill-pick story:agent-press-submit` today), `tasks::every_task`
(`crates/agent-tasks/tests/tasks.rs:74`) passes, and `slot task agent-tasks-report` prints
`task search pass` with `2 connections` (`crates/agent-tasks/tasks.txt`, task `search`) and no
`raise:` line (`crates/agent-tasks/src/report.rs:9-14`); today `search` fails. No test runs over
60 s, and the test step ends within 120 s.

## Out of scope

Scripts and elements a script made clickable; `formaction` and the other submitter attributes;
`changed` (`agent-change-and-log`).
