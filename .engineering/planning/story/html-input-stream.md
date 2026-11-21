---
format: aep.planning-md/3
id: story:html-input-stream
kind: story
status: active
title: Decode a response body by the encoding sniffing algorithm
summary: BOM, prescan and default decide the encoding; UTF-8, UTF-16 and windows-1252 are decoded; every encoding case passes.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-conformance-gate
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 4}
---
## Context

A response body reaches the `document` component as bytes (`surf.document.ParseDocument`,
input `body`). The parser reads characters. The HTML Standard's input byte stream section
(determining the character encoding: the byte order mark, the transport layer, the prescan of
`meta`, the default) and the Encoding Standard's decoders turn one into the other. Named from
memory, to be confirmed by this story. The preprocessing of the decoded stream (newlines,
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
  standard's error handling (U+FFFD replacement), BOM sniffing, the prescan algorithm, and the
  result: the decoded text, the encoding used, its confidence, and the name not honoured if any.
- Tests in `crates/html/tests/encoding.rs`: `utf8_truncated_sequence_is_replaced`,
  `utf8_overlong_form_is_replaced`, `utf8_encoded_surrogate_is_replaced`,
  `utf16le_unpaired_surrogate_is_replaced`, `utf16be_unpaired_surrogate_is_replaced`,
  `windows_1252_maps_0x80_to_0x9f_as_the_standard_table`, `replacement_decodes_to_one_u_fffd`,
  `bom_wins_over_meta`, `unsupported_encoding_is_decoded_as_windows_1252_and_named`,
  `every_label_of_the_source_table_is_known`.
- `crates/html-conformance/expected/encoding.txt`: the counts raised. The mark of the section is
  whatever the gate's rule gives; this story does not set it in advance.

Writes: `crates/html/src/encoding.rs`, `crates/html/tests/encoding.rs`,
`crates/html-conformance/expected/encoding.txt`.

## Acceptance

`slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 printing `tag encoding
n/n/n` with n the `encoding` count of `html-inventory`, and the encoding section printed either
`complete n/n` or `incomplete` with a reason that is not a failing case, and `cargo test -p
surf-html` passes the ten tests of `tests/encoding.rs` named in Change.

## Out of scope

Decoders for the other encodings of the Encoding Standard; carrying `charset` through
`surf.net` (a change to another component, reported, not drafted here); changing the encoding
after parsing began. Scripts: the encoding tests under `scripted/` need `document.write`; they
are tagged `not-run`.
