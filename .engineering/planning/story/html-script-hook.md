---
format: aep.planning-md/3
id: story:html-script-hook
kind: story
status: draft
title: The tree builder calls a hook at an inline classic script's end tag, and Tree gains a public write surface held to the parser's limits
summary: With scripting enabled, the text insertion mode calls a hook with the tree at each script the note says runs, and parsing goes on over the tree the hook left; public writes answer the limit they reach.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-decisions
- depends_on: story:html-tree-tables
- depends_on: story:html-parser-limits
scope:
- confidence: cited
  path: crates/html/src/lib.rs
- confidence: cited
  path: crates/html/src/tree.rs
- confidence: cited
  path: crates/html/src/tree_builder.rs
- confidence: cited
  path: crates/html/src/tree_builder/modes.rs
- confidence: inferred
  path: crates/html/tests/main.rs
- confidence: inferred
  path: crates/html/tests/script_hook.rs
- confidence: inferred
  path: docs/design/html-parser.md
revision: 3
---
## Context

The epic's seventh candidate (lines 452-457) and "What must exist in surf first" (lines 368-379).
The tree on `main` of 2026-11-24: at a `script` end tag the "text" insertion mode pops the element
and returns to the original insertion mode, "no script is prepared or run"
(`crates/html/src/tree_builder/modes.rs:1120-1141`; module doc of
`crates/html/src/tree_builder.rs:28-29`). `ParseOptions.scripting_disabled` exists, default
`true` (`crates/html/src/lib.rs:180-195`). Every write of `Tree` is `pub(crate)`
(`crates/html/src/tree.rs:204-425`: `create_element`, `insert`, `insert_new`, `text_before`,
`push_text`, `add_attributes`, `remove`, `remove_children`, `move_children`), each admitted
against the limits of `story:html-parser-limits` (`crates/html/src/limits.rs:16-47`). The stories
the epic waits for are on `main`: `story:html-tree-tables` and `story:html-parser-limits` are
implemented.

## Change

- `crates/html/src/tree_builder.rs`, `tree_builder/modes.rs`, `lib.rs`: a hook, of the signature
  the note's Q2 gives, called when scripting is enabled at the end tag of a script element that
  is parser-inserted, inline and classic and not skipped by the rows of Q1, after the steps Q2
  marks "taken"; parsing goes on with its stack of open elements and list of active formatting
  elements over the tree the hook left, including elements the hook removed or moved, as Q2
  says. A parse without a hook, or with scripting disabled, is byte for byte the parse of today.
- `crates/html/src/tree.rs`: a public write surface for the DOM (create a node, insert before,
  append, remove, replace, set and remove an attribute, set text and data), each admitted against
  the parser's limits and answering the `Limit` it reached instead of writing.
- `docs/design/html-parser.md`: item 9's sentences on the script hooks, and the line
  `tree-construction script-processing` (line 539) as Q2 leaves it.
- `crates/html/tests/`: a module `script_hook` in the crate's one test binary.

## Acceptance

`slot task check` exits 0. The folder's bounds hold: the test step ends within 120 s and no test
runs over 60 s; no model call and no network request in a test; nothing expensive is made per
test (each case parses a document of a few lines); no Python; no generator with a committed output is added (one that were would carry a digest of its sources, a fast check in the gate and full regeneration in the lane `slow`); a build or test job in the pool ends within 10 minutes or names its time with `slot --timeout`, at most 30 minutes; a test over 5 s is named.

1. A hook that records its calls sees one call per script element Q1 says runs, in document
   order, and none for the rows Q1 says are not run (`src`, `type=module`, `nomodule`, another
   type, inside `template` contents), and none with scripting disabled.
2. A hook that appends an element after the script: the element is in the final tree where it was
   put, and the markup after the end tag is parsed after it.
3. A hook that removes an open ancestor of the script (`<div><script></script>x</div>`, the hook
   removes the `div`): the text `x` goes where Q2 says (the removed `div`, as the standard's
   stack of open elements keeps it), and the parse ends without a panic.
4. Each public write over its limit (nodes, depth, attributes, stored bytes) answers that
   `Limit` and leaves the tree unchanged; a parse whose hook reached a limit records it as Q4 says.
5. `slot task html-conformance` prints the same counts as before the change (no case runs a hook).
6. `git diff --stat` names only the files of the Change.

## Out of scope

Running a script (`story:scripts-dom-mutation` joins the hook to swell); `document.write`, the
insertion point and a script nesting level above 1; SVG `script` elements beyond what Q1 says.

## Why `epic:scripts-in-a-page`

The parser's pause at the script end tag, which the epic runs a page's scripts in.
