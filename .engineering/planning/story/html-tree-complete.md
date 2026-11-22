---
format: aep.planning-md/3
id: story:html-tree-complete
kind: story
status: implemented
title: Every tree-construction case passing, and the section marks decided
summary: The remaining failing cases fixed; each tree-construction section marked complete only by the completeness rule.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-tree-foreign
scope:
- confidence: cited
  path: crates/html-conformance/expected/tree-construction.txt
- confidence: cited
  path: crates/html/src
- confidence: cited
  path: docs/design/html-parser.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-22T16:29:15Z", actor: "human:operator", revision: 10, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

After story `html-tree-foreign` every case of the thematic files passes, by tag. The mixed files
of `docs/design/html-parser.md` (item 8, rule 4: the 30 files `tests1.dat` to `tests26.dat`,
there being no `tests13.dat`, and `webkit01.dat`, `webkit02.dat`, `tricky01.dat`,
`html5test-com.dat`, `pending-spec-changes.dat`, 1,085 cases) hold cases that exercise several parts at once; they are tagged
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

Bounds for every command of this story. Every build and test runs through the build pool:
`cargo` is wrapped by the pool's shim, every other command that builds or tests gets the `slot`
prefix. The pool ends a job after 10 minutes; a command that needs longer names its time with
`slot --timeout <t>` or `SLOT_TIMEOUT=<t>`, at most 30 minutes; a `timeout` wrapped around the
command extends nothing. A script that measures or waits starts without the `slot` prefix; the
builds and test runs inside it carry it. No test runs over 60 s and the test step ends within
120 s. No Python.

## Acceptance

`slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 printing `tag mixed
1085/1085/1085` and every other tag that runs at pass = run = exist (`markup 1979`, `text 328`,
`charref 4739`, `body 476`, `formatting 22`, `tables 39`, `template 123`, `foreign 222`,
`encoding 82`), the 12 `not-run` cases left out by their reasons (`tag not-run 0/0/12`:
`xml-infoset` 4, `needs-script` 7, `autodetect` 1), the total line `html-conformance: 9107
cases, 9095 run, 9095 passed`, and each section of `html-sections` printed either `complete n/n`
or `incomplete` with a reason that is not a failing case.

## Out of scope

The 12 cases tagged `not-run` (`scripted_*.dat`, `encoding/scripted/`, `xmlViolation.test`,
`encoding/chardet/test_big5.txt`): they stay counted as
not run, and the sections they belong to stay incomplete. Scripts: as in the earlier tree
stories, nothing runs.

## As built

What differs from the text above, as of 280e8a7:

- Change, "whatever the failing `mixed` cases need": the four cases that still failed,
  `webkit02.dat` cases 45 to 48, needed no parser rule but the DOM's update of a `select`'s
  `selectedcontent` elements (4.10.12): the selected option's children are copied into each
  `selectedcontent` that is not disabled (`crates/html/src/tree_builder/selectedcontent.rs`).
- That copy is made once, after the end-of-file token, and for a document only, instead of
  from a microtask queued by the insertion steps. A fragment, template contents and a parse a
  limit stopped get no copy (`open` line `selectedcontent-update-timing`).
- The copy is bounded. A review after the first implementation found that the pass checked
  the step limit only before each `select`, and that each `selectedcontent` walked up to the
  root: a 16 MiB page ended at 244,425,635 steps of 134,217,728 with no limit named (commit
  280e8a7's message). One walk of the document now decides each `selectedcontent`'s `select`
  and disabledness, and the step limit is checked after every node a walk or the copy visits
  and after the pass. The node and stored-byte limits admit each copied node
  (`tree_costs::the_selectedcontent_*`,
  `adversary_tree::the_selectedcontent_pass_ends_within_the_step_limit` in `crates/html/tests/`).
- The display size of a `select` reads `size=-0` as 0, as the rules for parsing integers do,
  so no option is selected by default; another negative value gives 1 (`display_size` in
  `selectedcontent.rs`, `adversary_tree::a_select_of_size_minus_zero_*`).
- Change, "The `html-sections` block is not changed": it gained two `open` lines under
  `tree-construction`, `template-content-patching` and `selectedcontent-update-timing`, and the
  section on the marks says that content patching changes the tree.
- Writes: the test files `crates/html/tests/tree_costs.rs` and
  `crates/html/tests/adversary_tree.rs`, with their `mod` lines in `crates/html/tests/main.rs`,
  are outside this story's Writes.
- Left open at 280e8a7 and closed in the same round by 64c6621: because of the copy,
  `surf-page` listed a link inside the selected option twice, once in the option and once in
  the `selectedcontent`. The copy in a `selectedcontent` the parser fills is now hidden and
  lists no row (`is_hidden` and `hidden_roots` in `crates/page/src/lib.rs`,
  `adversary_select::a_link_in_the_selected_option_is_listed_once`).
- Left open: which other content of a `select` a reader sees as visible rows. The draft
  `story:select-content-rows` takes it.
