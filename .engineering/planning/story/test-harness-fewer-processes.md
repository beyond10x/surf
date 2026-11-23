---
format: aep.planning-md/3
id: story:test-harness-fewer-processes
kind: story
status: implemented
title: The test harnesses start fewer programs and wait shorter clocks
summary: html-conformance reads its base in one git call and commits its fixture once per digest, test directories find their processes among the test's descendants unless a helper that starts a built program marked them, five driver tests run one task, two view clocks drop to 250 ms, two log bounds shrink, surfd's bin directory is linked once per run
relations:
- decomposes: epic:headless-rust-target
- informed_by: story:test-step-holds-with-tls
- informed_by: story:time-bounds-in-tests
scope:
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
  path: crates/view-fixture/tests/common/mod.rs
- confidence: cited
  path: crates/view-fixture/tests/fixture.rs
- confidence: cited
  path: crates/view-fixture/tests/main.rs
- confidence: cited
  path: crates/view/tests/http.rs
- confidence: cited
  path: taskfiles/agent-tasks-model.sh
revision: 28
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T01:22:29Z", actor: "human:operator", revision: 25}
- {from: "proposed", to: "active", at: "2026-11-23T01:22:30Z", actor: "human:operator", revision: 26}
- {from: "active", to: "implemented", at: "2026-11-23T02:54:33Z", actor: "human:operator", revision: 28, decided_on: {"recorded":{"test_result":4,"verification":2}}, executor: "agent:coordinator"}
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
   `Summary` 7.5 s, load 161 to 164). At run time the module makes 17 fixtures: `Fixture::new`
   has 15 call sites in `crates/html-conformance/tests/check.rs`, one of them in `all_passing`
   (`:428`), which three calls run. Each fixture starts git 4 times: `git init` (`:152`), then
   `add`, `commit` and `rev-parse` in `commit()` (`:214-218`). 12 fixtures commit the bytes
   `Fixture::new` wrote, so their base is the same tree. 5 fixtures, of 4 tests, change the files
   before their first commit, so each has a base of its own: the three of `all_passing` write the
   passing data of `test1.test`, a note with other blocks and two edits of the tokenizer's
   expected file (`:430-443`); `a_count_above_the_expected_count_passes` edits that file twice
   (`:492-497`); `a_section_without_a_data_file_is_printed_incomplete_with_its_reasons` removes
   the expected files (`:533-534`). No fixture commits a second time. The module runs the
   harness 24 times (`check()`, 22 call sites, one in `all_passing`), and each run starts git up
   to 7 times: `rev-parse` (`crates/html-conformance/src/check.rs:616`), and for each of the
   three expected files `cat-file -e` (`:634`) and `show` (`:642`), called from `:296`. About
   260 programs in the module (17 times 4 git, 24 times 1 harness and 7 git), counted from the
   code, not traced.
2. The drop of a test directory in `surfd` (`crates/surfd/tests/common/mod.rs:168-173`, which
   calls `stop_processes_of`, `:191-219`) and in `surf-session`
   (`crates/session/tests/common/mod.rs:123-136` over `processes`, `:93-120`) reads
   `/proc/<pid>/environ` of every process of the host: 1,292 to 1,308 processes, 40 to 162 ms per
   scan (five scans by one `grep` outside the pool, load 162). The test files create 81 such
   directories (`TestDir::new` 39 in `surf-session`, 42 in `surfd`). In 44 of them no helper
   starts a built program, so every process of their cases descends from the test process:
   `surf-session` `events` (15), `adversary_events`, `adversary_parse`, `adversary_view` (1
   each), `surfd` `in_process` (14), `log` (2) and `view` (10 of 11; `case_27`, `view.rs:655`,
   starts `surfd` with `Surfd::start`). The programs the in-process daemon starts are children of
   the test process. A program a helper starts can leave the test's tree: a built `surf-agent`'s
   `surfd` runs in a process group of its own (`crates/agent/src/client.rs:202-203`) and is
   reparented when `surf-agent` exits; `Surfd`'s drop (`crates/surfd/tests/common/mod.rs:486-488`)
   kills its `surfd` before the `TestDir` drops (reverse declaration order), so a session that
   `surfd` started would be reparented. The kernel of this host lists a process's children in
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
     write limit of 1 s twice (`crates/view/tests/http.rs:839-886`, the limit at `:843`; 3.07 s
     third run, 2.99 s alone) and `case_20_half_a_request_is_closed_without_answer_after_the_
     receive_limit` a receive limit of 1 s (`:773-791`, the limit at `:776`; 1.14 s, 1.78 s);
   - `surfd` `in_process::case_09_a_session_that_never_reports_ready_is_killed` sets
     `session_ready_timeout` 1 s (`crates/surfd/tests/in_process.rs:165`; 2.11 s, 2.61 s) and
     `case_10_a_session_that_stalls_times_out_and_stays_open` `session_answer_timeout` 1 s
     (`:196`; 2.12 s, 2.97 s); this story keeps both (statement 4);
   - `surfd` `built::surfd_started_by_hand_writes_no_view_token_unless_asked` runs two `surfd`
     with `--idle-exit 1` one after the other (`crates/surfd/tests/built.rs:444-466`; 3.36 s,
     4.17 s); 1 s is the option's smallest value.
5. `surf-session` `events::case_14_a_subscriber_that_never_reads_is_disconnected` sends 86 `go`
   through a real session (`QUEUE / GO_ENTRIES + 1`, `crates/session/tests/events.rs:706`,
   `:713`; `GO_ENTRIES` 12 at `:59`) to pass the reader queue of 1,024 entries; 5.84 s third run,
   5.27 s alone (load 156). The queue is a field of the configuration,
   `log::Bounds::reader_queue` (`crates/surfd/src/log.rs:61`, default `READER_QUEUE` at `:49`).
6. `surf-view-fixture`: both tests run `cargo build -p surf-agent -p surf-session`
   (`crates/view-fixture/tests/fixture.rs:25-50`, `adversary_commands.rs:15-40`, two copies of
   one `bin_dir()`) although the step sets `SURF_BIN_DIR` after `task build`, which
   `crates/session/tests/common/mod.rs:229-231` reads. That cargo found everything fresh in
   0.99 s, and in 3.94 s when it waited for another session's lock on the package cache (load 154
   to 155); the module took 6.9 s in the third run and 13.2 s summed alone (load 167 to 168).
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

Expected, as summed test time of the third run: 1 about 15 s (about 260 programs become about 90),
2 about 2 to 7 s outside the pool (44 scans skipped; in the pool not measured), 3 about 10 s
(about 8 programs per task, three tasks fewer in five tests), 4 about 3.25 s (0.75 s for each of
three shortened waits, about 1 s for the two `surfd`), 5 about 5 s, 6 about 2 to 8 s, 7 about
2.5 s, 8 about 2.5 s, 9 under 1 s (not measured). About 42 to 54 s summed, 8.5 to 11 s of wall
time. The changed tests are bound by acceptance, not by these estimates.

### Round 2 branches

`git diff --stat 96145a2 impl/round2-commands` and `... impl/round2-view` (both branch from
`96145a2`) over every file of Scope:

- `impl/round2-commands`: `crates/session/tests/common/mod.rs` (+55, after `:611`: `name`,
  `dialed`, `resolve`, `connect`); `crates/session/tests/events.rs` (+329; `git diff -U0` hunks
  at `:26` and `:34`, the `use` lines of `surf_types::agent` and `surf_types::navigation`; at
  `:877`, `:952`, `:977`, `:982`, `:986` and `:1024` in `covered`; at `:1055` and `:1062` in
  `expected_value`; at `:1096` and `:1099` in `case_17_every_event_maps_with_all_its_fields`;
  and after `:1336`).
- `impl/round2-view`: `crates/session/tests/events.rs` (+215; hunks at `:877`, `:977`, `:982`
  and `:986` in `covered`, at `:1055` in `expected_value`, and after `:1336`);
  `crates/view-fixture/tests/main.rs` (+1, `mod acceptance;` before `mod adversary_commands;`);
  and a new `crates/view-fixture/tests/acceptance.rs` with a third copy of `bin_dir()` (`:21`),
  which runs cargo.
- No other file of Scope changes on either branch.

This story changes `TestDir` (`:92-136`), `agent()` (`:274`) and `Started::start` (`:359`) of
`crates/session/tests/common/mod.rs`, no function a branch changes; in `events.rs` it changes
`case_05` (`:422-432`) and `case_14` (`:703-747`), which neither branch touches, and adds no
`use` line. In `crates/view-fixture/tests/main.rs` it adds `mod common;` after
`mod adversary_commands;`, so the two added lines share no line. No proposal is left out for an
overlap.

## Change

All on `main`, from `96145a2`. One unit. Its scope no longer holds `AGENTS.md` (D281, statement
10), so it shares no file with story `test-step-scripts-and-binaries`, and `aep plan artifact
waves` reports no collision between the two (on `1f53d35` it reported one, `AGENTS.md`, and put
them in waves 11 and 12). By the scopes of the store on `1f53d35`, this story shares files with
stories built after these two: `story:page-test-step-within-budget` (`crates/view-fixture`),
`story:agent-cookie-jar` (`crates/agent-tasks`, `crates/session/tests`),
`story:agent-tasks-https` (`crates/agent-tasks`) and `story:tls-session-anchors`
(`crates/session/tests`).

1. `surf-html-conformance` `check`.
   - The harness (`crates/html-conformance/src/check.rs`) reads the three expected files at the
     base with one `git cat-file --batch` per run, after the `rev-parse` of `:616`, in place of
     `cat-file -e` (`:634`) and `show` (`:642`) per file: 2 git programs per run instead of 7.
     Each answer maps to today's lines, file by file at its place in the group loop: `missing`
     is skipped, as a failing `cat-file -e` is today; a blob is parsed as today; an object of
     another type (a tree at the path, where `cat-file -e` passes today and `parse_expected`
     refuses `git show`'s listing) makes the harness run `git show` for that file as today, so
     its line is today's; a git that cannot start gives `<path> at <base>: git could not run:
     <error>` for each of the three files, as today; a batch that ends before it answered a file
     gives `<path> at <base>: git cat-file: <its standard error>` for that file where today
     `git show:` stands, the one line whose text changes (no test reaches it).
   - The tests (`crates/html-conformance/tests/check.rs`) make the committed base once per
     digest of its bytes. `Fixture::new` hashes every path and content it would write. When
     `CARGO_TARGET_TMPDIR/check-base/<hash>` exists it copies the repository held there into the
     test's directory in process (`std::fs`, no program) and takes the base commit from the
     file beside that repository. Otherwise it writes the fixture, runs today's 4 git, and moves
     `<hash>.part-<pid>` (the repository and the file of its base) into place by `rename`; when
     that rename fails because another test made the directory first, it uses that directory and
     its base commit and removes its own copy (two writers make two different commits, and only
     one is kept). `commit()` answers that base without git while the fixture is as copied; after
     a change through `Fixture` it runs `add`, `commit` and `rev-parse` as today, so each of the
     5 fixtures of Why 1 that change their files first gets a base of its own with today's
     contents. The removal at `:533-534` goes through a `Fixture` method, so it counts as such a
     change.
2. The drop of a test directory in `surfd` and `surf-session` finds the case's processes among
   the descendants of the test process (`/proc/<pid>/task/<tid>/children`, followed down) and
   reads `environ` only of those. Every helper that starts a built program marks the directory,
   and its drop still reads every process's `environ`, as today: `agent()`
   (`crates/surfd/tests/common/mod.rs:378`, `crates/session/tests/common/mod.rs:274`),
   `Surfd::start` (`crates/surfd/tests/common/mod.rs:436`) and `Started::start`
   (`crates/session/tests/common/mod.rs:359`). When the children file cannot be read the drop
   reads every process too. Proofs, each in an existing test:
   - `surfd` `built::case_04_the_token_is_in_no_file_and_no_command_line` (`built.rs:121-149`,
     which runs a built `surf-agent`) keeps the pids of the `surfd` and the stand-in it reads
     and asserts after `drop(dir)` that neither is alive (`alive`, imported at `built.rs:18`);
   - `surfd` `in_process::case_01_open_answers_with_the_token_and_lists_the_session`
     (`in_process.rs:31-55`) leaks its daemon (`std::mem::forget`) after its assertions, drops
     the directory, and asserts that the session's listed pid is not alive;
   - `surf-session` `events::case_05_look_adds_no_entry` (`events.rs:422-432`) does the same with
     its in-process daemon, the pid read from `daemon.sessions()` (`pid`,
     `crates/surfd/src/daemon.rs:620`), through `surf_test_process::alive` by its path.
   The marks of `Surfd::start` and `Started::start` are held by no test: no case opens a session
   through a `surfd` that `Surfd::start` started, and `Started::start` starts `surf-session`
   itself.
3. The driver reads its tasks from `AGENT_TASKS_FILE` when it is set, else from
   `crates/agent-tasks/tasks.txt` (`taskfiles/agent-tasks-model.sh:79`; its head says so). It
   refuses `AGENT_TASKS_FILE` unless `AGENT_TASKS_MODEL_COMMAND` is set: exit 1 with one line
   naming both, before it removes `<out file>` (`:90`), so a run with the default model command
   never runs another task list and writes it as the lane's result (that file goes into release
   notes, `:47-48`). The five tests of Why 3 set it to `crates/agent-tasks/tests/one-task.txt`,
   the block of `article` from `tasks.txt`, and each asserts that the first line of the out file
   counts `1 tasks`. The other seven driver tests keep all four tasks.
   `adversary2::a_bound_of_zero_or_no_number_ends_the_lane_before_any_model_runs`
   (`adversary2.rs:279-299`) gains one run with `AGENT_TASKS_FILE` set and both
   `AGENT_TASKS_MODEL_COMMAND` and `AGENT_TASKS_MODEL` removed: exit other than 0, the stand-in
   not run, the out file of the earlier run left as it was, standard error naming
   `AGENT_TASKS_FILE`. With `AGENT_TASKS_MODEL` removed, a driver without the refusal stops at
   `:104` and never looks for a model.
4. The clocks of `surf-view` `http` become 250 ms: the write limit of `case_23` (`:843`) and the
   receive limit of `case_20` (`:776`). `case_23` keeps both of its lower bounds (`elapsed >=
   write`, `:862`, `:883`); the lower bound of `case_20` becomes 225 ms, 90 % of the limit as
   900 ms is of 1 s today (`:786`). `in_process::case_09` and `case_10` keep their 1 s (D280):
   surfd applies the answer timeout to the `open` exchange as well
   (`crates/surfd/src/daemon.rs:1253`), so a freshly started stand-in would have to answer
   within 250 ms in the pool, against `AGENTS.md:1357-1358` (a limit at least 10 times the time
   the test takes alone). `surfd_started_by_hand_writes_no_view_token_unless_asked` runs its two
   `surfd` at the same time, each in a directory of its own.
5. `case_14` sets `config.log.reader_queue` to 24 through `surfd_in_process`, and its `QUEUE`
   becomes 24 (3 `go`). It gains the assertion that `surfd::log::READER_QUEUE` is 1,024, the
   story's number, by its path (no `use` line).
6. The two tests of `surf-view-fixture` take `bin_dir()` from one new module,
   `crates/view-fixture/tests/common/mod.rs`, declared by `mod common;` in `tests/main.rs`: the
   programs from `SURF_BIN_DIR` when it is set, as `crates/session/tests/common/mod.rs:229-231`
   does, and today's cargo build only when it is not. The third copy that `impl/round2-view`
   adds (`crates/view-fixture/tests/acceptance.rs:21`) is replaced by `common::bin_dir()` in the
   merge of that wave: a change of the wave merge, not of this story's commit.
7. `case_17` of `surfd` `view` sets `entries_per_session` to 3 and sends the fewest `go` that pass
   it; its asserted numbers follow from those two values.
8. `other_argument_shapes_exit_2` and `more_argument_shapes_exit_2` assert each shape through
   `process::arguments` in process; each keeps one run of the binary per outcome it asserts
   (exit 2, nothing on standard output, no socket bound).
9. `surfd`'s `bin_dir()` makes its directory once per test run, not once per test process: each
   entry a hard link to its program, left alone when its inode is the program's. A missing or
   stale entry is made as a link under a partial name of the process (`<entry>.part-<pid>`) and
   renamed over the entry, as `copy` (`crates/surfd/tests/common/mod.rs:361-365`) does today, so
   a test process that starts a program while another replaces it finds the old entry or the
   new one, never none; a copy through the same partial name when a hard link fails. An existing
   test of `surfd` `built` gains the proof, in a directory of its own: an entry with another
   inode is replaced, an entry with the program's inode is left as it is.
10. `AGENTS.md` is not in this story's scope (D281). The unit hands over, in its report, the
    replacement text for `AGENTS.md` `:1546-1557` (line numbers of `96145a2`): the waits of
    statement 4 and how a test directory finds its processes. The coordinating session applies
    it at the merge.

Machine rules for every command this story names: builds and tests go through the pool (`cargo`
by its shim, any other command that compiles or tests with the `slot` prefix); a script that
measures, loops or waits starts without the prefix; a pool job over 10 minutes names its time
(`slot --timeout <t>`, at most 30 minutes); no test over 60 s and the step's budget is 120 s; no
Python, also not in a probe; no network and no model call in a test.

### Revisions of other stories this change needs

Made by the planning session in the change that accepts this story, not by the unit. Line numbers
are those of the store files on `1f53d35`.

- `story:test-step-holds-with-tls`, "Kept, each with its reason": `:357` (`surf-agent-tasks`
  lanes) says that five driver tests run one task by this story's statement 3, the lanes keep
  their claim; `:360` (`surf-view-fixture`) that the fixture's tests take the programs from
  `SURF_BIN_DIR` by statement 6, the pipe stays the claim; `:361` (`case_23`) that it waits a
  write limit of 250 ms by statement 4.
- `story:time-bounds-in-tests`: `:23` says that `case_20` has a lower bound of 225 ms for a
  receive limit of 250 ms, so a receive limit at 0.9 of its value still passes; `:32` says that
  `case_09` and `case_10` keep their 1 s here, so the bounds at `in_process.rs:179` and `:206`
  are as before.

## What it does not do

- No tasks seam in `lane::Config` for `tasks::a_call_that_never_answers_ends_at_its_bound` (a lane
  of two tasks): about 4 s summed, 0.8 s of wall time, does not pay for a new field of the lane.
- No shorter heartbeat in `http::case_32_an_idle_stream_receives_the_keep_alive_comment`: it has
  no lower bound, so a heartbeat not enforced still passes (`story:time-bounds-in-tests`).
- No shorter `session_ready_timeout` or `session_answer_timeout` in `in_process::case_09` and
  `case_10` (statement 4, D280).
- No clock the test controls in `crates/view` or `crates/fetch`, and no change of the upper
  bounds `crates/surfd/tests/in_process.rs:179` and `:206`: `story:time-bounds-in-tests`.
- No change of `kill_grace` in `in_process::case_24` and no change of a wait of 1 s that is an
  option's smallest value (`--idle-exit`, `--idle-timeout`, `--request-timeout`).
- No change of `crates/agent/src/client.rs` (round 2 holds it), and the host-wide scan stays for
  a case whose directory a helper marked.
- No file of story `test-step-scripts-and-binaries`: `Taskfile.yml`, `taskfiles/` but
  `agent-tasks-model.sh`, the manifests, `crates/surf/tests`, `AGENTS.md`.
- No change of the number of tests at once, of `.config/nextest.toml`, of the budget or of a
  timeout; no test removed, merged or added.

## Acceptance

1. Programs per run, counted with `strace -f -e trace=execve` outside the pool, before and after,
   quoted with the counts per program:
   - the driver with the stub model over the four tasks (75 in 52 processes before) and over
     `one-task.txt` after: the second at most 60 % of the first;
   - one `surf-html-conformance --root <fixture> --check --base <commit>`: 8 before (1 harness, 7
     git), at most 3 after;
   - one test of `check` whose fixture is not changed before its first commit: 4 git before; 0
     after when the cache directory of its digest exists, 4 in the test that makes it; one
     fixture of `all_passing`: 4 git before, 3 after;
   - each test of `surf-view-fixture` run with `SURF_BIN_DIR` set: no `cargo` after;
   - the `environ` files one drop opens (`strace -e trace=openat`) in `surfd`
     `in_process::case_01` and in `surf-session` `events::case_05`: about 1,300 each before,
     under 20 each after.
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
   - the fixture cache keyed without the fixture's bytes (one fixed name), the module run once,
     then the section name `coercing` renamed to `coercion` in every line of
     `crates/html-conformance/tests/check.rs` that holds it (16 lines at `96145a2`; the constants
     `SECTIONS` `:90-92` and `TOKENIZER_EXPECTED` `:112` among them):
     `a_section_map_that_differs_from_the_note_fails` fails at its `edit` (`:348`), the copied
     fixture holding `coercing`; with the real key the module passes after the rename;
   - `agent()` of `surfd` not marking its directory: `built::case_04` fails, its `surfd` alive
     after the drop;
   - the walk of children taken out of the drop: `in_process::case_01` and `events::case_05`
     fail, the leaked daemon's session alive;
   - the driver ignoring `AGENT_TASKS_FILE`: the five tests fail on `1 tasks`; the refusal of
     statement 3 taken out: `a_bound_of_zero_or_no_number_ends_the_lane_before_any_model_runs`
     fails on its standard error;
   - the write limit and the receive limit applied at once (the server closes the connection
     without waiting): `case_23` fails at `elapsed >= write` and `case_20` at its bound of
     225 ms; `--idle-exit` not enforced: `surfd_started_by_hand_writes_no_view_token_unless_asked`
     fails;
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
   before and after and its line `test step: N s`. A run that exits non-zero fails this
   statement. The time is reported, not bounded here; the bound is acceptance 1 of
   `story:test-step-holds-with-tls`.
7. The round 2 check of this story is run again on the story's commit and quoted: no new file of
   Scope in either branch's `git diff --stat`, and no hunk of either in a changed function.
8. `git diff --stat 96145a2` lists only the files of Scope; `AGENTS.md` is not among them.
9. The gate `slot --timeout 25m task check-runner` on the story's commit, run by the coordinating
   session, passes; its line `test step: N s of its budget of 120 s` is quoted.
10. Statement 9, written files: `stat -c '%i %h %Z %n'` of each entry of the bin directory
    (`CARGO_TARGET_TMPDIR/surfd-bin-<test binary>`) and `ls -a` of it are equal before and after
    a second run of `cargo nextest run -p surfd -E
    'test(=built::case_05_an_identity_never_opened_is_unknown)'`: the second test process writes
    0 files there. Before the change each run writes 3 copies (new inodes), quoted.

## Scope

`crates/html-conformance/src/check.rs`, `crates/html-conformance/tests/check.rs`;
`crates/surfd/tests/common/mod.rs`, `crates/surfd/tests/in_process.rs`,
`crates/surfd/tests/built.rs`, `crates/surfd/tests/view.rs`;
`crates/session/tests/common/mod.rs`, `crates/session/tests/events.rs`,
`crates/session/tests/built.rs`; `taskfiles/agent-tasks-model.sh`,
`crates/agent-tasks/tests/model_driver.rs`, `crates/agent-tasks/tests/adversary2.rs`,
`crates/agent-tasks/tests/adversary.rs`, `crates/agent-tasks/tests/one-task.txt`;
`crates/view/tests/http.rs`; `crates/view-fixture/tests/main.rs`,
`crates/view-fixture/tests/common/mod.rs`, `crates/view-fixture/tests/fixture.rs`,
`crates/view-fixture/tests/adversary_commands.rs`.

## As built

The change is `d846975` on `fix/test-harness-processes`, measured on 2026-11-23.

- One `--check --base` run of html-conformance: 8 programs before, 3 after.
- One drop of a test directory opened 1281 `environ` files before (`in_process::case_01`) and
  opens 1 after; `events::case_05` 1287 and 1.
- view-fixture with `SURF_BIN_DIR` set: `fixture` ran 19 programs, 1 of them cargo, and runs 9,
  none of them cargo.
- 893 test names before and after. The 37 changed tests are each under 2 s.
- Three runs of `task test` in a row at a load of 86 to 115: 33.1 s, 29.8 s and 40.3 s of the
  budget of 120 s.

Short of the story, decided (D296), not measured again:

- Acceptance 2, module `check` of html-conformance, third pair: 12.83 s after against 22.55 s
  before, 57 % where the bound is 50 % (38 % and 47 % in the other two pairs). The test
  `the_tag_counts_of_the_groups_sum_to_the_tag_lines` took 4.10 s of it; it runs the whole
  conformance suite in the test process and is not changed here.
- Acceptance 1, the driver with the stub model: 56 programs in 62 processes for 4 tasks before,
  counted as successful `execve` calls, where the story says 75 in 52. With `one-task.txt`: 32
  programs, 53 % of the run of 4 tasks after, inside the bound of 60 %.

The text for `AGENTS.md` (D281) is applied on the merged branch.
