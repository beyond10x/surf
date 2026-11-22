---
format: aep.planning-md/3
id: story:view-controls-and-changes
kind: story
status: active
title: Form controls and buttons in the now panel, and what the last action changed
summary: Controls and buttons join the now panel, password values never reach the view, and a changed panel shows the answer's changed block at the playhead.
relations:
- decomposes: epic:live-view
- depends_on: story:view-now-panel
- depends_on: story:agent-fill-pick
- depends_on: story:agent-change-and-log
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 4}
---
## Context

`epic:live-view`, Outcome: for the selected session, the actionable elements "grouped by kind"
(links, buttons, fields) "and what the last action changed". `story:view-now-panel` shows links.
Controls and buttons reach the `now` block with `story:agent-fill-pick`; the `changed` block with
`story:agent-change-and-log`. Since `docs/plan/agent-plan-revision-1.md` D2, the `changed` block of
an action inside one document holds control-state lines, and a new document gives
`changed page <old> -> <new>` and the text lines; no tree of tags, and no detailed lines (D3).

Needs from `epic:agent-cli-target`: `story:agent-fill-pick` (control and button lines in the `now`
block) and `story:agent-change-and-log` (the `changed` block), each with its parse in
`surf_answer::parse` and its round-trip case (decision D20 of `docs/plan/agent-plan-revision-2.md`),
and, under D27, the mapping of the events of `Fill`, `Pick` and `Press` in `crates/session`. Both
blocks reach `surfd` as the answer's lines, kept as the answer entry (D20).

What the log holds of a value, in the words of D22: a password's value is its length, as the
answer prints it; every other control value is kept as the answer prints it, which is what the
`changed` panel shows. So the panel shows `f1 value "" -> "ada"` for a text control (case 4) and
only a length for a password (cases 2 and 3).

## Domain relations

- `surf.document.Document → its actionable element entity`, as in `story:view-now-panel`
  (`story:html-document-spec`).
- A select element stores no value; its options' selectedness is the stored fact
  (`docs/plan/agent-plan-revision-1.md` D11; option rows added by `story:agent-forms-spec`). The
  panel shows what the answer derives from them.

## Change

- `docs/view-stream.md`: in the `answer` entry, the `now` member gains control and button elements
  (kind; the value as the answer prints it, and for a password its length as the answer prints
  it, never the value; checked, the chosen option, disabled, required; a button's form, method
  and action), and the entry gains the member `changed` after `now`, as that document reserves it
  (`page` from and to; `+`, `-` and `~` element lines with what changed, before and
  after, a password's before and after as lengths; `+ text` lines); one example each. The rule of
  D22 restated there in its words. The length comes from the `FillRequested` entry, not from the
  answer.
- `crates/view/src/model.rs`, `json.rs`, `crates/view/tests/golden/`, `crates/view/tests/json.rs`;
  `crates/surfd/src/view.rs`: the JSON of the `now` controls and the `changed` block that
  `surf_answer::parse` returns; no parsing of answer text here (D20).
- `crates/view/web/now.js`: buttons and fields groups. `crates/view/web/changes.js`: a centre
  panel "Changed by <action>" at the playhead. `index.html`, `view.css`, `view.js`, `assets.rs`.
- `crates/view-fixture/pages/forms/`: a login form (text, password, checkbox, select, submit) whose
  answer page holds the text "Wrong password".
- Cases:
  1. (`crates/view/tests/json.rs`) the `answer` of the form page and the `answer` after a `fill`
     equal their golden files;
  2. (`crates/surfd/tests/view.rs`) after `fill f2 -` with the password `hunter2-view` on standard
     input and `press b1`, no byte of `/api/sessions/<id>` or of the stream since the session
     opened contains `hunter2-view`;
  3. (`view-tests/tests/controls.spec.mjs`) the fields group lists the four controls with handles
     and states; after `fill f2 -` with `hunter2-view` the password shows as its length, 12, as the
     answer prints it, and the page holds no `hunter2-view` anywhere; the buttons group shows the
     submit button with its method and action;
  4. after `fill f1 -` with `ada` (a text control: its value is kept, D22), the changed panel shows
     `f1 value "" -> "ada"`;
  5. after `press b1`, the changed panel shows the page change and `+ text "Wrong password"`;
  6. with the playhead on the `fill`, the changed panel shows that action's lines, not the
     latest;
  7. a label, a value and a text line holding `"`, `<script>alert(1)</script>` and
     `<img src=x onerror=alert(1)>` are shown as text: no `script` or `img` element is added and no
     dialog opens.

Surfaces written: `docs/view-stream.md`, `crates/view/src/`, `crates/view/tests/`,
`crates/view/web/`, `crates/surfd/src/view.rs`, `crates/surfd/tests/view.rs`,
`crates/view-fixture/pages/forms/`, `view-tests/tests/controls.spec.mjs`.

## Acceptance

`slot task view-test` passes with cases 3 to 7 of `view-tests/tests/controls.spec.mjs`, `cargo test
-p surf-view --test main json::` with case 1 and the `surfd` package's `--test main view::` with case 2, each
named after its case, and the earlier view tests still pass.

## Out of scope

The detailed lines (`epic:detailed-mode`); acting from the view; computed visibility.
