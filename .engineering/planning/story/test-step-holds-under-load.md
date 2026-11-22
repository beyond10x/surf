---
format: aep.planning-md/3
id: story:test-step-holds-under-load
kind: story
status: active
title: Three tests with a time bound close to their run time hold under load
summary: 'Wave 10''s gate on a loaded machine: 2 of 661 tests failed on a time bound close to their run time and a third was close; the test step took 156.3 s there and 46 to 52 s of nextest when measured back to back'
relations:
- decomposes: epic:headless-rust-target
- depends_on: story:html-accessible-name
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/fetch/tests/fetch_bounds.rs
- confidence: cited
  path: crates/page/tests/label.rs
- confidence: cited
  path: crates/view/tests/http.rs
revision: 23
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T09:57:05Z", actor: "human:operator", revision: 20, decided_on: {"recorded":{"verification":1}}}
- {from: "proposed", to: "active", at: "2026-11-22T09:57:05Z", actor: "human:operator", revision: 21, decided_on: {"recorded":{"verification":1}}}
---
## Context

On 2026-11-22 the gate of wave 10 (`task check-runner` on `5649f83`, branch `wave/a10-label-limits`) ended red on a machine of 20 processors, at a load average the session read as 120 to 136 (the gate's log does not hold it): `661 tests: 659 passed, 2 failed`, and `test step: 156.3 s, over its budget of 120 s` (nextest 121.1 s, 35.2 s outside it). The 659 tests nextest ran sum to 598.1 s. The gate of release `0.3.0` the same morning, on `f593c41`, passed with 579 tests, nextest 47.5 s and `test step: 61.9 s`.

The time of the step, measured. The folder rule asks for the commit that made a step slower, with the timing as the test. Between `f593c41` and `5649f83` the code changed in the two units of wave 10 only; they added three test files (`crates/html/tests/limits.rs`, `crates/html/tests/adversary_limits.rs`, `crates/page/tests/label.rs`, 81 tests). Four runs of `cargo nextest run --workspace --locked` on `5649f83` in the gate's exported tree, back to back at 10:47 to 10:50 CET, load average 123 falling to 78 (1 minute): without the three files 43.1 s and 40.0 s (578 tests), with them 52.4 s and 45.9 s (659 tests, all passed), summed test time 258.7 s and 227.3 s. The three files add 6 to 9 s. The gate's 121.1 s did not repeat; its cause beyond the state of the shared machine is not known. This story therefore does not change what the tests of wave 10 cost; `story:limit-tests-at-small-limits` (draft) holds that.

Two tests failed in the gate that wave 10 did not change, and a third of the same kind was close:

- `surf-fetch::fetch_bounds a_server_that_never_reads_the_request_ends_at_the_read_timeout` (`crates/fetch/tests/fetch_bounds.rs:155`): "one fetch took 9.630169193s with a read timeout of 200ms and ended Err(Unreachable("connecting to 127.0.0.1:… failed: Connection refused (os error 111)"))". Cause, read in the code: the test's server (`accept_then`, line 22) stops accepting `BOUND` (10 s) after its thread starts and drops the listener (lines 28 to 35); the fetch connects only after its work on an address of 16 MiB (`Target::parse` in `crates/fetch/src/lib.rs`, 0.5 s alone in a build without optimisation, 9.6 s in the gate); so the connection was refused. The test's bound on the time is `BOUND / 2`, and its assertion on the result (`Err(Failure::Unreachable(_))`, line 176) also holds for a refused connection: in the gate the test did not exercise the read timeout it names. In the two full runs above it took 0.8 to 1.2 s; alone, at a load average of 92 to 97, the unit measured 6.8 s (failed), 2.2 s and 7.2 s (failed).
- `surf-view::http case_23_a_client_that_reads_nothing_is_closed_at_the_write_limit` (`crates/view/tests/http.rs:832`): its second assertion `assert!(elapsed <= Duration::from_secs(2))` (line 871, the stream) failed with 2.754918515 s; the first of the same form (line 853, the answer) passed. The write limit of the test is 1 s, so each bound leaves 1 s for everything else. Both halves already wait for the close as an event through `wait_for`, which fails after `WAIT` (10 s, lines 37 and 386). The whole test takes 2.4 to 2.5 s in the two full runs.
- `surf-page::label hidden_flags_cost_one_walk_for_many_references` (`crates/page/tests/label.rs:501`), added by wave 10: it asserts that each of two calls of `actionable_elements` takes under 10 s of wall time (lines 502, 515, 528). The whole test took 15.2 s in the gate and 7.4 s and 4.6 s in the two full runs. The next test of the file (`the_time_of_the_names_does_not_grow_with_the_attributes_of_the_nodes_read`, line 555) proves a cost by a count (`attribute_visits()`) and no time.

`story:tests-hold-under-load` fixed two tests of the same kind for release `0.3.0`.

## Change

1. Measure first, on the base of the unit: each of the three tests alone, three times (`cargo nextest run -p surf-fetch --test fetch_bounds a_server_that_never_reads`, `-p surf-view --test http case_23_a_client`, `-p surf-page --test label hidden_flags`), each with the time nextest prints and the load average after the run, in the evidence.
2. The fetch test proves the read timeout and nothing else ends it. The request that fills the socket buffers is a body sent through `send`, so that no 16 MiB address is parsed before the connect; if `send` cannot carry such a request (its method, the limit on the request body), the unit stops this statement and reports what it read. The test's server accepts until the test tells it to stop: its accept window is no longer `BOUND` from the start of its thread. The test fails when the fetch ends for another reason than the read timeout: it asserts on the detail of the failure (`Failure::detail`), so that a refused connection turns it red with that text. The time it bounds runs from the moment the server accepted to the end of the fetch. The bound on that time is at least 10 times the median of that time in three runs alone after the change, and at most 30 s. The server lets go of the connection at that bound plus 1 s, so that a fetch that ignores the read timeout ends and the test fails on its own assertion, with its message, before the runner's 60 s.
3. `case_23` asserts for both halves that the close did not come before the write limit (`elapsed >= limits.write`) and has no other upper bound than the `WAIT` of `wait_for`, which already fails the test when the close does not come. The two other upper bounds of 2 s in the file take the same form (added after the unit found them): `case_20` (half a request, a receive limit of 1 s) keeps its lower bound and its upper bound becomes `WAIT`; `case_32` (the keep-alive comment of an idle stream) has `WAIT` as its upper bound.
4. `hidden_flags_cost_one_walk_for_many_references` asserts its claim (one walk for many references) by a count and asserts no wall time. For each of its two pages it compares `attribute_visits()` of `actionable_elements` for the references with the visits for the same page where the `input`s carry `aria-label` in place of `aria-labelledby`. The unit measures the difference first; the test bounds it per reference by a number that one walk of hidden flags per reference exceeds (Acceptance 5 shows that it does). The pages hold 2,000 comments or children and 2,000 references each, not 50,000 (changed after the first form of the unit: with 50,000 the test took up to 51.9 s beside busy loops, and the planted walk per reference ended at the runner's 60 s and not on the count; with 2,000 the planted walk ends and the count names it). If `attribute_visits()` cannot tell the two apart without a change under `crates/page/src/`, the unit stops this statement and reports what it read; it does not change the product for it.
5. No timeout of the runner is raised, no test is removed or ignored, no assertion on a result is weakened, no build profile is changed.
6. `AGENTS.md`, in the entry on tests, one sentence: a test bounds a time only by a wait for the event with a limit at least 10 times the time it takes alone, and proves a cost by a count. `AGENTS.md` and the comments of the tests name no `--test <file>` selection; the commands stay in the evidence.

Bounds for every command of this story. Every build and test runs through the build pool: `cargo` is wrapped by the pool's shim, every other command that builds or tests gets the `slot` prefix. The pool ends a job after 10 minutes; a command that needs longer names its time with `slot --timeout <t>` or `SLOT_TIMEOUT=<t>`, at most 30 minutes; a `timeout` wrapped around the command extends nothing. A script that measures or waits starts without the `slot` prefix. No test runs over 60 s and the test step ends within 120 s. No Python.

## Acceptance

1. The evidence holds for each of the three tests three times before and three times after the change, each with the load average after the run, from the commands of statement 1.
2. With the listener of the fetch test's server dropped before the fetch connects (planted in the test file), the test goes red and its message holds the text of the refused connection. With the read timeout not applied to the send (planted in `crates/fetch/src/lib.rs`), the test goes red on its own assertion, with its message, in less than the runner's 60 s. Each plant is made after the unit's change of the test is complete; `git diff --stat` is taken before the plant and after its removal, and the two outputs are equal.
3. The fetch test's bound on its time is at least 10 times the median of that time in three runs alone after the change, and at most 30 s; both numbers are in the evidence. `git grep -n 'BOUND / 2' crates/fetch/tests/fetch_bounds.rs` prints nothing.
4. `case_23`: `sed -n` of the test in the evidence shows no `elapsed <=` and two assertions that `elapsed` is at least the write limit. With a view server that does not close a client that reads nothing at the write limit (planted in `crates/view/src/http.rs` for the answer, in `crates/view/src/sse.rs` for the stream, one after the other), the test goes red with the text of `wait_for`. With a server that closes at once, before the write limit (planted), it goes red on the new assertion. `case_20` and `case_32`: `git grep -n 'from_secs(2)' crates/view/tests/http.rs` prints no upper bound on a time; with a server that does not close at the receive limit (planted, `case_20`) and one that sends no keep-alive comment (planted, `case_32`), each test goes red before the runner's 60 s. `git diff` is saved before each plant and after its removal, and `cmp` finds the two equal.
5. `hidden_flags_cost_one_walk_for_many_references` holds no `Instant` and no `Duration`: `sed -n` of the test in the evidence, with the measured difference of visits per reference for each of its two pages. With the walk of hidden flags made to run once per reference (planted in `crates/page/src/`), one plant per half of the test (the `body` after comments; the `summary` after children), the test goes red on its count each time. `git diff --stat` before each plant and after its removal are equal. Or, by statement 4: the report of what the unit read, `git diff <base> -- crates/page/tests/label.rs` empty, and case 6 covers the other two tests only.
6. Each of the three tests passes ten times in a row beside busy loops; the same script run on the base, before the change, gives the count before, which is recorded and may be 10 of 10. The script builds first (`cargo nextest run -p <package> --test <file> --no-run`) and starts no build under the loops. For each run it starts `nproc` loops, each `timeout 90 sh -c 'while :; do :; done'`, runs the one test (the runner ends it at 60 s), then kills the loops and waits for them before the next run. It ends its loops on every exit (`trap` on EXIT, INT, TERM, HUP); it starts without `slot`; it writes the start and end of each run, the PIDs it started and the output of `ps` for them after it ended. `10 of 10 passed` for each of the three after the change, the counts before, and the load averages are in the evidence.
7. `git diff <base> --stat -- .config/nextest.toml Cargo.toml 'crates/*/Cargo.toml' .cargo` is empty, and `git grep -c '#\[ignore' -- crates conformance` gives the same counts before and after.
8. The gate (`slot --timeout 25m task check-runner`) passes on the commit of the unit: both conformance runs pass every scenario, the count line of the test step shows no failed test and no fewer than 661 tests, and the step prints `test step: N s of its budget of 120 s`. The line, the load average before and after the run, and every test over 5 s in that run with its time are in the evidence.
9. `git grep -n 'proves a cost by a count' AGENTS.md` prints one line, and `git grep -n -e '--test fetch_bounds' -e '--test http' -e '--test label' -- AGENTS.md crates` prints nothing.

## Out of scope

- What the tests of wave 10 cost at the default limits, and the other tests that took 10 s or more in the gate: `story:limit-tests-at-small-limits` (draft).
- One test binary per crate: `story:one-test-binary-per-crate`, which depends on this story.
- The load of the shared machine, and the time of the test step under it beyond case 8.
- The limits and the bounds of the label and their values: `story:html-parser-limits` and `story:html-accessible-name` set them; this story changes one test of `story:html-accessible-name` (`hidden_flags_cost_one_walk_for_many_references`) in how it proves its claim, not in the claim.
- A product change. The unit writes no file under `crates/*/src/`.

## Placement

The unit works on `wave/a10-label-limits`, in a worktree of its own from the head of that branch, before wave 10 merges into `main`: the wave's gate is red without it. It runs alone on that branch; `story:html-parser-limits` and `story:html-accessible-name` are built there and wait for this gate to be closed. `story:one-test-binary-per-crate` starts after wave 10, with this story in it, is on `main`.

## Writes

- `crates/fetch/tests/fetch_bounds.rs`
- `crates/view/tests/http.rs`
- `crates/page/tests/label.rs`
- `AGENTS.md`

Planted and removed again in the same step, never left changed: `crates/fetch/src/lib.rs`, `crates/view/src/http.rs`, `crates/view/src/sse.rs`, `crates/page/src/lib.rs`, `crates/page/src/label.rs`.

## What the change gives up

Found by the adversary on `01a40e2`; each mutant was red on the tests as they were before this story and passes them now. The story takes this loss: an upper bound on wall time tight enough to catch a limit applied at twice its value is the bound that failed on the loaded machine. `story:time-bounds-in-tests` (draft) holds the way back: a clock or a count the test controls.

- `case_23`: the write limit applied at twice its value (`crates/view/src/http.rs:364`, `crates/view/src/sse.rs:321`) passes; anything up to about 9 times the limit passes under `WAIT`. A close before the limit is still caught: 20 % early 5 of 5 beside busy loops, 5 % early alone.
- `case_20`: the receive limit at twice its value (`crates/view/src/http.rs:349`) passes.
- `case_32`: the heartbeat at twice its value (`crates/view/src/sse.rs:290`, `:334`) passes.
- The fetch test: a read timeout applied at 30 times its value (`crates/fetch/src/lib.rs:693`) passes; the bound of 10 s is 50 times the read timeout of 0.2 s.
- `hidden_flags_cost_one_walk_for_many_references`: a walk of hidden flags per reference that compares no attribute passes the count; with 50,000 nodes the old form ended at the runner's 60 s. The test's comment says what the count catches and what it does not.

Not introduced here, found on the way: a keep-alive comment sent at once passes `case_32` before and after (no lower bound); a send timeout applied per write and not to the whole send passes the fetch test before and after (no test sends to a server that reads slowly); a receive limit at 0.9 of its value passes `case_20` before and after; the drip tests of `crates/fetch/tests/fetch_bounds.rs:108` bound a wall time at 5 times the read timeout.
