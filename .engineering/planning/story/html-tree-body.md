---
format: aep.planning-md/3
id: story:html-tree-body
kind: story
status: active
title: Build the tree outside tables, templates and foreign content
summary: The node arena and the insertion modes from initial to after after frameset; every tree case tagged body passes.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-character-references
- depends_on: story:html-input-stream
scope:
- confidence: cited
  path: crates/html-conformance/expected/encoding.txt
- confidence: cited
  path: crates/html-conformance/expected/tree-construction.txt
- confidence: cited
  path: crates/html-conformance/tests/check.rs
- confidence: cited
  path: crates/html/src/encoding.rs
- confidence: cited
  path: crates/html/src/lib.rs
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
  path: crates/html/tests/not_yet_parsed.rs
- confidence: cited
  path: crates/html/tests/tree_body.rs
revision: 19
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 5}
---
## Context

Tree construction in the HTML Standard is a set of insertion modes over a stack of open elements
(13.2.6 "Tree construction" of the snapshot `docs/design/html-parser.md` names). This is the first
of the tree stories, cut by the case tags of `docs/design/html-parser.md` (story
`html-decisions`, item 8): `body`, `formatting`, `tables`, `template`, `foreign`, then `mixed`
(the mixed files, story `html-tree-complete`). It needs the whole tokenizer, because the tree
builder switches it into RCDATA, RAWTEXT, script data and PLAINTEXT (story
`html-character-references`, the last tokenizer story).

What is there on `main` at `01547cd`: `crates/html/src/tree.rs` holds the node types and three
read accessors (`root`, `data`, `children`) and no operation that adds a node;
`crates/html/src/tree_builder.rs` holds only its module comment; `parse_document`,
`parse_fragment` and `parse_document_bytes` in `crates/html/src/lib.rs` return a tree holding only
its root. The tokenizer runs from one state to the end of the input in one call
(`tokenizer::tokenize`, `crates/html/src/tokenizer.rs`); nothing can switch its state between two
tokens. The harness (`crates/html-conformance/src/runner.rs`, lines 6 to 8) compares the tree of
a tree-construction case byte for byte and does not compare its `#errors`; it calls
`parse_fragment` for every `#document-fragment` case, all of which are tagged `foreign`.

The `body` cases, 476 (`html-inventory`), are in 17 thematic files; per file, from the `file`
lines of the inventory: `blocks.dat` 48, `comments01.dat` 16, `doctype01.dat` 36,
`domjs-unsafe.dat` 32, `entities01.dat` 75, `entities02.dat` 26, `inbody01.dat` 4, `isindex.dat`
4, `main-element.dat` 2, `menuitem-element.dat` 18, `noscript01.dat` 18,
`plain-text-unsafe.dat` 15, `processing-instructions.dat` 119, `ruby.dat` 21,
`scriptdata01.dat` 26, `search-element.dat` 5, `void-in-phrasing.dat` 11. No `body` case has
`#script-on`; the scripting flag enabled is read by eight `mixed` cases (`tests16.dat` 84, 86, 88,
181, 183, 185, `tests5.dat` 16, `webkit02.dat` 2). No `body` case holds a formatting element's tag,
so no `body` case reads the list of active formatting elements beyond its markers.

It also takes the last step of the encoding cases. `docs/design/html-parser.md` item 4: a `meta`
that the tree builder meets while the confidence is tentative changes the encoding as 13.2.3.4
"Changing the encoding while parsing" says, and since surf holds the whole body it re-parses the
same bytes from memory with the new encoding and the confidence certain, inside the one parse,
with no second navigation; the optional on-the-fly switch is not taken. Seven encoding cases,
`encoding/tests1.dat` cases 48 to 54, pass only through this step: each puts a
`<meta charset="iso8859-2">` after the 1,024 bytes the prescan reads. Story `html-input-stream`
left them failing (`tag encoding 75/82/82`) and named `encoding::decode_with` as the entry for
this step (`docs/plan/agent-wave-5.md`, findings).

Two findings of earlier waves are addressed to this story:

- The tokenizer replaces its last start tag with each start tag it emits
  (`crates/html/src/tokenizer/machine.rs`, `emit_tag`), and no run of `tokenize` re-enters a text
  state after a start tag, so no test observed it (`docs/plan/agent-wave-6.md`, findings). This
  story is the first to switch the state between tokens: one tokenizer serves the whole parse,
  and the tree builder never passes a last start tag in; the end tag that ends a `title`,
  `textarea`, `style`, `script`, `xmp`, `iframe`, `noembed`, `noframes` or `noscript` is
  appropriate because the machine kept that element's start tag.
- The named character reference state reads the next input character for its attribute rule
  without first reporting that character's input stream error (`machine.rs`,
  `named_character_reference`: `self.input.get(self.input.position())` with no `examine`), where
  every other lookahead of the machine reports the next input character's error first
  (`next_characters_are`; the order decided in wave 5) (`docs/plan/agent-wave-7.md`, findings).
  The tree builder does not read the tokenizer's errors and the harness compares no tree case's
  errors, so the tree cases cannot hold it; this story changes `machine.rs` for the stepping
  anyway and corrects the order there, held by a tokenizer unit test.

`story:html-document-port` parses real pages before the tree chain ends
(`docs/plan/agent-plan-revision-1.md`, D12), so the parser says which parts it does not yet
parse as the standard does, and each later tree story shortens that list.

## Change

- `crates/html/src/tokenizer.rs`, `crates/html/src/tokenizer/machine.rs`: a tokenizer that gives
  one token at a time and whose state the tree builder can set between two tokens
  (`pub(crate)`), used by the tree builder for the whole parse. `tokenize` keeps its signature
  and its output and runs over the same machine, so the tokenizer counts do not move. In the
  named character reference state, when the reference was consumed as part of an attribute and
  the last character matched is not `;`, the next input character's parse error of 13.2.3.5 is
  reported before the state looks at that character, as `next_characters_are` does; outside an
  attribute nothing changes. A unit test in `tokenizer.rs`,
  `the_next_input_characters_error_comes_first_after_a_named_reference_in_an_attribute`:
  `<a b='&amp` U+0001 `'>` gives the errors `control-character-in-input-stream`,
  `missing-semicolon-after-character-reference` in that order (the reverse before the change),
  and `&amp` U+0001 in data gives them in the reverse order, unchanged.
- `crates/html/src/tree.rs`: the node arena with the operations tree construction uses
  (`pub(crate)`: create an element for a token, insert at the appropriate place, append text
  merging into an adjacent text node, add the attributes an `html` or `body` start tag brings
  that the element lacks, remove a node from its parent, which the `frameset` rule of in body
  does to `body`). Node identities stay the same for the life of the tree: a node is never
  dropped from the arena, and one removed from its parent is reached from the root no longer.
  Two read accessors are added for the stories that read the tree
  (`story:html-actionable-elements` reads ancestors and document order): `Tree::parent` and
  `Tree::descendants` (the nodes under a node in tree order, the node itself first, template
  contents not included). Neither recurses over the tree's depth. The tree has no public
  operation that changes it after parsing (D2): every `pub fn` of `tree.rs` takes `&self`.
- `crates/html/src/tree_builder.rs` (submodules under `crates/html/src/tree_builder/` allowed):
  the initial mode with the quirks mode decision from the DOCTYPE, before html, before head, in
  head, in head noscript, after head, in body as bounded below, text, after body, in frameset,
  after frameset, after after body, after after frameset; the scripting flag from
  `ParseOptions::scripting_disabled` for `noscript` in head and in body; the stack of open
  elements and the element scopes; generate implied end tags; the list of active formatting
  elements with its marker, "push onto the list" without the Noah's Ark clause, and "reconstruct
  the active formatting elements"; stop parsing. Parse errors are not collected: neither the
  tokenizer's nor the tree builder's reach a caller.
  - In body, bounded: every rule of the mode is this story's except those the later stories own.
    "Any other start tag" and "any other end tag" are this story's (`body` cases need them, for
    example `</menuitem>`). The start tags `b`, `big`, `code`, `em`, `font`, `i`, `s`, `small`,
    `strike`, `strong`, `tt`, `u`, `a` and `nobr` reconstruct, insert and push; the steps that
    run the adoption agency algorithm for an `a` already in the list or a `nobr` in scope, and
    Noah's Ark, are `html-tree-formatting`'s. The end tags of those elements go to the adoption
    agency hook, which until `html-tree-formatting` does what "any other end tag" does and
    removes the element from the list of active formatting elements. The start tags `caption`,
    `col`, `colgroup`, `frame`, `head`, `tbody`, `td`, `tfoot`, `th`, `thead`, `tr`, which in body
    ignores, are this story's. In the `option`, `optgroup` and `hr` rules the clauses that test
    for a `select` element are `html-tree-tables`'s; this story writes those rules as if no
    `select` is in scope.
  - Every token whose rule a later story writes (a `table` or `select` start tag, a `template`
    start or end tag in head or body, an `svg` or `math` start tag) is processed by in body's
    "any other start tag" or "any other end tag" until that story, so that no input loops,
    panics or reaches a missing mode; the case then fails and does not crash the harness.
  - A unit test in `tree_builder.rs`, `the_doctype_decides_the_quirks_mode`: no DOCTYPE and the
    public identifier `-//W3O//DTD W3 HTML Strict 3.0//EN//` give quirks mode, the XHTML 1.0
    Transitional public identifier with a system identifier gives limited-quirks mode,
    `<!DOCTYPE html>` gives no-quirks mode. No `body` case can see the mode: it changes only
    the `table` rule of in body (`html-tree-tables`).
- `NOT_YET_PARSED: &[&str]`, public in `crates/html/src/tree_builder.rs` and re-exported from
  `lib.rs`, naming each part the tree builder does not yet parse as the standard does, in this
  order: `misnested-formatting`, `tables-and-select`, `template`, `foreign-content`. Each entry
  matches `[a-z-]+`, as the answer's renderer requires (`docs/plan/agent-plan-revision-2.md`,
  D31). Each later tree story removes its entry; `html-tree-complete` leaves it empty. A test
  that holds a `note parser` line builds it from this constant, never from a literal (D31), so a
  story that shortens the list breaks no other story's test.
- `crates/html/tests/not_yet_parsed.rs`: `every_not_yet_parsed_entry_matches_a_z_and_hyphen`
  (every entry non-empty and of the characters `a` to `z` and `-` only). It does not assert the
  number or the names of the entries, so each later shortening keeps it passing without a
  change to this file.
- Processing instruction tokens (story `html-tokenizer-markup`) inserted as processing
  instruction nodes where the insertion modes of this story say, as the 119 `body` cases of
  `processing-instructions.dat` expect.
- The `meta` rules of in head (also reached from in body): with the confidence tentative, a
  `charset` attribute whose value gets an encoding (`encoding::get_an_encoding`), or else an
  `http-equiv` of `Content-Type` (ASCII case-insensitive) with a `content` from which the
  algorithm for extracting a character encoding from a `meta` element gets one, runs 13.2.3.4:
  in use UTF-16BE or UTF-16LE, or the new encoding the one in use, makes the confidence certain
  and nothing more; a new UTF-16BE or UTF-16LE becomes UTF-8 and `x-user-defined` becomes
  windows-1252 first; otherwise `parse_document_bytes` decodes the same bytes again with
  `encoding::decode_with` (confidence certain), discards the tree and starts the parse over.
  The encoding in use is `Decoded::encoding`, not `decoded_with`: an encoding decoded as
  windows-1252 (`ISO-8859-2` in cases 48 to 54) is still the one in use. A re-parse runs with the
  confidence certain, so a parse re-parses at most once. `parse_document`, which takes decoded
  text, has no confidence and never changes the encoding. The extraction is the one the prescan
  already has (`extract_from_meta_content` in `crates/html/src/encoding.rs`), made `pub(crate)`;
  nothing else of `encoding.rs` changes. The step numbers and their order are confirmed by the
  unit against the snapshot before it writes them.
- The entry points: `parse_document` and `parse_document_bytes` run the tokenizer and the tree
  builder; `parse_fragment` stays as it is (story `html-tree-foreign`). The section "State" of
  the crate documentation in `lib.rs` and the module documentation of `tokenizer.rs` say what is
  written after this story.
- `crates/html/tests/tree_body.rs`, each test holding behaviour of the standard that the later
  tree stories keep:
  - `node_identities_and_document_order_are_stable`: `<p>a<p>b` parsed once, `descendants` of
    the root read twice gives the same identities, in the order document, `html`, `head`,
    `body`, `p`, `"a"`, `p`, `"b"`.
  - `a_frameset_after_an_implied_body_takes_its_place`: `<div><frameset>` gives a root whose
    descendants are the document, `html`, `head`, `frameset`; the removed `body` is not among
    them.
  - `reconstructing_clones_a_formatting_element_after_a_p_end_tag`: `<p><b>x</p>y` gives `body`
    with the children `p` (holding `b` holding `"x"`) and a second `b` holding `"y"`.
  - `an_end_tag_of_another_element_stays_text_in_a_title`: `<title>a</textarea>b</title>c` gives
    a `title` holding the one text `"a</textarea>b"` and `"c"` in `body`.
  - `a_meta_after_the_prescan_changes_the_encoding_once`: 2,048 bytes of comment, then
    `<meta charset="iso-8859-2"><meta charset="utf-8">`: `ParsedDocument::encoding` is
    `ISO-8859-2`.
  - `a_meta_does_not_override_a_byte_order_mark`: a UTF-8 byte order mark, then
    `<meta charset="windows-1252">`: the encoding is `UTF-8`.
- `crates/html-conformance/tests/check.rs`: `TREE_DATA` only. Its case (`<p>` with the expected
  tree `html`, `head`, `body`, `p`) is the correct tree, so a working tree builder passes it, and
  three tests that assert the fixture's tree cases fail turn red:
  `a_count_above_the_expected_count_passes` (`html-conformance: 6 cases, 5 run, 1 passed`),
  `a_section_without_a_data_file_is_printed_incomplete_with_its_reasons` and
  `a_write_wrapper_line_counts_one_test_per_data_file_of_its_section` (`section
  tree-construction incomplete 0/2 failing:2 …`). The expected tree is changed to one that no
  parser gives for its input (`<q>` in place of `<p>`), as `TEST1_DATA` and `ENC_DATA` already
  do, and a comment says so; no assertion changes.
- `crates/html-conformance/expected/tree-construction.txt` and
  `crates/html-conformance/expected/encoding.txt`: every count raised to the one measured,
  those of the other tags and the mixed files that pass by the way included, so that the check
  prints no `raise:` line and the next tree story cannot lose a case this one passes.

Writes: `crates/html/src/tree.rs`, `crates/html/src/tree_builder.rs`,
`crates/html/src/tree_builder/`, `crates/html/src/lib.rs` (the parse entry points, the
re-export, the re-parse of 13.2.3.4 and the crate documentation), `crates/html/src/tokenizer.rs`
and `crates/html/src/tokenizer/machine.rs` (the stepping, the error order and their unit test),
`crates/html/src/encoding.rs` (one function made `pub(crate)`),
`crates/html/tests/not_yet_parsed.rs` (new), `crates/html/tests/tree_body.rs` (new),
`crates/html-conformance/tests/check.rs` (`TREE_DATA` only),
`crates/html-conformance/expected/tree-construction.txt`,
`crates/html-conformance/expected/encoding.txt`. No dependency is added; `Cargo.lock` does not
change.

For `AGENTS.md`, handed over in the report (the coordinator's file): the `crates/html` entry of
"Layout" (it says `parse_document` and `parse_fragment` return a tree holding only its root, and
"28 tests: 15 unit tests and 13 in `tests/encoding.rs`") rewritten for what this story leaves,
and the gate's numbers in "The gate's numbers".

## Acceptance

- `slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 and prints
  `tag body 476/476/476`, `tag encoding 82/82/82`, `tag markup 1979/1979/1979`,
  `tag text 328/328/328`, `tag charref 4739/4739/4739`, the line
  `section the-input-byte-stream incomplete 82/84 not-run:2 open:transport-layer-encoding,encodings-without-decoder,insertion-point wpt:17`
  (computed from `check.rs` `section_line` and the note's `html-sections` block: 84 cases, 2
  not run, 3 `open` and 17 `wpt` lines), a total line `html-conformance: 9107 cases, 9095 run,
  <n> passed` with `<n>` at least 7,604 (7,121 before, 476 and 7 more), and no line starting
  `raise:`.
- `crates/html-conformance/expected/tree-construction.txt` holds `tag body 476/476/476` and, for
  each of the 17 files above, a pass count at least its `body` count (`scriptdata01.dat
  26/26/26`, whose every case ends a `script` by the last start tag the tokenizer kept);
  `crates/html-conformance/expected/encoding.txt` holds `file
  html5lib-tests/encoding/tests1.dat 59/59/59` and `tag encoding 82/82/82`.
- `cargo run -p surf-html-conformance -- --list-failing encoding` prints nothing (cases 48 to 54
  of `encoding/tests1.dat` pass), and `cargo run -p surf-html-conformance -- --list-failing
  mixed` prints none of `wpt/html/syntax/parsing/resources/tests16.dat` 84, 86, 88, 181, 183,
  185, `tests5.dat` 16 and `webkit02.dat` 2, the cases that read `noscript` with scripting
  enabled.
- `cargo test -p surf-html` passes, among its tests
  `every_not_yet_parsed_entry_matches_a_z_and_hyphen`, the six tests of `tests/tree_body.rs`
  named in Change, `the_doctype_decides_the_quirks_mode` and
  `the_next_input_characters_error_comes_first_after_a_named_reference_in_an_attribute`; and
  `crates/html/src/tree_builder.rs` declares `NOT_YET_PARSED` as the four entries named in
  Change, in that order.
- `cargo test -p surf-html-conformance` passes, the 17 tests of `tests/check.rs` among them.
- `grep -n "pub fn" crates/html/src/tree.rs` lists no function taking `&mut self` (today `Namespace::url`,
  `Tree::root`, `data`, `children`; after it also `parent` and `descendants`), and no file under
  `crates/html/src/` holds `unsafe`.

## Out of scope

The adoption agency algorithm, Noah's Ark and the `a` and `nobr` steps that run it (story
`html-tree-formatting`), tables and select (story `html-tree-tables`), `template` (story
`html-tree-template`), MathML, SVG and fragments (story `html-tree-foreign`), limits (story
`html-parser-limits`): until that story the parser has no bound on depth, nodes or stored bytes,
and nothing of this story recurses over the tree's depth. Parse errors are not reported to a
caller. Scripts: a `script` element is inserted and its contents kept as text; at its end tag in
the text mode the standard prepares the script, pauses the parser for a parsing-blocking script
and runs it, and surf only pops the element and returns to the original mode. Left unsupported,
and listed by the note as open requirements: script execution, the script nesting level, parser
pausing, the insertion point and `document.write`, and in "the end" the deferred scripts and the
`DOMContentLoaded` and `load` events. Custom elements: "create an element for a token" looks up
no definition and runs no constructor.
