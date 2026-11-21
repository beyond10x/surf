---
format: aep.planning-md/3
id: story:html-tokenizer-text
kind: story
status: active
title: Tokenize RCDATA, RAWTEXT, script data and PLAINTEXT
summary: The text states and the appropriate end tag check; every tokenizer case tagged markup or text passes.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-tokenizer-markup
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 4}
---
## Context

The second tokenizer story. The tree builder switches the tokenizer into RCDATA (`title`,
`textarea`), RAWTEXT (`style`, `xmp`, `iframe`, `noembed`, `noframes`, and `noscript` when
scripting is on), script data (`script`) and PLAINTEXT (`plaintext`); an end tag ends them only
when it is an appropriate end tag for the last start tag. The tokenizer data sets these with
`initialStates` and `lastStartTag`; those cases are tagged `text` by the rule of
`docs/design/html-parser.md` (story `html-decisions`, item 8). Named from memory, to be
confirmed by this story. Story `html-tokenizer-markup` wrote the other states in the same
files, so this story follows it.

## Change

- `crates/html/src/tokenizer.rs` and its submodules: the RCDATA, RAWTEXT, script data, script
  data escaped and double escaped, and PLAINTEXT states with their less-than-sign and end tag
  states, the appropriate end tag check, and the entry point taking the last start tag.
- `crates/html-conformance/expected/tokenizer.txt`: the counts raised.

Writes: `crates/html/src/tokenizer.rs`, `crates/html/src/tokenizer/`,
`crates/html-conformance/expected/tokenizer.txt`.

## Acceptance

`slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 printing `tag markup
1979/1979/1979` and `tag text 328/328/328` (the `markup` and `text` counts of `html-inventory`),
tokens and parse error codes both compared.

## Out of scope

Character references inside RCDATA and attribute values (story `html-character-references`).
Scripts: the script data states are tokenized in full and need no engine; what they leave
unsupported is the insertion point that `document.write` writes at, which the tokenizer never
has. The note lists it as an open requirement of its section (`html-sections`).
