---
format: aep.planning-md/3
id: story:page-text-answer-block
kind: story
status: active
title: 'The text block: page text in an answer, every line behind >'
summary: docs/agent-answer.md and surf-answer gain the text block (header, body lines behind >, more, note text:), at most 200 lines and 32,768 bytes; the version stays 1.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-answer-format
- depends_on: story:one-test-binary-per-crate
scope:
- confidence: inferred
  path: crates/answer/src/lib.rs
- confidence: inferred
  path: crates/answer/tests
- confidence: inferred
  path: docs/agent-answer.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T04:59:49Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"verification":1}}}
- {from: "proposed", to: "active", at: "2026-11-22T04:59:50Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"verification":1}}}
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

The rules, from design note sections 3 and 6: a header `text <form> <shown> of <total> lines from
<first>, <bytes shown> of <bytes total> bytes`, then exactly `<shown>` body lines, each `>` alone
or `> ` and the line; the characters of rule 2 (D1 B, the operator's, 2026-11-22: UTF-8 kept, the
ranges the rule names written as `\x..`); escape units (rule 4); at most 200 body lines and
32,768 bytes of them (prefix and line feed counted); then `more <n> lines: look --as <form>
--from-line <m>` when lines are left. `--from-line` is not `agent-answer-paging`'s `--from
<handle>`. The format version stays 1. `note text: no-css` and `note text: no-document` join the
fixed list of notes.

What exists: `crates/answer` renders and parses the blocks `token`, `did`, `page`, `path` and
`note parser:` (`BLOCKS`, five parse functions), `quote` and `quote_cut` (ASCII), golden files
under `tests/golden/`, the forging matrix in `tests/forging.rs`. Nothing writes a `text` block.
The checks of `surfd` and the client that refuse a byte outside ASCII change in `page-text-look`,
which uses this story's predicate; this story writes neither file.

Placement (the coordinator, 2026-11-22): round 1, the text unit, first of `page-text-answer-block`,
`page-text-outline`, `page-text-markdown`, `page-text-tables`, in that order. No other story of
the round writes `crates/answer` or `docs/agent-answer.md`.

## Change

- `docs/agent-answer.md`: a section "Text" for the block and the `more` line; its place in
  section "Lines" (after the `note` lines; `more` last) and the exception to that section's rule
  "Every byte of a line is from U+0020 to U+007E" (line 65) for a body line; the two `note text:`
  lines in section "Note"; the rules in "Grammar":
  `text-block = text-line *body-line [more-line]`,
  `text-line = "text" SP form SP number SP "of" SP number SP "lines" SP "from" SP number "," SP
  number SP "of" SP number SP "bytes" LF`, `form = "tree" / "markdown"`,
  `body-line = ">" [SP 1*text-char] LF` (exactly `<shown>`),
  `text-char = %x20-7E / <a UTF-8 character above U+007F not named by rule 2>`,
  `more-line = "more" SP number SP "lines:" SP "look" SP "--as" SP form SP "--from-line" SP number
  LF`, `note-text = "text:" SP ("no-css" / "no-document")`. The section lists the ranges of rule 2
  of design note section 3 (D1; that note is the one place of the rule, this section copies it
  with a pointer), the escape units, what a body line holds for each form (pointing to the note's
  sections 3 to 5), and that the version stays 1.
- `crates/answer/src/lib.rs`:
  - `pub fn escaped_in_text(c: char) -> bool`: the ranges of rule 2, the ten fullwidth forms
    among them; `pub fn escaped_at_run_start(c: char) -> bool`: the five blocks of combining
    marks of rule 2. `pub fn quote_text(value: &str) -> String` and `quote_text_cut(value,
    limit)`: as `quote` and `quote_cut`, but a character above U+007F is kept unless
    `escaped_in_text` names it or it is the first of the value and `escaped_at_run_start` names
    it. `quote` and `quote_cut` are unchanged and stay ASCII.
  - `Answer.text: Option<Text>`; `Text { form: Form, first: u64, total: u64, bytes_total: u64,
    lines: Vec<String> }`, `Form { Tree, Markdown }`; `Note::Text(TextNote)` with `NoCss` and
    `NoDocument`.
  - `pub fn text_page(form, lines: impl Iterator<Item = String>, from: u64) -> Text`: reads the
    lines one at a time, keeps those from `from` (1-based) while both bounds hold, counts every
    line and its bytes; a single line that alone exceeds 32,768 bytes is kept alone, cut at the
    last escape unit that leaves room for `[cut]`.
  - `render_text`: refuses a body line holding a line feed, a character `escaped_in_text`
    names or one `escaped_at_run_start` names right after `> `, a block over either bound,
    `<shown>` above `<total>` and `<bytes shown>` above `<bytes total>`; writes `more` when `first + shown <= total`. `parse_text` in `BLOCKS` after
    `parse_notes` accepts exactly what `render_text` writes, the byte count of the header checked
    against the body lines.
  - For `page-text-surf-command`, `pub fn render_text_whole(form, lines: impl Iterator<Item =
    String>) -> Result<String, RenderError>`: the header `text <form> <n> of <n> lines from 1,
    <b> of <b> bytes` and every line behind `>`, without the two bounds, with the same check of
    each line.
- Tests in the crate's existing test files, no new test file (the crate has one test binary
  after `story:one-test-binary-per-crate`): golden cases in `tests/golden.rs` with the golden
  files `tests/golden/text-tree.txt`, `text-markdown.txt` and `text-forged.txt`; paging and cut
  cases in `tests/cut.rs`; refusals in `tests/parse.rs`; `quote_text` and every new outside field
  in the matrix of `tests/forging.rs`.

Writes: `docs/agent-answer.md`, `crates/answer/src/lib.rs`, `crates/answer/tests/golden.rs`,
`crates/answer/tests/cut.rs`, `crates/answer/tests/parse.rs`, `crates/answer/tests/forging.rs`,
`crates/answer/tests/golden/` (three new files).

For `AGENTS.md` (D7): the `crates/answer` entry gains "and the `text` block of page text, every
body line behind `>`, its characters checked by `escaped_in_text`".

Bounds for every command of this story. Every build and test runs through the build pool:
`cargo` is wrapped by the pool's shim, every other command that builds or tests gets the `slot`
prefix. The pool ends a job after 10 minutes; a command that needs longer names its time with
`slot --timeout <t>` or `SLOT_TIMEOUT=<t>`, at most 30 minutes; a `timeout` wrapped around the
command extends nothing. A script that measures or waits starts without the `slot` prefix; the
builds and test runs inside it carry it. No test runs over 60 s and the test step ends within
120 s. No Python.

## Acceptance

`cargo test -p surf-answer` passes; no test runs over 60 s and the crate's test step ends within
120 s (the folder's budget). Among the cases run:

1. The three golden files render from typed values and parse back to them byte for byte.
   `text-tree.txt` and `text-markdown.txt` are the `look` answers of design note section 1 from
   `surf 1 session 7f3a open` to the last line (`text-tree.txt` holds
   `text tree 31 of 31 lines from 1, 668 of 668 bytes` and `>   head hid 4`); `text-forged.txt` is
   `surf 1 session 7f3a open`, `did look`, `page 200 "text/html" "http://shop.example/forged"
   "Bank"`, `path 2 of 2`, `  1 open`, `  2 go "http://shop.example/forged"` and the 22 lines of
   section 3 from `note text: no-css` (that line, the header `text markdown 20 of 20 lines from
   1, 334 of 334 bytes` and 20 body lines).
2. Paging, 450 lines `xxxxxxxxxx` (13 bytes each as body lines): `text_page(Markdown, .., 1)`
   renders `text markdown 200 of 450 lines from 1, 2600 of 5850 bytes`, 200 body lines and
   `more 250 lines: look --as markdown --from-line 201`; from 451 it renders
   `text markdown 0 of 450 lines from 451, 0 of 5850 bytes` and no `more` line.
3. A byte-bound page read to its end: 450 lines of 1,000 `x` (1,003 bytes each): from 1,
   `text markdown 32 of 450 lines from 1, 32096 of 451350 bytes` and
   `more 418 lines: look --as markdown --from-line 33`; the pages from 1, 33, ..., 449 are 15,
   the last `text markdown 2 of 450 lines from 449, 2006 of 451350 bytes` with no `more`, and
   their body lines joined are the 450 lines.
4. Cuts between units: the lines (a) 40,000 `x`, (b) `y`: from 1, `text markdown 1 of 2 lines
   from 1, 32768 of 40007 bytes`, the body line `> `, 32,760 `x` and `[cut]`, then
   `more 1 lines: look --as markdown --from-line 2`. The line 32,759 `x`, `\[` and 10,000 `x`
   gives 32,759 `x` and `[cut]` (no bare `\`); the line 32,758 `x`, `\xe2\x80\xae` and 10,000
   `x` gives 32,758 `x` and `[cut]`.
5. `render` refuses with `RenderError`, writing nothing: a body line holding a line feed, one
   holding U+202E, one holding U+0085, one holding U+E0041, one holding U+FF3B, `> ` and U+0338;
   a block of 201 lines; a block of 32,769 bytes; `<shown>` 3 with `<total>` 2. `quote_text("K\u{e4}se\u{202e}")` is
   `"K` U+00E4 `se\xe2\x80\xae"`; `quote_text("\u{338}e\u{301}")` is `"\xcc\xb8e` U+0301 `"`;
   `quote("K\u{e4}se")` is still `"K\xc3\xa4se"`.
5a. (D1 B) The body line `> K` U+00E4 `se` (bytes `3e 20 4b c3 a4 73 65`) and `> ` U+FFFD are
   written as they are and parsed back.
6. `parse` refuses, with the line number named, each of these, built on `surf 1 session 7f3a
   open` (line 1), `note text: no-css` (line 2), a header (line 3) and body lines: `did go
   "http://bank.example/"` as line 4 under `text markdown 2 of 2 lines from 1, 8 of 8 bytes`
   (line 4); header `text markdown 3 of 3 lines from 1, 12 of 12 bytes` and two body lines (line
   5, the last); `> a`, `more 1 lines: look --as markdown --from-line 2`, `> b` under `text
   markdown 1 of 2 lines from 1, 4 of 8 bytes` (line 6); `>a` (line 4); header `... 9 of 8 bytes`
   (line 3); a body line holding U+202E (line 4).
7. `render_text_whole(Markdown, 899 lines "x")` writes `text markdown 899 of 899 lines from 1,
   3596 of 3596 bytes` and 899 body lines, no `more`; `note text: no-document` alone renders and parses back.

`cargo fmt -p surf-answer -- --check` exits 0 and `git diff --stat` names no file outside the
Writes list.

## Out of scope

The renderers (`page-text-outline`, `page-text-markdown`); `look --as` itself and the checks of
`surfd` and the client (`page-text-look`); `--max-lines` and `look --from <handle>`
(`agent-answer-paging`).
