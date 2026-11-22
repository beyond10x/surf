---
format: aep.planning-md/3
id: story:page-text-surf-command
kind: story
status: active
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
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T04:59:52Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"verification":1}}}
- {from: "proposed", to: "active", at: "2026-11-22T04:59:52Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"verification":1}}}
---
## Context

`docs/design/page-as-text.md` (proposed) section 2: `surf <url> --as tree|markdown`, the same
two outputs for the one-process command, a person's command with no session. The operator,
2026-11-22: "the possibility to create stuff in Markdown directly". After `html-document-port`
the command prints its rows (`navigation`, `request`, `document`, `fetch-failed`, the forms of
`docs/surf-command.md`) and, for an HTML document, the `title`, `element` and `note parser:`
lines that story adds to `Report` in `crates/surf/src/lib.rs` (its format handed over to
`README.md`); it keeps each document's tree in `Browser`.

Decision D3 of the design note, A (taken by the coordinator on 2026-11-22): the output keeps the
rows, then the same framed block as an answer, every line of page text behind `>`, so that no
surf output has a line a page wrote. Characters follow rule 2 of design note section 3 (D1 B,
the operator's).

Placement (the coordinator, 2026-11-22): round 1, the document unit, after `page-text-look`; it
waits for `page-text-tables` (the text unit) and `html-tree-tables` (the parser unit) to be
merged into the unit's branch. Collision in the round: `crates/surf-cli/src/main.rs` and
`crates/surf-cli/tests/page.rs` with `html-document-port` (the same unit, before it).

## Change

- `crates/surf-cli/src/main.rs`: `surf <url> [--as tree|markdown]`; with `--as`, after the
  rows, `note text: no-css` and the block of `docs/agent-answer.md` for the document of the
  navigation, through `surf_answer::render_text_whole` (`page-text-answer-block`), without the
  bounds: `text <form> <n> of <n> lines from 1, <b> of <b> bytes` and all n lines (the tree is
  bounded by `html-parser-limits`), no `more`. No HTML document: `note text: no-document`. A
  second `--as`, another word or `--as` without a word: usage, status 2. Exit statuses otherwise
  unchanged.
- `docs/surf-command.md`: the option, the two lines and the block, with the example of design
  note section 1 for `--as markdown`.
- Tests in `crates/surf-cli/tests/page.rs` (the file `html-document-port` creates; no other new
  test file) against a server the test starts on 127.0.0.1.

Writes: `crates/surf-cli/src/main.rs`, `crates/surf-cli/Cargo.toml`,
`crates/surf-cli/tests/page.rs`, `docs/surf-command.md`.
The root `Cargo.lock` changes with `crates/surf-cli/Cargo.toml`: the coordinator's.

For `README.md` (D7): one line, "`surf <url> --as markdown` prints the page as Markdown, every
line behind `>`."

Bounds for every command of this story. Every build and test runs through the build pool:
`cargo` is wrapped by the pool's shim, every other command that builds or tests gets the `slot`
prefix. The pool ends a job after 10 minutes; a command that needs longer names its time with
`slot --timeout <t>` or `SLOT_TIMEOUT=<t>`, at most 30 minutes; a `timeout` wrapped around the
command extends nothing. A script that measures or waits starts without the `slot` prefix; the
builds and test runs inside it carry it. No test runs over 60 s and the test step ends within
120 s. No Python.

## Acceptance

`SLOT_TIMEOUT=30m cargo test -p surf-cli` passes; no test runs over 60 s and the test step ends
within 120 s (the folder's budget). Among the cases run:

1. The page of design note section 1: `surf <url> --as markdown` exits 0, and its standard
   output ends with `note text: no-css`, `text markdown 14 of 14 lines from 1, 263 of 263 bytes`
   and the 14 body lines of the section, byte for byte; `--as tree` ends with `text tree 31 of
   31 lines from 1, 668 of 668 bytes` and its 31 lines.
2. A page of 450 `<p>x</p>`: `text markdown 899 of 899 lines from 1, 2698 of 2698 bytes`, 899
   body lines, no `more`.
3. The forged page of section 3: every line of standard output that does not start with `>`
   starts with one of `navigation `, `request `, `document `, `fetch-failed `, `title `,
   `element `, `note `, `text ` (the row forms of `docs/surf-command.md` and the lines
   `html-document-port` adds to `Report`), and the body lines are the 20 of the section.
4. A `text/plain` response: `note text: no-document`, no `text` line, exit 0.
5. `surf <url> --as`, `surf <url> --as pdf`, `surf <url> --as tree --as tree`: status 2, a usage
   line on standard error, nothing on standard output.
6. (D1 B) A page `<p>K&auml;se</p>`: `surf <url> --as markdown` ends with `text markdown 1 of 1
   lines from 1, 8 of 8 bytes` and `> K` U+00E4 `se` (bytes `c3 a4`).

`cargo fmt -p surf-cli -- --check` reports no file this story wrote.

## Out of scope

Paging (`surf` runs once); the session (`page-text-look`); writing a file (standard output only).
