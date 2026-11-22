---
format: aep.planning-md/3
id: story:test-step-holds-with-tls
kind: story
status: draft
title: The test step holds its budget on main on a loaded host
summary: main's test step took 180.2 s and 147.2 s at a load of 107 to 143, one test ended at 60 s. Opt-level for CPU-bound crates, small sizes and clocks in twelve tests, scripts without task per case, the conformance tests under nextest; at most 90 s at a load of 100
relations:
- decomposes: epic:headless-rust-target
- informed_by: story:time-bounds-in-tests
- supersedes: story:slow-tests-at-small-bounds
scope:
- confidence: cited
  path: .cargo/config.toml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: conformance/tests/main.rs
- confidence: cited
  path: crates/agent/examples/stand_in_surfd.rs
- confidence: cited
  path: crates/agent/tests/common/mod.rs
- confidence: cited
  path: crates/agent/tests/start.rs
- confidence: cited
  path: crates/html/tests/tree_body.rs
- confidence: cited
  path: crates/page/tests/elements.rs
- confidence: cited
  path: crates/session/tests/adversary_events.rs
- confidence: cited
  path: crates/session/tests/adversary_view.rs
- confidence: cited
  path: crates/session/tests/events.rs
- confidence: cited
  path: crates/session/tests/memory.rs
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: crates/surf/tests/common/mod.rs
- confidence: cited
  path: crates/surf/tests/conform_totals.rs
- confidence: cited
  path: crates/surf/tests/generated_check_links.rs
- confidence: cited
  path: crates/surf/tests/generated_check_modes.rs
- confidence: cited
  path: crates/surf/tests/lock_check.rs
- confidence: cited
  path: crates/surf/tests/review1_generated_check.rs
- confidence: cited
  path: crates/surf/tests/review2_generated_check.rs
- confidence: cited
  path: crates/view/tests/http.rs
- confidence: cited
  path: crates/view/tests/stream_bounds.rs
- confidence: cited
  path: taskfiles/conform-totals.sh
- confidence: cited
  path: taskfiles/generated-compare.sh
- confidence: cited
  path: taskfiles/lock-check.sh
revision: 42
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
its tests is ended at 60 s: 875 tests, 147.2 s at a load of about 140 at 20:22, against 53.8 s
at 19:10. This story is about `main`; the wave's row is a source of times for the tests of
`main` it runs. Sources: the TLS gate's `gate.log.plain` and `times.txt` (one line per
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
  75.8 s summed. They are `story:tls-tests-as-tables`, not this story.
- The 53.8 s of 19:10 against 147 to 180 s later is no regression to bisect (the folder's rule
  asks for `git bisect` when a step got slower): `git diff --stat c5c2895 afba5e3` changes 3
  files, `docs/plan/agent-waves.md` and two under `website/`, no Rust and the same 875 tests.
  The load at 19:10 was not measured. The difference is the load of the host.
- Outside nextest, 19 to 27 s: the doc step (8 to 13 s, above) and nextest's own start (the
  build check of `cargo`, then each of the 18 test binaries run once to list its tests), which
  no run measured apart. The doc step's `cargo test --doc` checks the build a second time; that
  check is the only repeated work and is under 1 s (`Finished ... in 0.70s` in the baseline's
  `test-doc.log`). Statement 14 measures the listing.

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

All on `main`. "Expected" is the time this story aims at; each changed test is bound by
acceptance 3, not by the estimate.

### Order inside the unit

One implementing unit in one tree (`/` is short of space; a second tree is a second `target`).
First statement 3 (the overrides), measured and decided, because statement 2 and statement 11
read the profile it leaves; then statements 1, 2 and 4 to 14 in any order.

### Statements

1. `crates/session/tests/memory.rs` `bodies_of_answered_commands_are_not_kept` (lines 17 to 63;
   a, d; 8.5 to 46 s, ended at 60 s alone at load 180 to 218). It parses 21 pages of 256 KiB.
   Its main assertion is a count: `Session::held()` is `(0, 0, 0)` after `open`, after each
   `go` and after `look`; a kept body, event or invocation makes it non-zero at the first command
   that keeps one, whatever the body's size. The body becomes 4 KiB and the loads `open <url>`
   and 5 `go`s; `open <url>` is two actions (`open`, then `go`), so the path holds 7 and the last
   assertion becomes `path 5 of 7` (line 63, today `path 5 of 22`), which still shows the path cut
   to five. The comment's 26.25 MiB sentence goes. Expected: under 1 s alone, under 3 s in the
   gate.
2. `crates/session/tests/events.rs` `case_08_a_page_of_6_mib_leaves_the_session_open` (d; 12 to
   37 s, ended at 60 s in the 20:22 gate and alone at load 180 to 218). The 6 MiB are the claim
   (case 8 of `story:agent-session-events`: a page over the answer text limit of 4 MiB,
   `wire::ANSWER_TEXT_MAX`, a constant of the wire with no seam), so the size stays; the time is
   the HTML parser of `surf-html` in the session process (`perf`, Why). Statement 3 makes it
   faster; the test does not change. If after statement 3 it is over 10 s in one of three runs of
   `cargo nextest run -p surf-session` at a load of at least 100, the unit stops and reports, and
   the page's size is decided in a story of its own (a page just over `ANSWER_TEXT_MAX`, or a
   seam for the limit); this story does not change case 8's claim.
3. `.cargo/config.toml`: `[profile.dev.package.<name>]` with `opt-level = 1` for each package
   that qualifies (d). The test profile inherits it. The forge reads the same file, and the
   repository's `.cargo/config.toml` wins over `[profile]` in `Cargo.toml` (`AGENTS.md`
   :208-213, which says the file holds only `debug = false` and gains a sentence per override).
   The conformance workspace is built from the root with `--manifest-path` (`task conform-rust`,
   `Taskfile.yml`:111), so it reads the same file and gets the overrides too.
   Candidates: `surf-html`, `surf-page`, `surf-url`, `surf-page-text`. Rule per override: two
   builds per level from `cargo clean -p <pkg>`, each without a hit of the shared compile cache
   for the package (`sccache --show-stats` before and after each build, in the evidence); the
   build at opt-level 1 is at most 30 s longer than at 0; and the summed test time is at most
   half. The summed time counts the package's own tests and the tests of other packages that
   this statement names for it (for `surf-html`: `case_08`, `memory`, the `core::*`,
   `adversary_parse::*` and `redirect::case_24` tests of `surf-session`); otherwise the override is
   not taken.
   Measured already on `surf-html` alone (its own tests only; build after `cargo clean -p
   surf-html`, tests with `cargo nextest run -p surf-html`), two runs each, 20:26 to 20:31, load
   115 to 157:

   | | opt-level 0 (today) | opt-level 1 |
   |---|---|---|
   | build (`Finished` line) | 56.0 s, 14.7 s | 54.5 s, 46.3 s |
   | nextest `Summary` | 10.7 s, 16.2 s | 6.3 s, 6.2 s |
   | summed test time | 41.8 s, 52.9 s | 31.0 s, 30.4 s |
   | `both_entry_points_give_the_same_tree_and_the_same_limit` | 10.3 s, (not in top 5) | 1.3 s, 1.6 s |

   The CPU-bound tests run 4 to 8 times faster; the package's own summed time falls by 26 to 43 %
   (the rest is process starts and tests that wait), which alone is not half. The 14.7 s build
   at level 0 was likely a hit of the shared cache, which this measurement did not check.
   On `wave/tls-1`, `.cargo/config.toml` already holds `[profile.dev.package.ring]` with
   `opt-level = 3` (`story:tls-decisions` item 4, lines 7 and 8 there). Both changes append after
   `[profile.test]`, so the wave's merge of `main` conflicts in that file; the merge keeps both
   tables, each once (a second `[profile.dev.package.ring]` table is a duplicate key TOML
   rejects). This story adds no override for `ring` and no line for it on `main`.
4. `crates/view/tests/stream_bounds.rs` `f2_a_large_frame_behind_a_full_queue_waits_and_comes_in_order`
   and the other users of `blocking_size` (a; 2.1 to 23.3 s). `blocking_size` is 3 times the
   largest send and receive buffers (`tcp_wmem` and `tcp_rmem`, third field): 108 MiB on this
   host. A client that never reads keeps its receive buffer near its default (second field of
   `tcp_rmem`, 128 KiB here); the size becomes 3 times (largest send buffer + default receive
   buffer), 12.4 MiB here. `wait_for("the pump took entry 5")` (line 383) passes whether or not
   the writer blocks, so the test gains the assertion that entry 4 is still queued when the pump
   takes entry 5 (the writer is still in entry 3), and that assertion holds at the new size.
   Expected: under 3 s.
5. `crates/view/tests/http.rs` `case_30_a_reader_that_never_reads_is_closed_alone_at_the_queue_bound`
   (a; 3.6 to 32.2 s). It publishes 2,000 entries of 16 KiB (32 MiB) to fill a queue bound of
   256 KiB behind the kernel's buffers. It publishes until the silent reader is closed (an event
   it already waits for, line 1135), at most the 2,000 it publishes now, and counts what it
   published. Lines 1140 to 1144 assert `arrived < published` (today `arrived < ENTRIES`, which
   is always true after an early stop). Expected: under 3 s.
6. `crates/surf/src/lib.rs` `escape` (lines 545 to 558) for `crates/surf/tests/escape.rs`
   `every_character_is_written_as_printable_ascii_and_reads_back` (d; 13.2 to 32.5 s). The test
   escapes each of the 1,112,064 scalar values, and `escape` formats each escaped byte with
   `format!`. `escape` writes the four bytes `\xHH` from a table of hex digits; its output is the
   same, and the tests of `escape.rs`, unchanged (`git diff <base> --
   crates/surf/tests/escape.rs` is empty), are the proof. Expected: under 3 s.
7. `crates/session/tests/adversary_view.rs` `adversary_an_entry_over_the_stream_queue_is_never_delivered_and_never_reset`
   (a; 7.8 to 20.7 s). It moves from the shipped queue of 1 MiB to a configured one of 64 KiB
   (`Config::view_limits.queue_bytes` through `surfd_in_process`) and `AMPERSANDS` 4,000, so one
   entry's JSON (about 24,000 bytes per address, five addresses) is still over the queue bound and
   the request under `wire::REQUEST_MAX`. The module comment says it no longer runs at the shipped
   limit. No test holds the shipped value today (`git grep` finds no assertion of
   `queue_bytes` against 1 MiB under `crates/*/tests`); this file gains one: `Limits::default().queue_bytes`
   is `1 << 20`. Expected: under 2 s.
8. (Moved to `story:session-request-timeout-option`: `--request-timeout` for
   `request_slower_than_its_deadline_is_closed`. Its time stays in this story's step until that
   story lands; the targets of acceptance 1 do not count on it.)
9. `crates/session/tests/adversary_events.rs` `adversary_an_action_recorded_after_a_session_timeout_has_its_entries_kept`
   (b; 7.3 to 9.7 s). Today: an answer timeout of 2 s, a server that answers after 4 s, a
   `sleep` of 5 s; `AGENTS.md` :1272-1283 gives the reason, each step 2 s or more apart so the
   order does not depend on the runner's speed. It sets the answer timeout to 500 ms and the
   server's delay to 1.5 s (each step still 1 s apart; the server's delay counts from its read,
   after `surfd` sent), and replaces the `sleep` with a wait for the event: `look` until the
   path shows the `go`, under the file's wait bound. `AGENTS.md` :1272-1283 says the new numbers.
   Expected: under 3 s.
10. `crates/agent/tests/start.rs` `surfd_that_writes_nothing_exits_3_within_6_seconds` (b; 5.1 to
    5.6 s; it waits `wire::START_TIMEOUT`, 5 s). The case runs `client::run` in the test process
    with `Config { start_timeout: 500 ms, .. }` and an `Environment` whose program directory holds
    the stand-in `surfd`; it asserts the returned status 3, the message `surfd did not report
    ready within 0.5 seconds` (formed from the configured time, `crates/agent/src/client.rs`:220),
    the killed stand-in and the missing socket. The binary's exit status stays tied to
    `client::run`'s by `surfd_that_refuses_exits_3_with_its_reason` (`start.rs`:103, a binary
    run that asserts exit 3; `src/main.rs` is `ExitCode::from(client::run(..))`). One assertion
    compares `Config::default().start_timeout` with 5 s; the test's name loses "6 seconds".
    `crates/agent/src/client.rs` is not changed (round 2's `story:agent-fill-pick` holds it). An
    in-process start passes `surfd` only `XDG_RUNTIME_DIR` and the test binary forbids unsafe
    code, so no `set_var`: the stand-in (`crates/agent/examples/stand_in_surfd.rs`:43) reads its
    control directory from `STAND_IN_DIR`, and when that is unset from the directory `c` beside
    `XDG_RUNTIME_DIR` (the `TestDir` layout, `r` and `c`, `crates/agent/tests/common/mod.rs`:69-75);
    `common/mod.rs` gains the in-process helper. `AGENTS.md` :1272-1283 drops the 5 s wait.
    Expected: under 1.5 s.
11. `crates/page/tests/elements.rs` `a_deep_tree_is_read_without_recursion` (line 736; a, d; 4.4
    to 43.6 s). It parses 100,000 nested `span`s and 20,000 nested `span` and `input` pairs. The
    walk runs in a thread of a 256 KiB stack, and the depth is set from that stack with a margin:
    the unit measures the frame of the recursive walk planted under the final profile of
    statement 3 and sets the depth to at least 4 times stack / frame (4,000 at 64 bytes would be
    256,000 bytes, under 262,144, and would not overflow). Expected: under 2 s.
12. `crates/session/tests/events.rs` `case_07_a_body_is_kept_as_its_length` (lines 571 to 605; a;
    1.1 to 10.8 s). It compares every 8-byte window of the page with every window of every entry
    (quadratic). The page's windows go into a `BTreeSet` once and each entry's windows are looked
    up; the failure text is the same. Expected: under 1 s.
13. The class-(c) tests of `surf` that use a process only as a vehicle (c; 23 tests, 74.8 s
    summed in the baseline: `review1_generated_check` 4 tests 29.0 s, `generated_check_links` 6
    17.6 s, `review2_generated_check` 2 9.4 s, `conform_totals` 5 7.7 s, `generated_check_modes` 2
    7.0 s, `lock_check` 4 4.1 s; 1.0 to 12.5 s per test alone). Each case runs `task <name>` in a
    copy: `task` parses the whole `Taskfile.yml` with its 12 includes (`task --list` 0.15 s at load
    125 outside the pool), then the task's shell; `generated-compare` also runs `ess` in step 2
    when step 1 passes (0.56 to 0.66 s for the whole task outside the pool at load 125). The copy
    is small (`spec/` and `generated/rust`, 65 files, under 1 MB). The cases are about the
    script, not about `task`. The scripts of `lock-check`, `generated-compare` and
    `conform-totals` move from the Taskfile into `taskfiles/lock-check.sh`,
    `taskfiles/generated-compare.sh` and `taskfiles/conform-totals.sh` (POSIX sh, as
    `taskfiles/slowest-tests.sh`); the tasks run them, and the tests run `sh <script>` per case,
    so no case starts `task`. Where several cases of one test would run `ess` over the same
    unchanged record, the test runs it once and reads its output per case. Expected: each test
    under 3 s; the 23 together under 25 s summed in the gate.
14. The doc step and nextest's start: unchanged (Why). The unit times `cargo nextest list
    --workspace` once at the start of its work and once at the end, with the load, so the part
    outside nextest's `Summary` is known; it changes nothing there.
15. The conformance runner's own tests (`conformance/tests/main.rs`, `task conform-rust`,
    `cargo test`, which ends no test at 60 s; 56.8 s in one run, from
    `story:slow-tests-at-small-bounds`): they run under cargo-nextest with the root's
    `.config/nextest.toml` (`--config-file`), so a test over 60 s is ended and fails, and the
    step prints its time like the test step. The two `grep -q` checks of `Taskfile.yml` for named
    tests read nextest's output.

### Kept, each with its reason

- `label::past_the_label_work_bound_names_come_from_attributes_only` (d): its size is
  `LABEL_WORK_MAX`, 16 MiB, a constant; served by statement 3 if `surf-page` qualifies.
- `adversary_document_text::adversary_document_kept_pages_equal_a_fresh_rendering_at_every_edge`
  (20.3 s in the baseline, 2.7 to 3.4 s alone and in the TLS gate; d): one `look` per edge of 7
  pages in 2 forms, rendering in `surf-page-text`; served by statement 3 if `surf-page-text`
  qualifies; otherwise kept (every edge is the claim).
- `surf-url` `limits::generated_inputs_neither_panic_nor_change_when_parsed_again` and
  `limits::input_longer_than_the_limit_is_too_long` (11.6 to 13.2 s in the baseline, 0.6 to 7.3 s
  alone; d): an address of `MAX_INPUT_LENGTH` (2 MiB, a constant) and generated inputs; served by
  statement 3 if `surf-url` qualifies.
- `surf-session` `core::a_page_over_the_byte_bound_is_read_to_its_end`,
  `core::long_address_still_gets_an_answer`, `core::a_page_read_to_its_end_is_rendered_once`
  (8 to 11 s in the baseline, 0.7 to 25.1 s alone; d): pages and addresses at the product's
  bounds, parsed and rendered; served by statement 3 (`surf-html`, `surf-page-text`).
- `surf-agent-tasks` lanes (c): the claim is the shell driver and the processes it starts.
- `surfd` idle-exit tests and `surf-session` `built::idle_timeout_ends_the_session` (b, c): about
  the process; 1 s is the smallest value `--idle-exit` and `--idle-timeout` take.
- `surf-view-fixture` (c): the pipe between processes is the claim.
- `surf-view` `case_23` (b): waits the write limit of 1 s, the event it is about.

### Overlap with round 2 and the merge order

Round 2 changes the same files: `crates/session/tests/memory.rs` (this story :17-63; round 2's
`impl/round2-commands` adds `kept_text_is_dropped_on_a_request_other_than_look` after :64),
`crates/session/tests/events.rs` (this story `case_07` :571-605 and `case_08` stays; round 2
:871-1056 and after :1332), `crates/surf/src/lib.rs` (this story `escape` :545-558; round 2 :412,
:493, :43-305, :763-1108). The remaining round-2 stories claim all of `crates/session/tests`
(`agent-fill-pick`, `agent-press-submit`, `agent-back-forward`, `agent-change-and-log`) and
`crates/agent/tests` (`agent-change-and-log`). This story lands on `main` first; then, in this
order: `impl/ess-0-97-4`, `wave/tls-1`, `impl/round2-commands`, `impl/round2-view`; each merges
`main` and keeps both sides.

### Taken over from `story:slow-tests-at-small-bounds`

This story supersedes it. Its Change: "a test that waits for a deadline sets it small" (here 9,
10; the read deadline goes to `story:session-request-timeout-option`); "a test of a bound sets
the bound small" (here 1, 4, 5, 7, 11, 12); "a test that loops over a large set does the
expensive part once" (here 6, 13); "the conformance runner's tests run under cargo-nextest with
the 60 s end" (here 15); "a page test waits on the event" (to `story:page-test-step-within-budget`);
"each test keeps what it asserts" (acceptance 5). Its Acceptance 1 (no test over 2 s at load
under 20) is replaced by acceptance 3 here; 2 (load over 100, no test over 20 s, step within
90 s) is acceptance 1 and 3 here; 3 (conformance test ended at 60 s) is acceptance 8; 4 and 5
(names, counts) are acceptance 6; 6 (`view-test` three runs under load, `timeout: 2000`) goes to
`story:page-test-step-within-budget`.

## What it does not do

- No higher test-thread count, no shards, no raised budget or timeout.
- No test moves to the lane `slow`: none of these regenerates anything from sources.
- No change of the doc step (measured: no gain), of the load, or of the pool.
- No TLS test and no file under `crates/tls*`: `story:tls-tests-as-tables`. No `ring` override.
- The session's `--request-timeout`: `story:session-request-timeout-option`.
- The page-test step (`task view-test`, Playwright's `timeout: 90_000` over the rule's 60 s):
  `story:page-test-step-within-budget`.
- The wall-time upper bounds left in tests: `story:time-bounds-in-tests`.
- No change of `crates/agent/src/client.rs`.

## Acceptance

1. Step time on `main`. Every gate or `task test` run started for this story is in the evidence
   with `cat /proc/loadavg` before and after it; none is dropped. Three consecutive runs of
   `ESS_TOOLCHAIN=0.97.3 slot --timeout 15m task test` on the story's commit, whose two readings
   are each at least 100, each print `test step: N s` with N at most 90. If the host's load is
   under 100 while the unit works, the runs are reported with their load and this statement stays
   open. Stop: if after all changes such a run is over 90 s, the unit reports the ten largest
   remaining costs from that run and the story stays `active`.
2. The forge: the `task check` of the pull request or push prints its `test step:` line; it is
   quoted in the evidence.
3. In each of the three runs of 1, every test changed by this story (statements 1, 4 to 7, 9 to
   13, and statement 2's `case_08`) takes under 3 s in the JUnit report, and no test among the ten
   slowest is over 20 s.
4. `memory::bodies_of_answered_commands_are_not_kept` and `events::case_08_a_page_of_6_mib_leaves_the_session_open`:
   neither over 10 s in three runs of `cargo nextest run -p surf-session` at a load of at least
   100, with the loads, in the evidence.
5. Each changed test turns red when its cause is planted, one plant per test, removed again
   after (`git diff --stat` equal before and after), each red run's failure text in the evidence:
   a body kept by `Session` (1); a writer that does not wait for the queue, the `queue_bytes`
   check at `crates/view/src/sse.rs`:237 and :251 removed (4 and 5, one plant each); `escape`
   that leaves U+2028 unescaped (6); an entry over the queue delivered without a reset (7); the
   action entry of a timed-out `go` dropped (9); a silent `surfd` not killed (10); a recursive
   walk (11); an entry holding 8 bytes of the page (12); for 13, per script one changed line that
   lets its defect through. For every package that gets an override, under the final profile,
   a recursive walk or drop planted per test that proves "without recursion"
   (`crates/html/tests/tree_body.rs`:118-143, `crates/page/tests/elements.rs`:736) turns that
   test red; the measured frame size is in the evidence.
6. `cargo nextest list --workspace` before and after gives the same 875 tests: no test is removed
   or merged.
7. Per override of statement 3: the two builds per level with `sccache --show-stats` before and
   after each, the build times, the summed test times, and the decision by the rule; the time of
   `task conform-rust` before and after. `git diff <base> -- .config/nextest.toml` is empty, and
   the overrides are in `.cargo/config.toml` only.
8. A conformance runner test made to sleep 70 s (planted) is ended at 60 s and fails `task
   conform-rust`; the step prints its time.
9. `git diff <base> -- crates/agent/src/client.rs crates/tls*` is empty.

## Scope

The files of `aep plan artifact scope`; `AGENTS.md` at :208-213 (`.cargo/config.toml`) and
:1272-1283 (the waits of statements 9 and 10).
