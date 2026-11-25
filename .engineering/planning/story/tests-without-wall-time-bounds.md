---
format: aep.planning-md/3
id: story:tests-without-wall-time-bounds
kind: story
status: proposed
title: Replace the wall-time bounds of five tests by counts and keep the rule in the gate
summary: Five tests of main bound a time by a clock (four assert an elapsed time, one waits 10 s for a parse); each gets the count that proves its cost (one new in webidl, one steps ceiling, two tightened), and a source check with a ledger of the other 43 refuses new ones.
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
revision: 16
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

The five are not alone. Read at `a14e3b0`, 43 more statements in 27 files under `tests/`
directories assert an upper bound on an elapsed time (the scan of decision 8). That is the work
of `story:time-bounds-in-tests`; this story fixes the five and keeps the rest from growing.

## Decisions

1. `adversary_5.rs:373` (the code getter loop). Cost the bound protects: each of the three
   loops ends at `steps_per_entry = 1_000_000` (`:375`), so its wall time is the cost of one
   round times the rounds; a round that does work no step pays for (a copy of the 48 KiB name,
   which the doc comment at `:368-371` puts at 768 steps per read: 2^14 code units of U+0800
   are 48 KiB, 48 KiB / 64 bytes = 768) makes the loop slow without ending it earlier. Count
   that proves it, existing: `G.rounds`, read by the script at `:393`
   (`G.rounds[2] * 2 > G.rounds[1] && G.rounds[0] > 1000`), and the three ends
   `Limit(StepsPerScript)` (`:398`). The time assertion (`:400-401`) goes; the time is printed.
   Red with the defect put back by hand: the `code` getter copies the name through
   `Copies::copy` as the comment at `:368-371` describes; `rounds[2]` falls to at most half of
   `rounds[1]` and the script's `check` fails.
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
   The time assertion (`:240-241`) goes; the time is printed. Red with the defect put back by
   hand: the loop bound `k < 40` of `texts` in `MAKE` (`:184`) written as `k < 4000`; each
   script charges far more steps and the ceiling fails.
3. `adversary_4.rs:641` (`a_dom_exception_holds_no_uncharged_copy`). Cost the bound protects:
   64 copies of a message of 2^18 bytes (`:645-646`, 64 x 256 KiB = 16 MiB) made by the
   constructor. Count that proves it, existing: the process grows by `mib` MiB or the script
   ends `Limit(Memory)` (`:656-659`). The story tightens it: the first arm stays, the second
   becomes `end == ScriptEnd::Ran && mib < 4`, so a script that ended for another reason does
   not pass on a small `mib`. The time assertion (`:654`) goes; the time is printed. Red with
   the defect put back by hand: the copies of `ExceptionData` made again without a charge
   (`prelude.rs:520-523 @ a14e3b0`, named in the doc comment at `:632-637`); the process grows
   by 16 MiB and the script is not stopped.
4. `depth.rs:55` (16,000 interfaces). Cost the bound protects: `Model::new` finds each name
   through one map (`model.rs:375-389`, `index`, `definition_of` `:417-421`), not by a scan of
   every definition per statement (the doc comment at `depth.rs:50-53`). Count that proves it,
   none exists: the story adds `surf_webidl::definition_visits()`, a thread-local counter behind
   `#[doc(hidden)] pub fn` in `crates/webidl/src/model.rs`, in the form of `attribute_visits`,
   re-exported in the `pub use model::{...}` of `crates/webidl/src/lib.rs:102`. A name lookup
   counts the definitions it looked at: `definition_of` counts 1 for the definition it
   returns, `all_named` (`:674-680`) counts each definition it yields. The test builds a model
   of 8,000 and one of 16,000 interfaces (the same text, `i` up to the count) and asserts that
   the visits of the second are at most 3 times those of the first. Arithmetic: a lookup
   through a map visits what it returns, so the visits grow with the statements,
   16,000 / 8,000 = 2; a scan of all definitions per statement grows with the square,
   2 x 2 = 4; the bound 3 lies between. The time assertion (`:71`) goes; the time is printed.
   Red with the defect put back by hand: `definition_of` written as
   `self.definitions.iter().find(...)` counting each definition it compares; the ratio is 4.
5. `script_hook.rs:1740` (`a_hook_that_keeps_every_element_open_ends_at_the_step_limit`). Cost
   the limit protects: 40,000 rounds of `<span><script>x</script>` with `KeepOpen` (`:1700`)
   would read about 8 x 10^8 entries of the stack of open elements (run `k` reads about `k`
   entries, so `n` runs read `n` x `n` / 2: 40,000 x 40,000 / 2 = 8 x 10^8); the step limit
   `max_steps = 1 << 27` (`crates/html/src/limits.rs:108 @ a14e3b0`, 134,217,728) ends the
   parse after about 16,000 runs (16,000 x 16,000 / 2 = 1.28 x 10^8, near 2^27). Count that
   proves it, existing: the parse ends `Some(Limit::Steps)` (`:1753`) and prints its `steps`
   (`:1752`). The story adds one count to it: `steps > Limits::default().max_steps`, so that an
   end by `Limit::Steps` before the limit does not pass. The wait goes: the thread, the
   channel and `recv_timeout(10 s)` (`:1742-1751`) are replaced by the parse run in the test's
   own thread, and the time is printed beside the steps as today. The bound of the test is the
   runner's 60 s (`.config/nextest.toml`, `slow-timeout` with `terminate-after`). Red with the
   defect put back by hand: the loop that reads the entries again after a hook run charges no
   step (`each_entry_read_again_after_a_hook_run_is_a_step`, `:1722`, asserts that charge at
   once: `four - two >= 200 * 200`); this test then reads 8 x 10^8 entries, is ended by the 60 s
   kill and counts as failed. The doc comment (`:1735-1739`, "within 10 s ... 1.4 s measured")
   is written again without the 10 s.
6. No test of the five needs a clock. None waits for an event whose time is not its own
   work: each measures work it did itself on its own thread. A wait with a limit of at least
   10 times the time alone stays allowed by the rule and is not touched; an elapsed time
   compared with a constant after the work is not that form, and a limit of 5.6 to 7.2 times
   the time alone is not either.
7. The name of the `depth` test: `the_definitions_a_model_visits_double_when_its_interfaces_double`.
   The module's other names (`depth.rs:40`, `:46`, `:75`: `types_32_deep_are_read`,
   `types_33_deep_are_refused`, `types_5000_deep_are_refused_on_the_test_threads_stack`) state
   the case and its result; the new one states the count and its result. `AGENTS.md:1054`
   ("`depth` also holds a model of 16,000 interfaces under 2 s") says what the test holds and is
   written again. The `script_hook` test keeps its name: it states the end, not a time.
8. The source check. Written in POSIX sh with POSIX awk only (no gawk extension: no
   `gensub`, `strftime`, `asort`, `\<`, `RT`, or a regular expression interval), in
   `taskfiles/`, because the repository has no xtask crate
   (`git ls-tree -r --name-only a14e3b0 | grep -c xtask` prints 0) and its other checks are sh
   (`taskfiles/doc-tests-check.sh`, which the new files copy in form: a script, a task, and a
   `-test` task with a `SCRIPT` variable). The header of the script says that it uses POSIX
   awk only. Files: `taskfiles/test-clocks-check.sh`, `taskfiles/test-clocks.yml` (tasks
   `test-clocks-check` and `test-clocks-check-test`, included like `doc-tests.yml`),
   `taskfiles/test-clocks-ledger.txt`. `Taskfile.yml` lists `test-clocks-check` in `check`
   after `doc-tests-check` and `test-clocks-check-test` after `doc-tests-check-test`. The forge
   runs `task check` (`.github/workflows/check.yml:159-161`), so the check runs there, on
   `ubuntu-latest`, whose `awk` is not gawk.
   Reading: every `*.rs` under any `tests/` directory below `crates/`, `generated/` and
   `conformance/`, comments and string literals blanked first (a clock inside a string or a
   `//` line is no clock). The sizes at `a14e3b0`:
   `git grep -n -E 'Instant::now|\.elapsed\(\)|duration_since|SystemTime' a14e3b0 --
   ':(glob)**/tests/**/*.rs'` prints 251 lines in 73 of the 291 files under `tests/`
   (`git ls-tree -r --name-only a14e3b0 | grep -c -E '(^|/)tests/.*\.rs$'`); 27 of those lines
   are under `crates/script/tests` and `crates/webidl/tests`, and 30 with `duration_since` and
   `SystemTime`. Refused: an upper bound on an elapsed value in a statement.
   - an `assert!`, `assert_eq!`, `assert_ne!` or `debug_assert!` whose condition compares an
     elapsed value with anything by `<` or `<=` (`took < Duration::from_secs(2)`), or puts it
     on the large side of `>` or `>=` (`LIMIT > took`);
   - an elapsed value is `.elapsed()`, `.duration_since(..)`, `.saturating_duration_since(..)`,
     any method chain on one (`.as_secs_f64()`, `.as_millis()`), and a name bound by `let` from
     one, directly, through another such name (three rounds), or in a tuple pattern whose
     element name holds `took`, `elapsed`, `seconds`, `secs`, `waited`, `duration` or `time`
     (`let (end, grown, seconds) = ...`, `adversary_4.rs:645`);
   - an `if` or `while` whose condition compares such a name upward with a `Duration`, a number
     or an upper-case constant (`if took >= Duration::from_secs(1) { panic }`), which is the
     same bound written as a branch.
   Allowed: a lower bound (`assert!(took >= ..)`); a print; a wait with a limit, written as a
   loop that leaves on `started.elapsed()` or `Instant::now()` against a deadline and asserts
   the event afterwards, or a `recv_timeout`. Not seen (stated in the script's header): a bound
   written through a helper function, a macro, or a comparison split over a `let` of a `bool`,
   and whether the limit of a wait is at least 10 times the time alone, which no source reading
   can know (the `script_hook` test of decision 5 is such a wait, so it is in no ledger line).
   The scan found 47 statements in 30 files at `a14e3b0` (the four assertions of this story and
   43 in 27 files);
   `taskfiles/test-clocks-ledger.txt` lists the 27 files, one `<path> <count>` per line, sorted,
   no comment (the form of `taskfiles/lock-allow.txt`). A file with more statements than its
   line (or with none and no line) fails, naming file and line of each statement; a file with
   fewer fails too and names the count to write, so the ledger only shrinks.
   `story:time-bounds-in-tests` empties it. `sh taskfiles/test-clocks-check.sh --count .`
   prints the ledger's lines for a tree. The two cases the story's own test shows first: a file
   it refuses and a file it lets pass.
   - Refused: `#[test] fn a() { let started = Instant::now(); work(); let took =
     started.elapsed(); assert!(took < Duration::from_secs(2), "{took:?}"); }` printing
     `<path>:<line>` of the `assert!` and exit 1.
   - Passes: `#[test] fn a() { let started = Instant::now(); while !done() { if
     started.elapsed() > LIMIT { break; } } assert!(done()); }` with exit 0 and the line
     `test-clocks-check: no elapsed-time bound outside taskfiles/test-clocks-ledger.txt`.

## Change

- `crates/script/tests/adversary_5.rs`: the assertions at `:241` and `:401` go and the times
  are printed (`println!`); the test at `:231` gains the steps ceilings of decision 2.
- `crates/script/tests/adversary_4.rs`: the assertion at `:654` goes, the time is printed, the
  second arm at `:657` becomes `end == ScriptEnd::Ran && mib < 4`.
- `crates/webidl/src/model.rs`: the counter `definition_visits` and its two counting calls;
  `crates/webidl/src/lib.rs`: its name in the `pub use model::{...}` list.
- `crates/webidl/tests/depth.rs`: the test at `:55` is replaced by the one of decision 7.
- `crates/html/tests/script_hook.rs`: the test at `:1740` runs the parse in its own thread
  with no channel and no timeout, asserts `steps > Limits::default().max_steps`, and its doc
  comment loses the 10 s (decision 5).
- `taskfiles/test-clocks-check.sh`, `taskfiles/test-clocks.yml`,
  `taskfiles/test-clocks-ledger.txt`: new, decision 8. `Taskfile.yml`: the include and two steps.
- `AGENTS.md`: the entry of `task check` lists the two steps; the entry of `task test` (the
  sentence at `:1965-1967`) names the check; the entry of `crates/webidl` (`:1054`) says what
  `depth` holds now. The test counts of the entries stay (no test is added or removed: `depth`
  keeps 4, `crates/script` and `crates/html` keep their counts).

A file under `src/` changes (`crates/webidl/src/model.rs` and `lib.rs`), which the draft said
no file would.

## Acceptance

1. No test of the four files asserts a comparison of an elapsed time with a constant or waits
   with a timeout: `git grep -n -E 'seconds < |total < Duration|recv_timeout' --
   crates/script/tests/adversary_4.rs crates/script/tests/adversary_5.rs
   crates/webidl/tests/depth.rs crates/html/tests/script_hook.rs` prints nothing, and
   `sh taskfiles/test-clocks-check.sh .` prints
   `test-clocks-check: no elapsed-time bound outside taskfiles/test-clocks-ledger.txt` and
   exits 0 on the tree; run on a copy of `a14e3b0`'s tree with the new script and an empty
   ledger it prints 47 lines `<path>:<line>`, in 30 files, and exits 1. Turns red by: a bound
   `assert!(took < Duration::from_secs(2))` written back into `depth.rs`; the script names its
   file and line.
2. `task test-clocks-check-test` prints `test-clocks-check-test: 14 cases` and exits 0; the
   cases are: refused, six: a name bound from `.elapsed()` against `Duration`; `.elapsed()`
   direct in the assert; `LIMIT > took`; `as_secs_f64` against a float; a tuple pattern with
   `seconds`; an `if took >= Duration` branch. Passed, five: the wait loop with a break; a
   lower bound; a print only; a deadline loop on `Instant::now()`; a clock inside a comment
   and a string. Ledger, three: a count equal passes, a count above is refused naming the
   line, a count below is refused naming the count to write. Turns red by: `SCRIPT=<copy>`
   with the `<=?` of the upper-bound pattern removed; the test prints each case decided
   otherwise and fails.
3. The count of each test, and the hand-made change of the code that turns it red:
   `adversary_5.rs:373` asserts `G.rounds` (`rounds[2] * 2 > rounds[1]`), red with the `code`
   getter copying the name; `adversary_5.rs:231` asserts the steps of its three scripts at the
   ceilings, red with `k < 4000`; `adversary_4.rs:641` asserts `Limit(Memory)` or `Ran` with
   `mib < 4`, red with the copies uncharged; `depth.rs` asserts that
   `definition_visits` of 16,000 interfaces is at most 3 times that of 8,000, red with the
   scan in `definition_of`; `script_hook.rs:1740` asserts `Some(Limit::Steps)` and
   `steps > max_steps`, red with the entries read again charging no step (the test of `:1722`
   fails at once, this one by the 60 s kill). Each is shown red once before the change is
   kept, and the report names the output.
4. The test step of `surf-script`, `surf-webidl` and `surf-html`, run alone
   (`slot cargo nextest run -p surf-script -p surf-webidl -p surf-html --locked`) and again
   while another build of the same tree's crates runs in the pool: the same tests pass in
   both. No test over 60 s, the step within 120 s, a test over 5 s named with its time (the
   three script tests, the new `depth` test and the `script_hook` test, each by its printed
   time), no pool job over 10 minutes, none that names a longer time without
   `slot --timeout` (at most 30 minutes), none started again unchanged.
5. Every command that compiles or tests carries `slot` (`cargo` and `go` through the shim). The
   check is POSIX sh and POSIX awk only: its header says so and it uses no gawk extension; no
   Python in anything that stays; a script of the story that measures or waits (the two loaded
   runs above, a timing of the check) starts without `slot`. No test makes a model call or a
   network request. A limit on a wait in a test the story writes is at least 10 times the
   event alone. The run time of `task test-clocks-check` is printed in the report (a step over
   5 s is named), and it passes under the `awk` of the forge's image: the forge's run of
   `task check` on the branch shows the step green.

## Out of scope

- The other 43 statements in 27 files (the ledger), and the 12 assertions in 7 files that
  `story:time-bounds-in-tests` lists: that story. This story does not edit those files.
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
  cites `model.rs:88` (its line 345); this story edits
  `definition_of` and `all_named` (`:417-421`, `:674-680`). No edge.
- `crates/html/tests/script_hook.rs` is also named by `story:scripts-dom-attributes`
  (proposed), which adds tests of reads in the module where the write tests are (`:1157-1274`);
  this story edits `:1740-1753`. No edge. `story:html-script-hook` (implemented) is history.
- `Taskfile.yml`: `story:agent-cookie-public-suffixes` (active) adds an include and a step to
  `check`, and this story adds an include and two steps, in the same two lists. No edge: the
  story that merges second takes the other's lines in its merge of `main`.
  `story:conformance-lock-and-lints` (draft) edits the lines of `lock-check`, five lines above
  `doc-tests-check` in `check`; `story:test-step-holds-with-tls` (active) edits the lines of
  the test steps. No edge.
- `AGENTS.md` is named by 40 stories; this story edits `:1054` and `:1965-1967` and the `check`
  paragraph. `story:scripts-dom-reading` edits `:365-368`. No edge.
- `story:time-bounds-in-tests` (draft) takes over the ledger, `taskfiles/test-clocks-ledger.txt`,
  and needs the scanner's output (47 statements in 30 files, against its list of 12 in 7). Its
  text is not changed here and this story has no relation to it.
- `crates/webidl/src/lib.rs`, `crates/script/tests/adversary_4.rs`, `crates/webidl/tests/depth.rs`
  and the new `taskfiles/` files are named by no other story.
- `story:agent-tasks-left-process-wait` edits `crates/agent-tasks/tests/adversary2.rs` and
  `bounded.rs`, which are two lines of the ledger (`adversary2.rs:120`, `bounded.rs:97` are
  statements the story does not touch). No edge.
