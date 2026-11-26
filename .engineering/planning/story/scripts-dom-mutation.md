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
  path: crates/html/src/limits.rs
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
revision: 51
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:13Z", actor: "human:operator", revision: 15}
---
## Context

The epic's ninth candidate (lines 472-475) and its slice "Tree mutation, 29" (lines 736-741),
read at plan commit `9a88a16`: the mutating members of `Node`, `Element`, `Document` and of the
interfaces they return, with the DOM Standard's exceptions, held to the parser's limits.

Read for this text:

- The plan at `9a88a16` (branch `plan/round-2`): the epic, the parse-run story at `f877bfd`,
  the selectors story at `77aaa6a`; and at `dfec030` the attributes story at `72f6059` (its
  eighth revision: `charge_to`, `charged_write`, `settle_stored`, `DomEnd::Fault` and the
  accessors are final there, and its decision on the stored-bytes handle names the contract
  this story calls). Two stories added since `aa70366` (`agent-tasks-left-process-wait`,
  `tests-without-wall-time-bounds`) touch nothing here.
- The code at `83e6b8a`, the reading unit as this story was first written against; at
  `14f8a2a`, the tip of that unit's branch with its fix round (`cd43273`, "Walk the tree without
  copying a child list"); and at `50a4239`, its new tip with the sixth fix round (not on
  `main`): a String argument is charged before its copy, a compare charges both sides, and a
  source test holds that no walk of `reads.rs` copies a child list. `main` at `a14e3b0` for the
  public strings and the vendored DOM Standard (`dom.html`). The engine, read at `swell`
  `0.8.0` (commit `0c18271`, the tag the runner pins at `83e6b8a`) and the same files and
  functions at `0.10.0` (`b9d3d46`, the pin of `story:swell-0-9-0`): `host/host_bytes.rs` is
  the same file at both; `Cx::charge_host_bytes` and `resize_host_bytes` are the same code
  (`host/mod.rs:577` and `:585` at `0.8.0`, `:620` and `:628` at `0.10.0`), and so are
  `Agent::running` and `Agent::end_entry` (`host/agent.rs:771-777` and `:559-565` at `0.8.0`,
  `:952-958` and `:606-612` at `0.10.0`).
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
  is an Element". Rows 63 (`Node cloneNode`), 157 (`Document importNode`) and 186, 187, 188
  and 197 (`insertData`, `deleteData`, `replaceData`, `splitText`) carry no note.

What the glue (`crates/webidl/src/prelude.rs`, generating `bindings.rs`) does for every member
at `83e6b8a`: one step, the brand check, the copy of each `String` argument, the slice's method,
then `finish`. A method answers `DomResult`; `DomEnd::Limit` ends the script with a parser limit
(`bindings.rs:120`). `end_with_limit` (`bindings.rs:512`) turns a `Limit` into the end. The node
charge `charge_node` (`prelude.rs:219`, `bindings.rs:284`) takes 3 x `NODE_BYTES` (136, so 408
bytes) plus twice the stored bytes through `charge_host_bytes`, pushes one `HostBytes` into
`DomState::charges` (`prelude.rs:112`) for each call, counts one node in `totals.nodes` and never
refunds. A variadic argument arrives as `Vec<NodeOrString>` (`bindings.rs:1116`); the conversion
`to_nodes_or_strings` (`bindings.rs:1135`, `prelude.rs:1036`) charges no step for an item.

The glue's copy of one `String` argument (`Copies::copy`), as this story builds on it: the
reading unit's sixth fix round (`50a4239`) charges the argument before the copy.
`Copies::copy` reads the length with `string_length(cx, string)`, which copies nothing, charges
`2 x units / 16` steps (`STRING_BYTES_A_STEP` = 16), and only then calls `to_utf16`
(`bindings.rs:624-631`, generated from `prelude.rs:554-561`; the unit's source test
`a_string_argument_is_charged_before_its_copy`, `dom_reading_cost.rs:669`). The amount is the
same at `14f8a2a`, where the copy came first (`bindings.rs:630`, `units` the units copied), and
`83e6b8a` charged `units / 64` with `units` the bytes of the copied text (`bindings.rs:628`).
With `u` the UTF-16 units of one argument that is `g(u) = floor(u / 8)` steps at `14f8a2a` and
`50a4239`. The text below uses `g`; neither of the two earlier commits is the implementor's
base, and acceptance 1 stops the work when the base lacks the price commit or the sixth fix
round.

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
  and by `drop_attribute` (`:1532`) for an attribute list. A bulk removal takes the list:
  `remove_children` (`:539-545`) does `mem::take` and leaves capacity 0, and `set_text_content`
  calls it (`:1458`). A text value never keeps spare room after a write that shortens it:
  `split_text` makes the head with `to_string` (`:1372`) and replaces the node's data with it,
  so the old string is freed and the head's capacity is its length; `replace_data` builds the
  new data with `String::with_capacity` of its exact size (`:1401-1404`) and replaces the old.
  So a loop of `splitText(1)` on a 1 MiB text holds no room beyond the tail's own bytes, and
  nothing in the Change makes a value give room back. The callers of `detach` are
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
   `b16(17) = 2`, `ids(2) = 1`, `ids(3) = 2` and `size_of::<NodeId>() == 8`. They are private:
   the cost module is an integration test and cannot name them, so it imports
   `surf_script::bindings::STRING_BYTES_A_STEP` (`pub`) and derives its own 16 and 2 from it and
   from `size_of::<NodeId>()`. Three list passes, each priced in `ids`:
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
     `document_allows` (1155); `next_sibling(node)` at 1164 when `child` is the node,
     `front(j)`; `admit_depth` (`s`); `insert_checked`, which for a node that is a child of the
     target runs `next_sibling(node)` at 988, `front(j)` again (a move inside one parent: the
     node's parent is the target's, so `index_in_parent` scans `j + 1` ids).
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
     0)` (1370: it can refuse for `Nodes` only, never for `StoredBytes`); the two `to_string`
     copies of the tail and the head (1371, 1372), `L` bytes in all; `create(Text(tail), 0)`
     stores the tail's bytes, and the head's bytes leave the node (`stored_bytes -= moved`,
     1376: the tree's stored bytes change by nothing); then, for a node with a parent,
     `next_sibling(node)` (1378, `front(j)`), and `insert_checked` of the new node with that
     sibling as reference: `front(j + 1)` at 1011 and `shift` at 1021.
   - `replace_data` (1386): `utf16_len` of the data (1394, a pass over `L`), `byte_index` twice
     (1398, 1399, each up to `L`), then the build of the new data (1401-1404, one copy of at
     most `L` plus the new data). So 3 scans and 1 copy of `L`, plus the new data once. `set_data`
     (1416) and `append_data` (1422) call `utf16_len` first: one scan more.
   - The offset check has one owner, a public function the Change adds,
     `Tree::check_offset(node, offset, count) -> WriteResult<(usize, usize)>`: the three scans
     of `replace_data` (1394-1399) moved into it, answering `InvalidNodeType` for a node that
     is not character data and `IndexSize` past the end or between the two units of a
     surrogate pair, and the byte range otherwise; with `count` 0 the range ends where it
     starts and the check is the one `byte_index` pass. `split_text` calls it with count 0 and
     `replace_data` with its count; the slice calls it too, before its memory charge, so the
     slice copies no rule about surrogates and a call runs the check twice, which the tables
     charge twice.
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
     decides it is the attributes story's private body, an associated function of `impl<'e>
     DomCx<'_, 'e>` that takes the engine and no `self`, `charge_to(engine: &mut Cx<'e,
     ScriptHost>, handle: &mut Option<HostBytes>, bytes: u64) -> DomResult<()>` (`charge_stored`
     and `charge_nodes` both go through it; `charge_node` becomes its wrapper): with `None` it
     makes the handle (`charge_host_bytes`), with `Some` it resizes it, to `bytes`. This story
     calls it as `Self::charge_to(&mut *self.engine, &mut self.state.nodes, wanted)`, two
     disjoint fields, with the new size `totals.bytes + wanted`:
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
     (charged.nodes - made_nodes) + 2 x (charged.stored - made_stored)` bytes by one resize
     down, lowers `totals.nodes` and `totals.bytes` by what it gave back and never grows the
     handle (`debug_assert!` that `made` is not over `charged`). It has the form of
     `settle_stored` of the attributes story (`DomCx::settle_stored(added, removed) ->
     DomResult<()>`, which that story owns), with one more case. It calls the engine's
     `Cx::resize_host_bytes` itself and not `charge_to`, because `charge_to` folds an inner
     `Err(Abrupt::Terminated(_))` into a throw and this settle has to tell the two apart. The
     layers of the answer:
     - `Ok(Ok(()))`, or no call because the size is equal: the totals change and the settle
       answers `Ok(())`;
     - an inner `Err(Abrupt::Terminated(_))`: the script's run has ended already; the settle
       answers `Ok(())` and changes no total. In `scoped_resize_host_bytes`
       (`host/host_bytes.rs:79-106 @ 0c18271`) the checks come first (`alive()?` at `:84`, the
       foreign-handle check at `:85-87`) and then `running()` (`:88`; `agent.rs:771-777 @
       0c18271`) answers the sticky termination for any resize, a shrink included. The script
       ends with its own word (`limit: steps-per-script` when the steps ended it); what follows
       is the paragraph "After a run that a limit ended" below. The variadic call below is
       where this happens;
     - an outer `Err(HostError)` (`alive()` or the foreign handle) and any other inner `Err`
       (`Abrupt::Throw`, which a live shrink cannot answer): `DomEnd::Fault`, as in
       `charge_to`, so that a script never catches it; the totals stay as they were.
     Where a run can end between a charge and its settle: the glue's variadic call charges
     `charge_nodes(k + 1, B)`, runs the slice's method and settles, and the method charges
     steps (decisions 1 and 3), so a step overrun there ends the run (`Err(Throw "the steps of
     the script are spent")`, `reads.rs:120-131`) before the settle. No other settle of this
     story or of the attributes story has slice code that charges steps between its charge and
     its settle: `charged_nodes` and `charged_write` run the tree call only, which counts no
     step, so their settles meet a live run, and `settle_stored` keeps the attributes story's
     rule that an inner `Err` there is a defect (`Fault`). That story owns `charge_to`,
     `charge_stored`, `settle_stored` and `Fault` and needs no change; this story owns
     `settle_nodes` and its `Terminated` case. Every other settle comes before
     `end_with_limit`, while the run is live. A caller takes the result after the settle: a `?`
     on the settle's own result is allowed. When the tree call answered an `Err` and the settle
     answers `Fault`, the `Fault` is what the member answers.
     After a run that a limit ended. The runner keeps one agent for all scripts of a document
     (`runner.rs:560` and `:586 @ 83e6b8a`), and a script that a limit ended is followed by the
     next one: the arm of `document_end` that holds `ScriptEnd::Limit(_)` goes on
     (`runner.rs:704-708`), and `end_entry` clears the sticky termination (`agent.rs:559-565 @
     0c18271`; `:606-612 @ 0.10.0`), so the next script runs on the same `DomState::nodes`
     handle. Two ends stop that: a second `limit: memory` end and a fault (`runner.rs:641-645`),
     and a wall-clock end, whose deadline covers the whole document, makes the next script
     `not run: wall-clock` (`:647-648`). The skipped settle leaves at most one call's charge
     above what the call made, in the handle and in `totals()`, until the runner drops:
     `charged - made` of that one variadic call, and a later script that ends the same way adds
     its own. That amount counts against `memory_bytes` for every later script of the page. The
     handle is never under the tree: what the glue charged is a bound of what the method makes,
     and a refused charge changes nothing. No `owed` amount is kept in `DomState` to give it
     back at the next resize: this story does not build one.
   - What is charged on a bound and settled to what was made:
     - a create, a clone and `splitText` go through one private function of
       `mutation_writes.rs`, `charged_nodes(cx, count, stored, write)`: `charge_nodes` (its `?`
       is before any change), the tree call `write`, then `settle_nodes` with the tree's changes
       of `Tree::node_count()` and `Tree::stored_bytes()` over the call, after an `Ok` and after
       any `Err` alike (an `IndexSizeError` included: a `?` on the tree call between the charge
       and the settle is a defect, and acceptance 3 holds the order with a source test); the
       function then answers the settle's `Fault` if it has one, else the tree call's result.
       One rule for all three: the settle gives back to the tree's change of `stored_bytes()`.
       For `splitText` that change is 0 (`tree.rs:1376`: the tail's bytes move out of the head),
       so the call charges the bound `L` for its transient copies, the settle gives all of
       `2 L` back, and the handle keeps only the node's 408 bytes, because the stored bytes
       that existed were charged when their node was made;
     - a variadic list: the glue charges `charge_nodes(k + 1, B)` before the first copy of an
       item (`k` the strings, `B` taken as 3 x their UTF-16 units: a UTF-8 byte is at most 3 per
       unit, so for ASCII text it is an overcharge of 3, and it is allowed; one string of 3 units
       is charged 9 bytes) and settles after the slice's method returns, with the tree's gain
       over the call. The glue will know `B` before any copy: `finish_nodes_or_strings`
       (`prelude.rs:1055-1074 @ 50a4239`, which only calls `copies.copy` for a string item) will
       sum `string_length(cx, string)` (`prelude.rs:928`, `ToObject(string).length`, which
       copies nothing) over the raw string items, as `Copies::copy` reads one length before its
       charge. When the method's steps end the run the settle answers `Ok(())` and the handle
       keeps what was charged (the rule above, and the paragraph on a run that a limit ended).
       `1,000` strings of 3 units: `charge_nodes(1,001, 9,000)` wants `1,001 x 408
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
     call leaves it where it was. `splitText` makes no stored call: the tree's stored bytes
     change by nothing (the head's bytes leave the node and enter the tail), so `live` stays
     equal to them; the new node is charged by `charged_nodes` (above).
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
     in `finish_nodes_or_strings` before the copy loop, after the sum of the lengths of the
     strings (`string_length`, above). A member that then does nothing (a
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
       while a growth copies (the old and the new vector), and 0 for a list that a bulk removal
       emptied (`remove_children` and `remove_all` take the list, `tree.rs:539-545`; decision
       15). At the node limit 4 x 1,048,576 x 8 = 33,554,432 bytes bounds the lists that a
       write of this story left; a list the parser built, or shortened with its own `remove`,
       keeps what `Vec`'s doubling gave it, at most twice the longest length it had, which the
       parser's own limits bound and this story does not change;
     - `replace_data` holds the old and the new text at once, so `2 L`; `splitText` holds the
       text, the tail and the head, so `2 L` more; `normalize` holds the merged text, up to the
       bytes of the longest run (twice, `merged` and `push_str`);
     - a removed node's arena slot and strings, bounded by 164,482 script-made nodes and the
       parser's own limits;
     - the single argument copies of the glue (`Copies::copy`): bounded by the string length
       limit, and the vector the copy makes, 2 bytes a unit plus the `String` of up to 3 bytes
       a unit (arithmetic: at 16,777,216 units, 33,554,432 bytes of `u16` and 50,331,648 bytes
       of `String`). The steps for them are charged before the copy since the reading unit's
       sixth fix round (`50a4239`), the amounts unchanged, so a call past its steps ends before
       any unit is copied; the memory the copy takes stays outside the budget.
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
   - after any call, refused or not, the nodes the tree gained equal the change of
     `totals().nodes`, and the change of `totals().bytes` over the call equals `3 x NODE_BYTES`
     (408) for each node gained plus 2 x the stored bytes the tree gained (`Tree::stored_bytes()`
     before and after): a create, a clone or a convert that the tree refuses settles the handle
     and `totals()` to the nodes and bytes the tree did gain (decision 5). For a `Nodes` refusal
     in the middle of a convert that is the nodes made before it, which stay in the arena. For
     `splitText` the stored bytes gained are 0, so the change is 408 bytes and one node. One
     exception, by decision 5: a variadic call whose method's steps end the run leaves the
     handle at what the glue charged, since the settle meets a terminated run: the handle and
     `totals()` stay above the tree's gain by at most that one call's charge, and a later
     script of the page runs with it (decision 5, "After a run that a limit ended");
   - the `Document.title` setter is not all or nothing when the `title` element is new: it
     inserts the new empty `title` under `head` and then makes its text node, so a refused text
     node (`Nodes` or `StoredBytes`) leaves the empty `title` in the tree (one node more, and
     `writes()` raised by 1). With a `title` that exists the text node is made first
     (`set_text_content` makes it before it removes a child, `tree.rs:1447-1451`), so a refusal
     changes nothing; the `limit_*_title` tests parse a page that has a `title`, and one more
     case (`title_refused_leaves_the_new_title`) holds the empty `title`.
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
     `document_get_element_by_id`), in `crates/script/tests/dom_mutation.rs`, proves the order
     by counting calls of `node_child_nodes` when the charge ends the script. The refill reads
     the child list through `reads::children`, which charges one step a child before its copy
     (`reads.rs:1050-1053 @ 50a4239`), so each refill costs the child count beside the getter's
     `8 x (kept_len + child_count)`.
8. No recursion, tested where the code is. No walk of the write surface recurses at `83e6b8a`
   (every one keeps a `pending` stack), and the walks of this slice are structs of decision 1:
   a source test shows no function of the three files or of the walks calls itself. The
   tree's write functions run directly on a thread of 256 KiB of stack, over a chain at a
   raised depth limit, with a guard that a trivial call runs on that thread: unit tests of
   `tree.rs` (acceptance 17). The walks are private to `dom/`, in a library whose manifest has
   `test = false`, so no test can run them on a thread of that size: the source test above
   and one case that runs them over a deep chain hold them (acceptance 17). That case runs on
   the runner's own thread (`runner.rs:55`, a fixed margin of 1 MiB, `:469-473`), so it notices
   only a recursion with a frame over 436 bytes a level; nothing is said of a smaller stack.
9. A `script` element a script inserts is recorded by `crate::run::record_inserted(cx, node)` of
   `story:scripts-parse-run` and not run. The path of each member is in the list below. The
   call keeps each node at most once for a document (that story's set, its rule): a node the
   parser reported gets no second record, an element moved out and back in gets none the
   second time, and a clone is another node, recorded when it is inserted.
10. `other_slices_still_throw` (in `crates/script/tests/dom_reading.rs`) holds three adjacent
    lines, cited by their text because the reading unit's hunks move them: the one with
    `document.appendChild(el)` (this story's), the one with `getAttribute` (the attributes
    story's) and the one with `querySelector` (the selectors story's). This story changes the
    first to `document.createRange()` with the message `Document.createRange is not supported:
    ranges`, and no other line; the later merger keeps all three edits. The reading module
    keeps its own `TestMutation` (`dom_reading.rs:73`, installed at `:130`), as do
    `dom_reading_cost.rs` and the `adversary_dom_reading*` modules, which replace the mutation
    slice. The three tests of `crates/script/tests/bindings.rs` that
    assert `createComment`, `appendChild`, `new Text` or `new DocumentFragment` throw
    (`a_member_of_a_slice_throws_until_its_slice`,
    `the_strings_of_a_dom_exception_outlive_a_collection`,
    `an_uncaught_dom_exception_names_its_name_and_message`) switch to members that still throw:
    `document.createRange()`, `new Document()` (message `Document constructor is not supported:
    documents made by script`) or `new EventTarget()` (`EventTarget constructor is not
    supported: events`).
11. A named deviation goes into the epic and beside its row in `selection.txt`, as a
    `deviation:` clause (the seven rows are printed in "Deviations this story names"). A member
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
    of the four members that take one (`splitText`, `replaceData`, `insertData`, `deleteData`)
    is checked first, by the tree's `check_offset` (decision 3), which the slice calls after
    charging its passes (`b16(L)` for `splitText`, a bound of `3 b16(L)` for the other three):
    `IndexSizeError` is thrown before any node or stored byte is charged, so it never ends
    `limit: memory` first. `appendData` and the setters pass the offset 0 or the length, which
    the check cannot refuse, and run no check of their own. Acceptance 15 lists the twelve pairs
    of calls that can throw two exceptions, one case each. So a script can end `limit:
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
    - at `50a4239` the unit holds it by a source test of its own,
      `no_walk_of_the_reads_copies_a_child_list` (`dom_reading_cost.rs:618`): cut into
      functions, `reads.rs` holds `.to_vec()`, `.collect`, `extend(` or `Vec::from(` only in
      `element_children`, `shallow_equal` and `children`, each charged before its copy. `find`
      is `:478`, `preorder` `:495`, `charge` `:133`, `children` `:1050`. This story keeps its
      commands beside that test: they read the working tree of the implementor's base, and the
      test is the unit's;
    - the commands of acceptance 1 test it, and each says "prints nothing else; if it does,
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
    - A bulk removal is not a shrink: `remove_children` (`tree.rs:539-545`) and the new
      `remove_all` take the list whole (`mem::take`), which frees it and leaves capacity 0, and
      the next insert grows from nothing and copies no id.
    - `shrink`'s doc comment (`tree.rs:708-709`) and the doc of `Limits::max_stored_bytes`
      (`limits.rs:45-51`, "it at most doubles") say the new rule: a child list keeps at most
      four times its entries after a removal and `2 (len + m)` after a growth. The heap bound
      holds: an entry of 4 ids is 32 bytes, inside the 408 bytes of a node's term.
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
    passes; `afterbegin` inserts before the first child (`i = 0`); `beforeend` appends. An
    element that is already a child of the target (`insertAdjacentElement`, or
    `appendChild` of a child) takes the same-parent terms of the list terms: `ids(j + 1)` at
    988 and its removal `2 k_o + 1`.
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
    `Document` pays `document_allows` and the count and text test over the fragment). The
    fragment has no parent, so no same-parent term. A single item (used as it is) that is a
    child of the target moves inside one parent and takes the same-parent terms of the list
    terms above: `ids(j + 1)` at 988 and its removal `2 k_o + 1`.
  - `replaceChildren` (5194): the convert, then `2d + s` for the validity check before any
    removal (the new public check of the Change, `check_insertion`, which runs the ancestor
    test, the `Document` checks and the depth check, and charges `5 c_p` once for a `Document`
    parent; it leaves the receiver's own children out of the `Document` tests, as the
    standard's `childrenToExclude` does, `dom.html:4064`, `:5194-5207`, so that
    `document.replaceChildren(element)` with `html` in place is accepted), then `2 c_p` for the
    removal (the any-element scan and `remove_all`, a step each
    child each pass; no share), then the insert, which sees `c_p = 0` and charges no `5 c_p`.
    A refused check leaves the children (`dom.html:4760-4795`). An item that is a child of the
    target is detached by `remove_all` with the others, so the insert sees a parentless node:
    no same-parent term. In all, for one parentless
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
      of the same parent; a parentless single node costs `(1 + n + membership + sibling) + 3d +
      2s + 3m + 2 + (j + 1) + 2 ids(j + 1) + k + ids(k - 1)` with `k = c_p - j`: the first
      bracket is 4 for one item (the glue's step, `n`, the membership step, the sibling step).
    - In `before`, `after` and `replaceWith` a single item (used as it is) that is already a
      child of the target's parent moves inside one parent: it adds `ids(j_x + 1)` at 988
      (`j_x` the item's own index) and its removal `2 k_x + 1` to the line above, as the list
      terms say. `replaceWith` adds `ids(j_x + 1)` once more, for the pass at `tree.rs:1121`
      (`next_sibling(node)` in `replace_child`, taken when `this`'s next sibling is the item,
      as for `replaceChild` below); `ids(j_x + 1)` bounds both passes, and no row has this
      shape.
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
    is `n (n + 1) + 3 n`), which is 4,008,000 for `n` = 2,000 and 25,020,000 for `n` = 5,000.
    Arithmetic, which no row asserts as an end because the engine's own steps move it: 3,161
    removals cost 3,161^2 + 4 x 3,161 = 10,004,565 and 3,160 cost 9,998,240, about the
    10,000,000 of the default limit, and the first 1,127 removals from a list of 5,000 cost
    `r (2 n - r + 4)` = 10,004,379. The rows hold the price by the form of acceptance 16: the
    steps of the loop without the removals, `steps_without`, and the difference `D` the row
    measured; a limit of `steps_without + D - 1` ends the script and `steps_without + D` lets
    it finish. The same loop from the back costs 5 steps a turn, 10,000 for 2,000. That is the
    price; the cost module holds both.
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
  the list terms. The list terms of `moveBefore` are the reference's `ids(i + 1)` and
  `ids(c_p' - i)`, the removal `2 k_o + 1`, and, when the node is a child of the target (the
  move inside one parent, which the member allows), `ids(j + 1)` for the `next_sibling(node)` at
  988 (`insert_checked`; `index_in_parent`, `tree.rs:871`), and the same again when `child` is
  the node (1164). A node of another parent has none of the `ids(j + 1)`. It connects no node
  that was not connected, so it records no script. Counter: raises.
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
  the first non-empty one, or all of them when each is empty), and the text bytes `B`. The
  walk counts a parent's runs as each child is visited: the step before a child pays for
  reading that child's kind and bytes and for updating the run its parent is in, and no pass
  over a parent's children runs at the parent's own visit, so a parent of many children is
  paid for child by child. Then,
  before the call: `6 s` (the six passes of decision 3), `3 x b16(B)`, and the sum over the
  parents of `removed_p x (c_p + 2)` (an upper bound of the `detach` of each removed text,
  `rm(k) <= c_p`, with 1 for the node and 1 for the share); and the merged bytes `added` through
  `charge_stored(added, 0)` inside `charged_write` (decision 5), which settles with the tree's
  rise of stored bytes after an `Ok` and with `settle_stored(0, 0)` after any `Err`. Limits:
  StoredBytes (merged bytes are stored again). Counter: raises when a text node is removed. The
  sum, not the longest run, is what bounds the work: many parents of long runs cost the sum.
- Character data (`appendData`, `insertData`, `deleteData`, `replaceData`, `splitText`, and the
  setters of `data`, `nodeValue` and `textContent`). The slice makes no pass of its own over
  the text: the offset check is the tree's `check_offset` (decision 3), which the four members
  that take an offset call before their memory charge, and every pass is charged before the
  call it belongs to.
  - Reads: the text's byte length `L` (constant, `Tree::data`).
  - `replaceData`, `insertData`, `deleteData`: `1 + g(u) + 3 b16(L)` for the slice's call of
    `check_offset` (a bound: the three scans of `utf16_len` and two `byte_index`, one scan for
    `insertData`), then `4 b16(L) + b16(data_len)` for the tree's own run (the same 3 scans
    and 1 copy of decision 3; the new data once): `1 + g(u) + 7 b16(L) + b16(data_len)` in
    all; the check comes first, so an offset past the end throws `IndexSizeError` having paid
    only `1 + g(u) + 3 b16(L)`. `appendData` and the `data` and `nodeValue` setters call no
    check (the offset is 0 or the length): `1 + g(u) + 5 b16(L) + b16(data_len)` (`deleteData`
    has no data argument: `g` and `data_len` are 0). Then, inside `charged_write`,
    `charge_stored(added, removed)` with `added = data.len()` and `removed = 0` (bounds known
    before the call); the call; after any `Err` `settle_stored(0, 0)` and, for a limit,
    `end_with_limit`, or the exception is thrown; after an `Ok`, `settle_stored(gain, loss)`
    from the tree's change of stored bytes.
  - `splitText(offset)`: the offset is checked first, by the tree's `check_offset` with count 0
    (one pass), charged `b16(L)` steps before it, and `IndexSizeError` is thrown before any
    node is charged. Else `charged_nodes(1, L)` on the bound `L` (the tail is at most `L`
    bytes), settled to the tree's change of stored bytes, which is 0 (decision 5), and `2 b16(L)
    + 1` steps (the tree's scan, the two copies, and the node); for a node with a parent the
    own front walk `j + 1` and `ids(j + 1) + ids(j + 2) + ids(c_p - j - 1)` (1378, 1011, 1021)
    and 2 for the insert of one id with its share; then the call. In all, with no parent
    `2 + 3 b16(L)`. Raises when the node has a parent.
  - The setter of `data` is `replaceData(0, length, value)`. The setter of `nodeValue` takes
    `Option<String>` (null is the empty string): on character data it is that, on others
    nothing. The setter of `textContent` (`dom.html:6850-6933`): on character data it replaces
    the data; on an element or fragment it is replace-all: `4 + g(u) + 2 c_p + b16(len)` steps
    (the glue's step and copy, the node, the insert of one id with its share, the any-element
    scan and `remove_children`, and the tree's copy), then `charged_nodes(1, len)` when the
    string is not empty (no node, and no `1 + 2` of those steps, when it is), the call
    (`tree.rs:1441`); on others nothing. Counter: character-data writes do not raise; the
    replace-all raises when it removed a child or made a child.
  - Limits: StoredBytes for the writers; Nodes for `splitText` only (`admit(1, 0)`, `tree.rs:
    1370`: the tail's bytes move out of the head, so no stored byte is admitted and it never
    ends `limit: stored-bytes`); Nodes and StoredBytes for the replace-all (`create_text_node`
    admits `1, len`, `tree.rs:1447`).
- `Document.title` setter. The HTML Standard's algorithm (section "document.title", not
  vendored here): with no document element, return; a document element in the SVG namespace
  takes its first `title` child in that namespace, or makes one and inserts it as its first
  child; an HTML element takes the first `title` element in tree order;
  with none, an HTML document element makes a `title` under the first `head` child of the
  document element and returns when there is none; then the text of that element is replaced
  by the value. It calls the reading unit's `find` (`reads.rs:349 @ 83e6b8a`, `:458 @
  14f8a2a`), a stepwise walk that charges a step before each node and copies no child list once
  the gate of decision 14 passes, so `find`, `Reads` and `html_element` (`reads.rs:218 @
  83e6b8a`) are made `pub(super)`. The walk starts at the document, as the reading unit's
  `title` getter does (`reads.rs:1208-1213 @ 50a4239`), so it visits the document, the `html`
  element and the rest of the tree in order, a step before each node. Charge: the glue's step
  and copy, the walk (`W` steps: `find` charges `1 + name_bytes / 16`, the division being an
  integer one, before each node it visits, `reads.rs:478-486 @ 50a4239`, so `W` is the number
  of nodes visited when every element name is under 16 bytes, as `html`, `head`, `meta`,
  `title` and the other names of the rows below are; a name of 16 bytes or more adds a step
  for each 16), then for a new `title` the scan of the document element's
  children for the first `head`, one step before each child is read (the slice's own loop,
  `h` steps for a `head` at index `h - 1`; the reading unit's `document_child`,
  `reads.rs:763 @ 50a4239`, charges all the children up front but is private, takes a `fn`
  test and would be a fifth token to widen, so it is not used), the create
  (`charged_nodes(1, 5)`, `1 + b16(5)` = 2 steps) and an insert under `head` (`3d + 2s + 3m`
  with `d` = 3), then the replace-all of `2 c_t + b16(len)`, its text node and its insert
  (1 + 2 steps). New `title` in all: `1 + g(u) + W + h + 2 + 14 + 3 + 2 c_t + b16(len)`, with
  `c_t` 0. With one existing `title` that holds no child, `1 + g(u) + W + 1 + 2 + b16(len)`.
  A page whose document element has many children before `head` (comments, which the parser
  puts there before the `head` start tag) and no `title` pays the walk over all of them and
  then the scan over all of them again, which is why the scan is charged. Limits: Nodes
  and StoredBytes. `Limit::Depth` is unreachable here: a depth counts elements
  (`element_depth`, `tree.rs:811`), a text node adds none, and a new `title` under the `head`
  child of the document element is at depth 3 (`html`, `head`, `title`), which is the smallest
  depth at which a script runs (a `script` in `head` or in `body`), so a parse that ran the
  script admits it. In one script the element comes before its text node.
  The SVG branch (a document element in the SVG namespace, which a script reaches by moving
  an `svg` element of the page's body to the document with `replaceChild`) does not call
  `find`: it scans the document element's children for the first `title` in the SVG
  namespace, one step before each child is read, `h` steps for a `title` at index `h - 1` and
  all `c_r` children when there is none. It charges: with a `title` found, `1 + g(u) + h + 1 +
  2 + b16(len)` for a `title` that holds no child (the replace-all, as above); with none, `1 +
  g(u) + c_r + 2 + 8 + (1 + 1 + ids(c_r)) + 3 + b16(len)` for `c_r` children, that is the
  scan, the create (2), the insert under a parent of `d` = 1 (`3d + 2s + 3m` = 8), the list
  terms of an insert before the first child (the own front walk 1, the tree's `front(0)`
  `ids(1)` = 1 and the shift `ids(c_r)`; with no child, none of the three) and the replace-all
  (3 and `b16(len)`). Limits: Nodes and StoredBytes. `Limit::Depth` is unreachable: the new
  `title` is at depth 2, below the 3 at which a parse ran its script. Counter: the insert
  raises it. This is a price, not a deviation: it is the standard's branch, charged by a
  formula from the tree's passes as every row here is. A row (acceptance 18) and a case
  (acceptance 5) hold it.

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
| replace-all (`textContent` of an element or fragment) | StoredBytes | `limit_stored_replace_all` |
| four writers, `data` | StoredBytes | `limit_stored_character_data` |
| `normalize` | StoredBytes | `limit_stored_normalize` |
| `title` setter | Nodes | `limit_nodes_title` |
| `title` setter | StoredBytes | `limit_stored_title` |
| every family at default limits | memory | `limit_memory_default` |

`splitText` has no `StoredBytes` row: it cannot reach it (`admit(1, 0)`). The depth rows run on
an element receiver. The receivers whose depth is 0, a `Document` or a `DocumentFragment` (the
six variadic rows `prepend`, `append` and `replaceChildren` on those two, the `Node` members
called on one, the two `moveBefore` rows on them, and the four `ChildNode` rows of
`DocumentType`, whose parent is a `Document`), cannot end `limit: depth`: `admit_depth`
(`tree.rs:848-866`) adds the receiver's depth, 0, to the height of the moved subtree, and a
subtree in a tree of these limits is at most `max_depth` high. They get a guard in
`limit_depth_insert` and `limit_depth_move`: the deepest subtree the limits admit (height 512)
is offered to each such receiver, and in no case does the script end `limit: depth`. Which
setup a receiver takes depends on what the tree and the standard do with an element there
(`insertion_allowed` and `document_allows`, `tree.rs:920-940`, `move_checked`,
`tree.rs:1140-1160 @ 83e6b8a`; the pre-insert validity steps, `dom.html:4128-4183`, the move
steps, `:4502-4545`, `replaceChildren`, `:5194-5207 @ a14e3b0`), in three classes:
- A fragment receiver takes the subtree: `prepend`, `append`, `replaceChildren`, `appendChild`,
  `insertBefore`, `replaceChild` (the fragment holds one child first) and `moveBefore` (the
  subtree is made inside the fragment first, then moved to its end). The script builds the
  chain of 512 `div` elements detached and puts it under the fragment, and ends `Ran`.
- A `Document` receiver that takes an element only when it has none: `append`, `prepend`,
  `appendChild`, `insertBefore` and the `DocumentType` rows `after` and `replaceWith` (the page
  starts with a doctype, whose parent is the `Document`). The tree refuses an element there
  while `html` is the document element (`has_element`, `tree.rs:935`; the standard's step at
  `dom.html:4138`). The script first removes the document element
  (`document.removeChild(document.documentElement)`), builds the chain of 512 by script, and
  then the call runs and the script ends `Ran`. `replaceChild` and `replaceChildren` of the
  `Document` need no removal: they replace `html` with the chain (the standard leaves the
  replaced child, and for `replaceChildren` all the receiver's children, out of the test,
  `dom.html:4139`, `:5194-5207`, and the `check_insertion` of the Change does the same). The
  script ends the page, so the parser has nothing left to put into the removed `html`.
- A call the standard refuses whatever the depth: `Document.moveBefore` of an element (the
  element shares the document's root, so the document already has an element child, `html`:
  `dom.html:4537-4538`, `document_allows`, `tree.rs:935`) and `DocumentType.before` (the receiver
  is the child that is a doctype, `child_is_doctype`, `tree.rs:926`; `dom.html:4148`). The
  script catches the call (`try { } catch (e) { }`), the case asserts that the exception is a
  `HierarchyRequestError` and that the script ends `Ran`.
The chain costs 512 creates and 511 appends: by the tables 398,074 steps (arithmetic: 511
appends at `3 i + 6` for `i` = 1 to 511, which is 395,514, and 512 creates of 5, which is
2,560) and 425,472 bytes of charge (arithmetic: 512 x (408 + 423) for the node and its
wrapper), under the defaults of 10,000,000 steps and 67,108,864 bytes; the case sets
`steps_per_entry` to 50,000,000 as the memory cases do, since the engine's own steps for the
loop are not in the tables.

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
4. An offset between the two units of a surrogate pair throws `IndexSizeError`, in `splitText`,
   `insertData`, `deleteData` and `replaceData` (the end of the range, `offset + count`, is
   checked the same way, `tree.rs:1398-1399`). The tree answers it (`byte_index`,
   `tree.rs:677-692 @ 83e6b8a`, which holds text as UTF-8, where half a pair has no place);
   the standard has no such step: it compares the offset with the length and throws only when
   it is greater (replace data, `dom.html:11546-11552`; split a Text, `:11794-11800`). The
   tree's own story records the rule as what the write surface leaves to its caller
   (`story:html-script-hook`, in the part that lists what the write surface leaves to its
   caller). `check_offset` (decision 3) keeps it, so the slice copies no rule about
   surrogates.

Each block below is one line of the file: its lines are joined by one space, and the blocks
are seven lines.

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

```
CharacterData insertData mutation deviation: an offset between the two units of a
surrogate pair throws IndexSizeError
```

```
CharacterData deleteData mutation deviation: an offset between the two units of a
surrogate pair throws IndexSizeError
```

```
CharacterData replaceData mutation deviation: an offset between the two units of a
surrogate pair throws IndexSizeError
```

```
Text splitText mutation deviation: an offset between the two units of a surrogate pair
throws IndexSizeError
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
  - `check_insertion(node, parent, child, excluded)`, the public validity-only check: the
    ancestor test, the `Document` checks and the depth check of `insertion_allowed` and
    `admit_depth` with no write, `2d + s` and `5 c_p` for a `Document`. `excluded` is a slice of
    the children the `Document` tests leave out (the standard's `childrenToExclude`): the one
    child for a replace, all of the parent's children for `replaceChildren`; the private
    `insertion_allowed` and `document_allows` take the same slice in place of their
    `Option<NodeId>`, and the other callers pass a slice of one or none;
  - `check_offset(node, offset, count)`, the one check of an offset and a count in UTF-16
    units against a value (decision 3), called by `split_text`, by `replace_data` and by the
    slice: 1 pass of `L` for count 0, else 3;
  - `remove_all(parent)`, the bulk removal of `remove_children` without a `Cell`, for
    `replaceChildren`: `c_p` visits, and the any-element scan before it, `c_p` reads; it takes
    the list whole and leaves capacity 0, as `remove_children` does (`tree.rs:539-545`);
  - `work()`, `WorkCount` and the private `Work` of decision 4, raised in each pass of the table
    of decision 3; the doc comment of each write function lists its passes;
  - `shrink_children` in place of `shrink` at `:893`, `reserve_exact(len + 2 m)` before the
    splice at `:1021` and `push_child` for the clone's pushes at `:1247` (decision 15); `shrink`
    stays for attribute lists, and its doc comment (`:708-709`) says which lists it serves;
  - no recursive function is changed: none recurses (decision 8).
  Rules that two places hold: the clone's cost (`clone_cost`, private, owns it; the slice's walk
  copies it on purpose because it charges before each node, and the clone row asserts they
  agree), a template as two nodes (`create_element_ns` owns it; the slice's
  `charge_nodes(2, ...)` copies it on purpose, held by the `createElement("template")` row),
  the `IndexSizeError` bound (the tree owns it in `check_offset`; the slice calls that function
  and charges its passes, and a source test holds that it names no surrogate rule), the depth
  (`admit_depth` owns it; the slice only prices the climbs), and `b16` and `ids` (private to the
  slice with their unit test; the cost module derives its 16 and 2 from `STRING_BYTES_A_STEP`
  and `size_of::<NodeId>()`, decision 2).
- `crates/webidl/src/prelude.rs`, `crates/webidl/src/generate.rs`, regenerating
  `crates/script/src/bindings.rs`: `charge_nodes`, `settle_nodes` (its `Terminated` case of
  decision 5, and one line that names `DomEnd::Fault`: the attributes story counts the lines
  that name it by a command at its own base, and acceptance 3 says `mutation_writes.rs` names
  none), `NodeCharge` and the `nodes`
  handle (`DomState` fields and `DomState::new`, `prelude.rs:104-131`), `DomState::charges`
  removed, `node_charges()` redefined; `charge_node` as a wrapper over `charge_nodes`; `n`
  steps in `to_nodes_or_strings`, and the memory charge and its settle around the copy loop of
  `finish_nodes_or_strings`; the refill charged in the getter template before the slice call
  and the inner bodies of `refresh_collection` and `collection_object` (decision 7).
- `crates/webidl/selection.txt`: the seven `deviation:` rows printed above (lines 63, 151, 157,
  186, 187, 188 and 197).
- `crates/script/src/runner.rs`: the comment at the arm of `Limit::InputBytes | Limit::Steps`
  (the four lines that begin "The limits of the parser's input and of its steps") says no
  member of the slices `reading`, `attributes` and `mutation` reaches either; the arm stays.
- Tests: `crates/script/tests/dom_mutation.rs` (the cases and the source tests; a one-line stub
  exists at `14f8a2a`), `crates/script/tests/dom_mutation_cost.rs` (new, with one `mod` line in
  `tests/main.rs`), `crates/script/tests/dom_reading.rs` (one line, decision 10),
  `crates/script/tests/bindings.rs` (three tests, decision 10; the variant `Writes::Nodes` and
  the glue unit tests of acceptance 9), and the unit tests of `crates/html/src/tree.rs`
  (acceptance 17 and 25).
- `crates/html/src/limits.rs`: the doc comment of `max_stored_bytes` (`:45-51`) says what
  decision 15 allows of a child list, one sentence; no code.
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

1. Five start checks, not results. Each is a command run in the implementor's working tree
   (no commit in the command, so it reads the files of the base) and prints what it says, or the
   work stops and the implementor reports. The outputs below are what the same command prints
   when given the named commit, with its prefix `<commit>:` left out.
   - The price gate: `git grep -n -F 'units / 64' -- crates/script/src/bindings.rs` prints
     `crates/script/src/bindings.rs:628` at `83e6b8a`, nothing at `14f8a2a` and `50a4239`, and
     `crates/script/src/bindings.rs:561` at `a14e3b0`. At the implementor's base it must print
     nothing (the reading story's price commit is merged first); if it prints a line, stop and
     report.
   - The price gate, positive: `git grep -n -F 'let length = string_length(cx, string)?;' --
     crates/script/src/bindings.rs` prints `crates/script/src/bindings.rs:624` at `50a4239`
     (`Copies::copy` reads the length, charges `count.saturating_mul(2) /
     STRING_BYTES_A_STEP` at `:630`, and only then copies) and nothing at `83e6b8a`, `14f8a2a`
     and `a14e3b0`. At the implementor's base it must print one line: the reading unit's sixth
     fix round is merged; if it prints nothing, stop and report. The unit's source test
     `a_string_argument_is_charged_before_its_copy` (`dom_reading_cost.rs:669 @ 50a4239`)
     holds the order; the step amounts do not depend on it.
   - The reads gate (a): `git grep -n -F 'pending.extend(' -- crates/script/src/dom/reads.rs`
     prints `:363`, `:914` and `:935` at `83e6b8a` and nothing at `14f8a2a` and `50a4239`. It
     must print nothing at the implementor's base, and nothing else.
   - The reads gate (b): `git grep -n -p -F -e 'pending.extend(' -e '.to_vec()' -e '.collect'
     -- crates/script/src/dom/reads.rs | grep -E '^[^:]+=[0-9]+=' | grep -v -e 'fn
     element_children(' -e 'fn shallow_equal(' -e 'fn children('` prints the headers of `find`,
     `text_content`, `is_equal` and `whole_text` at `83e6b8a` and nothing at `14f8a2a` and
     `50a4239`. It must print nothing at the implementor's base, and nothing else.
   - The reads gate (c), positive: `git grep -c -F 'fn preorder(' --
     crates/script/src/dom/reads.rs` prints `crates/script/src/dom/reads.rs:1` at `14f8a2a` and
     `50a4239` and nothing at `83e6b8a`. At the implementor's base it must print that one line:
     a gate that prints nothing because the file or the function was renamed is not a pass.
   When the price gates and the three reads gates pass, the `title` setter is started; until
   then it is not, and the story is not complete (decision 14). The reading unit's source test
   `no_walk_of_the_reads_copies_a_child_list` (`dom_reading_cost.rs:618 @ 50a4239`) now holds
   the walk property in that unit's own gate; the commands above stay, because they run at this
   story's base before anything is built.
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
   table entry list over one parse. Three more entries:
   - the brand check, two examples: `Node.prototype.appendChild.call({}, x)` and
     `CharacterData.prototype.appendData.call(document, "a")` each throw a `TypeError`, and the
     tree and `totals()` are as before the call;
   - `importNode(script, true)` with a `script` element, its copy inserted once: one record,
     `not run: inserted`, and none at the import;
   - `constructors_share_the_create_path`: the four constructors (`Text`, `Comment`,
     `DocumentFragment`, `ProcessingInstruction`) have no cost row of their own. A source test
     cuts `mutation.rs` into its methods and asserts that the body of each constructor method
     is, up to its name, the body of its `createX` member (`createTextNode`, `createComment`,
     `createDocumentFragment`, `createProcessingInstruction`): the same single call into
     `mutation_writes.rs` with the same arguments, so that the constructor has the charge of
     the `createX` rows. A behaviour entry for each constructor in the table above asserts the
     node it makes (the kind, the data and the serialization after an insert).
   The stubs `crates/script/src/dom/mutation.rs` (a unit struct and an empty `impl`) and
   `crates/script/tests/dom_mutation.rs` exist at `83e6b8a`, `14f8a2a` and `a14e3b0`;
   `mutation_reads.rs`, `mutation_writes.rs` and `dom_mutation_cost.rs` do not. Acceptance 4
   (its first two parts), 20 and 24 hold on `main` today: each says it is a guard.
3. `the_files_hold_their_parts`: a source test over `include_str!` asserts that `mutation.rs`
   holds none of `tree()`, `tree_mut()`, `charge(`, `charge_steps`, `charge_stored`,
   `settle_stored`, `charge_nodes`, `settle_nodes`, `charge_node`; `mutation_reads.rs` holds
   none of `tree_mut()`, `charge_stored`, `settle_stored`, `charge_nodes`, `settle_nodes`,
   `charge_node`; `mutation_writes.rs` holds no `tree()` and none of `put_attribute`,
   `set_attribute`, `set_attribute_ns`, `remove_attribute`, `remove_attribute_ns`,
   `toggle_attribute` (so `Limit::Attributes` is unreachable) and no `DomEnd::Fault` (the
   function `settle_nodes` of the glue answers it, decision 5); no function of the three files
   calls itself; and no walk of `mutation_reads.rs` holds `.to_vec()` or a `children(...)` call
   that is collected. In `mutation_writes.rs`, `charge_stored(`, `settle_stored(`,
   `charge_nodes(` and `settle_nodes(` occur only inside `charged_write` and `charged_nodes`,
   and in each of the two the text between the charge statement and the first settle holds no
   `?`. The offset check has one owner: `mutation_writes.rs` calls `check_offset(` for each of
   `splitText`, `replaceData`, `insertData` and `deleteData` and holds none of `0xD800`,
   `0xDBFF`, `0xDC00`, `0xDFFF`, `encode_utf16`, `len_utf16`, `utf16_len`, so it copies no rule
   about surrogates. Red on `main`: the files do not exist.
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
   for the reads gates of acceptance 1. A second case, `title_on_an_svg_root`: over
   `<body><svg><!--c--></svg><script>var s = document.body.firstChild;
   document.replaceChild(s, document.documentElement); document.title = "x"; document.title =
   "y"</script>` the end is `Ran`, the document element is the `svg` element, its first child
   is a `title` in the SVG namespace with the text `y`, and it holds one `title` child, not
   two.
6. `append_keeps_the_order`: the page
   `<body><div id="a"></div><script>document.getElementById("a").appendChild(
   document.createElement("p"))</script><i>after</i></body>` leaves the tree's `body` children
   in the order `div`, `script`, `i`; the only child of `div` is `p`; the text of `i` is
   `after`.
7. `limit_*` tests, one for each row of the limits table in "What each member charges" (15
   tests): for each member of the family, one parse; the script's end is the limit's word, also
   inside `try { } catch { }`, parsing goes on (in the `limit_nodes_*` and `limit_stored_*`
   cases only closing tags follow the script, see the count at the call below; in the other
   cases the rest of the page is in the tree), and the
   tree after is as decision 6 says: for each member the test also parses the page with the
   script without the refused call (with the convert made by hand for two items or more, as
   decision 6 gives it: `f = createDocumentFragment(); f.appendChild(a); f.appendChild(b)`,
   which makes 1 node and raises `writes()` by 2, by 4 when `a` and `b` had parents) and asserts
   equal serialization, node count and `writes()`. The node count the tree gained equals
   `totals().nodes` in every case that makes nodes. The `title` setter has no depth test, since
   it cannot reach `limit: depth` (the `Document.title` member says why); a guard test,
   `title_at_the_smallest_depth`, parses `<head><script>document.title = "x"</script>` at
   `max_depth` 3 and asserts the script ran to its end and the `title` is in the tree.
   - The count at the call. `admit` compares the nodes and the stored bytes the tree holds at
     the call with the limit, not the page's final count (`tree.rs:731-739`). So a case that
     sets `max_nodes` or `max_stored_bytes` from the page's count has the script as the last
     node-making and byte-making thing of its page: only closing tags follow it, and no text,
     element or comment. The count is read by a first parse of the same page at the default
     limits, in which the call runs and is admitted: `Tree::node_count()` or
     `Tree::stored_bytes()` after it, less what the call made (the member's lines above give
     it; the arena only grows, so a node the call removes stays in both counts, as
     `node_count`'s doc says, `tree.rs:584-586 @ 83e6b8a`). Since nothing follows the script,
     that is the count at the call. The limit is that count for a refusal of a call that adds,
     and the count at the call plus the part to be admitted for a call that must be admitted in
     part. Every `limit_nodes_*` and `limit_stored_*` case takes its limit this way.
   - `limit_nodes_title` and `limit_stored_title` parse a page that has a `title` element, so
     that the tree after the refusal equals the tree before (with a `title` that exists the text
     node is made first, decision 6: it is the one node and the one byte of `"x"` that the call
     makes, so the limit is the first parse's count less 1, and the new text node is refused).
     One more case,
     `title_refused_leaves_the_new_title`, parses a page with no `title` and the script
     `document.title = "x"`, and its limit is read as above: under a `max_nodes` of the first
     parse's `node_count()` less 1, which admits the new `title` (1 node) and refuses its text
     node (the second), and then under a `max_stored_bytes` of the first parse's
     `stored_bytes()` less 1, which admits the `title` element and refuses its one byte of text.
     Each ends with the limit's word and the tree holds the empty `title` under `head`, one node
     more than the page without the call and `writes()` raised by 1, and `totals().nodes` equals
     the nodes the tree gained.
   - `splitText` has a `Nodes` case only (`limit_nodes_split`: `max_nodes` the first parse's
     `node_count()` less 1, the tail being the one node the split makes, so the split is
     refused). A guard, `split_at_the_stored_limit`, parses a page whose last text node is
     followed by the script only, at a `max_stored_bytes` equal to the `stored_bytes()` of the
     first parse (the split moves bytes and makes none: the count at the call is the final
     count) and calls `t.splitText(1)` on that text node: the script ends `Ran` and the
     serialization shows the two nodes, since the tail's bytes move out of the head and the
     split admits none (`tree.rs:1370`). It goes red when the split charges the tail's bytes
     again, since nothing follows the script to hide it.
   - The depth rows run on an element receiver. For the receivers of depth 0 that the limits
     table names (a `Document`, a `DocumentFragment`, and the parent of a `DocumentType`),
     `limit_depth_insert` and `limit_depth_move` hold a guard, with the setup of the three
     classes under the limits table (the fragment takes the chain, a `Document` gets its
     document element removed first, and `Document.moveBefore` of an element and
     `DocumentType.before` are caught and answer `HierarchyRequestError`): in no case does the
     script end `limit: depth`, and the end is `Ran`. The chain's steps and bytes and the
     `steps_per_entry` of 50,000,000 are given there.
   `limit_memory_default` runs at the default parser limits under a memory limit of 4 MiB, one
   parse for each of five loops, each ends `limit: memory` and no parser limit:
   - strings: `var s = Array(100001).join("x"); for (;;) document.documentElement.append(s);`;
   - create: `for (;;) document.createTextNode("x");`;
   - clone: `var a = document.createElement("i"); for (;;) a.cloneNode(true);`;
   - split: `var t = document.createTextNode("xy"); for (;;) t.splitText(1);`;
   - character data: `var t = document.createTextNode("x"), s = "y".repeat(1048576);
     for (;;) t.appendData(s);`. The string is made by `repeat`, as the engine's own memory
     cases do (`adversary_3.rs:796 @ 50a4239`), not by an array's growth, which the engine ends
     itself under 4 MiB (`docs/design/scripts.md:320`, the row `memory 4 MiB, array growth:
     terminated MemoryBudget`). The case also runs the same script without the loop and
     asserts that it ends `Ran`, so the memory end comes from the loop's calls and not from the
     setup. The charge of `appendData` is on the stored handle (decision 5), not in
     `totals()`, whose bytes are the node handle's: after the run the case asserts
     `stored_live()` equal to `1 + 1,048,576 n` for the `n` appends made, and `stored_charged()`
     equal to `2 x stored_live()` (`STORED_FACTOR`), the member's charge on the books. Without
     the member's charge the loop would run to the tree's `max_stored_bytes` of 67,108,864 and
     end `limit: stored-bytes`, which the case's assertion of the end word `limit: memory`
     refuses.
8. `memory_before_nodes_at_defaults`: `var b = document.documentElement.lastChild; for (;;) {
   var e = document.createElement("i"); b.appendChild(e); b.removeChild(e); }` ends `limit:
   memory` at the session's limits after at most 164,482 nodes (408 bytes each; the wrapper's 423
   bytes make it about 80,756), and a removed node's charge is not refunded (`totals().nodes` and
   `totals().bytes` after one turn equal those after one create). The steps of a turn are 25 by
   the tables, without the engine's own (arithmetic: 5 for the create, 15 for the append under
   `body` at `d` = 3, 5 for the removal of the last child; 164,482 x 25 = 4,112,050, under
   10,000,000, but the engine's own steps for each turn come on top and are not in the tables);
   the case sets `steps_per_entry` to 50,000,000, as the cost module does, whatever a turn
   costs, so that the memory end comes before the step end.
9. `charge_nodes_is_bulk`: through the real slice, `append` of 1,000 strings of 3 UTF-16 units
   charges `charge_nodes(1,001, 9,000)` = 426,408 bytes (arithmetic: 1,001 x 408 + 2 x 9,000)
   and settles to 414,408 in `totals().bytes` (arithmetic: 1,001 x 408 + 2 x 3,000; the settle
   gives back 12,000), and `totals().nodes` changes by 1,001 (the strings and the fragment);
   the test reads 426,408 from the handle's size during the call (a spy `Mutation` in
   `dom_mutation.rs` that reads `totals()` in its method) and 414,408 after. A glue unit test in
   `crates/script/tests/bindings.rs` holds `charge_node(s)` equal to `charge_nodes(1, s)` and
   the counters of `charge_nodes` and `settle_nodes`. The test slice `TestMutation(Writes,
   Record)` is the attributes story's (it gives the slice its `Record` and the variants
   `Writes::Stored` and `Writes::Fault`); this story adds one variant, `Writes::Nodes`, and one
   arm of each exhaustive match on `Writes`: the arm of `Document.createTextNode` reads its
   argument as four numbers, `count stored made_nodes made_stored`, calls `charge_nodes(count,
   stored)` and `settle_nodes(charge, made_nodes, made_stored)`, and pushes `totals()` onto the
   `Record` after each. The two stories add their arms in the same two matches; the second to
   merge keeps both, the attributes story first. A `before()` with no parent and 1,000 strings
   leaves `totals().nodes` and `totals().bytes` as before the call; after the first
   `createTextNode`, `node_charges()` is 1 and after 10,000 more it is still 1 (the count of node
   handles, redefined by this story). Red on `main`: no `charge_nodes`.
   `a_variadic_call_that_ends_the_run_is_not_faulted`: the page nests 497 `div` elements under
   `body`, so that the innermost is the parent at `d` = 500, and the script is `p.append(a, b)`
   with `a` and `b` parentless elements made by the script; the call is the script's last
   statement, so that nothing of the script runs after the end. One call costs 1,532 steps by
   the tables (arithmetic: 1 + 2 + 1 + 2 x 8 + 3 x 500 + 6 + 6: the glue's step, the `n` of
   the list, the fragment, two items, the climbs, `2s` and `3m`), of which the first 3 are the
   glue's (its step and the `n` of the list) before the method. The test measures
   `steps_without`, the steps of the same script without the call, and runs the script with
   `steps_per_entry` at `steps_without + 100`: the limit falls 100 steps into the call, in the
   method, after `charge_nodes` and before the settle. The script ends `limit:
   steps-per-script` and the run is not faulted: its end is not the end that `DomEnd::Fault`
   makes. It goes red when `settle_nodes` maps the inner `Terminated` to `Fault` (decision 5).
   The runner goes on to the next script of the page (decision 5, "After a run that a limit
   ended"), so the page has a second script after the first, `document.createElement("i")`,
   which costs about 5 steps by the tables, well inside the 100 of margin, and the page's list
   of ends is `limit: steps-per-script` then `Ran`. After it the case asserts the books: the
   skipped settle left the one charge of the call above what was made, so `totals().nodes` is
   at most the nodes the tree gained plus 1 and `totals().bytes` is at most 408 x the nodes
   the tree gained plus 2 x the stored bytes it gained, plus 408 (arithmetic: the call's
   charge is `charge_nodes(1, 0)`, the fragment, 408 bytes), where the tree's gain is its
   `node_count()` and `stored_bytes()` less those of the same page parsed with each script
   replaced by `1`. The second script's own node is in both sides.
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
    during that refill. Red on `main`: the spy counts one call. The spy's method charges
    nothing; the real slice's `children` charges a step a child before its copy, which the
    refill row of acceptance 18 holds. The test of the record of a cut
    refill, `adversary_dom_reading_3.rs:149-220`, stays green and is not edited.
14. `the_records_of_inserted_scripts`: the page made of
    `<script>var s = document.createElement("script"); s.textContent = "1";`, then
    `document.documentElement.appendChild(s);</script>`, then `<script>1</script>`
    records `ran`, `not run: inserted`, `ran` in order and the element is in the tree, not run;
    two clones of it (`cloneNode(true)`), each inserted once, record two; a `moveBefore` records
    none; a script inside a `template` inserted records none; the parser's own scripts get no
    second record (through `parse_with_scripts`).
15. `every_exception_in_its_order`: the pairs of decision 13 are these twelve, one case each,
    asserted by name and message, and the test asserts that its table has twelve entries named
    for them. Each call has both faults and throws the first named:
    1. `insertBefore` of an element into its own descendant, with a `child` that is not a
       child of the target: HierarchyRequestError (ancestor), not NotFoundError;
    2. `insertBefore` of a `DocumentType` into an element with a `child` that is not a child:
       NotFoundError, not the HierarchyRequestError of the node kind;
    3. `insertBefore` of a second element into a `Document` with a `child` that is not a
       child: NotFoundError, not the HierarchyRequestError of the `Document` checks;
    4. `replaceChild`, the same call as 1: HierarchyRequestError;
    5. `replaceChild`, the same call as 2: NotFoundError;
    6. `moveBefore`, the same call as 1: HierarchyRequestError;
    7. `moveBefore` of a `Text` into a `Document` with a `child` that is not a child:
       NotFoundError, not the HierarchyRequestError of a text in a document;
    8. `insertAdjacentElement("bad", ancestor)`: SyntaxError, not HierarchyRequestError;
    9. `createElementNS("x", "a:b ")` (a valid prefix, a local name that ends in a space):
       InvalidCharacterError, not the deviation's NotSupportedError;
    10. `createElementNS(null, "a:b")`: NamespaceError, not NotSupportedError;
    11. `createElementNS("x", "a b:c")` (an invalid prefix) with a namespace outside the six:
        InvalidCharacterError, as the standard orders the prefix before the namespace;
    12. `t.splitText(L + 1)` under a `memory_bytes` of 4 MiB on a text of 4 MiB: IndexSizeError,
        not `limit: memory`; `t.splitText(L - 1)` under it ends `limit: memory`.
    A colon as the first or as a later character of the name of `createElementNS` gives
    InvalidCharacterError or, for a valid prefix and a valid local name, the deviation's
    NotSupportedError, and never an element.
16. `an_exception_after_a_charge`: for each of five families, a pair of cases on one page. The
    "limit one below" of a pair is defined by the cost row of the family: `steps_without` is the
    steps of the script with the call removed, `D` the difference the row measured (printed by
    the row); the lowered limit is `steps_without + D - 1`.
    - with the default step limit the script throws the standard's exception: `a.appendChild(
      ancestor)` over a subtree of 100,000 nodes (HierarchyRequestError by the ancestor test);
      `insertBefore(x, notAChild)` with `x` a subtree of 100,000 nodes (NotFoundError);
      `document.appendChild(second)` on a `Document` of 5,001 children (HierarchyRequestError
      by the `Document` checks); `t.splitText(L + 1)` on a text of 4 MiB (IndexSizeError);
      `createElementNS("x", name)` with `name` of 1,048,576 units, `"a:"` then 1,048,573 `b`
      and a last unit that is a space (InvalidCharacterError by the standard's check of the
      name; a valid long name would throw nothing by the standard);
    - with `steps_per_entry` at `steps_without + D - 1` the same script ends
      `limit: steps-per-script`, the named allowance of decision 13, and at `steps_without + D`
      it throws the exception. The subtree of 100,000 nodes comes from the page: a script cannot
      make it (decision 5, "Memory comes before nodes").
    Every ending row of acceptance 18 has this form: no row asserts a count of the engine's own
    steps, only the difference of two runs of one script.
17. No recursion, in two tests, each in the place where its code is.
    - `no_recursion`, a unit test of `crates/html/src/tree.rs` (the `surf-html` library's tests
      run): on a thread built with a stack of 256 KiB, named in the test, over a chain of
      20,000 elements under a raised `max_depth`, the tree functions `clone_node(root, true)`,
      `normalize`, `set_text_content`, `remove_node` of the chain's root and a valid
      `insert_before` run without a stack overflow. A guard in the same test: a trivial call
      (`Tree::children(root)`) runs on that named thread, and a probe that recurses to half
      the stack (frame size and depth printed, arithmetic) completes on it, so the stack is
      the one asked for.
    - `walks_over_a_deep_chain`, an integration test in `crates/script/tests/dom_mutation.rs`:
      `Subtree`, `Climb` and `Siblings` are private to `dom/` (`mod mutation;` at
      `dom/mod.rs:8`) and `surf-script` has `[lib] test = false` (`Cargo.toml:10-11 @
      83e6b8a`; `task test-layout` fails on a `#[test]` in a source file), so no integration
      test names them and no source test can hold them on a small stack. The case holds them
      through the members that call them, in a script over a page whose parser limit
      `max_depth` is raised to 30,000 for a chain of 20,000 `div` elements and which also
      holds 20,000 sibling `i` elements under one `p`: `appendChild` of the chain's root into a
      fragment
      (`Subtree`), `appendChild` under the chain's deepest element (`Climb`), and
      `insertBefore` of a new element before the last `i` (`Siblings`) each end the script
      `Ran`. It runs on the runner's own thread, whose stack is `stack_bytes()`, 8,739,584 bytes
      for `RunnerLimits::SESSION` (`runner.rs:109-115`), so a walk that recursed with a frame
      over 436 bytes a level (arithmetic: 8,739,584 / 20,000 = 436) would overflow it; this
      is a bound on the frames it can notice, not a proof, and no claim is made about a
      smaller stack. The source test of decision 8 shows no function of the walks calls
      itself, which holds the rest.
18. `dom_mutation_cost.rs` (new): worst shapes, each row one script, with the steps asserted
    exactly and never a time. A row over 5 s is named in the report with its time; a row that
    ends the script at a step limit runs under the 50 s wall-clock bound of
    `dom_reading_cost.rs:34`, since the engine's 10,000,000 steps take about 7 s in the test
    profile (`docs/design/scripts.md:302`) and a bound of 5 s would end the script first; no row
    runs over 60 s, and the module's step ends within 120 s (acceptance 23).
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
      (decision 4). That sentence holds per call for a list with spare capacity: a call that
      reallocates is paid by the writes since the last reallocation (decision 15), not by its
      own. So each row says how its lists got their capacity. A list that comes from the page
      has the capacity the parser's inserts gave it, a power of two above its length (arithmetic:
      32,768 for 20,000 children, 2,048 for 2,000, 8,192 for 5,001, 131,072 for 80,000 and for
      100,000), and one id more does not reallocate it; a list the script grows (the fragment of
      a variadic call, a loop's target) has `2 (len + m)` after each growth (decision 15). A row
      on a list the script shrinks or grows across a reallocation asserts the sum over the whole
      script, and the parser-list row below asserts the named slack. The rows run under
      `steps_per_entry` 50,000,000 (as `dom_reading_cost.rs:23`) unless the row says it ends. A
      row that ends or throws has the pair form of acceptance 16: the same script without the
      call gives `steps_without`; a limit of `steps_without + D - 1` ends the script and
      `steps_without + D` completes it, `D` the difference the row measured. No row asserts a
      count of the engine's own steps. Rows that share a page are one `#[test]`.
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
      - `createElementNS`, refused by the deviation: the XHTML namespace and a valid prefixed
        name of 1 MiB: 196,613 / 6 (`a:b`) = 196,607.
      - `createElementNS`, refused by the standard: the namespace `x` (1 unit, `g` 0) and the
        invalid name of acceptance 16 (1,048,576 units, the last a space): 196,610 / 3 (the
        name `a:b `) = 196,607.
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
        children: 2,041 / 43 = 1,998. The terms: the glue and climbs 36, the own front walk 1,
        the tree's front scan 1, the shift 1,000, the removal 3 and, for a node that is a child
        of the target, the scan of `next_sibling(node)` at `tree.rs:988`, 1,000 (`ids(2,000)`).
        The base has the same terms over 2 children.
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
      - an item with an old parent: `append(a, b)` with `a` the first of 20,000 children and `b`
        parentless, against a parent of one child, into `body` (`d` = 3): 40,042 / 44 = 39,998.
        The parts: `n` 2, the fragment 1, the two items 16, the old parent 40,001 (3 for the
        base), the final insert 22 (`1 + 3 x 3 + 2 x 3 + 3 x 2`).
    - Clone and template:
      - deep clone: `cloneNode(true)` of an `i` with 9,999 `i` children (`max_nodes` 25,000,
        `b + N` = 20,000): 41,250 / 5 (a childless `i`: `1 + 1 + 2 + 0 + b16(2) + 0`) =
        41,245. The parts: the own walk 10,001, the cost walk and the copy loop 20,000, the
        bytes 1,250, the share of the pushes 9,999. The row asserts that `s'` and `b` equal the
        tree's change in nodes and in stored bytes.
      - clone with attributes (illustrative; the row prints the `b` it reads from the tree): an
        element with 1,024 attributes of 5-byte names and 1,000-byte values. An attribute
        record costs `ATTRIBUTE_BYTES` = 80 beside its bytes (`tree.rs:217-222`, `:1666`), so
        `b` is 1,111,043 (3 + 1,024 x (80 + 5 + 1,000)), `N` 3, `b16(b + N)` 69,441; against a
        bare `div`: 71,749 / 5 = 71,744 (`1 + 257 + 2 + 2,048 + 69,441`).
      - template clone: a `template` of 10,000 contents, deep: 41,259 / 9 = 41,250.
      - template insert: a `template` of 100,000 nodes appended to the last child of `body`
        (`d` = 4), against an empty `template`: with `s` = 100,002 (the template, its contents
        fragment and 100,000 contents), 200,020 / 20 = 200,000 (`1 + 3 x 4 + 2 s + 3`; the
        base `s` = 2).
    - `normalize`:
      - one parent of 2,000 adjacent texts of 1 byte, against two: 4,016,381 / 29 =
        4,016,352 (1 + 2,001 + 12,006 + 375 + 4,001,998).
      - 8 parents of 1,000 adjacent texts, against two texts: 8,065,548 / 29 = 8,065,519
        (8,009 + 48,054 + 1,500 + 8,007,984 + 1).
      - one text of 4 MiB, against one of 1 byte: 786,447 / 18 = 786,429.
    - Other writers:
      - `textContent =` a string of 1 MiB on a node of 20,000 children, against `""` on a
        childless one: 236,612 / 1 = 236,611 (`4 + g(u) + 2 c_p + b16(len)`; an empty string
        makes no node and takes none of the 3 steps, so the base is the glue's step alone).
      - `splitText`, long: at `L - 1` of a text of 4 MiB, no parent, against 4 bytes: 786,434 /
        5 = 786,429.
      - `splitText`, parent: a text of 4 bytes at index 40,000 of 80,000 children, against
        index 0 of 2: 100,010 / 11 = 99,999.
      - `data =`: a text of 4 MiB set to 4 bytes, against a text of 4 bytes (5 passes):
        1,310,722 / 7 = 1,310,715. `nodeValue =` and `appendData` have the same shape and the
        same figures.
      - `replaceData(0, 1, "x")` on that text (the slice's check, 3 passes, and the tree's, 4):
        1,835,010 / 9 = 1,835,001 (`1 + 0 + 7 b16(L) + b16(1)`). `insertData(0, "x")`: the
        same. `deleteData(0, 1)`: 1,835,009 / 8 = 1,835,001.
      - `replaceData(L + 1, 0, "x")` on that text, refused: 786,433 / 4 = 786,429 (the check
        alone, `1 + 3 b16(L)`; no node or stored byte is charged).
      - `title`: `document.title = "x"` with a `head` of 100,000 `meta` elements before a
        `title`, against one `meta`, on a page with no doctype (the walk visits the document,
        `html`, `head`, the `meta` elements and the `title`): 100,009 / 10 = 99,999
        (`1 + g + (N + 4) + 1 + 2 + b16(1)`; `W` is the node count `N + 4` because every name
        in the page is under 16 bytes). It waits for the reads gates of acceptance 1.
      - `title`, new: the same setter on a page with no `title`, and 100,000 comments in the
        document element before `head` (the parser puts them there), against one comment:
        200,026 / 28 = 199,998 (the walk `N + 4`, the scan for `head` `N + 1`, the create 2,
        the insert under `head` 14, the replace-all 4, and the glue's step; every name in the
        page is under 16 bytes, so `W` is the node count). It holds the scan of the document
        element's children, charged a step before each child.
      - `title`, SVG document element: `document.title = "x"` after `document.replaceChild(s,
        document.documentElement)`, `s` being the `svg` element that begins the body, with
        20,000 comments as its children and no `title`, against an `svg` whose first child is a
        `title` followed by 19,999 comments (the same node count, so the replace and its
        walks are the same): 30,017 / 6 = 30,011. The parts of the first: the glue's step 1,
        the scan 20,000, the create 2, the insert 8, the list terms of an insert before the
        first child 10,002 (1 + 1 + `ids(20,000)`), the replace-all 4; the base is the glue's
        step, a scan of 1, and the replace-all of 1 + 2 + 1. It holds the scan and the shift
        of the SVG branch. It waits for the reads gates of acceptance 1.
      - refill: a kept `childNodes` read after each of 1,000 appends, from no children, against
        10 appends: 8,501,500 / 865 = 8,500,635. The parts, for 1,000: the glue's steps 1,000
        (one for each read), the getter's `8 x (kept + children)` 8,000,000, and the child
        counts `children` charges before its copy, one step each, 500,500.
    - Removals: `p` has 2,000 children from the page. From the front,
      `removeChild(p.firstChild)` 2,000 times: 4,008,000 (arithmetic: the sum of `2 c + 3` for
      `c` from 1 to 2,000); from the back, `removeChild(p.lastChild)` 2,000 times: 10,000 (5 a
      turn); a difference of 3,998,000. The loop from the back also has the pair form: `D` is
      the difference of the loop with its removals and the loop with `p.lastChild` read and no
      removal; a limit of `steps_without + D - 1` ends the script `limit: steps-per-script` and
      `steps_without + D` completes it. That holds the whole price of one removal, its share
      included, as one charge made before the call and no later; it does not hold the value of
      the share (see the terms below).
    - Rows that end the script. No row claims that a script ends or does not end at the
      default `steps_per_entry` of 10,000,000 by a count of the engine's own steps; the claims
      are the pair form above, or an inequality in which the engine's steps only add:
      - 5,000 removals from the front of a list of 5,000: the removals alone cost 25,020,000
        (arithmetic) and the first 1,127 cost 10,004,379 by `r (2 n - r + 4)` (1,126 cost
        9,996,628), so the script ends `limit: steps-per-script` at the 1,127th removal or before
        it, and `p` keeps at least 3,873 children (arithmetic: 5,000 - 1,127). The row also has
        the pair form on a list of 100 removed whole from the front, `D` measured;
      - a list of 3,161 children removed whole from the front: the removals alone cost
        10,004,565 (arithmetic: 3,161^2 + 4 x 3,161), over the default, so the script ends
        `limit: steps-per-script` whatever the engine's own steps are. A list of 3,160 costs
        9,998,240, 1,760 under the default: whether its script completes depends on the engine's
        own steps, so no row claims it;
      - `normalize` over 20 parents of 1,000 adjacent texts: its one charge is at least 20,019,960
        (arithmetic: 20 x 999 x 1,002, the removals alone), so the script ends `limit:
        steps-per-script` at that charge whatever the engine's own steps are;
      - a second deep clone under `max_nodes` 25,000 ends `limit: nodes`.
    - Throw rows (the call refuses, its walks are paid): the base is the same throw on a small
      shape, and the pair of acceptance 16 holds the limit one below the call's own charge.
      - ancestor: `a.appendChild(ancestor)`, `d` = 5, a subtree of 100,000 nodes from the page
        (`s` 100,000, `a` among them), against a subtree of 2 (the ancestor and `a`): 200,019 /
        23 = 199,996 (`1 + 3 x 5 + 2 s + 3`).
      - NotFound: `insertBefore(x, notAChild)`, `x` of 100,000 nodes at `d` = 5, against a `x` of
        2 nodes: 200,019 / 23 = 199,996.
      - `IndexSizeError`: `splitText(L + 1)` on a text of 4 MiB: 262,145 against 2 for a text of
        1 byte: 262,143. `replaceData(L + 1, 0, "x")`: the row above, 786,433 / 4 = 786,429.
      - `Document` with a second element: the figure of the `Document` row, 25,000.
      - `createElementNS`, refused by the deviation and by the standard: the two refused rows
        above (196,607 each).
    - The cycle row (decision 15). A page whose `<div id=p>` has 100,000 `<i>` children from
      the parser (capacity 131,072, which acceptance 25 asserts), and one element made once: `x
      = document.createElement("i")`; then 70,000 turns of `p.appendChild(x); p.removeChild(x);
      p.removeChild(p.lastChild)`. A turn is charged 28 steps (arithmetic: 18 for the append, 5
      and 5 for the removals), so the writes are charged 1,960,000 steps in all. The list shrinks
      once, at length 32,767 (4 x 32,767 = 131,068 is under 131,072), after 67,233 net removals,
      and copies 32,767 ids, 16,384 steps; the next shrink is at 16,383 or fewer and is not
      reached. This row reallocates, so it asserts the sum over the whole script and no figure
      of the engine's own steps: the end is `Ran`; the steps charged for the writes (the
      difference of the script and the same script with its three calls removed) are at least
      the converted `Tree::work()` of the whole script, the shrink's copy included; and the tree
      unit test `child_lists_keep_their_capacity` (acceptance 25) holds the count of capacity
      changes of one list over 1,000 cycles, at most 4 (the rule of `83e6b8a`: about 2,000).
    - The parser-list row (decision 15, the slack). A page whose `<div id=p>` has 65,536 `<i>`
      children from the parser, a list whose capacity equals its length (arithmetic: doubling
      from 4 reaches 65,536 at the 32,769th insert), and one `p.appendChild(x)` of a parentless
      element at `d` = 4. The call is charged 18 (arithmetic: `1 + 3 x 4 + 2 + 3`) and the
      growth to `2 (len + m)` = 131,074 copies 65,536 ids, 32,768 steps, which no script write
      has paid: the converted work is 32,779 (11 for the passes and 32,768 for the copy). The
      row asserts that the converted work minus the steps charged is at most `ids(65,536)` =
      32,768 (arithmetic: 32,779 - 18 = 32,761): the slack the parser's lists carry, at most
      524,288 steps for one list and 1,048,576 for a document (decision 15).
    - Terms with no row, named and with the reason. The glue's step `G` and the node's single
      step in creates, `insertAdjacentText` and `splitText` are constants that cancel in every
      difference of two shapes with the same calls; so is the constant 3 of `createElementNS`
      (the step, the match, the node), and so is the share of 1 in a single removal or insert
      (`removeChild` 3 in all, `adoptNode` 3, `replaceChild` 2, `splitText` 2 beside its node):
      no row can hold the value of a constant, because the engine's own steps for a call are not
      known to this story, and the pair form holds only that a charge is made once and before
      the call. No row holds the shares of 1: each row is a difference of two shapes that carry
      the same shares, so a share of 1 cancels from it. They pay for the reallocations that the
      work counter sees over a whole script, as decision 15 argues, and no assertion holds them
      one by one. The scan of the viable sibling beyond the first one (at
      most `n + 1` steps, covered by the items' own charges); `Document.title` until the reads
      gates pass.
    - For each charge a red row: the implementor removes each charge of `mutation_writes.rs` by
      hand (the mutation tool is not on the host), runs the rows, and the report names for each
      the line changed, the row that went red, and shows `git diff --quiet` after putting the
      line back. A charge with no red row is a defect of the table and gets a row, except the
      terms named above.
19. `other_slices_still_throw` is green with its one changed line, `git diff
    $(git merge-base main HEAD)..HEAD -- crates/script/tests/dom_reading.rs` is that line only,
    and the three tests of `tests/bindings.rs` of decision 10 are green with a member that still
    throws.
20. A guard, true on `main` today and to stay true: `slot task webidl-check` and
    `ESS_TOOLCHAIN=0.97.4 slot --timeout 30m task webidl-slow` exit 0 on the regenerated
    `bindings.rs`; the times of `webidl-slow` before and after are in the report (and the job is
    not started again unchanged if it ends at its limit).
21. `selection_names_the_deviations`: a test over `selection.txt` asserts that the seven lines
    printed in "Deviations this story names" are rows of the file, byte for byte. Red on `main`:
    `createElementNS` holds only the wrapper deviation, the other six no note. Each deviation has
    a behaviour entry in the table of acceptance 2: `createElementNS` with a prefix and with a
    namespace outside the six throws `NotSupportedError`; `cloneNode` of a `Document` throws
    `NotSupportedError`; `importNode` and `adoptNode` of a `Document` throw it; and one case,
    `an_offset_inside_a_surrogate_pair`, on a text node of the one character U+1F600 (two UTF-16
    units, `"😀"`): `splitText(1)`, `insertData(1, "x")`, `deleteData(1, 1)` and
    `replaceData(1, 1, "x")` each throw `IndexSizeError` (the standard would split, insert,
    delete or replace inside the pair), and so does `deleteData(0, 1)`, whose range ends inside
    it; the tree and `totals()` are as before each call. `insertData(0, "x")` and
    `insertData(2, "x")` on that node succeed. A named test,
    `element_names_at_the_name_boundary`, runs `createElement`, `createElementNS` and
    `createProcessingInstruction` on names at the edge of the XML `Name` production: the
    code points U+003A, U+005F, U+00B7 (not a start character), U+00C0, U+00D7 (excluded),
    U+0300 (not a start character), U+037E (excluded), U+2070, U+FFFE (excluded) and the first
    of the range U+10000-U+EFFFF, each as a first and as a later character, and asserts accept
    or InvalidCharacterError as the standard's production says.
22. Public surfaces: in the last commit, after merging `main`, the story changes every public
    surface its change makes false, each number from the `slot task check` log of its tip. The
    log counts the story's tests where they live: `no_recursion` and the tests of acceptance
    25 are unit tests of the `surf-html` library, `walks_over_a_deep_chain` and the other cases
    are tests of the one integration binary of `surf-script`, and the story adds no test to a
    source file of `surf-script` (`test = false`). The strings are those of `main` at `a14e3b0`
    (the same at `83e6b8a`) and are read again at the start with `git grep -n -e 'passed'
    -e 'test files' -e '2,397' -e '1845' -- README.md website/docs docs/badges`:
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
24. A guard (on `main` the diff is empty and `slot task check` exits 0): `slot task check` exits
    0, and `git diff --stat $(git merge-base main HEAD)..HEAD` names only the files the Change
    names (its scope paths, which now include `crates/html/src/limits.rs`, and the
    public-number files of acceptance 22; the front's scope list does not hold the
    public-number files). The edit of
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
      then to `max(2 l, 4)`; a bulk removal (`remove_children` and `remove_all`) takes the list
      and leaves capacity 0.
    - `removals_give_a_list_its_room_back`: 100 children appended one at a time (capacities 2,
      6, 14, 30, 62, 126), then 99 removed from index 1 with `remove_node`: the list shrinks at
      lengths 31, 15, 7, 3 and 1 (to 62, 30, 14, 6 and 4) and ends at capacity at most 4.
    - `attribute_lists_still_shrink_at_twice`: a guard that `shrink` of `drop_attribute` is as
      it was.
    - `a_shortened_value_keeps_no_room`: a guard, true at `83e6b8a`: after `split_text(1)` on a
      text of 1 MiB the head's string has a capacity under twice its length, and so has the
      value after a `replace_data` that shortens it (`tree.rs:1372`, `:1401-1404`; the
      allocator's rounding is not asserted), so a loop of `splitText(1)` holds no room beyond
      the tail's bytes.
    - `a_parsed_list_has_the_capacity_the_cycle_row_assumes`: a parsed list of 100,000 children
      has capacity 131,072 and one of 65,536 has capacity 65,536 (the parser-list row); if
      either does not, the numbers of the rows (acceptance 18) are redone, and the test says so.

## Out of scope

Attributes and selectors (their slices); the parse that runs scripts and its per-document limits
(`story:scripts-parse-run`); a session (`story:scripts-document-port`); an open namespace for an
element or an attribute (`story:html-open-namespace`); ranges, tree walkers and node iterators
(`throws`); `createRange`, the `Document` constructor and the `EventTarget` constructor; a step
budget inside the tree's write functions; a refund of a removed node's charge; a live
collection (the snapshot deviation of the reading story stands); the glue's single argument
copy (`Copies::copy`), which the reading unit's sixth fix round already charges before the copy
(`50a4239`: this story reads that and changes nothing of it); the end of a run on an outer
`Err(HostError)` (`DomEnd::Fault`, which the attributes story adds and owns: this story passes
it on).

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
  `story:scripts-dom-collection-names` rewrites them). `story:scripts-utf16-length` has no edge
  to this story. `story:scripts-dom-collection-names` gets a `depends_on` edge to this story, a
  plan edit made outside this story's file and delivered beside it: it rewrites `refill` and
  `refresh_collection`, whose inner bodies this story splits (decision 7), so it is built on
  that base. `settle_nodes` adds one line that names `DomEnd::Fault` in `prelude.rs`: the
  attributes story counts the lines that name it by a command run at its own base, and this
  story's acceptance 3 says `mutation_writes.rs` names none. `slot task webidl-check` and `slot
  task webidl-slow` hold the generated file.
- `crates/webidl/src/generate.rs`: this story is the one open editor. It edits
  `same_object_getter`, which `story:scripts-dom-collection-names` (a draft, with the edge
  above) names beside `refill`. The attributes story edits `prelude.rs`, where `strings_value`
  is (`:1441`), and not `generate.rs`, which only names it (`:239`); its scope has no
  `generate.rs` and its acceptance 10 keeps the file out of its diff.
- `crates/webidl/selection.txt`: `story:scripts-dom-attributes` (the note of row 96),
  `story:scripts-dom-selectors` (nine rows); this story rows 63, 151, 157, 186, 187, 188 and
  197. Rows 125, 174
  and 230 (`moveBefore`) are next to the selectors story's rows 126, 175 and 231 and are not
  edited by either.
- `crates/html/src/tree.rs`: `story:scripts-dom-attributes` adds three reads
  (`attribute_position`, `attribute_position_ns`, `attribute_names`), three owners
  (`Namespace::from_url`, `Tree::namespace_error`, `Attribute::qualified_len`) and
  `Attribute::stored_bytes`; the reading story edits the write counter `writes()` (it merges
  first); this story adds `work()`, `shrink_children`, `push_child`, `check_insertion`,
  `check_offset` and `remove_all` and makes `parent_or_host` public, in other regions. The
  `#[cfg(test)] mod tests` of the file is one region both stories append to (this story's tests
  are `no_recursion` of acceptance 17 and those of acceptance 25, the attributes story's are
  its own): the second to merge keeps both tests by hand. A draft that may change
  child-list writers, `story:scripts-dom-sibling-positions`, needs a `depends_on` edge to this
  story: this story's formulas and the work counter stay safe over a faster list, and its
  acceptance 18 holds front removals quadratic where that draft's acceptance 2 holds them
  linear. `story:html-open-namespace` (a draft: changes `Namespace` and `create_element_ns`,
  and leaves deviation 1 open, its decision 3) names neither `mutation_writes.rs` nor row 151;
  it depends on the attributes story only, and needs an edge to this story if it lifts the
  deviation. `story:html-parser-hook-cost` (a draft that touches the tree's parser-facing
  functions).
- `crates/html/src/limits.rs`: this story edits one doc sentence (`Limits::max_stored_bytes`,
  "it at most doubles") and the doc of `shrink` at `tree.rs:708-709` follows decision 15. The
  two stories that name the file, `story:html-parser-limits` and `story:html-script-hook`, are
  implemented; no story that is not names it.
- `crates/script/src/dom/reads.rs`: the token edits of three stories are different spellings:
  `charge` is made `pub(super)` by the attributes story, the selectors story and this story;
  the selectors story writes `pub(super) enum Reads<'a>` and widens the scan's name to `&str`,
  and this story makes `Reads` `pub(super)` without the lifetime; so the merger takes the
  selectors story's `Reads<'a>` and `find` is written against it (acceptance 24's diff counts
  the lifetime as visibility). The reading unit's fix round edits `find` (`:349-366 @ 83e6b8a`)
  and this story starts after it. This story makes `find` and `html_element` `pub(super)`.
- `crates/script/tests/main.rs`: `story:scripts-dom-attributes` and this story; this story's
  `mod dom_mutation_cost;` is next to the attributes story's `mod dom_attributes_cost;`: keep
  both. The selectors story does not edit it.
- `crates/script/src/dom/mod.rs`: `story:scripts-dom-attributes`, `story:scripts-dom-selectors`
  and this story, each one or two `mod` lines, in different places: keep all.
- `crates/script/tests/dom_reading.rs`: `other_slices_still_throw` holds three adjacent lines,
  cited by their text (a hunk of the reading unit moves them): the one with
  `document.appendChild(el)` (this story), the one with `getAttribute` (the attributes story)
  and the one with `querySelector` (the selectors story): keep all three.
- `crates/script/tests/bindings.rs`: the test slice `TestMutation(Writes)` becomes
  `TestMutation(Writes, Record)` in `story:scripts-dom-attributes`, which adds the variants
  `Writes::Stored` and `Writes::Fault`; this story adds the variant `Writes::Nodes` and its arm
  in the same two exhaustive matches (acceptance 9), and accepts the merge of the match arms,
  the attributes story first; the swell pin story (`dom_limit_drops_the_jobs` and a `then`
  variant of `limit_end`); this story changes three tests' members and adds glue unit tests.
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
  keeping the others' sentences. `AGENTS.md` is listed by four more stories that the
  selectors story names, `story:test-step-holds-with-tls`, `story:tls-tests-as-tables`,
  `story:session-argument-checks` and `story:conformance-lock-and-lints`. Of the public
  numbers, the first two change the count of the TLS step (the `552` of acceptance 22),
  `story:conformance-lock-and-lints` also edits `website/docs/status.md`, and
  `story:scripts-document-port` lists `status.md` and `what-works.md` (it comes after this
  story through `story:scripts-html-conformance`). Each merges `main` first; the later merger
  takes its numbers from its own log.
- Other plan files change with this story: the epic (this story's block, and its rule on the
  public-number files, which omits the swell pin story and the reading unit as writers),
  `story:html-open-namespace` (decision 3), `story:scripts-dom-reading` (the bullet in decision
  20 that owns the walk property of decision 14, which says that the unit holds it by a source
  test since its sixth fix round, and the sentence that says another story moves the argument's
  charge before the copy, which that round did), `story:scripts-dom-attributes` (its line on
  `Limit::InputBytes`, its cites of this story's lines, and the meaning of `node_charges()`),
  `story:scripts-parse-run` and `story:swell-0-9-0` (their cites of this story's lines) and
  `story:scripts-dom-collection-names` (the edge named above).

## Why `epic:scripts-in-a-page`

Writes to the tree, without which no script changes what surf shows (example 1 of the epic). The
document port's example appends an element built by a script, and the hostile-pages story builds
a deep tree by `appendChild` and expects `limit: depth`; both need the writes, each priced so
that a hostile loop of them ends inside the step limit.
