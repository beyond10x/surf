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
scope:
- confidence: cited
  path: crates/html-conformance/expected/tokenizer.txt
- confidence: cited
  path: crates/html/src/input.rs
- confidence: cited
  path: crates/html/src/tokenizer
- confidence: cited
  path: crates/html/src/tokenizer.rs
revision: 7
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
  wants them): the token types already declared in `tokenizer.rs`, kept as they are, and
  `tokenize` with its signature (both are read by `crates/html-conformance/src/runner.rs`),
  the parse error codes the standard names, and the states for data, tag open, end tag open, tag
  name, the attribute states, self-closing start tag, bogus comment, markup declaration open,
  the comment states, the DOCTYPE states, the CDATA section states, and the five processing
  instruction states (13.2.5.72 to 13.2.5.76, `#processing-instruction-open-state` and after, as
  the note cites them), whose token is compared in the three-member form the data uses,
  `["ProcessingInstruction", <target>, <data>]` (52 tokenizer tests have `<?` in their input). The
  markup declaration open state answers "adjusted current node is not in the HTML namespace"
  with no, as for the HTML namespace (the tokenizer data has no tree builder); story
  `html-tree-foreign` adds the hook the tree builder answers. A duplicate
  attribute is dropped with its parse error, as the standard says.
- The character reference states are left as a stub that emits `&` as a character; story
  `html-character-references` replaces it.
- `crates/html-conformance/expected/tokenizer.txt`: the counts raised.

Writes: `crates/html/src/input.rs`, `crates/html/src/tokenizer.rs`,
`crates/html/src/tokenizer/`, `crates/html-conformance/expected/tokenizer.txt`.

`crates/html-conformance/tests/check.rs` (its fixtures, made to fail for any parser by a unit of
its own before this one) and the crate documentation at `crates/html/src/lib.rs:6-11`
(rewritten on the wave branch after the merge) are not this unit's.

## Acceptance

`slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 printing `tag markup
1979/1979/1979` (1,979 the `markup` count of `html-inventory`) and `tag text` with at least 57
passed (the 4 that pass today and the 53 cases in `CDATA section state` without `&`), and no
`raise:` line; tokens, processing instruction tokens included, and parse error codes compared.
The `tag text` and `tag charref` counts and the `file` lines are measured by the unit after the
change with `cargo run -p surf-html-conformance` and written as measured into
`crates/html-conformance/expected/tokenizer.txt` and into its report.
`cargo run -p surf-html-conformance -- --list-failing markup` prints nothing, and
`cargo test -p surf-html -p surf-html-conformance` passes.

## Out of scope

RCDATA, RAWTEXT, script data and PLAINTEXT states (story `html-tokenizer-text`), character
references (story `html-character-references`), decoding bytes (story `html-input-stream`).
Scripts: nothing in these states needs a script engine.
