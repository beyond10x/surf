---
format: aep.planning-md/3
id: story:webidl-reader
kind: story
status: active
title: surf-webidl reads the vendored IDL Index of the DOM Standard and of Web IDL into a model, refusing every construct it does not know
summary: The DOM and Web IDL commit snapshots the coordinating agent fetched on 2026-11-24 are vendored with SOURCE and digests; a reader of their IDL Index builds a model; a test asserts 146 members and 2 constructors of the four interfaces and 75 members and 5 constructors of the interfaces they return (the epic said 68), and the extended-attribute counts.
relations:
- decomposes: epic:scripts-in-a-page
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/webidl
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:34:04Z", actor: "human:operator", revision: 15}
- {from: "proposed", to: "active", at: "2026-11-24T10:34:09Z", actor: "human:operator", revision: 16}
---
## Context

The epic's second candidate (lines 415-420) and its section "The DOM from Web IDL" (lines
605-772): the DOM of the first version is generated from Web IDL (S4, option b), so a reader of
Web IDL comes first. It needs no decision of `story:scripts-decisions` and no swell: the sources
are pinned by the epic (lines 614-620), and the selection that picks from the model is
`story:webidl-bindings`'s.

The tree on `main` of 2026-11-24: nothing of Web IDL exists (`git ls-tree --name-only main
crates/` lists no `webidl`). The form of vendored standard data is
`crates/html-conformance/data/whatwg/` (`entities.json` and a `SOURCE` file with address, fetch
date, the standard's date, licence, copyright, what is generated from it and the sha256 of each
file) with `* -text` in `crates/html-conformance/data/.gitattributes`. The root
`Cargo.toml:3` holds `members = ["crates/*", "generated/rust/crates/*"]`, so the new crate joins
the workspace with no line changed there (the epic, line 419, names a `members` line; the tree
wins). A crate has one integration-test binary, `tests/main.rs` with modules, `autotests =
false` and one `[[test]]` named `main` (`AGENTS.md:276-287`; `task test-layout` holds it).

## The files handed over

The coordinating agent fetched both pages, each a commit snapshot, on 2026-11-24 at 09:57 UTC and
hands them to the implementor unchanged. The address each was fetched from, for the `address:`
line of `SOURCE` (both answered 200 with `text/html; charset=utf-8`):

- `dom.html`: `https://dom.spec.whatwg.org/commit-snapshots/e4f4f4f0ace83fe6c52ab68e047351ce7f2dcd69/`
- `webidl.html`: `https://webidl.spec.whatwg.org/commit-snapshots/54a5f2de6a34e27a517a49370bc07d8943b0be11/`

| file | commit (read in the file) | bytes | sha256 | saved as |
|---|---|---|---|---|
| the DOM Standard, "Commit Snapshot — Last Updated 15 November 2026" (`dom.html` line 411) | `e4f4f4f0ace83fe6c52ab68e047351ce7f2dcd69` (`dom.html` line 410) | 2,720,882 | `80cf04abd88405c6c1f0ec7c28a0a53634df5fdd55e8833c6b6fe68511ca2aa6` | `crates/webidl/data/whatwg/dom.html` |
| the Web IDL Standard, "Commit Snapshot — Last Updated 14 November 2026" (`webidl.html` line 532) | `54a5f2de6a34e27a517a49370bc07d8943b0be11` (`webidl.html` line 531) | 2,553,003 | `3915f58aa923a43fd03dd6fee040bbf8468856ef51270071b0a8bf11378658fe` | `crates/webidl/data/whatwg/webidl.html` |

What the decomposer read from them, for this story to assert (shell commands over the handed
files, run 2026-11-24; the story's own reader replaces them):

- `dom.html` line 17412 is the `h2` with `id="idl-index"`; one `pre` follows it, and its text
  (`awk 'NR>=17413 && NR<=18085' dom.html`, tags removed and `&lt;`, `&gt;`, `&quot;`,
  `&amp;` decoded) is 673 lines, the last one empty, as the epic counted (line 617).
- `webidl.html` line 18713 is that `h2`; its `pre` text is 58 lines and holds `QuotaExceededError`,
  `QuotaExceededErrorOptions`, three `typedef`s (one with `[AllowShared]` on a union member),
  `DOMException` and the callbacks `Function` and `VoidFunction`.
- The table of the epic (lines 631-637) holds: `EventTarget` 3 operations and a constructor;
  `Node` 18 constants, 14 attributes, 15 operations; `Element` 36 own and 17 from its mixins;
  `Document` 28 own and 15 from its mixins: 146 members and 2 constructors.
- The interfaces they return (epic lines 644-649) differ in one row: `ProcessingInstruction`
  (extract lines 460-473) has a constructor and 8 members, `target` and the seven operations
  `hasAttributes`, `getAttributeNames`, `getAttribute`, `setAttribute`, `removeAttribute`,
  `toggleAttribute`, `hasAttribute`, where the epic counts 1 member and no constructor. So the
  returned interfaces hold 75 members, not 68: `CharacterData` 13 with its mixins, `Text` 3,
  `Comment` 0, `ProcessingInstruction` 8, `DocumentType` 7, `DocumentFragment` 11, `NodeList` 2,
  `HTMLCollection` 3, `DOMException` 28 (3 attributes, 25 constants); and 5 constructors (`Text`,
  `Comment`, `ProcessingInstruction`, `DocumentFragment`, `DOMException`). A constructor and
  `iterable<Node>` are not counted as members.
- Extended attributes of the DOM index, by `grep -o "[[ ,]<name>\b" | wc -l` over the extract:
  `CEReactions` 49, `Exposed` 37 (36 on definitions, one on the member `AbortSignal.timeout`,
  extract line 92), `NewObject` 27, `SameObject` 11, `Unscopable` 8, `LegacyNullToEmptyString` 5,
  `PutForwards` 1, `LegacyUnenumerableNamedProperties` 2 (`HTMLCollection` at line 162 and
  `NamedNodeMap` at line 414), `LegacyUnforgeable` 1, `Replaceable` 1, `EnforceRange` 1. The epic
  (lines 670-675) has `Exposed` 36 and `LegacyUnenumerableNamedProperties` once.
- `grep -c 'Promise<'` over the extract prints 0; `TrustedType` appears on lines 373 and 374
  (`setAttribute`, `setAttributeNS`).

## Decisions taken

1. The two sources are vendored unchanged. The reader takes the IDL from the section "IDL Index"
   of each page: the text content of each `pre` element after the `h2` element whose `id` is
   `idl-index` and before the next `h2`, parsed with `surf-html` (`surf_html::parse_document_bytes`,
   the repository's own parser, a path dependency of a crate that ships nothing).
2. The reader reads every definition of both indexes whole, also those no binding will be
   generated for, so the model is the whole index and the selection of `story:webidl-bindings`
   picks from it. It knows the constructs of the epic's list (lines 654-669) and those the other
   definitions of the two indexes use (among them `static`, `any`, `unsigned long long`,
   `unrestricted double`, `double`, a bare `stringifier`, an extended attribute on a type, a
   `required` dictionary member, `callback` with a variadic `any`, an identifier escaped with `_`
   such as `_any` at line 93 of the DOM index). The first run over the vendored
   files names each construct it refuses with its file and line; each one so named that is part of
   the IDL grammar of the vendored Web IDL Standard is added, listed in the crate's documentation
   and in a constant `ADDED` of `src/` (the report lists them). The names of the extended
   attributes it knows are a constant `EXTENDED_ATTRIBUTES` of `src/`. It always refuses, naming
   file and line: `Promise<T>`, an operation name twice in one interface (an overload),
   `namespace`, `maplike`, `setlike`, `async iterable`, `legacycaller`, an extended attribute not
   in `EXTENDED_ATTRIBUTES`, and every construct neither index uses.
3. The reader resolves no name outside the model: a type that names no definition of the two
   indexes (`EventHandler`, `DOMHighResTimeStamp`, `TrustedType`, ...) is kept as an unresolved
   name with its line, and the model lists them. The list for the two vendored indexes is
   committed as `crates/webidl/tests/unresolved.txt`, one name per line, sorted.
4. HTML's definitions (S6) are not read here: `story:webidl-bindings` vendors them and adds the
   selective extraction they need.
5. The model of the vendored files is built once per test run: one test function reads both pages
   and asserts every count over the one model. Every other test reads an IDL text of a few lines
   written in the test.
6. The vendored files have no generated product here, so there is no digest header and no lane
   `slow` step. A test runs `sha256sum --check --strict` over the sha256 lines of `SOURCE` (a
   child process of a few milliseconds), so a changed file fails the gate.
7. Where the vendored files give another count than the epic, the test asserts the files' count
   (above), and the epic's number is listed as a correction in the report.

## Change

- `crates/webidl/` (new; package `surf-webidl`, `publish = false`; the standard library and
  `surf-html` by path, nothing else; `#![forbid(unsafe_code)]`; `autotests = false` and one
  `[[test]]` `main`):
  - `data/whatwg/dom.html`, `data/whatwg/webidl.html` (the files handed over), `data/whatwg/SOURCE`,
    `data/.gitattributes` (`* -text`). `SOURCE` holds, per file, the lines `file:`, `address:`,
    `commit:`, `last-updated:`, `fetched:` (2026-11-24 09:57 UTC), `licence:` (CC BY 4.0, WHATWG
    IPR policy), `copyright:`, `index-lines:`, and a note that the generated bindings carry the
    BSD 3-Clause notice in their header as `crates/html/src/charref_table.rs:1-31` does; at its end
    the `sha256sum` lines of both files;
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
    - types: the epic's list (lines 666-668) and what decision 2 adds, unresolved names kept;
    - the merged view: an interface with its partials and the members of each mixin it includes,
      a mixin's member counted on each interface that includes it (epic line 628);
  - public items: `read(text, file) -> Result<Vec<Definition>, ReadError>` (`ReadError` names
    file, line and construct), `read_page(bytes, file)`, `Model::new`, `Model::counts(interface)
    -> Counts` (own members, members from mixins, constants, attributes, operations, whether it
    has a constructor; constructors and `iterable` are not members),
    `Model::extended_attribute_counts`, `Model::unresolved`;
  - `tests/main.rs` with the modules `reader`, `refusals`, `source` and `vendored`, and
    `tests/unresolved.txt`.
- `Cargo.lock`: the entry of `surf-webidl`, without a `source` line.

- In the last commit, the public surfaces of acceptance 12.

No binary: the binary `bindings` belongs to `story:webidl-bindings`.

## Acceptance

1. `source::sources_match_their_digests`: `sha256sum --check --strict` over the lines of `SOURCE`
   passes; on a copy of the directory under `CARGO_TARGET_TMPDIR` with one byte of `dom.html`
   changed, it fails naming `dom.html`.
2. `source::source_names_each_file`: `SOURCE` holds for each file the lines `file:`, `address:`,
   `commit:`, `last-updated:`, `fetched:`, `licence:`, `copyright:` and `index-lines:`, none empty,
   and each `commit:` equals the commit named in its file's title (`dom.html` line 410,
   `webidl.html` line 531). Outside the gate, at review, the
   coordinating agent's `sha256sum` of a fresh fetch of each address equals its `SOURCE` line.
3. `vendored::the_indexes_read_whole`: both pages read with no `ReadError`; the DOM index
   extracts to 673 lines and the Web IDL index to 58 (each equal to its `index-lines:`); the set of
   `Model::unresolved` equals `tests/unresolved.txt` and holds `EventHandler`,
   `DOMHighResTimeStamp` and `TrustedType`, each asserted by name; and from `Model::counts`: `EventTarget` 3 own,
   0 from mixins, 0 constants, 0 attributes, 3 operations, a constructor; `Node` 47, 0, 18, 14, 15,
   none; `Element` 36, 17, 0, 18, 35, none; `Document` 28, 15, 0, 15, 28, a constructor; together
   146 members and 2 constructors.
4. In the same test: `Element` includes `ParentNode` (10 members), `NonDocumentTypeChildNode`
   (2), `ChildNode` (4) and `Slottable` (1); `Document` includes `NonElementParentNode` (1),
   `DocumentOrShadowRoot` (1), `ParentNode` (10) and `XPathEvaluatorBase` (3); the writable
   attributes of the four are `nodeValue`, `textContent`, `id`, `className` and `slot`.
5. In the same test, the returned interfaces: `CharacterData` 13, `Text` 3 and a constructor,
   `Comment` 0 and a constructor, `ProcessingInstruction` 8 and a constructor, `DocumentType` 7,
   `DocumentFragment` 11 and a constructor, `NodeList` 2 and `iterable<Node>`, `HTMLCollection` 3,
   `DOMException` (from the Web IDL page) 3 attributes, 25 constants and a constructor: 75 members
   and 5 constructors.
6. In the same test, the extended attributes of the DOM index: `CEReactions` 49, `Exposed` 37,
   `NewObject` 27, `SameObject` 11, `Unscopable` 8, `LegacyNullToEmptyString` 5,
   `LegacyUnenumerableNamedProperties` 2, and `PutForwards`, `LegacyUnforgeable`, `Replaceable` and
   `EnforceRange` once each. A count of cases 3 to 6 that the reader finds different from these
   stops the story; the implementor reports the value and the command, and the coordinating agent
   corrects this story before the test changes.
7. `reader`, named cases, one per item of the model in the Change: `definition_interface`,
   `definition_interface_inherits`, `definition_partial_interface`, `definition_interface_mixin`,
   `definition_includes`, `definition_callback_interface`, `definition_callback`,
   `definition_dictionary` (inheritance, defaults, `required`), `definition_enum`,
   `definition_typedef`, `member_const`, `member_attribute`, `member_readonly_attribute`,
   `member_stringifier`, `member_operation`, `member_static_operation`, `member_constructor`,
   `member_getter`, `member_iterable`, `argument_optional_defaults` (`= {}`, `= false`, `= ""`,
   `= null`, `= 0xFFFFFFFF`), `argument_variadic_union`, `type_nullable`, `type_sequence`,
   `type_union`, `type_extended_attribute`, `extended_attribute_arguments`,
   `merged_mixin_counted_on_each`, `merged_partial`; and one case per entry of `ADDED`: the
   module holds a table `ADDED_CASES` of pairs, an entry of `ADDED` and the function that tests
   it, one test runs every function of the table, and it asserts that the table's entries equal
   `ADDED`.
8. `refusals`, named cases: `promise_type`, `overload`, `namespace`, `maplike`, `setlike`,
   `async_iterable`, `legacycaller`, `unknown_extended_attribute` (a name outside
   `EXTENDED_ATTRIBUTES`), each failing with `<file>:<line>: ...` naming the construct and the line
   it starts on; and `newer_index`: the extracted DOM index with one more line
   `Promise<undefined> x();` inside `interface Node` fails naming that line.
9. `slot task check` exits 0; `slot task lock-check` passes with `surf-webidl` in `Cargo.lock`
   without a `source` line.
10. `git diff --stat` of the story names only `crates/webidl/`, `Cargo.lock` and the files of
    acceptance 12.
11. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s
    (nextest's `slow-timeout` of 60 s with `terminate-after = 1`, `.config/nextest.toml:5`); no
    model call and no network request in a test, which reads only the vendored files and texts it
    holds; the parsed model of the vendored pages is made once per test run (decision 5), never
    per test; no Python; the crate generates no committed output, so it has no digest header and
    no lane `slow` step (decision 6); each test's time is in the report and one over 5 s is named;
    every build or test job in the pool ends within 10 minutes or names its time with `slot
    --timeout`, at most 30 minutes.
12. Public surfaces: the story names every public surface its change makes false (a badge, a
    `README.md` line, a website page, a count) and changes it in its last commit, every number
    from the log of a gate run on its tip, which the coordinating agent orders. For this story:
    lines on `main` at `29945f5` (2026-11-24): the gate block of `README.md:584-625` and of
    `website/docs/how-it-is-built.md:40-81`, the test count of `README.md:11` and `:285-286`, and
    `docs/badges/make.sh:38` with `docs/badges/tests.svg`.

## Out of scope

The selection, the generator and the generated bindings (`story:webidl-bindings`); HTML's IDL
(S6); behaviour of any member; comparing the vendored index with web-platform-tests'
`interfaces/dom.idl` (epic lines 625-626, not checked, a later story if wanted).

## Placement

Starts today, beside `story:scripts-decisions` (which writes only `docs/design/scripts.md`); no
edge between them, since the selection is no longer the note's. `Cargo.lock` is also written by
`story:scripts-engine-dependency`; whichever merges second takes the other's lock entries on its
merge (cargo writes the lock; nothing is edited by hand). The public surfaces of acceptance 12
are written by every story of the epic that adds tests: never two of them in one wave's last
commits at once; the coordinating agent orders the gate runs.

## Why `epic:scripts-in-a-page`

The DOM a page's script sees is generated from Web IDL (S4, b); the reader is the first part of
that path and the epic's second candidate.
