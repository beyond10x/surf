---
format: aep.planning-md/3
id: story:webidl-reader
kind: story
status: draft
title: surf-webidl reads the vendored IDL Index of the DOM Standard and of Web IDL into a model, refusing every construct it does not know
summary: The two WHATWG commit snapshots the epic pins are vendored with SOURCE and digests; a reader of their IDL Index builds a model; a test counts the 146 members and 2 constructors of EventTarget, Node, Element and Document and the 68 of the interfaces they return.
relations:
- decomposes: epic:scripts-in-a-page
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/webidl
revision: 2
---
## Context

The epic's second candidate (epic lines 415-420) and its section "The DOM from Web IDL" (lines
605-772): the DOM of the first version is generated from Web IDL (S4, option b), so a reader of
Web IDL comes first. It needs no decision of `story:scripts-decisions` and no swell: the sources
are pinned by the epic (lines 614-620).

The tree on `main` of 2026-11-24: nothing of Web IDL exists (`git ls-tree --name-only main
crates/` lists no `webidl`). The form of vendored standard data is
`crates/html-conformance/data/whatwg/` (`entities.json` and a `SOURCE` file with address, fetch
date, the standard's date, licence, copyright, what is generated from it and the sha256 of each
file) with `* -text` in `crates/html-conformance/data/.gitattributes`. The root
`Cargo.toml:3` holds `members = ["crates/*", "generated/rust/crates/*"]`, so the new crate joins
the workspace with no line changed there (the epic, line 419, names a `members` line; the tree
wins). A crate has one integration-test binary, `tests/main.rs` with modules, `autotests =
false` and one `[[test]]` named `main` (`AGENTS.md:276-287`; `task test-layout` holds it).

## Decisions taken

1. The two sources are vendored as the WHATWG publishes them, unchanged: the rendered commit
   snapshots of the DOM Standard and of the Web IDL Standard (table below). The reader takes the
   IDL from the section "IDL Index" of each page: the text content of each `pre` element after
   the `h2` element whose `id` is `idl-index` and before the next `h2`, parsed with `surf-html`
   (`surf_html::parse_document_bytes`, the repository's own parser, a path dependency of a crate
   that ships nothing). Markup and character references are gone in that text; the epic counted
   673 lines for the DOM index this way (line 617).
2. The reader reads every definition of both indexes whole, also those no binding will be
   generated for, so the model is the whole index and the selection of `story:webidl-bindings`
   picks from it. It knows the constructs of the epic's list (lines 654-669) and those the other
   definitions of the two indexes use; the first run names each construct it refuses with its file
   and line, and each one so named that is part of the IDL grammar of the vendored Web IDL
   Standard is added and listed in the crate's documentation (the report lists the additions). It
   always refuses, naming file and line: `Promise<T>`, an operation name twice in one interface
   (an overload), `namespace`, `maplike`, `setlike`, `async iterable`, `legacycaller`, an extended
   attribute whose name is not in its list, and every construct neither index uses. A newer IDL
   then fails the check instead of generating wrong bindings (epic lines 676-679).
3. The reader resolves no name outside the model: a type that names no definition of the two
   indexes (`EventHandler`, `DOMHighResTimeStamp`, `TrustedType`, ...) is kept as an unresolved
   name with its line, and the model lists them. What a generated member does with one is
   `story:webidl-bindings`' decision.
4. HTML's definitions (S6) are not read here. `story:webidl-bindings` vendors them under S6's
   answer.
5. The model of the vendored files is built once per test run: one test function reads both pages
   and asserts every count over the one model. Every other test reads an IDL text of a few lines
   written in the test.
6. The vendored files have no generated product here, so there is no digest header and no lane
   `slow` change. A test runs `sha256sum --check --strict` over the sha256 lines of `SOURCE` (a
   child process of a few milliseconds), so a changed file fails the gate.

## The files the coordinating agent fetches

| file | address | saved as |
|---|---|---|
| the DOM Standard, "Last Updated 15 November 2026" (epic line 614) | `https://dom.spec.whatwg.org/commit-snapshots/e4f4f4f0ace83fe6c52ab68e047351ce7f2dcd69/` | `crates/webidl/data/whatwg/dom.html` |
| the Web IDL Standard, "Last Updated 14 November 2026" (epic line 619) | `https://webidl.spec.whatwg.org/commit-snapshots/54a5f2de6a34e27a517a49370bc07d8943b0be11/` | `crates/webidl/data/whatwg/webidl.html` |

Each is fetched into an empty directory of its own under the session scratch directory with
`curl -fsSL --proto '=https' -o <name> <address>`, its status 200 and media type
`text/html` checked, and copied unchanged; the coordinating agent records the fetch date, the
output of `sha256sum` and of `wc -c` for each, and hands the two files to the implementor. If
either address answers otherwise, it stops and reports: the epic pins these two commits, and no
other source is taken without the operator.

The implementor writes `crates/webidl/data/whatwg/SOURCE` in the form of the html-conformance one:
for each file its address, commit, the standard's "Last Updated" date as the page states it, the
fetch date, the licence (CC BY 4.0, WHATWG IPR policy), the copyright line, that the generated
bindings carry the BSD 3-Clause notice in their header as `crates/html/src/charref_table.rs:1-26`
does, the line count of its extracted IDL Index, and at the end the `sha256sum` lines of both
files; and `crates/webidl/data/.gitattributes` holding `* -text`. Nobody edits a vendored file.

## Change

- `crates/webidl/` (new; package `surf-webidl`, `publish = false`; the standard library and
  `surf-html` by path, nothing else; `#![forbid(unsafe_code)]`; `autotests = false` and one
  `[[test]]` `main`):
  - `data/whatwg/dom.html`, `data/whatwg/webidl.html`, `data/whatwg/SOURCE`,
    `data/.gitattributes`;
  - `src/lib.rs` and its modules: the extraction (decision 1), a lexer of Web IDL's lexical
    grammar (integer, decimal, identifier, string, whitespace, comment, other), the parser of the
    definitions, and the model:
    - definitions: `interface` with inheritance, `partial interface`, `interface mixin` and
      `includes`, `callback interface`, `callback`, `dictionary` with inheritance and default
      values, `enum`, `typedef`; each with its name, file and line, and its extended attributes
      with their arguments;
    - members: `const`, `attribute` and `readonly attribute`, `stringifier`, operations,
      `constructor`, special `getter` operations, `iterable<T>`, and what decision 2 adds; each
      with name, type, line, extended attributes, and its arguments (`optional` with its default,
      variadic);
    - types: the epic's list (lines 666-668: `DOMString`, `USVString`, `boolean`, `unsigned
      short`, `unsigned long`, `undefined`, `object`, nullable, `sequence<T>`, unions, interface
      types) and what decision 2 adds, unresolved names kept (decision 3);
    - the merged view: an interface with its partials and the members of each mixin it includes,
      a mixin's member counted on each interface that includes it (epic line 628);
  - public items: `read(text, file) -> Result<Vec<Definition>, ReadError>` (`ReadError` names
    file, line and construct), `read_page(bytes, file)` for a vendored page, `Model::new`, and
    `Model::counts(interface) -> Counts` with own members, members from mixins, constants,
    attributes, operations and whether it has a constructor, and `Model::extended_attribute_counts`;
  - `tests/main.rs` with the modules `reader`, `refusals` and `vendored`.
- `Cargo.lock`: the entry of `surf-webidl`, without a `source` line.

No binary: the binary `bindings` is `story:webidl-bindings`'.

## Acceptance

`slot task check` exits 0. The folder's bounds hold: the test step ends within 120 s and no test
runs over 60 s (nextest's `slow-timeout` of 60 s with `terminate-after = 1`,
`.config/nextest.toml:5`); no model call and no network request in a test, which reads only the
vendored files and texts it holds; the parsed model of the vendored pages is made once per test
run (decision 5), never per test; no Python; the crate generates no committed output, so it has no digest header and no lane `slow` step (decision 6); each test's time is in the report and one over 5 s is
named. Every build or test job in the pool ends within 10 minutes, or names its time with `slot
--timeout` (at most 30 minutes).

1. `vendored::sources_match_their_digests`: `sha256sum --check --strict` over the lines of
   `SOURCE` passes; on a copy of the directory under `CARGO_TARGET_TMPDIR` with one byte of
   `dom.html` changed, it fails naming `dom.html`.
2. `vendored::the_indexes_read_whole`: both pages read with no `ReadError`; it prints the line
   count of each extracted index (the epic counted 673 for the DOM Standard) and the list of
   unresolved names, and asserts the epic's table (lines 631-637) from `Model::counts`:
   `EventTarget` 3 own, 0 from mixins, 0 constants, 0 attributes, 3 operations, a constructor;
   `Node` 47, 0, 18, 14, 15, none; `Element` 36, 17, 0, 18, 35, none; `Document` 28, 15, 0, 15, 28,
   a constructor; together 146 members and 2 constructors.
3. In the same test: `Element` includes `ParentNode` (10 members), `NonDocumentTypeChildNode`
   (2), `ChildNode` (4) and `Slottable` (1); `Document` includes `NonElementParentNode` (1),
   `DocumentOrShadowRoot` (1), `ParentNode` (10) and `XPathEvaluatorBase` (3); the writable
   attributes of the four are `nodeValue`, `textContent`, `id`, `className` and `slot` (epic lines
   639-642).
4. In the same test, the interfaces they return (epic lines 644-649): `CharacterData` 13 with its
   mixins, `Text` 3 and a constructor, `Comment` 0 and a constructor, `ProcessingInstruction` 1,
   `DocumentType` 7, `DocumentFragment` 11 and a constructor, `NodeList` 2 and `iterable<Node>`,
   `HTMLCollection` 3, `DOMException` (from the Web IDL page) 3 attributes, 25 constants and a
   constructor: 68 members.
5. In the same test, the extended attributes of the DOM index (epic lines 670-675):
   `[CEReactions]` 49, `[Exposed]` 36, `[NewObject]` 27, `[SameObject]` 11, `[Unscopable]` 8,
   `[LegacyNullToEmptyString]` 5, and `[PutForwards]`, `[LegacyUnenumerableNamedProperties]`,
   `[LegacyUnforgeable]`, `[Replaceable]` and `[EnforceRange]` once each.
   A count of cases 2 to 5 that differs from the epic stops the story: the implementor reports the
   measured value, and the coordinating agent corrects the epic before the test is changed.
6. `reader`: one case per construct of the model, each a text of a few lines read into the
   expected definition, member, argument or type (`optional` with each default form of the epic,
   `= {}`, `= false`, `= ""`, `= null`, `= 0xFFFFFFFF`; a variadic union argument; a nullable
   type; a `sequence`; a mixin counted on two interfaces; a partial merged).
7. `refusals`: each refused construct of decision 2 fails with `<file>:<line>: ...` naming the
   construct and the line it starts on; the extracted DOM index with one more line `Promise<undefined>
   x();` inside `interface Node` fails naming that line.
8. `Cargo.lock` lists `surf-webidl` with no `source`
   line, and `slot task lock-check` passes unchanged.
9. `git diff --stat` of the story names only `crates/webidl/` and `Cargo.lock`.

## Out of scope

The selection, the generator and the generated bindings (`story:webidl-bindings`); HTML's IDL
(S6); behaviour of any member; comparing the vendored index with web-platform-tests'
`interfaces/dom.idl` (epic line 625-626, not checked, a later story if wanted).

## Placement

Starts today, beside `story:scripts-decisions` (which writes only `docs/design/scripts.md`).
`Cargo.lock` is also written by `story:scripts-engine-dependency`; whichever merges second takes
the other's lock entries on its merge (cargo writes the lock; nothing is edited by hand). No open
story writes `crates/webidl/`.

## Why `epic:scripts-in-a-page`

The DOM a page's script sees is generated from Web IDL (S4, b); the reader is the first part of
that path and the epic's second candidate.
