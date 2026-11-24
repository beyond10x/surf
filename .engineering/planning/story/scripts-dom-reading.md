---
format: aep.planning-md/3
id: story:scripts-dom-reading
kind: story
status: draft
title: The 73 reading members, the Window global and one wrapper per node
summary: Node, Element and Document read a parsed tree as the DOM Standard says, with the Window global's document, window and self and document.title's getter (S6 = a), one wrapper per node for the document's life and the snapshot deviation, tested member by member.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:webidl-bindings
scope:
- confidence: cited
  path: crates/script/src/dom/reading.rs
- confidence: cited
  path: crates/script/src/dom/window.rs
- confidence: cited
  path: crates/script/src/dom/wrapper.rs
- confidence: inferred
  path: crates/script/src/lib.rs
- confidence: inferred
  path: crates/script/tests/dom_reading.rs
revision: 3
---
## Context

The epic's eighth candidate (lines 458-462); "Behaviour in the first version, by slice", tree
reading (lines 683-691); the deviations (lines 716-721); requirement 4 as the engine's session
answered it (line 821): surf keeps one wrapper per node strongly for the life of the document,
bounded by the parser's node limit. Assumes S6 = a: `Window`'s `document`, `window`, `self` and
`Document`'s `title` getter come from HTML's IDL.

## Change

- `crates/script/src/dom/reading.rs`: the 73 reading members of `Node`, `Element` and `Document`
  (Node's 18 constants, its 14 attribute getters and `getRootNode`, `hasChildNodes`,
  `isEqualNode`, `isSameNode`, `compareDocumentPosition`, `contains`, `lookupPrefix`,
  `lookupNamespaceURI`, `isDefaultNamespace`; Element's 15; Document's 17, as the epic lists them)
  and the reading members the note's Q8 assigns of the returned interfaces (`CharacterData`,
  `Text`, `DocumentType`, `NodeList`, `HTMLCollection`, ...); `document.title`'s getter (HTML).
- `crates/script/src/dom/wrapper.rs`: one wrapper per node, kept for the document's life, its
  host data naming the node of the `surf-html` tree; the 9 members that return a live collection
  return a snapshot with indexed own properties, and `childNodes` and `children` keep one snapshot
  object refreshed when read (`[SameObject]` as the selection names it).
- `crates/script/src/dom/window.rs`: the `Window` global with `document`, `window` and `self`; the
  global `this` is the `Window` (no `WindowProxy`, a named deviation).
- `crates/script/src/lib.rs`: a document over a parsed `Tree` and its address, on which a source
  is evaluated with the runner of `story:scripts-engine-dependency`.
- `crates/script/tests/`: a module `dom_reading`.

## Acceptance

`slot task check` exits 0. The folder's bounds hold: the test step ends within 120 s and no test
runs over 60 s; no model call and no network request in a test; each case parses a document of a
few lines with `surf_html::parse_document` and makes one realm (the realm's time measured and
reported; if over 50 ms, cases are grouped into table tests so fewer realms are made); no Python; no generator with a committed output is added (one that were would carry a digest of its sources, a fast check in the gate and full regeneration in the lane `slow`); a build or test job in the pool ends within 10 minutes or names its time with `slot --timeout`, at most 30 minutes;
a test over 5 s is named.

1. A named case per member of the 73, each evaluating a script over a parsed tree and comparing
   what it returns with what the DOM Standard says (the section cited in the case's comment); the
   test module prints the count of members covered, 73 plus those of Q8 for the returned
   interfaces.
2. `a.parentNode === a.parentNode` and the same for every member that returns a node; a property a
   script set on a wrapper is read back while its node is in the tree.
3. Each deviation is tested as the selection names it: a snapshot does not change when the tree
   would; `childNodes` returns the same object twice; an element's wrapper answers
   `Object.prototype.toString.call(a)` as Q8 says (class string, condition: requirement 9 in the
   tag).
4. `document.title` reads the first `title` element's text, stripped and collapsed, as
   `surf.document.Document.title` is computed (`spec/domains/document.yaml`, field `title`).
5. A member of another slice still throws `NotSupportedError` naming the member.
6. `git diff --stat` names only the files of the Change.

## Out of scope

Writes (the three other slices); the hook (`story:html-script-hook`); a page in a session.

## Why `epic:scripts-in-a-page`

The first members with behaviour, the reading the epic's two examples rest on.
