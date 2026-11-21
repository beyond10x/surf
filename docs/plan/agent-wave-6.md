# Agent interface plan, wave 6 of 22: the document's elements, the text states, and the view served

One wave of the [wave plan](agent-waves.md). Three stories, each in its own worktree, each
implemented by a sub-agent on Opus; two units beside them. The coordinating session wrote no
code. Every commit is the organization bot's.

After this wave the specification knows the elements of a document an agent can act on, and a
document is parsed in steps (`ParseDocument`, `AddElement`, `FinishParsing`). The tokenizer
passes every case of the vendored test data for the text states (RCDATA, RAWTEXT, script data,
PLAINTEXT). `surfd` serves the live view's stream over HTTP on 127.0.0.1, guarded by a token it
draws at start. No parser runs on a real page yet: a real page's document stays `Parsing`
until `html-document-port`, and no command of `surf-agent` prints anything it did not print
before.

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `html-tokenizer-text` | `crates/html/src/tokenizer/machine.rs`: the text states of the HTML Standard (13.2.5.2 to 13.2.5.5 and 13.2.5.9 to 13.2.5.31); `Tokenizer::new` takes the last start tag | 5 unit tests in place of 1; `tag text 328/328/328` (was 57), `tag charref 2060/4739/4739` (was 2053), `tag markup 1979/1979/1979`; 4 planted defects, 3 caught by the test data, 1 by a unit test written for it | `d8e7df1` |
| `html-document-spec` | `spec/domains/document.yaml`: the entity `Element`, the commands `AddElement` and `FinishParsing`, the events `ParsingStarted` and `ElementAdded`, the view `Elements`; three authored scenarios; `generated/rust`; the ports, `Browser::held`, the session's entries and the conformance target brought along | the whole gate with `REFUSALS_MAX=28`, exit 0: 93 scenarios (12 authored), 28 refusals, 93 of 93 on both runners; 5 planted defects, each red | `bb18024` |
| `view-surfd-serves` | `crates/view/src/model.rs` and `json.rs`: the JSON model and its writer; `crates/surfd/src/view.rs`: the source over the daemon's sessions and logs; the start of the server in `Daemon`'s `begin`; the file `view-url`; `taskfiles/view-stream-check.yml` | the story's cases 1 to 27: 10 tests in `crates/view/tests/json.rs`, 9 in `crates/surfd/tests/view.rs`, 5 with the real `surf-session` in `crates/session/tests/view.rs`; each new suite five times in a row; 7 planted defects, each red | `5600535` |
| the hand-offs (after the three merges, alone) | root `Taskfile.yml`: `REFUSALS_MAX: 28`, the include and the step `view-stream-check`; `docs/view-stream.md`: the two new event names and the example of a `go` | `view-stream-check: 19 event name(s)`; the packages `surf-view`, `surfd` and `surf-session` tested again; 3 planted defects, each red | `e555ef6` |
| two defects of the stream (after the adversary's pass, alone) | `crates/view/src/sse.rs`, `http.rs`, `crates/surfd/src/view.rs`: `Source::read_newest`; a frame over the queue's bound of bytes waits and is written alone | the adversary's 3 tests, red before; 6 tests in `crates/view/tests/stream_bounds.rs`; 6 planted defects, 5 red | `a846da5` |

Each unit's claim was checked by the coordinator before its commit: the unit's packages were
tested again with warnings denied; for the tokenizer the conformance task and `--list-failing`
were run again; for the specification the tasks `validate`, `suite`, `conform`, `conform-rust`,
`conform-totals` and `generated-check`. The view unit was then given to an adversary on Opus,
which attacks a green unit and writes failing tests; its three tests are kept in the tree.

## The stories were measured before the wave

All three stories were read against `main` by a reviewer first, as in waves 3 to 5. None could
be met as written: 15 places in `html-document-spec`, 12 in `html-tokenizer-text`, 16 in
`view-surfd-serves`. The bodies were revised and each story got a typed scope, so that
`aep plan artifact waves` puts the three in one wave with no file in two of them (`8eb61fa`).

Decided before the wave:

- **The model of the document was drafted and measured before the unit started.** The reviser
  wrote it in a scratch copy on ess 0.96.0: 93 scenarios, 28 refusals, 167 capabilities with
  the 4 obligations and 4 refused ones of `main`. The unit started from the draft and changed
  no name and no guard.
- **`AddElement` refuses an unknown document and one already parsed.** The price is one
  refusal: `ESS-SYNTH-014` for `AddElement/added`, because the guards leave synthesis one
  element where the ordered view `Elements` needs two. An authored scenario covers the
  outcome, and `REFUSALS_MAX` went from 27 to 28. Without the two guards the suite has 91
  scenarios and 27 refusals.
- **`ElementAdded` carries no label, target or value**, so no text of a page reaches the
  session's log by it. `title` is the empty string for a page without a title element.
- **Neither the signature of `tokenize` nor a file of the conformance runner changes.** The
  runner already hands the tokenizer the initial state and the last start tag. `&` in RCDATA
  goes to the existing stub of the character reference state.
- **The view's server starts in `Daemon`'s `begin`, not in `main.rs`**, and the drop of
  `Daemon` stops it and removes `view-url` (mode 0600) before the lock is released.
- **A reader of the view is not activity for the idle exit.** A replay of more than 256
  numbers is answered with `reset`. An entry kept for a session whose log was dropped whole is
  dropped at once. A pending late answer adds no member to any JSON.
- **The "Event names" block of `docs/view-stream.md`, the include of the new task and
  `REFUSALS_MAX` are the coordinator's**, written after the merges: two units would have
  written the same lines.

## What went wrong, and what was decided

- **A stream of the view could end with no answer.** `sse::run` read the newest number and
  then asked for a reader from it. `surfd`'s source refuses a number more than 256 below the
  newest; when more than 256 entries were kept between the two calls, the connection closed
  with no byte. The unit had reported the two calls; the adversary showed it with two tests.
  `Source` now has `read_newest`, and `surfd` gives the number and the reader in one hold of
  its lock.
- **An event over 1 MiB closed every stream that reached it, every time.** The pump closed a
  stream when the queue would pass its bound of bytes, also for a frame that alone is over the
  bound, so that entry and every later one were never delivered and no `reset` was sent. One
  session that goes to long addresses makes such an entry: the answer's entry carries the
  page's address and path, and the writer writes `&` as six bytes. The adversary showed it
  with the real `surf-session` and three `go`s. Decided: such a frame waits until the queue is
  empty and is then written alone; no shape is added to the stream and no text is cut. The
  contract says so.
- **The example of a `go` in the contract became false when two units met.** `view-surfd-serves`
  kept the example of `docs/view-stream.md`, whose entry 16 was `DocumentParsed`;
  `html-document-spec` made a real `go` publish `ParsingStarted` there. Neither unit could see
  it. The hand-off unit measured a real `go` with the built `surfd` and `surf-agent` (the
  action, nine events, the answer), rewrote the one entry and the pinned length of the test
  that reads the example (5255 to 5237 bytes).
- **A test outside the unit's list failed with the model.** `Browser::held` now counts the
  body of `ParsingStarted`, so a committed page holds its body five times;
  `crates/surf/tests/forget_delivered.rs` asserted four. The unit stopped and asked; the
  coordinator admitted that file and a comment of `crates/session/tests/memory.rs`, and both
  are in the story's scope in the store.
- **One planted defect of the tokenizer was not caught by the test data.** No html5lib case
  holds `<!-->` or `<!--->` in script data, the inputs for which the script data escape start
  dash state matters. The unit wrote a unit test for it.
- **The coordinator's brief to the adversary named a check of `Origin`.** Neither the contract
  nor `crates/view/src/http.rs` has one. The token, the absence of CORS headers and
  `Cross-Origin-Resource-Policy` cover a cross-site read; nothing was changed.
- **The coordinator's brief for the two defects asked for 19 event names in a worktree that
  had 17.** The unit's tree was cut before the hand-off commit. The unit reported the
  difference and changed nothing; the wave branch has 19.

## The gate on the wave branch

`slot task check` in the runner's form (`CI=true GITHUB_ACTIONS=true CARGO_INCREMENTAL=0
RUSTC_WRAPPER=""`), exit 0, on `wave/a6-document-text-view` after the five units, at
`a7641d0`. It ran first after the hand-offs, at `e555ef6` (106 suites, 443 passed), and again
after this page; the store's evidence for the stories names that last run.

```
surf v1 — 9 file(s), 12 scenario(s), valid
93 scenario(s) (12 authored), 28 refusal(s), written to .out/suite.json
  93 scenarios: 93 passed, 0 failed, 0 error, 0 unsupported
the interpreter and the Rust system ran 93 scenario(s) each, of suite sha256:4cd1257a06a838a1bb2779b0309c347b1b6262703ff814da340988c32ecb7a3a
generated/rust matches spec/ (30 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
Cargo.lock: 21 package(s), none from outside this repository
109 test suites: 453 passed, 0 failed, 0 ignored
view-stream-check: 19 event name(s), each a message of the AsyncAPI projection
html-conformance: 9107 cases, 9095 run, 4442 passed
```

Before the wave: 83 scenarios (9 authored) and 27 refusals, 102 suites with 407 passed, and
`html-conformance: 9107 cases, 9095 run, 4164 passed`. `check` has one step more,
`view-stream-check`, after `test`.

## Findings, and where each goes

| finding | from | goes to |
|---|---|---|
| every read of a stream (once per entry, and every 100 ms while idle) copies the session list under the daemon's lock, with addresses and titles of up to 64 KiB for up to 256 ended sessions | the adversary | backlog: `story:view-stream-list-copy`, a draft |
| the session list is ordered by the order of the spawns, and the time of opening is read before the lock is taken, so two `open`s at the same moment can be listed against "oldest `opened_at` first" | the adversary | backlog: `story:view-list-order`, a draft |
| a large frame that skips its own wait, while frames behind a large one still wait, keeps the order and breaks only the bound of bytes; no test tells it from the pump's copy of the frame without a timer | the unit of the two defects | accepted, stated in the unit's commit |
| the tests of the wait use an entry of three times the largest TCP send and receive buffers together, read from `/proc/sys/net/ipv4` (128 MiB when that cannot be read) | the unit of the two defects | accepted; it is what keeps a writer in its write without a sleep |
| not attacked: the mode of `view-url` under another umask, a client that sends its request slowly, 16 connections held open against `surfd`'s own work, a late answer after 600 s, a session killed before its answer | the adversary | backlog, with `story:view-first-page` (wave 7), the first reader of the stream |
| no test starts `surfd` where listening or writing `view-url` fails; `Action.sequence`, `Action.command` and `Event.name` are written `null` for an unreadable entry and no case reaches that | `view-surfd-serves` | backlog, with the same story |
| `task view-stream-check` fails for a name of the document that is no event; a new event whose name nobody adds to the document passes | the coordinator | accepted: the document says "a story that maps a new event adds its name here", and the hand-off unit did |
| the machine replaces the last start tag with each start tag it emits; no run of `tokenize` re-enters a text state after a start tag, so no test observes it | `html-tokenizer-text` | `story:html-tree-body`, the first that switches the state between tokens |
| `domjs.test` 46 still fails; the story does not name it among the text cases | `html-tokenizer-text` | `story:html-character-references` (wave 7), which reads the failing list first |
| a real page's document stays `Parsing`: nothing calls `AddElement` or `FinishParsing` outside the conformance target | `html-document-spec` | `story:html-document-port` (wave 11) |
| `docs/design/live-view.md` records "12 channels" and "12 event name(s)" as the result of a trial; the model has 19 events | the hand-off unit | kept: a dated result |
| `crates/view/examples/prototype.rs` shows `DocumentParsed` after a `go` in the prototype's own data | the coordinator | `story:view-first-page`, which replaces the prototype's page |
| the test `drift` of `surf-agent` fails when `.out/cli/binding.json` is missing; `task cli` writes it, and `check` runs `cli` before `test` | `view-surfd-serves` | accepted, as before this wave |
| `story:view-stream-string-example` (from wave 3) is met: the example is corrected and case 9 of `crates/view/tests/json.rs` reads it from the document | `view-surfd-serves` | closed in the store with this wave |

## After this wave

Wave 7 of [the wave plan](agent-waves.md): `agent-fetch-redirects`, `html-character-references`
and `view-first-page`, each read against `main` first.
