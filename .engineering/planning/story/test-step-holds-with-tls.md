---
format: aep.planning-md/3
id: story:test-step-holds-with-tls
kind: story
status: draft
title: The test step holds its budget on a loaded host, on main and with the TLS wave
summary: main's test step took 147.2 s and 180.2 s at a load of 107 to 143, one test ended at 60 s; the TLS wave's 126.3 s. Twelve slow tests set a small size or clock or run optimised HTML parsing; the TLS wave merges 207 table rows into 20 tests
relations:
- decomposes: epic:headless-rust-target
- informed_by: story:slow-tests-at-small-bounds
- informed_by: story:time-bounds-in-tests
scope:
- confidence: cited
  path: .cargo/config.toml
- confidence: cited
  path: crates/agent/tests/start.rs
- confidence: cited
  path: crates/page/tests/elements.rs
- confidence: cited
  path: crates/session/src/process.rs
- confidence: cited
  path: crates/session/tests/adversary_events.rs
- confidence: cited
  path: crates/session/tests/adversary_view.rs
- confidence: cited
  path: crates/session/tests/built.rs
- confidence: cited
  path: crates/session/tests/events.rs
- confidence: cited
  path: crates/session/tests/memory.rs
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: crates/tls-conformance/tests
- confidence: cited
  path: crates/tls-test-certs/tests
- confidence: cited
  path: crates/tls-test-server/tests
- confidence: cited
  path: crates/tls/tests/tls
- confidence: cited
  path: crates/view/tests/http.rs
- confidence: cited
  path: crates/view/tests/stream_bounds.rs
revision: 17
---
## Why

The rule (`AGENTS.md` of the folder, "Tests stay fast"): the test step of the gate ends within
120 s of wall time and no test runs over 60 s; a step over the budget is a defect that gets a
story and is fixed before the next feature; a higher timeout, shards or waiting for less load is
not a fix.

Four runs of `task test` on 2026-11-22 on this host (20 CPUs). Load is the 1-minute load average.

| Run | Tests | nextest `Summary` | `test step` | Load |
|---|---|---|---|---|
| `main`, 19:10 CET | 875 | | 53.8 s | similar to the runs below (as reported) |
| `wave/tls-1` at `32df67f`, 19:51 to 19:58 | 1,432 | 107.519 s, all passed | 126.3 s, over | 88 to 113 |
| `main` (`afba5e3`), this story's baseline, 20:05 to 20:08 | 875 | 153.023 s, all passed | 180.2 s, over | 107 to 115 |
| `impl/ess-0-97-4` at `48e1b6c` (no Rust change against `afba5e3`), 20:22 | 875 | 126.419 s, 874 passed, 1 timed out | 147.2 s, over | 134 to 143 |

So `main` itself is over the budget at a load of 107 to 143, without the TLS tests, and one of
its tests is ended at 60 s. Sources: the TLS gate's `gate.log.plain` and `times.txt` (one line per
test), the baseline's `.out/test.log` and JUnit report, the third run's `gate.log.plain`; the
evidence of this story holds copies.

What the time is spent on, measured:

- The step is nextest's run plus about 19 to 27 s outside it (126.3 - 107.5, 180.2 - 153.0,
  147.2 - 126.4): the doc tests and nextest's build check and listing. `cargo test --doc
  --workspace` alone took 8.2 s and 10.4 s (load 107 to 119, the pool's job time), and `cargo
  test --doc -p surf-ports` (the only crate with doc tests, 2 of them, one `compile_fail`) 5.3 s
  and 13.0 s: restricting the doc step to that crate gains nothing, so this story leaves it.
- Tests at a time: 5. The pool's daemon sets `NEXTEST_TEST_THREADS=5` in every job (`slot serve
  --test-threads 5` in the user unit `slot.service`; read from `/proc/<pid>/environ` of a running
  `cargo-nextest`); neither `Taskfile.yml` nor `.config/nextest.toml` sets a number. The sums of
  test times over the wall time agree: 526.7 s over 107.5 s (4.9) in the TLS gate, 754.3 s over
  153.0 s (4.9) in the baseline. The pool also runs every job at `CPUWeight=20` under a
  `CPUQuota` of 1800 %, so a CPU-bound test slows by the load of the whole host. This story does
  not change the number.
- The floor: in the TLS gate 705 tests took under 0.1 s, mean 0.046 s, fastest 0.006 s, tenth
  percentile 0.022 s; in the baseline 285 under 0.1 s, mean 0.050 s. One test process costs
  about 0.02 to 0.05 s at a load of about 110, so 1,432 tests cost 29 to 72 s summed, 6 to 15 s of
  wall at 5 at a time; 875 tests 4 to 9 s of wall.
- The time is in a few tests. The ten slowest of the 20:22 run hold 225 s of summed time; the
  ten slowest of the TLS gate 104 s. The same tests lead every run, and their times move by a
  factor of 3 to 7 with the load (table below). Profiled with `perf` alone (load 157 to 205):
  `case_08` spends 88 % of its samples in the `surf-session` process, its top symbols the HTML
  tokenizer and tree builder of `surf-html` and the precondition checks of a build without
  optimisation; `memory::bodies_of_answered_commands_are_not_kept` spends 92 % in the test
  process, the same symbols. Both parse megabytes of HTML in a build without optimisation
  (6 MiB; 21 times 256 KiB).
- The TLS tests are not slow one by one (none over 2.8 s) but many: `surf-tls` 465 tests,
  75.8 s summed.

Each test over 3 s in the TLS gate or the baseline, run alone twice (one nextest run per
package, `--test-threads 1`, the filter `test(=<name>)` per test; round 1 at 20:09 to 20:19, load
110 rising to 218; round 2 at 20:19 to 20:25, load 180 falling to 107; times by nextest).
Class: (a) work that grows with a size the test could make small; (b) a wait on a clock; (c) a
process or a build per test or per case; (d) CPU work in a build without optimisation; (e)
inherent. Times: TLS gate / baseline / 20:22 run (where in its ten slowest) / alone round 1 /
round 2, in seconds.

| Test | Gate times | Alone | Class |
|---|---|---|---|
| `surf-session` `memory::bodies_of_answered_commands_are_not_kept` | 8.5 / 42.9 / 43.5 | 60.1 ended / 16.9 | a, d |
| `surf-session` `events::case_08_a_page_of_6_mib_leaves_the_session_open` | 21.9 / 33.5 / 60.3 ended | 60.1 ended / 15.0 | d |
| `surf-view` `http::case_30_a_reader_that_never_reads_is_closed_alone_at_the_queue_bound` | 3.6 / 25.7 / 32.2 | 6.0 / 4.6 | a |
| `surf-view` `stream_bounds::f2_a_large_frame_behind_a_full_queue_waits_and_comes_in_order` | 2.1 / 23.3 / 19.6 | 20.2 / 6.9 | a |
| `surf` `escape::every_character_is_written_as_printable_ascii_and_reads_back` | 14.5 / 13.2 / 21.6 | 32.5 / 18.9 | d |
| `surf-session` `adversary_view::adversary_an_entry_over_the_stream_queue_is_never_delivered_and_never_reset` | 7.8 / 15.9 / 9.4 | 20.7 / 16.6 | a |
| `surf-session` `built::request_slower_than_its_deadline_is_closed` | 11.1 / 11.6 / 10.3 | 11.8 / 10.5 | b |
| `surf-session` `adversary_events::adversary_an_action_recorded_after_a_session_timeout_has_its_entries_kept` | 7.5 / 9.7 / 7.7 | 8.3 / 7.3 | b |
| `surf-page` `elements::a_deep_tree_is_read_without_recursion` | 4.9 / 4.4 / 11.8 | 43.6 / 25.1 | a, d |
| `surf-page` `label::past_the_label_work_bound_names_come_from_attributes_only` | 5.6 / 6.7 / 8.3 | 29.4 / 17.1 | d |
| `surf-session` `events::case_07_a_body_is_kept_as_its_length` | 1.1 / 10.8 / - | 1.8 / 2.8 | a |
| `surf-agent` `start::surfd_that_writes_nothing_exits_3_within_6_seconds` | 5.1 / 5.6 / - | 5.2 / 5.3 | b |
| `surf-session` `core::a_page_over_the_byte_bound_is_read_to_its_end` | 1.5 / 11.3 / - | 8.0 / 17.4 | d |
| `surf-session` `core::long_address_still_gets_an_answer` | 1.1 / 8.2 / - | 5.2 / 25.1 | d |
| `surf-session` `redirect::case_24_a_redirect_to_an_address_that_does_not_fit_the_record_is_answered_without_it` | 0.8 / 7.5 / - | 11.8 / 1.0 | d |
| `surf-session` `adversary_parse::a_page_with_a_long_title_leaves_go_answering_and_the_session_open` | 4.2 / 1.1 / - | 7.5 / 1.2 | d |
| `surf-session` `adversary_parse::a_title_one_byte_over_the_cut_is_cut_in_the_entry` | 0.9 / 6.6 / - | 1.0 / 8.6 | d |
| `surf-session` `core::a_page_read_to_its_end_is_rendered_once`, `e2e::e2e_open_go_look_close`, `adversary_parse::a_page_with_more_links_than_a_request_carries_answers_and_keeps_the_session_open` | 0.2 to 2.7 / 3.3 to 3.9 / - | 0.7 to 7.3 / 1.7 to 5.2 | d |
| `surf-html` `adversary_limits::both_entry_points_give_the_same_tree_and_the_same_limit` | 3.3 / 1.6 / - | 13.2 / 7.3 | d |
| `surf-html` `tree_body::a_deep_input_is_parsed_read_and_dropped_without_recursion`, `adversary_limits::every_stopped_tree_is_well_formed_and_its_stored_bytes_exact` | 0.6 to 1.6 / 4.8 to 4.9 / - | 0.4 to 0.9 / 0.9 to 5.6 | d |
| `surf-html-conformance` `check::the_tag_counts_of_the_groups_sum_to_the_tag_lines`, `inventory::*` (2) | 0.9 to 1.7 / 3.0 to 8.9 / - | 0.6 to 13.5 / 0.4 to 1.5 | d |
| `surf-url` `limits::generated_inputs_neither_panic_nor_change_when_parsed_again`, `limits::input_longer_than_the_limit_is_too_long` | 0.7 to 1.3 / 11.6 to 13.2 / - | 3.0, 7.3 / 2.5, 0.6 | d |
| `surf-page` `label::hidden_flags_cost_one_walk_for_many_references` | 0.9 / 3.1 / - | 5.6 / 1.7 | d |
| `surf` `review1_generated_check::*` (4), `review2_generated_check::*` (2), `generated_check_links::*` (3), `generated_check_modes::*` (1), `lock_check::a_line_cargo_does_not_write_fails_naming_it` (TLS wave) | 2.3 to 11.0 / 2.7 to 12.3 / - | 1.4 to 12.5 / 1.0 to 7.9 | c |
| `surf-agent-tasks` `tasks::*` (2), `model_driver::*` (3), `adversary2::*` (2), `adversary::a_call_with_a_line_feed_in_a_word_is_one_call` | 1.7 to 5.1 / 1.7 to 6.5 / - | 1.4 to 6.2 / 1.3 to 10.0 | c (and b for the two bounds of 1 s) |
| `surf-session` `events::case_14_a_subscriber_that_never_reads_is_disconnected`, `events::case_12_entries_after_a_number`, `built::idle_timeout_ends_the_session`, `built::malformed_requests_do_not_end_the_session` | 0.4 to 2.3 / 3.7 to 5.6 / - | 1.7 to 6.2 / 0.4 to 2.2 | b, c |
| `surfd` `built::case_12_idle_exit_after_the_last_close`, `built::surfd_started_by_hand_writes_no_view_token_unless_asked`, `view::case_17_a_stream_from_a_number_replays_or_resets`, `view::case_27_the_idle_exit_ends_the_stream_and_removes_view_url` | 0.6 to 2.4 / 3.2 to 3.6 / - | 2.0 to 5.2 / 1.5 to 4.5 | b, c |
| `surf-view` `http::case_23_a_client_that_reads_nothing_is_closed_at_the_write_limit` | 2.2 / 3.6 / - | 2.5 / 2.6 | b |
| `surf-view-fixture` `fixture::a_session_is_listed_the_pipe_cuts_and_mends_and_the_end_of_input_stops_everything` | 1.4 / 5.2 / - | 2.2 / 0.7 | c |
| `surf-session` `adversary_document_text::adversary_document_kept_pages_equal_a_fresh_rendering_at_every_edge` | 3.4 / 20.3 / - | 2.7 / 2.9 | e |

Alone is not faster than in the gate for the CPU-bound tests: at a load of 150 to 218 the pool's
jobs get little CPU. `task generated-compare`, the step the class-(c) tests of `surf` run per
case, took 0.56 s and 0.66 s outside the pool at a load of 126.

## Change

### On `main`

1. `crates/session/tests/memory.rs`, `bodies_of_answered_commands_are_not_kept` (a, d; 8.5 to
   46 s, ended at 60 s alone at load 180 to 218). It parses 21 pages of 256 KiB. Its assertion
   is a count: `Session::held()` is `(0, 0, 0)` after the `open`, after each `go` and after the
   `look`; a kept body, event or invocation makes it non-zero at the first command that keeps
   one, whatever the body's size. The body becomes 4 KiB and the loads `open` and 3 `go`s (each
   to its own address), then `look`; the comment's 26.25 MiB sentence goes. Expected: under 1 s
   alone, under 3 s in the gate at a load of about 100.
2. `crates/session/tests/events.rs`, `case_08_a_page_of_6_mib_leaves_the_session_open` (d; 12 to
   37 s, ended at 60 s in the 20:22 gate and alone at load 180 to 218). The 6 MiB are the claim
   (case 8 of `story:agent-session-events`: a page over the answer text limit of 4 MiB,
   `wire::ANSWER_TEXT_MAX`, a constant of the wire with no seam), so the size stays; the time is
   the HTML parser of `surf-html` in the session process (`perf`, above). It gets faster by
   statement 3. Expected: by the factor statement 3 measures for `surf-html`; the target is
   under 5 s in the gate at a load of about 100. If statement 3 does not bring it under 10 s in
   three runs of `cargo nextest run -p surf-session` at a load of about 100, the unit stops and
   reports, and the page's size is then decided in a story of its own (a page just over
   `ANSWER_TEXT_MAX`, or a seam for the limit); it does not change the claim of case 8 itself.
3. `.cargo/config.toml`: `[profile.dev.package.surf-html]` with `opt-level = 1` (d). The test
   profile inherits it, and the forge reads the same file (the repository's `.cargo/config.toml`
   wins over `[profile]` in `Cargo.toml`, the reason `debug = false` lives there), so both build
   the same way. Measured on `surf-html` alone (build from `cargo clean -p surf-html`, test with
   `cargo nextest run -p surf-html`), two runs each, 20:26 to 20:31, load 115 to 157:

   | | opt-level 0 (today) | opt-level 1 |
   |---|---|---|
   | build (`Finished` line) | 56.0 s, 14.7 s | 54.5 s, 46.3 s |
   | nextest `Summary` | 10.7 s, 16.2 s | 6.3 s, 6.2 s |
   | summed test time | 41.8 s, 52.9 s | 31.0 s, 30.4 s |
   | `both_entry_points_give_the_same_tree_and_the_same_limit` | 10.3 s, (not in top 5) | 1.3 s, 1.6 s |
   | `limits::seeded_1_200_inputs_end_within_bounds` | (not in top 5), 10.5 s | 2.6 s, (under 1.5 s) |

   The CPU-bound tests run 4 to 8 times faster; the package's summed time falls by 30 to 40 %,
   the rest being process starts and tests that wait. The build of the one package costs about
   the same in the first pair (56.0 s and 54.5 s); the second build at opt-level 0 (14.7 s) was
   likely served from the shared compile cache, so the extra build time is not established
   beyond "at most about 40 s for this package at this load". The unit then measures, the same way, one
   package at a time, and adds the override for each whose summed test time falls by at least a
   half for at most 30 s more of a clean build of that package: candidates `surf-page`,
   `surf-url`, `surf-page-text` (the session's rendering), and on the TLS wave `surf-tls` and
   `ring`. The tests this serves: `case_08`, the `core::*` and `adversary_parse::*` tests of
   `surf-session`, `case_24`, `label::*`, `both_entry_points_*`, the `surf-html-conformance`
   tests, the `surf-url` limit tests. No assertion changes; a planted defect is not needed for
   a build setting, and every test of those packages must still pass.
4. `crates/view/tests/stream_bounds.rs`, `f2_a_large_frame_behind_a_full_queue_waits_and_comes_in_order`
   and the other users of `blocking_size` (a; 2.1 to 23.3 s). `blocking_size` is 3 times the
   largest send and receive buffers (`tcp_wmem` and `tcp_rmem`, third field): 108 MiB on this
   host. A client that never reads keeps its receive buffer near its default (second field of
   `tcp_rmem`, 128 KiB here); the size becomes 3 times (largest send buffer + default receive
   buffer), 12.4 MiB here. The test already waits for the event that proves the writer blocks
   (`wait_for("the pump took entry 5")`) and asserts the peak; if the smaller frame does not
   block, that wait fails, so the change shows itself. Expected: under 3 s at a load of about
   100.
5. `crates/view/tests/http.rs`, `case_30_a_reader_that_never_reads_is_closed_alone_at_the_queue_bound`
   (a; 3.6 to 32.2 s). It publishes 2,000 entries of 16 KiB (32 MiB) to fill a queue bound of
   256 KiB behind the kernel's buffers. It publishes until the silent reader is closed (an event
   the test already waits for), at most the 2,000 it publishes now, so a defect still runs into
   the same end. Expected: under 3 s at a load of about 100.
6. `crates/surf/tests/escape.rs`, `every_character_is_written_as_printable_ascii_and_reads_back`
   (d; 13.2 to 32.5 s). It escapes each of the 1,112,064 scalar values, and `escape`
   (`crates/surf/src/lib.rs`) formats each escaped byte with `format!`. `escape` writes the four
   bytes `\xHH` from a table of hex digits; its output is the same, and this test, unchanged, is
   the proof (with `separators_and_format_characters_are_escaped`). Expected: under 5 s at a
   load of about 100 (an estimate: the test's own allocations stay).
7. `crates/session/tests/adversary_view.rs`, `adversary_an_entry_over_the_stream_queue_is_never_delivered_and_never_reset`
   (a; 7.8 to 20.7 s). It sets `Config::view_limits.queue_bytes` to 64 KiB through
   `surfd_in_process` and `AMPERSANDS` to 4,000, so one entry's JSON (about 24,000 bytes per
   address, five addresses) is still over the queue bound, with the request under
   `wire::REQUEST_MAX`; the module comment names the new numbers. The assertion is unchanged.
   Expected: under 2 s.
8. `crates/session/src/process.rs` and `crates/session/tests/built.rs`,
   `request_slower_than_its_deadline_is_closed` (b; 10.3 to 11.8 s). The test waits the request
   deadline of 10 s (`wire::REQUEST_TIMEOUT`). `surf-session` takes `--request-timeout
   <seconds>` beside `--idle-timeout`, of the same form, for a test that starts the binary
   itself (`surfd` passes neither); the test starts it with 1 and keeps its bound at five times
   the deadline (5 s); `arguments` and its tests read the new option (`other_argument_shapes_exit_2`
   gains its malformed forms). One assertion compares `wire::REQUEST_TIMEOUT` with 10 s.
   Expected: under 2 s.
9. `crates/session/tests/adversary_events.rs`, `adversary_an_action_recorded_after_a_session_timeout_has_its_entries_kept`
   (b; 7.3 to 9.7 s). It sets `session_answer_timeout` to 200 ms (now 2 s), the slow server's
   delay to 600 ms (now 4 s), and replaces the `sleep` of 5 s by a wait for the event: `look`
   until the path shows the `go`, under the file's wait bound. The assertions are unchanged.
   Expected: under 2 s.
10. `crates/agent/tests/start.rs`, `surfd_that_writes_nothing_exits_3_within_6_seconds` (b; 5.1
    to 5.6 s). It waits `wire::START_TIMEOUT` (5 s). `client::Config::start_timeout` is the seam
    the client already has; the case runs `client::run` in the test process with 500 ms and keeps
    its assertions on status, message, the killed `surfd` and the socket; the message's time is
    the configured one. One assertion compares `Config::default().start_timeout` with 5 s; the
    name says what it tests without "6 seconds". If the binary's message is not formed from the
    configured time, the unit stops this statement and reports. Expected: under 1.5 s.
11. `crates/page/tests/elements.rs`, `a_deep_tree_is_read_without_recursion` (a, d; 4.4 to
    43.6 s). It parses 100,000 nested `span`s and 20,000 nested `span` and `input` pairs. The
    walk runs in a thread of a 256 KiB stack, the depth becomes 10,000 and 4,000 (a recursive
    walk of at least 64 bytes a frame overflows it); `max_depth` follows. Planted defect: the
    walk made recursive; the test must fail (overflow of that thread, reported as its panic).
    Expected: under 2 s.
12. `crates/session/tests/events.rs`, `case_07_a_body_is_kept_as_its_length` (a; 1.1 to 10.8 s).
    It compares every 8-byte window of the page with every window of every entry (quadratic). The
    windows of the page go into a `BTreeSet` once, each entry's windows are looked up; the
    failure text is the same. Expected: under 1 s.

Kept, with their class and the reason, so a reviewer can see they were read:
`label::past_the_label_work_bound_names_come_from_attributes_only` (d; its size is
`LABEL_WORK_MAX`, 16 MiB, a constant; served by statement 3 if `surf-page` qualifies);
the class-(c) tests of `surf`, `surf-agent-tasks`, `surfd` and `surf-view-fixture` (a process or
a copy per case is what each case checks; 0.4 to 12.5 s; the one-off highs follow the load); the
waits of 1 s in `surfd`'s idle exit and `surf-agent-tasks`' bounds (whole seconds, the smallest
value the options take); `adversary_document_text` (e; 2.7 to 3.4 s alone and in the TLS gate).

### On `wave/tls-1`, after this story's merge into it

Counted from `git show wave/tls-1:<path>` and `list.txt` (`cargo nextest list` of the wave);
times from the TLS gate. 20 groups hold 207 tests (167 of `surf-tls`, 40 of `surf-tls-test-server`,
`surf-tls-test-certs`, `surf-tls-conformance`) whose rows run one claim with the same assertion
shape. Each group becomes one test that loops over its table and names the row in every failure
text (several helpers name no row today, so the message gains `{row}`):

| Group | Tests | Summed |
|---|---|---|
| `handshake_hostile.rs`: tests that end in `refused(..)` / `refused_by(..)` | 46 | 5.0 s |
| `path.rs`: `index_cases!` (one test per row of INDEX) | 33 | 2.2 s |
| `x509_parse.rs` and `adversary_x509.rs`: `Certificate::from_der` is `Err(E)` | 20 | 1.3 s |
| `der.rs` and `adversary_x509.rs`: `der::parse` is `Err(E)` | 16 | 1.0 s |
| `tls-test-server` `handshake.rs`: `suite_and_group` and `scheme` | 12 | 0.3 s |
| `record.rs`: `reader(..)` then `refused(..)` | 9 | 0.6 s |
| `tls-test-certs` `check.rs` and `adversary.rs`: a changed copy fails the check | 9 | 4.3 s |
| `tls-conformance` `check.rs`: `compare(..)` failures | 8 | 0.6 s |
| `names.rs` (7), `adversary_path.rs` (7), `pem.rs` and `adversary_x509.rs` (6), `adversary_handshake.rs` (5), `seam.rs` sign (5) and ring search (4), `tls-test-server` `modes.rs` (5) and `adversary_server.rs` (4 + 2), `handshake.rs` traces (3) and outcomes (3), `rfc8448.rs` replays (3) | 54 | 4.0 s |

1,432 tests become 1,245 (545 TLS tests become 358); 187 process starts go, about 4 to 9 s
summed, 1 to 2 s of wall at 5 at a time. This is small: the floor is not where the wave's time
is. The wave's time is the tests of `main` above and the CPU of `surf-tls` (statement 3).
Before the merge of a group: 61 of the grouped tests are rows of `rfc8446-client-musts.txt`,
`rfc8446-client-fields.txt` and `rfc8446-record-musts.txt`, and `check_named_table`
(`handshake.rs`) requires a `#[test] fn` per row; the table names the merged test and its row
(`<file>::<test>#<row>`), and the checker reads rows of a table test; `every_case_of_index_has_its_test`
compares INDEX with the table, not with the macro. A test of a group that is a mutant's name
(`adversary_x509`) keeps that name as its row name.

## What it does not do

- No higher test-thread count, no shards, no raised budget or timeout.
- No test moves to the lane `slow`: none of these regenerates anything from sources.
- No change of the doc step (measured above: no gain), of the gate's tasks or of the load.
- The wall-time upper bounds left in tests: `story:time-bounds-in-tests`. The conformance
  runner under nextest and the page tests' `timeout: 2000`: `story:slow-tests-at-small-bounds`,
  whose list of slow tests this story measures and takes over for the tests it names.

## Acceptance

1. Step time on `main`: three runs of `task test` (`ESS_TOOLCHAIN=0.97.3 slot --timeout 15m task
   test`) on the story's commit print `test step: N s of its budget of 120 s` with N at most
   80 s, each at a 1-minute load average between 90 and 110 read at start and end
   (in the evidence beside the line). At a load of about 140 one run prints N at most 120 s.
2. Step time on the wave: the same, on `wave/tls-1` with this story merged and the TLS part done,
   N at most 100 s at a load between 90 and 110, and at most 120 s at about 140.

   Why these numbers and not 60 s and 90 s: from the TLS gate's times (load 88 to 113), the
   expected times above and the factor measured in statement 3 for the packages it names, the
   summed test time falls from 526.7 s to about 420 s on the wave and from about 430 s to about
   325 s for the tests of `main`; at 4.9 tests at a time and about 19 s outside nextest that is
   about 105 s on the wave and 85 s on `main` at that load. 60 s and 90 s would need more: the
   override of statement 3 on more packages than measured here (each measured, as statement 3
   says), or fewer tests over 1 s. The targets 80 s and 100 s ask the unit for that part beyond
   the estimate; the evidence states the load of each run, and a run at a load outside 90 to 110
   does not count for 1 and 2. The 53.8 s of `main` at 19:10 shows the same tests can take half
   the time at a load reported as similar; the load average alone does not fix the time.
3. In each of those runs, the gate's list of the ten slowest holds no test over 20 s.
4. `memory::bodies_of_answered_commands_are_not_kept` and `events::case_08_a_page_of_6_mib_leaves_the_session_open`:
   neither over 10 s in three runs of `cargo nextest run -p surf-session` at a load of about 100,
   times and loads in the evidence.
5. Each changed test turns red when its cause is planted, one plant per test, in the source the
   test is about, removed again after (`git diff --stat` equal before and after): a body kept by
   `Session` (1); a session that closes on a page over the answer text limit (2); a writer that
   does not block, queue bound ignored (4, 5); `escape` that leaves U+2028 unescaped (6); an
   entry over the queue delivered after a reset not sent (7); a slow request not closed at its
   deadline (8); the action entry of a timed-out `go` dropped (9); a silent `surfd` not killed
   (10); a recursive walk (11); an entry holding 8 bytes of the page (12). Each red run's
   failure text is in the evidence.
6. The count: `cargo nextest list --workspace` on `main` before and after gives the same 875
   tests (no test is removed or merged on `main`). On the wave, 1,432 before and 1,245 after
   (or the count the unit reaches), with the list of merged tests and the test each went into.
7. `git diff <base> -- .config/nextest.toml Taskfile.yml` is empty, and the opt-level
   overrides are in `.cargo/config.toml` only, each with its measured build and test times in
   the evidence.

## Scope

The files listed by `aep plan artifact scope`; the TLS files only after the merge into the wave.
