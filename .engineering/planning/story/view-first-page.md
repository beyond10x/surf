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
  path: crates/view-fixture
- confidence: cited
  path: crates/view/examples/prototype.rs
- confidence: cited
  path: crates/view/web
- confidence: cited
  path: taskfiles/view-test.yml
- confidence: cited
  path: view-tests
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 4}
---
## Context

The operator asked to have the first version of the view open in his browser as soon as it
exists. `story:view-page-prototype` gave that first version over example data: the nine files
under `crates/view/web/`, `surf_view::page_files()` in `crates/view/src/assets.rs`, and the
example program `prototype`. `story:view-surfd-serves` (wave 6) made `surfd` serve those files
over real sessions: `crates/surfd/src/daemon.rs` (`start_view`) already passes
`files: surf_view::page_files()`, and `$XDG_RUNTIME_DIR/surf/view-url` holds the address. So on
`main` the page already runs against real data. This story completes what the page shows and
puts it in the gate: after it, the address in `view-url` shows every session live, with a
playhead over each session's actions. The panels for links, controls and changes come later
(`story:view-now-panel`, `story:view-controls-and-changes`), the detail panel after it
(`story:view-detail-panel`).

What the page does on `main` (read in `crates/view/web/`, not yet run in a browser by any test):
the session list with state, address, title and time since the last action (`sessions.js`); a
session added, closed or died by the stream's `session` events; the page line of the newest
answer entry (`state.js` `pageLine`, `page.js`); one segment per action with one mark per event
and the newest segment marked (`timeline.js`); the live marker; the stream from `newest` of
`/api/sessions`, reconnect from the last `id:` with `/api/sessions` loaded again, `reset`
(`stream.js`, `view.js`). What it lacks: a playhead, "Latest", the count of newer actions, the
arrow keys, the selection in the address, the page line at the playhead, the gap of dropped
entries, the marks grouped by domain, and the `null` values of an unreadable entry
(`timeline.js` `shortName` calls `lastIndexOf` on the event's `name`, which is `null` for an
unreadable entry, and the error ends the stream).

`docs/view-stream.md` is the contract, `docs/design/live-view.md` the decisions: the token's path
(item 6), the test tooling (item 4), the page files compiled into the binary (item 5).

Facts of the stream the page handles (wave 6, `docs/plan/agent-wave-6.md`): a real `open` is the
action, four events and the answer; a real `go` the action, nine events (ending with
`surf.document.ParsingStarted`; no `DocumentParsed` until `story:html-document-port`) and the
answer; an entry's `sequence`, `command` or an event's `name` is `null` for an unreadable entry;
an event over 1 MiB arrives alone; a replay of more than 256 numbers is answered `reset`; a
session can be listed with `log_dropped` and no entries.

Layout, after the SDC run inspector (design section 12): a header with the name and a "live"
marker with the time of the last update; the session list on the left; the selected session in
the centre; the timeline with a playhead at the bottom; a right column the detail panel fills
later. Semantic, never a tree of tags: the page shows sessions, navigations, requests, documents
and actions in the specification's terms.

## Domain relations

- `surf.agent.Session → surf.agent.Action`, one to many, the session owns its actions
  (`spec/domains/agent.yaml`); the timeline shows them in sequence order.
- An entry belongs to the action entry it follows: for each action, the action entry, then the
  events of that action in the order the components published them, then the answer entry
  (decision D19 of `docs/plan/agent-plan-revision-2.md`).

## Change

### The page, `crates/view/web/`

The nine files stay nine (`crates/view/tests/prototype.rs` holds the list), so
`crates/view/src/assets.rs` and `crates/view/src/lib.rs` do not change. Plain modules, no
framework, no build step, nothing from another origin, every value through `dom.js` and
`textContent` (`crates/view/tests/web.rs` cases 9 and 10 keep holding).

- `state.js`: the playhead: the selected action of the selected session, "Latest" on or off, and
  the count of actions newer than the playhead. Latest is on at load unless the address names an
  action. Selecting an action (click or arrow key) turns Latest off; the "Latest" control turns
  it on and moves the playhead to the newest action.
- `timeline.js`:
  - each segment selectable; the selected one carries the playhead (`aria-current="step"`);
  - with Latest on, each new action moves the playhead to it; with Latest off the playhead stays
    and a marker reads `<n> newer` (`1 newer` for one);
  - ArrowLeft and ArrowRight (no modifier, handled on the document: the page has no text input)
    move the playhead one action and turn Latest off; at either end they do nothing;
  - the marks under a segment grouped by domain, each group labelled with its domain, in this
    order: `surf.shell`, `surf.agent`, `surf.navigation`, `surf.net`, `surf.document`,
    `surf.render`, then one group `other` for any other prefix and for a `null` name;
  - a segment's sequence or command that is `null` is written `?`; nothing in the timeline
    throws on a `null` member of `docs/view-stream.md`;
  - a gap in front of the first segment when the session detail's `dropped` is above 0, its text
    `<dropped> dropped`; entries kept before the first kept action entry (the rest of an action
    whose start was dropped) are drawn in no segment. With `log_dropped` true the timeline holds
    the one gap, text `log dropped`, and no segment. The count is the one of the last detail
    loaded; the stream does not carry it.
- `page.js` and `state.js`: the page line of the action at the playhead's answer entry; for an
  action whose answer is not held, `running`.
- `view.js` and `stream.js`: the selection in the address's fragment,
  `#session=<id>&at=<sequence>`, written with `history.replaceState` as the selection changes
  (`at` absent while Latest is on, or when the selected action's sequence is `null`), and read at
  load. Not the query: the route table answers 404 to any target holding `?`
  (`docs/view-stream.md`, "The route table"; `route` in `crates/view/src/http.rs`), so a reload of
  `/?session=…` would get no page. `takeToken` (`stream.js`) keeps a fragment that is not the 64
  digits of a token; today it removes every fragment. A fragment of a session not listed, or of
  a sequence not held, falls back to the first session and Latest on.
- `index.html`, `view.css`, `sessions.js`, `dom.js`: changed only where the cases below need it
  (the "Latest" control, the newer marker, the gap, the groups). The stream rules stay those of
  `docs/view-stream.md` ("The stream") as `stream.js` has them.

### The prototype's data, `crates/view/examples/prototype.rs`

Its `open` publishes the four events of a real `open` and its `go` the nine of a real `go`, in
the order of the example of `docs/view-stream.md` (entries 2 to 5 and 8 to 16), with
`surf.document.ParsingStarted` (`document_id`, `body`) in place of `surf.document.DocumentParsed`
(line 579 today). Its tests (`crates/view/tests/prototype.rs`) stay green.

### The fixture, `crates/view-fixture` (package `surf-view-fixture`)

`publish = false`, `license = "Apache-2.0"`, edition 2024, `#![forbid(unsafe_code)]`, path
dependencies on `surfd` and `surf-view` only. A binary `surf-view-fixture`:

- its directory under `$XDG_RUNTIME_DIR` (as `test_base()` of
  `crates/agent/tests/common/base.rs` decides; never under the checkout: a session socket is
  `<dir>/surf/<36-character identity>/session.sock` and must stay within 107 bytes, which a
  checkout path under `.out/` does not leave), removed when it stops;
- a page server on `std::net::TcpListener`, 127.0.0.1 port 0, serving the files of
  `crates/view-fixture/pages/` by path (`text/html` for `.html`), the query ignored, 404 for
  anything else; it reads a request head of up to 64 KiB;
- `surfd` in process (`surfd::Daemon::start`, as `surfd_in_process` of
  `crates/session/tests/common/mod.rs` does, decision D29), with the `surf-session` built beside
  the fixture's executable, `idle_exit` of one hour, and the options `--entries-per-session <n>`
  and `--bytes-total <n>` setting `surfd::log::Bounds`; dropping the daemon kills its session
  processes, so the fixture leaves none;
- one JSON line on standard output once ready:
  `{"view":"http://127.0.0.1:<port>/#<token>","piped":"http://127.0.0.1:<port>/#<token>","pages":"http://127.0.0.1:<port>","unreachable":"http://127.0.0.1:<port>/"}`
  (`unreachable`: a port the fixture bound and released; `piped`: the same view through the
  pipe below);
- a pipe, so that a case can take the connection between the browser and `surfd` away: a
  `TcpListener` on 127.0.0.1 port 0 that passes each connection's bytes to the view's port and
  back, and writes the one `Host` line of the request head as `Host: 127.0.0.1:<the view's
  port>` (the view answers 403 to any other `Host`, and one request per connection, so only
  the first head of a connection is read, up to 64 KiB). The view's own port and checks are
  unchanged: `view` is `surfd`'s address, and only cases 15 and 16 load the page from `piped`;
- then one command per line on standard input, the words separated by a tab (no argument a case
  sends holds a tab), each answered with one JSON line, strings written with
  `surf_view::to_json`:
  - `open` or `open<TAB><url>`: runs the built `surf-agent open` and answers `{"id":"<id>"}`; the
    session's token stays in the fixture;
  - `agent<TAB><id><TAB><word>…`: runs the built `surf-agent <word>…` as that session
    (`SURF_SESSION`, `SURF_TOKEN`, the fixture's `XDG_RUNTIME_DIR`, nothing else) and answers
    `{"status":<exit status>}`;
  - `kill<TAB><id>`: kills that session's process (found under `/proc` by its identity argument)
    and answers `{}`;
  - `stop-surfd`: drops the daemon and answers `{}`; every later `open` or `agent` is answered
    `{"error":"stopped"}`, so no `surf-agent` starts a `surfd` of its own;
  - `cut`: shuts down both sides of every connection open through the pipe, closes every
    connection it accepts from then on without reading it, and answers `{}` once the open ones
    are shut; `mend`: the pipe passes connections again, and answers `{}`;
  - anything else: `{"error":"command"}`;
- the end of standard input stops everything (the daemon, the page server) and removes its
  directory.
- `crates/view-fixture/pages/first/`: `a.html` and `b.html`, two small pages.
- `crates/view-fixture/tests/fixture.rs`: the fixture started (finding `surf-agent` and
  `surf-session` beside the test's target directory, built with the running cargo when absent,
  as `bin_dir()` of `crates/session/tests/common/mod.rs` does), `open` and a `go` to `a.html`;
  `GET /api/sessions` with the token of the `view` member lists the session with a `loaded` page
  line of status 200; then standard input closed: the fixture exits within 10 s, the session's
  process is gone (`/proc/<pid>` absent) and the fixture's directory is removed.

### The page tests, `view-tests/`

- `package.json` pinning `@playwright/test` at `1.65.0`, the committed `package-lock.json`,
  `.gitignore` (`node_modules/`, `test-results/`, `playwright-report/`);
- `playwright.config.mjs`: Chromium's headless shell (revision 1254, which 1.65.0 names), one
  viewport of 1,400 by 900, `fullyParallel: true`, reporter `list`, `outputDir` under
  `../.out/view-test/`;
- `tests/fixture.mjs`: a Playwright fixture per test that starts
  `target/debug/surf-view-fixture` (its own `XDG_RUNTIME_DIR` comes from the fixture), reads its
  line, offers the commands above, records every request of the page and every console message,
  page error and `securitypolicyviolation`, and closes the fixture's standard input at the end;
- `tests/first-page.spec.mjs`, the cases below.

### The task, `taskfiles/view-test.yml`

A task `view-test` (decision D16; item 4 of `docs/design/live-view.md`), `dir` the repository
root, `env: RUSTFLAGS: -D warnings` (the flags of `task build`, so it rebuilds nothing `check`
built): `cargo build --locked --quiet -p surf-agent -p surf-session -p surf-view-fixture`, then
`npm --prefix view-tests ci --prefer-offline --no-audit --no-fund`, then
`npx playwright test --workers=2` in `view-tests/`. No `slot` inside it: it runs under
`slot task check`, already in a job of the pool (measured, item 4 (c)). It installs no browser:
on this machine `~/.cache/ms-playwright` holds `chromium_headless_shell-1254` and the npm cache
holds `@playwright/test` 1.65.0 (read 2026-11-21); the forge's runner installs both in the
workflow (below).

### `first-page.spec.mjs`, each case a test named after it, each with its own fixture

1. `loads without an error`: no console message of type `error`, no page error, no
   `securitypolicyviolation`.
2. `lists two sessions`: two sessions opened, each listed with its state `open`, its address and
   a time since its last action.
3. `a new session appears without a reload`: an `open` after the page loaded is listed within
   2 s, last, with no new request of `/`.
4. `ended sessions stay listed`: after `agent close` one session is listed `closed` and after
   `kill` another is listed `died`, each item with the class `ended`.
5. `page line`: a session after a `go` to `a.html` shows status `200`, media type `text/html`
   and the address; a session after a `go` to the `unreachable` address shows `failed`,
   `Unreachable` and the address.
6. `timeline of three actions`: after `open`, `go`, `go` the timeline holds three segments, their
   commands `open`, `go`, `go` and sequences 1, 2, 3; under each, as many marks as
   `/api/sessions/<id>` (read by the test with the view token) holds event entries between that
   action entry and its answer entry; the marks in groups labelled by domain, in the order of
   "The page", and in each group as many marks as that action has events of the domain.
7. `latest follows`: with Latest on, a `go` adds a segment, moves the playhead to it and shows
   its page line within 2 s.
8. `playhead stays`: with the first action selected, the page line is that of the first action's
   answer entry (`none`); a new `go` leaves the playhead there and the marker reads `1 newer`;
   the Latest control then moves the playhead to the newest action and removes the marker.
9. `arrow keys move the playhead`: ArrowLeft and ArrowRight move it one action each, and stop at
   either end.
10. `selection in the address`: after selecting the second action, `location.hash` is
    `#session=<id>&at=2`; a reload restores that selection; at no point after the first load
    does `location.href` hold the view token.
11. `live marker`: it reads `live` and a time; after `stop-surfd` it reads `reconnecting` within
    5 s.
12. `hostile address is text`: a `go` to `<pages>/first/a.html?"<script>alert(1)</script>` and
    one to `<pages>/first/<img src=x onerror=alert(1)>` (which the transport refuses for its
    spaces): each address's exact text is the `textContent` of the session list item's address,
    the page line's address and the segment's argument; the numbers of `script` and `img`
    elements in the document are those before the `go`; no request to `/x`; no dialog; no
    `securitypolicyviolation`. The case does not depend on whether the page loaded or failed.
13. `hostile title and unreadable entries are text`: `page.route` answers `/api/sessions`,
    `/api/sessions/<id>` and `/events` with bodies written as `surf_view::to_json` writes them
    (`<` as `<`): a summary and a page line whose title is
    `<img src=x onerror=alert(1)><script>alert(1)</script>`, an action entry with `sequence` and
    `command` `null`, and an event entry with `name` `null`. The title's exact text is the
    `textContent` of the list item's title, the heading and the page line; no `img` or `script`
    element is added; no page error; the `null` action is a segment written `?` and the `null`
    event a mark in the group `other`. With `textContent` replaced by `innerHTML` in `dom.js`
    the case fails on the element count.
14. `first stream from newest`: the first `/events` request carries `Last-Event-ID` equal to
    `newest` of the `/api/sessions` answer before it (both read from the recorded requests).
15. `reconnect after a lost connection`: the page loaded from `piped`; `cut`: the live marker
    reads `reconnecting` within 5 s; a `go`; `mend`: the page requests `/events` again with
    `Last-Event-ID` equal to the last `id:` it read, requests `/api/sessions` again after it,
    shows the new segment and page line without a reload, and holds no segment twice.
16. `reset after a long absence`: the page loaded from `piped`; `cut`, then 25 `go` (275
    entries, past the 256 a replay may hold), `mend`: the page loads `/api/sessions` and the
    selected session again and shows 26 segments (the `open` and the 25 `go`), each once.
17. `dropped entries are a gap`: a fixture with `--entries-per-session 30`, a session with
    `open` and four `go` (6 and 4 × 11, 50 entries, so 20 dropped): the gap reads `<dropped> dropped` with the `dropped` of
    `/api/sessions/<id>`, and the segments are as many as its kept action entries. A fixture
    with `--bytes-total` small enough that a closed session's log is dropped whole while another
    session goes on: selecting the closed session shows the gap `log dropped` and no segment.
18. `an entry over 1 MiB`: `open`, then three `go` to `<pages>/first/a.html?` followed by
    60,000 `<` (each written as six bytes by the JSON writer, so the third `go`'s answer entry,
    holding the address four times, is over 1,048,576 bytes; the test reads its length from
    `/api/sessions/<id>` and asserts it): the page shows four segments and the third `go`'s page
    line, with the address's exact text.

Before writing the page's changes, the unit runs the cases against the page of `main` and
reports which fail there (expected: 6, 7, 8, 9, 10, 13, 17, and 15 or 16 if the page's reconnect
misses an entry); each new behaviour has a case that fails without it.

Measured by the unit on 2026-11-21, before the page's changes (headless shell 1254, browser
version 158.0.8127.18): `context.setOffline(true)` fails a new request to 127.0.0.1
(`net::ERR_INTERNET_DISCONNECTED`) and does not end a `fetch` stream already open; 20 s
offline, past one keep-alive, the stream still delivered a `go`. So cases 15 and 16 take the
connection away with the fixture's pipe (`cut`, `mend`) and not with `setOffline`. The headless
shell does not request `/favicon.ico`: case 1 has no exception.

Surfaces written: `crates/view/web/` (the existing nine files), `crates/view/examples/prototype.rs`,
`crates/view-fixture/`, `view-tests/`, `taskfiles/view-test.yml`. Not `crates/surfd/`: the
server's page files are already `surf_view::page_files()` (`crates/surfd/src/daemon.rs`).

## For the coordinator (decisions D7, D16, D30)

- `Taskfile.yml`: the include of `taskfiles/view-test.yml` (`flatten: true`, `optional: true`),
  and `view-test` in `check` after `view-stream-check`, before `html-conformance`.
- `Cargo.lock`: the package `surf-view-fixture` (`cargo generate-lockfile --offline` on the wave
  branch); `lock-check` then counts 22 packages.
- `.github/workflows/check.yml`: before `task check`, `actions/setup-node` pinned by commit as in
  `.github/workflows/pages.yml` (`node-version: "22"`), and a step that runs
  `npm --prefix view-tests ci --no-audit --no-fund` and
  `npx --prefix view-tests playwright install --with-deps --only-shell chromium`. Without it the
  runner has no headless shell and `view-test` fails.
- `AGENTS.md` "Layout": "`crates/view/web/`: the live view's page, plain HTML, CSS and JavaScript
  modules compiled into `surfd`; page text only through `textContent`; the selection in the
  fragment." "`crates/view-fixture` (`surf-view-fixture`): test tooling, a binary that starts a
  page server, `surfd` in process and sessions for the page tests, driven by commands on its
  standard input; depends on `surfd` and `surf-view`. `view-tests/`: the Playwright tests of the
  page; Node and Playwright are gate tooling only." "Commands": "`task view-test`: the page
  tests in a browser." "The gate's numbers": the suites and tests of the root workspace, the 22
  packages, the line `task view-test` prints.
- `README.md`, a section "Watching sessions live": open a session with `surf-agent open <url>`,
  open the address in `$XDG_RUNTIME_DIR/surf/view-url` in a browser on the same machine; the view
  only reads; the token in the address is valid until `surfd` exits, and the browser may keep
  that address in its history.

## Acceptance

`slot task view-test` exits 0 and its output lists the 18 tests of
`view-tests/tests/first-page.spec.mjs` by the names above as passed;
`cargo test -p surf-view-fixture` passes `tests/fixture.rs`; `cargo test -p surf-view` passes
with `web` and `prototype` unchanged in count; after the coordinator's hand-offs,
`slot task check` exits 0 with `view-test` among its steps.

## Out of scope

The detail panel and the requests (`story:view-detail-panel`); links, controls and changes; a
real page's title (no session has one before the parser fills it: case 13 tests the title over
`page.route`, `story:view-acceptance` over a real page); screenshots; more than one viewport;
Firefox. `surfd`'s own start and bounds: the mode of `view-url` under another umask, a start
where listening or writing `view-url` fails, an unreadable entry from a session, a client that
sends its request slowly, 16 connections held open against `surfd`'s work, a late answer after
600 s and a session killed before its answer (`docs/plan/agent-wave-6.md`, findings) go to a
`surfd` story of their own and to the review planned after wave 7. The order of the session list
under two simultaneous opens (`story:view-list-order`) and the list's copy under the lock
(`story:view-stream-list-copy`): this story needs neither; its fixture opens sessions one after
the other.
