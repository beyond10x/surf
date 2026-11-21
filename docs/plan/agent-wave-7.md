# Agent interface plan, wave 7 of 22: redirects followed, character references, and the view's first page

One wave of the [wave plan](agent-waves.md). Three stories, each in its own worktree, each
implemented by a sub-agent on Opus; three units beside them. The coordinating session wrote no
code. Every commit is the organization bot's.

After this wave a `go` follows up to 20 redirects inside one fetch and the session's page line
holds the address the fetch ended at. The tokenizer reads named and numeric character
references and passes every tokenizer case of the vendored test data that needs no script
engine. The page `surfd` serves has a playhead over a session's actions and is tested in a
browser, as a step of the gate. No parser runs on a real page yet: a real page's document
stays `Parsing` until `html-document-port`.

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `html-character-references` | `crates/html/src/tokenizer/machine.rs`: the eight character reference states of the HTML Standard (13.2.5.77 to 13.2.5.84); `crates/html/src/charref.rs` and the generated `charref_table.rs`; `entities.json` of the standard vendored under `crates/html-conformance/data/whatwg/`; the binary `entities-table` | `tag charref 4739/4739/4739` (was 2060); `--list-failing charref` 2679 before, 0 after; 15 unit tests in `surf-html` (12), 27 tests in the harness crate; 7 planted defects, each red | `909c46a` |
| `agent-fetch-redirects` | `crates/fetch/src/lib.rs`: 301, 302, 303, 307 and 308 with a `Location` are followed for `GET`, up to 20; `Response.url`; `Failure::TooManyRedirects`; `surf-fetch` depends on `surf-url`; `surf::Answer::Response` carries the address the fetch ended at | the story's 23 cases: 20 in `crates/fetch/tests/redirects.rs`, 2 in `crates/surf-cli/tests/redirects.rs`, 1 in `crates/session/tests/redirect.rs`; 9 planted defects, each red | `38b08d2` |
| `view-first-page` | the nine files of `crates/view/web/`: the playhead, Latest, the marks by domain, the gap for dropped entries, the selection in the address's fragment; `crates/view-fixture` (package `surf-view-fixture`, test tooling); `view-tests/` with 18 Playwright cases; `taskfiles/view-test.yml` | `18 passed`; 7 of the 18 cases fail on the page of wave 6; 10 planted defects, each red | `cc1dbd7` |
| the hand-offs (after the three merges, alone) | root `Taskfile.yml`: the include and the step `view-test` in `check`; `.github/workflows/check.yml`: Node 22 and a step that installs the page tests' tooling | `task check --dry` lists `view-test` after `view-stream-check`; `18 passed` through the include | `27e1855` |
| five findings in the page and its fixture (after the adversary's pass, alone) | `crates/view/web/state.js`, `view.js`, `stream.js`, `dom.js`, `timeline.js`, `sessions.js`; `crates/view-fixture/src/main.rs` | the adversary's 3 tests, red before; `24 passed` (18 and 6); 1 test in `crates/view-fixture/tests/adversary_commands.rs`; 6 planted defects, each red | `61a44cf` |
| three findings in the redirects (after the adversary's pass, alone) | `crates/fetch/src/lib.rs`: what a fetch holds while a body is read (`read_rest`, `Framing`, `Wire::release`), the section "Memory" of the crate documentation, the resolving of a `Location` charged against the read timeout; the documentation of `surf-session` and case 24 in `crates/session/tests/redirect.rs` | the adversary's test, red before; 2 cases in `crates/fetch/tests/adversary_redirect_memory.rs`; the suites of `surf-fetch` five times in a row; 1 planted defect, red | `a8d7b88` |

Each unit's claim was checked by the coordinator before its commit: the unit's packages were
tested again with warnings denied; for the character references the conformance task and
`--list-failing`; for the page the task `view-test`, `lock-check`, `view-stream-check` and a
count of the fixture's directories left under the runtime directory. The redirects and the
page were then each given to an adversary on Opus, which attacks a green unit and writes
failing tests; its tests are kept in the tree.

## The stories were measured before the wave

All three stories were read against `main` by a reviewer first, as in waves 3 to 6. None could
be met as written: 14 places in `agent-fetch-redirects`, 9 in `html-character-references`, 14
in `view-first-page`. The bodies were revised and each story got a typed scope, so that
`aep plan artifact waves` puts the three in one wave with no file in two of them (`87e0dbf`).

Decided before the wave:

- **Every hop is followed inside `surf_fetch::fetch`.** A redirect adds no event and no entry;
  the response says where it ended in a member `url`. An address longer than 65,536 bytes
  after a redirect fails as `MalformedResponse`.
- **Three rules of the Fetch Standard in the story were written from the reviewer's memory**
  (a `Location` that is not UTF-8, two `Location` fields, the fragment carried over). The unit
  confirmed each in the published text before it implemented it.
- **The table of names is generated from a vendored `entities.json`** (2231 names) by a
  binary of the conformance crate and checked by a test, as item 3 of
  `docs/design/html-parser.md` decided. A numeric reference saturates in place of
  overflowing; the lookahead for a name is bounded by the longest name of the table.
- **`view-first-page` stayed one unit**: the page, the fixture crate and the cases depend on
  each other. `crates/surfd` is out of its scope. The cases run in Chromium through
  Playwright against a fixture that runs `surfd` in process.
- **The root `Cargo.lock` is the coordinator's.** Two units change it; what cargo writes to it
  in a unit's tree is committed with the unit, and `lock-check` runs on the wave branch.
- **One draft from the readings**: `story:view-surfd-start-cases`, for three things
  `view-surfd-serves` decided and no test holds.

## What went wrong, and what was decided

- **`context.setOffline(true)` does not end an open stream.** The story rested cases 15
  (reconnect) and 16 (`reset`) on it and told the unit to measure first. The unit measured and
  stopped: in the headless shell offline fails a new request to 127.0.0.1, and a `fetch`
  stream already open goes on. Decided: the fixture gets a pipe between the browser and the
  view, with the commands `cut` and `mend`; only cases 15 and 16 load the page through it
  (`ebcc069`).
- **The fixture's `kill` searched every process of the machine.** The first version found a
  `surf-session` by its arguments and killed it. It now acts only for a session the fixture
  opened and only on a process whose environment holds the fixture's own `XDG_RUNTIME_DIR`; a
  test starts a foreign process with the same id and finds it alive. Changed before the
  unit's commit.
- **The story named nine character reference states, 13.2.5.72 to 13.2.5.80.** The standard
  as fetched on 2026-11-21 has eight, 77 to 84, and no decimal start state. The unit followed
  the standard; the story's text is corrected in the store with this wave.
- **A `Location` that is valid UTF-8 and holds a space or a character outside ASCII is
  followed.** The Fetch Standard's ABNF for the field would fail it. Decided: kept as the
  story has it; `surf-url` writes the address percent-encoded. No case holds the input.
- **One fetch held more than its documentation said.** "At most the header limit plus the
  body limit plus a few KiB" was false after a redirect to a long address: the adversary's
  test read 130,394 bytes more, with 16 KiB for "a few". The parsed address, its fragment and
  the request target are now dropped before the body is read. That alone was not enough: the
  parsed head and the connection's buffer were also held, so the head is dropped and the
  buffer cut back to one read before the first body byte. The crate documentation states the
  bound while a body is read as a sum of five named terms, 16,917,505 bytes at the defaults,
  and a test compares with that sum. Resolving a `Location` at the header limit holds up to
  30 times the header limit, or 54 times after a redirect to a URL of 64 KiB (measured:
  29.003 and 53.003); the documentation says so and a second case holds it.
- **The address of a loaded page can be missing from the status record, and a server can
  cause it.** A redirect can lead to an address whose quoted form does not fit the record of
  64 KiB; before the wave only a request that never crossed the wire reached that. Decided:
  the rule stays (the record without the address, `surfd` lists the session with `address:
  null`, the page line of the answer holds it). Case 24 pins it and the documentation of
  `surf-session` says it.
- **The coordinator's brief said a quoted address is up to twice its length.** It is up to
  four times: the quoting writes a backslash as `\x5c`. The unit followed the code. The
  arithmetic of the session's bounds is written out with that factor: seven addresses in an
  answer are at most 1,835,022 bytes of 4,194,304, fifteen in the entries of one request
  3,932,190 of 8,388,608.
- **The time of resolving a `Location` counted against neither timeout.** It counts against
  the read timeout now.
- **The memory test needs `unsafe`**: a counting allocator is a `GlobalAlloc`. Decided: it
  stays, in that one test file, with a comment; no file under a `src/` holds any.
- **Entries were lost while a session's detail loaded.** `addEntry` dropped a stream entry of
  a session that was not loaded yet, also after a `reset`. The adversary showed it with a
  case. The page now holds such entries for a session whose detail has been asked for and
  merges them when it answers.
- **With Latest off, the playhead followed every new action once its own action was dropped
  by the log's bound.** Decided: the playhead moves once to the oldest action that is held,
  Latest stays off, and the marker counts the newer actions.
- **A command line that is not UTF-8 ended the fixture's command loop.** It is answered
  `{"error":"command"}` now.
- **A refused token was sent again every 5 s for ever.** When `surfd` has exited and another
  program listens on its port, the page so sent the token to that program. Decided: after a
  401 or 403 the page stops the stream, asks nothing again and reads `offline`.
- **"Every value through `dom.js` and `textContent`" was false as written**: a mark's `title`
  attribute holds an event's name and field values. `dom.js` is now the only module that sets
  an attribute, from a list of nine names, and throws for any other. No markup or script sink
  was found.
- **The view unit read `$XDG_RUNTIME_DIR/surf` once**, which its brief forbade; it wrote
  nothing and reported it. A planted run left one directory of the fixture, which the unit
  removed; the drop of the test's fixture now closes standard input and waits.
- **The coordinator's brief to the page's adversary said the token is kept in no storage.**
  Item 6 of `docs/design/live-view.md` keeps it in `sessionStorage`. The adversary reported
  the difference; nothing was changed.

## The gate on the wave branch

`slot task check` in the runner's form (`CI=true GITHUB_ACTIONS=true CARGO_INCREMENTAL=0
RUSTC_WRAPPER=""`), exit 0, on `wave/a7-redirects-charrefs-view` after the six units, at
`916a3c0`. It ran first after the hand-offs, at `27e1855` (116 suites, 482 passed, `18 passed`
in the page tests), and again after this page; the store's evidence for the stories names that
last run.

```
surf v1 — 9 file(s), 12 scenario(s), valid
93 scenario(s) (12 authored), 28 refusal(s), written to .out/suite.json
  93 scenarios: 93 passed, 0 failed, 0 error, 0 unsupported
the interpreter and the Rust system ran 93 scenario(s) each, of suite sha256:4cd1257a06a838a1bb2779b0309c347b1b6262703ff814da340988c32ecb7a3a
generated/rust matches spec/ (30 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
Cargo.lock: 22 package(s), none from outside this repository
118 test suites: 486 passed, 0 failed, 0 ignored
view-stream-check: 19 event name(s), each a message of the AsyncAPI projection
  24 passed (14.6s)
html-conformance: 9107 cases, 9095 run, 7121 passed
```

Before the wave: 21 packages, 109 suites with 453 passed, and `html-conformance: 9107 cases,
9095 run, 4442 passed`. The specification did not change: 93 scenarios, 28 refusals and the
suite's digest are those of wave 6. `check` has one step more, `view-test`, after
`view-stream-check`; it needs Node and Chromium's headless shell, which the forge's workflow
installs before `task check`. Of its 14.6 s, 12.9 s are the case that waits to see that a
refused token is not sent again.

## Findings, and where each goes

| finding | from | goes to |
|---|---|---|
| the page keeps every entry of every session it loaded, and rebuilds the whole timeline on each entry | the page's adversary | backlog: `story:view-page-memory`, a draft |
| a failed first load stays `offline` and is not retried; the refused token's text is also shown for a 403, which the view answers for a refused `Host` as well | the page's adversary, the unit of the five findings | backlog: `story:view-first-load-retry`, a draft |
| `crates/view/tests/web.rs` does not assert that `setAttribute` appears only in `dom.js` | the unit of the five findings | accepted: the page case "dom.js sets only the attributes of its list" holds it |
| the named character reference state reads the next input character for the attribute rule without reporting that character's input stream error first; no case of the test data holds it | `html-character-references` | `story:html-tree-body` (wave 8), the first reader of the tokenizer's errors in order |
| the sections `tokenization` and `named-character-references` stay `incomplete`: each has tests that need a script engine | `html-character-references` | accepted, as the story measured |
| a `Location` with a space or a character outside ASCII is followed, where the standard's ABNF fails it | `agent-fetch-redirects` | accepted, stated in the unit's commit |
| while a head is read, each parsed field takes 48 bytes besides its bytes: a header section of fields of three bytes held 1,611,203 bytes (24.59 times the header limit); older than the redirects; the documentation states no bound for that moment | the unit of the three findings | backlog: `story:fetch-head-memory`, a draft |
| an address asked through the wire at the request limit comes to about 34 bytes over the record's limit (calculated, not tested) | the unit of the three findings | accepted: the record is then sent without the address, as documented |
| the forge's step `playwright install --with-deps --only-shell chromium` fetches the headless shell and FFmpeg from `cdn.playwright.dev` and installs libraries with `apt-get` through `sudo`; `actionlint` was not on the machine | the hand-off unit | accepted: the first run of `check` on the forge is the check of the new steps |
| the machine the gate ran on builds the dev profile with line tables only (its cargo configuration); the forge's runner builds with full debug information, so the runner's form here is not the forge's build in that respect | the coordinator | accepted: the forge's run of `check` on `main` is the check; no test of this repository is known to depend on it |
| `domjs.test` 46 (from wave 6) | `html-character-references` | closed: `domjs.test 56/56/56` |
| no test starts `surfd` where listening or writing `view-url` fails; the mode of `view-url` under another umask; an entry `surf_answer` refuses (from wave 6) | the readings before the wave | backlog: `story:view-surfd-start-cases`, a draft |
| `crates/view/examples/prototype.rs` showed `DocumentParsed` after a `go` (from wave 6) | `view-first-page` | closed: the example publishes the events of a real `open` and a real `go` |

## On the forge, after the merge

The first run of the workflow `check` on the merged wave (`ff8d8fa`, runner image
`ubuntu-26.04`) failed. The new steps that install Node and the headless shell passed. The
step `task check` failed in `test`, at one test of `surf-view-fixture`
(`tests/fixture.rs`, "the foreign process did not start"), after 89 suites had passed; the
steps after `test`, the page tests among them, did not run.

The test started `/usr/bin/tail` through a symbolic link named `surf-session`, as a process
that is not the fixture's and that `kill` must leave alive. The process never showed a command
line on the runner. The reading of the cause, inferred from the log and not verified on the
image: `tail` there is one name of a program that serves many utilities and chooses by the
name it was started under, so started as `surf-session` it exits at once. On the machine the
gate ran on, `tail` is a program of its own (GNU coreutils 9.12), which is why the gate in the
runner's form passed there.

Fixed in a unit of its own on `main`: the package has a second binary, `surf-view-foreign`,
which waits until it is killed whatever its name, and the test starts that. The planted
defect of the test (a `kill` that acts on any `surf-session` with the id among its arguments)
is red with it. No other test of the repository starts a program of the system under another
name. The gate's numbers do not change: 118 suites, 486 passed. The run of `check` on the
commit that holds this section is the first that reaches the page tests on a runner.

What the gate in the runner's form does not hold: the programs of the runner's image. The
forge's run of `check` on `main` holds that, after the merge.

## After this wave

Wave 8 of [the wave plan](agent-waves.md): `agent-request-body-spec`, `html-tree-body` and
`view-detail-panel`, each read against `main` first.
