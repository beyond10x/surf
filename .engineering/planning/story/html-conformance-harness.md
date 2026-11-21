---
format: aep.planning-md/3
id: story:html-conformance-harness
kind: story
status: implemented
title: Vendor the parser's conformance data and count it against a parser skeleton
summary: html5lib-tests (tokenizer, encoding) and web-platform-tests (tree construction) at pinned commits with their licences; a std-only harness reads 81 files, 9,107 cases, and prints cases that exist, run and pass.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-decisions
scope:
- confidence: cited
  path: crates/html
- confidence: cited
  path: crates/html-conformance
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-21T02:10:31Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

Story `html-decisions` writes `docs/design/html-parser.md`: how the conformance data enters the
repository, what one case is, what is compared, the case tags, and two fenced blocks a program
reads: `html-inventory` (every data file with its case count and its count per tag) and
`html-sections` (the map from sections of the standard to data files). This story brings the
data in and builds what reads it, against a parser that does nothing yet, so that every later
story of `epic:html-parser` starts from a count and raises it.

The root workspace takes a crate by creating its directory (`Cargo.toml` members `crates/*`,
`AGENTS.md`, "Layout"). The root `Cargo.lock` must list no package from outside this repository
(`AGENTS.md`, "Goals"), so the readers of the JSON and `.dat` formats are written here, with the
standard library. `Cargo.lock` is the coordinator's file in a wave.

The note settles the data (items 3, 8 and 9). Two sources, each pinned: html5lib-tests at
`c5495c539ca8a79c3c5107f5f14010bcff9a35d5` for `tokenizer/*.test` (14 files) and every file of
`encoding/` (5 files), and web-platform-tests at `13f8051342260922e50fd1f9d9877007ea025d2b` for
`html/syntax/parsing/resources/*.dat` (62 files, 1,870 cases): the tree-construction data is no
longer in html5lib-tests. 81 data files, 9,107 cases, 12 of them not run (`not-run:xml-infoset`
4, `not-run:needs-script` 7, `not-run:autodetect` 1), 9,095 run. Tokenizer tests are JSON
(`tests`, or `xmlViolationTests`, with `description`, `input`, `output`, `errors`,
`initialStates`, `lastStartTag`, `doubleEscaped`); a `ProcessingInstruction` token has three
members in the data (`["ProcessingInstruction", <target>, <data>]`). Tree-construction `.dat`
files have `#description`, `#data`, `#errors`, `#new-errors`, `#document`, `#document-fragment`,
`#script-on`, `#script-off` sections; a line that starts with `#` and has more after it is a
heading, `#description` starts a case and the `#data` after it belongs to the same case (wpt's
`resources/test.js`, `parseDat`). The tree is written as `| ` lines, a processing instruction as
`| <?target data?>` (81 tree-construction cases hold one). Encoding `.dat` files have `#data` and
`#encoding`; `encoding/chardet/test_big5.txt` is one case with neither.

## Change

- `crates/html-conformance/data/html5lib-tests/`: `tokenizer/*.test` and every file of
  `encoding/` at the html5lib-tests commit the note pins, with the upstream `LICENSE` and a
  `SOURCE` file (address, commit, date).
- `crates/html-conformance/data/wpt/html/syntax/parsing/resources/`: the 62 `*.dat` files at the
  wpt commit the note pins, and nothing else of that directory (no `.js`, no `.py`), with
  `crates/html-conformance/data/wpt/LICENSE.md` and `crates/html-conformance/data/wpt/SOURCE`
  (address, commit, date). Both sources are fetched into empty directories under the scratch
  directory first, then copied; the inventory paths begin with the source name
  (`html5lib-tests/…`, `wpt/…`), relative to `crates/html-conformance/data/`.
- `crates/html/` (package `surf-html`, `#![forbid(unsafe_code)]`, no dependency): the public
  interface the harness calls, each entry point returning an empty result for now, and one empty
  module file per later story so that stories in parallel write different files: `encoding.rs`,
  `input.rs`, `tokenizer.rs`, `charref.rs`, `tree.rs` (the node arena: document, doctype,
  element with namespace, local name and attributes, text, comment, processing instruction with
  target and data, template contents),
  `tree_builder.rs`, `limits.rs`. Proposed entry points, names the story may change: prescan
  and decode of bytes; tokenize a string from a given initial state with a last start tag;
  parse a document with a scripting flag; parse a fragment for a context element.
- `crates/html-conformance/` (package `surf-html-conformance`, `publish = false`, standard
  library and `surf-html` only): a JSON reader (with the double-escaped strings of the
  tokenizer data), a `.dat` reader with the `#description` rule above, a reader of the note's
  `html-inventory` block in its line formats (`source <name> <address> <commit>`, `file <group>
  <path> <cases> <tag>=<n>...` with `not-run:<reason>=<n>`, `tag <name> <n>`, `total <n>`), the tag of
  each case by the rule of the note, the tree written in the `| ` format of the data (the only
  serialization the epic allows), the runner, and a binary that prints one line per data file,
  one line per tag in the form `tag <name> <pass>/<run>/<exist>`, and a total line
  `html-conformance: <exist> cases, <run> run, <pass> passed`. Cases tagged `not-run` are counted
  as existing and not run, with the reason; the `tag not-run` line reads `0/0/12`. With
  `--list-failing <tag>` the binary prints instead one line `<path> <case number>` (cases numbered
  from 1 in file order, as the note numbers them) for each run case of that tag that does not
  pass.
- Tests in `crates/html-conformance/tests/`: `every_data_file_is_read_without_a_reader_error`,
  `the_case_count_per_file_equals_the_inventory`, `the_case_count_per_tag_equals_the_inventory`
  (the tag the harness computes for each case, counted per tag, against the counts of the
  `html-inventory` block), `every_case_has_exactly_one_tag`,
  `the_tree_format_round_trips_the_expected_trees` (every `#document` block read and written back
  byte for byte, the 81 cases with a processing instruction among them),
  `json_reader_rejects_malformed_input`, `a_description_section_starts_a_case`.

Writes: `crates/html/` (new), `crates/html-conformance/` (new). `Cargo.lock` gains two
repository packages; the coordinator regenerates it.

## Acceptance

`cargo test -p surf-html-conformance` passes with the seven tests named in Change, among them
`the_case_count_per_tag_equals_the_inventory`; `cargo run -p surf-html-conformance` exits 0 and
prints 81 file lines, the eleven `tag` lines of the inventory with their case counts as the
third number (`markup` 1979, `text` 328, `charref` 4739, `body` 476, `formatting` 22, `tables`
39, `template` 123, `foreign` 222, `mixed` 1085, `encoding` 82, `not-run` 12), and the total line
`html-conformance: 9107 cases, 9095 run, <pass> passed`; and `cargo tree -p surf-html-conformance` lists
only `surf-html-conformance` and `surf-html`.

## Out of scope

Any parsing (later stories), the gate (story `html-conformance-gate`). The pass count is whatever
an empty parser passes, recorded and not raised here. No `AGENTS.md` or `README.md` change.

## Scope

Derived 2026-11-21 by `story-scoper`.

- **Primary surface:** `crates/html-conformance/` — cited, the story's Change and `Writes:` line (`.engineering/planning/story/html-conformance-harness.md:64`) name it as a new crate. The root `Cargo.toml` takes it through the `crates/*` glob (`Cargo.toml:3`, `AGENTS.md:93-96`). `crates/` today holds only `fetch`, `ports` and `surf`.
- **Files written:** `crates/html-conformance/data/html5lib-tests/` (`tokenizer/` 14 files, `encoding/` 5 files, the upstream `LICENSE` and `SOURCE`) and `crates/html-conformance/data/wpt/` (`html/syntax/parsing/resources/*.dat`, 62 files, `LICENSE.md` and `SOURCE`) — cited, Change bullets 1 and 2 and `docs/design/html-parser.md` item 3. Revised after wave 1: the second source and the directory lists come from the note.
- **Files written:** `crates/html-conformance/Cargo.toml` (package `surf-html-conformance`, `publish = false`, one dependency `surf-html` by path) — cited, Change bullet 3.
- **Files written:** `crates/html-conformance/src/` (JSON reader, `.dat` reader, inventory reader, tag rule, `| ` tree writer, runner, binary) — cited as a set, Change bullet 3. The module file names and `src/main.rs` are inferred. `cargo run -p surf-html-conformance` needs one binary in the package.
- **Files written:** `crates/html-conformance/tests/` with the seven tests named in the last Change bullet — cited. The file names inside `tests/` are inferred.
- **Files written:** `crates/html/Cargo.toml` (package `surf-html`, `#![forbid(unsafe_code)]`, no dependency) — cited, Change bullet 2.
- **Files written:** `crates/html/src/lib.rs` — cited, as the public interface the harness calls (Change bullet 2). That the entry points live in `lib.rs` is inferred.
- **Files written:** `crates/html/src/{encoding,input,tokenizer,charref,tree,tree_builder,limits}.rs` — cited, the seven empty module files named in Change bullet 2. `tree.rs` also holds the node arena types.
- **Files read, not written:** `docs/design/html-parser.md`, the `html-inventory` and `html-sections` blocks — cited, Context and Change bullet 3. The path is inferred from `html-decisions`' Scope (`story/html-decisions.md`, `scope: docs/design/html-parser.md`). The inventory reader reads this file at run time and in the tests, and the crate's tests fail without it.
- **Files read, not written:** `AGENTS.md` "Goals" and "Layout" (no third-party crates in the root lock; a crate joins by creating its directory) — cited.
- **Handed to the coordinator:** root `Cargo.lock` gains two packages, `surf-html` and `surf-html-conformance`. It is regenerated with `cargo generate-lockfile --offline`, because `task build` and `task test` use `--locked` (`AGENTS.md:94-96`, `AGENTS.md:237`) — cited.
- **Handed to the coordinator:** `AGENTS.md`, `README.md` and the root `Taskfile.yml`: nothing — cited, "Out of scope": "No `AGENTS.md` or `README.md` change". The story adds no task. The `html-conformance` task belongs to `html-conformance-gate` (`taskfiles/html-conformance.yml`). Inferred: "Layout" will want a line for the two crates, but the story does not ask for it.
- **Symbols:** `surf-html` public entry points: prescan and decode of bytes; tokenize a string from an initial state with a last start tag; parse a document with a scripting flag; parse a fragment for a context element. The story marks all of them as "names the story may change", and each returns an empty result — cited, with the exact names and types inferred. `surf-html` also gets the arena types in `tree.rs` (document, doctype, element with namespace, local name and attributes, text, comment, processing instruction, template contents) — cited. The `surf-html-conformance` library items and the binary are not public API for anyone (nothing depends on the crate), so the exact items are inferred.
- **Symbols:** the output format the binary prints (one line per data file, `tag <name> <pass>/<run>/<exist>`, and `html-conformance: <exist> cases, <run> run, <pass> passed`) — cited. Later acceptances and the gate read these lines.
- **Would collide with:** `crates/html/src/*.rs` and `crates/html/Cargo.toml`, written by later parser stories — cited from their `Writes:` lines: `html-tokenizer-markup` (`input.rs`, `tokenizer.rs`), `html-tokenizer-text` (`tokenizer.rs`), `html-character-references` (`charref.rs`, `tokenizer.rs`), `html-input-stream` (`encoding.rs`), `html-tree-body` (`tree.rs`, `tree_builder.rs`, `lib.rs`), `html-tree-formatting`, `html-tree-tables`, `html-tree-template` (`tree.rs`), `html-tree-foreign` (`lib.rs`), `html-tree-complete`, and `html-parser-limits` (`limits.rs`, `lib.rs`, `tree.rs`).
- **Would collide with:** `crates/html-conformance/` (`src/`, `expected/`, `tests/`, `data/`), written by `html-conformance-gate`, which is `depends_on` this story, and by `html-character-references` (`data/whatwg/`, `src/`, `expected/tokenizer.txt`).
- **Would collide with:** all of these list this story in their `depends_on` chain (the gate on the harness, the rest on the gate), so the collisions are ordered by edges and not concurrent — cited. Nothing outside the HTML epic writes either crate.
- **Would collide with:** `html-decisions` does not write them, but its `html-inventory` block is the data this story reads — cited.
- **Not yet existing when it starts:** `docs/design/html-parser.md`, with the `html-inventory` and `html-sections` blocks, the case tags, the pinned commit and the kept directories — delivered by `story:html-decisions` (cited, `depends_on`). Network access to fetch html5lib-tests, which no story delivers (inferred from `html-decisions`' own note on this).
- **Confidence:** high for the surface, because the story names both crates and every top-level directory. Medium inside the crates: the file names under `src/` and `tests/` of `surf-html-conformance`, the exact entry-point names, and the vendored directory list are not fixed by the story. I did not check the exact shape of `docs/design/html-parser.md` or the actual html5lib data, because neither exists on this branch yet.

