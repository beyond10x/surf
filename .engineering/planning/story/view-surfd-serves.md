---
format: aep.planning-md/3
id: story:view-surfd-serves
kind: story
status: implemented
title: surfd serves its sessions, their actions, events and page line on the view routes
summary: surfd starts the view server on 127.0.0.1, draws the view token, writes view-url, and answers /api/sessions, /api/sessions/<id> and /events from its session list and kept logs as docs/view-stream.md says.
relations:
- decomposes: epic:live-view
- depends_on: story:view-http-server
- depends_on: story:agent-surfd-supervisor
- depends_on: story:agent-session-events
- depends_on: story:agent-session-process
- depends_on: story:agent-secret-primitives
- depends_on: story:view-page-prototype
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/session/tests/view.rs
- confidence: cited
  path: crates/surfd/Cargo.toml
- confidence: cited
  path: crates/surfd/src/daemon.rs
- confidence: cited
  path: crates/surfd/src/directory.rs
- confidence: cited
  path: crates/surfd/src/lib.rs
- confidence: cited
  path: crates/surfd/src/link.rs
- confidence: cited
  path: crates/surfd/src/log.rs
- confidence: cited
  path: crates/surfd/src/main.rs
- confidence: cited
  path: crates/surfd/src/view.rs
- confidence: cited
  path: crates/surfd/tests/built.rs
- confidence: cited
  path: crates/surfd/tests/in_process.rs
- confidence: cited
  path: crates/surfd/tests/log.rs
- confidence: cited
  path: crates/surfd/tests/view.rs
- confidence: cited
  path: crates/view/src/json.rs
- confidence: cited
  path: crates/view/src/lib.rs
- confidence: cited
  path: crates/view/src/model.rs
- confidence: cited
  path: crates/view/tests/json.rs
- confidence: cited
  path: docs/view-stream.md
- confidence: cited
  path: taskfiles/view-stream-check.yml
revision: 28
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:15Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:15Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-21T14:59:44Z", actor: "human:operator", revision: 28, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

Design section 12, "How it gets its data": each session process publishes what its system
published and the rows of its action log to `surfd`, which keeps a bounded log per session and
serves `GET /api/sessions`, `GET /api/sessions/<id>` and `GET /events`. `story:view-http-server`
gives the server over a source trait (`surf_view::Source`, `Reader`, `Item`, `TooOld` in
`crates/view/src/http.rs` and `sse.rs`; it takes bodies as bytes and writes no JSON);
`docs/view-stream.md` (`story:view-decisions`, its examples rewritten by
`story:agent-session-events` to entry numbers 1 to 38) gives the JSON and the stream. This story
connects them to `surfd`'s sessions and kept log. After it the routes answer with real sessions;
the page is `story:view-first-page`.

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
- D29: cases that assert the list run `surfd` in the test process; cases on the built `surfd`
  assert only what an answer prints.
- D16 and D30: a new task goes to `taskfiles/<name>.yml`; the root `Taskfile.yml` is the
  coordinator's.

What the tree gives, and what it does not (read at `d437dc6`):

- `surfd::Daemon` (`crates/surfd/src/daemon.rs`): `sessions()`, `session_entries(id)` (`None`
  for an identity not listed), `subscribe_after(n)`, `entries_after(n)`, `newest_entry()`; each
  takes the table lock on its own. `begin` marks the place of this story's start: after
  `surfd.sock` is bound, before `ready` is written (`daemon.rs`, the comment after
  `UnixListener::bind`; `story:agent-surfd-supervisor`: "`story:view-surfd-serves` adds its one
  call there"). `main.rs` calls nothing but `Daemon::start`.
- `surfd::log` (`crates/surfd/src/log.rs`): `SessionEntries { entries, dropped, log_dropped }`,
  `Subscription` (`next_within`, `try_next`) with a backlog copied at subscription and a live
  queue of `Bounds::reader_queue` (1,024) after which the reader is disconnected, `OlderThanKept`.
  `dropped` counts entries dropped by any bound, the whole log included when it was dropped
  (`log.rs`, `SessionEntries::dropped`).
- Not given, per route: `/api/sessions` needs each session's newest action entry (`last_action_at`)
  and newest answer entry (`page`), and the log gives them only by copying the session's whole
  log; and `newest` is read under another lock than the list. `/api/sessions/<id>` is given whole.
  `/events` gets entries only: nothing tells a reader that a session was listed or ended, a
  replay is not bounded, and an entry kept for a session whose log was dropped whole is kept
  (below).
- The real session process reports no title (`crates/session/src/lib.rs`: `title: None`); only
  the stand-in session of `crates/surfd/examples/stand_in_session.rs` reports one (its control
  file `title`). The real `surf-session` binary is a package that depends on `surfd`, so a test
  with real events lives in `crates/session/tests/`, where `surfd_in_process` and `PageServer`
  are; a test with the stand-in lives in `crates/surfd/tests/`.

Taken from `docs/plan/agent-wave-5.md`, "Findings":

- Not attacked by the adversary: an entry of `close` kept after the log ended the session. The
  session process exits by itself after writing its `close` answer
  (`crates/session/src/process.rs`, `serve` returns `closed`), and `supervise` reaps it every
  10 ms (`REAP_POLL`), so `Log::end` can run before `settle` keeps the `close` entries. Kept
  after `end` they are kept like any other; kept after the bound over all sessions dropped that
  log whole, they would make a detail with `log_dropped` `true` and entries, against
  `docs/view-stream.md`. This story drops such an entry at once (case 23) and the `session`
  items do not depend on the order (case 12).
- Not attacked: the backlog of a subscriber between two reads. The backlog is a copy of every
  kept entry above the number, unbounded; the server's queue per stream is 256 entries
  (`Limits::queue_entries`) and is filled as fast as the reader yields, so a replay longer than
  that closes the stream and the page resumes from the same place again. This story answers a
  replay of more than `queue_entries` numbers with `reset` (case 17), and a disconnected
  subscription ends the stream (case 25).
- `dropped`: `docs/view-stream.md` line 172 says "dropped by the bound per session"; `log.rs`
  counts every bound and the whole dropped log, and the document's own example (`"dropped":21`
  for entries 18 to 38 of a log dropped whole) counts so. `log.rs` is right; this story corrects
  the document.
- A late answer: while it is pending the session's turn is taken and none of that action's
  entries exist yet: they travel with the answer. The view shows the session as before the
  request (`open`, the previous `last_action_at` and `page`), adds no member for it, and shows
  the entries when the late answer arrives, with the time they were kept (case 24).

## Domain relations

- `surf.agent.Session → surf.agent.Action`, one to many, owned by the session: introduced into
  `spec/domains/agent.yaml` by `story:agent-page-path-spec`; this story reads the rows, it writes
  none.
- `surf.agent.Session → surf.shell.Tab` and `→ surf.shell.Window`: relations `tab` and `window`,
  `references`, `one`, in `spec/domains/agent.yaml` (`story:agent-session-spec`). Not shown by
  the view.
- Navigation and request share one identity: `spec/domains/navigation.yaml`, `Navigation`
  identity `navigation_id: surf.net.RequestId`.

## Change

Every shape, member order and string rule is the one of `docs/view-stream.md`; where this text
and the document differ, the document holds, once corrected as below.

- `docs/view-stream.md`, corrected by this story (no ```` ```http ```` block is changed or added:
  `crates/view/tests/http.rs` case 24 compares each and counts 5):
  - the one-value string example (line 138) holds a raw `"`, `\`, `<`, `>`, `&` and U+2028,
    against its own rule; it becomes `"a\u0022b\u005cc\u003cd\u003e\u0026e\u000a\u2028é"`;
  - `dropped` (line 172): "entries of this session dropped by any bound: one by one, the oldest
    first, and every kept one when its log was dropped whole";
  - the detail whose log was dropped (lines 388 to 392): `last_action_at` and `page` are `null`,
    as the summary's rule gives them with no entry kept; 264 bytes (was 341);
  - "Where a stream starts": a `Last-Event-ID` more than 256 numbers below the newest
    (`Limits::queue_entries`) is answered with `reset`, as one older than what is kept;
  - "The entries": an action whose answer has not arrived has no entry; its entries arrive with
    its answer, a late one included (`LATE_ANSWER_TIMEOUT`, 600 s), with the time `surfd` kept
    them; a session killed before it answered ends `died` and that action has no entry.
- `crates/view/src/model.rs`: the view's own types for the page line (`none`, `loaded`,
  `failed`), a session summary (`id`, `state`, `address`, `title`, `opened_at`,
  `last_action_at`, `ended_at`, `page`), the session list (`newest`, `sessions`), a session
  detail (`session`, `dropped`, `log_dropped`, `unreadable`, `entries`), the three entry kinds,
  each entry with `number`, `session`, `index`, `kept_at` and `kind`, an `event` entry with
  `name`, `unmapped` and `fields`, and the `reset` data. No token field in any of them.
  `crates/view/src/json.rs`: one writer for them, by the document's rules (no whitespace outside
  strings, members in the document's order, absent values `null`, numbers in decimal; `"`, `\`,
  every character below U+0020, U+007F to U+009F, U+2028, U+2029, `<`, `>` and `&` as `\u` and
  four lower-case hexadecimal digits). Every event field value is a JSON string
  (`"status":"200"`), the page line's `status` a number. `crates/view/src/lib.rs`: the two
  modules and their exports. The example `prototype` keeps its own writer.
- `crates/surfd/src/link.rs`: the reading of an action line (sequence, command, quoted argument)
  and of an event line (name, `unmapped`, each field and its quoted value or none), exposed
  beside the grammar check that already walks them, so that one definition reads what `surfd`
  checks. `crates/surfd/src/log.rs`: (1) an entry kept for a session whose log was dropped whole
  is dropped at once: counted in `dropped`, given its number, handed to live readers as every
  entry is, not kept; (2) the newest kept action entry and answer entry of a session, without a
  copy of its log.
- `crates/surfd/src/view.rs`, the `Source` of `surf-view` over the daemon's shared state:
  - `/api/sessions`: `newest` and the list read in one hold of the table lock; per session the
    summary from `surfd::Sessions` (address and title through `surf_answer::unquote`,
    `opened_at`, `ended_at`) and the log (`last_action_at`, the `kept_at` of the newest action
    entry; `page`, the page line of the newest answer entry; `null` when none is kept).
  - `/api/sessions/<id>`: the summary and `Daemon::session_entries` of a listed session, 404
    otherwise; each entry converted when the route answers: an action entry as its sequence,
    command and argument; an event entry as its name, `unmapped` and each field through
    `surf_answer::unquote` (absent: `null`); an answer entry through `surf_answer::parse`, its
    page line and path. A value that `unquote` or `parse` refuses is `null`, the entry kept and
    counted in `unreadable`.
  - `/events`: `read_from(n)` answers `TooOld` when the log says `OlderThanKept` or when the
    newest number is more than `Limits::queue_entries` above `n`; otherwise the subscription's
    entries, after each answer entry a `session` item of its session with the same number, and
    a `session` item with the newest number when the reader finds a session listed that it did
    not know or one whose state changed to `closed` or `died` (it compares the list at each
    wait, so no change of `daemon.rs` sends it); `session` items are never replayed. A
    subscription that is disconnected (its queue of 1,024 full) ends the stream (`Next::End`).
- `crates/surfd/src/daemon.rs`: `Config` gains `view_limits` (`surf_view::Limits::default()`)
  and `view_line` (where the view line goes, `None` by default, like `start_line`); in `begin`,
  at the marked place: a listener on 127.0.0.1 port 0, the token drawn with `Token::draw`,
  `surf_view::serve` with `page_files()`, the line `http://127.0.0.1:<port>/#<token>` and a line
  feed written to `view_line` and to `$XDG_RUNTIME_DIR/surf/view-url`. `Daemon` holds the
  server; its drop stops the server and removes `view-url` before the lock is released. A
  failure to listen or to write `view-url` refuses the start (`refused cannot …` naming the
  path or the address, never the token), with `surfd.sock` removed and nothing left listening.
  A reader of the view is not activity for the idle exit: `surfd` still exits after
  `--idle-exit` with no session and no connection on `surfd.sock`, which ends every stream
  (accepted: a page reconnects for ever, so it would otherwise keep `surfd` alive while a tab is
  open; `story:agent-surfd-supervisor` counts sessions only). A `surfd` killed by a signal leaves
  `view-url`; the next start replaces it.
- `crates/surfd/src/directory.rs`: `VIEW_URL` (`view-url`), written by removing a regular file of
  that name and creating it with `create_new` and mode 0600, the mode set to 0600 after creation
  whatever the umask (the pattern of `prepare`); `remove_stale` already refuses a symbolic link
  of any name under `surf/`.
- `crates/surfd/src/main.rs`: `view_line` set to standard error. `surf-agent` starts `surfd`
  with standard error discarded (`crates/agent/src/client.rs`), so nothing reaches the agent.
- `crates/surfd/src/lib.rs`: `mod view;`; "Files" names `view-url`, "Bounds" the view's limits.
- `crates/surfd/Cargo.toml`: the dependency on `surf-view` (`surf-secret` is one already).
  `surf-view` depends on `surf-secret` only, so there is no cycle; `Cargo.lock` gains the line
  `"surf-view"` under `surfd` and still holds 21 packages, none from outside.
- The tests that list `surf/` gain `view-url`: `crates/surfd/tests/built.rs` cases 05, 19 and 20
  (`["surfd.lock", "surfd.sock"]`) and `crates/surfd/tests/in_process.rs` case 09
  (`ONLY_DAEMON`); each turns red otherwise.
- `taskfiles/view-stream-check.yml`, task `view-stream-check`, as item 2 of
  `docs/design/live-view.md` decides, with `dir: '{{.TASKFILE_DIR}}/..'` as the other included
  files have, and a variable `VIEW_STREAM_DOC` (default `docs/view-stream.md`): it runs
  `ess generate --path spec --kind asyncapi --out .out/view-stream-check/`, reads the one fenced
  block under "Event names", and fails with one line
  `docs/view-stream.md: <name>: no message in the AsyncAPI projection` per name that no line
  `    x-ess-event: <name>` (`grep -x`) holds; passing, it prints
  `view-stream-check: <n> event name(s), each a message of the AsyncAPI projection`. Measured at
  `d437dc6`: `ess generate` exits 0 with 6 artifacts, and each of the block's 17 names is such a
  line.

Tests. The examples are read from the fenced blocks of `docs/view-stream.md`, as
`crates/view/tests/http.rs` case 24 reads its own, so no copy can drift; each view case reads
the port and token from `view-url` and asserts what a route answers.

- `crates/view/tests/json.rs`, the writer from model values:
  1. the session list: the `GET /api/sessions` example body, 603 bytes (`newest` 38, an open and
     a closed session);
  2. a detail whose page is committed: the `GET /api/sessions/<id>` example body, 5,255 bytes;
  3. a detail whose page failed, with its reason, and 4. one before the first page
     (`{"kind":"none"}`), each written from the document's rules;
  5. the `action` entry 7 (182 bytes), 6. the `event` entry 13 (325 bytes; `"status":"200"`,
     `"body":"5000 bytes"`), 7. the `answer` entry 17 (393 bytes; `"status":200`);
  8. the detail whose log was dropped, as corrected (264 bytes);
  9. the string matrix: `"`, `\`, U+0000, U+001F, U+007F, U+0080, U+009F, U+2028, U+2029, `<`,
     `>`, `&` and `é`, each in a title, an address and an event field: a JSON reader written in
     the test reads back the exact value, and the output holds no raw `<`, `>`, `&` and no
     character below U+0020 or from U+007F to U+009F; the one-value example as corrected, byte
     for byte;
  22. the `data:` of the `session` event example (269 bytes) and of the `reset` example
     (`{"newest":38}`).
- `crates/surfd/src/view.rs`, unit tests: 25. a subscription of a log with `reader_queue` 2 and
  three entries kept unread yields two entry items and then `Next::End`; 26. an answer entry that
  `surf_answer::parse` refuses is written with `page` and `path` `null`, kept, and the detail's
  `unreadable` is 1; an event field with no value is `null`; an `unmapped` event has
  `"fields":{}`.
- `crates/surfd/tests/log.rs`: 23. with the bound over all sessions lowered, A ended and its log
  dropped whole by B's entries, an entry then kept for A leaves A's `entries` empty,
  `log_dropped` `true` and `dropped` one higher, and `entries_after` below its number answers
  `OlderThanKept`.
- `crates/surfd/tests/view.rs`, `surfd` in the test process (`common::start`) with the stand-in
  session, driven by the wire (`common::call`):
  10. `view-url` is a regular file of mode 0600 holding `http://127.0.0.1:<port>/#`, 64
      lower-case hexadecimal digits and a line feed, the same bytes `view_line` received; the
      port answers `/` on 127.0.0.1; after the daemon is dropped `view-url` is gone;
  11. two sessions open: `/api/sessions` lists both `open` with their addresses, and its
      `newest` equals the highest `number` in the two details;
  12. a session after `close` is listed `closed` with `ended_at` set; its detail holds the
      `close` action entry and answer entry; a stream open during the `close` receives, as the
      last `session` event of that session, one with state `closed`;
  17. a stream with `Last-Event-ID` set to the number of a `go` action entry receives that
      `go`'s answer entry and the later entries, nothing at or below the number, and no
      `session` event before them; with `log.entries_per_session` 10 and 25 entries kept, a
      number older than what is kept receives `event: reset` with `{"newest":<n>}` first; with
      `view_limits.queue_entries` 8, a number 9 below the newest receives `reset` first;
  18. (D24) with `log.bytes_total` lowered, two sessions closed one after the other and a third
      doing `go` until the bound is passed: the detail of the first-closed session has
      `log_dropped` `true`, `"entries":[]`, `dropped` its former entry count, `last_action_at`
      and `page` `null`; the third session's detail is whole;
  19. (D26, title) the stand-in's `title` holding `"`, a backslash and `é`: the session list's
      `title` is the exact value;
  20. the daemon dropped and started again on the same directory: `view-url` holds a new token
      and port; the old token sent to the new port answers 401;
  24. a late answer: `session_answer_timeout` 1 s, the stand-in in `stall` held
      (`TestDir::hold`); while the call is refused `session-timeout`, `/api/sessions` lists the
      session `open` and its detail is unchanged; once the hold is dropped, the detail gains
      that `go`'s action and answer entries with numbers above every entry kept before, and an
      open stream receives them and then a `session` event with the answer entry's number;
  27. the built `surfd` with `--idle-exit 1` and a stream open on `/events`, no session: `surfd`
      exits 0 within 5 s, the stream's connection reaches its end, and `view-url` is gone.
- `crates/session/tests/view.rs`, `surfd_in_process` with the built `surf-session`, a
  `PageServer`, driven by the built `surf-agent`; no count of a command's events is written in a
  case, each is compared with `Daemon::session_entries`:
  13. after `open <a>` and `go <b>`, the detail lists, in this order (D19): the action entry
      `1 open`, its events, the action entry `2 go "<a>"`, its events, the one answer entry of
      `open` (page line `<a>`, path `1 open` and `2 go "<a>"`), the action entry `3 go "<b>"`,
      its events, the answer entry of `go` (page line `<b>`); `index` 1 upward without a gap,
      `number` increasing, the same order and numbers as the log;
  14. a stream opened before a `go` receives that `go`'s action entry, its events in the order
      the detail lists them, its answer entry, then a `session` event of that session with the
      answer entry's `id:`; each `id:` is the entry's `number`;
  15. a `go` to a port nothing listens on: the page line is `failed` with reason `Unreachable`,
      in the detail and on the stream;
  16. (D21) the 64 digits of neither session's token occur in any byte of `/api/sessions`, of
      either detail, or of the stream during cases 13 to 15;
  21. (D26, address) a `go` to an address holding `"`, a backslash, `é` and a line feed: the
      session list's `address`, the detail's page line `address` and the `url` field of
      `surf.net.RequestStarted` are the exact value, with no `\x` escape of the link left.

## For Taskfile.yml (decisions D16 and D30, written by the coordinator)

- The include of `taskfiles/view-stream-check.yml` (`flatten: true`, `optional: true`, as the
  other includes), and `view-stream-check` in `check` after `test`.

## For AGENTS.md (decision D7)

- "Layout", the `crates/surfd` line, which says "nothing serves it yet": "… and serves the live
  view: `crates/view` over its sessions and kept logs, on 127.0.0.1, address and token in
  `$XDG_RUNTIME_DIR/surf/view-url` (mode 0600, removed at exit); it reads a session's text only
  through `surf_answer::parse` and `surf_answer::unquote`." The `crates/view` line, which says it
  shows example data only: the JSON model and writer, and its test counts.
- "Commands": "`task view-stream-check`: the event names of `docs/view-stream.md` against the
  AsyncAPI projection of `spec/`."

## Acceptance

- `cargo test -p surf-view --test json` passes with cases 1 to 9 and 22, each a test named after
  its case; `cargo test -p surf-view --test http` passes unchanged (case 24 compares 5 examples).
- `cargo test -p surfd` passes: the unit tests 25 and 26 of `view.rs`, case 23 in `--test log`,
  cases 10, 11, 12, 17, 18, 19, 20, 24 and 27 in `--test view`, and `--test built` and
  `--test in_process` with `view-url` in their lists.
- `cargo test -p surf-session --test view` passes with cases 13, 14, 15, 16 and 21.
- `task --taskfile taskfiles/view-stream-check.yml view-stream-check` exits 0 and prints
  `view-stream-check: 17 event name(s), each a message of the AsyncAPI projection`; with
  `VIEW_STREAM_DOC` naming a copy whose block adds `surf.agent.ActionTaken`, it exits 1 and
  prints `<the copy>: surf.agent.ActionTaken: no message in the AsyncAPI projection`.
- `task lock-check` prints `Cargo.lock: 21 package(s), none from outside this repository`.
- `slot task check` exits 0 with `view-stream-check` among its steps once the coordinator has
  added its include.

## Out of scope

The page (`story:view-first-page`); the `now` and `changed` blocks of the answer
(`story:view-now-panel`, `story:view-controls-and-changes`); how sessions publish their entries
and the numbers of the bounds (`story:agent-session-events`); a parser of answer text
(`surf-answer`); a flag to turn the view off; a member that marks a pending late answer; a test
of a late answer that never comes (backlog, with the stand-in session's next change); the event
names `story:html-document-spec` adds (the coordinator adds them to the "Event names" block after
both merge).
