---
format: aep.planning-md/3
id: story:html-input-stream
kind: story
status: implemented
title: Decode a response body by the encoding sniffing algorithm
summary: BOM, prescan and default decide the encoding; UTF-8, UTF-16 and windows-1252 are decoded; every encoding case passes but encoding/tests1.dat 48 to 54, which need the encoding changed while parsing (story html-tree-body).
relations:
- decomposes: epic:html-parser
- depends_on: story:html-conformance-gate
scope:
- confidence: cited
  path: crates/html-conformance/expected/encoding.txt
- confidence: cited
  path: crates/html/src/encoding.rs
- confidence: cited
  path: crates/html/src/lib.rs
- confidence: cited
  path: crates/html/tests/encoding.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-21T13:40:36Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

A response body reaches the `document` component as bytes (`surf.document.ParseDocument`,
input `body`). The parser reads characters. The HTML Standard's 13.2.3 "The input byte stream"
(the encoding sniffing algorithm: the byte order mark, the transport layer, the prescan of
`meta` with its "get an XML encoding" step, the default) and the Encoding Standard's decoders
turn one into the other (the snapshots `docs/design/html-parser.md` names). The preprocessing of the decoded stream (newlines,
parse errors for surrogates, noncharacters and controls) is the tokenizer's input and belongs to
story `html-tokenizer-markup`, so the two stories write different files and can run at once.

`docs/design/html-parser.md` (story `html-decisions`, item 4) fixes the encodings taken on:
proposed, the whole label table; decoders for UTF-8, UTF-16LE, UTF-16BE, windows-1252 and
replacement; windows-1252 as the default; an encoding without a decoder decoded as windows-1252
with the name recorded; no transport-layer step, because `surf.net.MediaType` drops the
`charset` parameter (`spec/domains/net.yaml`), listed in `html-sections` as an open requirement
of its section; no change of encoding after the prescan window. The cases of this story are
those tagged `encoding` in the note's `html-inventory` block.

## Change

- `crates/html/src/encoding.rs`: the label table (every label to its encoding name, from the
  standard's table, kept as data with its source named), the decoders listed above with the
  standard's error handling (U+FFFD replacement), BOM sniffing, the prescan algorithm over the
  first 1,024 bytes with its "get an XML encoding" step, and the result: the decoded text;
  `encoding`, the name of the encoding the sniffing algorithm chose (what `parse_document_bytes`
  reports and the harness compares, `crates/html-conformance/src/runner.rs:134`); the encoding
  the text was decoded with (windows-1252 when the chosen one has no decoder); and the confidence
  (tentative or certain). The doc comment of `ParsedDocument::encoding`
  (`crates/html/src/lib.rs:72-73`) says the same. Also
  the entry that `html-tree-body` calls to change the encoding: decode the same bytes again with
  a given encoding and the confidence certain.
- Tests in `crates/html/tests/encoding.rs`: `utf8_truncated_sequence_is_replaced`,
  `utf8_overlong_form_is_replaced`, `utf8_encoded_surrogate_is_replaced`,
  `utf16le_unpaired_surrogate_is_replaced`, `utf16be_unpaired_surrogate_is_replaced`,
  `windows_1252_maps_0x80_to_0x9f_as_the_standard_table` (each of the 32 bytes 0x80 to 0x9F
  against the code point the Encoding Standard's `index-windows-1252.txt` gives it at snapshot
  `7d2e3e08e11f57ba1b080390b490d53648c6f69c`, read by the unit from that index and written
  literally in the test, not taken from the crate's table), `replacement_decodes_to_one_u_fffd`,
  `bom_wins_over_meta`, `unsupported_encoding_is_decoded_as_windows_1252_and_named`,
  `prescan_reads_the_encoding_of_an_xml_declaration`,
  `every_one_of_the_228_labels_names_one_of_the_40_encodings` (and, one by one, one label each
  for ISO-8859-2, windows-1252, UTF-16LE, replacement and EUC-JP, which the unit picks from the
  labels of `encodings.json` at the same snapshot and writes literally in the test with the name
  it maps to), `a_bom_is_certain_and_a_meta_or_the_default_is_tentative`,
  `decoding_again_with_a_given_encoding_is_certain`.
- `crates/html-conformance/expected/encoding.txt`: the counts raised. The mark of the section is
  whatever the gate's rule gives; this story does not set it in advance.

Writes: `crates/html/src/encoding.rs`, `crates/html/tests/encoding.rs`,
`crates/html-conformance/expected/encoding.txt`, `crates/html/src/lib.rs` (the doc comment of
`ParsedDocument::encoding` only).

Not this unit's: `crates/html-conformance/tests/check.rs` (its fixture cases are made to fail
for any parser on the wave branch before the units start) and the crate documentation at
`crates/html/src/lib.rs:6-11` (rewritten on the wave branch after the merge).

## Acceptance

`slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 printing `tag encoding
75/82/82` and no `raise:` line, `expected/encoding.txt` holding
`file html5lib-tests/encoding/test-yahoo-jp.dat 1/1/1`,
`file html5lib-tests/encoding/tests1.dat 52/59/59`,
`file html5lib-tests/encoding/tests2.dat 22/22/22` and `tag encoding 75/82/82`,
the seven cases not passing being `encoding/tests1.dat` cases 48 to 54 and no other,
that is, `cargo run -p surf-html-conformance -- --list-failing encoding` printing exactly the
seven lines `html5lib-tests/encoding/tests1.dat 48` to `html5lib-tests/encoding/tests1.dat 54`, and `cargo test -p surf-html` passes the
thirteen tests of `tests/encoding.rs` named in Change, and `cargo test -p surf-html-conformance`
passes.

## Out of scope

Decoders for the other encodings of the Encoding Standard; carrying `charset` through
`surf.net` (a change to another component, reported, not drafted here); changing the encoding
after parsing began (13.2.3.4, story `html-tree-body`, with the seven cases it brings).
Scripts: the encoding tests under `scripted/` need `document.write`; they
are tagged `not-run`.
