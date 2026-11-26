---
format: aep.planning-md/3
id: story:surfd-died-session-order
kind: story
status: implemented
title: Order the session's end and its child's line by an event in two surfd tests
summary: Two cases of crates/surfd/tests/adversary_2.rs need a session shell to end by its own signal before a child writes a line or a 0.3 s ready timeout runs out, and decide that order by a clock of 0.3 s; under load the first case failed once in the gate. Both cases end the shell by a signal that dumps no core; in the first the child waits until the shell is waitable, in the second the child holds the output until the shell is collected. No product file changes.
scope:
- confidence: cited
  path: crates/surfd/tests/adversary_2.rs
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T04:22:21Z", actor: "human:operator", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-26T04:38:51Z", actor: "human:operator", revision: 10}
- {from: "active", to: "implemented", at: "2026-11-26T06:14:23Z", actor: "human:operator", revision: 16, decided_on: {"recorded":{"test_result":2,"verification":1}}, executor: "agent:coordinator"}
---
## Context

Two cases of `crates/surfd/tests/adversary_2.rs` (lines of `a14e3b0`) hold that an `open` whose
session died by its own signal names that signal. Each decides the order of two events by a
clock of 0.3 s:

- `another_first_line_after_the_session_died_by_its_own_signal_names_it` (`:135-148`): the
  session program is `( /usr/bin/sleep 0.3; printf 'x\n' ) &` and then `kill -s ABRT $$`. The
  case needs the shell to have ended by `SIGABRT` before the child writes `x`.
- `a_ready_timeout_after_the_session_died_by_its_own_signal_names_it` (`:153-171`): the ready
  timeout is 0.3 s (`:156`) and the session program is `( /usr/bin/sleep 1 ) &` and then `kill
  -s ABRT $$`. The case needs the shell to have ended before the ready timeout runs out.

What the product does with the two orders (`crates/surfd/src/daemon.rs`): `supervise` reads
the line and sends `Start::Other` (`:1647-1656`), or the opener's `recv_timeout` of the ready
timeout runs out (`:1474`); either way the opener calls `abandon` (`:1329-1334`), whose
`Process::kill` (`:425-432`) asks `try_wait` first. A shell that has ended, collected or not,
keeps its status, and the answer carries `note ended: signal SIGABRT`. A shell that is still
alive is killed by `surfd`, `killed` is set, and the answer has no note. Both answers are
right for their order.

Observed once: the test step of the gate, run on a branch that does not touch `crates/surfd`,
with a load average of 49.9 at the job's start, ended `1998 tests run: 1997 passed, 1 failed`:
the first case failed after 0.335 s at `adversary_2.rs:68` with `Answer { status: 3, reason:
Some("the session ended or wrote another line before it reported ready"), text: "" }`, the
answer for a shell that was still alive when `x` arrived. A gate run on the same code of
`crates/surfd` shortly before and a second run on the same tip after it were green. A red
gate that the change did not cause holds a merge.

`AGENTS.md:1965-1967 @ a14e3b0` asks that a limit on a wait in a test is at least 10 times the
event alone. In the first case the clock is no limit on a wait: it orders two events, and no
factor makes that safe.

Why the shell is slow to end, found in the review of this story's first implementation:
`SIGABRT` is a signal whose default action dumps core, and on a host whose core pattern is a
pipe the process is waitable only after the handler has taken the dump. Measured in a test
job on the build host (core pattern a pipe, at most 16 dumps at once): a shell that ended
itself by `SIGABRT` was waitable after 142 to 256 ms with one shell at a time, and after up
to 3.5 s with 24 such shells at once; by `SIGTERM` after 3 to 15 ms and at most 68 ms. Every
test of any repository that dumps core on that host shares the handler. A first
implementation that kept `SIGABRT` and waited for the shell's end with a limit of 300 turns
failed in a gate run under load (the case ended after 7.5 s at its assertion on `gave-up`),
beside seven other cases of `crates/surfd/tests` that end a process by such a signal.

What the two cases claim does not need that signal: a session that died by a signal `surfd`
did not send is named by that signal, and is not read as `bound cpu`, which belongs to
`SIGXCPU` alone (`ended_note`, `daemon.rs:410-414`). `signal_name` (`:309-349`) names
`SIGUSR1` like any other. That an aborted session names `SIGABRT` stays held by other cases
(`hardening.rs:302-320`, `adversary.rs:219`).

## Decisions

1. Both cases end the session shell by `SIGUSR1` (`kill -s USR1 $$`), a signal whose default
   action ends the process and dumps no core, and assert `note ended: signal SIGUSR1`. The
   time until the shell is waitable then does not depend on the host's core handler.
2. First case: the order is made by an event. The child writes `x` only after the session
   shell is waitable: it reads `/proc/<pid>/stat` of the shell (`$$` names the shell in the
   subshell too) until the state after the last `)` is `Z` or the file is gone, and then
   writes. The file is gone only after the opener collected the shell's status at its ready
   timeout (`:1474`, `:427`), a path on which the case fails by its reason; that half of the
   test only ends the loop. The end of a pipe or FIFO that the shell holds is not that event:
   the kernel closes a process's descriptors before the process is waitable, and in between
   `try_wait` is empty.
   The loop is a wait for an event with a limit: it sleeps 10 ms a turn (`/usr/bin/sleep
   0.01`; the session's environment is cleared, so commands are named by path) and gives up
   after 300 turns, over 10 times the 68 ms measured for a signal that dumps no core. A child
   that gives up must not look like one that wrote: when it exits without a line, `supervise`
   sends `Start::Nothing` (`:1651`), the opener takes `not_started` (`:1476`, `:1575-1590`),
   and the answer for a shell that ended is the same text the case asserts. So the child that
   gives up first makes a file `gave-up` in the case's directory (`: > <path>`, the path
   quoted by `quoted`) and then exits, and the case asserts after its answer that the file
   does not exist, with a message that names the loop. The case sets `session_ready_timeout`
   to 30 s, so that under load the 300 turns end before the product's timeout and the file,
   not the timeout, decides; a green run answers in milliseconds and never waits for it. The
   loop is sh, so it adds no elapsed value to a Rust test.
3. Second case: one clock stays, and the child's hold becomes an event.
   - The clock: the shell must have ended before the product's ready timeout T, which starts
     inside `open` right after the spawn (`:1409`, `:1474`) and which a test cannot hold
     back. The case sets T = 2 s (`session_ready_timeout`), over 10 times the 68 ms above.
     Its doc comment says that this is a clock and gives the margin.
   - The event: the child must hold the session's standard output past T (a child that lets
     go earlier takes the path of `Start::Nothing`, `:1476`) and let go inside `abandon`'s
     wait (`:1331`). At the timeout `abandon`'s `Process::kill` collects the ended shell
     (`:427`), and from then on `/proc/<pid>/stat` of the shell is gone. The child holds the
     output until that file is gone (`[ -e /proc/$$/stat ]` fails), in turns of 10 ms with a
     limit of 1,000 turns, and then exits. A child that gives up lets go after `abandon`'s
     wait has run out, and the case is red by its answer; no file is needed.
   - A second margin, named in the doc comment too: the child must see the file gone and
     exit within `abandon`'s wait of 2 s (`KILL_GRACE`, `:43`, `:1331`). One turn is 10 ms
     and the start of one `sleep`, so the wait is over 10 times a turn.
   - `kill_grace` keeps its default. The case lasts about 2 s.
4. No product file changes. If the implementor finds that `surfd` answers wrongly for an
   order the product must hold, that is a finding for a story of its own, with a failing
   test, and this story stops there.

## Change

- `crates/surfd/tests/adversary_2.rs`: the two session programs (the signal, the loop of the
  first case, the hold of the second); the settings (the first case's ready timeout of 30 s,
  the second case's of 2 s; no kill grace is set); the first case's assertion on the file
  `gave-up`; the note both cases assert (`note ended: signal SIGUSR1`); the reason the second
  case asserts, which names the timeout (`within 0.3 seconds` at `:164-169` becomes `within 2
  seconds`); and the doc comments of both cases, which say how the order is made and why the
  signal is one that dumps no core. The comments name the other kind as "a signal that dumps
  core" and do not hold the letters `ABRT`, which acceptance 7 counts. No case is added or
  renamed, so no count of `AGENTS.md` moves.

## Acceptance

1. Guard: each of the two cases still fails when the product loses the note. With the `note
   ended` line left out of the before-ready answer (a hand-made defect in `with_ended`,
   `daemon.rs:1561-1566`, made on the committed tip, run, taken back; `git diff --quiet --
   crates/surfd/src` after), both cases are red.
2. The first case is green when the session shell is slow to end: with `/usr/bin/sleep 1`
   written before `kill -s USR1 $$` in its session program (taken back after), the case is
   green; at `a14e3b0` the same line before its `kill` makes it red. Each shown once.
3. The second case is green when the session shell is slow to end: with `/usr/bin/sleep 1`
   written before `kill -s USR1 $$` in its session program (taken back after), the case is
   green; at `a14e3b0` the same line before its `kill` makes it red. Each shown once.
4. The first case is red when the child does not wait: with the loop of decision 2 replaced
   by nothing (the child writes `x` at once) and `/usr/bin/sleep 1` before the `kill`, the
   case is red. Shown once, taken back.
5. The first case is red when the loop never sees the shell's end: with the state letter the
   loop waits for changed to a letter no state has (`Q`), the child gives up and the case fails
   at its assertion on `gave-up`, not at an assertion on the answer; the report gives the
   case's time. Shown once, taken back.
6. The second case is red when the child does not hold: with the hold of decision 3 replaced
   by nothing (the child exits at once), the case fails at its assertion on the reason.
   Shown once, taken back.
7. A text check: `git grep -n 'sleep 0.3' -- crates/surfd/tests/adversary_2.rs` prints one
   line at `a14e3b0` (`:141`) and nothing after the change; `git grep -c 'from_millis(300)'
   -- crates/surfd/tests/adversary_2.rs` prints 2 at `a14e3b0` and 1 after (the number after
   the last colon; the case of the live session at `:118` keeps its timeout); `git grep -c
   'ABRT' -- crates/surfd/tests/adversary_2.rs` prints 6 at `a14e3b0` and no line after (it
   exits 1 then); and
   `git grep -c 'kill -s USR1' -- crates/surfd/tests/adversary_2.rs` prints 2 after.
8. One pool job after `slot cargo build --workspace --all-targets --locked`, with
   `RUSTFLAGS="-D warnings"` and `SURF_BIN_DIR=<tree>/target/debug` as `task test` sets them:
   `slot cargo nextest run -p surfd --locked` shows the two cases green; they are named with
   their times, the second about 2 s, neither over 5 s. A red among the seven cases the
   Context names (they are out of scope) is reported with its line and is not counted
   against this story; any other red is.
9. No test over 60 s, the test step within 120 s. No pool job over 10 minutes unless it names
   its time with `slot --timeout` (at most 30 minutes), none started again unchanged. Every
   command that compiles or tests carries `slot`; none carries `--local`, `--here`,
   `SLOT_NO_REMOTE` or `SLOT_SHIM_OFF`. No Python. A script that measures or waits starts
   without `slot`. No model call and no network request in a test.

## Out of scope

- The other cases of `crates/surfd/tests` that end a process by a signal whose default
  action dumps core and wait 5 s or 7 s for it (seven were red in the gate run named in the
  Context). Some of them claim something about `SIGXCPU` itself, so another signal is not
  the remedy for all; they get a story of their own.
- The other clocks of the file: the `thread::sleep` of 500 ms with its two elapsed bounds
  (`:210`, `:220`, `:232`) is listed by no story yet; the check of
  `story:tests-without-wall-time-bounds` finds such bounds, and `story:time-bounds-in-tests`
  decides them. The sleeps at `:102`, `:190` and `:319` order nothing.
- Any change of `surfd`'s ready timeout, kill grace or answers.

## Placement

`crates/surfd/tests/adversary_2.rs` is in the scope of no other story, read with `git grep -l
-F crates/surfd/tests/adversary_2.rs -- .engineering/planning/story`. The check of
`story:tests-without-wall-time-bounds` reads this file; the change adds no elapsed value, so
no ledger line moves, and no edge is needed.

## As built

The unit is on `main` at `b33aae7` (the merge of `impl/surfd-died-session-order` at `c91b313`,
which follows `b08ca8d`). It changes `crates/surfd/tests/adversary_2.rs` and no other file.

- Two commits. `b08ca8d` was built from the story's text before its last revision: it kept
  `SIGABRT`, waited for the shell's state `Z` in the first case and held the second case's
  output by a `sleep` of 3 s under a kill grace of 4 s. A gate run on it under load had the
  first case red after 7.5 s at its assertion on `gave-up`. `c91b313` is the story as it reads
  now: `kill -s USR1 $$` in both cases, the note `note ended: signal SIGUSR1`, and in the
  second case a child that holds the output until `[ -e /proc/$$/stat ]` fails.
- Nothing differs from the decisions. The text checks of acceptance 7 hold on `c91b313`: no
  line with `sleep 0.3` or `ABRT`, `from_millis(300)` once, `kill -s USR1` twice.
- Times on the build host: the first case 0.023 s and 0.035 s, the second 2.010 s and 2.030 s;
  the two cases 20 times in one job, `20/20 stress run iterations: 20 passed` in 40.5 s; the
  89 cases of the package green in 3.2 s and 2.4 s, none of the seven cases the Context names
  red in those runs.
- Shown red by the two reviews and the fix round: the product without the note (both cases,
  at their assertions on the text); the first case's child without its loop; the first case's
  loop waiting for a state no process has (red at `gave-up`); the second case's child without
  its hold (red at the reason); the second case's shell alive at the ready timeout (red at
  `died_id`, after 6.0 s).
- Left as it is, each seen by a review:
  - When the second case's shell is still alive at the ready timeout, the case fails in
    `died_id` with a message that names the answer and not the cause. It is told apart from
    a lost note by the line it fails at.
  - On that red path the child keeps testing for the file until the shell is collected or its
    1,000 turns end; in the run that showed it the child ended 19 ms after the test process.
    A green run leaves no process.
  - A doc comment of the second case says "the answer is red" where it means the case.
  - Two lines of another case of the file are not as `rustfmt` writes them; they were so
    before the unit and the gate has no format step.
- A case a reviewer wrote for a first line that is not `ready` (a session that writes `x` and
  exits with status 3) passes on the unit and is kept on a review branch. It adds one test
  and so moves the counts of `AGENTS.md` and the public numbers; it goes in with a later
  story of `crates/surfd/tests`.
