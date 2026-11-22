---
format: aep.planning-md/3
id: story:select-content-rows
kind: story
status: draft
title: The content of a select has a rule for what a reader sees
summary: surf-page hides the copy in a selectedcontent and reads everything else inside a select as any element; a rule per kind of content decides which rows are visible
relations:
- decomposes: epic:html-parser
scope:
- confidence: cited
  path: crates/page-text/tests/cases/select.rs
- confidence: cited
  path: crates/page/src/lib.rs
- confidence: cited
  path: crates/page/tests
- confidence: cited
  path: docs/design/html-parser.md
- confidence: cited
  path: docs/design/page-as-text.md
revision: 6
---
## Why

The tree builder follows the HTML Standard snapshot `0e292f402d0c3dd17fe00c1bb2ef3a3c94cd8bce`
("Last Updated 20 November 2026", the snapshot `docs/design/html-parser.md` pins). That snapshot
has no "in select" insertion mode: a `select` start tag leaves the insertion mode as it is, and
what follows it is parsed by the rules of "in body" (13.2.6.4.7). In
`crates/html/src/tree_builder/modes.rs` only the `select`, `input`, `hr`, `option` and
`optgroup` start tags look for a `select` in scope; every other tag inside a `select` becomes
an element under it, as anywhere in the body. The snapshot's content model of `select` (4.10.7)
admits "zero or one button elements if the select is a drop-down box, followed by zero or more
option elements, optgroup elements, hr elements, script-supporting elements, noscript elements,
or div elements". Content outside that model is still parsed into the tree, as the standard's
parsing rules build it.

Example, the case `webkit02.dat` (line 558 of the vendored data), which surf passes
(`file wpt/html/syntax/parsing/resources/webkit02.dat 49/49/49` in
`crates/html-conformance/expected/tree-construction.txt`):

    <select><div><i></div><option>option

builds `select` > `div` > `i`, and `select` > `i` > `option` > "option": an `i` element, which
the content model does not admit, is a child of the `select` and holds the option.

What a reader of the page sees: `surf-page` has one rule for the content of a `select`. The
copy of the selected option in a `selectedcontent` the parser fills is hidden and lists no row
(`is_hidden` and `hidden_roots` in `crates/page/src/lib.rs`, item 6 of
`docs/design/html-parser.md`, the case
`adversary_select::a_link_in_the_selected_option_is_listed_once`). Everything else inside a
`select` is read as any element: the link in the option of that case is a visible `Link` row,
no rule hides content of another kind, and the crate documentation says "a `select`'s options
are not read". What the two text forms of `surf-page-text` print for content of a `select`
other than the copy (`crates/page-text/tests/cases/select.rs` covers the copy): not known.

A browser shows a drop-down `select` as one control and its options only in the picker. An
agent that reads a row for a link inside an option can follow a link a person could reach
only with the picker open, and surf has no command that opens a picker.

## Change

- `crates/page/src/lib.rs` (`is_hidden`, `hidden_roots` and the crate documentation): a rule
  for each kind of content of a `select`: an `option` and what it holds, an `optgroup`, the
  `button` admitted as the first child, a `div`, and content the content model does not admit.
  The rule is stated as surf's reading where the snapshot's rendering section does not decide
  it, as the `selectedcontent` rule is.
- `crates/page/tests/`: one case per rule.
- `crates/page-text/tests/cases/select.rs`: one case per rule that changes what a text form
  prints.
- `docs/design/html-parser.md` item 6 ("What the rows hold") and `docs/design/page-as-text.md`:
  the rules.
- `crates/html/src/`: no change; the tree is the one the snapshot's parsing rules build.

Not known, and read first: what the snapshot's rendering section says about the content
of a drop-down `select`, of a list box (`multiple` or a display size over 1) and of the
picker; whether the `button` admitted as the first child stays a row.

## Acceptance

When the story is implemented:

1. A case for each element kind the rule decides (name pattern `select_content_*` in
   `crates/page/tests/`) passes and shows the `hidden` value the rule gives; a list box and a
   drop-down each have a case.
2. `adversary_select::*`, `tree_tables::*`, `tree_template::*` and the `select` cases of
   `elements::*` pass; a case whose expectation the rule changes is listed in the report with
   the old and the new value.
3. Both text forms print each element of a `select` at most once, and print no text of an
   element the rule hides (a case for each form).
4. `task html-conformance` prints `html-conformance: 9107 cases, 9095 run, 9095 passed` or
   more passed.
5. For an input of n and of 2n `select` elements with one link in their option each, the
   count of steps `actionable_elements` takes at 2n is at most twice the count at n plus 50.
