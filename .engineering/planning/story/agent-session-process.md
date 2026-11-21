---
format: aep.planning-md/3
id: story:agent-session-process
kind: story
status: implemented
title: 'The session process: one browser session that surfd starts and alone connects to'
summary: surf-session, started and supervised by surfd, issues the token from surf-secret, checks it in constant time and answers open, go, look and close over the link; the session core is a library usable without a socket.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-page-path-spec
- depends_on: story:agent-answer-format
- depends_on: story:agent-cli-client
- depends_on: story:agent-surfd-supervisor
- depends_on: story:agent-secret-primitives
scope:
- confidence: cited
  path: crates/session
- confidence: inferred
  path: docs/agent-answer.md
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 8}
- {from: "active", to: "implemented", at: "2026-11-21T04:50:54Z", actor: "human:timo", revision: 16, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

Design section 3, as amended on 2026-11-21. The session process holds one session: the system
`crates/surf` assembles (`Browser`, six component ports from `surf-ports`), the `agent`
component with its action log (`agent-session-spec`, `agent-page-path-spec`), and the page state
of its last answer. `surfd` (`crates/surfd`, wave 3) starts it in a directory `surfd` created and
talks to it over the link `surfd::link`; no client connects to it. `open` returns when this
process has answered `open`. One process per session is the isolation the goals ask for.

The scenario runner needs the same session without a socket or a process (`story:runner-run`:
"a library that opens a session over a caller-supplied `Transport` with no socket and no
process"). So the session core is a library in `crates/session`, and the socket and the process
are a thin binary over it. It depends on the library `surf`, so it is in neither the package
`surf` nor `surf-cli`.

The token is issued and checked here (`docs/plan/agent-cli-draft-1.md`, marker 4); `surfd` passes
it on unchanged and keeps it nowhere (`crates/surfd/src/lib.rs`, "The peer"). The design
(section 3) asks that the process keep only a hash; no hash function exists in this repository,
and `AGENTS.md` ("Goals") takes cryptographic primitives from an audited crate not chosen yet. So
the 32 bytes stay in process memory only, as a `surf_secret::Token`; backlog
`story:agent-token-hash` carries the hash.

No peer check is done anywhere: `UnixStream::peer_cred` is unstable in rustc 1.99.1
(`crates/surfd/src/lib.rs`, "The peer"). The session directory (mode 0700, owned by the caller,
created by `surfd`) and the token carry it.

### What `surfd` gives this process and expects from it

Each line is in `crates/surfd/src/link.rs` (module documentation), `crates/surfd/src/daemon.rs`
or the reference implementation of the session side, `crates/surfd/examples/stand_in_session.rs`:

- Arguments: `<identity> <session socket path>`, the identity a lower-case UUID of 36 bytes, the
  path `$XDG_RUNTIME_DIR/surf/<identity>/session.sock` (`link::SESSION_SOCKET`); neither secret.
  `surfd` passes these two and nothing else (`daemon.rs`, `open_session`), so an option of
  `surf-session` cannot reach a session `surfd` started.
- Environment: `surfd`'s own without `SURF_TOKEN` and `SURF_SESSION`, plus `Config::session_env`
  (empty in the binary). Working directory `/`, a process group of its own, standard error
  discarded, standard input a pipe `surfd` holds.
- Start: bind the socket at the path given (its directory exists, mode 0700), then write
  `ready` with `surfd::link::write_ready`, within `surfd::SESSION_READY_TIMEOUT` (5 seconds), or
  it is killed. Any further byte on standard output gets it killed and listed `died`
  (`daemon.rs`, `supervise`).
- Per request: one connection; read one request of `surf_agent::wire` (`wire::read_request`),
  passed unchanged by `surfd`, token included; read the identity with `wire::session_id`; write
  one `surfd::link::Answer` (`Answer::encode`) and close the connection, within
  `surfd::SESSION_ANSWER_TIMEOUT` (60 seconds). Answer lines at most `wire::ANSWER_TEXT_MAX`
  (4 MiB), the record before them at most `link::RECORD_MAX` (64 KiB), printable ASCII only. The
  status record's `address` and `title` are `link::Quoted::new(value)`; each answer replaces the
  last in `surfd`'s list, so a refusal carries them too.
- `open`: an answer with a status other than 0 makes `surfd` close the process's standard input,
  kill it after `surfd::KILL_GRACE` (2 seconds) and list nothing.
- End: status 0 when standard input reaches its end. After `close`, the answer has `state
  closed`; `surfd` then closes standard input and kills the process if it still runs after the
  kill grace. `surfd` reaps the process, removes the session directory itself and lists it
  `closed` (status 0, not killed) or `died`; a later call is answered by `surfd`, `did refused
  closed` with status 1 (`daemon.rs`, `ended_answer`).

### What the model asks this process to issue

From `spec/domains/agent.yaml` (the comments above `OpenSession` and `RecordAction`) and
`spec/components.yaml`. `n` is the session's last sequence number; a refused command records
nothing.

- `open`: `surf.shell.OpenWindow{}`, then `OpenTab{window_id}`, then
  `OpenSession{session_id, window_id, tab_id}` as `Opener` (the `surf_ports::Caller` naming no
  session). When it is `opened`: `RecordAction{session_id, sequence: 1, command: open, url:
  absent}`.
- `open <url>`: the same, then `Go{session_id, url}` as `SessionHolder{session_id}` (the
  `Caller` set to the identity). When it is `requested`: `RecordAction{session_id, sequence: 2,
  command: go, url}`. So `open <url>` is two actions under one answer.
- `go <url>`: `Go` as `SessionHolder`; when `requested`, `RecordAction{..., sequence: n+1,
  command: go, url}`.
- `look`: the views `SessionState` and `SessionLog` as `SessionHolder`; no action.
- `close`: `CloseSession{session_id}` as `SessionHolder`; when `closed`, `RecordAction{...,
  sequence: n+1, command: close, url: absent}`. `RecordAction` checks only that the session
  exists, so it records on a closed session.
- `RecordAction` has no actor grant: it is issued on the port, `Browser::agent_mut()`.
- The `page` line comes from the `SessionState` row: `url`, `navigation_state`, `status`,
  `media_type`, `failure_reason`, which the note commands set through bindings.

## Change

- `crates/session` (package `surf-session`, `license = "Apache-2.0"`; a library `surf_session`
  and the binary `surf-session`; `#![forbid(unsafe_code)]`). Normal dependencies, all by path:
  `surf`, `surf-ports`, `surf-fetch`, `surf-answer`, `surf-secret`, `surf-agent` (for
  `surf_agent::wire`), `surfd` (for `surfd::link`) and the generated `surf-types`. The wire, the
  link and the test base are used through the crates and the file that define them, never
  copied.
  - The core: a session over any `surf::Transport`, opened with a caller-chosen identity. It
    issues what "What the model asks" lists, each command on its component's port
    (`Browser::shell_mut`, `Browser::agent_mut`) followed by `Browser::run`, holds the sequence
    number, and returns the typed `surf_answer::Answer` and the status record (state, address).
    The `did` line names the command the agent gave, with its argument when it had one: `did
    open "<address>"` or `did open`, `did go "<address>"`, `did close`; `look` prints no `did`
    line. The `path` lists the last actions, at most 5 (design section 5.1), oldest first: after
    `open <a>` it is `path 2 of 2`, `1 open`, `2 go "<a>"`. The two differ on purpose: the
    `did` line names the command the agent issued (`did <command> "<argument>"`,
    `docs/agent-answer.md` lines 108 to 109), the path lists the actions it became; neither is
    to be changed to match the other. The `page` reason word is the name
    of the `surf.net.FailureReason` variant, written without a `match` over the variants (the
    `Debug` form of the generated unit variant), so that a variant added by a specification
    story compiles and renders unchanged.
  - Refusals: a request whose `session` is not this identity or whose token does not match:
    `did refused token`, status 1, the head line and the `did` line only (no `page`, no
    `path`, no `note`), so a caller without the token learns nothing the session list does not
    show, and nothing is acted on. `surf_answer::render` writes that form unchanged (an
    `Answer` with only `did` set; `crates/answer/tests/golden/did-refused.txt` is that form). A command this session does not know, an argument name it
    does not know (`Request::unknown_arg`), `go` without `arg url`, an `open` after the first:
    `did refused usage`, status 2, with the current state, nothing acted on.
  - The process: `surf-session [--idle-timeout <seconds>] <identity> <socket path>`. Any other
    argument shape exits 2 without `ready`. It checks that `<identity>` is what
    `wire::session_id` returns for it, draws the token, binds the socket, writes `ready`, and
    serves one connection at a time with the read deadline `wire::REQUEST_TIMEOUT`. It ends with
    status 0 after the answer to `close`, at its idle timeout (30 minutes without a request,
    design section 3; `--idle-timeout` shortens it for a test that starts the binary itself),
    and when its standard input reaches its end, each time removing its socket file first.
  - The token: `surf_secret::Token::draw` (decision D25), printed once, as the `token` line of
    the answer to `open`, compared only by `Token::matches`. This crate writes no read of the
    random source and no comparison of its own.
  - Transport: `surf-fetch` through `surf::answer`. A non-`http:` address ends as `page failed
    UnsupportedScheme "<address>"`; no local file is read (design section 8). One fetch takes at
    most 40 seconds at `surf-fetch`'s default limits, inside the 60 of the link.

### What it never does

- Write the token anywhere but the `token` line of the answer to `open`: not to a file, not on
  standard output or standard error, not in the status record, not in another answer, not in a
  `Debug` form. It names no generated `SessionToken` (it derives `Debug`, a finding of wave 1).
- Act on, or answer with the session's state, a request whose token does not match.
- Fetch on its own: it fetches only the addresses `Browser::run` hands its transport, and follows
  no redirect (`surf-fetch` returns a 3xx as a response, `crates/fetch/src/lib.rs`, module
  documentation; `story:agent-fetch-redirects` changes that).
- Panic on a request: a request that does not decode, is over `wire::REQUEST_MAX`, or ends
  before `end` is answered with status 2 or closed unanswered, and the next request is served by
  the same process.
- Write to standard output after `ready`.

### How the three binaries find each other

In a build tree all three are in `target/<profile>/`: `surf-agent open` starts `surfd` from the
directory of its own executable (`crates/agent/src/client.rs`, `start_surfd`), and `surfd`
starts `surfd::SESSION_PROGRAM`, `surf-session`, from the directory of its own executable
(`crates/surfd/src/daemon.rs`, `Config::new`). Nothing here changes that; `cargo build
--workspace` (`task build`) puts all three there.

### Tests

`crates/session/tests/`, the counted cases. Directories under `test_base()` of
`crates/agent/tests/common/base.rs`, included with `#[path]` as `crates/surfd/tests/common/mod.rs`
does, each case with an `XDG_RUNTIME_DIR` of its own whose length plus the 55 bytes of a session
socket is at most `wire::SOCKET_PATH_MAX` (asserted with a named cause). The page server is one
the test starts on `std::net::TcpListener` at `127.0.0.1:0`. A test that sets a timer allows
five times it, and waits on a process, a socket or a pipe, never on a sleep.

The built harness: `cargo test -p surf-session` builds `surf-session` only, so the test builds
`surfd` and `surf-agent` once per test binary with the cargo running it (`env!("CARGO") build
--locked -p surfd -p surf-agent --profile <the test's profile>`, as
`crates/view/tests/prototype.rs` builds its example; cargo does nothing when they are fresh) and
runs `surf-agent` from `target/<profile>/`, beside the other two. Case 1 is alone in
`crates/session/tests/e2e.rs`; when the environment variable `SURF_BIN_DIR` names a directory,
it builds nothing and runs `surf-agent`, `surfd` and `surf-session` from that directory, so that
`story:release-agent-binaries` runs the same case on the packed binaries (`SURF_BIN_DIR=<unpacked
archive> cargo test --locked --release -p surf-session --target <target> --test e2e`). Golden texts replace the
identity, the token and the port with `<id>`, `<token>` and `<port>`. A guard stops the case's
`surfd` (found by `XDG_RUNTIME_DIR=<the case's directory>` in `/proc/<pid>/environ`) at the end
of the case, also when it panics.

1. `e2e_open_go_look_close` (built `surf-agent`, `surfd`, `surf-session`): `open
   http://127.0.0.1:<port>/a`, `go .../b`, `look`, `close` with `SURF_SESSION` and `SURF_TOKEN`
   set from the `open` answer: each exits 0, each answer equals its golden text, written from
   `docs/agent-answer.md` (`open`: head, `token`, `did open "<a>"`, `page 200 "text/html" "<a>"`,
   `path 2 of 2`, `1 open`, `2 go "<a>"`; `close`: head `closed`, `did close`, the page of
   `<b>`, `path 4 of 4`), and each is accepted by `surf_answer::parse`. Afterwards the session
   process has ended, no file is under `$XDG_RUNTIME_DIR/surf/<id>/`, and `look` exits 1 with
   `surf 1 session <id> closed` and `did refused closed` (decision D29: only answers are
   asserted).
2. `token_of_another_session_is_refused` (built): the token of a second session on the first:
   exit 1, the answer is exactly the head line and `did refused token`; `look` on the first with
   its own token equals the `look` before.
3. `token_in_no_command_line_environment_or_file` (built): after `open`, `go` and `look`, the 64
   digits are in no `/proc/<pid>/cmdline` and no `/proc/<pid>/environ` of the case's `surfd` and
   `surf-session` (the session found by its socket path in its arguments), and in no regular
   file under `$XDG_RUNTIME_DIR/surf/`.
4. `token_differing_in_one_byte_is_refused` (core): a token differing from the session's only in
   its first byte, and one only in its last byte: `did refused token`, status 1, the log
   unchanged; the session's own token is accepted.
5. `idle_timeout_ends_the_session` (the built `surf-session` started by the test with
   `--idle-timeout 1`, as `surfd` starts it otherwise): after `ready` and `open`, no request; the
   process ends with status 0 within 5 seconds and its socket is gone.
6. `end_of_standard_input_ends_the_session` (built `surf-session`, started by the test): after
   `ready` and `open`, the test closes the pipe; the process ends with status 0 (the test waits
   on it; a deadline of 10 seconds only stops a hang) and its socket is gone.
7. `file_address_is_not_read` (core, `surf-fetch` transport): `go file:///etc/passwd` exits 0
   with `page failed UnsupportedScheme "file:///etc/passwd"` and the path line `go
   "file:///etc/passwd"`.
8. `outside_values_are_quoted` (core): an address holding `"`, a backslash and a character
   outside ASCII is written on the `page` and `path` lines as `surf_answer::quote` writes it.
9. `core_without_a_socket` (core, a transport the test supplies): the calls of case 1 give the
   golden texts of case 1.
10. `open_without_an_address` (core): `open`, then `close`: `did open` and `did close` with no
    argument; the path of the second is `path 2 of 2`, `1 open`, `2 close`.
11. `token_held_only_as_surf_secret_token`: the `Debug` output of the core after `open` holds
    none of the 64 digits; no file under `crates/session/src/` names `SessionToken`, and
    `Token::to_hex` is called in one place, the answer to `open`.
12. `unknown_command_and_argument_are_usage` (built `surf-session`, over the link): `look` with
    `arg colour "x"`, `command fly`, and `go` without `arg url`: each status 2 and `did refused
    usage` under the head line; the path afterwards is the one before.
13. `malformed_requests_do_not_end_the_session` (built `surf-session`, over the link): bytes not
    of the wire, a request of `wire::REQUEST_MAX` plus one bytes, a connection closed before
    `end`, and a second `open`: each answered with status 2 or closed unanswered; then `look`
    with the token is answered by the same process id.
14. `standard_output_holds_ready_only` (built `surf-session`): over the requests of cases 12 and
    13 and a `close`, the process's standard output read to its end is exactly `ready` and a
    line feed.

The golden texts hold no `note parser` line: neither `Browser::parser_not_yet_parsed` nor
`surf_html::NOT_YET_PARSED` exists yet (`grep -rn NOT_YET_PARSED crates` finds none). Once they do
(`html-document-port`), every test of this crate that expects a `note parser` line builds it from
`surf_html::NOT_YET_PARSED` joined by `,` and names no literal list (decision D31).

`docs/agent-answer.md`, section "Exit status and refusals": one sentence, "A `token` refusal
prints the head line and the `did` line only: a caller without the token learns nothing of the
session."

Surfaces written: `crates/session/`, `docs/agent-answer.md` (that sentence). No other wave 4
story writes either. `Cargo.lock` is the coordinator's.

## For AGENTS.md and README.md (handed over in the unit report, decision D7)

- `AGENTS.md` "Layout": "`crates/session` (`surf-session`): one agent session; a library core
  over any `Transport`, and the binary `surf-session` that `surfd` starts from its own directory
  and alone connects to." The `crates/agent` line loses "There is no `surfd` yet (wave 3), so it
  cannot open a session; its tests run against a stand-in."
- `AGENTS.md` "Commands": "`target/debug/surf-agent open [<url>]`, after `task build`, then
  `go`, `look`, `close` with `SURF_SESSION` and `SURF_TOKEN` set: drive one session; the three
  binaries find each other in `target/debug/`."
- `AGENTS.md` "Implementation targets": "The agent target has code since `agent-session-process`:
  `open`, `go`, `look`, `close`."
- `README.md`, a new section "Driving surf from an agent": open, export `SURF_SESSION` and
  `SURF_TOKEN`, go, look, close; and "Quoting lowers the risk of page text steering an agent; it
  does not remove it."
- The "to try" list for the operator after the wave: `slot task build`; `target/debug/surf-agent
  open http://example.org/`; export the two values it printed; `target/debug/surf-agent go
  http://example.org/`; `look`; `close`.

## Acceptance

`cargo test -p surf-session` passes with the 14 numbered cases, each a test named as its case;
`cargo tree -p surf-session -e normal --prefix none` prints these 16 packages and no other:
`surf-session`, `surf`, `surf-ports`, `surf-fetch`, `surf-answer`, `surf-secret`, `surf-agent`,
`surfd`, `surf-types`, `surf-system`, `agent`, `shell`, `navigator`, `net`, `document`,
`renderer`; and `grep -c 'license = "Apache-2.0"' crates/session/Cargo.toml` prints 1.

## Out of scope

The directory, the start and the supervision (`story:agent-surfd-supervisor`, done); publishing
events (`story:agent-session-events`); links, forms, history, `changed`, `log`; hashing the token
(`story:agent-token-hash`); closing the window (`story:agent-close-window`); `--record`
(`runner-record-option`); the response address of a redirect (`story:agent-redirect-spec`, same
wave: it changes the specification, not a call of this crate).

## Revised 2026-11-21, before wave 4

- Context: "the peer check on the user-facing socket" moved to `surfd` is removed; no peer check
  exists anywhere (`crates/surfd/src/lib.rs`, "The peer").
- The quotation of `story:runner-run` corrected to its text ("a library that opens a session over
  a caller-supplied `Transport` with no socket and no process"); the old one was in no file.
- Added what `surfd` gives and expects and what the model asks to issue, each confirmed in
  `link.rs`, `daemon.rs`, the stand-in, `wire.rs` and `spec/domains/agent.yaml`.
- `open <url>` is two actions, `1 open` and `2 go "<url>"`; the path line `1 open "<address>"`
  was wrong against the model (`Action.url` is absent for `open`). `did open "<address>"` stays:
  the `did` line names the command the agent gave.
- Case 5: the idle timeout is an option of the binary, used by a test that starts it, because
  `surfd` passes two arguments only; "a later call exits 1" is surfd's answer and is case 1's.
- Case 6: "within 2 seconds" removed; the test waits on the process.
- Cases 13 and 14 added: no panic on a malformed request, nothing on standard output but
  `ready`. Case 2: a token refusal prints no state, with its sentence in `docs/agent-answer.md`.
- The built harness: how the test gets `surfd` and `surf-agent`, which `cargo test -p
  surf-session` does not build, and the shared `test_base()`.
- Case 1 in `tests/e2e.rs` with `SURF_BIN_DIR`, so that the release workflow of
  `story:release-agent-binaries` runs the same case on the packed binaries.
- The reason word is written without a `match`, so the `TooManyRedirects` of
  `agent-redirect-spec` in the same wave breaks nothing here.
- Acceptance: `cargo tree` names its 16 packages instead of "none from outside".
