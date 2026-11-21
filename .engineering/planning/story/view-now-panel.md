---
format: aep.planning-md/3
id: story:view-now-panel
kind: story
status: active
title: 'What can be done now: the session''s links and title, at the playhead'
summary: The answer's title and now block reach the view; a centre panel lists the elements grouped by kind with handle, label, target and visibility, as after the selected action.
relations:
- decomposes: epic:live-view
- depends_on: story:view-detail-panel
- depends_on: story:agent-follow-links
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:15Z", actor: "human:timo", revision: 4}
---
## Context

`epic:live-view`, Outcome: for the selected session, "what can be done on it now (the actionable
elements of the agent's `now` block, grouped by kind, each with handle, label, state or target and
visibility)". Before the parser the `now` block is empty; links reach it with
`story:agent-follow-links`, which also puts the title on the page line. This story shows them. The
controls and the `changed` block follow in `story:view-controls-and-changes`.

Needs from `epic:agent-cli-target`: `story:agent-follow-links`: the `now` block with link lines
(handle, kind, visibility, label, target or unresolved target) and the title on the page line,
each with its parse in `surf_answer::parse` and its round-trip case (decision D20 of
`docs/plan/agent-plan-revision-2.md`: every story that adds a block adds its parse); and, under
D27, the mapping of the `Follow` events it adds, in `crates/session`. The answer reaches `surfd` as
its lines, kept as the answer entry (D20); labels, targets and the title in them are quoted on the
link and come out of `surf_answer::parse` as exact values (D26).

## Domain relations

- `surf.document.Document → its actionable element entity`, one to many, zero allowed, the
  document owns its elements: introduced into `spec/domains/document.yaml` by
  `story:html-document-spec` (design section 6, accepted in
  `approval-record:agent-design-accepted-2026-11-21`). The view reads them only as the session's
  answer carries them, never from the document component.

## Change

- `docs/view-stream.md`: the `answer` entry gains the member `now` after `path`, as that document
  reserves it (no member removed or renamed): the elements, each with handle, kind, visibility
  `vis`, `hid` or `?`, label, and target or unresolved target, and the note that visibility is the
  `hidden` attribute only; the page line's `title`, a member already, holds the title instead of
  `null`; one example each, in the document's JSON rules.
- `crates/view/src/model.rs`, `json.rs`: the element; `crates/view/tests/golden/` and
  `crates/view/tests/json.rs`: case 1 below.
- `crates/surfd/src/view.rs`: the JSON of the title and of the `now` block that
  `surf_answer::parse` returns for an answer entry; no parsing of answer text here (D20).
- `crates/view/web/now.js`: a centre panel "Now", grouped by kind (links; buttons and fields once
  they exist), each element with its handle, label, target and visibility, a hidden one marked;
  the note line when the answer has it; the set the `answer` after the playhead's action holds.
  `index.html`, `view.css`, `view.js`, `sessions.js`, `page.js` (the title), `assets.rs`.
- `crates/view-fixture/pages/now/`: a page with three links (one to a second page of the fixture, one inside a
  `hidden` element, one whose `href` does not resolve) and a page whose title holds `<script>` and
  `"`.
- Cases:
  1. (`crates/view/tests/json.rs`) the `answer` of the three-link page equals its golden file;
  2. (`view-tests/tests/now.spec.mjs`) the panel lists the three links under links with handles
     `l1` to `l3` in document order, their labels and targets;
  3. the hidden link is marked hidden; the visibility note is shown;
  4. after `follow l1`, a timeline segment `follow l1` and the next page's links; with the playhead
     on the earlier action, the links of that action's answer entry (D19);
  5. a title holding `"`, `<script>alert(1)</script>` and `<img src=x onerror=alert(1)>` is shown as
     text in the session list and on the page line;
  6. a label and a target holding the same three values are shown as text in the panel: no
     `script` or `img` element is added and no dialog opens.

Surfaces written: `docs/view-stream.md`, `crates/view/src/model.rs`, `crates/view/src/json.rs`,
`crates/view/src/assets.rs`, `crates/view/tests/`, `crates/view/web/`, `crates/surfd/src/view.rs`,
`crates/view-fixture/pages/now/`, `view-tests/tests/now.spec.mjs`.

## Acceptance

`slot task view-test` passes with cases 2 to 6 of `view-tests/tests/now.spec.mjs`, `cargo test -p
surf-view --test json` passes with case 1, each named after its case, and the earlier view tests
still pass.

## Out of scope

Form controls, buttons and the `changed` block (`story:view-controls-and-changes`); computed
visibility; a tree of tags.
