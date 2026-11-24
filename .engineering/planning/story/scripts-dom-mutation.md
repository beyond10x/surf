---
format: aep.planning-md/3
id: story:scripts-dom-mutation
kind: story
status: draft
title: The mutating members (46 rows), four setters and four constructors
summary: Tree mutation of Node, Element, Document and the interfaces they return, with the DOM Standard's exceptions, a write over a parser limit ending the script through the generated limit result, and host bytes charged to the agent; the case names asserted equal to the slice's rows.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:html-script-hook
- depends_on: story:scripts-dom-reading
- depends_on: story:scripts-parse-run
scope:
- confidence: cited
  path: crates/script/src/dom/mutation.rs
- confidence: cited
  path: crates/script/tests/dom_mutation.rs
revision: 14
---
## Context

The epic's ninth candidate (lines 463-466) and the slice "Tree mutation, 29" (lines 695-700):
`Node`'s `normalize`, `cloneNode`, `insertBefore`, `appendChild`, `replaceChild`, `removeChild`;
`Element`'s `insertAdjacentElement`, `insertAdjacentText` and the four mutating members of
`ParentNode` and of `ChildNode`; `Document`'s seven `create...` (`createCDATASection` throws
`NotSupportedError` in an HTML document, as the standard says), `importNode`, `adoptNode` and
`ParentNode`'s four; and the setters of `nodeValue` and `textContent`.

`story:webidl-bindings` lists this slice's rows in `crates/webidl/selection.txt`: those 29, the 17
of the returned interfaces, 46 member rows; four setters (`nodeValue`, `textContent`,
`CharacterData.data`, and `Document.title` of S6, decided a by the operator on 2026-11-24); and
the constructors of `Text`, `Comment`, `DocumentFragment` and `ProcessingInstruction`. Every
trait method answers `DomResult`, whose `DomEnd::Limit` ends the script with the parser limit it
reached (decision 9 of that story). Every write goes through the public write surface of
`story:html-script-hook`, in `crates/html/src/tree.rs`, which stays that story's: if this story
finds an operation it needs missing there, it adds it to that file and names it in its report.
The end words are the note's (Q4's column `outcome when reached`, Q7's closed list of ends);
where the note spells a word otherwise than this text, the note's spelling holds. The parse
that runs a page's scripts is `story:scripts-parse-run`'s, which this story uses in its tests.

## Change

- `crates/script/src/dom/mutation.rs`:
  - the 29 members and the setters of `nodeValue` and `textContent`;
  - the 17 of the returned interfaces: `CharacterData` `appendData`, `insertData`, `deleteData`,
    `replaceData`, `before`, `after`, `replaceWith`, `remove`; `Text` `splitText`;
    `DocumentType` `before`, `after`, `replaceWith`, `remove`; `DocumentFragment` `prepend`,
    `append`, `replaceChildren`, `moveBefore`;
  - the setters of `CharacterData.data` and `Document.title`;
  - the constructors of `Text`, `Comment`, `DocumentFragment` and `ProcessingInstruction`;
  each with the DOM Standard's exceptions (`HierarchyRequestError`, `NotFoundError`,
  `InvalidCharacterError`, `NotSupportedError`) as `DomEnd::Throw`, and a write over a parser
  limit answered as `DomEnd::Limit`. The bytes a write adds to the tree are charged to the agent
  through `Cx::charge_host_bytes` (`crates/swell/src/host/mod.rs:577 @ 0.8.0`) and credited
  when they leave it (requirement 3, as revised).
- `crates/script/tests/dom_mutation.rs`: the module `dom_mutation`, declared by
  `story:webidl-bindings`.
- In the last commit, the public surfaces of acceptance 7.

## Acceptance

1. A named case per member row, setter and constructor of the slice, each comparing the tree
   after the script with what the DOM Standard says, and each exception case named by the
   exception it throws; a test asserts that the set of case names equals the rows of
   `selection.txt` with the slice `mutation`: 46 member rows, 4 setters (`setter: mutation`) and
   4 constructors.
2. `title_written`: `surf_script::run::parse_with_scripts` (`story:scripts-parse-run`) over
   `<title>Before</title><script>document.title = "Written by script"</script>` answers a tree
   whose title text is `Written by script`, and its list of ends holds exactly one end, `Ran`.
3. A script that appends to `body`, then markup after its end tag: the appended node and the
   later markup are both in the tree, in the order the standard gives.
4. `node_limit_ends_script`: a script that creates and appends elements past the node limit of
   its `ParseOptions` ends `limit: <the node limit's word in Q4>`, also inside
   `try { ... } catch {}`; parsing goes on and the rest of the page is in the tree.
5. `host_bytes_end_memory_budget`: under a memory limit of 4 MiB and parser limits above what it
   holds, `const s = "x".repeat(100000); for (;;) document.body.append(s);` ends
   `limit: memory`, not at the parser's stored-bytes limit. If the tag lacks the host
   bytes of requirement 3, the case is `#[ignore = "HOST-<id>"]` beside the deviation line.
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
   call and no network request in a test; each case parses a few lines and makes one realm; no
   Python; no generator with a committed output is added; a build or test job in the pool ends
   within 10 minutes or names its time with `slot --timeout`, at most 30 minutes; a test over 5 s
   is named.

## Out of scope

Attributes and selectors (their slices); the parse that runs scripts and its per-document limits
(`story:scripts-parse-run`); a session (`story:scripts-document-port`).

## Placement

After `story:html-script-hook`, `story:scripts-dom-reading` and `story:scripts-parse-run`; beside
`story:scripts-dom-attributes` and `story:scripts-dom-selectors` (each writes its own file).

## Why `epic:scripts-in-a-page`

Writes to the tree, without which no script changes what surf shows (example 1 of the epic).
