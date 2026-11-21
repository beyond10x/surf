---
format: aep.planning-md/3
id: story:html-parser-limits
kind: story
status: active
title: Bound the parser on untrusted input
summary: Input, depth, node and attribute limits stop parsing with a named result; adversarial and 10,000 seeded inputs end without a panic.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-tree-formatting
- depends_on: story:html-input-stream
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 4}
---
## Context

The bytes come from an untrusted server (`epic:html-parser`, "Constraints"): every limit ends
parsing with a defined result, none is a hang, an unbounded allocation or a panic, and the
depth of the tree and the size of the input have limits stated like those of `crates/fetch`
(`crates/fetch/src/lib.rs`, the `Limits` value with documented defaults). The parser's own
algorithms reach far on hostile input: the stack of open elements grows with nesting, and
"reconstruct the active formatting elements" clones elements with their attributes for every
run of text, so nodes and stored bytes can grow faster than input. `docs/design/html-parser.md`
(story `html-decisions`, item 7) fixes the limits, their defaults and what reaching one does:
parsing stops, the document holds what was built and names the limit. The defaults proposed
there, which this story uses unless the note changed them:

| limit | default |
|---|---|
| input bytes | 16,777,216 (16 MiB, the `surf-fetch` body limit) |
| tree depth (elements on the stack of open elements) | 512 |
| nodes in the tree, template contents included | 1,048,576 |
| attributes on one element | 1,024 |
| stored bytes (text and attribute values the tree holds, clones included) | 67,108,864 (64 MiB) |

All but the input limit are not in the HTML Standard and are a deliberate deviation, documented
beside the code.

This story comes after the formatting story because that story adds the cloning, and before
the table, template and foreign stories, which keep its tests passing because they run in
`cargo test -p surf-html` every time.

## Change

- `crates/html/src/limits.rs`: a `Limits` value with the defaults above, and the result of a
  parse naming the limit that stopped it, if any, with the node count and stored bytes of the
  result readable by a test.
- `crates/html/src/lib.rs`, `crates/html/src/tree_builder.rs`, `crates/html/src/tree.rs`,
  `crates/html/src/tokenizer.rs`: each limit checked where it can be exceeded; parsing that
  reaches one stops as "stop parsing" does, with the open elements popped.
- `crates/html/tests/limits.rs`, one test per limit, each asserting the named limit at its
  default number and that the tree holds what came before:
  `input_of_16_777_217_bytes_stops_at_the_input_limit`,
  `element_at_depth_513_stops_at_the_depth_limit`,
  `node_1_048_577_stops_at_the_node_limit`,
  `attribute_1_025_on_one_element_stops_at_the_attribute_limit`,
  `cloned_attributes_past_64_mib_stop_at_the_stored_bytes_limit`;
  one test per adversarial input, each asserting it ends in under 10 seconds (measured with
  `std::time::Instant` in the test profile), with a node count at most 1,048,576 and stored
  bytes at most 67,108,864:
  `nested_div_100_000_ends_within_bounds`,
  `formatting_elements_100_000_then_text_after_p_end_within_bounds`,
  `table_with_100_000_misnested_a_ends_within_bounds`,
  `element_with_1_000_000_attributes_ends_within_bounds`,
  `input_of_16_mib_plus_one_byte_ends_within_bounds`;
  and `seeded_10_000_inputs_end_within_bounds`: 10,000 inputs of at most 4,096 bytes from a
  seeded generator written here over an alphabet of markup fragments, each parsed under
  `std::panic::catch_unwind` with no panic and each within the node and stored-byte limits, all
  10,000 in under 30 seconds.
- The crate documentation lists the limits, their defaults, and the deviation.

Writes: `crates/html/src/limits.rs`, `crates/html/src/lib.rs`, `crates/html/src/tree.rs`,
`crates/html/src/tree_builder.rs`, `crates/html/src/tree_builder/`,
`crates/html/src/tokenizer.rs`, `crates/html/tests/limits.rs`.

## Acceptance

`cargo test -p surf-html` passes with the eleven tests of `tests/limits.rs` named in Change,
five limits at their default numbers, five adversarial inputs and the seeded run each within
their stated time, node and stored-byte bounds, and `slot task html-conformance
HTML_BASE=<this story's base commit>` exits 0.

## Out of scope

A clock-based limit inside the parser (time is bounded by input, nodes and stored bytes; the
tests measure it); limits in `surf-fetch`, which already has its own. Scripts: no script runs,
so no script can grow the tree after parsing.

## Found before this story's wave

From wave 9 (`docs/plan/agent-wave-9.md`), for this story's own reading before its wave. Each
is measured; none is bounded.

- The adoption agency algorithm (`crates/html/src/tree_builder/formatting.rs`): each
  formatting end tag walks up to eight times from the common ancestor to the Document (the
  step that asks whether the last node is an ancestor of the place it is inserted at), and
  its steps 4.4 and 4.5 scan, and 4.26 shifts, a stack that is long above the formatting
  element. Two inputs of 1 MiB, release build, after the fix unit of wave 9: 60,000 nested
  `<object>`, then `<a>` + 8 `<div>` + `</a>` + 8 `</div>` repeated: 34.6 s (1.23 s before the
  algorithm existed); `<a>` + 20,000 `<div>`, then `</a>` repeated: 2.5 s (0.75 s). Both need
  a stack far deeper than 512, so the depth limit bounds them; neither is one of this
  story's five adversarial inputs. The tree can be deeper than the stack by a constant
  factor: the `a` start tag takes an element off the stack and leaves it as an ancestor.
- The list of active formatting elements has no bound when the tags differ in attributes,
  and Noah's Ark scans it back to the last marker for each formatting start tag: 30,000
  nested `b` with different ids took 7.9 s in a prototype's release build. The depth limit
  bounds this too, as long as each entry is an open element; say whether an entry can outlive
  its element on the stack in a number that matters.
- Each formatting end tag adds at most 32 elements (8 outer iterations, 3 clones of the inner
  loop and one new element each).
- Nested block start tags parse quadratically: 5,000 and 10,000 nested `fieldset` took 1.25 s
  and 5.09 s, 20,000 `<fieldset disabled><legend>` 55.4 s, in a debug build (the adversary of
  `surf-page`); 100,000 nested `div` took 545 s (wave 8).
