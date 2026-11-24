---
format: aep.planning-md/3
id: story:scripts-dom-reading
kind: story
status: proposed
title: The reading members (98 rows), the Window global and one wrapper per node
summary: Node, Element and Document's 73 reading members, 21 of the interfaces they return and the four of S6 (decided a on 2026-11-24) read a parsed tree as the standards say, with one wrapper per node for the document's life and the snapshot and global-this deviations, the case names asserted equal to the slice's rows.
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
- confidence: cited
  path: crates/script/src/lib.rs
- confidence: cited
  path: crates/script/tests/dom_reading.rs
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:13Z", actor: "human:operator", revision: 14}
---
## Context

The epic's eighth candidate (lines 458-462); "Behaviour in the first version, by slice", tree
reading (lines 683-691); the deviations (lines 716-721); requirement 4 as the engine's session
answered it (line 821): surf keeps one wrapper per node strongly for the life of the document,
bounded by the parser's node limit. S6 is decided (option a, the operator, 2026-11-24):
`Window`'s `document`, `window`, `self` and `Document`'s `title` getter come from HTML's IDL.

`story:webidl-bindings` generates the trait of the slice `reading`, the context type `DomCx` and
the trait `Wrap` with a stand-in that throws, and lists this slice's rows in
`crates/webidl/selection.txt`: the 73 of the four interfaces, 21 of the returned interfaces, and
the 4 of S6, 98 member rows, and the row `NodeList iterable<Node>`.

## Change

- `crates/script/src/dom/reading.rs`, the members of the slice:
  - the 73 of `Node`, `Element` and `Document` (`Node`'s 18 constants, its 14 attribute getters
    and `getRootNode`, `hasChildNodes`, `isEqualNode`, `isSameNode`, `compareDocumentPosition`,
    `contains`, `lookupPrefix`, `lookupNamespaceURI`, `isDefaultNamespace`; `Element`'s 15;
    `Document`'s 17, as the epic lists them, lines 683-691);
  - the 21 of the returned interfaces: `CharacterData` `data` (getter), `length`,
    `substringData`, `previousElementSibling`, `nextElementSibling`; `Text` `wholeText`,
    `assignedSlot`; `ProcessingInstruction` `target`; `DocumentType` `name`, `publicId`,
    `systemId`; `DocumentFragment` `getElementById`, `children`, `firstElementChild`,
    `lastElementChild`, `childElementCount`; `NodeList` `item`, `length` and its iteration;
    `HTMLCollection` `length`, `item`, `namedItem`;
  - `Document.title`'s getter (HTML).
- `crates/script/src/dom/wrapper.rs`: the implementation of `Wrap`, one wrapper per node, kept for
  the document's life, its host data naming the node of the `surf-html` tree; the 9 members that
  return a live collection return a snapshot with indexed own properties, and `childNodes` and
  `children` keep one snapshot object refreshed when read (`[SameObject]` as the selection names
  it).
- `crates/script/src/dom/window.rs`: the `Window` global with `document`, `window` and `self`; the
  global `this` is the `Window` (no `WindowProxy`, a named deviation).
- `crates/script/src/lib.rs`: a document over a parsed `Tree` and its address, on which a source
  is evaluated with the runner of `story:scripts-engine-dependency`.
- `crates/script/tests/dom_reading.rs`: the module `dom_reading`, declared by
  `story:webidl-bindings`.
- In the last commit, the public surfaces of acceptance 7.

## Acceptance

1. A named case per member row of the slice, each evaluating a script over a parsed tree and
   comparing what it returns with what the DOM Standard (or, for the four of S6, the HTML
   Standard) says, the section cited in the case's comment; a test asserts that the set of case
   names equals the rows of `selection.txt` with the slice `reading`: 98 member rows (73, 21 and
   4) and the `iterable<Node>` row.
2. `a.parentNode === a.parentNode`, and the same for every member that returns a node; a property
   a script set on a wrapper is read back while its node is in the tree.
3. Each deviation of the slice is tested as the selection names it: a snapshot does not change
   when the tree would; `childNodes` returns the same object twice; an element's wrapper answers
   `Object.prototype.toString.call(a)` as the selection says (if the tag lacks requirement 9, the
   case is `#[ignore = "HOST-<id>"]` beside its deviation line); `this === window` and
   `globalThis === window` at the top level of a script.
4. `document.title` reads the first `title` element's text, stripped and collapsed, as
   `surf.document.Document.title` is computed (`spec/domains/document.yaml`, field `title`).
5. A member of another slice still throws `NotSupportedError` naming the member.
6. `slot task check` exits 0; `git diff --stat` names only the files of the Change.
7. Public surfaces: the story changes, in its last commit and after merging `main`, every
   public surface its change makes false, each number taken from the `slot task check` log of
   its tip. After that commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges`
   prints nothing for each old string below, as the story's base holds it (the strings and lines
   given are those of `main` at `29945f5`, 2026-11-24); each new number equals the one in that log;
   and the gate block, `README.md:584-625` and `website/docs/how-it-is-built.md:40-81`, holds that
   log's lines. The old strings of this story: the test count `1976 passed` (`README.md:11`,
   `docs/badges/make.sh:38`, `docs/badges/tests.svg`), `1,976` and `1,424, and 552`
   (`README.md:285`, `website/docs/status.md:23`, `website/docs/what-works.md:17`).
8. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; no model
   call and no network request in a test; each case parses a document of a few lines with
   `surf_html::parse_document` and makes one realm (the realm's time measured and reported; if over
   50 ms, cases are grouped into table tests so fewer realms are made); no Python; no generator
   with a committed output is added; a build or test job in the pool ends within 10 minutes or
   names its time with `slot --timeout`, at most 30 minutes; a test over 5 s is named.

## Out of scope

Writes (the three other slices); the hook and the parse that runs scripts
(`story:html-script-hook`, `story:scripts-parse-run`); a page in a session.

## Placement

After `story:webidl-bindings`. `crates/script/src/lib.rs` is written after it by
`story:scripts-parse-run`, ordered by that story's edge.

## Why `epic:scripts-in-a-page`

The first members with behaviour, the reading the epic's two examples rest on.
