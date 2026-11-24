---
title: Project status
description: Where surf stands, in the numbers its gate prints.
---

# Project status

surf is young, and not a browser for daily use. It loads one address from the command line and
prints what happened, and on Linux `surf-agent` drives a session. It loads `http:`
addresses, and `https:` ones since release `0.7.0`; since release `0.8.0` an agent session
keeps cookies. It has no window, no rendering and no JavaScript yet.

## Numbers

On 2026-11-24, with ess 0.97.4, after round 2, which follows round 1 and wave 10 of the current
plan, after the live view's control states and changed panel, and after TLS 1.3 and `https:`:

| what | value |
|---|---|
| specification | ESS 0.97.4, valid |
| conformance scenarios | 203 of 203 pass, against the interpreter and against the Rust code |
| tests | 1,976 passed: 1,424, and 552 of the TLS crates |
| agent tasks | 6 of 6 pass (`article`, `table`, `link`, `search`, `https`, `login`) |
| live view | the panel "Now" lists a page's links, buttons and fields with their state; the panel `Changed by <action>` shows what the action changed; a typed password is shown as a count of bytes, never its value |
| live view's page tests | 71 passed, in Chromium's headless shell |
| X.509 test data (x509-limbo) | 5,143 cases counted, 5,089 run, 5,060 pass; `task check` holds these counts |
| URL test data | 872 of 904 cases pass; 32 skipped, listed by name |
| HTML parser test data | 9,107 cases counted, 9,095 run, 9,095 pass (82 encoding, 7,046 tokenizer and 1,967 tree construction cases); the parser decides the encoding, tokenizes, and builds a tree in every insertion mode, foreign content and fragments included, within six limits on untrusted input, and `surf` parses every `text/html` response with it; `task check` holds these counts |
| third-party crates in what ships | 0 up to release `0.6.0`; since release `0.7.0`, ring 0.17.14 and the four crates it needs, for the cryptographic primitives of TLS, through `surf-tls` alone |
| unsafe code | forbidden in every hand-written crate; three test files hold a counting allocator |
| standard sections marked complete | 0 |
| roadmap | 10 of 22 waves delivered, and round 2 delivered the step after wave 18 (follow, fill, press, back, forward, what changed); release `0.4.0` follows round 1, release `0.5.0` round 2, release `0.6.0` the live view's control states, release `0.7.0` TLS 1.3 and `https:` and release `0.8.0` the cookie jar and `surf-agent help --for-agent` ([releases](https://github.com/beyond10x/surf/releases)) |

## What is not checked yet

The specification was drafted by reasoning over the page load. Its names, lifecycle states and
command boundaries are not yet checked against the standards it follows.

The conformance reports say `conformance_status: inconclusive` with `execution_status: passed`:
the suite carries no coverage inventory, and ess 0.96.0 never qualifies a run over unknown
coverage. The gate reads the counts.

## Where things are

| path | what it holds |
|---|---|
| [`spec/`](https://github.com/beyond10x/surf/tree/main/spec) | the specification |
| [`generated/rust/`](https://github.com/beyond10x/surf/tree/main/generated/rust) | the components, generated from `spec/`, never edited by hand |
| [`crates/`](https://github.com/beyond10x/surf/tree/main/crates) | the hand-written crates |
| [`conformance/`](https://github.com/beyond10x/surf/tree/main/conformance) | the runner that holds the Rust code to the scenarios |
| [`docs/surf-command.md`](https://github.com/beyond10x/surf/blob/main/docs/surf-command.md) | the `surf` command: output format, exit statuses, limits |
| [`docs/design/`](https://github.com/beyond10x/surf/tree/main/docs/design) | accepted designs |
| [`AGENTS.md`](https://github.com/beyond10x/surf/blob/main/AGENTS.md) | the working rules, written for the coding agents that build surf |
