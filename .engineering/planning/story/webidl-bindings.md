---
format: aep.planning-md/3
id: story:webidl-bindings
kind: story
status: draft
title: The binary bindings generates the DOM's interfaces from the model and the selection, every member throwing NotSupportedError until its slice
summary: selection.txt from the note's table; generated interface objects, prototypes, brands, accessors, operations and conversions, one trait per slice; the HTML source of S6 = a; the digest check in check and regeneration in the lane slow.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:webidl-reader
- depends_on: story:scripts-engine-dependency
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/script/src/bindings.rs
- confidence: inferred
  path: crates/script/src/dom
- confidence: cited
  path: crates/script/src/lib.rs
- confidence: inferred
  path: crates/script/tests
- confidence: inferred
  path: crates/webidl/data
- confidence: cited
  path: crates/webidl/selection.txt
- confidence: cited
  path: crates/webidl/src
- confidence: inferred
  path: crates/webidl/tests
- confidence: cited
  path: taskfiles/webidl.yml
revision: 3
---
## Context

The epic's sixth candidate (lines 443-451), "The DOM from Web IDL" (lines 605-772) and the
constraint that the bindings are generated, never written by hand (lines 548-552). The model of
`story:webidl-reader` and the runner of `story:scripts-engine-dependency` exist before it; the
selection is the table of the note's Q8. The lane `slow` exists on `main` (`Taskfile.yml:612-617`;
the epic, line 760, calls it new) and gains a step here.

Assumes S6 = a (open, default a, epic lines 519-532): `Window`'s `document`, `window` and `self`
and `Document`'s `title` are generated from the HTML Standard's IDL, the rest of HTML's `Window`
and `Document` is not. This story vendors that source (the file of the note's Q12) beside the DOM
and Web IDL pages and reads the definitions Q12 names.

Conditions on swell, named and not known today: the tag pinned holds requirement 8
(`Cx::set_prototype_of`), 9 (the closed lists of intrinsics and well-known symbols), 5
(`HostObjectSpec.error_kind`) and 10 (the cost constants). A requirement the tag lacks makes
each member that needs it a named deviation in the selection, with its test, until a later tag;
moving the tag is one line of `crates/script/Cargo.toml`, the lock and the allow-list.

## Change

- `crates/webidl/selection.txt` (new): which definitions are generated, each member's slice or
  its `NotSupportedError` reason, each deviation, in the grammar of Q8.
- `crates/webidl/src/`: the selection's reader and the generator; the binary `bindings` writes
  `crates/script/src/bindings.rs`: interface objects as host functions (an interface without a
  constructor throws a `TypeError`), interface prototype objects as host objects chained to their
  parent's and to `%Object.prototype%` at `EventTarget`, attributes as accessors and operations
  as data properties with Web IDL's property attributes, constants on the interface object and
  its prototype, one brand per concrete interface and the generated table of which brand inherits
  which member, Web IDL's conversions over swell's `to_string`, `to_number` and the others, and
  one Rust trait per slice (`reading`, `attributes`, `mutation`, `selectors`) whose default for
  every method throws a `DOMException` named `NotSupportedError` whose message names the member,
  so that each slice story writes one file. The 23 members and 2 constructors of the slice
  `throws` (epic lines 706-715) are generated to throw with their reason and have no trait
  method. A member's steps charge steps in proportion to its work (requirement 3, as revised).
- `crates/webidl/data/`: the HTML source of Q12 with its lines in `SOURCE`.
- `crates/script/src/bindings.rs` (generated): its header names the sha256 of each source (the
  vendored files, the selection, the generator's sources) and carries the WHATWG BSD 3-Clause
  notice as `crates/html/src/charref_table.rs:1-26` does.
- `crates/script/src/lib.rs`, `crates/script/src/dom/mod.rs` and one file per slice
  (`reading.rs`, `attributes.rs`, `mutation.rs`, `selectors.rs`), each with an empty `impl` of
  its trait, so every member throws its `NotSupportedError` until its slice; the realm's setup
  defines the interface objects and the global's members in a `with_cx` right after
  `create_realm` (epic lines 310-314).
- `taskfiles/webidl.yml` (new) and `Taskfile.yml`: `webidl-check`, a step of `check`, recomputes
  the digests with `sha256sum` and compares them with the header, generating nothing;
  `webidl-slow`, a step of `slow`, runs `bindings` into `.out/` and compares the bytes with the
  committed file.
- `crates/webidl/tests/`, `crates/script/tests/`: the cases below; `crates/script/tests/main.rs`
  also declares the modules `dom_reading`, `dom_attributes`, `dom_mutation` and `dom_selectors`,
  each file holding only its head comment, so each slice story writes only its own test file.

## Acceptance

`slot task check` exits 0. The folder's bounds hold: the test step ends within 120 s and no test
runs over 60 s; no model call and no network request in a test; the bindings are generated once,
committed, and read by every test, never generated in a test, and a realm with them is made per
test only if its measured time is under 50 ms (else one per test binary process, the time in the
report); no Python; the generator has a digest of its sources, a fast check in the gate and full
regeneration in the lane `slow`; the generator is timed before and after each source it gains
(the HTML file, each interface added to the selection) and both times are in the report.

1. `slot task webidl-check` passes; with one byte of `selection.txt` changed it fails naming the
   file; `slot task slow` regenerates `bindings.rs` byte for byte.
2. Every member of the selection exists on its prototype or interface object with Web IDL's
   property attributes (enumerable, configurable, writable as Web IDL says for constants,
   attributes and operations), and `Element.prototype`'s [[Prototype]] is `Node.prototype`,
   `Node.prototype`'s is `EventTarget.prototype` (a table test over the 146 members and the 68 of
   the returned interfaces, its count printed).
3. A member called on an object of another brand, or on an object made by script, throws a
   `TypeError`; `new Node()` throws a `TypeError`.
4. Each of the 23 members and of `new EventTarget()` and `new Document()` throws a `DOMException`
   named `NotSupportedError` whose message names the member and its reason; a member of another
   slice throws `NotSupportedError` naming the member until its slice lands.
5. `DOMException` is constructible, has its 25 constants and its three attributes, and its
   prototype chain reaches `%Error.prototype%` (condition: requirement 9 in the tag).
6. A realm with the bindings stays within `BINDINGS_REALM_HEAP_BYTES`, the test reading the
   engine's constant (condition: requirement 10); the realm's heap and creation time are in the
   report.
7. `git diff --stat` names only the files of the Change.

## Out of scope

Behaviour of any member (the four slice stories); a page's scripts; the HTML definitions beyond
S6's answer.

## Why `epic:scripts-in-a-page`

The generated DOM of S4, b: every member exists before any has behaviour.
