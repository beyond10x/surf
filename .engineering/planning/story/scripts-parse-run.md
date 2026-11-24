---
format: aep.planning-md/3
id: story:scripts-parse-run
kind: story
status: draft
title: Run a page's inline scripts during its parse, with limits per document
summary: surf_script::run::parse_with_scripts joins the parser's script hook to swell through the runner, one agent per document, and adds the per-document policy (scripts per document, source size, step total, wall clock, the stop after a second memory end), each tested, every end written with the note's words, and the values of Q4 and Q5 measured again.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:html-script-hook
- depends_on: story:scripts-dom-reading
scope:
- confidence: cited
  path: crates/script/src/lib.rs
- confidence: cited
  path: crates/script/src/run.rs
- confidence: cited
  path: crates/script/src/runner.rs
- confidence: cited
  path: crates/script/tests/parse_run.rs
- confidence: cited
  path: docs/design/scripts.md
revision: 13
---
## Context

The coordinating agent cut this story out of `story:scripts-dom-mutation`: the function that joins
the parser's script hook (`story:html-script-hook`) to swell, with the per-document limits, is a
product of its own, tested here rather than first in `story:scripts-hostile-pages`. The epic's
"Limits per page" (lines 225-229): the agent's limits, a step total per document summed from
`last_entry_steps()`, a number of scripts run per document, a wall-clock bound per document
through the interrupt handle, and every DOM write held to the parser's limits; "A limit reached
ends that script; parsing goes on, and the document records which limit." Scripts run only while
the document is parsed, one agent per document, dropped with it (epic lines 117-119, 218-219).
After `Terminated(MemoryBudget)` surf calls `collect()` once and runs no further script in that
document if the next entry ends so again (epic line 840).

What exists before it, and what this story only calls:

- the runner of `story:scripts-engine-dependency` (`crates/script/src/runner.rs`): the agent and
  its limits with the starting values of Q4, the thread with the stack of Q5, `ScriptEnd`, the
  step and script totals, the wall-clock timer on the interrupt handle, and the rule after
  `Terminated(MemoryBudget)` (one `collect()`, no further script after a second);
- the bindings and `DomCx` of `story:webidl-bindings`, and its rule in `crates/script/src/lib.rs`
  that maps an interrupted entry to the limit `DomCx` recorded, otherwise to `limit: wall-clock`;
- the reading slice and the document over a parsed tree (`story:scripts-dom-reading`);
- the hook, which reports each script element it runs and each it skips with a word of Q1's line
  `reasons:` (`story:html-script-hook`, decision 1).

The end words are the note's: the words of Q1's line `reasons:`, the `limit: <word>` and `not run:
<word>` of Q4's column `outcome when reached`, and Q7's closed list of ends, which holds all of
them. This story invents none; where the note spells a word otherwise than this text, the note's
spelling holds.

## Waits for the note

- waits for the note: Q4, the starting values of the per-document limits (scripts per document,
  bytes of one script's source, step total per document, wall-clock bound), the agent's limits,
  and the column `outcome when reached`;
- waits for the note: Q5, the thread a scripted document is parsed on, its stack size, and when
  the agent is dropped;
- waits for the note: Q1, the line `reasons:`; Q7, the closed list of ends.

## Decisions taken

1. `surf_script::run::parse_with_scripts(bytes, address, options, limits) -> ScriptedParse`:
   parses with scripting enabled on the runner's thread, makes one runner with the bindings for
   the document, runs each script element the hook hands it through the runner, and answers the
   parsed document, the list of ends in document order (one per parser-inserted script element,
   run or not), and the per-document limit that stopped scripts, if one did. The runner and its
   agent are dropped when the parse ends, as Q5 says.
2. This story re-implements none of the runner's mechanisms: the memory end and its one
   `collect()`, the wall-clock timer, the step and script totals and the mapping of an interrupted
   entry are the runner's and `story:webidl-bindings`' (epic lines 225-229, 840), called here. Its
   own part is the per-document policy and its tests:
   - a skipped element the hook reports becomes the end `not run: <its word of Q1>`;
   - scripts per document: a script past the number is not run, end `not run:
     scripts-per-document`;
   - source bytes: a script whose text is longer is not run, end `not run: source-bytes`; the
     next script that fits runs;
   - step total per document: each entry runs with the smaller of `steps_per_entry` and what is
     left of the runner's step total; an entry that ends `Terminated(StepBudget)` on the total
     ends `limit: steps-per-document`;
   - wall clock per document: the runner's timer is set once for the document;
   - after a document limit (`steps-per-document`, `wall-clock`) or the runner's stop after a
     second memory end, every later script element ends `not run: <that word>`, and parsing goes
     on to the end of the document. A limit of the DOM (a node, depth, attribute or stored-byte
     limit) ends only the script that reached it: a later script of the document runs.
3. The values of Q4 and Q5 were measured without bindings or DOM; this story measures them again
   with both (steps per second in the test profile and in release, the heap of a realm with the
   bindings, `Limits::min_stack_bytes()`) with probes in the session scratch directory, never
   committed. A value holds if a script at that limit ends with that limit's end word. Where one
   does not hold, the story changes it where it lives in code (the agent's limits, the wall-clock
   bound and the stack in `crates/script/src/runner.rs`; the per-document limits in
   `crates/script/src/run.rs`) and in the note's Q4 or Q5, with its `probe:` line.

## Change

- `crates/script/src/run.rs` (new) and `crates/script/src/lib.rs`: decisions 1 and 2.
- `crates/script/src/runner.rs`: only a value that decision 3 changes.
- `crates/script/tests/parse_run.rs`: the module `parse_run`, declared by
  `story:webidl-bindings`.
- `docs/design/scripts.md`: Q4 and Q5 as decision 3 finds them, only where a value changes.
- In the last commit, the public surfaces of acceptance 11.

## Acceptance

1. `scripts_per_document`: with the limit 2, a page of three inline scripts `1` ends `Ran`,
   `Ran`, `not run: scripts-per-document`, and the markup after the third is in the tree.
2. `source_bytes`: with the limit 100, a script of 101 bytes ends `not run: source-bytes` and a
   later script of 10 bytes ends `Ran`.
3. `steps_per_document`: the test runs `for (let i = 0; i < 100000; i++) {}` once and reads its
   `last_entry_steps()`; with a per-entry limit that ends none of the scripts and a document total
   of 1.5 times that count, a page of that script three times ends `Ran`,
   `limit: steps-per-document`, `not run: steps-per-document`, and the returned limit is
   `steps-per-document`.
4. `wall_clock`: with a bound of 100 ms and step limits that end nothing first, `while (true) {}`
   ends `limit: wall-clock`, the elapsed time under 10 s; a later script ends `not run:
   wall-clock`; the returned limit is `wall-clock`; the markup after both is in the tree.
5. `skipped_reasons`: a page with `<script src=a.js>`, `<script type=module>`, `<script nomodule>`
   and an inline classic script ends `not run: src`, `not run: module`, `not run: nomodule`, `Ran`
   (the words of Q1's line `reasons:`).
6. `memory_twice`: under a memory limit of 1 MiB, two scripts `{ const a = []; for (;;)
   a.push(new Array(1000)); }` and then `1` end `limit: memory`, `limit: memory`, `not run:
   memory`; the returned limit is `memory`.
7. `scripting_off_same_tree`: with scripting disabled, `parse_with_scripts` answers the tree
   `surf_html` parses today and no end.
8. `dom_limit_then_next_runs`: the mutation slice comes after this story, so the test defines on the
   global a host function `reachNodeLimit` that does what the generated glue of
   `story:webidl-bindings` does for `DomEnd::Limit` of the node limit; a first script
   `reachNodeLimit(); 1` ends `limit: <the node limit's word of Q4>`, also inside `try { ... }
   catch {}`, and a later script `1` of the same document ends `Ran`.
9. `frame_depth_on_parse_thread`: `function f(n) { return 1 + f(n + 1); } f(0)` through
   `parse_with_scripts` at the `frame_depth` of Q4 ends `Threw` with kind `RangeError`, without a
   crash.
10. Decision 3: the report lists, per row of Q4 and Q5, `holds` or `<old> -> <new>` with its
    `probe:` line; `git diff --stat` names `docs/design/scripts.md` and
    `crates/script/src/runner.rs` only if a value changed.
11. Public surfaces: the story changes, in its last commit and after merging `main`, every
    public surface its change makes false, each number taken from the `slot task check` log of
    its tip. After that commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges`
    prints nothing for each old string below, as the story's base holds it (the strings and lines
    given are those of `main` at `29945f5`, 2026-11-24); each new number equals the one in that log;
    and the gate block, `README.md:584-625` and `website/docs/how-it-is-built.md:40-81`, holds that
    log's lines. The old strings of this story: the test count `1976 passed` (`README.md:11`,
    `docs/badges/make.sh:38`, `docs/badges/tests.svg`), `1,976` and `1,424, and 552`
    (`README.md:285`, `website/docs/status.md:23`, `website/docs/what-works.md:17`).
12. `slot task check` exits 0; `git diff --stat` names only the files of the Change.
13. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; no model
    call and no network request in a test; each case parses a page of a few lines and makes one
    agent; no Python, the probes of decision 3 included; a probe that only measures runs without
    `slot`, one that builds through it; no generator with a committed output is added; a build or
    test job in the pool ends within 10 minutes or names its time with `slot --timeout`, at most
    30 minutes; a test over 5 s is named (`wall_clock` waits 100 ms by design).

## Out of scope

The members that write (`story:scripts-dom-mutation`, `story:scripts-dom-attributes`); the
runner's mechanisms (`story:scripts-engine-dependency`); the harness of html5lib cases
(`story:scripts-html-conformance`); a session (`story:scripts-document-port`); a page meant to
break the limits (`story:scripts-hostile-pages`).

## Placement

After `story:html-script-hook` and `story:scripts-dom-reading`; before
`story:scripts-dom-mutation`, `story:scripts-html-conformance` and `story:scripts-document-port`,
which call `parse_with_scripts`. Beside `story:scripts-dom-attributes` and
`story:scripts-dom-selectors`, which write other files of `crates/script`.
`crates/script/src/runner.rs` is `story:scripts-engine-dependency`'s, merged before this story.

## Why `epic:scripts-in-a-page`

The epic's outcome runs a page's inline scripts during its parse, with limits per document; this
is that run.
