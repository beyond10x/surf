---
format: aep.planning-md/3
id: story:page-text-surf-command
kind: story
status: draft
title: surf <url> --as tree|markdown
summary: The one-process command prints its rows and then the outline or the Markdown of the page in the same framed text block, whole.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:page-text-answer-block
- depends_on: story:page-text-tables
- depends_on: story:html-document-port
- depends_on: story:runner-surf-cli-package
scope:
- confidence: inferred
  path: crates/surf-cli/Cargo.toml
- confidence: inferred
  path: crates/surf-cli/src/main.rs
- confidence: inferred
  path: crates/surf-cli/tests/page.rs
- confidence: inferred
  path: docs/surf-command.md
revision: 3
---
## Context

`docs/design/page-as-text.md` (proposed) section 2: `surf <url> --as tree|markdown`, the same
two outputs for the one-process command, a person's command with no session. The operator,
2026-11-22: "the possibility to create stuff in Markdown directly". After `html-document-port`
the command prints its rows and, for an HTML document, `title`, `element` and `note parser:`
lines (`docs/surf-command.md`); it keeps each document's tree in `Browser`.

Decision D3 of the design note, A (taken by the coordinator on 2026-11-22): the output keeps the
rows, then the same framed block as an answer, every line of page text behind `>`, so that no
surf output has a line a page wrote.

Round 1, placed by the coordinator in the document unit after `html-document-port` (same files,
one unit, in order). Collision in the round: `crates/surf-cli/src/main.rs` and
`crates/surf-cli/tests/page.rs` with `html-document-port` (the same unit, after it).

Characters outside ASCII follow rule 2 of design note section 3 (D1, with the operator); case 6
is the line B changes.

## Change

- `crates/surf-cli/src/main.rs`: `surf <url> [--as tree|markdown]`; with `--as`, after the
  rows, `note text: no-css` and the block of `docs/agent-answer.md` for the document of the
  navigation, through `surf_answer::render_text_whole` (`page-text-answer-block`), without the
  bounds: `text <form> <n> of <n> from 1` and all n lines (the tree is bounded by
  `html-parser-limits`), no `more`.
  No HTML document: `note text: no-document`. A second `--as`, another word or `--as` without a
  word: usage, status 2. Exit statuses otherwise unchanged.
- `docs/surf-command.md`: the option, the two lines and the block, with the example of design
  note section 1 for `--as markdown`.
- Tests in `crates/surf-cli/tests/page.rs` (the file `html-document-port` creates) against a
  server the test starts on 127.0.0.1.

Writes: `crates/surf-cli/src/main.rs`, `crates/surf-cli/Cargo.toml`,
`crates/surf-cli/tests/page.rs`, `docs/surf-command.md`.

For `README.md` (D7): one line, "`surf <url> --as markdown` prints the page as Markdown, every
line behind `>`."

## Acceptance

`cargo test -p surf-cli` passes, each test within 60 s, the step within 120 s, with:

1. The page of design note section 1: `surf <url> --as markdown` exits 0, and its standard
   output ends with `note text: no-css`, `text markdown 14 of 14 from 1` and the 14 body lines
   of the section, byte for byte; `--as tree` ends with `text tree 31 of 31 from 1` and its 31
   lines.
2. A page of 450 `<p>x</p>`: `text markdown 899 of 899 from 1`, 899 body lines, no `more`.
3. The forged page of section 3: every line of standard output that does not start with `>` is
   a row, `title`, `element`, `note` or `text` line of `docs/surf-command.md`.
4. A `text/plain` response: `note text: no-document`, no `text` line, exit 0.
5. `surf <url> --as`, `surf <url> --as pdf`, `surf <url> --as tree --as tree`: status 2, a usage
   line on standard error, nothing on standard output.
6. (D1) A page `<p>K&auml;se</p>`: `surf <url> --as markdown` prints `> K\xc3\xa4se`; under D1 B
   `> Käse`.

`cargo fmt -p surf-cli -- --check` reports no file this story wrote.

## Out of scope

Paging (`surf` runs once); the session (`page-text-look`); writing a file (standard output only);
UTF-8 output (decision D1).
