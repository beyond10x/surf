---
format: aep.planning-md/3
id: story:scripts-hostile-pages
kind: story
status: draft
title: Hostile pages end at a recorded limit with the rest of the page parsed, and a session at its process bounds is listed died with the bound named
summary: 'Tests only: an endless loop, many small scripts, a wall-clock-only loop, allocation, node creation, deep trees and recursion, 10,000 scripts, an attribute and a source over their limits each end with their limit within 5 s; a scripted session at an address-space bound is listed died with the bound named and surfd goes on.'
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-document-port
scope:
- confidence: cited
  path: crates/script/tests
- confidence: cited
  path: crates/session/tests
revision: 7
---
## Context

The epic's fourteenth candidate (lines 486-492) and its acceptance 6 (lines 583-584), with the
constraint "A script never hangs, panics or exhausts the session process" (lines 553-555) and
"limits per script, per document and in wall time, each ending in a defined outcome" (lines
90-92, 225-229). The limits and their recorded outcomes are the note's Q4 and
`story:scripts-parse-run`'s; the session's bounds of the operating system, and what a client is
answered when a session ends at one, are `story:session-process-hardening`'s (S5 decided a by the
operator on 2026-11-24). The epic lists `crates/surf/tests/`; by S7 no script runs outside a
session, so the pages are served to sessions, and the library cases go to `surf-script`'s tests.
This story adds tests only: every behaviour it shows is written by an earlier story.

## Change

- `crates/session/tests/`: a module `hostile_pages`, each case a page served on 127.0.0.1 to a
  session opened with the option.
- `crates/script/tests/hostile_pages.rs`, declared in `crates/script/tests/main.rs`: the same
  pages through `surf_script::run::parse_with_scripts`, without a session, where a case needs a
  limit the session does not show.
- In the last commit, the public surfaces of acceptance 5.

## Acceptance

1. Each page ends with its limit recorded as the script's end and the page's other elements
   present in the document's rows, one named case per page:
   - an endless loop: the step limit per script;
   - many small scripts (200 scripts of `for (let i = 0; i < 100000; i++) {}`): the step total
     per document, the later scripts `not run`;
   - a loop that the step limits do not end first (`while (true) {}` under step limits set high
     for the case): the wall-clock bound alone;
   - an endless allocation: memory, then no further script after the second (epic line 840);
   - endless node creation: the node limit;
   - a tree built deep by `appendChild`: the depth limit;
   - deep recursion: `frame_depth`, ending `Threw` with `RangeError`;
   - 10,000 script elements: scripts per document;
   - an attribute over the limit: attributes, then stored bytes;
   - a script over the source limit: `not run`, the reason named.
2. `bound_with_script`: a scripted session under an address-space bound low enough for a page of
   a few large allocations held by the host (strings appended to the tree) ends at that bound;
   `surfd` lists it `died` with the signal or bound line of `story:session-process-hardening`,
   keeps answering, and another session opened after it loads a page.
3. `slot task check` exits 0.
4. `git diff --stat` names only test files under `crates/session/tests/` and `crates/script/tests/`
   and the files of acceptance 5.
5. Public surfaces: the story names every public surface its change makes false (a badge, a
   `README.md` line, a website page, a count) and changes it in its last commit, every number
   from the log of a gate run on its tip, which the coordinating agent orders. For this story,
   lines on `main` at `29945f5` (2026-11-24): the test count of `README.md:11` and `:285-286` and
   `docs/badges/make.sh:38` with `docs/badges/tests.svg`; the gate block of `README.md:584-625` and
   of `website/docs/how-it-is-built.md:40-81`.
6. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; every
   case here ends within 5 s, its time in the report; the servers start in Rust on 127.0.0.1; no
   model call; one session per case; no Python; no generator with a committed output is added; a
   build or test job in the pool ends within 10 minutes or names its time with `slot --timeout`,
   at most 30 minutes.

## Out of scope

Limits the first version does not set; fuzzing; any change of behaviour (a case that fails
because a limit is missing is reported to the coordinating agent, not fixed here).

## Placement

After `story:scripts-document-port`.

## Why `epic:scripts-in-a-page`

The epic's acceptance 6 and the first condition's purpose: a hostile page costs a bounded amount
and the rest of the page is still read.
