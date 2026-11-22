---
title: Project status
description: Where surf stands, in the numbers its gate prints.
---

# Project status

surf is young, and not a browser for daily use. It loads one `http:` address from the command
line and prints what happened, and on Linux `surf-agent` drives a session over `http:`. It has no
window, no rendering, no JavaScript and no `https:` yet.

## Numbers

On 2026-11-22, with ess 0.97.3, after round 1, which follows wave 10 of the current plan:

| what | value |
|---|---|
| specification | ESS 0.97.3, valid |
| conformance scenarios | 169 of 169 pass, against the interpreter and against the Rust code |
| tests | 877 passed |
| agent tasks | 2 of 4 pass (`article`, `table`); `link` and `search` wait for later stories |
| live view's page tests | 43 passed, in Chromium's headless shell |
| URL test data | 872 of 904 cases pass; 32 skipped, listed by name |
| HTML parser test data | 9,107 cases counted, 9,095 run, 9,095 pass (82 encoding, 7,046 tokenizer and 1,967 tree construction cases); the parser decides the encoding, tokenizes, and builds a tree in every insertion mode, foreign content and fragments included, within six limits on untrusted input, and `surf` parses every `text/html` response with it; `task check` holds these counts |
| third-party crates in what ships | 0 |
| unsafe code | forbidden in every hand-written crate; two test files hold a counting allocator |
| standard sections marked complete | 0 |
| roadmap | 10 of 22 waves delivered; release `0.4.0` follows round 1 ([releases](https://github.com/beyond10x/surf/releases)) |

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
