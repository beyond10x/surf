---
format: aep.planning-md/3
id: story:agent-surfd-supervisor
kind: story
status: active
title: 'surfd: one daemon per user that starts, supervises and routes to the session processes'
summary: The surfd binary, its start on the first open, the session list, supervision of session processes, routing, idle exit and the directory and socket checks.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-cli-client
- depends_on: story:agent-answer-format
- informed_by: epic:live-view
- depends_on: story:agent-secret-primitives
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 4}
---
## Context

Design section 3, as amended on 2026-11-21: `surfd` is the daemon, one per user. It owns the list
of sessions, starts one session process per session and supervises it, passes each request to the
session it names, and keeps the events each session publishes (story `agent-session-events`). It
holds no page and parses no page, no header and no body: the only text of a session it reads is
the answer grammar, through `surf-answer` (decision D20 of `docs/plan/agent-plan-revision-2.md`,
which amends the design sentence "parses nothing a server sent"). A page that breaks its session
process has not reached `surfd` or another session. The live view it will serve is `epic:live-view`.

This story is `surfd` without the events and without the live view: the binary, its start, the
session list, the supervision of session processes, the routing, its end, and the directory and
socket checks that design section 3 puts on the user-facing socket. It comes before
`agent-session-process`, which is the first real session behind it; its tests run a stand-in
session.

Decided here, where the design does not say:

- The session list is bookkeeping of the supervisor, not specified rows. The specification holds
  a session (`surf.agent.Session`) inside the session process; what `surfd` lists is what it knows
  about processes: started, answering, ended. The shape the live view reads (`/api/sessions`) is
  specified by `epic:live-view`. The list is documented in the crate.
- `surfd` draws the session identity (design: "drawn by `open`"): a version 4 UUID from the
  operating system random source, read through `surf_secret::fill_random`
  (`story:agent-secret-primitives`, decision D25), so no client can choose one that exists. This
  crate writes no random read and no comparison of its own.
- Two racing `open`s: each client that finds no `surfd` starts one; every `surfd` takes an
  exclusive lock on `$XDG_RUNTIME_DIR/surf/surfd.lock` (`std::fs::File::try_lock`, stable since
  Rust 1.89 from memory, to be confirmed against rustc 1.99.1) before it touches the socket; the
  loser reports ready and exits, and both clients reach the winner.
- A dead session process: `surfd` reaps it; ended with status 0 (after `close`, its idle timeout
  or the end of `surfd`) it is listed `closed`, ended any other way `died`. A call on a `closed`
  session exits 1 with `did refused closed`; on a `died` session 3 with `did refused session-died`;
  on an identity `surfd` does not list 3 with `did refused no-such-session` (grammar:
  `agent-answer-format`). These answers need no token: the identity is not secret and the
  answer carries nothing of the session but its end.
- Shutdown: `surfd` exits 10 minutes after the last open session ended (an option shortens it).
  A signal ends it the default way: the standard library installs no handler without `unsafe`
  or a crate. Each session process holds a pipe from `surfd` on its standard input and ends
  itself when the pipe closes, so a killed `surfd` leaves no session behind; the next `surfd`
  removes a stale socket and stale session directories it finds.
- Bounds: at most 64 connections at once (the 65th is closed unanswered); a session answers
  within 60 seconds or the call ends with status 3 and `did refused session-timeout`, the session
  left running; a session reports ready within 5 seconds or it is killed and `open` exits 3; at
  most 256 ended sessions stay listed, the oldest ended dropped first (it then answers `unknown`).
  The client waits 75 seconds (story `agent-cli-client`), longer than `surfd` waits.
## Change

- `crates/surfd` (package `surfd`; a library and the binary `surfd`; depends on `surf-agent` for
  its `wire`, `surf-answer` for the answers it writes itself and `surf-secret` for the identity,
  and on nothing that holds or
  parses a page; `#![forbid(unsafe_code)]`):
  - start: `surfd` is started by `surf-agent open` (story `agent-cli-client`) with a pipe on its
    standard output; it checks the directory, takes the lock, binds `$XDG_RUNTIME_DIR/surf/surfd.sock`,
    writes `ready` or `refused <reason>` as one line and closes standard output. A stale
    `surfd.sock` is removed only when it is a socket that refuses a connection.
  - the directory: `$XDG_RUNTIME_DIR/surf/` with mode 0700, created if absent; refused when
    `XDG_RUNTIME_DIR` is unset (no fallback to a shared directory), when `surf` or a session
    directory is a symbolic link, owned by another user or of another mode, when `surfd.sock` is
    anything but a socket, or when a session directory exists before `surfd` creates it. The
    decision is one function over metadata and the caller user id, so the owner case is testable
    without a second user. A session socket path over 107 bytes refuses `open` before anything
    starts.
  - the peer: if `UnixStream::peer_cred` is stable in rustc 1.99.1, a connection from another user
    id is closed unanswered; otherwise the crate documentation says the directory mode and the
    token carry the check (design section 10, question 4).
  - the session list (`surfd::Sessions`, read in process through `Daemon::sessions()`, by the
    tests of this story and later by the live view): per session the identity, the state
    (`open`, `closed`, `died`), when it was opened, when its last action was, when it ended, its
    process id, and the address and title as the session last reported them. Those two arrive
    quoted by `surf-answer` on the link, which is printable ASCII, and are kept quoted; `surfd`
    reads them only through `surf_answer::unquote`, in the view routes, which then write the exact
    value as a JSON string (decision D26).
  - starting a session: `open [<url>]` draws the identity, creates `<identity>/` with mode 0700,
    starts `surf-session` from the directory of its own executable with the identity and the
    socket path as arguments (neither is secret), in a process group of its own, standard input a
    pipe `surfd` holds, standard output a pipe, standard error discarded; waits for `ready` on
    that pipe; then passes the `open` request on and relays the answer, `token` line included,
    without keeping it. `surfd` reports the session ready by that answer. A rule of the supervisor:
    an `open` whose answer has a non-zero exit status leaves no entry in the session list: `surfd`
    relays the answer, waits for the session process to end (it closes the session standard input
    and kills the process if it still runs 2 seconds later) and removes the session directory.
    Decision D28 relies on it: a `--record` path the session cannot record to is its `open`
    answer with status 2 (`runner-record-option` tests that case).
  - the link (module `surfd::link`, used by `surf-session`): the `ready` line; per request the
    request of the wire passed on unchanged, token included, and an answer of the exit status, the
    answer lines and a status record (state, address, title); at most 4 MiB, printable ASCII
    only. A session that breaks a rule of the link is killed and listed `died`.
  - routing: one thread per connection; a request names its session (none for `open`); an
    identity that is not a UUID in text form is answered with status 2 and touches no path;
    requests to one session go one at a time.
  - supervision: a thread per session waits on its process; at its end the session socket and
    directory are removed and the state set. A call that finds the session socket gone waits up
    to 2 seconds for the process to end and answers from its end state.
  - end: idle exit as decided above; `surfd.sock` removed.
  - the library: `Daemon::start(config)` in `crates/surfd/src/daemon.rs` runs `surfd` in the
    caller process, and the binary calls nothing else; it is the one start function, and the
    place a later server of `surfd` is started (`story:view-surfd-serves` adds its one call there,
    after `surfd.sock` is bound and before `ready` is written). The config names the
    runtime directory, the session program, the timeouts, the idle exit and the identity source
    (by default `surf-secret`; a test sets fixed identities). The binary is a thin `main` over it.
  - `crates/surfd/examples/stand_in_session.rs`: a stand-in session over `surfd::link` whose
    behaviour a test sets (answer, answer `open` with status 2, report an address, stall, exit,
    break the link, never ready); `cargo test` builds it.
- `crates/surfd/tests/`, the counted cases, under a short `XDG_RUNTIME_DIR` of their own, the
  stand-in as the session program, in one of two harnesses (decision D29): **in process**, a
  `surfd` started through `Daemon::start` in the test, spoken to with `surf_agent::wire`, its list
  read through `Daemon::sessions()`; **built**, copies of the built `surf-agent` and `surfd` with
  the stand-in beside them as `surf-session`, asserting only what an answer prints, exit statuses,
  files and processes, never the list:
  1. (in process) `open`: the answer of the stand-in with its `token` line; the list holds the
     session `open` with its opened time and the address the stand-in reported;
  2. (built) eight `open`s at once with no `surfd` running: all exit 0 with eight distinct
     identities, each answers `look` with its token, and `surfd.lock` cannot be locked by the
     test while `surfd` runs;
  3. (in process) routing: with two sessions, each stand-in records the requests it got, and each
     record holds only requests naming its own session;
  4. (built) the token of an `open` is in no file under `$XDG_RUNTIME_DIR/surf/` and in no
     `/proc/<pid>/cmdline` of `surfd` or the stand-in;
  5. (built) an identity never opened: exit 3, `surf 1 session <id> unknown`, `did refused
     no-such-session`, nothing started;
  6. (in process) a session process killed with SIGKILL: the next `look` answers status 3 with
     `surf 1 session <id> died` and `did refused session-died`; the list shows `died`; its
     directory is gone; a second session still answers;
  7. (in process) a session process that exits while answering: that call answers status 3 with
     `died`;
  8. (in process) `close`: the stand-in exits 0; the list shows `closed`; the next call answers
     status 1 with `surf 1 session <id> closed` and `did refused closed`;
  9. (in process) a stand-in that never reports ready: `open` answers status 3 within 6 seconds
     with a reason, the stand-in is gone, no directory and no list entry is left;
  10. (in process) a stand-in that stalls, with the session timeout set to 1 second: the call
      answers status 3 with `did refused session-timeout`, the session stays listed `open`;
  11. (in process) a stand-in that sends an answer over 4 MiB, and one that sends a byte outside
      printable ASCII: each session is ended and listed `died`;
  12. (built) idle exit set to 1 second: after the last `close` `surfd` exits within 3 seconds and
      removes `surfd.sock`; the next `open` starts a new one;
  13. (built) `surfd` killed with SIGKILL while a session is open: the session process ends within
      2 seconds; the next `open` succeeds and the stale session directory is gone;
  14. (built) no `XDG_RUNTIME_DIR`: `surfd` writes `refused`, exits non-zero and creates nothing;
  15. (built) `$XDG_RUNTIME_DIR/surf` a symbolic link: refused, nothing created through it;
  16. (built) `$XDG_RUNTIME_DIR/surf` existing with mode 0755: refused;
  17. the directory decision on metadata owned by another user id: refused;
  18. (built) `surfd.sock` existing as a regular file: refused and the file untouched;
  19. (built) an `XDG_RUNTIME_DIR` long enough that a session socket path exceeds 107 bytes:
      `open` exits 3 with a reason and no process starts;
  20. (built) a request of over 64 KiB, and one naming the session `../x`, sent raw to
      `surfd.sock`: the first is closed unanswered, the second answered with status 2, nothing
      outside `$XDG_RUNTIME_DIR/surf/` touched, and the next request is answered;
  21. (built) 64 calls held by stalling stand-ins: a 65th connection is closed unanswered and its
      client exits 3; the 64 are answered when the stand-ins go on;
  22. (in process) a session directory that is a symbolic link: with a fixed next identity,
      `surf/<identity>` planted as a link to a directory of the test: `open` answers status 3
      with a reason, nothing is created in the link target, no process starts, no list entry;
  23. (in process) a session directory that exists before `surfd` creates it: with a fixed next
      identity, `surf/<identity>/` planted holding a file: `open` answers status 3, the directory
      and the file are untouched, no process starts, no list entry;
  24. (in process) a stand-in that answers `open` with status 2, and one that answers it with
      status 1: each answer is relayed with its status and lines, each stand-in has ended within 3
      seconds, its directory is gone, and the list holds no entry for either;
  25. (in process) a stand-in that reports the address `a"b` followed by a line feed: the list
      holds it quoted, printable ASCII only, and `surf_answer::unquote` of it gives that address
      exactly.

Surfaces written: `crates/surfd/`.

## For AGENTS.md and README.md (handed over in the unit report, decision D7)

- `AGENTS.md` "Layout": "`crates/surfd` (`surfd`): the daemon, one per user: the session list,
  one session process per session, routing on `$XDG_RUNTIME_DIR/surf/surfd.sock` (mode 0700).
  It holds no page."
- `AGENTS.md` "Commands": "`surfd` is started by the first `surf-agent open` and exits 10
  minutes after its last session ended."
- `README.md` "Driving surf from an agent": "The first `open` starts `surfd`, which runs each
  session in a process of its own and ends 10 minutes after the last session closed."

## Acceptance

`cargo test -p surfd` passes with the 25 numbered cases, each a test named after its case, and
`cargo tree -p surfd -e normal` prints `surfd`, `surf-agent`, `surf-answer` and `surf-secret` and
no other package.

## Out of scope

The events and their log (`story:agent-session-events`); the HTTP side and the live view
(`epic:live-view`); the session itself (`story:agent-session-process`); hashing the token
(`story:agent-token-hash`); carrying `--record` (`runner-record-option`, decision D28).
