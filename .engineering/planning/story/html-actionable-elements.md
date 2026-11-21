---
format: aep.planning-md/3
id: story:html-actionable-elements
kind: story
status: active
title: Find a document's title and actionable elements in the tree
summary: surf-page lists links and form controls in document order with kind, target or value and the four flags, and the title.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-tree-body
- depends_on: story:html-document-spec
scope:
- confidence: cited
  path: crates/page
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 4}
---
## Context

The rows that story `html-document-spec` specifies are computed from the tree: which elements
are actionable, of which kind, with which target, value and flags, and the document's title.
The rules are the HTML Standard's (the snapshot `docs/design/html-parser.md` names): the title element
of a document and the `document.title` getter (the first HTML `title` element in tree order, its
child text content, ASCII whitespace stripped and collapsed); hyperlinks (`a` and `area` with an
`href`); the `input` element's type states and their default value and default checkedness; the
`textarea` default value (its child text content); "disabled" for form controls (the `disabled`
attribute, or a `fieldset` ancestor with a `disabled` attribute outside that fieldset's first
`legend` child); the `required` attribute where it applies. The mapping from element and `input`
type state to the nine kinds is the table of item 6 of `docs/design/html-parser.md` (story
`html-decisions`), an unknown or missing `type` being the Text state; the `type` keyword is
compared ASCII case-insensitively.

"Hidden" is item 5 of the note, not the `hidden` attribute alone: a node is hidden when it or an
ancestor has `aria-hidden="true"` or matches a rule of the HTML Standard's user-agent style sheet
in 15.3.1 "Hidden elements", which needs no style computation: the elements `area, base,
basefont, datalist, head, link, meta, noembed, noframes, param, rp, script, style, template,
title`; `[hidden]` on any element but `embed`, whatever its value (`until-found` included);
`input[type=hidden i]`. The style sheet's rules are in the HTML namespace (its `@namespace`), so
the element and attribute rules apply to HTML elements only; `aria-hidden` applies to any
element. `noscript` is not hidden, because surf parses with scripting disabled. An `area` is not
hidden by its own element rule: it is drawn through its image's map. A node that is not an
element (a text node) is hidden when an ancestor element is. The same function serves the label
computation of story `html-accessible-name` (wave 10).

Decisions that bind this story (`docs/plan/agent-plan-revision-1.md`): a `select` element is
listed with kind `select` and no value; option selectedness is computed by
`story:agent-form-port` (D11). The tests that read `select` and `template` parsing are in
`story:html-tree-tables` and `story:html-tree-template`, where that parsing is written (D12);
this story keeps its place after `story:html-tree-body`. The document base URL is a function in
this crate written by `story:agent-follow-links` (D10), not here.

What the tree of wave 8 gives (measured on `main` at `5129b50` with a probe over `surf-html`):
`NOT_YET_PARSED` names `misnested-formatting`, `tables-and-select`, `template` and
`foreign-content`, whose tokens go through in body's "any other start tag" and "any other end
tag". So: a `select` with `option` children in body has the standard's shape; a `form` or a
control inside a `table` is nested where the generic rules put it (`tr` and `td` start tags are
ignored), not foster-parented; `<a href=1>one<a href=2>two` nests the second `a` inside the first
(flat siblings after `story:html-tree-formatting`); an `svg` and its `title` and `a` children are
HTML elements, so an SVG `title` can be the document's title until `story:html-tree-foreign`; a
`template` in body gets its template contents, which `Tree::descendants` leaves out; a `template`
in head is popped in place of `head`, so `body` and the whole page become children of `head`
and every element of such a page is hidden by the `head` rule, until `story:html-tree-template`
(the defect is the tree's: `crates/html/src/tree_builder/modes.rs`, the `template` rule of in head
and "anything else"). This story reads the tree as it is; it adds no case for those shapes, and
its tests use none of them (no `table`, no `template`, no `svg` or `math`, no `a` inside an `a`,
no misnested formatting elements), so that they hold before and after the later tree stories.

What `surf-html` already has, and all this story reads: `Tree::root`, `Tree::data` (an
element's namespace, local name and attributes, its template contents), `Tree::children`,
`Tree::parent` and `Tree::descendants`; `NodeId` is `Copy`, `Eq`, `Hash` and `Ord`, and its
index is private. No reader is added to `surf-html`. The parser has no depth limit until
`story:html-parser-limits` (wave 10): a tree of 100,000 nested elements parses (a test of wave 8),
so nothing here recurses over the depth, and nothing is quadratic in it. Measured in a debug
build: 20,000 nested `select` parse in 34 to 45 ms; one walk up the parents per element then
takes 1.5 to 2.0 s (200,050,000 steps), and 100,000 would take about 25 times that.

The code is a library over `surf-html`'s tree, in a crate of its own so that it can be written
beside the later tree stories, which change `crates/html`. No crate of the product depends on it
until `story:html-document-port` (wave 11), which maps `Kind` to the generated
`surf.document.ElementKind` and each element's `NodeId` to its row; `story:html-accessible-name`
(wave 10) adds the label to each element and calls `is_hidden` for the nodes it walks.

## Change

- `crates/page/` (package `surf-page`, version `0.1.0`, edition 2024, `publish = false`,
  `license = "Apache-2.0"`, `#![forbid(unsafe_code)]`, depends on `surf-html` by path only, no
  dev-dependency):
  - `pub fn title(tree: &Tree) -> String`: the `document.title` value, the empty string when the
    document has no HTML `title` element. Template contents are not searched.
  - `pub enum Kind { Link, Button, Text, Password, Checkbox, Radio, File, Select, Textarea }`
    with `Kind::name(self) -> &'static str` giving `link`, `button`, `text`, `password`,
    `checkbox`, `radio`, `file`, `select`, `textarea` (the spelling of the specification's
    `ElementKind`).
  - `pub struct ActionableElement` with public fields `node: NodeId`, `kind: Kind`,
    `target: Option<String>` (a link's `href` as written, `Some` also when empty; `None` for every
    other kind), `value: Option<String>`, and `hidden`, `disabled`, `checked`, `required`
    (`bool`). Story `html-accessible-name` adds a field for the label.
  - `pub fn actionable_elements(tree: &Tree) -> Vec<ActionableElement>`: every actionable HTML
    element under the root in tree order, as `Tree::descendants` gives it (template contents left
    out, no special case for a `template` element's children). An `input` in the Hidden state is
    not listed. It computes the inherited flags (hidden, the disabled fieldsets above) in one walk
    with its own stack over `Tree::children`, so its time is linear in the number of nodes.
  - `value`: for the kinds `text` and `password`, the `value` attribute (the empty string when
    absent) after the value sanitization algorithm of the Text, Search, Telephone and Password
    states (newlines stripped) and of the URL and Email states (newlines stripped, then leading
    and trailing ASCII whitespace; for Email with `multiple`, each comma-separated part); for
    Number, Range, Color and the date and time states the attribute as written, a deviation the
    crate documentation names. `textarea`: its child text content. `file`: always the empty
    string (the value mode "filename" with no selected file; the `value` attribute is not read).
    `None` for `link`, `button`, `checkbox`, `radio` and `select`.
  - `checked`: the default checkedness (the `checked` attribute is present) of an `input` in the
    Checkbox or Radio Button state; `false` for every other element. The radio button group rule
    (a later checked radio unchecks the others of its group) needs the form owner and is
    `story:agent-form-port`'s; the crate documentation names it.
  - `disabled`: as Context says, for `button`, `input`, `select` and `textarea`; `false` for a
    link.
  - `required`: the `required` attribute is present and applies: `input` in the Text, Search,
    URL, Telephone, Email, Password, Date, Month, Week, Time, Local Date and Time, Number,
    Checkbox, Radio Button and File Upload states, `select`, `textarea`; `false` otherwise (Range,
    Color, the button states, `button`, links).
  - `pub fn is_hidden(tree: &Tree, node: NodeId) -> bool`: the rule of Context for any node, by
    a walk up `Tree::parent` (no recursion). Nothing that needs CSS is read (`epic:html-parser`,
    "Out of scope").
  - The crate documentation lists the rules taken, the deviations named above, and what is
    left to other stories (labels, form owner, option selectedness, base URL).
- Tests in `crates/page/tests/elements.rs`, each parsing a document with
  `surf_html::parse_document` and `ParseOptions::default()`:
  `title_is_the_first_title_with_whitespace_collapsed` (a second `title` in body is not read),
  `a_document_without_title_has_an_empty_title`, `links_are_a_and_area_with_href` (an empty
  `href` included, target as written), `an_a_without_href_is_not_a_link`,
  `input_type_states_map_to_kinds` (every row of the item 6 table: the 21 keywords other than
  `hidden`, a missing, an unknown and an upper-case `type`, `button`, `select` with no value,
  `textarea`), `a_hidden_input_is_not_listed`, `textarea_value_is_its_default_value` (the first
  line feed after the start tag dropped by the parser),
  `text_control_values_are_their_sanitized_default_values` (newlines stripped from a `text`
  value, whitespace stripped around a `url` value, a missing `value` gives the empty string, a
  `file` with a `value` attribute gives the empty string),
  `a_disabled_attribute_disables_a_control`,
  `a_control_in_a_disabled_fieldset_is_disabled`,
  `a_control_in_the_first_legend_is_not_disabled`,
  `checked_is_the_default_checkedness_of_a_checkbox_and_a_radio_button`,
  `required_applies_only_where_the_standard_says` (`text` and `checkbox` required; `range`,
  `submit` and `button` with the attribute not), `elements_come_in_document_order`,
  `hidden_attribute_on_an_ancestor_hides_an_element`, `hidden_until_found_hides_an_element`,
  `aria_hidden_true_on_an_ancestor_hides_an_element`, `an_element_inside_datalist_is_hidden`,
  `an_area_link_is_not_hidden_by_its_own_element`, `noscript_content_is_not_hidden` (the
  `noscript` in body: in head, its `a` is moved to body by the parser),
  `a_deep_tree_is_read_without_recursion` (100,000 nested `span` under a `div hidden`, a link at
  the bottom: one element, hidden; 20,000 nested `select`: 20,000 elements of kind `select`).

Writes: `crates/page/` (new: `Cargo.toml`, `src/lib.rs`, `tests/elements.rs`; more modules
under `src/` allowed). No file of `crates/html`. `Cargo.lock` gains a repository package
(`lock-check` then counts 23); the coordinator regenerates it and changes the numbers in the
documents.

For `AGENTS.md`, handed over in the report (D7), because the crate exists from this wave on: a
"Layout" entry, "`crates/page` (`surf-page`): the title and the actionable elements of a
`surf-html` tree, with their kind, target or value and the flags `hidden`, `disabled`, `checked`
and `required`; `is_hidden` is the rule of `docs/design/html-parser.md` item 5. No crate of the
product depends on it yet. Depends on `surf-html` only." Story `html-document-port` then hands
over only what it adds.

## Acceptance

In the unit's tree, with `RUSTFLAGS=-D warnings`: `cargo test -p surf-page --test elements`
prints `test result: ok. 21 passed; 0 failed` with the 21 tests named in Change;
`cargo tree -p surf-page -e normal,build,dev --prefix none` prints exactly two lines, starting
`surf-page v0.1.0` and `surf-html v0.1.0`; `cargo fmt -p surf-page -- --check` and
`RUSTDOCFLAGS=-D warnings cargo doc -p surf-page --no-deps` exit 0; `git diff --stat` names no
file outside `crates/page/` and `Cargo.lock`.

## Out of scope

Labels (story `html-accessible-name`); writing rows (story `html-document-port`); a control's
form owner, the radio button group rule and option selectedness (`story:agent-form-port`); the
document base URL (`story:agent-follow-links`); the trees of tables, `select` in a table,
templates and foreign content, and the cases that read them (`html-tree-tables`,
`html-tree-template`, `html-tree-foreign`). No reader is added to `surf-html`. Scripts: an
element a script would make clickable (an `onclick` on a `div`) is not actionable.
