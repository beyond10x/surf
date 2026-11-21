---
format: aep.planning-md/3
id: story:html-tree-body
kind: story
status: active
title: Build the tree outside tables, templates and foreign content
summary: The node arena and the insertion modes from initial to after after frameset; every tree case tagged body passes.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-character-references
- depends_on: story:html-input-stream
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 5}
---
## Context

Tree construction in the HTML Standard is a set of insertion modes over a stack of open elements
(13.2.6 "Tree construction" of the snapshot `docs/design/html-parser.md` names). This is the first
of the tree stories, cut by the case tags of `docs/design/html-parser.md` (story
`html-decisions`, item 8): `body`, `formatting`, `tables`, `template`, `foreign`, then `mixed`
(the mixed files, story `html-tree-complete`). It needs the whole tokenizer, because the tree
builder switches it into RCDATA, RAWTEXT, script data and PLAINTEXT (story
`html-character-references`, the last tokenizer story).

It also takes the last step of the encoding cases. `docs/design/html-parser.md` item 4: a `meta`
that the tree builder meets while the confidence is tentative changes the encoding as 13.2.3.4
"Changing the encoding while parsing" says, and since surf holds the whole body it re-parses the
same bytes from memory with the new encoding and the confidence certain, inside the one parse,
with no second navigation; the optional on-the-fly switch is not taken. Seven encoding cases,
`encoding/tests1.dat` cases 48 to 54, pass only through this step, so story `html-input-stream`
leaves them failing (`tag encoding 75/82/82`) and this story, which depends on it, claims them.

`story:html-document-port` parses real pages before the tree chain ends
(`docs/plan/agent-plan-revision-1.md`, D12), so the parser says which parts it does not yet
parse as the standard does, and each later tree story shortens that list.

## Change

- `crates/html/src/tree.rs`: the node arena the harness story declared, with the operations
  tree construction uses (create an element for a token, insert at the appropriate place,
  append text merging adjacent text nodes, attributes added to `html` and `body`), node
  identities that stay the same for the life of the tree, and document order traversal. The
  tree has no public operation that changes it after parsing (D2).
- `crates/html/src/tree_builder.rs` (submodules under `crates/html/src/tree_builder/` allowed):
  the initial mode with the quirks mode decision from the DOCTYPE, before html, before head, in
  head, in head noscript (both values of the scripting flag), after head, in body except what
  the formatting, tables and template stories own, text, after body, in frameset, after
  frameset, after after body, after after frameset; the stack of open elements and the scope
  checks; generate implied end tags; the list of active formatting elements with its marker and
  "reconstruct the active formatting elements", with the adoption agency algorithm as a hook
  that, until story `html-tree-formatting`, pops to the matching element; stop parsing.
- `NOT_YET_PARSED`, a public list in `crates/html/src/tree_builder.rs` re-exported from
  `lib.rs`, naming each part the tree builder does not yet parse as the standard does:
  `misnested-formatting`, `tables-and-select`, `template`, `foreign-content`. Each entry matches
  `[a-z-]+`, as the answer's renderer requires (`docs/plan/agent-plan-revision-2.md`, D31). Each
  later tree story removes its entry; `html-tree-complete` leaves it empty. A test that holds a
  `note parser` line builds it from this constant, never from a literal (D31), so a story that
  shortens the list breaks no other story's test.
- `crates/html/tests/not_yet_parsed.rs`: `every_not_yet_parsed_entry_matches_a_z_and_hyphen`
  (every entry non-empty and of the characters `a` to `z` and `-` only), which stays and holds
  each later shortening.
- Processing instruction tokens (story `html-tokenizer-markup`) inserted as processing
  instruction nodes where the insertion modes of this story say, as the 119 `body` cases of
  `processing-instructions.dat` expect.
- The `meta` rules of "in head": with the confidence tentative, a `charset` attribute, or an
  `http-equiv` of `Content-Type` with a `content` that names an encoding, runs 13.2.3.4: if the
  new encoding is not the one in use, the parse entry point decodes the same bytes again through
  the entry of `crates/html/src/encoding.rs` that `html-input-stream` provides, with the
  confidence certain, and starts the parse over; it does so at most once per parse.
- The entry point "parse a document" runs decoding, tokenizer and tree builder together.
- `crates/html-conformance/expected/tree-construction.txt` and
  `crates/html-conformance/expected/encoding.txt`: the counts raised.

Writes: `crates/html/src/tree.rs`, `crates/html/src/tree_builder.rs`,
`crates/html/src/tree_builder/`, `crates/html/src/lib.rs` (the parse entry point and the
re-export and the re-parse of 13.2.3.4), `crates/html/tests/not_yet_parsed.rs` (new),
`crates/html-conformance/expected/tree-construction.txt`,
`crates/html-conformance/expected/encoding.txt`.

## Acceptance

`slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 printing `tag body
476/476/476` and `tag encoding 82/82/82` (the counts of `html-inventory`; the seven of
`encoding/tests1.dat` 48 to 54 among the 82), the section `the-input-byte-stream` printed either
`complete n/n` or `incomplete` with a reason that is not a failing case, and `cargo test -p surf-html` passes
`every_not_yet_parsed_entry_matches_a_z_and_hyphen` with the four entries named in Change.

## Out of scope

The adoption agency algorithm (story `html-tree-formatting`), tables and select (story
`html-tree-tables`), `template` (story `html-tree-template`), MathML, SVG and fragments (story
`html-tree-foreign`), limits (story `html-parser-limits`). Scripts: a `script` element is
inserted and its contents kept as text; at its end tag in the text mode the standard prepares
the script, pauses the parser for a parsing-blocking script and runs it, and surf only pops the
element and returns to the original mode. Left unsupported, and listed by the note as open
requirements: script execution, the script nesting level, parser pausing, the insertion point
and `document.write`, and in "the end" the deferred scripts and the `DOMContentLoaded` and
`load` events. Custom elements: "create an element for a token" looks up no definition and runs
no constructor.
