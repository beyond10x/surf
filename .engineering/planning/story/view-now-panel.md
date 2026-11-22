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
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: README.md
- confidence: cited
  path: crates/surfd/src/view.rs
- confidence: cited
  path: crates/view-fixture/pages/now
- confidence: cited
  path: crates/view/src/assets.rs
- confidence: cited
  path: crates/view/src/json.rs
- confidence: cited
  path: crates/view/src/model.rs
- confidence: cited
  path: crates/view/tests
- confidence: cited
  path: crates/view/tests/prototype.rs
- confidence: cited
  path: crates/view/web
- confidence: cited
  path: docs/view-stream.md
- confidence: cited
  path: view-tests/tests/now.spec.mjs
- confidence: inferred
  path: website/docs/status.md
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:15Z", actor: "human:timo", revision: 4}
---
## Context

`epic:live-view`, Outcome: for the selected session, "what can be done on it now (the actionable
elements of the agent's `now` block, grouped by kind, each with handle, label, state or target and
visibility)". `surf_answer::Answer` has no `now` block on `main` (`crates/answer/src/lib.rs:42-59`);
`story:agent-follow-links` adds it with the links, and puts the title on the page line. The
view's pages render a title already (`crates/view/web/sessions.js:37-38`, `page.js`); the data
lacks it, because the session sends none: `title: None` in its reply to `surfd`
(`crates/session/src/lib.rs:258`) and on its page line (`:999`). This story shows them. The
controls and the `changed` block follow in `story:view-controls-and-changes`.

Needs from `epic:agent-cli-target`: `story:agent-follow-links`: the `now` block with link lines
(handle, kind, visibility, label, target or unresolved target) and the title on the page line,
each with its parse in `surf_answer::parse` and its round-trip case (decision D20 of
`docs/plan/agent-plan-revision-2.md`: every story that adds a block adds its parse); and the
`follow` command with its `path` entry. No `Follow` event exists: `follow` publishes
`NavigationRequested` and `ActionRecorded`, whose entries are mapped already
(`crates/session/src/entries.rs:144-155`). The answer reaches `surfd` as
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
  reserves it (`docs/view-stream.md:202-205`; no member removed or renamed): the elements, each
  with handle, kind, visibility `vis`, `hid` or `?`, label, and target or unresolved target, and
  the visibility note as the `now` block carries it, `note visibility: no-css`. The word is `hid`
  when `surf-page`'s `is_hidden` holds for the element (the row's `hidden` flag: `aria-hidden`,
  the hidden elements of the HTML Standard's style sheet, a closed `dialog`, a popover, the
  content of a closed `details` but its first `summary`; `AGENTS.md:553-556`,
  `crates/page/src/lib.rs:417-421`), otherwise `?`, because no CSS is computed
  (`story:agent-follow-links`); the view shows the answer's word and states no rule of its own. The page line's `title`, a member already, holds the title instead of
  `null`; one example each, in the document's JSON rules.
- `crates/view/src/model.rs` (`EntryKind::Answer`, `:129`), `json.rs` (`:252`): the element.
  `crates/view/tests/json.rs` reads its expected values from the fenced blocks of
  `docs/view-stream.md` (`crates/view/tests/json.rs:20-29`), so case 1's expected answer is the
  example this story adds there; no golden file.
- `crates/surfd/src/view.rs`: the JSON of the title and of the `now` block that
  `surf_answer::parse` returns for an answer entry (`:198-216`); no parsing of answer text here
  (D20).
- `crates/view/web/now.js`: a centre panel "Now", grouped by kind (links; buttons and fields once
  they exist), each element with its handle, label, target and visibility, a hidden one marked;
  the note line when the answer has it; the set the `answer` after the playhead's action holds.
  `index.html`, `view.css`, `view.js`, `sessions.js`, `page.js` (the title), `state.js` (which
  reads only `answer.page` today, `crates/view/web/state.js:301-310`), `assets.rs`.
  `crates/view/tests/prototype.rs` holds the files of `web/` as `TEN` (`:19`, asserted at `:618`
  and `:632`): it gains `now.js`.
- `crates/view-fixture/pages/now/`: a page with three links (one to a second page of the fixture, one inside a
  `hidden` element, one whose `href` does not resolve) and a page whose title holds `<script>` and
  `"`.
- The count of page tests, 38 today, is restated where it is stated: `AGENTS.md:621`,
  `README.md:463`, `website/docs/status.md:22`.
- Cases:
  1. (`crates/view/tests/json.rs`) the `answer` of the three-link page equals the example of
     `docs/view-stream.md`;
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
`crates/view/src/assets.rs`, `crates/view/tests/` (`json.rs`, `prototype.rs`), `crates/view/web/`,
`crates/surfd/src/view.rs`, `crates/view-fixture/pages/now/`, `view-tests/tests/now.spec.mjs`,
`AGENTS.md`, `README.md`, `website/docs/status.md`.

## Acceptance

`slot task view-test` passes with cases 2 to 6 of `view-tests/tests/now.spec.mjs`, `cargo nextest
run -p surf-view --test main json::` passes with case 1, each named after its case, and the
earlier view tests still pass. No test runs over 60 s, and the test step ends within 120 s.

## Out of scope

Form controls, buttons and the `changed` block (`story:view-controls-and-changes`); computed
visibility; a tree of tags.
