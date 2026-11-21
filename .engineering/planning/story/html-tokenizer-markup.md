---
format: aep.planning-md/3
id: story:html-tokenizer-markup
kind: story
status: active
title: Tokenize tags, attributes, comments and doctypes
summary: Input preprocessing and the markup states of the tokenizer; every tokenizer case tagged markup passes.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-conformance-gate
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 4}
---
## Context

The tokenizer of the HTML Standard is a state machine over the preprocessed input stream (13.2.3.5
"Preprocessing the input stream" and 13.2.5 "Tokenization" of the snapshot
`docs/design/html-parser.md` names). This is the first of three tokenizer stories, cut by the case tags of
`docs/design/html-parser.md` (story `html-decisions`, item 8): `markup`, then `text`, then
`charref`. The empty modules and the entry point "tokenize from an initial state" exist (story
`html-conformance-harness`); the gate holds the counts (story `html-conformance-gate`).

## Change

- `crates/html/src/input.rs`: input stream preprocessing (CR and CR LF to LF) and the parse
  errors it defines for surrogates, noncharacters and controls, read by the tokenizer.
- `crates/html/src/tokenizer.rs` (submodules under `crates/html/src/tokenizer/` if the story
  wants them): the token types (DOCTYPE, start tag, end tag, comment, processing instruction, character, end of
  file),
  the parse error codes the standard names, and the states for data, tag open, end tag open, tag
  name, the attribute states, self-closing start tag, bogus comment, markup declaration open,
  the comment states, the DOCTYPE states, the CDATA section states, and the five processing
  instruction states (13.2.5.72 to 13.2.5.76, `#processing-instruction-open-state` and after, as
  the note cites them), whose token is compared in the three-member form the data uses,
  `["ProcessingInstruction", <target>, <data>]` (52 tokenizer tests have `<?` in their input), with the hook the tree
  builder answers for "adjusted current node is not in the HTML namespace". A duplicate
  attribute is dropped with its parse error, as the standard says.
- The character reference states are left as a stub that emits `&` as a character; story
  `html-character-references` replaces it.
- `crates/html-conformance/expected/tokenizer.txt`: the counts raised.

Writes: `crates/html/src/input.rs`, `crates/html/src/tokenizer.rs`,
`crates/html/src/tokenizer/`, `crates/html-conformance/expected/tokenizer.txt`.

## Acceptance

`slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 printing `tag markup
1979/1979/1979` (1,979 the `markup` count of `html-inventory`), tokens, processing instruction
tokens included, and parse error codes compared.

## Out of scope

RCDATA, RAWTEXT, script data and PLAINTEXT states (story `html-tokenizer-text`), character
references (story `html-character-references`), decoding bytes (story `html-input-stream`).
Scripts: nothing in these states needs a script engine.
