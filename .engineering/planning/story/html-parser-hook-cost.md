---
format: aep.planning-md/3
id: story:html-parser-hook-cost
kind: story
status: draft
title: Count what the script hook's unit costs a parse without a hook, in instructions
summary: The parser on main after story:html-script-hook is 3.0 to 7.0 % slower in wall time than before it on three pages, measured on a host that counts no instructions and with a spread of up to 24 % inside one job; this story counts instructions for the same pages before and after the unit, names the functions that hold the difference, and removes what can be removed without changing a step count or a limit.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:html-script-hook
scope:
- confidence: inferred
  path: crates/html/src/tree.rs
- confidence: inferred
  path: crates/html/src/tree_builder.rs
- confidence: inferred
  path: docs/design/html-parser.md
revision: 2
---
## Context

- `story:html-script-hook` changed the parser for every parse, with a hook or without: the depth
  limit bounds the tree, the stack's changes are counted as steps, and the step limit is read
  inside a token (its "As built").
- The sixth pass of its review measured the unit's tip of fix round 5 against `main` before the
  unit, release build, default limits, no hook, 20 parses per run, wall time, the lowest of five
  runs: `dom.html` 2,278.4 ms against 2,426.5 ms (+6.5 %), `webidl.html` 2,262.3 ms against
  2,420.2 ms (+7.0 %), `aaa:250:540000` 3,146.7 ms against 3,240.8 ms (+3.0 %). The spread inside
  one job reached 24 % (`dom.html` on `main`, 2,278 to 2,814 ms). The build node counts no
  instructions (`perf instructions:u supported: 0`), so wall time was the only measure.
- The last instruction count of the unit is older: at its fix round 3, `perf stat -e
  instructions:u`, 20 parses, `namedEntities.test` 14,553,163,938 on `main` against
  14,397,645,760 (-1.07 %), one parse of `dom.html` 1,333,721,325 against 1,327,156,558
  (-0.5 %), and the adoption agency pages `aaa:250:250000` and `aaa:60:1000000` at 1.042 times
  `main`'s instructions. Fix rounds 4, 5 and 6 came after it: the depth of a node read by a
  counted walk, the stack's moves counted, the step limit read before each entry of a scan and
  before each write.
- The review's ruling on the finding: no code change in the unit; the measurement is this
  story's.
- Open before this story is proposed: where it is measured. The build node counts no
  instructions, and no load test or benchmark runs on the shared workstation. The story needs a
  host that counts user-space instructions and on which a measurement is allowed; which host is
  the operator's to name.

## Change

- A measurement, kept as a file of the design notes: for `dom.html`, `webidl.html`,
  `namedEntities.test`, the joined `.dat` inputs and `aaa:250:540000`, the instructions of 20
  parses without a hook on the commit before the unit's merge and on the commit of its merge,
  each counted three times, with the commands.
- For each page whose count rose by more than 2 %: the functions that hold the rise (a profile
  by instructions), and a change that removes it where one exists without changing the steps a
  parse counts, the limit a parse stops at, or the tree it builds.

## Acceptance

1. The file names, per page, the two counts and their ratio, and the three counts of each agree
   within 0.1 %.
2. After the story, no page's count is more than 2 % above the count before the unit, or the
   file names the function that holds the rest and why it stays.
3. The steps of the five pages of `story:html-script-hook`'s "As built" are unchanged, and
   `slot task html-conformance` prints the case counts it prints on `main`.
4. `slot task check` exits 0; no test over 60 s, the test step within 120 s; the measurement is
   no test and no gate step; no Python.

## Out of scope

- The cost of a parse with a hook, and of the write surface.
- A benchmark in the gate.

## Placement

After `story:html-script-hook`. It blocks no story of `epic:scripts-in-a-page`.

## Why `epic:scripts-in-a-page`

The cost came with the epic's hook; the epic's parse of a page runs on this parser.
