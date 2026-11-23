---
format: aep.planning-md/3
id: story:test-harness-fewer-processes
kind: story
status: draft
title: The test harnesses start fewer programs and wait shorter clocks
summary: html-conformance reads its base in one git call and makes its fixture once, test directories find their processes among the test's descendants, five driver tests run one task, four clocks drop to 250 ms, two bounds shrink
relations:
- decomposes: epic:headless-rust-target
- informed_by: story:test-step-holds-with-tls
- informed_by: story:time-bounds-in-tests
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/agent-tasks/tests/adversary.rs
- confidence: cited
  path: crates/agent-tasks/tests/adversary2.rs
- confidence: cited
  path: crates/agent-tasks/tests/model_driver.rs
- confidence: cited
  path: crates/agent-tasks/tests/one-task.txt
- confidence: cited
  path: crates/html-conformance/src/check.rs
- confidence: cited
  path: crates/html-conformance/tests/check.rs
- confidence: cited
  path: crates/session/tests/built.rs
- confidence: cited
  path: crates/session/tests/common/mod.rs
- confidence: cited
  path: crates/session/tests/events.rs
- confidence: cited
  path: crates/surfd/tests/built.rs
- confidence: cited
  path: crates/surfd/tests/common/mod.rs
- confidence: cited
  path: crates/surfd/tests/in_process.rs
- confidence: cited
  path: crates/surfd/tests/view.rs
- confidence: cited
  path: crates/view-fixture/tests/adversary_commands.rs
- confidence: cited
  path: crates/view-fixture/tests/fixture.rs
- confidence: cited
  path: crates/view/tests/http.rs
- confidence: cited
  path: taskfiles/agent-tasks-model.sh
revision: 19
---
## Why

The rule of the folder this repository is worked in ("Tests stay fast"): the test step ends within
120 s of wall time and an expensive step does not run inside each test. On `main` at `96145a2`
four runs of `slot --timeout 15m task test` (2026-11-23, 01:07 to 01:19 CET) printed
`test step:` 131.6 s, 71.4 s, 132.6 s and 102.9 s, at a 1-minute load of 205 to 127, 127 to 106,
106 to 137 and 137 to 153. `story:test-step-holds-with-tls` asks for at most 90 s at a load of
100 or more.

Two facts of the step decide where time goes:

- Five tests run at once (the pool's daemon starts with `--test-threads 5`; summed test time over
  nextest's `Summary` is 4.87 to 4.94 in the four runs' JUnit reports). One second of summed
  test time is about 0.2 s of wall time.
- In the pool a started program costs 20 to 30 times its time outside it: the pool's jobs share a
  CPU quota of 1800 % at CPU weight 20 (`systemctl --user show slot.service`), and the host's CPU
  pressure ("some", 10 s) read 85 %. One `task lock-check` takes 171 to 346 ms outside the pool
  (load 137) and about 4.2 s inside it (module `lock_check` alone, load 156 to 173).

So the cost of a test harness is mostly the programs it starts and the clocks it waits out. Times
below: "third run" is the run of 132.6 s, summed from its JUnit report; "alone" is one module run
by `cargo nextest run -p <package> -E 'test(/^<module>::/)'` in the pool on `96145a2`, five tests
at once, with the load read before and after.

1. `surf-html-conformance` `check` (17 tests; 22.9 s in the third run; alone 32.1 s summed,
   `Summary` 7.5 s, load 161 to 164). Each of 15 fixtures starts git 4 times: `git init`
   (`crates/html-conformance/tests/check.rs:152`), then `add`, `commit` and `rev-parse`
   (`:214-218`); the committed base is the same in every test. Each of the 22 harness runs
   (`:221-231`) starts git 7 times: `rev-parse` (`crates/html-conformance/src/check.rs:616`), and
   for each of the three expected files `cat-file -e` (`:634`) and `show` (`:642`), called from
   `:296`. About 258 programs in the module, counted from the code, not traced.
2. The drop of a test directory in `surfd` (`crates/surfd/tests/common/mod.rs:168-173`, which
   calls `stop_processes_of`, `:191-219`) and in `surf-session`
   (`crates/session/tests/common/mod.rs:123-136` over `processes`, `:93-120`) reads
   `/proc/<pid>/environ` of every process of the host: 1,292 to 1,308 processes, 40 to 162 ms per
   scan (five scans by one `grep` outside the pool, load 162). The test files create 81 such
   directories (`TestDir::new` 39 in `surf-session`, 42 in `surfd`); those of `events` (15),
   `adversary_events`, `adversary_parse`, `adversary_view` (1 each), `in_process` (14), `log` (2)
   and `view` of `surfd` (11) never start a built `surf-agent`, so every process of their cases
   descends from the test process. Only a built `surf-agent` starts a process that does not:
   its `surfd` runs in a process group of its own (`crates/agent/src/client.rs:202-203`) and is
   reparented when `surf-agent` exits. The kernel of this host lists a process's children in
   `/proc/<pid>/task/<tid>/children` (`CONFIG_PROC_CHILDREN=y`).
3. The model lane's driver. Twelve tests run `taskfiles/agent-tasks-model.sh` over all four
   tasks of `crates/agent-tasks/tasks.txt` (`crates/agent-tasks/tests/model_driver.rs:21-47`,
   `adversary2.rs:186-216`, `adversary.rs:67-87`). With the stub model one run starts 75
   programs in 52 processes (strace -f outside the pool, load 153) and takes 485 ms and 697 ms
   outside the pool (load 172); alone, `model_driver` took 27.4 s summed (load 172 to 179) and
   `adversary2` 29.3 s (load 169 to 173). Five tests read only the line of the task `article`:
   `model_driver::a_control_character_in_an_answer_is_written_as_hex` and
   `a_carriage_return_in_an_answer_is_escaped`, `adversary2::every_call_is_one_line_that_reads_back`
   and `a_model_that_leaves_a_child_leaves_no_process`, and
   `adversary::a_call_with_a_line_feed_in_a_word_is_one_call`: 4.30, 2.02, 6.34, 5.24 and 3.53 s
   in the third run, 21.4 s together. The script reads its tasks from a fixed path (`:79`).
4. Clocks of 1 s set through in-process values, which the tests wait out:
   - `surf-view` `http::case_23_a_client_that_reads_nothing_is_closed_at_the_write_limit` waits a
     write limit of 1 s twice (`crates/view/tests/http.rs:842-886`; 3.07 s third run, 2.99 s
     alone) and `case_20_half_a_request_is_closed_without_answer_after_the_receive_limit` a
     receive limit of 1 s (`:775-786`; 1.14 s, 1.78 s);
   - `surfd` `in_process::case_09_a_session_that_never_reports_ready_is_killed` sets
     `session_ready_timeout` 1 s (`crates/surfd/tests/in_process.rs:165`; 2.11 s, 2.61 s) and
     `case_10_a_session_that_stalls_times_out_and_stays_open` `session_answer_timeout` 1 s
     (`:196`; 2.12 s, 2.97 s);
   - `surfd` `built::surfd_started_by_hand_writes_no_view_token_unless_asked` runs two `surfd`
     with `--idle-exit 1` one after the other (`crates/surfd/tests/built.rs:444-466`; 3.36 s,
     4.17 s); 1 s is the option's smallest value.
5. `surf-session` `events::case_14_a_subscriber_that_never_reads_is_disconnected` sends 86 `go`
   through a real session (`QUEUE / GO_ENTRIES + 1`, `crates/session/tests/events.rs:706`,
   `:713`; `GO_ENTRIES` 12 at `:59`) to pass the reader queue of 1,024 entries; 5.84 s third run,
   5.27 s alone (load 156). The queue is a field of the configuration,
   `log::Bounds::reader_queue` (`crates/surfd/src/log.rs:61`, default `READER_QUEUE` at `:49`).
6. `surf-view-fixture`: both tests run `cargo build -p surf-agent -p surf-session`
   (`crates/view-fixture/tests/fixture.rs:25-50`, `adversary_commands.rs:15-40`) although the
   step sets `SURF_BIN_DIR` after `task build`, which `crates/session/tests/common/mod.rs:229-231`
   reads. That cargo found everything fresh in 0.99 s, and in 3.94 s when it waited for another
   session's lock on the package cache (load 154 to 155); the module took 6.9 s in the third run
   and 13.2 s summed alone (load 167 to 168).
7. `surfd` `view::case_17_a_stream_from_a_number_replays_or_resets` sends 11 `go` to pass
   `entries_per_session = 10` (`crates/surfd/tests/view.rs:451-459`); 4.71 s third run, 5.30 s
   alone.
8. `surf-session` `built::other_argument_shapes_exit_2` starts `surf-session` 12 times
   (`crates/session/tests/built.rs:302-347`) and `more_argument_shapes_exit_2` about 20 times
   (`:459-498`) to see exit 2; 1.02 s and 2.20 s third run, 1.80 s and 1.96 s alone. The parser
   is callable in process, `process::arguments` (used at `:412`).
9. `surfd`'s `bin_dir()` (`crates/surfd/tests/common/mod.rs:339-357`) copies three programs,
   4.65 MB (`examples/agent` 1.17 MB, `surfd` 2.44 MB, `examples/stand_in_session` 1.04 MB), in
   every test process that calls it: its `OnceLock` holds once per process, and nextest runs each
   test in a process of its own, the form the folder's rule names as not "made once".

Expected, as summed test time of the third run: 1 about 15 s (about 258 programs become about
110), 2 about 2 to 7 s outside the pool (45 scans skipped; in the pool not measured), 3 about 10 s
(about 8 programs per task, three tasks fewer in five tests), 4 about 4.75 s (0.75 s per shortened
wait, about 1 s for the two `surfd`), 5 about 5 s, 6 about 2 to 8 s, 7 about 2.5 s, 8 about 2.5 s,
9 under 1 s (not measured). About 44 to 56 s summed, 9 to 11 s of wall time. The changed tests
are bound by acceptance, not by these estimates.

### Round 2 branches

`git diff --stat 96145a2 impl/round2-commands` and `... impl/round2-view` (both branch from
`96145a2`) over every file of Scope:

- `impl/round2-commands`: `crates/session/tests/common/mod.rs` (+55, after `:611`: `name`,
  `dialed`, `resolve`, `connect`), `crates/session/tests/events.rs` (+329; hunks from `:877`,
  in `covered`, `expected_value`, `case_17_every_event_maps_with_all_its_fields` and after
  `:1336`), `AGENTS.md` (hunks at `:283` to `:845`, `:1431`, `:1434`).
- `impl/round2-view`: `crates/session/tests/events.rs` (+215; hunks from `:877` and after
  `:1336`), `AGENTS.md` (hunks at `:283` to `:615`, `:797`, `:1434`, `:1457`, `:1561`).
- No other file of Scope changes on either branch.

Statement 2 changes `TestDir` (`:92-136`) of `crates/session/tests/common/mod.rs`, not a function
the branch changes; statement 5 changes `case_14` (`:703-747`), which neither branch touches;
`AGENTS.md` `:1546-1557` is in no hunk of either. No proposal is left out for an overlap.

## Change

All on `main`, from `96145a2`. One unit; story `test-step-scripts-and-binaries` is built beside
it and shares no code file with it.

1. `surf-html-conformance` `check`.
   - The harness (`crates/html-conformance/src/check.rs`) reads the three expected files at the
     base with one `git cat-file --batch` per run, after the `rev-parse` of `:616`, in place of
     the `cat-file -e` and `show` per file (`:634`, `:642`): 2 git programs per run instead of
     7. A file missing at the base is skipped as today, and every failure line stays.
   - The tests (`crates/html-conformance/tests/check.rs`) make the committed fixture once per
     test run: the first test that needs it writes it, with its base commit, into a directory
     under `CARGO_TARGET_TMPDIR` named by a hash of the fixture's bytes and moved into place by
     rename; every test copies that directory in process (`std::fs`, no program) and edits its
     copy. A test that commits again after its edit keeps its own `git` calls.
2. The drop of a test directory in `surfd` and `surf-session` finds the case's processes among
   the descendants of the test process (`/proc/<pid>/task/<tid>/children`, followed down) and
   reads `environ` only of those. A case that ran a built `surf-agent` (the helper `agent()`,
   `crates/surfd/tests/common/mod.rs:378`, `crates/session/tests/common/mod.rs:274`) marks its
   directory, and its drop still reads every process's `environ`, as today. When the children
   file cannot be read the drop reads every process too. An existing test of `surfd`
   `in_process` gains the proof: a case whose daemon is leaked (`std::mem::forget`) still has its
   session process killed by the drop.
3. The driver reads its tasks from `AGENT_TASKS_FILE` when it is set, else from
   `crates/agent-tasks/tasks.txt` (`taskfiles/agent-tasks-model.sh:79`; its head says so). The
   five tests of Why 3 set it to `crates/agent-tasks/tests/one-task.txt`, the block of `article`
   from `tasks.txt`, and each asserts that the first line of the out file counts `1 tasks`. The
   other seven driver tests keep all four tasks.
4. The clocks of Why 4 become 250 ms: the write limit of `case_23` and the receive limit of
   `case_20` (whose lower bound becomes 90 % of the limit, as 900 ms is of 1 s today), and
   `session_ready_timeout` of `case_09` and `session_answer_timeout` of `case_10`. Each keeps its
   assertion that the event came no earlier than its limit. `surfd_started_by_hand_writes_no_view
   _token_unless_asked` runs its two `surfd` at the same time, each in a directory of its own.
5. `case_14` sets `config.log.reader_queue` to 24 through `surfd_in_process`, and its `QUEUE`
   becomes 24 (3 `go`). It gains the assertion that `surfd::log::READER_QUEUE` is 1,024, the
   story's number.
6. The two tests of `surf-view-fixture` take the programs from `SURF_BIN_DIR` when it is set, as
   `crates/session/tests/common/mod.rs:229-231` does, and build them with cargo only when it is
   not.
7. `case_17` of `surfd` `view` sets `entries_per_session` to 3 and sends the fewest `go` that pass
   it; its asserted numbers follow from those two values.
8. `other_argument_shapes_exit_2` and `more_argument_shapes_exit_2` assert each shape through
   `process::arguments` in process; each keeps one run of the binary per outcome it asserts
   (exit 2, nothing on standard output, no socket bound).
9. `surfd`'s `bin_dir()` makes its directory once per test run: hard links to the three programs,
   remade only when an entry's inode differs from its program's; a copy as today when a hard link
   fails. An existing test of `surfd` `built` gains the proof that a stale entry is replaced.
10. `AGENTS.md` `:1546-1557` (line numbers of `96145a2`): the waits of statement 4 and how a test
    directory finds its processes. Story `test-step-scripts-and-binaries` changes `:131-134`,
    `:165-169`, `:247-266`, `:1191-1224`, `:1302-1319` and `:1323-1359`, none of these.

Machine rules for every command this story names: builds and tests go through the pool (`cargo`
by its shim, any other command that compiles or tests with the `slot` prefix); a script that
measures, loops or waits starts without the prefix; a pool job over 10 minutes names its time
(`slot --timeout <t>`, at most 30 minutes); no Python, also not in a probe; no network and no
model call in a test.

## What it does not do

- No tasks seam in `lane::Config` for `tasks::a_call_that_never_answers_ends_at_its_bound` (a lane
  of two tasks): about 4 s summed, 0.8 s of wall time, does not pay for a new field of the lane.
- No shorter heartbeat in `http::case_32_an_idle_stream_receives_the_keep_alive_comment`: it has
  no lower bound, so a heartbeat not enforced still passes (`story:time-bounds-in-tests`).
- No clock the test controls in `crates/view` or `crates/fetch`, and no change of the upper
  bounds `crates/surfd/tests/in_process.rs:179` and `:206`: `story:time-bounds-in-tests`.
- No change of `kill_grace` in `in_process::case_24` and no change of a wait of 1 s that is an
  option's smallest value (`--idle-exit`, `--idle-timeout`, `--request-timeout`).
- No change of `crates/agent/src/client.rs` (round 2 holds it), and the host-wide scan stays for
  a case that ran a built `surf-agent`.
- No file of story `test-step-scripts-and-binaries`: `Taskfile.yml`, `taskfiles/` but
  `agent-tasks-model.sh`, the manifests, `crates/surf/tests`.
- No change of the number of tests at once, of `.config/nextest.toml`, of the budget or of a
  timeout; no test removed, merged or added.

## Acceptance

1. Programs per run, counted with `strace -f -e trace=execve` outside the pool, before and after,
   quoted with the counts per program:
   - the driver with the stub model over the four tasks (75 in 52 processes before) and over
     `one-task.txt` after: the second at most 60 % of the first;
   - one `surf-html-conformance --root <fixture> --check --base <commit>`: 8 before (1 harness, 7
     git), at most 3 after; one test of `check` that does not commit again: 4 git before, 0 after;
   - each test of `surf-view-fixture` run with `SURF_BIN_DIR` set: no `cargo` after;
   - the `environ` files one drop opens (`strace -e trace=openat`) in a case of `surfd`
     `in_process`: about 1,300 before, under 20 after.
2. Each changed module alone, `cargo nextest run -p <package> -E 'test(/^<module>::/)'`, before
   (at `96145a2`) and after, in three alternating pairs, with `/proc/loadavg` before and after each
   run: `surf-html-conformance` `check`; `surf-agent-tasks` `model_driver`, `adversary2`,
   `adversary`; `surfd` `built`, `in_process`, `log`, `view`; `surf-session` `events`, `built`,
   `adversary_events`, `adversary_parse`, `adversary_view`; `surf-view` `http`; `surf-view-fixture`
   `fixture`, `adversary_commands`. Reported; `check` and `case_14` alone each at most half their
   before time in each pair.
3. One plant per changed test or script, removed after (`git diff --stat` the same before and
   after), the failure text quoted:
   - the batch read returning the blob of another path: `an_expected_count_below_its_value_at_the
     _base_fails` fails;
   - the fixture cache serving a fixture of other bytes (one constant changed after the cache was
     made): a test of `check` fails;
   - a case process left alive: the leaked daemon of statement 2 with the walk of children taken
     out fails its test; with the walk in, the process is killed;
   - the driver ignoring `AGENT_TASKS_FILE`: the five tests fail on `1 tasks`;
   - each shortened clock not enforced (the write limit, the receive limit, the ready timeout, the
     answer timeout, `--idle-exit`): its test fails;
   - the reader queue not bounded: `case_14` fails; the per-session bound not applied: `case_17`
     fails;
   - `process::arguments` accepting `--request-timeout=1`: `more_argument_shapes_exit_2` fails;
   - `SURF_BIN_DIR` naming a directory without `surf-session`: a test of `surf-view-fixture`
     fails naming it;
   - `bin_dir()` keeping a stale entry: the proof of statement 9 fails.
4. `cargo nextest list --workspace` without the TLS packages gives the same 893 test names before
   and after (the sorted lists compared, not only the count).
5. No test over 60 s. Every changed test is timed alone after the change; one over 5 s is named
   with its time. No timeout is raised and no retry is added (`git diff` shows none).
6. Three runs of `slot --timeout 15m task test` on the story's commit, each with `/proc/loadavg`
   before and after and its line `test step: N s`. Reported, not bounded here; the bound is
   acceptance 1 of `story:test-step-holds-with-tls`.
7. The round 2 check of this story is run again on the story's commit and quoted: no new file of
   Scope in either branch's `git diff --stat`, and no hunk of either in a changed function.
8. `git diff --stat 96145a2` lists only the files of Scope.
9. The gate `slot --timeout 25m task check-runner` on the story's commit, run by the coordinating
   session, passes; its line `test step: N s of its budget of 120 s` is quoted.

## Scope

`crates/html-conformance/src/check.rs`, `crates/html-conformance/tests/check.rs`;
`crates/surfd/tests/common/mod.rs`, `crates/surfd/tests/in_process.rs`,
`crates/surfd/tests/built.rs`, `crates/surfd/tests/view.rs`;
`crates/session/tests/common/mod.rs`, `crates/session/tests/events.rs`,
`crates/session/tests/built.rs`; `taskfiles/agent-tasks-model.sh`,
`crates/agent-tasks/tests/model_driver.rs`, `crates/agent-tasks/tests/adversary2.rs`,
`crates/agent-tasks/tests/adversary.rs`, `crates/agent-tasks/tests/one-task.txt`;
`crates/view/tests/http.rs`; `crates/view-fixture/tests/fixture.rs`,
`crates/view-fixture/tests/adversary_commands.rs`; `AGENTS.md` at `:1546-1557` (line numbers of
`96145a2`).
