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
  path: crates/webidl/src/generate.rs
- confidence: cited
  path: crates/webidl/src/prelude.rs
- confidence: cited
  path: docs/design/scripts.md
revision: 46
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:13Z", actor: "human:operator", revision: 15}
---
## Context

The epic's ninth candidate (lines 472-475) and its slice "Tree mutation, 29" (lines 736-741),
read at plan commit `929269e`: the mutating members of `Node`, `Element`, `Document` and of the
interfaces they return, with the DOM Standard's exceptions, held to the parser's limits.

Read for this text:

- The plan at `929269e` (branch `plan/round-2`): the epic, the attributes story at its sixth
  revision (`9e5b717`: decision 8 and Placement), the parse-run story. The selectors story at
  `d5a7c45` (Placement).
- The code at `83e6b8a`, which is also the tip of the reading unit's branch
  (`impl/scripts-dom-reading`): its fix round is not committed. `main` at `a14e3b0` for the
  public strings and the vendored DOM Standard (`dom.html`).
- Numbers below that are arithmetic from the formulas are marked "arithmetic"; no build has
  measured them. The build measures each one and reports a difference as a defect of the
  formula or of the code, fixed in the same change.

What `crates/webidl/selection.txt` holds for this slice at `83e6b8a` (263 lines):

- 50 rows with the slice `mutation`: 46 members (`Node` 6, `Element` 10, `Document` 13,
  `CharacterData` 8, `Text` 1, `DocumentType` 4, `DocumentFragment` 4) and 4 constructors (`Text`
  line 196, `Comment` 201, `ProcessingInstruction` 203, `DocumentFragment` 221).
- 4 rows of `reading` with `setter: mutation`: `Node nodeValue` (line 60), `Node textContent`
  (61), `Document title` (180), `CharacterData data` (182).
- So the trait `Mutation` has 54 methods (`crates/script/src/bindings.rs:3267 @ 83e6b8a`).
- `Document body` and `Document head` are not rows (`document.body` is not a member at this
  slice). `Document documentElement` is a `reading` row (line 146), `Node firstChild` and
  `lastChild` are (56, 57), `Document getElementById` is (165). A case reaches `head` through
  `document.documentElement.firstChild`, `body` through `document.documentElement.lastChild`,
  and any other element through `getElementById`. Where a table below says "`body`" it means
  that element.
- `Document createRange` is a row of `throws` (line 162, reason ranges); `Document constructor`
  (line 136, reason documents made by script) and `EventTarget constructor` (line 29, reason
  events) throw too. Their messages are `"{member} is not supported: {reason}"` and
  `"{name} constructor is not supported: {reason}"` (`bindings.rs:2045`, `:2036 @ 83e6b8a`).
- Rows 150 and 151 (`createElement`, `createElementNS`) carry the deviation "an element's wrapper
  is an Element". Rows 63 (`Node cloneNode`) and 157 (`Document importNode`) carry no note.

What the glue (`crates/webidl/src/prelude.rs`, generating `bindings.rs`) does for every member
at `83e6b8a`: one step, the brand check, the copy of each `String` argument, the slice's method,
then `finish`. A method answers `DomResult`; `DomEnd::Limit` ends the script with a parser limit
(`bindings.rs:120`). `end_with_limit` (`bindings.rs:512`) turns a `Limit` into the end. The node
charge `charge_node` (`prelude.rs:219`, `bindings.rs:284`) takes 3 x `NODE_BYTES` (136, so 408
bytes) plus twice the stored bytes through `charge_host_bytes`, pushes one `HostBytes` into
`DomState::charges` (`prelude.rs:112`) for each call, counts one node in `totals.nodes` and never
refunds. A variadic argument arrives as `Vec<NodeOrString>` (`bindings.rs:1116`); the conversion
`to_nodes_or_strings` (`bindings.rs:1135`, `prelude.rs:1036`) charges no step for an item.
`Copies::copy` (`bindings.rs:620`; the division by 64 is at `:628`) charges `units / 64` after
the copy; the reading story's price commit (16 bytes a step, before the copy,
`STRING_BYTES_A_STEP`, `bindings.rs:1257`) is not built at `83e6b8a`.

What the tree's write surface does at `83e6b8a` (`crates/html/src/tree.rs`): it counts no step.
Each write walks, and the walks are not constant. Decision 3 lists every pass with its function
and line. The facts the rest rests on:

- Every tree walk uses its own stack (`pending`); no function of the write surface recurses.
- A removed node stays in the arena with its strings counted (`docs/design/scripts.md:193`).
- `clone_node` gives `NotSupported` for a `Document` (`tree.rs:1203`) and admits the whole clone
  before it makes a node (`:1206`); `clone_cost` (`:1175`) is private.
- The only places that make a `Limit` are `tree.rs:733` (Nodes), `:736` (StoredBytes), `:865`
  (Depth) and `:1517` (Attributes). `Limit::Attributes` is reached only through `put_attribute`,
  which no mutating member calls. `Limit::InputBytes` and `Limit::Steps` are made nowhere in
  `tree.rs`.
- Defaults (`crates/html/src/limits.rs`): `max_depth` 512, `max_nodes` 1,048,576,
  `max_attributes` 1,024, `max_stored_bytes` 67,108,864.

Memory comes before nodes. The engine's `memory_bytes` is 67,108,864; a node charges 408 bytes
(so at most 164,482 nodes alone; arithmetic: 164,482 x 408 = 67,108,656), and a wrapper with its
node data takes 423 bytes (`docs/design/scripts.md:329-332 @ 83e6b8a`, measured on the reading
slice), so 831 bytes a created node and 80,756 nodes. At the default limits a script that makes
nodes ends `limit: memory` before `limit: nodes`. A case that wants `limit: nodes` lowers
`max_nodes` in its own `ParseOptions` and `Document.limits`.

The script's end words are those of the runner: a step overrun is `limit: steps-per-script`
(`Reason::StepsPerScript`, `crates/script/src/runner.rs:310`, made at `:915`), the engine's
memory end is `limit: memory`, and the step limit is `steps_per_entry` 10,000,000 (`:41`).
`runner.rs:715-719 @ 83e6b8a` maps `Limit::InputBytes | Limit::Steps` to
`ScriptEnd::Limit(Reason::WallClock)` under a comment of four lines (715-718; line 714 is the
`StoredBytes` arm). No member of this slice makes either; this story shows that with a source
test and edits that comment only.

Found by the reviews of the stories before this one, for this story to hold with a test:

- A write over a limit leaves the tree as it was, and the script's end is the limit's word.
- A reader cannot be made to copy more than it paid for: the refill of a kept collection copies
  the old list and the child list before it charges (reading story, decision 20, the "Open"
  item). The generated getter does it: it calls `reads::children` (`reads.rs:882-884`, the
  `.to_vec()` at 883) before `refresh_collection` (`prelude.rs:1321`) clones the old list. The
  mutation slice is what makes a write between two reads possible, so this story closes it.
- A step must buy a bounded amount of work; a loop of writes must end within the step limit
  (`steps_per_entry`) or finish well inside it.
- A cost module records no time assertion and holds floors of steps that break when a charge is
  removed.
- The tree does not count its passes, so a formula and the tree can drift apart with every row
  still green. Decision 4 ties them.

A `NodeId` belongs to one tree; the bindings never hand a node of one document to another
document's tree.

## Decisions taken

1. Three files and a source test. `crates/script/src/dom/mutation.rs` holds one call per member
   and nothing else: no `tree()`, no `tree_mut()`, no charge. `mutation_reads.rs` holds the
   reads, `mutation_writes.rs` the writes, the charges and the error map. `reads.rs` changes
   only by visibility. A walk is a small struct with its own stack of (parent, next index)
   (`Subtree`, `Climb`, `Siblings`), advanced by `next(&mut self, tree: &Tree)`; one function
   `step` charges one step and then calls `next`, so a node is charged before it is read. A walk
   copies no child list: it reads `Tree::children(parent)[index]`. Each function does constant
   work, or charges before its loop. `SurfMutation` stays a unit struct: all surf documents are
   HTML documents, so it needs no state.
2. Prices. One step per 16 bytes, an argument's bytes included (the glue's copy, once the
   reading story's price commit is in); `b16(n)` is `ceil(n / 16)`. A step per node made, moved,
   removed, cloned or visited. A step per 4 attributes for a scan (`ATTRIBUTES_A_STEP`,
   `reads.rs:252`), and a step per attribute record that a clone copies. 8 steps per collection
   node in the glue (`COLLECTION_NODE_STEPS`, `bindings.rs:1251`). Ids of a child list: 2 ids
   of the 8-byte `NodeId` a step, `ids(n) = ceil(n / 2)` (the cost module prints
   `size_of::<NodeId>()`). Three list passes, each priced in `ids`:
   - `front(i) = ids(i + 1)`: the scan from the front to index `i` (`index_in_parent`,
     `tree.rs:871`);
   - `back(k) = ids(k)`: the scan from the back over `k` entries (the `rposition` of `remove`,
     `tree.rs:528`); a child at index `i` of `c` is `k = c - i` entries from the back;
   - `shift(r) = ids(r)`: `r` ids moved by `Vec::remove` (`tree.rs:532`) or by the `splice` of
     `insert_checked` (`tree.rs:1021`). The `shrink` after a removal (`tree.rs:710`, called at
     `:893`) and the growth of a list are not charged: a list is reallocated only when it
     passes twice its length or has no room, so a reallocation is paid by the writes between
     two of them.
   A charge pays a pass once, whoever makes it: the slice's own walks are charged as they go,
   the tree's passes by formula before the call. So a removal at distance `k` from the back
   costs `rm(k) = 2 x ids(k) + ids(k - 1)`: the slice's own walk that finds the position, the
   tree's scan and the shift. That is `3k / 2` for even `k`; the last child of a list costs 2
   (`k` = 1) and the first of `c` about `1.5 c`.
   `d` is the nodes on the path from a node up to the root, itself included (what a climb of
   `parent_or_host`, `tree.rs:773`, visits); `d_n` the same for the moved node.
   Each charge comes before the work it pays for. A climb takes one step before each read;
   `reads.rs::ancestors` collects the chain before it charges, so this slice does not use it.
3. The passes. The tree's write functions at `83e6b8a`, with the end each scan starts from. The
   tables of "What each member charges" are sums of these lines.
   - `remove` (524), reached by `detach` (887): `back(c - i)` at 528, `shift(c - i - 1)` at 532,
     the `shrink` at 893 (not charged).
   - `insert_checked` (987): for a node of the target parent, `front(j)` of the node for
     `next_sibling` at 988 (the test that a node is already where it would go); for a
     fragment, a pass over its `m` children at 994-996 and another at 1015-1020; for a node with
     a parent, `detach` (1000: the `remove` line above); for a given reference child at index
     `i` (after the node left), `front(i)` at 1011; `shift(c_p - i)` at 1021. With no reference
     child the index is the length (1013) and the splice moves nothing: an append of a
     parentless node costs no list step.
   - `insertion_allowed` (951): the climb of the parent for the ancestor test at 962 (`d`); for
     a `Document` parent `document_allows` (900): up to 5 passes over the children, each child
     dereferenced (909, 913, 917, 919 and 922, 930 and 932), so `5 c_p` steps.
   - `admit_depth` (848): `element_height` (828) walks the moved subtree, template contents
     included (840), `s` node visits; when the height is above 0, `element_depth` (811) climbs
     the parent (`d`). A fragment walks each child's subtree.
   - `insert_before` (1079): `insertion_allowed`, then `next_sibling(node)` (front of the node)
     when `child` is the node (1086-1088), then `admit_depth`, then `insert_checked`.
   - `replace_child` (1111): `insertion_allowed` (1117); `admit_depth` (1118);
     `next_sibling(child)` at 1119, `front(i)` of the child; when that sibling is the node,
     `next_sibling(node)` at 1121, `front(j)`; `detach(child)` at 1124, `rm`-without-own-walk,
     `back(c_p - i) + shift(c_p - i - 1)`; then `insert_checked` with the reference.
     For a parentless node: `front(i)` + `front(i)` + `back(c_p - i)` + `shift(c_p - i - 1)` +
     `front(i)` + `shift(c_p - i - 1)`, the first `front` being the slice's own walk. For a
     node of the same parent the pass at 988 and a `detach` of the node (1000) come on top:
     `front(j)` + `back` + `shift` of the node's own index, 11 passes in all.
   - `move_before` (1133): `root_of` (798) of the parent twice and of the node once (1143,
     1144), the ancestor test (1145) and `element_depth` inside `admit_depth` (1169): four
     climbs of the parent and one of the node, `4 d + d_n`; for an element into a `Document`,
     `document_allows` (1155); `next_sibling(node)` at 1164 when `child` is the node;
     `admit_depth` (`s`); `insert_checked`.
   - `clone_node` (1202): `clone_cost` (1175) visits each node once (`s'` nodes, a template's
     contents fragment counted as a node, 1187) and reads its attributes' sizes; the copy loop
     (1211) makes `s'` nodes and clones each node's data, one allocation for each attribute
     record and string; a shallow clone of a template counts its contents as a node and does not
     walk it.
   - `normalize` (1309): `descendants` (635) collects the subtree: `s` visits (1311); a filter
     over it (1313): `s` reads; `text_runs` (1282) reads each parent's children (1318): at most
     `s` reads in all; the first non-empty text of a run (1319) and the sum of the others'
     lengths (1321-1324): one pass each; the merge loop (1343): one more. That is 6 passes of at
     most `s` nodes each. The merge copies the merged bytes twice (`merged`, then `push_str`,
     1333-1340) and `push_str` may reallocate the first text: at most 3 x `B` bytes for `B` the
     text bytes of the subtree. Each removed text node is a `detach` (1345): `rm`-without-own-
     walk, `back + shift`, in the list of its parent at that time. `descendants` does not enter
     a template's contents.
   - `split_text` (1364): `byte_index` (677) to the offset, at most `L` bytes (1368); the two
     `to_string` copies of the tail and the head (1371, 1372), `L` bytes in all; then, for a
     node with a parent, `next_sibling(node)` (1378, `front(j)`), and `insert_checked` of the
     new node with that sibling as reference: `front(j + 1)` at 1011 and `shift` at 1021.
   - `replace_data` (1386): `utf16_len` of the data (1394, a pass over `L`), `byte_index` twice
     (1398, 1399, each up to `L`), then the build of the new data (1401-1404, one copy of at
     most `L` plus the new data). So 3 scans and 1 copy of `L`, plus the new data once. `set_data`
     (1416) and `append_data` (1422) call `utf16_len` first: one scan more.
   - `set_text_content` (1441): for an element or fragment, `create_text_node` copies the value
     (1447); an any-element test over the children (1450-1453): at most `c_p` reads;
     `remove_children` (539): `c_p` visits; `insert_checked` of the text with no reference.
     For character data it is `set_data`.
   - `adopt_node` (1270): `detach` for a node with a parent; nothing for a template's contents.
   - `create_*` (1027-1074): each string copied once; a `template` makes two nodes (1027).
   The doc comment of each charge in `mutation_writes.rs` lists these passes for its member; a
   count in the tree that is higher than the table's is a defect of the table.
4. The work counter, which ties the formulas to the tree. The tree counts the work of its writes
   in one `u64`, in the pattern of `Writes` (`tree.rs:286`: not part of what a tree holds, two
   trees compare equal whatever it is). It is an atomic with relaxed ordering so that a read
   function, which holds `&self`, can raise it and `Tree` stays `Sync`; raised once a pass by
   the length of the pass in steps at the prices of decision 2 (`ids(n)` for a pass over `n`
   ids, `b16(n)` for `n` bytes, 1 for a node visited, made or removed, 1 for an attribute
   record copied), saturating, and read by one public function, `Tree::work()`. There is no
   cargo feature: a feature gives the host's compile cache a second copy of the crate. The
   counter is in steps-equivalents so that "one `u64`" and "divided by the price" are one
   number. The ties:
   - For each write function on its worst shape a cost row asserts that the steps the slice
     charged for the call are at least `Tree::work()` raised by the call (acceptance 18).
   - A clone row asserts that `s'` and `b` of the slice equal the tree's change in nodes
     (`Tree::node_count()`) and stored bytes (`Tree::stored_bytes()`), read through
     `totals()`.
   - A tree edit that adds a pass and raises the counter for it breaks a row; a pass added
     without the counter is not caught by any row. Every edit of a write function of
     `tree.rs` raises the counter for each pass it adds; the doc comment of `Tree::work()` says
     so, and the sibling-positions story, which may change a list writer, keeps it.
5. Memory.
   - `DomState` holds one `HostBytes` for the nodes scripts made, `nodes: Option<HostBytes>`.
     `charge_nodes(count, stored_bytes)` (a new bulk form in the glue) wants
     `3 x NODE_BYTES x count + 2 x stored_bytes` more; it grows the handle through the engine's
     `Cx::resize_host_bytes` (a grow is charged first and ends the script with `limit: memory`
     when it does not fit) and pushes nothing into `DomState::charges`. A call with count 0
     and bytes 0 does nothing. It counts `count` in `totals.nodes`. Nodes are never refunded,
     so the handle only grows. `charge_node(s)` stays as a one-line wrapper over
     `charge_nodes(1, s)`: `adversary_3.rs:204`, `adversary_4.rs:218` and
     `tests/bindings.rs:176` call it and are not in scope.
   - The attributes story (decision 8 at `929269e`) keeps a second handle for the stored bytes
     of nodes that exist and resizes it to twice the live bytes: `DomCx::charge_stored(added,
     removed) -> DomResult<()>` before a write, `DomCx::settle_stored(added, removed)` after
     it (`settle_stored(0, 0)` after a write the tree refused), the accessors `stored_live()`,
     `stored_charged()`, `stored_handles()` and `node_charges()`. The private function that
     calls the engine and maps its answer (the body that story makes `charge_node` and
     `charge_stored` share) is the one place that owns "resize the handle, or end with
     `limit: memory`"; `charge_nodes` calls it and does not write a second. This story calls
     `charge_stored` and `settle_stored` for the bytes a write adds to a node that exists
     (the character-data writers, `normalize`) and adds no second pair. A removal refunds
     nothing for nodes (their 408 bytes and strings stay charged, because the node stays in the
     arena); stored bytes of a character-data write that shrinks are refunded by that story's
     settle.
   - A loop that makes and removes nodes ends `limit: memory`, at the session's limits, before
     `limit: nodes`.
   - The charge for a node is made before the node: a loop of `createTextNode` is charged by
     its count, not by an item. The temporary fragment of a variadic convert is a node
     (`create_document_fragment`, `tree.rs:1071`): for two items or more the convert charges
     `charge_nodes(k + 1, B)` with `k` the strings and `B` their bytes.
   - `B` comes from the lengths of the raw strings and is charged before the first copy of an
     item (a UTF-8 byte is at most 3 per UTF-16 unit, so `B` is taken as 3 x the units; for
     ASCII text that is an overcharge of 3, and it is allowed). The glue charges `n` steps in
     `to_nodes_or_strings`, before its loop, and the memory charge in
     `finish_nodes_or_strings` before the copy loop. A member that then does nothing (a
     `before()` with no parent) has paid: an allowance, named in decision 13.
   - Memory that the engine's budget does not see, and its bound:
     - the tree's transient vectors, an entry each and bounded by the node limit: `element_height`
       16 bytes, `clone_node` 24, `descendants` 8, `text_runs` 24 a run and 8 an id (sizes from
       the types; arithmetic: 1,048,576 x 16 = 16,777,216, x 24 = 25,165,824);
     - `replace_data` holds the old and the new text at once, so `2 L`;
     - a removed node's arena slot and strings, bounded by 164,482 script-made nodes and the
       parser's own limits;
     - the single argument copies of the glue (`Copies::copy`): bounded by the string length
       limit, not charged before the copy; no story holds the move yet.
6. A write over a limit ends through `end_with_limit` with the limit's word (`limit: nodes`,
   `limit: depth`, `limit: stored-bytes`, `limit: memory`), also inside `try { } catch { }`.
   The tree after it:
   - for a refused call that wrote nothing before the refusal, the tree equals the tree just
     before the call: serialization, node count and `writes()`;
   - for a variadic or `ChildNode` member with two items or more, the tree is as after the
     convert (the standard's own order for its exceptions): the items have left their old
     parents and sit in the temporary fragment, which is a node. The test says so: it runs the
     same script with the convert made by hand (`f = document.createDocumentFragment();
     f.append(a, b)`) and compares.
   A test sees "the tree just before the call" by a second parse of the same page that runs the
   same script without the refused call, and compares the serializations, the node counts and
   `writes()`. A limit ends the script, so a loop over members does not reach the second
   member: each member has its own parse inside one `#[test]`. `Limit::Attributes` is
   unreachable here: a source test shows `mutation_writes.rs` calls none of `put_attribute`,
   `set_attribute`, `set_attribute_ns`, `remove_attribute`, `remove_attribute_ns`,
   `toggle_attribute`.
7. The write counter and the refill. `Tree::writes()` (`tree.rs:331`) raises when a write
   changes a child list. The table below says, for each write function, whether it raises. A
   kept `childNodes` (the same object refilled) and a kept `getElementsByTagName` snapshot are
   read before and after a write by a test per row of the table. Reading decisions 17 and 18
   stand: a snapshot shows a change on a fresh call only; a `[SameObject]` list is refilled
   when the counter moved. The open item of the reading story's decision 20 is fixed in the
   generated getter, before the slice runs:
   - The getter template (`same_object_getter`, `crates/webidl/src/generate.rs:695-735`; the
     generated getter for `childNodes` is `bindings.rs:4061-4092`) reads two lengths in
     constant time: the tree's child count of `this` (`Tree::children(this).len()`, an upper
     bound of what the slice answers for `childNodes`, `children` and the document and fragment
     lists) and the kept object's list length (a new read next to `collection_written`,
     `prelude.rs:814`). It charges `8 x (kept_len + child_count)` (`COLLECTION_NODE_STEPS`) when
     an object is kept and `8 x child_count` when none is, before `node_child_nodes` copies the
     list (`reads.rs:882-884`) and before `refresh_collection` clones the old list
     (`prelude.rs:1321`).
   - `refresh_collection` and `collection_object` keep their names and their own charge for
     their other callers; each gets an inner body without the charge (`refresh_collection_inner`,
     `collection_object_inner`), which the getter calls after its own charge.
   - A spy `reading` slice with three methods (`window_document`, `node_child_nodes`,
     `document_get_element_by_id`) proves the order by counting calls of `node_child_nodes`
     when the charge ends the script.
8. No recursion, tested where the code is. No walk of the write surface recurses at `83e6b8a`
   (every one keeps a `pending` stack), and the walks of this slice are structs of decision 1:
   a source test shows no function of the three files or of the walks calls itself. The
   tree's write functions and the slice's walks run directly on a thread of 256 KiB of stack,
   over a chain at a raised depth limit, with a guard that a trivial call runs on that thread
   (acceptance 17). This says nothing about the runner's own thread (`runner.rs:55`, a fixed
   margin of 1 MiB, `:469-473`).
9. A `script` element a script inserts is recorded by `crate::run::record_inserted(cx, node)` of
   `story:scripts-parse-run` and not run. The path of each member is in the list below. The
   call keeps each node at most once for a document (that story's set, its rule): a node the
   parser reported gets no second record, an element moved out and back in gets none the
   second time, and a clone is another node, recorded when it is inserted.
10. `other_slices_still_throw` (`crates/script/tests/dom_reading.rs:2399-2432 @ 83e6b8a`) holds
    three adjacent lines, 2424 (`document.appendChild(el)`, this story's), 2425
    (`getAttribute`, the attributes story's) and 2426 (`querySelector`, the selectors story's).
    This story changes line 2424 to `document.createRange()` with the message
    `Document.createRange is not supported: ranges`, and no other line; the later merger keeps
    all three edits. The reading module keeps its own `TestMutation` (`dom_reading.rs:73`,
    installed at `:130`), as do `dom_reading_cost.rs` and the `adversary_dom_reading*` modules,
    which replace the mutation slice. The three tests of `crates/script/tests/bindings.rs` that
    assert `createComment`, `appendChild`, `new Text` or `new DocumentFragment` throw
    (`a_member_of_a_slice_throws_until_its_slice`,
    `the_strings_of_a_dom_exception_outlive_a_collection`,
    `an_uncaught_dom_exception_names_its_name_and_message`) switch to members that still throw:
    `document.createRange()`, `new Document()` (message `Document constructor is not supported:
    documents made by script`) or `new EventTarget()` (`EventTarget constructor is not
    supported: events`).
11. A named deviation goes into the epic and beside its row in `selection.txt`, as a
    `deviation:` clause (the three rows are printed in "Deviations this story names"). A member
    the epic excludes stays `throws`: `createRange`, the `Document` constructor, the
    `EventTarget` constructor, `createTreeWalker`, `createNodeIterator`.
12. A test's time bound is at least ten times its own time, or there is none
    (`AGENTS.md:1966 @ a14e3b0`). A case that checks a loop ends inside the step limit asserts
    the end word, not a time. `crates/script/src/bindings.rs` is generated and is never merged
    by hand: the second of the stories that change it takes `main`, regenerates it, and
    `slot task webidl-check` and `ESS_TOOLCHAIN=0.97.4 slot task webidl-slow` hold it (as the
    attributes story's Placement says).
13. The exceptions come in the standard's order, and the slice leaves the order to the tree where
    the tree has it: ensure pre-insertion validity (`dom.html:4061-4183 @ a14e3b0`) checks the
    parent kind (4069), the ancestor (4074), NotFound (4079), the node kind (4084), a doctype
    under a non-document (4095), then the Document-parent checks (4104-4183); `moveBefore`
    (4502-4685) checks root (4508), ancestor (4517), NotFound (4522), kind (4527), Text in a
    document (4532), element in a document (4537). The slice maps `WriteError` to the
    standard's `DOMException`: `HierarchyRequest`, `NotFound`, `IndexSize`, `InvalidNodeType`,
    `Namespace`, `NotSupported` keep their names; `Limit` ends the script.
    The order of check and charge: a check that needs no walk (the type of the parent, the
    type of the node, a null argument, the `where` of `insertAdjacent*`, the parent of the
    child in `removeChild`) runs before any charge; a check that needs a walk (the ancestor
    test, the `Document` checks, the `IndexSize` test over a text) runs after the charge of its
    walk, inside the tree. So a script can end `limit: steps-per-script` where a browser would
    throw `HierarchyRequestError`, `NotFoundError` or `IndexSizeError`: that is a named
    allowance, not a deviation of behaviour, and acceptance 16 has a pair for each of five
    families (an ancestor test, a `NotFound`, a `Document` parent, an `IndexSize` and an
    `InvalidCharacterError` or `NamespaceError` of a create).
14. The pre-charge rule. The tree counts no step, so the slice charges by formula before the
    call, from lengths it reads in constant time or walks step by step. The formulas are upper
    bounds of the tree's passes, except where a table says "exactly". The tree is not given a
    step budget (question 1); decision 4 holds the formulas to it. The `Document title`
    setter waits for the reading unit's fix round: its `find` (`reads.rs:349-366 @ 83e6b8a`)
    copies a child list at `reads.rs:363` (and a sibling walk at `:914`), which breaks "a walk
    copies no child list" (decision 1). When the reading branch holds a `find` that copies none
    (acceptance 1 prints it), the setter calls it; until then the setter's work is not started.
    The price gate of acceptance 1 and this wait are one merge condition.

### What each member charges and does

Symbols: `d` and `d_n` of decision 2; `s` the nodes of the subtree being moved or copied,
template contents included, `s'` the nodes a clone makes (a template counts as 2), `a` the
attribute records and `b` the stored bytes of the clone's subtree; `m` the nodes put into a child
list (1, or the children of a fragment); `c_p` the children of the target parent, `c_o` those of
the node's old parent, `i` the index of the reference child, `j` the index of the node in its
parent; `n` the items of a variadic list, `k` its strings, `B` their bytes; `L` the bytes of a
text. The glue's one step comes first for every member; its copy of a `String` argument is
priced by the glue (`b16` of the argument).

Own walks, which every table below includes as "own": `d` for a climb of the target (and `d_n`
of the node for `moveBefore`), `s` for the subtree walk that lists the `script` elements and
counts `s`, `front(i)` or `back(k)` for a position it must know.

- Create (11 rows): `createElement`, `createElementNS`, `createTextNode`, `createComment`,
  `createDocumentFragment`, `createProcessingInstruction`, `createCDATASection`, and the four
  constructors.
  - Reads: none. Writes: `Tree::create_*` (`tree.rs:1027-1074`), no parent.
  - Charge, in order: the check as one pass over the copied argument, `b16(len)` for each
    validated string; for `createElement` the ASCII lowercase, another `b16(len)`; then
    `charge_nodes(1, stored)` (2 for a `template`, `tree.rs:1027`) and `1 + b16(stored)` steps
    (the node, and the tree's own copy of each stored string); then the tree call.
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
  - Writes: `Tree::insert_before` (`tree.rs:1079`).
  - Charge before the call, over the own walks: `2d` (the two climbs at 962 and 861) + `s` (the
    height walk) + `2m` (the two passes over the moved nodes) and these list terms:
    - a reference child at index `i`: the slice's own `front(i)` to find it, then `front(i)`
      at 1011 and `shift(c_p - i)` at 1021;
    - the node already a child of the target (or `child` is the node): `front(j)` for 988 and
      1087;
    - the node has a parent: `back(c_o - j)` for the slice's own walk that finds `j`, then the
      tree's `back(c_o - j) + shift(c_o - j - 1)`;
    - the target is a `Document`: `5 c_p` (`document_allows`, a step each child each pass).
    An append of a parentless node to a non-document parent adds no list step. In all, an
    insert is `1 + 3d + 2s + 2m` plus the list terms (the glue's step, own `d` and `s`).
  - `insertAdjacentElement(where, element)` and `insertAdjacentText(where, data)`: `where` is
    matched ASCII case-insensitively against `beforebegin`, `afterbegin`, `beforeend`,
    `afterend` (`dom.html:11179-11221`), SyntaxError otherwise, before any charge and before
    any node is made (`insertAdjacentText` makes its text node after the check,
    `charge_nodes(1, len)`). `beforebegin` and `afterend` answer null (and write nothing) when
    `this` has no parent. `afterend` finds `this`'s index by the own `front(j)` and inserts
    before the next sibling (`i = j + 1`); `afterbegin` inserts before the first child
    (`i = 0`); `beforeend` appends.
  - Exceptions: those of decision 13. Limits: Depth; Nodes and StoredBytes for the text node of
    `insertAdjacentText`. Counter: raises when the child list changes.
- Variadic `ParentNode` (9 rows): `prepend`, `append`, `replaceChildren` on `Element`,
  `Document`, `DocumentFragment`.
  - Order: the glue charges `n` steps (one for each item) before its loop (a change to
    `to_nodes_or_strings`, `prelude.rs`); then, in `finish_nodes_or_strings`, before the copy
    loop, `charge_nodes(k + 1, B)` for two items or more (`charge_nodes(k, B)` for one) and `k`
    steps for the text nodes made and 1 for the fragment; then convert (`dom.html:5016-5041`):
    no items does nothing, one item is used as it is, more make a fragment and insert each
    item into it; then the insert of the result (`prepend` 5163, `append` 5179).
  - Each item put into the fragment pays `5 + 2 s_i` steps (the fragment's own `d = 1`, the
    item's `s_i`, `2d = 2`, `s_i`, `2m = 2`), plus `rm` of the item's old parent when it has
    one. The final insert of the fragment is an insert as above with `s` the fragment and its
    `m` children.
  - `replaceChildren` (5194): the convert, then `2d + s` for the validity check before any
    removal (the new public check of the Change, `check_insertion`, which runs the ancestor
    test, the `Document` checks and the depth check), then `2 c_p` for the removal (the
    any-element scan and `remove_children`, a step each child each pass), then the insert. A
    refused check leaves the children (`dom.html:4760-4795`).
  - Exceptions, limits and counter: as an insert; `replaceChildren` also raises when it
    removes. Limits: Depth, Nodes and StoredBytes (the text nodes and the fragment).
- `ChildNode` (12 rows): `before`, `after`, `replaceWith`, `remove` on `Element`,
  `CharacterData`, `DocumentType`.
  - `before`, `after`, `replaceWith` (5335, 5370, 5399): a `this` with no parent does nothing
    and charges nothing beyond the glue's step and its `n` item steps (the allowance of
    decision 5). Otherwise the viable sibling is computed before the conversion: the own
    `front(j)` for `this`'s index, `n` steps for the membership set of the items, and one step
    for each sibling examined on the walk away from `this` (at most the siblings that are in
    the set, plus the first that is not); then the conversion as above; then the pre-insert
    with the reference at its index, as an insert. `replaceWith` adds the removal of `this`
    (`rm` of its own index). `replaceWith()` with an empty list is a removal.
  - `remove` (5436): no parent, nothing. Else the removal below.
- Remove and replace (`Node removeChild`, `Node replaceChild`, and the removals above).
  - `removeChild(child)`: read `parent(child)` (constant); if it is not `this`, NotFoundError
    (`dom.html:4810`), before any charge. Else `rm(c_p - i)` steps (the own back walk, the
    tree's scan, the shift) and 1 for the node, then `Tree::remove_node` (`tree.rs:1104`).
    Counter: raises.
  - A removal of the last child costs `rm(1)` = 2 steps, with the glue's step and the node's 4;
    a removal of the first of `c` children costs about `1.5 c`. So a loop that removes from the
    front is quadratic: for `c` from 1 to `n` the sum of `rm(c) + 2` is about `0.75 n^2`
    (arithmetic: 3,006,000 for `n` = 2,000 and 18,765,000 for `n` = 5,000), and ends
    `limit: steps-per-script` at about 3,650 removals (arithmetic), while the same loop from
    the back is linear (4 steps each: 8,000 for 2,000). That is the price; the cost module
    holds both.
  - `replaceChild(node, child)` (`tree.rs:1111`): the own `d`, `s` and `front(i)`; then
    `2d + s + 2m`, the passes of decision 3 (for a parentless node: `front(i)` for 1119,
    `back(c_p - i) + shift(c_p - i - 1)` for the removal of the child, `front(i)` for 1011,
    `shift(c_p - i - 1)` for 1021; for a node of the same parent also `front(j)` for 988 and
    1121 and `back + shift` of the node), 1 for the child removed, and `5 c_p` for a
    `Document`. Exceptions: `dom.html:4691-4755`; the order is the tree's.
- `moveBefore` (3 rows: `Element`, `Document`, `DocumentFragment`): own `d`, `d_n`, `s`,
  `front(i)` and, since the node always has a parent, `back(c_o - j)`; then `4d + d_n + s + 2m`,
  the list terms of an insert, and `5 c_p` for an element into a `Document`; then
  `Tree::move_before` (`tree.rs:1133`). It connects no node that was not connected, so it records
  no script. Counter: raises.
- Clone, import, adopt (3 rows).
  - `cloneNode(deep)`: a `Document` throws NotSupportedError before any charge (deviation 2; the
    standard allows it). Else an own walk of the subtree, `1 + ceil(attributes / 4)` steps
    before each node, giving `s`, `a`, `b` (it descends a template's contents when deep, as
    `clone_cost` does); then `2 s' + 2 a + b16(b)` steps (the cost walk and the copy loop, one
    step each node and each attribute record on each, and the copy of the bytes); then
    `charge_nodes(s', b)`; then `Tree::clone_node` (`tree.rs:1202`). A clone at the node limit
    ends `limit: memory` first at the session's limits. The tree raises the counter once for
    each copied child (accepted, question 4): a kept collection is refilled, and the refill is
    charged.
  - `importNode(node, options)`: a `Document` throws NotSupportedError first (`dom.html:8572`);
    the depth is `options` as a boolean, or `not selfOnly`; then as a clone
    (`Tree::import_node`, `tree.rs:1264`).
  - `adoptNode(node)`: a `Document` throws NotSupportedError (8736); a node with a parent is
    removed: `rm(c_o - j)` and 1; a template's contents fragment is a no-op (`tree.rs:1273`);
    then `Tree::adopt_node` (`tree.rs:1270`). Counter: raises when it removes.
  - Limits: Nodes and StoredBytes. The clone admits before it makes a node already
    (`tree.rs:1206`): a guard. No Depth: a copy is a root.
- `normalize` (`Node`, `dom.html:6946-7016`): the glue's step; an own walk (`s` steps, a step
  before each node) over the subtree (not into template contents) that finds, for each parent
  `p`, the children `c_p`, the text nodes the tree will remove `removed_p` (a run's texts but
  the first non-empty one, or all of them when each is empty), and the text bytes `B`. Then,
  before the call: `6 s` (the six passes of decision 3), `3 x b16(B)`, and the sum over the
  parents of `removed_p x c_p` (an upper bound of the `detach` of each removed text:
  `rm(k) <= k <= c_p`); and the merged bytes `added` through `charge_stored(added, 0)`, with
  `settle_stored(0, 0)` when the tree refuses. Limits: StoredBytes (merged bytes are stored
  again). Counter: raises when a text node is removed. The sum, not the longest run, is what
  bounds the work: many parents of long runs cost the sum.
- Character data (`appendData`, `insertData`, `deleteData`, `replaceData`, `splitText`, and the
  setters of `data`, `nodeValue` and `textContent`). The slice makes no pass of its own over
  the text: the tree's `IndexSize` test is the one that reads it, so every pass is charged before
  the call, and the call answers the exception.
  - Reads: the text's byte length `L` (constant, `Tree::data`).
  - `replaceData`, `insertData`, `deleteData`: `4 x b16(L) + b16(data_len)` steps (the 3 scans
    and 1 copy of decision 3, and the new data once); `appendData` and the `data` and
    `nodeValue` setters: `5 x b16(L) + b16(data_len)`. Then `charge_stored(added, removed)`;
    the call; on a refusal `settle_stored(0, 0)` and `end_with_limit`; else
    `settle_stored(added, removed)`.
  - `splitText(offset)`: `2 x b16(L)` steps (the scan and the two copies), then
    `charge_nodes(1, tail_bytes)` and 1 step; for a node with a parent the own `front(j)` and
    `front(j) + front(j + 1) + shift(c_p - j - 1)` (1378, 1011, 1021); then the call.
    `IndexSizeError` is the tree's (an offset past the length). Raises when the node has a
    parent.
  - The setter of `data` is `replaceData(0, length, value)`. The setter of `nodeValue` takes
    `Option<String>` (null is the empty string): on character data it is that, on others
    nothing. The setter of `textContent` (`dom.html:6850-6933`): on character data it replaces
    the data; on an element or fragment it is replace-all: `2 c_p + b16(len)` steps, then
    `charge_nodes(1, len)` and 1 step when the string is not empty (no node when it is), the
    call (`tree.rs:1441`); on others nothing. Counter: character-data writes do not raise; the
    replace-all raises when it removed a child or made a child.
  - Limits: StoredBytes for the writers, Nodes and StoredBytes for `splitText` and for the
    replace-all.
- `Document.title` setter. The HTML Standard's algorithm (section "document.title", not
  vendored here): with no document element, return; a document element in the SVG namespace
  takes its first `title` child; an HTML element takes the first `title` element in tree order;
  with none, an HTML document element makes a `title` under the first `head` and returns when
  there is none; then the text of that element is replaced by the value. It calls the reading
  unit's `find` (`reads.rs:349`), a stepwise walk that charges a step before each node and
  copies no child list once the reading fix is in (decision 14), so `find`, `Reads` and
  `html_element` (`reads.rs:218`) are made `pub(super)`. Charge: the walk, then for a new
  `title` the create (`charge_nodes(1, 5)`, 1 step) and an insert under `head` as above, then
  the replace-all of `2 c_t + b16(len)` and its text node. Limits: Nodes and StoredBytes.
  `Limit::Depth` is unreachable here: a depth counts elements (`element_depth`, `tree.rs:811`),
  a text node adds none, and a new `title` under `head` is at depth 3 (`html`, `head`,
  `title`), which is the smallest depth at which a script runs (a `script` in `head` or in
  `body`), so a parse that ran the script admits it. Order of the text node and the element in
  one script: the element first.

Which tree write raises the counter, summarized from `tree.rs @ 83e6b8a`: insert, replace, move,
adopt and remove raise when a child list changes; `normalize` when it removes a text node;
`split_text` when the node has a parent; `set_text_content` on an element or fragment when it had
children or the value is not empty; a deep clone once for each copied child. Character-data
writes and the `create_*` functions do not raise. A refused write does not raise. The table the
test reads, one line for each row (member; tree function; raises when):

- `insertBefore`, `appendChild`, `insertAdjacent*`, `prepend`, `append`, `before`, `after`;
  `insert_before`; a node moves.
- `replaceChild`, `replaceWith`; `replace_child`; always.
- `moveBefore`; `move_before`; a node moves.
- `removeChild`, `remove`; `remove_node`; the node had a parent.
- `replaceChildren`; `remove_children` and `insert_before`; a child leaves or enters.
- `adoptNode`; `adopt_node`; the node had a parent.
- `cloneNode`, `importNode`; `clone_node`; the clone has a child.
- `normalize`; `normalize`; a text node leaves.
- `splitText`; `split_text`; the node has a parent.
- `textContent` of an element or fragment; `set_text_content`; a child left or one came.
- `data`, `nodeValue`, `appendData`, `insertData`, `deleteData`, `replaceData`;
  `replace_data`; never.
- the creates, the constructors, the text of `title`; `create_*`, `set_text_content`; never,
  except as above.

Which limits each family reaches, and the test that reaches each (each test has one parse for
each member of the family, inside one `#[test]`):

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
| every family at default limits | memory | `limit_memory_default` |

Which members record a script and which do not:

- Record, for each connected HTML or SVG `script` element in the inserted subtree, in tree
  order, after a successful write: `insertBefore`, `appendChild`, `replaceChild`,
  `insertAdjacent*`, `prepend`, `append`, `replaceChildren`, `before`, `after`, `replaceWith`,
  and the `title` setter (it makes no script). The subtree walk's list is reused; the walk that
  lists scripts skips the contents of a `template` (they are not connected), where the walk
  that gives `s` enters them (the tree's passes do).
- Do not record: `moveBefore` (nothing becomes connected), `adoptNode`, the setters,
  `splitText`, `normalize`, `remove`, and every `create` row.
- `cloneNode` and `importNode` return disconnected copies and record nothing; a copy is
  recorded when it is inserted, which is a deviation from a browser, where a copy of a started
  script is not prepared again (deviation 3, question 3). Every connected copy is recorded once.

Deviations this story names, and the row each one is printed on in `selection.txt` (the whole
line, asserted byte for byte by acceptance 21):

1. `createElementNS` with a namespace outside the six, or with a prefix, throws
   NotSupportedError (the tree's element has no prefix and no open namespace;
   `story:html-open-namespace` is the story that can lift it).
2. `cloneNode` of a `Document` throws NotSupportedError (the standard clones it,
   `dom.html:7301-7312`). `importNode` and `adoptNode` of a `Document` throw it as the standard
   says (8572, 8736).
3. A clone of a `script` element is recorded when it is inserted, as above (`cloneNode` and
   `importNode`).

Each block below is one line of the file: its lines are joined by one space, and the blocks
are three lines.

```
Document createElementNS mutation deviation: an element's wrapper is an Element, and a
prefix or a namespace outside the six known throws NotSupportedError
```

```
Node cloneNode mutation deviation: a Document throws NotSupportedError, and a clone of a
script element is recorded when it is inserted
```

```
Document importNode mutation deviation: a clone of a script element is recorded when it
is inserted
```

The price of `normalize`, of a removal from the front of a long list and of the `title` walk is
the tree's passes, charged up front: a price and not a deviation of behaviour.

## Change

- `crates/script/src/dom/mutation.rs`: the 54 methods of `Mutation`, one call each into the two
  files below. No tree access, no charge (decision 1).
- `crates/script/src/dom/mutation_reads.rs`: the walk structs and `step`, the length reads, the
  viable-sibling scan, the `title` search call. `crates/script/src/dom/mutation_writes.rs`: the
  writes, the charges of decision 3 in the order of the tables, the error map and the
  `record_inserted` calls.
- `crates/script/src/dom/mod.rs`: two `mod` lines. `crates/script/src/dom/reads.rs`: `charge`,
  `find`, `Reads` and `html_element` made `pub(super)`: visibility edits only; the same tokens
  that the attributes and selectors stories make `pub(super)` (`charge`, `Reads`), so the later
  merger keeps one.
- `crates/html/src/tree.rs`, each addition with its cost:
  - `parent_or_host` becomes `pub`: constant;
  - `check_insertion(node, parent, child, replaced)`, the public validity-only check: the
    ancestor test, the `Document` checks and the depth check of `insertion_allowed` and
    `admit_depth` with no write, `2d + s` and `5 c_p` for a `Document`;
  - `remove_all(parent)`, the bulk removal of `remove_children` without a `Cell`, for
    `replaceChildren`: `c_p` visits, and the any-element scan before it, `c_p` reads;
  - `work()` and the work counter of decision 4, raised in each pass of the table of
    decision 3; the doc comment of each write function lists its passes;
  - no recursive function is changed: none recurses (decision 8).
  Rules that two places hold: the clone's cost (`clone_cost`, private, owns it; the slice's walk
  copies it on purpose because it charges before each node, and the clone row asserts they
  agree), a template as two nodes (`create_element_ns` owns it; the slice's
  `charge_nodes(2, ...)` copies it on purpose, held by the `createElement("template")` row),
  the `IndexSizeError` bound (the tree owns it; the slice only charges the passes) and the depth
  (`admit_depth` owns it; the slice only prices the climbs).
- `crates/webidl/src/prelude.rs`, `crates/webidl/src/generate.rs`, regenerating
  `crates/script/src/bindings.rs`: `charge_nodes` and the `nodes` handle (`DomState`
  fields and `DomState::new`, `prelude.rs:104-131`); `charge_node` as its wrapper; `n` steps in
  `to_nodes_or_strings` and the memory charge before the copy loop of
  `finish_nodes_or_strings`; the refill charged in the getter template before the slice call
  and the inner bodies of `refresh_collection` and `collection_object` (decision 7).
- `crates/webidl/selection.txt`: the three `deviation:` rows printed above (lines 63, 151, 157).
- `crates/script/src/runner.rs`: the comment at the arm of `Limit::InputBytes | Limit::Steps`
  (the four lines that begin "The limits of the parser's input and of its steps") says no
  member of the slices `reading`, `attributes` and `mutation` reaches either; the arm stays.
- Tests: `crates/script/tests/dom_mutation.rs` (the cases and the source tests),
  `crates/script/tests/dom_mutation_cost.rs` (new, with one `mod` line in `tests/main.rs`),
  `crates/script/tests/dom_reading.rs` (one line, decision 10), `crates/script/tests/
  bindings.rs` (three tests, decision 10; the glue unit test of acceptance 9).
- `docs/design/scripts.md`: the row of Q1 that holds the node charge (cite it by its text, not
  its line) gains the clone sentence; the paragraphs of the node charge (`:322-332 @ 83e6b8a`)
  and of the `memory_bytes` list (`:902-904`), which the attributes story edits too, gain the
  removal and clone rules (prose written twice, merged by hand, the later merger keeping both
  sentences); the row `parser: steps` (`:318`) says a script's writes are priced in the
  engine's steps. `AGENTS.md`: the entry of `crates/script` names the slice and its files.
- The public numbers, in the last commit, after merging `main`, from the gate log of the tip:
  `README.md` (lines 11, 285, 612, 626, 661), `website/docs/how-it-is-built.md` (56, 70, 105),
  `website/docs/status.md` (31), `website/docs/what-works.md` (17), `docs/badges/make.sh` (38)
  and `docs/badges/tests.svg` (1, 2, 10; written by `sh docs/badges/make.sh .` after the edit
  of `make.sh`). These files are changed in the last commit, after merging `main`, from one gate
  log, and are not in the recorded scope by the epic's rule (the epic's correction that begins
  "Each story changes the public numbers it makes false", lines 955-961 at `929269e`): two units
  built at once both change them, and the one merged second repeats its numbers commit. They
  are files of the Change all the same.

## Acceptance

Every result is read from a named test or a command. Where a line holds on `main` already, it
says it is a guard. Every command that reads the diff names its base:
`git diff $(git merge-base main HEAD)..HEAD`.

1. Two start checks, not results.
   - The price gate: `git grep -n -F 'units / 64' -- crates/script/src/bindings.rs` prints
     `crates/script/src/bindings.rs:628` at `83e6b8a` and must print nothing at the
     implementor's base (the reading story's price commit is merged first). If it prints a
     line, stop and report.
   - The `title` gate: `git grep -n -F 'pending.extend(tree.children'
     -- crates/script/src/dom/reads.rs` prints `reads.rs:363` and `:914` at `83e6b8a`; the
     implementor reads `find` and the other walks (`:935`, `:1169` hold other child-list
     copies) after the reading unit's fix round and starts the `title` setter when no walk
     copies a child list.
2. `cases_equal_rows`: a case is a table entry of row name, page, script and expected end or
   tree, in `crates/script/tests/dom_mutation.rs`, that the test runs; an entry with no
   expectation is refused by the test. The test asserts that the set of row names of the table
   equals the 50 rows of `selection.txt` with the slice `mutation` plus the 4 rows with
   `setter: mutation` (read from the file): 54 names. Every row has at least one entry that
   asserts behaviour (a serialization, a value, or a thrown name and message), including the
   members whose only checks were a name or a limit: the `nodeValue` setter (null becomes the
   empty string), `appendData`, `insertData`, `deleteData`, `replaceData`,
   `createCDATASection` (NotSupportedError), `createDocumentFragment`, `createComment`,
   `createTextNode`, the constructors `Text`, `Comment`, `DocumentFragment`,
   `ProcessingInstruction`, `adoptNode` (the node has no parent after, same identity),
   `importNode` (a copy, not the same node), `insertAdjacentElement`, `insertAdjacentText`,
   `before`, `after`, `replaceWith` and `remove` of `DocumentType`, `Element.remove`,
   `CharacterData.remove`, `prepend`, `append` and `replaceChildren` of `Document` and
   `DocumentFragment`, and `replaceChild`. Each exception case is named by the exception it
   throws. A case that shares a page with another is the same table entry list over one parse.
3. `the_files_hold_their_parts`: a source test over `include_str!` asserts that `mutation.rs`
   holds none of `tree()`, `tree_mut()`, `charge(`, `charge_steps`, `charge_stored`,
   `settle_stored`, `charge_nodes`, `charge_node`; `mutation_reads.rs` holds none of
   `tree_mut()`, `charge_stored`, `settle_stored`, `charge_nodes`, `charge_node`;
   `mutation_writes.rs` holds no `tree()` and none of `put_attribute`, `set_attribute`,
   `set_attribute_ns`, `remove_attribute`, `remove_attribute_ns`, `toggle_attribute` (so
   `Limit::Attributes` is unreachable); no function of the three files calls itself; and no
   walk of `mutation_reads.rs` holds `.to_vec()` or a `children(...)` call that is collected.
   Red on `main`: the files do not exist.
4. `no_write_reaches_input_bytes_or_steps`: a source test over `crates/html/src/tree.rs` shows
   `Limit::InputBytes` and `Limit::Steps` occur nowhere in it (a guard, true at `83e6b8a`);
   and `git diff $(git merge-base main HEAD)..HEAD -- crates/script/src/runner.rs` shows the
   four comment lines above the arm that begins "The limits of the parser's input and of its
   steps" changed, and the arm `Limit::InputBytes | Limit::Steps => ScriptEnd::Limit(
   Reason::WallClock)` unchanged (lines 715-718 and 719 at `83e6b8a`, 719 not in the diff).
   The second half is red on `main`: the comment is as it was.
5. `title_written`: `surf_script::run::parse_with_scripts` over
   `<title>Before</title><script>document.title = "Written by script"</script>` answers a tree
   whose title text is `Written by script`, and its list of ends holds one end, `Ran`.
6. `append_keeps_the_order`: the page
   `<body><div id="a"></div><script>document.getElementById("a").appendChild(
   document.createElement("p"))</script><i>after</i></body>` leaves the tree's `body` children
   in the order `div`, `script`, `i`; the only child of `div` is `p`; the text of `i` is
   `after`.
7. `limit_*` tests, one for each row of the table in "What each member charges" (15 tests):
   for each member of the family, one parse; the script's end is the limit's word, also inside
   `try { } catch { }`, parsing goes on and the rest of the page is in the tree, and the tree
   after is as decision 6 says: for each member the test also parses the page with the script
   without the refused call (with the convert made by hand for two items or more) and asserts
   equal serialization, node count and `writes()`. The node count the tree gained equals
   `totals().nodes` in every case that makes nodes. The `title` setter has no depth test, since
   it cannot reach `limit: depth` (the `Document.title` member says why); a guard test,
   `title_at_the_smallest_depth`, parses `<head><script>document.title = "x"</script>` at
   `max_depth` 3 and asserts the script ran to its end and the `title` is in the tree.
   `limit_memory_default` runs at the default parser limits under a memory limit of 4 MiB and
   ends `limit: memory`, not a parser limit, by
   `var s = Array(100001).join("x"); for (;;) document.documentElement.append(s);`.
8. `memory_before_nodes_at_defaults`: a loop that creates and removes an element ends
   `limit: memory` at the session's limits after at most 164,482 nodes (408 bytes each; the
   wrapper's 423 bytes make it about 80,756), and a removed node's charge is not refunded
   (`totals().nodes` and `totals().bytes` after one create-and-remove equal those after one
   create). The steps of a turn are about 20 by the tables (arithmetic: 164,482 x 20 =
   3,289,640, under 10,000,000); the build measures them, and if a turn costs more than 60
   steps the case raises `steps_per_entry` to 50,000,000 as the cost module does.
9. `charge_nodes_is_bulk`: through the real slice, `append` of 1,000 strings of 3 bytes changes
   `totals().nodes` by 1,001 (the strings and the fragment) and `totals().bytes` by
   `1,001 x 408 + 2 x 3,000` = 414,408 (arithmetic); `charge_node(s)` equals
   `charge_nodes(1, s)` (a glue unit test in `crates/script/tests/bindings.rs`); after 10,000
   calls of `createTextNode` `node_charges()` has not grown (the attributes story's accessor,
   which counts the node handles: 0 or 1). Red on `main`: no `charge_nodes`.
10. `variadic_strings_are_charged_before_any_node`: `append` of 200,000 empty strings ends
    `limit: memory` with `totals().nodes` 0 and the tree's node count unchanged, because
    `charge_nodes(200,001, 0)` is 81,600,408 bytes (arithmetic), over the 67,108,864 of
    `memory_bytes`. The implementor checks that the engine accepts that many arguments through a
    spread, and uses `Function.prototype.apply` when it does not. Red on `main` because
    `append` throws `not supported` (the slice is absent); the step for each item has its
    floor in the cost module (acceptance 18).
    `a_loop_of_two_item_appends_ends_in_memory`: `var a = document.createElement("i"), b =
    a.cloneNode(false); for (;;) document.documentElement.append(a, b);` at the default limits under
    `steps_per_entry` 50,000,000 ends `limit: memory`, not `limit: nodes`, and the node count
    the tree gained equals `totals().nodes`: the temporary fragment of each call is a node and
    is charged.
11. `the_counter_per_write`: for each line of the table "Which tree write raises the counter"
    (member; tree function; raises when), one script makes the write and the test reads
    `Tree::writes()` through its tree handle before and after, and asserts raises or not as the
    table says; a refused write does not raise (a guard on `main`).
12. `a_kept_list_follows_a_write`: a kept `childNodes` (the same object) and a kept
    `getElementsByTagName` result are read before and after `appendChild`, `removeChild`,
    `textContent =` and `normalize`: the `childNodes` object is the same and shows the change
    after the write; the snapshot shows it only on a fresh call (reading decisions 17 and 18).
13. `a_refill_charges_before_it_copies`: with the spy `reading` slice, a kept `childNodes` of
    100,000 children (parsed, not appended) and a `steps_per_entry` of 1,000,000: the first read
    fills it (`8 x 100,000` = 800,000 steps, charged before the copy), an `appendChild` and a
    second read ask for `8 x (100,000 + 100,001)` steps (arithmetic: 1,600,008), the refill ends
    the script `limit: steps-per-script`, and the spy counted no call of `node_child_nodes`
    during that refill. Red on `main`: the spy counts one call.
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
16. `an_exception_after_a_charge`: for each of five families, a pair of cases on one page:
    - with the default step limit the script throws the standard's exception: `a.appendChild(
      ancestor)` over a subtree of 100,000 nodes (HierarchyRequestError by the ancestor test);
      `insertBefore(x, notAChild)` with `x` a subtree of 100,000 nodes (NotFoundError);
      `document.appendChild(second)` on a `Document` of 5,001 children (HierarchyRequestError
      by the `Document` checks); `t.splitText(L + 1)` on a text of 4 MiB (IndexSizeError);
      `createElementNS("x", "a:b")` with a name of 1 MiB (the standard's error of the name);
    - with `steps_per_entry` lowered to one below the call's own charge (computed from the
      tables: the cost row of each gives the figure) the same script ends
      `limit: steps-per-script`, the named allowance of decision 13.
17. `no_recursion`: on a thread built with a stack of 256 KiB, named in the test, over a chain of
    20,000 elements under a raised `max_depth`: the tree functions `clone_node(root, true)`,
    `normalize`, `set_text_content`, `remove_node` of the chain's root and a valid
    `insert_before`, and the walks `Subtree`, `Climb` and `Siblings` of the slice over the
    chain, run without a stack overflow. A guard in the same test: a trivial call
    (`Tree::children(root)`) runs on that named thread, and a probe that recurses to half the
    stack (frame size and depth printed, arithmetic) completes on it, so the stack is the one
    asked for. No claim is made about the runner's thread.
18. `dom_mutation_cost.rs` (new): worst shapes, each row one script and no row over 5 s, with
    a floor and a ceiling of steps asserted and never a time. A row runs the script with the
    measured call and the same script without it (`Runner::steps()` before and after each,
    as `dom_reading_cost.rs:146` does) and asserts the difference. The floor is the sum of the
    charges of the tables for the row's shape (arithmetic below); the ceiling is
    `floor + floor / 4 + 200`, the room for the glue's steps that no table lists. The rows run
    under `steps_per_entry` 50,000,000 (as `dom_reading_cost.rs:23`) unless the row says it
    ends. A row whose call writes the tree also asserts that the steps charged are at least
    `Tree::work()` raised by the call. The lines print the steps, the time and the time a step
    against the plain walk, as the reading cost module does. One row for each charge, in the
    order of the tables (G is the glue's step; every floor is arithmetic):

    - create text: `createTextNode` of 1 MiB. Floor 1 + 65,536 + 65,536 + 1 = 131,074.
    - create element: `createElement` with a name of 1 MiB (the glue's copy, the check, the
      lowercase, the tree's copy). Floor 1 + 4 x 65,536 + 1 = 262,146.
    - depth: 100 turns of `appendChild` then `removeChild` of a fresh element under an element
      at depth 511 (`d` = 512). Floor 100 x (1,541 + 4) = 154,500.
    - fragment: a fragment of 20,000 elements appended to `body` (`d` = 3). Floor
      1 + 3 x 3 + 2 x 20,001 + 2 x 20,000 = 80,012.
    - variadic: `append` of 10,000 strings of 1 byte and 10,000 elements to `body`. Floor
      20,001 + 10,000 + 625 + 1 + 20,000 x 7 + 80,011 = 250,638.
    - insert, moved out: `appendChild` to a small parent (`d` = 3) of the first child of a
      parent of 20,000 children. Floor 1 + 3 + 1 + 6 + 1 + 2 + 3 x 10,000 = 30,014.
    - `Document` parent: `appendChild` of a comment to a `Document` of 5,001 children. Floor
      1 + 1 + 1 + 2 + 1 + 2 + 5 x 5,001 = 25,013.
    - `replaceChild`: a parentless node for the first of 2,000 children (`d` = 4). Floor
      3,003 + 18 = 3,021.
    - `moveBefore`: the last of 2,000 children to the front (`d` = 4, `d_n` = 5). Floor
      2,004 + 35 = 2,039.
    - `adoptNode`: the first of 20,000 children. Floor 3 x 10,000 + 2 = 30,002.
    - `before`: one fresh element before the middle of 80,000 children (`j` = 40,000,
      `d` = 4). Floor 60,022. `after` and `replaceWith` have the same shape and a row each.
    - `insertAdjacentElement`: `afterend` on the same node. Floor 60,019.
    - `replaceChildren`: one element for 20,000 children (`d` = 4). Floor
      1 + 1 + 4 + 1 + (8 + 1) + 40,000 + (8 + 1 + 2) = 40,027.
    - clone, deep: `cloneNode(true)` of an element of 9,999 children (`max_nodes` 25,000).
      Floor 1 + 10,000 + 2 x 10,000 + b16(b) = 30,001 + b16(b), with `b` read from the tree.
    - clone, attributes: an element of 1,024 attributes of 1,000-byte values. Floor
      1 + 257 + 2 + 2,048 + 64,000 = 66,308, at least (`b` is read from the tree).
    - insert, template: a `template` of 100,000 nodes (`s` = 100,002) appended to a `div` from
      its place in `body`, which holds three children (`d` = 4). Floor
      1 + 12 + 2 x 100,002 + 2 + 3 = 200,022.
    - `normalize`, one parent: 2,000 adjacent texts of 1 byte. Floor
      1,999 x 2,000 + 6 x 2,001 + 3 x 125 + 2,001 + 1 = 4,012,383.
    - `normalize`, many parents: 8 parents of 1,000 adjacent texts. Floor
      8 x 999 x 1,000 + 6 x 8,009 + 3 x 500 + 8,009 + 1 = 8,049,564.
    - `normalize`, one text: one text of 4 MiB. Floor 3 x 262,144 + 12 + 2 + 1 = 786,447.
    - `textContent =`: a string of 1 MiB on a node of 20,000 children. Floor
      1 + 65,536 + 65,536 + 2 x 20,000 + 1 = 171,074.
    - removal, front: `removeChild(p.firstChild)` 2,000 times. Floor 3,006,000.
    - removal, back: `removeChild(p.lastChild)` 2,000 times. Floor 8,000.
    - `splitText`, long: at `L - 1` of a text of 4 MiB, no parent. Floor
      1 + 2 x 262,144 + 1 = 524,290.
    - `splitText`, parent: a text of 4 bytes in the middle of 80,000 children. Floor 80,007.
    - `data =`: a text of 4 MiB set to 4 bytes (5 passes). Floor
      1 + 1 + 5 x 262,144 + 1 = 1,310,723. `nodeValue =` and `appendData` have the same shape
      and floor.
    - `replaceData` and `insertData` on that text (4 passes). Floor
      1 + 1 + 4 x 262,144 + 1 = 1,048,579.
    - `deleteData(0, 1)` on that text. Floor 1 + 4 x 262,144 = 1,048,577.
    - `title`: `document.title = "x"` with a `head` of 100,000 `meta` elements before a
      `title`. Floor 1 + 1 + 100,003 + 4 = 100,009.
    - refill: a kept `childNodes` read after each of 1,000 appends, from no children. Floor
      8 x 1,000^2 = 8,000,000.

    The removal rows: `p` has 2,000 children from the page; for the front the sum is
    `4,000 + sum of rm(c) for c = 1 to 2,000` and for the back `2,000 x 4`. Rows that end the
    script, under the default `steps_per_entry` of 10,000,000: 5,000 removals from the front
    (arithmetic: 18,765,000) end `limit: steps-per-script`; `normalize` over 20 parents of
    1,000 adjacent texts (19,980,000) ends `limit: steps-per-script`; and a second deep clone
    ends `limit: nodes`. The row `title` uses the shape of the reading story's wide parent and
    is not started until the gate of acceptance 1 is met. For each charge one red row:
    the implementor removes each charge of `mutation_writes.rs` by hand (the mutation tool is
    not on the host), runs the rows, and the report names for each the line changed, the row
    that went red, and shows `git diff --quiet` after putting the line back. A charge with no
    red row is a defect of the table and gets a row.
19. `other_slices_still_throw` is green with its one changed line, `git diff
    $(git merge-base main HEAD)..HEAD -- crates/script/tests/dom_reading.rs` is that line only,
    and the three tests of `tests/bindings.rs` of decision 10 are green with a member that still
    throws.
20. `slot task webidl-check` and `ESS_TOOLCHAIN=0.97.4 slot task webidl-slow` exit 0 on the
    regenerated `bindings.rs`; the times of `webidl-slow` before and after are in the report.
21. `selection_names_the_deviations`: a test over `selection.txt` asserts that the three lines
    printed in "Deviations this story names" are rows of the file, byte for byte. Red on `main`:
    `createElementNS` holds only the wrapper deviation, the other two no note. Each deviation has
    a behaviour entry in the table of acceptance 2: `createElementNS` with a prefix and with a
    namespace outside the six throws `NotSupportedError`; `cloneNode` of a `Document` throws
    `NotSupportedError`; `importNode` and `adoptNode` of a `Document` throw it. A named test,
    `element_names_at_the_name_boundary`, runs `createElement`, `createElementNS` and
    `createProcessingInstruction` on names at the edge of the XML `Name` production: the
    code points U+003A, U+005F, U+00B7 (not a start character), U+00C0, U+00D7 (excluded),
    U+0300 (not a start character), U+037E (excluded), U+2070, U+FFFE (excluded) and the first
    of the range U+10000-U+EFFFF, each as a first and as a later character, and asserts accept
    or InvalidCharacterError as the standard's production says.
22. Public surfaces: in the last commit, after merging `main`, the story changes every public
    surface its change makes false, each number from the `slot task check` log of its tip.
    The strings are those of `main` at `a14e3b0` (the same at `83e6b8a`) and are read again at
    the start with `git grep -n -e 'passed' -e 'test files' -e '2,397' -e '1845' --
    README.md website/docs docs/badges`:
    - `- **2,397 tests** (1,845, and 552 of the TLS crates in a step of their own): 2,394 pass,
      and 3` at `README.md:285` and `website/docs/what-works.md:17`;
    - `(1845 tests, 3 of them ignored, and 230 test files, where` at `README.md:661` and
      `website/docs/how-it-is-built.md:105`;
    - `2394 passed` at `README.md:11`, `docs/badges/make.sh:38` (`badge tests.svg "tests"
      "2394 passed" "$green"`) and `docs/badges/tests.svg:1`, `:2`, `:10`;
    - `| tests | 2,394 passed and 3 ignored: 1,842 passed` at `website/docs/status.md:31`;
    - `230 test files` at `README.md:612` and `website/docs/how-it-is-built.md:56`;
    - `1845 tests: 1842 passed` at `README.md:626` and `website/docs/how-it-is-built.md:70`.
    `docs/badges/tests.svg` is written by `sh docs/badges/make.sh .` after the edit of
    `make.sh:38`. After the commit `git grep -n -F -e '<old string>' -- README.md website/docs
    docs/badges` prints nothing for each old string whose number changed. These files are
    changed in the last commit, from one gate log, and are not in the recorded scope by the
    epic's rule.
23. The folder's bounds hold:
    - no test runs over 60 s and the test step ends within 120 s; a test over 5 s is named in
      the report with its time;
    - a build or test job in the pool ends within 10 minutes or names `slot --timeout`
      (at most 30 minutes), and is not started again unchanged;
    - a script that measures or waits starts without the `slot` prefix;
    - no Python in anything that stays;
    - no model call and no network request in a test;
    - an expensive product is made once for a run: under nextest, the entries of a case table
      that share a page are one test over one parse, and no case builds a document more than
      once.
24. `slot task check` exits 0, and `git diff --stat $(git merge-base main HEAD)..HEAD` names
    only the files the Change names (its scope paths and the public-number files of
    acceptance 22; the front's scope list does not hold the public-number files).

## Out of scope

Attributes and selectors (their slices); the parse that runs scripts and its per-document limits
(`story:scripts-parse-run`); a session (`story:scripts-document-port`); an open namespace for an
element or an attribute (`story:html-open-namespace`); ranges, tree walkers and node iterators
(`throws`); `createRange`, the `Document` constructor and the `EventTarget` constructor; a step
budget inside the tree's write functions; a refund of a removed node's charge; a live
collection (the snapshot deviation of the reading story stands); moving the glue's single
argument copy before the charge (`Copies::copy`; no story holds it yet).

## Placement

After `story:html-script-hook`, `story:scripts-dom-reading` (its price commit and its fix round
first, acceptance 1), `story:scripts-parse-run` and `story:scripts-dom-attributes` (a
`depends_on` edge: that story writes `charge_stored`, `settle_stored` and the accessors, and
this story calls them by the names it has at `929269e`, so it is built on a base that holds
them); beside `story:scripts-dom-selectors`. The code of the slice is its own three files; the
files below are shared, and for each the rule is that the story that merges later merges `main`
first and takes the shared lines as they are.

- `crates/script/src/bindings.rs` (generated) and `crates/webidl/src/prelude.rs`:
  `bindings.rs` is regenerated by the second story to merge; `prelude.rs` is source and is
  merged by hand in these regions: the `DomState` fields and `DomState::new` (`:104-131`:
  the attributes story adds `stored`, `live`, `charged`; `story:scripts-parse-run` adds
  `swap_tree`; this story adds `nodes`); `charge_node` (`:219`: the attributes story makes it
  and `charge_stored` share one private body; `charge_nodes` goes through that body);
  `to_nodes_or_strings` (`:1036`: the draft `story:scripts-utf16-length` changes
  `string_length` and the glue's count); `refresh_collection` and `refill` (`:1321`, `:1367`:
  the draft `story:scripts-dom-collection-names` rewrites them). The two drafts have no edge
  to this story. `slot task webidl-check` and `slot task webidl-slow` hold the generated file.
- `crates/webidl/src/generate.rs`: this story edits `same_object_getter`; the attributes story
  edits the generator's `strings_value`; different functions.
- `crates/webidl/selection.txt`: `story:scripts-dom-attributes` (the note of row 96),
  `story:scripts-dom-selectors` (nine rows); this story rows 63, 151 and 157. Rows 125, 174
  and 230 (`moveBefore`) are next to the selectors story's rows 126, 175 and 231 and are not
  edited by either.
- `crates/html/src/tree.rs`: `story:scripts-dom-attributes` (six reads and
  `Attribute::stored_bytes`), the reading story (the write counter; it merges first),
  `story:scripts-dom-sibling-positions` (a draft that may change child-list writers; this
  story's formulas and the work counter stay safe over a faster list, and that draft needs a
  `depends_on` edge to this story, whose acceptance 18 holds front removals quadratic where
  that draft's acceptance 2 holds them linear), `story:html-open-namespace` (a draft: changes
  `Namespace` and `create_element_ns`, and lifts deviation 1), `story:html-parser-hook-cost`
  (a draft that touches the tree's parser-facing functions).
- `crates/script/src/dom/reads.rs`: the same token edits by three stories: `charge` is made
  `pub(super)` by the attributes story, the selectors story and this story, `Reads` by the
  selectors story and this story, and the selectors story widens the scan's name to `&str`;
  the later merger keeps one of each. This story adds `find` and `html_element`.
- `crates/script/src/dom/mod.rs`, `crates/script/tests/main.rs`: `story:scripts-dom-attributes`
  and `story:scripts-dom-selectors`, each two `mod` lines or one, in different places; this
  story's `mod dom_mutation_cost;` is next to the attributes story's `mod dom_attributes_cost;`
  (line 17 and 18): keep both.
- `crates/script/tests/dom_reading.rs`: `other_slices_still_throw` holds three adjacent lines,
  2424 (this story), 2425 (the attributes story), 2426 (the selectors story): keep all three.
- `crates/script/tests/bindings.rs`: `story:scripts-dom-attributes` (the `Writes` variant);
  the swell pin story (`dom_limit_drops_the_jobs` and a `then` variant of `limit_end`); this
  story changes three tests' members and adds a glue unit test, and no variant.
- `crates/script/src/runner.rs`: `story:scripts-parse-run` edits `evaluate` and `script_end`;
  this story edits the comment above the `Limit::InputBytes | Limit::Steps` arm only, cited by
  its text, at the implementor's base.
- `docs/design/scripts.md`: `story:scripts-parse-run` edits Q1, Q2, Q4, Q5 and Q11 (ordered
  first by the edge); `story:scripts-dom-selectors` adds a row to the table of Q4 after the row
  `parser: attributes per element` (two lines from this story's `parser: steps` row: no edge,
  a hand merge keeping both); the swell pin story edits Q4 (`:338-341`); the attributes story
  edits `:322-332` and `:902-904`, which this story edits too (prose written twice; the edge
  puts this story second). Q1's row is cited by its text.
- `AGENTS.md` and the public surfaces of acceptance 22: shared with
  `story:scripts-dom-attributes`, `story:scripts-dom-selectors` and `story:scripts-parse-run`;
  the later merger takes its numbers from its own log.
- Other plan files change with this story: the epic (a block of corrections),
  `story:html-open-namespace` (decision 3), `story:scripts-dom-reading` (the open item of
  decision 20), `story:scripts-dom-attributes` (its line on `Limit::InputBytes`, its cites of
  this story's lines, and the meaning of `node_charges()`) and `story:scripts-parse-run` (its
  cites of this story's lines).

## Why `epic:scripts-in-a-page`

Writes to the tree, without which no script changes what surf shows (example 1 of the epic). The
document port's example appends an element built by a script, and the hostile-pages story builds
a deep tree by `appendChild` and expects `limit: depth`; both need the writes, each priced so
that a hostile loop of them ends inside the step limit.
