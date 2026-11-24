---
format: aep.planning-md/3
id: story:session-process-hardening
kind: story
status: draft
title: surfd starts surf-session with a cleared environment, no new privileges and resource limits of its own
summary: 'S5 = a, decided 2026-11-24: env_clear and the note''s allow-list, setpriv --no-new-privs and prlimit with the note''s bounds, read from /proc by tests; a session ending at a bound is listed died and its answer names the signal and bound, shown with the stand-in session and no script.'
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-decisions
scope:
- confidence: cited
  path: crates/answer/src/lib.rs
- confidence: inferred
  path: crates/answer/tests
- confidence: cited
  path: crates/surfd/examples/stand_in_session.rs
- confidence: cited
  path: crates/surfd/src/daemon.rs
- confidence: inferred
  path: crates/surfd/src/link.rs
- confidence: cited
  path: crates/surfd/tests
- confidence: cited
  path: docs/agent-answer.md
revision: 15
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

- A session that ends without `close` is listed `died`, and every later request for it is
  answered with the head state `died` and the refusal `session-died` (`crates/surfd/src/daemon.rs:
  269-272`, `:850-855`, `:983`; `docs/agent-answer.md:148`, `:177`); the answer does not say why
  the process ended. An answer may carry `note` lines from a fixed list (`crates/answer/src/
  lib.rs:536-547`, grammar `docs/agent-answer.md:560`).
- The tests of `surfd` start a stand-in session, `crates/surfd/examples/stand_in_session.rs`,
  whose behaviour a test sets by files in its control directory (`behaviour`: `answer`, `stall`,
  `exit`, `break`, ...; lines 10-21), named as the session program in the test's `Config`
  (`crates/surfd/tests/common/mod.rs:304-321`).

## S5, decided

The operator decided S5 on 2026-11-24 (relayed by the coordinating agent; recorded in
`docs/design/scripts.md` Q11): option a. `surfd` starts `surf-session` through
`setpriv --no-new-privs` and `prlimit`, and refuses to start a session with scripts when either
program is missing; `#![forbid(unsafe_code)]` stays in every crate. Seccomp, Landlock, namespaces
and `PR_SET_DUMPABLE` are absent under a; option b (an audited `unsafe` module) needs the owner's
word and is not this story.

## Waits for the note

- waits for the note: Q10, the allow-list of variables (possibly empty) and the decision on the
  variables the standard library and the C library read;
- waits for the note: Q11, each bound and its starting value (address space, CPU seconds, open
  files, core size 0, no new privileges; processes only as Q11 says, since `RLIMIT_NPROC` counts
  every process of the user); the programs' absolute paths; whether every session or only a
  scripted one runs under them, and how `surfd` learns which; what `open` answers when a program
  is missing; the probe that shows both programs `exec` the next one, so the pid `surfd` holds is
  `surf-session`'s; and the words of the answer when a session ends at a bound.

## Decisions taken

1. The environment: `env_clear()`, then the allow-list of Q10, then `Config::session_env` for
   tests. `current_dir("/")` and `process_group(0)` stay.
2. The command line: `<setpriv> --no-new-privs -- <prlimit> <one --<resource>=<soft>:<hard> per
   bound of Q11> -- <session program> <identity> <socket>`, the program paths and the bounds in
   `Config` (the shipped values Q11 names; a test can name others), so that a test can start a
   session with a missing program or a bound of its own.
3. Nothing is read from the session to check the bounds; the tests read them from `/proc/<pid>/`
   from outside, as the epic says (lines 264-266).
4. A session that ends at a bound of the operating system: when `surfd` reaps a session process
   it did not kill and that did not close, it reads the signal from the exit status
   (`std::os::unix::process::ExitStatusExt::signal`, no `unsafe`), keeps it with the entry, and
   still lists the session `died`. Every answer that names the session `died` then holds one more
   line of the fixed list, `note ended: signal <NAME>`, with `bound <resource>` added where the
   signal names a bound (`SIGXCPU`: `cpu`, `SIGXFSZ`: `fsize`); its exact words are Q11's. A
   session that ended with a status and no signal gets no such line. `surf-answer` parses and
   renders the line, and `docs/agent-answer.md` documents it.
5. The test of decision 4 needs no script: the stand-in session gains the `behaviour` value
   `spin`, a loop on the processor after it answers `open`, and the test starts it with a CPU
   bound of 1 second.

## Change

- `crates/surfd/src/daemon.rs`: decisions 1, 2 and 4, and the answer of Q11 to a missing
  program; `Config` gains the programs and the bounds.
- `crates/surfd/src/link.rs`: its "Start" text, if it describes the environment or the command
  line (inferred).
- `crates/surfd/examples/stand_in_session.rs`: the `behaviour` value `spin` and its line in the
  head comment.
- `crates/answer/src/lib.rs` and its tests: the note line of decision 4.
- `docs/agent-answer.md`: the note line, beside the state `died` and the refusal `session-died`.
- `crates/surfd/tests/`: the cases below, in the crate's one test binary.
- In the last commit, the public surfaces of acceptance 8.

## Acceptance

1. A test opens a session and reads `/proc/<pid>/environ` of its session process: it holds
   exactly the variables of the allow-list and those of the test's `Config::session_env`, and
   none of a variable the test set in `surfd`'s environment before starting it
   (`SURF_PROBE_SECRET=1`).
2. The same session's `/proc/<pid>/limits` shows each bound of Q11 with its soft and hard value,
   `/proc/<pid>/status` holds `NoNewPrivs:	1`, and `/proc/<pid>/exe` names the session program.
3. With the path of `setpriv` (then of `prlimit`) in `Config` pointing to a file that does not
   exist, an `open` that Q11 says needs them is answered as Q11 says, and afterwards no process
   whose parent is the process that runs the `surfd` under test exists (a scan of the fourth
   field of `/proc/*/stat`).
4. `bound_ends_session`: a stand-in session with `behaviour` `spin` under a CPU bound of 1 second
   ends within 10 s; `surfd` lists it `died`, and the answer to the next request for it has the
   head state `died`, the refusal `session-died` and the line `note ended: signal SIGXCPU bound
   cpu` (in Q11's words); a stand-in that ends with `behaviour` `exit` gets no such line.
   `surf-answer`'s tests parse and render the line, and refuse it with another signal word.
5. The tests of `surfd` and `surf-session` that ran before pass unchanged (`slot task test`), the
   end-to-end case `e2e::` of `surf-session` included.
6. `slot task check` exits 0.
7. `git diff --stat` names only the files of the Change.
8. Public surfaces: the story changes, in its last commit and after merging `main`, every
   public surface its change makes false, each number taken from the `slot task check` log of
   its tip. After that commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges`
   prints nothing for each old string below, as the story's base holds it (the strings and lines
   given are those of `main` at `29945f5`, 2026-11-24); each new number equals the one in that log;
   and the gate block, `README.md:584-625` and `website/docs/how-it-is-built.md:40-81`, holds that
   log's lines. The old strings of this story: the test count `1976 passed` (`README.md:11`,
   `docs/badges/make.sh:38`, `docs/badges/tests.svg`), `1,976` and `1,424, and 552`
   (`README.md:285`, `website/docs/status.md:23`, `website/docs/what-works.md:17`).
9. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; no model
   call and no network request in a test; each case starts one `surfd` and one session, nothing
   more per test; no Python; no generator with a committed output is added; a build or test job in
   the pool ends within 10 minutes or names its time with `slot --timeout`, at most 30 minutes; a
   test over 5 s is named with its time (`bound_ends_session` spends about 1 s of processor time
   by design).

## Out of scope

Seccomp, Landlock, namespaces and `PR_SET_DUMPABLE` (absent under S5 a); bounds on `surf <url>`
and `surf run`, which run no page script (S7); a process of its own for scripts (S2 = a); a
session that ends at a bound while running a script (`story:scripts-hostile-pages` shows it with
the behaviour of this story).

## Placement

After `story:scripts-decisions`; beside `story:webidl-reader`, `story:scripts-engine-dependency`,
`story:scripts-spec` and `story:html-script-hook`. `crates/surfd/src/daemon.rs` is also written by
the active `story:agent-surfd-session-limit`, `story:agent-surfd-idle-exit-race` and
`story:agent-token-hash` (none on `main` on 2026-11-24). This story takes no edge to them, since
it needs nothing they deliver; whichever of the four starts later is based on the merge of the
one that started before it.

## Why `epic:scripts-in-a-page`

The second condition of the security review, which the epic puts before the first session that
runs a page's script.
