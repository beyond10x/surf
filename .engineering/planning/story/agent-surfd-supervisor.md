---
format: aep.planning-md/3
id: story:agent-surfd-supervisor
kind: story
status: implemented
title: 'surfd: one daemon per user that starts, supervises and routes to the session processes'
summary: The surfd binary, its start on the first open, the session list, supervision of session processes, routing, idle exit and the directory and socket checks.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-cli-client
- depends_on: story:agent-answer-format
- informed_by: epic:live-view
- depends_on: story:agent-secret-primitives
scope:
- confidence: cited
  path: .github/workflows/check.yml
- confidence: cited
  path: crates/agent/src/cli.rs
- confidence: cited
  path: crates/agent/src/wire.rs
- confidence: cited
  path: crates/agent/tests/common
- confidence: cited
  path: crates/surfd
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-21T02:59:46Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"verification":1}}}
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

What the client already fixes (`crates/agent`, `story:agent-cli-client`): `surf-agent open`
starts `surfd` from its own directory with no argument, with `XDG_RUNTIME_DIR` set, `SURF_TOKEN`
and `SURF_SESSION` removed and the rest of its environment passed on; it refuses by itself when
`XDG_RUNTIME_DIR` is unset or `surfd.sock` would pass 107 bytes; it waits 5 seconds
(`wire::START_TIMEOUT`) for `ready` and then connects once, with no retry
(`crates/agent/src/client.rs`); it sends an identity in lower case only (`wire::session_id`).
So a case about a start argument, a missing `XDG_RUNTIME_DIR` or an identity in upper case cannot
go through `surf-agent`; those cases start the built `surfd` themselves or write to `surfd.sock`
raw, as each says.

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
  exclusive lock on `$XDG_RUNTIME_DIR/surf/surfd.lock` with `std::fs::File::try_lock` before it
  touches the socket. `File::try_lock` is stable in rustc 1.99.1 (measured 2026-11-21: a probe
  compiled with `rustc --edition 2024` and no feature gate; a second open file of the same path
  got `TryLockError::WouldBlock`, and `Ok(())` once the first was dropped). The lock is held for
  the life of `surfd` and released by the kernel when it ends, also on SIGKILL. The loser writes
  `ready` only once `surfd.sock` accepts a connection, which it tries until `wire::START_TIMEOUT`
  (5 seconds) from its own start has passed; then it writes `refused another surfd holds the lock
  and its socket did not answer` and exits non-zero. Either way it exits without binding
  anything. The connection it tries with sends nothing and is closed; the winner reads no request
  on it and closes it. A client whose own 5 seconds run out first exits 3 just the same.
- A dead session process: `surfd` reaps it; ended with status 0 (after `close`, its idle timeout
  or the end of `surfd`) it is listed `closed`, ended any other way `died`. A call on a `closed`
  session exits 1 with `did refused closed`; on a `died` session 3 with `did refused session-died`;
  on an identity `surfd` does not list 3 with `did refused no-such-session` (grammar:
  `agent-answer-format`). These answers need no token: the identity is not secret and the
  answer carries nothing of the session but its end.
- Shutdown: `surfd` exits 10 minutes after the last open session ended. The option
  `surfd --idle-exit <seconds>` shortens it; it is an option of `surfd` only, and no environment
  variable changes a timer of the shipped binary. `surf-agent` passes no argument, so a `surfd`
  that `surf-agent` started always has the 10 minutes; a test that needs a shorter idle exit
  starts the built `surfd` itself with the option and then runs `surf-agent` against the running
  daemon. Any other argument: `refused` with the usage as the reason, exit 2. A signal ends it the
  default way: the standard library installs no handler without `unsafe` or a crate. Each
  session process holds a pipe from `surfd` on its standard input and ends itself when the pipe
  closes, so a killed `surfd` leaves no session behind; the next `surfd` removes a stale socket
  and stale session directories it finds, after it holds the lock.
- A stale entry under `surf/` that is a symbolic link makes `surfd` refuse to start, with
  `refused <the path> is a symbolic link`: `surfd` never follows and never removes a symbolic
  link, because a link in a directory only the user can write was put there by something that
  is not `surfd`, and removing or skipping it would hide that.
- The peer: `surfd` does not read the peer's user id on `surfd.sock` or on a session socket.
  `UnixStream::peer_cred` is unstable in rustc 1.99.1 (measured 2026-11-21: error E0658, library
  feature `peer_credentials_unix_socket`), and the crate uses no `unsafe` and no other crate. The
  directory's mode 0700, its owner check and the session token carry that check (design section
  3 and section 10, question 4); the crate documentation says so.
- The caller's user id is `surf_agent::wire::caller_uid()`, one definition shared by the client
  and `surfd`: the owner of `/proc/self`, which Linux gives the process's effective user id. It
  is Linux only. On a system without `/proc` it returns an error and `surfd` refuses to start
  with the reason; macOS is not in this story.
- The session directory and socket: `$XDG_RUNTIME_DIR/surf/<identity>/session.sock`, the
  identity in lower case (36 bytes). Its length is the length of `$XDG_RUNTIME_DIR` plus 55
  (`/surf/` 6, the identity 36, `/` 1, `session.sock` 12); `surfd.sock` is that length plus 16.
  So with an `XDG_RUNTIME_DIR` of at most 52 bytes every path fits in 107 bytes; from 53 to 91
  bytes `surfd` runs and every `open` is refused; from 92 bytes the client refuses before
  `surfd` starts.
- The link runs over the session socket, which the session process binds and only `surfd`
  connects to (design section 3: "one directory per session with that session's socket, which
  only `surfd` connects to"): one connection per request, the request of the wire passed on, one
  link answer, then the session closes the connection. The session process's standard output
  carries only its `ready` line; its standard input is the pipe whose close ends it.
- The environment of a session process: `surfd`'s own environment without `SURF_TOKEN` and
  `SURF_SESSION`, and the variables the config adds (none in the binary). Neither
  `story:agent-session-process` nor the design names another.
- Bounds: at most 64 connections at once (the 65th is closed unanswered); a connection counts
  from its accept to its close, counted by the accept loop before it starts the connection's
  thread, so the order of accepts decides which one is the 65th. A session answers within 60
  seconds or the call ends with status 3 and `did refused session-timeout`, the session left
  running; a session reports ready within 5 seconds or it is killed and `open` exits 3; at most
  256 ended sessions stay listed, the oldest ended dropped first (it then answers `unknown`).
  The client waits 75 seconds (story `agent-cli-client`), longer than `surfd` waits.
- The link's 4 MiB counts the answer lines only, `wire::ANSWER_TEXT_MAX`; everything else of a
  link answer, the status record included, is at most 64 KiB more, `wire::ANSWER_MAX` in all.
  Reason: `surfd` relays the answer lines unchanged into a wire answer, whose answer lines the
  client takes up to `wire::ANSWER_TEXT_MAX`; one limit on the same bytes at both hops means no
  answer passes the link and is then refused by the client.
- Times in the tests: every timer a case waits on is set by the test to 1 second (the config in
  process, `--idle-exit 1` for the built `surfd`) and the case allows 5 seconds for the outcome,
  a margin of four times the timer. A case that waits on no timer (a process ending after its
  pipe closed) allows 5 seconds. Case 2 runs the shipped 5 seconds of the client, which no test
  can shorten; within it each racing `surfd` does one process start and one bind, work of
  milliseconds, so the margin is over a hundred times. The acceptance names no time.

## Change

- `crates/surfd` (package `surfd`; a library and the binary `surfd`; `edition = "2024"`,
  `publish = false`, `license = "Apache-2.0"` (`AGENTS.md`, "Publishing"); depends on
  `surf-agent` for its `wire`, `surf-answer` for the answers it writes itself and `surf-secret`
  for the identity, and on nothing that holds or parses a page; `#![forbid(unsafe_code)]`):
  - start: `surfd` is started by `surf-agent open` (story `agent-cli-client`) with a pipe on its
    standard output; it checks the directory, takes the lock, removes what is stale, binds
    `$XDG_RUNTIME_DIR/surf/surfd.sock`, writes `ready` or `refused <reason>` as one line and
    closes standard output; the loser of the lock as decided above. A stale `surfd.sock` is
    removed only when it is a socket that refuses a connection.
  - the directory: `$XDG_RUNTIME_DIR/surf/` with mode 0700, created if absent; refused when
    `XDG_RUNTIME_DIR` is unset (no fallback to a shared directory), when `surf` or a session
    directory is a symbolic link, owned by another user or of another mode, when `surfd.sock` is
    anything but a socket, or when a session directory exists before `surfd` creates it. The
    decision is one function over metadata and the caller user id (`wire::caller_uid()`), so the
    owner case is testable without a second user. A session socket path over 107 bytes refuses
    `open` before anything starts: status 3, no answer lines, a reason that names the length,
    no session directory, no process.
  - the peer: not read, as decided above; the crate documentation says what carries the check.
  - the session list (`surfd::Sessions`, read in process through `Daemon::sessions()`, by the
    tests of this story and later by the live view): per session the identity, the state
    (`open`, `closed`, `died`), when it was opened, when its last action was, when it ended, its
    process id, and the address and title as the session last reported them. Those two arrive
    quoted by `surf-answer` on the link, which is printable ASCII, and are kept quoted; `surfd`
    reads them only through `surf_answer::unquote`, in the view routes, which then write the exact
    value as a JSON string (decision D26).
  - starting a session: `open [<url>]` draws the identity, creates `<identity>/` with mode 0700,
    starts `surf-session` from the directory of its own executable with the identity and the
    path of `session.sock` as arguments (neither is secret), in a process group of its own, the
    environment as decided above, standard input a pipe `surfd` holds, standard output a pipe,
    standard error discarded; waits for `ready` on that pipe; then passes the `open` request on
    over the session socket and relays the answer, `token` line included, without keeping it.
    `surfd` reports the session ready by that answer. A rule of the supervisor:
    an `open` whose answer has a non-zero exit status leaves no entry in the session list: `surfd`
    relays the answer, waits for the session process to end (it closes the session standard input
    and kills the process if it still runs after the kill grace, 2 seconds) and removes the
    session directory. Decision D28 relies on it: a `--record` path the session cannot record to
    is its `open` answer with status 2 (`runner-record-option` tests that case).
  - the link (module `surfd::link`, used by `surf-session`): the `ready` line on standard output;
    per request, over one connection to the session socket, the request of the wire passed on
    unchanged, token included, and an answer of the exit status, the answer lines and a status
    record (state, address, title); the answer lines at most 4 MiB (`wire::ANSWER_TEXT_MAX`), the
    rest at most 64 KiB, printable ASCII only. A session that breaks a rule of the link is killed
    and listed `died`.
  - routing: one thread per connection; a request names its session (none for `open`); an
    identity that is not a UUID in text form is answered with status 2, no answer lines and a
    reason for standard error (an answer needs a head line, and the head line takes only a
    session identity), and touches no path. A UUID is read in lower case (`wire::session_id`) and
    every answer `surfd` writes names the session in lower case, the `unknown` head line
    included: the renderer of `crates/answer` takes an identity of `[0-9a-z-]+` only. The answers
    `surfd` writes itself carry the exit status the reason table of `docs/agent-answer.md` gives:
    `closed` 1, `no-such-session`, `session-died` and `session-timeout` 3; requests to one
    session go one at a time.
  - supervision: a thread per session waits on its process; at its end the session socket and
    directory are removed and the state set. A call that finds the session socket gone waits up
    to 2 seconds for the process to end and answers from its end state.
  - end: idle exit as decided above; `surfd.sock` removed.
  - the library: `Daemon::start(config)` in `crates/surfd/src/daemon.rs` runs `surfd` in the
    caller process, and the binary calls nothing else; it is the one start function, and the
    place a later server of `surfd` is started (`story:view-surfd-serves` adds its one call there,
    after `surfd.sock` is bound and before `ready` is written). The config names the
    runtime directory, the session program, the variables added to a session's environment, the
    timeouts (session ready, session answer, kill grace), the idle exit, the connection limit and
    the identity source (by default `surf-secret`; a test sets fixed identities). The binary is a
    thin `main` over it that reads `XDG_RUNTIME_DIR` and `--idle-exit`.
  - `crates/surfd/examples/stand_in_session.rs`: a stand-in session over `surfd::link`, steered
    as the client's stand-in is (`crates/agent/examples/stand_in_surfd.rs`): by files in the
    directory `STAND_IN_DIR` names, which reaches it through `surf-agent` and `surfd` in the
    built harness and through the config's added variables in process; it records its process
    id per start and the requests it got there. Behaviours: answer, answer `open` with status 2
    or 1, report an address, stall until the test releases it, exit, break the link, never ready;
    `cargo test` builds it.
  - `crates/surfd/examples/agent.rs`: a thin `main` over `surf_agent::client::run` with
    `Environment::from_process()` and `Config::default()`, the same as `crates/agent/src/main.rs`;
    the built harness copies it as `surf-agent`, because `cargo test -p surfd` builds no binary of
    another package. `client::run`, `client::Config`, `cli::Environment` and
    `Environment::from_process` are public already; `crates/agent` needs no change for it.
- `crates/agent`, the smallest change for one definition of the caller's user id and one test
  base:
  - `src/wire.rs` gains `pub fn caller_uid() -> std::io::Result<u32>`, the body of the private
    `caller_uid` of `src/cli.rs` moved unchanged, documented as Linux only; `src/cli.rs` calls it
    and loses its own.
  - `tests/common/base.rs` (new) holds `test_base()` moved out of `tests/common/mod.rs`
    unchanged but for calling `surf_agent::wire::caller_uid()` instead of reading `/proc/self`;
    `tests/common/mod.rs` declares `mod base;` and re-exports it. `crates/surfd/tests/common/mod.rs`
    includes the same file with `#[path = "../../../agent/tests/common/base.rs"] mod base;`, so
    there is no second copy. The rule it holds: the test process's own `XDG_RUNTIME_DIR` when it
    is set, absolute, and an existing directory the caller owns; otherwise cargo's
    `CARGO_TARGET_TMPDIR`; never `/tmp`.
  - The 29 tests of `surf-agent` stay green.
- `.github/workflows/check.yml`: one step before `task check` that, when `XDG_RUNTIME_DIR` is
  unset or not an existing directory, creates a private directory with `mktemp -d /tmp/xdg.XXXXXX`
  and mode 0700 and exports it as `XDG_RUNTIME_DIR` through `$GITHUB_ENV`. `/tmp` is used on the
  runner only; on the workstation the jobs of `slot` see `XDG_RUNTIME_DIR=/run/user/<uid>`
  (measured 2026-11-21).
- `crates/surfd/tests/`, the counted cases, the stand-in as the session program, in one of two
  harnesses (decision D29): **in process**, a `surfd` started through `Daemon::start` in the
  test, spoken to with `surf_agent::wire`, its list read through `Daemon::sessions()`; **built**,
  copies of `examples/agent` as `surf-agent` and of the built `surfd` (`CARGO_BIN_EXE_surfd`) with
  the stand-in beside them as `surf-session`, the copies made as
  `crates/agent/tests/common/mod.rs` makes its (`bin_dir`), asserting only what an answer
  prints, exit statuses, files and processes, never the list. A built case that needs `surfd`'s
  process id or an option starts the built `surfd` itself, with the environment the client gives
  it, reads its `ready`, and then runs `surf-agent` against it. Every case runs under a directory
  of its own, `sdt-<process id>-<counter>-<8 random hexadecimal digits>` under `test_base()`,
  holding `r` (the case's `XDG_RUNTIME_DIR`) and `c` (the stand-in's directory): `r` is the base
  plus 27 bytes at most, so its session socket paths fit in 107 bytes with a base of up to 25
  bytes (`/run/user/<uid>` is 14 to 20). The directory's constructor asserts that
  `r` plus 55 bytes is at most 107 and otherwise fails with "the test's session socket path is
  <n> bytes, over 107; set XDG_RUNTIME_DIR to a short directory of yours". The cases:
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
  9. (in process) a stand-in that never reports ready, the session ready timeout set to 1 second:
     `open` answers status 3 with a reason within 5 seconds, the stand-in is gone, no directory
     and no list entry is left;
  10. (in process) a stand-in that stalls, with the session timeout set to 1 second: the call
      answers status 3 with `did refused session-timeout` within 5 seconds, the session stays
      listed `open`;
  11. (in process) a stand-in that sends answer lines of `wire::ANSWER_TEXT_MAX` plus one byte,
      and one that sends a byte outside printable ASCII: each session is ended and listed `died`;
  12. (built) the test starts the built `surfd` with `--idle-exit 1`; after the last `close`
      `surfd` exits within 5 seconds and removes `surfd.sock`; the next `open` starts a new one;
  13. (built) the test starts the built `surfd` and kills it with SIGKILL while a session is
      open: the session process ends within 5 seconds; the next `open` succeeds and the stale
      session directory is gone;
  14. (built) the test starts the built `surfd` directly with no `XDG_RUNTIME_DIR` (`surf-agent`
      would refuse first): `surfd` writes `refused`, exits non-zero and creates nothing;
  15. (built) `$XDG_RUNTIME_DIR/surf` a symbolic link: refused, nothing created through it; and
      `surf/<a UUID>` left as a symbolic link to a directory of the test, with no `surfd`
      running: refused with a reason naming that path, the link and its target untouched;
  16. (built) `$XDG_RUNTIME_DIR/surf` existing with mode 0755: refused;
  17. the directory decision on metadata owned by another user id than `wire::caller_uid()`:
      refused;
  18. (built) `surfd.sock` existing as a regular file: refused and the file untouched;
  19. (built) an `XDG_RUNTIME_DIR` of 53 bytes: `surfd.sock` is 69 bytes and starts, a session
      socket would be 108 bytes, so `open` exits 3 with a reason naming 108, and no session
      directory and no session process is made; with an `XDG_RUNTIME_DIR` of 52 bytes the
      session socket is 107 bytes and `open` exits 0;
  20. (built) a request of over 64 KiB, and one naming the session `../x`, sent raw to
      `surfd.sock`: the first is closed unanswered, the second answered with status 2, no answer
      line and a reason, nothing outside `$XDG_RUNTIME_DIR/surf/` touched, and the next request
      is answered;
  21. (in process) one session whose stand-in stalls, and 64 `look` connections to it, held by
      the stall and the one-at-a-time rule: a 65th connection is closed unanswered; when the test
      releases the stand-in, the 64 are answered. It costs one stand-in process, 65 sockets and
      a thread per connection, and no time beyond the stall; a smaller limit set in the config
      would prove the counting but not the 64, so the default stays;
  22. (in process) a session directory that is a symbolic link: with a fixed next identity,
      `surf/<identity>` planted as a link to a directory of the test: `open` answers status 3
      with a reason, nothing is created in the link target, no process starts, no list entry;
  23. (in process) a session directory that exists before `surfd` creates it: with a fixed next
      identity, `surf/<identity>/` planted holding a file: `open` answers status 3, the directory
      and the file are untouched, no process starts, no list entry;
  24. (in process) a stand-in that answers `open` with status 2, and one that answers it with
      status 1, the kill grace set to 1 second: each answer is relayed with its status and lines,
      each stand-in has ended within 5 seconds, its directory is gone, and the list holds no
      entry for either;
  25. (in process) a stand-in that reports the address `a"b` followed by a line feed: the list
      holds it quoted, printable ASCII only, and `surf_answer::unquote` of it gives that address
      exactly;
  26. (built) written raw to `surfd.sock` with `wire::Request::encode`, which keeps the case it is
      given (`surf-agent` lowercases): `look` naming an identity never opened, in upper case:
      status 3 and the head line `surf 1 session <the identity in lower case> unknown`; and,
      after an `open` through `surf-agent`, a `look` naming that session in upper case with its
      token reaches it, its head line in lower case.

Surfaces written: `crates/surfd/`, `crates/agent/src/wire.rs`, `crates/agent/src/cli.rs`,
`crates/agent/tests/common/`, `.github/workflows/check.yml`.

## For AGENTS.md and README.md (handed over in the unit report, decision D7)

- `AGENTS.md` "Layout": "`crates/surfd` (`surfd`): the daemon, one per user: the session list,
  one session process per session, routing on `$XDG_RUNTIME_DIR/surf/surfd.sock` (mode 0700).
  It holds no page."
- `AGENTS.md` "Commands": "`surfd` is started by the first `surf-agent open` and exits 10
  minutes after its last session ended."
- `README.md` "Driving surf from an agent": "The first `open` starts `surfd`, which runs each
  session in a process of its own and ends 10 minutes after the last session closed."

## Acceptance

`cargo test -p surfd` passes with the 26 numbered cases, each a test named after its case;
`cargo test -p surf-agent` still passes its 29; `crates/agent/src/` defines `caller_uid` once, in
`wire.rs`; `.github/workflows/check.yml` exports a private `XDG_RUNTIME_DIR` when the runner has
none; and `cargo tree -p surfd -e normal` prints `surfd`, `surf-agent`, `surf-answer` and
`surf-secret` and no other package.

## Out of scope

The events and their log (`story:agent-session-events`); the HTTP side and the live view
(`epic:live-view`); the session itself (`story:agent-session-process`); hashing the token
(`story:agent-token-hash`); carrying `--record` (`runner-record-option`, decision D28); macOS
and any system without `/proc`.

## Revised 2026-11-21, before wave 3

- Racing `open`s: the loser writes `ready` only once `surfd.sock` accepts, within 5 seconds, else
  `refused`, because the client connects once after `ready` and does not retry.
- Idle exit: `--idle-exit` is an option of `surfd` only; case 12 starts the built `surfd` itself,
  because `surf-agent` starts `surfd` with no argument.
- Case 14 starts the built `surfd` directly, because `surf-agent` refuses a missing
  `XDG_RUNTIME_DIR` before `surfd` runs.
- Named the session socket (`<identity>/session.sock`, `XDG_RUNTIME_DIR` plus 55 bytes) and the
  link over it; case 19 states the byte arithmetic, with the 52-byte boundary.
- Case 26 writes the request raw, because `surf-agent` lowercases the identity before sending.
- Added `examples/agent.rs` as the built harness's `surf-agent`; `crates/agent` is public enough
  already.
- The peer's user id is not read: `UnixStream::peer_cred` is unstable in rustc 1.99.1 (E0658).
- `caller_uid` moves to `surf_agent::wire`, one definition, Linux only; macOS out of scope.
- The session environment: `surfd`'s own without `SURF_TOKEN` and `SURF_SESSION`, plus the
  config's variables, so the stand-in is steered by `STAND_IN_DIR` in both harnesses.
- Test directories under the shared `test_base()` (included by `#[path]`), a length check with a
  named cause, and a CI step that exports a private `XDG_RUNTIME_DIR`.
- `File::try_lock` confirmed stable in rustc 1.99.1 by a probe.
- Timers in tests set to 1 second with 5 seconds allowed; case 21 holds 64 connections on one
  stand-in instead of 64 processes; the acceptance names no time.
- The 4 MiB counts the answer lines only; a stale symbolic link under `surf/` refuses the start.
- `crates/surfd/Cargo.toml` carries `license = "Apache-2.0"`.
