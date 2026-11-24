---
format: aep.planning-md/3
id: story:html-script-hook
kind: story
status: draft
title: The tree builder calls a hook at an inline classic script's end tag, and Tree gains a public write surface held to the parser's limits
summary: With scripting enabled, the text insertion mode calls a hook at each parser-inserted script end tag, to run it or to report it skipped with its reason, and parsing goes on over the tree the hook left; public writes answer the limit they reach; a parse without a hook is unchanged.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-decisions
- depends_on: story:html-tree-tables
- depends_on: story:html-parser-limits
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/html/src/lib.rs
- confidence: cited
  path: crates/html/src/tree.rs
- confidence: cited
  path: crates/html/src/tree_builder.rs
- confidence: cited
  path: crates/html/src/tree_builder/modes.rs
- confidence: cited
  path: crates/html/tests/main.rs
- confidence: cited
  path: crates/html/tests/script_hook.rs
- confidence: cited
  path: docs/badges
- confidence: cited
  path: docs/design/html-parser.md
- confidence: cited
  path: website/docs/how-it-is-built.md
revision: 12
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
implemented. `docs/design/html-parser.md:539` holds the line `open tree-construction
script-processing`. `task html-conformance` prints `html-conformance: 9107 cases, 9095 run, 9095
passed` on `main` of 2026-11-24.

## Decisions taken

1. The hook has the signature the note's Q2 gives, with two calls: one to run a script element,
   and one that reports a script element it does not run, with the reason word of Q1 (`src`,
   `module`, `nomodule`, another type, inside `template` contents, and the others Q1 names). The
   tree builder calls one of the two for every parser-inserted `script` end tag while scripting
   is enabled, so `story:scripts-parse-run` can turn every skipped element into an end `not run`
   with its reason.
2. A parse without a hook, or with scripting disabled, is the parse of today: no call is made,
   and the tree is the same.

## Change

- `crates/html/src/tree_builder.rs`, `tree_builder/modes.rs`, `lib.rs`: the hook of decision 1,
  called after the steps Q2 marks "taken"; parsing goes on with its stack of open elements and
  list of active formatting elements over the tree the hook left, including elements the hook
  removed or moved, as Q2 says.
- `crates/html/src/tree.rs`: a public write surface for the DOM (create a node, insert before,
  append, remove, replace, set and remove an attribute, set text and data), each admitted against
  the parser's limits and answering the `Limit` it reached instead of writing.
- `docs/design/html-parser.md`: item 9's sentences on the script hooks, and the line
  `tree-construction script-processing` (line 539) as Q2 leaves it.
- `crates/html/tests/main.rs` and `crates/html/tests/script_hook.rs`: a module `script_hook` in
  the crate's one test binary.
- In the last commit, the public surfaces of acceptance 8.

## Acceptance

1. A hook that records its calls sees, in document order, one run call per script element Q1
   says runs and one skip call with its reason word for each row Q1 says is not run (`src`,
   `type=module`, `nomodule`, another type, inside `template` contents), over a document holding
   one element of each row; with scripting disabled it sees no call.
2. `no_op_hook_same_tree`: a set of documents written in the test (a script in head, in body, in
   a table, in `select`, in `template` contents, in foreign content, and one of each row of Q1),
   each parsed with scripting enabled once without a hook and once with a hook that does nothing,
   gives equal trees.
3. A hook that appends an element after the script: the element is in the final tree where it was
   put, and the markup after the end tag is parsed after it.
4. A hook that removes an open ancestor of the script (`<div><script></script>x</div>`, the hook
   removes the `div`): the text `x` goes where Q2 says (the removed `div`, as the standard's
   stack of open elements keeps it), and the parse ends without a panic.
5. Each public write over its limit (nodes, depth, attributes, stored bytes) answers that
   `Limit` and leaves the tree unchanged.
6. `slot task html-conformance` prints `html-conformance: 9107 cases, 9095 run, 9095 passed`, the
   line of `main` (no case runs a hook); `git grep -n 'script-processing'
   docs/design/html-parser.md` shows the line as Q2 leaves it.
7. `slot task check` exits 0; `git diff --stat` names only the files of the Change.
8. Public surfaces: the story names every public surface its change makes false (a badge, a
   `README.md` line, a website page, a count) and changes it in its last commit, every number
   from the log of a gate run on its tip, which the coordinating agent orders. For this story,
   lines on `main` at `29945f5` (2026-11-24): the test count of `README.md:11` and `:285-286` and `docs/badges/make.sh:38` with
   `docs/badges/tests.svg`; the gate block of `README.md:584-625` and of
   `website/docs/how-it-is-built.md:40-81`.
9. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; no model
   call and no network request in a test; nothing expensive is made per test (each case parses a
   document of a few lines); no Python; no generator with a committed output is added; a build
   or test job in the pool ends within 10 minutes or names its time with `slot --timeout`, at
   most 30 minutes; a test over 5 s is named.

## Out of scope

Running a script (`story:scripts-parse-run` joins the hook to swell); `document.write`, the
insertion point and a script nesting level above 1; SVG `script` elements beyond what Q1 says.

## Placement

After `story:scripts-decisions` (Q1, Q2); beside `story:webidl-reader`,
`story:scripts-engine-dependency`, `story:scripts-spec`, `story:session-process-hardening` and
`story:webidl-bindings`. `docs/design/html-parser.md` is also written by
`story:scripts-html-conformance`, after this story by its edges.

## Why `epic:scripts-in-a-page`

The parser's pause at the script end tag, which the epic runs a page's scripts in.
