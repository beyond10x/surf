---
format: aep.planning-md/3
id: story:tests-hold-under-load
kind: story
status: active
title: Two tests that fail on a loaded machine pay their cost once
summary: review1's test of one changed byte copies the tree once per generated file (24 to 42 s, killed at 60 s at load 97), and a stream test waits 5 s for something that takes 4 to 9 s
relations:
- decomposes: epic:headless-rust-target
scope:
- confidence: cited
  path: crates/surf/tests/common/mod.rs
- confidence: cited
  path: crates/surf/tests/review1_generated_check.rs
- confidence: cited
  path: crates/view/src/sse.rs
- confidence: cited
  path: crates/view/tests/stream_bounds.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T05:27:49Z", actor: "human:operator", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-22T05:27:49Z", actor: "human:operator", revision: 6}
---
## Context

On 2026-11-22 the gate of the unit `security-review-fixes` ended red at a load average of 97 on two tests that the unit did not change:

- `surf::review1_generated_check review1_one_changed_byte_in_any_generated_file_fails_naming_it`: killed at 60 s. It took 24.0 s in the gate at `ef9f5d5` and 41.8 s in the gate at `263dd71`. It copies `Taskfile.yml`, `spec/` and `generated/rust` once per committed generated file (28 files then, 36 now) and runs `task generated-compare` in each copy. The cost of a copy and of a comparison is multiplied by the number of files: the mistake the folder rule "Tests stay fast" names.
- `surf-view::stream_bounds f2_the_wait_for_an_empty_queue_ends_when_the_stream_closes`: "not within 5s" at `crates/view/tests/stream_bounds.rs:183`. It took 4.6 s and 9.3 s in the two gates above, so its wait of 5 s is close to what the test needs on an idle machine.

The test step of that gate took 118.9 s of its budget of 120 s.

## Change

- The test of one changed byte keeps its claim (every committed file under `generated/rust`, one at a time, fails the comparison and is named) and stops paying a copy of the tree, and where possible a process of `task`, per file.
- The test of the stream that closes does not depend on a wait that is close to its own run time: the cause of the 4 to 9 s is found and named; the test waits on the event, and the bound that fails it is the runner's 60 s.
- No timeout of the runner is raised and no test is removed or ignored.

## Acceptance

- Each of the two tests, run alone three times before and three times after the change on the same machine, with the times and the load average in the story's evidence: after the change the first takes less than a third of its time before, and the second no longer fails when its process is slowed (shown by a run under `nice -n 19` beside a busy loop per core, or by the cause being removed).
- A planted defect still turns each test red: a comparison that skips one generated file; a stream whose threads do not end when the client goes away.
- The gate (`task check-runner`) passes; the test step stays within 120 s.

## Out of scope

- One test binary per crate: `story:one-test-binary-per-crate`.
- The load of the shared machine.
