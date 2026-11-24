---
format: aep.planning-md/3
id: story:scripts-parse-run
kind: story
status: draft
title: Run a page's inline scripts during its parse, with limits per document
summary: surf_script::run::parse_with_scripts joins the parser's script hook to swell, one agent per document, with the scripts-per-document, source-size, step-total and wall-clock limits, each tested, and every skipped script recorded as not run with its reason.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:html-script-hook
- depends_on: story:scripts-dom-reading
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/script/src/lib.rs
- confidence: cited
  path: crates/script/src/run.rs
- confidence: cited
  path: crates/script/tests/parse_run.rs
- confidence: cited
  path: docs/badges
- confidence: cited
  path: docs/design/scripts.md
- confidence: cited
  path: website/docs/how-it-is-built.md
revision: 9
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

What exists before it: the runner of `story:scripts-engine-dependency` (`ScriptEnd`, steps and
scripts totals, the wall-clock bound), the bindings and `DomCx` of `story:webidl-bindings`, the
reading slice and the document over a parsed tree of `story:scripts-dom-reading`, and the hook,
which reports each script element it runs and each it skips with its reason word
(`story:html-script-hook`, decision 1).

## Waits for the note

- waits for the note: Q4, the starting values of the per-document limits (scripts per document,
  bytes of one script's source, step total per document, wall-clock bound) and of the agent's
  limits;
- waits for the note: Q5, the thread a scripted document is parsed on, its stack size, and when
  the agent is dropped;
- waits for the note: Q1, the reason words of a script element that is not run.

## Decisions taken

1. `surf_script::run::parse_with_scripts(bytes, address, options, limits) -> ScriptedParse`:
   parses with scripting enabled on a thread with the stack of Q5, makes one agent with the
   bindings for the document, runs each script element the hook hands it, and answers the
   parsed document, the list of ends in document order (one per parser-inserted script element,
   run or not), and the per-document limit that stopped scripts, if one did. The agent is dropped
   when the parse ends, as Q5 says.
2. A skipped element the hook reports becomes the end `not run: <reason>` with Q1's word.
3. The per-document limits, each checked before a script runs or while it runs:
   - scripts per document: a script past the number is not run, end `not run: scripts-per-document`;
   - source bytes: a script whose text is longer is not run, end `not run: source-bytes`; the
     next script that fits runs;
   - step total per document: each entry runs with the smaller of `steps_per_entry` and what is
     left of the total; an entry that ends `Terminated(StepBudget)` on the total ends
     `limit: steps-per-document`;
   - wall clock per document: a timer sets the interrupt handle; the entry ends
     `limit: wall-clock`.
   After a per-document limit or the second memory end, every later script element ends
   `not run: <that limit>`, and parsing goes on to the end of the document.
4. The values of Q4 and Q5 were measured without bindings or DOM; this story measures them again
   with both (steps per second in the test profile and in release, the heap of a realm with the
   bindings, `Limits::min_stack_bytes()`) with probes in the session scratch directory, never
   committed, and changes the note's Q4 and Q5 where a value does not hold, each with its
   `probe:` line.

## Change

- `crates/script/src/run.rs` (new) and `crates/script/src/lib.rs`: decisions 1 to 3.
- `crates/script/tests/parse_run.rs`: the module `parse_run`, declared by
  `story:webidl-bindings`.
- `docs/design/scripts.md`: Q4 and Q5 as decision 4 finds them, only where a value changes.
- In the last commit, the public surfaces of acceptance 8.

## Acceptance

1. `scripts_per_document`: with the limit 2, a page of three inline scripts `1` ends `Ran`,
   `Ran`, `not run: scripts-per-document`, and the markup after the third is in the tree.
2. `source_bytes`: with the limit 100, a script of 101 bytes ends `not run: source-bytes` and a
   later script of 10 bytes ends `Ran`.
3. `steps_per_document`: with a per-entry limit that ends none of them and a document total that
   the second exceeds, three scripts of `for (let i = 0; i < 100000; i++) {}` end `Ran`,
   `limit: steps-per-document`, `not run: steps-per-document`.
4. `wall_clock`: with a bound of 100 ms and step limits that end nothing first, `while (true) {}`
   ends `limit: wall-clock` within 1 s; a later script ends `not run: wall-clock`; the markup after
   both is in the tree.
5. `skipped_reasons`: a page with `<script src=a.js>`, `<script type=module>`,
   `<script nomodule>` and an inline classic script ends `not run: src`, `not run: module`,
   `not run: nomodule`, `Ran` (the words as Q1 gives them).
6. `memory_twice`: two scripts that each allocate without end under a memory limit of 1 MiB, then
   `1`: the ends are the memory limit, the memory limit, `not run: memory`; with scripting
   disabled, `parse_with_scripts` answers the tree `surf_html` parses today and no end.
7. Decision 4: the report gives each re-measured value beside the note's, with its probe; `git
   diff --stat` shows `docs/design/scripts.md` only where a value changed.
8. Public surfaces: the story names every public surface its change makes false (a badge, a
   `README.md` line, a website page, a count) and changes it in its last commit, every number
   from the log of a gate run on its tip, which the coordinating agent orders. For this story,
   lines on `main` at `29945f5` (2026-11-24): the test count of `README.md:11` and `:285-286` and `docs/badges/make.sh:38` with
   `docs/badges/tests.svg`; the gate block of `README.md:584-625` and of
   `website/docs/how-it-is-built.md:40-81`.
9. `slot task check` exits 0; `git diff --stat` names only the files of the Change.
10. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; no model
    call and no network request in a test; each case parses a page of a few lines and makes one
    agent; no Python, the probes of decision 4 included; a probe that only measures runs without
    `slot`, one that builds through it; no generator with a committed output is added; a build or
    test job in the pool ends within 10 minutes or names its time with `slot --timeout`, at most
    30 minutes; a test over 5 s is named (`wall_clock` waits 100 ms by design).

## Out of scope

The members that write (`story:scripts-dom-mutation`, `story:scripts-dom-attributes`); the
harness of html5lib cases (`story:scripts-html-conformance`); a session
(`story:scripts-document-port`); a page meant to break the limits (`story:scripts-hostile-pages`).

## Placement

After `story:html-script-hook` and `story:scripts-dom-reading`; before
`story:scripts-dom-mutation`, `story:scripts-html-conformance` and `story:scripts-document-port`,
which call `parse_with_scripts`. Beside `story:scripts-dom-attributes` and
`story:scripts-dom-selectors`, which write other files of `crates/script`.

## Why `epic:scripts-in-a-page`

The epic's outcome runs a page's inline scripts during its parse, with limits per document; this
is that run.
