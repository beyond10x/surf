---
format: aep.planning-md/3
id: story:html-script-hook
kind: story
status: implemented
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
  path: crates/html/src/limits.rs
- confidence: cited
  path: crates/html/src/tree.rs
- confidence: cited
  path: crates/html/src/tree_builder.rs
- confidence: cited
  path: crates/html/src/tree_builder/foreign.rs
- confidence: cited
  path: crates/html/src/tree_builder/formatting.rs
- confidence: cited
  path: crates/html/src/tree_builder/modes.rs
- confidence: cited
  path: crates/html/src/tree_builder/selectedcontent.rs
- confidence: cited
  path: crates/html/src/tree_builder/tables.rs
- confidence: cited
  path: crates/html/src/tree_builder/template.rs
- confidence: cited
  path: crates/html/tests/adversary_script_hook.rs
- confidence: cited
  path: crates/html/tests/adversary_script_hook_2.rs
- confidence: cited
  path: crates/html/tests/adversary_script_hook_3.rs
- confidence: cited
  path: crates/html/tests/adversary_script_hook_4.rs
- confidence: cited
  path: crates/html/tests/adversary_script_hook_5.rs
- confidence: cited
  path: crates/html/tests/adversary_script_hook_6.rs
- confidence: cited
  path: crates/html/tests/limits.rs
- confidence: cited
  path: crates/html/tests/main.rs
- confidence: cited
  path: crates/html/tests/script_hook.rs
- confidence: cited
  path: crates/script/tests/bindings.rs
- confidence: cited
  path: docs/badges/make.sh
- confidence: cited
  path: docs/badges/tests.svg
- confidence: cited
  path: docs/design/html-parser.md
- confidence: cited
  path: docs/design/scripts.md
- confidence: cited
  path: website/docs/how-it-is-built.md
- confidence: cited
  path: website/docs/status.md
- confidence: cited
  path: website/docs/what-works.md
revision: 22
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:12Z", actor: "human:operator", revision: 15}
- {from: "proposed", to: "active", at: "2026-11-24T12:08:07Z", actor: "human:operator", revision: 19, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-25T08:44:47Z", actor: "human:operator", revision: 22, decided_on: {"recorded":{"test_result":2,"verification":1}}, executor: "agent:coordinator"}
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

## As built

The unit is on `main` at `f1d0e4f` (the merge of `impl/html-script-hook` at `1925eaf`, the commit of
the public numbers, which follows `58d720b`: the tip on which the reviews and the fix rounds
ended).
Where it differs from the text above:

- Decision 1, the entry: `parse_document_with_hook(input, options, hook)` is the one parse that
  takes a hook. `parse_document_bytes` takes none, because its second parse after a `meta` that
  changes the encoding would call each script twice; `story:scripts-parse-run` decides how bytes
  are parsed with a hook. `parse_fragment` calls none: the fragment algorithm marks its scripts
  already started. The trait and the enum are as the decision gives them (`NotRun` has the
  eleven variants and derives `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq` and `Hash`);
  `NotRun::word()` gives each its word of Q1 and is public, which the story does not list. The
  parse's run loop calls the hook after the token that ended the script, not inside the token,
  so the tree builder holds no lifetime for it; the order of the calls is Q2's. After a `run`
  the names that the `html` and `body` merges keep are read again from the elements, and, when
  the hook moved or removed an element, so are the depths of the stack's entries.
- Decisions 3 and 4 and acceptance 2: every script element the parser inserts gets one call,
  never two and never none, not only the HTML `script` at its end tag and the two rows of
  decision 3. The call comes when the element leaves the stack of open elements or the parse
  ends, whichever is first, in the order the elements left. An HTML `script` that its end tag
  popped gets `run`, or `skip` with Q1's word; any other HTML `script` (open at the end of the
  input, or open when a limit stopped the parse) gets `skip` with `NotRun::Eof`; an SVG `script`
  gets `skip` with `NotRun::Svg` however it is closed: its end tag, `<script/>`, `</svg>`, an
  HTML end tag, a breakout start tag, `</script>` while an SVG child is the current node, or
  the end of the input. A MathML `script` is no script element and gets none, nor does an
  element a script inserted. The stack of open elements watches each script element it is
  given and notes the ones that leave it, so the SVG calls are made in `tree_builder.rs` and
  not in `foreign.rs`. The docs say "once", not "at most once"; Q1's rows `svg` and `eof` in
  `docs/design/scripts.md` say the same. This replaced the first version, which gave an SVG
  script no call unless its own end tag closed it and gave an HTML script a limit stopped none.
- Acceptance 1: the recording hook sees 43 script elements, one per row of Q1 that the tree
  builder decides (the head rows and the body rows: the `type` and `language` values, `src`,
  `nomodule`, `async`, `event` and `for`), plus a `template` script, an SVG script and a
  self-closing SVG script; with scripting disabled it sees no call. `NotRun::Eof` is not in
  this case; acceptance 2 shows it.
- Acceptance 3 and acceptance 6: acceptance 3 has a fifth script, `last`, after the writer, so
  that parsing is shown to go on; the calls are `empty`, `type`, `disconnected`, the writer's
  `run`, then `last`'s. Acceptance 6 as written cannot be met: `<div><script></script>x</div>`
  holds an empty script, which Q1 skips as `empty`, and a skipped script's hook gets `&Tree`,
  so it cannot remove the `div`. The test uses `<div id=d><script id=s>r</script>x</div>
  <p>after</p>`: the hook removes the `div`, the `div` still holds the script and `x` (the
  stack of open elements keeps it), `body` holds the `p`, the parse stops by no limit and does
  not panic. The coordinator accepted the changed markup.
- Acceptance 7 and the Change, the write surface: the 27 operations of the Change are public,
  each with a `write_` case in `script_hook.rs`; `create_element` and `remove` keep their
  crate-private names, so the public ones are `create_element_ns` and `remove_node`. A document
  fragment's children are inserted by `insert_before` or `append_child` given the fragment, as
  the DOM's pre-insert does. Public beyond the Change: `WriteError` (`Limit`,
  `HierarchyRequest`, `NotFound`, `IndexSize`, `InvalidNodeType`, `Namespace`, `NotSupported`),
  `WriteResult` and `Tree::limits()`, since a tree now keeps the limits of its parse and two
  trees parsed with different limits compare unequal. The cases try each limit an operation can
  reach (nodes, stored bytes, depth, attributes) and assert the answer and a tree equal to its
  clone from before; `remove_node`, `remove_attribute`, `remove_attribute_ns`, `adopt_node` and
  `delete_data` reach none. Every write that gives a node a new parent refuses a cycle through
  parents and template hosts with `HierarchyRequest`, and the walks it makes are bounded by the
  tree's node count.
- What the write surface leaves to its caller (its module and doc comments): names are not
  validated, an element has no namespace prefix, an attribute's namespace is one of six; offsets
  are UTF-16 code units and one between the two units of a surrogate pair answers `IndexSize`;
  cloning, importing or adopting the document answers `NotSupported`; a node removed keeps its
  place in the node count and its strings in the stored bytes, so removal gives no room back for
  nodes; a `NodeId` is of one tree, and an id of another tree is the caller's error; a write may
  cost time in proportion to the children or the subtree it touches, which no limit counts, and
  the caller charges for it. A write never stops the parse and counts no step.
- Decision 2, acceptance 4 and acceptance 8, the parser change outside the Change: a parse
  without a hook is not byte for byte the parse of `main`, because the depth limit bounds the
  tree now, in every parse, with a hook or without. On `main` it bounded the stack of open
  elements, and markup alone built a tree deeper than the limit (28 deep at a limit of 16, by
  the `selectedcontent` copy of an option, 2N - 4 at a limit of N). After any parse no node is
  deeper than `max_depth`, counted as the write surface counts: the elements on its path from
  the root, itself included, a template counted for the nodes of its contents; a subtree out of
  the tree is counted from its own top. This was ruled during the unit, on the review's finding
  that a hook's writes could pass a stack-only bound, and a switch that checked the tree only
  after a write was refused: the parser makes the same checks with a hook, without one, and with
  one that writes nothing, so the tree of a hook that writes nothing is the tree of no hook (six
  held tests compare them). The comparison with `main`, markup only, 248,788 inputs (the
  vendored and the repository's inputs and random ones, at depth limits 512, 16, 10 and 8): 147,129
  agree in tree, stop and steps; 80,822 differ in steps only; 20,837 differ in tree or stop:
  1,393 where `main`'s tree is past the limit, 14 where a prefix of `main`'s input passed it, and
  19,430 where `main` stopped by its stack bound with a tree inside the limit. Stops move from
  none to `Depth` in 113 lines and from `Depth` to none in 2,293. No tree of the unit is deeper
  than its limit (1,393 of `main`'s are), and no `Depth` stop leaves no element at the limit
  (17,231 of `main`'s do). Of the vendored `.dat` inputs only `tests19.dat#98` at a limit of 8
  differs. These counts were taken after fix round 4; round 5 left every tree and step count of
  the 248,788 lines as round 4 had them. Round 6 changes a parse only past the step limit and
  did not run the comparison again (not measured).
- Where the parser stops at `Depth` as built: before an element is inserted under a parent at the
  limit (every element insertion, the in-head `script`, the `template` start tag and the `html`
  element included; a limit of 0 makes no element); before an outer iteration of the adoption
  agency whose moves would put a node past the limit, checked for the whole iteration before its
  first move, so the iteration changes nothing; before an element of the `selectedcontent` copy
  under a node at the limit, which leaves the copy cut there, a prefix of the option. One
  exception is known and held by a test: an adoption agency stop before a move down can leave
  no element at the limit, because the iteration moves nothing
  (`an_adoption_agency_move_past_the_limit_stops_with_no_element_at_it`). Where the walk to a
  template's host (in the check before an insertion) or the walk under the furthest block (in
  the check of a move down) is counted, the step limit is read again before the stop: a parse
  past the step limit stops at `Steps`, not at `Depth` (round 6).
- How the parser knows a depth, as built after two rejected forms: no node's record keeps a
  depth (`NODE_BYTES` is 136 as before). The parser keeps the depth of each entry of the stack
  of open elements, set when the entry is pushed, and reads the entries again from the tree
  after a hook's write that moved or removed an element and after each adoption agency
  iteration (from the formatting element's place up; all of them when a hook put an entry under
  one above it). An insertion place is an entry, the parent of the foster parenting `table` or
  a template's contents, so a check reads a kept depth; another place is walked up, each hop a
  step. The adoption agency walks the furthest block's subtree, each node a step, only when its
  children would end deeper than they were; a move that makes no node deeper walks nothing of
  what it moves. The depth the parser checks is the real one: a node a hook removed is counted
  from its own top, and markup under it is not stopped by the depth it had. The first form kept
  a depth in each node's record; it ended `aaa:250:540000` at `Steps` where `main` parses it
  whole, and left a removed node at the depth of its last place. With a hook that writes
  nothing, one that writes into a detached node and one no-op move, the 125,750 steps of 500
  nested `div` and 200,000 `br` cost 537,603,743, 537,606,055 and 537,603,889 instructions.
- The stack of open elements has no bound of its own: `main` stopped the parse when an element
  would be entry 513 of the stack, and this unit removed that bound with the depth change
  (19,430 of the differing lines above). A node is on the stack at most once, so the node limit
  bounds its length and the step limit bounds every read of it; the memory statement of
  `lib.rs` says one entry and 4 bytes for each node, not `max_depth` entries. A hook can keep
  every element open and without a parent, which is why the next two bullets exist.
- Other parser changes outside the Change, each from a finding: the `html` and `body` merge
  counts the element's attributes, not their distinct local names, and matches a token's
  attribute only to an attribute in no namespace, so attributes a hook set in a namespace keep
  their place and count against `max_attributes`; a MathML `annotation-xml` whose start tag had
  an `encoding` of `text/html` or `application/xhtml+xml` gets
  `Categories::HTML_INTEGRATION_POINT` when it is created, and no later read of the attribute
  decides it, so a hook's write to `encoding` changes nothing; the adoption agency finds a
  template's host from the tree (`Tree::parent_or_host`), not from the stack, so a template a
  hook took off the stack cannot let it make a cycle.
- The limits as built, what a step is: one entry visited in a scan of the stack of open elements
  or of the list of active formatting elements, and one child visited in a child list, as on
  `main`; and now one hop of a walk up the tree to read a depth, one entry of the stack whose
  depth is read again (also one with no parent, which no hop reaches), one node of the
  furthest block's subtree that a move down walks, one entry that an insertion into the stack
  or a removal from inside it moves, one scope boundary the stack's list of them visits then,
  and one entry the `frameset` rule pops. Not counted: the pops of "stop parsing", a script
  reported to the hook (a step for it made a parse with a hook that writes nothing differ in
  steps from one without, which six held tests compare), the write surface's own work, and
  what the hook costs. The walks of the tree are counted, but the limit is not read inside them:
  the depth limit and a node's children bound them.
- The step limit is read inside the token (the last fix round): `main` read it after each token
  and processed the token that passed it whole, and with a hook that keeps the stack long one
  end tag of the adoption agency could run to any multiple of the limit. It is now read before
  each entry of every scan of the stack of open elements, its scope boundaries or the list of
  active formatting elements, before every write the parser makes to the tree, before every
  stop at another limit, and in the dispatch before a token is reprocessed. The scans include
  the adoption agency's inner and outer loops and the pass of `settle_entries` that reads the
  stack's depths again. Round 5 wrote "every loop"; the sixth review found that wrong, and the
  documentation (the crate documentation, `limits.rs` at `max_steps`, `docs/design/html-parser.md`)
  now names what is not read inside, with how each is counted: the marking pass of
  `settle_entries` (no step); the moves of the entries above and of the boundaries in
  `OpenElements::remove` and the moves of `insert` (counted at once); `truncate` and `clear`
  (the scan that found where to stop counted the pops); `walk_up` (a step for each hop, up to
  the node count); a child list's scan or move and the walk under the furthest block (a step
  for each node). Past the limit a scan ends with its answer for "not found", the write is
  not made, and the parse stops at `Steps`, never at another limit (round 6); an
  adoption agency iteration whose inner loop made clones puts them back at the place of step
  4.14 (step 4.23), skips step 4.22 and stops before step 4.24, so every node the parse made
  is in the tree. The behaviour change: the token that passes the limit is processed only up to
  the next read, not whole. A parse below the limit is unchanged, and so are the 248,788
  compared lines, steps included. The bound is written once, in `crates/html/src/limits.rs` at
  `max_steps`, and again in the crate documentation's "Limits": a parse ends fewer than
  4 x `max_nodes` steps past `max_steps` (4,194,304 at the defaults). Past the limit only the
  rest of the stretch between two reads and the putting back are counted; the longest stretch
  holds four pieces of work, each of fewer than `max_nodes` visits (a node is on the stack once,
  in one child list and under one node).
- The pops of "stop parsing" are not steps, by ruling. Counting them changes the steps of the
  last token, and `steps_exactly_at_the_limit_do_not_stop_the_parse` (`<p>x` at a limit of 2,
  ends whole) turns red. They run once per parse pass, after its last token, one for each entry
  left, so at most the node limit. A `meta` that changes the encoding ends the first pass and
  the caller parses the bytes again; that second pass takes no hook, so its stack stays inside
  the depth limit. The docs say "once per parse pass" and name this exception wherever they say
  the step limit bounds every read of the stack.
- The reads before a comment's and a processing instruction's write stay, though they cannot
  fire today: no rule scans anything between the start of such a token and its write, and the
  run loop reads after the previous token, so the count cannot pass the limit between them. The
  coordinator kept them as written, in round 5 and again after the sixth review; they are dead
  reads today and the one-line mutants on them, and on the two pairs with `admit`, are called
  equivalent (below), which the sixth review agreed with after trying them.
- Steps on markup alone rise where the stack's changes are now counted: the adoption agency run
  of `<a><span><div></a>` costs 29 steps in `crates/html/tests/limits.rs` (21 on `main`). The
  count there is the story's neighbour, `html-parser-limits`'s own test, changed by this unit,
  with a comment that names each of the 29. `dom.html` and `webidl.html` take the same steps as
  on `main`; the three `aaa` pages take at most 0.3 % more (measured, below).
- Files changed outside the story's `scope:` list, counted at `58d720b` with `git diff
  --name-only main...impl/html-script-hook`: 22 paths, 7 inside the list and 15 outside it. The
  15 outside: `docs/design/scripts.md` (two rows of Q1, `svg` and `eof`, the steps row and the
  citations of the parser's `limits.rs`; 15 `Settled:` lines and 0 `Open:` as before),
  `crates/html/src/limits.rs` (the limits' docs, the step bound),
  `crates/html/src/tree_builder/foreign.rs`, `formatting.rs`, `selectedcontent.rs`, `tables.rs`
  and `template.rs` (the depth checks, the reads of the step limit, the integration-point
  flag), `crates/html/tests/limits.rs` (one count and a comment), the reviewer's six files
  `crates/html/tests/adversary_script_hook.rs` and `adversary_script_hook_2.rs` to `_6.rs`,
  and `crates/script/tests/bindings.rs` (one citation of `limits.rs` in a comment, moved by
  round 6). The 7 inside the list are `lib.rs`, `tree.rs`, `tree_builder.rs`,
  `tree_builder/modes.rs`, `tests/main.rs`, `tests/script_hook.rs` and
  `docs/design/html-parser.md`. So acceptance 9's "names only the files of the Change" does not
  hold; the story's `scope:` gains the 15. The commit of the public numbers (`1925eaf`) adds
  six paths, outside the list too, which the `scope:` gains as well: `README.md`,
  `website/docs/how-it-is-built.md`, `status.md` and `what-works.md`, `docs/badges/make.sh` and
  `tests.svg`. The unit's other commits change none of them. In all the unit changes 28 paths,
  21 of them outside the list as the story wrote it.
- Decision 5 and the docs: `git grep -n -F script-processing docs/design/html-parser.md` shows
  `open tree-construction script-processing` with Q2's text at line 604 and
  `open parsing-main-inforeign svg-script-processing` as `main` holds it, at line 692. Beyond
  the Change's item 9 sentences (the table rows at lines 1089 to 1092, where the in-head
  `script` row is now "not a gap" and no longer lists `script-processing`), the file's item 7
  gains the limits table's new wording (the depth and steps rows, lines 376 and 380) and four
  paragraphs (lines 384 to 447) on what the depth limit bounds, how the parser knows a depth,
  the stack without a bound of its own, and the read inside the token. In the fifth fix round
  three comments of the unit named a coordinator's ruling by its number (in `html-parser.md`,
  `formatting.rs` and `script_hook.rs`); round 6 took those numbers out of the unit's own
  files. Still in the tree, naming a ruling by its number, which this repository does not
  otherwise do: `script_hook.rs` (lines 1543, 1695 and 2631, a ruling of fix round 3 and
  one of round 4) and the reviewer's files, which the unit does not edit
  (`adversary_script_hook_2.rs:377`, `_3.rs:531`, `_4.rs:189` and `:213`, `_5.rs:6`, `_6.rs:2`
  and `:678`).
- Acceptance 8: `slot task html-conformance` prints `html-conformance: 9107 cases, 9095 run,
  9095 passed` before the first change, after the implementation, and after each fix round
  including the last; no case runs a hook. The comparison of the vendored inputs with a hook
  that does nothing (`a_hook_that_writes_nothing_leaves_every_vendored_tree_as_it_was`) read 1,973
  inputs (all 62 `.dat` files of the vendored parsing resources, scripting enabled), made 220
  hook calls and found 0 trees that differ.
- Acceptance 9 and acceptance 10: acceptance 9 does not hold as written (the files above). The
  coordinator's gate ran on `edbd073`, the first tip: exit 0, `check-runner` 155 s, the test step
  11.1 s of its 120 s with 1540 tests; on the tip it is `exit 0, `check-runner` 170 s, the test step 40.4 s of its 120 s with 1845 tests, on `1925eaf`, at a load of 36.42 at the job's start and 61.58 at its end (on `58d720b`, at a load of 33.20 and 57.83: 231 s and 32.1 s)`. Acceptance 10's old strings
  are not in the tree: the story's `1976 passed`, `1,976` and `1,424, and 552` are not at the
  unit's base `859118d` (`README.md:11` holds `2056 passed` there, `README.md:285` `2,056` and
  `1,504, and 552`), and `main` has moved since. The numbers commit takes the strings `main`
  holds when the unit merges (`main` holds `2272 passed` at `README.md:11` and `2,275 tests` at
  `README.md:285`). The numbers the unit moves: `surf-html` 123 tests at the base to 245 at
  `58d720b` ("245 tests run: 245 passed, 0 skipped"); the unit adds 7 test files
  (`script_hook.rs` and the reviewer's six) to the 10 of `crates/html/tests` at the base, and
  `test-layout` on the merged tree prints "26 test directories, 230 test files", `main`'s own
  additions included (199 files at `5d9b011` before the merge of `main`); the lock is
  unchanged by the unit (`Cargo.lock` is not in the diff; `lock-check` on the merged tree
  prints "58 package(s), 25 from outside this repository", `main`'s additions included; 51
  and 19 before the merge). `The numbers commit `1925eaf` wrote, from the log of the gate on `58d720b` and from nothing else: `2394 passed` at the badge (`README.md`, `docs/badges/make.sh`, `docs/badges/tests.svg`), `2,397 tests` (1,845, and 552 of the TLS crates) in `README.md`, `website/docs/what-works.md` and `website/docs/status.md`, and the gate blocks of `README.md` and `website/docs/how-it-is-built.md`, 48 lines of that one run. `AGENTS.md` and `CHANGELOG.md` are unchanged: no binary calls the hook, and `surf-html` is not published.`
- Measured, without a hook, release, 20 parses, `perf stat -e instructions:u`, `main` against the
  unit at fix round 3, the last instruction count taken (the build node does not count
  instructions): `namedEntities.test` 14,553,163,938 against 14,397,645,760 (-1.07 %); all
  vendored `.dat` inputs of the parsing resources joined, 6,266,714,337 against 6,193,937,258
  (-1.16 %); one parse of `dom.html` 1,333,721,325 against 1,327,156,558 and of `webidl.html`
  1,317,124,856 against 1,310,060,362 (-0.5 %). An adoption agency page that moves a large
  subtree many times (`aaa:250:250000`, `aaa:60:1000000`) takes 1.042 times `main`'s
  instructions. Rounds 4 and 5 are measured in steps and, for round 4, in wall time (lowest of
  three, within 2.3 % of round 3's on five pages).
- Measured, the steps of the five pages, no hook, default limits, each ending with no limit
  named (`main`, then the tip): `aaa:250:530000` 691,626 and 693,626, `aaa:250:540000` 701,626
  and 703,626, `aaa:250:600000` 761,626 and 763,626, `dom.html` 137,527 and 137,527,
  `webidl.html` 145,398 and 145,398. The `aaa:N:S` pages are N pairs `<b id=k><span>`, a `div`
  with S `br`, then N `</b>` (2,124,895, 2,164,895 and 2,404,895 bytes); at most 1.003 times
  `main`'s steps. The last fix round's read of each is equal to the round before it.
- Measured, the review's page of pass 5, `<body><b>`, N `span`, a `div`, N `span`, a script after
  each 480th with a hook that moves its parent to the end of `body`, then `</b>`, release on the
  node, default limits. For N = 19,200, 38,400, 76,800 and 105,600 the parse stopped by `Steps`
  at 372,077,388, 1,487,578,748, 5,948,853,468 and 11,246,297,508 steps before the last fix
  round (84 times the limit and 26.9 s at 105,600, a 1.3 MB page), and at 134,235,863,
  134,221,803, 134,253,055 and 134,222,227 after it, at most 35,327 past 2^27 (134,217,728), in
  45.5 to 912.9 ms. The same pages without the `</b>` end with no limit named, at 3,130,163 to
  93,247,883 steps.
- Tests and times at `58d720b`, the implementor's run of
  `cargo nextest run -p surf-html --locked` after the merge of `main` and the sixth fix round:
  "245 tests run: 245 passed, 0 skipped". The 245 are the base's 123, the 62 of
  `script_hook.rs`, the reviewer's 59 in six files and one unit test in `tree.rs`. Two tests ran
  over 5 s in that run, both the reviewer's and both far under the 60 s bound of one test:
  `a_scan_ended_at_the_step_limit_names_no_other_limit` 5.862 s and
  `every_limit_over_random_pages_leaves_a_tree_and_one_call_per_script` 5.310 s. The slowest of
  the unit's own tests was `script_hook::a_hook_that_keeps_every_element_open_ends_at_the_step_limit`
  at 4.047 s (it names a bound of 10 s). The reviewer's own run at its tip read 1.2 s for the
  first of the two and 1.6 s for the second, and "Slowest 1.54 s; no test over 5 s"; the two
  runs were on different nodes and loads, and the difference is not explained. Before the
  merge of `main`, at `5d9b011`, `surf-html` ran 236 tests, 236 passed, 0 skipped, in 1.549 s,
  the slowest at 1.406 s. The dependents' run: the 9 packages that depend on it
  (`surf-page`, `surf-page-text`, `surf-script`, `surf-session`, `surf`, `surf-tls`,
  `surf-tls-conformance`, `surf-webidl` and `surf-html-conformance`, `surf-script` and
  `surf-webidl` new on `main`) ran "1344 tests run: 1344 passed, 3 skipped", slowest 5.064 s
  (a `surf-script` test, not the unit's), with the `ess` version the repository pins; a first
  run had 14 failures, all of `surf`'s `generated_check_*` cases, because the merge of `main`
  changed the spec and `task generated-fresh` had not run first, and it passed after it. At
  `5d9b011` the six crates that then used `surf-html` ran 627 tests, 627 passed. No test ran
  over 60 s in any run the reports give; an earlier reviewer's red case took 4.062 s, which was
  its own bound. A seeded property test (6,000 parses at depth limits 8, 10 and 16, with and
  without a hook that writes at random) checks the depth bound, no cycle and one call for each
  script, in 0.134 s (read in an earlier run, not measured again). One reviewer's assertion was
  changed under the depth ruling (`a_refused_write_leaves_the_parse_as_without_a_hook`, `>` to
  `<=`) and the reviewer lowered one of its own (`the_walks_of_a_shift_down_are_steps`, to one
  walk). The new test of round 6, `a_parse_past_the_step_limit_names_no_other_limit`, took
  0.148 s.
- Review: five passes of the adversary review, each with its cases merged into the unit as
  `crates/html/tests/adversary_script_hook*.rs`, and a sixth, on `5d9b011` (the reviewer's tip
  is `8b3332b`): 8 cases, 1 red, no must, the unit may merge (the last bullets). Pass 1, on `edbd073`: 23
  cases, 7 red, three musts. Pass 2, on `72de3b9`: 13 cases, 3 red, one must. Pass 3, on
  `d69866b`: 9 cases, 1 red, no must. Pass 4, on `4ce6829`: 4 cases and one rewritten, 1 red,
  one must. Pass 5, on `dc175e9`: 2 cases, 1 red, one must. The musts: (1) after a hook's
  writes the adoption agency could make a cycle through a template's contents, and a later
  write then walked it for ever; (2) the `html` and `body` merge counted distinct names, so
  attributes a hook set in two namespaces let markup pass the attribute limit; (3) an SVG
  script not closed by its own end tag got no call (an HTML script a limit stopped got none
  either, which the coordinator took in with it); (4) the adoption agency moved a subtree past
  the depth limit after a hook's writes (2N + 1 deep at a limit of N); (5) the work of
  `settle_entries` and of the scope boundaries on a stack a hook keeps long was quadratic and
  no step counted it (80,000 rounds took 71.9 s); (6) one end tag of the adoption agency did
  quadratic work inside one token and passed the step limit 84-fold (the page above). The
  shoulds the coordinator also had fixed: a script a limit stopped gets a call; the depth after
  a write; the `annotation-xml` integration point read from the element; the parser's switch
  between two bounds and the uncounted walk to the root behind it; a removed element's stale
  depth; the pass-5 wording and the frameset count's two off-by-one mutants.
- The mutants of round 5, by hand on a copy, as round 5 reported them: 49 runs, 34 caught, 15
  called equivalent; as round 6 corrects them: 49 runs, 39 caught, 10 equivalent (the report's
  lists give the same: caught W1, 27 reads, W33, W35, X1, X2, P1 and P2 are 34, equivalent W2,
  W3, W4, W6, W7, W8, W16, W27, W28, W29, W31, W32, W36, X3 and X4 are 15, and five of the
  15 are caught, which leaves 8 single mutants and 2 pairs). The five are W3, W4, W8, W16 and
  W31: round 5's reason for them, that no step comes between a read and the next one, is wrong,
  and the sixth review's held cases catch each (below). The text that follows is round 5's
  list, with the five moved. 45 one-line changes (a read of the step limit removed or
  weakened, one for each read; the count of the `frameset` pops) and 4 pairs that remove two
  reads at once. One test,
  `each_loop_over_the_stack_or_the_list_ends_at_the_step_limit`, catches the 27 reads of the
  loops, `nothing_is_written_past_the_step_limit` the pairs of the write reads,
  `the_pops_of_the_frameset_rule_are_steps` the two on the count (the review's two survivors),
  and `a_limit_inside_the_adoption_agency_leaves_every_node_in_the_tree` the missing put-back.
  The reviewer's held cases catch the other five, each by no other case:
  `every_limit_of_these_small_pages_ends_one_step_past` the reads of `admit` (W3), `room_at`
  (W4) and a character (W8); `the_advance_of_the_reconstruction_reads_the_limit_after_a_removal_from_the_stack`
  the read before each entry of the reconstruction's advance (W16);
  `a_limit_inside_the_foster_parenting_scan_of_step_4_14_ends_one_step_past` the read after
  step 4.14 (W31). The 10 equivalent are 8 single changes (W2 the dispatch read, W6 and W7 the
  comment's and the processing instruction's reads, W27 the agency's start, W28 the pop of
  step 2, W29 step 4.1, W32 after step 4.23, W36 the `frameset` rule) and 2 pairs (`admit` with
  the comment's read, `admit` with the processing instruction's); each removes a read that
  another read follows with no step and no write between, or one in a place no rule reaches
  with the count past the limit (the comment's and the processing instruction's reads, a pop of
  step 2 that only changes the stack which stopping clears). The sixth review tried W2, W27,
  W28, W29, W32, W36, W6 and W7 and agreed. The reads before the comment's and the processing
  instruction's write are dead today and could be removed; they stay as written. Round 6 added
  three mutants of its own, each caught: `merge_attributes` without its read (by the review's
  case and the new test), `room_at` stopping at `Depth` before reading and the adoption
  agency's check stopping at `Depth` before reading (by the new test).
- Left to other stories and to `main`: the doc link of `crates/html/src/tree_builder.rs:21` to
  the private `parse_fragment` stays; `cargo doc` with warnings denied fails on it, on `main`
  too. Parsing bytes with a hook is `story:scripts-parse-run`'s, as decision 1's entry above
  says. The write surface's limits of use (ids per tree, no room back for removed nodes, a
  write's uncounted time) are written into the stories that build on it,
  `story:scripts-dom-mutation` and `story:scripts-parse-run`; the Context of
  `story:scripts-parse-run` says the depth limit holds "after a hook has written", which is to
  read "in every parse". The story's "Out of scope" is untouched: no insertion point, no script
  nesting level above 1, no pause, no `document.write`, no run of an SVG `script`.
- The sixth pass of the review, on fix round 5 (`5d9b011`; the reviewer's branch tip is
  `8b3332b`, its red case `6af3b53`): no must, and "the unit may merge". Four shoulds and two
  notes; 8 new cases in `crates/html/tests/adversary_script_hook_6.rs`, 1 red and 7 held, all
  merged into the unit; no file under `src/` changed by the reviewer. Its own run at its tip:
  `cargo nextest run -p surf-html --locked` "244 tests run: 243 passed, 1 failed, 0 skipped"
  (236 and its 8; the one that failed is the red case of S2 below, fixed in round 6, after
  which the run at `58d720b` reads 245 passed); `html-conformance: 9107 cases, 9095 run, 9095
  passed`; clippy with `-D warnings` exit 0. Its sweeps (12 pages with 4 hook writes each at
  every limit, and 1,500 random pages at every limit) found no panic, no tree that is no tree,
  no node lost and no hook `run` after a stop. Its 7 mutants: 6 caught (5 of them only by its
  cases), 1 survives and is equivalent.
- Review 6, S1: five mutants of round 5's equivalent list (W3, W4, W8, W16 and W31) are not
  equivalent; its held cases catch them (above, with the corrected totals of round 5: 39
  caught and 10 equivalent, not 34 and 15). The reads stay.
- Review 6, S2 (the red case): `a_scan_ended_at_the_step_limit_names_no_other_limit`. A scan
  that the step limit ended (the scan for a `template` on the stack, before the `html` or
  `body` start tag merges attributes) answered "no template", and `merge_attributes` checked
  the attribute limit before the step limit was read: with an `html` of 1,024 attributes, 40
  `div`, a `template` and `<html b=2>`, the whole parse ends with no limit, and 42 of its 989
  step limits stopped naming `Attributes`. Fixed in round 6: `merge_attributes` reads the step
  limit first. The implementor searched every stop at a limit other than `Steps` back to the
  last read of the step limit and found the same order at two more places, `room_at` (the walk
  to a template's host is counted before the depth check) and the adoption agency's depth
  check (`place_depth` and `height_below` are counted walks before it); both read again before
  they stop at `Depth`. The new test `a_parse_past_the_step_limit_names_no_other_limit`
  (0.148 s) covers the attribute, depth, node and stored-byte limits, swept over every step
  limit, and the two counted walks; each of the three fixes undone on a copy is caught by it.
  The rule: a parse that has passed the step limit stops with `stopped_by` `Steps` and never
  names another limit.
- Review 6, S3 (wording), corrected in round 6 and checked on `58d720b`: the crate
  documentation (`lib.rs`, "Steps"), `limits.rs` (`max_steps`) and `docs/design/html-parser.md`
  ("Read inside the token") now say "every scan" and name the pieces not read inside, as the
  step-limit bullet above says; the steps row of `docs/design/scripts.md` (line 318) says the
  steps are read inside each token, before each entry of a scan and before each write; the
  citations of the parser's `limits.rs` in `docs/design/scripts.md` (lines 194, 313 to 318, 327
  and 889) and in `crates/script/tests/bindings.rs:906` follow the lines the unit moved (the
  rows to `limits.rs:103` to `:108`, the heap bound to `:45-51`, the removed-nodes count to
  `:34-35`). The other hits of `limits.rs:` in `docs` cite swell's file and are left.
- Review 6, S4 (speed): the tip of round 5 against `main`, release, default limits, no hook,
  each run parses its input 20 times, wall time, five runs, the lowest; the build node counts
  no instructions (`perf instructions:u supported: 0`). The review ran the tip in two jobs and
  puts the lower of the two into its percentage, which is not always the first number it
  prints. `dom.html`: `main` 2,278.4 ms, the tip 2,433.6 and 2,426.5, +6.5 %; `webidl.html`:
  2,262.3, the tip 2,420.2 and 2,674.6, +7.0 %; `aaa:250:540000`: 3,146.7, the tip 3,271.1 and
  3,240.8, +3.0 %. So the unit is 3.0 to 7.0 % slower in wall time than `main`, above the 2 %
  of the review's brief. The spread inside one job reaches
  24 % (`main`'s `dom.html`, 2,278 to 2,814), and the round's own share (`215566c` to the tip)
  is -5.6 to +3.2 %, inside the noise; the steps are those of round 4. Ruling: the finding
  gets no code change in this unit and is recorded here; a separate draft story counts the
  parser's instructions, without a hook, on a host that counts them. There is no instruction
  count for this tip (not measured).
- Review 6, N1: the longest stretch the reviewer reaches, steps 4.26 to 4.30 of the adoption
  agency (the removal, the insertion and the move of the furthest block's children) with a
  hook that keeps 11,600 `span` open at a `max_nodes` of 12,000: 742,881 steps, 34,803 past the
  limit, 2.90 x `max_nodes`, under the bound of 4 x `max_nodes`; by its count that stretch is
  at most 3 x `max_nodes`, so the derivation's 4 x is supported and not reached.
- Review 6, N2: no hook `run` comes after a stop. `run` follows only a `</script>` end tag in
  the text mode, which counts and reads nothing, and the run loop reads after each token, so a
  token that ends a script that runs cannot pass the limit. A `script` open at a stop gets
  `skip` with `eof` once, after the final pops, and an SVG `script` that a cut scan pops gets
  `skip` with `svg` after that token; the docs promise both, and the code matches.
