---
format: aep.planning-md/3
id: story:html-tree-formatting
kind: story
status: implemented
title: 'Misnested formatting elements: the adoption agency algorithm'
summary: The adoption agency algorithm and Noah's Ark; every tree case tagged body or formatting passes, and the counts of the other tags are raised to the measured ones.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-tree-body
scope:
- confidence: cited
  path: crates/html-conformance/expected/tree-construction.txt
- confidence: cited
  path: crates/html/src/lib.rs
- confidence: cited
  path: crates/html/src/tree.rs
- confidence: cited
  path: crates/html/src/tree_builder.rs
- confidence: cited
  path: crates/html/src/tree_builder/formatting.rs
- confidence: cited
  path: crates/html/src/tree_builder/modes.rs
- confidence: cited
  path: crates/html/tests/tree_formatting.rs
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-21T20:39:46Z", actor: "human:operator", revision: 16, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

Misnested formatting elements (`<b><i></b></i>`, `<a><p></a>`) are repaired by the adoption
agency algorithm (13.2.6.4.7, "in body", of the snapshot `0e292f4` that
`docs/design/html-parser.md` names), with the "Noah's Ark" clause of "push onto the list of
active formatting elements" (13.2.4.3) bounding identical entries after the last marker, and the
in body rules that run the algorithm from a start tag: an `a` start tag while an `a` is in the
list after the last marker, a `nobr` start tag while a `nobr` is in scope. The unit confirms the
step numbers against the snapshot before it writes them.

What is on `main` at `5129b50` (story `html-tree-body`, commits `039104d` and `d72b41d`):

- `crates/html/src/tree_builder/modes.rs:768-781`: every formatting start tag, `a` and `nobr`
  included, reconstructs, inserts and pushes; no step runs the algorithm. The end tags of the
  formatting elements go to `adoption_agency_hook` (`modes.rs:984-993`), which runs "any other
  end tag" (`any_other_end_tag`, `modes.rs:967-982`, already written) and removes the element it
  popped from the list. When "any other end tag" ignores the token (a special element above the
  match), the list keeps the element.
- `crates/html/src/tree_builder.rs:584-599`: `push_active_formatting_element` without Noah's Ark.
  `active_position` (`:637-644`) finds a node scanning from the end of the list; the list holds a
  node at most once. The reconstruction (`:601-626`) asks "is in the stack of open elements" from
  the current node up.
- `crates/html/src/tree.rs`: `Tree::insert` requires a node without a parent (`:167`,
  `debug_assert`), `Tree::remove` (`:225-230`) takes a node out of its parent by a scan of the
  parent's children. No operation moves all children of one node to another.

The cases tagged `formatting` by `docs/design/html-parser.md` (item 8) are this story's: 22, of
which `adoption01.dat` holds 13 and `adoption02.dat` 2, `doctype01.dat` 1,
`menuitem-element.dat` 1, `processing-instructions.dat` 1, `search-element.dat` 2 and
`void-in-phrasing.dat` 2 (`html-parser.md:647-706`). At `main` 8 pass through the generic rules;
the 14 that fail (`surf-html-conformance --list-failing formatting`) are `adoption01.dat` 1, 2, 3,
5, 7, 8, 9, 14, 15, 17, `adoption02.dat` 1, 2 and `search-element.dat` 4, 5.

Measured in a scratch copy with a prototype of the algorithm, the clause and the two start tag
rules (not committed): `tag formatting 22/22/22`; `tag mixed` from 704 to 755, the 51 `mixed`
cases that hold no table, `select`, `template`, `svg` or `math` tag and are no fragment, which
are all `mixed` cases of that kind that fail at `main` (the 330 left each hold such a tag:
202 foreign, 127 tables, 1 template); `tag template` from 29 to 30 (`template.dat` 84,
`<body><template><i><menu>Foo</i>`, through the algorithm and the generic rules); `tag body`,
`tag tables` and `tag foreign` unchanged; no case that passed at `main` fails. The 51:
`tests1.dat` 23, 24, 32, 52, 53, 54, 57, 61, 71, 72, 73, 74, 75, 76, 96, 97; `tests2.dat` 10;
`tests3.dat` 21, 22; `tests8.dat` 9, 10; `tests19.dat` 96; `tests22.dat` 1 to 5; `tests23.dat` 1,
2, 3; `tests26.dat` 1, 2, 5, 6, 7, 8, 9; `tricky01.dat` 1, 2, 3, 5, 9; `webkit01.dat` 34, 39, 40,
41; `webkit02.dat` 13, 14, 15, 16; `html5test-com.dat` 27. Links are `a` elements, so a page
with links parses as the standard says only after this story.

Bounds. The algorithm has the standard's two: the outer loop runs at most 8 times, and in the
inner loop a node met after the third iteration is removed from the list and then from the
stack, so each outer iteration creates at most 3 elements in the inner loop and 1 in its step
"create an element for the token for which formatting element was created": at most 32 elements
per end tag. Noah's Ark scans the list from its end to the last marker for every formatting
start tag; after the reconstruction that precedes each push every entry after the last marker
is open, so the scan is as long as the stack holds formatting elements. In the prototype 30,000
nested `<b id=k>`, each different, took 7.9 s in a release build (identical `<b>`: 11 ms):
quadratic in nesting, like the nested start tags `html-tree-body`'s adversary found. That and
the reconstruction's n x m nodes (21,897 bytes gave 1,003,005 nodes, `docs/plan/agent-wave-8.md`)
stay with `story:html-parser-limits` (wave 10), whose depth limit bounds both scans. The tests
of this story hold node counts, never a time.

What the algorithm changes in the departures of `html-tree-body`: the hook goes; a formatting
end tag whose element is in the list but not on the stack removes it from the list (step 4.4),
one not in scope leaves it (step 4.5), one with no entry after the last marker runs "any other
end tag" (step 4.3). A node is still in the list at most once (steps "replace the entry" and
"insert the new element into the list at the bookmark" put a new node in; the `a` rule and Noah's
Ark only remove), so `active_position` stays correct; step 4.3 looks by tag name, from the end
back to the last marker, which `active_position` does not do.

## Change

- `crates/html/src/tree_builder/formatting.rs` (new submodule, `impl TreeBuilder`): the adoption
  agency algorithm, steps in the standard's order, with the outer loop limit 8 and the inner
  loop's removal after 3; step 4.3 falls back to the existing `any_other_end_tag`; step 4.14
  inserts at `appropriate_place(Some(common ancestor))`, so that `story:html-tree-tables` adds
  foster parenting there and nowhere else.
- `crates/html/src/tree_builder/modes.rs`: in body's `a` start tag (an `a` in the list after the
  last marker: the algorithm, then that element removed from the list and the stack if still
  there), `nobr` start tag (a `nobr` in scope: the algorithm, then reconstruct again), and the
  formatting end tags (`a`, `b`, `big`, `code`, `em`, `font`, `i`, `nobr`, `s`, `small`,
  `strike`, `strong`, `tt`, `u`) running the algorithm. `adoption_agency_hook` removed; the doc
  comments at `:490-492` and `:771-772` say what is written.
- `crates/html/src/tree_builder.rs`: Noah's Ark in `push_active_formatting_element` (same tag
  name, namespace and attributes, compared by name and value in any order; three already after
  the last marker: the earliest removed); `NOT_YET_PARSED` without `misnested-formatting` (the
  entries left still match `[a-z-]+`, D31 of `docs/plan/agent-plan-revision-2.md`, held by
  `every_not_yet_parsed_entry_matches_a_z_and_hyphen`); the module documentation (`:15-21`) and
  the comments at `:34-37` and `:584-585` say what is written.
- `crates/html/src/tree.rs`: one `pub(crate)` operation that moves every child of one node to
  the end of another in one pass (the algorithm's "take all of the child nodes of furthest block
  and append them"), placed after `remove`. No public item changes (D2: every `pub fn` takes
  `&self`). Moving a node that has a parent is `remove` then `insert`.
- `crates/html/src/lib.rs`: only the `tree_builder` bullet of the section "State" (`:51-58`),
  which says the algorithm and Noah's Ark are not yet written.
- `crates/html/tests/tree_formatting.rs` (new), each red at `main`, values measured with the
  prototype:
  - `the_inner_loop_clones_at_most_three_elements`: `<a><b><i><s><u><em><div>x</a>` gives 14
    elements, 2 of them `a` (16 without the removal after the third iteration).
  - `the_outer_loop_runs_at_most_eight_times`: `<a>`, n `<div>`, `</a>` gives 8 `a` elements
    for n = 7 and 9 for n = 8, 9, 20 and 1,000; n = 1,000 gives 1,013 nodes.
  - `noahs_ark_keeps_three_identical_formatting_elements`: `<p>`, 100 `<b>`, `</p>x` gives 103
    `b` elements and 109 nodes; with 100 different `<b id=k>`, 200 `b` and 206 nodes.
  - `an_a_start_tag_closes_an_open_a`: `<a>1<a>2` gives `body` two `a` children, `"1"` and
    `"2"`, 8 nodes.
  - `a_nobr_start_tag_closes_an_open_nobr`: `<nobr>1<nobr>2` likewise with `nobr`.
- `crates/html-conformance/expected/tree-construction.txt`: the counts raised to what the run
  gives.

Writes: `crates/html/src/tree_builder.rs`, `crates/html/src/tree_builder/modes.rs`,
`crates/html/src/tree_builder/formatting.rs`, `crates/html/src/tree.rs`, `crates/html/src/lib.rs`
(lines 51 to 58 only), `crates/html/tests/tree_formatting.rs`,
`crates/html-conformance/expected/tree-construction.txt`.

## Acceptance

- `cargo test -p surf-html` passes with the five tests of `tests/tree_formatting.rs` named in
  Change, and the 39 tests there before.
- `slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 and prints `tag body
  476/476/476`, `tag formatting 22/22/22`, `tag mixed 755/1085/1085`, `tag template
  30/123/123`, `tag tables 11/39/39`, `tag foreign 6/222/222` and `html-conformance: 9107 cases,
  9095 run, 8428 passed`, and no `raise:` line. `expected/tree-construction.txt` holds these tag
  lines and, changed from `main`, the file lines `adoption01.dat 13/18/18`, `adoption02.dat
  2/4/4`, `html5test-com.dat 21/30/30`, `search-element.dat 7/8/8`, `template.dat 29/127/127`,
  `tests1.dat 94/112/112`, `tests19.dat 85/107/107`, `tests2.dat 55/63/63`, `tests22.dat 5/5/5`,
  `tests23.dat 5/5/5`, `tests26.dat 10/20/20`, `tests3.dat 23/24/24`, `tests8.dat 6/10/10`,
  `tricky01.dat 6/9/9`, `webkit01.dat 42/52/52`, `webkit02.dat 19/49/49`. A count that differs
  from these is reported with the cases behind it.
- `cargo run -p surf-html-conformance -- --list-failing formatting` prints nothing.

## Out of scope

Foster parenting inside the adoption agency algorithm when the common ancestor is a table
(story `html-tree-tables`). Bounds on depth, nodes and the length of the list, and the time the
scans take (story `html-parser-limits`). Scripts: nothing in this algorithm needs a script
engine; custom elements created while cloning formatting elements run no constructor;
`scripted_adoption01.dat` and `scripted_ark.dat` stay not run (`needs-script`). Parse errors are
not collected. `AGENTS.md`, `README.md` and the website's numbers are the coordinator's.
