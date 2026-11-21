---
format: aep.planning-md/3
id: story:agent-session-process
kind: story
status: active
title: 'The session process: one browser session that surfd starts and alone connects to'
summary: surf-session, started and supervised by surfd, issues the token from surf-secret, checks it in constant time and answers open, go, look and close over the link; the session core is a library usable without a socket.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-page-path-spec
- depends_on: story:agent-answer-format
- depends_on: story:agent-cli-client
- depends_on: story:agent-surfd-supervisor
- depends_on: story:agent-secret-primitives
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 8}
---
## Context

Design section 3, as amended on 2026-11-21. The session process holds one session: the system
`crates/surf` assembles with the `agent` component (`agent-cli-spec`, `agent-session-spec`), the
action log (`agent-page-path-spec`) and the page state of its last answer. `surfd` starts it, in
the directory `surfd` created, and talks to it over the link `surfd::link`
(`agent-surfd-supervisor`); no client connects to it. `open` returns when `surfd` reports the
session ready, which is when this process has answered `open`. One process per session is the
isolation the goals ask for.

The scenario runner needs the same session without a socket or a process (story `runner-run`:
"an in-process entry point to the agent component ... over a caller-supplied `Transport`"). So
the session core is a library in `crates/session`, and the socket and the process are a thin
binary over it.

The token is issued and checked here (`docs/plan/agent-cli-draft-1.md`, marker 4); `surfd` passes
it on and keeps it nowhere. The design (section 3) asks that the process keep only a hash; no
hash function exists in this repository, and `AGENTS.md` ("Goals") takes cryptographic primitives
from an audited crate not chosen yet. So the 32 bytes stay in process memory only, never on disk,
in a log or in an answer after `open`; backlog `story:agent-token-hash` carries the hash.

Moved to `agent-surfd-supervisor` with the daemon: the directory and its refusals, the start of
the process, the peer check on the user-facing socket, the socket path length.

## Change

- `crates/session` (package `surf-session`; a library and the binary `surf-session`; depends on
  `surf`, `surf-ports`, `surf-fetch`, `surf-answer`, `surf-secret`, `surf-agent` (its `wire`), `surfd` (its
  `link`) and the generated crates; `#![forbid(unsafe_code)]`):
  - the core: a session over any `surf::Transport`, opened with a caller-chosen identity, that
    executes `open` (`OpenWindow`, `OpenTab`, `OpenSession`, then `Go` when an address was given),
    `go`, `look` (`SessionState`, `SessionLog`) and `close` (`CloseSession`), each through
    `Browser::run`, and returns the typed answer and the page state it compared. The `did` and
    `path` lines are those of `docs/agent-answer.md`: the argument quoted and left out when the
    command had none (`did open "<address>"` or `did open`, `did go "<address>"`, `did close`;
    `look` takes no action and prints no `did` line; path lines `1 open "<address>"`, `2 go
    "<address>"`, `3 close`). A refusal answers
    `did refused <reason>` with the current state and the exit status the reason table of
    `docs/agent-answer.md` gives (`token` and `closed` 1, `usage` 2). The next action sequence number is
    drawn here.
  - the process: `surf-session` takes the identity and the socket path from `surfd`, binds the
    socket inside the directory `surfd` created, writes `ready`, and answers one link request at a
    time with the exit status, the answer lines and the status record of the link. It ends with
    status 0 after `close`, at its idle timeout (30 minutes, an option so a test can shorten it)
    and when its standard input (the pipe from `surfd`) closes, removing its socket first.
  - the token: `surf_secret::Token` (`story:agent-secret-primitives`, decision D25 of
    `docs/plan/agent-plan-revision-2.md`): drawn from the operating system random source, 64
    lower-case hexadecimal digits, printed once in the answer of `open`, compared by
    `Token::matches`, which reads every byte whatever the input; this crate writes no random
    read and no comparison of its own; a wrong token answers
    `did refused token`, status 1, and acts on nothing.
  - a non-`http:` address ends as `page failed UnsupportedScheme "<address>"`; no local file is
    read (design section 8).
- `crates/session/tests/`, the counted cases, running the built `surf-agent`, `surfd` and
  `surf-session` against a page server the test starts with `std::net::TcpListener`, under a short
  `XDG_RUNTIME_DIR` of its own, unless a case says otherwise:
  1. `open` with an address, `go` to a second page, `look`, `close`: each answer equals its golden
     text; after `close` the session process has ended with status 0 and a following `look`
     exits 1 with `did refused closed` (a built `surfd` is asserted only through what an answer
     prints, decision D29);
  2. the token of a second session on the first: status 1, `did refused token`, and `look` on the
     first shows nothing changed;
  3. the token is in no `/proc/<pid>/cmdline` of `surf-agent`, `surfd` or `surf-session` and in no
     file under `$XDG_RUNTIME_DIR/surf/`;
  4. the token comparison: equal tokens accepted; tokens differing only in the first and only in
     the last byte refused;
  5. an idle timeout of one second ends the session with status 0; a later call exits 1 with
     `did refused closed`;
  6. the session standard input closed: the process ends within 2 seconds and its socket is gone;
  7. `go file:///etc/passwd` answers `page failed UnsupportedScheme`;
  8. an address holding `"`, a backslash and a byte outside ASCII is printed quoted and escaped
     on the `page` and the `path` lines;
  9. the core without a socket: the calls of case 1 over a transport the test supplies give the
     same answers;
  10. `open` without an address, then `close`: the answers hold `did open` and `did close` with no
      argument, and the `path` of the second holds `1 open` and `2 close`;
  11. `token_held_only_as_surf_secret_token`: the `Debug` output of the session core after `open`
      holds none of the 64 digits of the token, and no file under `crates/session/src/` names the
      generated `SessionToken` (it derives `Debug`, a finding of wave 1);
  12. a `look` request with an argument name the session does not know (`arg colour "x"`), sent
      over the link: status 2 and `did refused usage` under the head line of the session, and the
      session acts on nothing.

The golden texts of case 1 hold no `note parser` line: `Browser::parser_not_yet_parsed` does not
exist yet. Once it does (`html-document-port`), every test of this crate that expects a
`note parser` line builds it from `surf_html::NOT_YET_PARSED` joined by `,` and names no literal
list (decision D31).

Surfaces written: `crates/session/`.

## For AGENTS.md and README.md (handed over in the unit report, decision D7)

- `AGENTS.md` "Layout": "`crates/agent` (`surf-agent`): the agent command line, a parser held to
  `cli/surf-agent.yaml` by a test, and the wire to `surfd`." and "`crates/session`
  (`surf-session`): one agent session; a library core over any `Transport`, and the binary
  `surf-session` that `surfd` starts and serves it to `surfd` only."
- `AGENTS.md` "Commands": "`target/debug/surf-agent open [<url>]`, then `go`, `look`, `close`
  with `SURF_SESSION` and `SURF_TOKEN` set: drive one session."
- `AGENTS.md` "Implementation targets": "The agent target has code since `agent-session-process`:
  `open`, `go`, `look`, `close`."
- `README.md`, a new section "Driving surf from an agent": open, export `SURF_SESSION` and
  `SURF_TOKEN`, go, look, close; and "Quoting lowers the risk of page text steering an agent; it
  does not remove it."

## Acceptance

`cargo test -p surf-session` passes with the 12 numbered cases, each a test named after its case,
and `cargo tree -p surf-session -e normal` prints no package from outside this repository.

## Out of scope

The directory, the start and the supervision (`story:agent-surfd-supervisor`); publishing events
(`story:agent-session-events`); links, forms, history, `changed`, `log`; hashing the token
(`story:agent-token-hash`); closing the window (`story:agent-close-window`); `--record`
(`runner-record-option`).
