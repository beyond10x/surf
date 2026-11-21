---
format: aep.planning-md/3
id: story:view-page-prototype
kind: story
status: implemented
title: A prototype of the live view page over example data, before surfd serves it
summary: The first version of the page files under crates/view/web/, compiled into surf-view, and an example program that serves them over scripted sessions in the shapes of docs/view-stream.md, so a person can open the view before the daemon exists.
relations:
- decomposes: epic:live-view
- depends_on: story:view-http-server
scope:
- confidence: cited
  path: crates/view/examples/prototype.rs
- confidence: cited
  path: crates/view/src/assets.rs
- confidence: cited
  path: crates/view/src/lib.rs
- confidence: cited
  path: crates/view/tests/prototype.rs
- confidence: cited
  path: crates/view/tests/web.rs
- confidence: cited
  path: crates/view/web
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T02:01:50Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T02:01:51Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-21T02:59:46Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

The operator asked to see the `surfd` view in his browser as soon as a first prototype exists.
`story:view-first-page` shows real sessions, but it waits for `story:view-surfd-serves` and with it
for the daemon stories. This story puts the page in front of a person before them: the first
version of the page's files, served by the server `story:view-http-server` delivered, over
scripted example data that has the shapes of `docs/view-stream.md` and nothing else. After it,
`cargo run -p surf-view --example prototype` prints an address; opened in a browser on the same
machine, it shows three sessions, a page line, a timeline of actions and their events, and one
session that keeps moving.

What exists (`story:view-http-server`, crate `crates/view`, package `surf-view`): `serve(listener,
config, source) -> io::Result<Server>`, where `Config` holds the view token (`surf_secret::Token`),
the one `Host` value (`127.0.0.1:<port>`), the page's files (`Vec<PageFile>`, each `name`,
`content_type`, `bytes`; `index.html` answers `/`, every other file `/<name>`) and the `Limits`.
`serve` refuses a listener not on 127.0.0.1 and a `host` not naming the listener's port; the
returned `Server` stops when it is dropped. The data comes from a `Source` the caller implements:
`sessions() -> Vec<u8>`, `session(id) -> Option<Vec<u8>>`, `newest() -> u64`, and
`read_from(number) -> Result<Box<dyn Reader>, TooOld>`, where `Reader::next(wait) -> Next` gives
`Item { number, kind: EventKind::{Entry, Session}, data }`, `Idle` or `End`, waits no longer than
`wait`, and an item's `data` is one JSON line with no CR and no LF. `reset` is written by the
server itself. `read_from(newest())` never answers `TooOld`.

The page follows `docs/view-stream.md` (routes, JSON, the stream), item 4 to 6 and 9 of
`docs/design/live-view.md` (the page files compiled in with `include_str!`, the token in the
fragment, system fonts), and the layout `story:view-first-page` describes, after the SDC run
inspector: a header with the name and a "live" marker with the time of the last update; the
session list on the left; the selected session in the centre; the timeline at the bottom; an
empty right column the detail panel fills later. Semantic, never a tree of tags.

## Domain relations

- An entry belongs to the action entry it follows: for each action, the action entry, then the
  events of that action in the order the components published them, then the answer entry
  (decision D19 of `docs/plan/agent-plan-revision-2.md`; `docs/view-stream.md`, "The entries").
  The example data is written in that order.
- `surf.agent.Session → surf.agent.Action`, one to many, the session owns its actions:
  `spec/domains/agent.yaml` (written by `story:agent-page-path-spec`). The example data shows it
  only as JSON; nothing here reads `spec/`.

## Change

- `crates/view/web/`, the first version of the files `story:view-first-page` names, plain
  modules, no framework, no build step, nothing from another origin:
  - `index.html`: the regions (header with the live marker, session list, centre, timeline,
    right column), and one `<script type="module" src="view.js">`. No inline script, no `style`
    element or attribute, no event attribute.
  - `view.css`: the layout; system font stacks, no font file, no image.
  - `dom.js`: the only place the page creates elements; every value is written with
    `textContent`, never as markup.
  - `stream.js`: the token taken from the fragment into `sessionStorage` and the fragment removed
    with `history.replaceState`; every `/api/` request with `Authorization: Bearer`; `/events` read
    with `fetch` and a stream reader that parses the framing of `docs/view-stream.md` (`id:`,
    `event:` of `entry`, `session` or `reset`, `data:`, the `: keep-alive` comment); the first
    stream opened with `Last-Event-ID` set to `newest` of `/api/sessions`; on its end a reconnect
    after a delay that grows to 5 s, with `Last-Event-ID` set to the last `id:` read, and
    `/api/sessions` and the selected session loaded again; an `entry` whose `number` it holds is
    ignored; on `reset` everything loaded again from the `newest` it carries.
  - `state.js`: sessions, their entries and the selection; no DOM.
  - `sessions.js`: the session list: state (`open`, `closed`, `died`, the ended ones greyed),
    address, title when there is one, the time since the last action; a click selects.
  - `page.js`: the page line of the selected session's newest answer entry: status, media type,
    address, title; `failed <reason> <address>`; or `none`.
  - `timeline.js`: the selected session's actions in order, each a segment labelled with its
    command and argument, under each the events between its action entry and its answer entry,
    one mark per event named by its event name; the newest action marked; new actions appended
    live.
  - `view.js`: loads `/api/sessions`, selects the first session, loads it, follows the stream,
    and keeps the live marker (live and the time of the last update, or reconnecting).
- `crates/view/src/assets.rs`: `page_files() -> Vec<PageFile>`, each file of `crates/view/web/`
  by `include_str!` with its name and the content type of `docs/view-stream.md`
  (`text/html; charset=utf-8`, `text/css; charset=utf-8`, `text/javascript; charset=utf-8`).
  `crates/view/src/lib.rs`: the module and `pub use assets::page_files`.
- `crates/view/examples/prototype.rs`, standard library and `surf-view` (with its `surf-secret`)
  only:
  - a `Source` over example data kept in memory, every JSON body and entry in the members, order
    and string rule of `docs/view-stream.md` and nothing beside them, written with a small string
    writer of the document's rule;
  - three sessions in three states: one `open` that ran `open` and `go
    http://127.0.0.1:41000/a.html` (loaded, 200, `text/html`); one `closed` that ran `open`, `go
    http://127.0.0.1:9/` (failed, `Unreachable`) and `close`; one `died` after `open` and a `go`
    to a page whose title holds `<script>alert(1)</script>` and `"`, so a person sees it written
    as text. Each action with its events, by the names of the "Event names" block and their
    fields by their `spec/` names, each field value a string (a byte field `<n> bytes`), then its
    answer entry, in the order of D19, numbered across sessions from 1;
  - the `open` session gets a new entry about every second, one `go` at a time between `a.html`
    and `b.html` (its action entry, its five events, its answer entry), and after each answer
    entry a `session` item of that session with the answer's number, live only;
  - `read_from(n)`: the kept entries above `n`, then live; the example keeps the last 10,000
    entries and answers `TooOld` below them;
  - bound to `127.0.0.1:0`, a view token drawn with `surf_secret::Token::draw`, `serve` with
    `page_files()` and `Limits::default()`, and one line on standard output,
    `http://127.0.0.1:<port>/#<64 hexadecimal digits>`, flushed; then it runs until it is
    killed.
- `crates/view/tests/prototype.rs`: the counted cases against the example program that
  `cargo test` builds (`target/<profile>/examples/prototype`, found from the test executable's
  directory), started by the test and killed at its end, spoken to over `std::net::TcpStream`
  with the token of its line:
  1. the program prints exactly one line within 5 s, matching
     `http://127.0.0.1:<port>/#` and 64 lower-case hexadecimal digits, and is still running 2 s
     later;
  2. `/` answers 200, `text/html; charset=utf-8`, with the bytes of `crates/view/web/index.html`;
  3. each file of `page_files()` answers 200 under `/<name>` with its content type and the bytes
     of its file under `crates/view/web/`; the list holds the nine files of Change;
  4. `/api/sessions` with the token answers 200, `application/json`; the body starts with
     `{"newest":`, holds three sessions, one each with `"state":"open"`, `"state":"closed"` and
     `"state":"died"`, and the members of each summary in the order `id`, `state`, `address`,
     `title`, `opened_at`, `last_action_at`, `ended_at`, `page`;
  5. `/api/sessions/<id>` of each listed session answers 200; every entry starts with the
     members `number`, `session`, `index`, `kept_at`, `kind`; the `kind` values read in order are
     `action`, any number of `event`, `answer`, repeated (D19); `number` increases;
  6. `/events` with the token and no `Last-Event-ID` delivers, within 3 s, an `event: entry` whose
     `id:` is above the `newest` read just before, and, after an `answer` entry, an
     `event: session` with the same `id:`;
  7. `/events` with `Last-Event-ID: 0` delivers `id: 1` first;
  8. the example's title holding `<script>` is written in the JSON as `<script>`, and
     no body or event data of cases 4 to 7 holds a raw `<`, `>` or `&`.
- `crates/view/tests/web.rs`:
  9. `index.html` holds exactly one `script` element, with `type="module"` and a `src`, no
     `style` element, no `style=` attribute and no attribute starting `on`;
  10. no file under `crates/view/web/` contains `innerHTML`, `outerHTML`, `insertAdjacentHTML`,
      `document.write`, `eval(`, `new Function`, `setAttribute("on`, `javascript:`, `http://`,
      `https://` or `//` at the start of an attribute value.

Surfaces written: `crates/view/web/`, `crates/view/src/assets.rs`, `crates/view/src/lib.rs` (the
module line and the re-export), `crates/view/examples/prototype.rs`, `crates/view/tests/prototype.rs`,
`crates/view/tests/web.rs`.

## For AGENTS.md (decision D7)

- "Commands": "`cargo run -p surf-view --example prototype`: the live view over example data;
  open the address it prints in a browser on the same machine."

## Acceptance

`cargo test -p surf-view --test prototype` passes with cases 1 to 8 and `cargo test -p surf-view
--test web` with cases 9 and 10, each a test named after its case; `cargo test -p surf-view --test
http` still passes its 33 cases; and `cargo tree -p surf-view -e normal` prints `surf-view` and
`surf-secret` and no other package.

## Out of scope

Real sessions and `surfd` (`story:view-surfd-serves`, `story:view-first-page`); the Playwright
tooling, the fixture and every browser test (`story:view-first-page`); selecting an earlier
action with the playhead, the selection in the address and the gap of dropped entries
(`story:view-first-page`); the detail panel, the `now` panel and the changed panel
(`story:view-detail-panel`, `story:view-now-panel`, `story:view-controls-and-changes`); a JSON
writer for real data (`story:view-surfd-serves` writes `crates/view/src/json.rs`; the example
writes its own strings).
