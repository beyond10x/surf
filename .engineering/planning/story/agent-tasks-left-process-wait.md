---
format: aep.planning-md/3
id: story:agent-tasks-left-process-wait
kind: story
status: proposed
title: Decide that a model's processes ended by a wait, in the three tests that read /proc once
summary: The two model tests read /proc once after a group SIGKILL and take a zombie for a left process (1 red run in 10); the story makes them, and the same read in bounded.rs, wait with ended_within and shows the fix with two tests that make a zombie themselves.
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/agent-tasks/tests/adversary2.rs
- confidence: cited
  path: crates/agent-tasks/tests/bounded.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T17:30:08Z", actor: "human:operator", revision: 8}
---
## Context

`crates/agent-tasks/tests/adversary2.rs @ a14e3b0` holds two tests that end with "no process of
the model is left": `a_model_that_leaves_a_child_leaves_no_process` (`:256`) and
`a_model_over_its_bound_leaves_no_process` (`:275`). Both call `sleeping` (`:241-253`), which
reads the process ids the stand-in model wrote and keeps each whose `/proc/<pid>/comm` is
`sleep` (`comm`, `crates/agent-tasks/src/process.rs:215-219`). That is one read, made as soon as
the driver has returned. A third test of the crate with the same ending,
`a_command_that_leaves_a_child_leaves_no_process` (`bounded.rs:67`), reads once too
(`running`, `bounded.rs:41-50`, through `child_left`, `:53-64`).

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

## Decisions

1. Where the wait goes: in `sleeping` (`adversary2.rs:240-253`), through `ended_within` and the
   crate's `END_BOUND`. What the three helpers answer, read at `a14e3b0`:
   - `comm` (`process.rs:215-219`) reads `/proc/<pid>/comm` and answers the name of any process
     that has an entry, a zombie too; it says nothing of the state.
   - `signal_kill` (`process.rs:263-271`) runs `kill -9` and returns when `kill` has; the
     process ends after that call returned.
   - `ended_within(pid, bound)` (`crates/test-process/src/lib.rs:47-65`) reads
     `/proc/<pid>/stat` every 10 ms and returns `Ok` when the file cannot be read or when the
     state is `Z`, `X` or `x` (`runs`, `:30-35`), so a zombie counts as ended; at the bound it
     returns `Err` with the last `stat` text.
   The model's `sleep 20` is an orphan (its parent shell has ended, `adversary2.rs:262`,
   `:281`), so its parent is init or the nearest subreaper, and it stays a zombie, named
   `sleep`, until that one collects it: `comm` answers `sleep` for it. `sleeping` keeps a
   process whose `comm` is `sleep` and for which `ended_within(pid, bound)` is `Err`, then
   sends it SIGKILL as today. Its signature gains `bound: Duration`; the two model tests pass
   `END_BOUND`. The other form, reading the state once and counting `Z` as ended, is refused:
   a process that has been signalled and has not yet ended reads `S`, the same race one step
   earlier. The wait's limit: SIGKILL ends a process "within milliseconds on an idle host"
   (`process.rs:30-36`, and the comments of the cleanups at `session/tests/common/mod.rs:186` and
   `surfd/tests/common/mod.rs:249`); 5 s is at least 10 times 0.5 s, and `sleeping` prints the
   longest wait it saw so that the report shows the time alone. A model that does leave a
   process still waits the whole 5 s and returns it.
2. The driver does not change, so no file under `src/` is in scope. The two places of a group
   SIGKILL, read:
   - `agent.rs:457-465` (`signal_kill_group`, called at `:269` when the call did not exit): the
     in-process lane's path; it follows with `child.kill()` and `child.wait()` for the leader,
     and the lane decides the rest by `RuntimeDir::left_within` (a wait). The driver of the two
     tests does not use it.
   - `agent-tasks-serve.rs:140-148` (`kill_group`, `kill -9 -- -<group>`): the helper the driver
     runs the model through (`taskfiles/agent-tasks-model.sh:310`, `exec "$serve" --bounded`).
     After it, `child.wait()` and, when a process was signalled, `wait_group_ended`
     (`:152-157`): at most `GROUP_END` (2 s, `:31`) until `group_runs` (`:160-175`) finds no
     process whose group is the model's, "a zombie aside" (`:151`).
   So the program already waits until no process of the group runs, and what the test saw was a
   zombie, which only the process's parent can collect. A wait for the zombie to leave `/proc`
   would make the helper depend on the host's reaper and would end the group's wait late on a
   loaded host (`process.rs:30-36`: 37 to 263 load average on 2026-11-23). The claim "leaves no
   process" holds of the program as "no process of the group runs"; the test states it the same
   way, which is what `ended_within` answers.
3. The other tests that decide by one read whether a process is left, listed with
   `git grep -n -E 'comm\(' a14e3b0 -- ':(glob)**/tests/**/*.rs'` (1 line, the one in
   `sleeping`, `adversary2.rs:247`) and
   `git grep -n -E '!(surf_test_process::)?alive\(|!runs\(|\.alive\(\)' a14e3b0 --
   ':(glob)**/tests/**/*.rs'` (10 lines); the 58 other lines that read `/proc/` in a `tests/`
   directory read `cmdline`, `environ`, `exe`, `limits`, `status`, `statm` or `task/*/children`
   to identify or measure a process, not to decide that one is gone.
   - `crates/agent-tasks/tests/bounded.rs`, `running` (`:41-50`) and `child_left` (`:53-64`),
     used by the tests at `:67` and `:84`: one read, right after the helper returned. The helper
     waits at most 2 s (decision 2), so a host that takes longer makes the test red for a
     process that is ending. Converted: `child_left` waits with
     `ended_within(pid, END_BOUND)`, sends SIGKILL to what is still there at the bound, and
     `running` is removed (it is `alive` of `surf_test_process`, kept for the one purpose).
   - `crates/session/tests/events.rs:446`, `crates/surfd/tests/in_process.rs:61` and
     `crates/surfd/tests/built.rs:154`: `!alive(pid)` right after `drop(dir)`. The drop sends
     SIGKILL to each process and waits for each with `ended_within`
     (`session/tests/common/mod.rs:189`, `surfd/tests/common/mod.rs:252`), 10 s and 5 s
     for all of them. The read is after a wait. None needed.
   - `crates/test-process/tests/children.rs:67`, `:87`, `:132`: they test `alive` itself on
     a child the test reaped, a zombie the test made and a pid that never existed. None needed.
   - `crates/agent/tests/start.rs:87`, `crates/surfd/tests/in_process.rs:223`,
     `crates/view-fixture/tests/fixture.rs:303`, `:305`, `:311`: `alive` for a process that is
     expected to run (`test-process/src/lib.rs:36-39`: one read decides only for such a
     process). None needed.
4. How the fix is shown. The race cannot be forced by a model test without a change of the
   program, so two tests of `adversary2.rs` stand in, each with a process the test made:
   - `a_killed_process_nobody_has_collected_is_not_left`: the test spawns `/bin/sleep 30`,
     writes its pid to a file, sends it `signal_kill`, waits with `ended_within(pid,
     END_BOUND)` (it returns when the state is `Z`: the test is the parent and has not called
     `wait`), asserts that `comm(pid)` is still `Some("sleep")` (the window of the defect, made
     certain), asserts that `sleeping(&file, END_BOUND)` is empty, and then calls `wait` on the
     child. Red with `sleeping` written back as the one `comm` read: it returns the pid.
   - `a_process_that_still_runs_is_left_and_ended`: spawns `/bin/sleep 30`, asserts
     `sleeping(&file, 200 ms)` is that pid alone (the process runs for 30 s, so the answer does
     not depend on the load), asserts it has ended afterwards (`ended_within(pid, END_BOUND)`)
     and calls `wait`. Red with a `sleeping` that keeps nothing.
   With them `adversary2` holds 12 tests and the crate 50. The repeated runs: the three tests
   named `..._leaves_no_process` 50 times in one pool job that names its time, run once by the
   implementor and by no step of the gate (it is no step of `check` or `test`):
   `slot --timeout 10m cargo nextest run -p surf-agent-tasks --locked --stress-count 50 -E
   'test(leaves_no_process)'`. A nextest without `--stress-count` takes the same 50 runs from a
   POSIX sh loop over the same command, written out in Acceptance 3. Arithmetic:
   50 x 3 x 1.1 s = 165 s if they ran one after another, under the 10 minutes named; with the
   defect a run of the three is red with probability 1 in 10, so 50 green runs happen with
   probability 0.9^50 = 0.0051.

## Change

- `crates/agent-tasks/tests/adversary2.rs`: `sleeping` takes `bound` and waits (decision 1), the
  two model tests pass `END_BOUND`, and the two tests of decision 4 are added. The import line
  (`:21`) gains nothing: `END_BOUND`, `comm` and `signal_kill` are there; `ended_within` is
  imported from `surf_test_process`, a dependency of the crate (`Cargo.toml`, `[dependencies]`).
- `crates/agent-tasks/tests/bounded.rs`: `child_left` waits and `running` goes (decision 3).
- `AGENTS.md`: the entry of `crates/agent-tasks` says `50 tests (`adversary` 4, `adversary2` 12,
  ...)` (`:868-869`, now 48 and 10), and, beside the paragraph on `RuntimeDir::left_within`
  (`:2316-2320`), one sentence that the tests of the model driver (`adversary2`, `bounded`)
  decide that a model's processes ended by the same wait, `ended_within` and `END_BOUND`.

No file under any `src/` changes and `taskfiles/agent-tasks-model.sh` does not change.

## Acceptance

1. No test of `crates/agent-tasks/tests` decides by one read of `/proc` whether a process is
   left: `git grep -n -F 'ended_within' -- crates/agent-tasks/tests/adversary2.rs
   crates/agent-tasks/tests/bounded.rs` prints a line in each file, and
   `git grep -n -E 'fn running|comm\(pid\)' -- crates/agent-tasks/tests/bounded.rs` prints
   nothing; `git grep -n -F 'comm(pid)' -- crates/agent-tasks/tests/adversary2.rs` prints the one
   read that follows `ended_within` in `sleeping` and the premise line of the first test of
   decision 4. Turns red by: `fn running` written back into `bounded.rs`.
2. The test `a_killed_process_nobody_has_collected_is_not_left` passes, and fails with `sleeping`
   written as the one `comm` read (`left = [<pid>]` at its assertion). The test
   `a_process_that_still_runs_is_left_and_ended` passes, and fails with a `sleeping` that keeps
   nothing (it asserts the pid is returned). Each failure is shown once before the change is
   kept, and the report names the output.
3. The three tests whose names end in `leaves_no_process`, run 50 times in one pool job after
   the gate's build, once by the implementor and never by the gate, with
   `slot --timeout 10m cargo nextest run -p surf-agent-tasks --locked --stress-count 50 -E
   'test(leaves_no_process)'`: nextest's stress summary shows 50 iterations and no failure.
   With a nextest that refuses `--stress-count`, the same runs by the loop below, in one pool
   job (it runs tests only, from its first line to its last, so it keeps the `slot` prefix; it
   sets no `SLOT_SHIM_OFF` and no other variable of the pool):
   `slot --timeout 30m sh -c 'i=0; while [ "$i" -lt 50 ]; do cargo nextest run
   -p surf-agent-tasks --locked -E "test(leaves_no_process)" || exit 1; i=$((i + 1)); done'`,
   which exits 0 after 50 passes and 1 at the first failure. Turns red, with the probability
   of decision 4, by the one `comm` read written back into `sleeping` (the model tests).
4. A model that does leave a process still turns its test red: `kill_group`
   (`agent-tasks-serve.rs:140-148`) written to signal the leader only
   (`.args(["-9", &group.to_string()])`) makes `a_model_that_leaves_a_child_leaves_no_process`
   and `a_model_over_its_bound_leaves_no_process` fail with
   `children of the model left: [<pid>]` after the 5 s wait, and
   `a_command_that_leaves_a_child_leaves_no_process` fail at `bounded.rs:80`
   (`the child of the command is left`). The change is put back; the report names the three
   outputs.
5. The AGENTS.md lines: ``git grep -n -F '50 tests (`adversary` 4, `adversary2` 12' -- AGENTS.md``
   prints one line and the same with `48 tests` and `adversary2` 10 prints nothing;
   `git grep -n -F 'ended_within' -- AGENTS.md` prints a line in the entry of `crates/agent-tasks`.
6. The bounds of the folder hold: no test over 60 s, the test step within 120 s, a test over
   5 s named with its time (a model test that leaves a process waits 5 s only when it fails);
   no pool job over 10 minutes unless it names its time with `slot --timeout` (at most 30
   minutes: the stress job names 10 minutes and is 165 s by decision 4) and none started again
   unchanged after `timed_out`; every compile or test command carries `slot`; no Python (the
   story adds none); no script of the story measures or waits outside a pool job; the stress
   run is no step of the gate; no model call and no network request
   in a test (the stand-in model is a shell script); the wait's limit is at least 10 times the
   event alone (decision 1).

## Out of scope

- The program: `src/agent.rs`, `src/bin/agent-tasks-serve.rs`, `src/process.rs` and
  `taskfiles/agent-tasks-model.sh` (decision 2).
- The 43 wall-time bounds in 27 files of `tests/` directories, among them
  `adversary2.rs:120` and `bounded.rs:97`: `story:time-bounds-in-tests`, with its ledger in
  `story:tests-without-wall-time-bounds`.
- The tests listed in decision 3 as needing none.
- A change of the pid check that the lane uses (`RuntimeDir::left_within`), which already waits.

## Placement

Read with `git grep -l -F <path> -- .engineering/planning/story` for each scope path.

- `crates/agent-tasks/tests/adversary2.rs` is also named by `story:test-harness-fewer-processes`
  (implemented), which changed other lines of the file (the harness that reads the task file).
  No edge.
- `crates/agent-tasks/tests/bounded.rs` is named by no other story.
- `AGENTS.md` is named by 40 stories; this story edits the test-count line of the entry of
  `crates/agent-tasks` (`:868-869`) and the paragraph at `:2316-2320`. No story that is not
  implemented edits those lines. No edge.
- `story:tests-without-wall-time-bounds` (draft) lists `adversary2.rs` and `bounded.rs` in its
  ledger (one statement each, `adversary2.rs:120`, `bounded.rs:97`); this story edits other
  lines of both files and does not change the count. No edge.
