---
format: aep.planning-md/3
id: story:scripts-dom-reading
kind: story
status: active
title: The reading members (99 rows) and one wrapper per node
summary: Node, Element and Document's 73 reading members, 21 of the interfaces they return and the five of Window and Document.title (S6, decided a on 2026-11-24, and Window.event) read a parsed tree as the standards say, with one wrapper per node for the document's life, memory bounded by the engine's budget, sibling reads by position hints, the snapshot and global-this deviations, the four [SameObject] collections held by the generator and Wrap (one object per node and member, refreshed only when the tree's write counter moved), the document's address as a surf_url::Url in a public DocumentInfo, compatMode from the quirks mode the parser writes into the tree where the initial insertion mode decides it (so a script in the parse reads it) and baseURI as the HTML document base URL, the case names asserted equal to the slice's rows.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:webidl-bindings
- depends_on: story:html-script-hook
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Cargo.lock
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
  path: crates/html/tests/adversary_tree_state.rs
- confidence: cited
  path: crates/html/tests/main.rs
- confidence: cited
  path: crates/html/tests/tree_state.rs
- confidence: cited
  path: crates/script/Cargo.toml
- confidence: cited
  path: crates/script/src/bindings.rs
- confidence: cited
  path: crates/script/src/dom/mod.rs
- confidence: cited
  path: crates/script/src/dom/reading.rs
- confidence: cited
  path: crates/script/src/dom/wrapper.rs
- confidence: cited
  path: crates/script/src/lib.rs
- confidence: cited
  path: crates/script/src/runner.rs
- confidence: cited
  path: crates/script/tests/adversary_dom_reading.rs
- confidence: cited
  path: crates/script/tests/bindings.rs
- confidence: cited
  path: crates/script/tests/dom_reading.rs
- confidence: cited
  path: crates/webidl/selection.txt
- confidence: cited
  path: crates/webidl/src/generate.rs
- confidence: cited
  path: crates/webidl/src/prelude.rs
- confidence: cited
  path: crates/webidl/tests/adversary_same_object.rs
- confidence: cited
  path: crates/webidl/tests/main.rs
- confidence: cited
  path: crates/webidl/tests/same_object.rs
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
revision: 35
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:13Z", actor: "human:operator", revision: 14}
- {from: "proposed", to: "active", at: "2026-11-25T08:44:48Z", actor: "human:operator", revision: 27, executor: "agent:coordinator"}
---
## Context

The epic's eighth candidate (`.engineering/planning/epic/scripts-in-a-page.md:464-468`); "Behaviour
in the first version, by slice", tree reading (lines 716-724); the deviations (lines 749-754);
requirement 4 as the epic's "Answers of the script engine's session" give it (line 854) and the
sentence of the gaps (lines 342-343): surf keeps one wrapper per node strongly for the life of the
document. S6 is decided (option a, line 603): `Window`'s `document`, `window`, `self` and
`Document`'s `title` getter come from HTML's IDL. The epic's own lines that this story makes
false are corrected by one dated block (decision 16).

`story:webidl-bindings` is on `main` (merge `6bd9bc9`). What it left to this story, read from the
code at `6bd9bc9` (a line of `crates/html` is a line of `main` at `6bd9bc9`, or of the branch
`impl/html-script-hook` at its tip `58d720b` when it says "unit"; `git diff --stat
main...impl/html-script-hook` at that tip ends "22 files changed"):

- The slice `reading` has 99 member rows in `crates/webidl/selection.txt`, and one more row,
  `NodeList iterable` (line 236), which is no member. The 99 are `Node` 41 (its 18 constants
  included), `Element` 15, `Document` 18 (`title` included), the 21 members of the returned
  interfaces (`CharacterData` 5, `Text` 2, `ProcessingInstruction` 1, `DocumentType` 3,
  `DocumentFragment` 5, `NodeList` 2, `HTMLCollection` 3), and `Window` 4: `window`, `self`,
  `document` (lines 272-274) and `event` (line 278). `Window.event` is a row of the bindings
  unit: outside the dispatch of an event its value is `undefined` and surf dispatches none, so
  `undefined` is the standard's value (comment at lines 275-277).
- The trait `Reading` (`crates/script/src/bindings.rs:2051-2708`) holds 81 methods: the 99 rows
  less the 18 constants, which are generated. Every default answers `NotSupportedError` naming
  the member (`bindings.rs:133-135`). `crates/script/src/dom/reading.rs:8` is an empty `impl`.
- The `Window` global exists. `Runner::new` makes the global as a host object of `window_spec`
  (`crates/script/src/runner.rs:574-579`), and `install` (`runner.rs:857-880`) defines the
  interface objects and the global's members in one `with_cx` after `create_realm`.
  `run_document` (`runner.rs:431`) and `Document { tree, limits, slices }` (`runner.rs:421-425`)
  exist. `Window.window` and `Window.self` answer `TheWindow` (`bindings.rs:1182`), which the
  glue turns into the global object (`bindings.rs:1176`), but their trait methods
  (`bindings.rs:2689-2697`) still throw; so do `document` and `event` (`bindings.rs:2699-2707`).
- The `Document` of `runner.rs` holds no address, no content type and no encoding, and the
  generated `DomState` (`bindings.rs:151-161`, its `tree` private) has no field for them. The
  members `URL`, `documentURI`, `baseURI`, `characterSet`, `charset`, `inputEncoding` and
  `contentType` need them. `surf_html::Tree` holds none and no compat mode
  (`crates/html/src/tree.rs:195-200`; unit `tree.rs:249-255`).
- The parser decides the quirks mode in `pub(crate) enum QuirksMode`
  (`tree_builder.rs:64-68`; unit `:233`), in the initial insertion mode, at two places: the
  doctype token (`crates/html/src/tree_builder/modes.rs:207`) and any other token, which means
  quirks (`modes.rs:217`); the same lines on the unit. It keeps the mode in the builder
  (`tree_builder.rs:448`, `:656`; unit `:736`, `:951`), which `tables.rs` reads (`:486`; unit
  `:492`), and copies it into the crate-private `Parsed` when the parse ends (`tree_builder.rs:429`;
  unit `:712`), under `#[cfg_attr(not(test), allow(dead_code))]` (`:320`; unit `:577`).
  `parse_document` returns the tree alone (`crates/html/src/lib.rs:230-237`; unit `:284-292`) and
  `ParsedDocument` carries only the encoding (`lib.rs:212-219`).
- On the unit the parser calls the script hook in the middle of a parse: the loop calls
  `report_ended` after the token that ended a script (`tree_builder.rs:683`, `:696`), which
  calls `hook.run(&mut self.tree, script)` (unit `:993`, `:1008`; the trait at `:82-88`), and
  `parse_document_with_hook` is unit `lib.rs:301`. So a script reads the tree before `Parsed`
  exists.
  The unit also adds `Tree`'s write surface (`append_child` unit `tree.rs:1033`, `remove_node`
  `:1039`, `insert_before` `:1014`), which `main` has not (`git grep -n 'pub fn append_child'
  6bd9bc9 -- crates/html/src` prints nothing), and `Tree::document(limits)` and
  `fragment(limits)` (unit
  `tree.rs:273, 278`, both `pub(crate)`). Its `Tree` derives `PartialEq` and keeps its parser
  cache `moved` out of it with a newtype (unit `tree.rs:249-269`); its tests compare a tree a
  hook wrote with a plain parse (`crates/html/tests/adversary_script_hook_2.rs:539, 545, 574`).
  `Tree` has no counter of writes: `steps` is the parse's and counts no write (unit
  `tree.rs:539-550`), and `moved` is a flag for elements that the parser clears (unit
  `tree.rs:362-366`, set at `:830`, `:953`, `:1389`). `Tree` has no sibling read
  (`main:tree.rs:444, 485`; the unit's `index_in_parent` and `next_sibling`, `:811-826`, are
  private).
- `surf-script` depends on `surf-html`, `surf-secret` and `swell`
  (`crates/script/Cargo.toml:14-16`). `surf-url` is a crate of this workspace with no dependency
  (`crates/url/Cargo.toml:13`); `Url::parse(input, base)` and `Url::href()` are public
  (`crates/url/src/lib.rs:209, 259`), `Url` derives `Clone, PartialEq, Eq, Debug`
  (`lib.rs:83`), and an input over `MAX_INPUT_LENGTH` (`lib.rs:67`) is `ParseError::TooLong`
  (`lib.rs:99`). Neither parse takes an encoding, and HTML parses the `href` of a `base` with the
  document's encoding for the query (HTML Standard, "set the frozen base URL").
- `taskfiles/lock-check.sh` fails only for a package from outside the repository that is not on
  `taskfiles/lock-allow.txt`, or a crate that depends on a listed package or on `surf-script`
  without a line of `taskfiles/lock-owners.txt` (`lock-check.sh:69-76`, `:388`); `surf-url` is
  neither, so a dependency of `surf-script` on it changes nothing in `taskfiles/` and adds one
  line, `"surf-url",` between `"surf-secret",` and `"swell",`, to the entry of `surf-script` in
  `Cargo.lock` (lines 225-231). Nothing else fixes the dependencies of `surf-script`:
  `webidl-check` reads the lock for the packages `surf-html` and `surf-webidl` only
  (`taskfiles/webidl.yml:21-25`), and no file of `taskfiles/`, `Taskfile.yml`, `.github` or
  `conformance/tests` names `surf-script` but the lock check and its owners file. The crate-graph
  sentence of `docs/design/scripts.md:1102` and the entry of `crates/script` in `AGENTS.md:365-368`
  name its dependencies (the entry says "swell and `surf-secret`").
- `crates/script/src/dom/wrapper.rs:13-23` is a `Wrap` whose `wrap` throws `NotSupportedError`;
  its `unwrap` is complete (it calls `wrapper_node`, `bindings.rs:360`). `new_wrapper`
  (`bindings.rs:345`) makes a wrapper of an interface with `NodeData { node }`
  (`bindings.rs:283`). `dom::slices()` (`crates/script/src/dom/mod.rs:23-30`) takes no argument
  and builds the unit structs of the four slices and `SurfWrap`. The trait `Wrap` is
  `crates/webidl/src/prelude.rs:206-211` (the generator's text, written into `bindings.rs:273`);
  four test types implement it (`crates/script/tests/bindings.rs:190`, `adversary_3.rs:237`,
  `adversary_4.rs:255`, `adversary_5.rs:125`).
- The generated getters of a collection call the slice for a `Vec<NodeId>` and make a new host
  object each time (`collection_value`, `bindings.rs:1132-1160`: one step per node at
  `:1139-1140`, one wrapper per node through `Wrap` at `:1155`, an own index property each). So
  the slice cannot keep an object, and `a.childNodes === a.childNodes` is false as generated
  (`bindings.rs:3663-3677`). The helpers it uses (`prototype_object` `:613`, `data_property`
  `:1185`, `engine` `:580`, `defect` `:575`) are private; `CollectionData` is public (`:294`).
  `NodeList.length`, `item` and `HTMLCollection.length`, `item`, `namedItem` are `reading` rows
  whose glue clones the snapshot from the object's data on each call (`this_collection`,
  `prelude.rs:683-696`) and passes it as `&[NodeId]` (`bindings.rs:2645-2687`). `for (x of
  collection)` throws `NotSupportedError` until they have behaviour: `NodeList` and
  `HTMLCollection` have `@@iterator` from `%Array.prototype.values%` and `NodeList` has
  `entries`, `keys`, `values` and `forEach` (`bindings.rs:1459-1477`).
- `Slices` has public fields `reading`, `attributes`, `mutation`, `selectors` and `wrap`
  (`bindings.rs:455-466`), `DomCx::tree()` and `tree_mut()` are public (`:209`, `:215`), and the
  slice `mutation` has `Node.appendChild` and `Node.removeChild` (`bindings.rs:2917`, `:2938`).
- Three tests of `crates/script/tests/bindings.rs` read `Window.document` or `Window.event`
  through `dom::slices()` and expect `NotSupportedError`:
  `a_member_of_a_slice_throws_until_its_slice` (line 495, its strings at `:507-508`),
  `the_strings_of_a_dom_exception_outlive_a_collection` (line 563, string at `:580`) and
  `an_uncaught_dom_exception_names_its_name_and_message` (line 739, string at `:752`).
- `crates/script/tests/main.rs:15` declares `mod dom_reading;`; `tests/dom_reading.rs` holds
  its head comment only.

The epic's statement that the wrappers are "bounded by the parser's node limit" does not hold at
the session's values: the parser allows 1,048,576 nodes (`crates/html/src/limits.rs:65`), a
wrapper costs the engine 256 bytes and its data (`docs/design/scripts.md:322-330`), and the
agent's budget is 67,108,864 bytes (`crates/script/src/runner.rs:42`), so 262,144 wrappers alone
fill it (`docs/design/scripts.md:330`). Decision 4 says what bounds the memory.

## Decisions taken

1. The members are the 81 methods of `Reading` and the 18 constants and the `iterable` row, which
   the bindings generate and which need no method (`bindings.rs:2051-2708`,
   `selection.txt:236`). The 99 member rows are the counts of the Context. The title, the summary
   and acceptance 1 say 99.
2. The four members of `Window` are written in `reading.rs` with the others: `window` and `self`
   answer `TheWindow`, `event` answers `Ok(())` (`undefined`), `document` answers the document
   node, `cx.tree().root()`, which `Wrap` makes a wrapper. The `Window` global itself,
   `run_document` and `Document` are the bindings unit's and are not written again; there is no
   `crates/script/src/dom/window.rs`.
3. A wrapper is made with `new_wrapper` (`bindings.rs:345`) of the interface that matches the
   node's `surf_html::NodeData`: `Document`, `DocumentFragment`, `DocumentType` (`Doctype`),
   `Element`, `Text`, `Comment` and `ProcessingInstruction`; all seven are in
   `Interface` (`bindings.rs:1665-1680`). An element's wrapper is an `Element` whatever its name:
   the deviation of the epic (line 754), whose text is in the rows `Document createElement` and
   `createElementNS` of the slice `mutation` (`selection.txt:150-151`).
4. One wrapper per node for the document's life. `SurfWrap` keeps a table from `NodeId` (it is
   `Hash`, `crates/html/src/tree.rs:64-65`) to a `Root` made with `Cx::root` (as the realm's own
   roots are, `bindings.rs:1509-1515`), never emptied while the document lives, because a handle
   that is not a root is weak (requirement 4, `docs/design/scripts.md:1275`). What bounds it:
   - Each wrapper is one host object on the engine's heap, `HOST_OBJECT_LIVE_BYTES` (256) bytes
     plus its data's size, counted by `heap_bytes()` and charged against `memory_bytes`
     (`docs/design/scripts.md:322-330`); the engine charges it when `new_host_object` makes it
     (`bindings.rs:345-356`). No function of surf charges a wrapper: `DomCx::charge_node`
     (`bindings.rs:249-262`) is the charge of a node a script creates and a reading member never
     calls it.
   - `memory_bytes` is 67,108,864 (`runner.rs:42`) and the node limit is 1,048,576
     (`limits.rs:65`), so the memory budget ends a script that wraps about a quarter of a
     default-size tree first, `limit: memory` (`docs/design/scripts.md:320`), and the table has at
     most one entry per wrapper made, that is at most 262,144 at the session's values. A script
     that reaches more nodes than that ends `limit: memory`: at the session's values the budget,
     not the node limit, bounds the wrappers.
   - The kept collection objects of decision 10 are held the same way: at most four per node,
     each a host object with its node list as data, charged by the engine as any host object
     and bounded by the same budget; their cost is part of what the implementor measures first
     (acceptance 8).
   - The table's own bytes are surf's and not charged; they are at most the number of wrappers
     and kept objects times the size of an entry.
5. A collection a member returns is the bindings' own snapshot (`collection_value`): the slice
   answers the nodes in tree order and the glue wraps each, charging one step per node
   (`bindings.rs:1139-1140`). `length` answers the snapshot's length, `item(index)` the node at
   `index` or `null`, `namedItem(name)` the first element of the snapshot whose ID is `name` or,
   in the HTML namespace, whose `name` attribute is `name` (DOM Standard, `HTMLCollection`,
   `namedItem`), and `null` for the empty string and for no such element; an element outside the
   HTML namespace is matched by its ID only. The iteration methods are generated
   (`bindings.rs:1459-1477`) and need no code. The glue copies the snapshot on each call of
   `length`, `item` and `namedItem` (`this_collection`, `prelude.rs:683-696`) with no step.
   A member's work is bounded by steps, not by the wall clock alone, so the glue stops copying.
   A plain borrow of the node list cannot be built: `DomCx` holds `&'c mut Cx`
   (`bindings.rs:201-205`), and a slice gets `this: &[NodeId]` beside its `dcx`
   (`bindings.rs:2645-2687`), so the list cannot stay borrowed from the host data while `dcx`
   exists. The first form is take and restore: the glue takes the `Vec` out of the host data
   with `Cx::host_data_mut` (`crates/swell/src/host/mod.rs:549 @ 0.8.0`), passes `&vec` to the
   slice, and puts it back, also when the slice returns an error. `length` reads the length from
   the data and copies nothing, in every form. The fallback, for `item` and `namedItem` only, is
   the copy: the glue keeps it and charges one step per 64 node ids copied, before the copy. A
   call then costs `1 + ceil(n / 64)` steps over a collection of `n` nodes, and a loop of `m`
   such calls costs about `m * (1 + n / 64)`: with `m` equal to `n` that reaches the 10,000,000
   steps of an entry (`runner.rs:41`) at `n` of about 25,270 (`n + n * n / 64` is
   10,002,971 there), where the first form costs `m` steps. The implementor finds out first whether
   take and restore builds, reports the compiler's message if it does not, and the report says
   which of the two forms was built.
6. No reading member records a limit. The members read the tree and never write it, so none
   answers `DomEnd::Limit` and none calls `end_with_limit` (`bindings.rs:445`); `Limit::InputBytes`
   and `Limit::Steps` keep their placeholder word `limit: wall-clock` (`runner.rs:690-693`), which
   the first story with a member that writes into the tree replaces. A reading member ends a
   script only through the engine: `limit: steps-per-script` or `limit: memory`, or a throw.
7. Steps and copies. The glue charges one step per call (`bindings.rs:3609` and the like). A member
   whose work grows with the tree charges one step per node it visits (as `collection_value` does,
   `bindings.rs:1140`) and one per 64 bytes it builds (as `Copies::copy` does,
   `bindings.rs:561`), before the work. These members are `childNodes` and `children` (when the
   kept object is refreshed, decision 10), the three `getElementsBy...`, `textContent`,
   `wholeText`, `title`, `isEqualNode`, `contains`, `compareDocumentPosition`, `getRootNode`,
   `isConnected`, `lookupPrefix`, `lookupNamespaceURI`, `isDefaultNamespace`, `childElementCount`,
   `namedItem` and `baseURI`; `Tree::descendants` allocates a list of every node
   (`crates/html/src/tree.rs:455-463`) and is charged by its length. The sibling members are not
   in this list: decision 9 gives them their own rule. A String a member returns is made by
   `Cx::new_string`, which the engine charges, and one beyond `string_length` (16,777,216,
   `runner.rs:46`) is a `RangeError` (`docs/design/scripts.md:307`). swell at the pin applies
   `string_length` only to a String its own operations create, and `Cx::new_string` does not
   check it. So the glue's one conversion of a member's String answer throws the `RangeError`
   itself, before `Cx::new_string`: it counts UTF-16 code units against the limit the agent
   was made with, and counts only an answer of more bytes than the limit. A member of the slice
   does not build what is thrown away: a copy of one node's data is checked before the copy,
   and a member that concatenates stops when what it gathered passes the limit. The error is
   the realm's `RangeError`, which a script can catch. A reading member calls no
   `to_utf16` and no `to_rust_string_lossy`: its String arguments reach it as Rust strings copied
   by `Copies::copy` (`bindings.rs:553`), with a lone surrogate written U+FFFD
   (`selection.txt:24`).
8. The pin, the requirements and swell's story. `crates/script/Cargo.toml:16` pins swell `0.8.0`,
   and `docs/design/scripts.md:1275-1281` records requirements 4, 9 and 10 at `0.8.0` (row 9,
   `%Array.prototype.values%`, `@@iterator`, `held`, line 1280). The implementor reads the rows
   of Q18 at the tag the file pins when the work starts (`git show <tag>:conformance/host/
   requirements.txt` in a clone of swell) and the report holds them (acceptance 15). There is no
   edge between this story and `story:swell-0-9-0`: the two are built in separate trees and merged
   one after the other. They write the same files (Placement), so whichever merges second merges
   `main` first and takes its numbers from its own log, as the Placement of that story says for
   this pair; if swell's story merged first the pin is its tag and the rows are read there.
9. Sibling reads keep position hints in the slice, and `Tree` gets no sibling read. `Tree` has no
   sibling read, so a sibling member would scan the parent's children: `nextSibling` over `n`
   siblings visits about `n * n / 2` nodes, which passes the 10,000,000 steps of an entry
   (`runner.rs:41`) at about 4,500 siblings. `SurfReading` holds a table of eight
   `(parent, index)` pairs, replaced in turn, for the child it last answered or was asked
   about. A call of `previousSibling`, `nextSibling` (`Node`), `previousElementSibling` or
   `nextElementSibling` (`Element` and `CharacterData`) looks in the table for its node's parent
   and tests whether the
   parent's child at the hinted index, the one before it or the one after it is the node asked
   about; a hint is never trusted, so a stale one costs a scan and never a wrong answer. On a hit
   the call costs its own one step (the glue's) and visits nothing beyond the neighbours it
   checks; without a hit it scans the parent's children and charges one step per node visited.
   An element variant moves on over non-elements and charges one step per node it passes. The
   answer's index replaces the hint. The table is state of the slice, not of the tree: it needs
   no counter and survives a write (the test of acceptance 7).
10. `[SameObject]` is held by the generator and `Wrap`, with the refresh rule of a counter. The
    attributes of the selected definitions that are `[SameObject]` and return a `NodeList` or an
    `HTMLCollection` are four rows of `reading`: `Node childNodes` (`selection.txt:55`),
    `Element children` (`:118`), `Document children` (`:167`) and `DocumentFragment children`
    (`:223`), from `Node` (`crates/webidl/data/whatwg/dom.html:17640`) and the mixin `ParentNode`
    (`:17526`, included by `Document`, `DocumentFragment` and `Element` at `:17540-17542`). The
    other `[SameObject]` attributes of those definitions are rows of `throws` and have no getter
    to generate (`Document.implementation` `selection.txt:137`, `Element.classList` `:88`,
    `Element.attributes` `:91`). The mechanism:
    - The key of a member is a generated `pub enum SameObject` with one variant per such row, four
      now: `NodeChildNodes`, `ElementChildren`, `DocumentChildren`, `DocumentFragmentChildren`
      (the interface and the member in camel case, as `generate.rs:1131` names `Return`'s
      variants). The generator reads `[SameObject]` from the member
      (`has_attribute(member, "SameObject")`, `crates/webidl/src/generate.rs:642`; a member's
      attributes are `Member::extended_attributes`, `crates/webidl/src/model.rs:88`) where it
      chooses the return form (`generate.rs:1131`, `:227-229`), and writes the enum beside
      `Interface` (`write_interfaces`, `generate.rs:1768`).
    - `Wrap` gains two provided methods (`prelude.rs:206-211`): `same_object(cx, node, member)`
      answers the kept object of a node and member, if any (default `None`), and
      `keep_same_object(cx, node, member, object)` keeps one (default: nothing). `Wrap` only finds
      and keeps; creation and refresh stay in the glue, which calls `collection_value` for both
      paths, so no helper of `bindings.rs` becomes public. `CollectionData` gains a field
      `written: u64` (`prelude.rs:227-230`).
    - The getter of a row: with `writes = cx.tree().writes()`, it asks `Wrap` for the kept object.
      If there is one and its `written` equals `writes`, the getter answers it after its one
      step and does not ask the slice. If there is one and `written` differs, it asks the slice,
      charges one step per old and per new node, replaces the object's data (`Cx::host_data_mut`,
      `crates/swell/src/host/mod.rs:549 @ 0.8.0`), deletes the old index properties
      (`delete_property_or_throw`, `:838 @ 0.8.0`), defines the new ones as `collection_value`
      does, sets `written` and answers the same object. If there is none, it asks the slice,
      makes the object with `collection_value`, sets its `written` to `cx.tree().writes()` as it
      makes the object, and hands it to `keep_same_object`.
    - `SurfWrap` keeps a second table, `(NodeId, SameObject)` to `Root`, never emptied, and
      overrides both methods. With the defaults a getter makes a new object per read, as today:
      the four `TestWrap` implementations compile unchanged and their getters do not hold
      `[SameObject]`; only `SurfWrap` does (a test pins both, acceptance 4).
    - The write counter. `Tree` gains `writes(&self) -> u64`, kept out of `PartialEq` by a
      newtype as the unit keeps `moved` out (unit `tree.rs:258-269`). Neither `steps` nor `moved`
      can serve (Context). It is raised by each write that changes a child list, that is each
      function that assigns a node's `children` or `parent`, a parse's inserts included, and by
      nothing else. A write that changes only a node's `data` or its attributes does not raise it
      (`set_attribute`, `set_attribute_ns`, `remove_attribute`, `remove_attribute_ns`,
      `toggle_attribute`, `add_attributes`; `replace_data`, `set_data`, `append_data`,
      `insert_data`, `delete_data`; `push_text`, which only extends an existing text), and neither
      does `create` or a `create_*` function, which adds a node to the arena and to no child list.
      A function that can do both raises it when it changes a child list (`set_text_content`,
      `normalize`, `split_text`). The reason for the narrow rule: the counter exists only to
      refill the kept `childNodes` and `children` objects. If every write raised it, a loop
      `for (i = 0; i < p.children.length; i++) p.children[i].className = "x"` (the attribute
      slice's members, a later story) would read the kept object twice per turn after a write
      that changed no child list, cost `2 * n * n` steps and end `limit: steps-per-script` at
      about 2,236 children, as the loop below does.
      The reason it exists: `for (i = 0; i < el.childNodes.length; i++) el.childNodes[i]` reads
      the attribute twice per turn; refreshed at each read at one step per node, a parent of `n`
      children costs `2 * n * n` steps against 10,000,000 (`runner.rs:41`), so `n` of about 2,236
      ends `limit: steps-per-script` (`2 * 2237 * 2237` is 10,008,338 steps, `2 * 2236 * 2236`
      is 9,999,392). With the counter a read after no write costs one step.
    - The note of `DocumentFragment children` becomes the note of the other three: `deviation: a
      snapshot for a live collection until swell's slice 9 (HOST-112, HOST-114); deviation:
      [SameObject] as the same snapshot object refreshed when read`. That makes ten snapshot rows
      with a note, not nine (`selection.txt:55, 113-115, 118, 147-149, 167, 223`).
11. The compat mode comes from the tree, and the parser writes it where the initial insertion mode
    decides it. `Tree` gains a field of the mode, with `quirks_mode(&self)` public, and
    `QuirksMode` becomes `pub` (re-exported by `crates/html/src/lib.rs`). The parser writes it at
    the two places that decide it (`modes.rs:207`, `:217`), not when the parse ends, because the
    hook runs inside the parse (Context): a script in `head` of a page without a doctype reads
    the mode already. The mode is content: it takes part in the derived equality of `Tree`, and a
    hooked parse and a plain parse of one input decide the same mode (the unit's tests of
    `assert_eq!(tree, without)` still hold). `Tree::document` and `Tree::fragment` start in
    no-quirks mode, and the fragment parse keeps what the unit's text says, that its document is
    in no-quirks mode (`lib.rs:291`; unit `:366`). `Parsed.quirks_mode` and its
    `#[cfg_attr(not(test), allow(dead_code))]` go; the builder keeps its own field for
    `tables.rs`, and the parser's three tests of the mode (`tree_builder.rs:1416-1434`; unit
    `:1960-1978`) read `tree.quirks_mode()`. `Document.compatMode` is `BackCompat` for the mode
    "quirks" and `CSS1Compat` otherwise (DOM Standard, `dom.html:8121-8123`), and
    `getElementsByClassName` matches class names ASCII case-insensitively in "quirks" mode
    (`dom.html:7903-7906`). The caller does not give it.
12. The document's address is a `surf_url::Url` in a public
    `DocumentInfo { address: Url, content_type: String, encoding: String }` (the encoding is the
    name `characterSet` answers, a `String`, not a type), given to `dom::slices(info)` and held by
    `SurfReading`. `DocumentInfo::default()` is `about:blank`, `text/html` and `UTF-8`, so a test
    that gives no address gets `about:blank`. `URL` and `documentURI` answer `Url::href()`
    (`url/src/lib.rs:259`). `surf-script` gains the dependency
    `surf-url = { path = "../url" }` and its lock line (Context). `story:scripts-parse-run` builds
    the `DocumentInfo` of a page and calls `dom::slices(info)`; that story's own text takes the
    duty (decision 16), and which encoding name it passes when the runner is made while the
    encoding is still tentative (`scripts-parse-run.md:72-77`) is its to decide.
13. `Node.baseURI` is the document base URL as the HTML Standard defines it, read from the tree
    at each call, so a later write to the tree needs no cache. The DOM Standard's getter steps
    (`dom.html:6667-6677`): the node document's "get the base URL" result, or the document's URL
    when that is null, serialized; HTML overrides "get the base URL" for the `base` element
    (`dom.html:8015-8016`). The HTML Standard's text is not in the tree (`html-idl.html` holds
    its IDL blocks only), so its sections are named, not quoted: "document base URL", "fallback
    base URL" and "frozen base URL". The steps:
    - the fallback base URL is the address of `DocumentInfo`;
    - the first `base` element in tree order, in the HTML namespace, that has an `href` attribute
      decides; with none, the answer is the fallback base URL;
    - the `href` is parsed with `Url::parse(href, Some(&fallback))`; a failure
      (`ParseError`, an `href` over `MAX_INPUT_LENGTH` bytes, `url/src/lib.rs:67, 99`) answers the
      fallback base URL; the answer is serialized with `href()`;
    - the scan charges one step per node visited and the parse one step per 64 bytes of `href`
      (decision 7).
    Three deviations, each named in the note of the row `Node baseURI` (`selection.txt:48`): the
    check of the `base-uri` directive of a Content Security Policy is left out, because surf reads
    none (`docs/design/scripts.md:149`); the creator's base URL that a document at `about:blank`
    or `about:srcdoc` inherits is left out, because surf has no nested document and no creator;
    and the `href` is parsed as UTF-8 whatever the document's encoding, which HTML uses for the
    query (`Url::parse` takes no encoding, `url/src/lib.rs:209`). The frozen base URL is the
    result of the same parse, taken at each read: it is the value HTML freezes while the fallback
    base URL does not change, and surf's address does not change.
14. The refresh and the sibling cases write the tree through a test slice. The module
    `dom_reading` builds its `Slices` with `dom::slices(info)` and then replaces the public field
    `mutation` (`bindings.rs:455-466`) with a test implementation of the slice `mutation`; no
    public constructor for surf's reading or wrap is added. Its member `Node.removeChild`
    (`bindings.rs:2938`) removes the node through `DomCx::tree_mut()` and
    `Tree::remove_node`, and its `Node.appendChild` (`bindings.rs:2917`) appends through
    `Tree::append_child` (both of the unit's write surface). This is test code of `dom_reading`
    only; `story:scripts-dom-mutation` writes the real slice.
15. The cases are the rows. One table, `const CASES: &[Case]`, holds the cases; a `Case` is a row
    name, a JavaScript expression and its expected value as JavaScript text. One helper runs an
    entry: it makes the entry's own runner (`run_document`, a document of its page and
    `dom::slices(info)`), evaluates `expression === expected` and throws on a mismatch, asserts
    the end is `ScriptEnd::Ran`, and asserts `runner.document().and_then(|dom| dom.limit())` is
    `None`. A row name is `<interface>.<member>`, the first two words of its line of
    `selection.txt` (`Node.ELEMENT_NODE`, `Node.baseURI`, `NodeList.iterable`), as
    `selection()` reads them (`tests/bindings.rs:258-275`). A row may have several entries; each
    entry has a non-empty expression and expected value.
16. The texts that this story makes false are corrected in the same wave, each by a dated block
    or paragraph that is part of the plan, not of this story's code: the epic by one block, its
    separate last section "Corrections of 2026-11-25" (not its "Decisions taken"), which covers
    the epic's lines 464-468, 342-343, 854, 749-754, 704-705, 716-724, 910, 919-920 and 925-927;
    `story:scripts-parse-run` by a paragraph that gives it the `DocumentInfo` duty, and
    `story:swell-0-9-0` by a sentence in its Placement.
17. What the review of the unit's first tip found, as rules. Each narrows the decision it names
    and wins where the two differ.
    - The refill of a kept collection is all or nothing (decision 10). A kept `childNodes` or
      `children` object is a snapshot: its data (`length`, `item`, `namedItem`) and its index
      properties always show the same list. The refill writes the new nodes in place with
      define and never with set, so no script code runs in a refill and a setter a script put
      at an index is not called; it deletes the tail and writes the data and `written` last. If
      a step fails (the script froze or sealed the object, made it non-extensible or made an
      index non-configurable), the refill puts back what it wrote, leaves the data and `written`
      untouched, and the getter throws a `TypeError`. The object is then the old snapshot,
      whole, and each later read of the getter tries again. No index at or past `length`
      answers a node the glue put there. This is a deviation: a browser refuses the freeze or
      the define on the collection itself. The notes of the four rows and the design note name
      it.
    - A member charges what it does (decision 7). No member does work of `k * m` behind a charge
      of one. `getElementsByClassName` takes its argument as an ordered set (duplicates dropped,
      as the DOM says), charged one step per 64 bytes, and charges per element one step and one
      per 64 bytes of its `class` attribute before a scan that is linear in those bytes.
      `documentElement`, `doctype` and the lookups that start at the document charge one step
      per child of the document they scan. The reason: an interrupt is taken only after a host
      call, so one call whose time is not bounded by its steps runs past the wall clock.
    - `baseURI` (decision 13): the first `base` with an `href` decides; if its URL does not
      parse or its scheme is `data` or `javascript`, the answer is the fallback base URL, as
      "set the frozen base URL" says. A later `base` is not read instead.
    - A sibling hint holds two positions per parent, the two last answered (decision 9), so a
      parent read from both ends in turn stays linear. The table keeps its eight entries.
    - `lookupNamespaceURI`, `lookupPrefix` and `isDefaultNamespace` on a `DocumentType` and a
      `DocumentFragment` answer what the DOM's "locate a namespace" and "locate a namespace
      prefix" answer there, null also for the prefixes `xml` and `xmlns`.
    - `substringData` answers UTF-16 code units: a cut inside a surrogate pair answers the lone
      half. The answer is made with `Cx::new_string_utf16`; the length check of decision 7 and
      the charge per 64 bytes apply; the member walks to its offset and does not convert the
      whole data.
    - The write counter is not raised by an insert that moves no node, such as an empty
      `DocumentFragment` appended (decision 10: "by nothing else").
    - The generator refuses a `[SameObject]` row it cannot hold (an attribute of the global or
      of a non-node interface that returns a collection) with an error that names the row.
    - An `HTMLCollection` has no named properties: `collection["id"]` is undefined and
      `namedItem` answers. A named deviation in the notes of its rows; a later story builds
      them.

## Change

- `crates/script/src/dom/reading.rs`: the 81 methods of `Reading`, decisions 1 to 7, 9, 11, 13:
  - `Node`: `nodeType`, `nodeName`, `baseURI`, `isConnected`, `ownerDocument`, `getRootNode`,
    `parentNode`, `parentElement`, `hasChildNodes`, `childNodes`, `firstChild`, `lastChild`,
    `previousSibling`, `nextSibling`, `nodeValue` and `textContent` (getters), `isEqualNode`,
    `isSameNode`, `compareDocumentPosition`, `contains`, `lookupPrefix`, `lookupNamespaceURI`,
    `isDefaultNamespace`;
  - `Element`: `namespaceURI`, `prefix`, `localName`, `tagName`, `shadowRoot` (`null`: no shadow
    trees), `assignedSlot` (`null`), the three `getElementsBy...`, `children`,
    `firstElementChild`, `lastElementChild`, `childElementCount`, `previousElementSibling`,
    `nextElementSibling`;
  - `Document`: `URL`, `documentURI`, `compatMode`, `characterSet`, `charset`, `inputEncoding`,
    `contentType`, `doctype`, `documentElement`, `getElementById`, the three `getElementsBy...`,
    `children`, `firstElementChild`, `lastElementChild`, `childElementCount` and the getter of
    `title` (HTML);
  - the 21 of the returned interfaces: `CharacterData` `data` (getter), `length`,
    `substringData`, `previousElementSibling`, `nextElementSibling`; `Text` `wholeText`,
    `assignedSlot`; `ProcessingInstruction` `target`; `DocumentType` `name`, `publicId`,
    `systemId`; `DocumentFragment` `getElementById`, `children`, `firstElementChild`,
    `lastElementChild`, `childElementCount`; `NodeList` `item`, `length`; `HTMLCollection`
    `length`, `item`, `namedItem`;
  - `Window`: `window`, `self`, `document`, `event`.
  The struct `SurfReading` holds a `DocumentInfo` and the table of sibling hints, and answers
  `URL` and `documentURI` with the address (`Url::href()`), `contentType`, `characterSet`,
  `charset` and `inputEncoding` with the others, `compatMode` from the tree's mode and `baseURI`
  by decision 13.
- `crates/script/src/dom/wrapper.rs`: `SurfWrap`, decisions 3, 4 and 10: the table from `NodeId`
  to `Root`, `wrap` answering the one wrapper of a node, made on first use, the table of the kept
  collection objects and the two overridden methods of `Wrap`. `unwrap` stays.
- `crates/script/src/dom/mod.rs`: `pub struct DocumentInfo` (`Default` is `about:blank`,
  `text/html`, `UTF-8`) and `slices(info: DocumentInfo)`, which gives it to `SurfReading`
  (`slices()` has no argument now, `mod.rs:23-30`). `Document`, `DomState` and the generated
  `bindings.rs` do not gain a field for it.
- `crates/script/Cargo.toml`: the dependency `surf-url = { path = "../url" }` after `surf-secret`
  (the file's lines 14-16); `Cargo.lock`: `"surf-url",` in the entry of `surf-script`, between
  `"surf-secret",` and `"swell",`. Nothing of `taskfiles/` changes (Context).
- `crates/script/src/lib.rs`: the head comment, which says "No DOM yet" (line 5).
- `crates/script/src/runner.rs`: `ScriptHost` carries the agent's `string_length`
  (`ScriptHost::new(mode, string_length)`, a public read `string_length()`, and the one call in
  `Runner::new` that passes `limits.engine.string_length`), for the length check of decision 7.
  No other line of the file changes.
- `[SameObject]` (decision 10), in the generator's files and their outputs:
  - `crates/webidl/src/generate.rs` and `crates/webidl/src/prelude.rs`: the enum `SameObject`,
    the getters of the four rows, the two provided methods of `Wrap` and the field `written`,
    and `this_collection` and the glue of the `NodeList` and `HTMLCollection` members
    (decision 5: take and restore of the node list, or the copy with one step per 64 ids);
  - `crates/webidl/selection.txt`: the rows `Node childNodes` (line 55), `Element children`
    (118) and `Document children` (167) keep their notes; `DocumentFragment children` (223)
    gains both notes; `Node baseURI` (48) gains the three deviations of decision 13;
  - `crates/webidl/tests/same_object.rs` (declared in `tests/main.rs`, between `refusals` and
    `selection`): the test `same_object::the_getters_of_the_four_rows_ask_wrap_for_the_kept_object`
    over a small IDL, as `adversary_2.rs:32` calls `generate`, asserting that the getter of a
    `[SameObject]` collection attribute calls `same_object` and `keep_same_object` and that the
    getter of a collection operation does not;
  - `crates/script/src/bindings.rs`: regenerated with `slot cargo run -p surf-webidl --bin
    bindings`; its header's digests for `selection.txt`, `generate.rs`, `prelude.rs` and its own
    last line change (`bindings.rs:40-61`).
- `crates/html/src` (decisions 10 and 11), the files named here and no others: `tree.rs` (the
  mode field, `quirks_mode()`, the counter newtype and `writes()`, the constructors, the raise
  of the counter at each function that assigns a node's `children` or `parent`: on the unit
  `insert` `:404`, `remove` `:471` (when the node has a parent), `remove_children` `:485` and
  `move_children` `:495` (when a child moves), `insert_checked` `:927` (after its return for a
  node already in place) and `clone_node` `:1137` (when a copy gets a child: a deep clone of a
  node with children, or of a `template`). `insert_new`, `detach`, `remove_node`, `adopt_node`,
  `insert_before`, `append_child`, `replace_child`, `move_before`, `split_text` (with a parent),
  `normalize` (when it removes a node) and `set_text_content` (when it removes or adds a child)
  raise it through those, with no site of their own. Left out, because they assign `data` or
  attributes or add a node to no child list: `create` `:296` and the `create_*` functions,
  `push_text` `:444`, `add_attributes` `:455`, `put_attribute` `:1435`, `drop_attribute` `:1463`,
  `replace_data` `:1320` and the functions on it, `set_attribute` and the other attribute
  writes; the implementor greps the assignments to `self.nodes[...]` for any other, and checks
  this list against `tree.rs` at the merge of this story: a write function added after this story
  names it is added to the list),
  `tree_builder.rs` (`QuirksMode` public, `Parsed.quirks_mode` and its `allow` removed, the three
  tests), `tree_builder/modes.rs` (the two writes at `:207` and `:217`) and `lib.rs` (the
  re-export). Tests: `crates/html/tests/
  tree_state.rs`, declared in `crates/html/tests/main.rs`.
- `crates/script/tests/dom_reading.rs`: the module `dom_reading`, already declared in
  `tests/main.rs`.
- `crates/script/tests/bindings.rs`: the three tests of the Context change from `Window.document`
  and `Window.event` to a member of another slice that still throws (for example
  `document.appendChild`), so each keeps its subject; a test pins that a getter of a `TestWrap`
  makes a new object per read.
- `docs/design/scripts.md`: line 1102 (`surf-script` depends on `surf-html` for the tree) gains
  `surf-url` for the address.
- `AGENTS.md`: the entry of `crates/script` (lines 365-368: "no DOM yet", its dependencies, which
  name swell and `surf-secret` only, and the count of tests).
- In the last commit, the public surfaces of acceptance 13: `README.md`, `docs/badges/make.sh`,
  `docs/badges/tests.svg`, `website/docs/how-it-is-built.md`, `website/docs/status.md`,
  `website/docs/what-works.md`.

## Acceptance

1. The rows and the table (decision 15).
   a. `dom_reading::cases_are_the_rows` reads `crates/webidl/selection.txt` as `selection()` does
      (`tests/bindings.rs:258-275`) and asserts that the set of `Case` row names equals the 100
      rows with the slice `reading`, 99 members and `NodeList.iterable`: `Node` 41, `Element` 15,
      `Document` 18, `CharacterData` 5, `Text` 2, `ProcessingInstruction` 1, `DocumentType` 3,
      `DocumentFragment` 5, `NodeList` 3, `HTMLCollection` 3, `Window` 4.
   b. The same test asserts every entry has a non-empty expression and expected value.
   c. Each entry compares what the member returns with what the DOM Standard (the HTML Standard
      for `Window` and `Document.title`) says, the section cited in the entry's comment.
   d. The table tests (one per interface group) run each entry through the helper of decision 15,
      so each case makes its own runner and asserts the limit is `None` at its end.
2. Identity: `a.parentNode === a.parentNode`, the same for every member that returns a node,
   `document.documentElement === document.getElementById("a").ownerDocument.documentElement`, and a
   property a script set on a wrapper is read back while its node is in the tree, also after a
   `collect()` of the agent (`runner.agent_and_realm().0.collect()`).
3. Collections.
   a. `for (x of document.childNodes)`, `[...nodes]`, `nodes.length`, `nodes.item(i)`,
      `nodes[i]`, `nodes.forEach`, and the same node object from a collection and from a direct
      member (`document.documentElement.childNodes[0] === document.documentElement.firstChild`).
   b. `HTMLCollection.namedItem` by `id` and by `name` on an HTML element.
   c. `namedItem("")` is `null`; `namedItem("missing")` is `null`.
   d. An `svg` element with a `name` attribute is not found by that name, and one with an `id`
      is found by it.
4. Deviations of the slice, each as the selection names it.
   a. A snapshot does not change when the tree would.
   b. `a.childNodes === a.childNodes` and the same for the three `children` (`Element`,
      `Document`, `DocumentFragment`), with the old index properties gone after a refresh.
   c. With the counter: after a write through the test slice (decision 14) the second read
      answers the same object with the new children; with no write between two reads the second
      read adds exactly one step and visits no node.
   d. The `getElementsBy...` results are a new object per call
      (`document.getElementsByTagName("p") !== document.getElementsByTagName("p")`).
   e. With the default `Wrap` of a `TestWrap`, `a.childNodes !== a.childNodes`: the default does
      not hold `[SameObject]` (`tests/bindings.rs`).
   f. `Object.prototype.toString.call(a)` is `[object Element]` for a `div` and for a
      `custom-x` element, `[object Text]` for a text node: the tag is the interface's name on the
      prototype (`bindings.rs:1304`, as `adversary_3.rs:900` reads it), and a custom element's
      wrapper is an `Element`. Requirement 9 is `held` at `0.8.0` (`docs/design/scripts.md:1280`).
   g. `this === window`, `globalThis === window`, `window.window === window`,
      `self === window` and `window.document === document` at the top level of a script;
      `window.event === undefined`.
5. The document's facts.
   a. `document.title` reads the first `title` element's text, stripped and collapsed, as
      `surf.document.Document.title` is computed (`spec/domains/document.yaml:229-233`), and the
      empty string when there is none.
   b. `compatMode` follows the tree's mode: `BackCompat` for a page without a doctype and
      `CSS1Compat` for `<!DOCTYPE html>` and for a doctype that gives limited-quirks mode;
      `getElementsByClassName("A")` finds `class="a"` in the first and not in the second.
   c. `contentType`, `characterSet`, `charset`, `inputEncoding`, `URL` and `documentURI` read
      what the `DocumentInfo` holds; `DocumentInfo::default()` gives `about:blank`, `text/html`,
      `UTF-8`, and a test that gives no address reads `about:blank` from `URL`, `documentURI`
      and `baseURI`.
   d. `baseURI`, the address `https://example.com/dir/page` in each case, one case per line: no
      `base` element: the address; one `<base href="https://other.test/x/">`, also after the
      script's element: `https://other.test/x/`; two `base` elements with `href`: the first in
      tree order; a first `base` without `href` and a second with one: the second; a `base`
      without `href` alone: the address; an `href` for which `Url::parse` returns `Err` (taken
      from the url crate's own failing cases): the address; a relative `href` (`../up/`): the
      `href` parsed against the address (`https://example.com/up/`); an `href` over
      `MAX_INPUT_LENGTH` bytes: the address; a `base` in the SVG namespace: ignored.
   e. The three deviations are in the note of `Node baseURI`, and a case reads them from
      `selection.txt` as 1a does.
6. The hook cases (`story:html-script-hook`'s `parse_document_with_hook`, unit `lib.rs:301`). A
   `ScriptHook` of the test clones the tree in `run` (`Tree` is `Clone`), makes a `Document` of the
   clone with `dom::slices(info)` and reads `document.compatMode` through `run_document`.
   a. A script in `head` of a page without a doctype reads `BackCompat`.
   b. The same page with `<!DOCTYPE html>` reads `CSS1Compat`.
   c. The tree the hooked parse returns equals the plain parse's (equality includes the mode).
   d. `Parsed.quirks_mode` is gone: `git grep -n -F 'pub(crate) quirks_mode' --
      crates/html/src/tree_builder.rs` prints nothing (on the unit it prints the field of
      `Parsed`, `:578`; the builder's own field, which is not `pub(crate)`, and the three tests
      that read `tree.quirks_mode()` remain).
   Every other case of this story runs over `parse_document` and needs no hook.
7. Sibling reads (decision 9). `runner.steps()` is read around each script.
   a. `for (n = p.firstChild; n; n = n.nextSibling)` over 100,000 siblings ends `Ran` with the
      steps of the walk under 150 times those of the same walk over 1,000 siblings (a linear
      walk gives 100 times, a scan gives 10,000 times).
   b. The same walk backwards from `p.lastChild` with `previousSibling`, and the two element
      variants over a parent with a text node between each element.
   c. Two walks in step over two parents answer the right nodes.
   d. A walk that goes on after a write through the test slice (decision 14) between two reads
      answers the node after the removed one.
   e. A hint that names a node no longer a child of its parent costs a scan and answers the
      right node.
8. No reading member records a limit or reaches the tree's writers: every case asserts the
   record is `None` through the helper. Memory and steps. The implementor measures first, before
   any assertion below is written: the bytes per wrapper, per kept collection object and per
   node of a collection, and the steps of each case, recorded in the cases' comments; the
   assertions are the floor and the end named here, not the measured numbers.
   a. `wrapper_charge`: after a script that wraps 1,000 distinct nodes of one document,
      `heap_bytes()` has grown by at least 1,000 x 256 bytes over its value before, and the case
      names the number it measured per wrapper and per collection.
   b. `wrapper_limit`: under a memory limit of 1 MiB a script that wraps every node of a
      document of 100,000 elements ends `ScriptEnd::Limit` with the reason `memory`; a later
      `runner.evaluate("1 + 1")` ends `Ran` or `Limit` with `memory` (the one `collect()` after a
      memory end, `docs/design/scripts.md:320`, is the runner's, and the rooted wrappers are never
      freed, so the heap may still sit at the limit). The report says which of the two the case
      saw, and the case asserts that `heap_bytes()` after the later `evaluate` is below the
      limit plus one wrapper, that is under 1 MiB plus 256 bytes plus the size of a wrapper's
      data.
   c. `wrapper_reuse`: the script of 8a run a second time over the same nodes grows `heap_bytes()`
      by less than a tenth of the first run's growth: the table made no second wrapper per node.
   d. `collection_limit`: under the same limit `for (;;) document.getElementsByTagName("*")`
      ends `ScriptEnd::Limit` with `memory` or `steps-per-script`, and a later `evaluate` ends
      `Ran`.
   e. `steps_in_proportion`: `document.getElementsByTagName("*")` over a document of 1,000
      elements raises `runner.steps()` by at least 1,000.
   f. `text_content_steps`: `textContent` of a parent of 1,000 text-bearing children raises
      `runner.steps()` by at least 1,000, and so does `document.contains(last)` for the last of
      1,000 siblings.
   g. `string_steps`: the `textContent` of a text of 64,000 bytes raises `runner.steps()` by at
      least 1,000 beyond the same read of a text of 64 bytes (one step per 64 bytes).
   h. `string_length`: a document of one text of 16,777,217 bytes, parsed under limits raised
      for the case (`max_input_bytes` and `max_stored_bytes`), makes `textContent` throw a
      `RangeError`; if the case does not run within 5 s or ends `limit: memory` first, the report
      names its time and the end it reached and the case asserts that end instead. A text of
      16,777,216 units is read and its `length` is 16,777,216. The count is in UTF-16 units: a
      text of more bytes than the limit and at most the limit's units is read, and an astral
      character counts two. The `data` of one text beyond the limit and the `textContent` of
      several texts that pass it only together throw the same error. After a caught
      `RangeError` the same node answers `nodeName` and its parent's `childNodes` reads as
      before.
   i. The loop `for (i = 0; i < el.childNodes.length; i++) el.childNodes[i]` over 5,000 children
      ends `Ran`, with the steps under 15 times those of the same loop over 500 children (a
      refresh at each read gives 100 times). The loop calls `length` and reads index properties
      and no `item`, so its bound is the same in both forms of decision 5.
   j. `var c = document.getElementsByTagName("*")` over 5,000 elements, then
      `for (i = 0; i < c.length; i++) c[i]`, ends `Ran`, with `runner.steps()` under 15 times
      the steps of the same loop over 500 elements (a copy charged per node gives 100 times; the
      case states the bound after the first measure). This loop too calls `length` and reads
      index properties only, and `length` copies nothing in either form (decision 5), so the bound
      holds in both forms. It does not cover `item` and `namedItem`: for them the bound holds
      for take and restore only. With the copy form a loop of `m` calls over `n` nodes costs
      about `m * (1 + n / 64)` steps, which is `n * n / 64` for `m` equal to `n`: over 5,000
      nodes about 390,000 steps, over 500 about 4,000, a ratio of about 90, and an entry ends
      `limit: steps-per-script` at `n` of about 25,270 (a case that wants that end uses 26,000
      or more). The report says which form of
      decision 5 was built and, if the copy, the compiler's message that stopped take and
      restore.
9. Hostile trees: `getRootNode`, `contains`, `compareDocumentPosition`, `isEqualNode` and
   `textContent` end on the runner's thread without overflowing its stack, each `Ran`.
   a. Over a chain of 500 nested `div` (the parser's depth is 512 with `html` and `body` in it,
      `limits.rs:64`): `deepest.getRootNode() === document`; `document.contains(deepest)` true;
      `deepest.compareDocumentPosition(document)` is 10 (contains and preceding);
      `body.textContent` is the innermost text.
   b. Over two chains of 250 `div` with one text each: `a.isEqualNode(b)` true, and false when
      the texts differ.
   c. Over `<p>` with 100,000 `<i>a</i>` siblings: `p.textContent.length` is 100,000;
      `last.getRootNode() === document`; `last.compareDocumentPosition(first)` is 2 and
      `first.compareDocumentPosition(last)` is 4.
10. Other slices and the old strings.
    a. A member of another slice still throws `NotSupportedError` naming the member
       (`document.appendChild`, `el.getAttribute`, `el.querySelector`).
    b. `git grep -n -F -e 'Window.document is not supported' -e 'Window.event is not supported'
       -- crates/script/tests` prints nothing.
11. The crate `surf-html` (`crates/html/tests/tree_state.rs`).
    a. `quirks_mode()` is `Quirks` for a page without a doctype, `LimitedQuirks` and `NoQuirks`
       for the doctypes of the parser's own three tests, and `NoQuirks` for a fragment parse's
       tree.
    b. `writes()` is raised by a parse, and by each public write function of the unit's write
       surface that changes a child list, and left as it was by each that does not: one table,
       one case per function, each asserting "raised" or "left as it was" on a successful write
       (`tree.rs:962-1561`); a refused write may leave it.
       - Raised: `insert_before`, `append_child`, `replace_child` and `move_before` (a node that
         is already at that place is the "left as it was" case of each), `remove_node` and
         `adopt_node` of a node with a parent.
       - Left as it was: `create_element_ns`, `create_text_node`, `create_comment`,
         `create_processing_instruction`, `create_document_fragment`; `replace_data`, `set_data`,
         `append_data`, `insert_data`, `delete_data`; `set_attribute`, `set_attribute_ns`,
         `remove_attribute`, `remove_attribute_ns`, `toggle_attribute`.
       - One case per side: `remove_node` and `adopt_node` of a node with no parent (left);
         `clone_node` and `import_node` of a node with no child (left) and a deep clone of one
         with children (raised); `normalize` of a parent with two adjacent texts (raised) and
         with none (left); `split_text` of a text with a parent (raised) and with none (left);
         `set_text_content` on an element with a child, or with a non-empty value (raised), on an
         empty element with the empty value, on a text node and on a document (left).
    c. Two trees that differ only in `writes()` compare equal; two that differ in the mode do not.
    d. The unit's tests, which compare a hooked tree with a plain parse, still pass.
12. The repository's rules.
    a. The tests are the modules of the one test binary of their crate (`mod dom_reading;` in
       `crates/script/tests/main.rs`, `mod same_object;` in `crates/webidl/tests/main.rs`,
       `mod tree_state;` in `crates/html/tests/main.rs`), with no new `[[test]]` entry;
       `slot task test-layout` exits 0.
    b. `slot task webidl-check` exits 0 (the header of `bindings.rs` holds the new sha256 of
       `selection.txt`, `generate.rs`, `prelude.rs` and of the file).
    c. `slot task webidl-slow` regenerates `bindings.rs` byte for byte.
    d. The four `TestWrap` implementations compile unchanged: `git diff --stat` names no change
       in `crates/script/tests/adversary_3.rs`, `adversary_4.rs` or `adversary_5.rs`, and
       `crates/script/tests/bindings.rs` changes only as the Change says.
    e. `slot task check` exits 0; `git diff --stat` names only the files of the Change.
13. The housekeeping edits of the Change.
    a. `git grep -n -A8 'name = "surf-script"' -- Cargo.lock` holds `"surf-url",` in the entry,
       and `git grep -n -F 'surf-url' -- crates/script/Cargo.toml` prints the dependency line.
    b. `slot task lock-check` exits 0 with no line added to `taskfiles/lock-allow.txt` or
       `taskfiles/lock-owners.txt`.
    c. `git grep -n -F 'No DOM yet' -- crates/script/src/lib.rs` prints nothing, and
       `git grep -n -i -F 'no DOM yet' -- AGENTS.md` prints nothing.
    d. `git grep -n -F 'surf-url' -- docs/design/scripts.md` prints the line 1102 names, and
       `git grep -n -A6 -F '`crates/script` (`surf-script`' -- AGENTS.md` prints `surf-url`; the
       same entry's test count ("29 tests" at `6bd9bc9`, with its per-file counts) equals the
       tests of `crates/script` in the `slot task check` log of the tip, and
       `git grep -n -F '29 tests' -- AGENTS.md` prints no line of that entry.
14. Public surfaces: the story changes, in its last commit and after merging `main`, every
    public surface its change makes false, each number taken from the `slot task check` log of
    its tip.
    a. The old strings are read first, after the last merge of `main` into the unit's branch: a
       command run on the story's base before any change,
       `git grep -n -e 'passed' -e 'test files' -e 'tests:' -- README.md website/docs docs/badges`,
       whose output the report records. After `story:html-script-hook` merges, its counts are
       the base's, so the lines given below are the values of `main` at `6bd9bc9` and not
       the strings to look for.
    b. After the last commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges`
       prints nothing for each old string 14a recorded, and each new number equals the one in
       the `slot task check` log of the tip.
    c. The values of `6bd9bc9`: the test count `2272 passed` (`README.md:11`,
       `docs/badges/make.sh:38`, `docs/badges/tests.svg:1`, `:2` and `:10`); `2,275` and
       `2,272` (`README.md:285`, `website/docs/what-works.md:17`; `2,272` also
       `website/docs/status.md:29`); `1,723, and 552` (`README.md:285`,
       `website/docs/what-works.md:17`); `1,720 passed` (`website/docs/status.md:29`); the test
       line `1723 tests: 1720 passed, 0 failed, 3 ignored` (`README.md:626`,
       `website/docs/how-it-is-built.md:70`); the test-layout line `223 test files`
       (`README.md:612`, `website/docs/how-it-is-built.md:56`).
    d. The gate block, `README.md:596-645` and `website/docs/how-it-is-built.md:40-89` (lines of
       `6bd9bc9`), holds the log's lines.
    e. The paragraph that names the gate run on `5dcedfb` (`README.md:674-678`,
       `website/docs/how-it-is-built.md:119-123`) gains this story's sentence and keeps the
       bindings unit's.
    f. The epic's correction (decision 16): `git grep -n -F 'Corrections of 2026-11-25' --
       .engineering/planning/epic/scripts-in-a-page.md` prints one line.
15. The report holds, as text: the rows of Q18 (`docs/design/scripts.md:1275-1281`) read at the tag
    `crates/script/Cargo.toml` pins when the work starts, each marked unchanged or changed; the
    measured numbers of 8; the output of 14a.
16. The folder's bounds hold.
    a. The test step ends within 120 s and no test runs over 60 s.
    b. Each case makes its own runner, which the bindings unit's test
       `a_realm_with_the_bindings_stays_within_its_heap` (`tests/bindings.rs:673`) measured at
       5.9 to 16.4 ms, and the report gives the time of `dom_reading` as a whole; a test over 5 s
       is named with its time.
    c. No model call and no network request in a test; no Python in anything that stays in the
       repository; no generator with a committed output is added (the existing one gains a
       source: its time is measured before and after, both in the report).
    d. A build or test job in the pool ends within 10 minutes or names its time with
       `slot --timeout`, at most 30 minutes, and is not started again unchanged; every compiling
       or testing command carries `slot` (`slot cargo run -p surf-webidl --bin bindings`); a
       script that measures or waits starts without the `slot` prefix.
17. The review's rules (decision 17), in the modules `adversary_dom_reading` of
    `crates/script/tests`, `adversary_tree_state` of `crates/html/tests` and
    `adversary_same_object` of `crates/webidl/tests`.
    a. After a refill that failed, for each of the shapes frozen, sealed, non-extensible and
       non-configurable index: the getter throws a `TypeError`, and `c.length`, `c.item(i)` and
       `c[i]` agree for every `i` of the old list. After a failed refill and a later one to a
       shorter list, no index at or past `length` answers a node. A setter a script put at an
       index is not called by a refill.
    b. `getElementsByClassName` of 5,000 wanted tokens against elements of 5,000 class tokens
       ends `Ran` under the wall clock of 1 s, with steps that grow with the bytes scanned.
    c. One test fails when the step charge of a walking member is removed, for each of
       `getRootNode`, `isConnected`, `compareDocumentPosition`, `isEqualNode`, `children`,
       `wholeText`, the sibling members and `baseURI`; `wrapper_limit` fails when a wrapper is
       not kept; `collection_limit` fails when making a collection charges no step.
    d. A `base` whose `href` is a `data:` or a `javascript:` URL gives the document's address as
       `baseURI`.
    e. 5,000 children walked from both ends in turn end `Ran`, with the steps under 15 times
       those of 500 children.
    f. `lookupNamespaceURI("xml")` on a doctype and on a fragment is null.
    g. `substringData` cut inside a surrogate pair answers the lone half (`charCodeAt(0)` is the
       half's unit, not 0xFFFD).
    h. An empty fragment inserted leaves `Tree::writes()`; over random sequences of the public
       writes the counter moves if and only if a child list changed.
    i. The generator's run on a selection with a `[SameObject]` collection attribute of the
       global ends with an error that names the row.

## Out of scope

Writes (the three other slices); the hook and the parse that runs scripts
(`story:html-script-hook`, `story:scripts-parse-run`, which builds the `DocumentInfo` of a page
and passes it to `dom::slices`); the check of a Content Security Policy `base-uri` directive, the
creator's base URL of an `about:blank` or `about:srcdoc` document and the encoding of the `href`
query (the three deviations of `baseURI`, decision 13); a page in a session; the limit words of
`Limit::InputBytes` and `Limit::Steps` (the first story with a member that writes gives them);
changes to the collection members of `bindings.rs` beyond decision 5.

## Placement

`depends_on: story:html-script-hook`, besides `story:webidl-bindings` (on `main`). Two reasons.
The shared files: the unit (active, not on `main`) writes `crates/html/src/lib.rs`, `tree.rs`,
`tree_builder.rs`, `tree_builder/modes.rs`, `crates/html/tests/main.rs`,
`crates/script/tests/bindings.rs` (one comment line) and `docs/design/scripts.md` (11 insertions
and 11 deletions, at lines 85, 88, 194, 313-318, 327 and 889, none at line 1102), all of which
this story also writes (`git diff --stat main...impl/html-script-hook` lists 22 files at the tip
`58d720b`); its acceptance 10 also
edits, in its last commit and not yet on the branch, the public surfaces this story edits
(`html-script-hook.md:147-153`). And the cases that need it: acceptance 6 runs through
`parse_document_with_hook`, and the test slice of decision 14 calls the unit's write surface.
Every other case of this story needs no hook, which keeps the epic's "with no hook" true for
them. Every `crates/html` line cited here is a line of `main` at `6bd9bc9` unless it says "unit";
the implementor reads them again on the base of the unit's merge and says which moved.

The other html stories that write `tree.rs` and `tree_builder.rs` (`story:html-tree-template`,
`-foreign`, `-formatting`, `-body`, `-tables`, `-complete`) are implemented on `main` at `2a5a715`
(`aep plan artifact show`), so no wave orders them against this one; no story other than the
hook's, not implemented, has `tree.rs`, `tree_builder.rs`, `lib.rs` or `modes.rs` of `crates/html`
in its scope.

`story:swell-0-9-0`: no edge. The two are built in separate trees and merged one after the other.
They write `AGENTS.md`, `Cargo.lock`, `crates/script/Cargo.toml`, `crates/script/tests/bindings.rs`
and `docs/design/scripts.md`, and both rewrite the public surfaces (`README.md`,
`docs/badges/make.sh`, `docs/badges/tests.svg`, `website/docs/how-it-is-built.md`,
`website/docs/status.md`, `website/docs/what-works.md`). Whichever merges second merges `main`
first and takes its numbers from its own log, as the Placement of `story:swell-0-9-0` says for
this pair. This story adds the line `surf-url` right after
`surf-secret` in `crates/script/Cargo.toml` and in the lock; swell's story changes the pin on the
next line.

The generator, `selection.txt` and `bindings.rs` are on `main` since `6bd9bc9`: this story changes
them, and nothing else of the epic changes them in the same wave. This story edits `runner.rs`
only where `ScriptHost` is made (the Change); `story:swell-0-9-0` edits other lines of the file
and the second merge keeps both. `crates/script/src/lib.rs` and
`runner.rs` are written after it by `story:scripts-parse-run`, ordered by that story's edge; that
story builds the `DocumentInfo` of a page (address as a `surf_url::Url`, the response's content
type, the name of the encoding the parse decodes with) and calls `dom::slices(info)`, and
decides which encoding name it passes when the runner is made while the encoding is tentative.

`story:scripts-dom-selectors` inherits, from the merge of this story: the one object per node
and member that `Wrap` now holds for `[SameObject]` (its own members that return a collection
use the same methods or the default), the `DocumentInfo` and `slices(info)` signature, the
quirks mode on `Tree` (the class selector and `getElementsByClassName` match by it), the
dependency of `surf-script` on `surf-url`, and the regenerated `bindings.rs` with its digests.
If that story changes `selection.txt` it must list `selection.txt` and `bindings.rs` in its
scope and regenerate the file; its present scope lists neither.

## Why `epic:scripts-in-a-page`

The first members with behaviour, the reading the epic's two examples rest on.
