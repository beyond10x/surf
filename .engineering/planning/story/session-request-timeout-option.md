---
format: aep.planning-md/3
id: story:session-request-timeout-option
kind: story
status: draft
title: surf-session takes --request-timeout for a test that starts it
summary: request_slower_than_its_deadline_is_closed waits the 10 s read deadline in every run; an option of the form of --idle-timeout lets the test set 1 s
relations:
- decomposes: epic:agent-cli-target
- informed_by: story:test-step-holds-with-tls
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/session/src/process.rs
- confidence: cited
  path: crates/session/tests/built.rs
revision: 4
---
## Why

`crates/session/tests/built.rs` `request_slower_than_its_deadline_is_closed` waits out the
session's read deadline, `wire::REQUEST_TIMEOUT` (10 s, a constant): 10.3 to 11.8 s in every gate
and alone (2026-11-22, load 88 to 218), the same at every load. The folder's rule wants tests of a
minute or two in total, and `story:test-step-holds-with-tls` lists it as class (b), a wait on a
clock. A test that starts the binary itself can already shorten the idle timeout
(`--idle-timeout`); the read deadline has no such option.

## Change

1. `crates/session/src/process.rs`: `surf-session` takes `--request-timeout <seconds>`, a whole
   number from 1, of the form of `--idle-timeout`, for a test that starts the binary itself;
   `surfd` passes neither. The usage line at `process.rs`:1 names `[--request-timeout
   <seconds>]`; `Arguments` gains the field; the default stays `wire::REQUEST_TIMEOUT`.
2. The interface record of `story:agent-session-process` (its body at line 135: `surf-session
   [--idle-timeout <seconds>] <identity> <socket path>`) is amended through `aep plan artifact
   body` to name the option.
3. `crates/session/tests/built.rs`: `request_slower_than_its_deadline_is_closed` starts the
   session with `--request-timeout 1` and keeps its bound at five times the deadline (5 s); one
   assertion compares `wire::REQUEST_TIMEOUT` with 10 s. `other_argument_shapes_exit_2` gains
   `--request-timeout 0` and `--request-timeout` without a number.
4. `AGENTS.md` :1272-1283 (the 10 s wait of `built`) states the option and the new time.

## Acceptance

1. `other_argument_shapes_exit_2` holds `--request-timeout 0` and `--request-timeout` without a
   number, and passes.
2. `sed -n 1p crates/session/src/process.rs` names `[--request-timeout <seconds>]`.
3. `git grep request-timeout crates/surfd` prints nothing.
4. `built::request_slower_than_its_deadline_is_closed` takes under 2 s in three runs of `cargo
   nextest run -p surf-session` at a load of at least 100, loads in the evidence.
5. With the deadline not applied (planted in `process.rs`, removed after), the test goes red on
   its own assertion before the runner's 60 s.

## What it does not do

- No change of `wire::REQUEST_TIMEOUT` or of what `surfd` passes.
- It is not part of `story:test-step-holds-with-tls`'s step time or targets.
