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
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 4}
---
## Context

The rows that story `html-document-spec` specifies are computed from the tree: which elements
are actionable, of which kind, with which target, value and flags, and the document's title.
The rules are the HTML Standard's (the snapshot `docs/design/html-parser.md` names): the title element
of a document and the `document.title` getter (first `title` in tree order, ASCII whitespace
stripped and collapsed); hyperlinks (`a` and `area` with an `href`); the `input` element's type
states and their default value and default checkedness; the `textarea` default value; "actually
disabled" for form controls, including a disabled `fieldset` ancestor outside its first
`legend`; the `required` attribute where it applies. The mapping from element and `input` type state to
the nine kinds is the table of item 6 of `docs/design/html-parser.md` (story `html-decisions`),
an unknown or missing `type` being the Text state.

"Hidden" is item 5 of the note, not the `hidden` attribute alone: a node is hidden when it or an
ancestor has `aria-hidden="true"` or matches a rule of the HTML Standard's user-agent style sheet
in 15.3.1 "Hidden elements", which needs no style computation: the elements `area, base,
basefont, datalist, head, link, meta, noembed, noframes, param, rp, script, style, template,
title`; `[hidden]` on any element but `embed`, whatever its value (`until-found` included);
`input[type=hidden i]`. `noscript` is not hidden, because surf parses with scripting disabled. An
`area` is not hidden by its own element rule: it is drawn through its image's map. The same
function serves the label computation of story `html-accessible-name`.

Decisions that bind this story (`docs/plan/agent-plan-revision-1.md`): a `select` element is
listed with kind `select` and no value; option selectedness is computed by
`story:agent-form-port` (D11). The tests that read `select` and `template` parsing are in
`story:html-tree-tables` and `story:html-tree-template`, where that parsing is written (D12);
this story keeps its place after `story:html-tree-body`. The document base URL is a function in
this crate written by `story:agent-follow-links` (D10), not here.

The code is a library over `surf-html`'s tree, in a crate of its own so that it can be written
beside the later tree stories, which change `crates/html`.

## Change

- `crates/page/` (package `surf-page`, `#![forbid(unsafe_code)]`, depends on `surf-html` only):
  `title(tree)` and `actionable_elements(tree)`, the second returning, in document order, each
  actionable element with its node identity, kind, target (links: the `href` as written) or
  value (text-like controls, `password`, `textarea`, `file`; none for `select`), and the four
  flags. A hidden `input` is not actionable. Elements in template contents are not in the
  document and are not listed. `hidden` is true by the rule of item 5 above, through one
  public function `is_hidden(tree, node)`; nothing that needs CSS is read (`epic:html-parser`,
  "Out of scope").
- Tests in `crates/page/tests/elements.rs`: `title_is_the_first_title_with_whitespace_collapsed`,
  `a_document_without_title_has_an_empty_title`, `links_are_a_and_area_with_href`,
  `an_a_without_href_is_not_a_link`, `input_type_states_map_to_kinds`,
  `a_hidden_input_is_not_listed`, `textarea_value_is_its_default_value`,
  `a_control_in_a_disabled_fieldset_is_disabled`,
  `a_control_in_the_first_legend_is_not_disabled`, `elements_come_in_document_order`,
  `hidden_attribute_on_an_ancestor_hides_an_element`, `hidden_until_found_hides_an_element`,
  `aria_hidden_true_on_an_ancestor_hides_an_element`, `an_element_inside_datalist_is_hidden`,
  `an_area_link_is_not_hidden_by_its_own_element`, `noscript_content_is_not_hidden`.

Writes: `crates/page/` (new: `Cargo.toml`, `src/`, `tests/elements.rs`). `Cargo.lock` gains a
repository package; the coordinator regenerates it.

## Acceptance

`cargo test -p surf-page` passes with the sixteen tests of `tests/elements.rs` named in Change,
and `cargo tree -p surf-page` lists only `surf-page` and `surf-html`.

## Out of scope

Labels (story `html-accessible-name`); writing rows (story `html-document-port`); a control's
form owner and option selectedness (`story:agent-form-port`); the document base URL
(`story:agent-follow-links`). No `AGENTS.md` change: story `html-document-port` hands over the
Layout entry for `crates/page`. Scripts: an element a script would make clickable (an `onclick`
on a `div`) is not actionable.
