---
format: aep.planning-md/3
id: story:session-core-limit-one
kind: story
status: draft
title: Start a session with a core limit of 1 in place of 0, so that its end by a signal is not handed to the host's core handler
summary: 'surfd starts a session with --core=0:0, which on a host whose core pattern is a pipe does not stop the dump: a session that ends by SIGABRT, SIGXCPU or SIGSEGV is waitable only after the handler took it (113 to 220 ms on a second host, up to 7 s at once on the build host; seven surfd tests red in a gate run). Both bound sets start --core=1:1, which writes no core file and starts no handler; the tests, docs and changelog move with it, and one case of first_line.rs is taken.'
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/surfd/src/daemon.rs
- confidence: cited
  path: crates/surfd/tests/adversary.rs
- confidence: cited
  path: crates/surfd/tests/adversary_2.rs
- confidence: cited
  path: crates/surfd/tests/first_line.rs
- confidence: cited
  path: crates/surfd/tests/hardening.rs
- confidence: cited
  path: crates/surfd/tests/main.rs
- confidence: cited
  path: docs/badges/make.sh
- confidence: cited
  path: docs/badges/tests.svg
- confidence: cited
  path: docs/design/scripts.md
- confidence: cited
  path: website/docs/how-it-is-built.md
- confidence: cited
  path: website/docs/what-works.md
revision: 15
---
## Context

Cites are lines of `a14e3b0` unless a commit is named.

`surfd` starts every session through `prlimit` with a core file size limit of 0: the constant
`SESSION_BOUNDS` for a session without scripts (`crates/surfd/src/daemon.rs:62`) and
`SCRIPT_SESSION_BOUNDS` for a session with scripts (`:67-72`, the value at `:71`). Their docs
say "no core file" (`:61`, `:65-66`) and name `--core=0:0` (`:110`, `:112-113`). The reason is
in `docs/design/scripts.md:966`: a core of a session holds its token and pages. `README.md:293`
promises "no core file".

A limit of 0 does not stop a core dump on a host whose core pattern is a pipe to a handler
(`/proc/sys/kernel/core_pattern` starts with `|`). The kernel hands the dump to the handler, and
the process is waitable only after the handler took it. The kernel ignores the limit for a pipe
pattern except when it is 1 (`fs/coredump.c`; from the kernel's source, not shown: no kernel
source was read for this story, only the effects below).

Measured on the build host (pipe pattern, at most 16 dumps at once), a shell under `prlimit
--core=0:0` that ends itself by a signal (`kill -s <name> $$`) is waitable after:

- `SIGABRT`, one shell at a time: 142 to 256 ms (three runs); `SIGTERM`: 3 to 15 ms.
- 24 shells at once: `SIGABRT` after at most 953 ms and 3,521 ms (two rounds), `SIGXCPU` after
  at most 3,145 ms and 7,063 ms, `SIGTERM` after at most 68 ms and 52 ms.

Measured on a second host (pipe pattern, at most 16 dumps at once, kernel 6.18.55), five shells
of each pair, one at a time, `prlimit --core=<limit> -- /bin/sh -c 'kill -s <name> $$'`:

```
limit  signal   "(core dumped)" in the status   waitable after
0:0    SIGABRT  5 of 5                          115 to 220 ms
0:0    SIGXCPU  5 of 5                          113 to 125 ms
0:0    SIGSEGV  5 of 5                          116 to 136 ms
0:0    SIGTERM  0 of 5                          4 to 8 ms
0:0    SIGUSR1  0 of 5                          4 to 10 ms
1:1    SIGABRT  0 of 5                          3 to 9 ms
1:1    SIGXCPU  0 of 5                          5 to 10 ms
1:1    SIGSEGV  0 of 5                          3 to 6 ms
1:1    SIGTERM  0 of 5                          4 to 9 ms
1:1    SIGUSR1  0 of 5                          4 to 8 ms
```

Under 0 the three signals that dump core wait for the handler (113 to 220 ms) and the status
says the dump was made. Under 1 they end as fast as the two signals that dump none, and no dump
is made. `surfd` reads the signal from the exit status (`daemon.rs:1695`, `status.signal()`),
which the dump flag does not change.

The cost lands on the tests of `crates/surfd/tests`, which end a process by such a signal and
wait `ALLOWED`, 5 s (`tests/common/mod.rs:37`). In a gate run of the first implementation of
`story:surfd-died-session-order` (a change of `crates/surfd/tests/adversary_2.rs` alone; load
average 52.77 at the job's start) the test step took 182.4 s, over its budget of 120 s, and the
nextest summary read `1840 tests run: 1832 passed, 8 failed, 3 skipped`. Seven of the eight
failed after 5.04 to 7.06 s and end a process by a signal that dumps core; the eighth is a case
of that story. In a later gate run (load average 72.43) the same seven passed in 0.26 to 0.89 s.
The time of a dump depends on the handler's load, which every test of every repository on the
host shares, and no limit a test sets bounds it.

The story removes the wait at its source: a session starts with a limit of 1.

## Decisions

1. Both constants start a session with `--core=1:1` in place of `--core=0:0`:
   `SESSION_BOUNDS` (`daemon.rs:62`) and `SCRIPT_SESSION_BOUNDS` (`:71`). The docs of both
   (`:61`, `:65-66`) and of the two `Config` fields (`:110`, `:112-113`) name the new value.
   Nothing else of `surfd` changes: the start chain, the other bounds and every answer stay.
2. What the limit promises stays "no core file" (`README.md:293`). The texts add what it now
   also gives, in one clause: the end of a session by a signal is seen without a wait for the
   host's core handler, and the handler is not started for a session. With a file pattern a
   limit of 1 writes no file either: that is from the kernel's source (`fs/coredump.c`: a limit
   below the format's smallest dump ends the dump), not shown; acceptance 6 shows it on a host
   whose pattern is a file, or says "not shown".
3. Every place that states `0` moves, and each hit of `git grep -n -e 'core=0' -e 'core file'`
   is in the Change or in "does not change" below with its reason. The tests that read `Max
   core file size` expect `1 1`.
4. The cases that end a process by a signal that dumps core (twelve at `a14e3b0`; the two of
   `adversary_2.rs` that end the shell by `SIGABRT` at `:141` and `:160` belong to
   `story:surfd-died-session-order`, which changes the signal). Each case ends with its time in
   the red gate run of the Context, then in the later run (all in `crates/surfd/tests`); a case
   over 5 s is named.

   Inside `surfd`'s bounds: each starts `surfd` with `config(&dir)` (`common/mod.rs:319-324`),
   which holds the shipped bounds and the real `setpriv` and `prlimit`. Decision 1 removes
   their wait; none needs an edit of its own.
   - `adversary.rs:144` `a_session_that_exits_with_status_137_names_no_signal`: `SIGABRT` of a
     child of the session script (the child inherits the limit); 7.055 s red, 0.273 s.
   - `adversary.rs:150` `..._status_152_names_no_signal`: the same; 7.053 s red, 0.263 s.
   - `adversary.rs:212` `two_clients_asking_about_one_dead_session_read_the_same_note`:
     `SIGABRT`; 1.995 s passed, 0.475 s.
   - `hardening.rs:304` `an_aborted_session_names_its_signal`: `SIGABRT`; 5.054 s red, 0.628 s.
   - `hardening.rs:392` `a_session_without_a_cpu_bound_ended_by_sigxcpu_names_no_bound`:
     `SIGXCPU` sent by the test; 5.044 s red, 0.846 s.
   - `hardening.rs:473` `a_session_with_scripts_that_aborts_before_its_first_byte_...`:
     `SIGABRT`; 5.057 s red, 0.892 s.
   - `hardening.rs:478` `a_session_without_scripts_that_aborts_before_its_first_byte_...`:
     `SIGABRT`; 5.075 s red, 0.821 s.
   - `hardening.rs:486` `a_session_that_aborts_while_it_answers_open_names_its_signal`:
     `SIGABRT`; 16.691 s passed (over 5 s), 0.873 s.
   - `adversary_2.rs:267` `sigxfsz_reads_no_bound`: `SIGXFSZ` sent by the test; 2.185 s
     passed, 0.108 s.

   Outside `surfd`'s bounds: the process runs under a bound set the test made or under none.
   The claim of each needs its signal, so the edit keeps the signal and lowers the limit:
   - `hardening.rs:278` `bound_ends_session`: `SIGXCPU` from the kernel at the soft bound of
     the test's own set `["--cpu=1:2", "--core=0:0"]` (`:282`); 8.727 s passed (over 5 s),
     1.269 s. The set becomes `["--cpu=1:2", "--core=1:1"]`. The claim (the kernel sends
     `SIGXCPU` at the soft bound and the answer names it with `bound cpu`) does not read the
     limit.
   - `adversary_2.rs:243`
     `sigxcpu_without_a_cpu_bound_of_surfd_reads_no_bound_also_with_scripts`: `SIGXCPU` sent by
     the test to a session under the test's own set `["--nofile=64:64", "--core=0:0"]`
     (`:246`); 2.341 s passed, 0.106 s. The set becomes `["--nofile=64:64", "--core=1:1"]`.
   - `adversary_2.rs:290` `sigxcpu_to_a_session_started_without_the_programs_reads_no_bound`:
     `SIGXCPU` sent by the test to a session that `surfd` starts without `setpriv` and
     `prlimit` (`config.setpriv` names no file, `:293`), so no bound applies; 5.063 s red,
     0.474 s. The session program becomes a sh script, as in the case at `:312` (`:315-322`,
     the helpers `script` and `quoted`): its first line is
     `/usr/bin/prlimit --core=1:1 --pid $$` and its second `exec <stand-in> "$@"`. `exec`
     keeps the process and its limit, so `dir.pid_of` names the stand-in as before. The claim
     (a session started without the programs reads no `bound` after `SIGXCPU`, which
     `daemon.rs:1444-1447` decides) stays: `surfd` still starts the session directly, and the
     limit is the test's own.
   No case asserts an elapsed time for any of this; the proof of the time is the probe of the
   Context.
5. The case `crates/surfd/tests/first_line.rs` of commit `345e55b` (one commit, two files, one
   case declared in `tests/main.rs`) is taken as it is. It holds: a session program that writes
   `x` and exits 3 gets status 3, the reason `the session ended or wrote another line before
   it reported ready` and the text empty (`first_line.rs:33-40`). The two cases of
   `adversary_2.rs` on a session that died by its own signal give the same answer whether
   `surfd` read the line or saw no byte, so they cannot tell the two paths apart. The one-line
   change that makes the new case red: `_ => Start::Other` at `daemon.rs:1653` replaced by
   `_ => Start::Nothing`; the case then fails at `first_line.rs:35-39` with the reason `cannot
   start <dir>/session: it ended by an exit before the session wrote its first byte`. The
   case ended in 0.039 s; at the mutant, 0.084 s. The case adds one test to the count of
   `crates/surfd` and one test file; the Change names every count that moves.
6. The guard that holds on every host is the limits line the tests read from the session's
   `/proc/<pid>/limits` (`Max core file size`, soft and hard, expected `1` and `1`). With
   `--core=0:0` back in `SESSION_BOUNDS` these are red: `hardening.rs:175`
   `a_session_without_scripts_has_no_core_file_and_no_new_privileges` (`:181`) and
   `adversary.rs:307` `an_arg_scripts_other_than_true_takes_the_form_without_scripts` (`:318`).
   With it back in `SCRIPT_SESSION_BOUNDS` these are red: `hardening.rs:196`
   `a_session_with_scripts_has_the_bounds_of_q11` (`:207`) and `adversary.rs:241`
   `the_wrappers_get_fixed_arguments_and_session_env_only`, whose expected arguments are
   `SCRIPT_BOUNDS` (`:28-33`, `:281`, `:293`). The test-made sets and the script's `prlimit` line
   of decision 4 are not product code: nothing holds them at `1:1` but the time of the cases.
7. No product file but `daemon.rs` changes, and `daemon.rs` changes the two values and their
   docs only. If the implementor finds that `surfd` answers wrongly for a session that ends by
   a signal under the new limit, that is a finding for a story of its own, with a failing
   test, and this story stops there.

## Change

- `crates/surfd/src/daemon.rs`: `:62` and `:71` become `--core=1:1`; the docs at `:61`,
  `:65-66`, `:110` and `:112-113` name `1:1` and add one clause (a core limit of 1 byte, so the
  host's core handler is not started for a session).
- `crates/surfd/tests/adversary.rs`: `SCRIPT_BOUNDS` at `:32` becomes `--core=1:1`; `:318`
  expects `("1", "1")`.
- `crates/surfd/tests/hardening.rs`: `:181` and `:207` expect `pair("1", "1")`; the set at
  `:282` becomes `["--cpu=1:2", "--core=1:1"]`.
- `crates/surfd/tests/adversary_2.rs`: the set at `:246` becomes `--core=1:1`; the session
  program of the case at `:290` is the script of decision 4 (adds lines to the file).
- `crates/surfd/tests/main.rs`: the line `mod first_line;` after `mod common;`.
- `crates/surfd/tests/first_line.rs`: the file of `345e55b`.
- `README.md:293`: "no core file" gets the clause of decision 2:
  `with no new privileges and no core file (a core limit of 1 byte, so the host's core handler
  is not started for a session)`. `website/docs/what-works.md:25` holds the same sentence and
  gets the same clause.
- `CHANGELOG.md:8`: under `## [Unreleased]` a `### Changed` section with one entry in the
  words of the entries of release `0.9.0` (`:34-47`): `surfd` starts each session process with
  a core file size limit of 1 byte (`prlimit --core=1:1`) in place of 0. A session still writes
  no core file; one that ends by a signal that dumps core is seen as ended without a wait for
  the host's core handler, which is not started for it.
- `docs/design/scripts.md`: the row at `:966` gets the starting value `1` and, in its rule
  column, the clause `a limit of 1 also keeps a session that ends by a signal away from the
  host's core handler`; the two start forms at `:972-973` and `:974-976` read `--core=1:1`. The
  probe note at `:878-880` is history and stays; one sentence follows it: `A limit of 0 does
  not stop a dump to a pipe handler: on a second host a shell ended by SIGABRT, SIGXCPU or
  SIGSEGV under --core=0:0 showed (core dumped) and was waitable after 113 to 220 ms, and
  under --core=1:1 showed none and was waitable after 3 to 10 ms, so surfd starts a session
  with --core=1:1.`
- The counts the new case moves, recounted in the last step on the story's tip (other units in
  work move the same numbers, so the new totals are not written here): `AGENTS.md:644-646`
  (`crates/surfd`: its test total and the per-file list, which gets `first_line` 1);
  the gate's totals by the rule of `AGENTS.md:2029-2035`: `README.md:285` (the total, the
  count of the step, the passed count) and the badge alternative text at `:11`, `README.md:612`,
  `:626` and `:661` (test files, tests), `website/docs/what-works.md:17`,
  `website/docs/how-it-is-built.md:56`, `:70` and `:105` (the same numbers of the recorded gate
  run), `docs/badges/make.sh` (the `tests.svg` value, near its end) and `docs/badges/tests.svg`
  (written by `sh docs/badges/make.sh .`). A number of a recorded run changes only with a new
  run of the gate.

Does not change, each with its reason:

- `CHANGELOG.md:42`: the entry of the released `0.9.0`; a release note is history.
- `docs/design/scripts.md:396` and `:843`: probes, each of one measured run made with
  `--core=0:0`; `:857`: the limits of the build node in a probe; `:980`: "no core file" stays
  true.
- `README.md:543` and `website/docs/roadmap.md:14`: the sentence on release `0.9.0`, which
  says "no core file"; it stays true.
- `crates/surfd/tests/adversary.rs:267`: `--core=unlimited` is text in a url of an `open`
  that the case sends to show `surfd` ignores it; no bound reads it.
- `crates/surfd/tests/hardening.rs:66-72`, `:172`: `BOUNDED` names the limit and a doc says
  "no core file"; both stay true.
- `crates/surfd/src/link.rs:12`: names `Config::session_bounds`, not a value.
- Planning artifacts that quote `--core=0:0` (`.engineering/planning/story/
  session-process-hardening.md:102-108`, `:175-178`, `story/swell-0-9-0.md:352`, and
  `epic/scripts-in-a-page.md:614`, `story/scripts-decisions.md:373` with "no core file"): a
  story that is done states the value of its time; they are not edited.
- `crates/agent`, `crates/agent-tasks`, `crates/session`: no test of them ends a session by a
  signal that dumps core and none passes a core bound (`git grep -n -i -E 'ABRT|XCPU|XFSZ|SEGV|
  abort\(\)|--core' a14e3b0 -- crates/agent crates/agent-tasks crates/session` prints no case).

## Acceptance

1. Guard, session without scripts: with `--core=0:0` written back at `daemon.rs:62` (made on
   the story's tip, run, taken back; `git diff --quiet -- crates/surfd/src` after), the two
   cases of decision 6 for that constant are red, each at its assertion on `Max core file
   size`.
2. Guard, session with scripts: with `--core=0:0` written back at `daemon.rs:71` (taken back
   after), the two cases of decision 6 for that constant are red at their assertions.
3. Text checks: `git grep -n -e '--core=0' -- crates README.md website` prints 7 lines at
   `a14e3b0` and nothing after the change (exit 1); `git grep -c -e 'core=0' --
   docs/design/scripts.md` prints 5 at `a14e3b0` (`:396`, `:843`, `:879`, `:972`, `:975`) and
   4 after (the three history lines and the new sentence, which names `--core=0:0` once);
   `git grep -c -e 'core=0' -- CHANGELOG.md` prints 1 before and after; `git grep -c -e 'core
   limit of 1 byte' -- README.md website/docs/what-works.md` prints 1 for each file after.
4. The twelve cases of decision 4, `first_line.rs` and the four guard cases pass in one pool
   job: `slot cargo nextest run -p surfd --locked` after `slot cargo build --workspace
   --all-targets --locked`, with `RUSTFLAGS="-D warnings"` and `SURF_BIN_DIR=<tree>/target/
   debug`. The report names the twelve with their times; a case over 5 s is named with its
   time. The story's diff holds no elapsed value: `git diff <base> -- crates |
   /usr/bin/grep -c -E '^\+.*(elapsed|Instant)'` prints 0.
5. The probe on the build host, in the session's scratch directory and not committed (sh, no
   Python, started without `slot`: it only measures): five shells at a time for each pair of
   `0:0` and `1:1` with `SIGABRT`, `SIGXCPU`, `SIGSEGV`, `SIGTERM` and `SIGUSR1`, as in the
   Context. Under `1:1` no status shows `(core dumped)` and each shell is waitable after a few
   milliseconds; under `0:0` the three core signals show it. Where `coredumpctl` exists,
   `coredumpctl list` shows an entry per shell of the `0:0` runs (listed with `COREFILE
   none`, as `docs/design/scripts.md:880` read it) and none for the `1:1` runs, which is the
   shown form of "the handler is not started"; if the handler lists none for either, the
   report says "not shown". The numbers go in the report.
6. A host whose core pattern is a file: the same probe script reads `/proc/sys/kernel/
   core_pattern`. If it names a relative file (no `|`, no `/`, no `%`), the script runs
   `/usr/bin/prlimit --core=unlimited -- /bin/sh -c 'kill -s ABRT $$'` in an empty directory
   (a file `core` appears: the control) and the same with `--core=1:1` (no file). If the
   build host's pattern is a pipe or any other form, the report says "not shown" for a file
   pattern.
7. The case of `adversary_2.rs:290` keeps its claim: with `.is_some_and` at `daemon.rs:1447`
   written `.is_none_or` (made on the tip, run, taken back; `git diff --quiet --
   crates/surfd/src` after), the case is red at its assertion on the answer text.
8. `first_line.rs`: green on the tip; with `_ => Start::Other` at `daemon.rs:1653` replaced by
   `_ => Start::Nothing` (taken back after) the case is red at `first_line.rs:35-39`; `git
   grep -c -e 'mod first_line;' -- crates/surfd/tests/main.rs` prints 1.
9. Counts: the number after `Summary` of the job of 4 (`N tests run`) is the total at
   `AGENTS.md:644` and the per-file numbers of its list add up to it; `sh docs/badges/make.sh
   .` leaves `docs/badges/tests.svg` with the gate's total; the numbers of the Change's last
   bullet equal the gate's lines of the tip.
10. No test over 60 s, the test step within 120 s, a test over 5 s named with its time. No pool
    job over 10 minutes unless it names its time with `slot --timeout` (at most 30 minutes),
    none started again unchanged. Every command that compiles or tests carries `slot`; none
    carries `--local`, `--here`, `SLOT_NO_REMOTE` or `SLOT_SHIM_OFF`. No Python in anything
    that stays in the repository. A script that only measures or waits starts without `slot`.
    No model call and no network request in a test.

## Out of scope

- The host's core pattern or its handler's settings.
- A change of the wait bounds of the tests (`ALLOWED`, `tests/common/mod.rs:37`).
- Making a process undumpable by a system call (`prctl`, `setrlimit` in a session): it needs
  `unsafe`, which the workspace forbids.
- The two cases of `story:surfd-died-session-order` (`adversary_2.rs:135-171`).
- Any other change of `surfd`'s bounds, answers, ready timeout or kill grace.

## Placement

Read with `git grep -l -E '^  path: <path>$' -- .engineering/planning/story` and the `status:`
line of each, on the plan tree at `f3b0605`:

- `crates/surfd/tests/adversary_2.rs`: `story:surfd-died-session-order`, active. It comes
  first: this story edits `:246` and the case at `:290` of the same file, the other the two
  cases at `:135-171`. Edge: this story `depends_on` it.
- `README.md`, `website/docs/what-works.md`, `website/docs/how-it-is-built.md`,
  `docs/design/scripts.md`, `docs/badges/make.sh` and `docs/badges/tests.svg`:
  `story:swell-0-9-0` and `story:scripts-dom-reading`, both active, and the proposed
  `story:scripts-parse-run`, `story:scripts-document-port` (also `daemon.rs`),
  `story:scripts-dom-attributes`, `-mutation`, `-selectors` and `story:scripts-html-conformance`
  (some of these files each). The edited lines differ from theirs; the shared numbers are
  recounted in this story's last step on its tip, after whichever of them merged.
- `AGENTS.md`: active `story:agent-tasks-left-process-wait`, `story:test-step-holds-with-tls`,
  `story:tests-without-wall-time-bounds`, `story:swell-0-9-0` and `story:scripts-dom-reading`.
  Only `:644-646` is edited here; the recount decides the rest.
- `crates/surfd/src/daemon.rs`: only `story:scripts-document-port`, proposed.
- `CHANGELOG.md`, `crates/surfd/tests/adversary.rs`, `hardening.rs`, `main.rs` and
  `first_line.rs`: no story that is not implemented has them in its scope. The text of
  `story:tests-without-wall-time-bounds` names `hardening.rs:532` for its ledger; this story
  adds no elapsed value and moves no line of that file, so no ledger line moves and no edge is
  needed.
