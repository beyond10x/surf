---
format: aep.planning-md/3
id: story:scripts-dom-mutation
kind: story
status: proposed
title: The mutating members (46 rows), four setters and four constructors, charged first
summary: Tree writes, charged before they run and settled to what was made, with the standard's exceptions and the limits' ends.
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
revision: 48
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:13Z", actor: "human:operator", revision: 15}
---
## Context

The epic's ninth candidate (lines 472-475) and its slice "Tree mutation, 29" (lines 736-741),
read at plan commit `9a88a16`: the mutating members of `Node`, `Element`, `Document` and of the
interfaces they return, with the DOM Standard's exceptions, held to the parser's limits.

Read for this text:

- The plan at `9a88a16` (branch `plan/round-2`): the epic, the attributes story at `4437ed9`
  (its revision 7: `charge_to`, `charged_write`, `settle_stored`, `DomEnd::Fault` and the
  accessors are final there, and decision 5 names the contract this story calls), the parse-run
  story at `f877bfd`, the selectors story at `77aaa6a`. Two stories added since `aa70366`
  (`agent-tasks-left-process-wait`, `tests-without-wall-time-bounds`) touch nothing here.
- The code at `83e6b8a`, the reading unit as this story was first written against, and at
  `14f8a2a`, the tip of that unit's branch with its fix round (`cd43273`, "Walk the tree without
  copying a child list"; not reviewed, not on `main`). `main` at `a14e3b0` for the public
  strings and the vendored DOM Standard (`dom.html`). The engine at the tag the runner pins
  (`swell` `0.8.0`, commit `0c18271`), `host/host_bytes.rs` and `host/mod.rs`.
- A cite of code without a commit is a line of `83e6b8a`; a cite with `@` names its commit.
- Numbers below that are arithmetic from the formulas are marked "arithmetic"; no build has
  measured them. The build measures each one and reports a difference as a defect of the
  formula or of the code, fixed in the same change.

What `crates/webidl/selection.txt` holds for this slice at `83e6b8a` (278 lines, and 278 at
`a14e3b0`):

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

The glue's copy of one `String` argument (`Copies::copy`) charges after the copy: at `83e6b8a`
`units / 64` steps (`bindings.rs:628`); at `14f8a2a`, after the reading story's price commit,
`2 x units / 16` steps (`bindings.rs:630`), with `STRING_BYTES_A_STEP` = 16 (`bindings.rs:1259 @
14f8a2a`; `:1257 @ 83e6b8a`). With `u` the UTF-16 units of one argument that is
`g(u) = floor(u / 8)` steps. The text below uses `g`; the price commit is not built at
`83e6b8a`, and acceptance 1 says what happens when it is not built at the implementor's base.

What the tree's write surface does at `83e6b8a` (`crates/html/src/tree.rs`): it counts no step.
Each write walks, and the walks are not constant. Decision 3 lists every pass with its function
and line. The facts the rest rests on:

- Every tree walk uses its own stack (`pending`); no function of the write surface recurses.
- A removed node stays in the arena with its strings counted (`docs/design/scripts.md:193`).
- `clone_node` gives `NotSupported` for a `Document` (`tree.rs:1204`) and admits the whole clone
  before it makes a node (`:1207`); `clone_cost` (`:1175`) is private.
- The only places that make a `Limit` are `tree.rs:733` (Nodes), `:736` (StoredBytes), `:865`
  (Depth) and `:1517` (Attributes). `Limit::Attributes` is reached only through `put_attribute`,
  which no mutating member calls. `Limit::InputBytes` and `Limit::Steps` are made nowhere in
  `tree.rs`.
- A child list is a `Vec<NodeId>` (8 bytes an id). It grows by `children.splice` in
  `insert_checked` (`:1021`), by `children.insert` in the parser's `insert` (`:466`) and by
  `push` in `clone_node` (`:1247`); it shrinks in `shrink` (`:710-714`: when the capacity is over
  twice the length, and over 4, `shrink_to_fit`), called by `detach` (`:893`) for a child list
  and by `drop_attribute` (`:1532`) for an attribute list. The callers of `detach` are
  `insert_checked` (`:1000`), `remove_node` (`:1105`), `replace_child` (`:1124`), `adopt_node`
  (`:1275`) and `normalize` (`:1345`); those of `drop_attribute` are `:1600`, `:1618` and
  `:1652`. The parser calls none of them: it uses `remove` (`:524`), `remove_children`,
  `move_children` and `insert` (`:466`).
- Defaults (`crates/html/src/limits.rs`): `max_depth` 512, `max_nodes` 1,048,576,
  `max_attributes` 1,024, `max_stored_bytes` 67,108,864.

Memory comes before nodes. The engine's `memory_bytes` is 67,108,864; a node charges 408 bytes
(so at most 164,482 nodes alone; arithmetic: 164,482 x 408 = 67,108,656), and a wrapper with its
node data takes 423 bytes (`docs/design/scripts.md:329-332 @ 83e6b8a`, measured on the reading
slice), so 831 bytes a created node and 80,756 nodes (arithmetic: 67,108,864 / 831 = 80,756.5).
At the default limits a script that makes nodes ends `limit: memory` before `limit: nodes`. A
case that wants `limit: nodes` lowers `max_nodes` in its own `ParseOptions` and
`Document.limits`. A subtree of 100,000 nodes made by a script costs 83,100,000 bytes (arithmetic:
100,000 x 831), over `memory_bytes`: a case that needs a subtree that large takes it from the
page.

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
  (`steps_per_entry`) or finish well inside it. That includes the copies a child list makes when
  it grows or shrinks: decision 15.
- A cost module records no time assertion and holds each charge by a row that goes red when
  the charge is removed (acceptance 18).
- The tree does not count its passes, so a formula and the tree can drift apart with every row
  still green. Decision 4 ties them.

A `NodeId` belongs to one tree; the bindings never hand a node of one document to another
document's tree.

## Decisions taken

1. Three files and a source test. `crates/script/src/dom/mutation.rs` holds one call per member
   and nothing else: no `tree()`, no `tree_mut()`, no charge. `mutation_reads.rs` holds the
   reads, `mutation_writes.rs` the writes, the charges and the error map. `reads.rs` changes
   only by visibility (four tokens, decision 14). A walk is a small struct with its own stack of
   (parent, next index) (`Subtree`, `Climb`, `Siblings`), advanced by `next(&mut self, tree:
   &Tree)`; one function `step` charges one step and then calls `next`, so a node is charged
   before it is read, and each node a walk reads costs one step. A walk copies no child list: it
   reads `Tree::children(parent)[index]`. Each function does constant work, or charges before
   its loop. `SurfMutation` stays a unit struct: all surf documents are HTML documents, so it
   needs no state. The property this rests on, held by the reading story and checked by this
   one at its start: no walk of `reads.rs` builds its stack from a copy of a child list.
2. Prices. One step per 16 bytes, an argument's bytes included (the glue's copy, once the
   reading story's price commit is in: `g(u)` above); `b16(n)` is `ceil(n / 16)`. A step per
   node made, moved, removed, cloned or visited. A step per 4 attributes for a scan
   (`ATTRIBUTES_A_STEP`, `reads.rs:252`), and a step per attribute record that a clone copies. 8
   steps per collection node in the glue (`COLLECTION_NODE_STEPS`, `bindings.rs:1251`). Ids of a
   child list: 2 ids of the 8-byte `NodeId` a step, `ids(n) = ceil(n / 2)`. `b16` and `ids` are
   written once, in `mutation_writes.rs`, and a unit test there asserts `b16(16) = 1`,
   `b16(17) = 2`, `ids(2) = 1`, `ids(3) = 2` and `size_of::<NodeId>() == 8`. Three list passes,
   each priced in `ids`:
   - `front(i) = ids(i + 1)`: the scan from the front to index `i` (`index_in_parent`,
     `tree.rs:871`);
   - `back(k) = ids(k)`: the scan from the back over `k` entries (the `rposition` of `remove`,
     `tree.rs:528`); a child at index `i` of `c` is `k = c - i` entries from the back;
   - `shift(r) = ids(r)`: `r` ids moved by `Vec::remove` (`tree.rs:532`) or by the `splice` of
     `insert_checked` (`tree.rs:1021`).
   The slice's own walks charge one step for each node they read (`step`): an own walk to index
   `i` from the front costs `i + 1` steps, and an own walk over `k` entries from the back costs
   `k`. A charge pays a pass once, whoever makes it: the slice's own walks are charged as they
   go, the tree's passes by formula before the call. So a removal at distance `k` from the back
   costs `rm(k) = k + ids(k) + ids(k - 1) = 2k` (the own walk that finds the position, the
   tree's scan and the shift; `ids(k) + ids(k - 1) = k` for every `k`): the last child of a
   list costs 2 and the first of `c` costs `2c`.
   The copy that a child list makes when it grows or shrinks is paid by a reallocation share:
   one step for each id a write inserts into, or removes from, a child list (decision 15 gives
   the argument and the constant). A bulk removal (`remove_children`, `remove_all`) pays none:
   it moves no id.
   `d` is the nodes on the path from a node up to the root, itself included (what a climb of
   `parent_or_host`, `tree.rs:773`, visits); `d_n` the same for the moved node.
   Each charge comes before the work it pays for. A climb takes one step before each read;
   `reads.rs::ancestors` collects the chain before it charges, so this slice does not use it.
3. The passes. The tree's write functions at `83e6b8a`, with the end each scan starts from. The
   tables of "What each member charges" are sums of these lines.
   - `remove` (524), reached by `detach` (887): `back(c - i)` at 528, `shift(c - i - 1)` at 532,
     and the `shrink` at 893, which this story replaces for child lists (decision 15).
   - `insert_checked` (987): for a node of the target parent, `front(j)` of the node for
     `next_sibling` at 988 (the test that a node is already where it would go); for a
     fragment, a pass over its `m` children at 994-996 and another at 1015-1020; for a node with
     a parent, `detach` (1000: the `remove` line above); for a given reference child at index
     `i` (after the node left), `front(i)` at 1011; `shift(c_p - i)` at 1021, after the growth
     of the list (decision 15). With no reference child the index is the length (1013) and the
     splice moves nothing: an append of a parentless node costs no list step.
   - `insertion_allowed` (951): the climb of the parent for the ancestor test at 962 (`d`); for
     a `Document` parent `document_allows` (900): up to 5 passes over the children, each child
     dereferenced (909, 913, 917, 919 and 922, 930 and 932), so `5 c_p` steps; for a fragment
     node it also counts the fragment's children and tests them for a text (`any(is_text)`),
     `2m` more.
   - `admit_depth` (848): `element_height` (828) walks the moved subtree, template contents
     included (840), `s` node visits; when the height is above 0, `element_depth` (811) climbs
     the parent (`d`). A fragment walks each child's subtree. `element_height` keeps a stack
     of 16-byte entries.
   - `insert_before` (1079): `insertion_allowed`, then `next_sibling(node)` (front of the node)
     when `child` is the node (1086-1088), then `admit_depth`, then `insert_checked`.
   - `replace_child` (1111): `insertion_allowed` (1117); `admit_depth` (1118);
     `next_sibling(child)` at 1119, `front(i)` of the child; when that sibling is the node,
     `next_sibling(node)` at 1121, `front(j)`; `detach(child)` at 1124, `rm`-without-own-walk,
     `back(c_p - i) + shift(c_p - i - 1)`; then `insert_checked` with the reference.
     For a parentless node: `front(i)` + `back(c_p - i)` + `shift(c_p - i - 1)` + `front(i)` +
     `shift(c_p - i - 1)`, and the slice's own front walk. For a node of the same parent the
     pass at 988 and a `detach` of the node (1000) come on top: `front(j)` + `back` + `shift` of
     the node's own index, 11 passes in all.
   - `move_before` (1133): `root_of` (798) of the parent twice and of the node once (1143,
     1144), the ancestor test (1145) and `element_depth` inside `admit_depth` (1169): four
     climbs of the parent and one of the node, `4 d + d_n`; for an element into a `Document`,
     `document_allows` (1155); `next_sibling(node)` at 1164 when `child` is the node;
     `admit_depth` (`s`); `insert_checked`.
   - `clone_node` (1202): `clone_cost` (1175) visits each node once (`s'` nodes, a template's
     contents fragment counted as a node, 1187) and reads its attributes' sizes; the copy loop
     (1211) makes `s'` nodes and clones each node's data, one allocation for each attribute
     record and string, and copies an element's local name twice (`data.clone()` at 1211-1217,
     then `to_string` in `create_element`); each child is pushed to its parent's new list
     (1247, growth in decision 15); a shallow clone of a template counts its contents as a node
     and does not walk it.
   - `normalize` (1309): `descendants` (635) collects the subtree: `s` visits (1311); a filter
     over it (1313): `s` reads; `text_runs` (1282) reads each parent's children (1318): at most
     `s` reads in all; the first non-empty text of a run (1319) and the sum of the others'
     lengths (1321-1324): one pass each; the merge loop (1343): one more. That is 6 passes of at
     most `s` nodes each. The merge copies the merged bytes twice (`merged`, then `push_str`,
     1333-1340) and `push_str` may reallocate the first text: at most 3 x `B` bytes for `B` the
     text bytes of the subtree. Each removed text node is a `detach` (1345): `rm`-without-own-
     walk, `back + shift`, in the list of its parent at that time. `descendants` does not enter
     a template's contents.
   - `split_text` (1364): `byte_index` (677) to the offset, at most `L` bytes (1368); `admit(1,
     0)`; the two `to_string` copies of the tail and the head (1371, 1372), `L` bytes in all;
     `create(Text(tail), 0)` stores the tail's bytes, and the head's bytes leave the node (the
     tree's stored bytes change by nothing); then, for a node with a parent, `next_sibling(node)`
     (1378, `front(j)`), and `insert_checked` of the new node with that sibling as reference:
     `front(j + 1)` at 1011 and `shift` at 1021.
   - `replace_data` (1386): `utf16_len` of the data (1394, a pass over `L`), `byte_index` twice
     (1398, 1399, each up to `L`), then the build of the new data (1401-1404, one copy of at
     most `L` plus the new data). So 3 scans and 1 copy of `L`, plus the new data once. `set_data`
     (1416) and `append_data` (1422) call `utf16_len` first: one scan more.
   - `set_text_content` (1441): for an element or fragment, `create_text_node` copies the value
     (1447); an any-element test over the children (1450-1453): at most `c_p` reads;
     `remove_children` (539): `c_p` visits; `insert_checked` of the text with no reference.
     For character data it is `set_data`.
   - `adopt_node` (1270): `detach` for a node with a parent; nothing for a template's contents.
   - `create_*` (1027-1074): each string copied once; a `template` makes two nodes (1027);
     `create(data, categories)` adds `stored_bytes_of(data)`.
   The doc comment of each charge in `mutation_writes.rs` lists these passes for its member; a
   count in the tree that is higher than the table's is a defect of the table.
4. The work counter, which ties the formulas to the tree. The tree counts the work of its writes
   in raw units, and owns no price.
   - `Tree::work()` answers a public `Copy` snapshot `WorkCount { ids, bytes, nodes, attributes
     }` (four `u64`): ids of a child list read or moved or copied by a reallocation, bytes
     copied or scanned, nodes visited, made or removed, attribute records copied.
   - The live counters are a private `Work` of four `AtomicU64`, in the pattern of `Writes`
     (`tree.rs:286`: not part of what a tree holds): relaxed ordering, so that a read function
     holding `&self` can raise them and `Tree` stays `Sync`; saturating adds; a hand-written
     `Clone` (it loads each counter) and `PartialEq` (always equal: two trees compare equal
     whatever their work), and `Debug` and `Default`. `Tree` derives `Clone`, `Debug`,
     `PartialEq` and `Eq` (`tree.rs:257-296`); the new `work` field and `with_root` are a region
     of `tree.rs` that this story edits, and the doc comment of the field says why the two
     hand-written impls exist. There is no cargo feature: a feature gives the host's compile
     cache a second copy of the crate.
   - A pass raises the counter once, by its length in raw units (`ids` for a pass over `n` ids,
     `bytes` for `n` bytes, 1 for a node visited, made or removed, 1 for an attribute record
     copied), and a reallocation raises `ids` by the ids it copies (decision 15).
   - The cost module turns a snapshot into steps with the glue's constants:
     `ids.div_ceil(2) + bytes.div_ceil(16) + nodes + attributes`, where 16 is the glue's
     `STRING_BYTES_A_STEP` and 2 is 16 / `size_of::<NodeId>()`. The tree imports no price.
   The ties:
   - For each write function on its worst shape a cost row asserts that the steps the slice
     charged for the call are at least the converted `Tree::work()` raised by the call
     (acceptance 18).
   - A clone row asserts that `s'` and `b` of the slice equal the tree's change in nodes
     (`Tree::node_count()`) and stored bytes (`Tree::stored_bytes()`), read through
     `totals()`.
   - A tree edit that adds a pass and raises the counter for it breaks a row; a pass added
     without the counter is not caught by any row. Every edit of a write function of
     `tree.rs` raises the counter for each pass it adds; the doc comment of `Tree::work()` says
     so, and the sibling-positions story, which may change a list writer, keeps it.
5. Memory. The node handle and the shared body.
   - `DomState` holds one `HostBytes` for the nodes scripts made, `nodes: Option<HostBytes>`,
     and the tracked size of it is `totals.bytes` (the handle's size field is private to the
     engine). `DomState::charges` (`prelude.rs:112`) is removed: nothing pushes into it.
     `node_charges()` is redefined as the count of node handles, 0 or 1 (the attributes story's
     accessor; this story is built second and edits it).
   - `DomCx::charge_nodes(count, stored) -> DomResult<NodeCharge>` wants `3 x NODE_BYTES x count
     + 2 x stored` more bytes. `NodeCharge` is a `Copy` struct `{ nodes: usize, stored: usize }`
     that names what was charged. The first call makes the handle with the engine's
     `Cx::charge_host_bytes` (`host/mod.rs:577 @ 0c18271`); a later call resizes it with
     `Cx::resize_host_bytes(handle, totals.bytes + wanted)` (`host/mod.rs:585`). On `Ok` it
     adds `count` to `totals.nodes` and sets `totals.bytes`. A call with count 0 and stored 0
     does nothing. `charge_node(s)` stays a wrapper over `charge_nodes(1, s)` that drops the
     token: `adversary_3.rs:204`, `adversary_4.rs:218` and `tests/bindings.rs:176` call it and
     are not in scope.
   - What `charge_nodes` answers for each layer of the engine's result. The one place that
     decides it is the attributes story's private body, `DomCx::charge_to(&mut self, handle:
     &mut Option<HostBytes>, bytes: u64) -> DomResult<()>` (`charge_stored` and `charge_nodes`
     both go through it; `charge_node` becomes its wrapper): with `None` it makes the handle
     (`charge_host_bytes`), with `Some` it resizes it, to `bytes`. This story passes `&mut
     nodes` and the new size, `totals.bytes + wanted`:
     - the first call: `Ok(handle)` is kept; `Err(Abrupt::Throw(_) | Abrupt::Terminated(_))`
       maps as `prelude.rs:230` does, to `DomEnd::Throw(InvalidStateError "the memory budget is
       spent")`, a throw the engine never delivers, since the run is terminated with
       `MemoryBudget` and the script ends `limit: memory`;
     - a later call: `Ok(Ok(()))` updates the counters; `Ok(Err(Abrupt))` maps the same way;
     - an outer `Err(HostError)` is a defect of surf's own code: `charge_to` answers the variant
       `DomEnd::Fault`, which the generated glue turns into `Terminated(EngineFault)` (`defect`,
       `prelude.rs:572-574`) and the run ends faulted. The attributes story adds and owns that
       variant and its glue arm; this story only passes it on. No settle follows it, and the
       counters change only on `Ok(Ok(()))` or on the made handle.
   - The settle, one function beside `charge_nodes`: `DomCx::settle_nodes(charged: NodeCharge,
     made_nodes: usize, made_stored: usize) -> DomResult<()>`. It gives back `3 x NODE_BYTES x
     (charged.nodes - made_nodes) + 2 x (charged.stored - made_stored)` bytes through
     `charge_to` (a resize down), lowers `totals.nodes` and `totals.bytes` by what it gave back
     and never grows the handle (`debug_assert!` that `made` is not over `charged`). It has the
     form of `settle_stored` of the attributes story (`DomCx::settle_stored(added, removed) ->
     DomResult<()>`, which that story owns): it answers `Ok(())` only when the engine answered
     `Ok(Ok(()))` or was not called (the size is equal), and the totals change only then. An
     outer `Err(HostError)` is `DomEnd::Fault` as in `charge_to`, and an inner `Err` (which
     `charge_to` has made a `DomEnd::Throw`) is made `DomEnd::Fault` by the settle, so that a
     script never catches it; the totals stay as they were. A shrink cannot fail a live run: in
     `scoped_resize_host_bytes` (`host/host_bytes.rs:79-106 @ 0c18271`) the checks come first
     (`alive()?`, then the foreign-handle check) and a shrink only credits, so `Fault` here is
     a defect of surf's own code. Nothing shrinks after the run has ended, so the settle comes
     before `end_with_limit`, while the run is live. A caller takes the result after the settle:
     a `?` on the settle's own result is allowed. When the tree call answered an `Err` and the
     settle answers `Fault`, the `Fault` is what the member answers.
   - What is charged on a bound and settled to what was made:
     - a create, a clone and `splitText` go through one private function of
       `mutation_writes.rs`, `charged_nodes(cx, count, stored, write)`: `charge_nodes` (its `?`
       is before any change), the tree call `write`, then `settle_nodes` with the tree's changes
       of `Tree::node_count()` and `Tree::stored_bytes()` over the call, after an `Ok` and after
       any `Err` alike (an `IndexSizeError` included: a `?` on the tree call between the charge
       and the settle is a defect, and acceptance 3 holds the order with a source test); the
       function then answers the settle's `Fault` if it has one, else the tree call's result;
     - a variadic list: the glue charges `charge_nodes(k + 1, B)` before the first copy of an
       item (`k` the strings, `B` taken as 3 x their UTF-16 units: a UTF-8 byte is at most 3 per
       unit, so for ASCII text it is an overcharge of 3, and it is allowed; one string of 3 units
       is charged 9 bytes) and settles after the slice's method returns, with the tree's gain
       over the call. `1,000` strings of 3 units: `charge_nodes(1,001, 9,000)` wants `1,001 x 408
       + 2 x 9,000 = 408,408 + 18,000 = 426,408` bytes; the call makes 1,001 nodes and stores
       3,000 bytes; the settle gives back `2 x (9,000 - 3,000) = 12,000` and leaves `1,001 x 408
       + 2 x 3,000 = 408,408 + 6,000 = 414,408` in `totals.bytes`. A `before()` of a node with
       no parent made nothing: the settle gives back all of it (the steps stay paid: decision
       13).
   - The attributes story (its decision on the stored-bytes handle, by name) keeps a second
     handle for the stored bytes of nodes that exist and resizes it to twice the live bytes:
     `DomCx::charge_stored(added, removed) -> DomResult<()>` before a write,
     `DomCx::settle_stored(added, removed) -> DomResult<()>` after it, the accessors
     `stored_live()`, `stored_charged()`, `stored_handles()` and `node_charges()`, and the factor
     of two (`STORED_FACTOR`). It owns `charge_to`, those functions and the factor; this story
     owns `nodes`, `charge_nodes`, `settle_nodes` and the
     wrapper, and adds no second pair for stored bytes. Its own writes go through its private
     `charged_write`; this story does not call it and writes its own, in `mutation_writes.rs`,
     with the same order for stored bytes: `charged_write(cx, added, removed, write)` calls
     `charge_stored` (its `?` before any change), the tree call, then `settle_stored(gain,
     loss)` after an `Ok` and `settle_stored(0, 0)` after any `Err` of the tree call, whatever
     its kind (an `IndexSizeError` included, since the script goes on); `settle_stored` answers
     `DomResult<()>` and its `Fault` wins over the tree's `Err`. The writers that change
     a node that exists (the character-data writers and `normalize`) pass it their closure, so
     none repeats the order. One `charge_stored` and one `settle_stored` for one tree call, the
     sums computed before it; a settle never grows the handle. The order is held by a source
     test over `mutation_writes.rs` (acceptance 3): `charge_stored(`, `settle_stored(`,
     `charge_nodes(` and `settle_nodes(` occur only inside `charged_write` and `charged_nodes`,
     and in each of the two the text between the charge statement and the first settle holds no
     `?`.
   - Where the sums come from, for the writers that change a node that exists: `added` is a
     bound known before the call (`data.len()` for the character-data writers, the merged bytes
     for `normalize`), `removed` is 0 before the call, and the settle takes the change of the
     tree's stored bytes over the call, read from `Tree::stored_bytes()` before and after: its
     rise is `gain`, its fall is `loss`. The tree keeps the strings of a removed node, so
     `normalize`, which empties the later texts of a run into the first, raises the count by the
     merged bytes less the first text's own, and its settle is `settle_stored(gain, 0)`; a
     `deleteData` falls. So the handle ends at the stored bytes the tree holds, and a refused
     call leaves it where it was. `splitText` makes no stored call: the head's bytes leave the
     node that exists and are not lowered from `live`, which leaves the handle above its floor (a
     conservative error); the tail's bytes are charged with the new node.
   - A removal refunds nothing for nodes (their 408 bytes and strings stay charged, because the
     node stays in the arena); the stored bytes of a character-data write that shrinks are
     refunded by that settle.
   - A loop that makes and removes nodes ends `limit: memory`, at the session's limits, before
     `limit: nodes`.
   - The charge for a node is made before the node: a loop of `createTextNode` is charged by
     its count, not by an item. The temporary fragment of a variadic convert is a node
     (`create_document_fragment`, `tree.rs:1071`): for two items or more the convert charges
     `charge_nodes(k + 1, B)`.
   - The glue charges `n` steps in `to_nodes_or_strings`, before its loop, and the memory charge
     in `finish_nodes_or_strings` before the copy loop. A member that then does nothing (a
     `before()` with no parent) has paid its steps: an allowance, named in decision 13.
   - Memory that the engine's budget does not see, and its bound:
     - the tree's transient vectors, an entry each and bounded by the node limit: `element_height`
       16 bytes, `clone_node` 24, `descendants` 8, `text_runs` 24 a run and 8 an id (sizes from
       the types; arithmetic: 1,048,576 x 16 = 16,777,216, x 24 = 25,165,824);
     - the script list of a subtree walk (the `script` elements to record): 8 bytes each, up to
       the node limit (arithmetic: 1,048,576 x 8 = 8,388,608, 8 MiB);
     - the glue's two item vectors of a variadic list, `n x size_of::<RawNodeOrString>()` and `n
       x size_of::<NodeOrString>()`, each growing by doubling, `n` bounded by the engine's own
       argument array; and the membership set of `before` and `after`, `n` entries;
     - a child list's capacity over its length: at most `max(4 x len, 4)` ids after a write of
       this story that removes a child, at most `2 x (len + m)` after a growth, 3 x the length
       while a growth copies (the old and the new vector), and what it had before for a list
       that a bulk removal emptied (decision 15); at the node limit 4 x 1,048,576 x 8 =
       33,554,432 bytes;
     - `replace_data` holds the old and the new text at once, so `2 L`; `splitText` holds the
       text, the tail and the head, so `2 L` more; `normalize` holds the merged text, up to the
       bytes of the longest run (twice, `merged` and `push_str`);
     - a removed node's arena slot and strings, bounded by 164,482 script-made nodes and the
       parser's own limits;
     - the single argument copies of the glue (`Copies::copy`): bounded by the string length
       limit, not charged before the copy, and the vector the copy makes, 2 bytes a unit
       plus the `String` of up to 3 bytes a unit (arithmetic: at 16,777,216 units, 33,554,432
       bytes of `u16` and 50,331,648 bytes of `String`). No story holds the move before the
       charge yet.
6. A write over a limit ends through `end_with_limit` with the limit's word (`limit: nodes`,
   `limit: depth`, `limit: stored-bytes`, `limit: memory`), also inside `try { } catch { }`.
   The tree after it:
   - for a refused call that wrote nothing before the refusal, the tree equals the tree just
     before the call: serialization, node count and `writes()`;
   - for a variadic or `ChildNode` member with two items or more, the tree is as after the
     convert (the standard's own order for its exceptions): the items have left their old
     parents and sit in the temporary fragment, which is a node. The by-hand form of that
     convert is `f = document.createDocumentFragment(); f.appendChild(a); f.appendChild(b)`: it
     makes one node (the fragment) and raises `writes()` by 2 (by 4 when `a` and `b` had
     parents, since each leaves its old parent), as the refused call does. It is not
     `f.append(a, b)`, which is itself a two-item `append` and would build a second fragment;
   - after any call, refused or not, the nodes the tree gained equal `totals().nodes` and the
     bytes it gained equal what `totals().bytes` holds above the settled floor: a create, a
     clone or a convert that the tree refuses settles the handle and `totals()` to the nodes
     and bytes the tree did gain (decision 5). For a `Nodes` refusal in the middle of a convert
     that is the nodes made before it, which stay in the arena.
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
     `collection_object_inner`), which the getter calls after its own charge. The inner body of
     `refresh_collection` keeps the record of a cut refill, `cut_refill` (`prelude.rs:1347`),
     that `Runner::evaluate` reads (the parse-run story); its test is
     `adversary_dom_reading_3.rs:149-220`, which stays green and is not edited.
   - `strings_value` is in `prelude.rs` (`:1441`); `generate.rs` only names it
     (`generate.rs:239`), and the attributes story edits it there. This story alone edits
     `generate.rs`, in `same_object_getter` (`:695`).
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
    test, the `Document` checks) runs after the charge of its walk, inside the tree. The offset
    of `splitText` is checked first, by an own pass over the text charged `b16(L)` before it:
    `IndexSizeError` is thrown before any node is charged. So a script can end `limit:
    steps-per-script` where a browser would throw `HierarchyRequestError`, `NotFoundError` or
    `IndexSizeError`: that is a named allowance, not a deviation of behaviour, and acceptance 16
    has a pair for each of five families (an ancestor test, a `NotFound`, a `Document` parent,
    an `IndexSize` and an `InvalidCharacterError` or `NamespaceError` of a create). The same
    allowance covers the steps of a variadic call that then does nothing (`before()` with no
    parent): its `n` steps stay paid, and its memory is given back by the settle (decision 5).
14. The pre-charge rule, and the walks of `reads.rs`. The tree counts no step, so the slice
    charges by formula before the call, from lengths it reads in constant time or walks step by
    step. The formulas are upper bounds of the tree's passes, except where a table says
    "exactly". The tree is not given a step budget (question 1); decision 4 holds the formulas
    to it. The property that the walks of this slice rest on: no walk of `reads.rs` builds its
    stack from a copy of a child list. The reading story owns it (its decision 20 gets a
    sentence that says so) and this story checks it at its start, mechanically:
    - at `83e6b8a` the property is false: `pending.extend(` of a child list is at `reads.rs:363`
      (`find`, `:349-366`), `:914` (`text_content`) and `:935` (`is_equal`); `.to_vec()` at
      `:883` (`children`) and `:1169` (`whole_text`); `.collect` at `:527` (`element_children`)
      and `:768` (`shallow_equal`);
    - at `14f8a2a` (`cd43273`) it holds: `find` (`:458`, private) wraps `preorder` (`:475`),
      whose stack is a `Vec<(NodeId, usize)>` and which copies no child list; `pending.extend(`
      occurs nowhere; `.to_vec()` is at `:1023` (`children`, charged one step for each child
      before the copy), `.collect` at `:662` (`element_children`, charged first) and `:903`
      (`shallow_equal`, the attributes of one node in a map); `Reads` (`:332`), `charge`
      (`:130`), `html_element` (`:303`) and `find` are private there. At `a14e3b0` there is no
      `reads.rs`;
    - the two commands of acceptance 1 test it, and each says "prints nothing else; if it does,
      the work stops and the implementor reports";
    - this story's edit of `reads.rs` is `pub(super)` on `charge`, `find`, `Reads` and
      `html_element`, and nothing else: a layout-blind diff holds it (acceptance 24).
    The `Document title` setter is the one member that calls `find`. It waits for the gate: when
    the gate of acceptance 1 passes at the implementor's base, the setter calls `find`; until
    then the setter's work is not started, and the story is not complete. The price gate of
    acceptance 1 and this wait are one merge condition.
15. Child lists keep their capacity in proportion to their length, so that a reallocation is
    paid by the writes between two of them. The tree as built does not: `shrink` (`tree.rs:710-
    714`) gives the capacity back once it is over twice the length, and a splice that must grow
    doubles it, so on a list whose capacity equals its length the cycle append, remove, remove
    reallocates twice a cycle (a grow of `n` ids, then a shrink of `n - 1`) for about 28 steps
    (arithmetic: 18 for the append, 5 and 5 for the removals), and 200,000 cycles stay under the
    step limit. This story changes the tree so that it holds:
    - A child list shrinks only when its capacity is over `max(4 l, 4)` for `l` its length, and
      then to `max(2 l, 4)` (`shrink_to`). A new function `shrink_children` does it and replaces
      `shrink` at `tree.rs:893` (`detach`); `shrink` stays as it is for attribute lists
      (`drop_attribute`, `:1532`), which the attributes story's bound of at most twice keeps.
    - A splice that needs room (`len + m > capacity`) first reserves exactly, so that the
      capacity is `2 (len + m)`: one reallocation (`reserve_exact(len + 2 m)` before the splice
      at `tree.rs:1021`). The clone's pushes (`:1247`) go through one function, `push_child`,
      whose growth is `Vec`'s doubling (`2 x` the old capacity).
    - The tree's work count includes the ids a reallocation copies: the splice's growth,
      `shrink_children` and `push_child` raise `ids` by the ids copied (decision 4).
    - Each write that inserts into or removes from a child list pays one step for each id it
      inserts or removes (the reallocation share of decision 2): the insert formulas carry `3 m`
      where they carried `2 m`, a removal carries 2 for the node and its share, a removal from
      an old parent carries `+ 1`, `normalize` carries `c_p + 2` for each removed text, a clone
      carries `s' - 1` for the children it pushes, and a bulk removal carries none.
    The argument. Call a growth or a shrink that copies a list's ids to a new buffer a
    reallocation, and let a list's length just after one be `l`: its capacity is then `2 l` (a
    growth: `2 (len + m)`; a shrink: `max(2 l, 4)`; a push-grown list of a clone: `2 (l - 1)`,
    which only helps the first bound below). Let `W` be the ids inserted or removed by the writes
    after that reallocation, up to and including the write that causes the next one, and `x` the
    ids the next one copies.
    - The next is a growth: the length must pass the capacity `2 l`, so at least `l + 1` ids are
      inserted, `W >= l + 1`, and the copy is at most the capacity: `x <= 2 l < 2 W`.
    - The next is a shrink: the length must fall below a quarter of the capacity, `l / 2`, so
      more than `l / 2` ids are removed, `W > l / 2`, and the copy is the new length, `x < l / 2
      < W`.
    So the price of the copy, `ids(x) = ceil(x / 2)`, is at most `W` for a growth and at most
    `ceil(W / 2)` for a shrink, and one step for each id of `W` pays it. The intervals of `W` are
    consecutive and share no write (each ends with the write that causes a reallocation and the
    next begins after it), so each id is paid for once. For a list of 4 ids or fewer the copy
    is at most 4 ids. The sentence "a reallocation is paid by the writes between two of them" is
    true after this change, with the constant 1 step an id for a growth and for a shrink alike
    (a shrink needs only half of it).
    What the argument does not cover, named and not charged: ids the parser put into a list
    count in `W` and pay nothing, and a list the parser made has a capacity `Vec`'s growth left
    it, so the first reallocation a script causes in such a list, and a later one that the
    parser's inserts helped to cause, may copy ids that no script write paid. Their total is at
    most twice the ids the parser inserted, at most `2 x 1,048,576` ids over a document, that is
    1,048,576 steps in all (arithmetic: `2 x 1,048,576 / 2`) and at most 524,288 steps for one
    list (arithmetic: `1,048,576 / 2`), a tenth of `steps_per_entry`. The parser reaches none of
    `shrink`'s callers: `detach` is private and its callers are `insert_checked`, `remove_node`,
    `replace_child`, `adopt_node` and `normalize` (decision 3 lists the lines), none of which the
    parser calls; the parser's lists grow by `children.insert` (`tree.rs:466`) and are never
    shrunk.
    The tests that hold it (acceptance 18, a cost row, and acceptance 25, tree unit tests): the
    cycle append, remove, remove on a parser-made list of 100,000 children, and the count of
    capacity changes of one list over 1,000 cycles.

### What each member charges and does

Symbols: `d` and `d_n` of decision 2; `s` the nodes of the subtree being moved or copied,
template contents included, `s'` the nodes a clone makes (a template counts as 2), `a` the
attribute records and `b` the stored bytes of the clone's subtree, `N` the bytes of the local
names of its elements; `m` the nodes put into a child list (1, or the children of a fragment);
`c_p` the children of the target parent, `c_o` those of the node's old parent, `i` the index of
the reference child, `j` the index of the node in its parent, `k_o = c_o - j` its distance from
the back; `n` the items of a variadic list, `k` its strings, `B` their bytes; `L` the bytes of a
text; `g(u)` of the glue's copy (Context). The glue's one step, `G = 1`, comes first for every
member; its copy of a `String` argument is `g(u)` for each argument. Every figure below is a
sum of the lines of decision 3 and of the shares of decision 15.

Own walks, which every table below includes as "own": `d` for a climb of the target (and `d_n`
of the node for `moveBefore`), `s` for the subtree walk that lists the `script` elements and
counts `s`, `i + 1` for a front walk to index `i`, `k` for a back walk over `k` entries.

List terms of one node put into a child list, used by every insert, move and replace below:

- a reference child at index `i` (counted after the node left, when it came from this list):
  the own front walk `i + 1`, the tree's `front(i)` at 1011 `ids(i + 1)`, and the shift
  `ids(c_p - i)`; with no reference child nothing;
- the node is already a child of the target: `front(j)` at 988, `ids(j + 1)`; and the same again
  when `child` is the node (1087);
- the node has a parent: its removal, `2 k_o + 1` (the own back walk `k_o`, the tree's scan
  `ids(k_o)`, the shift `ids(k_o - 1)`, and the share 1);
- the target is a `Document`: `5 c_p`, and `2 m` more when the node is a fragment.

- Create (11 rows): `createElement`, `createElementNS`, `createTextNode`, `createComment`,
  `createDocumentFragment`, `createProcessingInstruction`, `createCDATASection`, and the four
  constructors.
  - Reads: none. Writes: `Tree::create_*` (`tree.rs:1027-1074`), no parent.
  - Charge, in order: the glue's step and `g(u)` for each argument; the check as one pass over
    the copied argument, `b16(len)` for each validated string; for `createElement` the ASCII
    lowercase, another `b16(len)`; then `charge_nodes(count, stored)` (count 2 for a `template`,
    `tree.rs:1027`) and `count + b16(stored)` steps (the nodes, and the tree's own copy of each
    stored string); then the tree call, all of it inside `charged_nodes` (decision 5). So:
    - `createTextNode`, `createComment`: `2 + g(u) + b16(L)`;
    - `createElement`: `1 + g(u) + 3 b16(L) + count`;
    - `createElementNS(ns, q)`: `3 + g(ns) + g(q) + 2 b16(L_q)`: the glue's step, one step for
      the match of `ns` against the six namespaces (constant), the copies, one pass over `q`
      (the split at the colon and the check), the tree's copy of the name, and the node; a call
      that throws pays `2 + g(ns) + g(q) + b16(L_q)` (the step, the match, the copies, the one
      pass over `q`) and makes no node;
    - `createProcessingInstruction(target, data)`: `2 + g(t) + g(d) + 2 b16(L_t) + 2 b16(L_d)`:
      the check of the target and the scan of the data for `?>`, and the tree's copy of both;
    - `createDocumentFragment`: 2;
    - `createCDATASection`: `1 + g(u)`, then the exception, before any node is charged.
  - Exceptions: `createElement`: InvalidCharacterError for a name that is not a valid element
    local name (`dom.html:775-809`, 8384); then ASCII-lowercase, the HTML namespace, `options`
    ignored (no custom elements). `createElementNS`: validate and extract (`dom.html:836-908`):
    InvalidCharacterError for the prefix (867) and the local name (881), then NamespaceError
    (886, 891, 896, 902); then NotSupportedError for a namespace outside the six values of
    `Namespace` (`tree.rs:79`) or for a prefixed name (deviation 1: the tree's element has no
    prefix). A colon as the first or a later character of the qualified name gives
    InvalidCharacterError or, when the prefix is valid, the deviation's NotSupportedError; never
    an element. `createCDATASection`: NotSupportedError always, first (8512), before the `]]>`
    check. `createProcessingInstruction` and its constructor: InvalidCharacterError when the
    target does not match the XML `Name` production (12019) and when the data holds `?>`
    (12024). The other creates and constructors have none. A constructor is the matching
    `createX` on the one document tree, with the same charge.
  - Limits: Nodes for every row but `createCDATASection`; StoredBytes for the 8 rows that store
    a string. Counter: none (`create_*` does not raise).
- Insert, one node (4 rows): `Node insertBefore`, `Node appendChild`, `Element
  insertAdjacentElement`, `Element insertAdjacentText`; the variadic ones are below.
  - Writes: `Tree::insert_before` (`tree.rs:1079`).
  - Charge before the call: `1 + 3d + 2s + 3m` (the glue's step; own `d` and `s`; the tree's `2d`
    for the two climbs at 962 and 861, `s` for the height walk, `2m` for the two passes over the
    moved nodes; `m` for the share) and the list terms above. An append of a parentless node to
    a non-document parent adds no list term.
  - `insertAdjacentElement(where, element)` and `insertAdjacentText(where, data)`: `where` is
    matched ASCII case-insensitively against `beforebegin`, `afterbegin`, `beforeend`,
    `afterend` (`dom.html:11179-11221`), SyntaxError otherwise, before any charge and before
    any node is made (`insertAdjacentText` makes its text node after the check, through
    `charged_nodes(1, len)`, and adds `g(u) + 1 + b16(L)` for it). `beforebegin` and `afterend`
    answer null (and write nothing) when `this` has no parent. `afterend` finds `this`'s index by
    the own front walk to `j + 1` (`j + 2` steps, which also reads the next sibling) and inserts
    before that sibling (`i = j + 1`), with `ids(j + 2) + ids(c_p - j - 1)` for the tree's two
    passes; `afterbegin` inserts before the first child (`i = 0`); `beforeend` appends.
  - Exceptions: those of decision 13. Limits: Depth; Nodes and StoredBytes for the text node of
    `insertAdjacentText`. Counter: raises when the child list changes.
- Variadic `ParentNode` (9 rows): `prepend`, `append`, `replaceChildren` on `Element`,
  `Document`, `DocumentFragment`.
  - Order: the glue charges `n` steps (one for each item) before its loop (a change to
    `to_nodes_or_strings`, `prelude.rs`); then, in `finish_nodes_or_strings`, before the copy
    loop, `charge_nodes(k + 1, B)` for two items or more (`charge_nodes(k, B)` for one); the
    copies, `g(u_i)` for each string; the slice's convert (`dom.html:5016-5041`): no items does
    nothing, one item is used as it is, more make a fragment (1 step) and insert each item into
    it; the insert of the result (`prepend` 5163, `append` 5179); and then the glue's settle of
    the node charge to what the tree gained (decision 5).
  - A string item becomes a text node: `1 + b16(L_i)` steps (the node and the tree's copy).
  - Each item put into the fragment pays `3 + 2 s_i + 3 m_i` (the fragment's own `d = 1` twice
    over, `3d`; the item's `s_i` twice; the passes and the share of its `m_i` nodes: `m_i` is 1
    for a node and the item's children for a fragment), plus:
    - `2 k_o + 1` when the item has an old parent (own back walk);
    - for an item whose parent is the fragment (the same node passed twice or more): the tree
      finds it with `front(j)` at 988 before it moves it, so `ids(c_f)` (`c_f` the fragment's
      children, read in constant time, an upper bound of `ids(j + 1)`), and `2 k_o + 1` from an
      own back walk for its removal from the fragment.
  - The final insert of the fragment is an insert as above with `s` the fragment and its
    children and `m` its children, and `5 c_p + 2 m` for a `Document` (a fragment into a
    `Document` pays `document_allows` and the count and text test over the fragment).
  - `replaceChildren` (5194): the convert, then `2d + s` for the validity check before any
    removal (the new public check of the Change, `check_insertion`, which runs the ancestor
    test, the `Document` checks and the depth check, and charges `5 c_p` once for a `Document`
    parent), then `2 c_p` for the removal (the any-element scan and `remove_all`, a step each
    child each pass; no share), then the insert, which sees `c_p = 0` and charges no `5 c_p`.
    A refused check leaves the children (`dom.html:4760-4795`). In all, for one parentless
    item: `1 + n + 4 + 1 + 9 + 2 c_p + 12` (own `d` and `s`, the check, the removal, the insert
    `2d + s + 3m` with `d = 4`), that is `28 + 2 c_p`; for a `Document` (`d = 1`): `13 + 7 c_p`.
  - Exceptions, limits and counter: as an insert; `replaceChildren` also raises when it
    removes. Limits: Depth, Nodes and StoredBytes (the text nodes and the fragment).
- `ChildNode` (12 rows): `before`, `after`, `replaceWith`, `remove` on `Element`,
  `CharacterData`, `DocumentType`.
  - `before`, `after`, `replaceWith` (5335, 5370, 5399): a `this` with no parent does nothing
    and charges nothing beyond the glue's step and its `n` item steps (the allowance of
    decision 13). Otherwise the viable sibling is computed before the conversion: the own front
    walk to `this`'s index (`j + 1` steps; the insert's reference walk continues from it, so it
    is one walk and not two), `n` steps for the membership set of the items, and one step for
    each sibling examined on the walk away from `this` (at most the siblings that are in the
    set, plus the first that is not); then the conversion as above; then:
    - `before`: the pre-insert with the reference `this` at index `j`, as an insert: `1 + 3d +
      2s + 3m` with the list terms `ids(j + 1) + ids(c_p - j)` (the own walk is paid);
    - `after`: the reference is the viable next sibling; the own walk reads one sibling more
      (`j + 2`), `ids(j + 2) + ids(c_p - j - 1)`;
    - `replaceWith`: the tree call is `replace_child(parent, node, this)` (or `insert_before`
      with the viable next sibling when `this`'s parent changed during the conversion), so it
      counts the passes of `replace_child`: 6 for a parentless or fragment node, 11 for a node
      of the same parent; a parentless single node costs `3 + 3d + 2s + 3m + 2 + (j + 1) + 2
      ids(j + 1) + k + ids(k - 1)` with `k = c_p - j`, the three at the front being `1 + n +
      membership + sibling`.
    `replaceWith()` with an empty list is a removal.
  - `remove` (5436): no parent, nothing. Else the removal below.
- Remove and replace (`Node removeChild`, `Node replaceChild`, and the removals above).
  - `removeChild(child)`: read `parent(child)` (constant); if it is not `this`, NotFoundError
    (`dom.html:4810`), before any charge. Else `3 + 2k` steps: the glue's step, `rm(k) = 2k` (the
    own back walk, the tree's scan, the shift), 1 for the node and 1 for the share; then
    `Tree::remove_node` (`tree.rs:1104`). Counter: raises.
  - A removal of the last child costs 5 steps (`k = 1`: `3 + 2`) and the first of `c` children
    `2 c + 3`. So a loop that removes from the front is quadratic: a list of `n` removed whole
    from the front costs `n^2 + 4 n` (arithmetic: the sum of `2 c + 3` for `c` from 1 to `n`
    is `n (n + 1) + 3 n`), which is 4,008,000 for `n` = 2,000 and 25,020,000 for `n` = 5,000. A
    list of about 3,161 children removed whole ends `limit: steps-per-script` (arithmetic: 3,161^2
    + 4 x 3,161 = 10,004,565, and 3,160 gives 9,998,240, before the script's own steps); the
    first 1,127 removals from a list of 5,000 do (arithmetic: `r (2 n - r + 4)` with `n` = 5,000
    is 10,004,379 at `r` = 1,127); the same loop from the back costs 5 steps a turn, 10,000 for
    2,000. That is the price; the cost module holds both.
  - `replaceChild(node, child)` (`tree.rs:1111`): the own `d`, `s` and the own front walk `i +
    1`; then `2d + s + 2m`, the passes of decision 3, `m` for the share, 1 for the child
    removed and 1 for its share, and `5 c_p` for a `Document`. For a parentless node, with `k = c_p
    - i`:
    `1 + 3d + 2s + 3m + 2 + (i + 1) + 2 ids(i + 1) + k + ids(k - 1)` (the two `front(i)` at 1119
    and 1011, the removal `ids(k) + ids(k - 1) = k` and the shift `ids(k - 1)` at 1021).
    For a node of the same parent the passes of the same-parent variant come on top, listed:
    `ids(j + 1)` at 988; its removal at 1000 with the own back walk `k_x = c_p - j`, `2 k_x`,
    and 1 for the share; and `ids(j + 1)` once more at 1121 when the child's next sibling is the
    node. `replaceChild(x, x)` moves no counter: `insert_checked` returns early (`tree.rs:988-
    991`). Exceptions: `dom.html:4691-4755`; the order is the tree's.
- `moveBefore` (3 rows: `Element`, `Document`, `DocumentFragment`): own `d`, `d_n`, `s`, the own
  front walk `i + 1` and, since the node always has a parent, the own back walk `k_o`; then `4d +
  d_n + s + 2m + m`, the list terms of an insert (`ids(i + 1)`, `ids(c_p' - i)` with `c_p'` the
  target's children after the node left, and `2 k_o + 1`), and `5 c_p` for an element into a
  `Document`; then `Tree::move_before` (`tree.rs:1133`). In all `1 + 5d + 2 d_n + 2s + 3m` and
  the list terms. It connects no node that was not connected, so it records no script.
  Counter: raises.
- Clone, import, adopt (3 rows).
  - `cloneNode(deep)`: a `Document` throws NotSupportedError before any charge (deviation 2; the
    standard allows it). Else an own walk of the subtree, `1 + ceil(attributes / 4)` steps
    before each node, giving `s`, `a`, `b`, `N` (it descends a template's contents when deep, as
    `clone_cost` does); then `2 s' + 2 a + b16(b + N) + (s' - 1)` steps (the cost walk and the
    copy loop, one step each node and each attribute record on each; the copy of the bytes, the
    element names counted twice; the share of each child pushed); then `charged_nodes(s', b)`;
    then `Tree::clone_node` (`tree.rs:1202`). A clone at the node limit ends `limit: memory`
    first at the session's limits. The tree raises the counter once for each copied child
    (accepted, question 4): a kept collection is refilled, and the refill is charged.
  - `importNode(node, options)`: a `Document` throws NotSupportedError first (`dom.html:8572`);
    the depth is `options` as a boolean, or `not selfOnly`; then as a clone
    (`Tree::import_node`, `tree.rs:1264`).
  - `adoptNode(node)`: a `Document` throws NotSupportedError (8736); a node with a parent is
    removed: `3 + 2 k_o` in all (the glue's step, `rm(k_o)`, the node and its share); a
    template's contents fragment is a no-op (`tree.rs:1273`); then `Tree::adopt_node`
    (`tree.rs:1270`). Counter: raises when it removes.
  - Limits: Nodes and StoredBytes. The clone admits before it makes a node already
    (`tree.rs:1207`): a guard. No Depth: a copy is a root.
- `normalize` (`Node`, `dom.html:6946-7016`): the glue's step; an own walk (`s` steps, a step
  before each node) over the subtree (not into template contents) that finds, for each parent
  `p`, the children `c_p`, the text nodes the tree will remove `removed_p` (a run's texts but
  the first non-empty one, or all of them when each is empty), and the text bytes `B`. Then,
  before the call: `6 s` (the six passes of decision 3), `3 x b16(B)`, and the sum over the
  parents of `removed_p x (c_p + 2)` (an upper bound of the `detach` of each removed text,
  `rm(k) <= c_p`, with 1 for the node and 1 for the share); and the merged bytes `added` through
  `charge_stored(added, 0)` inside `charged_write` (decision 5), which settles with the tree's
  rise of stored bytes after an `Ok` and with `settle_stored(0, 0)` after any `Err`. Limits:
  StoredBytes (merged bytes are stored again). Counter: raises when a text node is removed. The
  sum, not the longest run, is what bounds the work: many parents of long runs cost the sum.
- Character data (`appendData`, `insertData`, `deleteData`, `replaceData`, `splitText`, and the
  setters of `data`, `nodeValue` and `textContent`). The slice makes no pass of its own over
  the text except the offset check of `splitText`: the tree's `IndexSize` test is the one that
  reads it, so every pass is charged before the call, and the call answers the exception.
  - Reads: the text's byte length `L` (constant, `Tree::data`).
  - `replaceData`, `insertData`, `deleteData`: `1 + g(u) + 4 b16(L) + b16(data_len)` steps (the
    glue's step and copy; the 3 scans and 1 copy of decision 3; the new data once);
    `appendData` and the `data` and `nodeValue` setters: `1 + g(u) + 5 b16(L) + b16(data_len)`
    (`deleteData` has no data argument: `g` and `data_len` are 0). Then, inside `charged_write`,
    `charge_stored(added, removed)` with `added = data.len()` and `removed = 0` (bounds known
    before the call); the call; after any `Err` `settle_stored(0, 0)` and, for a limit,
    `end_with_limit`, or the exception is thrown; after an `Ok`, `settle_stored(gain, loss)`
    from the tree's change of stored bytes.
  - `splitText(offset)`: the offset is checked first by an own pass over the text, `b16(L)`
    steps, and `IndexSizeError` is thrown before any node is charged. Else `charged_nodes(1,
    L)` on the bound `L` (the tail is at most `L` bytes), settled to the tail's bytes (the new
    node's data length, read from the tree), and `2 b16(L) + 1` steps (the tree's scan, the two
    copies, and the node); for a node with a parent the own front walk `j + 1` and `ids(j + 1) +
    ids(j + 2) + ids(c_p - j - 1)` (1378, 1011, 1021) and 2 for the insert of one id with its
    share; then the call. In all, with no parent `2 + 3 b16(L)`. Raises when the node has a
    parent.
  - The setter of `data` is `replaceData(0, length, value)`. The setter of `nodeValue` takes
    `Option<String>` (null is the empty string): on character data it is that, on others
    nothing. The setter of `textContent` (`dom.html:6850-6933`): on character data it replaces
    the data; on an element or fragment it is replace-all: `4 + g(u) + 2 c_p + b16(len)` steps
    (the glue's step and copy, the node, the insert of one id with its share, the any-element
    scan and `remove_children`, and the tree's copy), then `charged_nodes(1, len)` when the
    string is not empty (no node, and no `1 + 2` of those steps, when it is), the call
    (`tree.rs:1441`); on others nothing. Counter: character-data writes do not raise; the
    replace-all raises when it removed a child or made a child.
  - Limits: StoredBytes for the writers, Nodes and StoredBytes for `splitText` and for the
    replace-all.
- `Document.title` setter. The HTML Standard's algorithm (section "document.title", not
  vendored here): with no document element, return; a document element in the SVG namespace
  takes its first `title` child; an HTML element takes the first `title` element in tree order;
  with none, an HTML document element makes a `title` under the first `head` child of the
  document element and returns when there is none; then the text of that element is replaced
  by the value. It calls the reading unit's `find` (`reads.rs:349 @ 83e6b8a`, `:458 @
  14f8a2a`), a stepwise walk that charges a step before each node and copies no child list once
  the gate of decision 14 passes, so `find`, `Reads` and `html_element` (`reads.rs:218 @
  83e6b8a`) are made `pub(super)`. Charge: the glue's step and copy, the walk, then for a new
  `title` the create (`charged_nodes(1, 5)`, 1 step) and an insert under `head` as above, then
  the replace-all of `2 c_t + b16(len)`, its text node and its insert (1 + 2 steps). With one
  existing `title` that holds no child, `1 + g(u) + walk + 1 + 2 + b16(len)`. Limits: Nodes
  and StoredBytes. `Limit::Depth` is unreachable here: a depth counts elements
  (`element_depth`, `tree.rs:811`), a text node adds none, and a new `title` under the `head`
  child of the document element is at depth 3 (`html`, `head`, `title`), which is the smallest
  depth at which a script runs (a `script` in `head` or in `body`), so a parse that ran the
  script admits it. In one script the element comes before its text node.

Which tree write raises the counter, summarized from `tree.rs @ 83e6b8a`: insert, replace, move,
adopt and remove raise when a child list changes; `normalize` when it removes a text node;
`split_text` when the node has a parent; `set_text_content` on an element or fragment when it had
children or the value is not empty; a deep clone once for each copied child. Character-data
writes and the `create_*` functions do not raise. A refused write does not raise. The table the
test reads, one line for each row (member; tree function; raises when):

- `insertBefore`, `appendChild`, `insertAdjacent*`, `prepend`, `append`, `before`, `after`;
  `insert_before`; a node moves.
- `replaceChild`, `replaceWith`; `replace_child`; always, except `replaceChild(x, x)`, whose
  `insert_checked` returns early (`tree.rs:988-991`) and moves no counter.
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
each member of the family, inside one `#[test]`; `limit_memory_default` is one case per family,
a loop of that family's member made until the memory ends, each in its own parse inside the
one test):

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
  `insertAdjacent*`, `prepend`, `append`, `replaceChildren`, `before`, `after` and
  `replaceWith`. The subtree walk's list is reused; the walk that lists scripts skips the
  contents of a `template` (they are not connected), where the walk that gives `s` enters them
  (the tree's passes do).
- Do not record: `moveBefore` (nothing becomes connected), `adoptNode`, the setters (the
  `title` setter makes no script either), `splitText`, `normalize`, `remove`, and every `create`
  row.
- `cloneNode` and `importNode` return disconnected copies and record nothing; a copy is
  recorded when it is inserted, which is a deviation from a browser, where a copy of a started
  script is not prepared again (deviation 3, question 3). Every connected copy is recorded once.


Deviations this story names, and the row each one is printed on in `selection.txt` (the whole
line, asserted byte for byte by acceptance 21):

1. `createElementNS` with a namespace outside the six, or with a prefix, throws
   NotSupportedError (the tree's element has no prefix and no open namespace;
   `story:html-open-namespace` is the story that can lift it, and then needs an edge to this
   story).
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

A script can also end `limit: steps-per-script` where a browser throws an exception, because a
check that needs a walk runs after the charge of its walk (decision 13): a named allowance, not a
deviation of behaviour. The price of `normalize`, of a removal from the front of a long list and
of the `title` walk is the tree's passes, charged up front: a price and not a deviation either.

## Change

- `crates/script/src/dom/mutation.rs`: the 54 methods of `Mutation`, one call each into the two
  files below. No tree access, no charge (decision 1).
- `crates/script/src/dom/mutation_reads.rs`: the walk structs and `step`, the length reads, the
  viable-sibling scan, the `title` search call. `crates/script/src/dom/mutation_writes.rs`: the
  writes, the charges of decision 3 in the order of the tables, `b16` and `ids` with their unit
  test, `charged_nodes` and `charged_write` (decision 5), the error map and the `record_inserted`
  calls.
- `crates/script/src/dom/mod.rs`: two `mod` lines. `crates/script/src/dom/reads.rs`: `charge`,
  `find`, `Reads` and `html_element` made `pub(super)`: visibility edits only, held by the diff
  of acceptance 24. The selectors story writes `pub(super) enum Reads<'a>`: `find` takes
  `Reads` as that story spells it, and this story takes `main`'s spelling when it merges second.
- `crates/html/src/tree.rs`, each addition with its cost:
  - `parent_or_host` becomes `pub`: constant;
  - `check_insertion(node, parent, child, replaced)`, the public validity-only check: the
    ancestor test, the `Document` checks and the depth check of `insertion_allowed` and
    `admit_depth` with no write, `2d + s` and `5 c_p` for a `Document`;
  - `remove_all(parent)`, the bulk removal of `remove_children` without a `Cell`, for
    `replaceChildren`: `c_p` visits, and the any-element scan before it, `c_p` reads;
  - `work()`, `WorkCount` and the private `Work` of decision 4, raised in each pass of the table
    of decision 3; the doc comment of each write function lists its passes;
  - `shrink_children` in place of `shrink` at `:893`, `reserve_exact(len + 2 m)` before the
    splice at `:1021` and `push_child` for the clone's pushes at `:1247` (decision 15); `shrink`
    stays for attribute lists;
  - no recursive function is changed: none recurses (decision 8).
  Rules that two places hold: the clone's cost (`clone_cost`, private, owns it; the slice's walk
  copies it on purpose because it charges before each node, and the clone row asserts they
  agree), a template as two nodes (`create_element_ns` owns it; the slice's
  `charge_nodes(2, ...)` copies it on purpose, held by the `createElement("template")` row),
  the `IndexSizeError` bound (the tree owns it; the slice only charges the passes), the depth
  (`admit_depth` owns it; the slice only prices the climbs), and `b16` and `ids` (the cost module
  and the slice each use the slice's two functions, not a copy).
- `crates/webidl/src/prelude.rs`, `crates/webidl/src/generate.rs`, regenerating
  `crates/script/src/bindings.rs`: `charge_nodes`, `settle_nodes`, `NodeCharge` and the `nodes`
  handle (`DomState` fields and `DomState::new`, `prelude.rs:104-131`), `DomState::charges`
  removed, `node_charges()` redefined; `charge_node` as a wrapper over `charge_nodes`; `n`
  steps in `to_nodes_or_strings`, and the memory charge and its settle around the copy loop of
  `finish_nodes_or_strings`; the refill charged in the getter template before the slice call
  and the inner bodies of `refresh_collection` and `collection_object` (decision 7).
- `crates/webidl/selection.txt`: the three `deviation:` rows printed above (lines 63, 151, 157).
- `crates/script/src/runner.rs`: the comment at the arm of `Limit::InputBytes | Limit::Steps`
  (the four lines that begin "The limits of the parser's input and of its steps") says no
  member of the slices `reading`, `attributes` and `mutation` reaches either; the arm stays.
- Tests: `crates/script/tests/dom_mutation.rs` (the cases and the source tests; a one-line stub
  exists at `14f8a2a`), `crates/script/tests/dom_mutation_cost.rs` (new, with one `mod` line in
  `tests/main.rs`), `crates/script/tests/dom_reading.rs` (one line, decision 10),
  `crates/script/tests/bindings.rs` (three tests, decision 10; the glue unit test of acceptance
  9), and the unit tests of `crates/html/src/tree.rs` (acceptance 25).
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
  "Each story changes the public numbers it makes false", lines 955-961 at `aa70366`): two units
  built at once both change them, and the one merged second repeats its numbers commit. They
  are files of the Change all the same.

## Acceptance

Every result is read from a named test or a command. Where a line holds on `main` already, it
says it is a guard. Every command that reads the diff names its base:
`git diff $(git merge-base main HEAD)..HEAD`.

1. Three start checks, not results. Each prints what it says or the work stops and the
   implementor reports.
   - The price gate: `git grep -n -F 'units / 64' -- crates/script/src/bindings.rs` prints
     `crates/script/src/bindings.rs:628` at `83e6b8a`, nothing at `14f8a2a` and
     `crates/script/src/bindings.rs:561` at `a14e3b0`. At the implementor's base it must print
     nothing (the reading story's price commit is merged first); if it prints a line, stop and
     report.
   - The reads gate (a): `git grep -n -F 'pending.extend(' -- crates/script/src/dom/reads.rs`
     prints `:363`, `:914` and `:935` at `83e6b8a` and nothing at `14f8a2a`. It must print
     nothing at the implementor's base, and nothing else.
   - The reads gate (b): `git grep -n -p -F -e 'pending.extend(' -e '.to_vec()' -e '.collect'
     -- crates/script/src/dom/reads.rs | grep -E '^[^:]+=[0-9]+=' | grep -v -e 'fn
     element_children(' -e 'fn shallow_equal(' -e 'fn children('` prints the headers of `find`,
     `text_content`, `is_equal` and `whole_text` at `83e6b8a` and nothing at `14f8a2a`. It must
     print nothing at the implementor's base, and nothing else. When both reads gates pass, the
     `title` setter is started; until then it is not, and the story is not complete (decision
     14).
2. `cases_equal_rows`: a case is a table entry of row name, page, script and expected end or
   tree, in `crates/script/tests/dom_mutation.rs`, that the test runs; an entry with no
   expectation is refused by the test, and so is an entry whose only expectation is the end
   `Ran`. The test asserts that the set of row names of the table equals the 50 rows of
   `selection.txt` with the slice `mutation` plus the 4 rows with `setter: mutation` (read from
   the file): 54 names. Every row has at least one entry that asserts behaviour (a
   serialization, a value, or a thrown name and message), including the members whose only
   checks were a name or a limit: the `nodeValue` setter (null becomes the empty string),
   `appendData`, `insertData`, `deleteData`, `replaceData`, `createCDATASection`
   (NotSupportedError), `createDocumentFragment`, `createComment`, `createTextNode`, the
   constructors `Text`, `Comment`, `DocumentFragment`, `ProcessingInstruction`, `adoptNode` (the
   node has no parent after, same identity), `importNode` (a copy, not the same node),
   `insertAdjacentElement`, `insertAdjacentText`, `before`, `after`, `replaceWith` and `remove`
   of `DocumentType`, `Element.remove`, `CharacterData.remove`, `prepend`, `append` and
   `replaceChildren` of `Document` and `DocumentFragment`, and `replaceChild`. Each exception
   case is named by the exception it throws. A case that shares a page with another is the same
   table entry list over one parse. Two more entries:
   - the brand check, two examples: `Node.prototype.appendChild.call({}, x)` and
     `CharacterData.prototype.appendData.call(document, "a")` each throw a `TypeError`, and the
     tree and `totals()` are as before the call;
   - `importNode(script, true)` with a `script` element, its copy inserted once: one record,
     `not run: inserted`, and none at the import.
3. `the_files_hold_their_parts`: a source test over `include_str!` asserts that `mutation.rs`
   holds none of `tree()`, `tree_mut()`, `charge(`, `charge_steps`, `charge_stored`,
   `settle_stored`, `charge_nodes`, `settle_nodes`, `charge_node`; `mutation_reads.rs` holds
   none of `tree_mut()`, `charge_stored`, `settle_stored`, `charge_nodes`, `settle_nodes`,
   `charge_node`; `mutation_writes.rs` holds no `tree()` and none of `put_attribute`,
   `set_attribute`, `set_attribute_ns`, `remove_attribute`, `remove_attribute_ns`,
   `toggle_attribute` (so `Limit::Attributes` is unreachable); no function of the three files
   calls itself; and no walk of `mutation_reads.rs` holds `.to_vec()` or a `children(...)` call
   that is collected. In `mutation_writes.rs`, `charge_stored(`, `settle_stored(`,
   `charge_nodes(` and `settle_nodes(` occur only inside `charged_write` and `charged_nodes`,
   and in each of the two the text between the charge statement and the first settle holds no
   `?`. Red on `main`: the files do not exist.
4. `no_write_reaches_input_bytes_or_steps`: a source test over `crates/html/src/tree.rs` shows
   `Limit::InputBytes` and `Limit::Steps` occur nowhere in it (a guard, true at `83e6b8a`);
   it also shows that `clone_node` still answers `NotSupported` for a `Document` and admits the
   whole clone before its first node (`:1204` and `:1207 @ 83e6b8a`, guards: deviation 2 and
   the node charge rest on them). The same test asserts that `crates/script/src/runner.rs`
   holds the phrase "no member of the slices `reading`, `attributes` and `mutation`" above the
   arm, and `git diff $(git merge-base main HEAD)..HEAD -- crates/script/src/runner.rs` shows
   the four comment lines above the arm that begins "The limits of the parser's input and of its
   steps" changed and the arm `Limit::InputBytes | Limit::Steps => ScriptEnd::Limit(
   Reason::WallClock)` unchanged (lines 715-718 and 719 at `83e6b8a`, 719 not in the diff). The
   phrase and the diff are red on `main`: the comment is as it was.
5. `title_written`: `surf_script::run::parse_with_scripts` over
   `<title>Before</title><script>document.title = "Written by script"</script>` answers a tree
   whose title text is `Written by script`, and its list of ends holds one end, `Ran`. It waits
   for the reads gates of acceptance 1.
6. `append_keeps_the_order`: the page
   `<body><div id="a"></div><script>document.getElementById("a").appendChild(
   document.createElement("p"))</script><i>after</i></body>` leaves the tree's `body` children
   in the order `div`, `script`, `i`; the only child of `div` is `p`; the text of `i` is
   `after`.
7. `limit_*` tests, one for each row of the limits table in "What each member charges" (15
   tests): for each member of the family, one parse; the script's end is the limit's word, also
   inside `try { } catch { }`, parsing goes on and the rest of the page is in the tree, and the
   tree after is as decision 6 says: for each member the test also parses the page with the
   script without the refused call (with the convert made by hand for two items or more, as
   decision 6 gives it: `f = createDocumentFragment(); f.appendChild(a); f.appendChild(b)`,
   which makes 1 node and raises `writes()` by 2, by 4 when `a` and `b` had parents) and asserts
   equal serialization, node count and `writes()`. The node count the tree gained equals
   `totals().nodes` in every case that makes nodes. The `title` setter has no depth test, since
   it cannot reach `limit: depth` (the `Document.title` member says why); a guard test,
   `title_at_the_smallest_depth`, parses `<head><script>document.title = "x"</script>` at
   `max_depth` 3 and asserts the script ran to its end and the `title` is in the tree.
   `limit_memory_default` runs at the default parser limits under a memory limit of 4 MiB, one
   parse for each of five loops, each ends `limit: memory` and no parser limit:
   - strings: `var s = Array(100001).join("x"); for (;;) document.documentElement.append(s);`;
   - create: `for (;;) document.createTextNode("x");`;
   - clone: `var a = document.createElement("i"); for (;;) a.cloneNode(true);`;
   - split: `var t = document.createTextNode("xy"); for (;;) t.splitText(1);`;
   - character data: `var t = document.createTextNode("x"), s = Array(1048577).join("y");
     for (;;) t.appendData(s);`.
8. `memory_before_nodes_at_defaults`: a loop that creates and removes an element ends
   `limit: memory` at the session's limits after at most 164,482 nodes (408 bytes each; the
   wrapper's 423 bytes make it about 80,756), and a removed node's charge is not refunded
   (`totals().nodes` and `totals().bytes` after one create-and-remove equal those after one
   create). The steps of a turn are about 20 by the tables (arithmetic: 164,482 x 20 =
   3,289,640, under 10,000,000); the build measures them, and if a turn costs more than 60
   steps the case raises `steps_per_entry` to 50,000,000 as the cost module does.
9. `charge_nodes_is_bulk`: through the real slice, `append` of 1,000 strings of 3 UTF-16 units
   charges `charge_nodes(1,001, 9,000)` = 426,408 bytes (arithmetic: 1,001 x 408 + 2 x 9,000)
   and settles to 414,408 in `totals().bytes` (arithmetic: 1,001 x 408 + 2 x 3,000; the settle
   gives back 12,000), and `totals().nodes` changes by 1,001 (the strings and the fragment);
   the test reads 426,408 from the handle's size during the call (a spy `Mutation` that reads
   `totals()` in its method) and 414,408 after. `charge_node(s)` equals `charge_nodes(1, s)` (a
   glue unit test in `crates/script/tests/bindings.rs`); a `before()` with no parent and 1,000
   strings leaves `totals().nodes` and `totals().bytes` as before the call; after the first
   `createTextNode`, `node_charges()` is 1 and after 10,000 more it is still 1 (the count of node
   handles, redefined by this story). Red on `main`: no `charge_nodes`.
10. `variadic_strings_are_charged_before_any_node`: `append` of 200,000 empty strings ends
    `limit: memory` with `totals().nodes` 0 and the tree's node count unchanged, because
    `charge_nodes(200,001, 0)` is 81,600,408 bytes (arithmetic: 200,001 x 408), over the
    67,108,864 of `memory_bytes`. The implementor checks that the engine accepts that many
    arguments through a spread, and uses `Function.prototype.apply` when it does not. Red on
    `main` because `append` throws `not supported` (the slice is absent); the step for each item
    has its row in the cost module (acceptance 18).
    `a_loop_of_two_item_appends_ends_in_memory`: `var a = document.createElement("i"), b =
    a.cloneNode(false); for (;;) document.documentElement.append(a, b);` at the default limits
    under `steps_per_entry` 50,000,000 ends `limit: memory`, not `limit: nodes`, and the node
    count the tree gained equals `totals().nodes`: the temporary fragment of each call is a node
    and is charged.
11. `the_counter_per_write`: for each line of the table "Which tree write raises the counter"
    (member; tree function; raises when), one script makes the write and the test reads
    `Tree::writes()` through its tree handle before and after, and asserts raises or not as the
    table says; `replaceChild(x, x)` moves no counter; a refused write does not raise (a guard
    on `main`).
12. `a_kept_list_follows_a_write`: a kept `childNodes` (the same object) and a kept
    `getElementsByTagName` result are read before and after `appendChild`, `removeChild`,
    `textContent =` and `normalize`: the `childNodes` object is the same and shows the change
    after the write; the snapshot shows it only on a fresh call (reading decisions 17 and 18).
13. `a_refill_charges_before_it_copies`: with the spy `reading` slice, a kept `childNodes` of
    100,000 children (parsed, not appended) and a `steps_per_entry` of 1,000,000: the first read
    fills it (`8 x 100,000` = 800,000 steps, charged before the copy), an `appendChild` and a
    second read ask for `8 x (100,000 + 100,001)` steps (arithmetic: 1,600,008), the refill ends
    the script `limit: steps-per-script`, and the spy counted no call of `node_child_nodes`
    during that refill. Red on `main`: the spy counts one call. The test of the record of a cut
    refill, `adversary_dom_reading_3.rs:149-220`, stays green and is not edited.
14. `the_records_of_inserted_scripts`: the page made of
    `<script>var s = document.createElement("script"); s.textContent = "1";`, then
    `document.documentElement.appendChild(s);</script>`, then `<script>1</script>`
    records `ran`, `not run: inserted`, `ran` in order and the element is in the tree, not run;
    two clones of it (`cloneNode(true)`), each inserted once, record two; a `moveBefore` records
    none; a script inside a `template` inserted records none; the parser's own scripts get no
    second record (through `parse_with_scripts`).
15. `every_exception_in_its_order`: one case for each pair of exceptions in decision 13 where a
    call can throw two (for example `insertBefore` of an element into its own descendant, with
    a `child` that is not a child of the target: HierarchyRequestError before NotFoundError),
    asserted by name and message; `createElementNS` with an invalid prefix and a bad namespace
    gives InvalidCharacterError, as the standard orders them; `createElementNS` with a colon as
    the first or as a later character of the name gives InvalidCharacterError or, for a valid
    prefix, the deviation's NotSupportedError, and never an element.
16. `an_exception_after_a_charge`: for each of five families, a pair of cases on one page. The
    "limit one below" of a pair is defined by the cost row of the family: `steps_without` is the
    steps of the script with the call removed, `D` the difference the row measured (printed by
    the row); the lowered limit is `steps_without + D - 1`.
    - with the default step limit the script throws the standard's exception: `a.appendChild(
      ancestor)` over a subtree of 100,000 nodes (HierarchyRequestError by the ancestor test);
      `insertBefore(x, notAChild)` with `x` a subtree of 100,000 nodes (NotFoundError);
      `document.appendChild(second)` on a `Document` of 5,001 children (HierarchyRequestError
      by the `Document` checks); `t.splitText(L + 1)` on a text of 4 MiB (IndexSizeError);
      `createElementNS("x", "a:b")` with a name of 1 MiB (the standard's error of the name);
    - with `steps_per_entry` at `steps_without + D - 1` the same script ends
      `limit: steps-per-script`, the named allowance of decision 13, and at `steps_without + D`
      it throws the exception. The subtree of 100,000 nodes comes from the page: a script cannot
      make it (decision 5, "Memory comes before nodes").
17. `no_recursion`: on a thread built with a stack of 256 KiB, named in the test, over a chain of
    20,000 elements under a raised `max_depth`: the tree functions `clone_node(root, true)`,
    `normalize`, `set_text_content`, `remove_node` of the chain's root and a valid
    `insert_before`, and the walks `Subtree`, `Climb` and `Siblings` of the slice over the
    chain, run without a stack overflow. A guard in the same test: a trivial call
    (`Tree::children(root)`) runs on that named thread, and a probe that recurses to half the
    stack (frame size and depth printed, arithmetic) completes on it, so the stack is the one
    asked for. No claim is made about the runner's thread.
18. `dom_mutation_cost.rs` (new): worst shapes, each row one script and no row over 5 s, with
    the steps asserted exactly and never a time.
    - The method. A row measures the call's own steps as the steps of the script with the call
      minus the steps of the same script without it (`Runner::steps()` before and after each, as
      `dom_reading_cost.rs:146` does), with the call line holding only variables, so that no
      argument is built on it. For each family there is a baseline shape in which every priced
      term is the smallest it can be. The row asserts `with - base == F(with) - F(base)`, `F`
      the family's formula of "What each member charges" evaluated on the two shapes: exact,
      with no floor, ceiling or band. The glue's copy is an integer division (`units x 2 / 16`
      at `14f8a2a`), so `g(u) = floor(u / 8)` is exact for any length; the rows use lengths that
      are a multiple of 8 UTF-16 units or under 8, except the namespace of 28 units (`g` 3). The
      row prints its name, `with`, `base` (the formula's values on the two shapes, the call's
      own steps) and the expected difference. A row whose call writes the tree also asserts that
      the steps it charged are at least the converted `Tree::work()` raised by the call
      (decision 4). The
      rows run under `steps_per_entry` 50,000,000 (as `dom_reading_cost.rs:23`) unless the row
      says it ends. Rows that share a page are one `#[test]`. A row over 5 s is named in the
      report with its time.
    - The rows, in the order of the tables (each figure is arithmetic from the formulas, as
      `with` / `base` = the difference that the row asserts). A shape written once serves the
      next rows that say "the same".
    - Creates:
      - create text: `createTextNode` of 1 MiB (1,048,576 units): 196,610 / 2 (the empty
        string) = 196,608 (131,072 + 65,536).
      - create element: `createElement` of a name of 1 MiB: 327,682 / 5 (the name `a`) =
        327,677.
      - create template: `createElement("template")` against `createElement("templatx")`:
        7 / 6 = 1 (the second node).
      - `createElementNS`: the XHTML namespace (28 units, `g` 3) and a name of 1 MiB:
        262,150 / 8 (the name `a`) = 262,142.
      - `createElementNS`, refused: a prefixed name of 1 MiB: 196,613 / 6 (`a:b`) = 196,607.
      - `createProcessingInstruction`: target `t`, data of 1 MiB: 262,148 / 4 = 262,144.
      - `insertAdjacentText`: `beforeend` with 1 MiB: 196,624 / 16 = 196,608.
    - Inserts:
      - depth: 100 turns of `appendChild` then `removeChild` of one element made once, under an
        element at depth 511 (`d` = 512; 1,547 steps a turn) against under `body` (`d` = 3;
        20 a turn): 154,700 / 2,000 = 152,700.
      - fragment: a fragment of 20,000 elements appended to `body` (`d` = 3), against a
        fragment of one: 100,012 / 17 = 99,995.
      - moved out: `appendChild` of the first child of a parent of 20,000 children to a small
        parent, against a parent of one child: 40,016 / 18 = 39,998.
      - `Document` parent: `appendChild` of a comment to a `Document` of 5,001 children,
        against one of one child: 25,014 / 14 = 25,000.
      - fragment into a `Document`: a fragment of 1,000 comments into a `Document` of one
        child, against a fragment of one: 7,011 / 18 = 6,993.
      - reference child: `insertBefore` of a fresh element before the child at index 10,000
        of 20,000, against index 0 of 2: 20,020 / 21 = 19,999.
      - child is the node: `insertBefore(x, x)` with `x` at index 10,000 of 20,000, against
        index 1 of 3: 50,023 / 29 = 49,994.
    - Replace and move:
      - `replaceChild`: a parentless node for the first of 2,000 children (`d` = 4), against 1
        child: 3,023 / 24 = 2,999.
      - `replaceChild`, same parent: the last of 2,000 for the first, against 3 children:
        4,026 / 32 = 3,994.
      - `replaceChild`, reference is the node: the second of 2,000 for the first, against 3
        children: 7,024 / 34 = 6,990.
      - `replaceChild` into a `Document` of 5,001 children, against one: 32,520 / 20 = 32,500.
      - `moveBefore`: the last of 2,000 children to the front (`d` = 4, `d_n` = 5), against 2
        children: 1,041 / 42 = 999.
      - `moveBefore` of an element into a `Document` of 5,001 children, against one: 25,027 /
        27 = 25,000.
      - `adoptNode`: the first of 20,000 children, against a parent of one: 40,003 / 5 =
        39,998.
    - `ChildNode` and `replaceChildren` (a node at index 40,000 of 80,000, `d` = 4, against
      index 0 of 2):
      - `before`: 80,023 / 24 = 79,999. `after`: 80,024 / 25 = 79,999.
      - `insertAdjacentElement`, `afterend`: 80,021 / 22 = 79,999.
      - `replaceWith`: 140,026 / 29 = 139,997.
      - `replaceChildren`: one element for 20,000 children (`d` = 4), against none:
        40,028 / 28 = 40,000. On a `Document` of 5,001 children, against one: 35,020 / 20 =
        35,000.
    - Variadic:
      - `append` of 10,000 strings of 1 byte and 10,000 elements to `body` (`n` = 20,000,
        `k` = 10,000), against one string and one element: 300,013 / 43 = 299,970. The parts
        the row prints: the glue's step 1, `n` 20,000, `k` 10,000, the sum of `b16` of the
        strings 10,000, the fragment 1, the items 160,000 (8 each), the final insert 100,011.
      - a repeated item: the same script with its last two items repeated (an item whose parent
        is the fragment, `c_f` = 20,000), against the script without them: a difference of
        20,028 (arithmetic: 2 for `n`, 16 for two items, 2 x 10,000 for `ids(c_f)` twice, 2 x 5
        for `2 k_o + 1` with `k_o` = 2).
      - an item with an old parent: `append(a, b)` with `a` the first of 20,000 children,
        against a parent of one child: 40,001 / 3 = 39,998.
    - Clone and template:
      - deep clone: `cloneNode(true)` of an `i` with 9,999 `i` children (`max_nodes` 25,000,
        `b + N` = 20,000): 41,250 / 9 (a childless `i`) = 41,241. The parts: the own walk
        10,001, the cost walk and the copy loop 20,000, the bytes 1,250, the share of the
        pushes 9,999. The row asserts that `s'` and `b` equal the tree's change in nodes and in
        stored bytes.
      - clone with attributes (illustrative; the row prints the `b` it reads from the tree): an
        element with 1,024 attributes of 5-byte names and 1,000-byte values (`b` 1,029,126,
        `N` 3, `b16` 64,321), against a bare `div`: 66,629 / 5 = 66,624.
      - template clone: a `template` of 10,000 contents, deep: 41,259 / 9 = 41,250.
      - template insert: a `template` of 100,000 nodes appended to the last child of `body`
        (`d` = 4), against an empty `template`: 200,023 / 23 = 200,000.
    - `normalize`:
      - one parent of 2,000 adjacent texts of 1 byte, against two: 4,016,381 / 29 =
        4,016,352 (1 + 2,001 + 12,006 + 375 + 4,001,998).
      - 8 parents of 1,000 adjacent texts, against two texts: 8,065,548 / 29 = 8,065,519
        (8,009 + 48,054 + 1,500 + 8,007,984 + 1).
      - one text of 4 MiB, against one of 1 byte: 786,447 / 18 = 786,429.
    - Other writers:
      - `textContent =` a string of 1 MiB on a node of 20,000 children, against `""` on a
        childless one: 236,612 / 5 = 236,607.
      - `splitText`, long: at `L - 1` of a text of 4 MiB, no parent, against 4 bytes: 786,434 /
        5 = 786,429.
      - `splitText`, parent: a text of 4 bytes at index 40,000 of 80,000 children, against
        index 0 of 2: 100,010 / 11 = 99,999.
      - `data =`: a text of 4 MiB set to 4 bytes, against a text of 4 bytes (5 passes):
        1,310,722 / 7 = 1,310,715. `nodeValue =` and `appendData` have the same shape and the
        same figures.
      - `replaceData` on that text (4 passes): 1,048,578 / 6 = 1,048,572. `insertData`: the
        same. `deleteData(0, 1)`: 1,048,577 / 5 = 1,048,572.
      - `title`: `document.title = "x"` with a `head` of 100,000 `meta` elements before a
        `title`, against one `meta`: 100,008 / 9 = 99,999. It waits for the reads gates of
        acceptance 1.
      - refill: a kept `childNodes` read after each of 1,000 appends, from no children, against
        10 appends: 8,001,000 / 810 = 8,000,190.
    - Removals: `p` has 2,000 children from the page. From the front,
      `removeChild(p.firstChild)` 2,000 times: 4,008,000 (arithmetic: the sum of `2 c + 3` for
      `c` from 1 to 2,000); from the back, `removeChild(p.lastChild)` 2,000 times: 10,000 (5 a
      turn); a difference of 3,998,000.
    - Rows that end the script, under the default `steps_per_entry` of 10,000,000:
      - 5,000 removals from the front of a list of 5,000 would cost 25,020,000 (arithmetic) and
        end `limit: steps-per-script` at about the 1,127th removal (10,004,379 by `r (2 n - r
        + 4)`);
      - a list of 3,161 children removed whole from the front ends `limit: steps-per-script`
        (arithmetic: 3,161^2 + 4 x 3,161 = 10,004,565), and a list of 3,160 does not;
      - `normalize` over 20 parents of 1,000 adjacent texts ends `limit: steps-per-script`: the
        removals alone cost 20,019,960 (arithmetic: 20 x 999 x 1,002);
      - a second deep clone under `max_nodes` 25,000 ends `limit: nodes`.
    - Throw rows (the call refuses, its walks are paid): the base is the same throw on a small
      shape, and the pair of acceptance 16 holds the limit one below the call's own charge.
      - ancestor: `a.appendChild(ancestor)`, `d` = 5, a subtree of 100,000 nodes from the page,
        against a subtree of 1: 200,022 / 26 = 199,996.
      - NotFound: `insertBefore(x, notAChild)`, `x` of 100,000 nodes, against 1: 200,019 / 23 =
        199,996.
      - `IndexSizeError`: `splitText(L + 1)` on a text of 4 MiB: 262,145 against 2 for a text of
        1 byte: 262,143.
      - `Document` with a second element: the figure of the `Document` row, 25,000.
      - `createElementNS` with a prefix: the refused row above.
    - The cycle row (decision 15). A page whose `<div id=p>` has 100,000 `<i>` children from
      the parser, and one element made once: `x = document.createElement("i")`; then about
      70,000 turns of `p.appendChild(x); p.removeChild(x); p.removeChild(p.lastChild)`. A turn
      costs 28 steps (arithmetic: 18 for the append, 5 and 5 for the removals), so about
      1,960,000 steps, and the script ends `Ran`. The parser's list has a capacity of 131,072
      (acceptance 25 asserts it). The list shrinks once, at length 32,767 (4 x 32,767 = 131,068
      is under 131,072), after 67,233 net removals, and copies 32,767 ids, 16,384 steps; the
      next shrink is at 16,383 or fewer and is not reached. The row asserts that the steps are
      at least the converted work and that they are under 2,000,000 (arithmetic: 70,000 x 28 =
      1,960,000, and the one shrink 16,384 is inside the share).
    - Terms with no row, named and with the reason: the glue's step `G` and the node's single
      step in creates, `insertAdjacentText` and `splitText` (constants that cancel in every
      difference); the constant 3 of `createElementNS` (the step, the match, the node: the same);
      the scan of the viable sibling beyond the first one (at most `n + 1` steps, covered by the
      items' own charges); `Document.title` until the reads gates pass.
    - For each charge a red row: the implementor removes each charge of `mutation_writes.rs` by
      hand (the mutation tool is not on the host), runs the rows, and the report names for each
      the line changed, the row that went red, and shows `git diff --quiet` after putting the
      line back. A charge with no red row is a defect of the table and gets a row, except the
      terms named above.
19. `other_slices_still_throw` is green with its one changed line, `git diff
    $(git merge-base main HEAD)..HEAD -- crates/script/tests/dom_reading.rs` is that line only,
    and the three tests of `tests/bindings.rs` of decision 10 are green with a member that still
    throws.
20. `slot task webidl-check` and `ESS_TOOLCHAIN=0.97.4 slot --timeout 30m task webidl-slow`
    exit 0 on the regenerated `bindings.rs`; the times of `webidl-slow` before and after are in
    the report (and the job is not started again unchanged if it ends at its limit).
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
      the report with its time, and the step prints its ten slowest tests;
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
    acceptance 22; the front's scope list does not hold the public-number files). The edit of
    `crates/script/src/dom/reads.rs` is visibility only: in zsh or bash,
    ```
    b=$(git merge-base main HEAD); f=crates/script/src/dom/reads.rs
    diff <(git diff -U0 "$b"..HEAD -- $f | sed -n 's/^-\([^-]\)/\1/p' | tr -d ' \n,') \
         <(git diff -U0 "$b"..HEAD -- $f | sed -n 's/^+\([^+]\)/\1/p' \
           | sed -e 's/pub(super)//g' -e "s/<'a>//g" -e "s/Reads<'_>/Reads/g" \
                 -e "s/&'a str/\&'static str/g" | tr -d ' \n,')
    ```
    prints nothing (the pattern of the selectors story's acceptance 9, so that its lifetime and
    string edits do not count as this story's).
25. `crates/html/src/tree.rs` unit tests (the capacity of a list is private, so they live in
    the file):
    - `work_sees_the_passes`: each write function raises `work()` for the passes its doc comment
      lists, a refused write raises none, `Tree` stays `Sync` (a function that takes `&Tree` on
      another thread compiles), a clone of a tree copies the counters and two trees compare equal
      whatever their work.
    - `child_lists_keep_their_capacity`: from a list of 4,000 with capacity equal to length
      (`shrink_to_fit`), 1,000 cycles of append, remove, remove change the capacity at most 4
      times (the new rule: once; the rule of `83e6b8a`: about 2,000); after a growth by `m` the
      capacity is `2 (len + m)`; a list shrinks only when its capacity is over `max(4 l, 4)` and
      then to `max(2 l, 4)`; a bulk removal leaves the capacity alone.
    - `attribute_lists_still_shrink_at_twice`: a guard that `shrink` of `drop_attribute` is as
      it was.
    - `a_parsed_list_has_the_capacity_the_cycle_row_assumes`: a parsed list of 100,000 children
      has capacity 131,072; if it does not, the numbers of the cycle row (acceptance 18) are
      redone, and the test says so.

## Out of scope

Attributes and selectors (their slices); the parse that runs scripts and its per-document limits
(`story:scripts-parse-run`); a session (`story:scripts-document-port`); an open namespace for an
element or an attribute (`story:html-open-namespace`); ranges, tree walkers and node iterators
(`throws`); `createRange`, the `Document` constructor and the `EventTarget` constructor; a step
budget inside the tree's write functions; a refund of a removed node's charge; a live
collection (the snapshot deviation of the reading story stands); moving the glue's single
argument copy before the charge (`Copies::copy`; no story holds it yet); the end of a run on an
outer `Err(HostError)` (`DomEnd::Fault`, which the attributes story adds and owns: this story
passes it on).

## Placement

After `story:html-script-hook`, `story:scripts-dom-reading` (its price commit and its fix round
first, acceptance 1), `story:scripts-parse-run` and `story:scripts-dom-attributes` (a
`depends_on` edge: that story writes `charge_to`, `charge_stored`, `settle_stored`, the
variant `DomEnd::Fault`, the accessors, and the reads and owners below, and this story calls
them by the names it has there, so it is built on a base that holds them); beside
`story:scripts-dom-selectors`. The code of the slice is its own three files; the files below are
shared, and for each the rule is that the story that merges later merges `main` first and takes
the shared lines as they are.

- `crates/script/src/bindings.rs` (generated) and `crates/webidl/src/prelude.rs`:
  `bindings.rs` is regenerated by the second story to merge; `prelude.rs` is source and is
  merged by hand in these regions: the `DomState` fields and `DomState::new` (`:104-131`:
  the attributes story adds `stored`, `live`, `charged`; `story:scripts-parse-run` adds
  `swap_tree`; this story adds `nodes` and removes `charges`); `charge_node` (`:219`: the
  attributes story makes it and `charge_stored` share the private body `charge_to`;
  `charge_nodes` goes through that body); `to_nodes_or_strings` (`:1036`: the draft
  `story:scripts-utf16-length` changes `string_length` and the glue's count);
  `refresh_collection` and `refill` (`:1321`, `:1367`: the draft
  `story:scripts-dom-collection-names` rewrites them). The two drafts have no edge to this
  story. `slot task webidl-check` and `slot task webidl-slow` hold the generated file.
- `crates/webidl/src/generate.rs`: this story edits `same_object_getter`; the attributes story
  edits the generator's `strings_value`; different functions.
- `crates/webidl/selection.txt`: `story:scripts-dom-attributes` (the note of row 96),
  `story:scripts-dom-selectors` (nine rows); this story rows 63, 151 and 157. Rows 125, 174
  and 230 (`moveBefore`) are next to the selectors story's rows 126, 175 and 231 and are not
  edited by either.
- `crates/html/src/tree.rs`: `story:scripts-dom-attributes` adds three reads
  (`attribute_position`, `attribute_position_ns`, `attribute_names`), three owners
  (`Namespace::from_url`, `Tree::namespace_error`, `Attribute::qualified_len`) and
  `Attribute::stored_bytes`; the reading story edits the write counter `writes()` (it merges
  first); this story adds `work()`, `shrink_children`, `push_child`, `check_insertion` and
  `remove_all` and makes `parent_or_host` public, in other regions. A draft that may change
  child-list writers, `story:scripts-dom-sibling-positions`, needs a `depends_on` edge to this
  story: this story's formulas and the work counter stay safe over a faster list, and its
  acceptance 18 holds front removals quadratic where that draft's acceptance 2 holds them
  linear. `story:html-open-namespace` (a draft: changes `Namespace` and `create_element_ns`,
  and leaves deviation 1 open, its decision 3) names neither `mutation_writes.rs` nor row 151;
  it depends on the attributes story only, and needs an edge to this story if it lifts the
  deviation. `story:html-parser-hook-cost` (a draft that touches the tree's parser-facing
  functions).
- `crates/script/src/dom/reads.rs`: the token edits of three stories are different spellings:
  `charge` is made `pub(super)` by the attributes story, the selectors story and this story;
  the selectors story writes `pub(super) enum Reads<'a>` and widens the scan's name to `&str`,
  and this story makes `Reads` `pub(super)` without the lifetime; so the merger takes the
  selectors story's `Reads<'a>` and `find` is written against it (acceptance 24's diff counts
  the lifetime as visibility). The reading unit's fix round edits `find` (`:349-366 @ 83e6b8a`)
  and this story starts after it. This story makes `find` and `html_element` `pub(super)`.
- `crates/script/src/dom/mod.rs`, `crates/script/tests/main.rs`: `story:scripts-dom-attributes`
  and `story:scripts-dom-selectors`, each two `mod` lines or one, in different places; this
  story's `mod dom_mutation_cost;` is next to the attributes story's `mod dom_attributes_cost;`:
  keep both.
- `crates/script/tests/dom_reading.rs`: `other_slices_still_throw` holds three adjacent lines,
  2424 (this story), 2425 (the attributes story), 2426 (the selectors story): keep all three.
- `crates/script/tests/bindings.rs`: `story:scripts-dom-attributes` (the `Writes` variant);
  the swell pin story (`dom_limit_drops_the_jobs` and a `then` variant of `limit_end`); this
  story changes three tests' members and adds a glue unit test, and no variant.
- `crates/script/src/runner.rs`: the swell pin story edits `document_end`, `evaluate`, the
  macro and `error_kind`; `story:scripts-parse-run` edits `evaluate` and `script_end` (which
  exists only at that story's base, `f463e24:691`); this story edits the comment inside
  `document_end` above the `Limit::InputBytes | Limit::Steps` arm only (`:694`, the comment at
  `:715-718 @ 83e6b8a`), cited by its text at the implementor's base. No edge is needed:
  parse-run depends on the swell pin.
- `docs/design/scripts.md`: `story:scripts-parse-run` edits Q2 (`:183`), Q4 (`:405`, `:444`), Q5
  (`:498`, `:503`), Q11 (`:1016`) and lines `:10` and `:479`, not Q1 (ordered first by the edge);
  `story:scripts-dom-selectors` adds a row to the table of Q4 after the row `parser: attributes
  per element` (two lines from this story's `parser: steps` row: no edge, a hand merge keeping
  both); the swell pin story edits Q4 (`:255-265`, `:302`, `:303`, `:307`, `:320`,
  `:338-341`) and Q11 (`:902-909`, `:908`, `:917`, `:925`, `:963`, `:1079`); the attributes
  story edits `:322-332` and `:902-904`, which this story edits too (prose written twice; the
  edge puts this story second). The row of Q1 this story edits, "a script element a script
  created and inserted into the document", is cited by its text, and the node charge is in the
  paragraph of Q4 at `:322-332`.
- `AGENTS.md` and the public surfaces of acceptance 22: the entry of `crates/script`
  (`AGENTS.md:365-369 @ a14e3b0`) is one paragraph that six stories edit (the reading story,
  the swell pin story, parse-run, attributes, selectors and this one), merged by hand, each
  keeping the others' sentences; the public numbers are shared with the same stories and the
  later merger takes its numbers from its own log.
- Other plan files change with this story: the epic (this story's block, and its rule on the
  public-number files, which omits the swell pin story and the reading unit as writers),
  `story:html-open-namespace` (decision 3), `story:scripts-dom-reading` (a sentence in decision
  20 that owns the walk property of decision 14), `story:scripts-dom-attributes` (its line on
  `Limit::InputBytes`, its cites of this story's lines, and the meaning of `node_charges()`) and
  `story:scripts-parse-run` (its cites of this story's lines).

## Why `epic:scripts-in-a-page`

Writes to the tree, without which no script changes what surf shows (example 1 of the epic). The
document port's example appends an element built by a script, and the hostile-pages story builds
a deep tree by `appendChild` and expects `limit: depth`; both need the writes, each priced so
that a hostile loop of them ends inside the step limit.
