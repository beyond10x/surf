---
format: aep.planning-md/3
id: story:scripts-dom-attributes
kind: story
status: proposed
title: The attribute members (14 rows), held to the attribute, stored-byte and memory limits and priced by what they scan
summary: Element's id, className and slot and its 11 attribute operations (17 trait methods) as the DOM Standard says, in three files (the members, every read of the tree, every write of the tree) with a source test that holds the members free of tree calls and charges, writes through the five attribute functions of the tree's write surface after the standard's name checks and reads through six public reads the tree gains, every pass charged before it runs at one step per 16 bytes and one per 4 attributes scanned, getAttributeNames charged per element, a write over the attribute or stored-byte limit ending the script through the generated limit result, the bytes script writes hold in the tree charged to the memory budget at their peak through a bytes-only charge added to the generator's prelude, a cost module with a floor of steps on the worst shape of each member, a namespace outside the six of the tree named as a deviation, the TrustedType deviation tested, the attribute node members and the two SameObject attribute rows left to throw, the 14 case names asserted equal to the slice's rows.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:html-script-hook
- depends_on: story:scripts-dom-reading
scope:
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
revision: 37
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:13Z", actor: "human:operator", revision: 13}
---
## Context

The epic's tenth candidate (`.engineering/planning/epic/scripts-in-a-page.md:473-474 @ a14e3b0`)
and the slice "Attributes, 14" (lines 725-727). The 14 rows of the slice `attributes` in
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
  decision 20 rules one step per 16 bytes, before the copy, and is not built), calls the slice
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
  of 16,384 one-byte names costs the glue 0 steps. The reading story left that open for this
  slice (decision 7).
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
  goes back. This story adds the reads to `tree.rs` (decision 3) so that a read and a write of
  the same name use one matching rule. The reading unit's own attribute reads (`reads.rs`
  `scan_for`, `positions`, `value_at`, `Reads`, `reads.rs:245-322`) match a name in no namespace
  for `class`, `id` and the like and are private to it; they do not match a qualified name.
- What a write counts in the tree. `put_attribute` (`tree.rs:1499-1526`) adds a new attribute
  with its record and strings, `attribute_bytes` (`tree.rs:215-222`): `ATTRIBUTE_BYTES` (80,
  `crates/html/src/limits.rs:90-92 @ a14e3b0`) plus the bytes of its prefix, local name and
  value; it replaces an existing attribute's value only (`stored_bytes - old + new`,
  `tree.rs:1509-1514`), keeping the name and prefix it had. A replacement by a shorter value
  lowers `Tree::stored_bytes` (`tree.rs:594`); `drop_attribute` (`tree.rs:1528-1534`) frees an
  attribute's bytes and shifts the later attributes down one place.
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
  agent is dropped, never refunded. Over the budget nothing is charged and the run ends
  `Terminated(MemoryBudget)`, which is `limit: memory`. `docs/design/scripts.md:322-332` and
  `:902-904` say so. No function charges bytes alone, so the bytes a script writes into a node
  that exists (an attribute's strings, and the character data of `story:scripts-dom-mutation`)
  are charged to nothing today. The charge is twice the stored bytes and the tree's limit is the
  stored bytes, 67,108,864 against a budget of 67,108,864, so a script that only adds attribute
  bytes to a small page spends the budget when it has added 32 MiB (33,554,432 bytes), less
  half the engine's own heap, before the tree's stored-byte limit is reached.
- How a slice charges steps, as built. `reads::charge(cx, steps: usize) -> DomResult<()>` is
  private to `reads.rs` (`reads.rs:118-131`): it calls `cx.engine().charge_steps(steps)`, does
  nothing for 0, and when the engine refuses answers `DomEnd::Throw(InvalidStateError "the steps
  of the script are spent")`, a throw the engine does not deliver: the termination is sticky and
  the run ends. `DomCx::charge_node` fails the same way for the memory budget. A read that
  returns a String is held to `string_length` first (`longer_than`, `bindings.rs:145`, giving
  `DomEnd::StringLength`), then charged, then copied (`reads::copied`, `reads.rs:139-153`). A scan
  by name charges its attributes at one step per 4 (`ATTRIBUTES_A_STEP`, `reads.rs:250-252`).
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
`Tree::writes() -> u64` (`tree.rs:331`, newtype `Writes` `:287`), raised by the private `wrote()`
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
(`selection.txt:26` lists the definitions; the epic, line 683), so `Element.attributes` is a
row of `throws` and nothing in the bindings can hold an `Attr`.

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
     `undo_stored`; decisions 7 and 8) and the map of `WriteError` (decision 5). It reads the
     tree only through `attribute_reads`. It holds no `tree()`.
   - `crates/script/src/dom/attributes.rs` holds the 17 methods and nothing else: each method
     calls one function below and gives the answer back. It holds none of `tree()`,
     `tree_mut()`, `charge(`, `charge_steps`, `charge_stored` and `undo_stored`.
   - `dom/mod.rs` gains `mod attribute_reads;` and `mod attribute_writes;`. `reads.rs` is
     edited in three tokens: `charge`, `ATTRIBUTES_A_STEP` and `attribute_count` become
     `pub(super)`, so that the slice has one function that says what a charge does when the
     script is over its steps and one rate for a pass over attributes, and no copy of either.
     The byte rate is `crate::bindings::STRING_BYTES_A_STEP`; the slice has no constant of
     its own.
   Member to function (`attribute_reads` for a read, `attribute_writes` for a write; each takes
   the `DomCx` and the element's `NodeId` first):
   - the getters of `id`, `className` and `slot`: `reflected(cx, element, "id" | "class" |
     "slot") -> DomResult<String>`, the value of the attribute with that local name and no
     namespace, the empty string for none (`dom.html:10212`);
   - `hasAttributes`: `any(cx, element) -> bool`, `attribute_count` above 0; it charges nothing
     beyond the glue's one step (a count of a length);
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
   A write also needs what an attribute holds before it is written (its value's bytes, its
   stored bytes): `attribute_reads::find(cx, element, wanted) -> DomResult<Option<Found>>`
   charges the scan of decision 7, calls the tree's read, and answers `Found { value_bytes,
   stored_bytes }`, two numbers, no reference held across a charge; `wanted` is a qualified name
   or a namespace and local name. A namespace argument is the empty string (null), or one of the
   six URLs of `Namespace::url` (`tree.rs:94-106`), mapped by `attribute_reads::namespace_of`,
   which compares the argument's length with the six URLs' before any byte, so its work is
   bounded by a constant; a URL that is none of the six matches no attribute, so `getAttributeNS`,
   `hasAttributeNS` and `removeAttributeNS` answer `null`, `false` and nothing for it, as the
   standard does, without a scan.
3. `crates/html/src/tree.rs` gains six public reads, so that the slice has one matching rule
   with the writes and no private copy of it, and one public method that is the tree's count of
   an attribute's bytes. Each read is `&self` and takes the element's `NodeId`:
   - `get_attribute(&self, element: NodeId, name: &str) -> Option<&Attribute>`: lowers `name`
     for an element in the HTML namespace with `attribute_name` and answers the first
     attribute for which `qualified_name_is` holds (the matching of `set_attribute`,
     `remove_attribute` and `toggle_attribute`, `tree.rs:1541`, `:1599`, `:1635`;
     `dom.html:10175`);
   - `has_attribute(&self, element: NodeId, name: &str) -> bool`: the same match, `true` when
     there is one;
   - `get_attribute_ns(&self, element: NodeId, namespace: Option<Namespace>, local_name: &str)
     -> Option<&Attribute>` and `has_attribute_ns(...) -> bool`: the first attribute whose
     namespace and local name equal the arguments (the match of `set_attribute_ns` and
     `remove_attribute_ns`, `tree.rs:1579-1581`, `:1614-1616`; `dom.html:10192`);
   - `attribute_names(&self, element: NodeId) -> Vec<String>`: the qualified names in order,
     duplicates kept, each the prefix, a colon and the local name, or the local name
     (`dom.html:10581`);
   - `attribute_count(&self, element: NodeId) -> usize`;
   - `Attribute::stored_bytes(&self) -> usize`: the tree's count of the attribute,
     `ATTRIBUTE_BYTES` and the bytes of its prefix, local name and value, which is what the
     private `attribute_bytes` (`tree.rs:215-222`) answers, made public and called by it, so
     that the slice's charge uses the tree's own count (decision 8).
   A node that is not an element answers `None`, `false`, an empty list and 0: the bindings
   brand-check `Element` first, and a read has no exception to answer, where a write answers
   `WriteError::InvalidNodeType` (`tree.rs:1471-1479`). What a read costs follows the write
   surface's form: the tree counts no step, and the documentation of each read says what it
   costs so that the caller charges for it: a read visits each attribute at most once and
   compares each attribute's name with the wanted one, at most the bytes of the shorter of the
   two; `attribute_names` also builds one String per attribute, in all the bytes of the
   names. The tree takes no `Cell` of steps, as the parser's private scans do (`tree.rs:246`): a
   public read reports nothing and bounds what it scanned. Only `attribute_reads` calls these
   reads, after its charge (decision 7). The tests of the reads are in the module of the write
   surface's tests (Change).
4. Names are validated in `attribute_writes`, before the scan and the write and in the standard's
   order, and a throw leaves the tree as it was and charges nothing for memory. `setAttribute`
   and `toggleAttribute` throw `InvalidCharacterError` for a name that is not a valid attribute
   local name. `setAttributeNS` does the whole of "validate and extract" (`dom.html:835`): the
   empty namespace is null; a prefix that is not a valid namespace prefix, then a local name
   that is not a valid attribute local name, throw `InvalidCharacterError`; then the four
   `NamespaceError` conditions, which the slice tests itself, as `Tree::set_attribute_ns` does
   (`tree.rs:1565-1578`), so that no write that is then refused for a `NamespaceError` has been
   charged for memory. The tree's answer `WriteError::Namespace` stays mapped (decision 5) and
   cannot occur after the slice's own test. The setters of `id`, `className` and `slot` and
   every read validate nothing. Each pass over a name is charged before it runs (decision 7).
5. A `WriteError` becomes a `DomEnd`: `Limit(limit)` becomes `DomEnd::Limit(limit)`, the one
   answer that ends the script; `Namespace` becomes the `DOMException` `NamespaceError`
   (`bindings.rs:2276`); the others (`HierarchyRequest`, `NotFound`, `IndexSize`,
   `InvalidNodeType`, `NotSupported`, `tree.rs:57-71`) are named by the exception each doc
   comment gives and are not answered by these writes for an element: `InvalidNodeType`
   comes only from a `this` that is no element, which the brand check of `Element` excludes.
6. The ends of a write over a limit are the words of Q4: `limit: attributes`,
   `limit: stored-bytes` and, from the charge of decision 8, `limit: memory`, nothing written.
   This story does not edit `crates/script/src/runner.rs`: its writes reach neither
   `Limit::InputBytes` nor `Limit::Steps` (Context), and the placeholder arm for those two
   (`runner.rs:719`) is settled by `story:scripts-dom-mutation`, whose Context names it.
7. What a call costs: one price for a byte, one for an attribute passed, each charged before the
   work it pays for, in addition to the glue's one step and to the glue's charge for each
   argument it copies and for each String it answers. The rates are `S`, one step per 16 bytes
   (`STRING_BYTES_A_STEP`), and `A`, one step per 4 attributes passed in a scan by name
   (`ATTRIBUTES_A_STEP`). A charge is one call of `reads::charge`, before the pass. The extent
   of every pass is known before the pass starts (the attribute count, a string's length), so
   no member charges step by step, and no member splits a value into tokens (`className` is the
   attribute's whole value; `classList` is `throws`).
   - A scan by qualified name (`get`, `has`, and the scan inside a write) over `c` attributes
     of an element, for a name of `n` bytes, where `k` is the number of attributes whose
     qualified name has the length of the lowered name: `c / A + (1 + k) * n / S`. The `1` is the
     lowering pass over the name; each of the `k` compares can read all `n` bytes; an attribute of
     another length is rejected on its length. A scan by namespace and local name (`get_ns`,
     `has_ns`, the scans of `set_ns` and `remove_ns`, `reflected`) over a local name of `l` bytes,
     `k` the attributes whose local name has that length: `c / A + k * l / S`. `k` comes from one
     pass over the lengths, paid by the `c / A` term.
   - A value returned (`get`, `get_ns`, `reflected`): held to the agent's `string_length` first
     (`longer_than`, then `DomEnd::StringLength`, as `reads::copied` does), then `v / S` for the
     `v` bytes of the value, then the copy; the glue then charges its own copy of the String it
     answers, `v / S` again: two passes over the bytes, two charges.
   - `getAttributeNames`: `c` steps, one per attribute (the pass over the list that adds up the
     names' lengths), and the names' bytes over `S`, charged before the list is built; then the
     glue charges the bytes of each name. The generator's `strings_value` charges
     `STRING_ELEMENT_STEPS`, 8 as a collection node does (`COLLECTION_NODE_STEPS`,
     `bindings.rs:1251`), for each element before its loop; this story changes `strings_value`
     in `crates/webidl/src/prelude.rs` (`:1441-1454`) and regenerates. The weight 8 is a
     choice: the cost module prints the time a step of `getAttributeNames` against the plain
     walk (acceptance 11), and a ratio over 20 raises the weight in the same change. So a list
     of `c` names costs at least `9 * c` steps.
   - A pass over an argument (`validate` of a name, the prefix and local name of `setAttributeNS`,
     the lowering): the bytes of the pass over `S`, charged before it runs.
   - A write. The order of a member is decision 8's. Its steps, each charged before the work
     they pay for: the validation passes; the scan of `find`; then, before the call of the tree,
     the tree's own scan again (the same formula, since `set_attribute` and the others scan the
     list a second time, `tree.rs:1541` and the like) and `(n + v) / S` for the bytes the tree
     copies (`n` the bytes of the name it stores, `v` of the value, whether it adds an
     attribute or replaces a value; `toggle` that adds, `v` is 0; `remove` copies nothing; the
     shift of `drop_attribute` is at most `c` moves, paid by the scan's `c / A`).
   So a write that adds costs at least `2 * (c / A) + (n + v) / S` steps of its own, beyond the
   glue's one step and its charge for the two argument copies, `(n + v) / S` again; the scan
   term is counted twice (`find` and the tree's own).
8. The bytes a script writes into the tree are charged to the memory budget against the peak of
   what script writes hold in the tree. The state of the bindings, `DomState` (the struct that
   holds `totals` and `charges`, `crates/webidl/src/prelude.rs:104-116`, written into
   `bindings.rs`), keeps two counts of stored bytes, both 0 at `DomState::new`
   (`prelude.rs:120-131`):
   - `live`: the stored bytes that script writes added, less those that script writes removed or
     replaced away, by the tree's own count of an attribute (`attribute_bytes`,
     `tree.rs:215-222`); never below 0, so removing what the parser stored brings it to 0 and
     no lower;
   - `peak`: the highest `live` that was charged.
   The generator's prelude (`crates/webidl/src/prelude.rs`, written into `bindings.rs`) gains,
   beside `charge_node` (`prelude.rs:214-235`):
   - `DomCx::charge_stored(&mut self, added: usize, removed: usize) -> DomResult<u64>`: the new
     `live` is the old one plus `added`, less `removed`, not below 0. When it is above `peak`,
     twice the difference is charged with `charge_host_bytes` before anything changes, the
     `HostBytes` is kept until the agent is dropped (one entry of `DomState::charges` for each
     raise of `peak`, so at most one for each step of the script), the charge is added to
     `Totals::bytes` (and no node to `Totals::nodes`), and `peak` becomes the new `live`. When it
     is not above `peak` nothing is charged. A call that lowers `live` charges nothing and
     refunds nothing. It answers the `live` it had before the call;
   - over the budget nothing is charged, `live`, `peak` and the tree stay as they were, and it
     answers as `charge_node` does (a throw the engine does not deliver; the run ends
     `Terminated(MemoryBudget)`, `limit: memory`);
   - `DomCx::undo_stored(&mut self, live_before: u64)`: puts `live` back to the value
     `charge_stored` answered and leaves `peak` and the charge as they are; and the accessors
     `DomState::stored_live()` and `DomState::stored_peak()`, which the tests read beside
     `DomState::totals()` (`prelude.rs:157`);
   - the doc comments of `tree_mut` (`prelude.rs:177-181`) and of `Totals::bytes`
     (`prelude.rs:92-99`) name them; `charge_node` and `charge_stored` share one private body.
   So the charge is twice the peak of what scripts hold, a loop that sets and removes one
   attribute charges its first round only, and a replacement by a longer value charges the
   growth over the peak. The node charge of `charge_node` stays as it is: a node is never taken
   out of the tree's arena (`tree.rs:19-21`), so nothing of it is ever released, where a
   stored byte is freed at a removal (`drop_attribute`, `tree.rs:1528-1534`).
   `story:scripts-dom-mutation` calls the same function, with the same two counts, for its
   character-data writes. The order of a member of this slice, all of it in `attribute_writes`:
   validate (decision 4); `find`, which charges the scan and answers the sizes; compute the bytes
   the write adds and removes by the tree's count; charge the steps of decision 7; call
   `charge_stored`; write; on a write that the tree then refuses (a limit), call `undo_stored`
   with what `charge_stored` answered, so that `live` is as before while `peak` and the charge
   stay (the script ends). The bytes, from what `find` answers (`Found`, `None` for none):
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
     charge is made and none refunded;
   - nothing found and nothing added (`removeAttribute` of an absent name, `toggleAttribute`
     with `force` false): no call.
   A write that the tree refuses for a limit has been charged once, and ends the script. A
   script that removes what the parser stored lowers `live` no further than 0, so the charge
   counts only what scripts wrote: the tree holds at most what the parse stored plus `peak`.
   With the defaults a script on a small page reaches `limit: memory` before `limit:
   stored-bytes`, and `limit: attributes` before `limit: memory` when its attributes are short
   (acceptance 6b, 6c). The tree is the document's one tree, lent to each script by
   `story:scripts-parse-run` (its `DomState::swap_tree`); `live` and `peak` stay in `DomState`
   and that swap leaves them alone.
9. A deviation of `setAttributeNS` that this story names in the note of its row
   (`selection.txt:96`): a namespace that is none of the six of `Namespace` cannot be stored,
   so the call throws `NotSupportedError` after validate and extract has passed and before the
   scan and the charge, with the message `Element.setAttributeNS is not supported` through
   `DomException::not_supported` (`bindings.rs:107`). An open namespace in `crates/html` is a
   story of its own (Out of scope).
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
    except `live` (decision 8). If a later story keeps an answer that depends on an attribute (a
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
    its `totals()`, `stored_live()`, `stored_peak()` and its tree's `stored_bytes()`. A value
    of many characters is built by the script with `"x".repeat(n)`, which the engine has
    (`crates/script/tests/adversary_3.rs:796`).

## Change

- `crates/html/src/tree.rs`: the six reads of decision 3, each with a doc comment that says
  what it matches, what it answers for a node that is not an element and what it costs; the
  module documentation's sentence on what the surface leaves to its caller (`tree.rs:21-23`)
  gains the reads; `Attribute::stored_bytes` (decision 3), which the private `attribute_bytes`
  then calls. No existing behaviour changes. The reading story edits this file too and
  merges first (Placement).
- `crates/html/tests/script_hook.rs @ a14e3b0`: the tests of the reads, in the module
  `script_hook` where the write surface's tests are (`write_set_attribute` is at `:1157`,
  `write_set_attribute_ns` at `:1197`, `write_remove_attribute` at `:1243`,
  `write_remove_attribute_ns` at `:1259`, `write_toggle_attribute` at `:1274`):
  `read_get_attribute`, `read_get_attribute_ns`, `read_has_attribute`, `read_has_attribute_ns`,
  `read_attribute_names`, `read_attribute_count` and `attribute_stored_bytes`; the module is
  declared in `crates/html/tests/main.rs`, so no `mod` line is added.
- `crates/webidl/src/prelude.rs`: the fields `live` and `peak` of `DomState` and their
  initial values in `DomState::new`, `DomCx::charge_stored`, `DomCx::undo_stored`,
  `DomState::stored_live` and `DomState::stored_peak`, the doc comments of decision 8, and the
  charge of `STRING_ELEMENT_STEPS` for each element in `strings_value` (decision 7);
  `crates/script/src/bindings.rs` regenerated with `slot cargo run -p surf-webidl --bin
  bindings`. The header's digests that change are those of `crates/webidl/selection.txt`, of
  `crates/webidl/src/prelude.rs` and the whole-file digest on the last line; the other lines of
  the digest block stay as they are on the merged `main`.
- `crates/webidl/selection.txt`: the note of `Element setAttributeNS` (line 96) gains the
  deviation of decision 9; the doc comment of the method carries the note (`bindings.rs:3200`).
- `crates/script/src/dom/attribute_reads.rs` (new): `reflected`, `any`, `names`, `get`,
  `get_ns`, `has`, `has_ns`, `find`, `namespace_of` and the scan charge (decisions 2, 7).
- `crates/script/src/dom/attribute_writes.rs` (new): `set_reflected`, `set`, `set_ns`, `remove`,
  `remove_ns`, `toggle`, the valid attribute local name and valid namespace prefix tests, the
  validate and extract, the bytes a write adds and removes, the charge and the undo of a write
  the tree refuses (decisions 2, 4, 5, 7, 8, 9).
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
  the `Writes::Charge` arm calls `charge_node` (`tests/bindings.rs:175-178`), calling
  `undo_stored` with the answer when the argument ends in `!`; and the test of acceptance 6f
  beside `node_charge` (`tests/bindings.rs:917-954`).
- `crates/script/tests/dom_reading.rs`: `other_slices_still_throw`
  (`dom_reading.rs:2399-2432`) asserts that `el.getAttribute("id")` throws (line 2425, message
  `Element.getAttribute is not supported`); it changes to `el.getAttributeNode("id")` with the
  message of a row of `throws`, `Element.getAttributeNode is not supported: attribute nodes`
  (`bindings.rs:2042-2047` makes the message `<member> is not supported: <reason>`). One line.
- `docs/design/scripts.md`: the paragraph on the charge of the DOM a script makes
  (`:322-332`) and the line that lists what `memory_bytes` holds (`:902-904`) say that the
  bytes a script writes into a node that exists are charged too: twice the peak of the bytes
  that script writes hold in the tree, before the write, never refunded when the peak falls,
  where a node's charge is never refunded because a node is never taken out of the arena.
  `story:scripts-dom-mutation` edits the same lines (Placement).
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
      tree unchanged and `totals().bytes` unchanged. `setAttribute("a:b", "v")` is not an error
      (no colon test).
   e. `setAttributeNS` in a namespace outside the six throws `NotSupportedError` naming the
      member, after the checks of decision 4 (`xml:a` in an outside namespace throws
      `NamespaceError`, not `NotSupportedError`) and with `totals().bytes` unchanged;
      `getAttributeNS`, `hasAttributeNS` and `removeAttributeNS` in it answer `null`, `false`
      and `undefined`, and raise `runner.steps()` by no more than the glue's charges (no scan).
   f. The table tests run each entry through the helper, so each case makes its own runner and
      asserts the end `Ran` and no recorded limit.
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
      (decision 5): `git grep -n -e 'Limit::Nodes' -e 'Limit::Depth' -e 'Limit::InputBytes'
      -e 'Limit::Steps' -- crates/script/src/dom/attribute_writes.rs` prints nothing, and no case
      asserts one of those four ends (no write of this slice reaches them, Context).
3. The write that the epic's example 2 needs. On a parsed `<a href="/old">`,
   `a.setAttribute("href", "/next")` leaves the tree's attribute `href` as `/next`, read through
   `runner.document()` and `Tree::data`; `a.setAttribute("x", { toString() { return "y" } })`
   sets `y` (the `TrustedType` deviation of rows 95 and 96; `Element.setAttribute` has the note,
   `bindings.rs:3189`); a lone surrogate in a value is stored as U+FFFD (`selection.txt:24`).
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
      `tree().writes()` where it was, and so do the 9 readers. This case is the held
      statement of decision 10: no kept object and no cached answer depends on an attribute.
5. Prices (decision 7). `runner.steps()` is read around each script, one call a script, on the
   session's limits; a floor is the sum of the charges the call makes, and each floor is stated
   with its terms in the case.
   a. `getAttribute` of an attribute at the end of an element of 1,024 attributes named `a0` to
      `a1023` raises `runner.steps()` by at least 1,024 / 4 - 4 / 4 = 255 more than the same
      call on an element of 4 attributes; `hasAttribute`, `getAttributeNS`, `hasAttributeNS`
      and the `id` getter likewise. `getAttributeNames` of the same element raises it by at
      least 1,024
      (the slice's) + 8 * 1,024 (`strings_value`) = 9,216.
   b. `getAttribute` of a value of 64,000 bytes raises `runner.steps()` by at least
      2 * (64,000 - 64) / 16 = 7,992 beyond the same read of a value of 64 bytes (the slice's
      charge and the glue's charge for the String it answers), and `setAttribute` of such a value
      likewise (the glue's copy of the argument and the tree's copy; the floor assumes the
      argument price of decision 20 is merged, acceptance 14).
   c. Before the work. With `steps_per_entry` at 50,000, `setAttribute("a", v)` with `v` of
      1,048,576 bytes ends `limit: steps-per-script`, and the tree, `stored_live()`,
      `stored_peak()` and `totals().bytes` are as before the script: the charges came before the
      write and `charge_stored` was not reached. A `getAttribute` of a value of 1,048,576
      bytes under the same budget ends the same way.
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
   `stored_live()`, `stored_peak()` and the tree's `stored_bytes()` before and after, starts
   with `live` and `peak` at 0, and gives `Document` the session's `memory_bytes` of 67,108,864
   (`SESSION_ENGINE_LIMITS`, `runner.rs:40-48`) and the default `Limits` unless it says
   otherwise.
   a. For each of `setAttribute` of a new name, `setAttribute` to a longer value, `setAttribute`
      to a shorter value, `setAttributeNS` of a new prefixed name, `toggleAttribute` that adds
      and the `id` setter on an element without `id`: `totals().bytes` rises by exactly twice
      the rise of `stored_bytes()` when it rose and not at all when it did not, `stored_peak()`
      is the rise of `stored_bytes()` or 0, and `totals().nodes` stays 0.
      `Attribute::stored_bytes` of the attribute added equals the rise of `stored_bytes()`.
      `removeAttribute`, `removeAttributeNS`, `toggleAttribute` that removes, a replacement by
      an equal value, and every throw of 1d and 1e leave `totals().bytes` and `stored_peak()`
      where they were, and a removal leaves `stored_live()` at 0 when the parser stored what it
      removed.
   b. Memory first. On `<p></p>` (the `html` element of it has no attribute), through
      `document.documentElement`, a loop `for (i = 0; ; i++) e.setAttribute("a" + i, v)` with
      `v` a string of 40,000 ASCII characters built by the script, ends `limit: memory` with
      fewer than 1,024 attributes and at most 837 written. Nothing is removed, so `live` equals
      `peak` after each write and each write charges twice its own bytes: 2 * (80 + len(name) +
      40,000) is 80,164 bytes for `a0` to `a9`, 80,166 for `a10` to `a99`, 80,168 from `a100`;
      837 writes charge 67,100,396 bytes, and the 838th would pass the budget of 67,108,864 even
      with an empty engine heap; 1,024 writes would charge 82,091,860, so the attribute limit is
      not reached; 837 writes store 33,550,198 bytes, just under half the stored-byte limit of
      67,108,864, so that limit is not reached either. The engine's own heap makes the end come
      earlier, never later. Steps: each write costs the two copies of the value, 2 * 40,000 / 16 =
      5,000, and the scans, which grow with the attributes held: about 4.4 million steps for 837
      writes, under the 10,000,000 of an entry. The case asserts the end, `attributes written <=
      837`, that `totals().bytes` equals twice `stored_peak()` and twice the rise of
      `stored_bytes()`, and that `stored_live()` equals `stored_peak()` (the write the budget
      refused changed no count and wrote nothing), and records the number written, the steps and
      `heap_bytes()` in a comment.
   c. Attributes first. The same loop with `v` of 1,000 bytes ends `limit: attributes` at the
      1,025th write: the first 1,024 writes store 1,024 * 1,080 + 4,010 = 1,109,930 bytes (4,010
      being the bytes of `a0` to `a1023`) and charge 2,219,860; the 1,025th adds 1,085 bytes
      (`a1024` is 5 bytes), so `live` and `peak` become 1,111,015 and 2,170 is charged before
      the tree refuses the write; 2,219,860 + 2,170 is 2,222,030, far under the budget. The case
      asserts the end, that the element holds 1,024 attributes, `totals().bytes` of 2,222,030
      (the refused write charged once), `stored_peak()` of 1,111,015 and `stored_live()` of
      1,109,930 (`undo_stored` put `live` back, `peak` stays). The steps are about 0.4 million.
   d. No charge for a repeat. A loop of N = 50 rounds of
      `e.setAttribute("a", v); e.removeAttribute("a")` with `v` of 1,048,576 bytes ends `Ran`,
      with no recorded limit. The first round's set adds 80 + 1 + 1,048,576 = 1,048,657 bytes,
      `live` and `peak` become 1,048,657 and 2 * 1,048,657 = 2,097,314 bytes are charged; its
      removal lowers `live` by the same 1,048,657 to 0; every later set brings `live` back to
      1,048,657, which is not above `peak`, and charges nothing. So `totals().bytes` is
      2,097,314 after the N rounds, and `stored_live()` is 0 and `stored_peak()` 1,048,657.
      Then one more set of a value of 1,048,577 bytes (1 byte longer) takes `live` to
      1,048,658, 1 above `peak`, and charges 2 bytes: `totals().bytes` is 2,097,316. The count
      by added bytes of the earlier text would end `limit: memory` at the 32nd round (31 *
      2,097,314 = 65,016,734; 32 * 2,097,314 = 67,114,048 is over 67,108,864); the case holds
      the opposite, which needs N above 31. N, derived: a set costs two copies of its value
      (the glue's and the tree's) at 16 bytes a step, 2 * 1,048,576 / 16 = 131,072 steps; an
      entry ends at 10,000,000 steps (`steps_per_entry`, `runner.rs:41`); N rounds and the last
      set are N + 1 sets, and (N + 1) * 131,072 <= 10,000,000 gives N + 1 <= 76 and so N <= 75
      before the validation, the scans and the loop's own steps. N is 50: 51 * 131,072 =
      6,684,672 steps of copies, leaving 3,315,328 steps for the rest. The case records
      `runner.steps()` in a comment and, if the first measure shows less room than that,
      lowers N to a round number above 31 that fits, and says so in the report.
   e. Growth by replacement. On the same page, `e.setAttribute("a", v)` with `v` of 1,024 bytes,
      then of 2,048, 4,096 and on by doubling to 1,048,576 bytes (11 sets), leaves `live` and
      `peak` at 81 + 1,048,576 = 1,048,657 and `totals().bytes` at 2 * 1,048,657 = 2,097,314:
      twice the last length plus the fixed terms (80 for the record, 1 for the name), not the
      sum. The sum of the 11 sets' own bytes would be 2 * (11 * 81 + 2,096,128) = 4,194,038.
      Then a replacement by a value of 1,024 bytes lowers `live` to 1,105 and charges nothing,
      and doubling again to 1,048,576 bytes charges nothing: `totals().bytes` stays 2,097,314
      and `stored_peak()` stays 1,048,657. The steps are about 524,032 (the 22 sets' values,
      4,192,256 bytes, twice over, at 16 bytes a step) plus their own cost.
   f. The function itself, in `crates/script/tests/bindings.rs`: through the test slice,
      from `live` and `peak` of 0 and `totals().bytes` of 0, each call giving (`live`,
      `peak`, `totals().bytes`) after it, and `totals().nodes` at 0 throughout:
      add 1,000 gives (1,000, 1,000, 2,000); add 500 over the peak gives (1,500, 1,500, 3,000);
      remove 700 gives (800, 1,500, 3,000), nothing charged or refunded; add 600 under the peak
      gives (1,400, 1,500, 3,000), nothing charged; add 300 over the peak gives (1,700, 1,700,
      3,400), 2 * 200 = 400 charged; remove 5,000, more than `live`, gives (0, 1,700, 3,400),
      never below 0; add 900 and remove 400 in one call gives (500, 1,700, 3,400); a call with
      both 0 changes nothing. `undo_stored` after an add of 100 from (0, 0, 0) gives (0, 100,
      200): `live` back, `peak` and the charge kept. A refused charge: with `memory_bytes` of
      1 MiB, an add of 10,000,000 ends `Reason::Memory` and leaves (`live`, `peak`,
      `totals().bytes`) as they were. The heap grows by at least the charge beyond an uncharged
      control, as `node_charge` measures it (`tests/bindings.rs:917-954`).
7. The reads of the tree (decision 3), in the module `script_hook` of `crates/html/tests`.
   a. Each of the six reads and `Attribute::stored_bytes` has a test (the last equals the rise
      of `Tree::stored_bytes` for each attribute `set_attribute`, `set_attribute_ns` and
      `toggle_attribute` add); `get_attribute` and `has_attribute` lower a name for an HTML
      element and not for an SVG one; the NS reads match by namespace and local name and not
      by prefix; `attribute_names` keeps order and duplicates and gives `xlink:href` for a
      prefixed attribute.
   b. One matching rule: for a list of names (capitals, a prefixed name, a duplicate, an absent
      one) on an HTML and an SVG element, `has_attribute` is true exactly when
      `remove_attribute` of the same name on a clone of the tree lowers the attribute count,
      and `has_attribute_ns` likewise against `remove_attribute_ns`.
   c. A node that is not an element (a text node, the document) answers `None`, `false`, an
      empty list and 0.
   d. Each read's doc comment names its cost: `git grep -n -i 'cost' -- crates/html/src/tree.rs`
      shows a line in the doc comment of each of the six.
8. Other slices and `[SameObject]` (decision 10). `el.appendChild`, `el.querySelector`,
   `el.attributes`, `el.classList`, `el.getAttributeNode("a")` and `document.createAttribute("a")`
   throw `NotSupportedError` naming the member (`<member> is not supported: <reason>` for a row
   of `throws`, `bindings.rs:2042-2047`; `<member> is not supported` for the others,
   `bindings.rs:149-155`), each in its own `try`; `el.attributes === el.attributes` and
   `el.classList === el.classList` each throw, never answer. `git grep -n -F 'el.getAttribute('
   -- crates/script/tests/dom_reading.rs` prints no line that expects a throw. The `SameObject`
   enum keeps its four variants: `git diff -U0 -- crates/script/src/bindings.rs | grep
   'SameObject'` prints nothing.
9. The generated files. `awk '$3=="attributes"' crates/webidl/selection.txt | wc -l` prints 14;
   the row `Element setAttributeNS` holds the deviation of decision 9;
   `slot task webidl-check` exits 0 and `slot task webidl-slow` regenerates `bindings.rs` byte
   for byte; `git diff -U0 -- crates/script/src/bindings.rs | grep '^[-+]//! [0-9a-f]\{64\}'`
   prints the digest lines of `crates/webidl/selection.txt` and of
   `crates/webidl/src/prelude.rs` and no other source's, and `charge_stored` is in the file.
10. Files, and the source test. `git diff --stat` names only the files of the Change and of the
    public surfaces; it names nothing else under `crates/html/` than `src/tree.rs` and
    `tests/script_hook.rs`, and not `crates/script/src/runner.rs`,
    `crates/script/tests/adversary_3.rs`, `adversary_4.rs`, `adversary_5.rs` or
    `crates/webidl/src/generate.rs`; in `reads.rs` it shows three changed lines. The module
    `dom_attributes` is the one declared at `tests/main.rs:17` and `dom_attributes_cost` the one
    added after it; no `[[test]]` entry is added; `slot task test-layout` exits 0.
    The source test `dom_attributes_cost::the_members_touch_the_tree_only_through_the_files`
    reads `attributes.rs`, `attribute_reads.rs` and `attribute_writes.rs` with `include_str!`
    (as `dom_reading_cost::the_members_read_the_tree_only_through_the_reads` does,
    `dom_reading_cost.rs:594`) and asserts: `attributes.rs` holds none of `tree()`,
    `tree_mut()`, `charge(`, `charge_steps`, `charge_stored` and `undo_stored`;
    `attribute_reads.rs` holds none of `tree_mut()` and `charge_stored`; `attribute_writes.rs`
    holds no `tree()`. The test is red when one of those strings is added to the file that must
    not hold it (the implementor shows it once, with the string added, and reports the output).
11. The cost module, `crates/script/tests/dom_attributes_cost.rs`: one case for each member that
    reads or writes more than its own node, on the worst shape the parser admits for it, one run
    a shape, no loop to steady a number. Each case records its steps, its time and the time a
    step against the plain walk of `dom_reading_cost.rs` (`getElementById('none')` over 20,000
    elements without attributes); it asserts a floor of steps, the sum of the charges of
    decision 7 for that shape, and no time. Under raised parse limits (16,384 attributes, room
    for long values) and `string_length` of 1 << 22:
    a. `getAttribute` and `hasAttribute` of an absent name over an element of 4,096 attributes
       whose names are all 1,000 bytes long, the wanted name 1,000 bytes long and differing
       only in its last byte (every compare reads the whole name; the names are distinct, 996
       letters and a 4-digit number, the wanted one 996 letters and `zzzz`): floor 4,096 / 4 +
       4,097 * 1,000 / 16 = 1,024 + 256,062 = 257,086, from the formula of decision 7.
    b. `getAttributeNS`, `hasAttributeNS` and the `id` getter over 16,384 attributes with short
       numbered names (`n0` to `n16383`): floor 16,384 / 4 = 4,096 each.
    c. `getAttributeNames` over 16,384 attributes: floor 16,384 + 8 * 16,384 plus the bytes of
       the names over 16.
    d. `getAttribute` of a value of 4,194,304 bytes (the bound): floor 2 * 4,194,304 / 16 =
       524,288; and a value of 4,194,305 bytes throws the `RangeError` with the steps raised by
       less than 262,144, the charge for the value (the check precedes the charge).
    e. `setAttribute` with a value of 4,194,304 bytes on an element of 16,384 attributes, adding
       a name: floor 2 * 4,194,304 / 16 (the glue's copy and the tree's) + 2 * 16,384 / 4 (two
       scans); the `id` setter (which shares `set_reflected` with `className` and `slot`) with
       the same value; `setAttributeNS` with a qualified name of 1 MiB (`"x:" +
       "y".repeat(1048574)`), the SVG namespace URL and an empty value: floor the passes of the
       validation over the prefix and the local name, the scans, and `(n + v) / 16`.
       `toggleAttribute` and `setAttribute` with a name of 1 MiB likewise (validate, lowering,
       two scans, the tree's copy, the glue's copy).
    f. `removeAttribute` and `removeAttributeNS` of the last of 16,384 attributes: floor
       2 * 16,384 / 4 = 8,192 (two scans; the shift is paid by them).
    g. The quadratic shape: `for (i = 0; i < 4096; i++) e.setAttribute("n" + i, "v")` on an
       element with no attribute: floor the sum of two scans for each write, about
       2 * 4,096 * 4,096 / 8 = 4,194,304, and the loop ends `Ran` under 50,000,000 steps.
    h. A floor must fail when its own member's charge is removed: the implementor removes each
       charge of decision 7 in turn (the scan, the value, the names, the validation, the tree's
       second scan, the tree's copy, the per-element step), runs the module, and the report
       names for each removal the cases that went red; a charge with no red case gets a case
       or a tighter floor. The report states each case's time and ratio against the plain walk;
       a ratio over 20 is a defect of the price, fixed by raising the charge in the same change.
       No case runs over 5 s; a case that does is named with its time and its shape is made
       smaller by a round number, with the floor from the smaller shape.
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
    step per 16 bytes in `Copies::copy`, charged before the copy) is merged first: at the
    start `git grep -n -F 'units / 64' -- crates/script/src/bindings.rs` (the line
    `cx.charge_steps(units / 64)`, `bindings.rs:628` at `83e6b8a`) prints nothing, else the
    floors of 5b, 6 and 11 are not the story's and the work waits for that commit. The
    test step ends within 120 s and no test runs over 60 s; a test over 5 s is named with its
    time in the report, and so is the time of `dom_attributes` and of `dom_attributes_cost` as
    a whole (the loops of 6b to 6d and the shapes of 11 are the ones to time); each case makes
    its own runner (5.9 to 16.4 ms in the bindings unit's measure, `webidl-bindings.md` "As
    built"); no model call and no network request in a test; no Python in anything that stays;
    an expensive product is made once per test run and never per test (the pages of 11 are
    built once for the module's cases that share a shape, not per case, and no case starts a
    server); no generator with a committed output is added: the existing one gains
    `charge_stored` and the per-element step, and the time of `slot task webidl-slow` is
    measured before and after, both in the report; a build or test job in the pool ends within
    10 minutes or names its time with `slot --timeout`, at most 30 minutes, and is not started
    again unchanged; every compiling or testing command carries `slot`; a script that measures
    or waits starts without it.
15. The report holds, as text: the numbers measured in 5, the steps of 5d, the number written,
    the steps and `heap_bytes()` of 6b, the steps and N of 6d, the cases of 11 with their steps,
    times and ratios and the cases that went red for each removed charge (11h), the times of
    `webidl-slow` before and after, and the output of 13a.

## Out of scope

`Element.attributes`, `Attr` and `NamedNodeMap`, which are not generated, and the five attribute
node members and `Document.createAttribute` and `createAttributeNS`, which are rows of `throws`
(an attribute has no identity in the tree, Context): they stay `throws`, and a later story gives
them an identity rule of its own, a wrapper keyed by the element, the namespace and the local
name; `classList` and the token lists (`selection.txt:88`, `reason: token lists`); the attributes
of a `ProcessingInstruction` (rows 205-211, `throws`); an open namespace in `crates/html`, so
that `setAttributeNS` can store an attribute in a namespace outside the six (decision 9), a
later story of its own; a refund of what the peak charged, or of a node's charge (decision 8);
the words of `Limit::InputBytes` and `Limit::Steps` (`story:scripts-dom-mutation`); creating an
element (`createElement` is the slice `mutation`'s); the parse that runs a page's scripts
(`story:scripts-parse-run`); a page in a session and the example of a link built by script
(`story:scripts-document-port`); the argument price of decision 20 itself and the 16-byte rate
of `Copies::copy` (the reading unit's price commit).

## Placement

After `story:scripts-dom-reading` (its structure is built at `83e6b8a`; its price commit comes
first, acceptance 14) and `story:html-script-hook` (on `main`); the edges stay. The tests reach an
element through the reading story's members and its wrapper, and the cases call the write
surface of the hook story and the reads this story adds. Beside `story:scripts-dom-mutation` and
`story:scripts-dom-selectors`; the slice's code is its own three files, `dom/attributes.rs`,
`dom/attribute_reads.rs` and `dom/attribute_writes.rs`, and the test files are its own.
`crates/script/src/bindings.rs` is generated and is never merged by hand: whichever story
merges second takes `main`, regenerates it with the generator (`slot task webidl-check` and
`slot task webidl-slow` hold it) and checks the digest lines (acceptance 9).
Shared with the reading story: `crates/html/src/tree.rs` (it adds the write counter, this story
adds six reads and `Attribute::stored_bytes`; the reading story merges first and this story
merges `main` before it edits the file), `crates/webidl/selection.txt`, `bindings.rs`,
`crates/webidl/src/prelude.rs` (`strings_value` is the reading unit's open item and this
story's change), `crates/script/src/dom/reads.rs` (three tokens), `dom/mod.rs` (two `mod`
lines), `crates/script/tests/dom_reading.rs` (one line), `docs/design/scripts.md`, `AGENTS.md`
and the public surfaces; this story merges `main` before it regenerates `bindings.rs`.
Shared with `story:scripts-parse-run`: `crates/webidl/src/prelude.rs` and `bindings.rs` (it adds
`DomState::swap_tree` beside the fields this story adds; no line of one is the other's, and the
second merger regenerates), `docs/design/scripts.md`, `AGENTS.md` and the public surfaces. That
story lends the one tree to each script and leaves `live` and `peak` as they are. This story does
not depend on it: its tests run a document through `run_document`, as the reading story's do.
Shared with `story:scripts-dom-mutation`: `DomCx::charge_stored`, `DomCx::undo_stored` and the
two counts `live` and `peak` of `DomState`. This story adds them to
`crates/webidl/src/prelude.rs` and the mutation story calls the same function, with the same two
counts, for its character-data writes, so it needs this story's function or adds the same one;
whichever of the two merges second merges `main` first, takes the function and the counts as
they are, regenerates `bindings.rs` and does not add a second. The mutation story writes its
own file, and so does this one for its writes (`attribute_writes.rs`); the mutation story takes
the same form, or moves both into one shared file when it merges second and says so. They
share `docs/design/scripts.md:322-332` and `:902-904`, `AGENTS.md`
and the public surfaces; whichever merges later merges `main` first and takes its numbers from
its own log.
Shared with `story:scripts-dom-selectors`: `AGENTS.md`, the public surfaces, `tests/main.rs`
(one `mod` line each, in different places) and `dom/mod.rs`, on the same rule.
Drafts that touch the same files, none a dependency: `story:scripts-utf16-length` edits
`prelude.rs` (`string_length`) and `bindings.rs` and moves the argument charge before the copy,
so it lands in the generator beside this story's change and the later one regenerates;
`story:scripts-dom-collection-names` edits the `HTMLCollection` rows of `selection.txt` and the
glue, not this slice's rows; `story:scripts-dom-sibling-positions` edits the child-list writers
of `tree.rs` and `reading.rs`/`reads.rs`, not the attribute writes this story reads, so a
kept position never depends on an attribute.
The last commit of the story rewrites the public-surface lines of acceptance 13 (the test
counts in `README.md`, `docs/badges/make.sh`, `docs/badges/tests.svg`, `website/docs/*`); a
commit before it leaves them.
`story:scripts-html-conformance` and `story:scripts-document-port` depend on this slice (the
epic, lines 478-479 and 486).

## Why `epic:scripts-in-a-page`

Example 2 of the epic (lines 69-71) sets `href` with `setAttribute` on an element a script made,
and the session shows it as a link. This story is the half of that example that is not creating
the element and putting it in the tree, and it holds every script's attribute write to the
parser's attribute and stored-byte limits, as the epic's "Security" section asks (line 234), and
to the agent's memory budget, as a node's bytes are, and every read and write to a price that
a step buys a bounded amount of work.
