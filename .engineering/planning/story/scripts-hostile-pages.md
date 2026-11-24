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
revision: 8
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
   present in the document's rows, one named case per page, each end written with the note's word
   (Q4's column `outcome when reached`, Q7's closed list; where the note spells a word otherwise
   than this text, the note's spelling holds):
   - an endless loop: `limit: steps-per-entry`;
   - many small scripts (200 scripts of `for (let i = 0; i < 100000; i++) {}`):
     `limit: steps-per-document`, the later scripts `not run: steps-per-document`;
   - a loop that the step limits do not end first (`while (true) {}` under step limits set high
     for the case): `limit: wall-clock`, the elapsed time under 10 s;
   - an endless allocation, `{ const a = []; for (;;) a.push(new Array(1000)); }` twice:
     `limit: memory` twice, then `not run: memory` (epic line 840);
   - endless node creation: the node limit's `limit:` word;
   - a tree built deep by `appendChild`: the depth limit's `limit:` word;
   - deep recursion: `Threw` with `RangeError`;
   - 10,000 script elements: `not run: scripts-per-document` from the limit on;
   - an attribute over the limit: the attribute limit's `limit:` word, then the stored-byte
     limit's;
   - a script over the source limit: `not run: source-bytes`.
2. `bound_with_script`: the test sets the address-space bound in `Config` to a constant of the
   case, `AS_BOUND_MIB`, written as the `VmPeak` of a session without scripts on the same page
   (measured once by a probe, the value and probe in the report) plus 64 MiB; a page of a few
   large strings appended to the tree passes that bound; the session ends at it; `surfd` lists it
   `died` with the `note` line of `story:session-process-hardening` acceptance 4 naming the
   signal, keeps answering, and another session opened after it loads a page.
3. `slot task check` exits 0.
4. `git diff --stat` names only test files under `crates/session/tests/` and `crates/script/tests/`
   and the files of acceptance 5.
5. Public surfaces: the story changes, in its last commit and after merging `main`, every
   public surface its change makes false, each number taken from the `slot task check` log of
   its tip. After that commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges`
   prints nothing for each old string below, as the story's base holds it (the strings and lines
   given are those of `main` at `29945f5`, 2026-11-24); each new number equals the one in that log;
   and the gate block, `README.md:584-625` and `website/docs/how-it-is-built.md:40-81`, holds that
   log's lines. The old strings of this story: the test count `1976 passed` (`README.md:11`,
   `docs/badges/make.sh:38`, `docs/badges/tests.svg`), `1,976` and `1,424, and 552`
   (`README.md:285`, `website/docs/status.md:23`, `website/docs/what-works.md:17`).
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
