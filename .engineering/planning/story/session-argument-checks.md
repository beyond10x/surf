---
format: aep.planning-md/3
id: story:session-argument-checks
kind: story
status: draft
title: surf-session refuses a relative socket path, and its wrong-shape test can fail on a bound socket
summary: 'Three findings of the adversary pass on the request timeout option that the option did not introduce: a check that cannot fail, a relative socket path accepted, and two sentences of the usage text'
relations:
- decomposes: epic:agent-cli-target
- informed_by: story:session-request-timeout-option
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/session/src/process.rs
- confidence: cited
  path: crates/session/tests/built.rs
revision: 2
---
## Why

The adversary pass on `story:session-request-timeout-option` (2026-11-22, commit `d82eb1c`)
returned three findings that the option did not introduce. None of them is reached by `surfd`,
which starts `surf-session` with an absolute socket path and no option.

1. `crates/session/tests/built.rs`, the check "binds no socket" of
   `other_argument_shapes_exit_2` cannot fail: `run` (`crates/session/src/process.rs`) removes
   its socket file before it exits, and with standard input closed a wrongly accepted shape
   exits at once. Only the exit status catches such a shape.
2. `surf-session <identity> --request-timeout` is accepted with `--request-timeout` as a
   relative socket path: the session binds that file in its working directory. Any relative
   socket path is accepted the same way.
3. The usage text and the interface record of `story:agent-session-process` say "a whole number
   from 1" without an upper bound; `whole_seconds` parses a `u32`, so a value over 4,294,967,295
   exits 2. The record says each option "shortens" its deadline; a value over the default
   lengthens it.

## Change

1. The test holds "binds no socket" by a check that can fail: the socket path is not created
   while the process runs with its standard input held open, read before the input is closed.
2. `surf-session` refuses a socket path that is not absolute, with exit 2, without `ready`.
   A test holds a relative path and an option name in the place of the path.
3. The usage line, `AGENTS.md` and the interface record name the upper bound and say that an
   option sets its deadline.

## Acceptance

1. With `arguments` planted to accept `--request-timeout 0` and to serve (by hand, removed
   after), the "binds no socket" check of statement 1 fails on its own assertion.
2. `surf-session <identity> relative.sock` and `surf-session <identity> --request-timeout` each
   exit 2 and write nothing on standard output; no file is created in the working directory.
3. `cargo nextest run -p surf-session` passes; each new test takes under 3 s.

## What it does not do

- No change of what `surfd` passes or of a deadline's default.
