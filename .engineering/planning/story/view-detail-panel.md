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
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 4}
---
## Context

`epic:live-view`, Outcome: "a detail panel for the selected action or event, and the session's
requests". `story:view-first-page` leaves the right column empty and the timeline's marks
selectable. The data is already in the kept entries (`docs/view-stream.md`): an event's fields by
their `spec/` names, each the exact value the view routes read with `surf_answer::unquote`
(decision D26 of `docs/plan/agent-plan-revision-2.md`); a field of bytes is the text `<n> bytes`
the session process wrote, never the bytes (D22); an action's events are the event entries
between its action entry and its answer entry (D19).

The `url` field of `surf.net.ResponseReceived` is added by `story:agent-redirect-spec` (the
response address). Under D27, `story:agent-session-events` depends on `story:agent-redirect-spec`
and maps every event and field on `main` when it starts, so that field reaches the log mapped.
This story reaches both through `story:view-first-page` → `story:view-surfd-serves` →
`story:agent-session-events`.

## Domain relations

- A request and its navigation share one identity: `spec/domains/navigation.yaml`, `Navigation`
  identity `navigation_id: surf.net.RequestId`. The requests list groups the `surf.net` events of
  one session by `request_id`: `RequestStarted` (`url`), `ResponseReceived` (`url`, `status`,
  `media_type`, `body` as `<n> bytes`), `FetchFailed` (`reason`), as `spec/domains/net.yaml`
  declares their fields.

## Change

- `crates/view/web/detail.js`: the right column.
  - The selected action: its sequence number, command, argument, the time it was kept, and the
    number of event entries between its action entry and its answer entry.
  - The selected event: its qualified name and every field by name and value; `body` as the
    `<n> bytes` the log holds.
  - The session's requests, newest last: address, status and media type, size (`<n> bytes`), or
    the failure reason; one row per `request_id`, filled in as its events arrive.
- `crates/view/web/index.html`, `view.css`, `view.js`, `timeline.js`: the column, selecting an
  event mark, `&event=<the entry's number across all sessions>` in the address (D23). `crates/view/src/assets.rs`: the new file.
- `view-tests/tests/detail.spec.mjs`, against the fixture of `story:view-first-page`:
  1. selecting the second action shows its sequence number, command `go`, its address and its
     event count, equal to the event entries `/api/sessions/<id>` holds between that action entry
     and its answer entry (D19);
  2. selecting the `surf.net.ResponseReceived` mark shows `request_id`, `url`, `status`,
     `media_type`, and `body` as `<n> bytes` with `<n>` the served page's length in bytes;
  3. after two `go` and one `go` to a port nothing listens on, the requests list has 3 rows: two
     with status 200 and their sizes, one failed with `Unreachable`;
  4. a `go` while the page is open adds a row without a reload;
  5. a reload with `&event=<id>` shows the same event;
  6. an address holding `"`, `<script>alert(1)</script>` and `<img src=x onerror=alert(1)>` is
     shown as text in the action's detail, in the event's `url` field and in the request row:
     no `script` or `img` element is added and no dialog opens.

Surfaces written: `crates/view/web/` (`detail.js`, `index.html`, `view.css`, `view.js`,
`timeline.js`), `crates/view/src/assets.rs`, `view-tests/tests/detail.spec.mjs`,
`crates/view-fixture/pages/detail/`.

## Acceptance

`slot task view-test` passes with the 6 cases of `view-tests/tests/detail.spec.mjs`, each a test
named after its case, and the cases of `first-page.spec.mjs` still pass.

## Out of scope

A link to a session's recording (design section 12; the recording is `epic:e2e-scenario-runner`'s
and the epic does not ask for it); a response body; acting on anything.
