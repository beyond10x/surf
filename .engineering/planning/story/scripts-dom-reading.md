---
format: aep.planning-md/3
id: story:scripts-dom-reading
kind: story
status: proposed
title: The reading members (99 rows) and one wrapper per node
summary: Node, Element and Document's 73 reading members, 21 of the interfaces they return and the five of Window and Document.title (S6, decided a on 2026-11-24, and Window.event) read a parsed tree as the standards say, with one wrapper per node for the document's life, memory bounded by the engine's budget, the snapshot and global-this deviations, the [SameObject] collections held by the generator (the same object per node and member), the document's address as a surf_url::Url in a public DocumentInfo, compatMode from the tree's quirks mode and baseURI as the HTML document base URL, the case names asserted equal to the slice's rows.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:webidl-bindings
- depends_on: story:html-script-hook
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Cargo.lock
- confidence: inferred
  path: crates/html/src
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
  path: crates/script/tests/bindings.rs
- confidence: cited
  path: crates/script/tests/dom_reading.rs
- confidence: cited
  path: crates/webidl/selection.txt
- confidence: cited
  path: crates/webidl/src
- confidence: cited
  path: crates/webidl/tests
- confidence: cited
  path: docs/design/scripts.md
revision: 21
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:13Z", actor: "human:operator", revision: 14}
---
## Context

The epic's eighth candidate (`.engineering/planning/epic/scripts-in-a-page.md:464-468`); "Behaviour
in the first version, by slice", tree reading (lines 716-724); the deviations (lines 749-754);
requirement 4 as the epic's "Answers of the script engine's session" give it (line 854): surf
keeps one wrapper per node strongly for the life of the document. S6 is decided (option a,
line 603): `Window`'s `document`, `window`, `self` and `Document`'s `title` getter come from
HTML's IDL.

`story:webidl-bindings` is on `main` (merge `6bd9bc9`). What it left to this story, read from the
code at `6bd9bc9`:

- The slice `reading` has 99 member rows in `crates/webidl/selection.txt`, and one more row,
  `NodeList iterable` (line 236), which is no member. The 99 are `Node` 41 (its 18 constants
  included), `Element` 15, `Document` 18 (`title` included), the 21 members of the returned
  interfaces, and `Window` 4: `window`, `self`, `document` (lines 272-274) and `event`
  (line 278). `Window.event` is a row of the bindings unit: outside the dispatch of an event its
  value is `undefined` and surf dispatches none, so `undefined` is the standard's value (comment
  at lines 275-277). As the story counted them: 73 (`Node` 41, `Element` 15, `Document` 17), 21,
  and 5 (`Window`'s four and `Document.title`), which make 99.
- The trait `Reading` (`crates/script/src/bindings.rs:2051-2708`) holds 81 methods: the 99 rows
  less the 18 constants, which are generated. Every default answers `NotSupportedError` naming
  the member. `crates/script/src/dom/reading.rs:8` is an empty `impl`.
- The `Window` global exists. `Runner::new` makes the global as a host object of `window_spec`
  (`crates/script/src/runner.rs:574-579`), and `install` (`runner.rs:857-880`) defines the
  interface objects and the global's members in one `with_cx` after `create_realm`.
  `run_document` (`runner.rs:431`) and `Document { tree, limits, slices }` (`runner.rs:421-425`)
  exist. `Window.window` and `Window.self` answer `TheWindow` (`bindings.rs:1182`), which the
  glue turns into the global object (`bindings.rs:1176`), but their trait methods
  (`bindings.rs:2689-2697`) still throw; so do `document` and `event` (`bindings.rs:2699-2707`).
- The `Document` of `runner.rs` holds no address, no content type, no encoding and no compat mode,
  and the generated `DomState` (`bindings.rs:151-161`) has no field for one. The members `URL`,
  `documentURI`, `baseURI`, `compatMode`, `characterSet`, `charset`, `inputEncoding` and
  `contentType` need them. `surf_html::Tree` holds none (`crates/html/src/tree.rs:195-200`). The
  parser decides the quirks mode in `pub(crate) enum QuirksMode`
  (`crates/html/src/tree_builder.rs:64-68`), keeps it in the crate-private `Parsed`
  (`tree_builder.rs:316`, returned at `:427-431`) and `parse_document` returns the tree alone
  (`crates/html/src/lib.rs:230-237`); `ParsedDocument` carries only the encoding
  (`lib.rs:212-219`). The lines cited in `crates/html` are those of `main` at `6bd9bc9`.
- `surf-script` depends on `surf-html`, `surf-secret` and `swell`
  (`crates/script/Cargo.toml:14-16`).
  `surf-url` is a crate of this workspace with no dependency (`crates/url/Cargo.toml:13`) and
  `Url::parse(input, base)` and `Url::href()` are public (`crates/url/src/lib.rs:209, 259`).
  `taskfiles/lock-check.sh` fails only for a package from outside the repository that is not on
  `taskfiles/lock-allow.txt`, or a crate that depends on a listed package or on `surf-script`
  without a line of `taskfiles/lock-owners.txt` (`lock-check.sh:69-76`, `:388`); `surf-url` is
  neither, so
  a dependency of `surf-script` on it changes nothing in `taskfiles/` and adds one line,
  `"surf-url",` between `"surf-secret",` and `"swell",`, to the entry of `surf-script` in
  `Cargo.lock` (lines 225-231). Nothing else fixes the dependencies of `surf-script`:
  `webidl-check` reads the lock for the packages `surf-html` and `surf-webidl` only
  (`taskfiles/webidl.yml:21-25`), and no file of `taskfiles/`, `Taskfile.yml`, `.github` or
  `conformance/tests` names `surf-script` but the lock check and its owners file. The crate-graph
  sentence of `docs/design/scripts.md:1102` and the entry of `crates/script` in `AGENTS.md:365-368`
  name its dependencies and say so.
- `crates/script/src/dom/wrapper.rs:13-23` is a `Wrap` whose `wrap` throws `NotSupportedError`;
  its `unwrap` is complete (it calls `wrapper_node`, `bindings.rs:360`). `new_wrapper`
  (`bindings.rs:345`) makes a wrapper of an interface with `NodeData { node }`
  (`bindings.rs:283`). `dom::slices()` (`crates/script/src/dom/mod.rs:23-30`) builds the unit
  structs of the four slices and `SurfWrap`.
- The generated getters of a collection call the slice for a `Vec<NodeId>` and make a new host
  object each time (`collection_value`, `bindings.rs:1132-1160`: one step per node, one wrapper
  per node through `Wrap`, an own index property each). So the slice cannot keep an object, and
  `a.childNodes === a.childNodes` is false as generated (`bindings.rs:3663-3677`).
  `HTMLCollection.length` and `NodeList.length` are `reading` rows, and `for (x of collection)`
  throws `NotSupportedError` until they have behaviour: `NodeList` and `HTMLCollection` have
  `@@iterator` from `%Array.prototype.values%` and `NodeList` has `entries`, `keys`, `values` and
  `forEach` (`bindings.rs:1459-1477`).
- Three tests of `crates/script/tests/bindings.rs` read `Window.document` or `Window.event`
  through `dom::slices()` and expect `NotSupportedError`:
  `a_member_of_a_slice_throws_until_its_slice` (line 495),
  `the_strings_of_a_dom_exception_outlive_a_collection` (line 563) and
  `an_uncaught_dom_exception_names_its_name_and_message` (line 739).
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
   `Interface` (`bindings.rs:1665-1680`). An element's wrapper is an `Element` whatever its name
   (`selection.txt:150-151`, the deviation of the epic, line 754).
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
   - The table's own bytes are surf's and not charged; they are at most the number of wrappers
     times the size of an entry.
5. A collection a member returns is the bindings' own snapshot (`collection_value`): the slice
   answers the nodes in tree order and the glue wraps each, charging one step per node
   (`bindings.rs:1139-1140`). `length` answers the snapshot's length, `item(index)` the node at
   `index` or `null`, `namedItem(name)` the first element of the snapshot that has the ID `name`
   or is in the HTML namespace and has a `name` attribute of that value, and `null` for the
   empty string or no such element (DOM Standard, `HTMLCollection`, `namedItem`). The iteration
   methods are generated (`bindings.rs:1459-1477`) and need no code.
6. No reading member records a limit. The members read the tree and never write it, so none
   answers `DomEnd::Limit` and none calls `end_with_limit` (`bindings.rs:445`); `Limit::InputBytes`
   and `Limit::Steps` keep their placeholder word `limit: wall-clock` (`runner.rs:690-693`), which
   the first story with a member that writes into the tree replaces. A reading member ends a
   script only through the engine: `limit: steps-per-script` or `limit: memory`, or a throw.
7. Steps and copies. The glue charges one step per call (`bindings.rs:3609` and the like). A member
   whose work grows with the tree charges one step per node it visits (as `collection_value` does,
   `bindings.rs:1140`) and one per 64 bytes it builds (as `Copies::copy` does,
   `bindings.rs:561`), before the work. These members are `childNodes` and `children`, the three
   `getElementsBy...`, `textContent`, `wholeText`, `title`, `isEqualNode`, `contains`,
   `compareDocumentPosition`, `getRootNode`, `isConnected`, `lookupPrefix`,
   `lookupNamespaceURI`, `isDefaultNamespace`, `previousSibling`, `nextSibling`,
   `previousElementSibling`, `nextElementSibling` and `childElementCount`; `Tree::descendants`
   allocates a list of every node (`crates/html/src/tree.rs:455-463`) and is charged by its
   length. A String a member returns is made by `Cx::new_string`, which the engine charges, and
   one beyond `string_length` (16,777,216, `runner.rs:46`) is a `RangeError`
   (`docs/design/scripts.md:307`). A reading member calls no `to_utf16` and no
   `to_rust_string_lossy`: its String arguments reach it as Rust strings copied by
   `Copies::copy` (`bindings.rs:553`), with a lone surrogate written U+FFFD (`selection.txt:24`).
8. The pin and the requirements. `crates/script/Cargo.toml:16` pins swell `0.8.0`, and
   `docs/design/scripts.md:1275-1281` records requirements 4, 9 and 10 at `0.8.0` (row 9,
   `%Array.prototype.values%`, `@@iterator`, `held`, line 1280). If the tag has moved when this
   story is implemented (`story:swell-0-9-0`), the implementor reads the rows of Q18 at the new
   tag before the first case and reports each row that changed.
9. The deviations of the slice are those the selection names and the epic lists
   (lines 749-754): the nine members that return a snapshot for a live collection
   (`selection.txt:55, 113-115, 118, 147-149, 167`); `childNodes` and `children` as one snapshot
   object refreshed when read (`[SameObject]`); an element's wrapper is an `Element`; the global
   `this` is the `Window`, no `WindowProxy` (`selection.txt:272-273`). `DocumentFragment.children`
   (`selection.txt:223`) returns a snapshot of an `[SameObject]` attribute
   (`crates/webidl/data/whatwg/dom.html:17526`, `DocumentFragment includes ParentNode` at
   `:17541`) and its row names neither deviation: it is the tenth snapshot row, and this story
   gives it both notes.
10. `[SameObject]` is held by the generator, not by a slice. The attributes of the selected
    definitions that are `[SameObject]` and return a `NodeList` or an `HTMLCollection` are four
    rows of `reading`: `Node childNodes` (`selection.txt:55`), `Element children` (`:118`),
    `Document children` (`:167`) and `DocumentFragment children` (`:223`), from `Node` and the
    mixin `ParentNode` (`dom.html:17526`, `:17640`, `:17541`). The other `[SameObject]`
    attributes of those definitions are rows of `throws` and have no getter to generate
    (`Document.implementation`, `Element.classList`, `Element.attributes`). The getter of each of
    the four asks `Wrap` for the one object of its node and member, with the nodes the slice
    answered; `SurfWrap` makes it on the first read and on each later read replaces its nodes and
    index properties with the new snapshot and answers the same object. The generator knows the
    attribute from the IDL (`ExtendedAttribute` on the member, `crates/webidl/src/model.rs:20`;
    `SameObject` is one of the names the reader accepts, `crates/webidl/src/lib.rs:122`); today
    `Return::Collection` writes the one closure that calls `collection_value`
    (`crates/webidl/src/generate.rs:227-229`) and `Wrap` has `wrap` and `unwrap` only
    (`crates/webidl/src/prelude.rs:206-211`). `Wrap` gains a provided method for the one object
    of a node and member, whose default makes a new snapshot as `collection_value` does today, so
    the four `TestWrap` implementations (`crates/script/tests/bindings.rs:190`,
    `adversary_3.rs:237`,
    `adversary_4.rs:255`, `adversary_5.rs:125`) compile unchanged; `SurfWrap` overrides it. The
    generator's own sources are in the header's digests (`bindings.rs:40-61`), among them
    `selection.txt`, `generate.rs` and `prelude.rs`, so the regenerated file has new digests for
    those three and for itself. The note of
    `DocumentFragment children` becomes the note of the other three: `deviation: a snapshot for a
    live collection until swell's slice 9 (HOST-112, HOST-114); deviation: [SameObject] as the
    same snapshot object refreshed when read`.
11. The compat mode comes from the tree. `surf-html` makes the mode its parser decides public and
    `Tree` holds it, so `Document.compatMode` is `BackCompat` for the mode "quirks" and
    `CSS1Compat` otherwise (DOM Standard, `dom.html:8121-8123`), and `getElementsByClassName`
    matches class names ASCII case-insensitively in "quirks" mode (`dom.html:7903-7906`). The
    caller does not give it.
12. The document's address is a `surf_url::Url` in a public `DocumentInfo` with the content type
    and the encoding name (`Strings`), given to `dom::slices(info)` and held by `SurfReading`.
    `DocumentInfo::default()` is `about:blank`, `text/html` and `UTF-8`, so a test that gives no
    address gets `about:blank`. `URL` and `documentURI` answer `Url::href()` (`url/src/lib.rs:259`).
    `surf-script` gains the dependency `surf-url = { path = "../url" }` and its lock line (Context).
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
    What stays out, each a named deviation in the note of the row `Node baseURI`: the check of the
    `base-uri` directive of a Content Security Policy, because surf reads none
    (`docs/design/scripts.md:149`); the creator's base URL that a document at `about:blank` or
    `about:srcdoc` inherits, because surf has no nested document and no creator. The frozen base
    URL is the result of the same parse, taken at each read: it is the value HTML freezes while the
    fallback base URL does not change, and surf's address does not change.

## Change

- `crates/script/src/dom/reading.rs`: the 81 methods of `Reading`, decisions 1 to 7:
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
  The struct `SurfReading` holds a `DocumentInfo` (address as a `surf_url::Url`, content type,
  encoding name) and answers `URL` and `documentURI` with the address (`Url::href()`),
  `contentType`, `characterSet`, `charset` and `inputEncoding` with the others, `compatMode`
  from the tree's mode and `baseURI` by decision 13.
- `crates/script/src/dom/wrapper.rs`: `SurfWrap`, decisions 3, 4 and 10: the table from `NodeId`
  to `Root`, `wrap` answering the one wrapper of a node, made on first use, and the table of the
  one collection object per node and member. `unwrap` stays.
- `crates/script/src/dom/mod.rs`: `pub struct DocumentInfo` (`Default` is `about:blank`,
  `text/html`, `UTF-8`) and `slices(info: DocumentInfo)`, which gives it to `SurfReading`
  (`slices()` has no argument now, `mod.rs:23-30`); `story:scripts-parse-run` builds the
  `DocumentInfo` of a page from its address and its parse. `Document`, `DomState` and the
  generated `bindings.rs` do not gain a field for it.
- `crates/script/Cargo.toml`: the dependency `surf-url = { path = "../url" }` after `surf-secret`
  (the file's lines 14-16); `Cargo.lock`: `"surf-url",` in the entry of `surf-script`, between
  `"surf-secret",` and `"swell",`. Nothing of `taskfiles/` changes (Context).
- `crates/script/src/lib.rs`: the head comment, which says "No DOM yet" (line 5).
- `[SameObject]` (decision 10), in the generator's files and their outputs:
  - `crates/webidl/src/generate.rs` and `crates/webidl/src/prelude.rs`: the getter of a
    `[SameObject]` collection asks `Wrap` for the one object of its node and member, and `Wrap`
    gains the provided method;
  - `crates/webidl/selection.txt`: the rows `Node childNodes` (line 55), `Element children`
    (118) and `Document children` (167) keep their notes; `DocumentFragment children` (223)
    gains both notes; `Node baseURI` (48) gains `deviation: no Content Security Policy
    base-uri check; deviation: no creator's base URL for about:blank and about:srcdoc`;
  - `crates/webidl/tests`: a case that the generated getters of the four rows call `Wrap`'s new
    method and the three `getElementsBy...` getters, which return a new object per call, do not;
  - `crates/script/src/bindings.rs`: regenerated with `cargo run -p surf-webidl --bin bindings`;
    its header's digests for `selection.txt`, `generate.rs`, `prelude.rs` and its own last line
    change, and `slot task webidl-check` and the lane `slow` (`webidl-slow`) pass.
- `crates/html/src` (decision 11): `QuirksMode` becomes public and `Tree` holds the mode the
  parser decided, with a public read of it; the parser sets it when it finishes
  (`tree_builder.rs:64-68`, `:316-321`, `:427-431`). The lines are those of `6bd9bc9`; the unit
  `story:html-script-hook` writes the same three files first (Placement).
- `docs/design/scripts.md`: line 1102 (`surf-script` depends on `surf-html` for the tree) gains
  `surf-url` for the address.
- `crates/script/tests/dom_reading.rs`: the module `dom_reading`, declared in `tests/main.rs`.
- `crates/script/tests/bindings.rs`: the three tests of the Context change from `Window.document`
  and `Window.event` to a member of another slice that still throws (for example
  `document.appendChild`), so each keeps its subject.
- `AGENTS.md`: the entry of `crates/script` (lines 365-368: "no DOM yet", its dependencies, which
  name swell and `surf-secret` only, and the count of tests).
- In the last commit, the public surfaces of acceptance 9.

## Acceptance

1. A named case per member row of the slice, each a script run over a document parsed with
   `surf_html::parse_document`, with `dom::slices(info)`, comparing what it returns with what
   the DOM
   Standard (the HTML Standard for `Window` and `Document.title`) says,
   the section cited in the case's comment. A test, `dom_reading::cases_are_the_rows`, reads
   `crates/webidl/selection.txt` as `bindings::every_member_row_has_its_property` does
   (`tests/bindings.rs:258-275`) and asserts that the set of case names equals the 99 member rows
   with the slice `reading` and the `NodeList iterable` row: `Node` 41, `Element` 15, `Document` 18,
   the 21, `Window` 4 and the one iterable row.
2. Identity: `a.parentNode === a.parentNode`, the same for every member that returns a node,
   `document.documentElement === document.getElementById("a").ownerDocument.documentElement`, and a
   property a script set on a wrapper is read back while its node is in the tree, also after a
   `collect()` of the agent (`runner.agent_and_realm().0.collect()`).
3. Collections: `for (x of document.childNodes)`, `[...nodes]`, `nodes.length`,
   `nodes.item(i)`, `nodes[i]`, `nodes.forEach`, `HTMLCollection.namedItem` by `id` and by `name`,
   and the same node object from a collection and from a direct member
   (`document.body.childNodes[0] === document.body.firstChild`).
4. Each deviation of the slice is tested as the selection names it: a snapshot does not change when
   the tree would; `a.childNodes === a.childNodes` and the same for the three `children`
   (`Element`, `Document`, `DocumentFragment`), the second read showing the tree as it is then
   and the old index properties gone; the `getElementsBy...` results stay a new object per call
   (`document.getElementsByTagName("p") !== document.getElementsByTagName("p")`); an element's
   wrapper answers
   `Object.prototype.toString.call(a)` as the selection says (requirement 9 is `held` at `0.8.0`,
   `docs/design/scripts.md:1280`); `this === window`, `globalThis === window`,
   `window.window === window`, `self === window` and `window.document === document` at the top level
   of a script; `window.event === undefined`; the two deviations of `baseURI` (acceptance 5).
5. `document.title` reads the first `title` element's text, stripped and collapsed, as
   `surf.document.Document.title` is computed (`spec/domains/document.yaml:229-233`), and the empty
   string when there is none. The document's facts:
   - `compatMode` follows the tree's mode: `BackCompat` for a page without a doctype (quirks) and
     `CSS1Compat` for `<!DOCTYPE html>` and for a doctype that gives limited-quirks mode; a case
     on each, and `getElementsByClassName("A")` finds `class="a"` in the first and not in the
     second;
   - `contentType`, `characterSet`, `charset`, `inputEncoding`, `URL` and `documentURI` read what
     the `DocumentInfo` holds; `DocumentInfo::default()` gives `about:blank`, `text/html`,
     `UTF-8`, and a test that gives no address reads `about:blank` from `URL`, `documentURI` and
     `baseURI`;
   - `baseURI`, address `https://example.com/dir/page` in each case, one case per line:
     no `base` element: the address; one `<base href="https://other.test/x/">` anywhere in the
     page, also after the script's element: `https://other.test/x/`; two `base` elements with
     `href`: the first in tree order; a first `base` without `href` and a second with one: the
     second; a `base` without `href` alone: the address; an `href` that does not parse
     (a value for which `Url::parse` returns `Err`, taken from the url crate's own failing
     cases): the address; a relative `href` (`../up/`): the `href` parsed against the address
     (`https://example.com/up/`); an `href` over `MAX_INPUT_LENGTH` bytes: the address; a `base`
     in the SVG namespace: ignored;
   - the two deviations are in the note of `Node baseURI` and a case reads them from
     `selection.txt`, as acceptance 1's test reads the rows.
6. A member of another slice still throws `NotSupportedError` naming the member
   (`document.appendChild`, `el.getAttribute`, `el.querySelector`); `git grep -n -F -e
   'Window.document is not supported' -- crates/script/tests` prints nothing.
7. No reading member records a limit or reaches the tree's writers: after a script of every case
   `runner.document().and_then(|dom| dom.limit())` is `None`. Memory and steps. The implementor
   measures first, before any assertion below is written: the bytes per wrapper, per collection
   and per node of a collection, and the steps of each case, recorded in the cases' comments; the
   assertions are the floor and the end named here, not the measured numbers:
   - `wrapper_charge`: after a script that wraps 1,000 distinct nodes of one document,
     `heap_bytes()` has grown by at least 1,000 × 256 bytes over its value before, and the case
     names the number it measured per wrapper and per collection;
   - `wrapper_limit`: under a memory limit of 1 MiB a script that wraps every node of a document of
     100,000 elements ends `limit: memory` and the agent is still usable after the end;
   - `collection_limit`: under the same limit `for (;;) document.getElementsByTagName("*")` ends
     `limit: memory` or `limit: steps-per-script`, the script's end is `ScriptEnd::Limit`, and the
     table of wrappers does not grow beyond the wrappers made;
   - `steps_in_proportion`: `document.getElementsByTagName("*")` over a document of 1,000 elements
     raises `runner.steps()` by at least 1,000.
8. Hostile trees: `getRootNode`, `contains`, `compareDocumentPosition`, `isEqualNode`
   and `textContent` over a chain of the parser's depth, 512 elements (`limits.rs:64`), and over a
   tree of 100,000 siblings, end on the runner's thread without overflowing its stack.
9. Public surfaces: the story changes, in its last commit and after merging `main`, every public
   surface its change makes false, each number taken from the `slot task check` log of its tip.
   After that commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges` prints
   nothing for each old string below, as the story's base holds it (the strings and lines given are
   those of `main` at `6bd9bc9`); each new number equals the one in that log; and the gate block,
   `README.md:596-645` and `website/docs/how-it-is-built.md:40-89`, holds that log's lines. The old
   strings: the test count `2272 passed` (`README.md:11`, `docs/badges/make.sh:38`,
   `docs/badges/tests.svg:1`, `:2` and `:10`); `2,275` and `2,272` (`README.md:285`,
   `website/docs/what-works.md:17`; `2,272` also `website/docs/status.md:29`); `1,723, and 552`
   (`README.md:285`, `website/docs/what-works.md:17`); `1,720 passed` (`website/docs/status.md:29`);
   the test line `1723 tests: 1720 passed, 0 failed, 3 ignored` (`README.md:626`,
   `website/docs/how-it-is-built.md:70`); the test-layout line `223 test files`
   (`README.md:612`, `website/docs/how-it-is-built.md:56`). The paragraph that names the gate run
   on `5dcedfb` (`README.md:674-678`, `website/docs/how-it-is-built.md:119-123`) gains this
   story's sentence and keeps the bindings unit's.
10. The repository's rules: the tests are the module `dom_reading` of the one test binary of
    `surf-script` (`crates/script/tests/main.rs`, `mod dom_reading;` stays in its sorted place),
    with no new `[[test]]` entry; `slot task test-layout` exits 0;
    `slot task webidl-check` exits 0 (the header of `bindings.rs` holds the new sha256 of
    `selection.txt`, `generate.rs` and `prelude.rs`, and of the file) and `slot task webidl-slow`
    regenerates `bindings.rs` byte for byte (the generator and the selection changed);
    `slot task lock-check` exits 0 with `surf-url` among the dependencies of `surf-script` in
    `Cargo.lock` and no line added to `taskfiles/lock-allow.txt` or `lock-owners.txt`;
    `git grep -n -F -e 'surf-url' -- crates/script/Cargo.toml Cargo.lock` prints the two lines.
11. `slot task check` exits 0; `git diff --stat` names only the files of the Change.
12. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; each
    case makes its own runner, which the bindings unit's test
    `a_realm_with_the_bindings_stays_within_its_heap` (`tests/bindings.rs:673`) measured at 5.9 to
    16.4 ms, and the report gives the time of `dom_reading` as a whole; a test over 5 s is named
    with its time; no model call and no network request in a test; no Python in anything that
    stays in the repository; no generator with a committed output is added (the existing one gains
    a source: its time is measured before and after, both in the report); a build or test job in
    the pool ends within 10 minutes or names its time with `slot --timeout`, at most 30 minutes,
    and is not started again unchanged; a script that measures or waits starts without the `slot`
    prefix.

## Out of scope

Writes (the three other slices); the hook and the parse that runs scripts
(`story:html-script-hook`, `story:scripts-parse-run`, which builds the `DocumentInfo` of a page
and passes it to `dom::slices`); the check of a Content Security Policy `base-uri` directive and
the creator's base URL of an `about:blank` or `about:srcdoc` document (deviations of
`baseURI`, decision 13); a page in a session; the limit words of `Limit::InputBytes` and
`Limit::Steps` (the first story with a member that writes gives them).

## Placement

`depends_on: story:html-script-hook`, besides `story:webidl-bindings` (on `main`). The unit
`story:html-script-hook` (not merged) writes `crates/html/src/tree.rs`, `tree_builder.rs` and
`lib.rs`, the files this story changes to make the quirks mode public. Every `crates/html` line
cited here is a line of `main` at `6bd9bc9`; the implementor reads them again on the base of the
unit's merge and says which moved. The generator, `selection.txt` and `bindings.rs` are the
bindings unit's: nothing else of the epic changes them in the same wave.
`crates/script/src/lib.rs` and `runner.rs` are written after it by `story:scripts-parse-run`,
ordered by that story's edge.

`story:scripts-dom-selectors` inherits, from the merge of this story: the one object per node and
member that `Wrap` now holds for `[SameObject]` (its own members that return a collection use
the same method or the default), the `DocumentInfo` and `slices(info)` signature, the quirks
mode on `Tree` (the class selector and `getElementsByClassName` match by it), the dependency of
`surf-script` on `surf-url`, and the regenerated `bindings.rs` and its digests, which it
regenerates again if it changes the selection.

## Why `epic:scripts-in-a-page`

The first members with behaviour, the reading the epic's two examples rest on.
