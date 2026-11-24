---
format: aep.planning-md/3
id: story:session-process-hardening
kind: story
status: active
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
  path: crates/surfd/src/sessions.rs
- confidence: cited
  path: crates/surfd/src/view.rs
- confidence: cited
  path: crates/surfd/tests
- confidence: cited
  path: crates/view/src/json.rs
- confidence: cited
  path: crates/view/src/model.rs
- confidence: cited
  path: crates/view/tests/json.rs
- confidence: inferred
  path: crates/view/web/view.js
- confidence: cited
  path: docs/agent-answer.md
- confidence: cited
  path: docs/view-stream.md
revision: 25
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:12Z", actor: "human:operator", revision: 16}
- {from: "proposed", to: "active", at: "2026-11-24T12:08:07Z", actor: "human:operator", revision: 25, executor: "agent:coordinator"}
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

The operator session decided S5 on 2026-11-24 (relayed by the coordinating agent; recorded in
`docs/design/scripts.md`, Q11): option a. `surfd` starts `surf-session` through
`setpriv --no-new-privs` and `prlimit`, and refuses to start a session with scripts when either
program is missing; `#![forbid(unsafe_code)]` stays in every crate. Seccomp, Landlock, namespaces
and `PR_SET_DUMPABLE` are absent under a, and Q11 lists what stays open under it; option b (an
audited `unsafe` module) needs the owner's word and is not this story.

## Decisions taken

1. The environment (`docs/design/scripts.md`, Q10): `env_clear()`, then no variable of `surfd`'s
   own (the allow-list is empty), then `Config::session_env` for tests, so the tests that set
   `XDG_RUNTIME_DIR` or `STAND_IN_DIR` keep working. `RUST_MIN_STACK`, `RUST_BACKTRACE`,
   `LOCALDOMAIN`, `RES_OPTIONS`, `HOSTALIASES` and `TZ` are dropped for Q10's reasons.
   `current_dir("/")` and `process_group(0)` stay.
2. The start, in Q11's two forms, the program paths and the bounds in `Config` with these shipped
   values (a test can name others, so a test can start a session with a missing program or a bound
   of its own):
   - a session without scripts:
     `/usr/bin/setpriv --no-new-privs /usr/bin/prlimit --core=0:0 -- <session program> <identity> <socket>`:
     no new privileges and no core file, the two bounds that cannot end a session that works today;
   - a session with scripts:
     `/usr/bin/setpriv --no-new-privs /usr/bin/prlimit --as=4294967296:4294967296 --cpu=3600:3601 --nofile=64:64 --core=0:0 -- <session program> <identity> <socket>`:
     the address space at the 4 GiB backstop of Q11 (for the stories before
     `story:scripts-document-port`, which ships 2,147,483,648), CPU time 3,600 s soft and 3,601 s
     hard, 64 open files and no core file.
   The CPU bound is written `--cpu=<n>:<n+1>`: with soft and hard equal the kernel sends `SIGKILL` at
   the bound, with the soft one below the hard one `SIGXCPU` at the soft one (Q11's probe). No process
   bound (`RLIMIT_NPROC` counts every process of the user, Q11). Both programs replace themselves by
   `exec`, so the process `surfd` holds is the session's (Q11's probe).
3. Which form: `surfd` reads `arg scripts` from the `open` request it already decodes (`route`,
   `crates/surfd/src/daemon.rs:865-871`) before it starts the process; the value `true` takes the
   form with scripts, anything else the form without. The request is still passed on unchanged.
   `surfd` checks both programs at its start; when either is missing, a session without scripts
   starts as today, without either program, and an `open` that asks for scripts is answered with
   the action line `did refused scripts-unavailable` and exit status 3, and `docs/agent-answer.md`
   gains the row "`setpriv` or `prlimit` is missing, and a session with scripts runs only under
   both".
4. Nothing is read from the session to check the bounds; the tests read them from `/proc/<pid>/`
   from outside, as the epic says (lines 264-266).
5. A session that ends by a signal: when `surfd` reaps a session process, it reads the signal from
   the exit status (`std::os::unix::process::ExitStatusExt::signal`, no `unsafe`), keeps it with
   the entry, and still lists the session `died`; the head reads `died` and the action
   `did refused session-died`, as today. The answer gains one note line of the fixed list, in Q11's
   literal words:
   - a signal that names a bound: `note ended: signal <NAME> bound <word>`, `SIGXCPU` naming `cpu`
     (`note ended: signal SIGXCPU bound cpu`) and `SIGXFSZ` naming `fsize`;
   - a signal that names no bound (an allocation over the address space or a stack overflow, both
     `SIGABRT`): `note ended: signal <NAME>` (`note ended: signal SIGABRT`);
   - a session `surfd` killed itself (the `killed` flag of its process, set by `Process::kill`,
     `crates/surfd/src/daemon.rs:265`, `:275`; after the late read of 600 s, or at its stop) keeps
     today's answer: `did refused session-timeout` for the command that waited, the head `died` and
     `did refused session-died` for a later one, and no `note ended` line;
   - a session that ended with a status and no signal: no `note ended` line.
   `surf-answer` parses and renders both forms, and `docs/agent-answer.md` documents them beside the
   state `died` and the refusal `session-died`.
6. The live view lists a session that ended by a signal as died with the signal (Q11): the session
   summary of `docs/view-stream.md` gains the field `signal`, the signal's name (`"SIGXCPU"`) for
   such a session and `null` otherwise, and the page shows it beside `died`. The field's name is
   this story's choice; the note fixes only that the signal is shown.
7. The tests of decisions 5 and 6 need no script: the stand-in session gains the `behaviour` values
   `spin`, a loop on the processor after it answers `open`, and `abort`, which calls
   `std::process::abort` after it answers `open`; the test starts `spin` with the CPU bound
   `--cpu=1:2` (soft below hard, else the kernel sends `SIGKILL`).
8. The forge: the check workflow runs on `ubuntu-latest`, Ubuntu 26.04, whose essential package
   `util-linux` holds `/usr/bin/setpriv` and `/usr/bin/prlimit` (Q11); the story's first forge run
   checks `command -v setpriv prlimit` and the report quotes its output.

## Change

- `crates/surfd/src/daemon.rs`: decisions 1 to 3 and 5; `Config` gains the programs and the bounds
  of both forms.
- `crates/surfd/src/sessions.rs` and `crates/surfd/src/view.rs`: the signal kept with the entry and
  listed (decisions 5 and 6).
- `crates/surfd/src/link.rs`: its "Start" text, if it describes the environment or the command line
  (inferred).
- `crates/surfd/examples/stand_in_session.rs`: the `behaviour` values `spin` and `abort` and their
  lines in the head comment.
- `crates/answer/src/lib.rs` and its tests: the two note lines of decision 5.
- `docs/agent-answer.md`: the note lines and the refusal `scripts-unavailable` with its row, beside
  the state `died` and the refusal `session-died`.
- `crates/view/src/model.rs`, `crates/view/src/json.rs`, `crates/view/web/view.js`,
  `crates/view/tests/json.rs` and `docs/view-stream.md`: the field `signal` of decision 6.
- `crates/surfd/tests/`: the cases below, in the crate's one test binary.
- In the last commit, the public surfaces of acceptance 9.

## Acceptance

1. A test opens a session and reads `/proc/<pid>/environ` of its session process: it holds exactly
   the variables of the test's `Config::session_env` (the allow-list of Q10 is empty), and none of
   a variable the test set in `surfd`'s environment before starting it (`SURF_PROBE_SECRET=1`).
2. The limits per form, read from `/proc/<pid>/limits` of the session process: a session opened
   without scripts shows `Max core file size` 0 soft and 0 hard and no other bound of Q11 (its
   address space, CPU time and open files are those `surfd` runs with); a session opened with
   `arg scripts "true"` shows `Max address space` 4294967296 soft and hard, `Max cpu time` 3600
   soft and 3601 hard, `Max open files` 64 soft and hard, and `Max core file size` 0 soft and hard.
   Both show `NoNewPrivs:	1` in `/proc/<pid>/status`, and `/proc/<pid>/exe` names the session
   program.
3. With the path of `setpriv` (then of `prlimit`) in `Config` pointing to a file that does not
   exist: an `open` without scripts starts the session as today (`NoNewPrivs:	0`, no bound of Q11);
   an `open` with `arg scripts "true"` is answered with the action line
   `did refused scripts-unavailable` and exit status 3, and afterwards no process whose parent is
   the process that runs the `surfd` under test exists but the session opened without scripts (a
   scan of the fourth field of `/proc/*/stat`).
4. `bound_ends_session`: a stand-in session with `behaviour` `spin`, opened with scripts under the
   CPU bound `--cpu=1:2`, ends within 10 s; `surfd` lists it `died`, and the answer to the next
   request for it has the head state `died`, the action `did refused session-died` and the line
   `note ended: signal SIGXCPU bound cpu`. A stand-in with `behaviour` `abort` gets
   `note ended: signal SIGABRT`; a stand-in that ends with `behaviour` `exit` gets no `note ended`
   line; a stand-in that `surfd` kills itself (a behaviour after which `surfd` kills it, such as
   `break`, `crates/surfd/src/link.rs:85`) gets today's answer and no `note ended` line.
   `surf-answer`'s tests parse and render both forms of the line and refuse a `bound` word other
   than `cpu` and `fsize`.
5. The live view: after the cases of acceptance 4, `/api/sessions` lists the `spin` session `died`
   with `"signal":"SIGXCPU"`, the `abort` one with `"signal":"SIGABRT"`, and the `exit` one with
   `"signal":null`; `crates/view/tests/json.rs` holds the field in both values, and
   `docs/view-stream.md` names it in the session summary.
6. The tests of `surfd` and `surf-session` that ran before pass unchanged (`slot task test`), the
   end-to-end case `e2e::` of `surf-session` included; the report quotes
   `command -v setpriv prlimit` from the story's first forge run (decision 8).
7. `slot task check` exits 0.
8. `git diff --stat` names only the files of the Change.
9. Public surfaces: the story changes, in its last commit and after merging `main`, every public
   surface its change makes false, each number taken from the `slot task check` log of its tip.
   After that commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges` prints
   nothing for each old string below, as the story's base holds it (the strings and lines given are
   those of `main` at `29945f5`, 2026-11-24); each new number equals the one in that log; and the
   gate block, `README.md:584-625` and `website/docs/how-it-is-built.md:40-81`, holds that log's
   lines. The old strings of this story: the test count `1976 passed` (`README.md:11`,
   `docs/badges/make.sh:38`, `docs/badges/tests.svg`), `1,976` and `1,424, and 552`
   (`README.md:285`, `website/docs/status.md:23`, `website/docs/what-works.md:17`).
10. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; no model
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
The live view of decision 6 writes `docs/view-stream.md` and `crates/view/tests/json.rs`, which
`story:scripts-spec` also holds in its scope (only where its generated change reaches
`crates/view`); the two do not merge in the same wave, and whichever starts later is based on the
merge of the other.

## Why `epic:scripts-in-a-page`

The second condition of the security review, which the epic puts before the first session that
runs a page's script.
