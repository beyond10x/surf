---
format: aep.planning-md/3
id: story:html-accessible-name
kind: story
status: active
title: Compute an actionable element's label without CSS
summary: The accessible name computation over the tree, with the rendering-dependent steps listed as not taken.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-actionable-elements
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 4}
---
## Context

The label of an actionable element is its accessible name (`docs/design/agent-interface.md`
section 5.2): the W3C Accessible Name and Description Computation with the HTML Accessibility
API Mappings for HTML elements, from memory, to be confirmed by this story. Parts of the
computation depend on rendering, and surf has no CSS. `docs/design/html-parser.md` (story
`html-decisions`, item 5) fixes what label is before CSS: proposed, the computation over the
tree with "hidden" read as the `hidden` attribute or `aria-hidden="true"`, no generated content,
the rendering-dependent steps listed as not taken, and the whole name kept (the answer format
cuts it at 60 characters). Story `html-actionable-elements` wrote the crate this story adds to.

## Change

- `crates/page/src/label.rs`: the name of each actionable element in the order the computation
  gives, from thirteen sources: `aria-labelledby`; `aria-label`; a `label` element by `for`; a
  wrapping `label` element; a submit button's `value`; the default name `Submit` of a submit
  button without one; the default name `Reset` of a reset button without one; the `alt` of an
  image button (`input type=image`); the `alt` of an `img` inside a link; a link's text from its
  subtree; a `button` element's text from its subtree; `title`; `placeholder`. Whitespace is
  collapsed as the computation says. The crate documentation lists each step not taken and why.
- `actionable_elements(tree)` returns the label with each element.
- Tests in `crates/page/tests/label.rs`, one per source:
  `aria_labelledby_names_from_the_referenced_elements`, `aria_label_names_the_element`,
  `label_for_names_a_control`, `wrapping_label_names_a_control`,
  `submit_button_value_names_it`, `submit_button_without_value_is_named_submit`,
  `reset_button_without_value_is_named_reset`, `image_button_alt_names_it`,
  `img_alt_inside_a_link_names_it`, `link_text_names_the_link`,
  `button_text_names_the_button`, `title_names_an_element_without_another_name`,
  `placeholder_names_a_text_control_without_another_name`; and five more:
  `aria_label_wins_over_label_element`, `aria_labelledby_is_not_followed_twice`,
  `hidden_subtree_is_left_out_of_link_text`, `element_without_any_source_has_an_empty_name`,
  `whitespace_in_a_name_is_collapsed`.

Writes: `crates/page/src/label.rs`, `crates/page/src/lib.rs`, `crates/page/tests/label.rs`.

## Acceptance

`cargo test -p surf-page` passes with the eighteen tests of `tests/label.rs` named in Change,
thirteen of them one per name source, and the crate documentation lists the steps of the
computation that are not taken.

## Out of scope

Names that depend on CSS (generated content, `display: none`, `visibility`), ARIA roles beyond
what the nine kinds need, descriptions. A completeness claim: the computation is marked
incomplete, because web-platform-tests checks it in a running browser with script. Scripts:
none.
