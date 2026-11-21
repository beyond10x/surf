---
format: aep.planning-md/3
id: story:html-conformance-harness
kind: story
status: active
title: Vendor the html5lib test data and count it against a parser skeleton
summary: The data is in the repository with its licence; a std-only harness reads it and prints cases that exist, run and pass.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-decisions
scope:
- confidence: cited
  path: crates/html
- confidence: cited
  path: crates/html-conformance
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 4}
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

Data formats, from memory, to be confirmed by this story against the files: tokenizer tests are
JSON (`tests` with `description`, `input`, `output`, `errors`, `initialStates`, `lastStartTag`,
`doubleEscaped`); tree-construction and encoding tests are `.dat` files of `#data`, `#errors`,
`#document`, `#document-fragment`, `#script-on`, `#script-off`, `#encoding` sections, with the
tree written as `| ` lines.

## Change

- `crates/html-conformance/data/html5lib-tests/`: the data at the commit the note pins, only the
  directories the note keeps, with the upstream `LICENSE` and a `SOURCE` file (address, commit,
  date). Fetched into an empty directory under the scratch directory first, then copied.
- `crates/html/` (package `surf-html`, `#![forbid(unsafe_code)]`, no dependency): the public
  interface the harness calls, each entry point returning an empty result for now, and one empty
  module file per later story so that stories in parallel write different files: `encoding.rs`,
  `input.rs`, `tokenizer.rs`, `charref.rs`, `tree.rs` (the node arena: document, doctype,
  element with namespace, local name and attributes, text, comment, template contents),
  `tree_builder.rs`, `limits.rs`. Proposed entry points, names the story may change: prescan
  and decode of bytes; tokenize a string from a given initial state with a last start tag;
  parse a document with a scripting flag; parse a fragment for a context element.
- `crates/html-conformance/` (package `surf-html-conformance`, `publish = false`, standard
  library and `surf-html` only): a JSON reader (with the double-escaped strings of the
  tokenizer data), a `.dat` reader, a reader of the note's `html-inventory` block, the tag of
  each case by the rule of the note, the tree written in the `| ` format of the data (the only
  serialization the epic allows), the runner, and a binary that prints one line per data file,
  one line per tag in the form `tag <name> <pass>/<run>/<exist>`, and a total line
  `html5lib: <exist> cases, <run> run, <pass> passed`. Cases tagged `not-run` are counted as
  existing and not run, with the reason.
- Tests in `crates/html-conformance/tests/`: `every_data_file_is_read_without_a_reader_error`,
  `the_case_count_per_file_equals_the_inventory`, `the_case_count_per_tag_equals_the_inventory`
  (the tag the harness computes for each case, counted per tag, against the counts of the
  `html-inventory` block), `every_case_has_exactly_one_tag`,
  `the_tree_format_round_trips_the_expected_trees` (every `#document` block read and written back
  byte for byte), `json_reader_rejects_malformed_input`.

Writes: `crates/html/` (new), `crates/html-conformance/` (new). `Cargo.lock` gains two
repository packages; the coordinator regenerates it.

## Acceptance

`cargo test -p surf-html-conformance` passes with the six tests named in Change, among them
`the_case_count_per_tag_equals_the_inventory`; `cargo run -p surf-html-conformance` exits 0 and
prints one `tag` line per tag of the inventory and a total line whose case count equals the
inventory total of `docs/design/html-parser.md`; and `cargo tree -p surf-html-conformance` lists
only `surf-html-conformance` and `surf-html`.

## Out of scope

Any parsing (later stories), the gate (story `html-conformance-gate`). The pass count is whatever
an empty parser passes, recorded and not raised here. No `AGENTS.md` or `README.md` change.

## Scope

Derived 2026-11-21 by `story-scoper`.

- **Primary surface:** `crates/html-conformance/` — cited, the story's Change and `Writes:` line (`.engineering/planning/story/html-conformance-harness.md:64`) name it as a new crate. The root `Cargo.toml` takes it through the `crates/*` glob (`Cargo.toml:3`, `AGENTS.md:93-96`). `crates/` today holds only `fetch`, `ports` and `surf`.
- **Files written:** `crates/html-conformance/data/html5lib-tests/` (the vendored data, the upstream `LICENSE` and `SOURCE`) — cited, Change bullet 1. Which directories it keeps (proposed `tokenizer/`, `tree-construction/`, `encoding/`) is settled by `html-decisions` item 3, so the directory list is inferred.
- **Files written:** `crates/html-conformance/Cargo.toml` (package `surf-html-conformance`, `publish = false`, one dependency `surf-html` by path) — cited, Change bullet 3.
- **Files written:** `crates/html-conformance/src/` (JSON reader, `.dat` reader, inventory reader, tag rule, `| ` tree writer, runner, binary) — cited as a set, Change bullet 3. The module file names and `src/main.rs` are inferred. `cargo run -p surf-html-conformance` needs one binary in the package.
- **Files written:** `crates/html-conformance/tests/` with the six tests named in Change bullet 4 — cited. The file names inside `tests/` are inferred.
- **Files written:** `crates/html/Cargo.toml` (package `surf-html`, `#![forbid(unsafe_code)]`, no dependency) — cited, Change bullet 2.
- **Files written:** `crates/html/src/lib.rs` — cited, as the public interface the harness calls (Change bullet 2). That the entry points live in `lib.rs` is inferred.
- **Files written:** `crates/html/src/{encoding,input,tokenizer,charref,tree,tree_builder,limits}.rs` — cited, the seven empty module files named in Change bullet 2. `tree.rs` also holds the node arena types.
- **Files read, not written:** `docs/design/html-parser.md`, the `html-inventory` and `html-sections` blocks — cited, Context and Change bullet 3. The path is inferred from `html-decisions`' Scope (`story/html-decisions.md`, `scope: docs/design/html-parser.md`). The inventory reader reads this file at run time and in the tests, and the crate's tests fail without it.
- **Files read, not written:** `AGENTS.md` "Goals" and "Layout" (no third-party crates in the root lock; a crate joins by creating its directory) — cited.
- **Handed to the coordinator:** root `Cargo.lock` gains two packages, `surf-html` and `surf-html-conformance`. It is regenerated with `cargo generate-lockfile --offline`, because `task build` and `task test` use `--locked` (`AGENTS.md:94-96`, `AGENTS.md:237`) — cited.
- **Handed to the coordinator:** `AGENTS.md`, `README.md` and the root `Taskfile.yml`: nothing — cited, "Out of scope": "No `AGENTS.md` or `README.md` change". The story adds no task. The `html-conformance` task belongs to `html-conformance-gate` (`taskfiles/html-conformance.yml`). Inferred: "Layout" will want a line for the two crates, but the story does not ask for it.
- **Symbols:** `surf-html` public entry points: prescan and decode of bytes; tokenize a string from an initial state with a last start tag; parse a document with a scripting flag; parse a fragment for a context element. The story marks all of them as "names the story may change", and each returns an empty result — cited, with the exact names and types inferred. `surf-html` also gets the arena types in `tree.rs` (document, doctype, element with namespace, local name and attributes, text, comment, template contents) — cited. The `surf-html-conformance` library items and the binary are not public API for anyone (nothing depends on the crate), so the exact items are inferred.
- **Symbols:** the output format the binary prints (one line per data file, `tag <name> <pass>/<run>/<exist>`, and `html5lib: <exist> cases, <run> run, <pass> passed`) — cited. Later acceptances and the gate read these lines.
- **Would collide with:** `crates/html/src/*.rs` and `crates/html/Cargo.toml`, written by later parser stories — cited from their `Writes:` lines: `html-tokenizer-markup` (`input.rs`, `tokenizer.rs`), `html-tokenizer-text` (`tokenizer.rs`), `html-character-references` (`charref.rs`, `tokenizer.rs`), `html-input-stream` (`encoding.rs`), `html-tree-body` (`tree.rs`, `tree_builder.rs`, `lib.rs`), `html-tree-formatting`, `html-tree-tables`, `html-tree-template` (`tree.rs`), `html-tree-foreign` (`lib.rs`), `html-tree-complete`, and `html-parser-limits` (`limits.rs`, `lib.rs`, `tree.rs`).
- **Would collide with:** `crates/html-conformance/` (`src/`, `expected/`, `tests/`, `data/`), written by `html-conformance-gate`, which is `depends_on` this story, and by `html-character-references` (`data/whatwg/`, `src/`, `expected/tokenizer.txt`).
- **Would collide with:** all of these list this story in their `depends_on` chain (the gate on the harness, the rest on the gate), so the collisions are ordered by edges and not concurrent — cited. Nothing outside the HTML epic writes either crate.
- **Would collide with:** `html-decisions` does not write them, but its `html-inventory` block is the data this story reads — cited.
- **Not yet existing when it starts:** `docs/design/html-parser.md`, with the `html-inventory` and `html-sections` blocks, the case tags, the pinned commit and the kept directories — delivered by `story:html-decisions` (cited, `depends_on`). Network access to fetch html5lib-tests, which no story delivers (inferred from `html-decisions`' own note on this).
- **Confidence:** high for the surface, because the story names both crates and every top-level directory. Medium inside the crates: the file names under `src/` and `tests/` of `surf-html-conformance`, the exact entry-point names, and the vendored directory list are not fixed by the story. I did not check the exact shape of `docs/design/html-parser.md` or the actual html5lib data, because neither exists on this branch yet.

