---
format: aep.planning-md/3
id: story:scripts-html-conformance
kind: story
status: draft
title: The four DOM-only needs-script parsing cases run and pass
summary: The conformance harness runs the needs-script cases that use no document.write through surf-script; not-run:needs-script counts the 3 that do.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-dom-mutation
- depends_on: story:scripts-dom-attributes
- depends_on: story:scripts-dom-selectors
- depends_on: story:html-tree-tables
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: cited
  path: crates/html-conformance/Cargo.toml
- confidence: cited
  path: crates/html-conformance/expected
- confidence: cited
  path: crates/html-conformance/src
- confidence: cited
  path: docs/design/html-parser.md
revision: 4
---
## Context

The epic's twelfth candidate (lines 472-475) and its acceptance 4 (lines 577-578). The tree on
`main` of 2026-11-24: every case of a file `wpt/html/syntax/parsing/resources/scripted_*` and of
`html5lib-tests/encoding/scripted/` is `not-run:needs-script`
(`crates/html-conformance/src/tags.rs:73-80`); the expected counts are
`scripted_adoption01.dat` 1, `scripted_ark.dat` 1, `scripted_foster01.dat` 2 and
`scripted_webkit01.dat` 2 (`crates/html-conformance/expected/tree-construction.txt:25-28`) and
`encoding/scripted/tests1.dat` 1 (`expected/encoding.txt:2`): 7 cases. Six of them are files of
web-platform-tests, not of html5lib (the epic, line 93, says html5lib). Four need only a DOM
(epic lines 387-394); three need `document.write` and stay not run. `story:html-tree-tables` is
implemented.

## Change

- `crates/html-conformance/src/`: a case that is `needs-script` and does not use
  `document.write` runs through the parse of `surf-script` that runs scripts
  (`story:scripts-dom-mutation`), with parser scripting enabled; `tags.rs`'s rule keeps the other
  three `not-run:needs-script`.
- `crates/html-conformance/Cargo.toml`: `surf-script` as a dependency (a tooling crate); `Cargo.lock`
  gains the edge. `story:scripts-document-port` writes the lock too: the second to merge takes the
  other's entries (cargo writes the lock, nobody edits it).
- `crates/html-conformance/expected/`: the four cases counted as run and passed.
- `docs/design/html-parser.md`: the `html-inventory` block's tags for those files and the
  `html-sections` marks as the note's Q2 leaves them.

## Acceptance

`slot task check` exits 0. The folder's bounds hold: the test step and `task html-conformance`
each end within 120 s and no test runs over 60 s; no model call and no network request; one realm
per case for the four cases, nothing else added per case; no Python; no generator with a committed output is added (one that were would carry a digest of its sources, a fast check in the gate and full regeneration in the lane `slow`); a build or test job in the pool ends within 10 minutes or names its time with `slot --timeout`, at most 30 minutes; a test over 5 s is named.

1. `slot task html-conformance` prints the four cases of `scripted_adoption01.dat`,
   `scripted_ark.dat` and `scripted_foster01.dat` as run and passed.
2. `not-run:needs-script` counts 3 cases, each one that uses `document.write`
   (`scripted_webkit01.dat` 2, `encoding/scripted/tests1.dat` 1).
3. The total count of cases is unchanged; every other case keeps its result.
4. `git diff --stat` names only the files of the Change and `Cargo.lock`.

## Out of scope

`document.write` and the three cases that need it; web-platform-tests files of the DOM (epic
lines 764-772).

## Why `epic:scripts-in-a-page`

The epic's acceptance 4: the standard's own parsing cases that need a script run and pass.
