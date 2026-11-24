---
format: aep.planning-md/3
id: story:scripts-html-conformance
kind: story
status: proposed
title: The four DOM-only needs-script parsing cases run and pass
summary: The conformance harness runs the needs-script cases that use no document.write through parse_with_scripts and prints 9107 cases, 9099 run, 9099 passed; not-run:needs-script counts the 3 that do; surf-html-conformance joins the closed list of surf-script dependents.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-dom-mutation
- depends_on: story:scripts-dom-attributes
- depends_on: story:scripts-dom-selectors
- depends_on: story:html-tree-tables
- depends_on: story:scripts-parse-run
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/html-conformance/Cargo.toml
- confidence: cited
  path: crates/html-conformance/expected
- confidence: cited
  path: crates/html-conformance/src
- confidence: cited
  path: docs/design/html-parser.md
- confidence: cited
  path: taskfiles/lock-owners.txt
- confidence: cited
  path: website/docs/status.md
- confidence: cited
  path: website/docs/what-works.md
revision: 20
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:13Z", actor: "human:operator", revision: 16}
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
implemented. `task html-conformance` prints `html-conformance: 9107 cases, 9095 run, 9095
passed`. The crate `surf-html-conformance` depends on `surf-html` only
(`crates/html-conformance/Cargo.toml:24-25`).

## Decisions taken

1. A case tagged `needs-script` that does not use `document.write` runs through
   `surf_script::run::parse_with_scripts` (`story:scripts-parse-run`) with parser scripting
   enabled; `tags.rs`'s rule keeps the three that use it `not-run:needs-script`. The 14
   `#script-on` cases already run with the flag false and pass (`docs/design/scripts.md`, Q3);
   their expectations do not change.
2. `surf-html-conformance` depends on `surf-script` (a tooling crate that ships nothing).
   `taskfiles/lock-owners.txt`, which `story:scripts-engine-dependency` writes
   (`docs/design/scripts.md`, Q15), gains the line `surf-script surf-html-conformance` in this
   story's change: the second entry of the closed list of Q14.

## Change

- `crates/html-conformance/src/`: decision 1.
- `crates/html-conformance/Cargo.toml` and `Cargo.lock`: decision 2 (cargo writes the lock).
- `taskfiles/lock-owners.txt`: the line `surf-script surf-html-conformance` (decision 2).
- `crates/html-conformance/expected/`: the four cases counted as run and passed.
- `docs/design/html-parser.md`: the `html-inventory` block's tags for those files, the
  `html-sections` marks as the note's Q2 leaves them, and the counts of lines 677 and 731.
- In the last commit, the public surfaces of acceptance 6.

## Acceptance

1. `slot task html-conformance` prints `html-conformance: 9107 cases, 9099 run, 9099 passed` (the
   total of `main`, four cases more run), and lists the four cases of `scripted_adoption01.dat`,
   `scripted_ark.dat` and `scripted_foster01.dat` as run and passed.
2. `not-run:needs-script` counts 3 cases, each one that uses `document.write`
   (`scripted_webkit01.dat` 2, `encoding/scripted/tests1.dat` 1); every other case keeps its
   result.
3. `slot task lock-check` passes with the line `surf-script surf-html-conformance` in
   `taskfiles/lock-owners.txt`, and its test that a crate without a line depending on `surf-script`
   fails still passes.
4. `slot task check` exits 0.
5. `git diff --stat` names only the files of the Change.
6. Public surfaces: the story changes, in its last commit and after merging `main`, every public
   surface its change makes false, each number taken from the `slot task check` log of its tip.
   After that commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges` prints
   nothing for each old string below, as the story's base holds it (the strings and lines given are
   those of `main` at `29945f5`, 2026-11-24); each new number equals the one in that log; and the
   gate block, `README.md:584-625` and `website/docs/how-it-is-built.md:40-81`, holds that log's
   lines. The old strings of this story: the parser counts, `9095 run` (`README.md:616`,
   `website/docs/how-it-is-built.md:72`), `9,095 run` (`website/docs/status.md:29`), `runs 9,095`
   and `9,095: 82` (`README.md:509-510`, `website/docs/what-works.md:265-266`) and
   `1,967 tree construction` (`README.md:510`, `website/docs/status.md:29`,
   `website/docs/what-works.md:266`); the test count `1976 passed` (`README.md:11`,
   `docs/badges/make.sh:38`, `docs/badges/tests.svg`), `1,976` and `1,424, and 552`
   (`README.md:285`, `website/docs/status.md:23`, `website/docs/what-works.md:17`).
7. The folder's bounds hold: the test step and `task html-conformance` each end within 120 s and no
   test runs over 60 s; no model call and no network request; one agent per case for the four
   cases, nothing else added per case; no Python; no generator with a committed output is added; a
   build or test job in the pool ends within 10 minutes or names its time with `slot
   --timeout`, at most 30 minutes; a test over 5 s is named.

## Out of scope

`document.write` and the three cases that need it; web-platform-tests files of the DOM (epic
lines 764-772).

## Placement

After `story:scripts-dom-mutation`, `story:scripts-dom-attributes`, `story:scripts-dom-selectors`
and `story:scripts-parse-run`; before `story:scripts-document-port`, which writes `Cargo.lock`,
the closed list and `crates/script/Cargo.toml` after it (its edge).

## Why `epic:scripts-in-a-page`

The epic's acceptance 4: the standard's own parsing cases that need a script run and pass.
