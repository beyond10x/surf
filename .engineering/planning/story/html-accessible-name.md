---
format: aep.planning-md/3
id: story:html-accessible-name
kind: story
status: implemented
title: Compute an actionable element's label without CSS
summary: The accessible name of each actionable element, computed over the tree by the standards' steps, cut at 1,024 bytes and within a work bound per page; the rendering-dependent steps are listed as not taken.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-actionable-elements
scope:
- confidence: cited
  path: crates/page/src/label.rs
- confidence: cited
  path: crates/page/src/lib.rs
- confidence: cited
  path: crates/page/tests/label.rs
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-22T10:53:03Z", actor: "human:operator", revision: 14, decided_on: {"recorded":{"test_result":2,"verification":1}}}
---
## Context

The label of an actionable element is its accessible name (`docs/design/agent-interface.md`
section 5.2): Accessible Name and Description Computation 1.2 (4.3.2 "Computation steps",
steps 2A to 2I) with HTML Accessibility API Mappings 4.1 "Accessible Name Computations By HTML
Element", at the versions `docs/design/html-parser.md` names (Editor's Drafts of 29 May 2026
and 29 July 2026), and the HTML Standard's `label` element (4.10.4: labelable elements, the
labeled control, `labels`). Parts of the computation depend on rendering, and surf has no CSS.
The note (story `html-decisions`, item 5) fixes what label is before CSS: the computation over
the tree, with "hidden" in step 2A "Hidden Not Referenced" read by the rule of item 5 as wave 9
widened it: `aria-hidden` true in any ASCII case (not read on the document element and the body
element), the hidden elements of 15.3.1, a `dialog` without `open` (15.3.3), the content of a
`details` without `open` but for its first `summary` child (15.5.5); the fallback content of
`video`, `audio` and `object` is not hidden. That rule is `surf_page::is_hidden`
(`crates/page/src/lib.rs`), and the one walk of `actionable_elements` carries the same flag
down. The steps not taken are 2F.ii "Name From Generated Content", hiding by author CSS, 2F.iii
"Determine Child Nodes" for shadow roots and slots, the spaces joined by CSS `display` (AccName
#225), and an explicit `role` attribute (the role is the one the kind implies, item 6).

The computation lives in `crates/page` beside `actionable_elements`, and needs nothing from
`crates/html` that is not there: `Tree::children`, `Tree::parent`, `Tree::data` and `NodeId`
(`Hash`, `Eq`) are enough. What the tree does not offer, an element by its ID, a control's
`label` elements and a dense node index, is built in `crates/page` in the one walk. This
story writes no file of `crates/html`, which `story:html-parser-limits` writes in the same
wave.

What each kind's name is made of, in order, first non-empty wins (HTML-AAM 4.1):

| element | sources, in order |
|---|---|
| `a` with `href` (4.1.13) | `aria-labelledby`, `aria-label`, its subtree, `title` |
| `area` with `href` (4.1.14) | `aria-labelledby`, `aria-label`, `alt`, `title` |
| `button` (4.1.4) | `aria-labelledby`, `aria-label`, its `label` elements, its subtree, `title` |
| `input` Submit Button, Reset Button, Button (4.1.2) | `aria-labelledby`, `aria-label`, its `label` elements, `value`, `Submit` or `Reset` when `value` is absent (Submit Button and Reset Button only), `title` |
| `input` Image Button (4.1.3) | `aria-labelledby`, `aria-label`, its `label` elements, `alt` (not blank), `title` (not empty), `Submit` |
| `input` Text, Search, Telephone, URL, Email, Password, Number, and `textarea` (4.1.1) | `aria-labelledby`, `aria-label`, its `label` elements, `title`, `placeholder`, `aria-placeholder` |
| every other `input` state, and `select` (4.1.7) | `aria-labelledby`, `aria-label`, its `label` elements, `title` |

A control's `label` elements are the HTML Standard's `labels`: every `label` whose labeled
control it is, in tree order, their names joined by one space. A `label` with a `for` attribute
labels the first element in tree order whose ID is the value, if that element is labelable, and
nothing else (no fallback to a descendant); a `label` without `for` labels its first labelable
descendant in tree order. When the control is inside its label, its own value, `alt` or subtree
is left out of that label's name (HTML-AAM 4.1.1 to 4.1.4).

Inside a subtree, a `label`, or a node referenced by `aria-labelledby`, each node is read by
steps 2A to 2I: a hidden node gives nothing (2A); a descendant's own `aria-labelledby` is
followed unless the walk is already inside an `aria-labelledby` traversal (2B); a control
embedded in the name of another element gives its value (2C: a Text, Search, Telephone, URL,
Email or Password `input` and a `textarea` their default value; a Number or Range `input`
`aria-valuetext`, else `aria-valuenow`, else its `value` attribute; a `select` the empty string,
because option selectedness is `story:agent-form-port`'s); `aria-label` not blank (2D); `alt` of
`img`, `area` and an Image Button `input`, and an `img`'s `title` when it has no `alt` (2E, with
HTML-AAM 4.1.10); text nodes (2G); the children of an element (2F, 2H); `title` (2I). Each node
is read at most once per name (accname, the note under 2F: "Each node in the subtree is
consulted only once"), so a repeated or cyclic reference adds nothing and every name ends.

Found before this story's wave (`docs/plan/agent-wave-9.md`, "Findings, and where each goes"),
settled here:

- `is_hidden` walks up from its node on each call, and two shapes cost more than the depth (a
  `body` with `aria-hidden` true after many comments; a `summary` after many children of a
  closed `details`). The computation calls `is_hidden` for no node: the hidden flag of every
  node it reads comes from the walk that reaches it (carried down from the element named, the
  `label` or the referenced node), and the flag of each `label` and each element with an `id`
  is recorded by the one walk of `actionable_elements`. The two shapes are then paid once per
  page; a case holds it (below).
- "Hidden" since wave 9: the computation uses the same rule, by the same code, as `hidden`.
- A node outside the document (template contents): the ID lookup and the `label` search walk
  `Tree::children`, which never enters template contents (`crates/html/src/tree.rs`, the
  documentation of `children`), so the computation never reaches such a node. No case: the tree
  for `template` changes with `story:html-tree-template` (wave 12).
- A hidden node referenced by `aria-labelledby`: step 2A as written, the referenced node and
  its whole subtree, hidden parts included, give the name; a visible referenced node leaves its
  hidden descendants out (accname, examples 1 and 2). A hidden `label` names its control the
  same way.
- A hidden actionable element (its row is kept and flagged, the note's decision): step 2A would
  give it the empty name. Its name is computed as for a hidden node referenced by
  `aria-labelledby`, its hidden subtree included, so that `story:agent-follow-links` can show a
  hidden link by its label. A deviation, written in the crate documentation.

Cost without bounds, inferred from the steps (not measured): one name reads at most every node
once, so a page costs the number of actionable elements times the nodes each reads. Two shapes
make that quadratic in the input: many controls whose `aria-labelledby` names one large node (a
16 MiB page holds about 335,000 `<input aria-labelledby=s>` beside an 8 MiB `span id=s`: about
2.8 TB of names), and links nested through `object` (a marker of the list of active formatting
elements lets an `a` hold an `a`), each reading the subtree below it. The names are copies, so
memory grows the same way. The bounds below make the time linear in the nodes and the attribute
bytes plus a fixed budget, and the memory of all names at most that budget plus the attribute
bytes the tree stores.

## Change

- `crates/page/src/label.rs` (new): the name of each actionable element by the table and the
  steps above. Whitespace: the children of an element are joined without a space (2F.iv;
  joining by CSS `display` is not taken), the results of the IDREFs of `aria-labelledby` and of
  several `label` elements are joined by one space, and the name is stripped and collapsed as
  Infra's "strip and collapse ASCII whitespace" (accname's "flat string"; the function exists
  in `lib.rs`). IDREFs are the ASCII-whitespace-separated tokens; a token that names no element
  is skipped; IDs are compared exactly. The `aria-label`, `alt` and `title` tests for "not
  empty" are those of the steps cited. No function recurses over the depth of the tree.
- Bounds, public constants with documented values, a deliberate deviation stated beside them:
  `LABEL_BYTES_MAX = 1_024`: a name longer than that is cut at the last UTF-8 character
  boundary at or below 1,024 bytes, and the walk for that name stops there.
  `LABEL_WORK_MAX = 16_777_216`: the work for the names of one call of `actionable_elements`,
  one unit per node read and one per byte appended to a name. Once it is spent, each later name
  is taken from the element's own attributes only (`aria-label`; an `input`'s `value` or its
  `Submit` or `Reset`; `alt`; `title`; `placeholder`; `aria-placeholder`, in the table's order),
  without `aria-labelledby`, `label` elements or the subtree.
- `crates/page/src/lib.rs`: `ActionableElement` gains `label: String` (the name, stripped and
  collapsed, at most `LABEL_BYTES_MAX` bytes) and `label_cut: bool` (cut at `LABEL_BYTES_MAX`, or
  taken from attributes only because `LABEL_WORK_MAX` was spent); `actionable_elements` fills
  both. Its one walk also records the first element per ID, each `label` with its labeled
  control, and the hidden flag of each; a control's wrapping `label` is found in that walk, not
  by walking up from the control. `pub mod label` or a private module with the two constants
  re-exported. The crate documentation: "Rules taken" gains the label; a section "Steps not
  taken" lists 2F.ii, author CSS, 2F.iii for shadow roots and slots, CSS `display` spaces, the
  `role` attribute and the `select` in 2C, each with its reason; "Deviations" gains the hidden
  element's name and the two bounds; the item "Each element's label" leaves "Left to other
  stories"; the paragraph on cost states the time and memory of the names.
- `docs/design/html-parser.md`, item 5, is not written by this story's unit: the file is
  `story:html-parser-limits`'s in this wave (item 7). The unit hands over in its report the
  sentences that replace "The row holds the whole name" (the two bounds, the hidden element's
  name), and the wave's documents commit writes them.
- Tests in `crates/page/tests/label.rs`, each parsing with `surf_html::parse_document` and
  `ParseOptions::default()` as `tests/elements.rs` does, and, like it, using no table,
  template, `svg`, `math` or misnested formatting, and no tree deeper than 512 elements except
  the last case. One per source:
  `aria_labelledby_names_from_the_referenced_elements` (two IDREFs joined by a space, a token
  that names nothing skipped), `aria_label_names_the_element`, `label_for_names_a_control`,
  `wrapping_label_names_a_control`, `every_label_of_a_control_is_joined_in_tree_order`,
  `button_input_value_names_it` (Submit Button, Reset Button and Button with `value`),
  `submit_button_without_value_is_named_submit`,
  `reset_button_without_value_is_named_reset`,
  `image_button_is_named_by_alt_then_title_then_submit`, `area_alt_names_the_link`,
  `img_alt_inside_a_link_names_it`, `link_text_names_the_link`,
  `button_text_names_the_button`, `title_names_an_element_without_another_name`,
  `placeholder_names_a_text_control_without_another_name` (then `aria-placeholder`; a Date
  `input` and a `select` with a `placeholder` have the empty name). The rules:
  `aria_label_wins_over_label_element`, `aria_labelledby_is_not_followed_twice` (accname
  example 3: the second element's name is empty),
  `aria_labelledby_naming_the_element_itself_uses_its_aria_label` (example 5: "Delete
  Documentation.pdf"), `a_label_with_for_labels_only_its_target` (a wrapped control whose label
  names another is not named by it; a `for` whose first element is not labelable labels
  nothing), `a_control_inside_its_label_leaves_its_own_value_out` (`<label>Name <input
  value=v></label>` is `Name`; example 4 with an `input value=5` gives the checkbox `Flash the
  screen 5 times.`), `hidden_subtree_is_left_out_of_link_text` (the `hidden` attribute,
  `aria-hidden="TRUE"`, the content of a closed `details`),
  `hidden_node_referenced_by_aria_labelledby_names_with_its_hidden_subtree` (examples 1 and 2),
  `hidden_label_still_names_its_control`, `hidden_element_is_named_from_its_own_subtree`,
  `element_without_any_source_has_an_empty_name`, `whitespace_in_a_name_is_collapsed` (`<a
  href> a\n\tb <span>c</span>d</a>` is `a b cd`), `a_reference_cycle_ends` (an element whose
  `aria-labelledby` names its ancestor, two links naming each other, a repeated IDREF). The
  bounds: `a_name_is_cut_at_label_bytes_max` (a link over 5,000 bytes of text: 1,024 bytes or
  fewer at a character boundary of a two-byte character, `label_cut` true),
  `past_the_label_work_bound_names_come_from_attributes_only` (20,000 `input`s with a `title`
  and `aria-labelledby` naming one `span` of 65,536 bytes: every name at most 1,024 bytes, the
  first names the span's first 1,024 bytes and their sum at most `LABEL_WORK_MAX`, the last
  names the `title`, every row `label_cut`),
  `hidden_flags_cost_one_walk_for_many_references` (2,000 comments before `html` and a `body`
  with `aria-hidden` true holding a `span id=t` and 2,000 `input`s naming it; a closed
  `details` with 2,000 children and then a `summary id=s` named by 2,000 `input`s: for each
  shape `attribute_visits()` of `actionable_elements` exceeds that of the same page with
  `aria-label` on the `input`s by less than one visit per reference, and no time is asserted;
  the unit built it with 50,000 each and a bound of 10 seconds by `std::time::Instant`, and
  `story:test-step-holds-under-load` changed it to this form),
  `the_name_of_a_deep_subtree_is_computed_without_recursion` (`<a href=deep>d` and 100,000
  nested `span`: the name is `d`; it holds also once the depth limit stops the parse at 512).

Writes: `crates/page/src/label.rs` (new), `crates/page/src/lib.rs`, `crates/page/tests/label.rs`
(new).

For `AGENTS.md`, handed over in the report: the `crates/page` entry gains "and the label of
each, the accessible name, cut at 1,024 bytes, with a work bound per page", and its test count.

## Acceptance

In the unit's tree, with `RUSTFLAGS=-D warnings`: `cargo test -p surf-page` passes, with the 31
tests of `tests/label.rs` named in Change and the 24 of `tests/elements.rs` unchanged; a
planted defect that calls `is_hidden` once per referenced node fails
`hidden_flags_cost_one_walk_for_many_references`, and one that drops the once-per-name rule
fails `a_reference_cycle_ends` (the unit reports both runs). `cargo tree -p surf-page -e
normal,build,dev --prefix none` prints two lines, `surf-page v0.1.0` and `surf-html v0.1.0`.
`cargo fmt -p surf-page -- --check`, `cargo clippy -p surf-page --all-targets` and
`RUSTDOCFLAGS=-D warnings cargo doc -p surf-page --no-deps` exit 0. The crate documentation has
the section "Steps not taken" with its six entries and no longer lists the label under "Left to
other stories". `git diff --stat` names no file outside `crates/page/`. Test time (the
gate's test step has 2 minutes of wall time, a single test 60 s): `cargo test -p surf-page` ends within 20 s of wall time after the build in
the test profile on the unit's machine, and the report names each test over 5 s with its time;
`hidden_flags_cost_one_walk_for_many_references` asserts a count and no time since
`story:test-step-holds-under-load`. `crates/page/tests/elements.rs` is not this story's: its test
`a_deep_tree_is_read_without_recursion` is adapted by `story:html-parser-limits` in the same
wave.

## Out of scope

Names that depend on CSS (generated content, `display: none`, `visibility`, spaces by
`display`), ARIA roles beyond what the nine kinds need, descriptions, the chosen option of a
`select` (`story:agent-form-port`), a descendant control's own `label` elements during a walk
(the content model of `label` allows neither; only its attributes are read, 2E), and the
specification's comment on `label` ("whole", `spec/domains/document.yaml`), which
`story:html-document-port` brings in line when it writes the rows. A completeness claim: the
computation is marked incomplete, because web-platform-tests checks it in a running browser
with script. Scripts: none.
