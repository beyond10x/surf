---
format: aep.planning-md/3
id: story:html-tokenizer-text
kind: story
status: implemented
title: Tokenize RCDATA, RAWTEXT, script data and PLAINTEXT
summary: The text states and the appropriate end tag check; every tokenizer case tagged markup or text passes.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-tokenizer-markup
scope:
- confidence: cited
  path: crates/html-conformance/expected/tokenizer.txt
- confidence: cited
  path: crates/html/src/lib.rs
- confidence: cited
  path: crates/html/src/tokenizer.rs
- confidence: cited
  path: crates/html/src/tokenizer/
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-21T14:59:44Z", actor: "human:operator", revision: 11, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

The second tokenizer story. The tree builder switches the tokenizer into RCDATA (`title`,
`textarea`), RAWTEXT (`style`, `xmp`, `iframe`, `noembed`, `noframes`, and `noscript` when
scripting is on), script data (`script`) and PLAINTEXT (`plaintext`); an end tag ends them only
when it is an appropriate end tag for the last start tag. The tokenizer data sets these with
`initialStates` and `lastStartTag`. By rule 2 of `docs/design/html-parser.md` item 8
(`crates/html-conformance/src/tags.rs`, `tokenizer`) a case is tagged `text` when its initial
state is not the data state and its input holds no `&`; an input holding `&` is `charref`
whatever its state. So no `text` case needs a character reference.

The state names the data uses, as `crates/html-conformance/src/cases.rs` (`state`) reads them:
`Data state`, `PLAINTEXT state`, `RCDATA state`, `RAWTEXT state`, `Script data state`,
`CDATA section state`. A test object with several `initialStates` is one case per state, each
with the object's `lastStartTag` (absent: `None`).

Measured on the wave 5 tree (`d437dc6`) with `cargo run -p surf-html-conformance --
--list-failing text` in a scratch copy: of the 328 `text` cases, 53 start in `CDATA section
state` and pass already (story `html-tokenizer-markup`); 275 start in the four text states
(PLAINTEXT 49, RAWTEXT 69, RCDATA 69, script data 88; 51 of them carry `lastStartTag`), 4 of
those pass today (the empty input), and 271 fail: `test3.test` 188, `domjs.test` 42,
`contentModelFlags.test` 20, `test1.test` 13, `escapeFlag.test` 8. Every one of the 271 fails
because `tokenize` returns no token in these states (`crates/html/src/tokenizer.rs:113-121`).

The entry point already carries what the data gives: `tokenize(input: &[u32], state: State,
last_start_tag: Option<&str>) -> Tokenized` (`crates/html/src/tokenizer.rs:113`), called by
`crates/html-conformance/src/runner.rs:107` with the case's state and `lastStartTag`. The
signature and the token types stay as they are; no file of the runner changes.

Fourteen `charref` cases start in a text state or the CDATA section state. Seven of them pass
through this story alone, because their `&` begins no character reference in that state, or
reaches the stub of the character reference state at the end of the input:
`contentModelFlags.test` 1, 2 (PLAINTEXT) and 23 (RAWTEXT), and `test3.test` 56 (PLAINTEXT),
57 (RCDATA, `&` alone), 58 (RAWTEXT) and 59 (script data). The other seven need the character
reference states (story `html-character-references`): `contentModelFlags.test` 24,
`domjs.test` 29, 30 and 51, `escapeFlag.test` 7 fail today and keep failing, and
`domjs.test` 50 and `test3.test` 60 already pass.

## Change

- `crates/html/src/tokenizer/machine.rs` (further submodules under
  `crates/html/src/tokenizer/` if the unit wants them): the RCDATA, RAWTEXT, script data and
  PLAINTEXT states, the RCDATA, RAWTEXT and script data less-than sign, end tag open and end tag
  name states, the script data escape start, escape start dash, escaped, escaped dash, escaped
  dash dash, escaped less-than sign, escaped end tag open, escaped end tag name, double escape
  start, double escaped, double escaped dash, double escaped dash dash, double escaped less-than
  sign and double escape end states (13.2.5.2 to 13.2.5.5 and 13.2.5.9 to 13.2.5.31), with
  their parse error codes, in the standard's order and with its names, as the markup states are
  written.
- The appropriate end tag check: an end tag token whose tag name equals the last start tag's.
  `machine::Tokenizer::new` takes the last start tag (`tokenize`'s `last_start_tag`), and the
  machine sets it to the name of each start tag it emits, as 13.2.5 says ("the last start tag to
  have been emitted from this tokenizer"). With none, no end tag is appropriate.
- `&` in the RCDATA state sets the return state to RCDATA and switches to the character
  reference state, the stub of story `html-tokenizer-markup`
  (`crates/html/src/tokenizer/machine.rs:1186`), so that story `html-character-references`
  replaces only the stub.
- `crates/html/src/tokenizer.rs`: `tokenize` maps the four states to the machine's and passes
  `last_start_tag`; the signature and the token types unchanged. Its documentation (lines
  98-112) and the module header of `machine.rs` (lines 1-8) say what is written now. The unit
  test `a_text_state_is_not_written_yet` (`tokenizer.rs:236`), which turns red with this
  change, is replaced by unit tests of the text states: at least an appropriate and an
  inappropriate end tag in RCDATA, and a script data double escape.
- `crates/html/src/lib.rs`: the `tokenizer` item of the section "State" (lines 26-34) rewritten
  from the source: the states written, `last_start_tag` read, the character reference state
  still a stub. Nothing else of `lib.rs`.
- `crates/html-conformance/expected/tokenizer.txt`: every line raised to what the unit
  measures with `cargo run -p surf-html-conformance` after the change. Predicted from the
  measurement above (not measured, since it needs the change): `file
  html5lib-tests/tokenizer/contentModelFlags.test 23/24/24`, `domjs.test 52/56/56`,
  `escapeFlag.test 8/9/9`, `test1.test 59/69/69`, `test3.test 1802/1802/1802`, the other
  `file` lines unchanged, `tag markup 1979/1979/1979`, `tag text 328/328/328`, `tag charref
  2060/4739/4739`; the three `section` lines stay `incomplete`. A measured value that differs
  from the prediction is explained in the unit's report.

Writes: `crates/html/src/tokenizer.rs`, `crates/html/src/tokenizer/`,
`crates/html/src/lib.rs` (the `tokenizer` item of "State" only),
`crates/html-conformance/expected/tokenizer.txt`.

Not this unit's: `crates/html-conformance/src/` and `crates/html-conformance/tests/check.rs`
(the runner needs no change; the fixtures use `test1.test` in the data state only), and
`crates/html/src/input.rs`. `AGENTS.md`, `README.md` and `website/docs/` name the tokenizer's
state and the count `4164 passed`; the coordinator brings them up to date after the merge.

## Acceptance

- `crates/html-conformance/expected/tokenizer.txt` holds `tag markup 1979/1979/1979`, `tag
  text 328/328/328`, and a `tag charref` line with at least 2060 passed of 4739 run.
- `slot task html-conformance HTML_BASE=<this story's base commit>` exits 0 and prints no
  `raise:` line (the check prints one for every count above the expected file,
  `crates/html-conformance/src/check.rs:463`, and fails on one below it: exit 0 with no
  `raise:` line means every line of the expected files equals the measured count). Tokens and
  parse error codes both compared.
- `cargo run -p surf-html-conformance -- --list-failing text` prints nothing, and
  `cargo run -p surf-html-conformance -- --list-failing markup` prints nothing.
- `cargo run -p surf-html-conformance -- --list-failing charref` prints none of
  `html5lib-tests/tokenizer/contentModelFlags.test` 1, 2, 23 and
  `html5lib-tests/tokenizer/test3.test` 56, 57, 58, 59.
- `cargo test -p surf-html -p surf-html-conformance` passes, with the unit tests of the text
  states named in Change, and `grep -rn "no token" crates/html/src` prints nothing.

## Out of scope

Character references inside RCDATA and attribute values (story `html-character-references`):
the seven `charref` cases named in Context that need them, and every `charref` case in the
data state. The interface through which a tree builder switches the tokenizer's state and
sets the last start tag between tokens: `tokenize` runs to the end of the input from one
state, and the tree builder that needs more is story `html-tree-body`'s. Scripts: the script
data states are tokenized in full and need no engine; what they leave unsupported is the
insertion point that `document.write` writes at, which the tokenizer never has. The note lists
it as an open requirement of section `the-input-byte-stream` (`html-sections`, `open
the-input-byte-stream insertion-point`).
