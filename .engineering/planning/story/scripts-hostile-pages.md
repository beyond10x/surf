---
format: aep.planning-md/3
id: story:scripts-hostile-pages
kind: story
status: proposed
title: Hostile pages end at a recorded limit with the rest of the page parsed, and a session at its process bounds is listed died with its signal named
summary: 'Tests only: an endless loop, many small scripts, a wall-clock-only loop, allocation, node creation, deep trees and recursion, 10,000 scripts, an attribute and a source over their limits each end with their limit within 10 s; a scripted session at an address-space bound is listed died with note ended: signal SIGABRT and surfd goes on.'
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-document-port
scope:
- confidence: cited
  path: crates/script/tests
- confidence: cited
  path: crates/session/tests
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:14Z", actor: "human:operator", revision: 9}
---
## Context

The epic's fourteenth candidate (lines 486-492) and its acceptance 6 (lines 583-584), with the
constraint "A script never hangs, panics or exhausts the session process" (lines 553-555) and
"limits per script, per document and in wall time, each ending in a defined outcome" (lines 90-92,
225-229). The limits and their recorded outcomes are those of `docs/design/scripts.md`, Q4, and
`story:scripts-parse-run`'s; the session's bounds of the operating system, and what a client is
answered when a session ends at one, are Q11's and `story:session-process-hardening`'s (S5 decided
a by the operator session on 2026-11-24). The epic lists `crates/surf/tests/`; by S7 no script runs
outside a session, so the pages are served to sessions, and the library cases go to `surf-script`'s
tests. In the test profile a script runs about 1.4 million steps per second (Q4), so under the
session's limits the wall clock of 5 s ends a long script before `steps_per_entry` (10,000,000) or
the document's step total (50,000,000) does; the cases of the step limits run through
`parse_with_scripts` with limits of the test's own, as Q4 says a test does. This story adds tests
only: every behaviour it shows is written by an earlier story.

## Change

- `crates/session/tests/`: a module `hostile_pages`, each case a page served on 127.0.0.1 to a
  session opened with the option.
- `crates/script/tests/hostile_pages.rs`, declared in `crates/script/tests/main.rs`: the same
  pages through `surf_script::run::parse_with_scripts`, without a session, where a case needs a
  limit the session does not show.
- In the last commit, the public surfaces of acceptance 5.

## Acceptance

1. Each page ends with its limit recorded as the script's end and the page's other elements present
   in the document's rows, one named case per page, each end written with a word of Q7's closed
   list:
   - an endless loop, under a step limit of the case: `limit: steps-per-script`;
   - many small scripts (200 scripts of `{ for (let i = 0; i < 100000; i++) {} }`, under a document
     step total of the case): `ran` up to the script whose steps take the total to its limit, which
     keeps its own end, and `not run: steps-per-document` for each later one;
   - a loop that the step limits do not end first (`while (true) {}` in a session, under its
     shipped limits): `limit: wall-clock`, after about 5 s, and each later script of the page
     `not run: wall-clock`;
   - an endless allocation, `{ const a = []; for (;;) a.push(new Array(1000)); }` twice:
     `limit: memory` twice, then `not run: memory`;
   - endless node creation: `limit: memory` under the session's limits, since the charge of Q4, at
     least 1,224 bytes a node, reaches `memory_bytes` (67,108,864) at about 54,800 nodes, before
     the node limit of 1,048,576; and `limit: nodes` through `parse_with_scripts` under a node
     limit of the case below that;
   - a tree built deep by `appendChild`: `limit: depth`;
   - deep recursion: `threw RangeError "frame_depth exceeded"`;
   - 10,000 script elements: `ran` for the first 1,000 and `not run: scripts-per-document` for each
     from the 1,001st on;
   - an attribute over the limit: `limit: attributes`, then a value over the stored bytes
     `limit: stored-bytes`;
   - a script over 524,288 bytes of source: `limit: source-bytes`, and the next script runs.
2. `bound_with_script`: the test sets the address-space bound in `Config` to a constant of the
   case, `AS_BOUND_MIB`, written as the `VmPeak` of a session without scripts on the same page
   (measured once by a probe, the value and probe in the report) plus 64 MiB; a page of a few large
   strings appended to the tree passes that bound; the session ends at it in `SIGABRT` (Q11: an
   allocation over the address space); `surfd` lists it `died`, the answer to the next request for
   it holds `note ended: signal SIGABRT` (`story:session-process-hardening`, decision 5), `surfd`
   keeps answering, and another session opened after it loads a page.
3. `slot task check` exits 0.
4. `git diff --stat` names only test files under `crates/session/tests/` and `crates/script/tests/`
   and the files of acceptance 5.
5. Public surfaces: the story changes, in its last commit and after merging `main`, every public
   surface its change makes false, each number taken from the `slot task check` log of its tip.
   After that commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges` prints
   nothing for each old string below, as the story's base holds it (the strings and lines given are
   those of `main` at `29945f5`, 2026-11-24); each new number equals the one in that log; and the
   gate block, `README.md:584-625` and `website/docs/how-it-is-built.md:40-81`, holds that log's
   lines. The old strings of this story: the test count `1976 passed` (`README.md:11`,
   `docs/badges/make.sh:38`, `docs/badges/tests.svg`), `1,976` and `1,424, and 552`
   (`README.md:285`, `website/docs/status.md:23`, `website/docs/what-works.md:17`).
6. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; every case
   here ends within 10 s, its time in the report, and a case over 5 s (the wall-clock case, about 5
   s by design) is named with its time; the servers start in Rust on 127.0.0.1; no model call; one
   session per case; no Python; no generator with a committed output is added; a build or test job
   in the pool ends within 10 minutes or names its time with `slot --timeout`, at most 30 minutes.

## Out of scope

Limits the first version does not set; fuzzing; any change of behaviour (a case that fails
because a limit is missing is reported to the coordinating agent, not fixed here).

## Placement

After `story:scripts-document-port`.

## Why `epic:scripts-in-a-page`

The epic's acceptance 6 and the first condition's purpose: a hostile page costs a bounded amount
and the rest of the page is still read.
