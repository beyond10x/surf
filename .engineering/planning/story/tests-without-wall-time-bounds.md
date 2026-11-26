---
format: aep.planning-md/3
id: story:tests-without-wall-time-bounds
kind: story
status: implemented
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
  path: crates/script/src/bindings.rs
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
revision: 25
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T17:30:08Z", actor: "human:operator", revision: 16}
- {from: "proposed", to: "active", at: "2026-11-26T04:50:10Z", actor: "human:operator", revision: 23}
- {from: "active", to: "implemented", at: "2026-11-26T10:01:46Z", actor: "human:operator", revision: 25, decided_on: {"recorded":{"test_result":2,"verification":1}}, executor: "agent:coordinator"}
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
   (`crates/script/src/bindings.rs:1617-1625 @ a14e3b0`, its doc comment from `:1613`) answers
   0 for a name longer than 26 code units without a copy (`:1619-1621`); a copy goes through
   `Copies::copy` (`:553-563`), which charges `units / 64` steps (`:561`): the name of 2^14
   code units of U+0800 is 48 KiB, and 48 KiB / 64 bytes = 768 steps per read (doc comment at
   `:368-371`).
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
   twice) and of the four collections `collect` makes (two per call, `:171-176`). No count
   proves it today: `ran` asserts `ScriptEnd::Ran` only (`:168`), so a script that charged ten
   times the steps still passes under the 200,000,000 of `steps_per_entry` (`:46`). The story
   adds the count: `runner.steps()` (`crates/script/src/runner.rs:821-823 @ a14e3b0`, read the
   same way at `adversary_4.rs:466`) before and after each of the three scripts, asserted at a
   ceiling per script that is 1.5 times the steps of the first green run (the steps are
   deterministic, so the margin is for a change of the engine, not for a machine). The
   measured steps and the ceilings are printed by the test and named in the commit message.
   The time assertion (`:240-241`) goes; the time is printed.
   - Proves: the steps the three scripts charge. Red with a defect put back by hand, on each
     side. In the test: the loop bound `k < 40` of `texts` in `MAKE` (`:184`) written as
     `k < 4000`; each script charges far more steps and the ceiling fails. In the code under
     test: the getter `dom_exception_message` (`bindings.rs:1605-1608`) made to copy its String
     through `Copies::copy` (`:553-563`) on each read, and to return the handle as before; each
     read of `READ` then charges `units / 64` steps more. Whether that adds more than the 50 %
     margin of the ceilings is not known before the first green run: the implementor reports
     the steps the defect adds against the ceilings; when it stays inside the margin the report
     says so with the figures, and that defect is then one of "Does not see".
   - Does not see: the four `collect` calls. They charge no script step (`runner.steps()` sums
     the scripts only), so their cost is dropped on purpose; it is the part of the old bound
     that no count here stands for. Also slow work that charges no step, which the old bound
     saw too; and the allocations of `churn` (`:200` in `MAKE`, `:206` in `READ`, defined at
     `:30-35`) if an allocation of the engine charges no step: the implementor says which when
     it reports the ceilings.
3. `adversary_4.rs:641` (`a_dom_exception_holds_no_uncharged_copy`). Cost the bound protects:
   64 copies of a message of 2^18 bytes (`:645-646`, 64 x 256 KiB = 16 MiB) made by the
   constructor. Count that proves it, existing: the process grows by `mib` MiB or the script
   ends `Limit(Memory)` (`:656-659`). The story tightens it: the first arm stays, the second
   becomes `end == ScriptEnd::Ran && mib < 4`, so a script that ended for another reason does
   not pass on a small `mib`. The time assertion (`:654`) goes; the time is printed.
   - On `main`, `ExceptionData` (`crates/script/src/bindings.rs:315-318`) holds two
     `StringRef`s, handles into the engine's heap, and copies nothing; `new_exception`
     (`:657-668`) stores them. The doc comment at `adversary_4.rs:632-637`, which says the
     constructor copies both Strings out of the engine (`prelude.rs:1347-1357`), is out of date
     in the same way, and the story writes it again.
   - Proves: the process does not grow by the 16 MiB, or the script is stopped by the memory
     limit, and in the second case it ran to its end. Red with the defect put back by hand:
     `ExceptionData` made to own its strings (`name: String, message: String`, filled at `:666`
     by `to_utf16` and `String::from_utf16_lossy` with no charge); the process grows by 16 MiB
     and the script is not stopped. Red, for the new `ScriptEnd::Ran` conjunct, with a second
     defect: `small.engine.steps_per_entry` (the `limits()` of `adversary_4.rs:38-49` sets it to
     10,000,000) lowered in the test to a value that ends the loop of `:650` by
     `Limit(StepsPerScript)` before 4 MiB of messages are held; on `main` that passes (`mib < 4`
     holds), after the change it fails (the end is neither `Ran` nor `Limit(Memory)`).
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
   `check_inheritance` (`:503`, the read of `definitions[parent]`) and the cycle report (`:570`,
   `:576`) call it. `index` calls it only inside its duplicate-name error return, so it counts
   on its error path only and adds nothing for the clean text of this test. The loops that
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
     counts; the ratio is near 4. A scan per statement may run long at 16,000 interfaces in a
     test build: if the 60 s kill ends the test before the ratio assert reads, the ratio is shown
     with a lower `n` for that one run (the two counts at the lower `n`, printed), and the `n`
     of the committed test, 8,000 and 16,000, does not change.
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
   a parse by `Limit::Steps` only when `steps > max_steps`: two lines call
   `stop_at(Limit::Steps)`, the check after each token (`tree_builder.rs:687`) and
   `past_step_limit` (`:1039-1045`, through `spent(0)`, `:1034-1036`), and both compare with
   `>` (`:687` and `:1035`). `limits.rs:75` says a parse ends fewer than `4 * max_nodes` steps
   past `max_steps`; `max_nodes` is `1024 * 1024` (`limits.rs:105`), so 4 x 1,048,576 =
   4,194,304.
   The test asserts the end `Some(Limit::Steps)` (`:1753`, as today) and the documented
   window of the steps at that end:
   `Limits::default().max_steps < steps < Limits::default().max_steps + 4 *
   Limits::default().max_nodes`, that is more than 134,217,728 and less than 138,412,032 steps.
   The window is built from `Limits::default()` (`limits.rs:108`, `:105`), so it moves with a
   changed default: the test sees a change of the comparisons, not of the default.
   It prints the steps and the time as today. The wait goes: the thread, the channel and
   `recv_timeout(10 s)` (`:1742-1751`) are replaced by the parse run in the test's own thread,
   with no spawn and no channel. The bound of the test is the runner's 60 s
   (`.config/nextest.toml:5`, `slow-timeout` with `terminate-after`). The doc comment
   (`:1735-1739`, "within 10 s ... 1.4 s measured") is written again without the 10 s.
   - Lower side, strict: an end by `Limit::Steps` has more than `max_steps` steps, never
     `max_steps` or fewer. Both stop lines compare with `>` (`:687`; `:1035`, read by
     `spent(0)` at `:1040`). `spent(pending)` with a `pending` above 0 only ends a scan
     (`:1289`, `:1661`, `:1748`), and the scan adds the `visited` it counted before it ends
     (`:1298`, `:1673`, `:1757`), which puts the steps above `max_steps` before any stop reads
     them. The test asserts `steps > max_steps`, not `>=`. If the first green run reads `steps`
     at or under `max_steps`, this reading of the code is wrong: the implementor stops and
     reports the steps and the end, and does not widen the comparison.
   - Proves: the parse ends by the step limit and charged between `max_steps` and the window's
     end: every entry read again after a hook run is a step. Red with the defect put back by
     hand: the loop that reads the entries again after a hook run charges no step (the
     `self.visit()` of `settle_entries`, `tree_builder.rs:1199`, removed;
     `each_entry_read_again_after_a_hook_run_is_a_step`, its `fn` at `:1721`, asserts that
     charge at once, `four - two >= 200 * 200` at `:1730`). The parse then probably ends
     with `stopped_by() == None`, the 120,000 nodes of the 40,000 rounds (three per round: the
     `span`, the `script` and its text) being under `max_nodes` (1,048,576), and
     `assert_eq!(stopped, Some(Limit::Steps))` fails; if it does not end, the 60 s kill does,
     whichever comes first. This red is shown once as a demonstration, in one pool job, and
     not run again unchanged.
   - Upper side: no one-line defect turns it red, and a two-line one does. Each stop line alone
     leaves the other:
     - `spent` written `> 2 * self.limits.max_steps` (`:1035`) alone: the scans and writes no
       longer end at `max_steps`, and `settle_entries` (`:1187-1204`; its read at `:1196` no
       longer stops it) reads each entry of the stack once more. The hook's run is reported
       inside the token (`:680-684`), and the check after the token (`:687`, `steps >
       max_steps`) then stops the parse at `Limit::Steps`. The steps at the end pass
       `max_steps` by the work between two token checks: one pass over the stack, about
       16,400 entries at the 16,400 rounds the parse has made by then, which is 0.4 % of the
       4,194,304 of the window.
     - `:687` written `> 2 * self.limits.max_steps` alone: `spent(0)` at `:1040` ends the parse
       at the first read past `max_steps`, which `settle_entries` makes at its next entry
       (`:1196`); the steps at the end are `max_steps` plus a few.
     - Both written with `2 *`: nothing stops the parse at `max_steps`; it ends at the first
       check past 268,435,456 steps, about 23,000 rounds (23,000 x 23,000 / 2 = 2.6 x 10^8),
       with `Some(Limit::Steps)` and more than 138,412,032 steps. The window's upper side goes
       red and the 60 s kill is not reached (not measured). This red is shown once.
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
   - Usage: `sh taskfiles/test-clocks-check.sh [--count] [--ledger <file>] [<root>]`; `<root>`
     is the top of a git work tree: `git -C <root> rev-parse --show-toplevel` prints the path
     that `cd <root> && pwd -P` prints. When it does not (no `.git`, or a directory inside an
     outer work tree, as the case trees are inside this repository's), the script prints
     `test-clocks-check: <root> is not the top of a git work tree` and exits 2. The awk is the
     one named by the variable `AWK` (default `awk`; it may hold words, as in `AWK="gawk
     --posix"`). `--ledger /dev/null` is an empty ledger. `--count` prints the ledger's lines
     (`<path> <count>`) for a tree.
   - The header of the script says that it uses POSIX awk only and forbids by name:
     `nextfile` (not in every mawk), `length(array)`, `IGNORECASE`,
     `gensub`, `asort`, `asorti`, `strftime`, `\<` and `\>`, and a regular expression interval
     (`{n,m}`). Which runner sees which: `gawk --posix` ends on `gensub`, `asort` and
     `strftime` (the functions are undefined in it) and reads `\<` as a literal `<` (with a
     warning, so a word-boundary match never comes); it accepts `nextfile`, `length(array)`,
     `IGNORECASE` (a plain variable) and intervals, as do the awks of the machine, so those
     four are kept out by the script's own rules and by the greps of Acceptance.
     `length(array)` is seen by no runner (gawk accepts it in every mode, `--posix` included):
     every array of the script is named with the prefix `arr_`, and `length(` is written only
     on a string (`length(arr_lines[n])` is a string; `length(arr_lines)` is refused by the
     grep of Acceptance). A line of the script whose first non-blank character is `#` may hold
     any of these words, as the header does, in the sh text and in the awk text; a `#` comment
     after code on the same line is not used, so that no grep reads one by mistake.
   - Reading: the file list is `git ls-files` of `<root>` with the path patterns
     `crates/**/tests/**/*.rs`, `generated/**/tests/**/*.rs` and `conformance/**/tests/**/*.rs`
     (the `:(glob)` pathspec form), so only tracked files are read: an untracked file, a
     `target` directory and a dot directory that git does not track are not read, and a local
     tree and a clean checkout scan the same files. Each file is read with comments, strings,
     raw strings and character literals blanked first (a clock inside a string or a `//` line
     is no clock).
   - Blanking, one pass over a file, line by line, with the state kept across lines: code, line
     comment, block comment (with its depth), string, raw string (with its count of `#`). Every
     character of a comment, a string, a raw string and a character literal, its delimiters
     included, becomes a space; line feeds stay, so a line number of the blanked text is the
     file's. In code: `//` (the `///` and `//!` doc comments too) opens a comment to the line
     feed; `/*` opens a block comment, a `/*` inside it deepens it and each `*/` ends one level
     (Rust nests them); `"` opens a string that ends at the next `"` that no backslash hides (a
     backslash hides the next character); `r`, `br` or `cr` not after a letter, a digit or `_`,
     then `n` times `#` and `"`, opens a raw string that ends at the first `"` followed by `n`
     times `#`, with no escape inside; `'` opens a character literal in two cases. A backslash
     follows: the reader skips the backslash and the character it hides, and reads on to the
     next `'`, which closes the literal (`'\''`, `'\\'`, `'\n'`, `'\u{41}'`, `'\u{2028}'`). Or
     one character follows and the character after it is a `'` (`'"'`, `'{'`, `'a'`, `'é'`;
     the awk runs under `LC_ALL=C`, so a character is an ASCII byte or a UTF-8 sequence of 2 to
     4 bytes). Any other `'` is a lifetime or a label (`'a`, `'static`, `'reading`) and stays
     code: after `'a` or `'r` comes no `'` (`<'a,'b>` reads `'a,`, `'reading:` reads `'re`).
     A probe of this paragraph alone, an awk run under `LC_ALL=C` over the lines
     `crates/agent/tests/drift.rs:463 @ a14e3b0` (`'\u{8}'`) and
     `crates/answer/tests/forging.rs:20 @ a14e3b0` (nine literals: `'a'`, `'"'`, `'\\'`,
     `'\n'`, `'\r'`, `'\u{2028}'`, `'\u{202e}'`, `'é'`, `' '`), blanks every literal and
     leaves no `'` in the second line; the cases below hold the lifetime, the label and each
     escape form. The implementor reads first the literal rule of `code_of` in
     `taskfiles/test-layout.sh` (`:137-143 @ a14e3b0`), which blanks a line by one pattern per
     literal and reads `'a'` and `'\n'` but not `'\u{41}'`: this check is held to the rule
     above, not to that one, and the two scripts share no code.
   - Refused: an upper bound on an elapsed value in a statement.
     - An `assert!`, `assert_eq!`, `assert_ne!` or `debug_assert!` (the `debug_assert_eq!` and
       `debug_assert_ne!` forms too) whose arguments, read to the closing parenthesis of the
       macro over any number of lines, hold a comparison with an elapsed value as its smaller
       operand: left of `<` or `<=` (`took < Duration::from_secs(2)`, `took <= LIMIT`), right of
       `>` or `>=` (`LIMIT > took`, `LIMIT >= took`). Every comparison of the arguments is
       read, not the first operator: `assert!(took >= IDLE && took < IDLE +
       Duration::from_secs(2))` (`crates/tls-test-server/tests/adversary_request_body.rs:119-122
       @ a14e3b0`) holds a lower and an upper bound and is refused for the upper one.
       `assert_eq!` and `assert_ne!` compare by equality,
       which is no upper bound: `assert_eq!(took, LIMIT)` is not seen (the time of a run is
       never equal to a constant, so such a test fails at once on any machine and cannot be a
       flake), and `assert_eq!(took < LIMIT, true)` is refused, because its argument holds the
       comparison.
     - An `assert!` on an elapsed value inside a poll loop (`while !condition() {
       assert!(started.elapsed() < WAIT, ..); sleep(..); }`) is refused like any other, though
       `AGENTS.md:1965-1967` allows a wait with a limit of 10 times the time alone: the scan
       cannot know that limit, and the loop leaves through the assertion, not through the
       `break` that Allowed names. Three statements of `a14e3b0` have that form:
       `crates/view/tests/http.rs:389` (limit `WAIT`), `crates/view/tests/stream_bounds.rs:183`
       (limit `within`, a parameter) and `crates/surfd/tests/hardening.rs:532` (limit
       `ALLOWED`). Each goes to the ledger, and `story:time-bounds-in-tests` decides them; this
       story does not edit them.
     - An elapsed value is `.elapsed()`, `.duration_since(..)`,
       `.saturating_duration_since(..)`, any method chain on one (`.as_secs_f64()`,
       `.as_millis()`, `.expect(..)`), and a name bound by `let` from one, directly (round 1),
       through a name of round 1 (round 2) or through a name of round 2 (round 3), or in a
       tuple pattern whose element name holds `took`, `elapsed`, `seconds`, `secs`, `waited`,
       `duration` or `time` (`let (end, grown, seconds) = ...`, `adversary_4.rs:645`). A name
       bound through a name of round 3 is not seen, the stated limit of the check. A name
       directly after `.` is a field or method of another value and is not the tracked name,
       whatever the file binds: `counts.total > 0`
       (`conformance/tests/two_of_each_entity.rs:411 @ a14e3b0`) holds no operand `total`
       even in a file that binds `total` from an elapsed value.
     - A `let` is read from its keyword to the first `;` outside brackets (`()`, `[]` and `{}`,
       on the blanked text), over any number of lines: the four lines
       `let elapsed = accepted_at.try_recv().ok().map(|at| ended.saturating_duration_since(at));`
       of `crates/fetch/tests/fetch_bounds.rs:207-210 @ a14e3b0` are one statement that binds
       `elapsed` from an elapsed value. The name is the identifier after `let` and an optional
       `mut` (`_` binds nothing). A tracked name stays tracked to the end of the file: it is
       not dropped at the end of its `fn`, and a later `let` of the same name from a value that
       holds no elapsed value does not drop it, because a reader of POSIX awk decides the end
       of a file and does not parse the end of a `fn`. The cost is a false refusal where a name
       bound from an elapsed value in one `fn` is a plain `Duration` (a parameter, a loop
       variable, a `let` from a constant) in an `assert!` of another `fn` of the same file. A
       probe over the 291 files of `a14e3b0` (an awk over `git show`; one round of names, the
       text not blanked, tuple patterns not read, so not the check) finds 34 files that bind
       a name by `let` from `.elapsed()` or `duration_since(..)` (a tuple `let` over more than
       one line was not counted by it): none binds the same name
       elsewhere from a value that holds no elapsed value (but the `.expect(..)` of
       `fetch_bounds.rs:221`, the same value), and none uses the name as a parameter, closure
       parameter or loop variable, so the false refusals today are 0.
       Three of the 291 files hold such a `let` over more than one line (`fetch_bounds.rs:207`,
       `crates/script/tests/engine.rs:323`, `crates/view/tests/http.rs:379`); a reader that
       took a `let` line by line would miss them. Only the first is an upper bound on a
       measured time: `:221` rebinds `elapsed` through `elapsed.expect(..)` and `:222-223`
       asserts `elapsed < NEVER_READ_BOUND`. By these rules that statement is in the ledger's
       lines (the count of `crates/fetch/tests/fetch_bounds.rs`), reported at `:222`; a
       line-by-line reader had missed it. The other two bind a reading of the system clock
       and compare `(now - before).abs()` and `(now - then).abs()`: an operand is an elapsed
       value when it is exactly the call or the name, with its method chain, so a difference
       in parentheses is not one and neither is refused.
     - An `if` or `while` whose condition holds upward, with an elapsed value (the call or such
       a name) as its larger operand against a `Duration`, a number or an upper-case constant:
       left of `>` or `>=`, right of `<` or `<=`; and whose block fails the test. The scanner
       reads the condition from the keyword to the `{` that opens the block, over any number of
       lines, and the block from that `{` to the matching `}` (nested braces counted, on the
       blanked text) and no further. The block fails the test when it holds `panic!`,
       `unreachable!`, or a word that starts with `assert` or `debug_assert` (any `assert!`
       form, `assert_fails(..)` and the like). A branch that leaves a loop (`break`, `continue`,
       a `return` of a value) passes, and so does a block that only sets a flag, whatever the
       flag is named (`failed = true; break;`): the word `fail` is no part of the rule. At
       `a14e3b0`, `git grep -n -i -E '\bfail[a-z_]*\s*(\(|!)' -- '*/tests/*.rs'` finds the value
       `Verdict::Fail(..)`, the helpers `fail`, `failed`, `failure`, `failed_with`, and
       `fails` (`crates/agent-tasks/tests/https_row.rs:64`), `fails_after_changing` and
       `fails_without` (`crates/tls-test-certs/tests/check.rs:82`, `:156`) that pass when the
       code under test fails,
       and `assert_fails*(..)` helpers that the word `assert` already holds; no call fails a
       test by itself and is spelled with `fail` without `assert`.
     - A `while` or `if` whose condition holds downward (`while started.elapsed() < LIMIT`, the
       loop runs while the time is under the limit) is not read by this rule: it is the wait
       with its limit in the loop head. An `assert!` in its block that holds no elapsed value
       (`assert!(!early())`) passes; one that does (`assert!(started.elapsed() < LIMIT)`) is
       refused by the first rule.
   - Allowed: a lower bound (`assert!(took >= ..)`, `assert!(.. <= took)`); a print; a wait with
     a limit, written as a loop that leaves on the deadline (`started.elapsed()` or
     `Instant::now()` against it, by `break` or `return`) and asserts the event after the loop,
     or a `recv_timeout`. The script's header names that spelling as the one to use.
   - Not seen (stated in the script's header): a bound written through a helper function, a
     macro, or a comparison split over a `let` of a `bool` (`let ok = took < LIMIT;
     assert!(ok)`); a name bound through a name of round 3; the negated spelling
     `assert!(!(took >= LIMIT))`; `assert_eq!(took, ..)` by equality; a bound recorded in a
     list that is asserted after the loop (`if took >= LIMIT { slow.push(..) }`, then
     `assert!(slow.is_empty())`, the form of `crates/webidl/tests/adversary_4.rs:229`, `:238`
     and `crates/webidl/tests/extraction.rs:195`); the passing form of a test that
     returns `Result` (`if took > LIMIT { return Err(..) }`; no `#[test]` of `a14e3b0` is
     followed within two lines by a `fn` that returns `Result`); a failure by `.expect(..)`,
     `.unwrap()` or `std::process::exit` in the block; a difference of two instants written
     with `-`; and whether the limit of a wait is at least 10 times the time alone, which no
     source reading can know (the `script_hook` test of decision 5 is such a wait, so it is in
     no ledger line).
   - Ledger: `taskfiles/test-clocks-ledger.txt` lists the files of the committed tree that
     hold statements the scan refuses and are not fixed by this story, one `<path> <count>`
     per line, sorted, no comment (the form of `taskfiles/lock-allow.txt`). A file with more
     statements than its line fails, naming file and line of each statement; a file with
     refused statements and no line is that case, and a file with none needs no line. A file
     with fewer fails too and names the count to write, so the ledger only shrinks. A line
     whose file the scan did not read (not tracked, renamed, deleted, or outside the path
     patterns) fails, naming the line and the remedy, `delete the line`.
     `story:time-bounds-in-tests` empties it. The implementor writes the ledger from
     `--count` on the changed tree and reports both totals: the lines the scan prints on
     `a14e3b0` with an empty ledger and the sum of the committed ledger.
   - The cases of the check's own test, by name (no count is asserted beside the list). Each
     case is a small tree under `.out/test-clocks-check-test/case`, made the top of a git work
     tree by `git init -q --template=` and `git -c core.excludesFile=/dev/null add` (no commit,
     so no identity is needed). Every `git` of the test runs with `GIT_CONFIG_NOSYSTEM=1` and
     `GIT_CONFIG_GLOBAL=/dev/null` and with `GIT_DIR`, `GIT_WORK_TREE` and `GIT_INDEX_FILE`
     unset: no configuration of the user or the system is read (no template directory, no
     hook). The ignore file that git reads without any configuration
     (`$XDG_CONFIG_HOME/git/ignore`) is switched off by `core.excludesFile` on each `git add`,
     which is taken and not `git add -f`: `-f` would also add what a case's own `.gitignore`
     hides, and no global ignore rule of the machine can then decide a case. Everything git
     writes (`.git/` and its index) is inside the case tree. The case
     `plain_directory_exit_2` is the one tree that is not made a work tree: a directory under
     `.out/` inside this repository's own work tree. Refused:
     `elapsed_name_against_duration`, `elapsed_direct_in_assert`, `limit_above_name`,
     `float_seconds`, `tuple_with_seconds`, `branch_that_panics`, `assert_inside_poll_loop`,
     `less_or_equal_refused`, `limit_at_least_name_refused`, `duration_since_refused`,
     `rebound_through_a_second_name_refused`, `rebound_through_a_third_name_refused`,
     `assert_eq_over_a_comparison_refused`, `compound_assert_refused`,
     `nested_block_with_assert_refused`, `condition_over_two_lines_refused`,
     `assert_over_two_lines_refused`, `let_over_two_lines_refused`,
     `quote_char_before_a_bound_refused`, `lifetime_before_a_bound_refused`,
     `label_before_a_bound_refused`, `unicode_escape_before_a_bound_refused`,
     `two_unicode_escapes_before_a_bound_refused`, `non_ascii_literal_before_a_bound_refused`,
     `three_byte_literal_before_a_bound_refused`, `untracked_file_added_refused`.
     Passed: `wait_loop_with_break`,
     `branch_that_breaks_on_a_name`, `branch_that_returns_a_value`, `lower_bound`,
     `lower_bound_with_the_limit_first`, `print_only`, `deadline_loop_on_now`,
     `clock_in_comment_and_string`, `negated_spelling_not_seen`, `assert_eq_on_equality_not_seen`,
     `branch_that_sets_failed_and_breaks`, `assert_after_a_nested_block_passes`,
     `loop_head_under_the_limit_with_assert`, `raw_string_with_quote_and_clock`,
     `block_comment_with_a_bound`, `nested_block_comment_with_a_bound`,
     `fourth_round_of_rebinding_not_seen`, `field_of_another_value_not_refused`,
     `untracked_file_not_read`. Exit 2:
     `plain_directory_exit_2`. Ledger: `count_equal_passes`, `count_above_refused`,
     `count_below_refused`, `ledger_line_for_unread_file_refused`. The test task runs every
     case under each of `awk`, `mawk`, `gawk`, `gawk --posix` and `busybox awk` that exists on
     the machine and prints which ran.
   - What the new cases hold, each decided by hand against the rules above:
     - `let_over_two_lines_refused`: the shape of `fetch_bounds.rs:207-223`, a `let` of
       `elapsed` over four lines ending in `.saturating_duration_since(at));`, a rebinding
       `let elapsed = elapsed.expect("..");` and `assert!(` / `elapsed < LIMIT,` / `"m"` / `);`.
     - `compound_assert_refused`: `assert!(took >= IDLE && took < IDLE + SLACK, "m");`.
     - `rebound_through_a_third_name_refused`: `let a = s.elapsed(); let b = a; let c = b;
       assert!(c < LIMIT);` (round 3). `fourth_round_of_rebinding_not_seen`: the same with
       `let d = c;` and `assert!(d < LIMIT);` (round 4, the stated limit, passes).
     - `unicode_escape_before_a_bound_refused`: `let a = ['\u{41}','"']; assert!(s.elapsed() <
       LIMIT);`; a reader that does not blank the first literal pairs the `','` and opens a
       string at the `"`, which hides the assert. `two_unicode_escapes_before_a_bound_refused`:
       `let e = ['\u{2028}','\u{202e}','"'];` before the same assert, the same failure.
       `non_ascii_literal_before_a_bound_refused`: `let m = ['é','"'];` before it.
       `three_byte_literal_before_a_bound_refused`: `let m = ['€','"'];` before it (U+20AC, the
       three bytes e2 82 ac, the literal of 3 bytes that the rule reads).
     - `field_of_another_value_not_refused`: `let total = s.elapsed(); println!("{total:?}");
       assert!(counts.total < LIMIT);`; the name after `.` is not the tracked name, so the
       assert holds no elapsed value and passes.
     - `untracked_file_added_refused`: the tree of `untracked_file_not_read` whose only refused
       statement is in `crates/x/tests/late.rs`, after `git add` of that file; exit 1, naming
       `crates/x/tests/late.rs:<line>`.
     - `ledger_line_for_unread_file_refused`: a ledger `crates/x/tests/gone.rs 1` and a tree
       with no such tracked file; exit 1, naming the line and `delete the line`.
     - `label_before_a_bound_refused`: `'reading: loop { assert!(s.elapsed() < LIMIT); break
       'reading; }`; a reader that took any `'` to the next `'` for a literal would blank from
       `'reading` to the `'` of `break 'reading` and hide the assert. The case
       `lifetime_before_a_bound_refused` holds `'a` (as before).
     - `plain_directory_exit_2`: `<root>` is a directory with an untracked
       `crates/x/tests/t.rs` and no `.git`, inside the outer work tree; exit 2 and the line of
       Usage, though `git ls-files` run there succeeds and lists nothing, which a check without
       the root test would end with exit 0 and the line `no elapsed-time bound`.
   - Refused example: `#[test] fn a() { let started = Instant::now(); work(); let took =
     started.elapsed(); assert!(took < Duration::from_secs(2), "{took:?}"); }` printing
     `<path>:<line>` of the `assert!` and exit 1.
   - Passing example: `#[test] fn a() { let started = Instant::now(); while !done() { if
     started.elapsed() > LIMIT { break; } } assert!(done()); }`, exit 0 and the line
     `test-clocks-check: no elapsed-time bound outside taskfiles/test-clocks-ledger.txt`. The
     branch leaves the loop and holds none of `panic!`, `unreachable!`, `assert`.

## Change

- `crates/script/tests/adversary_5.rs`: the assertions at `:241` and `:401` go and the times
  are printed (`println!`); the test at `:231` gains the steps ceilings of decision 2.
- `crates/script/tests/adversary_4.rs`: the assertion at `:654` goes, the time is printed, the
  second arm at `:657` becomes `end == ScriptEnd::Ran && mib < 4`, and the doc comment at
  `:632-637` is written again (decision 3).
- `crates/webidl/src/model.rs`: the counter `definition_visits`, the accessor `definition_at`
  and the calls that go through it; `crates/webidl/src/lib.rs`: its name in the
  `pub use model::{...}` list.
- `crates/script/src/bindings.rs`: written again by the generator after the edit of
  `crates/webidl/src`, with `slot cargo run -p surf-webidl --bin bindings` (the command that
  `taskfiles/webidl.yml` and the file's header name). The header lists the sha256 of `lib.rs`
  and `model.rs` (`bindings.rs:56-57 @ a14e3b0`) and of the whole file (`:72`), and the step
  `webidl-check` of `check` compares them. Expected to differ: those three lines and no
  other, because `definition_at` and the counter change no generated byte. The implementor
  shows it with `git diff --stat` (one file, 3 insertions, 3 deletions) and `git diff -U0 --
  crates/script/src/bindings.rs | grep -c -E '^[-+]//! '`, which prints 6. If another line
  differs, that is a finding the report names before anything else.
- `crates/webidl/tests/depth.rs`: the test at `:55` is replaced by the one of decision 7.
- `crates/html/tests/script_hook.rs`: the test at `:1740` runs the parse in the test's own
  thread, with no spawn, no channel and no timeout, asserts the window of decision 5, and its
  doc comment loses the 10 s.
- `taskfiles/test-clocks-check.sh`, `taskfiles/test-clocks.yml`,
  `taskfiles/test-clocks-ledger.txt`: new, decision 8. `Taskfile.yml`: the include and two steps.
- `AGENTS.md`, in five places (line numbers at `a14e3b0`). The test counts of the entries stay
  (no test is added or removed: `depth` keeps 4, `crates/script` and `crates/html` keep their
  counts). The lines written for the first four are these; they hold 15 occurrences of
  `test-clocks`.
  - The list `taskfiles/<name>.yml` (`:137-170`): after the clause of `doc-tests.yml`
    (`:160-161`, ending ``over `taskfiles/doc-tests-check.sh`;``), the lines

          `taskfiles/test-clocks.yml` the tasks `test-clocks-check` and `test-clocks-check-test`
          over `taskfiles/test-clocks-check.sh` and the ledger `taskfiles/test-clocks-ledger.txt`;

  - The paragraph of `task check` (`:1593-1624`): after the sentence on `doc-tests-check`
    (`:1612-1614`), the lines

          Since `story:tests-without-wall-time-bounds` it runs `test-clocks-check` after
          `doc-tests-check` and `test-clocks-check-test` after `doc-tests-check-test`.

  - The entries of the tasks: after the entry of `task doc-tests-check-test`
    (`:1840-1847`), the entries of `task test-clocks-check` and `task test-clocks-check-test`
    in the form of the two `doc-tests` entries (`:1826-1847`):

        - `task test-clocks-check`: `sh taskfiles/test-clocks-check.sh .`. A test proves a cost by
          a count and bounds a time only by a wait with a limit (`task test`), so this reads the
          tracked `*.rs` files under a `tests/` directory of `crates/`, `generated/` and
          `conformance/` (`git ls-files`: an untracked file is not read), blanks comments, strings
          and character literals, and fails, naming `<file>:<line>`, on an `assert!` that holds an
          upper bound on an elapsed value (`.elapsed()`, `.duration_since(..)` or a name bound from
          one) and on an `if` or `while` on such a value whose block fails the test. The ledger
          `taskfiles/test-clocks-ledger.txt` (`<path> <count>`, one file per line) lists the files
          that still hold such statements: a file with more fails, a file with fewer fails and names
          the count to write, so the ledger only shrinks. `AWK=<command>` names the awk (POSIX awk
          only), `--ledger <file>` and `--count` are options, a root that is not the top of a git
          work tree exits 2. Else it prints `test-clocks-check: no elapsed-time bound outside
          taskfiles/test-clocks-ledger.txt`. It calls no cargo.
        - `task test-clocks-check-test`: that script on small trees under
          `.out/test-clocks-check-test/`, one case per name (refused, passed, exit 2, ledger), each
          compared on its exit status and the lines printed, under each of `awk`, `mawk`, `gawk`,
          `gawk --posix` and `busybox awk` that exists on the machine. `SCRIPT=<path>` runs a copy
          of the script.

  - The entry of `task test` (`:1965-1967`): its last line is written again as

          `surf_page::attribute_visits()`), not by a wall time. `task test-clocks-check` (a step of
          `check`) refuses the assertion of an upper bound on an elapsed time that its ledger does
          not list.

  - The entry of `crates/webidl` (`:1054`): `depth` holds now that the definitions a model
    visits double when its interfaces double; the words `under 2 s` go. The sentence holds
    no `test-clocks`.

A file under `src/` changes (`crates/webidl/src/model.rs` and `lib.rs`), which the draft said
no file would; the generated file that holds their digests changes with them.

## Acceptance

Each item is one outcome, shown by a command and what it prints or by a test and what it
asserts. An item that is true on `main` and stays true is labelled a guard.

1. On a checkout of `a14e3b0` with the new files added to its index,
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
4. The check reads tracked files only: in a work tree whose only refused statement is in an
   untracked `crates/x/tests/late.rs`, and in another whose only one is in an untracked
   `crates/x/target/debug/tests/late.rs`, `sh taskfiles/test-clocks-check.sh --ledger /dev/null .`
   prints the line of item 3 and exits 0; after `git add` of the first file it exits 1 and
   names it. The case `untracked_file_not_read` holds the two trees and the case
   `untracked_file_added_refused` the first tree after `git add`. Turns red by: the file
   list taken from `find` instead of `git ls-files`.
5. The root must be the top of a git work tree: `sh taskfiles/test-clocks-check.sh --ledger
   /dev/null <dir>`, for a directory `<dir>` with an untracked `crates/x/tests/t.rs` and no
   `.git`, inside the repository's own work tree (the case `plain_directory_exit_2`), prints
   `test-clocks-check: <dir> is not the top of a git work tree` and exits 2; with the top of
   a work tree it does not exit 2. Turns red by: the test replaced by `git -C <root> rev-parse
   --is-inside-work-tree` (the plain directory then ends with exit 0 and the line `no
   elapsed-time bound`).
6. `git grep -n -E 'seconds < |total < Duration|recv_timeout' --
   crates/script/tests/adversary_4.rs crates/script/tests/adversary_5.rs
   crates/webidl/tests/depth.rs crates/html/tests/script_hook.rs` prints five lines at
   `a14e3b0` (`script_hook.rs:1750`, `adversary_4.rs:654`, `adversary_5.rs:241`, `:401`,
   `depth.rs:71`) and nothing on the changed tree. Turns red by: the `recv_timeout` of
   `script_hook.rs:1750` written back.
7. `git grep -n 'under 2 s' -- AGENTS.md` prints one line at `a14e3b0`
   (`AGENTS.md:1054:    `depth` also holds a model of 16,000 interfaces under 2 s).`, with the
   revision prefix when the revision is named) and nothing, exit 1, on the changed tree. Turns
   red by: the sentence of `:1054` left as it is.
8. `git grep -c 'test-clocks-check' -- Taskfile.yml` prints nothing, exit 1, at `a14e3b0` and
   `Taskfile.yml:2` on the changed tree (the two steps; the include names the file
   `test-clocks.yml`, which the string does not match), and
   `git grep -n 'taskfiles/test-clocks.yml' -- Taskfile.yml` prints nothing at `a14e3b0` and
   one line, the include, on the changed tree. Turns red by: the second step left out (the
   count is 1).
9. `AGENTS.md` names the check in the places of the Change. `git grep -o -F test-clocks --
   AGENTS.md | wc -l` prints 0 at `a14e3b0` and 15 on the changed tree (the 15 occurrences in
   the text of the Change; a line count would change with a rewrap, an occurrence count does
   not), and each place is shown by one `git grep -n -F` of one phrase, which prints nothing
   at `a14e3b0` and one line on the changed tree: `'taskfiles/test-clocks.yml'` (the list
   `taskfiles/<name>.yml`), ``'it runs `test-clocks-check` after'`` (the paragraph of `task
   check`), ``'- `task test-clocks-check`: '`` and ``'- `task test-clocks-check-test`: '`` (the
   two entries) and `'refuses the assertion of an upper bound'` (the entry of `task test`; the
   place `:1054` is item 7). Turns red by: the entry of `task test` left as it is (its phrase
   prints nothing and the count is 14).
10. `task test-clocks-check-test` decides each case of decision 8 as written: it prints one
    line `case <name>: ok` per case and ends with `test-clocks-check-test: all cases decided as
    written`, exit 0. Turns red by: `SCRIPT=<copy>` with the upper-bound pattern weakened to
    refuse only `<`; it prints each case decided otherwise (among them `less_or_equal_refused`
    and `limit_at_least_name_refused`) and fails.
11. The same task prints, for each of `awk`, `mawk`, `gawk`, `gawk --posix` and `busybox awk`,
    whether it ran the cases (`ran under <name>` or `not on this machine`); every one that ran
    decides all cases as written. Turns red by: a `gensub(` written into the script (it fails
    under `gawk --posix`, where `gensub` is undefined).
12. The header of `taskfiles/test-clocks-check.sh` (its lines that start with `#`) names every
    forbidden feature of decision 8, and the rest of the script uses none of them. For each of
    `nextfile`, `length(array)`, `IGNORECASE`, `gensub`, `asort`, `asorti`, `strftime`, `\<`,
    `\>` and `{n,m}`, `sed -n '/^#/p' taskfiles/test-clocks-check.sh | grep -c -w -F -e
    '<name>'` prints a number above 0 (`-w`: a header that names only `asorti` prints 0 for
    `asort`); and
    `sed '/^[[:space:]]*#/d' taskfiles/test-clocks-check.sh | grep -c -E -e
    'nextfile|IGNORECASE|gensub' -e 'asorti?|strftime|\\[<>]' -e
    '[^$]\{([0-9]+(,[0-9]*)?|,[0-9]+)\}|length\(arr_[A-Za-z0-9_]*\)'` prints 0 (the `sed`
    drops the header and every comment line, an indented one too; the interval pattern wants
    a character other than `$` before the brace and digits inside it, so `${1}` and `${10}`
    of sh, an action block `{ n++ }`, an empty block `{}` after a space or `)`, `a[1,2]` and
    `length(arr_lines[n])` of a string do not match, and `a{3}`, `a{,3}`, `a{3,}` and
    `a{1,2}` do). Both commands were run on a sample script with those lines: the first
    printed 1 for each of the ten names, the second 0; with a header that left out `asort`,
    the first printed 0 for it, and with `a{3}`, `a{,3}`, `a{3,}`, `a{1,2}` or `length(arr_x)`
    written in, the second printed 1. An interval at the very start of a line is not found by
    it; the forge run of item 13 is the catch for that form and for any other the pattern
    misses. Turns red by: a regular expression `/a{1,2}/` written into the script.
13. The forge's run of `task check` on the branch shows the step `test-clocks-check` and
    `test-clocks-check-test` green under the forge's `awk` (the report names the run).
14. Guard: `adversary_5.rs:373` asserts `G.rounds` (`rounds[2] * 2 > rounds[1]`) and the three
    ends; true on `main` at `:393` and `:398`, and the change only removes the time assertion
    (item 6 shows the removal). Red with the guard of `dom_exception_code`
    (`bindings.rs:1619-1621`) removed. That text is generated: `bindings.rs` holds it from the
    constant `PRELUDE` of `crates/webidl/src/prelude.rs`, which `generate.rs` writes after the
    header (`generate.rs:1753 @ a14e3b0`). The defect is made by hand in
    `crates/script/src/bindings.rs` alone; the story asks for no regeneration with a defect
    in it. While it is in place `slot task webidl-check` is red (item 30), so it is taken back
    before any step of the gate runs: the regenerated file is `git add`ed first, and `git
    diff --quiet -- crates/script/src/bindings.rs` exits 0 after the defect is taken back.
15. `adversary_5.rs:231` asserts the steps of its three scripts at the ceilings; red with
    `k < 4000`, and with the code-side defect of decision 2 when its steps pass the margin (the
    report gives the steps it adds either way). The code-side defect (`dom_exception_message`,
    `bindings.rs:1605-1608`) is made in `bindings.rs` alone and taken back as in item 14.
16. `adversary_4.rs:641` asserts `Limit(Memory)` or `Ran` with `mib < 4`; red with
    `ExceptionData` owning uncharged copies of its strings, made in `bindings.rs` alone (the
    struct at `:315-318`, its construction at `:666`) and taken back as in item 14.
17. `adversary_4.rs:641` fails when the loop ends early by another limit: with
    `small.engine.steps_per_entry` lowered so that the script ends `Limit(StepsPerScript)`, the
    test passes at `a14e3b0` and fails on the changed tree, which shows the `Ran` conjunct.
18. `depth.rs` asserts that the `definition_visits` of `Model::new` over 16,000 interfaces is
    at most 3 times that over 8,000; red with the counted scan in `definition_of`.
19. `script_hook.rs:1740` asserts `Some(Limit::Steps)` and `max_steps < steps < max_steps + 4 *
    max_nodes` from `Limits::default()`; red on the lower side with the entries read again
    charging no step (`tree_builder.rs:1199`), shown once in one pool job.
20. `script_hook.rs:1740` is red on the upper side with `spent` (`tree_builder.rs:1035`) and
    the check at `:687` both compared with `2 * max_steps`, shown once; and green with either
    one alone, which the report states from one run each.
21. Each red of items 14 to 20 (the defects the items name) is shown once before the change is
    kept, and the report names the output of each.
22. Record, not a check: the test step of `surf-script`, `surf-webidl` and `surf-html`, run
    alone (`slot cargo nextest run -p surf-script -p surf-webidl -p surf-html --locked`) and
    again while another build runs in the pool; the report lists both outcomes and the load
    average read beside each, and claims no particular load.
23. No test over 60 s and the test step within 120 s; a test over 5 s is named with its time
    (the three script tests, the new `depth` test and the `script_hook` test by their printed
    times).
24. No pool job over 10 minutes unless it names its time with `slot --timeout` (at most 30
    minutes), and none started again unchanged after `timed_out`.
25. Every command that compiles or tests carries `slot` (`cargo` and `go` through the shim).
26. No command of the story or of its report carries `--local`, `--here`, `SLOT_NO_REMOTE` or
    `SLOT_SHIM_OFF`: `git grep -n -E -e '--local|--here|SLOT_NO_REMOTE|SLOT_SHIM_OFF' --
    taskfiles/test-clocks.yml taskfiles/test-clocks-check.sh` prints nothing, and the list of
    commands in the report holds none of the four.
27. No Python in anything that stays; the check is POSIX sh and POSIX awk.
28. A script of the story that measures or waits (the two loaded runs of item 22, a timing of
    the check) starts without `slot`; the run time of `task test-clocks-check` is printed in
    the report, and a step over 5 s is named.
29. No test of the story makes a model call or a network request, and a limit on a wait in a
    test the story writes is at least 10 times the event alone.
30. The generated bindings follow the edit of `crates/webidl/src`. `slot task webidl-check`
    (a step of `check`, `Taskfile.yml:570 @ a14e3b0`) exits non-zero after the edit of `src/`
    alone, once shown with its message line `webidl-check: crates/script/src/bindings.rs is
    older than the sources above; run cargo run -p surf-webidl --bin bindings`, and exits 0
    after the file is written again by the command of the Change; the report shows `git diff
    --stat` and the count of changed lines of that file (6), and names any other line that
    differs before anything else.
    `slot task webidl-slow` is the lane `slow`'s step, never a step of `check`: no workflow
    runs the lane (`git grep -n -i slow a14e3b0 -- .github` prints nothing) and no list of paths
    starts it (`taskfiles/webidl.yml`, the summary of `webidl-slow`; `AGENTS.md:1739`), so a
    person runs it by hand (`AGENTS.md:1747`). It must be green for this change: it prints
    `webidl-slow: crates/script/src/bindings.rs is what the generator writes, byte for byte`.
    Turns red by: the file left as the generator wrote it before the edit (the first command
    fails with the message above).

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
- `crates/script/src/bindings.rs`, generated, holds the sha256 of every file of
  `crates/webidl/src` in its header, so a change of any of them writes it again. The command
  above prints nine files: this story, `story:webidl-bindings` (implemented, history) and
  seven that each say they write the file again: `story:scripts-dom-attributes`,
  `story:scripts-dom-mutation`, `story:scripts-dom-selectors`, `story:scripts-parse-run` (all
  proposed), `story:scripts-dom-reading`, `story:swell-0-9-0` (both active) and
  `story:scripts-utf16-length` (draft). The rule for the file is not the one for text
  lists: the story that merges second writes the file again by the generator after the merge
  of `main`, and does not keep lines of the other's side (the digest lines differ on both).
  No edge.
- `crates/html/tests/script_hook.rs` is also named by `story:scripts-dom-attributes`
  (proposed), which adds tests of reads in the module where the write tests are (`:1157-1274`);
  this story edits `:1735-1753`. No edge. `story:html-script-hook` (implemented) is history.
- `Taskfile.yml`: `story:agent-cookie-public-suffixes` (active) adds an include and a step to
  `check`, and this story adds an include and two steps, in the same two lists. No edge: the
  story that merges second keeps both sides of those two lists in its merge of `main`; the
  generated `bindings.rs` is not such a list (next bullet).
  `story:conformance-lock-and-lints` (draft) edits the lines of `lock-check`, five lines above
  `doc-tests-check` in `check`; `story:test-step-holds-with-tls` (active) edits the lines of
  the test steps. No edge.
- `AGENTS.md` is in the scope of eleven other stories not yet implemented:
  `story:agent-tasks-left-process-wait`, `story:conformance-lock-and-lints`,
  `story:scripts-dom-attributes`, `story:scripts-dom-mutation`, `story:scripts-dom-reading`,
  `story:scripts-dom-selectors`, `story:scripts-parse-run`, `story:session-argument-checks`,
  `story:swell-0-9-0`, `story:test-step-holds-with-tls`, `story:tls-tests-as-tables`. Read
  with `git grep -n -E 'AGENTS|taskfiles/\*|:126-145' -- <story>` over each: none edits
  `:1054`, `:1965-1967`, the `check` paragraph or the entries of the tasks (`:1826-1847`).
  One edits the list `taskfiles/<name>.yml` that this story adds a clause to (`:137-170`):
  `story:test-step-holds-with-tls` (its statement on `taskfiles/*`, at `:126-145` of its own
  base). No edge: the clauses are different lines of one list, and the story that merges
  second takes the other's lines in its merge of `main`. No other of the eleven names that
  list or an entry of a task.
- `story:time-bounds-in-tests` (draft) has a `depends_on` edge to this story (made at plan
  commit `8c56b6c`), takes over the ledger, `taskfiles/test-clocks-ledger.txt`, and needs the
  scanner's output (the lines the scan prints against its list of 12 assertions in 7 files).
  Its text is not changed here.
- A story that removes a bound from a file the ledger lists lowers that file's ledger line in
  its own change, since the check fails on a file with fewer statements than its line.
  `story:agent-tasks-left-process-wait` changes `crates/agent-tasks/tests/bounded.rs`
  (`running` and `child_left`, `:41-64 @ a14e3b0`), but the one ledger statement of that file,
  the assertion at `:97` (`took < Duration::from_secs(4)`), is outside those lines and stays.
  No ledger line changes, and no edge.
- `story:tls-tests-as-tables` (draft) claims whole directories (`crates/tls/tests/tls`,
  `crates/tls-test-server/tests`, `crates/tls-test-certs/tests`,
  `crates/tls-conformance/tests`) and turns groups of tests, some across files
  (`x509_parse.rs` and `adversary_x509.rs` into one table test), into table tests: its scope
  and its statement 2 were read. Files of those directories can be ledgered
  (`crates/tls-test-server/tests/adversary_request_body.rs:119-122` is one statement of
  decision 8). When that story renames or deletes a ledgered file, or moves a refused
  statement to another file, it moves or deletes the ledger line in the same change: the check
  fails on a line for a file it does not read. No edge.
- `story:html-parser-hook-cost` (draft) edits `crates/html/src/tree_builder.rs` and `tree.rs`,
  the code the window of decision 5 reads. Its summary says it removes what can be removed
  without changing a step count or a limit; the window test is the guard of that promise.
  No edge.
- `crates/webidl/src/lib.rs` (as a source; `git grep -l -F` over the story files prints this
  story alone), `crates/script/tests/adversary_4.rs`, `crates/webidl/tests/depth.rs` and the
  new `taskfiles/` files are named by no other story. What follows for `lib.rs` is not that
  nobody else's change meets it: its digest line sits in `bindings.rs`, which the stories of
  the bullet above write again.

## As built

The unit is on `main` at `e4604fb` (the merge of `impl/tests-without-wall-time-bounds` at
`52ebbd7`). Against the two units that go to `main` before it (`ac1901f`) it changes 12 files,
1,677 lines added and 60 removed: exactly the 12 paths of the story's scope. Three are new
(`taskfiles/test-clocks-check.sh` 980 lines, `taskfiles/test-clocks.yml` 498,
`taskfiles/test-clocks-ledger.txt` 31). Two files under `src/` change
(`crates/webidl/src/model.rs`, `lib.rs`), and the generated `crates/script/src/bindings.rs`
with them (6 lines, all in its header).

- What was built.
  - The five tests. No time assertion is left in any of them; each prints its time.
    - `adversary_5.rs`, the code getter loop: keeps `rounds[2] * 2 > rounds[1]` and the three
      ends `Limit(StepsPerScript)`. Red with the guard of `dom_exception_code` switched off
      (`rounds 23255,30302,1249`). Does not see: a copy that is made and not charged.
    - `adversary_5.rs`, `strings_only_exceptions_hold_survive_collections_inside_and_between_
      scripts`: `runner.steps()` before and after each of the three scripts, asserted at
      `STEPS_CEILINGS` = 6,623,919, 3,501,804 and 3,501,804, 1.5 times the first green steps
      (4,415,946, 2,334,536, 2,334,536). The four `collect` calls charge no step and are not
      bounded.
    - `adversary_4.rs`, `a_dom_exception_holds_no_uncharged_copy`: the second arm is now
      `end == ScriptEnd::Ran && mib < 4`; the doc comment says that `ExceptionData` holds two
      `StringRef`s and copies nothing.
    - `depth.rs`: `surf_webidl::definition_visits()` (a thread-local counter behind
      `#[doc(hidden)] pub fn`, every lookup of a definition by index through the one accessor
      `definition_at`). The test
      `the_definitions_a_model_visits_double_when_its_interfaces_double` builds a model of 8,000
      and one of 16,000 interfaces and asserts `half > 0` and `whole <= 3 * half`; it visits
      40,000 and 80,000. The scan per statement it was written against visits 4 times as many.
    - `script_hook.rs`, `a_hook_that_keeps_every_element_open_ends_at_the_step_limit`: the parse
      runs on the test's own thread (no spawn, no channel, no `recv_timeout`) and the test
      asserts `Some(Limit::Steps)` and `max_steps < steps < max_steps + 4 * max_nodes` of
      `Limits::default()`. The green run ends at 134,217,729 steps.
  - The check, `sh taskfiles/test-clocks-check.sh [--count] [--ledger <file>] [<root>]`, with
    the tasks `test-clocks-check` and `test-clocks-check-test` (`taskfiles/test-clocks.yml`),
    two steps of `check` in `Taskfile.yml` and five places of `AGENTS.md` (16 occurrences of
    `test-clocks`). POSIX sh and POSIX awk only; its header names the ten features it never
    uses and the rest of the script uses none.
    - Reads the tracked `*.rs` files under a `tests/` directory of `crates/`, `generated/` and
      `conformance/` (`git ls-files`), with comments, strings, raw strings and character
      literals blanked. A root that is not the top of a git work tree exits 2.
    - Refuses: an `assert!` form with an upper bound on an elapsed value (`.elapsed()`,
      `.duration_since(..)` and its `saturating_` and `checked_` forms, or a name bound from
      one), on a clock read (`Instant::now() < deadline`) or a lower bound on the time left;
      an `if` or `while` on such a value whose block fails the test; a downward `if` whose
      `else` block fails it; a match guard that compares upward with a failing arm. Prints
      `<path>:<line>`, exit 1.
    - Four rounds of name tracking: a name is tracked when the value of its binding holds an
      elapsed value (round 1) or a name of rounds 1 to 3 (rounds 2 to 4). Bindings read: `let`,
      `if let`, `while let`, a `for` pattern, `NAME =`, `NAME +=`, the pattern of a match arm.
      Of a pattern that binds several names only those holding `took`, `elapsed`, `seconds`,
      `secs`, `waited`, `duration` or `time` are tracked. The fourth round came last
      (`f72a740`); the first three came with the check, widened in the first fix round.
    - The ledger `taskfiles/test-clocks-ledger.txt` (`<path> <count>`, sorted by path, no
      comment) lists the files that still hold refused statements. A file with more fails and
      names each statement; a file with fewer fails and names the count to write, so the ledger
      only shrinks; a line for a file the scan did not read fails (`delete the line`); a line
      out of order fails, naming the first.
    - 96 cases in `taskfiles/test-clocks.yml`, each a small tree made the top of a git work
      tree, run under each of `awk`, `mawk`, `gawk`, `gawk --posix` and `busybox awk` that the
      machine has. A case is held by its own defect: the reviews and fix rounds wrote a defect
      into a copy of the script (`SCRIPT=<copy>`) for each rule and want a case that turns red
      under it, one that the other cases of the rule do not already supply; the first fix
      round ran 17 such defects, the second review 25, the second fix round 9.
- What differs from the story. Every decision, Change line and acceptance item not named here
  was built as written. 22 deviations.
  - From the build (first report):
    1. Acceptance 17 cannot be shown as written. The first script of the test takes 262,240
       steps and the loop of 64 constructions 1,936, so every `steps_per_entry` under 262,240
       ends the first `ran` before the loop starts, and none ends the loop by
       `Limit(StepsPerScript)` with the first script `Ran`. Shown instead: a loop that throws
       at `i == 8`. The old arm (`|| mib < 4`) passes it, the new arm fails it (`threw value
       1:46; the process grew by 0 MiB`). The first review recomputed both counts.
    2. Acceptance 14: removing the guard of `dom_exception_code` fails the build under
       `-D warnings` (`constant LEGACY_NAME_UNITS is never used`), so the guard was switched
       off with `if false && ..`.
    3. Acceptance 16: `ExceptionData` is `Copy`, so the defect added a field
       `_owned: (&'static str, &'static str)` filled at the construction by `to_utf16` and
       `String::from_utf16_lossy`, leaked, no charge. Red: `the process grew by 16 MiB for
       16 MiB of messages`.
    4. Acceptance 15: `k < 4000` turns the test red by `ran` (the 200,000,000 steps of an
       entry), not by a ceiling. `k < 400` was added: `MAKE charged 10172346 steps, over its
       ceiling of 6623919`. The code-side defect adds 63 steps per `READ` (steps 4,415,946,
       2,334,599, 2,334,599) and stays inside the margin: it passes, so it is one of "Does
       not see", as the story allows. Whether an allocation of `churn` charges a step was not
       separated.
    5. The `depth` test asserts `half > 0` too, because a zero count would pass
       `whole <= 3 * half`.
    6. The entry of `task test-clocks-check` in `AGENTS.md` is the story's text rewrapped,
       plus one sentence on a ledger line for a file the scan did not read.
    7. Acceptance 11 is shown in part: the cases ran under `awk`, `gawk` and `gawk --posix`;
       `mawk` and `busybox awk` are not on the machine and never ran. Acceptance 13 (the
       forge's run of `task check`) is not shown here.
  - The 13 points where the story's text is behind the script (second review):
    8. Comparisons. The story refuses an elapsed value as the smaller operand; the script also
       refuses a clock read as the smaller operand and the time left as the larger one. The
       first review found four poll-loop asserts `assert!(Instant::now() < deadline, ..)`
       (`crates/view-fixture/tests/fixture.rs:174` and `:318`,
       `crates/tls-test-server/tests/hostile.rs:46`,
       `crates/surfd/tests/adversary.rs:193`) that were neither refused nor ledgered, which
       made the `AGENTS.md` sentence false.
    9. Elapsed value. The story names `.elapsed()`, `.duration_since(..)`,
       `.saturating_duration_since(..)` and a chain. The script adds `.checked_duration_since(..)`,
       a cast after the value, an operand behind `*`, `&`, `&mut` or parentheses, and decides
       `a.duration_since(b)` by its sides: no elapsed value when `b` is `UNIX_EPOCH` or both
       sides are fixed instants, the time left when `b` is a clock read, else an elapsed value.
       Each form passed in the first review; a Unix timestamp and a difference of two fixed
       instants were refused with no clock in them.
    10. Bindings. The story reads names bound by `let`, and the word rule for tuples only. The
        script binds by `if let`, `while let`, a `for` pattern, `NAME =`, `NAME +=` and match
        arms, and applies the word rule to every pattern. The first review found
        `let took; took = ..`, `total += ..` and `if let` untracked; the second a `for`
        pattern.
    11. The `if` rule. The story compares against a `Duration`, a number or an upper-case
        constant; the script against any operand that is not of the elapsed value's kind. An
        `if s.elapsed() > limit { panic!() }` with a lower-case `limit` passed.
    12. Downward conditions. The story does not read them. The script refuses a downward `if`
        whose `else` block fails the test and a match guard that compares upward with a
        failing arm; both passed in the first review.
    13. The story reads `<`, `<=`, `>`, `>=`; the script also reads `.lt`, `.le`, `.gt`, `.ge`.
    14. Allowed. The script adds an upper bound on the time left
        (`deadline.saturating_duration_since(Instant::now()) < WAIT`).
    15. Not seen. The script adds: a clock read through an alias or a name; a name bound from
        the time left; a failing `else if` branch; a downward guard with a failing arm; and,
        from the second review and its fix round, an expression around an elapsed value
        (`took + SLACK`), a comparison through `cmp`, `partial_cmp`, `min` or `max`, a
        destructuring assignment, a value assigned through a reference or to a field, the time
        left with a name or a parameter as argument, a bound between two Unix timestamps, and
        the names of a pattern of several names none of which holds a word of the list.
    16. Ledger. The story says sorted and states no failure; the script fails on a line out of
        order and names the first (the first review saw an unsorted ledger pass).
    17. The pre-scan. The story has none. The script reads a file further only when a line
        holds `elapsed` or `duration_since`, or the word `now` in a file that also holds
        `Instant` or `SystemTime`. The first review found a file skipped whole when `.elapsed
        ()` has a space or a line feed before the parenthesis. The word `now` came with the
        clock-read rule and admitted 110 files (54,776 lines); the condition on `Instant` or
        `SystemTime` brings it to 68 files (30,279 lines) with no decision changed.
    18. Cases. The story lists 50 by name; the file holds 96.
    19. Acceptance 9: `git grep -o -F test-clocks -- AGENTS.md | wc -l` prints 16, not 15,
        because the sentence added to the entry of `task test` names
        `taskfiles/test-clocks-check.sh`. Left as it is, that entry holds 2 of the 16, so the
        count without it is 14, as the story says.
    20. Acceptance 1 and 2. With the script as built the scan of `a14e3b0` with an empty
        ledger prints the six deadline statements too; the ledger holds 30 lines and 46
        statements after the first fix round (25 and 40 at `996c05c`), 47 statements after the
        fourth round of names, and 31 lines and 48 statements at the merge.
  - Two more:
    21. The `task test` entry of `AGENTS.md` does not say that the check refuses every upper
        bound outside its ledger, which the forms it does not see would make false. It says
        "in the forms the header of `taskfiles/test-clocks-check.sh` lists as refused, that
        its ledger does not list; the same header names the forms it does not see". Likewise
        the entry of `task test-clocks-check` ends its list of bindings with "in the forms the
        header of the script lists".
    22. Four rounds of names; the story states three and a fourth round as not seen. A bound
        at `crates/surfd/tests/adversary_2.rs:232` is reached through four bindings by name,
        so the fourth round was built (see the second review).
  - The forms the check does not read, as its header lists them: a bound written through a
    helper function, a macro or a `bool` of a `let`; a name bound through a name of round 4;
    the names of a pattern that binds several names of which none holds a word of the list;
    the negated spelling `assert!(!(took >= LIMIT))`; `assert_eq!(took, ..)` by equality; a bound
    recorded in a list asserted after the loop; a test that returns `Err` from the block; a
    failure by `.expect`, `.unwrap` or `std::process::exit` in the block; a difference of two
    instants written with `-`; a clock read through an alias or a name; a name bound from the
    time left; a failing `else if` branch; a match guard that compares downward with a failing
    arm; an expression around an elapsed value; a comparison through `cmp`, `partial_cmp`, `min`
    or `max`; a name bound by a destructuring assignment, through a reference or to a field; the
    time left whose argument is a name or a parameter; a bound between two Unix timestamps;
    whether the limit of a wait is at least 10 times the time alone.
- The two reviews. Both ended NEEDS-CHANGE. Lines of other tests are those of the reviewed
  commit; at `52ebbd7` the bound of `adversary_2.rs:232` is at `:292`, the deadline assert of
  `crates/surfd/tests/adversary.rs:193` at `:196` and the heap bound of `bindings.rs:693` at
  `:722`.
  - First review (on `996c05c`; origin all introduced). Report: 11 findings, 2 must,
    4 should, 5 notes; its findings block holds 7 (2 blockers, 4 warnings, 1 note).
    - Must: the pre-scan skipped a file whose `.elapsed()` has a space or line feed before the
      parenthesis; the `AGENTS.md` sentence was false, four deadline asserts being neither
      refused nor ledgered.
    - Should: operand forms around an elapsed value pass (`checked_duration_since`, a cast,
      parentheses, a dereference, a comparison method); names bound other than by `let NAME =`
      are not tracked; the `if` rule misses a lower-case limit, a failing `else` and a match
      guard; `duration_since` is an elapsed value whichever side reads the clock.
    - Notes: gawk's syntax error ends with exit 1 and no line of the script's own; edges of
      the ledger (a path with a space, an unsorted ledger passes, a renamed file); the window
      test alone does not hold that every entry read again is a step (one step per four
      entries stays green, the test `each_entry_read_again_after_a_hook_run_is_a_step` goes
      red); `DEFINITION_VISITS` is compiled in every build and no crate depends on
      `surf-webidl`; the first deviation above, checked and confirmed.
    - 23 cases written (50 to 73): 19 red on `996c05c`, 4 green. Two hand-made defects in the
      code under test, taken back: `definition_at` without its count turns `depth` red
      (`no definition visited at 8000 interfaces`); a `settle_entries` that charges one step
      per four entries leaves `a_hook_that_keeps_every_element_open_ends_at_the_step_limit`
      green.
  - Fix round 1 (`af3bc95`, `89cf1fd`, `0398d9a`): every finding repaired (deviations 8 to 17);
    73 to 77 cases, 19 red to none. The scan then found six new statements, the four
    deadline asserts and `crates/surf-cli/tests/cli.rs:99` and
    `crates/surf-cli/tests/review1_output.rs:65` (the second was not in the review's list);
    the ledger went from 25 lines and 40 statements to 30 and 46. Two cases were added for
    the pre-scan beyond the review's: the first defect left the review's two cases green
    because every file that reads a clock holds the word `now`.
  - Second review (on `0398d9a`; origin 7 introduced, 1 pre-existing). Report: 12 findings,
    2 must, 2 should, 8 notes; its block holds 8 (2 blockers, 2 warnings, 4 notes).
    - Must: `crates/surfd/tests/adversary_2.rs:232`, `assert!(longest < KILL_GRACE_OF_SURFD * 2,
      ..)`, an upper bound on measured times that the check passed and the ledger did not
      count (pre-existing; the times pass through a tuple `let` of two unworded names and a
      `for` pattern); eleven rules that the round added or the header states were held by no
      case.
    - Should: the word `now` admitted 42 files (24,497 lines) that hold neither `Instant` nor
      `SystemTime`, doubling the main scan; the `AGENTS.md` clause on names bound had no
      limit.
    - Notes: operand expressions, `cmp` and `min`, bindings not read, the time left with a
      name, two Unix timestamps, a clock read against an instant of the product, helper
      bounds (`crates/test-process/tests/children.rs:70`, `:79`, `:135`,
      `crates/webidl/tests/adversary_3.rs:250`), one equivalent defect.
    - 12 cases written (77 to 89): 1 red (the `for` pattern), 11 green and each held by a
      defect in a copy of the script.
  - Fix round 2: merge of the review's cases (`aa7915d`); `162f64b` reads a `for` pattern as a
    binding, narrows the pre-scan, limits the `AGENTS.md` clause and extends "Not seen" (6
    cases, 89 to 95); `f72a740` adds the fourth round (cases 95 to 96). The bound at
    `adversary_2.rs:232` is the one the check did not see and now refuses: `opens` (round 1),
    `answers` (2), `took` (3), `longest` (4). The only new line of the tree's scan is that
    one; the ledger line of `crates/surfd/tests/adversary_2.rs` goes from 1 to 2 (47
    statements). The review's case was reshaped to the tree's statement (a binding inside the
    closure was added). `fourth_round_of_rebinding_refused` is new,
    `fourth_round_of_rebinding_not_seen` became `fifth_round_of_rebinding_not_seen`, and the
    case of the unworded pattern became
    `pattern_of_unworded_names_from_an_elapsed_value_not_seen` (it wants exit 0).
  - Merge of the two units that go to `main` before it (`7e9dc11`, parents `f72a740` and
    `ac1901f`): two conflicts, the `crates/webidl` paragraph of `AGENTS.md` (the other side's
    test count kept, this side's last clause kept) and the whole-file digest in the header of
    `bindings.rs` (written again by the generator, not by hand). The check then refused
    `crates/script/tests/adversary_dom_reading.rs:245`, `assert!(took < Duration::from_secs(1),
    ..)`, a bound that `main` holds; `52ebbd7` ledgers it. The four files of the five tests are
    not touched by the merge.
- Numbers.
  - Cases of `test-clocks-check-test`: 50 at `996c05c`, 73 after the first review, 77 after
    the first fix round, 89 with the second review's, 95 and 96 after the second fix round.
  - Ledger: 25 lines and 40 statements at `996c05c` (the scan of `a14e3b0` printed 44 lines
    with an empty ledger, 4 of them the story's), 30 and 46, then 30 and 47, then 31 and 48 at
    `52ebbd7`.
  - Merge report, `slot cargo nextest run -p <crate> --locked`: `surf-html` 253 passed in
    5.396 s; `surf-script` 229 passed, 3 skipped, 38.015 s; `surfd` 98 passed, 5.121 s;
    `surf-webidl` 131 passed, 1.446 s. `sh taskfiles/test-layout.sh .`: 26 test directories,
    246 test files, one test binary each. The workspace build under `-D warnings`: exit 0.
  - The three crates of the five tests, first report: 455 passed, 3 skipped in 8.236 s, and in
    17.135 s with another build running.
  - The step `test-clocks-check`: 733 ms under `awk` and 661 ms under `gawk --posix` at
    `996c05c`; 6,485, 4,126 and 3,348 ms at `0398d9a`, after the pre-scan grew (110 files
    admitted in place of 48); 1,402, 1,364 and 1,385 ms at `f72a740`. The merge report gives
    none for its tip. All run alone, without `slot`.
  - The step `test-clocks-check-test`: 6,749 ms for 73 cases (first review), 8,450 ms for 89
    (second review), 20,091 ms for 96 at `f72a740`, 12,873 ms for 96 at the merge. All over
    5 s, none over 60 s; the step is under 120 s.
  - The five tests (first review): `script_hook` 2.163 s, `adversary_4` 0.112 s,
    `strings_only` 2.516 s, `depth` 0.546 s, the code getter loop 6.195 s. Over 5 s: the code
    getter loop (5.192 to 11.802 s in the first report, 5.835 s in the merge report) and
    `script_hook` (5.995 s). No test over 60 s.
- Left as it is.
  - A rule that tracks every name of a pattern with no worded name was written and not taken:
    it refused a bound on heap bytes, `crates/script/tests/bindings.rs:693`
    (`assert!(heap <= BINDINGS_REALM_HEAP_BYTES, ..)`), and no narrowing separated the two.
    The fourth round took its place.
  - The window test of `script_hook.rs` does not see a step charged at less than one per
    entry; the test `each_entry_read_again_after_a_hook_run_is_a_step` does.
  - The `depth` test sees a scan only when it passes `definition_at`.
  - The count of `adversary_5.rs:231` does not see the four `collect` calls or a defect that
    stays inside the 50 % margin of its ceilings.
  - gawk's syntax error ends the check with exit 1 and no line of its own; a ledger path with
    a space cannot be written; the message for a renamed ledgered file says `delete the line`.
  - The pre-scan rule of the word `now` alone, put back, changes no decision and costs time
    only; a defect in the direction of `later.duration_since(Instant::now())` changes no case,
    and no test would write that form.
  - `README.md` and `website/docs/how-it-is-built.md` show sample gate lines of
    `doc-tests`; none for `test-clocks` was added.
  - One job of the first report ended with client exit 0 while nextest printed `test run
    failed`.
- A later story takes up: `story:time-bounds-in-tests` (the ledger, 31 lines and 48
  statements, and the helper bounds above).
