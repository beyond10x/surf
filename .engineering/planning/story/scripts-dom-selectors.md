---
format: aep.planning-md/3
id: story:scripts-dom-selectors
kind: story
status: draft
title: The 9 selector members over a subset of Selectors Level 4 this story settles
summary: querySelector, querySelectorAll, closest, matches and webkitMatchesSelector, and DocumentFragment's two, for the subset (formerly the note's Q9) with its grammar cited; a valid selector outside it throws NotSupportedError, an invalid one SyntaxError.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-dom-reading
scope:
- confidence: cited
  path: crates/script/src/dom/selectors.rs
- confidence: cited
  path: crates/script/tests/dom_selectors.rs
revision: 12
---
## Context

The epic's eleventh candidate (lines 469-471) and the slice "Selectors, 7" (lines 701-705):
`querySelector` and `querySelectorAll` on `Element` and on `Document`, `closest`, `matches` and
`webkitMatchesSelector`, for a subset of Selectors Level 4; a valid selector outside it throws
`NotSupportedError`, a named deviation (a browser would not throw), and an invalid one a
`SyntaxError`. `querySelectorAll` returns a static `NodeList`.

`story:webidl-bindings` lists this slice's rows in `crates/webidl/selection.txt`: those 7 and the
2 of `DocumentFragment` (`querySelector`, `querySelectorAll`), 9 member rows.

The coordinating agent moved the question of the subset here from `story:scripts-decisions`
(its number Q9 stays unused in the note).

## Decisions taken

1. The subset, as the epic gives it (lines 701-705): type selectors, the universal selector,
   `#id`, `.class`, attribute presence `[a]` and equality `[a=v]`, compound selectors, the
   descendant and child combinators, and selector lists. The head comment of `selectors.rs`
   gives its grammar, each production cited to its section of Selectors Level 4 with the date of
   the text read, and the case rules in an HTML document (type and attribute names compared
   ASCII case-insensitively for HTML elements, IDs and classes case-sensitively in no-quirks
   mode) cited to the DOM and HTML Standards.
2. Input that parses as a selector of Selectors Level 4 but uses a construct outside the subset
   (a pseudo-class, a pseudo-element, the sibling combinators, an attribute matcher other than
   `=`, a namespace prefix) throws `NotSupportedError` naming the construct; input that does not
   parse throws a `SyntaxError`. `:scope` is outside the subset.

## Change

- `crates/script/src/dom/selectors.rs`: the parser of the subset and the matcher, each step of a
  match charged as the bindings charge steps (requirement 3, as revised).
- `crates/script/tests/dom_selectors.rs`: the module `dom_selectors`, declared by
  `story:webidl-bindings`.
- In the last commit, the public surfaces of acceptance 7.

## Acceptance

1. A named case per member row and per production of the subset (type, universal, `#id`,
   `.class`, attribute presence and equality, compound, descendant, child, list), each comparing
   the elements found with what Selectors Level 4 says over a parsed tree; a test asserts that the
   member cases equal the rows of `selection.txt` with the slice `selectors`: 9.
2. A valid selector outside the subset (`a:hover`, `a + b`, `a ~ b`, `[a^=b]`) throws
   `NotSupportedError` naming the construct; an invalid one (`a[`) throws a `SyntaxError`.
3. `document.querySelector("table")` on the parsed document of each case of
   `crates/html-conformance/data/wpt/html/syntax/parsing/resources/scripted_foster01.dat` finds
   its `table`.
4. A selector over a tree near the node limit, matched in a loop, ends `limit: steps-per-entry`
   under 5 s (the word Q4's column gives the row `steps_per_entry`; where the note spells it
   otherwise, the note's spelling holds).
5. `slot task check` exits 0; `git diff --stat` names only the files of the Change.
6. A member of another slice still throws `NotSupportedError` naming the member.
7. Public surfaces: the story changes, in its last commit and after merging `main`, every
   public surface its change makes false, each number taken from the `slot task check` log of
   its tip. After that commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges`
   prints nothing for each old string below, as the story's base holds it (the strings and lines
   given are those of `main` at `29945f5`, 2026-11-24); each new number equals the one in that log;
   and the gate block, `README.md:584-625` and `website/docs/how-it-is-built.md:40-81`, holds that
   log's lines. The old strings of this story: the test count `1976 passed` (`README.md:11`,
   `docs/badges/make.sh:38`, `docs/badges/tests.svg`), `1,976` and `1,424, and 552`
   (`README.md:285`, `website/docs/status.md:23`, `website/docs/what-works.md:17`).
8. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; no model
   call and no network request in a test; each case parses a few lines and makes one realm; no
   Python; no generator with a committed output is added; a build or test job in the pool ends
   within 10 minutes or names its time with `slot --timeout`, at most 30 minutes; a test over 5 s
   is named.

## Out of scope

Selectors beyond the subset, `:scope` among them.

## Placement

After `story:scripts-dom-reading`; beside `story:scripts-dom-mutation` and
`story:scripts-dom-attributes` (each writes its own file).

## Why `epic:scripts-in-a-page`

The selector members the epic's acceptance 3 names, which a `needs-script` case of
web-platform-tests calls.
