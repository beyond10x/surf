---
format: aep.planning-md/3
id: story:surfd-died-session-order
kind: story
status: active
title: Order the session's end and its child's line by an event in two surfd tests
summary: Two cases of crates/surfd/tests/adversary_2.rs need a session shell to end by its own signal before a child writes a line or a 0.3 s ready timeout runs out, and decide that order by a clock of 0.3 s; under load the first case failed once in the gate. In the first case the child waits until the shell is waitable; the second keeps two clocks with margins of 1 s and more. No product file changes.
scope:
- confidence: cited
  path: crates/surfd/tests/adversary_2.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T04:22:21Z", actor: "human:operator", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-26T04:38:51Z", actor: "human:operator", revision: 10}
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

## Decisions

1. First case: the order is made by an event. The child writes `x` only after the session
   shell is waitable: it reads `/proc/<pid>/stat` of the shell (`$$` names the shell in the
   subshell too) until the state after the last `)` is `Z` or the file is gone, and then
   writes. The file is gone only after the opener collected the shell's status at its ready
   timeout (`:1474`, `:427`), a path on which the case fails by its reason; that half of the
   test only ends the loop. The end of a pipe or FIFO that the shell holds is not that event:
   the kernel closes a process's descriptors before the process is waitable, and in between
   `try_wait` is empty.
   The loop is a wait for an event with a limit: it sleeps 10 ms a turn (`/usr/bin/sleep
   0.01`; the session's environment is cleared, so commands are named by path) and gives up
   after 300 turns, a count that is over 10 times the 0.3 s the shell failed to meet. A child
   that gives up must not look like one that wrote: when it exits without a line, `supervise`
   sends `Start::Nothing` (`:1651`), the opener takes `not_started` (`:1476`, `:1575-1590`),
   and the answer for a shell that ended is the same text the case asserts. So the child that
   gives up first makes a file `gave-up` in the case's directory (`: > <path>`, the path
   quoted by `quoted`) and then exits, and the case asserts after its answer that the file
   does not exist, with a message that names the loop. The case sets `session_ready_timeout`
   to 30 s, so that under load the 300 turns end before the product's timeout and the file,
   not the timeout, decides; a green run answers in milliseconds and never waits for it. The
   loop is sh, so it adds no elapsed value to a Rust test.
2. Second case: the order stays one of two clocks, because the event the case reaches is the
   product's ready timeout, which starts inside `open` right after the spawn (`:1409`,
   `:1474`) and which a test cannot hold back. The child must hold the session's standard
   output past the ready timeout T (a child that lets go earlier takes the path of
   `Start::Nothing`, `:1476`), and must let go inside `abandon`'s wait, so its hold S obeys
   `T < S < T + max(kill_grace, KILL_GRACE)` (`:1331`; `KILL_GRACE` is 2 s, `:43`). The case
   sets T = 2 s (`session_ready_timeout`), S = 3 s (`/usr/bin/sleep 3`) and `kill_grace` = 4
   s: the shell has 2 s to end where it had 0.3 s, the timeout has 1 s before the child lets
   go, and the wait has 3 s left after it. The case lasts about 3 s. Its doc comment says
   that it orders clocks, with these three margins.
3. No product file changes. If the implementor finds that `surfd` answers wrongly for an
   order the product must hold, that is a finding for a story of its own, with a failing
   test, and this story stops there.

## Change

- `crates/surfd/tests/adversary_2.rs`: the two session programs; the settings of both cases (the
  first case's ready timeout; the second case's ready timeout, hold and kill grace); the first
  case's assertion on the file `gave-up`; the reason the second case asserts, which names the
  timeout (`within 0.3 seconds` at `:164-169` becomes `within 2 seconds`); and the doc
  comments of both cases, which say how the order is made. No case is added or renamed, so no
  count of `AGENTS.md` moves.

## Acceptance

1. Guard: each of the two cases still fails when the product loses the note. With the `note
   ended` line left out of the before-ready answer (a hand-made defect in `with_ended`,
   `daemon.rs:1561-1566`, made on the committed tip, run, taken back; `git diff --quiet --
   crates/surfd/src` after), both cases are red.
2. The first case is green when the session shell is slow to end: with `/usr/bin/sleep 1`
   written before `kill -s ABRT $$` in its session program (taken back after), the case is
   green; at `a14e3b0` the same line makes it red. Each shown once.
3. The second case is green when the session shell is slow to end: with `/usr/bin/sleep 1`
   written before `kill -s ABRT $$` in its session program (taken back after), the case is
   green; at `a14e3b0` the same line makes it red. Each shown once.
4. The first case is red when the child does not wait: with the loop of decision 1 replaced
   by nothing (the child writes `x` at once) and `/usr/bin/sleep 1` before the `kill`, the
   case is red. Shown once, taken back.
5. The first case is red when the loop never sees the shell's end: with the state letter the
   loop waits for changed to a letter no state has (`Q`), the child gives up and the case fails
   at its assertion on `gave-up`, not at an assertion on the answer; the report gives the
   case's time. Shown once, taken back.
6. A text check: `git grep -n 'sleep 0.3' -- crates/surfd/tests/adversary_2.rs` prints one
   line at `a14e3b0` (`:141`) and nothing after the change, and `git grep -c
   'from_millis(300)' -- crates/surfd/tests/adversary_2.rs` prints 2 at `a14e3b0` and 1 after
   (the number after the last colon; the case of the live session at `:118` keeps its
   timeout).
7. One pool job after `slot cargo build --workspace --all-targets --locked`, with
   `RUSTFLAGS="-D warnings"` and `SURF_BIN_DIR=<tree>/target/debug` as `task test` sets them:
   `slot cargo nextest run -p surfd --locked` is all green; the two cases are named with their
   times, the second about 3 s, neither over 5 s.
8. No test over 60 s, the test step within 120 s. No pool job over 10 minutes unless it names
   its time with `slot --timeout` (at most 30 minutes), none started again unchanged. Every
   command that compiles or tests carries `slot`; none carries `--local`, `--here`,
   `SLOT_NO_REMOTE` or `SLOT_SHIM_OFF`. No Python. A script that measures or waits starts
   without `slot`. No model call and no network request in a test.

## Out of scope

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
