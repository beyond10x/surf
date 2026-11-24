---
format: aep.planning-md/3
id: story:scripts-dom-mutation
kind: story
status: draft
title: The 29 mutating members, three setters and three constructors, and a parse that runs a document's scripts
summary: Tree mutation with the DOM Standard's exceptions, held to the parser's limits, and the surf-script function that parses a document and runs its scripts at their end tags under the per-document limits.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:html-script-hook
- depends_on: story:scripts-dom-reading
scope:
- confidence: cited
  path: crates/script/src/dom/mutation.rs
- confidence: inferred
  path: crates/script/src/lib.rs
- confidence: inferred
  path: crates/script/src/run.rs
- confidence: inferred
  path: crates/script/tests/dom_mutation.rs
revision: 3
---
## Context

The epic's ninth candidate (lines 463-466) and the slice "Tree mutation, 29" (lines 695-700):
`Node`'s `normalize`, `cloneNode`, `insertBefore`, `appendChild`, `replaceChild`, `removeChild`;
`Element`'s `insertAdjacentElement`, `insertAdjacentText` and the four mutating members of
`ParentNode` and of `ChildNode`; `Document`'s seven `create...` (`createCDATASection` throws
`NotSupportedError` in an HTML document, as the standard says), `importNode`, `adoptNode` and
`ParentNode`'s four; and the setters of `nodeValue`, `textContent` and `document.title` (HTML,
S6 = a), and the constructors of `Text`, `Comment` and `DocumentFragment`. Every write goes
through the public write surface of `story:html-script-hook` and is held to the parser's limits.

This story also joins the hook to swell, because it is the first slice whose script changes the
tree the parser goes on over: a function of `surf-script` parses a document and runs its scripts
at their end tags, one agent per document, with the per-document limits and the wall-clock bound
of the note's Q4, and answers the tree and each script's end. `story:scripts-html-conformance`
and `story:scripts-document-port` both call it.

## Change

- `crates/script/src/dom/mutation.rs`: the members, setters and constructors above, each with the
  DOM Standard's exceptions (`HierarchyRequestError`, `NotFoundError`, `InvalidCharacterError`,
  `NotSupportedError`) as `DOMException`s; a write over a parser limit ends the script as Q4
  says; the bytes of a node charged against the agent's memory through the engine's host bytes
  where the tag has them (requirement 3).
- `crates/script/src/run.rs` and `crates/script/src/lib.rs`: the parse that runs scripts (above).
- `crates/script/tests/dom_mutation.rs`.

## Acceptance

`slot task check` exits 0. The folder's bounds hold: the test step ends within 120 s and no test
runs over 60 s; no model call and no network request in a test; each case parses a few lines and
makes one realm; no Python; no generator with a committed output is added (one that were would carry a digest of its sources, a fast check in the gate and full regeneration in the lane `slow`); a build or test job in the pool ends within 10 minutes or names its time with `slot --timeout`, at most 30 minutes; a test over 5 s is named.

1. A named case per member, setter and constructor (32 members and setters and 3 constructors),
   each comparing the tree after the script with what the DOM Standard says, and each exception
   case named by the exception it throws.
2. The parse that runs scripts, over
   `<title>Before</title><script>document.title = "Written by script"</script>`, answers a tree
   whose title text is `Written by script` and one end `ran`; with scripting disabled it answers
   the tree of today and no end.
3. A script that appends to `body` and then markup after its end tag: the appended node and the
   later markup are both in the tree, in the order the standard gives.
4. A script that creates nodes past the node limit ends with that limit recorded; parsing goes on
   and the rest of the page is in the tree.
5. `git diff --stat` names only the files of the Change.

## Out of scope

Attributes and selectors (their slices); a session (`story:scripts-document-port`).

## Why `epic:scripts-in-a-page`

Writes to the tree, without which no script changes what surf shows (example 1 of the epic).
