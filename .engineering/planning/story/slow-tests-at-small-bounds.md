---
format: aep.planning-md/3
id: story:slow-tests-at-small-bounds
kind: story
status: archived
title: The slow tests run at small bounds and the conformance tests end at 60 s
summary: Seven old tests take 5 s to 11 s unloaded and up to 48 s under load; they set their deadline or bound small, and the conformance runner's tests run under cargo-nextest
relations:
- decomposes: epic:headless-rust-target
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: conformance/tests
- confidence: cited
  path: crates/session/tests
- confidence: cited
  path: crates/surf-agent/tests
- confidence: cited
  path: crates/surf/tests
- confidence: cited
  path: view-tests/tests
revision: 8
transitions:
- {from: "draft", to: "archived", at: "2026-11-22T19:52:28Z", actor: "human:operator", revision: 8, decided_on: {"recorded":{"verification":1}}}
---
## Why

The gate's test step has a budget of 120 s and no test may run over 60 s. On a host with
little load the step takes 15 s to 50 s. On the shared host under load the same tests come
close to both bounds, and the step once failed on its time alone with every test passing
(129.8 s at a load average of 123, during `story:ess-0-97-3`).

The tests that take the time are few and old. Measured in round 1:

- The gate's ten slowest at a load average near 70 (875 tests, step 48.5 s):
  `surf-session::main built::request_slower_than_its_deadline_is_closed` 10.6 s,
  `surf::main escape::every_character_is_written_as_printable_ascii_and_reads_back` 7.8 s,
  `surf-session::main adversary_events::adversary_an_action_recorded_after_a_session_timeout_has_its_entries_kept`
  7.2 s, `surf::main review1_generated_check::review1_one_changed_byte_in_any_generated_file_fails_naming_it`
  5.6 s, `surf-agent::main start::surfd_that_writes_nothing_exits_3_within_6_seconds` 5.1 s,
  `surf-session::main events::case_08_a_page_of_6_mib_leaves_the_session_open` 5.1 s,
  `surf-session::main memory::bodies_of_answered_commands_are_not_kept` 4.8 s.
- The same tests at a load average of 110 to 130, run per crate: `events::case_08` 40.3 s,
  `memory::bodies_of_answered_commands_are_not_kept` 27.7 s and 47.8 s in one run,
  `escape::every_character…` 32.5 s, `review1_generated_check` 17.6 s, `adversary_view` 12.1 s,
  `built::request_slower…` 10.8 s.
- The conformance runner's own tests (`conformance/tests/main.rs`, 15 tests, run by
  `cargo test`, which ends no test at 60 s) took 56.8 s in one run.

The page tests bound a wall time from above too: 12 assertions in `view-tests/tests/` carry
`timeout: 2000` (6 in `first-page.spec.mjs`, 3 in `detail.spec.mjs`, 3 in
`adversary-detail.spec.mjs`). `first-page.spec.mjs` "latest follows" failed once on its first
such assertion (2 segments of 3 after 2 s) in a gate run whose load average rose to 110; the
same commit's other runs passed it in 1.3 s and less.

Three causes show in the names: a test waits for a real deadline of several seconds
(`request_slower_than_its_deadline`, `exits_3_within_6_seconds`, the session timeout); a test
of a bound runs at the product's full bound (a page of 6 MiB, 21 bodies of 256 KiB); a test
loops over a large set in one process (every character, every generated file).

## Change

- A test that waits for a deadline sets the deadline small through the seam the code already
  has for it, or gets one; no test sleeps for a product default.
- A test of a bound sets the bound small (the rule `AGENTS.md` states for limits); one test
  per bound may stay at the default when a comment says why, as the depth-limit test of the
  tree builder does.
- A test that loops over a large set does the expensive part once (one generation, one
  session) and reads it per case, or is split by a fixed partition so that no part is over
  5 s unloaded.
- The conformance runner's tests run under cargo-nextest with the same 60 s end as the root
  workspace, and their step's time is printed like the test step's.
- A page test that waits for the view to show an entry waits on the event it follows (the
  stream's message, the list's row) under the test's own timeout, and asserts a time
  only where a sentence of the documents states one, in one test with its reason.
- Each test keeps what it asserts; a test whose assertion needs the full size says so and is
  the listed exception.

## Acceptance

When the story is implemented:

1. On a host with a load average under 20 no test of the root workspace runs over 2 s, except
   the listed exceptions, each with its reason in a comment; the gate's ten slowest are in the
   report before and after.
2. Under a load average over 100 (measured once, the load named in the report) no test runs
   over 20 s and the test step ends within 90 s.
3. A test in `conformance/` that runs over 60 s is ended and counts as failed.
4. No test was removed or weakened: the list of test names before and after is in the report,
   and each test whose input size changed names the bound it now sets.
5. The count of tests and the gate's numbers are unchanged apart from tests that were split.
6. `task view-test` passes three runs in a row under a load average over 100, and the count of
   `timeout: 2000` assertions left is stated with the sentence each one proves.
