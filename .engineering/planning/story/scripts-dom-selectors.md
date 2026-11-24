---
format: aep.planning-md/3
id: story:scripts-dom-selectors
kind: story
status: draft
title: The 7 selector members over the subset of Selectors Level 4 the note names
summary: querySelector, querySelectorAll, closest, matches and webkitMatchesSelector for the note's subset; a valid selector outside it throws NotSupportedError, an invalid one SyntaxError.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-dom-reading
scope:
- confidence: cited
  path: crates/script/src/dom/selectors.rs
- confidence: inferred
  path: crates/script/tests/dom_selectors.rs
revision: 3
---
## Context

The epic's eleventh candidate (lines 469-471) and the slice "Selectors, 7" (lines 701-705):
`querySelector` and `querySelectorAll` on `Element` and on `Document`, `closest`, `matches` and
`webkitMatchesSelector`, for the subset of Selectors Level 4 the note's Q9 gives; a valid
selector outside it throws `NotSupportedError`, a named deviation (a browser would not throw), and
an invalid one a `SyntaxError`. `querySelectorAll` returns a static `NodeList`.

## Change

- `crates/script/src/dom/selectors.rs`: the parser of the subset and the matcher, each step of a
  match charged as the bindings charge steps (requirement 3, as revised).
- `crates/script/tests/dom_selectors.rs`.

## Acceptance

`slot task check` exits 0. The folder's bounds hold: the test step ends within 120 s and no test
runs over 60 s; no model call and no network request in a test; each case parses a few lines and
makes one realm; no Python; no generator with a committed output is added (one that were would carry a digest of its sources, a fast check in the gate and full regeneration in the lane `slow`); a build or test job in the pool ends within 10 minutes or names its time with `slot --timeout`, at most 30 minutes; a test over 5 s is named.

1. A named case per member (7) and per production of the subset (type, universal, `#id`,
   `.class`, attribute presence and equality, compound, descendant, child, list), each comparing
   the elements found with what Selectors Level 4 says over a parsed tree.
2. A valid selector outside the subset (`a:hover`, `a + b`) throws `NotSupportedError` naming the
   construct; an invalid one (`a[`) throws a `SyntaxError`.
3. `document.querySelector("table")` on the tree of the `scripted_foster01.dat` case that calls it finds its
   `table`.
4. A selector over a tree near the node limit ends within the step limit, the script's end naming
   it, under 5 s.
5. `git diff --stat` names only the files of the Change.

## Out of scope

Selectors beyond the subset; `:scope` unless Q9 takes it.

## Why `epic:scripts-in-a-page`

The selector members the epic's acceptance 3 names, which a `needs-script` case of web-platform-tests calls.
