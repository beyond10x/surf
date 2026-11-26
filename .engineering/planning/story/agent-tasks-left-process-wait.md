---
format: aep.planning-md/3
id: story:agent-tasks-left-process-wait
kind: story
status: proposed
title: Decide that a model's processes ended by a wait, in the three tests that read /proc once
summary: The two model tests read /proc once after a group SIGKILL and take a zombie for a left process (one red run in ten observed); the story makes them, and the same read in bounded.rs, wait with ended_within and shows the fix with two tests that make a zombie themselves.
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/agent-tasks/tests/adversary2.rs
- confidence: cited
  path: crates/agent-tasks/tests/bounded.rs
revision: 10
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
of 10 with the same line and green in 9. The test took 1.1 s in every run. The 1 in 10 is one
observation of ten runs, not a rate.

A gate that goes red for no defect of the code stops a merge and costs a second run.

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
     returns `Err` with the last `stat` text. With a bound of zero it reads once and returns
     `Err` for a process that runs.
   The model's `sleep 20` is an orphan (its parent shell has ended, `adversary2.rs:262`,
   `:281`), so its parent is init or the nearest subreaper, and it stays a zombie, named
   `sleep`, until that one collects it: `comm` answers `sleep` for it. `sleeping` keeps a
   process whose `comm` is `sleep` and for which `ended_within(pid, bound)` is `Err`, then
   sends it SIGKILL as today. Its signature gains `bound: Duration`; the two model tests pass
   `END_BOUND`. The other form, reading the state once and counting `Z` as ended, is refused:
   a process that has been signalled and has not yet ended reads `S`, the same race one step
   earlier. The wait's limit: SIGKILL ends a process "within milliseconds on an idle host"
   (`process.rs:30-36`, and the comments of the cleanups at `session/tests/common/mod.rs:186`
   and `surfd/tests/common/mod.rs:249`); 5 s is at least 10 times 0.5 s. A model that does leave
   a process still waits the whole 5 s and returns it.
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
3. The other tests that decide by one read whether a process is left, listed from the output of
   `git grep -n -E 'comm\(' a14e3b0 -- ':(glob)**/tests/**/*.rs'` (1 line, the one in
   `sleeping`, `adversary2.rs:247`) and
   `git grep -n -E '!(surf_test_process::)?alive\(|!runs\(|\.alive\(\)' a14e3b0 --
   ':(glob)**/tests/**/*.rs'` (10 lines, each below); the 58 lines that read `/proc/` in a
   `tests/` directory read `cmdline`, `environ`, `exe`, `limits`, `status`, `statm` or
   `task/*/children` to identify or measure a process, not to decide that one is gone, and
   the one in `bounded.rs` (`running`) is converted here.
   - `crates/agent-tasks/tests/bounded.rs`, `running` (`:41-50`) and `child_left` (`:53-64`),
     used by the tests at `:67` and `:84`: one read, right after the helper returned. The helper
     waits at most 2 s (decision 2), so a host that takes longer makes the test red for a
     process that is ending. Converted: `child_left` waits with
     `ended_within(pid, END_BOUND)`, sends SIGKILL to what is still there at the bound, and
     `running` is removed.
   - `crates/session/tests/events.rs:446`, `crates/surfd/tests/in_process.rs:61` and
     `crates/surfd/tests/built.rs:154`: `!alive(pid)` right after `drop(dir)`. The drop sends
     SIGKILL to each process and waits for each with `ended_within`
     (`session/tests/common/mod.rs:189`, `surfd/tests/common/mod.rs:252`), 10 s and 5 s for
     all of them. The read is after a wait. None needed.
   - `crates/test-process/tests/children.rs:67`, `:87` and `:132`: they test `alive` itself on
     a child the test reaped, a zombie the test made and a pid that never existed. None needed.
   - `crates/test-process/tests/children.rs:86` and `crates/test-process/tests/lines.rs:73`:
     `runs` on a text, no process read. None needed.
   - `crates/view-fixture/tests/fixture.rs:303` and `:305`: `foreign.alive()` for a process
     that is expected to run (`test-process/src/lib.rs:36-39`: one read decides only for such
     a process). None needed.
4. How the fix is shown. The race cannot be forced by a model test without a change of the
   program, so two tests of `adversary2.rs` make the process themselves. Each spawns
   `/bin/sleep 30` with `Stdio::null()` for its stdin, stdout and stderr and holds the child in
   a guard whose `Drop` calls `kill()` and then `wait()` (both results ignored), so a panic
   in the test leaves no `sleep` behind. Each writes the pid to a file named with the test's
   name and `std::process::id()`, as the two model tests do (`adversary2.rs:257`).
   - `a_killed_process_nobody_has_collected_is_not_left`: sends the child `signal_kill`, waits
     with `ended_within(pid, END_BOUND)` (it returns when the state is `Z`: the test is the
     parent and has not called `wait`), asserts that `comm(pid)` is still `Some("sleep")` (the
     window of the defect, made certain), asserts that `sleeping(&file, END_BOUND)` is empty,
     and then calls `wait` on the child. Red with `sleeping` written back as the one `comm`
     read: it returns the pid.
   - `a_process_that_still_runs_is_left_and_ended`: asserts `sleeping(&file, Duration::ZERO)`
     is that pid alone (the process runs for 30 s and the bound is zero, so the answer does not
     depend on the load and no clock literal is in the test), asserts the process has ended
     afterwards (`ended_within(pid, END_BOUND)`, since `sleeping` sent it SIGKILL) and calls
     `wait`. Red with a `sleeping` that keeps nothing.
   With them `adversary2` holds 12 tests and the crate 50. What proves what: the two tests are
   the proof that `sleeping` decides by a wait; the 50 runs below are a check that the three
   named tests show no failure in 50 runs, not a proof. No run with the defect written back is
   asked for, since 1 in 10 is one observation and the two tests need none.
   The stress command, run once by the implementor and by no step of the gate (it is no step of
   `check` or `test`), as one pool job that builds and then runs, from the tree's root, with the
   environment the `test` step sets (`Taskfile.yml:444-452`: `RUSTFLAGS=-D warnings` and
   `SURF_BIN_DIR=<root>/target/debug`; its `build`, `:399-406`, is
   `cargo build --workspace --all-targets --locked` with the same `RUSTFLAGS`):
   `slot --timeout 10m sh -c 'export RUSTFLAGS="-D warnings" SURF_BIN_DIR="$PWD/target/debug";
   cargo build --workspace --all-targets --locked && cargo nextest run -p surf-agent-tasks
   --locked --stress-count 50 -E "test(leaves_no_process)"'`. `--stress-count` needs nextest
   0.9.143 or later, which `.config/nextest.toml:1` already requires, so there is no fallback.
   Arithmetic: 50 x 3 x 1.1 s = 165 s if the runs went one after another, plus the build; under
   the 10 minutes named.

## Change

- `crates/agent-tasks/tests/adversary2.rs`: `sleeping` takes `bound` and waits (decision 1),
  the two model tests pass `END_BOUND`, and the two tests of decision 4 and their guard are
  added. Imports: `use surf_test_process::ended_within;` is added (`surf-test-process` is a
  dependency of the crate, `Cargo.toml` `[dependencies]`), `Child` joins the `std::process`
  import at `:14`, and the `surf_agent_tasks::process` import at `:21` keeps `END_BOUND`,
  `comm` and `signal_kill`.
- `crates/agent-tasks/tests/bounded.rs`: `child_left` waits and `running` goes (decision 3).
  Imports: `use surf_agent_tasks::process::END_BOUND;` and `use surf_test_process::ended_within;`
  are added.
- `AGENTS.md`, in one place, the entry of `crates/agent-tasks` (`:868-869 @ a14e3b0`): the
  test counts become `50 tests (`adversary` 4, `adversary2` 12, ...)` (now 48 and 10), and
  one sentence follows them: the tests of the model driver (`adversary2`, `bounded`) decide
  that a model's processes ended by a wait, `ended_within` with `END_BOUND`, never by one read.
  The paragraph at `:2316-2320` and the entry of `crates/test-process` (`:836`) are not edited.

No file under any `src/` changes and `taskfiles/agent-tasks-model.sh` does not change.

## Acceptance

Each item is one outcome.

1. `git grep -n -F 'ended_within(' -- crates/agent-tasks/tests/adversary2.rs` prints the line in
   `sleeping` and the lines of the two new tests. Turns red by: the wait removed from `sleeping`
   (the `sleeping` line is not printed).
2. `git grep -n -E 'fn running' -- crates/agent-tasks/tests/bounded.rs` prints nothing and
   `git grep -n -F 'ended_within(' -- crates/agent-tasks/tests/bounded.rs` prints the line in
   `child_left`. Turns red by: `fn running` written back.
3. `git grep -n -F 'use surf_test_process::ended_within;' --
   crates/agent-tasks/tests/adversary2.rs crates/agent-tasks/tests/bounded.rs` prints one line
   in each file, and `git grep -n -F 'use surf_agent_tasks::process::END_BOUND;' --
   crates/agent-tasks/tests/bounded.rs` prints one line.
4. `a_killed_process_nobody_has_collected_is_not_left` passes, and fails with `sleeping`
   written as the one `comm` read (`left = [<pid>]` at its assertion).
5. `a_process_that_still_runs_is_left_and_ended` passes, and fails with a `sleeping` that keeps
   nothing (it asserts the pid is returned). Each failure of items 4 and 5 is shown once before
   the change is kept, and the report names the output.
6. The stress command of decision 4, run once as written there: it exits 0 and the nextest
   summary shows 50 iterations and no failure. The report states that this is a check of 50
   runs and not a proof.
7. A model that does leave a process still turns its tests red: `kill_group`
   (`agent-tasks-serve.rs:140-148`) written to signal the leader only
   (`.args(["-9", &group.to_string()])`), then, as one pool job with the same build and
   environment as item 6 and the filter
   `-E "test(leaves_no_process) | test(is_ended_with_its_group)"` and no `--stress-count`,
   these four tests fail:
   `a_model_that_leaves_a_child_leaves_no_process` and
   `a_model_over_its_bound_leaves_no_process` with `children of the model left: [<pid>]` after
   the 5 s wait, `a_command_that_leaves_a_child_leaves_no_process` at `bounded.rs:80`
   (`the child of the command is left`), and
   `a_command_over_its_bound_is_ended_with_its_group` (`bounded.rs:84-99`) at `:98`. The change
   is put back; the report names the four outputs.
8. In `AGENTS.md`, `git grep -n -F -e ended_within --and -e adversary2 -- AGENTS.md` prints
   nothing at `a14e3b0` and one line after, inside the entry of `crates/agent-tasks`; and
   ``git grep -n -F '50 tests (`adversary` 4, `adversary2` 12' -- AGENTS.md`` prints one line
   while the same with `48 tests` and `adversary2` 10 prints nothing.
9. The new tests leave nothing behind: `git grep -n -F 'Stdio::null()' --
   crates/agent-tasks/tests/adversary2.rs` prints at least six lines (the three streams of
   each of the two spawns), and `git grep -n -F 'impl Drop for' --
   crates/agent-tasks/tests/adversary2.rs` prints the guard whose `Drop` calls `kill()` and
   `wait()`. The pid files are named with the test's name and `std::process::id()`.
10. The folder's bounds hold: no test over 60 s, the test step within 120 s, a test over 5 s
    named with its time (a model test that leaves a process waits 5 s only when it fails);
    no pool job over 10 minutes unless it names its time with `slot --timeout` (at most 30
    minutes: the jobs of items 6 and 7 name 10 minutes) and none started again unchanged after
    `timed_out`; every compile or test command carries `slot`; no Python; no model call and no
    network request in a test (the stand-in model is a shell script); the wait's limit is at
    least 10 times the event alone (decision 1).
11. A script that measures or waits starts without the `slot` prefix; the story adds none. No
    command of the story sets `--local`, `--here`, `SLOT_NO_REMOTE` or `SLOT_SHIM_OFF`. The
    report does not rely on a wait printed by `sleeping`: a passing test shows no output.

## Out of scope

- The program: `src/agent.rs`, `src/bin/agent-tasks-serve.rs`, `src/process.rs` and
  `taskfiles/agent-tasks-model.sh` (decision 2).
- The wall-time bounds in files of `tests/` directories, among them `adversary2.rs:120` and
  `bounded.rs:97`: `story:time-bounds-in-tests`, with its ledger in
  `story:tests-without-wall-time-bounds`.
- The tests listed in decision 3 as needing none.
- A change of the pid check that the lane uses (`RuntimeDir::left_within`), which already waits.
- Any run with the defect written back to count how often it goes red.

## Placement

Read with `git grep -l -F <path> -- .engineering/planning/story` for each scope path.

- `crates/agent-tasks/tests/adversary2.rs` is also named by `story:test-harness-fewer-processes`
  (implemented), which changed other lines of the file (the harness that reads the task file).
  No edge.
- `crates/agent-tasks/tests/bounded.rs` is named by `story:tests-without-wall-time-bounds`
  (draft), whose ledger lists one statement of it, `bounded.rs:97`
  (`took < Duration::from_secs(4)`). This story edits `:41-64` and the imports; the statement
  stays and the ledger line does not change. No edge.
- `AGENTS.md` is named by 40 stories; this story edits the test-count line of the entry of
  `crates/agent-tasks` (`:868-869`) and adds one sentence after it. No story that is not
  implemented edits those lines. No edge.
- `story:tests-without-wall-time-bounds` lists `adversary2.rs` in its ledger too
  (`adversary2.rs:120`); this story edits other lines of the file and does not change the
  count. No edge.
