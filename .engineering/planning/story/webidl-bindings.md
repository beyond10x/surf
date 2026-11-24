---
format: aep.planning-md/3
id: story:webidl-bindings
kind: story
status: draft
title: The binary bindings generates the DOM's interfaces from the model and the selection, every member throwing NotSupportedError until its slice
summary: selection.txt is the one table of the Web IDL selection (formerly Q8) with every member's slice, S6 = a decided 2026-11-24 with the HTML source and its selective extraction (formerly Q12); generated interfaces, brands, conversions, one trait per slice, the context type, the wrapper hook and the limit result; DOMException; the digest check and regeneration in the lane slow.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:webidl-reader
- depends_on: story:scripts-engine-dependency
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: README.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/script/Cargo.toml
- confidence: cited
  path: crates/script/src/bindings.rs
- confidence: cited
  path: crates/script/src/dom
- confidence: cited
  path: crates/script/src/lib.rs
- confidence: cited
  path: crates/script/tests
- confidence: inferred
  path: crates/webidl/Cargo.toml
- confidence: cited
  path: crates/webidl/data
- confidence: cited
  path: crates/webidl/selection.txt
- confidence: cited
  path: crates/webidl/src
- confidence: cited
  path: crates/webidl/tests
- confidence: cited
  path: docs/badges
- confidence: cited
  path: taskfiles/webidl.yml
- confidence: cited
  path: website/docs/how-it-is-built.md
revision: 16
---
## Context

The epic's sixth candidate (lines 443-451), "The DOM from Web IDL" (lines 605-772) and the
constraint that the bindings are generated, never written by hand (lines 548-552). The model of
`story:webidl-reader` and the runner of `story:scripts-engine-dependency` exist before it. The
lane `slow` exists on `main` (`Taskfile.yml:612-617`; the epic, line 760, calls it new) and gains
a step here.

The coordinating agent moved two questions of `story:scripts-decisions` here (their numbers Q8
and Q12 stay unused in the note): the Web IDL selection, whose one table is
`crates/webidl/selection.txt`, and S6 with the source of HTML's definitions.

S6 is decided: the operator chose option a on 2026-11-24 (relayed by the coordinating agent).
`Window`'s `document`, `window` and `self` and `Document`'s `title` are generated from the HTML
Standard's IDL; the rest of HTML's `Window` and `Document` is not generated, so `document.body`,
`document.write`, `location` and the event handler attributes read `undefined` (epic lines
519-532).

The counts the selection rests on, from the DOM index that `story:webidl-reader` vendors (its
body lists the commands): `EventTarget`, `Node`, `Element` and `Document` hold 146 members and 2
constructors (epic lines 631-642, confirmed); the interfaces they return hold 75 members and 5
constructors, not the epic's 68 (lines 644-650): `ProcessingInstruction` has a constructor and 8
members (`target` and seven attribute operations), where the epic counts 1 and none.

`surf-html` (`crates/html`) exports `Tree` and `NodeId` (`crates/html/src/lib.rs:172`) and the
parser limits as `surf_html::limits::Limit` (`crates/html/src/limits.rs:76-89`: `InputBytes`,
`Depth`, `Nodes`, `Attributes`, `StoredBytes`, `Steps`). swell `0.8.0` ends an entry
`Terminated(Interrupted)` once its `InterruptHandle` is set (`crates/swell/src/host/
interrupt.rs:1-30` at the tag), and a host function returns `Completion<T>`, a value or an
`Abrupt` (`crates/swell/src/host/outcome.rs:41-48`).

Conditions on swell, named and not known today: the tag pinned holds requirement 8
(`Cx::set_prototype_of`), 9 (the closed lists of intrinsics and well-known symbols, with
`%Array.prototype.entries%`, `keys`, `values` and `forEach` for an iterable `NodeList`), 5
(`HostObjectSpec.error_kind`) and 10 (the cost constants). A requirement the tag lacks makes each
member that needs it a named deviation in the selection, with its test, until a later tag;
moving the tag is one line of `crates/script/Cargo.toml`, the lock and the allow-list.

## Decisions taken

1. The selection. `crates/webidl/selection.txt` holds one row per member and one per
   constructor of the selected definitions, `<interface> <member> <slice> [<note>]`, the slice
   one of `reading`, `attributes`, `mutation`, `selectors`, `throws` and `exception`. An
   attribute whose setter belongs to another slice than its getter names it in the note
   (`setter: mutation`; `nodeValue`, `textContent`, `CharacterData.data`, `Document.title`). A
   `throws` row names its reason; a deviation names itself (`deviation: <words>`). `iterable<Node>`
   is one row of `reading`, not counted as a member. The file's head states the grammar; the
   reader of the file is in `crates/webidl/src`.
2. The slices of the 146 members are the epic's (lines 681-715): `reading` 73, `attributes` 14,
   `mutation` 29 and the setters of `nodeValue` and `textContent`, `selectors` 7, `throws` 23 and
   the constructors of `EventTarget` and `Document`.
3. The slices of the 75 members of the returned interfaces, by the same rules:
   - `reading`, 21: `CharacterData` `data` (getter), `length`, `substringData`,
     `previousElementSibling`, `nextElementSibling`; `Text` `wholeText`, `assignedSlot`;
     `ProcessingInstruction` `target`; `DocumentType` `name`, `publicId`, `systemId`;
     `DocumentFragment` `getElementById`, `children`, `firstElementChild`, `lastElementChild`,
     `childElementCount`; `NodeList` `item`, `length`; `HTMLCollection` `length`, `item`,
     `namedItem`;
   - `mutation`, 17, the setter of `CharacterData.data`, and the constructors of `Text`,
     `Comment`, `DocumentFragment` and `ProcessingInstruction`: `CharacterData` `appendData`,
     `insertData`, `deleteData`, `replaceData`, `before`, `after`, `replaceWith`, `remove`;
     `Text` `splitText`; `DocumentType` `before`, `after`, `replaceWith`, `remove`;
     `DocumentFragment` `prepend`, `append`, `replaceChildren`, `moveBefore`;
   - `attributes`, 0;
   - `selectors`, 2: `DocumentFragment` `querySelector`, `querySelectorAll`;
   - `throws`, 7: `ProcessingInstruction` `hasAttributes`, `getAttributeNames`, `getAttribute`,
     `setAttribute`, `removeAttribute`, `toggleAttribute`, `hasAttribute` (reason: attributes of
     a processing instruction);
   - `exception`, 28 and its constructor: `DOMException`'s 3 attributes and 25 constants, whose
     behaviour this story writes, since every `NotSupportedError` is one.
4. The four members of S6: `Window` `document`, `window`, `self` and `Document` `title` (getter),
   slice `reading`; the setter of `title` is `mutation`'s.
5. The totals per slice, members (constructors): `reading` 98 (0), `mutation` 46 and four
   setters (4), `attributes` 14 (0), `selectors` 9 (0), `throws` 30 (2), `exception` 28 (1); 225
   member rows and 7 constructor rows.
6. Deviations, each a note on its row: the 9 members that return a snapshot for a live collection
   (until swell's slice 9); `[SameObject]` on `childNodes` and `children` as the same snapshot
   object refreshed when read; an element's wrapper is an `Element`; `TrustedType` read as
   `DOMString` (`setAttribute`, `setAttributeNS`); the global `this` is the `Window` (no
   `WindowProxy`); and each member a missing requirement of the tag holds back
   (`deviation: HOST-<id>`).
7. The HTML source of S6: a file of the HTML Standard at a commit snapshot, vendored unchanged
   under `crates/webidl/data/whatwg/` with its lines in `SOURCE` as the other two, the smallest
   file of that snapshot that holds the definitions of `Window` and of the partial `Document`
   that has `title`; its size measured with `wc -c` and named in the report. The reader gains a
   selective extraction: from that file it reads only the `pre` blocks of class `idl` that define
   `Window` or a partial `Document`, and from them only the members the selection names; every
   other member of those blocks is listed as skipped with its line, so a construct the reader
   refuses (HTML uses `Promise<T>`) is refused only where it is selected.
8. The context type and the wrapper hook. `bindings.rs` declares `DomCx`, the context every
   generated member receives: the document's `surf_html::Tree`, the parser's `Limits`, the
   document's totals and the record of a limit reached, and the agent's `InterruptHandle`; and
   the trait `Wrap` with `fn wrap(&mut self, cx, node: surf_html::NodeId) -> Completion<Value>`
   and `fn unwrap(&self, cx, value) -> Option<NodeId>`, which every node return and brand check
   calls. This story gives `Wrap` an implementation that throws `NotSupportedError`;
   `story:scripts-dom-reading` writes the real one in `wrapper.rs`.
9. The result of a write over a limit. Every trait method returns `DomResult<T> = Result<T,
   DomEnd>`, `DomEnd` being `Throw(DomException)` (thrown to the script as a `DOMException` of
   that name) or `Limit(surf_html::limits::Limit)`. On `Limit` the generated glue changes nothing
   in the tree, records the limit in `DomCx`, sets the interrupt handle and returns a throw, so
   the entry ends `Terminated(Interrupted)` at swell's next check, even under a `catch`; the
   runner reports the end as "ended by a limit" with that limit, not as an interrupt.
   `story:scripts-dom-mutation` and `story:scripts-dom-attributes` both use it.
10. `crates/script` gains the dependency `surf-html` by path (the context type names its types).

## Change

- `crates/webidl/selection.txt` (new): decisions 1 to 6.
- `crates/webidl/src/`: the reader of the selection, the selective extraction of decision 7, the
  generator, and the binary `bindings` that writes `crates/script/src/bindings.rs`: interface
  objects as host functions (an interface without a constructor throws a `TypeError`), interface
  prototype objects as host objects chained to their parent's and to `%Object.prototype%` at
  `EventTarget`, attributes as accessors and operations as data properties with Web IDL's
  property attributes, constants on the interface object and its prototype, one brand per
  concrete interface and the generated table of which brand inherits which member, Web IDL's
  conversions over swell's `to_string`, `to_number` and the others, `DomCx`, `Wrap`,
  `DomResult` and `DomEnd` (decisions 8 and 9), and one Rust trait per slice (`reading`,
  `attributes`, `mutation`, `selectors`) whose default for every method answers
  `NotSupportedError` naming the member, so that each slice story writes one file. The members
  and constructors of `throws` are generated to throw with their reason and have no trait method;
  those of `exception` are generated with their behaviour. A member's steps charge steps in
  proportion to its work (requirement 3, as revised). The generator refuses a definition the
  selection does not name, and a selected member the model does not hold.
- `crates/webidl/data/whatwg/`: the HTML file of decision 7 and its lines in `SOURCE`.
- `crates/script/src/bindings.rs` (generated): its header names the sha256 of each source (the
  vendored files, the selection, the generator's sources) and carries the WHATWG BSD 3-Clause
  notice as `crates/html/src/charref_table.rs:1-31` does.
- `crates/script/Cargo.toml`, `Cargo.lock`: decision 10.
- `crates/script/src/lib.rs`, `crates/script/src/dom/mod.rs` and one file per slice
  (`reading.rs`, `attributes.rs`, `mutation.rs`, `selectors.rs`), each with an empty `impl` of
  its trait, and `wrapper.rs` with the `Wrap` of decision 8, so every member throws its
  `NotSupportedError` until its slice; the realm's setup defines the interface objects and the
  global's members in a `with_cx` right after `create_realm` (epic lines 310-314).
- `taskfiles/webidl.yml` (new) and `Taskfile.yml`: `webidl-check`, a step of `check`, recomputes
  the digests with `sha256sum` and compares them with the header, generating nothing;
  `webidl-slow`, a step of `slow`, runs `bindings` into `.out/` and compares the bytes with the
  committed file.
- `crates/webidl/tests/`, `crates/script/tests/`: the cases below; `crates/script/tests/main.rs`
  also declares the modules `dom_reading`, `dom_attributes`, `dom_mutation`, `dom_selectors` and
  `parse_run`, each file holding only its head comment, so each later story writes only its own
  test file.
- In the last commit, the public surfaces of acceptance 11.

## Acceptance

1. `slot task webidl-check` passes; with one byte of `selection.txt` changed it fails naming the
   file; `slot task slow` regenerates `bindings.rs` byte for byte.
2. `selection::rows_match_the_model` (in `crates/webidl/tests`): every member and constructor of
   the selected definitions in the reader's model has exactly one row, and every row names a
   member of the model; the counts per slice are those of decision 5 (225 member rows, 7
   constructor rows), and over the four interfaces of the table `reading` 73, `attributes` 14,
   `mutation` 29, `selectors` 7, `throws` 23. A count the model gives otherwise stops the story:
   the implementor reports it with the command, and the coordinating agent corrects this story
   and lists it as a correction of the epic before the test changes.
3. `selection::outside_definition_refused`: a selection naming `Attr` (a definition not
   generated) or a member the model lacks makes the generator fail naming the row.
4. A table test in `crates/script/tests` asserts, for each member row of `selection.txt`, that
   the member exists on its prototype or interface object with Web IDL's property attributes
   (enumerable, configurable, writable as Web IDL says for constants, attributes and
   operations); the number of cases equals the number of rows (225), and `Element.prototype`'s
   [[Prototype]] is `Node.prototype`, `Node.prototype`'s is `EventTarget.prototype`.
5. A member called on an object of another brand, or on an object made by script, throws a
   `TypeError`; `new Node()` throws a `TypeError`.
6. Each of the 30 members of `throws` and `new EventTarget()` and `new Document()` throws a
   `DOMException` named `NotSupportedError` whose message names the member and its reason; a
   member of another slice throws `NotSupportedError` naming the member until its slice lands.
7. `DOMException` is constructible, has its 25 constants and its three attributes, and its
   prototype chain reaches `%Error.prototype%`. If the tag lacks requirement 9, this case is
   `#[ignore = "HOST-<id>"]`, `selection.txt` carries the deviation line of that HOST id, and a
   test asserts that line exists.
8. A realm with the bindings stays within `BINDINGS_REALM_HEAP_BYTES`, the test reading the
   engine's constant; the realm's heap and creation time are in the report. If the tag lacks
   requirement 10, the case is ignored and the deviation line asserted as in acceptance 7.
9. `trusted_type_as_domstring`: with a test implementation of the `attributes` trait,
   `el.setAttribute("a", { toString() { return "x" } })` hands the trait the string `x`; a
   `limit_end` case with a test implementation of `mutation` that answers
   `DomEnd::Limit(Nodes)` ends the script with that limit, `try { ... } catch {}` around the call
   included, and the tree unchanged.
10. The generator is timed before and after each source it gains (the HTML file, each interface
    added to the selection), both times in the report; a run over twice its time before the
    source was added is a defect fixed in this story.
11. Public surfaces: the story names every public surface its change makes false (a badge, a
    `README.md` line, a website page, a count) and changes it in its last commit, every number
    from the log of a gate run on its tip, which the coordinating agent orders. For this story,
    lines on `main` at `29945f5` (2026-11-24): the test count of `README.md:11` and `:285-286` and `docs/badges/make.sh:38` with
    `docs/badges/tests.svg`; the gate block of `README.md:584-625` and of
    `website/docs/how-it-is-built.md:40-81`.
12. `slot task check` exits 0; `git diff --stat` names only the files of the Change.
13. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; no model
    call and no network request in a test; the bindings are generated once, committed, and read
    by every test, never generated in a test, and a realm with them is made per test only if its
    measured time is under 50 ms (else one per test binary process, the time in the report); no
    Python; the generator has a digest of its sources, a fast check in the gate and full
    regeneration in the lane `slow`; a build or test job in the pool ends within 10 minutes or
    names its time with `slot --timeout`, at most 30 minutes; a test over 5 s is named with its
    time.

## Out of scope

Behaviour of any member but `DOMException`'s (the four slice stories); the real `Wrap`
(`story:scripts-dom-reading`); running a page's scripts (`story:scripts-parse-run`); HTML
definitions beyond S6's four members.

## Placement

After `story:webidl-reader` and `story:scripts-engine-dependency`; beside `story:scripts-spec`,
`story:session-process-hardening` and `story:html-script-hook`. `Taskfile.yml` is also written by
`story:scripts-engine-dependency` (ordered by the edge) and by active stories outside the epic:
never in the same wave.

## Why `epic:scripts-in-a-page`

The generated DOM of S4, b: every member exists before any has behaviour.
