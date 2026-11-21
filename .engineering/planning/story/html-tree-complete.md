---
format: aep.planning-md/3
id: story:html-tree-complete
kind: story
status: active
title: Every tree-construction case passing, and the section marks decided
summary: The remaining failing cases fixed; each tree-construction section marked complete only by the completeness rule.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-tree-foreign
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 5}
---
## Context

After story `html-tree-foreign` every case of the thematic files passes, by tag. The mixed files
of `docs/design/html-parser.md` (item 8: from memory `tests1.dat` to `tests26.dat`,
`webkit01.dat`, `webkit02.dat`, `tricky01.dat`, `html5test-com.dat`,
`pending-spec-changes.dat`) hold cases that exercise several parts at once; they are tagged
`mixed` and are this story's. Its count after `html-tree-foreign` is whatever those stories
reached by the way; this story raises it to the run count. And no tree-construction section has
its mark decided yet: `AGENTS.md` ("Goals") allows `complete` only when every requirement of the
section is specified and every test for it passes, and the gate (story `html-conformance-gate`)
refuses it otherwise.

## Change

- `crates/html/src/`: whatever the failing `mixed` cases need, each fix named in the story report
  with the case it fixes and the rule of the standard behind it.
- `NOT_YET_PARSED` left empty, so `story:html-document-port` prints no parser `note` line; a
  test that holds a `note parser` line builds it from the constant (D31), so emptying it breaks
  no other story's test.
- `crates/html-conformance/expected/tree-construction.txt`: every count raised to the run count,
  and each tree-construction section given the mark the gate's rule allows; where the note lists
  an open requirement or a web-platform-tests test that does not run, it stays `incomplete` with
  that reason.
- `docs/design/html-parser.md`: a short section listing the marks and their reasons as of this
  story, for the agents that read the note. The `html-sections` block is not changed.

Writes: `crates/html/src/`, `crates/html-conformance/expected/tree-construction.txt`,
`docs/design/html-parser.md` (one section added, outside the fenced blocks).

## Acceptance

`slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 printing `tag mixed
x/x/x` with x the `mixed` count of `html-inventory`, every other tree-construction `tag` line at
`n/n/n`, pass equal to run for every file under `tree-construction/`, and each tree-construction
section printed either `complete n/n` or `incomplete` with a reason that is not a failing case.

## Out of scope

The cases tagged `not-run` (`scripted/`, and any feature the note excludes): they stay counted as
not run, and the sections they belong to stay incomplete. Scripts: as in the earlier tree
stories, nothing runs.
