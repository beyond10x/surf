---
format: aep.planning-md/3
id: story:view-detail-panel
kind: story
status: active
title: 'The detail panel: the selected action or event, and the session''s requests'
summary: The right column shows the selected action's or event's fields and the session's requests with status, media type, size or failure reason.
relations:
- decomposes: epic:live-view
- depends_on: story:view-first-page
scope:
- confidence: cited
  path: crates/view/src/assets.rs
- confidence: cited
  path: crates/view/tests/prototype.rs
- confidence: cited
  path: crates/view/web/detail.js
- confidence: cited
  path: crates/view/web/index.html
- confidence: cited
  path: crates/view/web/state.js
- confidence: cited
  path: crates/view/web/timeline.js
- confidence: cited
  path: crates/view/web/view.css
- confidence: cited
  path: crates/view/web/view.js
- confidence: cited
  path: view-tests/tests/adversary-page.spec.mjs
- confidence: cited
  path: view-tests/tests/detail.spec.mjs
revision: 15
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 4}
---
## Context

`epic:live-view`, Outcome: "a detail panel for the selected action or event, and the session's
requests". `story:view-first-page` (wave 7, `docs/plan/agent-wave-7.md`) left the right column
empty (`<aside class="detail" aria-label="Detail"></aside>` in `crates/view/web/index.html`,
`display: none` in `view.css` until 1,200 px) and the timeline's marks without a handler: a mark
is an `li.mark` with a `title`, and a click on it reaches the segment's handler, which moves the
playhead. The data is already in the kept entries (`docs/view-stream.md`, "The entries"): an
event's fields by their `spec/` names, each the exact value the view routes read with
`surf_answer::unquote` (decision D26 of `docs/plan/agent-plan-revision-2.md`), `null` where the
field is an absent optional value or `unquote` refused it (`crates/surfd/src/view.rs`); a field
of bytes is the text `<n> bytes` the session process wrote, never the bytes (D22); an action's
events are the event entries between its action entry and its answer entry (D19).

The `url` field of `surf.net.ResponseReceived` (the address the fetch ended at, after the last
redirect) is on `main`: `story:agent-redirect-spec` and `story:agent-session-events` are
implemented, and `crates/session/src/entries.rs` maps it. In the same wave
`story:agent-request-body-spec` adds `method`, `body` and `body_type` to `RequestStarted` and
`NavigationStarted`; the panel lists an event's fields as the entry holds them, so those show
without a change here, and no case of this story asserts the full field set of `RequestStarted`.

Since wave 7, `crates/view/web/dom.js` is the only module that sets an attribute, from a list of
nine names (`class`, `title`, `type`, `hidden`, `aria-current`, `aria-pressed`, `data-at`,
`data-domain`, `data-number`), and throws for any other; the page case "adversary: dom.js sets
only the attributes of its list, and no other page file sets one"
(`view-tests/tests/adversary-page.spec.mjs`) reads six named page files for a set of an attribute
or of a property. This story needs no attribute outside the list: a mark becomes a button with
`type`, `data-number` and `aria-pressed`, and every other node of the panel has a class or a
static `id` in `index.html`. `dom.js` is not changed; a unit that finds it needs another name
stops and reports it.

The selection lives in the address's fragment, never the query (the route table answers a query
404): `#session=<id>&at=<sequence>` today (`view.js` `readSelection`, `writeSelection`); `at` is
written only while Latest is off.

## Domain relations

- A request and its navigation share one identity: `spec/domains/navigation.yaml`, `Navigation`
  identity `navigation_id: surf.net.RequestId`. The requests list groups the `surf.net` events of
  one session by `request_id`: `RequestStarted` (`request_id`, `url`), `ResponseReceived`
  (`request_id`, `url`, `status`, `media_type`, `body` as `<n> bytes`), `FetchFailed`
  (`request_id`, `url`, `reason`), as `spec/domains/net.yaml` declares their fields and
  `crates/session/src/entries.rs` writes them.

## Change

- `crates/view/web/detail.js` (new): the right column, built only through `dom.js` (`element`,
  `address`, `text`, `clear`); it reads no `textContent`, sets no property, and holds no
  `http://` or `https://` (`crates/view/tests/web.rs` case 10 reads every file of `web/`).
  - The selected action, which is the action at the playhead (the newest with Latest on): its
    sequence number (`.sequence`), command (`.command`), argument as an address (`.argument`,
    absent when `null`), the time it was kept (`.kept`, `kept_at` written as
    `new Date(kept_at).toLocaleTimeString([], { hour12: false })`, the form of the live
    marker), and the number of event entries between its action entry and its answer entry
    (`.events`, `actions(id)[at].events.length` of `state.js`). A `null` sequence or command is
    written `?`, as on the timeline. With no action held: the text `no action`.
  - The selected event, when one is selected, in place of the action: its qualified name
    (`.name`; `unreadable event` for a `null` name) and every field in the entry's order as a
    `dt`/`dd` pair of `dl.fields`, the value as text; a `url` value as an address; a `null`
    value as the words `no value`; `body` as the `<n> bytes` the log holds; an `unmapped` event
    reads `unmapped: no fields`.
  - The session's requests (`#requests`, an `ol`), newest last: one `li.request` per
    `request_id` among the held event entries of the selected session (all of them, not only
    up to the playhead), in the order of the first entry held for it. Each row: the address
    (`.address`; the `url` of `RequestStarted`, or of the first entry held for that
    `request_id` when `RequestStarted` was dropped), then `.status` and `.media-type` and
    `.size` (`<n> bytes`) from `ResponseReceived`, or `.failed` with `.reason` from
    `FetchFailed`, or `.pending` while neither is held. A row is filled in as its events arrive.
    Rows come from what the page holds; `surfd` bounds that at 10,000 entries per session
    (`crates/surfd/src/log.rs` `ENTRIES_PER_SESSION`); the page's own bound is
    `story:view-page-memory`'s.
- `crates/view/web/state.js`: the selected event, by its entry's number: set with the playhead
  moved to its action (Latest off); cleared by `select`, `followLatest`, `setPlayhead` from a
  segment, `stepPlayhead`, and when the entry is no longer held after a load or a `reset`.
- `crates/view/web/timeline.js`: each mark holds a `button` (`type` `button`, `data-number` the
  event entry's number, `aria-pressed` for the selected event) with the short name as its text;
  the `li.mark` keeps its class and its `title`, so the cases of `first-page.spec.mjs` and
  `adversary-page.spec.mjs` that count `.mark` and read its `title` and text hold unchanged. A
  click on a mark selects that event and stops the event's propagation, so the segment's
  handler does not clear it. `renderTimeline` takes the selected number and the handler.
- `crates/view/web/view.js`: renders the panel in `renderSelected`; `readSelection` reads
  `event=<number>` (`^[0-9]{1,15}$`, the number across all sessions, D23); `writeSelection`
  writes `#session=<id>&at=<sequence>&event=<number>` while an event is selected, and the
  fragment as today otherwise. At load, an `event` whose entry is held in the named session
  selects it and puts the playhead on its action; an `event` not held is ignored and `at`
  applies as today. The file's opening comment states the new fragment.
- `crates/view/web/index.html`: the column's static structure inside the `aside`: a heading,
  the selection's container and `#requests` under a heading "Requests".
- `crates/view/web/view.css`: the panel, the mark's button (no border or background of a
  button, the mark's look kept, a focus ring), the rows. The column is shown from 1,200 px as
  the grid has it now; below that it is placed after the centre (decided for the revision, to
  confirm): the page tests run at 1,400 px only, so no case holds the narrower layouts.
- `crates/view/src/assets.rs`: `detail.js` in `page_files()`.
- `crates/view/tests/prototype.rs`: case 03 compares `page_files()` and the files of `web/`
  with the constant `NINE` (nine names); it becomes ten names with `detail.js`, or it fails.
- `view-tests/tests/adversary-page.spec.mjs`: `detail.js` added to the list of page files the
  case "adversary: dom.js sets only the attributes of its list, and no other page file sets
  one" reads, so that the rule holds the new file.
- `view-tests/tests/detail.spec.mjs` (new), with the fixture of `view-tests/tests/fixture.mjs`
  and its existing commands only (`open` with and without an address, `agent <id> go <url>`,
  `api`; `surf.pages`, `surf.unreachable`); no command is missing and the fixture crate is not
  changed. The served pages are `crates/view-fixture/pages/first/a.html` (209 bytes) and
  `b.html` (210 bytes), measured with `wc -c`; no new fixture page. Each case is a test named
  after it:
  1. "selected action": `open` without an address, `go a.html`, `go b.html`; a click on the
     second segment shows sequence `2`, command `go`, the argument equal to
     `surf.pages + "/first/a.html"`, the kept time equal to the browser's formatting of that
     action entry's `kept_at` from `/api/sessions/<id>`, and an event count equal to the event
     entries that detail holds between that action entry and its answer entry (D19);
  2. "selected event": a click on the mark `ResponseReceived` of the second segment shows the
     name `surf.net.ResponseReceived` and the fields `request_id` (equal to that entry's in the
     API), `url`, `status` `200`, `media_type` `text/html`, and `body` `<n> bytes` with `<n>` the
     byte length of the body Node fetches from `surf.pages + "/first/a.html"`; that mark's
     button reads `aria-pressed="true"`;
  3. "requests list": after `go a.html`, `go b.html` and `go surf.unreachable` on a session
     opened without an address, `#requests > li.request` has 3 rows in that order: two with
     status `200`, media type `text/html` and sizes `209 bytes` and `210 bytes` (each equal to
     the fetched length), one with `.reason` `Unreachable` and no `.status`;
  4. "a go adds a row live": with the page open on that session, a `go b.html` adds a fourth
     row within 2 s, and the page asks `/` no more times than before;
  5. "event in the address": after case 2's click the fragment is
     `#session=<id>&at=2&event=<n>`, `<n>` the `number` of that entry in the API; after a
     reload the panel shows the same name and `request_id`, the playhead is on the second
     segment, the mark's button is pressed, and the address holds no token; a click on a
     segment then removes `&event=` from the fragment;
  6. "hostile address is text in the panel": a `go` to
     `surf.pages + '/first/a.html?"<script>alert(1)</script><img src=x onerror=alert(1)>'`
     (the page server ignores the query and answers 200); the API's `argument` and the
     `ResponseReceived` `url` are first asserted to hold `<script>` (so the case cannot pass on
     an escaped value), then the action's `.argument`, the event's `url` field and the request
     row's `.address` each have `textContent` equal to the API's value; the document still has
     1 `script` and 0 `img` elements, no dialog opened, no request to `/x`, and no
     `securitypolicyviolation` was recorded.

Surfaces written: `crates/view/web/detail.js`, `crates/view/web/index.html`,
`crates/view/web/view.css`, `crates/view/web/view.js`, `crates/view/web/state.js`,
`crates/view/web/timeline.js`, `crates/view/src/assets.rs`, `crates/view/tests/prototype.rs`,
`view-tests/tests/detail.spec.mjs`, `view-tests/tests/adversary-page.spec.mjs`. Not `dom.js`,
not `crates/view-fixture`, not `AGENTS.md` (D7 of `docs/plan/agent-plan-revision-1.md`).

## For AGENTS.md (handed over in the unit report, decision D7)

- `crates/view`: "`web/` holds the page's nine files" becomes ten, with `detail.js`.
- The page tests' count: 30 cases (18 in `first-page.spec.mjs`, 6 in `adversary-page.spec.mjs`,
  6 in `detail.spec.mjs`).

## Acceptance

- `slot task view-test` prints `30 passed`: the 6 cases of `view-tests/tests/detail.spec.mjs`,
  each a test named after its case, and the 24 cases of `first-page.spec.mjs` and
  `adversary-page.spec.mjs` unchanged but for `detail.js` in the adversary case's file list.
  Cases 1, 2, 3, 5 and 6 fail on `main` (no panel); the unit reports each red before its change.
- `cargo test -p surf-view` passes, with case 03 of `crates/view/tests/prototype.rs` listing ten
  files and `crates/view/tests/web.rs` cases 9 and 10 reading `detail.js`.
- The unit reports one planted defect per case, each red: a field value written as markup, a
  row per event in place of per `request_id`, a mark click that reaches the segment's handler,
  `event=` not read at load, an event count that includes the answer entry, a row filled only
  at a reload.

## Out of scope

A link to a session's recording (design section 12; the recording is `epic:e2e-scenario-runner`'s
and the epic does not ask for it); a response body; acting on anything; the response address of
a redirected request in the requests row (the event's `url` field shows it; the fixture's page
server serves no redirect); a bound on what the page holds and building the timeline in place
(`story:view-page-memory`); the first load's retry (`story:view-first-load-retry`).
