---
format: aep.planning-md/3
id: story:agent-tasks-left-process-wait
kind: story
status: implemented
title: Decide that a model's processes ended by a wait, in the three tests that read /proc once
summary: The two model tests read /proc once after a group SIGKILL and take a zombie for a left process (one red run in ten observed); the story makes them, and the same read in bounded.rs, wait with ended_within and shows the fix with two tests that make a zombie themselves.
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/agent-tasks/tests/adversary2.rs
- confidence: cited
  path: crates/agent-tasks/tests/bounded.rs
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T17:30:08Z", actor: "human:operator", revision: 8}
- {from: "proposed", to: "active", at: "2026-11-26T03:56:23Z", actor: "human:operator", revision: 12}
- {from: "active", to: "implemented", at: "2026-11-26T10:01:39Z", actor: "human:operator", revision: 14, decided_on: {"recorded":{"test_result":2,"verification":1}}, executor: "agent:coordinator"}
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
   `/bin/sleep 30` through one helper of the test file. The helper sets `Stdio::null()` for the
   stdin, stdout and stderr of the child, in its own body, and returns the child in a guard
   whose `Drop` calls `kill()` and then `wait()` (both results ignored), so a panic in the
   test leaves no `sleep` behind. Each writes the pid to a file named with the test's
   name and `std::process::id()`, as the two model tests do (`adversary2.rs:257`).
   - `a_killed_process_nobody_has_collected_is_not_left`: sends the child `signal_kill`, waits
     with `ended_within(pid, END_BOUND)` (it returns when the state is `Z`: the test is the
     parent and has not called `wait`), asserts that `comm(pid)` is still `Some("sleep")` (the
     window of the defect, made certain), asserts with `assert_eq!` that
     `sleeping(&file, END_BOUND)` is empty, and then calls `wait` on the child. Red with
     `sleeping` written back as the one `comm` read: it returns the pid, and the message of
     the assertion shows that answer.
   - `a_process_that_still_runs_is_left_and_ended`: asserts with `assert_eq!` that
     `sleeping(&file, Duration::ZERO)` is that pid alone (the process runs for 30 s and the
     bound is zero, so the answer does not depend on the load and no clock literal is in the
     test), asserts the process has ended afterwards (`ended_within(pid, END_BOUND)`, since
     `sleeping` sent it SIGKILL) and calls `wait`. Red with a `sleeping` that keeps nothing:
     the message of the assertion shows the empty answer where the pid was expected.
   With them `adversary2` holds 12 tests and the crate 50. What proves what: the two tests are
   the proof that `sleeping` decides by a wait; the 50 runs below are a check that the three
   named tests show no failure in 50 runs, not a proof. No run with the defect written back is
   asked for, since 1 in 10 is one observation and the two tests need none.
   The stress command, run once by the implementor and by no step of the gate (it is no step of
   `check` or `test`), as one pool job that builds and then runs, from the tree's root, with the
   environment the `test` step sets (`Taskfile.yml:444-452`: `RUSTFLAGS=-D warnings` and
   `SURF_BIN_DIR=<root>/target/debug`; its `build`, `:399-406`, is
   `cargo build --workspace --all-targets --locked` with the same `RUSTFLAGS`):
   `slot --timeout 30m sh -c 'export RUSTFLAGS="-D warnings" SURF_BIN_DIR="$PWD/target/debug";
   cargo build --workspace --all-targets --locked && cargo nextest run -p surf-agent-tasks
   --locked --stress-count 50 -E "test(leaves_no_process)"'`. `--stress-count` needs nextest
   0.9.143 or later, which `.config/nextest.toml:1` already requires, so there is no fallback.
   Arithmetic: 50 x 3 x 1.1 s = 165 s if the runs went one after another. The build of the same
   job is not in that sum, which is why the job names 30 minutes, the most a job may name. A
   job that ends `timed_out` is not started again unchanged: the build and the run are then two
   jobs.

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
  The sentence is written so that `ended_within` and `adversary2` stand on one physical line
  of the file (lines there wrap at about 100 columns), which acceptance 8 checks. The
  paragraph at `:2316-2320` and the entry of `crates/test-process` (`:836`) are not edited.

No file under any `src/` changes and `taskfiles/agent-tasks-model.sh` does not change.
`sleeping` prints nothing, and the report does not rely on a wait that it prints.

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
   written as the one `comm` read; the failure shows the pid that `sleeping` answered in the
   message of the test's assertion. The run command, one pool job that builds as the job of
   decision 4 does and runs the two new tests, no `--stress-count`:
   `slot --timeout 30m sh -c 'export RUSTFLAGS="-D warnings" SURF_BIN_DIR="$PWD/target/debug";
   cargo build --workspace --all-targets --locked && cargo nextest run -p surf-agent-tasks
   --locked -E "test(is_not_left) | test(is_left_and_ended)"'`.
5. `a_process_that_still_runs_is_left_and_ended` passes, and fails with a `sleeping` that keeps
   nothing; the failure shows `sleeping`'s empty answer in the message of the test's assertion.
   The run command is the one of item 4. Each failure of items 4 and 5 is shown once before the
   change is kept, and the report names the output.
6. The stress command of decision 4, run once as written there: it exits 0 and the nextest
   summary shows 50 iterations and no failure. The report states that this is a check of 50
   runs and not a proof.
7. A model that does leave a process still turns its tests red: `kill_group`
   (`agent-tasks-serve.rs:140-148`) written to signal the leader only
   (`.args(["-9", &group.to_string()])`), then, as one pool job with the same `slot --timeout
   30m` build and environment as item 6 and the filter
   `-E "test(leaves_no_process) | test(is_ended_with_its_group)"` and no `--stress-count`,
   these four tests fail, each identified by its message and not by a line, since the edit of
   `bounded.rs` moves the lines:
   `a_model_that_leaves_a_child_leaves_no_process` and
   `a_model_over_its_bound_leaves_no_process` with `children of the model left: [<pid>]` after
   the 5 s wait; `a_command_that_leaves_a_child_leaves_no_process` with `the child of the
   command is left` (the last assertion of the test, `bounded.rs:80 @ a14e3b0`); and
   `a_command_over_its_bound_is_ended_with_its_group`, which fails at its check of the time,
   `the helper took` (`bounded.rs:97 @ a14e3b0`), or at the assertion after it, `the child of
   the command is left` (`:98 @ a14e3b0`), whichever the load of the host gives (with the
   leader alone signalled, the helper takes about 3 s against the check of 4 s). The change is
   put back; the report names the four outputs.
8. In `AGENTS.md`, two commands, each with its output before and after the change:
   - `git grep -n -F -e ended_within --and -e adversary2 -- AGENTS.md` prints nothing before
     (at `a14e3b0`) and one line after, inside the entry of `crates/agent-tasks`;
   - ``git grep -n -F '50 tests (`adversary` 4, `adversary2` 12' -- AGENTS.md`` prints nothing
     before and one line after, and ``git grep -n -F '48 tests (`adversary` 4, `adversary2` 10'
     -- AGENTS.md`` prints one line before and nothing after.
9. The new tests leave nothing behind: `git grep -c -F 'Stdio::null()' --
   crates/agent-tasks/tests/adversary2.rs` prints the count 1 before (`adversary2.rs:219 @
   a14e3b0`, in `drive_without`) and the count 4 after, the three new lines being the body of
   the helper that spawns the child of both new tests (the three streams, each on its own
   line), and
   `git grep -n -F 'impl Drop for' -- crates/agent-tasks/tests/adversary2.rs` prints nothing
   before and the guard after, whose `Drop` calls `kill()` and `wait()`. The pid files are
   named with the test's name and `std::process::id()`.
10. The folder's bounds hold: no test over 60 s, the test step within 120 s, a test over 5 s
    named with its time (a model test that leaves a process waits 5 s only when it fails);
    no pool job over 10 minutes unless it names its time with `slot --timeout` (at most 30
    minutes: the jobs of items 4 to 7 name 30 minutes) and none started again unchanged after
    `timed_out`; every compile or test command carries `slot`; no Python; no model call and no
    network request in a test (the stand-in model is a shell script); the wait's limit is at
    least 10 times the event alone (decision 1).
11. A script that measures or waits starts without the `slot` prefix; the story adds none. No
    command of the story sets `--local`, `--here`, `SLOT_NO_REMOTE` or `SLOT_SHIM_OFF`.

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

## As built

The unit is on `main` at `553b977` (the merge of `impl/agent-tasks-left-process-wait` at
`dea1c18`). It changes 9 files, 207 lines added and 77 removed: the two test files of the
model driver, the `crates/agent-tasks` entry of `AGENTS.md`, and the public test counts. No
file under any `src/` and no file of `taskfiles/` changes.

- What was built.
  - `crates/agent-tasks/tests/adversary2.rs`: `sleeping(pids, bound)` keeps a process whose
    `comm` is `sleep` and for which `ended_within(pid, bound)` is `Err`, then sends it SIGKILL;
    the two model tests pass `END_BOUND`. `spawn_sleep` starts `/bin/sleep 30` with the three
    streams null and returns it in `Spawned`, whose `Drop` calls `kill()` and `wait()`.
  - `crates/agent-tasks/tests/bounded.rs`: `child_left` waits with
    `ended_within(pid, END_BOUND)` and sends SIGKILL to what is still there; `running` is
    removed. Imports: `END_BOUND` and `ended_within`.
  - `AGENTS.md`, entry of `crates/agent-tasks`: 52 tests (`adversary2` 13, `bounded` 5) and one
    sentence that the tests of the model driver decide by a wait, never by one read.
  - Four cases, each shown red by one defect written back:
    - `a_killed_process_nobody_has_collected_is_not_left` (`adversary2`): the test kills a
      child it has not collected, waits until it is a zombie, checks that `comm` still answers
      `sleep`, and `sleeping` must answer nothing. Red: `sleeping` as the one `comm` read
      (answer `[194663]`).
    - `a_process_that_still_runs_is_left_and_ended` (`adversary2`): a running `sleep` and a
      bound of zero; `sleeping` must answer that pid alone and end it. Red: a `sleeping` that
      keeps nothing (answer `[]`).
    - `a_process_that_ends_during_the_wait_is_not_left` (`adversary2`): `/bin/sleep 0.5` still
      runs at the first read and ends inside `END_BOUND`; `sleeping` must answer nothing. Red:
      the bound cut to zero, or `sleeping` as one `comm` read.
    - `a_child_that_ends_during_the_wait_is_not_left` (`bounded`): the same for `child_left`,
      which must answer `None`. Red: the bound cut to zero.
  - The model tests and the two `bounded` tests that end with "no process is left" go red with
    `kill_group` signalling the leader only: 4 of 4, each with its own message.
- What differs from the story. Every decision, Change line and acceptance item not named here
  was built as written.
  - Decision 4 calls the two new tests the proof that `sleeping` decides by a wait. They are
    not: with the wait cut to one read of the state both stayed green (one waits for the zombie
    itself, the other passes a bound of zero). The review added the third and fourth case
    above. The crate has 52 tests, not 50 (`adversary2` 13, `bounded` 5): the count line of
    `AGENTS.md` says so, and the story's grep for 50 tests and `adversary2` 12 prints nothing
    at the tip.
  - `bounded.rs` gains a test; the Change line for it named `child_left` and `running` only.
  - Acceptance 9: `Stdio::null()` counts 7 in `adversary2.rs` (story: 4) and 4 in `bounded.rs`
    (1 before): each of the two review cases spawns its `sleep` in its own body with the three
    streams null, and only `adversary2.rs` has the guard `Spawned`; `bounded.rs` kills and
    waits in its case.
  - The public numbers: `README.md`, three pages of `website/docs`, `docs/badges/make.sh` and
    `docs/badges/tests.svg` change (`2552` to `2556` passed, the gate block); the story's
    scope names `AGENTS.md` and the two test files.
  - Acceptance 6: the 50-run job ran on the first commit, before the two review cases; the
    filter `test(leaves_no_process)` matches three tests and neither review case, so it was
    not run again.
  - Acceptance 7, fourth test: its time check passed and it failed at the assertion after it,
    the second of the two forms the story allows.
  - The review cases first came as nested modules (`review_left_process.rs`, then `wait.rs`)
    and are tests of `adversary2.rs` and `bounded.rs` at the end; no nested file is left.
- The review (one pass on the first commit; verdict NEEDS-CHANGE, origin all introduced):
  - 7 findings: 1 must (the two new tests could not tell the wait from one read of the
    state), 1 should (no test held the wait in `child_left`), 5 notes: a pid whose `comm` is
    not `sleep` is decided by one read of `comm` and nothing reaches it; the new waits are
    clock bounds; a process read `R` right after `kill -9`; the counts of `AGENTS.md` become
    52, 13 and 5 once the cases merge; the diff does every item of the story's Change and
    nothing outside it. A run with `END_BOUND` set to zero left the four repaired tests green
    and failed only the first new test, at its own precondition and racily.
  - 2 cases written, before any run, both green at the tip and red under the mutants; each
    mutant was made in the reviewer's tree and taken back.
  - Acceptance 4 and 5 repeated on the commit: both red runs reproduced; the crate ran 52 of 52.
  - Fix round 1 (merge of the two cases, then a commit): renamed the case files, set the counts
    of `AGENTS.md` to 52, 13 and 5; both cut-bound mutants red on the tip.
  - Fix round 2: moved the two cases into their parent files, deleted the nested files, changed
    one doc comment to "no other test of this file" and one state name; `task test-layout`
    exit 0; both cut-bound mutants red; the crate ran 52 of 52. The run of the first command
    of the round (no `SURF_BIN_DIR`) failed 16 tests for the environment; the run with the
    gate's environment (programs built, `SURF_BIN_DIR` set) passed.
  - The merge of `main` (surfd cases, the DOM's reading members) changed no test file of the
    unit; `AGENTS.md` took in the entries of `main`. The last commit set the public numbers.
- Numbers:
  - Tests of `surf-agent-tasks`: 48 before, 52 after (50 after the first commit).
  - Workspace: 2,003 tests before (2,000 passed, 3 ignored), 2,007 after (2,004 passed,
    3 ignored), from the gate logs of the DOM story's tip and of the merge's tree; the 4 are
    the four cases.
  - The 50-run job, as printed: `Summary [  57.506s] 50/50 stress run iterations: 50 passed`
    (150 passes, slowest test 1.249 s); a check of 50 runs, not a proof.
  - Time of the new cases in the last run: `a_killed_process_nobody_has_collected_is_not_left`
    0.013 s, `a_process_that_still_runs_is_left_and_ended` 0.025 s,
    `a_process_that_ends_during_the_wait_is_not_left` 0.532 s,
    `a_child_that_ends_during_the_wait_is_not_left` 0.530 s; the crate ran in 11.163 s.
  - Over 5 s: only the four tests of the leader-only defect run (5.0 to 8.2 s, the failing path
    that waits `END_BOUND`) and `tasks::a_call_that_never_answers_ends_at_its_bound` (10.482 s,
    not touched). None over 60 s.
  - The crate ran in 12.586 s before the unit.
- Left as it is:
  - The two review cases stand on a `sleep` of 0.5 s: on a host where the first read comes more
    than 0.5 s after the spawn, a one-read `sleeping` passes them too; the real code passes
    there.
  - The new waits are clock bounds (`ended_within` against `END_BOUND`, 5 s, polling every
    10 ms) in `adversary2.rs` (`sleeping`, `:293`, `:321`) and `bounded.rs:52`; the older bounds
    at `adversary2.rs:122`, `:140` and `bounded.rs:90` stay.
  - A process read `R` in `/proc/<pid>/stat` right after `kill -9` returned: the race of one
    read of the state, which the story refused, happens on the pool's hosts.
  - `cargo fmt --all -- --check` fails on 445 blocks in 41 files (447 on the remote build),
    none of the unit's; the remote `cargo fmt` ignored `rustfmt.toml` (edition 2024) and
    ordered two `use` lines of `adversary2.rs` in the older style. `rustfmt --edition 2024
    --config-path rustfmt.toml --check` on the two test files: exit 0.
- A later story takes up: `story:time-bounds-in-tests` (the new clock bounds above, with the
  ledger of `story:tests-without-wall-time-bounds`, which lists `adversary2.rs:120` and
  `bounded.rs:97`).
