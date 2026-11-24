---
format: aep.planning-md/3
id: story:scripts-hostile-pages
kind: story
status: draft
title: Hostile pages end at a recorded limit with the rest of the page parsed, and a session at its process bounds fails as a script
summary: 'Endless loop, allocation and node creation, deep trees and recursion, 10,000 scripts, an attribute and a source over their limits: each ends with its limit recorded within 5 s; a session at an operating-system bound fails as the script, not as surfd.'
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-document-port
scope:
- confidence: inferred
  path: crates/script/tests
- confidence: cited
  path: crates/session/tests
- confidence: cited
  path: crates/surf/tests
revision: 3
---
## Context

The epic's fourteenth candidate (lines 486-492) and its acceptance 6 (lines 583-584), with the
constraint "A script never hangs, panics or exhausts the session process" (lines 553-555). The
limits and their recorded outcomes are the note's Q4; the session's bounds are those of
`story:session-process-hardening` (S5 = a assumed). The epic lists `crates/surf/tests/`; by S7
no script runs outside a session, so the pages are served to sessions, and the library cases go
to `surf-script`'s tests.

## Change

- `crates/session/tests/`: a module `hostile_pages`, each case a page served on 127.0.0.1 to a
  session opened with the option.
- `crates/script/tests/`: the same pages through the parse that runs scripts, without a session,
  where a case needs a bound the session does not show.
- `crates/surf/tests/`: only if a case of the parser step needs it.

## Acceptance

`slot task check` exits 0. The folder's bounds hold: the test step ends within 120 s and no test
runs over 60 s; every case here ends within 5 s, its time in the report; the servers start in Rust
on 127.0.0.1; no model call; one session per case; no Python; no generator with a committed output is added; a build or test job in the pool ends within 10 minutes or names its time with `slot --timeout`, at most 30 minutes.

1. Each page ends with its limit recorded as the script's end and the page's other elements
   present in the document's rows: an endless loop (step limit, or the wall-clock bound); an
   endless allocation (memory, then no further script after the second, epic line 840); endless
   node creation (node limit); a tree built deep by `appendChild` (depth); deep recursion
   (`frame_depth`); 10,000 script elements (scripts per document); an attribute over the limit
   (attributes or stored bytes); a script over the source limit (not run, the reason named).
2. A session whose process reaches a bound of the operating system (the address space of Q11)
   ends as a failure of the script in its answer and its log, not of `surfd`: `surfd` keeps
   answering, and another session opened after it loads a page.
3. `git diff --stat` names only the files of the Change.

## Out of scope

Limits the first version does not set; fuzzing.

## Why `epic:scripts-in-a-page`

The epic's acceptance 6 and the first condition's purpose: a hostile page costs a bounded amount
and the rest of the page is still read.
