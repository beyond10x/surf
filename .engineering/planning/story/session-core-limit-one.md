---
format: aep.planning-md/3
id: story:session-core-limit-one
kind: story
status: draft
title: Start a session with a core limit of 1 in place of 0 (0 where surfd's own hard limit is 0), so that its end by a signal is seen without a wait for the host's core handler
summary: 'surfd starts a session with --core=0:0, which on a host whose core pattern is a pipe does not stop the dump: a session that ends by SIGABRT, SIGXCPU or SIGSEGV is waitable only after the host''s core handler took it (113 to 220 ms; seven surfd tests red in a gate run). surfd now reads its own hard core limit and starts a session with --core=1:1, or --core=0:0 where that limit is 0 (prlimit cannot raise it). Tests, docs and changelog move with it; one case of first_line.rs is taken.'
relations:
- depends_on: story:surfd-died-session-order
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
  path: crates/surfd/tests/common/mod.rs
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
  path: website/docs/status.md
- confidence: cited
  path: website/docs/what-works.md
revision: 20
---
## Context

Cites are lines of `b33aae7` (`main`, with commit `c91b313` of
`story:surfd-died-session-order`) unless a commit is named.

`surfd` starts every session through `prlimit` with a core file size limit of 0: the constant
`SESSION_BOUNDS` for a session without scripts (`crates/surfd/src/daemon.rs:62`) and
`SCRIPT_SESSION_BOUNDS` for a session with scripts (`:67-72`, the value at `:71`). The two reach
the start as the fields `Config::session_bounds` and `Config::script_session_bounds` (`:111`,
`:114`), which `Config::new` fills (`:154-155`); `Hardening::of` copies them (`:272-273`) and
`Hardening::command` joins the set of the form after `prlimit` (`:284`, `bounds` at `:291-297`).
A test that names its own set writes the field after `config(&dir)` (`common/mod.rs:319-324`).
The docs say "no core file" (`:61`, `:65-66`) and name `--core=0:0` (`:110`, `:113`). The reason
is in `docs/design/scripts.md:966`: a core of a session holds its token and pages.
`README.md:293` promises "no core file".

A limit of 0 does not stop a core dump on a host whose core pattern is a pipe to a handler
(`/proc/sys/kernel/core_pattern` starts with `|`). The kernel hands the dump to the handler, and
the process is waitable only after the handler took it. The kernel ignores the limit for a pipe
pattern except when it is 1 (`fs/coredump.c`; from the kernel's source, not shown: no kernel
source was read for this story, only the effects below).

First measure (a host with a pipe pattern, at most 16 dumps at once): a shell under `prlimit
--core=0:0` that ends itself by a signal (`kill -s <name> $$`) is waitable after:

- `SIGABRT`, one shell at a time: 142 to 256 ms (three runs); `SIGTERM`: 3 to 15 ms.
- 24 shells at once: `SIGABRT` after at most 953 ms and 3,521 ms (two rounds), `SIGXCPU` after
  at most 3,145 ms and 7,063 ms, `SIGTERM` after at most 68 ms and 52 ms.

Second measure (a host with a pipe pattern, at most 16 dumps at once, kernel 6.18.55),
five shells of each pair, one at a time, `prlimit --core=<limit> -- /bin/sh -c 'kill -s <name>
$$'`:

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
says the dump was made. Under 1 they end as fast as the two signals that dump none, and the
status says no dump was made. What is shown is the status and the wait; that the handler is not
started under 1 is the kernel's rule, not shown. `surfd` reads the signal from the exit status
(`daemon.rs:1695`, `status.signal()`), which the dump flag does not change.

The cost lands on the tests of `crates/surfd/tests`, which end a process by such a signal and
wait `ALLOWED`, 5 s (`tests/common/mod.rs:37`). In a gate run of the first implementation of
`story:surfd-died-session-order` (a change of `crates/surfd/tests/adversary_2.rs` alone; load
average 52.77 at the job's start) the test step took 182.4 s, over its budget of 120 s, and the
nextest summary read `1840 tests run: 1832 passed, 8 failed, 3 skipped`. Seven of the eight
failed after 5.04 to 7.06 s and end a process by a signal that dumps core; the eighth is a case
of that story. In a later gate run of another unit's change on the same code of `crates/surfd`
(load average 72.43) the same seven passed in 0.26 to 0.89 s. The time of a dump depends on the
handler's load, which every test of every repository on the host shares, and no limit a test
sets bounds it.

A limit of 1 cannot be set everywhere. `prlimit --core=1:1` raises the hard limit of a process
whose hard limit is 0, which a process without privilege cannot do. Shown: `/usr/bin/prlimit
--core=0:0 -- /usr/bin/prlimit --core=1:1 -- /usr/bin/true` prints `prlimit: failed to set the
CORE resource limit: Operation not permitted` and exits 1; with `--core=0:0` twice it exits 0.
Shown too: `--core=1:1` starts under a hard limit of `unlimited`, 1 and 5 (exit 0), and `prlimit
--core=5:1` is refused (`the soft limit CORE cannot exceed the hard limit`), so a soft value
above the hard value does not occur in `/proc/self/limits`. A `surfd` whose hard core limit is 0
(a service's `LimitCORE=0`, a `limits.conf` line `hard core 0`) would fail every session start
with `--core=1:1`: the start chain exits non-zero and `not_started` (`:1575-1594`) answers
`scripts-unavailable` with scripts and `cannot start ...` without. No session failed on that
before this story. So `surfd` chooses the limit by its own hard limit.

The story removes the wait at its source where the host allows: a session starts with a limit
of 1, or of 0 where `surfd`'s own hard limit is 0.

## Decisions

1. `surfd` reads its own hard core limit and starts a session with `--core=1:1` when that value
   is `unlimited` or a number of at least 1, and with `--core=0:0` when it is 0 or the line
   cannot be read or parsed. `--core=0:0` can always be set, so no session fails to start by
   this choice. The form, the smallest that does it, all in `crates/surfd/src/daemon.rs`:
   - Two constants, `CORE_BOUND_ONE` (`"--core=1:1"`) and `CORE_BOUND_ZERO` (`"--core=0:0"`).
   - `fn hard_core_limit(limits: &str) -> Option<u64>`, pure over a text such as
     `/proc/self/limits`: the line that starts with `Max core file size` and two spaces, its
     second value (`unlimited` is `u64::MAX`, a number is itself); `None` for a text without the
     line, a line with fewer than two values, or a second value that is no number or is above
     `u64::MAX`. The soft value is not read; a soft value above the hard one cannot occur (shown
     above).
   - `fn core_bound(limits: &str) -> &'static str`, pure: `CORE_BOUND_ONE` for `Some(hard)` with
     `hard >= 1`, else `CORE_BOUND_ZERO`.
   - `SESSION_BOUNDS` (`:62`) is taken out, and `SCRIPT_SESSION_BOUNDS` (`:67-72`) becomes
     `[&str; 3]`: the `--as`, `--cpu` and `--nofile` items, without the core item.
   - `Config::new` (`:143`) reads `/proc/self/limits` (an unreadable file is an empty text),
     takes `core_bound` of it, and fills `session_bounds` with that item alone and
     `script_session_bounds` with the three items and then that item, so the arguments of
     `prlimit` keep today's order, the core item last.
   Where it is read: once, in `Config::new`, which `surfd`'s `main` calls once for its one daemon
   (`crates/surfd/src/main.rs:43`). Why there and not per start: the two `Config` fields are
   where a test already names its own set and where it can read the choice; `Hardening` copies
   what it is given and needs no second source; a read per start would need the form in
   `Hardening` as well. What it does not follow: a hard limit that an outside `prlimit --pid`
   lowers to 0 after the start. A start then fails as it does under a hard limit of 0 that the
   choice did not see; the daemon is restarted. That is not changed here. A test that lowers
   its own hard limit builds its `Config` after it (decision 7).
2. What the limit promises stays "no core file" (`README.md:293`). The shipped texts (the docs
   of the two constants and of the two `Config` fields, `README.md`, `website/docs/what-works.md`,
   `CHANGELOG.md`, `docs/design/scripts.md`) say what is shown: a session that ends by a signal
   is seen without the wait for the host's core handler, and its status carries no dump. They
   do not say that the handler is not started: that is the kernel's rule, which only this
   Context gives, marked "not shown". The same holds for a file pattern: "no core file" stays
   the README's promise; it rests on the kernel's rule for a limit below the format's smallest
   dump (`fs/coredump.c`, not shown) and on the case of decision 8 where a host has a file
   pattern. Each text says in one clause that where `surfd`'s own hard core limit is 0 a
   session keeps a limit of 0 and, with a pipe pattern, the wait for the handler. A process of
   the session can lower its soft limit to 0 (allowed under a hard limit of 1) and then dumps
   through a pipe handler again, with the wait; it cannot raise the limit above 1.
   `docs/design/scripts.md` says that in one clause.
3. Every place that states `0` moves, and each hit of `git grep -n -e 'core=0' -e 'core file'`
   is in the Change or in "does not change" below with its reason. The tests that read `Max
   core file size` expect `1 1`, or `0 0` where their own process's hard limit is 0 (decision
   6). The three hits of `git grep -n -e '--core=0' b33aae7 -- crates/surfd/tests`
   (`adversary.rs:32`, `adversary_2.rs:306`, `hardening.rs:282`) are all edited by the Change;
   after it the one literal left in `crates/surfd/tests` is the constant of decision 6.
4. The cases that end a process by a signal that dumps core (twelve at `b33aae7`; the two of
   `adversary_2.rs` that ended the shell by `SIGABRT` now end it by `SIGUSR1`
   (`another_first_line_after_the_session_died_by_its_own_signal_names_it`, `:147`, and
   `a_ready_timeout_after_the_session_died_by_its_own_signal_names_it`, `:203`) and are no case
   of this story). Each case ends with its time in the red gate run of the Context, then in the
   later run (all in `crates/surfd/tests`); a case over 5 s is named. Each case is named by its
   function; `adversary_2.rs` lines are those of `b33aae7` (60 later than at `a14e3b0`).

   Inside `surfd`'s bounds: each starts `surfd` with `config(&dir)`, which holds the shipped
   bounds and the real `setpriv` and `prlimit`. Decision 1 removes their wait; none needs an
   edit of its own.
   - `adversary.rs:144` `a_session_that_exits_with_status_137_names_no_signal`: `SIGABRT` of a
     child of the session script (the child inherits the limit); 7.055 s red, 0.273 s.
   - `adversary.rs:150` `a_session_that_exits_with_status_152_names_no_signal`: the same;
     7.053 s red, 0.263 s.
   - `adversary.rs:212` `two_clients_asking_about_one_dead_session_read_the_same_note`:
     `SIGABRT`; 1.995 s passed, 0.475 s.
   - `hardening.rs:304` `an_aborted_session_names_its_signal`: `SIGABRT`; 5.054 s red, 0.628 s.
   - `hardening.rs:392` `a_session_without_a_cpu_bound_ended_by_sigxcpu_names_no_bound`:
     `SIGXCPU` sent by the test; 5.044 s red, 0.846 s.
   - `hardening.rs:473`
     `a_session_with_scripts_that_aborts_before_its_first_byte_names_its_signal`: `SIGABRT`;
     5.057 s red, 0.892 s.
   - `hardening.rs:478`
     `a_session_without_scripts_that_aborts_before_its_first_byte_names_its_signal`: `SIGABRT`;
     5.075 s red, 0.821 s.
   - `hardening.rs:486` `a_session_that_aborts_while_it_answers_open_names_its_signal`:
     `SIGABRT`; 16.691 s passed (over 5 s), 0.873 s.
   - `adversary_2.rs:327` `sigxfsz_reads_no_bound`: `SIGXFSZ` sent by the test; 2.185 s
     passed, 0.108 s.

   Outside `surfd`'s bounds: the process runs under a bound set the test made or under none.
   The claim of each needs its signal, so the edit keeps the signal and takes the core item
   `surfd` chose, `config.session_bounds[0]` (decision 1: that field holds that item alone):
   - `hardening.rs:278` `bound_ends_session`: `SIGXCPU` from the kernel at the soft bound of
     the test's own set `["--cpu=1:2", "--core=0:0"]` (`:282`); 8.727 s passed (over 5 s),
     1.269 s. The set becomes `["--cpu=1:2", <that item>]`, on the same line (99 columns, as
     counted: `config.session_bounds[0].clone()` is the item). The claim (the kernel sends
     `SIGXCPU` at the soft bound and the answer names it with `bound cpu`) does not read the
     limit.
   - `adversary_2.rs:303`
     `sigxcpu_without_a_cpu_bound_of_surfd_reads_no_bound_also_with_scripts`: `SIGXCPU` sent
     by the test to a session under the test's own set `["--nofile=64:64", "--core=0:0"]`
     (`:306`); 2.341 s passed, 0.106 s. The set becomes `["--nofile=64:64", <that item>]`;
     the line is then over 100 columns and `rustfmt` splits it.
   - `adversary_2.rs:350` `sigxcpu_to_a_session_started_without_the_programs_reads_no_bound`:
     `SIGXCPU` sent by the test to a session that `surfd` starts without `setpriv` and
     `prlimit` (`config.setpriv` names no file, `:353`), so no bound applies; 5.063 s red,
     0.474 s. The session program becomes a sh script, as in the case at `:372` (`:375-382`,
     the helpers `script` and `quoted`, `:25`, `:33`): its first line is
     `/usr/bin/prlimit <that item> --pid $$` and its second `exec <stand-in> "$@"`. `exec`
     keeps the process and its limit, so `dir.pid_of` names the stand-in as before. The claim
     (a session started without the programs reads no `bound` after `SIGXCPU`, which
     `daemon.rs:1444-1447` decides) stays: `surfd` still starts the session directly, and the
     limit is the test's own, which the doc comment of the case (`:345-348`) says.
   A line that `rustfmt` splits moves the lines after it in its file; no ledger of a clock names
   a line of these files on `main`. New cases go after the last case of `hardening.rs`
   (`:548` is the file's last line), so no line of that file moves for them.
   No case added or edited by this story adds an elapsed value; `bound_ends_session` keeps the
   one it has (`:284-287`, `began.elapsed() < BOUND_ENDS_WITHIN`). The proof of the time is the
   probe of acceptance 7.
5. The case `crates/surfd/tests/first_line.rs` of commit `345e55b` (one commit, two files, one
   case declared in `tests/main.rs`) is taken as it is, and travels in this story. Why here:
   this story adds cases of its own (decisions 1, 7 and 8), so it recounts the numbers of the
   gate anyway; the case lies in the same directory and came from the same review; a story of
   its own would recount the same files a second time. It holds: a session program that writes
   `x` and exits 3 gets status 3, the reason `the session ended or wrote another line before
   it reported ready` and the text empty (`first_line.rs:33-40`). The two cases of
   `adversary_2.rs` on a session that died by its own signal give the same answer whether
   `surfd` read the line or saw no byte, so they cannot tell the two paths apart. The one-line
   change that makes the new case red: the arm `_ => Start::Other` (`daemon.rs:1653`) replaced
   by `_ => Start::Nothing`; the case then fails at `first_line.rs:35-39` with the reason
   `cannot start <dir>/session: it ended by an exit before the session wrote its first byte`.
   The case ended in 0.039 s; at the mutant, 0.084 s. The case adds one test to the count of
   `crates/surfd` and one test file; the Change names every count that moves.
6. The guards. The limits line a test reads from the session's `/proc/<pid>/limits` (`Max core
   file size`) is expected by the hard limit of the test's own process: `1 1` where it is
   `unlimited` or at least 1, `0 0` where it is 0. `crates/surfd/tests/common/mod.rs` gets two
   items, the smallest form:
   - the constant `CORE_ARGUMENT_ZERO` (`"--core=0:0"`);
   - one helper that reads the test process's `/proc/self/limits` with a parse of its own (the
     hard value is `0` or not; a line it cannot read panics) and gives the pair and the
     argument: `1 1` and `--core=1:1`, or `0 0` and `CORE_ARGUMENT_ZERO` where the hard value
     is 0. It does not call `daemon.rs`.
   A constant and not a function of the hard limit: the helper already answers for the host's
   limit, and the case of decision 7 needs the zero argument whatever the host's limit is. The
   constant is the one place the tests write `--core=0`; no comment of a test file writes it
   either (acceptance 20). The story does not know the hard limit of a forge's job; a defect
   that makes the choice answer `--core=0:0` always holds in two ways. The pure cases of
   decision 1 are red on every host, whatever its hard limit. The limits-line cases are red on
   a host whose hard limit is at least 1 and hold nothing on a host whose hard limit is 0, which
   the report says (acceptance 4). The four limits-line cases:
   - `hardening.rs:175` `a_session_without_scripts_has_no_core_file_and_no_new_privileges`
     (`:181`) and `adversary.rs:307` `an_arg_scripts_other_than_true_takes_the_form_without_
     scripts` (`:318`): the form without scripts.
   - `hardening.rs:196` `a_session_with_scripts_has_the_bounds_of_q11` (`:207`) and
     `adversary.rs:242` `the_wrappers_get_fixed_arguments_and_session_env_only`, whose expected
     arguments are `SCRIPT_BOUNDS` (`:28-33`, `:281`, `:292-293`): the form with scripts.
   `SCRIPT_BOUNDS` becomes a function that builds the four arguments with the helper's core
   item, so `adversary.rs:32` holds no literal. The sets a test makes itself and the script's
   `prlimit` line of decision 4 are not product code: nothing holds them but the time of the
   cases.
7. A hard limit of 0 on the whole path. One case in `hardening.rs` (after the last case),
   `a_session_keeps_a_core_limit_of_0_under_a_hard_limit_of_0`, in the form of
   `adversary.rs:364-386` (`a_prlimit_that_fails_before_the_session_answers_scripts_unavailable`
   lowers the test process's own `--cpu` hard limit with `/usr/bin/prlimit --pid <own>` and
   builds its `Config` after): it lowers the test process's own core limit with
   `/usr/bin/prlimit --pid <own>` and `CORE_ARGUMENT_ZERO` (the constant of decision 6, so
   `hardening.rs` writes no such literal), asserts its `/proc/self/limits` reads `0 0`, builds
   `config(&dir)` and starts `surfd`, and opens a session without scripts and one with scripts:
   each `open` answers status 0, and the session's `/proc/<pid>/limits` reads `0 0`. It needs
   no `unsafe` and does not start the built `surfd` under `prlimit`: `main` only calls
   `Config::new`, which the case calls too. Like the case at `adversary.rs:364` it assumes one
   process per test, which cargo-nextest gives and the gate uses. With the choice fixed at
   `--core=1:1` the case is red on every host, at the `open` (the start chain fails) or at its
   limits assertion (a runner with `CAP_SYS_RESOURCE` can raise the limit, and the session
   then reads `1 1`).
8. The effect on the host the tests run on. One case in `hardening.rs` (after the last case),
   `a_process_under_the_core_bound_surfd_chose_ends_by_abort_without_a_dump`: it takes the
   argument from `Config::new(None).session_bounds` (the item the daemon's start uses, `pub`
   at `daemon.rs:111`), runs `/usr/bin/prlimit <that argument> -- /bin/sh -c 'kill -s ABRT $$'`
   in a new empty directory under the case's `TestDir`, and asserts that the status is the
   signal 6 (`std::os::unix::process::ExitStatusExt::signal`), that `core_dumped()` is false,
   and that the directory holds no entry. It prints one line to the standard error: the
   argument, the first line of `/proc/sys/kernel/core_pattern` and the hard limit it read. It
   asserts no elapsed time. Where it holds something: a pipe pattern and a hard limit of at
   least 1 (a defect that chooses `--core=0:0` always makes `core_dumped()` true; shown by the
   table of the Context). Where it holds nothing: a hard limit of 0 (the argument is
   `--core=0:0`; the case prints "not shown: hard core limit 0" and ends), and a file pattern
   with a limit of 0 (no file and no flag are expected either: not shown). Under a file
   pattern and a limit of 1 it is the check of the README's "no core file" on a relative
   pattern (no flag, empty directory); it has no control (a run under a larger limit that
   writes a file), so a green run shows no file, not that the host would write one; for an
   absolute file pattern it shows nothing about a file. The kernel's rule stays "not shown". A
   red run on a host with a file pattern is reported with the pattern and the kernel, and is no
   failure of the choice: what the kernel does there is not shown. The doc comment of the case
   names where it holds something and where it holds nothing, in the words of this decision.
9. No product file but `daemon.rs` changes, and `daemon.rs` changes the choice and the
   constants, their docs and the three items of the Context. If the implementor finds that
   `surfd` answers wrongly for a session that ends by a signal under the new limit, that is a
   finding for a story of its own, with a failing test, and this story stops there.

## Change

- `crates/surfd/src/daemon.rs`: decision 1 (the two constants, `hard_core_limit`, `core_bound`,
  `SESSION_BOUNDS` out, `SCRIPT_SESSION_BOUNDS` of three items, the read in `Config::new`). The
  doc of `CORE_BOUND_ONE` says: a core limit of 1 byte, so that a session that ends by a signal
  is seen without the wait for the host's core handler, and its status carries no dump; the doc
  of `CORE_BOUND_ZERO` says it is the choice where `surfd`'s own hard limit is 0, which cannot
  be raised, and that the wait stays there. The docs at `:64-66` (the constant's "and no core
  file"), `:110` and `:112-113` name the choice. At the end of the file a `#[cfg(test)] mod
  tests` with two unit cases over strings: `hard_core_limit_reads_the_second_value_of_the_line`
  and `a_hard_limit_of_at_least_1_chooses_core_1_1_and_anything_else_core_0_0`. Both run over
  these ten texts, the lines as the kernel writes them (the name padded to 26 columns, each
  value to 21, then the units; rows 6 and 7 are whole texts of several lines):

  ```
  1  Max core file size        unlimited            unlimited            bytes
  2  Max core file size        0                    0                    bytes
  3  Max core file size        0                    unlimited            bytes
  4  Max core file size        1                    1                    bytes
  5  Max core file size        0                    18446744073709551616 bytes
  6  Limit                     Soft Limit           Hard Limit           Units
     Max cpu time              unlimited            unlimited            seconds
     Max core file size        0                    unlimited            bytes
     Max open files            1024                 524288               files
  7  Limit                     Soft Limit           Hard Limit           Units
  8  (the empty text)
  9  Max core file size        0
  10 Max core file size        0                    many                 bytes
  ```

  The first case: `hard_core_limit` answers `Some(u64::MAX)` for rows 1, 3 and 6 (in row 6 the
  other rows hold other values, so a read of the wrong row is red), `Some(0)` for row 2,
  `Some(1)` for row 4, and `None` for row 5 (a hard value above `u64::MAX`), row 7 (no core
  line), row 8, row 9 (one value) and row 10 (a second value that is a word). The second:
  `core_bound` answers `CORE_BOUND_ONE` for rows 1, 3, 4 and 6, and `CORE_BOUND_ZERO` for rows
  2, 5, 7, 8, 9 and 10. The assertion of each row carries the row's number and text in its
  message, so a red run names the row. No row has a soft value above the hard one: it cannot
  occur. `test-layout` allows these cases: `taskfiles/test-layout.sh` fails on
  `#[test]` only in a target with `test = false` (the `[[bin]]` of `crates/surfd/Cargo.toml`)
  and in generated packages, the library of `surfd` has no such key, `crates/surfd/src/view.rs:
  631` holds a `#[cfg(test)] mod tests` with four `#[test]`, and `taskfiles/test-layout.yml`
  lists `#[test]` in a module of the library of `surfd` among its right layouts.
- `crates/surfd/tests/common/mod.rs`: the constant `CORE_ARGUMENT_ZERO` and the helper of
  decision 6 (the pair of limits and the core argument, from the test process's own hard
  limit).
- `crates/surfd/tests/adversary.rs`: `SCRIPT_BOUNDS` (`:28-33`) becomes a function over the
  helper; `:281` uses it; `:318` expects the helper's pair.
- `crates/surfd/tests/hardening.rs`: `:181` and `:207` expect the helper's pair; the set at
  `:282` takes the item of `config.session_bounds[0]` (decision 4); the two cases of decisions
  7 and 8 after the last case; the first takes `CORE_ARGUMENT_ZERO` from `common`.
- `crates/surfd/tests/adversary_2.rs`: the set at `:306` takes that item; the session program
  of the case at `:350` is the script of decision 4 (adds lines to the file).
- `crates/surfd/tests/main.rs`: the line `mod first_line;` after `mod common;`.
- `crates/surfd/tests/first_line.rs`: the file of `345e55b`.
- `README.md:293-294`: after "...without them it starts the session directly." one sentence:
  `Its core file size limit is 1 byte, so a session that ends by a signal is seen without the
  wait for the host's core handler; where surfd's own hard core limit is 0 it keeps a limit of
  0, and the wait stays.` `website/docs/what-works.md:25-26` holds the same bullet and gets the
  same sentence. The bullet sits under the heading "shipped in release `0.9.0`"; the sentence is
  newer than that release and stays in the bullet until the next release.
- `CHANGELOG.md:8`: under `## [Unreleased]` a `### Changed` section with one entry in the
  words of the entries of release `0.9.0` (`:34-47`): `surfd` starts each session process with
  a core file size limit of 1 byte (`prlimit --core=1:1`) in place of 0, and with 0
  (`--core=0:0`) where its own hard core limit is 0, which `prlimit` cannot raise. A session
  still writes no core file. One that ends by a signal that dumps core is seen without the wait
  for the host's core handler, and its status carries no dump; where the hard limit is 0 the
  wait stays. Each of `--core=1:1` and `--core=0:0` is named once, on one line.
- `docs/design/scripts.md`: the row at `:966` gets the starting value `1` (`0` where `surfd`'s
  own hard limit is 0) and, in its rule column, the clause `a limit of 1 lets a session that
  ends by a signal be seen without the wait for the host's core handler`; the two start forms at
  `:972-973` and `:974-976` read `--core=1:1`, and one sentence follows them: `Where surfd's own
  hard core limit is 0 the item is --core=0:0, because prlimit cannot raise a hard limit; the
  session keeps a limit of 0 and, with a pipe pattern, the wait. A process of the session can
  lower its soft limit to 0 and then dumps through a pipe handler again; it cannot raise the
  limit above 1.` The probe note at `:878-880` is history and stays; one sentence follows it:
  `A limit of 0 does not stop a dump to a pipe handler: on a host with a pipe pattern a shell
  ended by SIGABRT, SIGXCPU or SIGSEGV under --core=0:0 showed (core dumped) and was waitable
  after 113 to 220 ms, and under --core=1:1 showed none and was waitable after 3 to 10 ms, so
  surfd starts a session with --core=1:1 where its hard limit allows.`
- The counts the new cases move, recounted in the last step on the story's tip after `main` is
  merged into it (other units in work move the same numbers, so the new totals are not written
  here): `AGENTS.md:644-646` (`crates/surfd`: its test total and the per-file list, which gets
  `first_line` 1, `hardening` 2 more, and the unit tests of `src/daemon.rs` next to the 4 of
  `src/view.rs`); the gate's totals by the rule of `AGENTS.md:2029-2035`: `README.md:285` (the
  total, the count of the step, the passed count) and the badge alternative text at `:11`,
  `README.md:612`, `:626` and `:661` (test files, tests), `website/docs/what-works.md:17`,
  `website/docs/how-it-is-built.md:56`, `:70` and `:105`, and `website/docs/status.md:31` (the
  row "tests": the same numbers of the recorded gate run), `docs/badges/make.sh` (the
  `tests.svg` value, `:38`) and `docs/badges/tests.svg` (written by `sh docs/badges/make.sh .`).
  A number of a recorded run changes only with a new run of the gate.

Does not change, each with its reason:

- `CHANGELOG.md:42`: the entry of the released `0.9.0`; a release note is history.
- `docs/design/scripts.md:396` and `:843`: probes, each of one measured run made with
  `--core=0:0`; `:857`: the limits of a node in a probe; `:980`: "no core file" stays
  true.
- `README.md:543` and `website/docs/roadmap.md:14`: the sentence on release `0.9.0`, which
  says "no core file"; it stays true.
- `crates/surfd/tests/adversary.rs:267`: `--core=unlimited` is text in a url of an `open`
  that the case sends to show `surfd` ignores it; no bound reads it.
- `crates/surfd/tests/hardening.rs:66-72`, `:172`: `BOUNDED` names the limit and a doc says
  "no core file"; both stay true.
- `crates/surfd/src/link.rs:12-13`: names `Config::session_bounds` and
  `Config::script_session_bounds`, not a value.
- Planning artifacts that quote `--core=0:0` (`.engineering/planning/story/
  session-process-hardening.md:102-108`, `:175-178`, `story/swell-0-9-0.md:352`, and
  `epic/scripts-in-a-page.md:614`, `story/scripts-decisions.md:373` with "no core file"): a
  story that is done states the value of its time; they are not edited.
- `crates/agent`, `crates/agent-tasks`, `crates/session`: no test of them ends a session by a
  signal that dumps core and none passes a core bound (`git grep -n -i -E 'ABRT|XCPU|XFSZ|SEGV|
  abort\(\)|--core' b33aae7 -- crates/agent crates/agent-tasks crates/session` prints no case).

## Acceptance

Mutants are made on the story's tip, run, and taken back (`git diff --quiet --
crates/surfd/src` after). Each run is one pool job: `slot cargo nextest run -p surfd --locked
-E '<the named cases>'` after `slot cargo build --workspace --all-targets --locked`. Where an
item says "prints nothing (exit 1)" the command is a `git grep -c` with no match; where it
prints a path and a count, that is the line as printed.

1. The two unit cases of `src/daemon.rs` (decision 1) pass: `slot cargo nextest run -p surfd
   --lib --locked` names both as passed.
2. Guard, choice always `0:0`, on every host: with `core_bound` made to answer
   `CORE_BOUND_ZERO` always, the case `a_hard_limit_of_at_least_1_chooses_core_1_1_and_
   anything_else_core_0_0` is red, and the message of its failed assertion carries the text of
   a row that answers `CORE_BOUND_ONE` (row 1, 3, 4 or 6 of the Change).
3. Guard, choice always `1:1`, on every host: with `core_bound` made to answer
   `CORE_BOUND_ONE` always, the same case is red, and the message of its failed assertion
   carries the text of a row that answers `CORE_BOUND_ZERO` (row 2, 5, 7, 8, 9 or 10).
4. Guard, the sessions: with `core_bound` made to answer `CORE_BOUND_ZERO` always, the four
   limits-line cases of decision 6 are red on a host whose hard limit is at least 1: three at
   their assertion on `Max core file size` (`hardening.rs:181` and `:207`, `adversary.rs:318`),
   and `the_wrappers_get_fixed_arguments_and_session_env_only` at its assertion on the recorded
   arguments (`adversary.rs:292`, `:293`: both recorded sets hold the core item). The report
   says which hard limit the run had (the line of acceptance 6) and, where the four stay green
   because it is 0, says that they hold nothing on that host.
5. Whole path under a hard limit of 0: the case of decision 7 passes on the tip, and with
   `core_bound` made to answer `CORE_BOUND_ONE` always it is red, at its `open` or at its
   limits assertion, on every host.
6. The case of decision 8 passes on the tip. The report prints its standard-error line (run
   with `--success-output final`) and says where it holds something and where nothing, by the
   terms of decision 8. With `core_bound` made to answer `CORE_BOUND_ZERO` always it is red
   (`core_dumped()` true) on a host with a pipe pattern and a hard limit of at least 1; on
   a host without both the report says "holds nothing" and why. On a host with a file pattern a red
   run of the case, on the tip or at the mutant, is reported with the pattern and the kernel
   and is no failure of this item: what the kernel does there is not shown.
7. The probe, a script written in sh in the session's scratch directory, not committed, and
   started without `slot` (it only measures); it runs on the host of whoever builds the story,
   and the report names that host by its role and prints `core_pattern`, `core_pipe_limit`, the
   kernel and the line `Max core file size` of the probe's own shell. It starts by reading
   `core_pattern`: when the pattern does not start with `|` the item reads "not shown: no pipe
   pattern on this host" and ends there; when the hard limit of the probe's shell is 0 it
   reads "not shown: hard core limit 0" and ends. Otherwise, one shell at a time, five rounds
   for each of `SIGABRT`, `SIGXCPU`, `SIGSEGV`, `SIGTERM` and `SIGUSR1` under `--core=0:0` and
   under `--core=1:1`, as the Context's table: each shell prints its pid
   (`/bin/sh -c 'echo "pid $$" >&2; kill -s <name> $$'`) and the wait in ms. When no round of
   the three signals that dump core shows "(core dumped)" under `--core=0:0` the item reads
   "not shown: the handler of this host takes no dump" and ends, with the table printed.
   Otherwise the criterion is the status: under `1:1` no round shows "(core dumped)"; under
   `0:0` every round of `SIGABRT`, `SIGXCPU` and `SIGSEGV` does. The times are printed and not
   judged. `coredumpctl` is no part of the criterion; if the report lists it, its entries are
   matched by the pids the probe printed.
8. The case `sigxcpu_to_a_session_started_without_the_programs_reads_no_bound` keeps its claim:
   with the `.is_some_and` of the call `.is_some_and(|hardening| hardening.sets_cpu(scripts))`
   (the field `cpu_bound`, `daemon.rs:1444-1447` at `b33aae7`) written `.is_none_or`, it is red
   at its assertion on the answer text.
9. The script line of that case: `git grep -c -F -e '--pid $$' --
   crates/surfd/tests/adversary_2.rs` prints nothing (exit 1) at `b33aae7` and
   `crates/surfd/tests/adversary_2.rs:1` after the change.
10. `first_line.rs` keeps its claim: with the arm `_ => Start::Other` (the third arm of the
    `match` that makes `start`, `daemon.rs:1653` at `b33aae7`) replaced by `_ =>
    Start::Nothing`, the case is red at `first_line.rs:35-39`.
11. The case is declared: `git grep -c -e 'mod first_line;' -- crates/surfd/tests/main.rs`
    prints nothing (exit 1) at `b33aae7` and `crates/surfd/tests/main.rs:1` after.
12. The twelve cases of decision 4, `first_line.rs` and the four limits-line cases pass in one
    pool job: `slot cargo nextest run -p surfd --locked` after `slot cargo build --workspace
    --all-targets --locked`, with `RUSTFLAGS="-D warnings"` and `SURF_BIN_DIR=<tree>/target/
    debug`. The report names the twelve with their times; a case over 5 s is named with its
    time.
13. The story's own commits add no elapsed value: with the base the parent of the story's first
    commit (`b33aae7` when the branch starts there), `git diff <base> -- crates/surfd/src
    crates/surfd/tests/hardening.rs crates/surfd/tests/adversary.rs crates/surfd/tests/
    adversary_2.rs crates/surfd/tests/common/mod.rs crates/surfd/tests/first_line.rs |
    /usr/bin/grep -c -E '^\+.*(Instant::now|\.elapsed\()'` prints 0.
14. The shipped texts say what is shown. With `flat() { sed -e 's|^ *//[/!] \{0,1\}||' "$1" |
    tr '\n' ' ' | tr -s ' '; }`, `flat <file> | /usr/bin/grep -c -F -e "seen without the wait
    for the host's core handler"` prints 1 for each of `crates/surfd/src/daemon.rs`,
    `README.md`, `website/docs/what-works.md`, `CHANGELOG.md` and `docs/design/scripts.md`
    after the change, and 0 for each at `b33aae7`.
15. No text says the handler is not started: `git grep -n -i -e 'handler is not started'
    -e 'handler is not run' -- crates README.md website docs CHANGELOG.md` prints nothing
    (exit 1), at `b33aae7` and after.
16. The changelog: `git grep -c -e 'core=1:1' -- CHANGELOG.md` prints nothing (exit 1) at
    `b33aae7` and `CHANGELOG.md:1` after; `git grep -c -e 'core=0' -- CHANGELOG.md` prints
    `CHANGELOG.md:1` at `b33aae7` (the `0.9.0` entry) and `CHANGELOG.md:2` after (the entry
    names `--core=0:0` once).
17. The start forms of `docs/design/scripts.md`: `flat docs/design/scripts.md | /usr/bin/grep
    -c -F -e 'prlimit --core=1:1 -- <session program>'` and the same for
    `'--nofile=64:64 --core=1:1 -- <session program>'` each print 0 at `b33aae7` and 1 after.
18. The child clause of `docs/design/scripts.md`: `flat docs/design/scripts.md |
    /usr/bin/grep -c -F -e 'can lower its soft limit to 0'` prints 0 at `b33aae7` and 1 after.
19. The history lines of `docs/design/scripts.md` stay (a guard: it holds at `b33aae7` too):
    `git diff <base> -U0 -- docs/design/scripts.md | /usr/bin/grep -c -E
    '^-(probe: |the cores of two early probe runs)'` prints 0; the lines are `:396`, `:843`
    and `:879` of `b33aae7`, which still name `--core=0:0`.
20. The tests carry no set that names `--core=0:0` but the constant: `git grep -l -e
    '--core=0' -- crates/surfd/tests` prints three files at `b33aae7` (`adversary.rs`,
    `adversary_2.rs`, `hardening.rs`) and after the change only
    `crates/surfd/tests/common/mod.rs`.
21. Count of `crates/surfd`: the number after `Summary` of the job of 12 (`N tests run`) is the
    total at `AGENTS.md:644`, and the per-file numbers of its list (with `first_line`, the
    unit tests of `src/daemon.rs` and the two new cases of `hardening`) add up to it. The
    recount is made on the story's tip after `main` is merged into it.
22. The badge: `sh docs/badges/make.sh .` leaves `docs/badges/tests.svg` with the gate's
    total.
23. The pages: the numbers of the Change's last bullet in `README.md`, `what-works.md`,
    `how-it-is-built.md` and `status.md` equal the gate's lines of the tip, which the rule of
    `AGENTS.md:2029-2035` has the coordinator copy into the pages.
24. The test budget: no test over 60 s, the test step within 120 s, a test over 5 s named with
    its time.
25. The pool: no pool job over 10 minutes unless it names its time with `slot --timeout` (at
    most 30 minutes), none started again unchanged. Every command that compiles or tests
    carries `slot`; none carries `--local`, `--here`, `SLOT_NO_REMOTE` or `SLOT_SHIM_OFF`. A
    script that only measures or waits starts without `slot`.
26. No Python in anything that stays in the repository; no model call and no network request
    in a test.

## Out of scope

- The host's core pattern or its handler's settings.
- A change of the wait bounds of the tests (`ALLOWED`, `tests/common/mod.rs:37`).
- Making a process undumpable by a system call (`prctl`, `setrlimit` in a session): it needs
  `unsafe`, which the workspace forbids.
- A hard core limit that changes after `surfd` started (decision 1).
- The two cases of `story:surfd-died-session-order` (`adversary_2.rs:147` and `:203`, now ended
  by `SIGUSR1`).
- Any other change of `surfd`'s bounds, answers, ready timeout or kill grace.

## Placement

Read with `git grep -l -E '^  path: <path>$' -- .engineering/planning/story` and the `status:`
line of each, on the plan tree at the tip of the branch that holds this story; the code base is
`main` at `b33aae7`:

- `crates/surfd/tests/adversary_2.rs`: `story:surfd-died-session-order`, active in the store
  (its code is on `main` as `c91b313`). It comes first: this story edits `:306` and the case at
  `:350` of the same file, the other the two cases it made. The edge is declared: this story
  `depends_on` it, in its relations.
- `crates/surfd/src/daemon.rs`: only `story:scripts-document-port`, proposed, and it edits the
  same lines: the doc comment of `SCRIPT_SESSION_BOUNDS` (`daemon.rs:64-66`, "4 GiB of address
  space ... no core file"), the declaration and the `--as` value (`:67-68`: this story rewrites
  the `[&str; 4]` line, that story changes the value on the line below it), the `Config` doc of
  `script_session_bounds` (`:112-113`) and, in `docs/design/scripts.md`, the form line
  `:975-976` (`--as=4294967296:4294967296` ... `--core=0:0` on one line). Its text is at
  `scripts-document-port.md:170` (decision 3, the `--as` value) and `:231` (its scope entry for
  `daemon.rs`). This story goes first: `story:scripts-document-port` gets `depends_on
  story:session-core-limit-one`, declared by the coordinator in this tree.
- `README.md`, `website/docs/what-works.md`, `website/docs/how-it-is-built.md`,
  `docs/design/scripts.md`, `docs/badges/make.sh` and `docs/badges/tests.svg`:
  `story:swell-0-9-0` and `story:scripts-dom-reading`, both active, and the proposed
  `story:scripts-parse-run`, `story:scripts-document-port` (also `daemon.rs`),
  `story:scripts-dom-attributes`, `-mutation`, `-selectors` and `story:scripts-html-conformance`
  (some of these files each). Their edited lines differ from this story's, except
  `docs/design/scripts.md:975-976` (above); the shared numbers are recounted in this story's
  last step on its tip, after whichever of them merged.
- `website/docs/status.md`: active `story:swell-0-9-0` and `story:scripts-dom-reading`; proposed
  `story:scripts-parse-run`, `story:scripts-document-port` and `story:scripts-html-conformance`;
  draft `story:conformance-lock-and-lints`. Only `:31` is edited here; the recount decides the
  rest.
- `AGENTS.md`: active `story:agent-tasks-left-process-wait`, `story:test-step-holds-with-tls`,
  `story:tests-without-wall-time-bounds`, `story:swell-0-9-0` and `story:scripts-dom-reading`;
  draft `story:conformance-lock-and-lints`, `story:session-argument-checks` and
  `story:tls-tests-as-tables`. Only `:644-646` is edited here; the recount decides the rest.
- `CHANGELOG.md`, `crates/surfd/tests/adversary.rs`, `hardening.rs`, `main.rs`,
  `common/mod.rs` and `first_line.rs`: no story that is not implemented has them in its scope.
  The text of `story:tests-without-wall-time-bounds` names `hardening.rs:532` for its ledger;
  this story adds no elapsed value, edits lines of that file in place and puts its new cases
  after the last line, so no line of the ledger moves and no edge is needed.
