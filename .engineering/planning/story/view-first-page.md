---
format: aep.planning-md/3
id: story:view-first-page
kind: story
status: active
title: 'The first live view: session list, page line, timeline of actions and events, live follow'
summary: The page compiled into surfd shows every session, the selected session's page line, its actions with their events on a timeline with a playhead and Latest, live; with the Playwright tooling and the fixture in the gate.
relations:
- decomposes: epic:live-view
- depends_on: story:view-surfd-serves
- depends_on: story:view-page-prototype
scope:
- confidence: cited
  path: crates/surfd/src/view.rs
- confidence: cited
  path: crates/view-fixture
- confidence: cited
  path: crates/view/web
- confidence: cited
  path: taskfiles/view-test.yml
- confidence: cited
  path: view-tests
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 4}
---
## Context

The operator asked to have the first version of the view open in his browser as soon as it
exists. `story:view-page-prototype` gives that first version over example data: the page's
files under `crates/view/web/`, `surf_view::page_files()` in `crates/view/src/assets.rs`, and the
example program `prototype`. This story puts real sessions behind those files and holds them in
the gate: after it, `surf-agent open <url>` and the address in `$XDG_RUNTIME_DIR/surf/view-url`
show every session live. It shows what exists before the parser: sessions, their page line, their
actions and the components' events. The panels for links, controls and changes come later
(`story:view-now-panel`, `story:view-controls-and-changes`), the detail panel after it
(`story:view-detail-panel`).

`story:view-surfd-serves` gives the routes over real sessions, `docs/view-stream.md` what they
answer, `docs/design/live-view.md` (`story:view-decisions`) the token's path (item 6), the test
tooling (item 4) and the page files compiled into the binary (item 5).

Layout, after the SDC run inspector (design section 12): a header with the name and a "live"
marker with the time of the last update; the session list on the left; the selected session in
the centre; the timeline with a playhead at the bottom; a right column the detail panel fills
later. The selection is carried in the address. Semantic, never a tree of tags: the page shows
sessions, navigations, requests, documents and actions in the specification's terms.

## Domain relations

- `surf.agent.Session → surf.agent.Action`, one to many, the session owns its actions: written
  into `spec/domains/agent.yaml` by `story:agent-page-path-spec`; the timeline shows them in
  sequence order.
- An entry belongs to the action entry it follows: for each action, the action entry, then the
  events of that action in the order the components published them, then the answer entry
  (decision D19 of `docs/plan/agent-plan-revision-2.md`, stated once by
  `story:agent-session-events`).

## Change

- `crates/view/web/`, completing the files `story:view-page-prototype` wrote (no new file, so
  `crates/view/src/assets.rs` and `crates/view/src/lib.rs` are not changed); plain modules, no
  framework, no build step, nothing from another origin, every value through `dom.js` and
  `textContent`:
  - `state.js`: the playhead: the selected action of the selected session, "Latest" on or off,
    and the count of newer actions.
  - `timeline.js`: selecting an action moves the playhead to it and shows the page line of that
    action's answer entry; "Latest" follows live and moves the playhead to each new action; with
    an earlier action selected the playhead stays and the marker says how many actions are newer;
    the left and right arrow keys move the playhead one action; the entries the log dropped
    (the detail's `dropped`, and `log_dropped`) are shown as one gap with its count; the event
    marks under a segment grouped by domain (`surf.navigation`, `surf.net`, `surf.document`,
    `surf.agent`).
  - `page.js`: the page line at the playhead, not only of the newest answer entry.
  - `view.js`: `?session=<id>&at=<sequence>` in the address, written as the selection changes and
    read at load; never the token.
  - `stream.js`, `sessions.js`, `index.html`, `view.css`, `dom.js`: changed only where the cases
    below need it; the stream rules stay those of `docs/view-stream.md` ("The stream") and item 6
    of `docs/design/live-view.md` as the prototype has them (first stream from `newest`,
    reconnect from the last `id:` with `/api/sessions` loaded again, `reset`).
- `crates/surfd/src/view.rs`: one line: the server's `Config.files` is `surf_view::page_files()`.
- The test tooling of item 4:
  - `view-tests/`: `package.json` pinning `@playwright/test` at `1.65.0`, the committed
    `package-lock.json`, `playwright.config.mjs` (Chromium's headless shell, revision 1254, which
    that version names; one wide viewport), `.gitignore` for `node_modules`, and
    `tests/first-page.spec.mjs`;
  - `crates/view-fixture` (package `surf-view-fixture`, `publish = false`, standard library only):
    a binary that starts a page server over `crates/view-fixture/pages/` on
    `std::net::TcpListener`, starts the built `surfd` with a private `XDG_RUNTIME_DIR` under
    `.out/`, opens the sessions its arguments ask for with the built `surf-agent`, prints one JSON
    line (the view address with its token, each session's identity and token, the page server's
    address, the path of `surf-agent`), and stops everything when its standard input closes;
  - `crates/view-fixture/pages/first/`: the pages the cases load;
  - `taskfiles/view-test.yml`: a task `view-test` as item 4 of `docs/design/live-view.md` decides:
    it builds the binaries the fixture starts, then runs `npm ci --prefer-offline --no-audit
    --no-fund` and `npx playwright test --workers=2` in `view-tests/`, with no `slot` inside the
    task: the task runs under `slot task check`, already in a job of the pool, and a `slot` inside
    it would take a second job (measured, item 4). Decision D16; the root `Taskfile.yml` lines are
    handed to the coordinator (below).
- `view-tests/tests/first-page.spec.mjs`, each case against the fixture:
  1. the page loads with no console error and no `securitypolicyviolation` event;
  2. two sessions are listed with state, address and the time since their last action;
  3. a session opened after the page loaded appears without a reload;
  4. a closed session stays listed, greyed and marked closed;
  5. selecting a session shows its page line: status, media type and address; a session whose
     page failed shows `failed`, the reason and the address;
  6. after `open`, `go`, `go`, the timeline holds three segments in order, and under each as many
     event marks as `/api/sessions/<id>` holds event entries between that action's action entry
     and its answer entry (D19);
  7. with "Latest" on, a `go` adds a segment, moves the playhead to it and updates the page line
     within 2 s;
  8. with the first action selected, the page line is the one of the first action's answer
     entry, and a new `go` leaves the playhead there and shows 1 newer action;
  9. the left and right arrow keys move the playhead one action;
  10. a reload of `?session=<id>&at=1` restores the selection; after the first load no part of
      `location.href` holds the token;
  11. the live marker shows live and the time of the last update; with `surfd` stopped it shows
      that it is reconnecting;
  12. an address holding `"`, `<script>alert(1)</script>` and `<img src=x onerror=alert(1)>`, sent
      with `go`, is shown as text in the session list, the page line and the timeline segment: the
      number of `script` and `img` elements in the page is unchanged and no dialog opens;
  13. the first `/events` request of the page carries `Last-Event-ID` equal to `newest` of the
      `/api/sessions` answer the page loaded before it (both read from the requests the test
      records);
  14. with the browser context set offline (`context.setOffline(true)`), a `go` issued, and the
      context set online again: the page reconnects with `Last-Event-ID` set to the last `id:` it
      had read, requests `/api/sessions` again after the reconnect, shows the new action's
      segment and page line without a reload, and holds no segment twice.

Surfaces written: `crates/view/web/` (the files of `story:view-page-prototype`), `crates/surfd/src/view.rs`
(one line), `crates/view-fixture/`, `view-tests/`, `taskfiles/view-test.yml`.

## For Taskfile.yml (decisions D16 and D30, written by the coordinator)

- The include of `taskfiles/view-test.yml`, and `view-test` in `check` after `test` (and after
  `view-stream-check`).

## For AGENTS.md and README.md (decision D7)

- `AGENTS.md` "Layout": "`crates/view/web/`: the live view's page, plain HTML, CSS and
  JavaScript modules compiled into `surfd`; page text only through `textContent`."
  "`crates/view-fixture`: test tooling, a binary that starts a page server, `surfd` and sessions
  for the page tests. `view-tests/`: the Playwright tests of the page; Node and Playwright are
  gate tooling only."
- `AGENTS.md` "Commands": "`task view-test`: the page tests in a browser."
- `README.md`, a new section "Watching sessions live": open a session with `surf-agent open
  <url>`, open the address in `$XDG_RUNTIME_DIR/surf/view-url` in a browser on the same machine;
  the view only reads; the token in the address is valid until `surfd` exits, and the browser may
  keep that address in its history.

## Acceptance

`slot task view-test` passes with the 14 cases of `view-tests/tests/first-page.spec.mjs`, each a
test named after its case, and `slot task check` exits 0 with `view-test` among its steps once the
coordinator has added its include.

## Out of scope

The page's first files and the example program (`story:view-page-prototype`); the detail panel and
the requests (`story:view-detail-panel`); links, controls and changes; the
title as hostile input (no session has a title before `story:agent-follow-links`; its case is in
`story:view-now-panel`); screenshots; more than one viewport.
