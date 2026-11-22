---
format: aep.planning-md/3
id: story:limit-tests-at-small-limits
kind: story
status: draft
title: Tests of a limit set a small limit; the slow tests of the gate get a cause
summary: Wave 10's three test files hold 15 to 28 % of the summed test time and add 6 to 9 s to the step; five older tests took 10 s or more in the gate of 5649f83
relations:
- decomposes: epic:headless-rust-target
- depends_on: story:one-test-binary-per-crate
scope:
- confidence: cited
  path: crates/html/tests/adversary_limits.rs
- confidence: cited
  path: crates/html/tests/limits.rs
- confidence: cited
  path: crates/page/tests/label.rs
revision: 4
---
## Context

The gate of wave 10 on `5649f83` (2026-11-22) took `test step: 156.3 s` on a loaded machine of 20 processors; the 659 tests nextest ran sum to 598.1 s there. Measured back to back afterwards on the same commit, nextest took 52.4 s and 45.9 s with the three test files wave 10 added and 43.1 s and 40.0 s without them (`story:test-step-holds-under-load`, Context). The three files add 6 to 9 s to the step and hold 15 to 28 % of its summed test time:

- in the gate: `surf-html::limits` 36 tests 92.4 s, `surf-page::label` 34 tests 32.3 s, `surf-html::adversary_limits` 11 tests 22.4 s;
- in the two runs after it: 48.1 s and 28.5 s, 14.6 s and 8.0 s, 10.2 s and 6.6 s.

Their slowest tests in the gate: `input_of_16_777_217_bytes_as_bytes_stops_at_the_input_limit` 12.1 s, `input_of_16_777_217_bytes_stops_at_the_input_limit` 11.0 s, `seeded_10_000_inputs_end_within_bounds` 10.9 s (9.0 s in a run after it), `node_1_048_577_stops_at_the_node_limit` 9.5 s (5.3 s), `past_the_label_work_bound_names_come_from_attributes_only` 9.4 s, `b_505_with_1_024_attributes_each_ends_within_bounds` 8.7 s, `cloned_attributes_past_64_mib_stop_at_the_stored_bytes_limit` 8.6 s, `tags_with_1_024_attributes_to_16_mib_end_within_bounds` (6.7 s in a run after it). They pay for input at the default limits (16 MiB of input, 1,048,576 nodes, 64 MiB of stored bytes, 16,777,216 steps of label work) in a build without optimisation. The six limits of the parser are public fields of `Limits` (`crates/html/src/limits.rs`), reached through `ParseOptions.limits`, so a test can set a small one. The two bounds of the label are constants (`LABEL_BYTES_MAX`, `LABEL_WORK_MAX`, `crates/page/src/label.rs:23,34`) and `actionable_elements(tree)` takes none, so a test cannot set them today.

Tests the wave did not add that took 10 s or more in the gate, with their times in the two runs after it where they were over 5 s: `surf-view::http case_30_a_reader_that_never_reads_is_closed_alone_at_the_queue_bound` 23.8 s (6.8 s), `surf-view::stream_bounds f2_a_large_frame_behind_a_full_queue_waits_and_comes_in_order` 21.8 s (9.1 s), `surf-session::adversary_view adversary_an_entry_over_the_stream_queue_is_never_delivered_and_never_reset` 15.0 s, `surf-url::limits generated_inputs_neither_panic_nor_change_when_parsed_again` 14.1 s, `surf-session::built request_slower_than_its_deadline_is_closed` 11.9 s.

`story:html-parser-limits` says of its tests "each test with the default `Limits`", and that `stack_visits_past_2_27_stop_at_the_step_limit` alone holds the default step limit; `story:html-accessible-name` describes the label's tests at the default bounds. A change here amends those sentences.

## Change

Not decomposed. Questions the story answers before it is proposed:

1. Which tests of a limit keep the default value (one per limit, the cheapest that can) and which set a small one, with the time of each alone before.
2. Whether the label's bounds become settable by a test (`actionable_elements_with(tree, LabelBounds)`, a change of `crates/page/src/`), or the label's tests keep their sizes.
3. For each of the five other tests: the cause of its time (a cost paid per item, a wait close to the run time, or the size of its input), and whether it is fixed.

## Acceptance

Not written. It will hold the sum of the times of the files before and after, measured back to back, and a planted defect per limit that still turns the small-limit test red.

## Out of scope

- The tests with a time bound close to their run time: `story:test-step-holds-under-load`.
- One test binary per crate: `story:one-test-binary-per-crate`.
