---
format: aep.planning-md/3
id: story:session-process-hardening
kind: story
status: draft
title: surfd starts surf-session with a cleared environment, no new privileges and resource limits of its own
summary: 'Under S5 = a: env_clear and the note''s allow-list, setpriv --no-new-privs and prlimit with the note''s bounds; a test reads the running session''s environ, limits and status from /proc.'
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-decisions
scope:
- confidence: cited
  path: crates/surfd/src/daemon.rs
- confidence: inferred
  path: crates/surfd/src/link.rs
- confidence: cited
  path: crates/surfd/tests
revision: 3
---
## Context

The epic's fifth candidate (lines 437-442), "Security" (lines 237-275) and the second condition of
"From the security review of 2026-11-22" (line 603): before a session process runs a script, its
environment is reduced to what it needs, its privileges are restricted and it has resource limits
of its own. This story does that for `surf-session` before any script exists; it needs no swell
and no parser.

The tree on `main` of 2026-11-24:

- `crates/surfd/src/daemon.rs:1189-1200` starts the session: `Command::new(&shared.program)`
  with the identity and the socket path as arguments, `env_remove("SURF_TOKEN")`,
  `env_remove("SURF_SESSION")`, `envs(shared.env)` (the tests' `Config::session_env`, line 83;
  none in the binary), piped standard input and output, `current_dir("/")`, `process_group(0)`.
  Every other variable of `surfd`'s environment, which is the caller's, reaches the session.
- `:1154-1157`: the `open` request is passed to the session unchanged over its socket; the
  anchors of `open --ca-file` reach it there, never as an argument or in its environment
  (`crates/session/src/process.rs:17-19`).
- No crate of the session reads an environment variable: `git grep -n 'env::var\|var_os'
  main -- 'crates/*/src/*.rs' 'crates/*/src/**/*.rs'` finds reads in
  `crates/agent-tasks`, `crates/agent`, `crates/surfd/src/main.rs` and `crates/view-fixture` only.
- `#![forbid(unsafe_code)]` holds in every shipped crate; `setrlimit`, `prctl`, seccomp and the
  rest need a system call the standard library does not expose, and `CommandExt::pre_exec` is
  `unsafe` (epic lines 266-271). On this host `command -v setpriv prlimit` prints
  `/usr/bin/setpriv` and `/usr/bin/prlimit` (`setpriv --version`: util-linux 2.42.4, 2026-11-24).

## Assumes S5 = a

S5 is open with default a (epic lines 504-517): `surfd` starts `surf-session` through
`setpriv --no-new-privs` and `prlimit`, and refuses to start a session with scripts when either
is missing. Under b (an audited `unsafe` module in `surf-session`) this story would write
`crates/session/src/` instead of the helper; under c it writes the reduced environment only, and
no session ever runs a page's script. The note records the answer (Q11); this body is written for
a.

## Waits for the note

- waits for the note: Q10, the allow-list of variables (possibly empty) and the decision on the
  variables the standard library and the C library read;
- waits for the note: Q11, the operator's answer to S5; each bound and its value (address space,
  CPU seconds, open files, core size 0, no new privileges; processes only as Q11 says, since
  `RLIMIT_NPROC` counts every process of the user); the programs' absolute paths; whether every
  session or only a scripted one runs under them, and how `surfd` learns which; what `open`
  answers when a program is missing; and the probe that shows both programs `exec` the next one,
  so the pid `surfd` holds is `surf-session`'s.

## Decisions taken

1. The environment: `env_clear()`, then the allow-list of Q10, then `Config::session_env` for
   tests. `current_dir("/")` and `process_group(0)` stay.
2. The command line under a: `<setpriv> --no-new-privs -- <prlimit> <one --<resource>=<soft>:<hard>
   per bound of Q11> -- <session program> <identity> <socket>`, the program paths in `Config`
   (the shipped values Q11 names; a test can name others), so that a test can start a session
   with a missing program.
3. Before the session runs: nothing is read from the session to check the bounds; the tests read
   them from `/proc/<pid>/` from outside, as the epic says (lines 264-266).

## Change

- `crates/surfd/src/daemon.rs`: decisions 1 and 2, and the answer of Q11 to a missing program;
  `Config` gains the programs and the bounds.
- `crates/surfd/src/link.rs`: its "Start" text, if it describes the environment or the command
  line (inferred).
- `crates/surfd/tests/`: the cases below, in the crate's one test binary.

## Acceptance

`slot task check` exits 0. The folder's bounds hold: the test step ends within 120 s and no test
runs over 60 s; no model call and no network request in a test; each case starts one `surfd` and
one session, nothing more per test; no Python; no generator with a committed output is added (one that were would carry a digest of its sources, a fast check in the gate and full regeneration in the lane `slow`); a build or test job in the pool ends within 10 minutes or names its time with `slot --timeout`, at most 30 minutes; a test over 5 s is named with its time.

1. A test opens a session and reads `/proc/<pid>/environ` of its `surf-session`: it holds exactly
   the variables of the allow-list and those of the test's `Config::session_env`, and none of a
   variable the test set in `surfd`'s environment before starting it (`SURF_PROBE_SECRET=1`).
2. The same session's `/proc/<pid>/limits` shows each bound of Q11 with its soft and hard value,
   and `/proc/<pid>/status` holds `NoNewPrivs:	1`; `/proc/<pid>/exe` names `surf-session`.
3. With the path of `setpriv` (then of `prlimit`) in `Config` pointing to a file that does not
   exist, an `open` that Q11 says needs them is answered as Q11 says, and no process is left.
4. The tests of `surfd` and `surf-session` that ran before pass unchanged (`slot task test`), the
   end-to-end case `e2e::` of `surf-session` included.
5. `git diff --stat` names only `crates/surfd/src/daemon.rs`, `crates/surfd/src/link.rs` and
   `crates/surfd/tests/`.

## Out of scope

Seccomp, Landlock, namespaces and `PR_SET_DUMPABLE` (not reachable under S5 a); bounds on `surf
<url>` and `surf run`, which run no page script (S7); a process of its own for scripts (S2 = a).

## Placement

After `story:scripts-decisions`; beside `story:webidl-reader`, `story:scripts-engine-dependency`
and `story:scripts-spec`. `crates/surfd/src/daemon.rs` is also written by the active
`story:agent-surfd-session-limit`, `story:agent-surfd-idle-exit-race` and `story:agent-token-hash`
(none on `main` on 2026-11-24): never in the same wave; this story takes no edge to them, since it
needs nothing they deliver.

## Why `epic:scripts-in-a-page`

The second condition of the security review, which the epic puts before the first session that
runs a page's script.
