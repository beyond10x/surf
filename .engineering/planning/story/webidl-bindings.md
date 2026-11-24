---
format: aep.planning-md/3
id: story:webidl-bindings
kind: story
status: proposed
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
  path: taskfiles/webidl.yml
revision: 24
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:12Z", actor: "human:operator", revision: 19}
---
## Context

The epic's sixth candidate (lines 443-451), "The DOM from Web IDL" (lines 605-772) and the
constraint that the bindings are generated, never written by hand (lines 548-552). The model of
`story:webidl-reader` and the runner of `story:scripts-engine-dependency` exist before it. The lane
`slow` exists on `main` (`Taskfile.yml:612-617`; the epic, line 760, calls it new) and gains a step
here.

The coordinating agent moved two questions of `story:scripts-decisions` here (their numbers Q8 and
Q12 stay unused in the note): the Web IDL selection, whose one table is
`crates/webidl/selection.txt`, and S6 with the source of HTML's definitions.

S6 is decided: the operator chose option a on 2026-11-24 (relayed by the coordinating agent).
`Window`'s `document`, `window` and `self` and `Document`'s `title` are generated from the HTML
Standard's IDL; the rest of HTML's `Window` and `Document` is not generated, so `document.body`,
`document.write`, `location` and the event handler attributes read `undefined` (epic lines
519-532).

The counts the selection rests on, from the DOM index that `story:webidl-reader` vendors (its body
lists the commands): `EventTarget`, `Node`, `Element` and `Document` hold 146 members and 2
constructors (epic lines 631-642, confirmed); the interfaces they return hold 75 members and 5
constructors, not the epic's 68 (lines 644-650): `ProcessingInstruction` has a constructor and 8
members (`target` and seven attribute operations), where the epic counts 1 and none.

`surf-html` (`crates/html`) exports `Tree` and `NodeId` (`crates/html/src/lib.rs:172`) and the
parser limits as `surf_html::limits::Limit` (`crates/html/src/limits.rs:76-89`: `InputBytes`,
`Depth`, `Nodes`, `Attributes`, `StoredBytes`, `Steps`). swell ends an entry
`Terminated(Interrupted)` once its `InterruptHandle` is set, and clears the flag itself when it
ends the entry so (`crates/swell/src/host/interrupt.rs:1-5 @ 0.8.0`); a host function returns
`Completion<T>`, a value or an `Abrupt` (`crates/swell/src/host/outcome.rs:41-48 @ 0.8.0`).

The requirements on swell this story needs: 8 (`Cx::set_prototype_of`), 9 (the closed lists of
intrinsics and well-known symbols, with `%Array.prototype.entries%`, `keys`, `values` and `forEach`
for an iterable `NodeList`), 5 (`HostObjectSpec.error_kind`) and 10 (the cost constants). Each is
`held` at `0.8.0` (`docs/design/scripts.md`, Q18), so no member is a deviation for a missing
requirement; a later tag that lacks one makes each member that needs it a named deviation in the
selection, with its test.

## Decisions taken

1. The selection. `crates/webidl/selection.txt` holds one row per member and one per constructor of
   the selected definitions, `<interface> <member> <slice> [<note>]`, the slice one of `reading`,
   `attributes`, `mutation`, `selectors`, `throws` and `exception`. An attribute whose setter
   belongs to another slice than its getter names it in the note (`setter: mutation`; `nodeValue`,
   `textContent`, `CharacterData.data`, `Document.title`). A `throws` row names its reason; a
   deviation names itself (`deviation: <words>`). `iterable<Node>` is one row of `reading`, not
   counted as a member. The file's head states the grammar; the reader of the file is in
   `crates/webidl/src`.
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
   - `mutation`, 17, the setter of `CharacterData.data`, and the constructors of `Text`, `Comment`,
     `DocumentFragment` and `ProcessingInstruction`: `CharacterData` `appendData`, `insertData`,
     `deleteData`, `replaceData`, `before`, `after`, `replaceWith`, `remove`; `Text` `splitText`;
     `DocumentType` `before`, `after`, `replaceWith`, `remove`; `DocumentFragment` `prepend`,
     `append`, `replaceChildren`, `moveBefore`;
   - `attributes`, 0;
   - `selectors`, 2: `DocumentFragment` `querySelector`, `querySelectorAll`;
   - `throws`, 7: `ProcessingInstruction` `hasAttributes`, `getAttributeNames`, `getAttribute`,
     `setAttribute`, `removeAttribute`, `toggleAttribute`, `hasAttribute` (reason: attributes of a
     processing instruction);
   - `exception`, 28 and its constructor: `DOMException`'s 3 attributes and 25 constants, whose
     behaviour this story writes, since every `NotSupportedError` is one.
4. The four members of S6: `Window` `document`, `window`, `self` and `Document` `title` (getter),
   slice `reading`; the setter of `title` is `mutation`'s.
5. The totals per slice, members (constructors): `reading` 98 (0), `mutation` 46 and four setters
   (4), `attributes` 14 (0), `selectors` 9 (0), `throws` 30 (2), `exception` 28 (1); 225 member
   rows and 7 constructor rows.
6. Deviations, each a note on its row: the 9 members that return a snapshot for a live collection
   (until swell's slice 9; the override table, HOST-112 and HOST-114, is `pending slice 9` at
   `0.8.0`, `docs/design/scripts.md`, Q13); `[SameObject]` on `childNodes` and `children` as the
   same snapshot object refreshed when read; an element's wrapper is an `Element`; `TrustedType`
   read as `DOMString` in its form Q17 gives: the reader reads the whole union
   `(TrustedType or DOMString)` of `setAttribute` and `setAttributeNS` as `DOMString`; the global
   `this` is the `Window` (no `WindowProxy`). Requirements 1 to 10 are `held` at `0.8.0` (Q18), so
   no member is held back by a missing requirement; a later tag that lacks one marks the member
   `deviation: HOST-<id>`.
7. The HTML source of S6 (the operator session's pick of 2026-11-24, option B of three): the
   single-page commit snapshot of the HTML Standard at
   `546d834701d171e3b164da8463d62878deb1e954`
   (`https://html.spec.whatwg.org/commit-snapshots/546d834701d171e3b164da8463d62878deb1e954/`;
   15,266,649 bytes and 212 `pre` blocks of class `idl` when measured on 2026-11-24), vendored
   unchanged under `crates/webidl/data/whatwg/` with its lines in `SOURCE` as the other two,
   held by its address and its sha256. No multipage page exists at a commit snapshot address
   (three addresses answered 404), and `Window` and the partial `Document` that has `title`
   are on two pages (`nav-history-apis.html`, `dom.html`), so no smaller file of the snapshot
   holds both. The implementor measures the file with `wc -c`, names the size in the report,
   and times the reader on it before any test uses it: the file is read once per test run,
   not once per test. The reader gains a
   selective extraction: from that file it reads only the `pre` blocks of class `idl` that define
   `Window` or a partial `Document`, and from them only the members the selection names; every
   other member of those blocks is listed as skipped with its line, so a construct the reader
   refuses (HTML uses `Promise<T>`) is refused only where it is selected.
8. The context type and the wrapper hook. `bindings.rs` declares `DomCx`, the context every
   generated member receives: the document's `surf_html::Tree`, the parser's `Limits`, the
   document's totals and the record of a limit reached, and the agent's `InterruptHandle`; and the
   trait `Wrap` with `fn wrap(&mut self, cx, node: surf_html::NodeId) -> Completion<Value>` and
   `fn unwrap(&self, cx, value) -> Option<NodeId>`, which every node return and brand check calls.
   This story gives `Wrap` an implementation that throws `NotSupportedError`;
   `story:scripts-dom-reading` writes the real one in `wrapper.rs`.
9. The result of a write over a limit. Every trait method returns
   `DomResult<T> = Result<T, DomEnd>`, `DomEnd` being `Throw(DomException)` (thrown to the script
   as a `DOMException` of that name) or `Limit(surf_html::limits::Limit)`. On `Limit` the generated
   glue, one public function `end_with_limit(cx, limit)`, changes nothing in the tree, records the
   limit in `DomCx`, sets the interrupt handle and returns a throw, so the entry ends
   `Terminated(Interrupted)` at swell's next check, even under a `catch`.
   `story:scripts-dom-mutation` and `story:scripts-dom-attributes` both use it.
10. The runner of `story:scripts-engine-dependency` knows nothing of `DomCx`, so this story extends
    it in `crates/script/src/lib.rs` with one rule: when an entry of a document ends
    `Terminated(Interrupted)`, a limit recorded in `DomCx` is the end and the record is cleared,
    otherwise the end is `limit: wall-clock`. The limit words are those of Q4's column
    `outcome when reached` for the parser's limits: `limit: nodes`, `limit: depth`,
    `limit: attributes`, `limit: stored-bytes`; this story invents none.
11. `crates/script` gains the dependency `surf-html` by path (the context type names its types).
12. Copies out of the engine (Q4's rule for surf's bindings): the host interface's reads of a
    String's units, `Cx::to_utf16` and `Cx::to_rust_string_lossy`
    (`crates/swell/src/host/mod.rs:505-513 @ 0.8.0`, over `scoped_to_utf16` at `:205 @ 0.8.0`),
    copy without a charge. So a generated binding converts every argument (Web IDL's conversions
    may run script) before it copies any String out of the engine, or charges the copy with
    `Cx::charge_host_bytes` (`crates/swell/src/host/mod.rs:577 @ 0.8.0`) before it makes it and
    drops the `HostBytes` with the copy: no uncharged copy of surf's is alive while script runs.
    The generator emits every copy through one function of `bindings.rs`, which counts the
    uncharged copies alive, so a test can read the count.
13. What the DOM a script makes is charged (Q4, requirement 3): each wrapper is a host object on
    the engine's heap, `HOST_OBJECT_LIVE_BYTES` (256) bytes plus its data's size
    (`crates/swell/src/host/limits.rs:80-83 @ 0.8.0`), counted by `heap_bytes()`; and for each node
    a script creates, `DomCx` charges with `Cx::charge_host_bytes`, before the write, the node's
    term of the tree's heap bound, 3 × `NODE_BYTES` (408 bytes) plus twice the bytes it stores
    (`crates/html/src/limits.rs:36-42`), and keeps the `HostBytes` until the agent is dropped.
    `DomCx` offers that charge as one function, which `story:scripts-dom-mutation` and
    `story:scripts-dom-attributes` call before each write that makes a node or stores bytes. So a
    script that makes nodes ends at `limit: memory` or at a parser limit, whichever comes first.

## Change

- `crates/webidl/selection.txt` (new): decisions 1 to 6.
- `crates/webidl/src/`: the reader of the selection, the selective extraction of decision 7, the
  generator, and the binary `bindings` that writes `crates/script/src/bindings.rs`: interface
  objects as host functions (an interface without a constructor throws a `TypeError`), interface
  prototype objects as host objects chained to their parent's and to `%Object.prototype%` at
  `EventTarget`, attributes as accessors and operations as data properties with Web IDL's property
  attributes, constants on the interface object and its prototype, one brand per concrete interface
  and the generated table of which brand inherits which member, Web IDL's conversions over swell's
  `to_string`, `to_number` and the others, `DomCx` with its node charge, `Wrap`, `DomResult` and
  `DomEnd` (decisions 8, 9 and 13), every copy of a String out of the engine through the one
  counting function of decision 12, and one Rust trait per slice (`reading`, `attributes`,
  `mutation`, `selectors`) whose default for every method answers `NotSupportedError` naming the
  member, so that each slice story writes one file. The members and constructors of `throws` are
  generated to throw with their reason and have no trait method; those of `exception` are generated
  with their behaviour. A member's steps charge steps in proportion to its work (requirement 3, as
  revised). The generator refuses a definition the selection does not name, and a selected member
  the model does not hold.
- `crates/webidl/data/whatwg/`: the HTML file of decision 7 and its lines in `SOURCE`.
- `crates/script/src/bindings.rs` (generated): its header names the sha256 of each source (the
  vendored files, the selection, the generator's sources) and carries the WHATWG BSD 3-Clause
  notice as `crates/html/src/charref_table.rs:1-31` does.
- `crates/script/Cargo.toml`, `Cargo.lock`: decision 11.
- `crates/script/src/lib.rs`: the rule of decision 10 over the runner;
  `crates/script/src/dom/mod.rs` and one file per slice (`reading.rs`, `attributes.rs`,
  `mutation.rs`, `selectors.rs`), each with an empty `impl` of its trait, and `wrapper.rs` with the
  `Wrap` of decision 8, so every member throws its `NotSupportedError` until its slice; the realm's
  setup defines the interface objects and the global's members in a `with_cx` right after
  `create_realm` (epic lines 310-314).
- `taskfiles/webidl.yml` (new) and `Taskfile.yml`: `webidl-check`, a step of `check`, recomputes
  the digests with `sha256sum` and compares them with the header, generating nothing;
  `webidl-slow`, a step of `slow`, runs `bindings` into `.out/` and compares the bytes with the
  committed file.
- `crates/webidl/tests/`, `crates/script/tests/`: the cases below; `crates/script/tests/main.rs`
  also declares the modules `dom_reading`, `dom_attributes`, `dom_mutation`, `dom_selectors` and
  `parse_run`, each file holding only its head comment, so each later story writes only its own
  test file.
- In the last commit, the public surfaces of acceptance 13.

## Acceptance

1. `slot task webidl-check` passes; with one byte of `selection.txt` changed it fails naming the
   file; `slot task slow` regenerates `bindings.rs` byte for byte.
2. `selection::rows_match_the_model` (in `crates/webidl/tests`): every member and constructor of
   the selected definitions in the reader's model has exactly one row, and every row names a member
   of the model; the counts per slice are those of decision 5 (225 member rows, 7 constructor
   rows), and over the four interfaces of the table `reading` 73, `attributes` 14, `mutation` 29,
   `selectors` 7, `throws` 23. A count the model gives otherwise stops the story: the implementor
   reports it with the command, and the coordinating agent corrects this story and lists it as a
   correction of the epic before the test changes.
3. `selection::outside_definition_refused`: a selection naming `Attr` (a definition not generated)
   or a member the model lacks makes the generator fail naming the row.
4. A table test in `crates/script/tests` asserts, for each member row of `selection.txt`, that the
   member exists on its prototype or interface object with Web IDL's property attributes
   (enumerable, configurable, writable as Web IDL says for constants, attributes and operations);
   the number of cases equals the number of rows (225), and `Element.prototype`'s [[Prototype]] is
   `Node.prototype`, `Node.prototype`'s is `EventTarget.prototype`.
5. A member called on an object of another brand, or on an object made by script, throws a
   `TypeError`; `new Node()` throws a `TypeError`.
6. Each of the 30 members of `throws` and `new EventTarget()` and `new Document()` throws a
   `DOMException` named `NotSupportedError` whose message names the member and its reason; a member
   of another slice throws `NotSupportedError` naming the member until its slice lands.
7. `DOMException` is constructible, has its 25 constants and its three attributes, and its
   prototype chain reaches `%Error.prototype%` (requirement 9 is `held` at `0.8.0`, Q18, so the
   case runs).
8. A realm with the bindings stays within `BINDINGS_REALM_HEAP_BYTES` (506,112 bytes at `0.8.0`,
   Q4), the test reading the engine's constant; the realm's heap and creation time are in the
   report (requirement 10 is `held` at `0.8.0`, so the case runs).
9. `trusted_type_as_domstring`: with a test implementation of the `attributes` trait,
   `el.setAttribute("a", { toString() { return "x" } })` hands the trait the string `x`; a
   `limit_end` case with a test implementation of `mutation` that answers `DomEnd::Limit(Nodes)`
   ends the script `limit: nodes`, `try { ... } catch {}` around the call included, and the tree
   unchanged; a `wall_clock_without_record` case ends `while (true) {}` under a wall-clock bound of
   100 ms `limit: wall-clock`, the elapsed time under 10 s; and a script `1` after the `limit_end`
   script ends `ran`, the record cleared.
10. The generator is timed before and after each source it gains (the HTML file, each interface
    added to the selection), both times in the report; a run over twice its time before the source
    was added is a defect fixed in this story.
11. `convert_before_copy`: with a test implementation of the `attributes` trait and a host function
    `uncharged()` of the test that answers the count of decision 12,
    `el.setAttribute("a", { toString() { return String(uncharged()) } })` hands the trait the value
    `0`, and so does the same call with the two arguments' roles swapped
    (`el.setAttribute({ toString() { return "n" + uncharged() } }, "v")` names the attribute `n0`).
    The report lists each call of `to_utf16` and `to_rust_string_lossy` in `bindings.rs`
    (`git grep -n`) with the member it serves, each after the last conversion of that member's
    arguments or beside a `charge_host_bytes`.
12. `node_charge`: with a test implementation of `mutation` whose `createTextNode` calls the node
    charge of decision 13 and nothing else, `heap_bytes()` after `document.createTextNode("abcd")`
    exceeds its value before by at least 1,224 + 8 bytes (3 × 408 and twice the four bytes stored);
    under a memory limit of 1 MiB, `for (;;) document.createTextNode("x")` ends `limit: memory`.
13. Public surfaces: the story changes, in its last commit and after merging `main`, every public
    surface its change makes false, each number taken from the `slot task check` log of its tip.
    After that commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges` prints
    nothing for each old string below, as the story's base holds it (the strings and lines given
    are those of `main` at `29945f5`, 2026-11-24); each new number equals the one in that log; and
    the gate block, `README.md:584-625` and `website/docs/how-it-is-built.md:40-81`, holds that
    log's lines. The old strings of this story: the test count `1976 passed` (`README.md:11`,
    `docs/badges/make.sh:38`, `docs/badges/tests.svg`), `1,976` and `1,424, and 552`
    (`README.md:285`, `website/docs/status.md:23`, `website/docs/what-works.md:17`).
14. `slot task check` exits 0; `git diff --stat` names only the files of the Change.
15. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; no model
    call and no network request in a test; the bindings are generated once, committed, and read by
    every test, never generated in a test, and a realm with them is made per test only if its
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
