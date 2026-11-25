---
format: aep.planning-md/3
id: story:scripts-dom-attributes
kind: story
status: proposed
title: The attribute members (14 rows), held to the attribute, stored-byte and memory limits and priced by what they scan
summary: Element's id, className and slot and its 11 attribute operations (17 trait methods) as the DOM Standard says, in three files (the members, every read of the tree, every write of the tree) with a source test that holds the members free of tree calls and charges, writes through the attribute functions of the tree's write surface after the standard's name checks and reads through three public reads and three owners of shared rules that the tree gains, every pass charged before it runs at one step per 16 bytes and one per 4 attributes scanned and a read of a value charging its bytes before any count, getAttributeNames charged per element, a write over the attribute or stored-byte limit ending the script through the generated limit result, the bytes script writes hold in existing nodes charged to the memory budget through one handle that the engine resizes to twice the live bytes, so that a write that removes bytes refunds them, a cost module with a floor of steps on the worst shape of each member and one budget row for each writer and each read that returns a value, a namespace outside the six of the tree named as a deviation, the TrustedType deviation tested, the brand check of the members tested, the attribute node members and the two SameObject attribute rows left to throw, the 14 case names asserted equal to the slice's rows.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:html-script-hook
- depends_on: story:scripts-dom-reading
scope:
- confidence: cited
  path: .engineering/planning/epic/scripts-in-a-page.md
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: README.md
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
  path: docs/badges/make.sh
- confidence: cited
  path: docs/badges/tests.svg
- confidence: cited
  path: docs/design/scripts.md
- confidence: cited
  path: website/docs/how-it-is-built.md
- confidence: cited
  path: website/docs/status.md
- confidence: cited
  path: website/docs/what-works.md
revision: 42
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
`main`. `story:scripts-dom-reading` is built on a branch at `83e6b8a`: its structure is in the
tree (every read of the tree in `crates/script/src/dom/reads.rs`, the members in `reading.rs`),
its prices (one step per 16 bytes, a step a token) are ruled and not built yet. Every cite below
is `file:line @ 83e6b8a`, or `@ a14e3b0` where the file is one the reading unit does not touch.
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
  run the script's `toString`), copies each String with `Copies::copy` (`bindings.rs:620-630`: a
  lone surrogate written U+FFFD, a step per 64 bytes charged after the copy; the reading story's
  decision 20 rules one step per 16 bytes and keeps the charge after the one copy, bounded by
  `string_length`, because the engine reads a String only by copying it: it is not built, and
  the move of that charge before the copy, which that decision gives to
  `story:scripts-utf16-length`, is in no story's text yet), calls the slice
  and gives the answer to `finish` (`bindings.rs:947-956`, `answered` `:958`): `DomEnd::Throw`
  becomes a `DOMException`, `DomEnd::StringLength` a `RangeError`, `DomEnd::Limit` goes to
  `end_with_limit` (`bindings.rs:512`), which records the limit and ends the script where it is,
  so no `catch` and no `finally` of the script runs. `Runner::document_end`
  (`crates/script/src/runner.rs:694`) turns the record into the end: `Limit::Attributes` is
  `limit: attributes` and `Limit::StoredBytes` is `limit: stored-bytes` (`runner.rs:713-714`,
  words at `:316-317`); `Termination::MemoryBudget` is `limit: memory` (`runner.rs:311`, `:916`).
  `docs/design/scripts.md` gives the same words (Q4 rows at lines 316-317; Q7 at line 435).
- The glue charges what it answers: a String at one step per 16 bytes before the copy
  (`string_value`, `bindings.rs:1263-1270`, `STRING_BYTES_A_STEP` `:1257`), a collection node at 8
  steps (`COLLECTION_NODE_STEPS`, `:1251`). `getAttributeNames` answers a `sequence<DOMString>`
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
  namespace (`attribute_name`, `tree.rs:1489-1497`, called by `set_attribute`,
  `remove_attribute` and `toggle_attribute`) and matches a qualified name against prefix, colon
  and local name (`qualified_name_is`, `tree.rs:695-706`, private). Each of the five scans the
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
  keeps one handle for each node a script makes (`DomState::charges`, `prelude.rs:112`).
- Memory the 64 MiB budget does not see, which this story bounds or names. The glue copies each
  argument before any charge (`Copies::copy`, `bindings.rs:620-630`): `to_utf16` makes a
  `Vec<u16>` of up to `string_length` units, 16,777,216 (`runner.rs:46`), 33,554,432 bytes, and
  `String::from_utf16_lossy` a `String` of up to 3 bytes a unit, 50,331,648 bytes, both live at
  once (the capacity of the `String` may exceed its length; not measured); `setAttributeNS`
  has three such arguments. A write then lets the tree copy the value with `to_string()`
  (`tree.rs:1549`) before `put_attribute` checks a limit, so the glue's `String` and the tree's
  copy live at once, and a replacement holds the old value and the new. A name is lowered
  into a copy by `attribute_name` (`tree.rs:1491-1493`) in each write. These are bounded by
  `string_length` and by the stored-byte limit and are not charged to `memory_bytes` by the glue
  or by this story (the engine's own copies, which `story:swell-0-9-0` records as charged at
  `0.10.0`, are another matter); the story removes the cheap parts: a read compares a name
  without a lowered copy, `attribute_names` makes its list with `Vec::with_capacity(count)`,
  the tree copies a value with `to_string()` and never takes the glue's `String`, and an error
  message holds no argument's text (decision 4).
- How a slice charges steps, as built. `reads::charge(cx, steps: usize) -> DomResult<()>` is
  private to `reads.rs` (`reads.rs:118-131`): it calls `cx.engine().charge_steps(steps)`, does
  nothing for 0, and when the engine refuses answers `DomEnd::Throw(InvalidStateError "the steps
  of the script are spent")`, a throw the engine does not deliver: the termination is sticky and
  the run ends. `DomCx::charge_node` fails the same way for the memory budget. A read that
  returns a String is held to `string_length` first (`longer_than`, `bindings.rs:145`, giving
  `DomEnd::StringLength`), then charged, then copied (`reads::copied`, `reads.rs:139-153`; the
  glue's `string_value`, `bindings.rs:1263-1270`, has the same order). `longer_than` counts the
  UTF-16 units of the whole value once its bytes pass `string_length`, a pass of up to the
  value's length that nothing has charged yet: a defect of the reading unit, passed to its fix
  round, which charges the byte length before the count (acceptance 14 reads it). A scan by
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
an element by `getElementById` or `document.documentElement`.

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
     tree only through `attribute_reads`. It holds no `tree()`.
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
3. `crates/html/src/tree.rs` gains three public reads, three small public owners of rules the
   tree and the slice would otherwise copy, and the writes call all of them, so that the slice
   has one matching rule with the writes, one rule for the six namespaces, one for the four
   `NamespaceError` conditions and one count of an attribute's bytes. Each read is `&self` and
   takes the element's `NodeId`:
   - `attribute_position(&self, element: NodeId, name: &str) -> Option<usize>`: the position
     in the element's list of the first attribute whose qualified name is `name`, `None` for
     none and for a node that is not an element (`qualified_name_is`, `tree.rs:695-706`;
     `dom.html:10175`). For an element in the HTML namespace the name is lowered, but the
     compare lowers the wanted name byte by byte and makes no copy (the `attribute_name` of
     `tree.rs:1489-1497` copies);
   - `attribute_position_ns(&self, element: NodeId, namespace: Option<Namespace>, local_name:
     &str) -> Option<usize>`: the first attribute whose namespace and local name equal the
     arguments (`dom.html:10192`);
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
   - `Tree::namespace_error(namespace: Option<&str>, qualified_name: &str) -> bool`, an
     associated function that reads no tree: whether "validate and extract" throws
     `NamespaceError` for that namespace URL (`None` is null) and that qualified name, the
     four conditions of `tree.rs:1569-1577`. `set_attribute_ns` calls it with
     `namespace.map(Namespace::url)` and the slice calls it with the argument before any
     charge for memory, so the rule has one text and a namespace outside the six is judged by
     the same conditions.
   What the writes change: `set_attribute`, `remove_attribute` and `toggle_attribute` find the
   attribute with `attribute_position` and `set_attribute_ns` and `remove_attribute_ns` with
   `attribute_position_ns` where they scan with `position` today (`tree.rs:1541`, `:1579-1581`,
   `:1599`, `:1614-1616`, `:1635`), so a read and a write cannot disagree; none of their
   answers changes. `put_attribute` reserves exactly while the list has fewer than four
   entries (`reserve_exact(1)` before the push at `tree.rs:1522`), so that the bytes the tree
   counts are the bytes it holds for the first four attributes (decision 8); a unit test
   beside `tree.rs:1690` reads `attributes.capacity()` after each of the first four adds.
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
   local name. `setAttributeNS` does the whole of "validate and extract" (`dom.html:835`): the
   empty namespace is null; a prefix that is not a valid namespace prefix, then a local name
   that is not a valid attribute local name, throw `InvalidCharacterError`; then the four
   `NamespaceError` conditions, which the slice tests with `Tree::namespace_error` (decision 3),
   the one function that `Tree::set_attribute_ns` calls too, so that no write that is then
   refused for a `NamespaceError` has been charged for memory. The tree's answer
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
   argument it copies and for each String it answers. The glue's charge for an argument is the
   one exception: it follows that argument's one copy (Context) until a story moves it, and no
   story's text holds that move yet, `story:scripts-utf16-length` included; every charge of the
   slice comes before its pass. The rates are `S`, one step per 16 bytes
   (`STRING_BYTES_A_STEP`), and `A`, one step per 4 attributes passed in a scan by name
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
     count, the reverse of `reads::copied` and of the glue's `string_value`, which count before
     they charge (Context): the slice does not inherit that, and the case of acceptance 11d
     holds it. The reading unit's fix round is to give `reads::copied` the same order; the
     two functions are copied on purpose, because `copied` reads the character data of a node
     and its fix round rewrites its body, so a function shared between them would change a
     built function of another unit's signature; each has its own case. The glue then charges
     its own copy of the String it answers, `v / S` again: two passes over the bytes, two
     charges.
   - `getAttributeNames`: the order is (a) charge `c`, one step per attribute, from the count;
     (b) one pass over the list that adds up the names' lengths (`Attribute::qualified_len`);
     (c) charge the sum over `S`; (d) the tree's `attribute_names`, which builds the list, the
     second pass; then the glue charges the bytes of each name. The generator's `strings_value`
     charges `STRING_ELEMENT_STEPS`, 8 as a collection node does (`COLLECTION_NODE_STEPS`,
     `bindings.rs:1251`), for each element before its loop; this story changes `strings_value`
     in `crates/webidl/src/prelude.rs` (`:1441-1454`) and regenerates. The weight 8 is a
     choice: the cost module prints the time a step of `getAttributeNames` against the plain
     walk (acceptance 11), and a ratio over 20 raises the weight in the same change. So a list
     of `c` names costs at least `9 * c` steps.
   - A pass over an argument (the validation of a name, or of the prefix and local name of
     `setAttributeNS`): the bytes of the pass over `S`, charged before it runs. The split of a
     qualified name at its first colon is made once in the slice, inside that validation pass,
     and paid by its charge; the tree splits it again when it writes, inside its copy of the
     name, paid by the `(n + v) / S` charge below; the compare of `find` reads the local name
     only, paid by `k * l / S`; the glue's copy of the name is paid by the glue's charge. Each
     pass over the name has one charge.
   - A write. The order of a member is decision 8's. Its steps, each charged before the work
     they pay for: the validation, whose pass over a name is charged (`n / S`) and then made;
     `find`, a scan as above (two passes over the list, (b) and (d)); then, before the call of
     the tree, the charge of the tree's own scan, `c / A + k * n / S` with the `k` that `find`
     answered (the list has not changed since, so the pass that counts `k` is not made
     again), and `(n + v) / S` for the bytes the tree copies, which is the last charge before
     the write (`n` the bytes of the name it stores, `v` of the value, whether it adds an
     attribute or replaces a value; `toggle` that adds, `v` is 0; `remove` copies nothing);
     then the call, whose own scan is a third pass over the list. A write that finds or adds
     so makes three passes over the list, each paid: the first two by `find`'s charges and the
     third by the second `c / A` and byte charge; the shift of `drop_attribute` is at most `c`
     moves, paid by the same `c / A`.
   So a write that adds costs at least `2 * (c / A) + (n + v) / S` steps of its own, beyond the
   glue's one step and its charge for the two argument copies, `(n + v) / S` again; the scan
   term is counted twice (`find` and the tree's own).
8. The bytes a script writes into nodes that exist are charged to the memory budget through one
   handle that the engine resizes, so a write that removes bytes gives them back. The state of
   the bindings, `DomState` (the struct that holds `totals` and `charges`,
   `crates/webidl/src/prelude.rs:104-116`, written into `bindings.rs`), gains, all empty or 0 at
   `DomState::new` (`prelude.rs:120-131`):
   - `stored: Option<HostBytes>`: the one handle for the bytes scripts wrote into existing
     nodes, `None` until the first write that adds bytes; it is not an entry of
     `DomState::charges`, which this story gives no entry (the node charges stay as they are);
   - `live: u64`: the stored bytes that script writes added, less those that script writes
     removed or replaced away, by the tree's own count of an attribute (`Attribute::stored_bytes`,
     decision 3); never below 0, so removing what the parser stored brings it to 0 and no lower;
   - `charged: u64`: the bytes the handle holds, twice `live` after each settle; `Totals` stays
     the node totals: the bytes of this handle are not added to it.
   The handle holds twice `live`, the same factor as the node charge (`charge_node`,
   `prelude.rs:214-235`: the tree's heap bound counts a stored byte twice, as the engine's
   `String` growth and the tree's copy may both be live). The engine's function is
   `Cx::resize_host_bytes(&mut self, charge: &mut HostBytes, bytes: u64) ->
   Result<Completion<()>, HostError>` (`crates/swell/src/host/mod.rs:585` at `0.8.0`, the tag
   `main` pins, and `:628` at `0.10.0`, the tag of `story:swell-0-9-0`; its body
   `scoped_resize_host_bytes`, `host/host_bytes.rs:79-106`, is the same at both): a grow first
   credits the charges that were dropped, then charges the difference, and when the budget does
   not hold the difference it charges nothing, leaves the handle at its bytes and ends the run
   in `Terminated(MemoryBudget)`; a shrink credits the difference at once; a handle that
   another engine made answers `Err(ForeignHandle)`. It calls `running()` first, so after the
   run has ended it errors and shrinks nothing: a settle comes before any end of the script.
   The first charge, with no handle yet, is `charge_host_bytes` (as `charge_node` calls it)
   and the handle it answers is kept in `stored`.
   The prelude gains, beside `charge_node`, the functions that a slice calls. Their names and
   signatures are final; `story:scripts-dom-mutation` calls them for its character-data writes
   (Placement):
   - `DomCx::charge_stored(&mut self, added: usize, removed: usize) -> DomResult<()>`: before a
     write. The new live is `(live + added)` less `removed`, not below 0, and the wanted
     charge is twice that. When the wanted charge is above `charged` it is a grow: the handle
     is made (first) or resized to the wanted charge, and over the budget nothing changes
     (neither `live`, `charged` nor the tree) and the run ends `Terminated(MemoryBudget)`,
     `limit: memory`, the answer being a throw the engine does not deliver, as with
     `charge_node`. When the wanted charge is not above `charged` it does nothing: the shrink
     comes after the write;
   - `DomCx::settle_stored(&mut self, added: usize, removed: usize)`: after a write. It sets
     `live` to `(live + added)` less `removed`, not below 0, and resizes the handle to twice
     `live` (a shrink when bytes went; a grow that `charge_stored` already made is not made
     twice), ignoring an error of the engine (the run has ended, nothing is left to refund).
     After a write that the tree refuses (a limit), the member calls `settle_stored(0, 0)`
     before it returns the end, which puts the handle back to twice `live`, the bytes of the
     tree as they were; the script ends;
   - the accessors `DomState::stored_live() -> u64`, `DomState::stored_charged() -> u64`,
     `DomState::stored_handles() -> usize` (0 or 1) and `DomState::node_charges() -> usize`
     (the length of `charges`), which the tests read beside `DomState::totals()`
     (`prelude.rs:157`);
   - the doc comments of `tree_mut` (`prelude.rs:177-181`) and of `Totals::bytes`
     (`prelude.rs:92-99`) name them; `charge_node` and `charge_stored` share one private body
     for the call of the engine and the mapping of its answer.
   So the charge is twice the bytes scripts hold now, and not their peak: a loop that sets and
   removes one attribute keeps the handle at one size, and removing what a script wrote lowers
   the charged bytes at once. The node charge of `charge_node` stays as it is: a node is never
   taken out of the tree's arena (`tree.rs:19-21`), so nothing of it is ever released, where a
   stored byte is freed at a removal (`drop_attribute`, `tree.rs:1528-1534`). What a script
   removes of what the parser stored lowers `live` no further than 0, so the handle counts only
   what scripts wrote: the tree holds at most what the parse stored plus `live`. A removal of
   parser bytes gives nothing back to the budget, because the parser's bytes were never charged
   to it.
   The order of a member of this slice, all of it in `attribute_writes`: validate (decision 4);
   `find`, which charges the scan and answers the sizes; compute the bytes the write adds and
   removes by the tree's count; charge the steps of decision 7; call `charge_stored`; write; on
   a write that the tree refuses (a limit), call `settle_stored(0, 0)` and return the end; else
   call `settle_stored(added, removed)`. The bytes, from what `find` answers (`Found`, `None`
   for none):
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
     Some(true) => Ok(true)` of `toggle_attribute`, `tree.rs:1627`), so the member makes no
     call of `charge_stored` or `settle_stored` and no tree call; the steps of the scan stay
     charged;
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
    and its tree's `stored_bytes()`. A value
    of many characters is built by the script with `"x".repeat(n)`, which the engine has
    (`crates/script/tests/adversary_3.rs:796`).

## Change

- `crates/html/src/tree.rs`: the three reads of decision 3 (`attribute_position`,
  `attribute_position_ns`, `attribute_names`), each with a doc comment that says what it
  matches, what it answers for a node that is not an element and, in a line that starts with
  `Cost:`, what it costs; the three owners (`Namespace::from_url` beside `Namespace::url`,
  `Tree::namespace_error`, `Attribute::qualified_len` beside `Attribute::stored_bytes`);
  `Attribute::stored_bytes`, which the private `attribute_bytes` then calls; the five writes
  that scan call the position reads, `set_attribute_ns` calls `namespace_error`, and
  `put_attribute` reserves exactly while the list has fewer than four entries; the module
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
  `attributes.capacity()` at `:1690`), because it needs the private list: it reads
  `attributes.capacity()` after each of the first four adds. The count is not a tree read:
  the slice calls `reads::attribute_count` (decision 2), so there is no
  `Tree::attribute_count`.
- `crates/webidl/src/prelude.rs`: the fields `stored`, `live` and `charged` of `DomState` and
  their initial values in `DomState::new`, `DomCx::charge_stored`, `DomCx::settle_stored`, the
  accessors `DomState::stored_live`, `stored_charged`, `stored_handles` and `node_charges`, the
  doc comments of decision 8, and the charge of `STRING_ELEMENT_STEPS` for each element in
  `strings_value` (decision 7); `crates/script/src/bindings.rs` regenerated with `slot cargo
  run -p surf-webidl --bin bindings`. The header's digests that change are those of
  `crates/webidl/selection.txt`, of `crates/webidl/src/prelude.rs` and the whole-file digest on
  the last line; the other lines of the digest block stay as they are on the merged `main`.
- `crates/webidl/selection.txt`: the note of `Element setAttributeNS` (line 96) gains the
  deviation of decision 9; the doc comment of the method carries the note (`bindings.rs:3200`).
- The epic, `.engineering/planning/epic/scripts-in-a-page.md`: the bullet of this story in the
  section "Corrections of 2026-11-25" is in the plan store since plan commit `60a4317`. This
  story edits no line of the epic and its scope list does not name the file; the corrections
  to that bullet (its cites and the sentence on the refund) are a plan edit made in the
  store, outside this story.
- `crates/script/src/dom/attribute_reads.rs` (new): `reflected`, `any`, `names`, `get`,
  `get_ns`, `has`, `has_ns`, `find`, `namespace_of` and the scan charge (decisions 2, 7).
- `crates/script/src/dom/attribute_writes.rs` (new): `set_reflected`, `set`, `set_ns`, `remove`,
  `remove_ns`, `toggle`, the valid attribute local name and valid namespace prefix tests, the
  call of `Tree::namespace_error`, the bytes a write adds and removes, the charge and the
  settle of a write, also after one the tree refuses (decisions 2, 4, 5, 7, 8, 9).
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
  of 50,000,000 steps and 256 MiB, `raised()`, `call`, `report`, `plain`); it defines its own
  copies of those helpers.
- `crates/script/tests/bindings.rs`: a variant of `Writes` whose `Document.createTextNode`
  reads its argument as two numbers, `added removed`, and calls `charge_stored` with them as
  the `Writes::Charge` arm calls `charge_node` (`tests/bindings.rs:175-178`), and then calls
  `settle_stored` with the same two numbers (with `0 0` when the argument ends in `!`, the
  case of a write the tree refuses); and the test of acceptance 6f beside `node_charge`
  (`tests/bindings.rs:917-954`).
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
  This story alone edits these two places for the memory charge (Placement).
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
      as `data-x`) and on an SVG element, whose names the parser keeps in mixed case
      (`getAttribute("viewBox")` answers, `getAttribute("viewbox")` is `null`); an attribute in
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
   g. The brand check, which the epic promises ("a member called on an object of another brand
      throws a `TypeError`", epic `:583`) and which `tests/bindings.rs:419 @ 83e6b8a` samples
      for one member and one getter only. Three cases, in the form of that test, each in its
      own `try`: `Element.prototype.setAttribute.call(document, "a", "b")`; the `id` setter
      taken from the property descriptor of `Element.prototype` and called on `document`; and
      `Element.prototype.getAttribute.call(textNode, "a")` with a text node of the page. Each
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
      likewise (the glue's copy of the argument and the tree's copy; the floor assumes the
      argument price of decision 20 is merged, acceptance 14).
   c. Before the work: one helper, `budget_row`, over rows, in `dom_attributes`, one row for each
      of the eight writers and each member that returns a value. The glue's own copy of a 1 MiB
      argument costs 65,536 steps, so a budget under that ends the script in the glue and shows
      nothing about the slice; each row sets the budget between the charges that come before the
      slice's last charge and that charge. A row names its page, a setup script, the call, and
      `parts`, the list of the charges the call makes in their order, each computed from the
      prices of decision 7 (the arithmetic is in the row). The helper makes four runs of the
      row, each on its own runner, in one `#[test]`, and the rows that share a page parse it
      once:
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
        `setAttribute("a", v)`: [65,536 (the glue's copy of `v`; the name of 1 byte costs 0),
        65,536 (the bytes the tree copies, (1 + 1,048,576) / 16)];
        `e.id = v`: [65,536, 65,536 ((2 + 1,048,576) / 16)];
        `e.className = v`: [65,536, 65,536 ((5 + 1,048,576) / 16)];
        `e.slot = v`: [65,536, 65,536 ((4 + 1,048,576) / 16)];
        `setAttributeNS("http://www.w3.org/2000/svg", n, "")` with `n = "x:" +
        "y".repeat(1048574)`: [65,537 (the glue: 1,048,576 / 16 for the name, 26 / 16 = 1 for the
        URL), 65,535 (the validation of the local name, 1,048,574 / 16; the prefix of 1 byte costs
        0), 65,535 (the bytes the tree copies, (1 + 1,048,574) / 16)];
        `toggleAttribute(n)` with `n = "y".repeat(1048576)`, adding it: [65,536 (the glue),
        65,536 (the validation), 65,536 (the bytes the tree copies, (1,048,576 + 0) / 16)].
        Each row's effect is the attribute set to `v` (or `n` added, in the SVG namespace for
        `setAttributeNS`), read back through the tree.
      - The two removal rows, on a page `<html n0="" n1="" ... n16382="" LONG="">` whose last
        attribute has the name `LONG`, `"z".repeat(1048576)` (16,384 attributes; the test builds
        the page string once and parses it under `Limits` raised for it); `k` is 1, as no other
        name has that length. `removeAttribute(n)` with `n = "z".repeat(1048576)`: [65,536 (the
        glue), 4,096 (`find`'s scan, 16,384 / 4), 65,536 (`find`'s bytes, 1 * 1,048,576 / 16),
        4,096 (the tree's own scan), 65,536 (the tree's bytes)]; `removeAttributeNS(null, n)`
        likewise, with the glue's charge for the local name and the scans by pair
        (`k * l / S`, `l` the same 1,048,576). The last part is a charge of 65,536 and the
        budget B ends in it with the attribute still on the element; the effect is the element
        without it.
      - The read rows, with the value from the page: `<html a="V">` with `V` of 1,048,576 bytes
        built into the page string once and parsed under `Limits` raised for it; the script
        `var e = document.documentElement;`. `parts` is [the slice's charges, then G2, the
        glue's charge for the String it answers], and B1 = S0 + M + (the slice's charges) + G2 /
        2 ends `limit: steps-per-script` in the glue's charge; B2 = S0 + C runs. The helper
        asserts `sum(parts) <= C <= sum(parts) + 1,000` for these rows too, so that M stays far
        under G2 / 2 = 32,768 and B1 ends where the row says. `getAttribute("a")`,
        `getAttributeNS(null, "a")`: [65,536 (`V` / 16), 65,536 (G2)]; the `id`, `className` and
        `slot` getters on `<html id="V">`, `<html class="V">` and `<html slot="V">`:
        [65,536, 65,536]; `getAttributeNames()` on `<html n0="" ... n4095="">`: [4,096 + 1,210
        = 5,306 (the count, and the names' 19,370 bytes over 16: 10 names of 2 bytes, 90 of 3,
        900 of 4, 3,096 of 5), 8 * 4,096 = 32,768 (the glue's per-element step; each name is
        under 16 bytes and its bytes charge is 0)]. A read that charged nothing would pass
        with B1.
      - What these rows cannot show. A charge whose work has no visible effect before the budget
        ends (a scan, a validation) passes at any place before the last charge, so the order of
        a scan or a validation against the work it pays for is held by review and by the source
        test of acceptance 10, not by a row. For a read, a charge that comes after the copy
        ends at the same place as one that comes before it, so the order of the slice's charge
        and its copy is held by review too; acceptance 11d holds the one case that shows it (a
        value over `string_length`). The amounts are held by 5b.
      - The terms assume the argument price of decision 20 (16 bytes a step), merged first
        (acceptance 14). The case records S0, S1, C and M of each row in a comment. In the
        report, for each writer row, the implementor moves the row's last charge to after the
        write, and for each read row removes the slice's charge, runs the helper and reports the
        rows that went red, as in 11h.
   d. `var a = p.children; for (i = 0; i < p.children.length; i++) p.children[i].className = "x";`
      over 5,000 children ends `Ran`, with no recorded limit, with `runner.steps()` under 15
      times those of the same loop over 500 children, and `a === p.children` afterwards. The
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
   `stored_bytes()` before and after, starts with `stored_live()`, `stored_charged()`,
   `stored_handles()` and `node_charges()` at 0, and gives `Document` the session's
   `memory_bytes` of 67,108,864 (`SESSION_ENGINE_LIMITS`, `runner.rs:40-48`) and the default
   `Limits` unless it says otherwise. The figures of 6a to 6f (80, 837, 1,109,930 and the rest)
   hold only because `ATTRIBUTE_BYTES` is `size_of::<Attribute>()` (`limits.rs:90-92 @
   a14e3b0`), 80 on a 64-bit target: each case reads `surf_html::limits::ATTRIBUTE_BYTES` and
   computes its numbers from it, and the figures below are comments in the case, stated for a
   value of 80. Throughout, `totals()` stays (0, 0) and `node_charges()` stays 0: this story
   adds no entry to `DomState::charges` and no node.
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
      (`stored_live()` stays 0 and `stored_charged()` stays as it was) and every throw of 1d and
      1e leave `stored_live()` and `stored_charged()` where they were.
   b. Memory first. On `<p></p>` (the `html` element of it has no attribute), through
      `document.documentElement`, a loop `for (i = 0; ; i++) e.setAttribute("a" + i, v)` with
      `v` a string of 40,000 ASCII characters built by the script, ends `limit: memory` with
      fewer than 1,024 attributes and at most 837 written. Nothing is removed, so each write
      grows the handle by twice its own bytes: 2 * (80 + len(name) + 40,000) is 80,164 bytes
      for `a0` to `a9`, 80,166 for `a10` to `a99`, 80,168 from `a100`; 837 writes charge
      67,100,396 bytes, and the 838th would pass the budget of 67,108,864 even with an empty
      engine heap; 1,024 writes would charge 82,091,860, so the attribute limit is not reached;
      837 writes store 33,550,198 bytes, just under half the stored-byte limit of 67,108,864,
      so that limit is not reached either. The engine's own heap makes the end come earlier,
      never later. Steps: each write costs the two copies of the value, 2 * 40,000 / 16 =
      5,000, and the scans, which grow with the attributes held: about 4.4 million steps for 837
      writes, under the 10,000,000 of an entry. The case asserts the end, `attributes written <=
      837`, that `stored_charged()` equals twice `stored_live()` and twice the rise of
      `stored_bytes()`, that `stored_handles()` is 1, and that the write the budget refused
      changed no count and wrote nothing, and records the number written, the steps and
      `heap_bytes()` in a comment.
   c. Attributes first. The same loop with `v` of 1,000 bytes ends `limit: attributes` at the
      1,025th write: the first 1,024 writes store 1,024 * 1,080 + 4,010 = 1,109,930 bytes (4,010
      being the bytes of `a0` to `a1023`) and hold 2,219,860; the 1,025th adds 1,085 bytes
      (`a1024` is 5 bytes), so the handle grows to 2 * 1,111,015 = 2,222,030 before the tree
      refuses the write, and `settle_stored(0, 0)` puts it back; 2,222,030 is far under the
      budget. The case asserts the end, that the element holds 1,024 attributes,
      `stored_live()` of 1,109,930, `stored_charged()` of 2,219,860 (put back after the refused
      write) and `stored_handles()` of 1. The steps are about 0.4 million.
   d. A repeat is a grow and a refund, not a sum. A loop of N = 50 rounds of
      `e.setAttribute("a", v); e.removeAttribute("a")` with `v` of 1,048,576 bytes ends `Ran`,
      with no recorded limit. A set adds 80 + 1 + 1,048,576 = 1,048,657 bytes and grows the
      handle to 2 * 1,048,657 = 2,097,314; the removal lowers `stored_live()` by the same
      1,048,657 to 0 and the handle to 0. After the N rounds `stored_live()` is 0,
      `stored_charged()` is 0 and `stored_handles()` is 1. A charge that only grew would end
      `limit: memory` at the 32nd round (31 * 2,097,314 = 65,016,734; 32 * 2,097,314 =
      67,114,048 is over 67,108,864); the case holds the opposite, which needs N above 31.
      After the loop one more `e.setAttribute("a", v)` leaves `stored_charged()` at 2,097,314.
      N, derived: a set costs two copies of its value (the glue's and the tree's) at 16 bytes
      a step, 2 * 1,048,576 / 16 = 131,072 steps; an entry ends at 10,000,000 steps
      (`steps_per_entry`, `runner.rs:41`); N rounds and the last set are N + 1 sets, and
      (N + 1) * 131,072 <= 10,000,000 gives N + 1 <= 76 and so N <= 75 before the validation,
      the scans and the loop's own steps. N is 50: 51 * 131,072 = 6,684,672 steps of copies,
      leaving 3,315,328 steps for the rest. The case records `runner.steps()` in a comment and,
      if the first measure shows less room than that, lowers N to a round number above 31 that
      fits, and says so in the report.
   e. Growth by replacement. On the same page, `e.setAttribute("a", v)` with `v` of 1,024 bytes,
      then of 2,048, 4,096 and on by doubling to 1,048,576 bytes (11 sets), leaves
      `stored_live()` at 81 + 1,048,576 = 1,048,657 and `stored_charged()` at 2 * 1,048,657 =
      2,097,314: twice the last length plus the fixed terms (80 for the record, 1 for the
      name), not the sum of the 11 sets' own bytes, which would be 2 * (11 * 81 + 2,096,128) =
      4,194,038. Then a replacement by a value of 1,024 bytes lowers `stored_live()` to 1,105
      and `stored_charged()` to 2,210, the refund of 2,095,104; and doubling again to
      1,048,576 bytes brings them back to 1,048,657 and 2,097,314, each grow charged by its
      difference. `stored_handles()` is 1 throughout. The steps are about 524,032 (the 22
      sets' values, 4,192,256 bytes, twice over, at 16 bytes a step) plus their own cost.
   f. The functions themselves, in `crates/script/tests/bindings.rs`: through the test slice,
      from (`stored_live()`, `stored_charged()`, `stored_handles()`) of (0, 0, 0), each call of
      `charge_stored(added, removed)` followed by `settle_stored(added, removed)` giving these
      three after it, and `totals()` at (0, 0) and `node_charges()` at 0 throughout: add 1,000
      gives (1,000, 2,000, 1); add 500 gives (1,500, 3,000, 1); remove 700 gives (800, 1,600,
      1), 1,400 refunded; add 600 gives (1,400, 2,800, 1); add 300 gives (1,700, 3,400, 1);
      remove 5,000, more than `live`, gives (0, 0, 1), never below 0; add 900 and remove 400 in
      one call gives (500, 1,000, 1); a call with both 0 changes nothing. A refused write,
      `charge_stored(100, 0)` then `settle_stored(0, 0)`, from (500, 1,000, 1) leaves (500,
      1,000, 1): the handle grew and was put back. Many raises: 10,000 calls of add 1, remove 0
      leave `stored_handles()` at 1 and `stored_charged()` at 20,000 (one handle, not 10,000
      entries), and then remove 5,000 lowers `stored_charged()` to 10,000. A refused charge:
      with `memory_bytes` of 1 MiB, an add of 10,000,000 ends `Reason::Memory` and leaves
      (`stored_live()`, `stored_charged()`, `stored_handles()`) as they were. The heap grows
      by at least the charge beyond an uncharged control, as `node_charge` measures it
      (`tests/bindings.rs:917-954`).
7. The reads and owners of the tree (decision 3), in the module `script_hook` of
   `crates/html/tests` (the unit test of 7f in `tree.rs`).
   a. Each of the three reads, `Namespace::from_url`, `Tree::namespace_error`,
      `Attribute::stored_bytes` and `Attribute::qualified_len` has a test (`stored_bytes` equals
      the rise of `Tree::stored_bytes` for each attribute `set_attribute`, `set_attribute_ns`
      and `toggle_attribute` add); `attribute_position` lowers a name for an HTML element and
      not for an SVG one, byte by byte; `attribute_position_ns` matches by namespace and local
      name and not by prefix; `attribute_names` keeps order and duplicates and gives
      `xlink:href` for a prefixed attribute, and each of its strings has the length
      `Attribute::qualified_len` answers for that attribute; `Namespace::from_url` answers
      `Some(ns)` for `ns.url()` of each of the six and `None` for `""`, a near miss (a URL with
      one byte changed) and an unknown URL.
   b. One matching rule: for a list of names (capitals, a prefixed name, a duplicate, an absent
      one) on an HTML and an SVG element, `attribute_position(..).is_some()` is true exactly
      when `remove_attribute` of the same name on a clone of the tree lowers the attribute
      count, `attribute_position_ns` likewise against `remove_attribute_ns`, and the position
      answered is the position `set_attribute` replaces in (the order of `attribute_names` is
      kept by a replacement).
   c. A node that is not an element (a text node, the document) answers `None` and an empty
      list.
   d. One rule for `NamespaceError`: for a table of (namespace, qualified name) pairs holding
      each of the four conditions of `tree.rs:1569-1577` and a pair that passes,
      `Tree::namespace_error` answers true for the first four kinds and false for the last, and
      `set_attribute_ns` with the same pair answers `WriteError::Namespace` exactly when
      `namespace_error` is true (for a namespace of the six).
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
   and of `crates/webidl/src/prelude.rs` and no other source's, and `git grep -c -F
   'charge_stored' -- crates/script/src/bindings.rs` prints a number above 0 (nothing at
   `a14e3b0`).
10. Files, and the source test. `git diff --stat $(git merge-base main HEAD)..HEAD` names only
    the files of the Change and of the public surfaces (a guard); it names nothing else under
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
    `dom_reading_cost.rs:594`) and asserts: `attributes.rs` holds none of `tree()`,
    `tree_mut()`, `charge(`, `charge_steps`, `charge_stored` and `settle_stored`;
    `attribute_reads.rs` holds none of `tree_mut()`, `charge_stored` and `settle_stored`;
    `attribute_writes.rs`
    holds no `tree()`. The test is red when one of those strings is added to the file that must
    not hold it (the implementor shows it once, with the string added, and reports the output).
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
    process, so the page of 11a, 11b and 11c is parsed once for the three, and one test
    holds 11d. Under raised parse limits (16,384 attributes, room for long values) and
    `string_length` of 1 << 22:
    a. `getAttribute` and `hasAttribute` of an absent name over an element of 4,096 attributes
       whose names are all 1,000 bytes long, the wanted name 1,000 bytes long and differing
       from every name only in its last four bytes (every compare reads the whole name; the
       names are distinct, 996 letters and a 4-digit number, the wanted one 996 letters and
       `zzzz`, so `k` is 4,096): floor 4,096 / 4 + 4,096 * 1,000 / 16 = 1,024 + 256,000 =
       257,024, from the formula of decision 7 (the name is the slice's term; the glue's charge
       for the 1,000-byte argument, 62, is not in the floor).
    b. `getAttributeNS`, `hasAttributeNS` and the `id` getter over 16,384 attributes with short
       numbered names (`n0` to `n16383`), the wanted local name `x` (1 byte, so `k` is 0): floor
       16,384 / 4 = 4,096 each.
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
       slice's alone. A slice that counted before it charged and threw would raise the steps by
       less than the first of these and fail, and one that charged the count in the first band
       only once fails the 524,288. The page holds the three values in one attribute each, in
       one `#[test]`, under raised parse limits.
    e. `setAttribute` with a value of 4,194,304 bytes on an element of 16,384 attributes
       (`n0` to `n16383`), adding a name of 1 byte: floor 2 * 4,194,304 / 16 (the glue's copy
       and the tree's, 524,288) + 2 * 16,384 / 4 (two charged scans, three passes, 8,192) =
       532,480; the `id` setter (which shares `set_reflected` with `className` and `slot`)
       with the same value likewise; `setAttributeNS` with a qualified name of 1 MiB (`"x:" +
       "y".repeat(1048574)`), the SVG namespace URL and an empty value: floor 65,537 (the glue)
       + 65,535 (the validation) + 65,535 (the tree's bytes) + 8,192 (the scans, `k` is 0) =
       204,799. `toggleAttribute` that adds a name of 1 MiB and `setAttribute` with a name of
       1 MiB and an empty value: floor 65,536 (the glue) + 65,536 (the validation) + 65,536 (the
       tree's bytes) + 8,192 = 204,800 each.
    f. `removeAttribute("n16383")` and `removeAttributeNS(null, "n16383")` of the last of 16,384
       attributes `n0` to `n16383`: `k` is 6,384 (the names of 6 bytes, `n10000` to `n16383`),
       so each scan charges 16,384 / 4 + 6,384 * 6 / 16 = 4,096 + 2,394 = 6,490, and the floor
       is 2 * 6,490 = 12,980 (two scans; the shift is paid by them).
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
       charge was moved after its write, for each read row the row that went red when the
       slice's charge was removed, and the case of 11d that went red when `longer_than` was
       called before the slice's charge. It also says what 5c cannot show (the order of a scan
       or a validation, and the order of a read's charge and its copy), which review holds.
       The report states each case's time and ratio against the plain walk; a ratio over 20 is
       a defect of the price, fixed by raising the charge in the same change. No case runs over
       5 s; a case that does is named with its time and its shape is made smaller by a round
       number, with the floor from the smaller shape.
12. `slot task check` exits 0.
13. Public surfaces: the story changes, in its last commit and after merging `main`, every
    public surface its change makes false, each number taken from the `slot task check` log of
    its tip.
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
    step per 16 bytes in `Copies::copy` and in the reading unit's own charges, charged after
    the one copy: no story's text holds the move before the copy yet) is merged first. The
    first command of the work is `git grep -n -F '/ 64' -- crates/script/src/bindings.rs
    crates/webidl/src/prelude.rs crates/script/src/dom/reads.rs`; it prints nothing, else the
    floors of 5b, 5c, 6 and 11 are not the story's and the work waits for that commit. It
    prints 18 lines at `83e6b8a`: 16 in `reads.rs` (`:135`, `:150`, `:195`, `:313`, `:334`,
    `:462`, `:666`, `:705`, `:805`, `:807`, `:808`, `:813`, `:1084`, `:1097`, `:1117`,
    `:1249`), `bindings.rs:628` and `prelude.rs:558`; at `a14e3b0` it prints 2 (`bindings.rs:561`
    and `prelude.rs:491`). At the same start the implementor reads `longer_than`
    (`bindings.rs:145-147`), `reads::copied` (`reads.rs:142-153`) and `string_value`
    (`bindings.rs:1263-1270`) and reports whether each charges a value's bytes before it counts
    UTF-16 units: at `83e6b8a` none does, which is the reading unit's defect, passed to its
    fix round; this story's own reads charge first (decision 7, 11d) whatever that round has
    done. The test step ends within 120 s and no test runs over 60 s; a test over 5 s is named
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
    show, the steps of 5d, the number written, the steps and `heap_bytes()` of 6b, the steps and
    N of 6d, the cases of 11 with their steps, times and ratios and the cases that went red for
    each removed charge (11h), the times of `webidl-slow` before and after, and the output of
    13a.

## Out of scope

`Element.attributes`, `Attr` and `NamedNodeMap`, which are not generated, and the five attribute
node members and `Document.createAttribute` and `createAttributeNS`, which are rows of `throws`
(an attribute has no identity in the tree, Context): they stay `throws`, excluded by the epic
(`epic:128`, `:683` and `:741 @ a14e3b0`; no story); `classList` and the token lists
(`selection.txt:88`, `reason: token lists`), which the epic excludes in the same lines (no
story; the draft `story:scripts-style-and-layout` names "`classList` effects" at its `:26` and
does not give the member a behaviour); the attributes of a `ProcessingInstruction` (rows
205-211, 7 rows of `throws`, `story/webidl-bindings.md:110`; the epic's `:966` says the
interface has 8 members, read at plan commit `d5a7c45`); an open namespace in `crates/html`,
so that `setAttributeNS` can store
an attribute in a namespace outside the six (decision 9), which is `story:html-open-namespace`
(a draft); a refund of a node's charge (decision 8): a node is never taken out of the arena and
no story is planned; the comment at `runner.rs:715-718 @ 83e6b8a` on `Limit::InputBytes` and
`Limit::Steps` (`story:scripts-dom-mutation`), because that story gives the two limits no word,
as no write makes either; creating an element (`createElement` is the slice `mutation`'s); the
parse that runs a page's scripts (`story:scripts-parse-run`); a page in a session and the
example of a link built by script (`story:scripts-document-port`); the argument price of
decision 20 itself and the 16-byte rate of `Copies::copy` (the reading unit's price commit);
the move of the glue's argument charge before its copy, which no story's text holds
(`story:scripts-utf16-length` swaps the count of units and holds nothing of it), so a write of
this slice charges the glue's copy of an argument after the copy until a story moves it; the
uncharged count of `longer_than` in `reads::copied` and `string_value` (the reading unit's fix
round, Context); the pin of a released tag of the engine, `story:swell-0-9-0`.
## Placement

Read at plan commit `d5a7c45`, the tip of the plan branch when this revision was written (the
mutation story's text is the same at `7226f2e`). After `story:scripts-dom-reading` (its structure
is built at `83e6b8a`; its price commit comes first, acceptance 14) and `story:html-script-hook`
(on `main`); the edges stay. The tests reach an element through the reading story's members and
its wrapper, and the cases call the write surface of the hook story and the reads this story
adds. Beside `story:scripts-dom-selectors`; `story:scripts-dom-mutation` has a `depends_on` edge
to this story (since plan commit `439fd32`), so it is built on a base that holds the functions
of decision 8. The slice's code is its own three files, `dom/attributes.rs`,
`dom/attribute_reads.rs` and `dom/attribute_writes.rs`, and the test files are its own.
`crates/script/src/bindings.rs` is generated and is never merged by hand: whichever story
merges second takes `main`, regenerates it with the generator (`slot task webidl-check` and
`slot task webidl-slow` hold it) and checks the digest lines (acceptance 9).

What the mutation story calls. Its text at `d5a7c45` (decision 4, `:171-179`, and the order of
the character-data writers, `:349-354`) calls `charge_stored` and `undo_stored` "as they are",
with `live` and `peak`. Decision 8 changes the names it calls: `charge_stored(added, removed)`
keeps its name and its two arguments and answers `DomResult<()>` where the earlier text had it
answer the `live` before the call; `undo_stored` is gone, and a write calls
`settle_stored(added, removed)` after the tree has written and `settle_stored(0, 0)` after a
write the tree refused, before it returns the end; `live` and `peak` are `stored_live()` and
`stored_charged()` (decision 8 lists the four accessors). A write that removes bytes
(`deleteData`, `replaceData`, the setters of `data` and `textContent`) refunds through that
settle, so the mutation story's sentence that a removal refunds nothing is true of nodes only.
Its text does not hold these names yet; its revision takes them, and until then the names in
its text are the earlier ones. Whichever of the two stories is built first uses the signatures
of decision 8 as they are.

The files shared, one by one:

- `crates/script/src/bindings.rs` (generated) and `crates/webidl/src/prelude.rs`: this story
  adds `stored`, `live`, `charged`, `charge_stored`, `settle_stored`, the four accessors and the
  `strings_value` change; `story:scripts-parse-run` adds `DomState::swap_tree` and
  `story:scripts-dom-mutation` adds `charge_nodes`, the item charge and the refill charge (and
  no second `charge_stored`); `story:scripts-utf16-length` changes `string_length`. The edits to
  the struct and the `impl DomState` region of `prelude.rs` are in one area and are merged by
  hand; `bindings.rs` is regenerated.
- `crates/webidl/selection.txt`: this story edits the note of row 96 (`setAttributeNS`); the
  selectors story edits nine rows (110-112, 126-127, 175-176 and 231-232 at `83e6b8a`), the
  mutation story the notes of rows 63, 151 and 157 and three `deviation:` clauses; none are
  adjacent. The header digest lines of `bindings.rs` conflict as text; the second to merge runs
  the generator again.
- `crates/html/src/tree.rs`: this story adds three reads (`attribute_position`,
  `attribute_position_ns`, `attribute_names`), three owners (`Namespace::from_url`,
  `Tree::namespace_error`, `Attribute::qualified_len`) and `Attribute::stored_bytes`, changes
  the scans of five writes to call the position reads and `put_attribute` to reserve exactly;
  the reading story adds the write counter and merges first; the mutation story makes
  `parent_or_host` `pub`, writes doc comments on the passes, makes a recursive function
  iterative and changes the clone (its text names "six reads" of this story, which is the
  earlier revision's count); the drafts `story:scripts-dom-sibling-positions` and
  `story:html-parser-hook-cost` name the file too. The edits are in different functions; the
  later merger takes `main` first.
- `crates/script/src/dom/reads.rs`: this story changes it by visibility only, three tokens:
  `fn charge` (`:120 @ 83e6b8a`), `const ATTRIBUTES_A_STEP` (`:252`) and `fn attribute_count`
  (`:255`) become `pub(super)`. `reads::copied` (`:142-153`) is not touched, because the
  function that holds the position, charges and copies a value (decision 7) is copied on
  purpose rather than shared with it. The selectors story at `d5a7c45` makes seven tokens
  `pub(super)` (`charge`, `ATTRIBUTES_A_STEP`, `attribute_count`, `Reads`, `scan_for`,
  `positions`, `value_at`) and widens `Reads` to a lifetime with `&'a str`; the mutation story
  makes `charge`, `find`, `Reads` and `html_element` `pub(super)`. The three tokens of this
  story are the same edit on the same lines as the selectors story's: whichever merges second
  keeps one of them and adds none twice.
- `crates/script/src/dom/mod.rs`: this story adds `mod attribute_reads;` and `mod
  attribute_writes;` (after `mod attributes;`, lines 6-8 @ 83e6b8a); the selectors story adds
  `mod selector_reads;` and a `pub use`; the mutation story two `mod` lines after `mod
  mutation;`. They are in different places of the lists; where git reports adjacent hunks the
  later merger keeps every line.
- `crates/script/tests/main.rs`: this story adds `mod dom_attributes_cost;` after `mod
  dom_attributes;` (line 17 @ 83e6b8a); the mutation story adds `mod dom_mutation_cost;` (its
  Change, `:452`), after `mod dom_mutation;` on line 18, one line from this story's: adjacent
  hunks, resolved by keeping both; the selectors story adds none (`dom_selectors` is declared
  at `:21`).
- `crates/script/tests/dom_reading.rs`: `other_slices_still_throw` (`:2399-2432 @ 83e6b8a`)
  holds three adjacent lines, `:2424` `document.appendChild(el)` (the mutation story's, its
  decision 9), `:2425` `el.getAttribute("id")` (this story's) and `:2426` `el.querySelector("p")`
  (the selectors story's, its decision 13, which says the same of this story). Each of the
  three stories changes its own line; the later merger keeps all three changes.
- `crates/script/tests/bindings.rs`: this story adds a `Writes` variant and the test of
  acceptance 6f; the mutation story changes three tests' members and adds no variant; the
  swell story adds cases.
- `crates/script/src/runner.rs`: this story does not edit it. The mutation story edits the
  comment at `:714-718` (`:715-718 @ 83e6b8a`), which says no member of `reading`,
  `attributes` and `mutation` reaches `Limit::InputBytes` or `Limit::Steps`.
- `docs/design/scripts.md`: this story edits the paragraphs at `:322-332` and `:902-904 @
  83e6b8a`; the mutation story's Change edits the same two places (the removal and clone
  rules), so those hunks are prose written twice and are merged by hand, the later merger
  keeping both sentences (the edge puts the mutation story second). The selectors story adds
  one row to the table of Q4 after `:316`, two lines from the mutation story's `:318`. The swell
  story (decision 13, `story/swell-0-9-0.md:565-592`) edits Q4 at `:255-265`, `:302`, `:303`,
  `:307`, `:320` and `:338-341`, and Q11 at `:902-909`, `:917`, `:925`, `:963` and
  `:1079`: `:322-332` is clear of `:320` and `:338-341`, but its Q11 range `:902-909` contains
  this story's `:902-904`, so that hunk is resolved by hand by whoever merges later, who re-reads
  the file after the merge.
- `AGENTS.md` (`:365-369`, one entry of five lines), `README.md`, `docs/badges/make.sh`,
  `docs/badges/tests.svg` and the other public surfaces: every story beside this one rewrites
  them (the mutation story's Change lists `AGENTS.md`, and `story:swell-0-9-0`,
  `story:scripts-parse-run` and the selectors story list the surfaces); whichever merges later
  merges `main` first and takes its numbers from its own log. `docs/badges/tests.svg` is made
  by `docs/badges/make.sh`: the story edits `make.sh:38` and runs the script, and on a
  conflict the merger takes `main`'s `tests.svg`, sets the number in `make.sh` and runs it
  again.
- What a merger resolves by hand: `prelude.rs` (the `DomState` region), `AGENTS.md:365-369`,
  `README.md:11` and `:285`, `docs/badges/make.sh:38`, `website/docs/status.md:31`,
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
  `reading.rs`/`reads.rs`, not the attribute writes this story reads, so a kept position never
  depends on an attribute; `story:html-open-namespace` (a draft) lifts the deviation of
  decision 9, after this story; `story:scripts-style-and-layout` names `classList` effects as
  style and does not touch this slice.
- The epic: the bullet of this story is in the store since plan commit `60a4317`; its cites and
  its sentence on the refund are corrected by a plan edit made in the store, outside this
  story, and this story edits no line of the epic.
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
