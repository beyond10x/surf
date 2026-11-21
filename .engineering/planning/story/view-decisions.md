---
format: aep.planning-md/3
id: story:view-decisions
kind: story
status: implemented
title: Measure what ESS gives the live view, and decide its stream, tooling, port and token
summary: 'Runs the AsyncAPI projection and an ess-http/1 trial on a scratch copy and writes docs/design/live-view.md and docs/view-stream.md: what ESS specifies and what docs/ does, the page''s test tooling, the port, the view token''s path and the kept log''s bound.'
relations:
- decomposes: epic:live-view
scope:
- confidence: cited
  path: docs/design/live-view.md
- confidence: cited
  path: docs/view-stream.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-21T01:25:58Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

`epic:live-view`, "How it gets there": the stream's shape is specified, "projected by ESS as an
AsyncAPI document where ESS can, and the read routes as an `ess-http/1` binding where ESS accepts
one; what ESS cannot give is specified in `docs/` and the first story says which is which from a
measurement". This is that story. Its outcome is a design note and the route and stream document
every later story of the epic is held to. It writes no code.

What is known before it runs (read on 2026-11-21, ess 0.96.0, not run):

- `ess generate --help` offers `--kind asyncapi`, `openapi` and `schema`. Design section 12: the
  AsyncAPI projection has not been run in this repository, and what it gives for components that
  are `reached_by: in_process` (all five, `spec/components.yaml`) is unknown.
- `AGENTS.md` ("Implementation targets"): with every component `reached_by: network`, ess 0.96.0
  generates one server binary per component that adds `clap`, `uuid` and `time`, and `validate`
  refuses both authored scenarios with `ESS-AUTHOR-040` (beyond10x/ess#792). An `ess-http/1`
  binding may need that reach.
- `ess generate ui --help`: the two targets are `react` (React and esbuild: a framework and a
  build step) and `tui` (`clap` over `ess-ui-tui`: third-party crates). Both conflict with the
  epic's Constraints as written.
- The reference for the layout, the SDC run inspector (`sdc/ui` in the organization's folder,
  read-only): a Node server that pushes `event:`/`data:` server-sent events with a `hello` event
  and a comment line every 15 s; a page of plain modules that writes markup with `innerHTML` and an
  escaping helper; vendored fonts; tests in Playwright 1.65.0 against a server started by hand,
  two viewports, screenshots. Its structure is taken; its code, its use of markup strings and its
  vendored files are not.
- `EventSource` cannot send a request header (HTML Standard, server-sent events, from memory, to
  be confirmed by this story). So a token in a header needs the stream read some other way.

Needs from `epic:agent-cli-target`, read as drafted (this story does not wait for any
implementation): the kept log of `story:agent-session-events` as `docs/plan/agent-plan-revision-2.md`
fixes it: its order (D19), the answer entry as the answer's lines (D20), the token kept out (D21),
what the log holds of a value (D22), the number across all sessions and the reader from a number
(D23), and the bounds (D24). The documents of this story follow those decisions; they decide
nothing about the log.

## Change

1. Measurements. Scratch output goes to `.out/view-measure/`. Nothing under `spec/` or
   `generated/` changes. Each command and the part of its output a decision rests on is quoted in
   the note.
   a. `ess generate --path spec --kind asyncapi --out .out/view-measure/asyncapi`, and the same
      with `--kind schema`: whether the in-process events appear as messages, with which channel
      and payload schema per field type (`surf.net.RequestId`, `surf.url.Url`, the `body` bytes
      of `surf.net.ResponseReceived`), and whether the output is JSON or YAML.
   b. On a copy of `spec/` under `.out/view-measure/spec/`: an `ess-http/1` binding that serves
      the read views (`surf.net.Requests`, `surf.navigation.Navigations` or whatever read views
      `main` has when this runs) with the components left `in_process`; the validate result; if it
      requires `reached_by: network`, the change in the gate's scenario and refusal counts; what
      `--kind openapi` then writes.
   c. Test tooling on this machine: `node --version`; whether `@playwright/test` at the SDC pin
      and its Chromium headless shell are in the user cache; whether `npm ci` succeeds without the
      network; whether a task whose command runs `npx playwright test --workers=2` completes under
      `slot task <name>` without a second `slot` prefix, and what a nested `slot` does.
   d. In the Chromium the tooling drives: whether a URL fragment removed by
      `history.replaceState` is still in `history` after a reload, and whether a fetch from the
      page sends a `Referer` with the fragment. Firefox only if it is installed.
2. `docs/design/live-view.md`, numbered items, each with the decision, the measurement it rests
   on (or "not measured" and why), what was rejected, and the answer the operator can overrule.
   The answers this story starts from, to be kept or changed by what it measures:
   1. **What ESS specifies.** The event names and field types the stream carries are those of
      `spec/` and are not redeclared. If (a) gives a message per in-process event, that projection
      is the reference; otherwise `spec/domains/*.yaml` is.
   2. **How the stream is held to it.** A task `view-stream-check`, part of `check`, that
      generates the projection of (a) into `.out/` and fails naming each event name
      `docs/view-stream.md` lists that the projection lacks. With no usable projection: the same
      check against the `events:` names of `spec/domains/*.yaml`.
   3. **The routes.** Specified in `docs/view-stream.md`, unless (b) shows an `ess-http/1`
      binding that validates with the components `in_process` and changes no count of the gate.
      `ess generate ui` is not used: both targets bring a framework, a build step or a crate.
   4. **Test tooling for the page.** Playwright, pinned like the SDC inspector, Chromium only, in
      a directory `view-tests/` of its own (`package.json`, committed `package-lock.json`,
      `playwright.config.mjs`), run by a task `view-test` that is part of `check` and calls
      `npx playwright test --workers=2` without a second `slot`. Node and Playwright are gate
      tooling like `ess` and `task`; nothing built for `surfd` depends on them. A Rust fixture
      binary starts what a test needs: a page server on `std::net::TcpListener`, `surfd` with a
      private `XDG_RUNTIME_DIR` under `.out/`, the sessions; no Python, no Node server.
   5. **Page files.** Compiled into `surfd` with `include_str!` from `crates/view/web/`. The
      binary is then the whole program: there is no install path to find, and no file that
      another program could replace between the test and the run. The cost, a rebuild for each
      page change, is accepted.
   6. **Port and token.** 127.0.0.1 and a port the operating system picks (bind to port 0): no
      fixed port on a machine with many users and other local servers. A view token drawn at each
      start with `surf-secret` (`story:agent-secret-primitives`, decision D25): 32 bytes from the
      operating system's random source, written as 64 hexadecimal digits, compared in constant
      time by `surf-secret`'s comparison. `surfd` prints `http://127.0.0.1:<port>/#<token>` on its
      standard error and writes it to `$XDG_RUNTIME_DIR/surf/view-url`, mode 0600, removed at exit,
      because `surfd` is usually started by `surf-agent open` and nobody sees its output. The token
      is in the fragment: a browser does not send a fragment in a request or a `Referer`. The page
      keeps it in `sessionStorage`, removes it from the address with `history.replaceState`, and
      sends it as `Authorization: Bearer <token>` on every `/api/` request. It reads `/events` with
      `fetch` and a stream reader (the server-sent-event framing is kept), because `EventSource`
      sends no header. The page's own files (`/` and the files the page loads) are served without
      the token: they hold no data (decision D18; the epic's constraint says so). Rejected: the
      token in the query, which lands in the history; a cookie, because cookies are not separated
      by port, so any other server on 127.0.0.1 would receive it. Accepted and said in
      `README.md`: the browser may keep the first address, fragment included, in the history of
      the user's own profile until `surfd` exits.
   7. **The kept log.** The view follows the bounds of `story:agent-session-events` and decides
      none: per session at most 10,000 records or 16 MiB, whichever is reached first, the oldest
      dropped first and counted; over all sessions at most 64 MiB; above it the logs of ended
      sessions go first, the longest-ended first, and such a session's detail says that its log
      was dropped (decision D24). `docs/view-stream.md` gives the session detail a field for the
      dropped count and one for a dropped log.
   8. **Server limits.** Request line and headers at most 8 KiB and 50 lines; 5 s to receive a
      request; 16 open connections; 8 open streams; per stream a queue of 256 entries or 1 MiB,
      past which that stream is closed and the reader resumes with `Last-Event-ID`; a comment line
      every 15 s; 10 s to write one answer or one stream write, past which the connection is
      closed. Each a field of a limits value, so tests shorten them.
   9. **Fonts and assets.** System font stacks; no font file and no image ships.
   10. **What the view holds of a value**, in the words of decision D22, and D21: a field of bytes
       (a response body, a request body) is written by the session process as its length,
       `<n> bytes`, never as the bytes; a password's value is its length, as the answer prints it;
       every other control value is kept as the answer prints it, which is what the `changed`
       panel shows. The token is not kept: the answer entry of `open` omits the `token` line, and
       no route and no stream event holds a session's token. Stated as a rule of
       `docs/view-stream.md`.
3. `docs/view-stream.md`: the route and stream document the later stories are held to.
   - The route table: path, whether the token is required, the answers and their status codes
     (200, 400, 401, 403 for another `Host`, 404, 405 with `Allow: GET`, 431, 503, 505), the
     content type of each, and the headers on every answer: the content security policy
     `default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'`,
     `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `Cache-Control: no-store`,
     `Cross-Origin-Resource-Policy: same-origin`.
   - The JSON shapes: a session summary (identity, state `open`, `closed` or `died`, address,
     title or none, the time of its last action, the page line), a session detail (the summary,
     the kept entries in order, the count of dropped entries, and whether the whole log was
     dropped, D24), and the three entry kinds in the order of decision D19: for each action, the
     action entry, then the events of that action in the order the components published them,
     then the answer entry; an entry belongs to the action entry it follows. `action` (action
     sequence number, command, argument), `event` (the qualified event name of `spec/` and its
     fields by their `spec/` names, each the exact value `surf_answer::unquote` reads from the
     quoted text the log keeps, D26; a byte field is the text `<n> bytes`, D22), `answer` (the
     blocks `surf_answer::parse` reads from the answer's lines, D20: the page line and the path
     here; `now` and `changed` are added by the stories that show them). Each entry carries its
     per-session number, the number across all sessions (D23) and the time `surfd` kept it. The
     JSON string rule: `"`, `\`, every character below U+0020, U+2028, U+2029, `<`, `>` and `&`
     written as `\u` escapes.
   - The stream: `id:` the entry's number across all sessions (D23), `event:` one of `entry`,
     `session` (a summary changed), `reset` (the reader's `Last-Event-ID` is older than anything
     kept: load again), `data:` one JSON line, and the comment line. A reader resumes with
     `Last-Event-ID` through the reader from a number that `story:agent-session-events` gives.
   - One example answer for each of these 6 requests: `GET /`, `GET /view.js` (a page file),
     `GET /api/sessions`, `GET /api/sessions/<id>`, `GET /events`, and `GET /api/sessions` without
     the token (401); one example of each of the 3 entry kinds and each of the 3 stream events;
     in fenced blocks a later story copies into golden files.

Surfaces written: `docs/design/live-view.md`, `docs/view-stream.md`. Scratch only under `.out/`.

## Domain relations

The documents name these and decide none of them:

- `surf.navigation.Navigation` and `surf.net.Request` share one identity:
  `spec/domains/navigation.yaml`, `Navigation` identity `navigation_id: surf.net.RequestId`.
- `surf.navigation.Navigation → surf.shell.Tab`: `spec/domains/navigation.yaml`, relation `tab`,
  `references`, `one`, via `tab_id`.
- `surf.agent.Session → surf.agent.Action`, one to many, the session owns its actions: in no
  `ess/1` document yet; `story:agent-page-path-spec` writes it into `spec/domains/agent.yaml`, from
  design section 6 (`approval-record:agent-design-accepted-2026-11-21`).
- An entry belongs to the action entry it follows: for each action the action entry, then the
  events of that action in the order the components published them, then the answer entry
  (decision D19, stated once by `story:agent-session-events`). A property of the kept log, not of
  the specification.

## For AGENTS.md (decision D7)

- "Layout": "`docs/design/live-view.md`: the decisions of the live view. `docs/view-stream.md`:
  its routes and its event stream."

## Acceptance

`docs/design/live-view.md` holds the ten items of Change, each quoting the command and output it
rests on or saying "not measured" with the reason; `docs/view-stream.md` holds one example answer
for each of the 6 requests `GET /`, `GET /view.js`, `GET /api/sessions`, `GET /api/sessions/<id>`,
`GET /events` and `GET /api/sessions` without the token, one example of each of the 3 entry kinds
(`action`, `event`, `answer`) and of each of the 3 stream events (`entry`, `session`, `reset`); and
`git status --short` in the unit's worktree lists those two files and nothing under `spec/`,
`generated/`, `crates/` or `conformance/`.

## Out of scope

Any code; changing `spec/` (a binding found usable in (b) is written by a later story that this
story names); acting on a session from the view.

## Scope

Derived 2026-11-21 by `story-scoper`.

- **Primary surface:** `docs/` (the two new files `docs/design/live-view.md` and `docs/view-stream.md`) — cited, the story's "Surfaces written" line and its Acceptance both list exactly these two files
- **Files written:** `docs/design/live-view.md` — cited, the story's "Surfaces written" line. `docs/design` exists and holds `agent-interface.md`; the file is new.
- **Files written:** `docs/view-stream.md` — cited, the story's "Surfaces written" line. The file is new.
- **Files written:** `.out/view-measure/` (scratch only; `.out` is git-ignored) — cited, Change item 1 and the "Surfaces written" line. It is left out of `typed-scope` because the repository does not track it.
- **Files read, not written:** `spec/` (`spec/domains/*.yaml`, `spec/components.yaml`, `spec/ess-inputs.yaml`), read-only input to the `ess generate` runs. Change item 1 says "Nothing under `spec/` or `generated/` changes", and (b) works on a copy under `.out/view-measure/spec/`. — cited
- **Files read, not written:** `AGENTS.md` ("Implementation targets"), `docs/design/agent-interface.md` (sections 3, 6, 12), `docs/plan/agent-plan-revision-1.md` (D7, D16, D17) — cited
- **Files read, not written:** the drafted body of `story:agent-session-events`, with no wait for its implementation. The story says "This story reads the draft; it does not wait for its implementation." — cited
- **Files read, not written:** `sdc/ui` in the organization's folder, as the layout reference only. The story says "read-only". — cited
- **Handed to the coordinator:** `AGENTS.md` "Layout": "`docs/design/live-view.md`: the decisions of the live view. `docs/view-stream.md`: its routes and its event stream." — cited, the story's "For AGENTS.md (decision D7)" section
- **Handed to the coordinator:** `README.md`, `Taskfile.yml`, `Cargo.lock`: nothing — cited. The story writes no code and adds no task. Items 2 and 4 name the tasks `view-stream-check` and `view-test`, but those are decisions in the note. `story:view-surfd-serves` and `story:view-first-page` write the tasks.
- **Symbols:** none, no crate. The story writes documents only. — cited, "It writes no code" and "Out of scope: Any code"
- **Would collide with:** `docs/view-stream.md` is also written by `story:view-now-panel` (adds the title and `now` elements to the `answer` entry) and by `story:view-controls-and-changes` (adds control and button elements). Both read their own bodies, "Surfaces written". — cited
- **Would collide with:** `docs/design/live-view.md` is written by no other story's "Surfaces written" line. The other view stories only read it. — cited
- **Would collide with:** the two `docs/view-stream.md` writers sit downstream of this story by `depends_on` edges, so they cannot run in the same wave. The chain is `view-http-server` → `view-decisions`, `view-surfd-serves` → `view-http-server`, `view-first-page` → `view-surfd-serves`, `view-detail-panel` → `view-first-page`, `view-now-panel` → `view-detail-panel`, and `view-controls-and-changes` → `view-now-panel`. — inferred, from reading the `depends_on` lines of those bodies. I did not run `aep plan artifact graph`.
- **Would collide with:** any other unit touching `docs/` in the same wave, which is nothing, because no other draft story names these two files. This unit touches no crate, `spec/`, `generated/`, `conformance/` or root file, so it does not collide with any code unit. — inferred
- **Not yet existing when it starts:** the drafted body of `story:agent-session-events` (delivered by that story, already drafted) for the kept entry and its bound. It needs no code and no `spec/domains/agent.yaml`. The Domain relations only name `surf.agent.Session → surf.agent.Action` as coming from `story:agent-page-path-spec`, and that is not needed to write the documents.
- **Not yet existing when it starts:** the tooling the measurements use is not in the repository (`view-tests/`, `crates/view-fixture`, `taskfiles/`). The story only measures it on this machine, in `.out/`.
- **Confidence:** high — the story names both files in "Surfaces written" and in Acceptance, and no `crates/`, `spec/` or root file is involved.

