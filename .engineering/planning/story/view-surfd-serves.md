---
format: aep.planning-md/3
id: story:view-surfd-serves
kind: story
status: active
title: surfd serves its sessions, their actions, events and page line on the view routes
summary: surfd starts the view server on 127.0.0.1, draws the view token, writes view-url, and answers /api/sessions, /api/sessions/<id> and /events from its session list and kept logs as docs/view-stream.md says.
relations:
- decomposes: epic:live-view
- depends_on: story:view-http-server
- depends_on: story:agent-surfd-supervisor
- depends_on: story:agent-session-events
- depends_on: story:agent-session-process
- depends_on: story:agent-secret-primitives
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:15Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:15Z", actor: "human:timo", revision: 4}
---
## Context

Design section 12, "How it gets its data": each session process publishes what its system
published and the rows of its action log to `surfd`, which keeps a bounded log per session and
serves `GET /api/sessions`, `GET /api/sessions/<id>` and `GET /events`. `story:view-http-server`
gives the server over a source trait; `docs/view-stream.md` (`story:view-decisions`) gives the JSON
and the stream. This story connects them to `surfd`'s sessions. After it the routes answer with
real sessions; the page is `story:view-first-page`.

The decisions of `docs/plan/agent-plan-revision-2.md` this story follows, in their words:

- D19: for each action, the action entry, then the events of that action in the order the
  components published them, then the answer entry. An entry belongs to the action entry it
  follows.
- D20: the answer travels as its lines; `surfd` keeps the lines of each answer as the answer entry
  and does not read them when it keeps them; the view routes call `surf_answer::parse` to write
  JSON. This story writes no parser of answer text.
- D21: the answer entry of `open` omits the `token` line; a named case finds the token in no
  route's answer.
- D22: a field of bytes is written by the session process as its length, `<n> bytes`; a
  password's value is its length, as the answer prints it; every other control value is kept as
  the answer prints it.
- D23: `surfd` stamps every entry with a number that increases across all sessions, beside the
  per-session number, and offers a reader from a number: entries after it, or the statement that
  the number is older than what is kept. `/events` and `Last-Event-ID` use them.
- D24: beside the bound per session, at most 64 MiB of entries over all sessions; above it the
  logs of ended sessions go first, the longest-ended first, and such a session's detail says that
  its log was dropped.
- D25: the view token is drawn with `surf-secret` (`Token::draw`, `to_hex`); this story writes no
  random source and no comparison.
- D26: an address, a title and an event field arrive quoted by `surf-answer`'s quoting function,
  and `surfd` keeps them so; it reads them only through `surf_answer::unquote`, in the view routes,
  which then write the exact value as a JSON string.
- D16 and D30: a new task goes to `taskfiles/<name>.yml`; the root `Taskfile.yml` is the
  coordinator's.

Needs from `epic:agent-cli-target` (edges set):

- `story:agent-surfd-supervisor`: the `surfd` library and binary, its session list
  `surfd::Sessions` (identity, state `open`, `closed` or `died`, opened, last action, ended,
  address and title as the session reported them, quoted), its start, where the view server
  starts, and its end, where `view-url` is removed.
- `story:agent-session-events`: the kept log and its reader as D19 to D24 state them.
- `story:agent-session-process`: sessions that `surf-agent open`, `go` and `close` drive.
- `story:agent-answer-format` (through `agent-session-process`): `surf_answer::parse` and
  `surf_answer::unquote`, held to `render` by a round trip (D20).
- `story:agent-secret-primitives`: `surf-secret`'s `Token`.

## Domain relations

- `surf.agent.Session → surf.agent.Action`, one to many, owned by the session: introduced into
  `spec/domains/agent.yaml` by `story:agent-page-path-spec` (design section 6, accepted in
  `approval-record:agent-design-accepted-2026-11-21`); this story reads the rows, it writes none.
- `surf.agent.Session → surf.shell.Tab` and `→ surf.shell.Window`: relations `tab` and `window`,
  `references`, `one`, in the draft `ess/1` document `spec/agent-session:spec/domains/agent.yaml`
  (landed on `main` by `story:agent-session-spec`). Not shown by the view.
- Navigation and request share one identity: `spec/domains/navigation.yaml`, `Navigation`
  identity `navigation_id: surf.net.RequestId`.

## Change

- `crates/view/src/model.rs`: the view's own types for a session summary, a session detail (with
  the dropped count and whether the log was dropped) and the three entry kinds of
  `docs/view-stream.md`, each entry with its per-session number and its number across all
  sessions. No token field exists in any of them. `crates/view/src/json.rs`: one writer for them,
  by the string rule of that document. `crates/view/src/lib.rs`: the two modules.
- `crates/surfd/src/view.rs`:
  - the source trait of `surf-view` over `surfd::Sessions` and the reader of the kept log: the
    session list, one session, and the reader from a number of D23 for `/events`;
  - the conversion of each kept entry into the view's types at the time a route answers: an
    action entry as its sequence number, command and argument; an event entry as its name and each
    field through `surf_answer::unquote` (a byte field is then the text `<n> bytes`, D22); an
    answer entry through `surf_answer::parse`, of which the page line and the path are written
    here; the address and title of `surfd::Sessions` through `surf_answer::unquote`. A value that
    `unquote` or `parse` refuses is written as JSON `null` with the entry kept, and counted in the
    detail (one `unreadable` count);
  - at start: a listener on 127.0.0.1 port 0; the view token drawn with `surf-secret`;
    `http://127.0.0.1:<port>/#<token>` printed on standard error and written to
    `$XDG_RUNTIME_DIR/surf/view-url` with mode 0600, removed at exit; the server started on a
    thread of its own.
  - `crates/surfd/src/main.rs` (or the start function `story:agent-surfd-supervisor` names): one
    call. `crates/surfd/Cargo.toml`: the dependencies on `surf-view` and `surf-secret`.
- The check of item 2 of `docs/design/live-view.md` as the task file
  `taskfiles/view-stream-check.yml` (task `view-stream-check`). The root `Taskfile.yml` lines are
  handed to the coordinator (below).
- `crates/view/tests/json.rs` and `crates/view/tests/golden/`, the examples of
  `docs/view-stream.md` as golden files:
  1. the session list with an open and a closed session;
  2. a detail whose page is committed;
  3. a detail whose page failed, with its reason;
  4. a detail before the first page (`page none`);
  5. an `action` entry; 6. an `event` entry; 7. an `answer` entry: each byte for byte;
  8. a detail whose log was dropped (D24): its entries empty, its dropped flag set, byte for byte;
  9. the string matrix: `"`, `\`, U+0000, U+001F, U+007F, U+2028, U+2029, `<`, `>`, `&` and a
     letter outside ASCII, each in a title, an address and an event field: a JSON reader written
     in the test reads back the exact value, and the output holds no raw `<`, `>`, `&` or control
     character.
- `crates/surfd/tests/view.rs`, against a page server the test starts on `std::net::TcpListener`
  and a `surfd` the test starts with a private `XDG_RUNTIME_DIR`, driven by the built
  `surf-agent`; every case asserts what a route answers (decision D29):
  10. `view-url` exists with mode 0600 and holds `http://127.0.0.1:<port>/#` and 64 hexadecimal
      digits, and the port answers on 127.0.0.1;
  11. two sessions open: `/api/sessions` with the token lists both with state `open` and their
      addresses;
  12. a session after `close` is still listed, with state `closed`;
  13. after `open <a>` and `go <b>`, the detail holds, in this order (D19): the `open` action
      entry, the events of `open` in publish order, the `open` answer entry whose page line names
      `<a>`; then the `go` action entry, the events of `go`, the `go` answer entry whose page line
      names `<b>`; per-session numbers consecutive, numbers across sessions increasing;
  14. a stream opened before a `go` receives, in this order (D19): the `go` action entry, its
      events in the order the detail lists them afterwards, its answer entry; each `id:` is the
      entry's number across all sessions;
  15. a `go` to a port nothing listens on: the page line says failed with reason `Unreachable`,
      in the detail and on the stream;
  16. (D21) the `open` answer entry of each session has no `token` line, and the 64 digits of
      neither session's token occur in any byte of `/api/sessions`, of either detail, or of the
      stream during cases 13 to 15;
  17. a stream reconnecting with `Last-Event-ID` set to the number of the `go` action entry of
      case 14 receives that `go`'s events and answer and nothing before; with a number older
      than what is kept (the per-session bound lowered to 10 through the option of
      `story:agent-session-events`, then 25 entries) it receives `reset` first;
  18. (D24) with the bound over all sessions lowered through the option of
      `story:agent-session-events`, two sessions closed one after the other and a third doing
      `go` until the bound is passed: the detail of the first-closed session says its log was
      dropped and holds no entry, and the third session's detail is whole;
  19. (D26) a page whose title holds `"`, a backslash and `é`, and an address holding `"`: the
      session list, the detail's page line and the `url` field of `surf.net.RequestStarted` hold
      the exact values as JSON strings, with no `\x` escape of the link left in them;
  20. `surfd` stopped and started again: a new token; the old one answers 401; `view-url` holds
      the new one;
  21. `task view-stream-check` passes.

Surfaces written: `crates/view/src/model.rs`, `crates/view/src/json.rs`, `crates/view/src/lib.rs`
(module lines), `crates/view/tests/json.rs`, `crates/view/tests/golden/`, `crates/surfd/src/view.rs`,
`crates/surfd/src/main.rs` (one call), `crates/surfd/Cargo.toml`, `crates/surfd/tests/view.rs`,
`taskfiles/view-stream-check.yml`.

## For Taskfile.yml (decisions D16 and D30, written by the coordinator)

- The include of `taskfiles/view-stream-check.yml`, and `view-stream-check` in `check` after
  `test`.

## For AGENTS.md (decision D7)

- "Layout", the `crates/surfd` line: "… and serves the live view: `crates/view` over its sessions
  and kept logs, on 127.0.0.1, address and token in `$XDG_RUNTIME_DIR/surf/view-url`; it reads a
  session's text only through `surf_answer::parse` and `surf_answer::unquote`."
- "Commands": "`task view-stream-check`: the event names of `docs/view-stream.md` against the
  specification." (or what item 2 decided).

## Acceptance

`cargo test -p surf-view --test json` passes with cases 1 to 9 and `cargo test -p surfd --test
view` passes with cases 10 to 21, each a test named after its case, and `slot task check` exits 0
with `view-stream-check` among its steps once the coordinator has added its include.

## Out of scope

The page (`story:view-first-page`); the `now` and `changed` blocks of the answer
(`story:view-now-panel`, `story:view-controls-and-changes`); how sessions publish their entries and
how the log is bounded (`story:agent-session-events`); a parser of answer text (`surf-answer`); a
flag to turn the view off.
