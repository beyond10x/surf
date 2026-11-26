---
format: aep.planning-md/3
id: story:scripts-dom-attributes
kind: story
status: proposed
title: The attribute members (14 rows), held to the attribute, stored-byte and memory limits and priced by what they scan
summary: Element's id, className and slot and its 11 attribute operations (17 trait methods) as the DOM Standard says, in three files (the members, every read of the tree, every write of the tree) with a source test that holds the members free of tree calls and charges, writes through the attribute functions of the tree's write surface after the standard's name checks and reads through three public reads and four owners of shared rules that the tree gains, every pass charged before it runs at one step per 16 bytes and one per 4 attributes scanned and a read of a value charging its bytes before any count, getAttributeNames charged per element, a write over the attribute or stored-byte limit ending the script through the generated limit result, the bytes script writes hold in existing nodes charged to the memory budget through one handle that the engine resizes to twice the live bytes, so that a write that removes bytes refunds them, a cost module with a floor of steps on the worst shape of each member and one budget row for each writer and each read that returns a value, a namespace outside the six of the tree named as a deviation, the TrustedType deviation tested, the brand check of the members tested, the attribute node members and the two SameObject attribute rows left to throw, the 14 case names asserted equal to the slice's rows.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:html-script-hook
- depends_on: story:scripts-dom-reading
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/html/src/tree.rs
- confidence: cited
  path: crates/html/tests/script_hook.rs
- confidence: cited
  path: crates/script/src/bindings.rs
- confidence: cited
  path: crates/script/src/dom/attribute_reads.rs
- confidence: cited
  path: crates/script/src/dom/attribute_writes.rs
- confidence: cited
  path: crates/script/src/dom/attributes.rs
- confidence: cited
  path: crates/script/src/dom/mod.rs
- confidence: cited
  path: crates/script/src/dom/reads.rs
- confidence: cited
  path: crates/script/tests/bindings.rs
- confidence: cited
  path: crates/script/tests/dom_attributes.rs
- confidence: cited
  path: crates/script/tests/dom_attributes_cost.rs
- confidence: cited
  path: crates/script/tests/dom_reading.rs
- confidence: cited
  path: crates/script/tests/main.rs
- confidence: cited
  path: crates/webidl/selection.txt
- confidence: cited
  path: crates/webidl/src/prelude.rs
- confidence: cited
  path: docs/design/scripts.md
revision: 53
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:13Z", actor: "human:operator", revision: 13}
---
## Context

The epic's tenth candidate (`.engineering/planning/epic/scripts-in-a-page.md:473-474 @ a14e3b0`)
and the slice "Attributes, 14" (lines 725-727 @ a14e3b0). The 14 rows of the slice `attributes` in
`crates/webidl/selection.txt` are `Element` `id` (line 86), `className` (87), `slot` (89),
`hasAttributes` (90) and the ten operations at lines 92-101: `getAttributeNames`, `getAttribute`,
`getAttributeNS`, `setAttribute`, `setAttributeNS`, `removeAttribute`, `removeAttributeNS`,
`toggleAttribute`, `hasAttribute`, `hasAttributeNS` (`awk '$3=="attributes"'` over the file
prints 14 lines). `classList` (line 88) and `attributes` (line 91) are rows of `throws`, and so
are `getAttributeNode`, `getAttributeNodeNS`, `setAttributeNode`, `setAttributeNodeNS`,
`removeAttributeNode` (lines 102-106), `Document` `createAttribute` and `createAttributeNS`
(159-160) and the 7 attribute members of `ProcessingInstruction` (205-211). No returned
interface has a row in this slice. Lines of the selection are those of `83e6b8a`.

The anchors. `main` is `a14e3b0`. `story:webidl-bindings` and `story:html-script-hook` are on
`main`. `story:scripts-dom-reading` is built on a branch: its structure is in the tree at `83e6b8a`
(every read of the tree in `crates/script/src/dom/reads.rs`, the members in `reading.rs`), and
its prices (one step per 16 bytes, a step a token) are built at `14f8a2a`, where `/ 64` is gone.
Every cite below is `file:line @ 83e6b8a`, or `@ a14e3b0` where the file is one the reading
unit does not touch, or `@ 14f8a2a` where it names a price of the reading unit (the glue's
charges in `bindings.rs`, the charges of `reads.rs`), which that unit moved.
The code files of `main` equal those of `77e8f3f`, the base the earlier text of this story cited.

What the code holds for this slice:

- The trait `Attributes` (`crates/script/src/bindings.rs:3108-3265`) has 17 methods for the 14
  rows: a getter and a setter each for `id`, `className` and `slot`, and the 11 operations. Every
  default answers `NotSupportedError` naming the member (`not_supported`, `bindings.rs:149-155`).
  `crates/script/src/dom/attributes.rs:1-8` is `SurfAttributes`, a unit struct with an empty
  `impl`; `crates/script/tests/dom_attributes.rs` holds its head comment only and
  `crates/script/tests/main.rs:17` declares it.
- The glue of a member (`operation_element_set_attribute`, `bindings.rs:4711-4730`) charges one
  step, checks the brand of `this` (`Element`), converts each argument with ToString (which may
  run the script's `toString`), copies each String with `Copies::copy`
  (`bindings.rs:624-632 @ 14f8a2a`: a lone surrogate written U+FFFD, and a charge of
  `units * 2 / 16` (`:630`), one step per 8 UTF-16 units, which is one step per 16 bytes of the
  units; a step per 64 bytes at `83e6b8a`).
  At `14f8a2a` the charge follows the copy; from `50a4239` (the reading unit's sixth fix round,
  on its branch, not merged) it precedes it: the glue reads `string_length` first
  (`prelude.rs:923-937 @ 14f8a2a`, which copies nothing), charges `2 x units / 16`, then copies
  (`bindings.rs:623-635 @ 50a4239`). The amount is the same, so no figure depends on it. The
  glue then calls the slice and gives the answer to `finish` (`bindings.rs:947-956`, `answered`
  `:958`): `DomEnd::Throw` becomes a `DOMException`, `DomEnd::StringLength` a `RangeError`,
  `DomEnd::Limit` goes to
  `end_with_limit` (`bindings.rs:512`), which records the limit and ends the script where it is,
  so no `catch` and no `finally` of the script runs. `Runner::document_end`
  (`crates/script/src/runner.rs:694`) turns the record into the end: `Limit::Attributes` is
  `limit: attributes` and `Limit::StoredBytes` is `limit: stored-bytes` (`runner.rs:713-714`,
  words at `:316-317`); `Termination::MemoryBudget` is `limit: memory` (`runner.rs:311`, `:916`).
  `docs/design/scripts.md` gives the same words (Q4 rows at lines 316-317; Q7 at line 435).
- The glue charges what it answers: a String at one step per 16 bytes of its UTF-8 text
  (`text.len() / 16`), before its code units are counted and before the copy (`string_value`,
  `bindings.rs:1266-1272 @ 14f8a2a`, `STRING_BYTES_A_STEP` `:1259`; the count came first at
  `83e6b8a`), a collection node at 8 steps (`COLLECTION_NODE_STEPS`, `:1251 @ 83e6b8a`,
  `:1253 @ 14f8a2a`). `getAttributeNames` answers a `sequence<DOMString>`
  through `strings_value` (`bindings.rs:1546-1560`, written by `prelude.rs:1441-1454` of
  `crates/webidl/src`), which charges each element's bytes and nothing for the element: a list
  of 16,384 one-byte names costs the glue 0 steps. Its one caller is `getAttributeNames`
  (`crates/webidl/src/generate.rs:239`, `bindings.rs:4660`), so a charge added to it changes no
  other member. The reading story left that open for this slice (decision 7).
- The tree's public write surface of `story:html-script-hook` has every attribute write this
  story needs: `set_attribute` (`crates/html/src/tree.rs:1538`), `set_attribute_ns` (`:1557`),
  `remove_attribute` (`:1596`), `remove_attribute_ns` (`:1607`) and `toggle_attribute` (`:1627`),
  with `WriteError` (`:50-72`), `WriteResult` and `Tree::limits()` (`:614`). Each write checks
  what it checks, is admitted against the limits and, over one, answers `WriteError::Limit` with
  the tree as it was (`tree.rs:12-23`). A write counts no step: "a write never stops the parse
  and counts no step" (`tree.rs:19-20`), the caller charges for it.
- What the surface does and leaves to its caller. It lowers a name for an element in the HTML
  namespace (`attribute_name`, `tree.rs:1489-1497`) and matches a qualified name against
  prefix, colon and local name (`qualified_name_is`, `tree.rs:695-706`, private).
  `attribute_name` copies the whole name into a new `String` and lowers it, and three writes
  call it before they scan, whatever the scan finds: `set_attribute` (`tree.rs:1540`),
  `remove_attribute` (`:1598`) and `toggle_attribute` (`:1634`, also on the arms that add
  nothing, `:1636`, and remove, `:1650-1654`). So a removal of a name of 50,331,648 bytes
  (3 bytes a unit at the `string_length` of 16,777,216 units) copies and lowers all of it before
  any charge could cover the pass: decision 3 changes that. Each of the five scans the
  element's attributes with `position` (`tree.rs:1541`, `:1579-1581`, `:1599`, `:1614-1616`,
  `:1635`): one pass over the list, a compare of each name against the wanted one.
  `set_attribute_ns` and `remove_attribute_ns` match an attribute by namespace and local name.
  `set_attribute_ns`
  takes the namespace as `Option<Namespace>`, one of six (`tree.rs:77-92`); it splits the
  qualified name at the first colon and answers `WriteError::Namespace` for the four conditions
  of the DOM Standard's "validate and extract" (`tree.rs:1565-1578`). It validates no name: no
  `InvalidCharacterError` (`tree.rs:21-22`). Its attribute list holds no namespace other than the
  six, so an attribute in another namespace cannot be stored.
- What the tree offers for reading attributes: `Tree::data(node)` (`tree.rs:655`) answers
  `NodeData::Element { namespace, attributes, .. }` (`tree.rs:135-173`), and `Attribute` has
  public `namespace`, `prefix`, `local_name` and `value` (`tree.rs:108-120`). There is no read by
  name, by namespace and local name, or of a qualified name, and the matching functions are
  private; `Namespace::url` (`tree.rs:94-106`) goes from a namespace to its URL and no function
  goes back; the four `NamespaceError` conditions are written inline in `set_attribute_ns`
  (`tree.rs:1569-1577`). This story adds the reads and the two small owners of those rules to
  `tree.rs` (decision 3) so that a read and a write of the same name use one matching rule, and
  the writes call them. The reading unit's own attribute reads (`reads.rs`
  `scan_for`, `positions`, `value_at`, `Reads`, `reads.rs:245-322`) match a name in no namespace
  for `class`, `id` and the like and are private to it; they do not match a qualified name.
- What a write counts in the tree. `put_attribute` (`tree.rs:1501-1526`) adds a new attribute
  with its record and strings, `attribute_bytes` (`tree.rs:215-222`): `ATTRIBUTE_BYTES` (80,
  `crates/html/src/limits.rs:90-92 @ a14e3b0`) plus the bytes of its prefix, local name and
  value; it replaces an existing attribute's value only (`stored_bytes - old + new`,
  `tree.rs:1509-1514`), keeping the name and prefix it had. A replacement by a shorter value
  lowers `Tree::stored_bytes` (`tree.rs:594`); `drop_attribute` (`tree.rs:1528-1534`) frees an
  attribute's bytes and shifts the later attributes down one place. The list grows with
  `Vec::push` (`tree.rs:1522`), which on an empty `Vec<Attribute>` reserves room for 4 records
  (320 bytes) though the first attribute is counted at 80 bytes plus its strings: the tree's
  note that "an attribute list that was empty grows exactly" (`limits.rs:44-47 @ a14e3b0`) does
  not hold for a push. `shrink` (`tree.rs:710-714`) returns room past twice the length, or past
  4.
- Which limit a write of this slice can reach. `put_attribute` answers `Limit::Attributes` when
  a new attribute would pass `max_attributes` (1,024 by default, `limits.rs:36-39`, `:106 @
  a14e3b0`) and `Limit::StoredBytes` when a new attribute or a longer value would pass
  `max_stored_bytes` (64 MiB, `limits.rs:40-52`, `:107`); a new attribute is checked against the
  attribute limit first. `removeAttribute` and `removeAttributeNS` reach no limit. A write of
  this slice reaches neither `Limit::Nodes` nor `Limit::Depth`, creates no node, and no write
  answers `Limit::InputBytes` or `Limit::Steps` (`WriteError::Limit` names four, `tree.rs:54-55`).
  The surface reads `tree.limits()`, not `DomCx::limits()` (`bindings.rs:249`) and not
  `Document.limits` (`runner.rs:434-438`).
- The memory budget. A script's DOM is charged to the agent's `memory_bytes`, 67,108,864 in a
  session (`SESSION_ENGINE_LIMITS`, `runner.rs:40-48`), by `DomCx::charge_node`
  (`bindings.rs:284`, written by `crates/webidl/src/prelude.rs:214-235`): the node's term of the
  tree's heap bound, 3 * `NODE_BYTES` plus twice the stored bytes, charged with
  `charge_host_bytes` before the write, counted in `Totals` (`prelude.rs:92-99`;
  `DomState::totals`, `prelude.rs:157`; `Runner::document`, `runner.rs:191`) and kept until the
  agent is dropped, never refunded (a node is never taken out of the arena). Over the budget
  nothing is charged and the run ends `Terminated(MemoryBudget)`, which is `limit: memory`.
  `docs/design/scripts.md:322-332` and `:902-904 @ 83e6b8a` say so. No function charges bytes
  alone, so the bytes a script writes into a node that exists (an attribute's strings, and the
  character data of `story:scripts-dom-mutation`) are charged to nothing today. The charge is
  twice the stored bytes and the tree's limit is the stored bytes, 67,108,864 against a budget
  of 67,108,864, so a script that only adds attribute bytes to a small page spends the budget
  when it has added 32 MiB (33,554,432 bytes), less half the engine's own heap, before the
  tree's stored-byte limit is reached.
- The engine can change a charge in place. `Cx::resize_host_bytes(&mut self, charge: &mut
  HostBytes, bytes: u64) -> Result<Completion<()>, HostError>` (`crates/swell/src/host/mod.rs:585
  @ 0.8.0`, the tag on `main`; `:628 @ 0.10.0`, the tag of `story:swell-0-9-0`; the same
  signature and the same documentation at both, the body `scoped_resize_host_bytes` in
  `host/host_bytes.rs:79-106` at both, read in the engine's source) changes the bytes a
  `HostBytes` holds to `bytes`. A grow charges the difference first; when the budget does not
  hold it nothing is charged, the handle keeps its bytes and the run ends in
  `Terminated(MemoryBudget)`. A shrink gives the difference back at once. A handle of another
  agent answers `Err(ForeignHandle)`. A `HostBytes` is 24 bytes (a `u64`, a `u32` and an `Rc`,
  `host/host_bytes.rs:27-34`). `charge_host_bytes` makes a handle; the glue's one use of it
  keeps one handle for each node a script makes (`DomState::charges`, `prelude.rs:112`), until
  `story:scripts-dom-mutation` keeps one handle for all the nodes (Placement).
- Memory the 64 MiB budget does not see, which this story bounds or names. The glue copies each
  argument, up to `string_length` units, whatever the order of its charge (`Copies::copy`,
  `bindings.rs:624-632 @ 14f8a2a`; the charge is steps, not memory): `to_utf16` makes a
  `Vec<u16>` of up to `string_length` units, 16,777,216 (`runner.rs:46`), 33,554,432 bytes, and
  `String::from_utf16_lossy` a `String` of up to 3 bytes a unit, 50,331,648 bytes, both live at
  once; `setAttributeNS` has three such arguments. A `Vec` that grows by pushes keeps at most
  twice its length as capacity (the amortised growth of `Vec`: a bound derived from the rule,
  not measured), so such a `String` is at most 2 * 50,331,648 = 100,663,296 bytes and such a
  `Vec<u16>` at most 2 * 33,554,432 = 67,108,864. A write then lets the tree copy the value
  with `to_string()` (`tree.rs:1549`) before `put_attribute` checks a limit, so the glue's
  `String` and the tree's copy live at once, and a replacement holds the old value and the new;
  the name is copied once in its exact length (`attribute_name`, `tree.rs:1491-1493`, or
  `to_string()`). The worst call, derived and not measured, is `setAttributeNS` with three
  arguments of `string_length` units of 3 bytes: three `String`s at twice their length,
  3 * 100,663,296 = 301,989,888 bytes; one `Vec<u16>` at twice its length, 67,108,864; the
  tree's copy of the value and of the name, 2 * 50,331,648 = 100,663,296; in all 469,762,048
  bytes, 448 MiB. A read holds less: the clone of a value, at most 3 * `string_length` =
  50,331,648 bytes (a longer value is the `RangeError` before the clone), and `attribute_names`,
  at most the stored-byte limit, 67,108,864 bytes of names, and 24 bytes for each of at most
  1,024 of them. None of this is charged to `memory_bytes` by the glue or by this story (the
  engine's own copies, which `story:swell-0-9-0` records as charged at `0.10.0`, are another
  matter); the story removes the cheap parts: a read and a removal compare a name without a
  lowered copy, `attribute_names` makes its list with `Vec::with_capacity(count)`, the tree
  copies a value with `to_string()` and never takes the glue's `String`, and an error message
  holds no argument's text (decision 4).
- How a slice charges steps, as built. `reads::charge(cx, steps: usize) -> DomResult<()>` is
  private to `reads.rs` (`reads.rs:118-131`): it calls `cx.engine().charge_steps(steps)`, does
  nothing for 0, and when the engine refuses answers `DomEnd::Throw(InvalidStateError "the steps
  of the script are spent")`, a throw the engine does not deliver: the termination is sticky and
  the run ends. `DomCx::charge_node` fails the same way for the memory budget. A read that
  returns a String is charged first, one step per 16 bytes, then held to `string_length`
  (`longer_than`, `bindings.rs:145-147`, giving `DomEnd::StringLength`), then copied
  (`reads::copied`, `reads.rs:226-237 @ 14f8a2a`; the glue's `string_value`,
  `bindings.rs:1266-1272 @ 14f8a2a`, has the same order). `longer_than` counts the UTF-16 units
  of the whole value once its bytes pass `string_length`, a pass of up to the value's length;
  at `83e6b8a` nothing had charged it yet, a defect that the reading unit's price commits
  closed in these two functions (acceptance 14 reads it again). A scan by
  name charges its attributes at one step per 4 (`ATTRIBUTES_A_STEP`, `reads.rs:250-252`), and
  `reads::attribute_count(tree: &Tree, node: NodeId) -> usize` (`reads.rs:254-265`) answers 0
  for a node that is no element.
- The vendored DOM Standard (`crates/webidl/data/whatwg/dom.html`, snapshot `e4f4f4f0`) does
  not use the XML `Name` production. `setAttribute` and `toggleAttribute` throw
  `InvalidCharacterError` for a name that is not a "valid attribute local name": at least one
  code point and none of ASCII whitespace, U+0000, `/`, `=`, `>` (lines 769-773, 10634, 10742).
  "Validate and extract" (line 835) tests the prefix against "valid namespace prefix" (no ASCII
  whitespace, U+0000, `/`, `>`; lines 764-768) and the local name against "valid attribute local
  name", then the four `NamespaceError` conditions. `id`, `className` and `slot` reflect their
  content attribute (line 10445): the getter is "get an attribute value" (the value of the
  attribute with that local name and no namespace, or the empty string; line 10212) and the
  setter is "set an attribute value" (line 10281, which finds the attribute by namespace and
  local name, not by qualified name).

What the reading unit built that this story uses, as built. One wrapper per node through
`SurfWrap` (`dom/wrapper.rs`); `dom::slices(info: DocumentInfo) -> Slices` (`dom/mod.rs:49-56`),
which builds `attributes: Box::new(SurfAttributes)`; the four kept `[SameObject]` collection
objects of `childNodes` and `children` (the `SameObject` enum has four variants,
`bindings.rs:2051-2060`), refreshed at a read when the tree's write counter has moved since they
were filled; the cases of a slice as entries of one table `CASES` in `dom_reading.rs`, run by one
helper that asserts the end `Ran` and no recorded limit; and the members `getElementById`,
`getElementsByTagName`, `getElementsByClassName`, `documentElement`, `firstElementChild` and
`children`, which this story's tests use to reach an element. The write counter is
`Tree::writes() -> u64` (`tree.rs:331`, newtype `Writes` `:286`), raised by the private `wrote()`
(`tree.rs:321`), which only a write that changes a child list calls (`tree.rs:465`, `:526`,
`:541`, `:554`, `:1008`, `:1245`); no attribute write calls it. So an attribute write refills no
kept `childNodes` or `children` object, and a loop that writes an attribute of each child costs
a constant number of steps per turn. No member of this slice returns a node or a collection, so
none calls `Wrap` and none touches a kept object. `Document.body` is no row of the selection
and no member of the bindings, so no case of this story names `document.body`; a case reaches
an element by `getElementById`, `document.documentElement` or `getElementsByTagName`
(`selection.txt:147`).

An attribute is no node of the tree. `Tree` has no node kind for it (`tree.rs:135-173`), an
`Attribute` has no `NodeId`, and it moves its position in `attributes` when an earlier one is
removed. Under the reading story's rule a wrapper is found by `NodeId` (its decision 4), so an
`Attr` object would need an identity that rule does not give: at least the element, the
namespace and the local name, and a rule for what an `Attr` is once its attribute is removed
(the DOM keeps it, with no owner element), and `Attr` is a `Node`, so every inherited `Node`
member would need a `this` that is not a `NodeId`. `Attr` and `NamedNodeMap` are not generated
(`selection.txt:26` lists the definitions; the epic, line 683 @ a14e3b0), so
`Element.attributes` is a row of `throws` and nothing in the bindings can hold an `Attr`.

Members that only read. Of the 17 methods, 9 never write the tree: the getters of `id`,
`className` and `slot`, `hasAttributes`, `getAttributeNames`, `getAttribute`, `getAttributeNS`,
`hasAttribute` and `hasAttributeNS`. They can end a script only through the engine
(`limit: steps-per-script`, `limit: memory`) or by a throw such as the `RangeError` of a String
over `string_length` (16,777,216, `runner.rs:46`). The other 8 write: the setters of `id`,
`className` and `slot`, `setAttribute`, `setAttributeNS` and `toggleAttribute` can end the
script with `limit: attributes`, `limit: stored-bytes` or, through the charge of decision 8,
`limit: memory`; `removeAttribute` and `removeAttributeNS` write and reach no limit. Sixteen of
the 17 read or write more than one field of their own node (every member but `hasAttributes`:
each scans the list, copies a name or a value, or builds the list of names); the cost module
(acceptance 11) holds a case on the worst shape for each, the members that share one function
sharing one case.

## Decisions taken

1. The members are the 17 methods of `Attributes` (`bindings.rs:3108-3265`) for the 14 rows. The
   title, the summary and acceptance 1 say 14 rows. The rows of the slice are the Context's.
2. Where the code lives: three files, so that a source test holds the rule of the reading
   story's decisions 17 to 20 for this slice.
   - `crates/script/src/dom/attribute_reads.rs` (new) holds every read of the tree this slice
     makes, in the form of `reads.rs`: a function takes the `DomCx`, charges (decision 7), then
     reads. It holds the pure helpers the reads need (the six-namespace mapping, the scan
     charge). It holds no `tree_mut()` and no `charge_stored`.
   - `crates/script/src/dom/attribute_writes.rs` (new) holds every `tree_mut()` call of this
     slice, the name validation (decision 4), the charges of a write (steps, `charge_stored`,
     `settle_stored`; decisions 7 and 8) and the map of `WriteError` (decision 5). It reads the
     tree only through `attribute_reads`. It holds no `tree()`. Every tree call of the eight
     writers goes through one private function, `charged_write` (decision 8), the only place
     that calls `charge_stored`, the tree's write and `settle_stored`.
   - `crates/script/src/dom/attributes.rs` holds the 17 methods and nothing else: each method
     calls one function below and gives the answer back. It holds none of `tree()`,
     `tree_mut()`, `charge(`, `charge_steps`, `charge_stored` and `settle_stored`.
   - `dom/mod.rs` gains `mod attribute_reads;` and `mod attribute_writes;`. `reads.rs` is
     edited in three tokens: `charge`, `ATTRIBUTES_A_STEP` and `attribute_count` become
     `pub(super)`, so that the slice has one function that says what a charge does when the
     script is over its steps, one rate for a pass over attributes and one count of them, and no
     copy of any. The count is `reads::attribute_count(tree: &Tree, node: NodeId)`, which
     answers 0 for a node that is no element (`reads.rs:254-265`): it does the job, so the tree
     gains no `attribute_count` of its own. The byte rate is
     `crate::bindings::STRING_BYTES_A_STEP`; the slice has no constant of its own.
   Member to function (`attribute_reads` for a read, `attribute_writes` for a write; each takes
   the `DomCx` and the element's `NodeId` first):
   - the getters of `id`, `className` and `slot`: `reflected(cx, element, "id" | "class" |
     "slot") -> DomResult<String>`, the value of the attribute with that local name and no
     namespace, the empty string for none (`dom.html:10212`);
   - `hasAttributes`: `any(cx, element) -> bool`, `reads::attribute_count` above 0; it charges
     nothing beyond the glue's one step (a count of a length);
   - `getAttributeNames`: `names(cx, element) -> DomResult<Vec<String>>`;
   - `getAttribute`: `get(cx, element, name) -> DomResult<Option<String>>`; `getAttributeNS`:
     `get_ns(cx, element, namespace, local_name)`; `hasAttribute`: `has(cx, element, name) ->
     DomResult<bool>`; `hasAttributeNS`: `has_ns(cx, element, namespace, local_name)`;
   - the setters of `id`, `className` and `slot`: `set_reflected(cx, element, local_name,
     value)`, which calls the tree's `set_attribute_ns` with no namespace and the local name
     `id`, `class` or `slot`, because the DOM's "set an attribute value" finds the attribute by
     namespace and local name (`dom.html:10281`) and `set_attribute` finds it by qualified name
     (`tree.rs:1541`);
   - `setAttribute`: `set` (the tree's `set_attribute`); `setAttributeNS`: `set_ns` (the tree's
     `set_attribute_ns`, once its arguments are validated, decision 4); `removeAttribute`:
     `remove`; `removeAttributeNS`: `remove_ns`; `toggleAttribute`: `toggle` (its `force` is the
     glue's `Option<bool>`, `bindings.rs:4801-4825`).
   A read and a write both need the attribute found, and a write also what it holds before it
   is written: `attribute_reads::find(cx, element, wanted) -> DomResult<Option<Found>>`
   charges the scan of decision 7, calls the tree's position read, and answers `Found { at,
   value_bytes, stored_bytes, same_length }`: the position `at` in the element's list, the
   value's bytes, the attribute's stored bytes and the `k` of decision 7 (which the write's
   second charge reuses). `Found` holds numbers and no reference, so none crosses a charge; a
   value is copied after its charge by reading `attributes[at]` of `tree.data(element)` by
   index, with no scan, so a read makes two passes over the list ((b) and (d) of decision 7)
   and the copy a third access of one attribute. `wanted` is a qualified name or a namespace
   and local name. A namespace argument is the glue's `Option<String>`, the empty string
   meaning null, or one of the six URLs of `Namespace::url` (`tree.rs:94-106`), mapped by
   `attribute_reads::namespace_of(namespace: Option<&str>) -> Option<Option<Namespace>>`:
   `None` and `Some("")` give `Some(None)` (null), a URL that `Namespace::from_url` (decision
   3, the one owner of the six URLs) knows gives `Some(Some(ns))`, any other gives `None`. A
   URL that is none of the six matches no attribute, so `getAttributeNS`, `hasAttributeNS` and
   `removeAttributeNS` answer `null`, `false` and nothing for it, as the standard does,
   without a scan.
3. `crates/html/src/tree.rs` gains three public reads, four small public owners of rules the
   tree and the slice would otherwise copy, and the writes call all of them, so that the slice
   has one matching rule with the writes, one rule for the six namespaces, one for the four
   `NamespaceError` conditions and one count of an attribute's bytes. Each read is `&self` and
   takes the element's `NodeId`:
   - `attribute_position(&self, element: NodeId, name: &str) -> Option<usize>`: the position
     in the element's list of the first attribute whose qualified name is `name`, `None` for
     none and for a node that is not an element (`qualified_name_is`, `tree.rs:695-706`;
     `dom.html:10175`). For an element in the HTML namespace the name is lowered, but the
     compare lowers the wanted name byte by byte and makes no copy (the `attribute_name` of
     `tree.rs:1489-1497` copies): an attribute whose qualified length differs from the wanted
     name's is rejected on the lengths, and one of the same length is compared byte by byte,
     each stored byte against the wanted byte in ASCII lower case, so the pass is inside the
     scan that decision 7 charges (the `k * n / S` term) and no other pass reads the name. The
     stored byte is not folded: the rule lowers the wanted name only and compares it with the
     stored name byte by byte, as `qualified_name_is` does against the lowered name
     (`tree.rs:695-706`, `:1540-1541 @ 83e6b8a`), so an attribute stored with capitals, which
     `setAttributeNS` can make on an HTML element because it lowers nothing, is matched by no
     qualified name (`setAttributeNS(null, "FOO", "1")` on an HTML element: `getAttribute("FOO")`
     is `null`, `getAttributeNS(null, "FOO")` is `"1"` and `hasAttribute("foo")` is `false`).
     It is the public face of one private function, `position_of(namespace: Namespace,
     attributes: &[Attribute], name: &str) -> Option<usize>`, the one owner of the rule, which
     `set_attribute`, `remove_attribute` and `toggle_attribute` call too, and which lowers each
     wanted byte of an HTML element with a private `lower_byte(byte: u8) -> u8`;
   - `attribute_position_ns(&self, element: NodeId, namespace: Option<Namespace>, local_name:
     &str) -> Option<usize>`: the first attribute whose namespace and local name equal the
     arguments (`dom.html:10192`). It is the public face of one private function,
     `position_of_ns(attributes: &[Attribute], namespace: Option<Namespace>, local_name: &str)
     -> Option<usize>`, the one owner of that rule, which `set_attribute_ns` and
     `remove_attribute_ns` call too;
   - `attribute_names(&self, element: NodeId) -> Vec<String>`: the qualified names in order,
     duplicates kept, each the prefix, a colon and the local name, or the local name
     (`dom.html:10581`), in a `Vec::with_capacity(count)` and each name in a `String` of its
     exact length;
   - `Attribute::stored_bytes(&self) -> usize`: the tree's count of the attribute,
     `ATTRIBUTE_BYTES` and the bytes of its prefix, local name and value, which is what the
     private `attribute_bytes` (`tree.rs:215-222`) answers, made public and called by it;
   - `Attribute::qualified_len(&self) -> usize`: the length of its qualified name, the prefix,
     one for the colon and the local name, or the local name; `attribute_position`,
     `attribute_names` and the slice's count of `k` (decision 7) use it, so the length rule has
     one owner;
   - `Namespace::from_url(url: &str) -> Option<Namespace>`, beside `Namespace::url`
     (`tree.rs:94-106`): the namespace whose URL is `url`, `None` for any other string, by a
     match on the six constants;
   - `Tree::namespace_error(namespace: Option<&str>, prefix: Option<&str>, local_name: &str) ->
     bool`, an associated function that reads no tree and makes no pass over the name: it
     takes the split of the qualified name at its first colon, which its caller has made, and
     answers whether "validate and extract" throws `NamespaceError` for that namespace URL
     (`None` is null), prefix and local name: the four conditions of `tree.rs:1569-1577`,
     where "the qualified name is `xmlns`" is "no prefix and the local name is `xmlns`", and
     every compare is of a string with a constant of at most 36 bytes (the longest of the six
     URLs), which reads no byte when the lengths differ, so it makes no pass over the name.
     `set_attribute_ns` calls it with the split it makes at `tree.rs:1565-1568` and
     `namespace.map(Namespace::url)`, and the slice calls it with the split that its
     validation made (decision 4), before any charge for memory, so the rule has one text and
     a namespace outside the six is judged by the same conditions. The tree's own split stays
     in `set_attribute_ns`, whose signature stays (the write surface's tests call it): it is a
     pass over at most the bytes of the qualified name that the tree copies in the same call,
     and it is paid by the `(n + v) / S` charge that precedes the tree call (decision 7).
   What the writes change: the five writes find the attribute with the position rule, not
   with `position` and a closure of their own (`tree.rs:1541`, `:1579-1581`, `:1599`,
   `:1614-1616`, `:1635`): `set_attribute`, `remove_attribute` and `toggle_attribute` call
   `position_of` with the name as given, and `set_attribute_ns` and `remove_attribute_ns` call
   `position_of_ns`, so a read and a write cannot disagree; none of their
   answers changes. Two of them stop copying the name: `remove_attribute` makes no copy of the
   name (it reads `tree.rs:1596-1603`: `element_attributes`, the copy at `:1598`, the scan at
   `:1599`, `drop_attribute`), and `toggle_attribute` calls `attribute_name` only in the arm
   that adds (`tree.rs:1637-1649`, where the lowered name becomes the new attribute's local
   name), not before the `match` (`:1634`) and not in the arms that remove or answer `true` or
   `false` (`:1636`, `:1650-1654`). The removal is therefore a scan and a shift and nothing
   that reads the name's bytes outside the scan; the scan is charged by the slice before the
   call, `c / A + k * n / S`, and `k` is 0 for a name whose length no attribute has, so a
   removal of such a name costs the glue's charge for the argument and `c / A`. The copy that
   stays is `set_attribute`'s (`tree.rs:1540`): it lowers the name into a `String` that the
   attribute takes when it is added and that `put_attribute` drops when it only replaces a
   value (the `Attribute` built at `:1542-1548`, dropped at `:1509-1514`), so the copy is made
   also when it is thrown away, and the `(n + v) / S` charge pays for it in both cases.
   `put_attribute` reserves exactly while the list has fewer than four
   entries (`reserve_exact(1)` before the push at `tree.rs:1522`), so that the bytes the tree
   counts are the bytes it holds for the first four attributes (decision 8); a unit test
   beside `tree.rs:1691` reads `attributes.capacity()` after each of the first four adds.
   A node that is not an element answers `None` and an empty list: the bindings brand-check
   `Element` first, and a read has no exception to answer, where a write answers
   `WriteError::InvalidNodeType` (`tree.rs:1471-1479`). What a read costs follows the write
   surface's form: the tree counts no step, and the documentation of each new public function
   says what it costs so that the caller charges for it, in a line that starts with the token
   `Cost:`: a position read visits each attribute at most once and compares each attribute's
   name with the wanted one, at most the bytes of the shorter of the two, after a compare of
   the lengths; `attribute_names` also builds one String per attribute, in all the bytes of the
   names; the others take a constant number of steps over at most the six URLs' lengths or the
   attribute's own strings. The tree takes no `Cell` of steps, as the parser's private scans do
   (`tree.rs:246`): a public read reports nothing and bounds what it scanned. Only
   `attribute_reads` calls the position reads and `attribute_names`, after its charge (decision
   7); `attribute_writes` calls `Tree::namespace_error`. The tests of the reads are
   in the module of the write surface's tests (Change).
4. Names are validated in `attribute_writes`, before the scan and the write and in the standard's
   order, and a throw leaves the tree as it was and charges nothing for memory. `setAttribute`
   and `toggleAttribute` throw `InvalidCharacterError` for a name that is not a valid attribute
   local name. `setAttributeNS` does the whole of "validate and extract" (`dom.html:835`) in
   one pass over the qualified name, charged `n / S` for the whole name before the pass runs
   (decision 7): the pass finds the first colon and tests the characters of the prefix before
   it and of the local name after it in the same loop (a name with no colon is all local
   name), so it reads each byte once, and its answer is the split. The empty namespace is
   null; a prefix that is not a valid namespace prefix, then a local name that is not a valid
   attribute local name, throw `InvalidCharacterError`; then the four `NamespaceError`
   conditions, which the slice tests with `Tree::namespace_error` given that split
   (decision 3), the one function that `Tree::set_attribute_ns` calls too, so that no write
   that is then refused for a `NamespaceError` has been charged for memory. The tree's answer
   `WriteError::Namespace` stays mapped (decision 5) and cannot occur after the slice's own
   test. The setters of `id`, `className` and `slot` and every read validate nothing. Each pass
   over a name is charged before it runs (decision 7).
   The messages are constants that name the member and hold no argument's text, so that an
   error path builds no String of an argument's size: `InvalidCharacterError` has the message
   `Element.setAttribute: the name is not a valid attribute name` (for `toggleAttribute` and
   `setAttributeNS` the member's own name in its place), and `NamespaceError` has
   `Element.setAttributeNS: the namespace does not fit the name`.
5. A `WriteError` becomes a `DomEnd`: `Limit(limit)` becomes `DomEnd::Limit(limit)`, the one
   answer that ends the script; `Namespace` becomes the `DOMException` `NamespaceError`
   (`bindings.rs:2276`); the others (`HierarchyRequest`, `NotFound`, `IndexSize`,
   `InvalidNodeType`, `NotSupported`, `tree.rs:57-71`) are named by the exception each doc
   comment gives and are not answered by these writes for an element: `InvalidNodeType`
   comes only from a `this` that is no element, which the brand check of `Element` excludes.
6. The ends of a write over a limit are the words of Q4: `limit: attributes`,
   `limit: stored-bytes` and, from the charge of decision 8, `limit: memory`, nothing written.
   This story does not edit `crates/script/src/runner.rs`: its writes reach neither
   `Limit::InputBytes` nor `Limit::Steps` (Context), and the comment on the placeholder arm for
   those two (`runner.rs:715-718 @ 83e6b8a`) is `story:scripts-dom-mutation`'s: that story
   gives the two limits no word, because no write makes either.
7. What a call costs: one price for a byte, one for an attribute passed, each charged before the
   work it pays for, in addition to the glue's one step and to the glue's charge for each
   argument it copies and for each String it answers. The glue's charge for a String argument
   is `units * 2 / 16`, one step per 8 UTF-16 units (`Copies::copy`, Context). At `14f8a2a` the
   charge follows the copy; from `50a4239` (the reading unit's sixth fix round, on its branch,
   not merged) it precedes it. The amount is the same, so no figure depends on it. Every charge
   of the slice comes before its pass. A lookup runs in this order: the glue's charge and copy
   of the argument, then the slice's charges, each before its pass. The rates are `S`, one step
   per 16 bytes (`STRING_BYTES_A_STEP`), and `A`, one step per 4 attributes passed in a scan by name
   (`ATTRIBUTES_A_STEP`). A charge is one call of `reads::charge`. Each charge is computed from a
   length known without a pass (the attribute count, a string's byte length), and no pass over a
   list or over bytes comes before the charge that pays it. The extent of every pass is known
   before it starts, so no member charges step by step, and no member splits a value into
   tokens (`className` is the attribute's whole value; `classList` is `throws`).
   - A scan by qualified name (`get`, `has`, and the scan inside a write) over `c` attributes
     of an element, for a name of `n` bytes, where `k` is the number of attributes whose
     qualified name (`Attribute::qualified_len`) has the length of the name: `c / A + k * n / S`.
     Each of the `k` compares can read all `n` bytes; an attribute of another length is rejected
     on its length. The compare lowers the wanted name byte by byte and makes no copy, so there
     is no pass for a lowering. A scan by namespace and local name (`get_ns`, `has_ns`, the
     scans of `set_ns` and `remove_ns`, `reflected`) over a local name of `l` bytes, `k` the
     attributes whose local name has that length: `c / A + k * l / S`. The order of a scan: (a)
     charge `c / A`, from the count; (b) one pass over the list that counts `k` (lengths only);
     (c) charge the bytes of the formula; (d) the tree's position read, the second pass. A scan
     makes two passes over the list, (b) and (d), and `c / A` pays both; the cost module's
     ratio against the plain walk (acceptance 11) holds the pair to the 20-times mark.
   - A value returned (`get`, `get_ns`, `reflected`): `find` has answered the position `at`, and
     the value's byte length `v` is known from it without a pass, so the order is: (a) charge `v
     / S`; (b) hold the value to the agent's `string_length` (`L`, in UTF-16 units): a value of
     at most `L` bytes has at most `L` units and needs no count; a value of more than `3 * L`
     bytes has more than `L` units (a unit is at most 3 bytes of UTF-8 apart from the pairs,
     which are 4 bytes for 2 units) and is the `RangeError` (`DomEnd::StringLength`) at once,
     with no count; in the band between, `longer_than` counts the UTF-16 units of the whole
     value, a pass over its bytes, which is charged `v / S` a second time before it runs;
     (c) the copy, reading `attributes[at]` by index. This charges the byte length before any
     count, the order of `reads::copied` and of the glue's `string_value` at `14f8a2a`, where
     they charge, then count, then copy (they counted first at `83e6b8a`); the slice keeps the
     order whatever the reading unit does, and the case of acceptance 11d holds it. The steps
     (a) to (c) are one private function of `attribute_reads`, `copy_value(cx, element, at) ->
     DomResult<String>`. The story keeps this function of its own and does not call
     `reads::copied`: `copied` is `pub(super)` (`reads.rs:226-237 @ 14f8a2a`), so the module
     could call it, but it takes a node and copies its character data (`character_data`, which
     answers `None` for an element) and has no way to name an attribute's value at a position.
     The two functions are alike on purpose and each has its own case; the doc comment of
     `copy_value` names `reads::copied` and says so, and this story does not edit the doc
     comment of `copied`, a function of the reading slice's file, where it changes visibility
     tokens only. The glue then charges its own copy of the String it answers, `v / S` again
     (`text.len() / 16`): two passes over the bytes, two charges.
   - `getAttributeNames`: the order is (a) charge `c`, one step per attribute, from the count;
     (b) one pass over the list that adds up the names' lengths (`Attribute::qualified_len`);
     (c) charge the sum over `S`; (d) the tree's `attribute_names`, which builds the list, the
     second pass; then the glue charges the bytes of each name. The generator's `strings_value`
     makes one charge of `STRING_ELEMENT_STEPS x elements`, before the loop, with
     `STRING_ELEMENT_STEPS` 8 as a collection node costs (`COLLECTION_NODE_STEPS`,
     `bindings.rs:1251`); this story changes `strings_value`
     in `crates/webidl/src/prelude.rs` (`:1441-1454`) and regenerates. The weight 8 is a
     choice: the cost module prints the time a step of `getAttributeNames` against the plain
     walk (acceptance 11), and a ratio over 20 raises the weight in the same change. So a list
     of `c` names costs at least `9 * c` steps.
   - A pass over an argument (the validation of the name of `setAttribute` and
     `toggleAttribute`, or the validation of the qualified name of `setAttributeNS`): the bytes
     of the pass over `S`, charged before it runs, `n / S` for the whole name. The split of a
     qualified name at its first colon is made once in the slice, in that same pass (decision
     4), and paid by its charge; `Tree::namespace_error` takes the split and makes no pass; the
     tree splits the name again when it writes (`tree.rs:1565-1568`), a pass over at most the
     whole name, one byte (the colon) more than the bytes it then copies, paid by the
     `(n + v) / S` charge below, which precedes the tree call; the compare of `find` reads the
     local name only, paid by `k * l / S`; the glue's copy of the name is paid by the glue's
     charge. Each pass over the name has one charge.
   - A write. The order of a member is decision 8's. Its steps, each charged before the work
     they pay for: the validation, whose pass over a name is charged (`n / S`) and then made;
     `find`, a scan as above (two passes over the list, (b) and (d)); then, before the call of
     the tree, one call of `reads::charge` for the tree's own scan, `c / A + k * n / S` with
     the `k` that `find` answered (the list has not changed since, so the pass that counts `k`
     is not made again; both terms are known, so it is one charge call, not two), and, where
     the tree copies bytes, `(n + v) / S` for them, which is the last step charge before the
     write (`n` the bytes of the name it stores, `v` of the value, whether it adds an
     attribute or replaces a value; `toggle` that adds, `v` is 0); then `charge_stored` and the
     call, whose own scan is a third pass over the list. The tree copies bytes in `set`,
     `set_ns`, `set_reflected` and the adding arm of `toggle`, and in no removal:
     `remove_attribute`, `remove_attribute_ns` and the removing arm of `toggle_attribute`
     make no copy of the name (decision 3), so a removal has no `(n + v) / S` term and its
     last step charge is the tree's scan; the name's bytes are read only inside that scan, at
     most `k * n` of them, which its `k * n / S` term pays, and for a name whose length no
     attribute has `k` is 0 and the removal charges `c / A` and no byte. `set_attribute` copies
     the name also when it only replaces a value and the tree drops the copy (decision 3);
     `(n + v) / S` pays that copy in both cases. A write that finds or adds so makes three
     passes over the list, each paid: the first two by `find`'s charges and the third by the
     tree's scan charge; the shift of `drop_attribute` is at most `c` moves, paid by the same
     `c / A`.
   So a write that adds costs at least `2 * (c / A) + (n + v) / S` steps of its own, beyond the
   glue's one step and its charge for the argument copies, `2 * (n + v) / S` for ASCII
   arguments (`units * 2 / 16` each, so twice the tree's rate); the scan term is counted twice
   (`find` and the tree's own).
8. The bytes a script writes into nodes that exist are charged to the memory budget through one
   handle that the engine resizes, so a write that removes bytes gives them back. The state of
   the bindings, `DomState` (the struct that holds `totals` and `charges`,
   `crates/webidl/src/prelude.rs:104-116`, written into `bindings.rs`), gains, all empty or 0 at
   `DomState::new` (`prelude.rs:120-131`):
   - `stored: Option<HostBytes>`: the one handle for the bytes scripts wrote into existing
     nodes, `None` until the first write that adds bytes; it is not an entry of
     `DomState::charges`, which this story gives no entry (the node charges stay as they are);
   - `live: u64`: the net rise of the stored bytes of existing nodes since the parse, by this
     story's writes and by the tree's own count of an attribute (`Attribute::stored_bytes`,
     decision 3): the bytes the writes added less the bytes they removed or replaced away, the
     origin of a removed byte not asked (a byte the parser stored and a byte a script wrote
     count the same); never below 0;
   - `charged: u64`: the bytes the handle holds, twice `live` after each settle that the engine
     answered `Ok(Ok(()))` (a refused shrink leaves it above); `Totals` stays the node totals:
     the bytes of this handle are not added to it.
   The handle holds twice `live`, the same factor as the node charge (`charge_node`,
   `prelude.rs:214-235 @ 83e6b8a`: the tree's heap bound counts a stored byte twice, as the
   engine's `String` growth and the tree's copy may both be live). The factor has one name in
   `prelude.rs`, `STORED_FACTOR: u64 = 2`, which `charge_node` uses for its stored term and the
   functions below use for the handle. `crates/html/src/limits.rs:44-47 @ 83e6b8a` holds the
   factor only as prose in the doc comment of `max_stored_bytes` ("2 x `stored_bytes()`"), a
   constant nowhere, so that is not a name and this story adds the constant. The next live
   bytes have one owner, a private function `next_live(live: u64, added: u64, removed: u64) ->
   u64`, `live + added` less `removed`, saturating at 0, which `charge_stored` and
   `settle_stored` both call.
   The engine's function is `Cx::resize_host_bytes(&mut self, charge: &mut HostBytes, bytes:
   u64) -> Result<Completion<()>, HostError>` (`crates/swell/src/host/mod.rs:585` at `0.8.0`,
   the tag `main` pins, and `:628` at `0.10.0`, the tag of `story:swell-0-9-0`; its body
   `scoped_resize_host_bytes`, `host/host_bytes.rs:79-106`, is the same at both): a grow first
   credits the charges that were dropped, then charges the difference, and when the budget does
   not hold the difference it charges nothing, leaves the handle at its bytes and ends the run
   in `Terminated(MemoryBudget)`; a shrink credits the difference at once. It checks `alive()`
   (`:84`), then whether a handle is another engine's (`:85-87`, `Err(ForeignHandle)`), then
   `running()` (`:88`). A run that has already ended therefore answers through the inner
   completion (`Ok(Err(..))`) and shrinks nothing: a settle comes before any end of the script.
   The first charge, with no handle yet, is `charge_host_bytes` (as `charge_node` calls it).
   One private body covers both engine calls and the mapping of both layers of the answer,
   `fn charge_to(engine: &mut Cx<'e, ScriptHost>, handle: &mut Option<HostBytes>, bytes: u64)
   -> DomResult<()>`, an associated function of the block `impl<'e> DomCx<'_, 'e>`
   (`prelude.rs:171 @ 14f8a2a`; `'e` is the lifetime of the field `engine: &'c mut Cx<'e,
   ScriptHost>`, `:165-169`). It takes no `self`, so that a field of `DomState` can be the
   `handle` while the engine is borrowed: `charge_stored` calls it as
   `Self::charge_to(&mut *self.engine, &mut self.state.stored, wanted)`, two disjoint fields
   of `self`, which borrow-checks. `charge_node` calls it with a local `let mut handle = None;`
   and pushes the handle it made. With `None` it makes the handle (`charge_host_bytes`), with
   `Some` it resizes it. It answers `Ok(())` only when the engine answered a handle or
   `Ok(Ok(()))`. The inner completion `Err(Abrupt::Throw(_) | Abrupt::Terminated(_))` becomes
   `DomEnd::Throw` of an `InvalidStateError` "the memory budget is spent", as `charge_node`
   maps it (`prelude.rs:229-233 @ 83e6b8a`). An outer `Err(HostError)` is a defect of surf's
   own code, as every `HostError` of the glue is (`defect`, `prelude.rs:572-574 @ 83e6b8a`,
   answers `Terminated(EngineFault)`), and it ends the run faulted: the body answers the new
   variant `DomEnd::Fault`, never `Ok` and never a throw, because a script that caught a
   throw would go on with a handle in an unknown state. Only the resize can answer it:
   `charge_host_bytes` answers `Result<HostBytes, Abrupt>` and has no outer layer (the two
   arms of `charge_node`'s `match`), so the answer of `charge_node` does not change. No
   member of this slice can reach it, since the handle is this engine's own and `alive()`
   holds while a member runs. `charged` changes only when the engine answered `Ok(Ok(()))` or
   made the handle: on `Fault` it changes nothing, and `live` neither, and no settle follows,
   because the member returns the `Err` at the `?` of `charge_stored`, before any tree call,
   and the run is over. `charge_node` becomes a wrapper of `charge_to` for its own per-node
   handle, so its answer and its totals do not change; the mutation story later keeps one
   node handle through the same body (Placement).
   The variant. `DomEnd` (`prelude.rs:54-65 @ 83e6b8a`, written to `bindings.rs:119-129`, the
   enum of `story:webidl-bindings`, decision 9) gains `Fault`, with the doc comment "A defect
   of surf's own code that no page causes: the run ends faulted, as `defect` does, and no
   script can catch it." The derives (`Clone, Debug, PartialEq, Eq`) hold for a variant with
   no field. The one `match` on `DomEnd` at `83e6b8a` is in `answered` (`prelude.rs:862-869`,
   written to `bindings.rs:958-965`), which the glue of every member reaches through `finish`
   (`prelude.rs:851-859`); it gains the arm `Err(DomEnd::Fault) =>
   Err(Abrupt::Terminated(Termination::EngineFault))`, the value `defect` answers. `in_dom`
   (`bindings.rs:969-983`) passes a `Terminated` through: it changes only an `Abrupt::Throw`
   beside a noted `TypeError`. The runner then ends the entry `ScriptEnd::Faulted`
   (`terminated`, `runner.rs:918 @ 83e6b8a`) and marks itself faulted (`runner.rs:679`). Every
   other use of `DomEnd` constructs it and matches nothing: the reading slice (`reads.rs:39`,
   `:126`, `:148`, `:187`, `:1087`, `:1139`, `:1145`), `charge_node` (`prelude.rs:230`),
   `not_supported` (`prelude.rs:87`) and the nine files of `crates/script/tests` that name it
   (`git grep -n DomEnd 83e6b8a -- crates/script/tests` has no `match` and no `matches!`); the
   generator's source, `crates/webidl/src/generate.rs`, does not name `DomEnd` and copies the
   prelude text whole (`:1902`), so it does not change, and `bindings.rs` changes by
   regeneration only. The name registry of `generate.rs` (`:535-562`) reads items at the
   start of a line of the prelude: the variant is indented and adds none.
   The prelude gains, beside `charge_node`, the functions that a slice calls. Their names and
   signatures are final; `story:scripts-dom-mutation` calls them for its character-data writes
   (Placement). Both take the sums of one tree call: a call that changes several nodes sums
   `added` and `removed` over the whole call before it, calls `charge_stored` once and
   `settle_stored` once with the same two sums, so that the handle is sized once for the call:
   - `DomCx::charge_stored(&mut self, added: usize, removed: usize) -> DomResult<()>`: before a
     write. The new live is `next_live(live, added, removed)` and the wanted charge is
     `STORED_FACTOR` times that. When the wanted charge is above `charged` it is a grow: the
     handle is made (first) or resized to the wanted charge through `charge_to`, and over the
     budget nothing changes (neither `live`, `charged` nor the tree) and the run ends
     `Terminated(MemoryBudget)`, `limit: memory`, the answer being a throw the engine does not
     deliver, as with `charge_node`. When the wanted charge is not above `charged` it does
     nothing: the shrink comes after the write. It changes `charged` and never `live`: two calls
     with no settle between them are a defect, since the second computes from the `live` that
     the first left, and does not see the first;
   - `DomCx::settle_stored(&mut self, added: usize, removed: usize) -> DomResult<()>`: after a
     write. The next live is `next_live(live, added, removed)` and the wanted charge is
     `STORED_FACTOR` times it. When the wanted charge is below `charged` it resizes the handle
     down to it through `charge_to`; when it is equal it makes no engine call. It sets `live`
     to the next live and `charged` to the wanted charge, and answers `Ok(())`, only when the
     engine answered `Ok(Ok(()))` or was not called: on any `Err`, `charged` and `live` stay
     as they were. The engine cannot refuse a shrink or an equal size for memory: on its
     `Less` arm `scoped_resize_host_bytes` credits the difference (`accounting.credit`) and
     its `Equal` arm does nothing, and neither has a step that answers an `Err`
     (`host/host_bytes.rs:97-101` at `0.8.0`, the same file at `0.10.0`). The only inner `Err`
     it can answer for them is the one of `running()` at `:88`, a run already terminated,
     which cannot hold while a member runs, and its outer `Err` is `alive()` at `:84` (a
     faulted agent) or the foreign handle (`:85-87`). Both layers are therefore a defect
     here, and `settle_stored` has one meaning for them: an outer `Err(HostError)` is
     `DomEnd::Fault` as in `charge_to`, and an inner `Err` (which `charge_to` has made a
     `DomEnd::Throw`) is made `DomEnd::Fault` by `settle_stored`, so that a script never
     catches it and a handle in an unknown state never goes on, in a release build too. It
     never grows the handle: a settle whose wanted size is above `charged` is a caller's
     defect; it sets `live`, leaves `charged` and answers `Ok(())`, and a `debug_assert!`
     names it;
   - the accessors `DomState::stored_live() -> u64`, `DomState::stored_charged() -> u64`,
     `DomState::stored_handles() -> usize` (0 or 1) and `DomState::node_charges() -> usize`
     (the length of `charges`), which the tests read beside `DomState::totals()`
     (`prelude.rs:157`), and two of them on `DomCx`, `DomCx::stored_live() -> u64` and
     `DomCx::stored_charged() -> u64`, beside `DomCx::totals()` (`prelude.rs:189`), which a
     test slice reads from inside a member, where it holds a `DomCx` and no `DomState`.
     At this story's build `charge_node` still pushes one handle for each
     node into `charges`, so `node_charges()` is the node count; `story:scripts-dom-mutation`,
     built second, keeps one node handle grown through `charge_to`, turns `charge_node` into a
     wrapper of that, and then defines `node_charges()` as the count of node handles (0 or 1).
     The tests of this story that read `node_charges()` assert only what holds under both
     meanings: that it does not change across a write of this story (acceptance 6);
   - the doc comments of `tree_mut` (`prelude.rs:177-181`) and of `Totals::bytes`
     (`prelude.rs:92-99`) name them.
   So the charge is twice the net rise of the stored bytes, and not their peak: a loop that sets
   and removes one attribute keeps the handle at one size, and removing what a script wrote
   lowers the charged bytes at once. The node charge of `charge_node` stays as it is: a node is
   never taken out of the tree's arena (`tree.rs:19-21`), so nothing of it is ever released,
   where a stored byte is freed at a removal (`drop_attribute`, `tree.rs:1528-1534`). A removal
   of a byte that the parser stored lowers `live` when `live` is above 0, and the handle gives
   back twice the lowered part: the refund is never more than was charged, since the handle is
   twice `live` and `live` stops at 0. A removal from a node that a script made refunds bytes of
   this handle only, never of the node charge. The handle is never below twice the net rise of
   the stored bytes since the parse (the floor at 0 only ever keeps it higher): the tree holds at
   most what the parse stored plus that rise. What the handle does not count is the parser's own
   bytes, which were never charged to it; a script that removes `r` parser bytes while `live` is
   below `r` takes `live` to 0 and refunds only what the handle held.
   The tree's refusal and the order of a member. The write the tree refuses leaves the tree as
   it was (`tree.rs:12-23`), whatever its kind (a limit, `Namespace`, `InvalidNodeType`), and
   that is what the rule assumes. After any `Err` of the tree call the member calls
   `settle_stored(0, 0)` before it returns the end, which puts the handle back to twice `live`,
   the bytes of the tree as they were; a `?` on the tree call between `charge_stored` and
   `settle_stored` is a defect. One private function in `attribute_writes`, `charged_write`,
   holds this order and is the only place of the slice that calls `charge_stored`, the tree's
   write and `settle_stored`: `charged_write(cx, added, removed, write)` calls `charge_stored`
   (its `?` is before any change), calls `write(cx.tree_mut())` (the tree call) and holds its
   answer, not propagating it, and then:
   - on `Err(end)` it calls `settle_stored(0, 0)`; if the settle answers `Fault` the function
     answers `Fault` and the tree's `end` is dropped (the run ends faulted, and the tree's
     error would be taken by nobody); else it answers the end of decision 5;
   - on `Ok(value)` it calls `settle_stored(added, removed)`; if the settle answers `Fault`
     the function answers `Fault`, the write stays in the tree and the run ends faulted; else
     it answers `Ok(value)`. After such a settle `charged` holds the grown size and `live` the
     old one, since a failed settle changes neither; a later script of the runner is `not run:
     faulted` (`runner.rs:626-641 @ 83e6b8a`), so no script reads the pair again.
   The settle's own answer is taken after the settle, in the statement that calls it
   (`settle_stored(..)?;`): the `?` that is a defect is the one on the tree call, between the
   charge and the settle, and the source test of acceptance 10 holds it. The eight writers
   pass `charged_write` their closure, so no writer repeats the order.
   The order of a member of this slice, all of it in `attribute_writes`: validate (decision 4);
   `find`, which charges the scan and answers the sizes; compute the bytes the write adds and
   removes by the tree's count; charge the steps of decision 7, the last of them the byte
   charge that precedes the tree call; `charged_write`. The bytes, from what `find` answers
   (`Found`, `None` for none):
   - no attribute found, and the write adds one: `added` is `ATTRIBUTE_BYTES` plus the prefix,
     the local name and the value: `setAttribute` of a name not present (the lowered name as
     local name, no prefix); `setAttributeNS` of a pair not present (the prefix and the local
     name of the split at the first colon); `toggleAttribute` that adds (the lowered name and the
     empty value); the setters of `id`, `className` and `slot` (the local name `id`, `class` or
     `slot`);
   - an attribute found, and the write replaces its value (`setAttribute`, `setAttributeNS`, the
     setters): `added` is the new value's bytes and `removed` the old value's (`value_bytes`),
     since the tree replaces the value only and keeps the name and prefix (`tree.rs:1509-1514`);
   - an attribute found, and the write removes it (`removeAttribute`, `removeAttributeNS`,
     `toggleAttribute` that removes): `removed` is `stored_bytes` of it and `added` is 0; no
     grow is charged and the settle after the write is the refund;
   - an attribute found and `toggleAttribute` with `force` true: the tree writes nothing and
     answers `true` (`tree.rs:1501-1526` for the put, and the arm `Some(_) if force ==
     Some(true) => Ok(true)` of `toggle_attribute`, `tree.rs:1650 @ 83e6b8a`), so the member
     makes no call of `charge_stored` or `settle_stored` and no tree call; the steps of the scan
     stay charged;
   - nothing found and nothing added (`removeAttribute` of an absent name, `toggleAttribute`
     with `force` false): no call.
   The bytes the tree counts are the bytes it holds: a value is held in a `String` of its exact
   length, a name and a prefix too, and `put_attribute` reserves exactly while the list has
   fewer than four entries (decision 3), so no counted byte is a byte the list holds twice over
   for the first four; beyond four the list grows by the `Vec` rule, at most twice, which the
   factor of two covers. With the defaults a script on a small page reaches `limit: memory`
   before `limit: stored-bytes`, and `limit: attributes` before `limit: memory` when its
   attributes are short (acceptance 6b, 6c). The tree is the document's one tree, lent to each
   script by `story:scripts-parse-run` (its `DomState::swap_tree`); `stored`, `live` and
   `charged` stay in `DomState` and that swap leaves them alone.
9. A deviation of `setAttributeNS` that this story names in the note of its row
   (`selection.txt:96`): a namespace that is none of the six of `Namespace` cannot be stored,
   so the call throws `NotSupportedError` after validate and extract has passed and before the
   scan and the charge, with the message `Element.setAttributeNS is not supported` through
   `DomException::not_supported` (`bindings.rs:107`). The epic's "Corrections of 2026-11-25" holds
   a bullet for this deviation since plan commit `60a4317` (Change); this story edits no line of
   the epic. `story:html-open-namespace` (a draft in the plan store, commits `2e38276` and
   `7226f2e`) lifts it: a tree that stores any namespace, and `setAttributeNS` that then stores
   one; until it merges the deviation stays.
10. Kept objects, `[SameObject]` and cached answers. No row of this slice is `[SameObject]`: no
    member returns an object, and the `SameObject` enum (`bindings.rs:2051-2060`) keeps its
    four variants. The two `[SameObject]` attributes of `Element` next to this slice,
    `classList` (`selection.txt:88`) and `attributes` (`selection.txt:91`), are rows of
    `throws`: the generator's `throws` arm answers `NotSupportedError` on every read and does
    not consult `SameObject`, so no object is ever returned and none needs to be held
    (the reading story's decisions 10 and 18); a story that gives either a behaviour holds it as
    those decisions say before it removes the row from `throws`. The slice keeps no collection
    and no cached answer: each read scans the attributes. A write of this slice raises no
    counter: no attribute write calls `wrote()` (Context), so the kept `childNodes` and
    `children` objects and the sibling-hint table, which are filled from child lists, stay valid
    and nothing is refilled; this slice records no cut refill and puts nothing back on a throw
    except the stored handle, which `settle_stored(0, 0)` puts back to twice `live`
    (decision 8). If a later story keeps an answer that depends on an attribute (a
    list by `id` or by class), its write must raise a counter that story names, and the case of
    acceptance 4b changes with it.
11. The cases are the rows. The table, the row names (`Element.id`, `Element.setAttribute`), the
    helper and the equality test are those of the reading story's decision 15, but they are not
    shared code: `Case`, `case()`, `CASES`, `run_case`, `limits()`, `with_page`, `recorded` and
    `cases_are_the_rows` are private to the module `dom_reading` (`dom_reading.rs`), so the
    module `dom_attributes` defines its own copies of the type, the helper (which runs
    `run_document(limits(), Mode::Test, ..)`, evaluates `expression === expected`, asserts
    `ScriptEnd::Ran` and asserts no recorded limit) and the equality test, with its own
    `limits()`. A case that needs a limit of the parse of its own, or reads steps or totals, is a
    test beside the table that builds its page and its `Document` itself, because the helper
    asserts that no limit is recorded.
12. A test builds its tree with `parse_document(page, &ParseOptions { limits, .. })` and gives
    the same `limits` to `Document { limits, .. }` (`runner.rs:434-438`); the write surface
    reads the tree's. A test that reads the totals reads `runner.document()` (`runner.rs:191`),
    its `totals()`, `stored_live()`, `stored_charged()`, `stored_handles()`, `node_charges()`
    and its tree's `stored_bytes()` (`Tree::stored_bytes`, `tree.rs:594 @ 83e6b8a`, the bytes
    the tree stores: the accessor that acceptance 6b reads). A value
    of many characters is built by the script with `"x".repeat(n)`, which the engine has
    (`crates/script/tests/adversary_3.rs:796`).

## Change

- `crates/html/src/tree.rs`: the three reads of decision 3 (`attribute_position`,
  `attribute_position_ns`, `attribute_names`), each with a doc comment that says what it
  matches, what it answers for a node that is not an element and, in a line that starts with
  `Cost:`, what it costs; the three owners (`Namespace::from_url` beside `Namespace::url`,
  `Tree::namespace_error`, `Attribute::qualified_len` beside `Attribute::stored_bytes`);
  `Attribute::stored_bytes`, which the private `attribute_bytes` then calls; the five writes
  that scan call the private owners of the position rule, `position_of` and the new
  `position_of_ns` (which `attribute_position_ns` calls too), so that a read and a write share
  them (so `remove_attribute`, `remove_attribute_ns` and the
  removing arm of `toggle_attribute` make no copy of the name, and `toggle_attribute` lowers
  the name into a `String` only in its adding arm), `set_attribute_ns` calls `namespace_error`
  with the split it makes, and `put_attribute` reserves exactly while the list has fewer than
  four entries; the module
  documentation's sentence on what the surface leaves to its caller (`tree.rs:21-23`) gains
  the reads. No answer of an existing function changes. The reading story edits this file too
  and merges first (Placement).
- `crates/html/tests/script_hook.rs @ a14e3b0`: the tests of the reads and owners, in the
  module `script_hook` where the write surface's tests are (`write_set_attribute` is at
  `:1157`, `write_set_attribute_ns` at `:1197`, `write_remove_attribute` at `:1243`,
  `write_remove_attribute_ns` at `:1259`, `write_toggle_attribute` at `:1274`):
  `read_attribute_position`, `read_attribute_position_ns`, `read_attribute_names`,
  `attribute_stored_bytes`, `attribute_qualified_len`, `namespace_from_url`
  and `namespace_error_conditions`; the module is declared in `crates/html/tests/main.rs`, so
  no `mod` line is added. The test `put_attribute_reserves_exactly` is a unit test in the
  `#[cfg(test)]` module of `tree.rs` (`:1659-1692 @ 83e6b8a`, beside the test that reads
  `attributes.capacity()` at `:1691`), because it needs the private list: it reads
  `attributes.capacity()` after each of the first four adds. The count is not a tree read:
  the slice calls `reads::attribute_count` (decision 2), so there is no
  `Tree::attribute_count`.
- `crates/webidl/src/prelude.rs`: the fields `stored`, `live` and `charged` of `DomState` and
  their initial values in `DomState::new`, the constant `STORED_FACTOR`, the private
  `next_live` and the associated function `charge_to` of `impl<'e> DomCx<'_, 'e>` (`charge_node`
  calls it and pushes the handle it makes), `DomCx::charge_stored`, `DomCx::settle_stored`, the
  accessors `DomState::stored_live`, `stored_charged`, `stored_handles` and `node_charges` and
  `DomCx::stored_live` and `DomCx::stored_charged` beside `DomCx::totals()` (`:189`), the
  doc comments of decision 8, the variant `DomEnd::Fault` (`:54-65`) with its arm in `answered`
  (`:862-869`) and the answer of `charge_to` to an outer `Err(HostError)` (decision 8), and the
  charge of `STRING_ELEMENT_STEPS` for each element in
  `strings_value` (decision 7); `crates/script/src/bindings.rs` regenerated with `slot cargo
  run -p surf-webidl --bin bindings`, which is its only change: it is written from the prelude
  text, so the variant and the arm reach it by regeneration and no line of it is edited by
  hand. The header's digests that change are those of `crates/webidl/selection.txt`, of
  `crates/webidl/src/prelude.rs` (lines 46 and 59 at `83e6b8a`) and the whole-file digest, header
  line 72 (`//! The sha256 of this whole file ...`); the other lines of the digest block stay as
  they are on the merged `main`.
- `crates/webidl/src/generate.rs`: no change. It does not name `DomEnd` (`git grep -n DomEnd
  83e6b8a -- crates/webidl/src` finds only `prelude.rs`), it writes the prelude text whole
  (`:1902`), and its registry of names (`:535-562`) takes only items at the start of a line,
  which the indented variant is not. The scope names no file of the generator beyond
  `prelude.rs`, and acceptance 10 holds that `generate.rs` stays out of the diff.
- `crates/webidl/selection.txt`: the note of `Element setAttributeNS` (line 96) gains the
  deviation of decision 9; the doc comment of the method carries the note (`bindings.rs:3200`).
- The epic, `.engineering/planning/epic/scripts-in-a-page.md`: the bullet of this story in the
  section "Corrections of 2026-11-25" is in the plan store since plan commit `60a4317`. This
  story edits no line of the epic and its scope list does not name the file; the corrections
  of the bullet were made in plan commits `4c76597`, `9a88a16` (the edits the bullet leaves
  out, the handle's rule, the departure from `epic:880`) and `dfec030` (the private position
  functions, the one charge of `strings_value`), outside this story.
- `crates/script/src/dom/attribute_reads.rs` (new): `reflected`, `any`, `names`, `get`,
  `get_ns`, `has`, `has_ns`, `find`, `namespace_of`, the private `copy_value` and `Found`, and
  the scan charge (decisions 2, 7).
- `crates/script/src/dom/attribute_writes.rs` (new): `set_reflected`, `set`, `set_ns`, `remove`,
  `remove_ns`, `toggle`, the valid attribute local name and valid namespace prefix tests, the
  call of `Tree::namespace_error`, the bytes a write adds and removes, the charge and the
  settle of a write through `charged_write`, also after one the tree refuses (decisions 2, 4,
  5, 7, 8, 9).
- `crates/script/src/dom/attributes.rs`: the 17 methods, each one call (decision 2).
  `SurfAttributes` stays a unit struct: `dom::slices(info)` builds it with no argument
  (`dom/mod.rs:49-56`), and the members need no document info (the document is an HTML
  document, so there is no quirks-mode rule in an attribute name).
- `crates/script/src/dom/mod.rs`: the two `mod` lines. `crates/script/src/dom/reads.rs`: the
  three `pub(super)` tokens (decision 2).
- `crates/script/tests/dom_attributes.rs`: the module `dom_attributes`, already declared in
  `crates/script/tests/main.rs:17`: the table `CASES` with its own helper, the equality test, and
  the limit, charge, price and cross-slice tests of acceptance 2 to 8.
- `crates/script/tests/dom_attributes_cost.rs` (new) and one line, `mod dom_attributes_cost;`,
  after `mod dom_attributes;` in `crates/script/tests/main.rs`: the cost module of acceptance 11
  and the source test of acceptance 10. It is modelled on `dom_reading_cost.rs` (its `limits()`
  of 50,000,000 steps, 256 MiB and a `wall_clock` of 50 s, `raised()`, `call`, `report`,
  `plain`); it defines its own copies of those helpers and sets its own parse limits (the list
  in acceptance 11).
- `crates/script/tests/bindings.rs`: the test slice `TestMutation(Writes)` (`:150`) becomes
  `TestMutation(Writes, Record)`, so that its member can push onto the `Record` the test
  shares with `handed.taken()`; `test_document` (`:214-226 @ 83e6b8a`) builds it with the
  `Record` it also gives `TestAttributes` (cloned: the `Record` is an `Arc`). Two variants of
  `Writes`, `Stored` and `Fault`, and the two exhaustive matches on `Writes` name them
  (`document_create_element`, `:160-165`, answers `not_supported("Document.createElement")`
  for both, as for `Charge`; `document_create_text_node`, `:174-181`, gets the arms below). The
  arm of `Writes::Stored` in `Document.createTextNode` reads its argument as two numbers,
  `added removed`, and calls `charge_stored` with them as the `Writes::Charge` arm calls
  `charge_node` (`tests/bindings.rs:175-178 @ 83e6b8a`), and then calls `settle_stored` with
  the same two numbers; the last character of the argument picks `0 0` for the settle (`!`, a
  write the tree refuses), the same with a throw that is not a limit (`?`), or a stop after the
  charge that records `stored_charged()` before and after the settle (`~`, read through
  `DomCx::stored_charged()`); and the test of acceptance 6f beside `node_charge`
  (`tests/bindings.rs:917-954`); and a variant `Writes::Fault` whose `Document.createTextNode`
  answers `Err(DomEnd::Fault)` (`TestMutation`, `tests/bindings.rs:150-183`), with the test of
  acceptance 6h beside `limit_end` (`:803`).
- `crates/script/tests/dom_reading.rs`: `other_slices_still_throw`
  (`dom_reading.rs:2399-2432`) asserts that `el.getAttribute("id")` throws (line 2425, message
  `Element.getAttribute is not supported`); it changes to `el.getAttributeNode("id")` with the
  message of a row of `throws`, `Element.getAttributeNode is not supported: attribute nodes`
  (`bindings.rs:2042-2047` makes the message `<member> is not supported: <reason>`). One
  line, and no other: the test holds three adjacent lines, `document.appendChild(el)` (the
  mutation slice's), `el.getAttribute("id")` (this slice's) and `el.querySelector("p")` (the
  selectors slice's); each slice's story changes its own line and leaves the other two
  (Placement).
- `docs/design/scripts.md`: the paragraph on the charge of the DOM a script makes
  (`:322-332 @ 83e6b8a`) and the line that lists what `memory_bytes` holds (`:902-904 @
  83e6b8a`) say that the bytes a script writes into a node that exists are charged too: twice
  the bytes that scripts hold in the tree now, through one handle that the engine resizes,
  before the write and refunded by a write that removes bytes, where a node's charge is never
  refunded because a node is never taken out of the arena.
  The mutation story's Change edits the same two paragraphs for its own rules (Placement).
- `AGENTS.md`: the entry of `crates/script` (`AGENTS.md:365-369`) names the slice `attributes`,
  its three files and its test count.
- In the last commit, the public surfaces of acceptance 13: `README.md`, `docs/badges/make.sh`,
  `docs/badges/tests.svg`, `website/docs/how-it-is-built.md`, `website/docs/status.md`,
  `website/docs/what-works.md`.

## Acceptance

1. The rows and the table (decision 11).
   a. `dom_attributes::cases_are_the_rows` reads `crates/webidl/selection.txt` as `selection()`
      does (`crates/script/tests/bindings.rs:283`) and asserts that the set of `Case` row
      names equals the rows with the slice `attributes`: 14, named `Element.id`,
      `Element.className`, `Element.slot`, `Element.hasAttributes`,
      `Element.getAttributeNames`, `Element.getAttribute`, `Element.getAttributeNS`,
      `Element.setAttribute`, `Element.setAttributeNS`, `Element.removeAttribute`,
      `Element.removeAttributeNS`, `Element.toggleAttribute`, `Element.hasAttribute` and
      `Element.hasAttributeNS`; every entry has a non-empty expression and expected value.
   b. Each entry compares what the member answers, and the element's attributes read back
      through the tree or through `getAttribute`, with what the DOM Standard says, the section
      cited in the entry's comment. Each exception case is named by its exception.
   c. The entries hold at least: the getters and setters of the three reflected attributes (the
      empty string when absent; `id` written, then `getElementById` finds it); a name in
      capitals on an HTML element (`getAttribute("ID")`, `setAttribute("DATA-X", "1")` stored
      as `data-x`, `removeAttribute("ID")` and `toggleAttribute("ID")` on `<p id="a">` each
      removing `id`) and on an SVG element, whose names the parser keeps in mixed case
      (`getAttribute("viewBox")` answers, `getAttribute("viewbox")` is `null`); on an HTML
      element, `setAttributeNS(null, "FOO", "1")` stores `FOO` as given, so that
      `getAttribute("FOO") === null`, `getAttributeNS(null, "FOO") === "1"` and
      `hasAttribute("foo") === false` (the stored byte is not folded, decision 3); an attribute in
      a namespace from the parse (`xlink:href` of an SVG `use`) read by `getAttributeNS` with
      the XLink URL and listed by `getAttributeNames` as `xlink:href`; `setAttribute` replacing
      a value in place (the order of `getAttributeNames` is kept); `removeAttribute` of an
      absent name; the four forms of `toggleAttribute` (no `force`, `true`, `false`, and an
      added attribute with the empty value) with their answers.
   d. The exceptions: `InvalidCharacterError` from `setAttribute` and `toggleAttribute` for the
      empty name and for names holding a space, `=`, `/`, `>` and U+0000, and from
      `setAttributeNS` for `":a"`, `"a b"` and `"x:a b"`; `NamespaceError` from `setAttributeNS`
      for a prefix with a null or empty namespace, `xml:a` in another namespace, `xmlns` and
      `xmlns:a` in another namespace, and the XMLNS namespace with another name; each with the
      tree unchanged and `totals().bytes` and `stored_charged()` unchanged.
      `setAttribute("a:b", "v")` is not an error
      (no colon test).
   e. `setAttributeNS` in a namespace outside the six throws `NotSupportedError` naming the
      member, after the checks of decision 4 (`xml:a` in an outside namespace throws
      `NamespaceError`, not `NotSupportedError`) and with `totals().bytes` unchanged;
      `getAttributeNS`, `hasAttributeNS` and `removeAttributeNS` in it answer `null`, `false`
      and `undefined`, and the same calls on an element of 4 attributes and on one of 1,024
      raise `runner.steps()` by equal amounts (no scan).
   f. The table tests run each entry through the helper, so each case makes its own runner and
      asserts the end `Ran` and no recorded limit.
   g. A guard, true on `main` in substance (the glue's brand check runs before the slice;
      only the `stored_charged()` assertion is new): the brand check, which the epic promises ("a
      member called on an object of another brand
      throws a `TypeError`", `epic:581 @ a14e3b0`) and which `tests/bindings.rs:419 @ 83e6b8a`
      samples for one member and one getter only. Three cases, in the form of that test, each in its
      own `try`: `Element.prototype.setAttribute.call(document, "a", "b")`; the `id` setter
      taken from the property descriptor of `Element.prototype` and called on `document`; and
      `Element.prototype.getAttribute.call(textNode, "a")` with `textNode` the `firstChild`
      (`selection.txt:56`) of a `<p id="t">x</p>` of the page. Each
      throws a `TypeError` (not another exception and not an end of the script), and leaves
      the tree, `totals().bytes` and `stored_charged()` as they were.
   h. The messages. `setAttribute` with a name of 1,048,576 bytes that holds a space throws an
      `InvalidCharacterError` whose `message` equals that of the same call with the name
      `"a b"`, and is the constant of decision 4 (`Element.setAttribute: the name is not a
      valid attribute name`); `setAttributeNS` with a namespace that does not fit a 1 MiB name
      throws a `NamespaceError` whose `message` equals that of the short case and is the
      constant of decision 4. The length of a message does not depend on an argument.
2. Limits of the tree (outside the table). Each case parses a page with
   `ParseOptions { limits, .. }` of its own and gives the same limits to `Document`.
   a. With `max_attributes` at 3: `setAttribute`, `setAttributeNS`, `toggleAttribute` and the
      `id` setter, each adding a fourth attribute, end `ScriptEnd::Limit(Reason::Attributes)`
      (`limit: attributes`), also inside `try { ... } catch (e) {}`; the `catch` and a `finally`
      do not run (a flag a later script reads stays unset); the element keeps its three
      attributes and their values. Replacing the value of an existing attribute at the limit
      ends `Ran`.
   b. With `max_stored_bytes` just above what the parsed page holds: `setAttribute` of a new
      attribute whose value would pass it, and of an existing attribute to a longer value,
      end `limit: stored-bytes`, the same way, and the value is unchanged. `removeAttribute`
      of a large attribute, then the same `setAttribute`, ends `Ran`: the removal freed the bytes.
   c. `removeAttribute` and `removeAttributeNS` record no limit at any state; a later script of
      the same runner runs after each end above (the record is cleared).
   d. The members map `WriteError::Limit(limit)` to `DomEnd::Limit(limit)` for any `limit`
      (decision 5): `git ls-files crates/script/src/dom/attribute_writes.rs` prints the path and
      `git grep -n -e 'Limit::Nodes' -e 'Limit::Depth' -e 'Limit::InputBytes' -e 'Limit::Steps'
      -- crates/script/src/dom/attribute_writes.rs` prints nothing, and no case asserts one of
      those four ends (no write of this slice reaches them, Context).
3. The write that the epic's example 2 needs. On a parsed `<a href="/old">`,
   `a.setAttribute("href", "/next")` leaves the tree's attribute `href` as `/next`, read through
   `runner.document()` and `Tree::data`; `a.setAttribute("x", { toString() { return "y" } })`
   sets `y`, and `a.setAttributeNS(null, "x", { toString() { return "y" } })` sets `y` (the
   `TrustedType` deviation of rows 95 and 96, one case for each row; `Element.setAttribute` has
   the note, `bindings.rs:3189`); a lone surrogate in a value is stored as U+FFFD
   (`selection.txt:24`).
4. Reading what a write made, through members of the reading story, in one script each: after
   `el.id = "z"`, `document.getElementById("z") === el` and the old id finds nothing; after
   `el.className = "k"`, `document.getElementsByClassName("k")` holds `el`; `el` is the same
   wrapper object before and after every write of this story.
   a. Kept objects. `p.children` and `p.childNodes` read before and after
      `child.setAttribute("x", "1")`, `child.removeAttribute("x")` and `child.id = "y"` are
      the same object each time and hold the same nodes.
   b. The tree's write counter. Each of the 8 writers of this story (the three setters,
      `setAttribute`, `setAttributeNS`, `removeAttribute`, `removeAttributeNS`,
      `toggleAttribute`), run once on an element, leaves `runner.document()`'s
      `tree().writes()` where it was, and so do the 9 readers. Each writer's case also asserts
      that the write took effect, read back through `getAttribute` or the tree (the attribute
      is set, changed or gone), so a writer that throws fails the case and the unchanged
      counter is not vacuous. This case is the held statement of decision 10: no kept object
      and no cached answer depends on an attribute.
5. Prices (decision 7). `runner.steps()` is read around each script, one call a script, on the
   session's limits; a floor is the sum of the charges the call makes, and each floor is stated
   with its terms in the case.
   a. `getAttribute` of an attribute at the end of an element of 1,024 attributes named `a0` to
      `a1023` raises `runner.steps()` by at least 1,024 / 4 - 4 / 4 = 255 more than the same
      call on an element of 4 attributes (the terms `k * n / S` only add to it); `hasAttribute`,
      `getAttributeNS`, `hasAttributeNS` and the `id` getter likewise. `getAttributeNames` of
      the same element raises it by at least 1,024 (the slice's) + 8 * 1,024 (`strings_value`)
      = 9,216.
   b. `getAttribute` of a value of 64,000 bytes raises `runner.steps()` by at least
      2 * (64,000 - 64) / 16 = 7,992 beyond the same read of a value of 64 bytes (the slice's
      charge and the glue's charge for the String it answers), and `setAttribute` of such a value
      raises it by at least 7,992 for the glue's copy of the argument ((64,000 - 64) * 2 / 16,
      `units * 2 / 16`) and 3,996 for the tree's copy ((64,001 - 65) / 16), 11,988 in all (the
      floor assumes the reading unit's price commit, acceptance 14).
   c. Before the work: one helper, `budget_row`, over rows, in `dom_attributes`, one row for each
      of the eight writers and each member that returns a value. The glue's own copy of a 1 MiB
      argument costs 131,072 steps (1,048,576 units * 2 / 16), so a budget under that ends the
      script in the glue and shows
      nothing about the slice; each row sets the budget between the charges that come before the
      slice's last charge and that charge (the rows of a removal that finds nothing are the
      exception: their last part is the glue's charge, and their weight is the upper bound
      below). A row names its page, a setup script, the call, and
      `parts`, the list of the charges the call makes in their order, each computed from the
      prices of decision 7 (the arithmetic is in the row); a row whose page holds more than
      1,024 attributes (the 4,096 of the names row, the 16,384 of the removal rows) parses it
      with `max_attributes` of 16,384 and the other parse limits as the default. The helper
      makes four runs of the
      row, each on its own runner, in one `#[test]`: each row is its own `#[test]` that calls the
      helper, so the 60 s bound applies to a row. A row builds its page string in its own test
      (under nextest a value shared between tests is built once for each test); each run parses
      its own tree from it, because a run that writes changes its tree:
      - S0 is `runner.steps()` after a run of the setup script alone, with `steps_per_entry`
        at the session's value (`runner.rs:41`); S1 is `runner.steps()` after a run with the
        call added, with the same value. The helper asserts that both runs ended `Ran`: a setup
        that ended the script early would prove nothing. C = S1 - S0 is what the call costs.
        The helper asserts `sum(parts) <= C <= sum(parts) + 1,000`: M = C - sum(parts) is the
        rest (the glue's one step, a scan of 0 attributes, the lowering), measured, and a few
        steps, not a term that hides the others.
      - B = S0 + M + (every part but the last) + last / 2, which is S0 + C - last / 2 less the
        rounding of the half. The run with the call and `steps_per_entry` set to B ends `limit:
        steps-per-script`, and for a writer the tree and `stored_charged()` are as before the
        call and the attribute is as before (nothing was written): every charge but the last
        passed and the last did not fit. The run with `steps_per_entry` set to B + last ends
        `Ran` and shows the effect of the write read back through the tree. A slice that charges
        its last part after the write ends with B at the same place with the attribute written,
        and fails the first assertion; so does one that charges half of it before the write and
        half after, and one that charges after `charge_stored` (the charge for memory), because
        the stored handle has then grown.
      - The writer rows, on the page `<p></p>` (the `html` element has no attribute) with the
        script `var e = document.documentElement; var v = "x".repeat(1048576);` for the
        setters and `var n = ...` as the row says; `parts` for each, from decision 7 (the
        glue's charge for each argument, then the slice's charges in order):
        `setAttribute("a", v)`: [131,072 (the glue's copy of `v`, 1,048,576 * 2 / 16; the name of
        1 unit costs 0), 65,536 (the bytes the tree copies, (1 + 1,048,576) / 16)];
        `e.id = v`: [131,072, 65,536 ((2 + 1,048,576) / 16)];
        `e.className = v`: [131,072, 65,536 ((5 + 1,048,576) / 16)];
        `e.slot = v`: [131,072, 65,536 ((4 + 1,048,576) / 16)];
        `setAttributeNS("http://www.w3.org/2000/svg", n, "")` with `n = "x:" +
        "y".repeat(1048574)`: [131,075 (the glue: 1,048,576 * 2 / 16 = 131,072 for the name,
        26 * 2 / 16 = 3 for the URL), 65,536 (the one pass of validate and extract over the
        whole qualified name, 1,048,576 / 16, which finds the colon and tests prefix and local
        name together), 65,535 (the bytes the tree copies, (1 + 1,048,574) / 16 = 65,535; the
        colon is not stored)]; `toggleAttribute(n)` with `n = "y".repeat(1048576)`, adding it:
        [131,072 (the glue), 65,536 (the validation), 65,536 (the bytes the tree copies,
        (1,048,576 + 0) / 16)]. Each row's effect is the attribute set to `v` (or `n` added, in
        the SVG namespace for `setAttributeNS`), read back through the tree.
      - The row of a value replaced, on the page `<html a="">` with the same script:
        `setAttribute("a", v)` finds `a`: [131,072 (the glue), 65,536 (the bytes the tree
        copies, (1 + 1,048,576) / 16: `set_attribute` copies the name also when `put_attribute`
        keeps the old one, decision 3)]; the scans of an element of 1 attribute charge 1 / 4 = 0 and
        `k * n / S` is 1 * 1 / 16 = 0. The effect is the value `v`, and B leaves the value as the
        page had it, the empty string.
      - The removal rows, on a page `<html n0="" n1="" ... n16382="" LONG="">` whose last
        attribute is named by 1,048,576 `z` bytes (`LONG` stands for that name here, and the test
        writes it as `"z".repeat(1048576)` into the page string; 16,384 attributes; the test
        builds the page string once and parses it with the raised `max_attributes`); `k` is 1, as no
        other name has that length. With `n = "z".repeat(1048576)`: `removeAttribute(n)`: [131,072
        (the glue), 4,096 (`find`'s scan, 16,384 / 4), 65,536 (`find`'s bytes, 1 * 1,048,576 /
        16), 69,632 (the tree's own scan, one charge call: 16,384 / 4 + 1 * 1,048,576 / 16 =
        4,096 + 65,536; its byte term is the only pass over the name's bytes that the tree
        makes, because the removal copies nothing)]; `removeAttributeNS(null, n)` likewise, with
        the glue's charge for the local name and the scans by pair (`k * l / S`, `l` the same
        1,048,576); `toggleAttribute(n)` removing it: [131,072 (the glue), 65,536 (the
        validation), 4,096, 65,536, 69,632]. The last part is the one charge of 69,632 and the
        budget B ends in it, 34,816 steps into it, with the attribute still on the element; the
        effect is the element without it.
      - The rows of a removal that finds nothing, on the page `<p></p>` with
        `var e = document.documentElement; var n = "z".repeat(1048576);`: `removeAttribute(n)`:
        [131,072 (the glue's copy of `n`, 1,048,576 * 2 / 16)]; `removeAttributeNS(null, n)`:
        [131,072]; `toggleAttribute(n, false)`: [131,072, 65,536 (the validation)]. `k` is 0 and
        `c` is 0,
        so the scans charge nothing, and the tree reads no byte of `n`. B ends in the last
        part, and the row's weight is its upper bound: `C <= sum(parts) + 1,000` shows that the
        slice charged nothing for a pass over the name that no scan makes. That the tree makes
        no copy of the name is not a step count; the source test of acceptance 7a holds it.
      - The read rows, with the value from the page: `<html a="V">` with `V` of 1,048,576 bytes
        built into the page string once and parsed under the default `Limits`; the script
        `var e = document.documentElement;`. `parts` is [the slice's charges, then G2, the
        glue's charge for the String it answers], and B1 = S0 + M + (the slice's charges) + G2 /
        2 ends `limit: steps-per-script` in the glue's charge; B2 = S0 + C runs. The helper
        asserts `sum(parts) <= C <= sum(parts) + 1,000` for these rows too, so that M stays far
        under G2 / 2 = 32,768 and B1 ends where the row says. `getAttribute("a")`,
        `getAttributeNS(null, "a")`: [65,536 (`V` / 16), 65,536 (G2)]; the `id`, `className` and
        `slot` getters on `<html id="V">`, `<html class="V">` and `<html slot="V">`:
        [65,536, 65,536]; for the `id` getter `k` is 1 (the attribute `id` has the length 2 of
        the wanted name, so the scan's byte term is 1 * 2 / 16 = 0) and the scan's count term
        is 1 / 4 = 0, so neither adds a part; `getAttributeNames()` on `<html n0="" ...
        n4095="">`: [4,096 + 1,210 = 5,306 (the count, and the names' 19,370 bytes over 16: 10
        names of 2 bytes, 90 of 3, 900 of 4, 3,096 of 5), 8 * 4,096 = 32,768 (the glue's
        one charge of `STRING_ELEMENT_STEPS x elements` before its loop; each name is under 16
        bytes and its bytes charge is 0)]. For this row
        M is measured and recorded in the comment, with the steps the engine takes to build an
        array of 4,096 strings, and the helper caps it at a quarter of the last part, 32,768 / 4
        = 8,192, in place of 1,000: `sum(parts) <= C <= sum(parts) + 8,192`, so B1 still ends in
        the glue's charge (M is under G2 / 2 = 16,384). A read that charged nothing would pass
        with B1.
      - What these rows cannot show. A charge whose work has no visible effect before the budget
        ends (a scan, a validation) passes at any place before the last charge, so the order of
        a scan or a validation against the work it pays for is held by review and by the source
        test of acceptance 10, not by a row. The order of `settle_stored` against the write
        is not shown either: a removal grows nothing, so its settle is a refund that the row's
        budget never reaches; acceptance 6 holds the settle by the figures it reads after the
        run, and the source test of acceptance 10 holds that every write goes through
        `charged_write`. For a read, a charge that comes after the copy
        ends at the same place as one that comes before it, so the order of the slice's charge
        and its copy is held by review too; acceptance 11d holds the one case that shows it (a
        value over `string_length`). The amounts are held by 5b.
      - At `14f8a2a` the glue's argument charge follows the copy; from `50a4239` (the reading
        unit's sixth fix round, on its branch, not merged) it precedes it. The amount is the
        same, so no figure depends on it and no row ends at another place in either order.
      - The terms assume the reading unit's price commit (the 16-byte rate, and the glue's
        `units * 2 / 16`), merged first (acceptance 14). The case records S0, S1, C and M of each
        row in a comment. In the
        report, for each writer row, the implementor moves the row's last charge to after the
        write, and for each read row removes the slice's charge, runs the helper and reports the
        rows that went red, as in 11h.
   d. On the page `<p id="p">` + 5,000 times `<i></i>` + `</p>`, with the setup `var p =
      document.getElementById("p");`, the script
      `var a = p.children; for (i = 0; i < p.children.length; i++) p.children[i].className = "x";`
      over its 5,000 children ends `Ran`, with no recorded limit, with `runner.steps()` under 15
      times those of the same loop on the page with 500 `<i>` children, and `a === p.children`
      afterwards. The
      bound is derived as the reading story derives those of its cases 8i and 8j: a loop that
      costs a constant number of steps per turn costs 10 times as much over 10 times the
      children, and a loop that refreshed the kept object at each write would cost 100 times
      (`n * n`); 15 holds the first apart from the second. The case records the steps it
      measured in a comment.
   e. `getAttributeNames` of an element of 1,000 attributes returns an array of 1,000 strings in
      order, and its steps are at least 9 * 1,000 plus the names' bytes over 16, which fails if
      either the slice's charge or the generator's per-element charge is removed.
6. The memory charge (decision 8). Each case reads `runner.document()` for `totals()`,
   `stored_live()`, `stored_charged()`, `stored_handles()`, `node_charges()` and the tree's
   `stored_bytes()` (`Tree::stored_bytes`, `tree.rs:594 @ 83e6b8a`, the bytes the tree stores)
   before and after, starts with `stored_live()`, `stored_charged()`, `stored_handles()` and
   `node_charges()` at 0, and gives `Document` the session's
   `memory_bytes` of 67,108,864 (`SESSION_ENGINE_LIMITS`, `runner.rs:40-48`) and the default
   `Limits` unless it says otherwise. The figures of 6a to 6g (80, 837, 1,109,930 and the rest)
   hold only because `ATTRIBUTE_BYTES` is `size_of::<Attribute>()` (`limits.rs:90-92 @
   a14e3b0`), 80 on a 64-bit target: each case reads `surf_html::limits::ATTRIBUTE_BYTES` and
   computes its numbers from it, and the figures below are comments in the case, stated for a
   value of 80. Throughout, `totals()` stays (0, 0) and `node_charges()` stays 0: this story
   adds no entry to `DomState::charges` and no node. The cases read `node_charges()` only to
   assert that it is 0 and does not change across a write of this story (the preamble, 6a to
   6g and the test of 6f); that holds under both meanings of the accessor (decision 8): the
   length of `charges` before the mutation story, which is 0 while no node is made, and the
   count of node handles after it, which is 0 while no node is made.
   a. For each of `setAttribute` of a new name, `setAttribute` to a longer value, `setAttribute`
      to a shorter value, `setAttributeNS` of a new prefixed name, `toggleAttribute` that adds
      and the `id` setter on an element without `id`: `stored_live()` rises by exactly the rise
      of `stored_bytes()` when it rose and falls by its fall when it fell, `stored_charged()`
      equals twice `stored_live()` after the call, and `stored_handles()` is 1 after the first
      call that adds bytes. `Attribute::stored_bytes` of the attribute added equals the rise of
      `stored_bytes()`. `removeAttribute` of what a script wrote, `removeAttributeNS` and
      `toggleAttribute` that removes lower `stored_live()` by the bytes of the attribute and
      `stored_charged()` to twice the new `stored_live()`: the refund, read as a fall of
      `stored_charged()`. A replacement by an equal value, a removal of what the parser stored
      while `stored_live()` is 0 (it stays 0 and `stored_charged()` stays as it was) and every
      throw of 1d and 1e leave `stored_live()` and `stored_charged()` where they were. 6g holds
      the removal of what the parser stored while `stored_live()` is above 0.
   b. Memory first, for every writer that adds bytes. One helper, `memory_first`, over rows, in
      `dom_attributes`; each row runs a loop of writes until the script ends `limit: memory`,
      and each row is its own `#[test]` that calls the helper, so the 60 s bound applies to a
      row. A row names its page, its setup script, its write, and the bytes `b` one write adds (the
      attribute's `stored_bytes`: 80 for the record, the bytes of the prefix, the local name and
      the value). Rows r1 to r7 use the page `<body>` + 1,000 times `<i></i>` + `</body>` (r2:
      1,000 times `<i a="x"></i>`), parsed under the default `Limits`, so that each write goes to
      a new element and no element's list grows; the setup script collects the 1,000 elements
      into an array before the loop (`var els = []; var c = document.getElementsByTagName("i");
      for (var j = 0; j < c.length; j++) els.push(c[j]);`, `selection.txt:147 @ 83e6b8a`, a
      member of the reading slice; `document.body` is no row of the selection) and builds `v =
      "x".repeat(40000)`, so that the wrapper of each element is made before the first write
      and no allocation of the loop itself ends the run in place of the charge under test. The
      collection is a snapshot, and an attribute write does not move the write counter
      (Context), so the snapshot stays valid through the loop. The loop is `for (i = 0; ; i++)`
      followed by the row's write on `els[i]`. After the run the helper asserts, on the tree
      read through `runner.document()`:
      - the run ended `ScriptEnd::Limit(Reason::Memory)` and `stored_handles()` is 1;
      - `Tree::stored_bytes()` after the run less before it equals `stored_live()`, and
        `stored_charged()` equals twice `stored_live()`;
      - N, the number of writes the tree holds, is at most the row's bound below, and is the
        count of elements that hold the write (the count of elements with the attribute, or
        with the long value for r2). For r1 to r7, where `b` is the same for every write,
        `stored_live() / b` is N, a whole number of `b`. For r0, where `b` grows with the name,
        `stored_live()` equals the sum of the N writes' bytes, which the helper adds up by name
        length, and N is the count of attributes on the element;
      - the write that was refused is not in the tree: element N has no such attribute (for r2
        it still has the value `x`), and element N - 1 has it.
      A slice that writes first and charges after leaves the refused write in the tree: the
      second assertion is false (the tree rose by (N + 1) * b and `stored_live()` is N * b) and
      so is the fourth. The helper records N, the steps and `heap_bytes()` of each row in a
      comment. The rows, each at 16 bytes a step for the tree's copy and 8 units a step for the
      glue's copy of an argument; nothing is removed, so each write grows the handle by twice
      its own net bytes, and the budget is 67,108,864:
      - r0, one element and new names: on `<p></p>` (the `html` element of it has no
        attribute), through `document.documentElement`, the loop `e.setAttribute("a" + i, v)`.
        Here the list grows and N is the count of attributes on that element, and `stored_live()`
        is the sum of the writes' bytes, which the helper adds up by name length. 2 * (80 +
        len(name) + 40,000) is 80,164 bytes for `a0` to `a9`, 80,166 for `a10` to `a99`, 80,168
        from `a100`; 837 writes charge 10 * 80,164 + 90 * 80,166 + 737 * 80,168 = 67,100,396
        bytes, and the 838th would charge 80,168 more, 67,180,564, over the budget even with an
        empty engine heap; 1,024 writes would charge 10 * 80,164 + 90 * 80,166 + 924 * 80,168 =
        82,091,812, so the attribute limit is not reached; 837 writes store 67,100,396 / 2 =
        33,550,198 bytes, just under half the stored-byte limit of 67,108,864, so that limit is
        not reached either. Bound: 837.
      - r1, `els[i].setAttribute("a", v)`: b = 80 + 1 + 40,000 = 40,081, so a write charges
        80,162; 837 * 80,162 = 67,095,594 fits and 838 * 80,162 = 67,175,756 does not. Bound:
        837.
      - r2, a longer value replaces the parser's, `els[i].setAttribute("a", v)` on `<i
        a="x">`: the write adds 40,000 and removes 1, so b = 39,999 and a write charges 79,998;
        838 * 79,998 = 67,038,324 fits and 839 * 79,998 = 67,118,322 does not. Bound: 838.
      - r3, `els[i].setAttributeNS("http://www.w3.org/2000/svg", "x:a", v)`: b = 80 + 1 + 1 +
        40,000 = 40,082, charges 80,164; 837 * 80,164 = 67,097,268 fits and 838 * 80,164 =
        67,177,432 does not. Bound: 837.
      - r4, `els[i].toggleAttribute(n)` with `n = "y".repeat(40000)`, adding: b = 80 + 40,000 +
        0 = 40,080, charges 80,160; 837 * 80,160 = 67,093,920 fits and 838 * 80,160 =
        67,174,080 does not. Bound: 837.
      - r5, `els[i].id = v`: b = 80 + 2 + 40,000 = 40,082, charges 80,164: bound 837 as r3.
      - r6, `els[i].className = v`: b = 80 + 5 + 40,000 = 40,085, charges 80,170; 837 * 80,170 =
        67,102,290 fits and 838 * 80,170 = 67,182,460 does not. Bound: 837.
      - r7, `els[i].slot = v`: b = 80 + 4 + 40,000 = 40,084, charges 80,168; 837 * 80,168 =
        67,100,616 fits and 838 * 80,168 = 67,180,784 does not. Bound: 837.
      The engine's own heap makes each end come earlier, never later. Steps: a write of r0 to
      r3 and r5 to r7 costs the glue's copy of the value, 40,000 * 2 / 16 = 5,000, and the
      tree's copy, (about 40,000) / 16 = 2,500: 7,500, and 838 of them are 6,285,000 (r3 adds
      3 for the URL a write); a write of r4 costs the glue's copy of the name, 5,000, the
      validation, 40,000 / 16 = 2,500, and the tree's bytes, 2,500: 10,000, and 838 of them
      are 8,380,000; the scans of r0 grow with the attributes held: the count terms of its
      two scans over the 838 writes are 174,724 and the byte terms 136,846, so the 838 writes
      of r0 cost about 6,285,000 + 174,724 + 136,846 = 6,596,570, and the scans of the other
      rows are of an element with at most one attribute (0 steps). Each row stays under the
      10,000,000 steps of an entry, r4 by about 1.6 million, which the loop's own steps and
      the glue's one step a call, recorded, must stay under; a row makes at most 838 writes
      that succeed and one that is refused, and its time is recorded in a comment (no row takes
      over 5 s; a row that does is named with its time in the report). The report states, for
      each row, that the row went red when the implementor wrote first and charged after.
   c. Attributes first. The same loop as r0 with `v` of 1,000 bytes ends `limit: attributes` at
      the 1,025th write: the first 1,024 writes store 1,024 * 1,080 + 4,010 = 1,109,930 bytes
      (4,010 being the bytes of `a0` to `a1023`) and hold 2,219,860; the 1,025th adds 1,085
      bytes (`a1024` is 5 bytes), so the handle grows to 2 * 1,111,015 = 2,222,030 before the
      tree refuses the write, and `settle_stored(0, 0)` puts it back; 2,222,030 is far under the
      budget. The case asserts the end, that the element holds 1,024 attributes,
      `stored_live()` of 1,109,930, `stored_charged()` of 2,219,860 (put back after the refused
      write) and `stored_handles()` of 1. The steps are measured and recorded in a comment; no
      figure of them is asserted.
   d. A repeat is a grow and a refund, not a sum. On the page `<p></p>`, with the setup `var e
      = document.documentElement; var v = "x".repeat(1048576);`, a loop of N = 40 rounds of
      `e.setAttribute("a", v); e.removeAttribute("a")` ends `Ran`,
      with no recorded limit. A set adds 80 + 1 + 1,048,576 = 1,048,657 bytes and grows the
      handle to 2 * 1,048,657 = 2,097,314; the removal lowers `stored_live()` by the same
      1,048,657 to 0 and the handle to 0. After the N rounds `stored_live()` is 0,
      `stored_charged()` is 0 and `stored_handles()` is 1. A charge that only grew would end
      `limit: memory` at the 32nd round (31 * 2,097,314 = 65,016,734; 32 * 2,097,314 =
      67,114,048 is over 67,108,864); the case holds the opposite, which needs N above 31.
      After the loop one more `e.setAttribute("a", v)` leaves `stored_charged()` at 2,097,314.
      N, derived: a set costs two copies of its value, the glue's at 8 units a step,
      1,048,576 * 2 / 16 = 131,072 steps, and the tree's at 16 bytes a step, (1 + 1,048,576) /
      16 = 65,536 steps: 196,608 steps; an entry ends at 10,000,000 steps (`steps_per_entry`,
      `runner.rs:41`); N rounds and the last set are N + 1 sets, and (N + 1) * 196,608 <=
      10,000,000 gives N + 1 <= 50 and so N <= 49 before the validation, the scans and the
      loop's own steps. N is 40: 41 * 196,608 = 8,060,928 steps of copies, leaving 1,939,072
      steps for the rest. The case records `runner.steps()` in a comment and, if the first
      measure shows less room than that, lowers N to a round number above 31 that fits, and
      says so in the report.
   e. Growth by replacement. On the page `<p></p>`, with the setup `var e =
      document.documentElement; var v = "x".repeat(1048576);` (`v` is the largest value; each
      smaller one is `"x".repeat(n)`), `e.setAttribute("a", ...)` with a value of 1,024 bytes,
      then of 2,048, 4,096 and on by doubling to 1,048,576 bytes (11 sets), leaves
      `stored_live()` at 81 + 1,048,576 = 1,048,657 and `stored_charged()` at 2 * 1,048,657 =
      2,097,314: twice the last length plus the fixed terms (80 for the record, 1 for the
      name), not the sum of the 11 sets' own bytes, which would be 2 * (11 * 81 + 2,096,128) =
      4,194,038. Then a replacement by a value of 1,024 bytes lowers `stored_live()` to 1,105
      and `stored_charged()` to 2,210, the refund of 2,095,104; and doubling again to
      1,048,576 bytes brings them back to 1,048,657 and 2,097,314, each grow charged by its
      difference. `stored_handles()` is 1 throughout. The steps are about 786,048: the 22
      sets' values are 4,192,256 units and bytes, whose glue copies cost 4,192,256 * 2 / 16 =
      524,032 and whose tree copies cost 262,016, plus their own cost.
   f. The functions themselves, in `crates/script/tests/bindings.rs`: through the test slice,
      a variant `Writes::Stored` beside `Writes::Charge` (the arm of `Document.createTextNode`
      at `tests/bindings.rs:175-178 @ 83e6b8a`). That arm reads its argument as `A R`, two
      numbers, and the last character of the argument picks the end (each settle's answer is
      taken with `?`, after it, as `charged_write` does): no mark, `charge_stored(A, R)` then
      `settle_stored(A, R)`; `!`, `charge_stored(A, R)` then `settle_stored(0, 0)`, the
      case of a write the tree refuses; `?`, the same, then it answers `Err(DomEnd::Throw(..))`
      of an `InvalidStateError`, a refusal that is not a limit; `~`, `charge_stored(A, R)`, then
      it pushes `("mid", cx.stored_charged())` onto the `Record` that the test shares
      (`convert_before_copy`, `tests/bindings.rs:894-914`, reads one through `handed.taken()`),
      then `settle_stored(0, 0)`, then pushes `("after", cx.stored_charged())`; the member holds
      a `DomCx` and no `DomState`, so it reads the number through `DomCx::stored_charged()`
      (decision 8). From (`stored_live()`,
      `stored_charged()`, `stored_handles()`) of (0, 0, 0), each call giving these three after
      it, and `totals()` at (0, 0) and `node_charges()` at 0 throughout: add 1,000 gives (1,000,
      2,000, 1); add 500 gives (1,500, 3,000, 1); remove 700 gives (800, 1,600, 1), 1,400
      refunded; add 600 gives (1,400, 2,800, 1); add 300 gives (1,700, 3,400, 1); remove
      5,000, more than `live`, gives (0, 0, 1), never below 0; add 900 and remove 400 in one
      call gives (500, 1,000, 1); a call with both 0 changes nothing. The put-back is seen: from
      (500, 1,000, 1), `"100 0~"` records `mid` 1,200 (the handle grew to twice 600) and `after`
      1,000, and leaves (500, 1,000, 1); a settle that did not put the handle back would leave
      `after` at 1,200. The `~` arm by hand, for `document.createTextNode("100 0~")` run on a
      runner built with `let record = Record::default(); let handed = record.clone();` and
      `test_document(Writes::Stored, record)`, from (500, 1,000, 1): the glue of the member
      charges its one step, brand-checks `this`, converts the argument and copies it
      (`100 0~`), and calls `TestMutation::document_create_text_node`, whose `self.0` is
      `Writes::Stored`; the arm takes the last character `~` and reads `100 0` as A = 100 and
      R = 0; `cx.charge_stored(100, 0)?` finds the next live 600 and the wanted charge 1,200
      above `charged`, grows the handle to 1,200 and leaves `live` 500; the arm pushes
      `("mid", "1200")` onto `self.1`, the shared `Record`; `cx.settle_stored(0, 0)?` finds the
      next live 500 and the wanted charge 1,000, shrinks the handle, and sets `charged` to
      1,000; the arm pushes `("after", "1000")` and answers `Ok(this)`; the script ends `Ran`;
      after `with_document` returns, the test asserts `handed.taken()` equals `[("mid",
      "1200"), ("after", "1000")]`. `"100 0!"` leaves (500, 1,000, 1). The refusal that is not a
      limit,
      `"100 0?"`, ends the call with the throw and leaves (500, 1,000, 1): `stored_charged()`
      equals twice `stored_live()` after it, as a `?` on the tree call between the charge and
      the settle would not leave it. Many raises: 10,000 calls of add 1, remove 0 leave
      `stored_handles()` at 1 and `stored_charged()` at 20,000 (one handle, not 10,000
      entries), and then remove 5,000 lowers `stored_charged()` to 10,000. A refused charge:
      with `memory_bytes` of 1 MiB, an add of 10,000,000 ends `Reason::Memory` and leaves
      (`stored_live()`, `stored_charged()`, `stored_handles()`) as they were. The heap grows
      by at least the charge beyond an uncharged control, as `node_charge` measures it
      (`tests/bindings.rs:917-954`).
   g. A removal of what the parser stored while `stored_live()` is above 0 refunds, and never
      more than was charged. On the page `<html p="P">` with `P` of 1,000 bytes (the parser's
      attribute `p` stores 80 + 1 + 1,000 = 1,081 bytes), the script `var e =
      document.documentElement; e.setAttribute("s", "x".repeat(2000));` adds 80 + 1 + 2,000 =
      2,081 bytes and leaves (`stored_live()`, `stored_charged()`, `stored_handles()`) at
      (2,081, 4,162, 1). Then `e.removeAttribute("p")` removes 1,081 bytes the parser stored:
      live 2,081 - 1,081 = 1,000, charged 2,000, a refund of 2,162, and the tree's net rise
      since the parse is 1,000, so the handle holds twice it. Then `e.removeAttribute("s")`
      removes 2,081: live would be 1,000 - 2,081, which stops at 0, and the handle goes to 0, a
      refund of 2,000, the whole of what it held and no more, while the tree is 1,081 bytes
      below its size at the parse. Each of the three steps is read after its own script on the
      same runner. A slice that gave nothing back for a parser byte would leave (2,081, 4,162,
      1) after the second step and fail; a slice that let `live` go below 0 fails the third.
   h. A defect ends the run and no script catches it (decision 8, "The variant"). In
      `tests/bindings.rs`, with `Writes::Fault` (`Document.createTextNode` answers
      `Err(DomEnd::Fault)`, which the test slice makes, so no engine has to fail), the script
      `try { document.createTextNode("x"); } catch (e) { } finally { }` ends
      `ScriptEnd::Faulted` (`terminated`, `runner.rs:918`), the case asserts that end with
      `assert_eq!`, and `Ran` is what a throw taken by the `catch` would give, so a mapping to
      `DomEnd::Throw` fails it. After the end the case reads `stored_live()`, `stored_charged()`
      and `stored_handles()` through `runner.agent_and_realm().0.host().document()`, as `recorded`
      does (`tests/bindings.rs:796`), and they are (0, 0, 0). This holds the arm of `answered`
      and the generated glue, from a made `DomEnd::Fault`. No test reaches the other half, the
      answer of `charge_to` to an outer `Err(HostError)` and the answer of `settle_stored` to
      either layer: the engine gives them only for a handle of another engine or a faulted
      agent, `DomState::stored` is a private field of the generated file, and no engine fails
      on demand. The source check stands in for them: `git grep -n -F 'DomEnd::Fault' --
      crates/webidl/src/prelude.rs | grep -v -E '^[^:]*:[0-9]+:[[:space:]]*//'` prints each of
      three code lines (the doc comments that name the variant start with `//` and are
      filtered): `Err(_) => Err(DomEnd::Fault),` in the body of `charge_to` (the outer
      `Err(HostError)`), `Err(DomEnd::Throw(_)) => Err(DomEnd::Fault),` in `settle_stored` (the
      inner `Err` made `Fault`) and the arm `Err(DomEnd::Fault) => ..` of `answered`, which
      answers `Abrupt::Terminated(Termination::EngineFault)`; and it prints no line outside
      `charge_to`, `settle_stored` and `answered` (read at the line numbers it prints; a
      correct `settle_stored` may name the variant a fourth time, to pass an outer `Fault`
      through, or fold two of the lines into one arm; neither turns the check red); and
      `git grep -n -F 'DomEnd::Fault'
      -- crates/script/src/dom | grep -v -E '^[^:]*:[0-9]+:[[:space:]]*//'` prints nothing (no
      slice makes it: only the glue ends a run). The
      case is red against an arm that answers `Err(throw(cx, ..))`, which the implementor shows
      once and reports.
7. The reads and owners of the tree (decision 3), in the module `script_hook` of
   `crates/html/tests` (the unit test of 7f in `tree.rs`).
   a. Each of the three reads, `Namespace::from_url`, `Tree::namespace_error`,
      `Attribute::stored_bytes` and `Attribute::qualified_len` has a test (`stored_bytes` equals
      the rise of `Tree::stored_bytes` for each attribute `set_attribute`, `set_attribute_ns`
      and `toggle_attribute` add); `attribute_position` lowers a name for an HTML element and
      not for an SVG one, byte by byte, and does not fold the stored byte: an attribute stored
      as `FOO` on an HTML element (by `set_attribute_ns` with no namespace) is found by no
      `attribute_position(element, "FOO")` and by no `"foo"`, and by
      `attribute_position_ns(element, None, "FOO")`, as in acceptance 1;
      `attribute_position_ns` matches by namespace and local name and not by prefix;
      `attribute_names` keeps order and duplicates and gives
      `xlink:href` for a prefixed attribute, and each of its strings has the length
      `Attribute::qualified_len` answers for that attribute; `Namespace::from_url` answers
      `Some(ns)` for `ns.url()` of each of the six and `None` for `""`, a near miss (a URL with
      one byte changed) and an unknown URL. The source test of the removals,
      `removals_copy_no_name`, reads `crates/html/src/tree.rs` with `include_str!` and strips
      every line comment and the rest of a line after `//` before any search (as acceptance 10
      does). It cuts the
      text of a function from its opening line to the next line that starts a `fn` (after its
      blanks, `fn ` or `pub fn `): `fn position_of(` and `fn position_of_ns(` each to its own
      end, `pub fn remove_attribute(` (the parenthesis keeps the `_ns` twin out of it),
      `pub fn remove_attribute_ns(` and `pub fn toggle_attribute(`. The function
      `toggle_attribute` keeps a `match` with two arms that each start a line, the adding arm
      `None =>` and the removing arm `Some(at) =>`; an `if let` in place of the `match` is not
      admitted. The test asserts both starts and fails with a message that names the one it
      does not find. It cuts the removing arm by its braces, from the `{` that ends the line of
      `Some(at) =>` to the `}` that closes it, counting the braces of the stripped text, not to
      the end of the function (a line of `Some(at) =>` with no `{` fails with a message too).
      It asserts one forbidden list, `attribute_name(`, `to_ascii_lowercase(`,
      `to_lowercase(`, `make_ascii_lowercase(`, `.to_owned(`, `.to_string(`,
      `ToString::to_string(`, `to_vec(`, `.clone(`, `.cloned(`, `.into(`, `.into_owned(`,
      `.repeat(`, `.push_str(`, `.extend(`, `String::`, `Vec::`, `Box::`, `Rc::from(`,
      `Arc::from(`, `Cow`, `format!` and `.collect`, against the text of `position_of`, of
      `position_of_ns`, of `remove_attribute`, of `remove_attribute_ns` and of the removing arm
      of `toggle_attribute`: so the position rule lowers the wanted name byte by byte and copies
      nothing, and no removal copies the name. A `git grep` of each of
      `make_ascii_lowercase(`, `ToString::to_string(`, `to_vec(`, `.cloned(`, `.into(`,
      `.into_owned(`, `.repeat(`, `.push_str(`, `.extend(`, `Vec::`, `Box::`, `Rc::from(`,
      `Arc::from(` and `Cow` at `83e6b8a` over the code the cuts cover today
      (`qualified_name_is`, `tree.rs:696-706`, the bodies of `remove_attribute` and
      `remove_attribute_ns`, `:1596-1621`, and the removing arm, `:1651-1654`) finds none:
      correct code holds none of them, so no token turns it red (`Vec::` is on 8 other lines of
      the file, `.push_str(` on 6 and `.extend(` on 10, none in those). The cuts call no
      function of the file other than `lower_byte`, `element_attributes`, `drop_attribute`,
      `position_of`, `position_of_ns`, `qualified_len` (which the length compare of
      `position_of` may call) and `qualified_name_is` (the private compare of today,
      `tree.rs:696`, which `position_of` may keep for an element outside the HTML namespace):
      the test takes the names after `fn ` in the stripped file and
      asserts that no call in the five texts has one of them but those; a call is a name
      before `(` with no receiver, or the receiver `self.` or `Self::`, or `.qualified_len(`.
      The lowering of one byte is a private function of its own, `lower_byte(byte: u8) -> u8`,
      outside every cut, which `position_of` calls for each wanted byte of an HTML element. In
      `toggle_attribute` the first `attribute_name(` of the function comes after the line that
      starts `None =>`, the adding arm (the name is lowered into a `String` only there, and no
      earlier line copies it). The test is red when `remove_attribute` is given back its call
      of `attribute_name` (the implementor shows it once and reports the output). The
      case that `removeAttribute("ID")` on an HTML element still removes `id` is in
      acceptance 1.
   b. One matching rule: for a list of names (capitals, a prefixed name, a duplicate, an absent
      one) on an HTML and an SVG element, `attribute_position(..).is_some()` is true exactly
      when `remove_attribute` of the same name on a clone of the tree lowers the attribute
      count, `attribute_position_ns` likewise against `remove_attribute_ns` (the ns writes call the
      private `position_of_ns`, as `attribute_position_ns` does, so a copy of the rule that
      drifts in either shows here), and the position answered is the position `set_attribute`
      replaces in, and `attribute_position_ns` the position `set_attribute_ns` replaces in (the
      order of `attribute_names` is kept by a replacement).
   c. A node that is not an element (a text node, the document) answers `None` and an empty
      list.
   d. One rule for `NamespaceError`: for a table of (namespace, prefix, local name) triples
      holding each of the four conditions of `tree.rs:1569-1577 @ 83e6b8a` and a triple that
      passes, `Tree::namespace_error(namespace, prefix, local_name)` answers true for the first
      four kinds and false for the last, and `set_attribute_ns` with the qualified name the
      triple makes (`prefix:local_name`, or the local name alone with no prefix) answers
      `WriteError::Namespace` exactly when `namespace_error` is true (for a namespace of the
      six). The function makes no pass over the name: the same test reads its text and asserts
      that it holds none of `split`, `find(`, `rfind(`, `chars()`, `char_indices(`, `bytes()`,
      `contains(`, `matches(` and `trim`, so each compare is against a constant of at most 36
      bytes. The four conditions the function replaces (`tree.rs:1569-1577 @ 83e6b8a`) hold
      none of the nine tokens, so none turns correct code red.
   e. Each new public function's doc comment carries a line that starts with the token
      `Cost:`: `git grep -c -F 'Cost:' -- crates/html/src/tree.rs` prints nothing today (the
      token is free at `83e6b8a`, and `git grep -c` prints nothing for a file with no match) and
      `crates/html/src/tree.rs:7` after, one for each of `attribute_position`,
      `attribute_position_ns`, `attribute_names`, `Namespace::from_url`, `Tree::namespace_error`,
      `Attribute::stored_bytes` and `Attribute::qualified_len`. A grep cannot show that each line
      is in the right doc comment; the test of 7a for each function reads the file's text of
      that function's comment with `include_str!` and asserts the token follows the function's
      name before the next `fn`.
   f. `put_attribute_reserves_exactly`, in the `#[cfg(test)]` module of `tree.rs`: on a fresh
      element, after each of the first four adds, `attributes.capacity()` equals
      `attributes.len()`, so the bytes the tree counts for those attributes are the bytes the
      list holds; a fifth add may grow the list by the `Vec` rule.
8. Other slices and `[SameObject]` (decision 10), a guard that holds on `main` today and stays
   green only if this story does not move one of its own rows out of `throws` by mistake:
   `el.attributes`, `el.classList`, `el.getAttributeNode("a")` and
   `document.createAttribute("a")`, rows of `throws` beside this slice that stay `throws`, throw
   `NotSupportedError` naming the member (`<member> is not supported: <reason>` for a row
   of `throws`, `bindings.rs:2042-2047`), each in its own `try`; `el.attributes === el.attributes`
   and `el.classList === el.classList` each throw, never answer. The case names no row of
   another slice: `el.appendChild` is the mutation slice's and `el.querySelector` the
   selectors slice's, and each of those stories changes its own line of
   `other_slices_still_throw` (Change, Placement). `git grep -n -F 'el.getAttribute(' --
   crates/script/tests/dom_reading.rs` prints no line that expects a throw, and
   `git diff -U0 $(git merge-base main HEAD)..HEAD -- crates/script/tests/dom_reading.rs`
   shows one removed and one added line of that test, the `getAttribute` line. The
   `SameObject` enum keeps its four variants: `git diff -U0 $(git merge-base main HEAD)..HEAD
   -- crates/script/src/bindings.rs | grep 'SameObject'` prints nothing (a guard).
9. The generated files. `awk '$3=="attributes"' crates/webidl/selection.txt | wc -l` prints 14
   (a guard, true on `main`); the row `Element setAttributeNS` holds the deviation of decision
   9; `slot task webidl-check` exits 0 and `slot task webidl-slow` regenerates `bindings.rs`
   byte for byte; `git diff -U0 $(git merge-base main HEAD)..HEAD -- crates/script/src/bindings.rs
   | grep '^[-+]//! [0-9a-f]\{64\}'` prints the digest lines of `crates/webidl/selection.txt`
   and of `crates/webidl/src/prelude.rs` and no other source's (the pattern matches the digest
   lines of the sources, lines 43-60; it does not print the whole-file digest, header line 72
   of `bindings.rs`, which changes by regeneration and which `slot task webidl-check` holds),
   and `git grep -c -F 'charge_stored' -- crates/script/src/bindings.rs` prints a number above
   0 (nothing at `a14e3b0`).
10. Files, and the source test. `git diff --stat $(git merge-base main HEAD)..HEAD` names only
    the files of the Change and these six public surfaces, as an explicit list: `README.md`,
    `docs/badges/make.sh`, `docs/badges/tests.svg`, `website/docs/how-it-is-built.md`,
    `website/docs/status.md` and `website/docs/what-works.md` (a guard); it names nothing else
    under
    `crates/html/` than `src/tree.rs` and `tests/script_hook.rs`, and not
    `crates/script/src/runner.rs`, `crates/script/tests/adversary_3.rs`, `adversary_4.rs`,
    `adversary_5.rs` or `crates/webidl/src/generate.rs`; the same diff of `reads.rs` holds only
    the `pub(super)` tokens this story needs that `main` lacks (three, or fewer when the
    selectors story merged some first). The module `dom_attributes` is the one declared at
    `tests/main.rs:17` and `dom_attributes_cost` the one added after it; no `[[test]]` entry is
    added; `slot task test-layout` exits 0.
    The source test `dom_attributes_cost::the_members_touch_the_tree_only_through_the_files`
    reads `attributes.rs`, `attribute_reads.rs` and `attribute_writes.rs` with `include_str!`
    (as `dom_reading_cost::the_members_read_the_tree_only_through_the_reads` does,
    `dom_reading_cost.rs:594`), strips from each text, before any search, a line comment and
    the rest of a line after `//`, and a line left blank, as `code_lines` does
    (`adversary_dom_reading_5.rs:308-316 @ 50a4239`; a comment that names a forbidden string
    does not turn the test red), and asserts: `attributes.rs` holds none of `tree()`,
    `tree_mut()`, `charge(`, `charge_steps`, `charge_stored` and `settle_stored`;
    `attribute_reads.rs` holds none of `tree_mut()`, `charge_stored` and `settle_stored`;
    `attribute_writes.rs` holds no `tree()`, and it holds `charge_stored(`, `tree_mut()` and
    `settle_stored(` only inside the text of the function `charged_write` (the text outside
    that function holds none of the three), where `charge_stored(` is called once and
    `settle_stored(` twice, once as `settle_stored(0, 0)`. Each settle is one statement
    spelled `cx.settle_stored(..)?;`, and that spelling is the rule: the test finds each
    statement from `cx.settle_stored(` to its `;` and asserts that it ends in `)?;`, so
    `let put_back = cx.settle_stored(0, 0); put_back?;` is red. The tree's answer is taken by
    a `match` of `charged_write`, the first that starts at or after the line of its call of
    `write(cx.tree_mut())` (the pattern is the whole call, never the bare `write(`, which the
    signature of `charged_write` holds; a `charged_write` that does not hold the pattern once
    is red, with a message), one arm for the `Err` and one for the `Ok`. The test cuts each
    arm by counted braces of the stripped text, as 7a cuts the removing arm, from the `{`
    after its `=>` to the `}` that closes it (an arm with no `{` after its `=>` is red, with
    a message), and asserts that each arm holds one settle statement: an arm with no settle,
    such as `Err(e) => Err(map(e))`, is red. The text from the tree call to the end of the
    last settle statement, with the two settle statements cut out (each up to its `;`, so
    that the `?` that takes the settle's own answer goes with it) and with the rest of each
    arm after its own settle statement left out, holds none of `?`, `return`, `unwrap` and
    `expect(`, so that no refusal of the tree, of any kind, skips the settle: before the
    settle of an arm nothing leaves the function. A `return` after an arm's own settle skips
    nothing and is not read: `Err(e) => { cx.settle_stored(0, 0)?; return Err(map(e)); }` is
    green, and so is any text after the last settle statement. The test is red when one of those
    strings is added to the file that must not hold it, when a `?`, a `return` or an
    `unwrap()` is put between the tree call and the settle of an arm, and when the settle of
    an arm is removed (the implementor shows each once and reports the output). The
    signature of the settle is held by the compile, since the writers and the
    test slice of 6f take its answer with `?`, and by one source line: `git grep -n -F 'pub fn
    settle_stored(&mut self, added: usize, removed: usize) -> DomResult<()>' --
    crates/webidl/src/prelude.rs` prints one line.
11. The cost module, `crates/script/tests/dom_attributes_cost.rs`: one case for each member that
    reads or writes more than its own node, on the worst shape the parser admits for it, one run
    a shape, no loop to steady a number. Each case records its steps, its time and the time a
    step against the plain walk of `dom_reading_cost.rs` (`getElementById('none')` over 20,000
    elements without attributes); it asserts a floor of steps, the sum of the charges of
    decision 7 for that shape, and no time: no assertion of the module reads a time (the 5 s,
    the ratio of 20 and the 60 s below are the report's and the runner's bounds, not a test's;
    `AGENTS.md:1966 @ a14e3b0` lets a test bound a time only at 10 times the time it takes
    alone, and proves a cost by a count). Cases that share a page are one `#[test]` that
    builds the page once and runs its members in turn, each on its own runner: a shared
    in-process value is built once for each test under nextest, which runs each test in its own
    process, so the page string of 11a, 11b and 11c, which holds two elements (one of 4,096
    attributes with names of 1,000 bytes for 11a, one of 16,384 short names for 11b and 11c),
    is built once for the three, each run parses its own tree from it, and one test holds the
    case 11d, the exception: its three reads change nothing, so it makes one runner and one
    parse of its page and runs its three scripts in turn, and its time is named in the
    implementor's report. The module sets the limits it raises in one place, its own `limits()` and
    `raised()` (copied from `dom_reading_cost.rs:22-46 @ 50a4239`), and raises no other:
    - parse limits: `max_attributes` 20,000 (the shapes hold 16,384, and 11e adds the 16,385th);
      `max_input_bytes` 23,068,672 (22 MiB; the default is 16,777,216, `limits.rs:103`, and the
      page of 11d holds 20,971,522 bytes of values); `max_stored_bytes` 268,435,456 (256 MiB, as
      `dom_reading_cost.rs:44 @ 50a4239`; the page of 11d, the largest, stores about 21,000,000
      bytes, under a tenth of it).
    - engine and runner limits: `steps_per_entry` 50,000,000, `steps_per_runner`
      1,000,000,000, `memory_bytes` 256 MiB, `string_length` 1 << 22, and `wall_clock` 50 s
      (`dom_reading_cost.rs:36 @ 50a4239`; a session's is 5 s, `runner.rs:97 @ 83e6b8a`, and
      the 50 s is ten times it and under the 60 s bound of one `#[test]`); the other fields as
      in that file.
    `Limits` has no check on a value: the tokenizer stops at `attributes.len() > max_attributes`
    (`tokenizer/machine.rs:494 @ 83e6b8a`), the tree builder at `held + new.len() >
    max_attributes` (`tree_builder.rs:1562`), `put_attribute` at `len >= max_attributes`
    (`tree.rs:1516`), and `cut_str` takes `max_input_bytes` as it is, so no cap forbids 20,000
    or 23,068,672 (`a14e3b0` has the same tests, `tree.rs:1450`). Under those
    limits:
    a. `getAttribute` and `hasAttribute` of an absent name over an element of 4,096 attributes
       whose names are all 1,000 bytes long, the wanted name 1,000 bytes long and differing
       from every name only in its last four bytes (every compare reads the whole name; the
       names are distinct, 996 letters and a 4-digit number, the wanted one 996 letters and
       `zzzz`, so `k` is 4,096): floor 4,096 / 4 + 4,096 * 1,000 / 16 = 1,024 + 256,000 =
       257,024, from the formula of decision 7 (the name is the slice's term; the glue's charge
       for the 1,000-byte argument, 1,000 * 2 / 16 = 125, is not in the floor).
    b. `getAttributeNS` and `hasAttributeNS` over 16,384 attributes with short numbered names
       (`n0` to `n16383`), the wanted local name `x` (1 byte, so `k` is 0): floor 16,384 / 4 =
       4,096 each. The `id` getter on the same element: the wanted name is `id` (2 bytes), so
       `k` is 10 (`n0` to `n9`) and the floor is 16,384 / 4 + 10 * 2 / 16 = 4,096 + 1 = 4,097.
    c. `getAttributeNames` over 16,384 attributes `n0` to `n16383`: the names have 10 * 2 + 90 *
       3 + 900 * 4 + 9,000 * 5 + 6,384 * 6 = 87,194 bytes; floor 16,384 + 87,194 / 16 (5,449) +
       8 * 16,384 (131,072) = 152,905.
    d. `getAttribute` of a value of 4,194,304 bytes (`string_length` itself, 1 << 22): floor
       2 * 4,194,304 / 16 = 524,288 (the slice's charge and the glue's charge for the answer; no
       count is made, as a value of at most `string_length` bytes has at most that many units).
       A value of 4,194,305 bytes is in the band where bytes exceed `string_length` and units
       may not: it throws the `RangeError` with the steps raised by at least 2 * 262,144 =
       524,288 (4,194,305 / 16 = 262,144 for the value's bytes, and again for the count that
       `longer_than` makes, charged before it runs). A value of 12,582,913 bytes (over
       `3 * string_length`) throws the `RangeError` with the steps raised by at least 786,432
       (12,582,913 / 16) and by less than 1,572,864, so no count was made or charged. The
       glue's charge for the answer is not reached in the two throws, so the floors are the
       slice's alone. Each throwing read runs in `try { .. } catch (e) { if (!(e instanceof
       RangeError)) throw e; }`, its try ending in `throw new Error('no RangeError')` so that a
       read that answers is red (`dom_reading_cost.rs:794-795 @ 50a4239`), and the case ends
       `Ran`, as `call` asserts (`:152` there). A slice that counted before it charged and
       threw would raise the steps by less than the first of these and fail, and one that
       charged the count in the first band only once fails the 524,288. The page holds the
       three values in one attribute each, in one `#[test]` with one runner and one parse, the
       steps of each script taken from `runner.steps()` before and after it, under the limits
       above: the page is 20,971,522 bytes of values and a few hundred of markup, under the
       `max_input_bytes` of 23,068,672, so the parse cuts no value. No case holds a fourth
       value, one of 4,194,305 to 50,331,648 bytes whose units do not pass `string_length`,
       which a read then answers: it meets the glue's own count of the answer (`longer_than` in
       `string_value`) after the glue's charge of `text.len() / 16` at `14f8a2a`, and before it
       at `83e6b8a`. The reading unit owns that order.
    e. `setAttribute` with a value of 4,194,304 bytes on an element of 16,384 attributes
       (`n0` to `n16383`), adding a name of 1 byte: floor 4,194,304 * 2 / 16 (the glue's
       copy, 524,288) + (1 + 4,194,304) / 16 (the tree's, 262,144) + 2 * 16,384 / 4 (two
       charged scans, three passes, 8,192) = 794,624; the `id` setter (which shares
       `set_reflected` with `className` and `slot`) with the same value likewise;
       `setAttributeNS` with a qualified name of 1 MiB (`"x:" + "y".repeat(1048574)`), the SVG
       namespace URL and an empty value: floor 131,075 (the glue: 1,048,576 * 2 / 16 for the
       name, 26 * 2 / 16 = 3 for the URL) + 65,536 (the validation and split of the whole name)
       + 65,535 (the tree's bytes) + 8,192 (the scans, `k` is 0) = 270,338. `toggleAttribute`
       that adds a name of 1 MiB and `setAttribute` with a name of 1 MiB and an empty value:
       floor 131,072 (the glue) + 65,536 (the validation) + 65,536 (the tree's bytes) + 8,192 =
       270,336 each. Each case adds the 16,385th attribute, under `max_attributes` of 20,000, and
       ends `Ran`. The limit case of the attribute count (`limit: attributes` at the limit) is
       not in this module: acceptance 2a holds it for the four writers that add, with
       `max_attributes` at 3.
    f. `removeAttribute("n16383")` and `removeAttributeNS(null, "n16383")` of the last of 16,384
       attributes `n0` to `n16383`: `k` is 6,384 (the names of 6 bytes, `n10000` to `n16383`),
       so each scan charges 16,384 / 4 + 6,384 * 6 / 16 = 4,096 + 2,394 = 6,490, and the floor
       is 2 * 6,490 = 12,980 (two scans, `find`'s and the tree's, each one charge call, and no
       byte term for a copy, as a removal copies nothing; the shift is paid by them); the same
       floor holds for `toggleAttribute("n16383")` removing it, plus 6 / 16 = 0 for the
       validation of its short name.
    g. The quadratic shape: `for (i = 0; i < 4096; i++) e.setAttribute("n" + i, "v")` on an
       element with no attribute: the write of index `i` finds `i` attributes and two scans
       each charge `floor(i / 4)`, so the floor is the sum over `i` from 0 to 4,095 of
       2 * floor(i / 4) = 2 * 4 * (0 + 1 + ... + 1,023) = 2 * 4 * 523,776 = 4,190,208 (the terms
       `k * n / S` ride above it), and the loop ends `Ran` under 50,000,000 steps.
    h. A floor must fail when its own member's charge is removed: the implementor removes each
       charge of decision 7 in turn (the scan's count term, the scan's byte term, the value, the
       count of the band, the names, the validation, the tree's second scan, the tree's copy,
       the per-element step), runs the module, and the report names for each removal the cases
       that went red; a charge with no red case gets a case or a tighter floor. The same
       report names, for each writer row of 5c, the row that went red when that row's last
       charge was moved after its write, for each row of 6b the row that went red when the
       write was made before `charge_stored`, for each read row the row that went red when the
       slice's charge was removed, and the case of 11d that went red when `longer_than` was
       called before the slice's charge. It also says what 5c cannot show (the order of a scan
       or a validation, the order of `settle_stored` against a write, and the order of a read's
       charge and its copy), which review and the source test of 10 hold.
       The report states each case's time and ratio against the plain walk; a ratio over 20 is
       a defect of the price, fixed by raising the charge in the same change. No case runs over
       5 s; a case that does is named with its time and its shape is made smaller by a round
       number, with the floor from the smaller shape.
12. `slot task check` exits 0.
13. Public surfaces: the story changes, in its last commit and after merging `main`, every
    public surface its change makes false, each number taken from the `slot task check` log of
    its tip. The six files below are outside the story's recorded scope, by the epic's rule
    (`epic:955-961 @ aa70366`), and are changed here all the same.
    a. The old strings are read first, after the last merge of `main` (the reading story and
       any other merge moved them): `git grep -n -e 'passed' -e 'test files' -e 'tests:' --
       README.md website/docs docs/badges`, whose output the report records. At `a14e3b0` they
       are the test count `2394 passed` (`README.md:11`, `docs/badges/make.sh:38`,
       `docs/badges/tests.svg:1`, `:2`, `:10`), `2,397 tests` and `1,845` (`README.md:285`,
       `website/docs/what-works.md:17`; `2,394 passed and 3 ignored: 1,842 passed` at
       `website/docs/status.md:31`), the test line `1845 tests: 1842 passed, 0 failed, 3 ignored`
       (`README.md:626`, `website/docs/how-it-is-built.md:70`) and `230 test files`
       (`README.md:612`, `website/docs/how-it-is-built.md:56`). They are not the strings to
       look for after the reading story merges.
    b. After the last commit `git grep -n -F -e '<old>' -- README.md website/docs docs/badges`
       prints nothing for each old string 13a recorded, and each new number equals the one in
       the log of the tip. The gate block (`README.md:596-645`,
       `website/docs/how-it-is-built.md:40-89` at `a14e3b0`) holds that log's lines, and the
       paragraph that names the gate run gains this story's sentence.
    c. The entry of `crates/script` in `AGENTS.md` (the lines that name `surf-script`,
       `AGENTS.md:365-369 @ 83e6b8a`, where it says "29 tests"; the reading story edits the same
       entry) has a test count equal to the tests of `surf-script` in the log of the tip.
14. The folder's bounds hold, and the order of the work. The reading unit's price commit (one
    step per 16 bytes in its own charges, and `units * 2 / 16` in `Copies::copy`,
    `bindings.rs:630 @ 14f8a2a`) is merged first. At `14f8a2a` the glue's argument charge
    follows the copy; from `50a4239` (the reading unit's sixth fix round, on its branch, not
    merged) it precedes it. The amount is the same, so no figure of 5b, 5c, 6 and 11 depends
    on it. The
    first command of the work is `git grep -n -F '/ 64' -- crates/script/src/bindings.rs
    crates/webidl/src/prelude.rs crates/script/src/dom/reads.rs`; it prints nothing, else the
    floors of 5b, 5c, 6 and 11 are not the story's and the work waits for that commit. It
    prints nothing at `14f8a2a`; it prints 18 lines at `83e6b8a`: 16 in `reads.rs` (`:135`,
    `:150`, `:195`, `:313`, `:334`, `:462`, `:666`, `:705`, `:805`, `:807`, `:808`, `:813`,
    `:1084`, `:1097`, `:1117`, `:1249`), `bindings.rs:628` and `prelude.rs:558`; at `a14e3b0`
    it prints 2 (`bindings.rs:561` and `prelude.rs:491`). At the same start the implementor
    reads `longer_than` (`bindings.rs:145-147`), `reads::copied` (`reads.rs:226-237 @
    14f8a2a`) and `string_value` (`bindings.rs:1266-1272 @ 14f8a2a`) and reports whether each
    charges a value's bytes before it counts UTF-16 units: at `14f8a2a` `copied` and
    `string_value` do (`longer_than` itself charges nothing), at `83e6b8a` none did; this
    story's own reads charge first (decision 7, 11d) whatever the reading unit has done. The
    test step ends within 120 s and no test runs over 60 s; a test over 5 s is named
    with its time in the report, and so is the time of `dom_attributes` and of
    `dom_attributes_cost` as a whole (the loops of 6b to 6d and the shapes of 11 are the ones to
    time); each case makes its own runner (5.9 to 16.4 ms in the bindings unit's measure,
    `webidl-bindings.md` "As built"); no model call and no network request in a test; no Python
    in anything that stays; an expensive product is made once per test run and never per test
    (cases of 11 that share a page are one `#[test]` that builds the page once and runs the
    members in turn, each on its own runner, and no case starts a server); no generator with a
    committed output is added: the existing one gains `charge_stored`, `settle_stored` and the
    per-element step, and the time of `slot task webidl-slow` is measured before and after,
    both in the report; a build or test job in the pool ends within 10 minutes or names its
    time with `slot --timeout`, at most 30 minutes, and is not started again unchanged; every
    compiling or testing command carries `slot`; a script that measures or waits starts without
    it.
15. The report holds, as text: the output of the gate command of 14, the numbers measured in 5,
    S0, S1, C and M of each row of 5c and the rows that went red with the row's last charge
    moved after its write (and the read rows with the slice's charge removed), what 5c cannot
    show, the steps of 5d, the number written, the steps, the time and `heap_bytes()` of each row
    of 6b and the rows that went red when the write came before `charge_stored`, the steps
    measured in 6c, the steps and N of 6d, the output of the source tests of 7a and 10 with the
    string added that turns each red, the cases of 11 with their steps, times and ratios and
    the cases that went red for each removed charge (11h), the times of `webidl-slow` before
    and after, and the output of 13a.

## Out of scope

`Element.attributes`, `Attr` and `NamedNodeMap`, which are not generated, and the five attribute
node members and `Document.createAttribute` and `createAttributeNS`, which are rows of `throws`
(an attribute has no identity in the tree, Context): they stay `throws`, excluded by the epic
(`epic:128`, `:683` and `:741 @ a14e3b0`; no story); `classList` and the token lists
(`selection.txt:88`, `reason: token lists`), which the epic excludes in the same lines (no
story; the draft `story:scripts-style-and-layout` names "`classList` effects" at its `:26` and
does not give the member a behaviour); the attributes of a `ProcessingInstruction` (rows
205-211, 7 rows of `throws`, `story/webidl-bindings.md:110`; the epic's `:964-965` says the
interface has 8 members, read at plan commit `aa70366`); an open namespace in `crates/html`,
so that `setAttributeNS` can store an attribute in a namespace outside the six (decision 9),
which is `story:html-open-namespace` (a draft); a refund of a node's charge (decision 8): a
node is never taken out of the arena and no story is planned; the comment at
`runner.rs:715-718 @ 83e6b8a` on `Limit::InputBytes` and `Limit::Steps`
(`story:scripts-dom-mutation`), because that story gives the two limits no word, as no write
makes either; creating an element (`createElement` is the slice `mutation`'s); the parse that
runs a page's scripts (`story:scripts-parse-run`); a page in a session and the example of a
link built by script (`story:scripts-document-port`); the argument price of decision 20 itself
and the 16-byte rate of `Copies::copy` (the reading unit's price commit); the order of the
glue's argument charge and its copy, which the reading unit owns (at `14f8a2a` the charge
follows the copy; from `50a4239`, its sixth fix round, on its branch, not merged, it precedes
it; the amount is the same, so no figure depends on it);
the order of the count of `longer_than` in `reads::copied` and `string_value` against their
charge (the reading unit's, Context); the pin of a
released tag of the engine, `story:swell-0-9-0`.

## Placement

Read at plan commit `aa70366`, the tip of the plan branch when this revision was written. After
`story:scripts-dom-reading` (its structure is built at `83e6b8a`; its price commit comes first,
acceptance 14) and `story:html-script-hook` (on `main`); the edges stay. The tests reach an
element through the reading story's members and its wrapper, and the cases call the write
surface of the hook story and the reads this story adds. Beside `story:scripts-dom-selectors`,
which its four lenses approved at `77aaa6a`. `story:scripts-dom-mutation` has a `depends_on`
edge to this story (since plan commit `439fd32`), so it is built second, on a base that holds
the functions of decision 8. The mutation story is being revised a third time while this one is
written, so its text cited here is that of `79bdbb2` with the scope edit of `d0f29b9`, and a
line number of it may move. The slice's code is its own
three files, `dom/attributes.rs`, `dom/attribute_reads.rs` and `dom/attribute_writes.rs`, and
the test files are its own. `crates/script/src/bindings.rs` is generated and is never merged by
hand: whichever story merges second takes `main`, regenerates it with the generator (`slot task
webidl-check` and `slot task webidl-slow` hold it; `webidl-slow` compares and writes nothing
into `bindings.rs`) and checks the digest lines (acceptance 9).

What the mutation story calls. Its text at `79bdbb2` (decision 5, file lines 283-290, and the
character-data writers and `normalize`, lines 556-557 and 567-569) already calls the functions
by the names of decision 8 and the signature of `charge_stored(added, removed) ->
DomResult<()>` before a write, `settle_stored(added, removed)` after it, `settle_stored(0, 0)`
after a write the tree refused, and the accessors (`DomState`'s four, and `DomCx`'s two,
`stored_live()` and `stored_charged()`, decision 8); it has `settle_stored` as a statement,
and decision 8 gives it `-> DomResult<()>`, which the mutation story takes in its revision.
It says its `charge_nodes` goes through the private body
that `charge_node` and `charge_stored` share, which decision 8 names `charge_to`, an associated
function that takes the engine, a handle and the bytes; `charge_nodes`
passes `charge_to` the whole size of its one `nodes` handle. Through that body `charge_nodes`
and the mutation story's settle answer the same variant for the same cause: an outer
`Err(HostError)` of the engine is `DomEnd::Fault`, which ends the run faulted. This story owns
the variant and its arm in `answered` (decision 8, "The variant"); the mutation story adds no
variant of its own, its `charge_nodes` returns `Fault` at the `?` of its charge as
`charge_stored` does, and each of its calls of `settle_stored`, which answers
`DomResult<()>` (decision 8), takes the answer with `?` after the settle, so that the
`Fault` of the outer layer ends the run there. Its `node_charges()` is the count of node
handles, 0 or 1, which decision 8 gives as the meaning after the mutation story and leaves at
the length of `charges` before it (the tests of this story hold under both). Read
against the contract of decision 8 (one `charge_stored` and one `settle_stored` for one tree
call, with the sums over the call; a settle after every call, a success included; a settle
`(0, 0)` after any `Err` of the tree), two of its calls do not follow it. `normalize` (lines
556-557) has `charge_stored(added, 0)` and the refusal's `settle_stored(0, 0)` but no
`settle_stored(added, 0)` after a success, so `live` would never rise and the handle would
stay a grow ahead of it. The character-data writers (lines 567-569) tie the `(0, 0)` settle to
a refusal that ends the script (`end_with_limit`), so an `IndexSizeError` or another `Err` that
is not a limit would leave a grown handle. These two are lines for that story's next revision
and are not edited here. A write that removes bytes (`deleteData`, `replaceData`, the setters
of `data` and `textContent`) refunds through the settle, so the mutation story's sentence that
a removal refunds nothing is true of nodes only. Whichever of the two stories is built first
uses the signatures of decision 8 as they are.

The files shared, one by one:

- `crates/script/src/bindings.rs` (generated) and `crates/webidl/src/prelude.rs`: this story
  adds `stored`, `live`, `charged`, `STORED_FACTOR`, `next_live`, `charge_to`, `charge_stored`,
  `settle_stored`, the accessors (four on `DomState`, two on `DomCx`) and the `strings_value`
  change, the variant
  `DomEnd::Fault` with its arm in `answered`, and `charge_node` calls `charge_to`;
  `story:scripts-parse-run` adds `DomState::swap_tree`; `story:scripts-dom-mutation`
  adds `nodes`, `charge_nodes` (built on `charge_to`), turns `charge_node` into a wrapper of
  `charge_nodes`, and adds the item charge and the refill charge (and no second
  `charge_stored`); `story:scripts-utf16-length` changes `string_length`. The shared region
  includes the `impl DomCx` block that holds `charge_node` (`prelude.rs:214-235 @ 83e6b8a`) and
  the struct and the `impl DomState` region: this story writes `charge_to` there and the
  mutation story, built second by its edge, builds on it; the hunks are merged by hand and
  `bindings.rs` is regenerated.
- `crates/webidl/selection.txt`: this story edits the note of row 96 (`setAttributeNS`); the
  selectors story edits nine rows (110-112, 126-127, 175-176 and 231-232 at `83e6b8a`), the
  mutation story the notes of rows 63, 151 and 157 and three `deviation:` clauses; none are
  adjacent. The header digest lines of `bindings.rs` conflict as text; the second to merge runs
  the generator again.
- `crates/html/src/tree.rs`: this story adds three reads (`attribute_position`,
  `attribute_position_ns`, `attribute_names`), three owners (`Namespace::from_url`,
  `Tree::namespace_error`, `Attribute::qualified_len`) and `Attribute::stored_bytes`, with the
  private `position_of_ns` and `lower_byte` beside `position_of`, changes the scans of five
  writes to call `position_of` and `position_of_ns` (the removing functions make no copy of
  the name, decision 3) and `put_attribute` to reserve exactly. The reading story adds the write
  counter and merges first. The mutation story's Change (the entry of `crates/html/src/tree.rs`,
  file lines 1282-1319 at revision 7) makes `parent_or_host` `pub`, adds the validity check
  `check_insertion` (with the enum `Exclude` it takes), the bulk removal `remove_all`, the work
  counter `work()` and its doc comments, and changes no recursive function, so it makes no
  change of its own to the clone. This story has three reads and
  three owners. `story:scripts-dom-sibling-positions` (a draft with no scope list; that it
  edits the child-list writers of `tree.rs` is inferred from its body, and it depends on the
  mutation story since `aa70366`) and `story:html-parser-hook-cost` name the file too. The edits
  are in different functions; the later merger takes `main` first. The `#[cfg(test)]` module is
  the last item of the file (`:1659-1692 @ 83e6b8a`); this story adds one test at its end
  (`put_attribute_reserves_exactly`), and no other story of the plan names that module.
- `crates/script/src/dom/reads.rs`: this story changes it by visibility only, three tokens:
  `fn charge` (`:120 @ 83e6b8a`), `const ATTRIBUTES_A_STEP` (`:252`) and `fn attribute_count`
  (`:255`) become `pub(super)`. `reads::copied` (`:226-237 @ 14f8a2a`, `pub(super)`) is not
  touched, not even its
  doc comment, and this story does not call it: it copies the character data of a node, and the
  function that holds the position, charges and copies an attribute's value (`copy_value`,
  decision 7) is a function of its own. The selectors story at `77aaa6a` makes seven tokens
  `pub(super)` (`charge`, `ATTRIBUTES_A_STEP`, `attribute_count`, `Reads`, `scan_for`,
  `positions`, `value_at`) and widens `Reads` to a lifetime with `&'a str`; the mutation story
  makes `charge`, `find`, `Reads` and `html_element` `pub(super)`. The three tokens of this
  story are the same edit on the same lines as the selectors story's: whichever merges second
  keeps one of them and adds none twice.
- `crates/script/src/dom/mod.rs`: this story adds `mod attribute_reads;` and `mod
  attribute_writes;` (after `mod attributes;`, which is line 7 @ 83e6b8a); the selectors story
  adds `mod selector_reads;` and a `pub use`; the mutation story two `mod` lines after `mod
  mutation;`. They are in different places of the lists; where git reports adjacent hunks the
  later merger keeps every line.
- `crates/script/tests/main.rs`: this story adds `mod dom_attributes_cost;` after `mod
  dom_attributes;` (line 17 @ 83e6b8a); the mutation story adds `mod dom_mutation_cost;` (its
  text `:1336` and `:2285` at revision 7), after `mod dom_mutation;` on line 18, one
  line from this story's: adjacent
  hunks, resolved by keeping both; the selectors story adds none (`dom_selectors` is declared
  at `:21`).
- `crates/script/tests/dom_reading.rs`: `other_slices_still_throw` (`:2399-2432 @ 83e6b8a`)
  holds three adjacent lines, `:2424` `document.appendChild(el)` (the mutation story's, its
  decision 10), `:2425` `el.getAttribute("id")` (this story's) and `:2426` `el.querySelector("p")`
  (the selectors story's, its decision 13, which says the same of this story). Each of the
  three stories changes its own line; the later merger keeps all three changes.
- `crates/script/tests/bindings.rs`: this story adds two `Writes` variants (`Stored`
  and `Fault`), gives `TestMutation` a `Record` beside its `Writes` (`TestMutation(Writes,
  Record)`) and adds the tests of acceptance 6f and 6h; the mutation story changes three
  tests' members and adds no variant; the swell story adds cases.
- `crates/script/src/runner.rs`: this story does not edit it. The mutation story edits the
  comment at `:714-718` (`:715-718 @ 83e6b8a`), which says no member of `reading`,
  `attributes` and `mutation` reaches `Limit::InputBytes` or `Limit::Steps`.
- `docs/design/scripts.md`: this story edits the paragraphs at `:322-332` and `:902-904 @
  83e6b8a`; the mutation story's Change edits the same two places (the removal and clone
  rules), so those hunks are prose written twice and are merged by hand, the later merger
  keeping both sentences (the edge puts the mutation story second). The parse-run story edits
  the file too, at `:10`, `:310` (the Q4 row of scripts run per document), `:403` (Q5) and
  `:893` (the Q11 memory bullet on the records of Q7) @ 83e6b8a, and at the notes of Q4 and Q5
  that say "measures again": `:310` is 12 lines before this story's `:322` and `:893` 9 lines
  before its `:902`, so those hunks are not adjacent to this story's. The selectors story adds
  one row to the table of Q4 after `:316`, two lines from the mutation story's `:318`. The
  swell story (decision 13, `story/swell-0-9-0.md:565-592`) edits Q4 at `:255-265`, `:302`,
  `:303`, `:307`, `:320` and `:338-341`, and Q11 at `:902-909`, `:917`, `:925`, `:963` and
  `:1079`: `:322-332` is clear of `:320` and `:338-341`, but its Q11
  range `:902-909` contains this story's `:902-904`, so that hunk is resolved by hand by whoever
  merges later, who re-reads the file after the merge.
- `AGENTS.md` (`:365-369`, one entry of five lines) is in the recorded scope of this story, as
  of the mutation story and the selectors story. The public surfaces of acceptance 13
  (`README.md`, `docs/badges/make.sh`, `docs/badges/tests.svg`, `website/docs/how-it-is-built.md`,
  `website/docs/status.md`, `website/docs/what-works.md`) are outside the recorded scope of this
  story (the epic's rule, `epic:955-961` at `aa70366`: they are not in a story's scope, because
  two units built at once both change them), and are changed in its last commit all the same,
  after merging `main`, from one gate log. The scope of `story:swell-0-9-0` and of
  `story:scripts-parse-run` still lists them, and the selectors and mutation stories name them
  in their Change and acceptance only; whichever merges later takes `main` first and its
  numbers from its own log. `docs/badges/tests.svg` is made by `docs/badges/make.sh`: the
  story edits `make.sh:38` and runs the script, and on a conflict the merger takes `main`'s
  `tests.svg`, sets the number in `make.sh` and runs it again.
- What a merger resolves by hand: `prelude.rs` (the `DomState` region and `impl DomCx`),
  `AGENTS.md:365-369`, `README.md:11` and `:285`, `docs/badges/make.sh:38`,
  `website/docs/how-it-is-built.md`, `website/docs/status.md:31`,
  `website/docs/what-works.md:17` and the gate blocks, `dom_reading.rs:2424-2426`,
  `docs/design/scripts.md`. What the generator rewrites: `bindings.rs` and `tests.svg`. What
  merges clean: `selection.txt`, `tree.rs` and `tests/bindings.rs`.
- Drafts that touch the same files, none a dependency: `story:html-parser-hook-cost` names
  `crates/html/src/tree.rs` too, and whichever merges second takes `main` first;
  `story:scripts-utf16-length` edits `prelude.rs` (`string_length`) and `bindings.rs` (the count
  of UTF-16 units) and does not move the argument charge, so it lands in the generator beside
  this story's change and the later one regenerates; `story:scripts-dom-collection-names` edits
  the `HTMLCollection` rows of `selection.txt` and the glue, not this slice's rows;
  `story:scripts-dom-sibling-positions` edits the child-list writers of `tree.rs` and
  `reading.rs`/`reads.rs` (inferred from its body), not the attribute writes this story reads,
  so a kept position never depends on an attribute; `story:html-open-namespace` (a draft) lifts
  the deviation of decision 9, after this story; `story:scripts-style-and-layout` names
  `classList` effects as style and does not touch this slice.
- The epic: the bullet of this story has been in the store since plan commit `60a4317`, and a
  corrections of it were made in plan commits `4c76597`, `9a88a16` and `dfec030`. This story
  edits no line of the epic. The first two made the bullet name the edits it had left out and
  the one handle for the document that decision 8 keeps (not one for each node's bytes); the
  third made it name the private position functions and the one charge of `strings_value`.
The last commit of the story rewrites the public-surface lines of acceptance 13 (the test
counts in `README.md`, `docs/badges/make.sh`, `docs/badges/tests.svg`, `website/docs/*`); a
commit before it leaves them.
`story:scripts-html-conformance` and `story:scripts-document-port` depend on this slice (the
epic, lines 478-479 and 486 @ a14e3b0).

## Why `epic:scripts-in-a-page`

Example 2 of the epic (lines 69-71 @ a14e3b0) sets `href` with `setAttribute` on an element a
script made, and the session shows it as a link. This story is the half of that example that is
not creating the element and putting it in the tree, and it holds every script's attribute write
to the parser's attribute and stored-byte limits, as the epic's "Security" section asks (line
234 @ a14e3b0), and to the agent's memory budget, as a node's bytes are, and every read and
write to a price that a step buys a bounded amount of work.
