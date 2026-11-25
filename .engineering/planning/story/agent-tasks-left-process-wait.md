---
format: aep.planning-md/3
id: story:agent-tasks-left-process-wait
kind: story
status: draft
title: Decide by a wait whether a model left a process
summary: Two tests of crates/agent-tasks read /proc once, right after a SIGKILL, to decide whether a process is left; the read finds a process that is not yet collected, and the gate went red in 1 run of 10 for no defect. This draft makes the decision a wait, as the crate's own rule says.
scope:
- confidence: inferred
  path: crates/agent-tasks/tests/adversary2.rs
revision: 2
---
## Context

`crates/agent-tasks/tests/adversary2.rs @ a14e3b0` holds two tests that end with "no process of
the model is left": `a_model_that_leaves_a_child_leaves_no_process` (`:256`) and
`a_model_over_its_bound_leaves_no_process` (`:275`). Both call `sleeping` (`:241-253`), which
reads the process ids the stand-in model wrote and keeps each whose `/proc/<pid>/comm` is
`sleep` (`comm`, `crates/agent-tasks/src/process.rs:215-219`). That is one read, made as soon as
the driver has returned.

The driver ends a model at its bound with SIGKILL to the process group
(`crates/agent-tasks/src/agent.rs:457-465`, and `src/bin/agent-tasks-serve.rs:138`). A process
that was sent SIGKILL keeps its entry under `/proc`, with its name, until it has ended and its
parent has collected it. One read in that window finds `sleep`.

The crate's own rule is the opposite (`src/process.rs:3-5`): "Ending one is SIGKILL through
`kill -9` and a wait with `surf_test_process::ended_within`. Whether the processes of a
directory have ended is decided by a wait too, `RuntimeDir::left_within`, never by one read."
`ended_within` is at `crates/test-process/src/lib.rs:47` and `END_BOUND` (5 s) at
`src/process.rs:36`.

Measured on 2026-11-25 on the build node, on a tree that changes no file of
`crates/agent-tasks`: the gate's test step failed once with
`children of the model left: [<pid>]` at `adversary2.rs:290`; then the three tests of the crate
whose names end in `leaves_no_process`, run ten times after the gate's build, were red in 1 run
of 10 with the same line and green in 9. The test took 1.1 s in every run.

A gate that goes red in one run of ten for no defect of the code stops a merge and costs a
second run.

## Decisions to take when the story is revised

1. Where the wait goes: `sleeping` waits for each listed process with `ended_within` and the
   crate's `END_BOUND` before it calls the process left (the form the crate's rule names), or
   it reads the process state from `/proc/<pid>/stat` and counts a process that has ended and
   waits for its parent as not left. Read `ended_within` first and say what it answers for
   such a process.
2. Whether the driver itself should wait for the group to end before it returns (the claim
   "leaves no process" as a property of the program, not of the test's patience), and what
   that costs a run that ends at its bound. If the program changes, `src/agent.rs` and
   `src/bin/agent-tasks-serve.rs` come into scope.
3. The other tests of the crate and of `crates/session`, `crates/surfd` that decide by one read
   whether a process is left: list them (`git grep` for `comm(` and for a read of `/proc/` in a
   `tests/` directory) and give each the same treatment or the reason it needs none.
4. How the fix is shown: the race cannot be forced by a test without a change of the program.
   Say what stands in (a unit test of the helper against a process that was sent SIGKILL and is
   not yet collected, made by the test itself), and the count of repeated runs the acceptance
   asks for.

## Change

- `crates/agent-tasks/tests/adversary2.rs`: `sleeping` decides by a wait.
- Other files as decisions 2 and 3 give.

## Acceptance

Draft. To be written when the story is revised; it holds at least:

1. No test of `crates/agent-tasks/tests` decides by one read of `/proc` whether a process is
   left (the command of decision 3 prints nothing, or only the lines the story lists).
2. The three tests whose names end in `leaves_no_process`, run 50 times in one pool job after
   the gate's build: 50 green runs. The job names its time if it needs more than the pool's
   limit, and no test runs over 5 s.
3. A model that does leave a process still turns its test red: the hand-made change of the
   program (the group kill removed) and the red test.
4. The bounds of the folder hold: no test over 60 s, the test step within 120 s, a test over
   5 s named with its time; the wait's limit is at least 10 times the time the event takes
   alone.

## Placement

No edge. The DOM slices and the swell pin change no file of `crates/agent-tasks` and none of
`crates/test-process`.
