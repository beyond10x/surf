---
format: aep.planning-md/3
id: story:html-script-hook
kind: story
status: proposed
title: The tree builder calls a hook at an inline classic script's end tag, and Tree gains a public write surface held to the parser's limits
summary: With scripting enabled, the text insertion mode calls a hook at each parser-inserted script end tag, to run it or to report it skipped with its reason, and parsing goes on over the tree the hook left; public writes answer the limit they reach; a parse without a hook is unchanged.
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
- confidence: cited
  path: crates/html/tests/main.rs
- confidence: cited
  path: crates/html/tests/script_hook.rs
- confidence: cited
  path: docs/design/html-parser.md
revision: 18
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:12Z", actor: "human:operator", revision: 15}
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

1. The hook is the one of `docs/design/scripts.md`, Q2: a trait `ScriptHook` in `surf-html` with
   `run(&mut self, tree: &mut Tree, script: NodeId)` and
   `skip(&mut self, tree: &Tree, script: NodeId, reason: NotRun)`, and an enum `NotRun` with eleven
   variants, `Empty`, `Src`, `Type`, `Module`, `Importmap`, `Speculationrules`, `Disconnected`,
   `Nomodule`, `Event`, `Svg` and `Eof`: one per word of Q1's line `reasons:` (`empty`, `src`,
   `type`, `module`, `importmap`, `speculationrules`, `disconnected`, `nomodule`, `event`, `svg`,
   `inserted`, `eof`; 12 words) but `inserted`, which no end tag reports. This story invents no
   word. The tree builder calls one of the two for every parser-inserted HTML `script` end tag
   while scripting is enabled and a hook is set, deciding the row of Q1's table from the element as
   the parser made it, so `story:scripts-parse-run` can turn every skipped element into an end
   `not run: <word>`.
2. A parse without a hook, or with scripting disabled, is the parse of today: no call is made, and
   the tree is the same.
3. The rows of Q1 that are not an end tag of an HTML script: a `script` element that is the current
   node when the end-of-file token arrives in the "text" insertion mode is popped, never prepared,
   and reported with `skip` and `NotRun::Eof` (Q1, Q2; HTML Standard 13.2.6.4.8, "An end-of-file
   token"); an SVG `script` at its end tag in foreign content is reported with `skip` and
   `NotRun::Svg` (Q2).
4. The tree builder calls the hook at most once per element, at the parser's end tag or at the end
   of the input. An element skipped for `empty`, `disconnected` or `type` that a script later gives
   text, a usable `type` or a connected place gets no second call, and its first call stands (Q1's
   row "no second record; its first record stays", a deviation the note names: a browser runs it
   then). An element skipped for `event`, `nomodule` or `src` and later changed or moved gets no
   second call either, as in a browser. An element a script inserts gets no call from the tree
   builder; its `not run: inserted` is written by the DOM write that connects it (Q7,
   `story:scripts-parse-run`).
5. The `html-sections` line `open tree-construction script-processing`
   (`docs/design/html-parser.md:539`) stays open, with the text Q2 gives it: "The script end tag in
   the text insertion mode: the insertion point, a script nesting level above 1, the parser pause
   and the pending parsing-blocking script, and `document.write`." The line
   `open parsing-main-inforeign svg-script-processing` (`docs/design/html-parser.md:627`) stays as
   it is.

## Change

- `crates/html/src/tree_builder.rs`, `tree_builder/modes.rs`, `lib.rs`: the hook of decision 1,
  called after the steps Q2 marks "taken", and the calls of decision 3 at the end of the input and
  in foreign content; parsing goes on with its stack of open elements and list of active formatting
  elements over the tree the hook left, including elements the hook removed or moved, as Q2 says.
- `crates/html/src/tree.rs`: the whole public write surface the DOM slices need, so that this
  file stays this story's alone: create an element, a text node, a comment, a processing
  instruction and a document fragment; insert before, append, remove, replace, and move a node
  (`moveBefore`, keeping the node); insert a document fragment's children; clone a node, shallow
  or deep (`cloneNode`, and `importNode`, which in the one document of a parse clones);
  adopt a node (`adoptNode`, which in that document removes it from its parent); normalize a
  subtree (merge adjacent text nodes, drop empty ones); split a text node at an offset
  (`splitText`); set a character data's text and replace a range of it (`data`, `appendData`,
  `insertData`, `deleteData`, `replaceData`); replace all children by one text node
  (`textContent`); set, remove and toggle an attribute, with and without a namespace
  (`setAttribute`, `setAttributeNS`, `removeAttribute`, `removeAttributeNS`,
  `toggleAttribute`). Each is admitted against the parser's limits and answers the `Limit` it
  reached instead of writing. A slice that finds an operation missing adds it to this file and
  names it in its report.
- `docs/design/html-parser.md`: item 9's sentences on the script hooks, and the text of the line
  `open tree-construction script-processing` (line 539) as decision 5 gives it.
- `crates/html/tests/main.rs` and `crates/html/tests/script_hook.rs`: a module `script_hook` in the
  crate's one test binary.
- In the last commit, the public surfaces of acceptance 10.

## Acceptance

1. A hook that records its calls, over a document holding one element of each row of Q1 the tree
   builder decides, sees in document order one `run` call per element Q1 says runs and one `skip`
   call per element Q1 says is not run, with that row's variant; each variant of `NotRun` but `Eof`
   is seen at least once (`Empty`, `Src`, `Type`, `Module`, `Importmap`, `Speculationrules`,
   `Disconnected` from `template` contents, `Nomodule`, `Event`, `Svg` from an SVG `script` in
   foreign content); with scripting disabled it sees no call.
2. `eof_reports_open_script`: `<script>x = 1` with no end tag, parsed with scripting enabled and
   the recording hook, gives one call, `skip` with `NotRun::Eof`, and no `run` call; the tree is
   the tree of today's parse of the same input.
3. `no_second_call`: a document of three skipped elements (one empty, one with
   `type="text/template"`, one in `template` contents) followed by a script whose `run`, through
   the write surface, gives the first one text, sets the second one's `type` to `text/javascript`,
   moves the third into `body` and inserts a new `script` element with text: the recording hook
   sees each of the three skipped elements once, with its first word, and no call for the inserted
   element.
4. `no_op_hook_same_tree`: a set of documents written in the test (a script in head, in body, in a
   table, in `select`, in `template` contents, in foreign content, and one of each row of Q1), each
   parsed with scripting enabled once without a hook and once with a hook that does nothing, gives
   equal trees.
5. A hook that appends an element after the script: the element is in the final tree where it was
   put, and the markup after the end tag is parsed after it.
6. A hook that removes an open ancestor of the script (`<div><script></script>x</div>`, the hook
   removes the `div`): the text `x` goes where Q2 says (the removed `div`, as the standard's stack
   of open elements keeps it), and the parse ends without a panic.
7. Each operation of the write surface in the Change has a named case, and each write over its
   limit (nodes, depth, attributes, stored bytes) answers that `Limit` and leaves the tree
   unchanged.
8. `slot task html-conformance` prints `html-conformance: 9107 cases, 9095 run, 9095 passed`, the
   line of `main` (no case runs a hook);
   `git grep -n -F script-processing docs/design/html-parser.md` shows the line
   `open tree-construction script-processing` with the text of decision 5 and the line
   `open parsing-main-inforeign svg-script-processing` as `main` holds it.
9. `slot task check` exits 0; `git diff --stat` names only the files of the Change.
10. Public surfaces: the story changes, in its last commit and after merging `main`, every public
    surface its change makes false, each number taken from the `slot task check` log of its tip.
    After that commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges` prints
    nothing for each old string below, as the story's base holds it (the strings and lines given
    are those of `main` at `29945f5`, 2026-11-24); each new number equals the one in that log; and
    the gate block, `README.md:584-625` and `website/docs/how-it-is-built.md:40-81`, holds that
    log's lines. The old strings of this story: the test count `1976 passed` (`README.md:11`,
    `docs/badges/make.sh:38`, `docs/badges/tests.svg`), `1,976` and `1,424, and 552`
    (`README.md:285`, `website/docs/status.md:23`, `website/docs/what-works.md:17`).
11. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; no model
    call and no network request in a test; nothing expensive is made per test (each case parses a
    document of a few lines); no Python; no generator with a committed output is added; a build or
    test job in the pool ends within 10 minutes or names its time with `slot --timeout`, at most 30
    minutes; a test over 5 s is named.

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
