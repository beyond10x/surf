---
format: aep.planning-md/3
id: story:agent-session-events
kind: story
status: active
title: Session processes publish their actions, events and answers to surfd, which keeps a bounded log
summary: Each session process sends, per action, its action, events and answer lines to surfd, which stamps them with a per-session number and one across sessions, keeps at most 10,000 entries or 16 MiB per session and 64 MiB over all, and hands them to in-process readers from a number.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-surfd-supervisor
- depends_on: story:agent-session-process
- informed_by: epic:live-view
- depends_on: story:agent-redirect-spec
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 6}
---
## Context

Design section 12, "How it gets its data": each session process publishes what its system
published (`System::published()`, in order) and the rows of its action log to `surfd`, which keeps
a bounded log per session; design section 3: it publishes them as they happen. The readers are
the routes of the live view (`epic:live-view`), in the `surfd` process; this story gives them the
kept log, a reader from a number and a live feed, and nothing on HTTP.

`Browser::run` (`crates/surf`) pumps the system to rest before it returns, so a command is the
smallest step at which events exist; "as they happen" is per command. The entries travel in the
link answer of that command (`surfd::link`, story `agent-surfd-supervisor`), before its lines, so
`surfd` holds them before it relays the answer and needs no second channel.

The decisions of `docs/plan/agent-plan-revision-1.md` (D17) and `docs/plan/agent-plan-revision-2.md`
(D19 to D24, D26, D27) this story carries, in their words where another epic must say the same:

- **Three kinds of entry** (D17): `action` (an action row: its sequence number, command and
  argument as the `path` line prints it), `event` (the qualified name the specification gives,
  such as `surf.navigation.NavigationStarted`, and its fields in the order the `ess/1` document
  declares them) and `answer` (the answer of the action).
- **One order** (D19): for each action, the action entry, then the events of that action in the
  order the components published them, then the answer entry. An entry belongs to the action
  entry it follows. A command that records no action (`look`, a refused command) adds no entry.
- **The answer travels as its lines** (D20): the answer entry is the answer lines the session
  rendered for the client, as the link carries them; there is no second encoding. `surfd` keeps
  them and does not read them when it keeps them; the view routes call `surf_answer::parse`
  (`story:agent-answer-format`) to write JSON.
- **The token is not kept** (D21): the answer entry of `open` omits the `token` line.
- **What the log holds of a value** (D22): a field of bytes (a field of the specification type
  `Bytes`: today `surf.net.ResponseReceived.body` and `surf.navigation.NavigationCommitted.body`;
  later a request body) is
  written by the session process as its length, `<n> bytes`, never as the bytes. A password
  value is its length, `<n> characters`. Every other control value is kept as the answer prints
  it.
- **Text stays quoted** (D26): every value from a page, a server or an address is written by
  `surf-answer`'s quoting function and kept quoted; it is read only through
  `surf_answer::unquote`, in the view routes.
- **One identity across sessions** (D23): `surfd` stamps every entry it keeps with a number that
  increases across all sessions, beside the per-session number, and offers a reader from a
  number: entries after it, or the statement that the number is older than what is kept.
- **The bounds** (D24, and decided here for the numbers per session): per session at most 10,000
  entries or 16 MiB, the oldest dropped first and counted; over all sessions at most 64 MiB.
  Above it, the logs of ended sessions go first, the longest-ended first, and such a session's
  detail says that its log was dropped. If the open sessions alone are above it, the oldest
  entries by the number across sessions go next (decided here). A live reader has a queue of
  1,024 entries and is disconnected when it is full, so a slow reader never holds `surfd` up
  (decided here).
- **Who maps an event** (D27): this story depends on `story:agent-redirect-spec` and maps every
  event and field on `main` when it starts. A story that adds an event or an event field after
  that maps it in `crates/session`, with a named case, and depends on this story: the stories of
  `Follow` (`agent-follow-links`), of `GoBack` and `GoForward` (`agent-back-forward`), of the form
  commands (`agent-fill-pick`, `agent-press-submit`), and `story:html-document-spec` (its case
  `document_events_map_with_their_fields`). Until a story maps an event, it is kept with its name,
  the mark `unmapped` and no field, so nothing it carries leaks and no build breaks.
- `surfd`, not the session process, stamps the session, both numbers and the time, so a broken
  session process cannot write into another session or choose a number (decided here).

## Change

- `crates/session`: after each command that records an action, the entries in the order above:
  the action entry from the view `SessionLog`; one event entry for every event its system
  published since the last command, read in order through `Browser::system().published()` with a
  cursor of its own; the answer entry, the rendered lines without a `token` line. One function
  maps a `SystemEvent` to an event entry, each field value quoted by `surf-answer`; the rule is by
  field type: every field of the specification type `Bytes` (generated `Vec<u8>`; on `main` the
  `body` of `surf.net.ResponseReceived` and the `body` of `surf.navigation.NavigationCommitted`)
  is written as the text `<n> bytes`, which reaches the log
  quoted by `surf-answer`'s quoting function like every other event field (D26); an event it does not map is
  written with `SystemEvent::name()` (generated), the mark `unmapped` and no field. The mapping
  test (case 17) holds a list of the variants it covers; each story that maps an event adds it.
- `crates/surfd`: the entries of a link answer, checked against the link rules; one log per
  session; each entry stamped with the session, its per-session number from 1, its number across
  sessions from 1 for each run of `surfd`, and the time `surfd` received it (milliseconds since
  the Unix epoch). In process, beside `Daemon::sessions()` of `agent-surfd-supervisor`:
  - `Daemon::session_entries(session)`: the kept entries of one session in order and its dropped
    count, or that its log was dropped;
  - `Daemon::entries_after(number)`: the kept entries of every session with a number across
    sessions above `number`, in that order, or `OlderThanKept` when an entry above `number` has
    been dropped;
  - `Daemon::subscribe_after(number)`: the same entries, then every new entry as it is kept, in
    order, through a queue of 1,024; or `OlderThanKept`.
  The session list entry gains `log dropped`. All three bounds (entries per session, bytes per
  session, bytes over all sessions) and the queue of a live reader are options of the `surfd`
  config, so a test can lower each, here and in `story:view-surfd-serves`.
- `crates/session/tests/` and `crates/surfd/tests/`, the counted cases. Cases that read the log run
  `surfd` in the test process through `Daemon::start` with the built `surf-session` as the session
  program, a page server the test starts, and requests over `surf_agent::wire` (decision D29):
  1. `open` without an address, then `go` to a served page: the session log holds the `open`
     action entry, its event entries, its answer entry, the `go` action entry, its event entries
     in the order the in-process core of the same calls lists `System::published()`, and its
     answer entry, per-session numbers consecutive from 1;
  2. the answer entry of that `go` equals the lines the client printed, byte for byte, and
     `surf_answer::parse` of it gives the page line;
  3. `token_in_no_entry`: the answer entry of `open` holds every line the client printed but the
     `token` line, and after `open`, `go`, `look`, `close` the 64 digits of the token are in no
     entry of either of two sessions;
  4. two sessions doing `go` at the same time: each session log holds only its own entries, and
     the numbers across sessions increase with none used twice;
  5. `look` adds no entry;
  6. a command refused for a wrong token adds no entry;
  7. `go` to a page of 5,000 bytes: the `surf.net.ResponseReceived` entry and the
     `surf.navigation.NavigationCommitted` entry each have `body` as `5000 bytes`, and no entry
     holds a byte of the page;
  8. `go` to a page of 6 MiB: the session answers and stays `open`, and both entries have `body`
     as `6291456 bytes`;
  9. the entry bound per session set to 10, after 25 entries: 10 kept, the first kept is
     per-session number 16, dropped 15;
  10. the log alone, with entries of exactly 1,000 bytes and the byte bound per session set to
      4,096, after 10 entries: 4 kept, the first kept is 7, dropped 6;
  11. the log alone, entries of 1,000 bytes, the bound over all sessions set to 8,000: session A
      ended with 4 entries, session B open with 4; after 2 more of B the log of A is dropped
      whole, its list entry says `log dropped`, and B keeps all 6;
  12. `entries_after` the number of the second entry gives every later entry in order; after
      entries were dropped, `entries_after` a number below the first kept answers `OlderThanKept`;
  13. `subscribe_after` the last kept number, taken before a `go`, delivers the entries of that
      `go` equal to the log, in order;
  14. a subscriber that never reads is disconnected after 1,024 entries; `surfd` and the session
      go on, and a new subscriber receives the next `go`;
  15. an address holding a line feed, `"` and a bidirectional control reaches the `url` field of
      the `surf.net.RequestStarted` entry quoted and escaped, in one entry with no extra line, and
      `surf_answer::unquote` of the field gives the address exactly;
  16. after `close`, and after a session process is killed, the entries kept before stay readable
      while the session is listed;
  17. every `SystemEvent` variant on `main` when this story starts, `url` of
      `surf.net.ResponseReceived` and of `surf.navigation.NavigationCommitted`
      (`agent-redirect-spec`) among the fields, maps to an event entry with all its fields, none
      `unmapped`;
  18. `byte_fields_are_lengths`: the mapping test lists the fields of type `Bytes` of the variants
      it covers, exactly `surf.net.ResponseReceived.body` and
      `surf.navigation.NavigationCommitted.body` on `main`, and each maps to `<n> bytes` for a body
      of 3 bytes, never to the bytes; a story that adds a `Bytes` field adds it to the list.

Surfaces written: `crates/session/`, `crates/surfd/`.

## For AGENTS.md (handed over in the unit report, decision D7)

- "Layout", the `crates/surfd` line gains: "keeps, per action, its action, events and answer: at
  most 10,000 entries or 16 MiB per session and 64 MiB over all; no token, no body, no password,
  only their lengths."

## Acceptance

`cargo test -p surf-session -p surfd` passes with the 18 numbered cases, each a test named after
its case (case 3 named `token_in_no_entry`), case 1 comparing the event names with
`System::published()` of the in-process core, case 8 leaving the session `open`, and case 11
reporting the log of A dropped and 6 entries kept for B.

## Out of scope

The HTTP routes, the event stream on the wire and its specified shape (`epic:live-view`); mapping
events added after this story (each adding story, D27); keeping entries after `surfd` ends.
