---
format: aep.planning-md/3
id: story:tests-without-wall-time-bounds
kind: story
status: proposed
title: Replace the wall-time bounds of five tests by counts and keep the rule in the gate
summary: Five tests of main bound a time by a clock (four assert an elapsed time, one waits 10 s for a parse); each gets the count that proves its cost (one new in webidl, one steps ceiling, two tightened), and a source check with a ledger of the other statements it finds refuses new ones.
relations:
- decomposes: epic:scripts-in-a-page
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/html/tests/script_hook.rs
- confidence: cited
  path: crates/script/tests/adversary_4.rs
- confidence: cited
  path: crates/script/tests/adversary_5.rs
- confidence: cited
  path: crates/webidl/src/lib.rs
- confidence: cited
  path: crates/webidl/src/model.rs
- confidence: cited
  path: crates/webidl/tests/depth.rs
- confidence: cited
  path: taskfiles/test-clocks-check.sh
- confidence: cited
  path: taskfiles/test-clocks-ledger.txt
- confidence: cited
  path: taskfiles/test-clocks.yml
revision: 18
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T17:30:08Z", actor: "human:operator", revision: 16}
---
## Context

The repository's rule on a test that reads a clock (`AGENTS.md:1965-1967 @ a14e3b0`): "A test
bounds a time only by a wait for the event with a limit at least 10 times the time it takes
alone, and proves a cost by a count (such as `surf_page::attribute_visits()`), not by a wall
time." The count it names is a thread-local counter behind a `#[doc(hidden)] pub fn`
(`crates/page/src/lib.rs:174-190 @ a14e3b0`).

Five tests of `main` at `a14e3b0` break it: four assert an upper bound on their own wall time,
and one waits at most 10 s for a parse. Two of them went red on a loaded machine while every
count they hold was right:

- `crates/script/tests/adversary_5.rs:373`,
  `the_code_getter_in_a_loop_copies_at_most_a_legacy_name_and_ends_at_the_step_limit`, bound
  20 s (`:401`). Alone, three runs: 2.839 s, 2.618 s, 2.624 s, so the bound is 7.0 to 7.6 times
  the time alone. Red at 20.611 s (load average 117 to 134) and at 29.206 s (load average 80 to
  110). It already asserts the counts that carry its claim (the rounds of the loop).
- `crates/webidl/tests/depth.rs:55`, `a_model_of_16000_interfaces_is_built_within_2_s`, bound
  2 s. Alone, three runs: 0.198 s, 0.184 s, 0.186 s, so the bound is 10.1 to 10.9 times the time
  alone. Red at 2.380 s (load average 5 rising to 77) and at 4.027 s (load average 117 to 134).
  Its name states a wall time as its claim.
- `crates/html/tests/script_hook.rs:1740`,
  `a_hook_that_keeps_every_element_open_ends_at_the_step_limit`, waits at most 10 s
  (`recv_timeout`, `:1750`). Measured alone on a build node, three runs: 1.398 s, 1.598 s,
  1.801 s, so the limit is 5.6 to 7.2 times the time alone, under the ten of the rule.
- `crates/script/tests/adversary_5.rs:241` (bound 20 s) and
  `crates/script/tests/adversary_4.rs:654` (bound 30 s): the same form as the first. Measured
  alone on a build node, three runs: 1.208 s to 1.927 s for the first, so its bound is 10.4 to
  16.6 times the time alone; 0.05 s for the second.

The times alone of the first two were measured on the build node on 2026-11-25, the red runs on
the workstation the same day, by the unit of `story:scripts-dom-reading` (its fix round 5),
which changed none of these tests because they are on `main`.

A red test with right counts stops a gate and a merge for a reason that is no defect of the
code. The folder's rule on budgets says a raised bound is not a fix.

The five are not alone. A scratch scan of the tests at `a14e3b0`, not kept and not binding,
found 43 more statements in 27 files under `tests/` directories that assert an upper bound on
an elapsed time. That is the work of `story:time-bounds-in-tests`; this story fixes the five
and keeps the rest from growing. The check of decision 8 makes its own scan; its figures, not
these, are the ones the story reports. Clock use at `a14e3b0`:
`git grep -n -E 'Instant::now|\.elapsed\(\)' a14e3b0 -- crates/script/tests crates/webidl/tests`
prints 27 lines; with `duration_since` and `SystemTime` added, 30; the four spellings over
every `tests/` directory give 251 lines in 73 of the 291 files, and `duration_since` and
`SystemTime` alone 38 of them.

## Decisions

Each of the five tests has a sentence that says what its count proves and one that says what it
does not see.

1. `adversary_5.rs:373` (the code getter loop). Cost the bound protects: each of the three
   loops ends at `steps_per_entry = 1_000_000` (`:375`), so its wall time is the cost of one
   round times the rounds. The legacy-code getter `dom_exception_code`
   (`crates/script/src/bindings.rs:1615-1625 @ a14e3b0`) answers 0 for a name longer than 26
   code units without a copy (`:1619-1621`); a copy goes through `Copies::copy`
   (`:553-563`), which charges `units / 64` steps (`:561`): the name of 2^14 code units of
   U+0800 is 48 KiB, and 48 KiB / 64 bytes = 768 steps per read (doc comment at `:368-371`).
   Count that proves it, existing: `G.rounds`, read by the script at `:393`
   (`G.rounds[2] * 2 > G.rounds[1] && G.rounds[0] > 1000`), and the three ends
   `Limit(StepsPerScript)` (`:398`). The time assertion (`:400-401`) goes; the time is printed.
   - Proves: a copy that is charged lowers `rounds[2]` against `rounds[1]`. Red with the defect
     put back by hand: the guard `:1619-1621` of `dom_exception_code` removed, so the long name
     is copied and charged 768 steps per read; `rounds[2]` falls under half of `rounds[1]` and
     the script's `check` fails.
   - Does not see: a copy that is not charged. Written without the `charge_steps` of `:561`,
     it keeps the rounds and no count moves; only the time bound saw it.
2. `adversary_5.rs:231`
   (`strings_only_exceptions_hold_survive_collections_inside_and_between_scripts`). Cost the
   bound protects: the work of the three scripts (`MAKE` makes 64 exceptions, `READ` reads them
   twice) and of the four collections `collect` makes (two per call, `:171-177`). No count
   proves it today: `ran` asserts `ScriptEnd::Ran` only (`:168`), so a script that charged ten
   times the steps still passes under the 200,000,000 of `steps_per_entry` (`:12`). The story
   adds the count: `runner.steps()` (`crates/script/src/runner.rs:821-823 @ a14e3b0`, read the
   same way at `adversary_4.rs:466`) before and after each of the three scripts, asserted at a
   ceiling per script that is 1.5 times the steps of the first green run (the steps are
   deterministic, so the margin is for a change of the engine, not for a machine). The
   measured steps and the ceilings are printed by the test and named in the commit message.
   The time assertion (`:240-241`) goes; the time is printed.
   - Proves: the steps the three scripts charge. Red with the defect put back by hand: the
     loop bound `k < 40` of `texts` in `MAKE` (`:184`) written as `k < 4000`; each script
     charges far more steps and the ceiling fails.
   - Does not see: the four `collect` calls. They charge no script step (`runner.steps()` sums
     the scripts only), so their cost is dropped on purpose; it is the part of the old bound
     that no count here stands for.
3. `adversary_4.rs:641` (`a_dom_exception_holds_no_uncharged_copy`). Cost the bound protects:
   64 copies of a message of 2^18 bytes (`:645-646`, 64 x 256 KiB = 16 MiB) made by the
   constructor. Count that proves it, existing: the process grows by `mib` MiB or the script
   ends `Limit(Memory)` (`:656-659`). The story tightens it: the first arm stays, the second
   becomes `end == ScriptEnd::Ran && mib < 4`, so a script that ended for another reason does
   not pass on a small `mib`. The time assertion (`:654`) goes; the time is printed.
   - On `main`, `ExceptionData` (`crates/script/src/bindings.rs:315-318`) holds two
     `StringRef`s, handles into the engine's heap, and copies nothing; `new_exception`
     (`:656-668`) stores them. The doc comment at `adversary_4.rs:632-637`, which says the
     constructor copies both Strings out of the engine (`prelude.rs:1347-1357`), is out of date
     in the same way, and the story writes it again.
   - Proves: the process does not grow by the 16 MiB, or the script is stopped by the memory
     limit. Red with the defect put back by hand: `ExceptionData` made to own its strings
     (`name: String, message: String`, filled at `:666` by `to_utf16` and
     `String::from_utf16_lossy` with no charge); the process grows by 16 MiB and the script is
     not stopped.
   - Does not see: the time of the 64 constructions, nor a copy that is slow and charged within
     the budget.
4. `depth.rs:55` (16,000 interfaces). Cost the bound protects: `Model::new` finds each name
   through one map (`crates/webidl/src/model.rs:391-421`, `index` and `definition_of`), not by a
   scan of every definition per statement (the doc comment at `depth.rs:50-53`). Count that
   proves it, none exists: the story adds `surf_webidl::definition_visits()`, a thread-local
   counter behind `#[doc(hidden)] pub fn` in `crates/webidl/src/model.rs`, in the form of
   `attribute_visits`, re-exported in the `pub use model::{...}` of `crates/webidl/src/lib.rs:102`.
   Every lookup of a definition by its index goes through one private accessor,
   `definition_at(&self, index)`, which counts 1 and returns `&self.definitions[index]`:
   `definition_of` (`:417-421`), `all_named` (`:674-680`), `index` (`:401`),
   `check_inheritance` (`:503`) and the cycle report (`:570`, `:576`) call it. The loops that
   walk every definition once (`index`, `check_partials`, `check_includes`, `check_members`)
   are scans by design and are not lookups. The test builds a model of 8,000 and one of 16,000
   interfaces (the same text, `i` up to the count), reads the counter as the difference around
   each `Model::new` (the `read` of the text is outside it), and asserts that the visits of
   the second are at most 3 times those of the first. The text has `n` interfaces, `n`
   statements `A includes M;` and one mixin, and no `inherits`. The visits it makes:
   - `check_includes` through `definition_of`: 2 per statement (the mixin, the interface), 2n;
   - `check_members` through `all_named`: per interface name 2 definitions of that name (the
     interface and the `includes`) and 1 of the mixin, 3n;
   - so 5n: 40,000 at 8,000 interfaces and 80,000 at 16,000. Arithmetic: a lookup through a
     map visits what it returns, so the visits grow with the statements, 16,000 / 8,000 = 2;
     a scan of all definitions per statement grows with the square, 2 x 2 = 4; the bound 3 lies
     between. The time assertion (`:71`) goes; the time is printed.
   - Proves: the two counted lookups, `check_includes` through `definition_of` and
     `check_members` through `all_named`, do not scan. Red with the defect put back by hand:
     `definition_of` written as a loop over `0..self.definitions.len()` that fetches each
     definition through `definition_at` and compares its name, so each compared definition
     counts; the ratio is near 4.
   - Does not see: `read` (the old test timed it too, `depth.rs:61-63`), `index`'s map,
     `parent_of` (`:482-491`) and `unresolved` (`:750-782`), which read `self.defined` directly,
     and a scan that does not pass the counter (`self.definitions.iter().find(..)` keeps the
     ratio at 2). The test catches a scan only if it runs through `definition_at`.
5. `script_hook.rs:1740` (`a_hook_that_keeps_every_element_open_ends_at_the_step_limit`). Cost
   the limit protects: 40,000 rounds of `<span><script>x</script>` with `KeepOpen` (`:1700`)
   would read about 8 x 10^8 entries of the stack of open elements (run `k` reads about `k`
   entries, so `n` runs read `n` x `n` / 2: 40,000 x 40,000 / 2 = 8 x 10^8); the step limit
   `max_steps = 1 << 27` (`crates/html/src/limits.rs:108 @ a14e3b0`, 134,217,728) ends the
   parse after about 16,000 runs (16,000 x 16,000 / 2 = 1.28 x 10^8, near 2^27). The code stops
   a parse by `Limit::Steps` only when `steps > max_steps` (`tree_builder.rs:687`, `:1035-1041`,
   `limits.rs:128`), and `limits.rs:75` says it ends fewer than `4 * max_nodes` steps past
   `max_steps`; `max_nodes` is `1024 * 1024` (`limits.rs:104`), so 4 x 1,048,576 = 4,194,304.
   The test asserts the end `Some(Limit::Steps)` (`:1753`, as today) and the documented
   window of the steps at that end:
   `Limits::default().max_steps < steps < Limits::default().max_steps + 4 *
   Limits::default().max_nodes`, that is more than 134,217,728 and less than 138,412,032 steps.
   It prints the steps and the time as today. The wait goes: the thread, the channel and
   `recv_timeout(10 s)` (`:1742-1751`) are replaced by the parse run in the test's own thread,
   with no spawn and no channel. The bound of the test is the runner's 60 s
   (`.config/nextest.toml:5`, `slow-timeout` with `terminate-after`). The doc comment
   (`:1735-1739`, "within 10 s ... 1.4 s measured") is written again without the 10 s.
   - Proves: the parse ends by the step limit and charged between `max_steps` and the window's
     end: every entry read again after a hook run is a step. Red with the defect put back by
     hand: the loop that reads the entries again after a hook run charges no step
     (`each_entry_read_again_after_a_hook_run_is_a_step`, `:1722`, asserts that charge at once:
     `four - two >= 200 * 200`). The parse then probably ends with `stopped_by() == None`, 80,000
     nodes being under `max_nodes`, and `assert_eq!(stopped, Some(Limit::Steps))` fails; if it
     does not end, the 60 s kill does, whichever comes first. This red is shown once as a
     demonstration, in one pool job, and not run again unchanged.
   - Upper side: no one-line defect turns it red. A stop checked too late overshoots by at most
     one token's scan of the stack, which holds at most 40,000 open elements here, and the
     window is 4,194,304 steps, 104 times that (4,194,304 / 40,000 = 104.9); a defect that made
     the overshoot reach it would add a hundred scans per token. The upper side keeps the
     documented contract of `limits.rs:75` under the test and is not claimed to be red by a
     defect.
   - Does not see: the cost of one step (a slower entry read that is still charged one step),
     which the old 10 s bound saw in part.
6. No test of the five needs a clock. None waits for an event whose time is not its own
   work: each measures work it did itself, on the test's own thread. A wait with a limit of at
   least 10 times the time alone stays allowed by the rule and is not touched; an elapsed time
   compared with a constant after the work is not that form, and a limit of 5.6 to 7.2 times
   the time alone is not either.
7. The name of the `depth` test: `the_definitions_a_model_visits_double_when_its_interfaces_double`.
   The module's other names (`depth.rs:40`, `:46`, `:75`: `types_32_deep_are_read`,
   `types_33_deep_are_refused`, `types_5000_deep_are_refused_on_the_test_threads_stack`) state
   the case and its result; the new one states the count and its result. `AGENTS.md:1054`
   ("`depth` also holds a model of 16,000 interfaces under 2 s") says what the test holds and is
   written again. The `script_hook` test keeps its name: it states the end, not a time.
8. The source check. Written in POSIX sh with POSIX awk only, in `taskfiles/`, because the
   repository has no xtask crate
   (`git ls-tree -r --name-only a14e3b0 | grep -c xtask` prints 0) and its other checks are sh
   (`taskfiles/doc-tests-check.sh`, which the new files copy in form: a script, a task, and a
   `-test` task with a `SCRIPT` variable). Files: `taskfiles/test-clocks-check.sh`,
   `taskfiles/test-clocks.yml` (tasks `test-clocks-check` and `test-clocks-check-test`,
   included like `doc-tests.yml`), `taskfiles/test-clocks-ledger.txt`. `Taskfile.yml` lists
   `test-clocks-check` in `check` after `doc-tests-check` and `test-clocks-check-test` after
   `doc-tests-check-test`. The forge runs `task check` (`.github/workflows/check.yml:159-161`),
   so the check runs there, on `ubuntu-latest`, whose `awk` is not gawk.
   - Usage: `sh taskfiles/test-clocks-check.sh [--count] [--ledger <file>] [<root>]`; the awk
     is the one named by the variable `AWK` (default `awk`). `--ledger /dev/null` is an empty
     ledger. `--count` prints the ledger's lines (`<path> <count>`) for a tree.
   - The header of the script says that it uses POSIX awk only and forbids by name:
     `nextfile` (POSIX since 2012 but not in every mawk), `length(array)`, `IGNORECASE`,
     `gensub`, `asort`, `asorti`, `strftime`, `\<` and `\>`, and a regular expression interval
     (`{n,m}`).
   - Reading: every `*.rs` under any `tests/` directory below `crates/`, `generated/` and
     `conformance/`, comments and string literals blanked first (a clock inside a string or a
     `//` line is no clock).
   - Refused: an upper bound on an elapsed value in a statement.
     - An `assert!`, `assert_eq!`, `assert_ne!` or `debug_assert!` whose condition compares an
       elapsed value with anything by `<` or `<=` (`took < Duration::from_secs(2)`), or puts it
       on the large side of `>` or `>=` (`LIMIT > took`). An `assert!` on an elapsed value
       inside a poll loop is refused like any other.
     - An elapsed value is `.elapsed()`, `.duration_since(..)`,
       `.saturating_duration_since(..)`, any method chain on one (`.as_secs_f64()`,
       `.as_millis()`), and a name bound by `let` from one, directly, through another such
       name (three rounds), or in a tuple pattern whose element name holds `took`, `elapsed`,
       `seconds`, `secs`, `waited`, `duration` or `time`
       (`let (end, grown, seconds) = ...`, `adversary_4.rs:645`).
     - An `if` or `while` whose condition compares such a name upward with a `Duration`, a
       number or an upper-case constant, and whose block fails the test. The scanner reads the
       block from its opening brace to the matching closing brace (nested braces counted, on
       the text with comments and strings blanked) and no further; the block fails the test
       when it holds `panic!`, `unreachable!`, `assert` (any `assert!` form) or the word
       `fail`. A branch that leaves a loop (`break`, `continue`, a `return` of a value) passes.
   - Allowed: a lower bound (`assert!(took >= ..)`); a print; a wait with a limit, written as a
     loop that leaves on the deadline (`started.elapsed()` or `Instant::now()` against it, by
     `break` or `return`) and asserts the event after the loop, or a `recv_timeout`. The
     script's header names that spelling as the one to use.
   - Not seen (stated in the script's header): a bound written through a helper function, a
     macro, or a comparison split over a `let` of a `bool` (`let ok = took < LIMIT;
     assert!(ok)`); the negated spelling `assert!(!(took >= LIMIT))`; a bound recorded in a list
     that is asserted after the loop (`if took >= LIMIT { slow.push(..) }`, then
     `assert!(slow.is_empty())`); and whether the limit of a wait is at least 10 times the time
     alone, which no source reading can know (the `script_hook` test of decision 5 is such a
     wait, so it is in no ledger line).
   - Ledger: `taskfiles/test-clocks-ledger.txt` lists the files of the committed tree that
     hold statements the scan refuses and are not fixed by this story, one `<path> <count>`
     per line, sorted, no comment (the form of `taskfiles/lock-allow.txt`). A file with more
     statements than its line (or with none and no line) fails, naming file and line of each
     statement; a file with fewer fails too and names the count to write, so the ledger only
     shrinks. `story:time-bounds-in-tests` empties it. The implementor writes the ledger from
     `--count` on the changed tree and reports both totals: the lines the scan prints on
     `a14e3b0` with an empty ledger and the sum of the committed ledger.
   - The cases of the check's own test, by name (no count is asserted beside the list). Refused:
     `elapsed_name_against_duration`, `elapsed_direct_in_assert`, `limit_above_name`,
     `float_seconds`, `tuple_with_seconds`, `branch_that_panics`, `assert_inside_poll_loop`.
     Passed: `wait_loop_with_break`, `branch_that_breaks_on_a_name`,
     `branch_that_returns_a_value`, `lower_bound`, `print_only`, `deadline_loop_on_now`,
     `clock_in_comment_and_string`, `negated_spelling_not_seen`. Ledger: `count_equal_passes`,
     `count_above_refused`, `count_below_refused`. The test task runs every case under each of
     `awk`, `mawk`, `gawk` and `busybox awk` that exists on the machine and prints which ran.
   - Refused example: `#[test] fn a() { let started = Instant::now(); work(); let took =
     started.elapsed(); assert!(took < Duration::from_secs(2), "{took:?}"); }` printing
     `<path>:<line>` of the `assert!` and exit 1.
   - Passing example: `#[test] fn a() { let started = Instant::now(); while !done() { if
     started.elapsed() > LIMIT { break; } } assert!(done()); }`, exit 0 and the line
     `test-clocks-check: no elapsed-time bound outside taskfiles/test-clocks-ledger.txt`. The
     branch leaves the loop and holds none of `panic!`, `unreachable!`, `assert`, `fail`.

## Change

- `crates/script/tests/adversary_5.rs`: the assertions at `:241` and `:401` go and the times
  are printed (`println!`); the test at `:231` gains the steps ceilings of decision 2.
- `crates/script/tests/adversary_4.rs`: the assertion at `:654` goes, the time is printed, the
  second arm at `:657` becomes `end == ScriptEnd::Ran && mib < 4`, and the doc comment at
  `:632-637` is written again (decision 3).
- `crates/webidl/src/model.rs`: the counter `definition_visits`, the accessor `definition_at`
  and the calls that go through it; `crates/webidl/src/lib.rs`: its name in the
  `pub use model::{...}` list.
- `crates/webidl/tests/depth.rs`: the test at `:55` is replaced by the one of decision 7.
- `crates/html/tests/script_hook.rs`: the test at `:1740` runs the parse in the test's own
  thread, with no spawn, no channel and no timeout, asserts the window of decision 5, and its
  doc comment loses the 10 s.
- `taskfiles/test-clocks-check.sh`, `taskfiles/test-clocks.yml`,
  `taskfiles/test-clocks-ledger.txt`: new, decision 8. `Taskfile.yml`: the include and two steps.
- `AGENTS.md`: the entry of `task check` lists the two steps; the entry of `task test` (the
  sentence at `:1965-1967`) names the check; the entry of `crates/webidl` (`:1054`) says what
  `depth` holds now. The test counts of the entries stay (no test is added or removed: `depth`
  keeps 4, `crates/script` and `crates/html` keep their counts).

A file under `src/` changes (`crates/webidl/src/model.rs` and `lib.rs`), which the draft said
no file would.

## Acceptance

Each item is one outcome, shown by a command and what it prints or by a test and what it
asserts.

1. On a copy of `a14e3b0`'s tree with the new script,
   `sh taskfiles/test-clocks-check.sh --ledger /dev/null .` exits 1 and prints one
   `<path>:<line>` per statement it refuses, among them these four:
   `crates/script/tests/adversary_4.rs:654`, `crates/script/tests/adversary_5.rs:241`,
   `crates/script/tests/adversary_5.rs:401` and `crates/webidl/tests/depth.rs:71`. Turns red
   by: the tracking of names bound in a tuple pattern removed (`adversary_4.rs:654` is not
   printed).
2. The sum of the counts of the committed ledger equals the number of `<path>:<line>` lines of
   item 1 less those four: `awk '{ s += $2 } END { print s }' taskfiles/test-clocks-ledger.txt`
   and that number are the same, and the report gives both totals and the list of the lines.
   Turns red by: a line of the ledger raised by 1 (the check names the count to write).
3. On the changed tree `sh taskfiles/test-clocks-check.sh .` prints
   `test-clocks-check: no elapsed-time bound outside taskfiles/test-clocks-ledger.txt` and
   exits 0. Turns red by: `assert!(took < Duration::from_secs(2))` written back into
   `depth.rs`; the script names file and line.
4. `git grep -n -E 'seconds < |total < Duration|recv_timeout' --
   crates/script/tests/adversary_4.rs crates/script/tests/adversary_5.rs
   crates/webidl/tests/depth.rs crates/html/tests/script_hook.rs` prints nothing. Turns red by:
   the `recv_timeout` of `script_hook.rs:1750` written back.
5. `task test-clocks-check-test` decides each case of decision 8 as written: it prints one line
   `case <name>: ok` per case and ends with `test-clocks-check-test: all cases decided as
   written`, exit 0. Turns red by: `SCRIPT=<copy>` with the upper-bound pattern weakened to
   refuse only `<`; it prints each case decided otherwise and fails.
6. The same task prints, for each of `awk`, `mawk`, `gawk` and `busybox awk`, whether it ran
   the cases (`ran under <name>` or `not on this machine`); every one that ran decides all
   cases as written.
7. The header of `taskfiles/test-clocks-check.sh` names the forbidden features of decision 8:
   `git grep -c -E 'nextfile|IGNORECASE|gensub|asort|strftime' -- taskfiles/test-clocks-check.sh`
   prints a number above 0 for the header lines and the script itself uses none of them
   outside the header (`sed '/^#/d'` of the file, piped to the same `grep -c`, prints 0).
8. The forge's run of `task check` on the branch shows the step `test-clocks-check` and
   `test-clocks-check-test` green under the forge's `awk` (the report names the run).
9. `adversary_5.rs:373` asserts `G.rounds` (`rounds[2] * 2 > rounds[1]`); red with the guard of
   `dom_exception_code` (`bindings.rs:1619-1621`) removed.
10. `adversary_5.rs:231` asserts the steps of its three scripts at the ceilings; red with
    `k < 4000`.
11. `adversary_4.rs:641` asserts `Limit(Memory)` or `Ran` with `mib < 4`; red with
    `ExceptionData` owning uncharged copies of its strings.
12. `depth.rs` asserts that the `definition_visits` of `Model::new` over 16,000 interfaces is
    at most 3 times that over 8,000; red with the counted scan in `definition_of`.
13. `script_hook.rs:1740` asserts `Some(Limit::Steps)` and the window of decision 5; red with
    the entries read again charging no step, shown once in one pool job.
14. Each red of items 9 to 13 is shown once before the change is kept, and the report names the
    output of each.
15. The test step of `surf-script`, `surf-webidl` and `surf-html`, run alone
    (`slot cargo nextest run -p surf-script -p surf-webidl -p surf-html --locked`) and again
    while another build runs in the pool: the same tests pass in both. The report records the
    load average read beside each run and claims no particular load.
16. No test over 60 s and the test step within 120 s; a test over 5 s is named with its time
    (the three script tests, the new `depth` test and the `script_hook` test by their printed
    times).
17. No pool job over 10 minutes unless it names its time with `slot --timeout` (at most 30
    minutes), and none started again unchanged after `timed_out`.
18. Every command that compiles or tests carries `slot` (`cargo` and `go` through the shim).
19. No Python in anything that stays; the check is POSIX sh and POSIX awk.
20. A script of the story that measures or waits (the two loaded runs of item 15, a timing of
    the check) starts without `slot`; the run time of `task test-clocks-check` is printed in
    the report, and a step over 5 s is named.
21. No test of the story makes a model call or a network request, and a limit on a wait in a
    test the story writes is at least 10 times the event alone.

## Out of scope

- The statements the scan finds outside this story (the ledger), and the 12 assertions in 7
  files that `story:time-bounds-in-tests` lists: that story. This story does not edit those
  files.
- Raising a bound, adding a clock to any test, changing `RunnerLimits.wall_clock`
  (`adversary_5.rs:22`, 40 s) or the runner's steps.
- The lower bounds and waits with a limit that other tests hold, among them the waits whose
  limit is not measured against the time alone.
- Anything outside `tests/` directories: the check reads only them.

## Placement

Read with `git grep -l -F <path> -- .engineering/planning/story` for each scope path.

- `crates/script/tests/adversary_5.rs` is also named by `story:scripts-utf16-length` (draft) and
  `story:swell-0-9-0` (active); both name the doc comments near `:226`, not the lines of the
  two tests or of the assertions. No edge.
- `crates/webidl/src/model.rs` is also named by `story:scripts-dom-reading` (active), which
  cites `model.rs:88` (its line 345); this story edits `index`, `definition_of`, `all_named`
  and `check_inheritance` (`:391-421`, `:503`, `:674-680`). No edge.
- `crates/html/tests/script_hook.rs` is also named by `story:scripts-dom-attributes`
  (proposed), which adds tests of reads in the module where the write tests are (`:1157-1274`);
  this story edits `:1735-1753`. No edge. `story:html-script-hook` (implemented) is history.
- `Taskfile.yml`: `story:agent-cookie-public-suffixes` (active) adds an include and a step to
  `check`, and this story adds an include and two steps, in the same two lists. No edge: the
  story that merges second takes the other's lines in its merge of `main`.
  `story:conformance-lock-and-lints` (draft) edits the lines of `lock-check`, five lines above
  `doc-tests-check` in `check`; `story:test-step-holds-with-tls` (active) edits the lines of
  the test steps. No edge.
- `AGENTS.md` is in the scope of eleven other stories not yet implemented:
  `story:agent-tasks-left-process-wait`, `story:conformance-lock-and-lints`,
  `story:scripts-dom-attributes`, `story:scripts-dom-mutation`, `story:scripts-dom-reading`,
  `story:scripts-dom-selectors`, `story:scripts-parse-run`, `story:session-argument-checks`,
  `story:swell-0-9-0`, `story:test-step-holds-with-tls`, `story:tls-tests-as-tables`; none
  edits the lines this story edits (`:1054`, `:1965-1967` and the `check` paragraph).
- `story:time-bounds-in-tests` (draft) takes over the ledger, `taskfiles/test-clocks-ledger.txt`,
  and needs the scanner's output (the lines the scan prints against its list of 12 assertions
  in 7 files). Its text is not changed here. It gets a `depends_on` edge to this story, made
  outside this story's text.
- A story that removes a bound from a file the ledger lists lowers that file's ledger line in
  its own change, since the check fails on a file with fewer statements than its line.
  `story:agent-tasks-left-process-wait` changes `bounded.rs` (`running` and `child_left`,
  `:41-64 @ a14e3b0`), but the one ledger statement of that file, the assertion at
  `bounded.rs:97` (`took < Duration::from_secs(4)`), is outside those lines and stays. No
  ledger line changes, and no edge.
- `crates/webidl/src/lib.rs`, `crates/script/tests/adversary_4.rs`, `crates/webidl/tests/depth.rs`
  and the new `taskfiles/` files are named by no other story.
