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
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 4}
---
## Context

The third and last tokenizer story. Character references (`&amp;`, `&#x41;`, `&notin`) are
consumed by their own tokenizer states, in data, in RCDATA and in attribute values, with the
rules for a missing semicolon, for a named reference inside an attribute, and for numeric
references to null, surrogates, noncharacters, controls and values above U+10FFFF. The named
references come from the HTML Standard's table, published as `entities.json`. Named from memory,
to be confirmed by this story. `docs/design/html-parser.md` (story `html-decisions`, item 3)
confirms the licence of `entities.json` and where it goes. Cases tagged `charref` are this
story's; with them every tokenizer case the note runs is claimed.

## Change

- `crates/html-conformance/data/whatwg/entities.json` with its `SOURCE` (address, date, licence).
- `crates/html/src/charref.rs`: the character reference states replacing the stub of story
  `html-tokenizer-markup`, the numeric reference replacement table, and the named reference
  lookup, longest match first.
- `crates/html/src/charref_table.rs`: the named table as Rust data, written by a binary of the
  harness crate (`cargo run -p surf-html-conformance --bin entities-table`) from the vendored
  JSON and never edited by hand; a test `the_named_reference_table_matches_entities_json`
  fails when the two differ.
- `crates/html/src/tokenizer.rs`: the calls into `charref.rs`.
- `crates/html-conformance/expected/tokenizer.txt`: every count raised to the case count. Each
  tokenizer section gets the mark the gate's rule gives (`complete` only where every case passes
  and `html-sections` lists nothing open for it); this story does not set it in advance.

Writes: `crates/html/src/charref.rs`, `crates/html/src/charref_table.rs`,
`crates/html/src/tokenizer.rs`, `crates/html-conformance/data/whatwg/`,
`crates/html-conformance/src/` (the table binary), `crates/html-conformance/expected/tokenizer.txt`.

## Acceptance

`slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 printing `tag markup
n/n/n`, `tag text m/m/m` and `tag charref c/c/c` with the counts of `html-inventory`, pass equal
to run for every file under `tokenizer/`, and each tokenizer section printed either
`complete n/n` or `incomplete` with a reason that is not a failing case; and `cargo test -p
surf-html-conformance` passes `the_named_reference_table_matches_entities_json`.

## Out of scope

Character references as the tree builder sees them (they arrive as characters; the tree stories
own the tree). Scripts: nothing here needs a script engine.
