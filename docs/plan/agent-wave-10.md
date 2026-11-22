# Agent interface plan, wave 10 of 22: the parser's limits and the label of an element

One wave of the [wave plan](agent-waves.md). Two stories, each in its own worktree
(`impl/html-parser-limits`, `impl/html-accessible-name`), each given to an adversary once it was
green; the fixes of each adversary's findings are one commit on the unit's branch. The
coordinating session wrote no code. Every commit is the organization bot's.

After this wave a parse of untrusted input is bounded: input bytes, depth, nodes, attributes of
one element, stored bytes and steps each have a limit with a documented default, and a parse
that meets one stops, keeps what it built and names the limit. `surf-page` gives each
actionable element a label, its accessible name computed over the tree without CSS, cut at
1,024 bytes, with a bound on the work of all labels of a page. The specification's
`surf.document.ParseLimit` gained `Steps`. No command runs the parser on a page yet: a real
page's document stays `Parsing` until `html-document-port`, and a user of the commands sees no
change.

Goal story or consumer gap moved: none.

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `html-parser-limits` | `crates/html/src/limits.rs`: `Limits` (`ParseOptions::limits`) with `max_input_bytes` 16,777,216, `max_depth` 512, `max_nodes` 1,048,576, `max_attributes` 1,024, `max_stored_bytes` 67,108,864 and `max_steps` 134,217,728 (2^27), and `Limit`; `Tree::stopped_by`, `node_count`, `stored_bytes`; the checks in the tokenizer and the tree builder; the special and scope categories decided once per element; Noah's Ark and the `html` and `body` attribute merge without a scan per attribute. `spec/domains/document.yaml`: `ParseLimit` gains `Steps`; `parse-stopped-at-a-limit.yaml` a sixth document; `generated/rust/` and `conformance/src/target.rs` follow. `docs/design/html-parser.md` item 7 | 28 tests in `tests/limits.rs`; the 126 tests of `surf-html`, `surf-page` and `surf-html-conformance` in 7.1 s; `html-conformance: 9107 cases, 9095 run, 8428 passed`, as before; 95 scenarios against the Rust code; 12 planted defects, 12 red | `eb5e615` |
| four findings and seven mutants of the limits review (after the adversary's pass) | the adoption agency algorithm and the reconstruction copy no start tag token per step; letters after an end tag opener in RCDATA, RAWTEXT and script data from one buffer; 80 stored bytes per attribute (`limits::ATTRIBUTE_BYTES`); a count per node on the stack of open elements; `Tree::steps`; `tests/adversary_limits.rs` (new) and `tests/common/mod.rs`, a counting allocator per thread | 146 tests of `surf-html`, `surf-page` and `surf-html-conformance`: `tests/limits.rs` 36, `tests/adversary_limits.rs` 11, one unit test in `src/tree.rs`; `html-conformance` unchanged | `3a8e882` |
| `html-accessible-name` | `crates/page/src/label.rs` (new): the accessible name by Accessible Name and Description Computation 1.2, 4.3.2, with HTML-AAM 4.1, each node read once per name, no recursion; `LABEL_BYTES_MAX` 1,024 and `LABEL_WORK_MAX` 16,777,216. `crates/page/src/lib.rs`: `ActionableElement` gains `label` and `label_cut`; the one walk records the dense node index with the hidden flags, the first element per ID and each `label` with its control; the crate documentation's "Steps not taken" and "Deviations" | 31 tests in `tests/label.rs`, 55 in the crate; `cargo clippy` and `cargo fmt` exit 0; 12 planted defects, each red (a thirteenth, a submit button with an empty value, never ran and went to the adversary) | `677836f` |
| the work bound did not bound time (after the adversary's pass) | the walk passes once over the attributes of each element and records where the eleven attributes a name reads sit; a name looks them up in constant time; `surf_page::attribute_visits()`, hidden from the documentation, counts attribute comparisons per thread | 34 tests in `tests/label.rs`, 58 in the crate, 58 passed; the new bound test counts 20,590,004 comparisons for 26,001 attributes on the old code | `a6ca450` |

The merges into the wave branch: `a22e1e6` (`html-parser-limits`), `a4fa398`
(`html-accessible-name`), and `1b234c4`, which merged `main` with release `0.3.0` into the
wave: both sides changed the specification, so every file under `generated/` was written again
by `task generate` from the merged specification. The plan commits on the wave branch beside
them: `9eff815` (the two stories revised), `f551a0d`, `8c74ad3`, `daedab1`, `328ca26`, `b7f7be0`
(the rounds after wave 10 and the work they are ordered by), `7faaa1f` and `3002dc3`
(`one-test-binary-per-crate` and the TLS stories beside the rounds).

## The stories were measured before the wave

Both stories were read against `main` by a reviewer first, as in waves 3 to 9. The bodies were
revised and each got a typed scope with no path in both (`9eff815`).

Decided before the wave:

- **Six limits, the sixth a step limit.** The five of the design note bound memory, not time:
  an input inside all five can still cost tokens times depth stack visits (505 nested `span`,
  then `</q>` to 16 MiB, ran 122.9 s in a release build). `steps` (2^27) went spec first:
  `surf.document.ParseLimit`, the scenario, `task generate`, `conformance/src/target.rs`.
- **An input over the input limit is cut and parsed to its end**, so the tree is that of the
  prefix. No clock inside the parser.
- **A label is cut at 1,024 bytes and says so** (`label_cut`), and the work of all labels of a
  page has a bound (`LABEL_WORK_MAX`); past it a name comes from the element's own attributes.
- **The test time is part of each acceptance**: the gate's test step has 120 s, a test 60 s,
  and a test over 5 s is named with its time.
- **`docs/design/html-parser.md` is `html-parser-limits`'s alone** (item 7); the label's
  sentences for item 5 were handed over and written by the coordinator after the wave.

The findings wave 9 sent here are settled. The inputs of the records (a formatting end tag on
a deep stack, the list of active formatting elements that grows with different attributes,
nested blocks) are stopped by the depth limit after a few hundred tokens and are cases of
`tests/limits.rs`. `is_hidden` is called for no node by the labels: the hidden flag of every
node a name reads comes from the one walk, and `hidden_flags_cost_one_walk_for_many_references`
holds the two shapes that cost more than the depth.

## What went wrong, and what was decided

- **The limits cost more than they said in four places.** The adversary of the limits found
  that the adoption agency algorithm and the reconstruction copied a start tag token per step
  (a formatting end tag behind a `table` copied 1,024 attributes for 4 input bytes); that the
  letters after an end tag opener in RCDATA, RAWTEXT and script data cost 168 bytes of heap
  per input byte; that an attribute with a one-byte name counted 1 stored byte while it held
  80, so the default admitted about 2.3 GB of heap; and that text 20 elements below an
  unclosed `<b>` cost 20 steps a byte, so a 1 MiB page of it was cut. Fixed in `3a8e882`: no
  copy per step, 11 bytes of heap per input byte, 80 stored bytes per attribute (a tree's heap
  is at most 2 × `stored_bytes()` + 3 × 136 × `node_count()`), and a count per node on the
  stack, so asking whether a node is open visits no entry. Seven mutants the tests did not
  kill have a test each now.
- **Bounds of 10 s wall time failed on a loaded machine.** Decided: the tests of
  `tests/limits.rs` and `tests/adversary_limits.rs` assert no time. They assert the limit that
  stopped a parse, its steps and the bytes it allocated, read from a counting allocator per
  thread; seven tests compare two inputs that differ only in what one scan visits. The times
  are printed, and read by no assertion.
- **The work bound of the labels did not bound their time.** Each element a name read was
  searched for `aria-label`, `title` and `aria-labelledby` among all its attributes and charged
  one unit: with 1,024 attributes on each element read the names took 1.56 s against 6.6 ms
  with one. Fixed in `a6ca450`: the walk records where the attributes a name reads sit, and a
  test holds the comparisons at 16 per attribute of the page by a count, not a time. Two more
  cases of the adversary stay: a button `input` embedded in a label, and a text control named
  in its own `aria-labelledby`. A recheck found no failing case.
- **The deep label test could not nest under a link.** With an `a` in the list of active
  formatting elements each `span` start tag reconstructs the list against the whole stack, and
  100,000 nested `span` in a link took over 60 s to parse at the base of the unit. The test
  nests the `span` elements in a `button` named by a link through `aria-labelledby`.
- **The story's whitespace example was wrong.** `<a href> a\n\tb <span>c</span>d</a>` is
  `a b cd`, as the test has it: the text of an inline child joins without a space (4.3.2,
  2F and 2G). The story's Change section is corrected.
- **The specification said a label is carried whole.** The comment on `label` in
  `spec/domains/document.yaml` now says it is cut at 1,024 bytes, at the last UTF-8 character
  boundary at or below that; no byte of the suite changed.

Measured after the wave: the worst page measured within all default limits parses in 12.1 s in
a dev build and 0.75 s in a release build (2^27 steps); nesting is stopped at depth 512 in
under 16 ms.

## The gate on the wave branch

`slot task check-runner` runs on the wave branch after this page. Read before it, each with
its own task on the wave branch with this page's changes (`slot task validate`, `suite`,
`conform`, `generated-check`, the planted-defect test alone):

```
surf v1 — 9 file(s), 15 scenario(s), valid
96 scenario(s) (15 authored), 28 refusal(s), written to .out/suite.json
  96 scenarios: 96 passed, 0 failed, 0 error, 0 unsupported
generated/rust matches spec/ (30 artifact(s) and .ess-output)
planted defect: 96 scenarios: 62 passed, 34 failed, 0 error, 0 unsupported
```

The suite's digest is `sha256:3041446cb0234779316d887f5a96d127f60fc03893977e2b341f92e69af28b81`
(`sha256:579a46b3…` before the wave): `ParseLimit` gained `Steps` and the scenario a sixth
document. `task test` is to print `661 tests: 661 passed, 0 failed, 0 ignored (cargo-nextest
659, doc tests 2)`: the 579 of `main` and the 82 `#[test]` lines the wave adds (36 in
`crates/html/tests/limits.rs`, 11 in `adversary_limits.rs`, 34 in `crates/page/tests/label.rs`,
1 in `crates/html/src/tree.rs`; none removed). The wave changed no manifest and no
`Cargo.lock`; the units reported `html-conformance: 9107 cases, 9095 run, 8428 passed`, as
before.

## Findings, and where each goes

| finding | from | goes to |
|---|---|---|
| a label passes U+2028 and U+000B through unchanged (they are not ASCII whitespace, so they are neither stripped nor collapsed); no test covers it | the close of the wave | open: no story names it yet |
| `crates/page/tests/label.rs`, `crates/html/tests/limits.rs` and `adversary_limits.rs` are test binaries of their own | the units | `story:one-test-binary-per-crate`, alone, before round 1 |
| the worst page measured within all default limits parses in 12.1 s in a dev build and 0.75 s in a release build (2^27 steps) | the close of the wave | accepted: the step limit bounds it; nesting is stopped at depth 512 in under 16 ms |
| still unbounded: the scan of one token before the step check, end tags in the public `tokenizer::tokenize`, and the decoded text of `parse_document_bytes` (up to 3 bytes per input byte) | `html-parser-limits`, the unit's report | open: no story names it yet |
| `hidden_flags_cost_one_walk_for_many_references` asserts 10 s of wall time, which the limits tests dropped after a loaded machine failed such bounds | the close of the wave | open: to settle when `story:one-test-binary-per-crate` moves the file |
| the comment on `label` in `spec/domains/document.yaml` said "whole" | `html-accessible-name`, its Out of scope | closed: written in the close of the wave |
| item 5 of `docs/design/html-parser.md` said "The row holds the whole name" | `html-accessible-name`, handed over | closed: written in the close of the wave |

## After this wave

`story:one-test-binary-per-crate`, alone, then round 1 of [the wave plan](agent-waves.md)
("Rounds after wave 10").
