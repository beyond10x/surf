---
format: aep.planning-md/3
id: story:session-request-timeout-option
kind: story
status: implemented
title: surf-session takes --request-timeout for a test that starts it
summary: request_slower_than_its_deadline_is_closed waits the 10 s read deadline in every run (10.3 to 11.8 s at load 88 to 218); --request-timeout lets it set 1 s; lands on main before story:tls-session-anchors
relations:
- decomposes: epic:agent-cli-target
- informed_by: story:test-step-holds-with-tls
- blocks: story:tls-session-anchors
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/session/src/process.rs
- confidence: cited
  path: crates/session/tests/built.rs
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T21:48:50Z", actor: "human:operator", revision: 8}
- {from: "proposed", to: "active", at: "2026-11-22T21:48:50Z", actor: "human:operator", revision: 9}
- {from: "active", to: "implemented", at: "2026-11-23T00:05:25Z", actor: "human:operator", revision: 12, decided_on: {"recorded":{"test_result":4,"verification":1}}, executor: "agent:coordinator"}
---
## Why

`crates/session/tests/built.rs` `request_slower_than_its_deadline_is_closed` (:248-262) waits out
the session's read deadline, `wire::REQUEST_TIMEOUT` (10 s, a constant,
`crates/agent/src/wire.rs`:104), which `serve` applies to each connection
(`crates/session/src/process.rs`:181). The session serves one connection at a time, so the test's
`look` waits until the slow connection's deadline has passed. Measured on 2026-11-22 (the table
of `story:test-step-holds-with-tls`): 11.1 s, 11.6 s and 10.3 s in three gates and 11.8 s and
10.5 s alone, at loads of 88 to 218; the time is the deadline, the same at every load. The rule
of the folder wants tests of a minute or two in total, and `story:test-step-holds-with-tls` lists
this test as class (b), a wait on a clock. A test that starts the binary itself can already
shorten the idle timeout (`--idle-timeout`, `process.rs`:66-80); the read deadline has no such
option.

The tree and the order (D246). This story is built on a branch from `main` (`0f0c157`) and lands
on `main` before `story:tls-session-anchors` starts. As built (D266): the gate of its branch
`impl/session-request-timeout` at `9f83444`, with `main` `7d15e06` merged, ended red on time
alone (`test step: 121.9 s, over its budget of 120 s`, 893 of 893 tests passed, load 142 to
177). `story:doc-tests-of-one-package` is built on that branch as `fix/doc-tests-of-ports`, and
the two stories land on `main` together after one gate. That story changes
`crates/session/src/process.rs` (`Arguments` and the usage line, for the anchors) and
`crates/session/tests`, so `story:tls-session-anchors` depends on this story, and this story
blocks it, both edges for `process.rs`. No branch changes `process.rs` or `built.rs` today
(`git diff --stat main...<branch>` for `wave/tls-1`, `impl/round2-commands` and
`impl/round2-view`).

D251: every time target here is read on three consecutive runs that print `/proc/loadavg` before
and after. The story closes on those three runs at the load that is present, and names that load.
A later run over the target at a higher load is a new defect with its own story.

## Change

1. `crates/session/src/process.rs`: `surf-session` takes `--request-timeout <seconds>`, a whole
   number from 1, of the form of `--idle-timeout`, for a test that starts the binary itself.
   Each of the two options comes at most once, before the identity, in either order. `Arguments`
   (:56-63) gains `request_timeout`, its default `wire::REQUEST_TIMEOUT`; `serve` (:181) takes its
   deadline from it. The usage line at :1 reads `surf-session [--idle-timeout <seconds>]
   [--request-timeout <seconds>] <identity> <socket path>`. `surfd` passes neither option.
2. The interface record of `story:agent-session-process` (its body at :135, `surf-session
   [--idle-timeout <seconds>] <identity> <socket path>`) is amended through `aep plan artifact
   body` to name the option.
3. `crates/session/tests/built.rs`: `request_slower_than_its_deadline_is_closed` starts the
   session with `--request-timeout 1` and bounds the next request at five times that deadline,
   5 s, with the failure text `the next request waited <time>, five times the read deadline of 1
   second is 5`. A new test, `the_shipped_read_deadline_is_10_seconds`, asserts that
   `wire::REQUEST_TIMEOUT` is 10 s. `other_argument_shapes_exit_2` (:265) gains `--request-timeout
   0` and `--request-timeout` without a number.
4. `AGENTS.md`, the waits of `built` (:1306-1307 on `main`: "`built` waits out a deadline of the
   product (`wire::REQUEST_TIMEOUT`, 10 s, in `crates/agent/src/wire.rs`)"): the sentence names
   the option and the new time. `story:test-step-holds-with-tls` already wrote the rest of that
   paragraph on `main`.

### Overlap

- `story:conformance-step-within-budget` changes `AGENTS.md` at the bullets of `task
  conform-rust` and of `conformance/`; `story:page-test-step-within-budget` at the bullets of
  `task view-test` and `task check` and the page tests' times (:1319-1320 on `main`). None of
  them overlaps the sentence of statement 4. `impl/round2-view` rewrites the page tests' times
  right after it; the merge keeps both.
- `story:tls-session-anchors` starts after this story lands (above).

## Acceptance

1. `other_argument_shapes_exit_2` holds `--request-timeout 0` and `--request-timeout` without a
   number, and passes.
2. `sed -n 1p crates/session/src/process.rs` names `[--request-timeout <seconds>]`.
3. Guard, true on `0f0c157` too: `git grep -n request-timeout crates/surfd crates/agent` prints
   nothing.
4. Three consecutive runs of `cargo nextest run -p surf-session`, each with `cat /proc/loadavg`
   before and after: `built::request_slower_than_its_deadline_is_closed` takes under 3 s in each
   (`target/nextest/default/junit.xml`). The evidence names the load of the three runs (D251).
   As built: the bound was 2 s; the test waits out its deadline of 1 s and took 1.685 s, 2.028 s
   and 2.028 s at a load of 107 to 115, down from 10.455 s. 3 s is the bound that
   `story:test-step-holds-with-tls` uses for a changed test (D261).
5. With the option parsed and the default of 10 s kept in `serve` (planted in `process.rs` by
   hand, removed after, `git diff --stat` equal before and after), the test fails on its 5 s
   assertion after about 10 s, before the runner's 60 s, with the failure text `the next request
   waited 10.…s, five times the read deadline of 1 second is 5`; the text is in the evidence.
6. Statement 2: `aep plan artifact show story:agent-session-process` prints the option in its
   interface record, and the story's commit changes
   `.engineering/planning/story/agent-session-process.md` with a revision one higher. As built:
   `aep plan artifact history` lists moves and evidence and no change of a body, so the commit
   is the record.
7. Statement 3: `built::the_shipped_read_deadline_is_10_seconds` passes; with
   `wire::REQUEST_TIMEOUT` planted at 11 s (by hand, removed after) it fails.
8. Statement 4: `grep -n -- --request-timeout AGENTS.md` shows the sentence of the waits of
   `built` with 1 s.
9. "At most once": `other_argument_shapes_exit_2` also holds `--request-timeout 1
   --request-timeout 1 <identity> <socket>` and `--idle-timeout 1 --idle-timeout 1 <identity>
   <socket>`. Each exits 2, writes nothing on standard output and binds no socket, as the test
   asserts for every shape (`built.rs`:296-305). `surf-session` writes no usage line on a wrong
   shape today (`process.rs`:102-104 returns `USAGE_STATUS`), and this story adds none.
10. "Either order": a test of `built.rs` starts `surf-session --request-timeout 3 --idle-timeout 5
    <identity> <socket>` and the same with the two options in the reverse order. Each writes
    `ready`, and `surf_session::process::arguments` (`process.rs`:66) returns `request_timeout`
    3 s and `idle_timeout` 5 s for both orders. `cargo nextest run -p surf-session -E
    'test(/^built::/)'` on the story's commit passes.

## What it does not do

- No change of `wire::REQUEST_TIMEOUT` or of what `surfd` passes.
- It is not part of `story:test-step-holds-with-tls`'s step time or targets.
