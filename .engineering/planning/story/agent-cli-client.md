---
format: aep.planning-md/3
id: story:agent-cli-client
kind: story
status: implemented
title: 'surf-agent: an argument parser held to cli/surf-agent.yaml, the token, and the wire'
summary: The surf-agent binary parses its arguments from the ess-cli/2 document, reads the token from SURF_TOKEN or a private file, starts surfd on open when none runs, sends one request to surfd and prints the answer.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-cli-spec
- depends_on: story:agent-answer-format
- depends_on: story:agent-secret-primitives
scope:
- confidence: cited
  path: crates/agent
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-11-21T02:10:31Z", actor: "human:timo", revision: 12, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

Design section 3, as amended on 2026-11-21: `surf-agent` is a stateless client: it parses its
arguments, reads the token, sends one request to `surfd` and prints the answer; `open` starts
`surfd` when none is running. Both ESS command-line generators need third-party crates
(`docs/plan/agent-cli-draft-1.md`, part 4 A and B), which `AGENTS.md` keeps out of anything that
ships, so the parser is written here from `cli/surf-agent.yaml`, standard library only, and a test
holds the two together (design section 6).

Story `agent-cli-spec` (decision D5) writes `cli/surf-agent.yaml` and the task `cli`, which puts
what `ess generate cli` makes of it into `.out/cli/` (`binding.json`, `help.txt`); `task test`
depends on it. This story does not wait for `agent-session-spec`. `surfd` itself is
`agent-surfd-supervisor`, which uses the wire written here; this story tests against stand-ins.

The token is a credential; other local users can read the argument words of a process
(`/proc/<pid>/cmdline`), so it is never one (design section 3). The operating system limits a Unix
socket path to 107 bytes (from memory, to be confirmed).

## Change

- `crates/agent` (package `surf-agent`; library and binary `surf-agent`; standard library,
  `surf-answer` (its quoting function and `unquote`) and `surf-secret` (the token) only;
  `#![forbid(unsafe_code)]`):
  - the parser: `open [<url>]`, `go <url>`, `look`, `close`; the session from `--session <id>` or
    `SURF_SESSION`; the token from `SURF_TOKEN` or `--token-file <path>`, as the document declares;
    `open` takes neither; a session identity is a UUID in text form, sent in lower case, the only
    form the answer renderer writes (`[0-9a-z-]+`);
  - the token: exactly 64 lower-case hexadecimal digits, read with `surf_secret::Token::from_hex`
    and held only as a `surf_secret::Token`, whose `Debug` writes `Token(..)`; this crate uses no
    generated type for it (the generated `surf.agent.SessionToken` derives `Debug`, a finding of
    wave 1, `docs/plan/agent-wave-1.md`). A token file only if it is a regular file, not a
    symbolic link, owned by the caller, with no permission bit for group or others; no option
    takes the token as a word;
  - the three `UNMAPPED:` markers of `cli/surf-agent.yaml`, each with what the client does: (a)
    no environment source in `ess-cli/2`: the client reads the session from `SURF_SESSION` when
    `--session` is absent and the token from `SURF_TOKEN` when `--token-file` is absent, and an
    option given wins over the environment; (b) `--token-stdin` and `--token-prompt`, declared
    because a `protected` source requires them: the client refuses each with exit 2 and the
    usage, reading nothing from standard input or a terminal; (c) the state global
    `--session-dir`, which `ess-cli/2` requires and the design does not name: the client refuses
    it with exit 2 and touches no path, the directory being `$XDG_RUNTIME_DIR/surf/` always.
    These are the only differences between the binding and the parser;
  - the wire (module `wire`, public; `surfd` and the session process use it): one request and one
    answer per connection. The request is generic over named arguments, not typed per command:
    a version line `surf-wire 1`, then `command <name>`, `session <id>` (not for `open`), `token
    <64 digits>` (not for `open`), and one line `arg <name> "<value>"` per argument, the value
    written by `surf-answer`'s quoting function and read back with `surf_answer::unquote`.
    `open [<url>]` sends `arg url`. A later argument is a new `arg` name and needs no new version
    (`runner-record-option` adds `arg record`, an absolute path); the receiver answers an
    argument name it does not know with status 2. The answer is the exit status, the answer lines
    and a reason for standard error. Each side bounds what it reads (request 64 KiB, answer a
    documented limit) and times out; the client waits 75 seconds for an answer; the crate
    documents the format;
  - the socket `$XDG_RUNTIME_DIR/surf/surfd.sock`;
  - starting `surfd`: `open` that finds no listener on the socket starts `surfd` from the
    directory of its own executable, in a process group of its own, with standard input and
    standard error discarded and standard output a pipe; reads one line within 5 seconds:
    `ready` and it connects, `refused <reason>` and it exits 3 with the reason; it does not wait
    for `surfd` to end;
  - exit statuses (design section 4): the status of the answer passed through (0, or the status
    the reason table of `docs/agent-answer.md` gives a refusal: 1, 2 or 3); 2 usage of the
    client; 3 `surfd` cannot be reached or started.
- `crates/agent/examples/stand_in_surfd.rs`: a stand-in `surfd` over the wire whose behaviour a
  test sets; `cargo test` builds it.
- `crates/agent/tests/`, the counted cases:
  1. `drift`: reads `.out/cli/binding.json`, which `task cli` writes, with a reader written in the
     test, and checks both ways that every command, positional argument, option and source there
     is the parser's and every one of the parser's is there, except exactly the differences
     of the three markers, listed in the test: `--token-stdin`, `--token-prompt` and
     `--session-dir` in the binding and refused by the parser; `SURF_SESSION` and `SURF_TOKEN` in
     the parser and not in the binding;
  2. `planted_drift`: one command renamed in a copy of that reading makes the comparison fail;
  3. token from `SURF_TOKEN`; 4. token from a file of mode 0600;
  5. a token file of mode 0644: exit 2; 6. a token file that is a symbolic link: exit 2;
  7. 63 digits: exit 2; 8. 65 digits: exit 2; 9. upper-case digits: exit 2;
  10. `--token <64 digits>` as a word: exit 2, and the digits are not echoed;
  11. a session that is not a UUID in text form (`../x`): exit 2 and nothing sent;
  12. an unknown command: exit 2 with the usage on standard error;
  against the stand-in serving `surfd.sock` under a short `XDG_RUNTIME_DIR` of its own:
  13. `go`, `look`, `close` and `open <url>` send the documented request (for `open`, `arg url`
      with the address quoted) and print the answer byte for byte, exit 0;
  14. a stand-in answering status 1: exit 1; 15. answering status 3: exit 3 with its lines;
  16. a stand-in that closes without answering: exit 3;
  17. a stand-in that stalls: exit 3 within the documented timeout (an option shortens it);
  18. a stand-in answer over the limit: exit 3;
  19. no `XDG_RUNTIME_DIR`: exit 3 with a reason, nothing created and nothing started;
  20. an `XDG_RUNTIME_DIR` long enough that the socket path exceeds 107 bytes: exit 3 with a reason;
  with a copy of the built `surf-agent` in a directory of the test and the stand-in beside it as
  `surfd`:
  21. `open` with no listener: the stand-in is started once, writes `ready`, and its `open` answer
      is printed, exit 0;
  22. `open` with the stand-in already listening: nothing is started (the stand-in marks each
      start in a file);
  23. a stand-in that writes `refused surf is a symbolic link`: exit 3 with that reason;
  24. a stand-in that writes nothing: exit 3 within 6 seconds;
  the three markers and the token in memory:
  25. `--token-stdin`: exit 2 with the usage, and standard input is not read;
  26. `--token-prompt`: exit 2 with the usage, and no terminal is opened;
  27. `--session-dir /x`: exit 2, and nothing under `/x` or `$XDG_RUNTIME_DIR` is touched;
  28. `--session` with `SURF_SESSION` set to another identity, and `--token-file` with
      `SURF_TOKEN` set to another token: the stand-in receives the identity and the token of the
      options;
  29. `token_not_in_debug_output`: the `Debug` output of the parsed request holds none of the 64
      digits of its token, and a usage error after the token was read prints none of them.

Surfaces written: `crates/agent/`.

## Acceptance

`cargo test -p surf-agent` passes with the 29 numbered cases, each a test named after its case,
and `cargo tree -p surf-agent` prints no package from outside this repository.

## Out of scope

`surfd` itself (`agent-surfd-supervisor`); what the answer says (`agent-answer-format`); the
commands of later slices.

## Scope

Derived 2026-11-21 by `story-scoper`.

- **Primary surface:** `crates/agent/` — cited. The story's "Surfaces written" line is `crates/agent/`. The crate is new: `crates/` holds only `fetch`, `ports` and `surf` on this branch. The root `Cargo.toml` takes it by the member glob `crates/*` (`Cargo.toml:3`).
- **Files written:** `crates/agent/Cargo.toml` — cited for package `surf-agent`, a library and a binary `surf-agent`, and dependencies of the standard library, `surf-answer` and `surf-secret` only (story, Change). The path dependencies `surf-answer = { path = "../answer" }` and `surf-secret = { path = "../secret" }` are inferred from the layout rule `AGENTS.md:97`. The edition (2021 as `crates/fetch/Cargo.toml`, or 2024 as `ports` and `surf`) and `publish = false` are inferred.
- **Files written:** `crates/agent/src/lib.rs` — inferred. The story names the library, the parser, the token rules and the public module `wire`, but no file names.
- **Files written:** `crates/agent/src/wire.rs` — inferred. The story says "module `wire`, public"; `agent-surfd-supervisor.md:64-65` and `agent-session-events.md:98` use it as `surf_agent::wire`.
- **Files written:** `crates/agent/src/main.rs` — inferred. The story says the package has a binary `surf-agent`; the layout follows `crates/surf`. It could be `src/bin/surf-agent.rs` instead.
- **Files written:** `crates/agent/src/` other modules (parser, token, socket, spawn of `surfd`) — inferred. The names are not given.
- **Files written:** `crates/agent/examples/stand_in_surfd.rs` — cited (story, Change).
- **Files written:** `crates/agent/tests/` — cited. It holds the 29 counted cases, each a test named after its case; the file names and the split into files are inferred.
- **Files read, not written:** `cli/surf-agent.yaml` — cited. Case 1 `drift` holds the parser to it. `agent-cli-spec` writes it (`agent-cli-spec.md` Change, Surfaces written).
- **Files read, not written:** `.out/cli/binding.json` (and `help.txt`) — cited. They are made by `task cli` from `taskfiles/cli.yml`, which `agent-cli-spec` writes. The story names `binding.json`, which `taskfiles/cli.yml` checks it writes.
- **Files read, not written:** `surf-answer` in `crates/answer/` — cited. The quoting function and `unquote` are the only dependency (story, Change; D20 in `docs/plan/agent-plan-revision-2.md:41-43`). The test reader of the binding is written in the test, not taken from a crate.
- **Files read, not written:** `docs/design/agent-interface.md` sections 3, 4 and 6 — cited (story, Context). The token rules, exit statuses and the drift-test rule come from there. I did not read them.
- **Files read, not written:** `docs/plan/agent-cli-draft-1.md` part 4 A and B — cited (story, Context), as the reason no generator is used.
- **Handed to the coordinator:** root `Cargo.lock` — cited by D7 (`docs/plan/agent-plan-revision-1.md:67-69`) and `AGENTS.md:236-237`. It gains one package, `surf-agent`, which depends on `surf-answer` and `surf-secret`. A wave that adds a crate fails `--locked` until the lock is regenerated (`AGENTS.md:93-95`).
- **Handed to the coordinator:** `AGENTS.md` "Layout" — cited from another story, not this one. This story has no "For AGENTS.md" section. `agent-session-process.md:96-97` hands the sentence "`crates/agent` (`surf-agent`): the agent command line, a parser held to `cli/surf-agent.yaml` by a test, and the wire to `surfd`." The coordinator should take it from there, or ask the decomposer to add a section here. Whether it needs a line for the socket path or the `SURF_TOKEN` and `SURF_SESSION` variables is inferred and not decided.
- **Handed to the coordinator:** root `Taskfile.yml` — none, cited. The story adds no task. `task test` already depends on `cli` through `agent-cli-spec`, so `.out/cli/` exists before `cargo test`.
- **Handed to the coordinator:** `README.md` — none, inferred. The story does not mention it.
- **Symbols:** crate `surf-agent`, lib name `surf_agent` — cited for the package and the binary; the lib name follows `agent-session-events.md:98` (`surf_agent::wire`).
- **Symbols:** public module `surf_agent::wire`, the request and answer of one connection (version line `surf-wire 1`, `command`, `session`, `token`, `arg <name> "<value>"`), with the documented limits of 64 KiB, the answer limit and 75 seconds — cited. The type and function names are inferred.
- **Symbols:** the binary `surf-agent`, commands `open [<url>]`, `go <url>`, `look`, `close`, `--session`, `--token-file`, and the variables `SURF_SESSION` and `SURF_TOKEN` — cited.
- **Symbols:** the socket `$XDG_RUNTIME_DIR/surf/surfd.sock` and the `ready` and `refused <reason>` lines on the pipe of `surfd` — cited. Later stories (`agent-surfd-supervisor`, `agent-session-process`, `agent-session-events`) use all three.
- **Symbols:** the example `stand_in_surfd` — cited.
- **Would collide with:** `crates/agent/` and `cli/surf-agent.yaml` — cited, from "Surfaces written" in each body.
  - `agent-follow-links`, `agent-fill-pick`, `agent-press-submit`, `agent-back-forward`, `agent-change-and-log`, `runner-detail-option` and `runner-record-option` each write the parser in `crates/agent`. All of them add a command or option "together so the drift test holds". Most depend on this story through the chain; I traced none of those chains. The parallel critic round 2 says every `crates/agent/**` surface is ordered by a `depends_on` path (`agent-plan-critic-parallel-round-2.md:70`).
  - `agent-surfd-supervisor`, `agent-session-process`, `agent-session-events` and `view-surfd-serves` use `surf_agent::wire` and write no file here — cited. They collide only if the wire changes.
  - `agent-cli-spec` writes `cli/surf-agent.yaml`, which this story reads — cited. It is an input, not a write collision. Its generated crate `generated/rust/crates/agent` (package `agent`) shares a directory name with this `crates/agent` (package `surf-agent`) — inferred. The two trees do not clash on disk, and the package names differ.
  - `agent-answer-format` writes `crates/answer/`, which this story reads — cited. It is an input, not a collision.
- **Not yet existing when it starts:** `crates/answer` with `surf-answer`'s quoting function and `unquote` — delivered by `story:agent-answer-format` (edge `depends_on`; the D20 `unquote` is in its Change). `cli/surf-agent.yaml`, `taskfiles/cli.yml` and `.out/cli/binding.json` after `task cli` — delivered by `story:agent-cli-spec` (edge `depends_on`). Nothing from `agent-session-spec` (cited: "does not wait for `agent-session-spec`"), and no real `surfd`: the tests use the stand-in.
- **Confidence:** high — the story names the one directory it writes, the symbols and the 24 cases, and the crate is new, so nothing on `main` can overlap. The file names inside `crates/agent` are inferred and cannot collide outside it. The one open point is which `.out/cli/` file the drift test reads.
