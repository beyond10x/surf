---
format: aep.planning-md/3
id: story:page-text-tables
kind: story
status: draft
title: Tables in a page's Markdown
summary: 'An HTML table becomes a GitHub table: header row, colspan and rowspan as empty cells, nested tables after their outer one, cells escaped and cut.'
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:page-text-markdown
- depends_on: story:html-tree-tables
scope:
- confidence: inferred
  path: crates/page-text/src/markdown.rs
- confidence: inferred
  path: crates/page-text/src/tables.rs
- confidence: inferred
  path: crates/page-text/tests/cases/tables.rs
- confidence: inferred
  path: crates/page-text/tests/main.rs
revision: 3
---
## Context

The operator, 2026-11-22: "Like render tables and stuff." `docs/design/page-as-text.md`
(proposed) section 4, the `table` item: a GitHub Flavored Markdown table per HTML table, with
spans, a header row and nested tables, every cell's text escaped as the rest of the Markdown is
(`|` included). `page-text-markdown` writes a `table` as one paragraph of its text, because until
`html-tree-tables` the tree builder ignores `tr` and `td` start tags and a table's cells are not
in the tree. After that story they are, with the `tbody` the parser implies.

Round 1, after `page-text-markdown` and `html-tree-tables`: it writes only `crates/page-text`.
Characters outside ASCII in a cell follow rule 2 of design note section 3 (D1); no acceptance
case of this story depends on it.

## Change

- `crates/page-text/src/markdown.rs` (and a `tables` submodule if wanted): a `table` element
  that is not hidden is written as:
  - the `caption`'s text as a paragraph before it, when there is one;
  - a header row: the first row of the first `thead`, or the first row when all its cells are
    `th`; otherwise a row of empty cells; then the delimiter row `| --- |` per column;
  - one line per other row, in tree order over `thead`, `tbody`, `tfoot` and rows directly in
    the table; columns: the widest row after `colspan` and `rowspan` (the HTML Standard's
    table processing model for the slots, from memory, confirmed here), at most 20, a 21st
    column holding `[cut]`; a spanned slot is an empty cell, the text in the slot of the cell
    that spans;
  - a cell: its inline text, its blocks and `br` joined by one space, escaped (`|` as `\|`), cut
    at 200 characters and ending in `[cut]`; an empty cell is written `|  |`;
  - a table inside a cell: `[table <n>]` in that cell (n from 1 in document order over nested
    tables), and after the outer table an empty line, the line `[table <n>]`, an empty line and
    that table, in the same form.
- The test `fruit_shop_outline` (the outline of the whole page of design note section 1, its
  31 lines with the table), which `page-text-outline` could not have before `html-tree-tables`.

Writes: `crates/page-text/src/markdown.rs`, `crates/page-text/src/` (a new `tables.rs` if
wanted), `crates/page-text/tests/cases/tables.rs` (new), `crates/page-text/tests/main.rs` (one
module line).

## Acceptance

`cargo test -p surf-page-text` passes, each test within 60 s, the step within 120 s, with:

1. `fruit_shop_markdown`: the page of design note section 1 gives the 14 Markdown lines of its
   `look --as markdown` answer without the `> ` prefix, among them `| Fruit | Price | Stock |`,
   `| --- | --- | --- |`, `| Apple | 0.50 | 120 |`; `fruit_shop_outline` gives its 31 outline
   lines.
2. `forged_page_markdown`: the forged page of section 3 gives its 20 lines, among them `|  |  |`,
   `| --- | --- |` and `| a\|b | c d |`.
3. `colspan_leaves_empty_cells`: `<table><tr><th colspan=2>A<th>B<tr><td>1<td>2<td>3</table>`
   gives `| A |  | B |`, `| --- | --- | --- |`, `| 1 | 2 | 3 |`.
4. `rowspan_leaves_empty_cells`: `<table><tr><th>A<th>B<tr><td rowspan=2>1<td>2<tr><td>3</table>`
   gives `| A | B |`, `| --- | --- |`, `| 1 | 2 |`, `|  | 3 |`.
5. `a_nested_table_follows_its_outer_table`:
   `<table><tr><td>x<td><table><tr><td>y</table></table>` gives `|  |  |`, `| --- | --- |`,
   `| x | [table 1] |`, ``, `[table 1]`, ``, `|  |`, `| --- |`, `| y |`.
6. `wide_and_long_cells_are_cut`: a row of 25 cells gives 21 columns, the last `[cut]`; a cell of
   300 characters gives 200 and `[cut]`.
7. `a_hostile_cell_cannot_end_its_row`: `<table><tr><td>|&#10;&lt;/td&gt; | --- |</table>` gives
   exactly three lines, `|  |`, `| --- |` and `| \| \</td\> \| --- \| |`.

`cargo fmt -p surf-page-text -- --check` exits 0.

## Out of scope

Layout of a table (widths, alignment from CSS); `align` attributes; a table in `template`
contents (not walked).
