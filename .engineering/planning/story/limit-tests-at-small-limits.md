---
format: aep.planning-md/3
id: story:limit-tests-at-small-limits
kind: story
status: active
title: The parser's limit tests set small limits and build no input of a default size
summary: The two limit test files of crates/html took 261 s of 443 s summed in a gate on a machine that swapped, one test 39.8 s of the runner's 60 s; the default values become asserted numbers
relations:
- decomposes: epic:headless-rust-target
- depends_on: story:html-parser-limits
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/html/tests/adversary_limits.rs
- confidence: cited
  path: crates/html/tests/common/mod.rs
- confidence: cited
  path: crates/html/tests/limits.rs
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T11:09:29Z", actor: "human:operator", revision: 12, decided_on: {"recorded":{"verification":1}}}
- {from: "proposed", to: "active", at: "2026-11-22T11:09:29Z", actor: "human:operator", revision: 13, decided_on: {"recorded":{"verification":1}}}
---
## Context

The two test files of `story:html-parser-limits`, `crates/html/tests/limits.rs` (36 tests) and `crates/html/tests/adversary_limits.rs` (11 tests), prove the six limits of the parser on input at the default sizes: 16 MiB of input, 1,048,576 nodes, 64 MiB of stored bytes. They run in a build without optimisation. Their time depends on the state of the shared machine by a factor of about ten:

- the gate on `54f7a4e` (2026-11-22 11:37 CET, load average 61): `test step: 42.6 s`; three of the ten slowest tests are theirs: `seeded_10_000_inputs_end_within_bounds` 4.0 s, `input_of_16_777_217_bytes_as_bytes_stops_at_the_input_limit` 3.9 s, `b_505_with_1_024_attributes_each_ends_within_bounds` 3.2 s;
- the gate on `28e6d1a`, the same tests 11 minutes later (load average 53 before and 80 after): `test step: 100.8 s of its budget of 120 s`; the two files sum to 261.1 s of 442.6 s, and nine of the ten slowest tests are theirs: `b_505_with_1_024_attributes_each_ends_within_bounds` 39.8 s, `input_of_16_777_217_bytes_stops_at_the_input_limit` 36.2 s, `input_of_16_777_217_bytes_as_bytes_stops_at_the_input_limit` 33.4 s, `node_1_048_577_stops_at_the_node_limit` 21.4 s, `cloned_attributes_past_64_mib_stop_at_the_stored_bytes_limit` 19.3 s, `seeded_10_000_inputs_end_within_bounds` 16.4 s, `element_with_1_000_000_attributes_ends_within_bounds` 13.3 s, `the_heap_of_a_tree_is_bounded_by_its_stored_bytes_and_nodes` 12.2 s, `object_490_then_misnested_a_to_16_mib_ends_within_bounds` 9.9 s;
- the gate on `5649f83` the same morning: `test step: 156.3 s, over its budget of 120 s`.

The runner ends a test at 60 s; 39.8 s is two thirds of that. The folder rule 'Tests stay fast' asks for a step within 120 s and names a test over 5 s.

No commit separates the runs: `git diff --stat 5649f83 54f7a4e -- crates/html` and `git diff --stat 54f7a4e 28e6d1a -- crates/html` are empty, so the bisect the folder rule asks for has no commit to find; the two times are 42.6 s and 100.8 s. That the slow run comes from memory is inferred from the times, not measured: of the tests in both lists of slowest tests, the two that allocate most took 8.4 times (the input test through bytes) and 12.3 times (`b_505_…`) as long, the seeded test, which is processor work, 4.1 times; 10 GB of swap were in use when that gate ended (read once at 11:52 CET, in no log file). Statement 1 measures the cause.

Read in the code (a survey without a run, 2026-11-22):

- Every parse first turns the whole input into a list of code points (`crates/html/src/tree_builder.rs:270`), and the input stream holds a second copy: an input that the parse stops early in still pays 8 bytes of memory for each of its bytes.
- The six limits are public fields of `Limits` (`crates/html/src/limits.rs`, the defaults at lines 63-68), set through `ParseOptions.limits`; `Limits` and `ParseOptions` compare with `==`. The checks: input `crates/html/src/lib.rs:225-229` (text) and `:244-248` (bytes); steps `tree_builder.rs:280-281`; nodes `:575-576`; stored bytes `:579-580`; depth `:592-593`; attributes `crates/html/src/tokenizer/machine.rs:483-489`, and `tree_builder.rs:768` for merged attributes.
- Tests at the default value that are cheap already: `element_at_depth_513_stops_at_the_depth_limit` (0.4 s in the slow gate) and `attribute_1_025_on_one_element_stops_at_the_attribute_limit` (0.1 s). No test runs at the default step limit: `stack_visits_past_2_27_stop_at_the_step_limit` sets 2^25 and asserts the default as a constant.
- The tests named `…_to_16_mib_…` and `input_of_16_mib_plus_one_byte_ends_within_bounds` already run at 1 MiB with a step limit of 2^23 (8 steps a byte) through the helper `one_mib_form`.
- The helper `bounded` of `limits.rs` compares nodes and stored bytes with constants of the default values (`MAX_NODES`, `MAX_STORED_BYTES`), not with the limits of the parse. `allocation_bound` of `crates/html/tests/common/mod.rs` has a constant term of 1 MiB (`common/mod.rs:111`): 1 byte per input byte at 1 MiB, 16 bytes per input byte at 64 KiB, about a fifth of the bound there.
- `parse_document` (`crates/html/src/lib.rs:224`) and `parse_document_bytes` (`:243`) both take `&ParseOptions`: there is no entry point without options. Every check reads the field of the options it was given (`lib.rs:225`, `:244`; `tree_builder.rs:280`, `:487`, `:502`, `:575`, `:579`, `:592`, `:768`); no check reads a constant or `Limits::default()`.
- `every_stopped_tree_is_well_formed_and_its_stored_bytes_exact` asserts that at least 6 of the 7 outcomes (no stop, and the six limits) were seen: one limit may never stop an input and the test passes.
- Under nextest each test is a process of its own, so an input built once and shared saves nothing.
- The sizes are spelled in several ways in the two files: `16_777_216`, `16_777_217`, `1_048_576`, `1_048_577`, `1_048_580`, `16 * MIB`, `67_108_864`, `MAX_NODES`, `MAX_STORED_BYTES`, `1 << 27`.

## Change

1. Measure first, on the base. (a) Each of the two files run alone three times (`cargo nextest run -p surf-html --test limits`, `--test adversary_limits`), with the time of each test, the load average and the lines `some` and `full` of `/proc/pressure/memory` after each run. (b) For each test of the two files the largest resident set of its process: `/usr/bin/time -f %M <executable> --exact <name>`, the executable taken from `cargo nextest list` or the build's output. Each such run is a test run and is bounded like one: it starts as `slot --timeout 60s /usr/bin/time -f %M <executable> --exact <name>`, one test at a time, never two at once; the loop around the runs starts without `slot`. A test ended at 60 s goes into the evidence as ended, with no resident set. The line `full` of `/proc/pressure/memory` is written beside each number: a resident set taken while the machine swaps is too small, so a number taken with `full avg10` at 1.00 or more is marked and taken again once.
2. A test whose claim is "input past limit X stops at X, and the result names X" sets X small in `Limits` and proves the stop there, with the other limits at their defaults: the input limit at 1,024 bytes (text and bytes), the node limit at 1,024, the stored-byte limit at 1 MiB, and so on, each with the smallest input that crosses it. The assertions on the result stay: the limit named, the count at the stop equal to the limit, the tree's own sums.
3. The default values stay tested without input of their size: (a) one test asserts each of the six fields of `Limits::default()` against its number and `ParseOptions::default().limits == Limits::default()`; (b) the two tests that are cheap at the default value stay (`element_at_depth_513_stops_at_the_depth_limit`, `attribute_1_025_on_one_element_stops_at_the_attribute_limit`), each parsing through `parse_document` and also through `parse_document_bytes` with `ParseOptions::default()`; (c) the tests that stop at the default depth on a small input (`nested_div_100_000_…` and the like, at most 0.27 s in the quiet run) stay unchanged and unrenamed. No test parses 16 MiB, builds 1,048,576 nodes or stores 64 MiB.
4. The tests of the worst inputs (`…_ends_within_bounds`, the `…_to_16_mib_…` tests, `b_505_with_1_024_attributes_each_ends_within_bounds`, `element_with_1_000_000_attributes_ends_within_bounds`, and those of `adversary_limits.rs` that hold a bound per byte or per item) keep their claim at a smaller size: the same shape of input, the same ratio (8 steps a byte where the test sets a step limit), a size of 64 KiB, an attribute limit of 64 for the test of 505 nested `b`, an input that ends shortly after the stop where the rest was only converted. Sizes that differ from 64 KiB, and why:
   - `a_then_div_500_then_a_end_tags_to_16_mib_end_within_bounds` (expects no stop) runs at 64 KiB and no smaller: its 500 `div` cost about 127,000 steps once, a quarter of the 2^19 steps of that size; the defect it guards (each `</a>` scanning the 500 elements again) costs about 8 million.
   - `b_505_with_1_024_attributes_each_ends_within_bounds` at an attribute limit of 64 is about 141 KB of input.
   - `cloned_attributes_past_64_mib_stop_at_the_stored_bytes_limit`: the prefix of formatting elements alone parses with no stop at the stored-byte limit the test sets (asserted in the test), so that the stop comes from the clones.
   - `element_with_1_000_000_attributes_ends_within_bounds` keeps the default attribute limit and an input far past it: 64 KiB, about 10,000 attributes. It does not become the test of attribute 1,025.
   - `the_heap_of_a_tree_is_bounded_by_its_stored_bytes_and_nodes` (400 `span` of 1,024 attributes each, about 33 MiB of stored bytes) runs with 40 `span` or fewer.

   `one_mib_form` becomes a form that takes the size and sets the step limit from it; `bounded` compares with the limits of the parse it checks. The constant term of `allocation_bound` becomes 64 KiB (`1 << 16`), 1 byte per input byte at the new size as it was at the old; the bound of the seeded tests is measured again with it, and the unit shows with a planted copy that the bound still fails (case 5).
5. The seeded tests keep their seeds. `seeded_10_000_inputs_end_within_bounds` runs fewer inputs, at least 1,000, and keeps the quarter that runs at the default limits (the only inputs over 1,000 bytes parsed to their end under `catch_unwind`); every one of the six limits still stops at least one input and the test asserts that; `every_stopped_tree_is_well_formed_and_its_stored_bytes_exact` asserts all 7 outcomes (`== 7`) and may run fewer inputs under the same condition. The unit names the counts it took and the outcomes seen.
6. No test is removed or ignored; a test whose name holds a size that is no longer its size is renamed, and the report lists old and new names. No timeout of the runner is raised and no build profile is changed, in a file or in the environment a task sets. No file under `crates/html/src/` is left changed.
7. `AGENTS.md`: in the entry of `crates/html` on its limits, one sentence: a test of a limit sets a small limit, the default values are asserted as numbers, and no test builds input of a default size. The numbers of tests there follow the files.
8. At close, not by the unit: the note in `.engineering/planning/story/html-parser-limits.md` that "What the change gives up" describes.

Bounds for every command of this story. Every build and test runs through the build pool: `cargo` is wrapped by the pool's shim, every other command that builds or tests gets the `slot` prefix. The pool ends a job after 10 minutes; a command that needs longer names its time with `slot --timeout <t>` or `SLOT_TIMEOUT=<t>`, at most 30 minutes; a `timeout` wrapped around the command extends nothing. A script that measures or waits starts without the `slot` prefix; the builds and test runs inside it carry it. No test runs over 60 s and the test step ends within 120 s. No Python.

How a defect is planted (cases 4, 5, 6 and 11). One plant at a time. The full `git diff` is saved before the plant and after its removal, in the unit's scratch directory outside the tree, and the two files are found equal by `cmp`. A plant changes a condition or a number and leaves the code compiling: a check is planted by making its condition `false` (for the input limit: `cut` set to `false`), not by deleting its lines. The planted tree builds, and each test the evidence lists for the plant fails on an assertion, whose line the evidence holds; a build that fails proves nothing.

## Acceptance

1. The evidence holds the measurements of statement 1 before and after: three runs of each file with the time of each test, the load average and the memory pressure lines; and the largest resident set of each test's process before and after, each with its `full` line.
2. After the change no test process of the two files has a largest resident set over 64 MiB, whatever its `full` line; a number at or under 64 MiB counts only when taken at `full avg10` under 1.00, and a test with no such number is named in the report with its numbers and their `full` lines; before, the evidence names those over 64 MiB. The list of all 47 with their numbers is in the evidence.
3. The two files, run one after the other, before and after back to back within 15 minutes: the sum of the times of their tests after the change is at most a quarter of the sum before; both sums, the load averages and the memory pressure lines are in the evidence. No test of the two files takes over 2 s in the three runs after the change, or the report names it with its three times and what it spends them on.
4. A planted defect per check turns a test red: the check of the input limit for text (`crates/html/src/lib.rs:225-229`) and for bytes (`:244-248`); the step check (`tree_builder.rs:280-281`); the node check (`:575-576`); the stored-byte check (`:579-580`); the depth check (`:592-593`); the attribute check of the tokenizer (`tokenizer/machine.rs:483-489`); the attribute check for merged attributes (`tree_builder.rs:768`). One more plant moves the comparison of the node check by one (`tree_builder.rs:575`): a test this story changed fails on its count at the stop. For each plant the evidence names the tests that went red, and each list holds at least one test this story changed; where the merged-attribute check turns no test red before the change either, the evidence says so and names it as a gap that was there.
5. A planted copy turns the allocation bound red at the new sizes: the start tag token of a formatting element copied for each of its end tags (the defect `allocation_bound` names in its comment; in `crates/html/src/tree_builder/formatting.rs`), or a clone per compared attribute in the test of 505 nested `b`. The evidence names the tests that went red.
6. `every_stopped_tree_is_well_formed_and_its_stored_bytes_exact` goes red when one limit can stop no input: with the depth check planted `false`, the test fails on its count of outcomes.
7. `git grep -n -E '(16|64) \* (MIB|1024 \* 1024)|16_777_21[67]|1_048_5[0-9]{2}|67_108_864|MAX_NODES|MAX_STORED_BYTES|1 << 2[4-7]' -- crates/html/tests/limits.rs crates/html/tests/adversary_limits.rs` prints only lines of the test of statement 3 (a) and comments; the evidence holds the output.
8. The count of tests: `cargo nextest list -p surf-html --test limits` and `--test adversary_limits` give 36 and 11 or more; `git grep -c '#\[ignore' -- crates/html` is unchanged; `git diff <base> --stat -- .config/nextest.toml Cargo.toml 'crates/*/Cargo.toml' .cargo Taskfile.yml taskfiles crates/html/src` is empty.
9. The gate (`slot --timeout 25m task check-runner`) passes on the commit of the unit: the count line shows no failed test and no fewer than 661 tests, and the step prints `test step: N s of its budget of 120 s`. The line, the load average before and after, and every test over 5 s in that run with its time are in the evidence; none of them is a test of the two files, or the report says why.
10. `git grep -n 'no test builds input of a default size' AGENTS.md` prints one line.
11. Each of the six default values (`crates/html/src/limits.rs:63-68`), changed by one and planted alone, turns the test of statement 3 (a) red on its assertion of that field.

## Out of scope

- The label's tests (`crates/page/tests/label.rs`): 3.9 s summed in the slow gate. Its two bounds stay constants.
- The upper bounds on wall time left in other test files: `story:time-bounds-in-tests` (draft).
- The slow tests outside the two files; `story:time-bounds-in-tests` takes them in its question 4. Of the seven such tests among the ten slowest of the gate on `5649f83`, `story:test-step-holds-under-load` changed two (`hidden_flags_cost_one_walk_for_many_references` 15.2 s, then 0.2 s in the gate on `28e6d1a`; `a_server_that_never_reads_the_request_ends_at_the_read_timeout` 10.3 s, then 0.3 s). Five are left, with their times in the gates on `5649f83` and `28e6d1a`: `case_30_a_reader_that_never_reads_is_closed_alone_at_the_queue_bound` 23.8 s and 2.8 s, `f2_a_large_frame_behind_a_full_queue_waits_and_comes_in_order` 21.8 s and 1.9 s, `adversary_an_entry_over_the_stream_queue_is_never_delivered_and_never_reset` 15.0 s and 2.5 s, `generated_inputs_neither_panic_nor_change_when_parsed_again` 14.1 s and 1.6 s, `request_slower_than_its_deadline_is_closed` 11.9 s and 10.1 s. Two more took over 5 s in the gate on `28e6d1a`: `adversary_an_action_recorded_after_a_session_timeout_has_its_entries_kept` 7.2 s and `surfd_that_writes_nothing_exits_3_within_6_seconds` 5.4 s.
- An optimised build of `crates/html` for its tests, and a lane named `slow` for tests at the default sizes: not taken. The story takes the loss it names in "What the change gives up".
- The cost of turning the whole input into code points before the parse (`tree_builder.rs:270`): a property of the product, named here because the tests showed it.
- One test binary per crate: `story:one-test-binary-per-crate`, which depends on this story.

## What the change gives up

No test runs the parser on input of a default size any more: that 16 MiB + 1 byte of input, 1,048,577 nodes or more than 64 MiB of stored bytes are stopped is shown by the same check at a small limit, by the number of the default asserted as a constant, and by the code: every check reads the field of the options it was given (the sites in Context), and no check reads a default constant. A defect that shows only at the default size (an overflow of a counter, a time that grows faster than the input) is not caught by a test. Nor is the way the defaults work together: with one limit small and the others at their defaults, no test shows which default limit stops a parse first (before, 16 MiB of cloned attributes stopped at 64 MiB of stored bytes ahead of 2^27 steps). `story:html-parser-limits` measured the worst page within the default limits once (12.1 s without optimisation, 0.75 s with it, in its evidence).

`story:html-parser-limits` is implemented, and its text describes the tests at their old sizes. The sentences this story makes false, by line of `.engineering/planning/story/html-parser-limits.md` on `e84918a`: 7 (10,000 inputs); 209 (each test with the default `Limits`); 211-213, 216-217 and 221-223 (the sizes of the tests of a limit, `</q>` to 2 MiB); 226-228 (what `bounded` asserts); 232-233 and 240-245 (names with `16_mib`, the table of worst inputs); 247 and 251 (the count of seeded inputs); 260 (the count of tests of `limits.rs`); 272-274 (1 MiB with 8,388,608 steps, the test that holds the default step limit). When this story closes, the session that closes it adds one note under that story's Acceptance: the sizes, counts and names of its limit tests were changed by `story:limit-tests-at-small-limits`, with the list of old and new names; it does not rewrite the sentences.

## Placement

The unit works alone in a worktree from `main` after wave 10 (`e84918a`), on a branch of its own, and merges into `main` before `story:one-test-binary-per-crate` starts; that story is active, depends on this one by an edge, and no unit of it has started.

## Writes

- `crates/html/tests/limits.rs`
- `crates/html/tests/adversary_limits.rs`
- `crates/html/tests/common/mod.rs` (the constant term of `allocation_bound`)
- `AGENTS.md`

Written at close by the session that closes the story, not by the unit: the note in `.engineering/planning/story/html-parser-limits.md`.

Planted and removed again in the same step, never left changed: `crates/html/src/lib.rs`, `crates/html/src/limits.rs`, `crates/html/src/tree_builder.rs`, `crates/html/src/tree_builder/formatting.rs`, `crates/html/src/tokenizer/machine.rs`.
