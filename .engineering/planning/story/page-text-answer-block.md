---
format: aep.planning-md/3
id: story:page-text-answer-block
kind: story
status: draft
title: 'The text block: page text in an answer, every line behind >'
summary: docs/agent-answer.md and surf-answer gain the text block (header, body lines behind >, more, note text:), at most 200 lines and 32,768 bytes; the version stays 1.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-answer-format
scope:
- confidence: inferred
  path: crates/answer/src/lib.rs
- confidence: inferred
  path: crates/answer/tests
- confidence: inferred
  path: docs/agent-answer.md
revision: 3
---
## Context

The operator, 2026-11-22, after watching a `surf-agent` session whose answers held only `page`
and `path` ("No title, no content"), asked to see a page "as the tree structure rendered in some
way" and "in Markdown directly". `docs/design/page-as-text.md` (proposed) settles the two outputs
and how page text in bulk is framed in an answer. This story writes that framing into the answer
format, before any renderer or command uses it: the format is specified where a format can be,
its grammar in `docs/agent-answer.md` with golden tests (design `docs/design/agent-interface.md`
section 6), and `crates/answer` is its one implementation (D20, D26 of
`docs/plan/agent-plan-revision-2.md`).

The rules, from design note sections 3 and 6: a block `text <form> <shown> of <total> from
<first>`, then exactly `<shown>` body lines, each `>` alone or `> ` and the line; printable ASCII
only; at most 200 body lines and 32,768 bytes of them (prefix and line feed counted); then
`more <n> lines: look --as <form> --from <m>` when lines are left. The format version stays 1.
`note text: no-css` and `note text: no-document` join the fixed list of notes.

What exists: `crates/answer` renders and parses the blocks `token`, `did`, `page`, `path` and
`note parser:` (`BLOCKS`, five parse functions), `quote` and `quote_cut`, golden files under
`tests/golden/`, the forging matrix in `tests/forging.rs`. Nothing writes a `text` block.

The bytes a body line may hold are rule 2 of design note section 3, decision D1 (with the
operator, default B: UTF-8 kept but for the invisible characters). This story is written under
rule 2 as it stands; the grammar's `body-line` and acceptance case 5a are the lines B changes,
together with the checks of `crates/surfd/src/link.rs` and `crates/agent/src/wire.rs` that the
note's section 8 names.

Round 1 (`docs/plan/agent-waves.md`, rounds after wave 10): no other story of the round writes
`crates/answer` or `docs/agent-answer.md`.

## Change

- `docs/agent-answer.md`: a section "Text" for the block and the `more` line; its place in
  section "Lines" (after the `note` lines; `more` last); the two `note text:` lines in section
  "Note"; the rules in "Grammar":
  `text-block = text-line *body-line [more-line]`,
  `text-line = "text" SP form SP number SP "of" SP number SP "from" SP number LF`,
  `form = "tree" / "markdown"`, `body-line = ">" [SP 1*%x20-7E] LF` (exactly `<shown>`; D1),
  `more-line = "more" SP number SP "lines:" SP "look" SP "--as" SP form SP "--from" SP number LF`,
  `note-text = "text:" SP ("no-css" / "no-document")`. The section states what a body line holds
  for each form by pointing to the rules of `docs/design/page-as-text.md` sections 3 to 5 (the
  tree line, the escapes of Markdown, the meaning of `\x..`), and that the version stays 1.
- `crates/answer/src/lib.rs`: `Answer.text: Option<Text>`; `Text { form: Form, first: u64,
  total: u64, lines: Vec<String> }` and `Form { Tree, Markdown }`; `Note::Text(TextNote)` with
  `NoCss` and `NoDocument`; `pub fn text_page(form, lines: impl Iterator<Item = String>, from:
  u64) -> Text`, which reads the lines one at a time, keeps those from `from` (1-based) while
  both bounds hold and only counts the others, and cuts a single line that alone
  exceeds 32,768 bytes to fit, at a character boundary, ending in `[cut]`; `render_text`, which
  refuses a body line holding a line feed or a byte rule 2 (D1) does not allow, a block
  over either bound, and a `<shown>` above `<total>`, and writes `more` when
  `first + shown <= total`; `parse_text` in `BLOCKS` after `parse_notes`, accepting exactly
  what `render_text` writes. For `page-text-surf-command`, `pub fn render_text_whole(form,
  lines: impl Iterator<Item = String>) -> Result<String, RenderError>`: the header `text <form>
  <n> of <n> from 1` and every line behind `>`, without the two bounds, with the same check of
  each line's bytes.
- Tests, in the crate's existing test files (no new test binary): golden files
  `tests/golden/text-tree.txt`, `text-markdown.txt` and `text-forged.txt`, each the answer of
  `docs/design/page-as-text.md` (section 1 twice, section 3) byte for byte, with golden and
  round-trip cases; paging cases; forging cases.

Writes: `docs/agent-answer.md`, `crates/answer/src/lib.rs`, `crates/answer/tests/`.

For `AGENTS.md` (D7): the `crates/answer` entry gains "and the `text` block of page text, every
body line behind `>`".

## Acceptance

`cargo test -p surf-answer` passes, each test within 60 s and the crate's step within 120 s,
with these cases among those run:

1. The three golden files render from typed values and parse back to them byte for byte; the
   first two lines of the block in `text-tree.txt` are `text tree 31 of 31 from 1` and `> html`.
2. Paging, 450 lines of `x` (10 bytes each): `text_page(Markdown, .., 1)` renders
   `text markdown 200 of 450 from 1`, 200 body lines and
   `more 250 lines: look --as markdown --from 201`; from 451 it renders
   `text markdown 0 of 450 from 451` and no `more` line.
3. 450 lines of 1,000 bytes: `text markdown 32 of 450 from 1` and
   `more 418 lines: look --as markdown --from 33`.
4. One line of 40,000 bytes: one body line of at most 32,765 bytes ending in `[cut]`, then
   `more` naming the next line.
5. Hostile values, each refused by `render` with `RenderError` and none written: a body line
   holding a line feed, one holding U+202E, one holding the byte 0x7F.
5a. (D1) A body line holding `K` U+00E4 `se` is refused by `render` the same way; under D1 B it
   is written as is, and `parse` reads it back.
6. Hostile text, each refused by `parse` with the number of the line: a body line without `>`
   (`did go "http://bank.example/"` where line 3 of the block should be), a block with one body
   line fewer than its header says, a `>` line after `more`, `>x` without the space.

`cargo fmt -p surf-answer -- --check` exits 0 and `git diff --stat` names no file outside the
Writes list.

## Out of scope

The renderers (`page-text-outline`, `page-text-markdown`); `look --as` itself (`page-text-look`);
UTF-8 in the body (decision D1 of the design note, default: escaped); `--max-lines`
(`agent-answer-paging`).
