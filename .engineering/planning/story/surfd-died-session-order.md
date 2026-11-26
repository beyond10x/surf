---
format: aep.planning-md/3
id: story:surfd-died-session-order
kind: story
status: proposed
title: Order the session's end and its child's line by an event in two surfd tests
summary: Two cases of crates/surfd/tests/adversary_2.rs need a session shell to end by its own signal before a child writes a line or a 0.3 s ready timeout runs out, and decide that order by a clock of 0.3 s; under load the first case failed once in the gate. The story makes the order by an event and changes no product file.
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/surfd/tests/adversary_2.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T04:22:21Z", actor: "human:operator", revision: 4}
---
## Context

Two cases of `crates/surfd/tests/adversary_2.rs` (lines of `a14e3b0`) hold that an `open` whose
session died by its own signal names that signal. Each decides the order of two events by a
clock of 0.3 s:

- `another_first_line_after_the_session_died_by_its_own_signal_names_it` (`:135-148`): the
  session program is `( /usr/bin/sleep 0.3; printf 'x\n' ) &` and then `kill -s ABRT $$`. The
  case needs the shell to have ended by `SIGABRT` before the child writes `x`. When the shell
  takes longer than 0.3 s from the fork of the child to its own end, `surfd` reads `x` from a
  live session, kills it itself and answers without a `note ended` line, which is the right
  answer for that order and the wrong one for the case.
- `a_ready_timeout_after_the_session_died_by_its_own_signal_names_it` (`:153-171`): the ready
  timeout is 0.3 s (`:156`) and the session program is `( /usr/bin/sleep 1 ) &` and then `kill
  -s ABRT $$`. The case needs the shell to have ended before the ready timeout runs out.

Observed once: the test step of the gate, run on a branch that does not touch `crates/surfd`,
under a load average of 50, ended `1998 tests run: 1997 passed, 1 failed`: the first case
failed after 0.335 s at `adversary_2.rs:68` with `Answer { status: 3, reason: Some("the
session ended or wrote another line before it reported ready"), text: "" }`. A gate run nine
minutes earlier on the same code of `crates/surfd` was green. A red gate that the change did
not cause holds a merge.

`AGENTS.md:1965-1967 @ a14e3b0` asks that a limit on a wait in a test is at least 10 times the
event alone. Here the clock is no limit on a wait: it orders two events, and no factor makes
that safe.

## Decisions

1. The order is made by an event. In the first case the child writes its line after the
   session process has ended, which it learns without a clock: for example it reads a FIFO
   whose only writer is the session shell, so the read ends when that shell ends. The
   implementor picks the construction and shows that it holds the child's standard output
   open, as the case needs, and that it cannot block for ever (the case's own bound ends it).
2. In the second case the ready timeout stays a clock, because the product's timeout is what
   the case reaches. The case makes the session's end certain before the timeout can run out
   by the same event: `surfd` is told to open the session only after the shell has ended, or
   the ready timeout is raised so that it is at least 10 times the time the shell needs to
   end, measured in the case's report. The implementor picks one and says why.
3. No product file changes. If the implementor finds that `surfd` answers wrongly for an
   order the product must hold (a session that died first and is reported as killed), that is
   a finding for a story of its own, with a failing test, and this story stops there.

## Change

- `crates/surfd/tests/adversary_2.rs`: the two session programs and, where needed, a helper in
  the same file. The doc comments of both cases say how the order is made.
- `AGENTS.md`: only if the entry of `crates/surfd` counts the cases or names the 0.3 s.

## Acceptance

1. Each of the two cases still fails when the product loses the note: with the `note ended`
   line left out of the before-ready answer (a hand-made defect in `crates/surfd/src/`, made,
   run, taken back, `git diff --quiet` after), both cases are red.
2. The first case is green when the session shell is slow to end: with `/usr/bin/sleep 1`
   written between the fork of the child and `kill -s ABRT $$` (a hand-made change of the
   session program, taken back), the case is green; at `a14e3b0` the same change makes it red,
   shown once.
3. The second case is green under the same slow shell, or, if decision 2 took the raised
   timeout, the report gives the measured time of the shell's end and the factor.
4. `git grep -n 'sleep 0.3' -- crates/surfd/tests/adversary_2.rs` prints one line at `a14e3b0`
   (`:141`) and nothing after the change.
5. One pool job, the gate's environment, `slot cargo nextest run -p surfd --locked`: all
   green; each of the two cases is named with its time, none over 5 s.
6. No test over 60 s, the test step within 120 s. No pool job over 10 minutes unless it names
   its time with `slot --timeout` (at most 30 minutes), none started again unchanged. Every
   command that compiles or tests carries `slot`; none carries `--local`, `--here`,
   `SLOT_NO_REMOTE` or `SLOT_SHIM_OFF`. No Python. A script that measures or waits starts
   without `slot`. No model call and no network request in a test.

## Out of scope

- The other clocks of `crates/surfd/tests/` (`story:time-bounds-in-tests` lists the bounds;
  `story:tests-without-wall-time-bounds` brings the check that finds them).
- Any change of `surfd`'s ready timeout, kill grace or answers.

## Placement

`crates/surfd/tests/adversary_2.rs` is in the scope of no story that is not implemented, read
with `git grep -l -F crates/surfd/tests/adversary_2.rs -- .engineering/planning/story`. The
check of `story:tests-without-wall-time-bounds` reads this file; the change adds no elapsed
bound, so no ledger line moves.
