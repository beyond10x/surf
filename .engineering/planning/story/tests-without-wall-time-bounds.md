---
format: aep.planning-md/3
id: story:tests-without-wall-time-bounds
kind: story
status: draft
title: Replace the wall-time bounds of four tests by counts
summary: Four tests of main compare their own elapsed time with a constant, against the repository's rule on clocks in tests; two went red under load with every count right. This draft replaces each bound by the count that proves the cost.
relations:
- decomposes: epic:scripts-in-a-page
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: crates/script/tests/adversary_4.rs
- confidence: inferred
  path: crates/script/tests/adversary_5.rs
- confidence: inferred
  path: crates/webidl/tests/depth.rs
revision: 2
---
## Context

The repository's rule on a test that reads a clock (`AGENTS.md:1965-1967 @ a14e3b0`): "A test
bounds a time only by a wait for the event with a limit at least 10 times the time it takes
alone, and proves a cost by a count (such as `surf_page::attribute_visits()`), not by a wall
time."

Tests of `main` at `a14e3b0` break it. Each asserts an upper bound on its own wall time, and two
of them went red on a loaded machine while every count they hold was right:

- `crates/script/tests/adversary_5.rs:373`,
  `the_code_getter_in_a_loop_copies_at_most_a_legacy_name_and_ends_at_the_step_limit`, bound
  20 s (`:401`). Alone, three runs: 2.839 s, 2.618 s, 2.624 s, so the bound is 7.0 to 7.6 times
  the time alone. Red at 20.611 s (load average 117 to 134) and at 29.206 s (load average 80 to
  110). It already asserts the counts that carry its claim (the rounds of the loop).
- `crates/webidl/tests/depth.rs:55`, `a_model_of_16000_interfaces_is_built_within_2_s`, bound
  2 s. Alone, three runs: 0.198 s, 0.184 s, 0.186 s, so the bound is 10.1 to 10.9 times the time
  alone. Red at 2.380 s (load average 5 rising to 77) and at 4.027 s (load average 117 to 134).
  Its name states a wall time as its claim.
- `crates/script/tests/adversary_5.rs:241` (bound 20 s) and
  `crates/script/tests/adversary_4.rs:654` (bound 30 s): the same form, not measured.

The times alone were measured on the build node on 2026-11-25, the red runs on the workstation
the same day, by the unit of `story:scripts-dom-reading` (its fix round 5), which changed none
of these tests because they are on `main`.

A red test with right counts stops a gate and a merge for a reason that is no defect of the
code. The folder's rule on budgets says a raised bound is not a fix.

## Decisions to take when the story is revised

1. For each of the four tests: what cost the bound protects, and the count that proves it
   (steps charged, rounds of a loop, nodes or interfaces visited, allocations counted). Where
   the test holds the count already, the time assertion goes and the time is printed. Where it
   does not, the count is added first and shown red with the defect put back by hand.
2. Whether any of the four needs a clock at all. A wait for an event with a limit of at least
   10 times the time alone stays allowed by the rule; an elapsed time compared with a constant
   after the work is not that form.
3. The name of the `depth` test: a name that states the count it proves, with its module's
   other names read first.
4. A check that keeps the rule: a source check in the gate that finds a comparison of an
   elapsed time with a literal in a file under any `tests/` directory (the spellings to refuse,
   read from the four places and from the 27 lines that hold `Instant::now` or `elapsed()` under
   `crates/script/tests` and `crates/webidl/tests @ a14e3b0`), written in POSIX sh or Rust, with
   a case for both sides. Or the sentence that says why no such check is added.

## Change

- `crates/script/tests/adversary_5.rs`, `crates/script/tests/adversary_4.rs`,
  `crates/webidl/tests/depth.rs`: the four time assertions replaced as decision 1 gives.
- `AGENTS.md`: the entry of the test step names the check of decision 4 if one is added.
- `Taskfile.yml` and the check's file, only if decision 4 adds a check.

No file under any `src/` changes.

## Acceptance

Draft. To be written when the story is revised; it holds at least:

1. No test of the three files asserts a comparison of an elapsed time with a constant
   (`git grep` with the spellings of decision 4 prints nothing for them).
2. For each of the four tests: the count it asserts, and the hand-made change of the code that
   turns it red.
3. The test step of the three crates run twice, once alone and once beside another build of
   the same tree's crates: the same tests pass in both, within the budget of the folder (no test
   over 60 s, the step within 120 s, a test over 5 s named with its time).

## Placement

No edge. The four tests are in files the DOM slices do not edit. `crates/script/tests/main.rs`
is not changed. The unit of `story:scripts-dom-reading` adds modules beside them and changes
none of the three files.
