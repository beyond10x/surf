---
format: aep.planning-md/3
id: story:html-parser-limits
kind: story
status: active
title: Bound the parser on untrusted input
summary: Input, depth, node, attribute, stored-byte and step limits stop parsing with a named result; adversarial and 10,000 seeded inputs end within stated times and without a panic.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-tree-formatting
- depends_on: story:html-input-stream
scope:
- confidence: cited
  path: conformance/src/target.rs
- confidence: cited
  path: crates/html/src/lib.rs
- confidence: cited
  path: crates/html/src/limits.rs
- confidence: cited
  path: crates/html/src/tokenizer.rs
- confidence: cited
  path: crates/html/src/tokenizer/machine.rs
- confidence: cited
  path: crates/html/src/tree.rs
- confidence: cited
  path: crates/html/src/tree_builder.rs
- confidence: cited
  path: crates/html/src/tree_builder/
- confidence: cited
  path: crates/html/tests/limits.rs
- confidence: cited
  path: crates/html/tests/tree_body.rs
- confidence: cited
  path: crates/html/tests/tree_formatting.rs
- confidence: cited
  path: crates/page/tests/elements.rs
- confidence: cited
  path: docs/design/html-parser.md
- confidence: cited
  path: generated/rust/
- confidence: cited
  path: spec/domains/document.yaml
- confidence: cited
  path: spec/scenarios/parse-stopped-at-a-limit.yaml
revision: 23
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 4}
---
## Context

The bytes come from an untrusted server (`epic:html-parser`, "Constraints"): every limit ends
parsing with a defined result, none is a hang, an unbounded allocation or a panic, and the
depth of the tree and the size of the input have limits stated like those of `crates/fetch`
(`crates/fetch/src/lib.rs`, the `Limits` value with documented defaults and public fields).
`docs/design/html-parser.md` (story `html-decisions`, item 7) fixes the limits, their defaults
and what reaching one does: parsing stops, the document holds what was built and names the
limit. `spec/domains/document.yaml` names them in `surf.document.ParseLimit` (`InputBytes`,
`Depth`, `Nodes`, `Attributes`, `StoredBytes`), the value `story:html-document-port` (wave 11)
writes into `FinishParsing.stopped_by`.

| limit | default |
|---|---|
| input bytes | 16,777,216 (16 MiB, the `surf-fetch` body limit) |
| tree depth (elements on the stack of open elements) | 512 |
| nodes in the tree, template contents included | 1,048,576 |
| attributes on one element | 1,024 |
| stored bytes (every string the tree holds, clones included) | 67,108,864 (64 MiB) |
| steps (entries tree construction visits in its scans) | 134,217,728 (2^27) |

The standard places none of them (13.2.6, before 13.2.6.1); all but the depth limit are a
deliberate deviation, documented beside the code, and so is what reaching any of them does.

### What was measured before this story (wave 8, wave 9, and the reading before wave 10)

Each input was run through a copy of `crates/html` at `61fb7a9` with a crude check after each
token (stack deeper than 512, more than 1,048,576 nodes, more than 64 MiB of text and attribute
values), in the release and the dev (test) profile, on a shared and loaded machine:

| input | without limits | the limit that stops it, and the work before |
|---|---|---|
| 100,000 nested `div` (wave 8) | 545 s, debug | depth, at the 511th `div`: 0.037 s debug |
| 1,000 different `<b id=k>` after `<p>`, then `<p>x` 1,000 times (13,893 bytes; the reconstruction) | 1,003,005 nodes; 0.30 s release, 1.22 s debug | depth, at the first reconstruction: 0.011 s debug |
| 60,000 nested `object`, then `<a>` + 8 `<div>` + `</a>` + 8 `</div>` to 1 MiB (wave 9) | 34.6 s release | depth, at the 511th `object`: 0.026 s debug |
| `<a>` + 20,000 `<div>`, then `</a>` to 1 MiB (wave 9) | 2.5 s release | depth: 0.037 s debug |
| 30,000 nested `<b id=k>` (wave 9) | 7.9 s release | depth: 0.016 s debug |
| 10,000 nested `fieldset`; 20,000 `<fieldset disabled><legend>` (wave 9) | 5.09 s and 55.4 s debug | depth: 0.014 s and 0.021 s debug |

So the inputs the records name are stopped by the depth limit after a few hundred tokens. The
five limits do not bound time, though: an input can stay inside all of them and still cost
tokens × depth stack visits, and each visit of the present code compares strings
(`is_special` searches 81 names, scope checks 10 or more):

| input inside the five limits (at most 16 MiB) | release | debug | limit reached |
|---|---|---|---|
| 505 nested `span`, then `</q>` to 16 MiB | 122.9 s | over 150 s | none |
| 505 nested `span`, then `<div></div>` to 16 MiB | 2.3 s | 67.6 s | nodes |
| 490 nested `object`, then the wave 9 pattern to 16 MiB | 1.19 s | 4.8 s | nodes |
| `<a>` + 500 `<div>`, then `</a>` to 16 MiB | 0.55 s | 4.0 s | none |
| 25, 50, 100 nested `<b>` with 1,024 attributes differing in the last (Noah's Ark) | 0.35, 1.31, 5.99 s | | none |
| the same, 505 nested (2.5 MiB) | over 150 s | over 150 s | none |
| `<span>` tags with 1,024 attributes each, to 16 MiB (duplicate check per attribute) | 6.4 s | 24.9 s | none |
| 25,000 and 50,000 `<html aK>` (the `html` attribute merge) | 0.80 s, 3.23 s | | none (no limit applies yet) |
| 16 MiB of text; 16 MiB in one comment | 1.0 s; 0.32 s | 6.8 s; 2.1 s | none |
| 500 `<b v=…1 KiB… id=k>` after `<p>`, then `<p>x` (clones) | 0.16 s | 0.87 s | stored bytes, after 761 tokens |

Deciding the special category once per element (a prototype flag in the arena) took the first
row from 7.2 s to 0.37 s release per MiB (4.5 s debug per MiB); the `div` pairs stayed at 78 s
debug. That is why this story adds the step limit beside the five: no constant factor brings
16 MiB of such tokens under 10 s in the test profile, and a fixed budget of visits does. The
default is chosen from five large real pages (ffmpeg, cmake, sane and doxygen documentation,
1 to 3.7 MB): the sum of the stack and list lengths over their tokens is at most 3.8 per input
byte, so a 16 MiB page of their kind stays near 64 million, half the default; 2^27 visits took
about 4.5 to 6 s in the debug prototype.

Also settled from the records: the list of active formatting elements was never longer than
the stack in any run (511 at depth 513, 500 at 503): an entry after the last marker that is not
open is pushed again by the next formatting start tag or character (the reconstruction), which
meets the depth limit, and entries before a marker belong to elements below that marker's
`object`, `applet` or `marquee` (inferred from the rules, not proved); the step limit bounds the
scans either way. The tree was never deeper than the stack plus one in any run, and the walk to
the root of the adoption agency algorithm counts as steps. A formatting end tag creates at most
32 elements (8 outer iterations, 3 clones and 1 new element each).

The vendored conformance data stays far inside every limit: over all tree-construction and
encoding cases, measured with the harness on the same copy, at most 39 open elements, 7 list
entries, 42 nodes, 8,307 stored bytes, 59 attributes on a start tag and 8,390 input bytes. No
count can move.

This story comes after the formatting story because that story adds the cloning, and before
the table, template and foreign stories, which keep its tests passing because they run in
`cargo test -p surf-html` every time; foster parenting's child scans (`html-tree-tables`) count
as steps.

## Change

- `crates/html/src/limits.rs`: `Limits` (`Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, public
  fields `max_input_bytes`, `max_depth`, `max_nodes`, `max_attributes`, `max_stored_bytes`,
  `max_steps`, `Default` with the table's numbers, each documented as `surf-fetch` documents its
  own) and `Limit`, the limit that stopped a parse: `InputBytes`, `Depth`, `Nodes`,
  `Attributes`, `StoredBytes`, `Steps`, the names of `surf.document.ParseLimit`.
- `crates/html/src/lib.rs`: `ParseOptions` gains `pub limits: Limits`. No public signature
  changes: `parse_document`, `parse_document_bytes` and `parse_fragment` keep theirs, so
  `crates/html-conformance/src/runner.rs` (which builds `ParseOptions` with
  `..ParseOptions::default()`) and `crates/page` compile unchanged. The crate documentation's
  "State" entry for `limits` is rewritten, and a section "Limits" lists each limit, its default,
  the deviation, what a caller gets, and what a parse holds besides the tree (the input as code
  points, 4 bytes each).
- `crates/html/src/tree.rs`: `Tree::stopped_by(&self) -> Option<Limit>`,
  `Tree::node_count(&self) -> usize` (every node of the arena: the Document, detached nodes,
  template contents) and `Tree::stored_bytes(&self) -> usize`, kept as the tree is built.
- What a caller gets when a limit stops the parse: the tree with every node inserted before
  that point, the open elements popped as "stop parsing" pops them (no end-of-file step runs),
  and `stopped_by()` naming the limit; `None` for a parse that ended without one. For
  `parse_document_bytes` it is `parsed.tree.stopped_by()`; a limit reached before a `meta`
  that changes the encoding means no re-parse. The point at which each limit stops:
  - input bytes: a `&str` longer than the limit is cut to its longest prefix of at most
    `max_input_bytes` bytes that ends on a character boundary, and bytes are cut before
    decoding (a sequence cut in two decodes as the decoder decodes an end of input); the parse
    runs over the cut input to its end, so the tree equals that of the prefix, and names
    `InputBytes`. The re-parse after a `meta` uses the same cut bytes.
  - depth: an element that would be entry 513 of the stack of open elements is not created.
  - nodes: a creation that would make `node_count()` exceed the limit is not made (a
    `template` creates two nodes). The reconstruction checks before each element; the adoption
    agency algorithm checks before each outer iteration for the four elements it can create,
    so an iteration runs whole or not at all and leaves no node out of its parent.
  - attributes: the tokenizer stops when a start tag token would keep attribute 1,025
    (a duplicate it drops is not counted), and that tag creates no element; the `html` and
    `body` start tags of "in body" stop before a merge that would give the element attribute
    1,025.
  - stored bytes: the UTF-8 bytes of every string the tree holds: element local names,
    attribute names, prefixes and values, text, comment data, doctype fields, processing
    instruction target and data. A node or an append that would pass the limit is not made;
    text is appended a character at a time, so a text node holds everything up to the limit.
    (Design note item 7 counted "text and attribute values"; names are counted too because
    reconstruction clones attribute names, which would otherwise grow without bound.)
  - steps: every entry visited in a scan of the stack of open elements or the list of active
    formatting elements (scopes, popping, "any other end tag", Noah's Ark, reconstruction, the
    adoption agency algorithm's scans and its walk to the root), and every child visited in a
    parent's child list (`Tree::insert`, `insert_text`, `remove`). Checked after each token: the
    token that passes the limit is processed whole, then parsing stops.
- `crates/html/src/tree_builder.rs`, `crates/html/src/tree_builder/` (`modes.rs`,
  `formatting.rs`): the checks above, at the places named; the special category and the scope
  categories decided once per element when it is created, not by searching name lists at
  every visit; Noah's Ark compares two entries in O(1) when tag names, attribute counts or a
  key kept with each entry (its attributes sorted by name, hashed) differ, and in O(k) for k
  attributes otherwise (now O(k²) per pair); the `html` and `body` attribute merge costs O(1)
  per attribute of the token (a set of the names kept for those two elements).
- `crates/html/src/tokenizer.rs`, `crates/html/src/tokenizer/machine.rs`: the attribute limit
  (the public `tokenize` runs without it) and a duplicate-attribute check in O(1) per attribute
  (a set of the token's names; now a scan of every earlier attribute).
- Tests the change turns red, adapted in it:
  - `crates/html/tests/tree_body.rs` `a_deep_input_is_parsed_read_and_dropped_without_recursion`
    (100,000 nested `span`): parsed with `max_depth` raised above 100,002, so it still proves
    reading and dropping without recursion.
  - `crates/html/tests/tree_formatting.rs` `the_outer_loop_runs_at_most_eight_times`:
    `a_elements(1_000)` has 1,003 open elements; it becomes `a_elements(500)`, expected
    `(9, 513)` (1,013 less 500 `div`).
  - `crates/page/tests/elements.rs` `a_deep_tree_is_read_without_recursion` (100,000 nested
    `span`, and 20,000 `<span><input>`): its parse raises `max_depth`; no other line of that
    file changes.
- The sixth limit in the specification, spec first: `spec/domains/document.yaml`,
  `surf.document.ParseLimit` gains `Steps` ("Tree construction visited as many entries as the
  step limit"); `spec/scenarios/parse-stopped-at-a-limit.yaml` gains a sixth document stopped by
  `Steps`; `task generate` rewrites `generated/rust/`; `conformance/src/target.rs` adds `Steps`
  to `parse_limit` and `parse_limit_named`; `docs/design/html-parser.md` item 7 gains the row,
  the stored-bytes definition above and the measurement of this story.
- `crates/html/tests/limits.rs`, each test with the default `Limits` unless it says otherwise:
  - one per limit, each asserting `stopped_by()` and what the tree holds:
    `input_of_16_777_217_bytes_stops_at_the_input_limit` (a comment `<!--` filling 16,777,217
    bytes, through `parse_document` and `parse_document_bytes`: one comment of 16,777,212 bytes,
    `InputBytes`), `a_cut_input_parses_as_its_prefix` (`max_input_bytes: 100`, markup with a
    two-byte character across byte 100: the same nodes as the prefix of 99 bytes),
    `element_at_depth_513_stops_at_the_depth_limit` (`<div>` x 600: 510 `div`,
    `node_count()` 514), `node_1_048_577_stops_at_the_node_limit` (`<!---->` x 1,048,580:
    `node_count()` 1,048,576, equal to `descendants(root).len()`),
    `attribute_1_025_on_one_element_stops_at_the_attribute_limit` (`<p>x</p>` then a `span`
    with 1,025 attributes: the `p` and its text, no `span`),
    `attribute_1_025_merged_into_html_stops_at_the_attribute_limit`,
    `cloned_attributes_past_64_mib_stop_at_the_stored_bytes_limit` (500 `b` with a 1 KiB value
    after `<p>`, then `<p>x`: `stored_bytes()` at most 67,108,864),
    `stack_visits_past_2_27_stop_at_the_step_limit` (505 nested `span`, then `</q>` to 2 MiB),
    `a_node_limit_inside_the_adoption_agency_leaves_no_element_out_of_the_tree` (a small
    `max_nodes` that falls inside a `</a>`: `node_count()` equals `descendants(root).len()`).
  - one per adversarial input, each asserting it ends in under 10 seconds (measured with
    `std::time::Instant` in the test profile), `node_count()` at most 1,048,576,
    `stored_bytes()` at most 67,108,864, and the limit it is stopped by, which the unit
    measures and reports: `nested_div_100_000_ends_within_bounds`,
    `formatting_elements_100_000_then_text_after_p_end_within_bounds`,
    `table_with_100_000_misnested_a_ends_within_bounds`,
    `element_with_1_000_000_attributes_ends_within_bounds`,
    `input_of_16_mib_plus_one_byte_ends_within_bounds` (text);
    from the records: `reconstruction_of_1_000_formatting_elements_ends_within_bounds`,
    `nested_object_60_000_then_misnested_a_ends_within_bounds`,
    `a_then_div_20_000_then_a_end_tags_end_within_bounds`,
    `nested_b_30_000_with_different_ids_ends_within_bounds`,
    `nested_fieldset_with_legend_20_000_ends_within_bounds`;
    inside the five limits, from the table above:
    `object_490_then_misnested_a_to_16_mib_ends_within_bounds`,
    `a_then_div_500_then_a_end_tags_to_16_mib_end_within_bounds`,
    `span_505_then_unmatched_end_tags_to_16_mib_end_within_bounds`,
    `span_505_then_div_pairs_to_16_mib_end_within_bounds`,
    `b_505_with_1_024_attributes_each_ends_within_bounds`,
    `tags_with_1_024_attributes_to_16_mib_end_within_bounds`,
    `html_start_tags_with_new_attributes_end_within_bounds`.
  - `seeded_10_000_inputs_end_within_bounds`: 10,000 inputs of at most 4,096 bytes from a
    seeded generator written here (no crate) over an alphabet of markup fragments, the tags of
    the later tree stories (`table`, `td`, `select`, `template`, `svg`, `math`) and misnested
    formatting included, each parsed under `std::panic::catch_unwind` with no panic and each
    within the node and stored-byte limits, all 10,000 in under 30 seconds.

Writes: the files of `html-parser-limits-scope.txt`. For `AGENTS.md`, handed over in the report:
the `crates/html` entry's sentence "The parser has no limits yet (depth, nodes, time) until
`story:html-parser-limits` (wave 10)" becomes the six limits with their defaults, and the test
counts.

## Acceptance

- `cargo test -p surf-html` passes, among its tests the 27 of `tests/limits.rs` named in
  Change, and the adapted tests of `tests/tree_body.rs` and `tests/tree_formatting.rs`; the
  report quotes the time each adversarial test measured.
- `cargo test -p surf-page` passes with `a_deep_tree_is_read_without_recursion` adapted.
- `slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 and prints
  `html-conformance: 9107 cases, 9095 run, 8428 passed`.
- `slot task validate`, `slot task generated-check` and `slot task conform-rust` exit 0, and
  `spec/scenarios/parse-stopped-at-a-limit.yaml` writes and reads back all six variants.
- Test time (the gate's test step has 2 minutes of wall time, a single test 60 s): the test
  binary of `tests/limits.rs` ends within 30 s of wall time in the test profile on the unit's
  machine, no test of it runs over 10 s, and `cargo test -p surf-html` ends within 60 s after
  the build. The report names each test over 5 s with its time and lists the ten slowest. A
  shape that cannot meet that at 16 MiB runs at 1 MiB with `max_steps` 8,388,608 (2^23, the
  same visits per input byte), and `stack_visits_past_2_27_stop_at_the_step_limit` alone holds
  the default step limit; the unit says for each test which form it took. The times asserted
  in a test stay at 10 s (and 30 s for the seeded inputs): the bound of a loaded machine, not
  the budget.

## Out of scope

A clock inside the parser: the six limits bound the work and the tests measure the time.
Limits in `surf-fetch`, which has its own. Mapping `Limit` to `FinishParsing.stopped_by`
(`story:html-document-port`). Scripts: none runs, so none can grow the tree after parsing.
