---
format: aep.planning-md/3
id: story:html-character-references
kind: story
status: implemented
title: Character references, and every tokenizer case that needs no script passing
summary: Named and numeric character references from a vendored entities.json; every tokenizer case of the test data passes (7121 of 9095 run), and the tokenizer sections stay incomplete until the cases that need a script engine can run.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-tokenizer-text
scope:
- confidence: cited
  path: crates/html-conformance/Cargo.toml
- confidence: cited
  path: crates/html-conformance/data/whatwg/
- confidence: cited
  path: crates/html-conformance/expected/tokenizer.txt
- confidence: cited
  path: crates/html-conformance/src/bin/entities-table.rs
- confidence: cited
  path: crates/html-conformance/src/entities_table.rs
- confidence: cited
  path: crates/html-conformance/src/lib.rs
- confidence: cited
  path: crates/html-conformance/tests/entities_table.rs
- confidence: cited
  path: crates/html/src/charref.rs
- confidence: cited
  path: crates/html/src/charref_table.rs
- confidence: cited
  path: crates/html/src/lib.rs
- confidence: cited
  path: crates/html/src/tokenizer.rs
- confidence: cited
  path: crates/html/src/tokenizer/machine.rs
revision: 21
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-21T16:32:39Z", actor: "human:operator", revision: 21, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

The third and last tokenizer story. Character references (`&amp;`, `&#x41;`, `&notin`) are
consumed by their own tokenizer states (13.2.5.77 to 13.2.5.84), in data, in RCDATA and in
attribute values, with the rules for a missing semicolon, for a named reference inside an
attribute, and for numeric references to null, surrogates, noncharacters, controls and values
above U+10FFFF. The named references come from the table of 13.5 "Named character references",
published as `https://html.spec.whatwg.org/entities.json`. `docs/design/html-parser.md` (story
`html-decisions`, item 3) settles its licence: the WHATWG IPR policy licenses the Living Standard
under CC BY 4.0, and portions incorporated into source code under the BSD 3-Clause License
instead. So the vendored JSON is CC BY 4.0, and the generated table, which ships, carries the
BSD 3-Clause notice. Cases tagged `charref` are this story's; with them every tokenizer case the
note runs is claimed.

What is there on `main` after wave 6:

- The plumbing of the states exists in `crates/html/src/tokenizer/machine.rs`: the data, RCDATA
  and the three attribute value states set the return state and switch to
  `State::CharacterReference`; `temporary_buffer`, `consumed_as_part_of_an_attribute` and
  `flush_code_points_consumed_as_a_character_reference` are written. The state itself is a stub
  (its "anything else" entry only, `machine.rs`, the arm `State::CharacterReference`), so `&` is
  a character. `crates/html/src/charref.rs` holds no item.
- `entities.json` is not vendored yet. Fetched for this revision (2026-11-21): 145,897 bytes,
  2,231 names (2,125 ending in `;`, 106 without), 2,138 of one code point and 93 of two, the
  longest name 33 code points with `&` and `;`. Its 2,231 names and code points are exactly the
  2,231 references `namedEntities.test` (html5lib-tests `c5495c5`) expects.
- `tag charref 2060/4739/4739`. The 2,679 failing cases (`--list-failing charref`), by what
  they need:
  - a named reference outside an attribute: 2,243 (`namedEntities.test` 2,231: 2,125 with `;`
    and 106 without; `entities.test` 10 to 12; `contentModelFlags.test` 24, `domjs.test` 29 and
    30, `escapeFlag.test` 7, `test1.test` 53 to 56, `test2.test` 38);
  - a named reference in an attribute value: 2 (`domjs.test` 46, `test1.test` 65);
  - a numeric reference outside an attribute: 432 (`numericEntities.test` 336, `entities.test`
    13 to 80, `test1.test` 51, 52, 59, 60, `test2.test` 16 to 21, `test4.test` 27 to 43,
    `domjs.test` 51);
  - a numeric reference in an attribute value: 2 (`test1.test` 61, `test4.test` 13);
  - of these, 122 expect `missing-semicolon-after-character-reference` (110 named, 12
    numeric);
  - none needs anything that is not a character reference.
- 2,060 `charref` cases pass today only because `&` stays a character, and must keep passing:
  among them the 1,979 of `namedEntities.test` that are a name without `;` that is no name of
  the table, and the 12 of the attribute rule (`entities.test` 1 to 9, `test1.test` 62 to 64:
  `&not` followed by `=` or an alphanumeric inside an attribute value is not a reference).
- `domjs.test` 46 (`<p id="&NotEqualTilde;">`, the wave 6 finding): a named reference of two
  code points in a double-quoted attribute value. It needs the named character reference state
  and the attribute branch of the flush, nothing of the text states.
- Both tokenizer sections keep a `wpt` line in the note's `html-sections` block
  (`tokenization`: 7 tests, `named-character-references`: 1, all needing a script engine), so
  the check prints them `incomplete` whatever the counts (`check.rs`, `section_line`). No
  section line of `expected/tokenizer.txt` changes in this story, and none can be `complete`
  until a script engine runs those tests.

## Change

- `crates/html-conformance/data/whatwg/entities.json`, copied unchanged from
  `https://html.spec.whatwg.org/entities.json`, and `crates/html-conformance/data/whatwg/SOURCE`
  in the form of `data/html5lib-tests/SOURCE`: the address, the date fetched, the "Last
  Updated" date of the standard it was fetched with, the licence CC BY 4.0 (with the line
  "Copyright © WHATWG (Apple, Google, Mozilla, Microsoft)."), the sentence that the generated
  `crates/html/src/charref_table.rs` carries the BSD 3-Clause notice under the WHATWG IPR policy,
  the number of names (2231) and the sha256 of `entities.json`. No `source` line for `whatwg` in
  the note's `html-inventory` block: `entities.json` holds no case, and `paths::data_files` walks
  only the inventory's sources.
- `crates/html/src/tokenizer/machine.rs`: the eight states 13.2.5.77 to 13.2.5.84 in `State`
  and in `step` (character reference, named character reference, ambiguous ampersand, numeric
  character reference, hexadecimal character reference start, hexadecimal and decimal
  character reference, numeric character reference end), replacing the stub, each with
  the standard's name and parse error codes in the standard's order. The character reference
  code is held in a field of `Tokenizer` and saturates above 0x10FFFF (`test4.test` 38 to 42 go
  up to `&#18446744073709551681;`, 20 digits). The doc comment of the module loses "Not written
  here".
- `crates/html/src/charref.rs`: the named reference lookup, longest match first, reading at most
  32 code points after `&` (the longest name; `entities.test` 12 holds a run of several hundred
  letters), and the numeric replacement table of 13.2.5.84 (0x80 to 0x9F). No public item.
- `crates/html/src/charref_table.rs`: the named table as Rust data, written by the binary
  `entities-table` of the harness crate (`cargo run -p surf-html-conformance --bin
  entities-table`) from the vendored JSON's `codepoints`, never edited by hand. Its header,
  written by the same binary, is the BSD 3-Clause License notice for the WHATWG portion: the line
  "Copyright © WHATWG (Apple, Google, Mozilla, Microsoft)." (the copyright line of the
  standard's repository licence; the IPR policy gives none and links the licence text at
  `https://opensource.org/licenses/BSD-3-Clause`), then the three conditions and the disclaimer
  of that text. The binary writes rustfmt-clean output, so that `cargo fmt -p surf-html` leaves
  the file as written. Declared private in `crates/html/src/lib.rs` (`mod charref_table;`).
- `crates/html-conformance/src/entities_table.rs` (module `pub mod entities_table` in the
  crate's `lib.rs`): the one function that turns the JSON into the file's text, read with the
  crate's own `json` reader. `crates/html-conformance/src/bin/entities-table.rs` calls it and
  writes the file. `crates/html-conformance/Cargo.toml` gets `default-run =
  "surf-html-conformance"`: with a second binary, `cargo run -p surf-html-conformance` (the task
  `html-conformance`, `taskfiles/html-conformance.yml`) fails with "could not determine which
  binary to run" (measured in a copy). The lock does not change.
- `crates/html-conformance/tests/entities_table.rs`: `the_named_reference_table_matches_entities_json`
  asserts that the function's text for `data/whatwg/entities.json` equals
  `crates/html/src/charref_table.rs` byte for byte, so an edit of either fails it; and
  `the_named_reference_table_carries_the_bsd_3_clause_notice` asserts that the file starts with
  the copyright line and holds the three conditions and the disclaimer.
- `crates/html/src/tokenizer.rs` and `crates/html/src/lib.rs`: the documentation only (the
  module comment, the comment of `tokenize`, the section "State" of `lib.rs`, which says the
  character reference states are a stub and `charref` holds nothing). Neither `tokenize`'s
  signature nor the public `State` changes: the character reference states are not states a
  caller starts in.
- `crates/html-conformance/expected/tokenizer.txt`: the ten counts of the cases this story makes
  pass raised to them (listed under "Acceptance"); the three `section` lines stay `incomplete`.

The story leaves for `story:html-tree-body` the switching of the state between tokens: it adds
no entry point and keeps every value of a reference (return state, temporary buffer, character
reference code) in fields of `Tokenizer`, not in a loop that must run to the end of the input.

Writes: `crates/html/src/tokenizer/machine.rs`, `crates/html/src/charref.rs`,
`crates/html/src/charref_table.rs`, `crates/html/src/tokenizer.rs`, `crates/html/src/lib.rs`,
`crates/html-conformance/data/whatwg/`, `crates/html-conformance/Cargo.toml`,
`crates/html-conformance/src/lib.rs`, `crates/html-conformance/src/entities_table.rs`,
`crates/html-conformance/src/bin/entities-table.rs`,
`crates/html-conformance/tests/entities_table.rs`,
`crates/html-conformance/expected/tokenizer.txt`.

## Acceptance

1. `slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 and prints no
   `raise:` line and no `fail:` line. It prints `tag markup 1979/1979/1979`, `tag text
   328/328/328`, `tag charref 4739/4739/4739`, `tag encoding 75/82/82`, `tag not-run 0/0/12`,
   the total `html-conformance: 9107 cases, 9095 run, 7121 passed`, and the three tokenizer
   sections as
   `section tokenization incomplete 2836/2836 wpt:7`, `section named-character-references
   incomplete 4210/4210 wpt:1` and `section coercing-an-html-dom-into-an-infoset incomplete 0/4
   not-run:4 open:not-taken`.
2. `crates/html-conformance/expected/tokenizer.txt` holds these lines, each where it is now:
   `file html5lib-tests/tokenizer/contentModelFlags.test 24/24/24`, `file
   html5lib-tests/tokenizer/domjs.test 56/56/56`, `file html5lib-tests/tokenizer/entities.test
   80/80/80`, `file html5lib-tests/tokenizer/escapeFlag.test 9/9/9`, `file
   html5lib-tests/tokenizer/namedEntities.test 4210/4210/4210`, `file
   html5lib-tests/tokenizer/numericEntities.test 336/336/336`, `file
   html5lib-tests/tokenizer/test1.test 69/69/69`, `file html5lib-tests/tokenizer/test2.test
   46/46/46`, `file html5lib-tests/tokenizer/test4.test 85/85/85` and `tag charref
   4739/4739/4739`. Every other line of the file is unchanged, the three `section` lines
   included. From then on a case of these files that stops passing fails the task with
   `fail: crates/html-conformance/expected/tokenizer.txt: file <path>: passed <n>, expected
   <m>` and the same for `tag charref`.
3. `cargo run -p surf-html-conformance -- --list-failing charref` prints nothing (2,679 lines
   on `main`), and `cargo run -p surf-html-conformance` runs the harness, not an error about
   two binaries.
4. `cargo test -p surf-html-conformance` passes `the_named_reference_table_matches_entities_json`
   and `the_named_reference_table_carries_the_bsd_3_clause_notice`, 27 tests in all (25 on
   `main`). Planted defects, each reverted after: one code point of a name changed in
   `charref_table.rs` fails the first; the copyright line removed fails the second.
5. `cargo run -p surf-html-conformance --bin entities-table` on the unit's tree leaves `git
   status` unchanged (the committed table is the binary's output), and `cargo fmt -p surf-html
   -- --check` prints nothing.
6. `cargo test -p surf-html` passes with warnings denied, the 12 unit tests of `main` among
   them. `data/whatwg/SOURCE` names the sha256 that `sha256sum
   crates/html-conformance/data/whatwg/entities.json` prints.

## For AGENTS.md (handed over in the unit report)

- "Layout", the `crates/html` entry: the tokenizer runs the character reference states too
  (`src/charref.rs`, the table `src/charref_table.rs` generated from the vendored
  `entities.json`, BSD 3-Clause notice in its header); the sentence "A character reference is
  not read" goes.
- The `crates/html-conformance` entry: `data/whatwg/entities.json` (CC BY 4.0, with `SOURCE`),
  the binary `entities-table` that writes `crates/html/src/charref_table.rs`, `default-run`, the
  number of tests, and `html-conformance: 9107 cases, 9095 run, 7121 passed`.
- "The gate's numbers": `tag charref 4739/4739/4739` and the total.

## Out of scope

Character references as the tree builder sees them (they arrive as characters; the tree stories
own the tree). Switching the tokenizer's state between tokens (`story:html-tree-body`). Marking
a tokenizer section `complete`: each has a `wpt` line that needs a script engine. Scripts:
nothing here needs a script engine.
