---
title: Project status
description: Where surf stands, in the numbers its gate prints.
---

# Project status

surf is young, and not a browser for daily use. It loads one address from the command line and
prints what happened, and on Linux `surf-agent` drives a session. It loads `http:`
addresses, and `https:` ones since release `0.7.0`; since release `0.8.0` an agent session
keeps cookies; since release `0.9.0` `surfd` starts each session with an empty environment
and names the signal a session died by, and since release `0.10.0` a session it starts through
`setpriv` and `prlimit` has a core file size limit of 1 byte. It has no window, no rendering
and no JavaScript yet.

## Numbers

On 2026-11-24, with ess 0.97.4, after round 2, which follows round 1 and wave 10 of the current
plan, after the live view's control states and changed panel, after TLS 1.3 and `https:`, and
after the cookie jar and `surf-agent help --for-agent`, and with the Web IDL reader
`surf-webidl` (the gate run on `91f7ff1`, its tests counted again after it), and with the
specification of scripts' ends (`story:scripts-spec`, its scenarios counted on its tip), and
with the generated DOM bindings (`story:webidl-bindings`, its tests counted by the gate run on
its tip `5dcedfb` on 2026-11-25), and with the HTML parser's script hook
(`story:html-script-hook`, its tests counted by the gate run on its tip `58d720b` on
2026-11-25), and with the DOM's read members in the script runner of `surf-script`, which no
command runs yet (`story:scripts-dom-reading`, its tests counted by the gate run on its tip
`5664184` on 2026-11-26), and with the agent tasks' wait for the processes a model left
(`story:agent-tasks-left-process-wait`, which changes tests only, its tests counted by the gate
run on its tip `e61d766` on 2026-11-26), and with the core limit of 1 byte for a session
process (`story:session-core-limit-one`) and the gate step that refuses a new bound on elapsed
time in a test (`story:tests-without-wall-time-bounds`, its tests counted by the gate run on
`52ebbd7`, the tip of this story with the two before it merged into it, on 2026-11-26), and
with the script engine's pin moved from swell `0.8.0` to `0.10.0` (`story:swell-0-9-0`, its
tests counted by the gate run on its tip `6b4593f`, with `main` merged into it, on
2026-11-26), and with the DOM's nine selector members in the script runner of `surf-script`,
which no command runs yet (`story:scripts-dom-selectors`; the table's test counts are those of
the gate run on its tip `b5c2fc6`, with `main` merged into it, on 2026-11-26):

| what | value |
|---|---|
| specification | ESS 0.97.4, valid |
| conformance scenarios | 221 of 221 pass, against the interpreter and against the Rust code |
| tests | 2,665 passed and 2 ignored: 2,113 passed and the 2 ignored tests of `surf-script`, and 552 of the TLS crates |
| agent tasks | 6 of 6 pass (`article`, `table`, `link`, `search`, `https`, `login`) |
| live view | the panel "Now" lists a page's links, buttons and fields with their state; the panel `Changed by <action>` shows what the action changed; a typed password is shown as a count of bytes, never its value |
| live view's page tests | 72 passed, in Chromium's headless shell |
| X.509 test data (x509-limbo) | 5,143 cases counted, 5,089 run, 5,060 pass; `task check` holds these counts |
| URL test data | 872 of 904 cases pass; 32 skipped, listed by name |
| HTML parser test data | 9,107 cases counted, 9,095 run, 9,095 pass (82 encoding, 7,046 tokenizer and 1,967 tree construction cases); the parser decides the encoding, tokenizes, and builds a tree in every insertion mode, foreign content and fragments included, within six limits on untrusted input, and `surf` parses every `text/html` response with it; `task check` holds these counts |
| third-party crates in what ships | 0 up to release `0.6.0`; since release `0.7.0`, ring 0.17.14 and the four crates it needs, for the cryptographic primitives of TLS, through `surf-tls` alone |
| unsafe code | forbidden in every hand-written crate; three test files hold a counting allocator |
| standard sections marked complete | 0 |
| roadmap | 10 of 22 waves delivered, and round 2 delivered the step after wave 18 (follow, fill, press, back, forward, what changed); release `0.4.0` follows round 1, release `0.5.0` round 2, release `0.6.0` the live view's control states, release `0.7.0` TLS 1.3 and `https:`, release `0.8.0` the cookie jar and `surf-agent help --for-agent`, release `0.9.0` the hardened start of a session process and release `0.10.0` the core limit of a session process ([releases](https://github.com/beyond10x/surf/releases)) |

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
