---
format: aep.planning-md/3
id: story:html-character-references
kind: story
status: active
title: Character references, and every tokenizer case passing
summary: Named and numeric character references from the vendored entities.json; the tokenizer section is marked complete only if every case passes.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-tokenizer-text
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 4}
---
## Context

The third and last tokenizer story. Character references (`&amp;`, `&#x41;`, `&notin`) are
consumed by their own tokenizer states, in data, in RCDATA and in attribute values, with the
rules for a missing semicolon, for a named reference inside an attribute, and for numeric
references to null, surrogates, noncharacters, controls and values above U+10FFFF. The named
references come from the table of 13.5 "Named character references", published as
`https://html.spec.whatwg.org/entities.json`. `docs/design/html-parser.md` (story
`html-decisions`, item 3) settles its licence: the WHATWG IPR policy licenses the Living Standard
under CC BY 4.0, and portions incorporated into source code under the BSD 3-Clause License
instead. So the vendored JSON is CC BY 4.0, and the generated table, which ships, carries the
BSD 3-Clause notice. Cases tagged `charref` are this
story's; with them every tokenizer case the note runs is claimed.

## Change

- `crates/html-conformance/data/whatwg/entities.json` with its `SOURCE`: the address, the date
  fetched, the licence CC BY 4.0, and the sentence that the generated
  `crates/html/src/charref_table.rs` carries the BSD 3-Clause notice under the WHATWG IPR policy.
- `crates/html/src/charref.rs`: the character reference states replacing the stub of story
  `html-tokenizer-markup`, the numeric reference replacement table, and the named reference
  lookup, longest match first.
- `crates/html/src/charref_table.rs`: the named table as Rust data, written by a binary of the
  harness crate (`cargo run -p surf-html-conformance --bin entities-table`) from the vendored
  JSON and never edited by hand. Its header carries the BSD 3-Clause License notice for the WHATWG
  portion (the copyright line and the licence text as the WHATWG IPR policy gives them), written
  by the same binary. A test `the_named_reference_table_matches_entities_json`
  fails when the two differ, and a test
  `the_named_reference_table_carries_the_bsd_3_clause_notice` fails when the header lacks the
  notice.
- `crates/html/src/tokenizer.rs`: the calls into `charref.rs`.
- `crates/html-conformance/expected/tokenizer.txt`: every count raised to the case count. Each
  tokenizer section gets the mark the gate's rule gives (`complete` only where every case passes
  and `html-sections` lists nothing open for it); this story does not set it in advance.

Writes: `crates/html/src/charref.rs`, `crates/html/src/charref_table.rs`,
`crates/html/src/tokenizer.rs`, `crates/html-conformance/data/whatwg/`,
`crates/html-conformance/src/` (the table binary), `crates/html-conformance/expected/tokenizer.txt`.

## Acceptance

`slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 printing `tag markup
1979/1979/1979`, `tag text 328/328/328` and `tag charref 4739/4739/4739` (the counts of
`html-inventory`), pass equal to run for each of the 14 files of the `tokenizer` group
(`xmlViolation.test` has 0 run, its 4 cases `not-run:xml-infoset`), and each tokenizer section printed either
`complete n/n` or `incomplete` with a reason that is not a failing case; and `cargo test -p
surf-html-conformance` passes `the_named_reference_table_matches_entities_json` and
`the_named_reference_table_carries_the_bsd_3_clause_notice`.

## Out of scope

Character references as the tree builder sees them (they arrive as characters; the tree stories
own the tree). Scripts: nothing here needs a script engine.
