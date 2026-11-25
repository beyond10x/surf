---
format: aep.planning-md/3
id: story:scripts-dom-mutation
kind: story
status: proposed
title: The mutating members (46 rows), four setters and four constructors, charged first
summary: Tree writes, charged before they run, with the standard's exceptions and the limits' ends.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:html-script-hook
- depends_on: story:scripts-dom-reading
- depends_on: story:scripts-parse-run
- depends_on: story:scripts-dom-attributes
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/html/src/tree.rs
- confidence: cited
  path: crates/script/src/bindings.rs
- confidence: cited
  path: crates/script/src/dom/mod.rs
- confidence: cited
  path: crates/script/src/dom/mutation.rs
- confidence: cited
  path: crates/script/src/dom/mutation_reads.rs
- confidence: cited
  path: crates/script/src/dom/mutation_writes.rs
- confidence: cited
  path: crates/script/src/dom/reads.rs
- confidence: cited
  path: crates/script/src/runner.rs
- confidence: cited
  path: crates/script/tests/bindings.rs
- confidence: cited
  path: crates/script/tests/dom_mutation.rs
- confidence: cited
  path: crates/script/tests/dom_mutation_cost.rs
- confidence: cited
  path: crates/script/tests/dom_reading.rs
- confidence: cited
  path: crates/script/tests/main.rs
- confidence: cited
  path: crates/webidl/selection.txt
- confidence: cited
  path: crates/webidl/src/prelude.rs
- confidence: cited
  path: docs/badges/make.sh
- confidence: cited
  path: docs/badges/tests.svg
- confidence: cited
  path: docs/design/scripts.md
- confidence: cited
  path: website/docs/how-it-is-built.md
- confidence: cited
  path: website/docs/status.md
revision: 43
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:13Z", actor: "human:operator", revision: 15}
---
## Context

The epic's ninth candidate (lines 469-472) and its slice "Tree mutation, 29" (lines 732-737),
read at plan commit `2e38276`: the mutating members of `Node`, `Element`, `Document` and of the
interfaces they return, with the DOM Standard's exceptions, held to the parser's limits.

What `crates/webidl/selection.txt` holds for this slice at `83e6b8a` (263 lines):

- 50 rows with the slice `mutation`: 46 members (`Node` 6, `Element` 10, `Document` 13,
  `CharacterData` 8, `Text` 1, `DocumentType` 4, `DocumentFragment` 4) and 4 constructors (`Text`
  line 196, `Comment` 201, `ProcessingInstruction` 203, `DocumentFragment` 221).
- 4 rows of `reading` with `setter: mutation`: `Node nodeValue` (line 60), `Node textContent`
  (61), `Document title` (180), `CharacterData data` (182).
- So the trait `Mutation` has 54 methods (`crates/script/src/bindings.rs:3267 @ 83e6b8a`).
- `Document body` and `Document head` are not rows. `Document documentElement` is a `reading` row
  (line 146). A case reaches an element through `document.documentElement` or `getElementById`.
- `Document createRange` is a row of `throws` (line 162, reason ranges); `Document constructor`
  (line 136, reason documents made by script) and `EventTarget constructor` (line 29, reason
  events) throw too. Their messages are `"{member} is not supported: {reason}"` and
  `"{name} constructor is not supported: {reason}"` (`bindings.rs:2045`, `:2036 @ 83e6b8a`).
- Rows 150 and 151 (`createElement`, `createElementNS`) carry the deviation "an element's wrapper
  is an Element".

What the glue (`crates/webidl/src/prelude.rs`, generating `bindings.rs`) does for every member
at `83e6b8a`: one step, the brand check, the copy of each `String` argument, the slice's method,
then `finish`. A method answers `DomResult`; `DomEnd::Limit` ends the script with a parser limit
(`bindings.rs:120`). `end_with_limit` (`bindings.rs:512`) turns a `Limit` into the end. The node
charge `charge_node` (`bindings.rs:284`) takes 3 x `NODE_BYTES` (136, so 408 bytes) plus twice
the stored bytes through `charge_host_bytes`, counts one node in `totals.nodes`, and never
refunds. A variadic argument arrives as `Vec<NodeOrString>` (`bindings.rs:1116`); the conversion
`to_nodes_or_strings` (`bindings.rs:1135`) charges no step for an item. `Copies::copy`
(`bindings.rs:620`) charges `units / 64` after the copy; the reading story's price commit
(16 bytes a step, before the copy, `STRING_BYTES_A_STEP`, `bindings.rs:1257`) is not built at
`83e6b8a`.

What the tree's write surface does at `83e6b8a` (`crates/html/src/tree.rs`): it counts no step.
Each write walks, and the walks are not constant:

- `insertion_allowed` (951) checks the parent kind and the ancestor, then NotFound, then the
  node kind. For a `Document` parent it calls `document_allows` (900), up to 5 passes over the
  children.
- `admit_depth` (848) calls `element_height` (828), a pass over the moved subtree, and
  `element_depth` (811), a climb. `is_host_including_inclusive_ancestor` (780) climbs too.
- `remove` (524) scans the parent's child list from the back (`rposition`), then `Vec::remove`
  shifts the tail, then `shrink` (710) may reallocate. `remove_node` (1104) does not check the
  parent: `removeChild` must check it itself.
- `clone_node` (1202) gives `NotSupported` for a `Document`; it calls the write counter per
  copied child. `clone_cost` (1175) is private.
- `normalize` (1309) detaches each removed text node, so it is quadratic in the adjacent run;
  merged bytes are stored again.
- `replace_data` (1386) throws IndexSize when the offset is over the length, and measures the
  UTF-16 length with an O(length) pass on every call.
- `set_text_content` (1441) makes the text node first, then removes the children.
- A removed node stays in the arena with its strings counted (`docs/design/scripts.md:193`).
- The only places that make a `Limit` are `tree.rs:733` (Nodes), `:736` (StoredBytes), `:865`
  (Depth) and `:1517` (Attributes). `Limit::Attributes` is reached only through
  `put_attribute`, which no mutating member calls. `Limit::InputBytes` and `Limit::Steps` are
  made nowhere in `tree.rs`.
- Defaults (`crates/html/src/limits.rs`): `max_depth` 512, `max_nodes` 1,048,576,
  `max_attributes` 1,024, `max_stored_bytes` 67,108,864.

Memory comes before nodes. The engine's `memory_bytes` is 67,108,864; a node charges 408 bytes
(so at most 164,482 nodes alone), and a wrapper with its node data takes 423 bytes
(`docs/design/scripts.md:329-332 @ 83e6b8a`, measured on the reading slice), so 831 bytes a
created node and 80,756 nodes. At the default limits a script that makes nodes ends
`limit: memory` before `limit: nodes`. A case that wants `limit: nodes` lowers `max_nodes` in its
own `ParseOptions` and `Document.limits`.

`crates/script/src/runner.rs:719 @ 83e6b8a` maps `Limit::InputBytes | Limit::Steps` to
`ScriptEnd::Limit(Reason::WallClock)` under a comment at lines 714-718. No member of this slice
makes either; this story shows that with a source test and edits that comment only.

Found by the reviews of the stories before this one, for this story to hold with a test:

- A write over a limit leaves the tree as it was, and the script's end is the limit's word.
- A reader cannot be made to copy more than it paid for: the refill of a kept collection copies
  the old list and the child list before it charges (reading story, decision 20, the "Open"
  item). The mutation slice is what makes a write between two reads possible, so this story
  closes it.
- A step must buy a bounded amount of work; a loop of writes must end within the step limit
  (`steps_per_entry` 10,000,000, `runner.rs:41 @ 83e6b8a`) or finish well inside it.
- A cost module records no time assertion and holds floors of steps that break when a charge is
  removed.

A `NodeId` belongs to one tree; the bindings never hand a node of one document to another
document's tree.

## Decisions taken

1. Three files and a source test. `crates/script/src/dom/mutation.rs` holds one call per member
   and nothing else: no `tree()`, no `tree_mut()`, no charge. `mutation_reads.rs` holds the
   reads, `mutation_writes.rs` the writes, the charges and the error map. `reads.rs` changes
   only by visibility. Each function does constant work, or charges before its loop; a walk of
   unknown end charges step by step, before it reads each node. `SurfMutation` stays a unit
   struct: all surf documents are HTML documents, so it needs no state.
2. Prices. One step per 16 bytes, an argument's bytes included (the glue's copy, once the
   reading story's price commit is in). A step per node made, moved, removed, cloned or visited.
   A step per 4 attributes (`ATTRIBUTES_A_STEP`, `reads.rs:252`). 8 steps per collection node in
   the glue (`COLLECTION_NODE_STEPS`, `bindings.rs:1251`). A read of a child list of `c` ids
   costs `ceil(c x size_of::<NodeId>() / 16)` steps (`c / 2` for an 8-byte id; the cost module
   prints the size) and is called a list step below; `visit(n)` is `1 + ceil(attributes / 4)`.
   Each charge comes before the work it pays for. A climb follows `parent_or_host`
   (`tree.rs:773`, made `pub`) one step before each read; `reads.rs::ancestors` collects the
   chain before it charges, so this slice does not use it.
3. What each member charges, in order, and the passes it makes (the tables below). The glue's
   one step comes first for every member. The implementor recounts every pass in `tree.rs` at
   its base and writes the count into the doc comment of the charge; a count that is higher
   than the table's is a defect of the table, fixed in the same change.
4. Memory. `charge_nodes(count, stored_bytes)` (a new bulk form in the glue; `charge_node(s)`
   becomes `charge_nodes(1, s)`) comes before any node is made, so a loop of
   `createTextNode` is charged by its count and not by an item. A node's own strings and the
   attributes of a clone go through that charge: twice their bytes, once. The attributes story's
   `charge_stored` and `undo_stored` (with `live` and `peak`) are called for the bytes a write
   adds to a node that exists (the character-data writers); this story adds no second pair. A
   removed node is not refunded: its 408 bytes and its strings stay charged, because the node
   stays in the arena. A loop that makes and removes nodes ends `limit: memory`, at the session's
   limits, before `limit: nodes`.
5. A write over a limit ends through `end_with_limit` with the limit's word (`limit: nodes`,
   `limit: depth`, `limit: stored-bytes`, `limit: memory`), also inside `try { } catch { }`, and
   the tree is as before: node count, serialization and `writes()` unchanged. The table below
   names each limit a member can reach and the test that reaches it. `Limit::Attributes` is
   unreachable here: a source test shows `mutation_writes.rs` calls none of `put_attribute`,
   `set_attribute`, `remove_attribute` or the other attribute functions.
6. The write counter. `Tree::writes()` (`tree.rs:331`) raises when a write changes a child
   list. The table below says, for each write function, whether it raises. A kept `childNodes`
   (the same object refilled) and a kept `getElementsByTagName` snapshot are read before and
   after a write by a test per row of the table. Reading decisions 17 and 18 stand: a snapshot
   shows a change on a fresh call only; a `[SameObject]` list is refilled when the counter
   moved. The open item of decision 20 is fixed in the glue: `refresh_collection`
   (`prelude.rs:1321`) reads the old list's length and the child list's length, charges
   `8 x (old_len + children_len)` and then clones and copies; it charges nothing again later.
   The getter's own copy of the child list (`reading.rs:978`) is covered by that charge. A spy
   `reading` slice with three methods (`window_document`, `node_child_nodes`,
   `document_get_element_by_id`) proves the order by counting calls of `node_child_nodes`
   when the charge ends the script.
7. No recursion. `cloneNode(true)`, `normalize`, the `textContent` setter, the removal of a
   subtree and the validity check use their own stack or indexes. The implementor reads those
   functions in `tree.rs` for a recursive call; a function that recurses is rewritten in the
   same change (`tree.rs` is in the scope).
8. A `script` element a script inserts is recorded by `record_inserted(cx, node)` of
   `story:scripts-parse-run` and not run. The path of each member is in the table below. The
   call is idempotent per node (that story's rule), so an element the parser reported gets no
   second record, and an element moved out and back in gets one.
9. `other_slices_still_throw` (`crates/script/tests/dom_reading.rs:2399-2432 @ 83e6b8a`)
   asserts at line 2424 that `document.appendChild(el)` throws `Node.appendChild is not
   supported`. This story changes that one line to `document.createRange()` with the message
   `Document.createRange is not supported: ranges`, and no other line of the test. The reading
   module keeps its own `TestMutation` (`dom_reading.rs:73`, installed at `:130`), as do
   `dom_reading_cost.rs` and the `adversary_dom_reading*` modules, which replace the mutation
   slice. The three tests of `crates/script/tests/bindings.rs` that assert `createComment`,
   `appendChild`, `new Text` or `new DocumentFragment` throw
   (`a_member_of_a_slice_throws_until_its_slice`,
   `the_strings_of_a_dom_exception_outlive_a_collection`,
   `an_uncaught_dom_exception_names_its_name_and_message`) switch to members that still throw:
   `document.createRange()`, `new Document()` (message `Document constructor is not supported:
   documents made by script`) or `new EventTarget()` (`EventTarget constructor is not supported:
   events`).
10. A named deviation goes into the epic and beside its row in `selection.txt`, as a
    `deviation:` clause; the edit is in the story's plan edits. A member the epic excludes
    stays `throws`: `createRange`, the `Document` constructor, the `EventTarget` constructor,
    `createTreeWalker`, `createNodeIterator`.
11. A test's time bound is at least ten times its own time, or there is none
    (`AGENTS.md:1966 @ a14e3b0`). A case that checks a loop ends inside the step limit asserts
    the end word, not a time.
12. `crates/script/src/bindings.rs` is generated and is never merged by hand. The second of the
    stories that change it takes `main`, regenerates it, and `slot task webidl-check` and
    `ESS_TOOLCHAIN=0.97.4 slot task webidl-slow` hold it (as the attributes story's Placement
    says).
13. The exceptions come in the standard's order, and the slice leaves the order to the tree where
    the tree has it: ensure pre-insertion validity (`dom.html:4061-4183 @ a14e3b0`) checks the
    parent kind (4069), the ancestor (4074), NotFound (4079), the node kind (4084), a doctype
    under a non-document (4095), then the Document-parent checks (4104-4183); `moveBefore`
    (4502-4685) checks root (4508), ancestor (4517), NotFound (4522), kind (4527), Text in a
    document (4532), element in a document (4537). The slice charges before the call and maps
    `WriteError` to the standard's `DOMException`: `HierarchyRequest`, `NotFound`, `IndexSize`,
    `InvalidNodeType`, `Namespace`, `NotSupported` keep their names; `Limit` ends the script.
14. The pre-charge rule. The tree counts no step, so the slice charges by formula before the
    call, from lengths it reads in constant time or walks step by step. The formulas are upper
    bounds of the tree's passes. The tree is not given a step budget (question 1).

### What each member charges and does

`d` is the depth of the target parent (its ancestors), `s` the nodes of the subtree being moved
or copied (with attributes `a` and stored bytes `b`), `m` the nodes put into a child list (1,
or the children of a fragment), `c_p` the children of the parent, `c_o` the children of the
node's old parent, `n` the items of a variadic list, `k` its strings, `B` their total bytes.

- Create (11 rows): `createElement`, `createElementNS`, `createTextNode`, `createComment`,
  `createDocumentFragment`, `createProcessingInstruction`, `createCDATASection`, and the four
  constructors.
  - Reads: none. Writes: `Tree::create_*` (`tree.rs:1027-1074`), no parent.
  - Order: the glue's step and copy; the exception, as one pass over the copied argument
    (priced by the copy); then `charge_nodes(1, stored)` (2 for a `template`, which makes two
    nodes, `tree.rs:1027`) and one step; then the tree call.
  - Exceptions: `createElement`: InvalidCharacterError for a name that is not a valid element
    local name (`dom.html:775-809`, 8384); then ASCII-lowercase, the HTML namespace, `options`
    ignored (no custom elements). `createElementNS`: validate and extract (`dom.html:836-908`):
    InvalidCharacterError for the prefix (867) and the local name (881), then NamespaceError
    (886, 891, 896, 902); then NotSupportedError for a namespace outside the six values of
    `Namespace` (`tree.rs:79`) or for a prefixed name (deviation 1: the tree's element has no
    prefix). `createCDATASection`: NotSupportedError always, first (8512), before the `]]>`
    check. `createProcessingInstruction` and its constructor: InvalidCharacterError when the
    target does not match the XML `Name` production (12019) and when the data holds `?>`
    (12024). The other creates and constructors have none. A constructor is the matching
    `createX` on the one document tree.
  - Limits: Nodes for every row but `createCDATASection`; StoredBytes for the 8 rows that store
    a string. Counter: none (`create_*` does not raise).
- Insert, one node (4 rows): `Node insertBefore`, `Node appendChild`, `Element
  insertAdjacentElement`, `Element insertAdjacentText`; the variadic ones are below.
  - Reads: the parent's depth, the node's subtree, the old parent's list. Writes:
    `Tree::insert_before` (`tree.rs:1079`): validity, then `admit_depth`, then `insert_checked`
    (987).
  - Order: the glue's step; the argument copies; an own climb of `this`, one step before each
    ancestor read (`d`); an own walk of the node's subtree on a stack, `visit` before each
    node (`s`), which also lists the script elements; then, before the call, `2d + s + m` steps
    and the list steps: 2 x `list(c_p)` when `child` is given, 1 more when the node is already
    a child of the parent, 3 x `list(c_o)` when the node has a parent, 5 x `list(c_p)` when
    the parent is a `Document` (`document_allows`); then the call; then `record_inserted`.
    An append of a parentless node to a non-document parent adds no list step, so a loop of
    appends is cheap.
  - `insertAdjacentElement(where, element)` and `insertAdjacentText(where, data)`: `where` is
    matched ASCII case-insensitively against `beforebegin`, `afterbegin`, `beforeend`,
    `afterend` (`dom.html:11179-11221`), SyntaxError otherwise, before any node is made
    (`insertAdjacentText` makes its text node after the check, `charge_nodes(1, len)`).
    `beforebegin` and `afterend` answer null (and write nothing) when `this` has no parent.
  - Exceptions: those of decision 13. Limits: Depth; Nodes and StoredBytes for the text node of
    `insertAdjacentText`. Counter: raises when the child list changes.
- Variadic `ParentNode` (9 rows): `prepend`, `append`, `replaceChildren` on `Element`,
  `Document`, `DocumentFragment`.
  - Order: the glue charges one step for each of the `n` items before the loop (a change to
    `to_nodes_or_strings`, `prelude.rs`); then `charge_nodes(k, B)` and `k` steps before any
    node is made; then convert (`dom.html:5016-5041`): no items does nothing, one item is
    used as it is, more make a fragment and insert each item, each paying its own insert as
    above, with the old parent's removal; then the insert of the result (`prepend` 5163,
    `append` 5179).
  - `replaceChildren` (5194): after the convert, `2c_p` steps (a step for each child removed
    and each child tested), the removal, then the insert. The validity check runs before the
    removal (`dom.html:4760-4795`), so a refused `replaceChildren` leaves the children.
  - Exceptions, limits and counter: as an insert; `replaceChildren` also raises when it
    removes. Limits: Depth, Nodes and StoredBytes (the text nodes).
- `ChildNode` (12 rows): `before`, `after`, `replaceWith`, `remove` on `Element`,
  `CharacterData`, `DocumentType`.
  - `before`, `after`, `replaceWith` (5335, 5370, 5399): the viable sibling is computed before
    the conversion: `list(c_p)` for the scan of `this`'s index, `n` steps for the membership
    set of the items, `c_p` steps for the walk; then `charge_nodes(k, B)`; then the convert;
    then the reference node (an index scan, `list(c_p)`, when the viable sibling changed);
    then the pre-insert as above. `replaceWith()` with an empty list is a removal.
  - A `this` with no parent does nothing and charges nothing more than the glue's step.
  - `remove` (5436): no parent, nothing. Else the removal below.
- Remove and replace (`Node removeChild`, `Node replaceChild`, and the removals above).
  - `removeChild(child)`: read `parent(child)` (constant); if it is not `this`, NotFoundError
    (`dom.html:4810`), before any charge. Else `3 x list(c_p)` steps (the `rposition`, the shift,
    the shrink) and 1 for the node, then `Tree::remove_node` (`tree.rs:1104`). Counter: raises.
  - A loop that removes from the front is quadratic in the list: about `0.75 n^2` steps, so
    5,000 removals from the front end `limit: steps`, while the same from the back is linear.
    That is the price, and the cost module holds both.
  - `replaceChild(node, child)` (`tree.rs:1111`): the insert's charges, plus
    `16 x list(c_p)` (`5 x list(c_p)` more for a `Document`) and `3 x list(c_o)`.
    Exceptions: `dom.html:4691-4755`; the order is the tree's.
- `moveBefore` (3 rows: `Element`, `Document`, `DocumentFragment`): `3d + d_n + s + m` steps
  (the tree climbs for `root_of` twice on the parent and once on the node, and for the ancestor
  check: `tree.rs:1133`), the list steps of the insert, then `Tree::move_before`. It connects
  no node that was not connected, so it records no script. Counter: raises.
- Clone, import, adopt (3 rows).
  - `cloneNode(deep)`: a `Document` throws NotSupportedError before any charge (deviation 2; the
    standard allows it). Else an own walk of the subtree (`visit` before each node, giving
    `s`, `a`, `b`); then the tree's cost walk `s + a / 4`, the copy `b / 16`, `s` for the nodes
    made; then `charge_nodes(s', b)` with a `template` counted twice; then
    `Tree::clone_node` (`tree.rs:1202`). A clone at the node limit ends `limit: memory` first at
    the session's limits. The tree raises the counter once for each copied child (accepted,
    question 6): a kept collection is refilled, and the refill is charged.
  - `importNode(node, options)`: a `Document` throws NotSupportedError first (`dom.html:8572`);
    the depth is `options` as a boolean, or `not selfOnly`; then as a clone
    (`Tree::import_node`, `tree.rs:1264`).
  - `adoptNode(node)`: a `Document` throws NotSupportedError (8736); a node with a parent is
    removed: `3 x list(c_o)` and 1; then `Tree::adopt_node` (`tree.rs:1270`). A `template`'s
    contents fragment is a no-op there. Counter: raises when it removes.
  - Limits: Nodes and StoredBytes (the tree admits the whole clone before it makes a node; if
    `clone_node` does not, the Change makes it so). No Depth: a copy is a root.
- `normalize` (`Node`, `dom.html:6946-7016`): the glue's step; an own walk (`visit`, `s`) that
  finds the text bytes `B` and the longest run `R`; then before the call `3s + 3B / 16` steps
  and `R x 3 x list(c_max)` for the quadratic `detach` (`tree.rs:1309`); then the call. About
  2,580 adjacent text nodes end the script by this price (question 2). Limits: StoredBytes
  (merged bytes are stored again). Counter: raises when a text node is removed.
- Character data (`appendData`, `insertData`, `deleteData`, `replaceData`, `splitText`, and the
  setters of `data`, `nodeValue` and `textContent`).
  - Reads: the old length `L` in bytes (constant). Writes: `Tree::replace_data` (`tree.rs:1386`)
    and `split_text` (1364).
  - Order for the four writers: the glue's copy; `(L + data_len) / 16` steps (the UTF-16 length
    and the rebuild are O(length) a call); the IndexSize check against the UTF-16 length
    (`dom.html:11557`) before any memory charge; then `charge_stored(added, removed)`; then the
    call; on a refusal `undo_stored` and `end_with_limit`.
  - `splitText(offset)`: IndexSize first (11804); `charge_nodes(1, tail_bytes)`; then the call.
    Raises when the node has a parent.
  - The setter of `data` is `replaceData(0, length, value)`. The setter of `nodeValue` takes
    `Option<String>` (null is the empty string): on character data it is that, on others nothing.
    The setter of `textContent` (`dom.html:6850-6933`): on character data it replaces the data;
    on an element or fragment it is replace-all: `2c_p` steps, then `charge_nodes(1, len)` when
    the string is not empty (no node when it is), the call (`tree.rs:1441`); on others nothing.
    Counter: character-data writes do not raise; the replace-all raises when it removed a child
    or made a child.
  - Limits: StoredBytes for the writers, Nodes and StoredBytes for `splitText` and for the
    replace-all.
- `Document.title` setter. The HTML Standard's algorithm (section "document.title", not
  vendored here): with no document element, return; a document element in the SVG namespace
  takes its first `title` child; an HTML element takes the first `title` element in tree order;
  with none, an HTML document element makes a `title` under the first `head` and returns when
  there is none; then the text of that element is replaced by the value. It uses `find`
  (`reads.rs:349`), a stepwise walk, so this story needs `find`, `Reads` and `html_element`
  (`reads.rs:218`) to be `pub(super)`. Limits: Nodes, StoredBytes and Depth (a `max_depth` of 3
  puts `title` under `head` at the bound).

Which tree write raises the counter, summarized from `tree.rs @ 83e6b8a`: insert, replace, move,
adopt and remove raise when a child list changes; `normalize` when it removes a text node;
`split_text` when the node has a parent; `set_text_content` on an element or fragment when it had
children or the value is not empty; a deep clone once for each copied child. Character-data
writes and the `create_*` functions do not raise. A refused write does not raise.

Which limits each family reaches, and the test that reaches each (each test loops its members,
one page, one test):

| family | limit | test |
| --- | --- | --- |
| create (10 rows) | Nodes | `limit_nodes_create` |
| create (8 rows with a string) | StoredBytes | `limit_stored_create` |
| insert, variadic, `ChildNode`, `replaceChild` | Depth | `limit_depth_insert` |
| `moveBefore` (3 rows) | Depth | `limit_depth_move` |
| variadic and `insertAdjacentText` strings | Nodes | `limit_nodes_strings` |
| variadic and `insertAdjacentText` strings | StoredBytes | `limit_stored_strings` |
| `cloneNode`, `importNode` | Nodes | `limit_nodes_clone` |
| `cloneNode`, `importNode` | StoredBytes | `limit_stored_clone` |
| `splitText`, replace-all | Nodes | `limit_nodes_split` |
| `splitText`, replace-all | StoredBytes | `limit_stored_split` |
| four writers, `data` | StoredBytes | `limit_stored_character_data` |
| `normalize` | StoredBytes | `limit_stored_normalize` |
| `title` setter | Nodes | `limit_nodes_title` |
| `title` setter | StoredBytes | `limit_stored_title` |
| `title` setter | Depth | `limit_depth_title` |
| every family at default limits | memory | `limit_memory_default` |

Which members record a script and which do not:

- Record, for each connected HTML or SVG `script` element in the inserted subtree, in tree
  order, after a successful write: `insertBefore`, `appendChild`, `replaceChild`,
  `insertAdjacent*`, `prepend`, `append`, `replaceChildren`, `before`, `after`, `replaceWith`,
  and the `title` setter (it makes no script). The subtree walk's list is reused; the contents
  of a `template` are skipped, as they are not connected.
- Do not record: `moveBefore` (nothing becomes connected), `adoptNode`, the setters,
  `splitText`, `normalize`, `remove`, and every `create` row.
- `cloneNode` and `importNode` return disconnected copies and record nothing; a copy is
  recorded when it is inserted, which is a deviation from a browser, where a copy of a started
  script is not prepared again (deviation 3, question 4). Every connected copy is recorded once.

Deviations this story names:

1. `createElementNS` with a namespace outside the six, or with a prefix, throws
   NotSupportedError (the tree's element has no prefix and no open namespace;
   `story:html-open-namespace` is the story that can lift it).
2. `cloneNode` of a `Document` throws NotSupportedError (the standard clones it,
   `dom.html:7301-7312`). `importNode` and `adoptNode` of a `Document` throw it as the standard
   says (8572, 8736).
3. A clone of a `script` element is recorded when it is inserted, as above (`cloneNode` and
   `importNode`).

The price of `normalize`, of a removal from the front of a long list and of the `title` walk is
the tree's passes, charged up front: a price and not a deviation of behaviour.

## Change

- `crates/script/src/dom/mutation.rs`: the 54 methods of `Mutation`, one call each into the two
  files below. No tree access, no charge (decision 1).
- `crates/script/src/dom/mutation_reads.rs`: the reads (climb, subtree walk, list reads, the
  viable-sibling scan, the `title` search, the length reads), each charging before each node it
  reads (decision 2). `crates/script/src/dom/mutation_writes.rs`: the writes, the charges of
  decision 3 in the order of the tables, the error map and the `record_inserted` calls.
- `crates/script/src/dom/mod.rs`: two `mod` lines. `crates/script/src/dom/reads.rs`: `charge`,
  `find`, `Reads` and `html_element` made `pub(super)`; with the attributes story's three tokens
  the file gains five or six visibility edits and no other.
- `crates/html/src/tree.rs`: `parent_or_host` becomes `pub`; the passes of decision 3 are
  recounted and written as doc comments; a recursive function is made iterative (decision 7);
  an operation a member needs that is missing is added and named in the report; the clone
  admits before it makes a node.
- `crates/webidl/src/prelude.rs`, regenerating `crates/script/src/bindings.rs`: `charge_nodes`;
  one step for each item before the loop of `to_nodes_or_strings`; the refill charged before
  it copies (decision 6). `crates/webidl/selection.txt`: the `deviation:` clauses of the three
  rows of acceptance 20. `crates/script/src/runner.rs`: the comment at lines 714-718
  says no member of the slices `reading`, `attributes` and `mutation` reaches
  `Limit::InputBytes` or `Limit::Steps`; the arm stays.
- Tests: `crates/script/tests/dom_mutation.rs` (the cases), `crates/script/tests/
  dom_mutation_cost.rs` (new, with one `mod` line in `tests/main.rs`),
  `crates/script/tests/dom_reading.rs` (one line, decision 9), `crates/script/tests/
  bindings.rs` (three tests, decision 9).
- `docs/design/scripts.md`: Q1's row (line 86) gains the clone sentence; the lines of the node
  charge (322-332) and of the `memory_bytes` list (902-904), which the attributes story edits
  too, gain the removal and clone rules; the row of steps (318) says a script's writes are
  priced in the engine's steps. `AGENTS.md`: the entry of `crates/script` names the slice and
  its files. In the last commit: the public surfaces of acceptance 21.

## Acceptance

Every result is read from a named test or a command. Where a line holds on `main` already, it
says it is a guard.

1. Start check, not a result: `git grep -n -F 'units / 64' -- crates/script/src/bindings.rs`
   prints nothing (the reading story's price commit is merged first). It prints a line on `main`
   now and nothing at the implementor's base; if it prints a line, stop and report.
2. `cases_equal_rows`: a test asserts that the set of case names of `dom_mutation.rs` equals
   the 50 rows of `selection.txt` with the slice `mutation` plus the 4 rows with `setter:
   mutation`: 54 names, and each exception case is named by the exception it throws.
3. `the_files_hold_their_parts`: a source test over `include_str!` asserts that `mutation.rs`
   holds none of `tree()`, `tree_mut()`, `charge(`, `charge_steps`, `charge_stored`,
   `undo_stored`, `charge_nodes`; `mutation_reads.rs` holds neither `tree_mut()` nor
   `charge_stored`; `mutation_writes.rs` holds no `tree()` and none of the attribute functions
   (so `Limit::Attributes` is unreachable). Red on `main`: the files do not exist.
4. `no_write_reaches_input_bytes_or_steps`: a source test over `crates/html/src/tree.rs` shows
   `Limit::InputBytes` and `Limit::Steps` occur nowhere in it; and `git diff` of `runner.rs`
   shows the comment lines 714-718 changed and the arm at 719 unchanged. A guard for the first
   half, red for the second.
5. `title_written`: `surf_script::run::parse_with_scripts` over
   `<title>Before</title><script>document.title = "Written by script"</script>` answers a tree
   whose title text is `Written by script`, and its list of ends holds one end, `Ran`.
6. `append_keeps_the_order`: a script that appends an element to `getElementById("a")`, with
   markup after its end tag, leaves the appended node and the later markup in the tree, in the
   standard's order.
7. `limit_*` tests, one for each row of the table in "What each member charges" (16 tests):
   each ends the script with the limit's word, also inside `try { } catch { }`, parsing goes on
   and the rest of the page is in the tree, and the tree after is as before: serialization,
   node count and `writes()` equal. `limit_memory_default` runs at the default parser limits
   under a memory limit of 4 MiB and ends `limit: memory`, not a parser limit, by
   `var s = Array(100001).join("x"); for (;;) document.documentElement.append(s);`.
8. `memory_before_nodes_at_defaults`: a loop that creates and removes an element ends
   `limit: memory` at the session's limits after at most 164,482 nodes (408 bytes each; the
   wrapper's 423 bytes make it about 80,756), and a removed node's charge is not refunded
   (`totals().nodes` and `totals().bytes` after one create-and-remove equal those after one
   create).
9. `charge_nodes_is_bulk`: through the real slice, `append` of 1,000 strings changes
   `totals().nodes` by 1,000 and `totals().bytes` by `1,000 x 408 + 2 x B`, and `charge_node(s)`
   equals `charge_nodes(1, s)` (a glue unit test). Red on `main`: no `charge_nodes`.
10. `variadic_strings_are_charged_before_any_node`: `append` of 200,000 empty strings ends
    `limit: memory` with `totals().nodes` 0, because `charge_nodes(200,000, 0)` is 81,600,000
    bytes, over the 67,108,864 of `memory_bytes`. Red on `main`: the nodes are charged and made
    one by one, and `totals().nodes` is above 0. The charge of one step for each item has its
    floor in the cost module (acceptance 17).
11. `the_counter_per_write`: for each row of the counter table in decision 6's list (the tree
    functions of "What each member charges"), one script makes the write and the test reads
    `Tree::writes()` through its tree handle before and after, and asserts raises or not as
    the table says; a refused write does not raise (a guard on `main`).
12. `a_kept_list_follows_a_write`: a kept `childNodes` (the same object) and a kept
    `getElementsByTagName` result are read before and after `appendChild`, `removeChild`,
    `textContent =` and `normalize`: the `childNodes` object is the same and shows the change
    after the write; the snapshot shows it only on a fresh call (reading decisions 17 and 18).
13. `a_refill_charges_before_it_copies`: with the spy `reading` slice, a kept `childNodes` of
    100,000 children (parsed, not appended) and a `steps_per_entry` of 1,000,000: the first read
    fills it (800,000 steps), an `appendChild` and a second read ask for
    `8 x (100,000 + 100,001)` steps, the refill ends the script `limit: steps`, and the spy
    counted no call of `node_child_nodes` during that refill. Red on `main`: the spy counts
    one call.
14. `the_records_of_inserted_scripts`: the page made of
    `<script>var s = document.createElement("script"); s.textContent = "1";`, then
    `document.documentElement.appendChild(s);</script>`, then `<script>1</script>`
    records `ran`, `not run: inserted`, `ran` in order and the element is in the tree, not run;
    a `cloneNode(true)` of it inserted twice records two; a `moveBefore` records none; a script
    inside a `template` inserted records none; the parser's own scripts get no second record
    (through `parse_with_scripts`).
15. `every_exception_in_its_order`: one case for each pair of exceptions in decision 13 where a
    call can throw two (for example `insertBefore` of an element into its own descendant, with
    a `child` that is not a child of the target: HierarchyRequestError before NotFoundError),
    asserted by name and message; and `createElementNS` with an invalid prefix and a bad
    namespace gives InvalidCharacterError, as the standard orders them.
16. `no_recursion`: a chain of 20,000 elements under a raised `max_depth`, on a test thread of
    256 KiB of stack, runs `cloneNode(true)`, `normalize`, `textContent =`, `removeChild` of the
    chain's root and a valid insert without a stack overflow.
17. `dom_mutation_cost.rs` (new): worst shapes, each case one script and no case over 5 s, with
    a floor of steps asserted and never a time; the lines print the steps, the time and the time
    a step against the plain walk (`getElementById('none')` over 20,000 bare elements), as the
    reading cost module does. The cases and their floors:
    - a deep clone of 10,000 elements under `max_nodes` 25,000: at least `3 x 10,000` steps, and
      a second clone ends `limit: nodes`;
    - a clone of an element of 1,024 attributes of values of 1,000 bytes: at least
      `2 x 256 + 1,024 x 1,000 / 16` steps;
    - the ancestor walk at the depth limit, in a loop of 100: an insert and a removal under a
      chain of 511, at least `3 x 511` steps each;
    - a fragment of 20,000 children inserted: at least `3 x 20,000` steps;
    - `append` and `replaceChildren` of 10,000 strings and 10,000 nodes: at least 20,000 steps
      for the items and `m` for the nodes;
    - `normalize` over 2,000 adjacent text nodes: at least `R x 3 x list(c)` steps; and over one
      text of 4 MiB: at least `3 x 4 MiB / 16`;
    - `textContent =` a string of 1 MiB on a node of 20,000 children: at least
      `2 x 20,000 + 1 MiB / 16`;
    - `remove` in a loop of 2,000 from the front (the total at least `0.75 x 2,000^2`) and from
      the back (linear), and 5,000 from the front ends `limit: steps`;
    - a kept collection refilled after each of 1,000 appends: at least `8 x (old + new)` steps
      for each refill.
    The implementor removes each charge of `mutation_writes.rs` in turn and the report names the
    case that went red, one per charge.
18. `other_slices_still_throw` is green with its one changed line, `git diff` of
    `dom_reading.rs` is that line only, and the three tests of `tests/bindings.rs` of decision 9
    are green with a member that still throws.
19. `slot task webidl-check` and `ESS_TOOLCHAIN=0.97.4 slot task webidl-slow` exit 0 on the
    regenerated `bindings.rs`; the times of `webidl-slow` before and after are in the report.
20. `selection_names_the_deviations`: a test over `selection.txt` asserts that the rows
    `Document createElementNS`, `Node cloneNode` and `Document importNode` each carry a
    `deviation:` clause for the deviations of "Deviations this story names" (1, then 2 and 3,
    then 3). Red on `main`: `createElementNS` holds only the wrapper deviation, the other two
    none.
21. Public surfaces: in the last commit, after merging `main`, the story changes every public
    surface its change makes false, each number from the `slot task check` log of its tip.
    The strings are those of `main` at `a14e3b0` and are read again at the start with
    `git grep -n -e 'passed' -e 'test files' -- README.md website/docs docs/badges`: `2394
    passed` (`README.md:11`, `docs/badges/make.sh:38`, `docs/badges/tests.svg:1`), `2,394 passed
    and 3 ignored: 1,842 passed` (`website/docs/status.md:31`), `1845 tests: 1842 passed`
    (`README.md:626`, `website/docs/how-it-is-built.md:70`), `230 test files` (`README.md:612`,
    `website/docs/how-it-is-built.md:56`). After the commit `git grep -n -F -e '<old>' --
    README.md website/docs docs/badges` prints nothing for each old string that changed.
22. The folder's bounds hold:
    - no test runs over 60 s and the test step ends within 120 s; a test over 5 s is named in
      the report with its time;
    - a build or test job in the pool ends within 10 minutes or names `slot --timeout`
      (at most 30 minutes), and is not started again unchanged;
    - a script that measures or waits starts without the `slot` prefix;
    - no Python in anything that stays;
    - no model call and no network request in a test;
    - an expensive product is made once for a run: under nextest, cases that share a page are
      one test, and no case builds a document more than once.
23. `slot task check` exits 0, and `git diff --stat` names only the files of the Change.

## Out of scope

Attributes and selectors (their slices); the parse that runs scripts and its per-document limits
(`story:scripts-parse-run`); a session (`story:scripts-document-port`); an open namespace for an
element or an attribute (`story:html-open-namespace`); ranges, tree walkers and node iterators
(`throws`); `createRange`, the `Document` constructor and the `EventTarget` constructor; a step
budget inside the tree's write functions; a refund of a removed node's charge; a live
collection (the snapshot deviation of the reading story stands).

## Placement

After `story:html-script-hook`, `story:scripts-dom-reading` (its price commit first,
acceptance 1), `story:scripts-parse-run` and `story:scripts-dom-attributes` (a `depends_on`
edge: that story writes `charge_stored`, `undo_stored`, `live` and `peak`, and this story calls
them as they are, so it is built on a base that holds them); beside
`story:scripts-dom-selectors`. The code of the slice is its own three files; the files below
are shared, and for each the rule is that the story that merges later merges `main` first and
takes the shared lines as they are.

- `crates/script/src/bindings.rs` (generated) and `crates/webidl/src/prelude.rs`:
  `story:scripts-dom-attributes` (it adds `charge_stored`, `undo_stored`, `live`, `peak` and
  the `strings_value` change; on `main` before this story starts), `story:scripts-parse-run`
  (`DomState::swap_tree`). This story adds `charge_nodes`, the item charge and the refill
  charge, and no second `charge_stored`. `slot task webidl-check` and `slot task webidl-slow`
  hold it.
- `crates/webidl/selection.txt`: `story:scripts-dom-attributes` (its rows); the rows of this
  story are different lines.
- `crates/html/src/tree.rs`: `story:scripts-dom-attributes` (six reads and
  `Attribute::stored_bytes`), `story:scripts-dom-sibling-positions` (a draft that may change
  child-list writers; this story's charges stay safe over a faster list), the reading story
  (the write counter; it merges first).
- `crates/script/src/dom/reads.rs`: `story:scripts-dom-attributes` (`charge`,
  `ATTRIBUTES_A_STEP`, `attribute_count`); this story adds `find`, `Reads`, `html_element` and
  shares `charge`. One edit each, in different lines.
- `crates/script/src/dom/mod.rs`, `crates/script/tests/main.rs`: `story:scripts-dom-attributes`
  and `story:scripts-dom-selectors`, each two `mod` lines or one, in different places.
- `crates/script/tests/dom_reading.rs`: `story:scripts-dom-attributes` edits one line of
  `other_slices_still_throw` (line 2425, `getAttribute`); this story edits line 2424. Adjacent
  lines: the second merger resolves by keeping both edits.
- `crates/script/tests/bindings.rs`: `story:scripts-dom-attributes` (the `Writes` variant);
  this story changes three tests' members and adds no variant.
- `crates/script/src/runner.rs`: `story:scripts-dom-attributes` does not edit it (its writes
  reach neither limit); this story edits the comment only.
- `docs/design/scripts.md:322-332` and `:902-904`: `story:scripts-dom-attributes` (the bytes of
  a node that exists); this story adds the removal and clone sentences. Also `AGENTS.md` and the
  public surfaces, shared with `story:scripts-dom-attributes`, `story:scripts-dom-selectors` and
  `story:scripts-parse-run`: the later merger takes its numbers from its own log.
- Drafts that touch the same files, none a dependency: `story:scripts-utf16-length` (it touches
  `string_length` in the glue; it does not move the argument charge) and
  `story:scripts-dom-collection-names`.
- Other plan files change with this story: the epic (a block of corrections),
  `story:html-open-namespace` (decision 3), `story:scripts-dom-reading` (the open item of
  decision 20) and `story:scripts-dom-attributes` (its line on `Limit::InputBytes`).

## Why `epic:scripts-in-a-page`

Writes to the tree, without which no script changes what surf shows (example 1 of the epic). The
document port's example appends an element built by a script, and the hostile-pages story builds
a deep tree by `appendChild` and expects `limit: depth`; both need the writes, each priced so
that a hostile loop of them ends inside the step limit.
