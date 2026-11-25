---
format: aep.planning-md/3
id: story:scripts-dom-sibling-positions
kind: story
status: draft
title: A sibling read costs no scan of its parent's children
summary: The sibling members answer from a position the tree or the slice keeps per child, so any count of readers on one parent stays linear, where two positions a parent leave a third reader quadratic.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-dom-reading
- depends_on: story:scripts-dom-mutation
revision: 1
---
## Context

`story:scripts-dom-reading` answers the sibling members (`previousSibling`, `nextSibling`,
`previousElementSibling`, `nextElementSibling`) from position hints kept in the slice: eight
parents, two positions a parent (its decisions 9, 17 and 18). `Tree` in `crates/html` keeps a
parent's children as a list and gives a node no position in it, so a read without a hint scans
the parent's children and charges one step per child. Two readers on one parent stay linear.
A third reader on the same parent misses at each read: the unit's second review measured 500
children read by three readers in turn at 180,250 steps, and 5,000 children ending at the
step limit of a script. No answer is wrong and every scan is charged; a page that walks one
long child list with three cursors ends at a limit where a browser finishes.

How many real pages do that is not measured. `story:scripts-conformance` gives a count over
real pages; this story is ordered by that count.

## Change

One of these, decided when the story is revised, with the numbers of both measured on the
build node:

- the tree gives a node its position among its parent's children (kept by the tree's writers
  of a child list, the six raise sites of the write counter), and the sibling members read it;
  the slice's hint table goes;
- or the slice keeps one index per parent, built once after a write to that parent's child
  list and charged one step per child, with the hints kept for a parent whose list is written
  between reads.

The step charges of the sibling members are restated for the form chosen.

## Acceptance

1. Three, four and sixteen readers on one parent of 5,000 children, read in turn, end `Ran`,
   with steps under 15 times those of 500 children.
2. A loop that removes each child after reading its next sibling stays linear in the children.
3. Over random writes every sibling member equals the parent's list (the reading unit's
   tests of sibling answers hold unchanged).
4. The reading unit's test of three readers asserts the new numbers, and the design note's
   sentence that names the quadratic cost is rewritten.
5. The folder's bounds hold: no test over 60 s, the test step within 120 s, a test over 5 s
   named with its time, no Python, no model call and no network request in a test.

## Out of scope

Live collections, and the index of a child in `childNodes` (the kept snapshot answers it).
