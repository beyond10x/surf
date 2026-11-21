# Agent interface plan, wave 8 of 22: a request's method and body, the tree builder, and the view's detail panel

One wave of the [wave plan](agent-waves.md). Three stories, each in its own worktree, each
implemented by a sub-agent on Opus; two units of fixes beside them, after two adversaries'
passes. The coordinating session wrote no code. Every commit is the organization bot's.

After this wave the specification gives a request a method (`Get` or `Post`), an optional body
and its media type, and every transport takes the started request; a request that is not a
plain `Get` still ends as `UnsupportedScheme`, before any connection, until
`agent-fetch-post`. The parser builds a document from bytes with the insertion modes of the
HTML Standard outside tables, templates and foreign content: every tree case of the vendored
test data tagged `body` passes. The page `surfd` serves has a right column with the selected
action or event and the session's requests. No parser runs on a real page yet: a real page's
document stays `Parsing` until `html-document-port`, and no crate of the product depends on
`surf-html`.

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `view-detail-panel` | `crates/view/web/detail.js` (the tenth page file): the action at the playhead or the selected event with its fields, and the session's requests, one row per `request_id`; a mark holds a button; the selected event is in the address's fragment (`&event=<number>`) | `30 passed` (6 new cases in `view-tests/tests/detail.spec.mjs`, all six red on the page of wave 7); 8 planted defects, each red | `e7ca343` |
| `agent-request-body-spec` | `spec/`: the enum `surf.net.Method`, the inputs `method`, `body` and `body_type` on `Navigate` and `Fetch`, the fields on `NavigationStarted`, `RequestStarted`, `Request` and the view `Requests`; two authored scenarios; `generated/rust/`; `surf::Transport::fetch(&mut self, request: &RequestStarted)`; `surf::fetch_http`; `Browser::held` counts request bodies; `crates/session/src/entries.rs` writes the new fields, a body as its length | 95 scenarios (14 authored) on both runners, 28 refusals (the same 28 lines); 5 cases in `crates/surf/tests/request_method.rs`, `fetch_refuses_post`; the whole gate in the unit's tree; 7 planted defects, each red | `85fac64` |
| `html-tree-body` | `crates/html/src/tree_builder.rs` and `tree_builder/modes.rs`: thirteen insertion modes, the stack of open elements and its scopes, implied end tags, the list of active formatting elements and its reconstruction, the quirks decision, the `meta` rules with one re-parse; `crates/html/src/tree.rs`: the arena, `Tree::parent`, `Tree::descendants`; a tokenizer that gives one token at a time; `NOT_YET_PARSED` | `tag body 476/476/476` (was 0), `tag encoding 82/82/82` (was 75), `html-conformance: 9107 cases, 9095 run, 8362 passed` (was 7,121); 38 tests in `surf-html`; 8 planted defects, each red | `039104d` |
| three findings in the detail panel (after the adversary's pass, alone) | `crates/view/web/timeline.js`, `view.css`, `index.html`; the targets of three clicks in `first-page.spec.mjs` and `adversary-page.spec.mjs` | the adversary's 3 cases, red before; 5 more cases; `38 passed` (18, 6, 6 and 8 in `adversary-detail.spec.mjs`); 6 planted defects, each red | `3b1c4ed` |
| three findings in the tree builder (after the adversary's pass, alone) | `crates/html/src/tree_builder.rs`, `tree_builder/modes.rs`, one test in `crates/html/tests/tree_body.rs` | the conformance counts unchanged; 39 tests in `surf-html`; 1 planted defect, red | `d72b41d` |
| the example events (with this page) | `crates/view/examples/prototype.rs` and the example of `docs/view-stream.md`: `NavigationStarted` and `RequestStarted` with `method` `Get` and `body` and `body_type` without a value; the example answer is 5,325 bytes (5,237 before), and case 02 of `crates/view/tests/json.rs` holds the document's example against its own entries, which gained the three fields | the tests of `surf-view` (red with `method` left out of the document's example); `view-stream-check`; no test holds the example's fields against `spec/` | the commit that holds this page |

Each unit's claim was checked by the coordinator before its commit: the unit's packages were
tested again with warnings denied; for the tree builder the conformance task, `--list-failing`
for five tags, `cargo fmt --check` and `cargo doc`; for the page the task `view-test`,
`lock-check`, `view-stream-check` and a count of the fixture's directories left under the
runtime directory; for the specification the named cases, the build of the conformance crate
and `task open`. The page and the tree builder were then each given to an adversary on Opus,
which attacks a green unit and writes failing tests. `agent-request-body-spec` got no
adversary: until `agent-fetch-post` no caller outside the process can supply a method or a
body.

## The stories were measured before the wave

All three stories were read against `main` by a reviewer first, as in waves 3 to 7. None could
be met as written: 12 places in `agent-request-body-spec`, 13 in `html-tree-body`, 15 in
`view-detail-panel`. The bodies were revised and each story got a typed scope, so that
`aep plan artifact waves` puts the three in one wave with no file in two of them (`b701593`).

Decided before the wave:

- **The specification change was drafted and measured before the unit started**, in a scratch
  copy on ess 0.96.0. It adds no synthesized scenario and no refusal, so the story's "at least
  6 scenarios more" could not be met and was replaced by the measured numbers. A required
  `method` on `Navigate` makes validation refuse every authored step without it: nine steps in
  seven files got `method: Get`. Each optional input has an `example:`; without it one
  synthesized mapping errors on the interpreter.
- **A request's method and media type are stored; its body is not.** An authored step can
  claim only its own command's events, so the two new scenarios read the method from the view
  `Requests`. A row is kept for the browser's life, so a body in a row would never be
  released; it travels in events and invocations only. The `Navigation` row does not store
  the method; `agent-history-spec` adds it there.
- **The refusal of a request that is not a plain `Get` is in a public function**,
  `surf::fetch_http`, used by `surf-cli` and the session, so that a test can show no
  connection is made.
- **`html-tree-body` stayed one unit**, the largest of the plan so far, with an order of work
  in two parts and a way out after the first (the gate holds partial counts). The way out was
  not needed: 80 `body` cases failed after the first part, none at the end.
- **The tokenizer gives one token at a time** to the tree builder, which sets its state
  between two tokens; one tokenizer serves a whole parse, so the last start tag it keeps is
  the one the standard means (the finding of wave 6). The named character reference state
  reports the next character's input stream error first inside an attribute (the finding of
  wave 7), held by a unit test: the conformance harness compares the errors of no tree case.
- **A token whose rule a later story owns goes to "any other start tag" or "any other end
  tag"**, which this story writes. No input reaches a missing mode.
- **The selected event is in the fragment**, never in a query: the route table answers a
  query with 404. A mark holds a button; `dom.js` keeps its nine attribute names.

## What went wrong, and what was decided

- **The reading missed one file.** `crates/ports/tests/ports.rs` builds a `RequestData`,
  which gained two fields; the unit's whole gate stopped at `build`. The unit reported the
  file and the change and did not write outside its list. The file was added to the story's
  scope (`38f6828`) and the unit resumed; its gate then passed.
- **The story said a 303 continues as `Get` "after any method".** The unit read the Fetch
  Standard (as last updated on 2026-11-20): after any method other than `GET` or `HEAD`. The
  comment of `spec/domains/net.yaml` follows the standard.
- **A page case of the story could not run as written.** Its address had a space in the
  query, and a fetch of such an address ends `Unreachable` (see the findings). The unit
  measured and used an address without a space.
- **A click on a mark beside its short name did not select the event.** The adversary's case:
  the button covered only the name, and the rest of the mark reached the segment's handler.
  Decided: the whole mark selects its event; the button stays for the keyboard. Three clicks
  of wave 7's cases that hit a segment at its centre could then land on a mark (one did); they
  click the segment's own button now. No assertion of an existing case changed.
- **A media type of 4,000 characters widened the page to 28,897 px.** A web server chooses
  that value. Decided: every value the panel writes wraps inside the panel.
- **300 requests pushed the timeline 16,313 px down.** Decided: the requests list and the
  selection have a bounded height and scroll inside themselves; the list follows a new row
  while it is at its end. That a list a reader scrolled up stays in place rests on the
  browser's scroll anchoring, checked in Chromium, the one browser the page tests run.
- **The adversary found no input that breaks the tree builder**: 112 trees worked out by hand
  from the snapshot of the standard, 100,000 random inputs compared with another parser (6
  differences, all on `search`, where this parser follows the snapshot), 30,000 inputs with
  the later stories' tags. It found that the reconstruction of the active formatting elements
  scanned the stack of open elements from the bottom for every character of text: 1,000
  nested `b` and 1 MiB of text took 4.458 s in a debug build, 0.303 s after the change. It
  also found a defect no test would catch (the check for a tentative confidence taken out of
  the `meta` rule); a test holds it now.
- **The parser has no limits.** The unit and the adversary both measured it: 100,000 nested
  `div` took 545 s in a debug build; 21,897 bytes of input gave 1,003,005 nodes through the
  reconstruction. The standard's algorithms do the same. The bounds are
  `story:html-parser-limits` (wave 10), before `story:html-document-port` (wave 11) makes a
  crate of the product depend on `surf-html`. Decided: nothing is bounded in this wave.

## The gate on the wave branch

`slot task check` in the runner's form (`CI=true GITHUB_ACTIONS=true CARGO_INCREMENTAL=0
RUSTC_WRAPPER=""`), exit 0, on `wave/a8-request-body-tree-detail` after the five units, at
`9e1b18c`. It ran first after the three stories, at `3c6aa7b` (121 suites, 502 passed, `30
passed` in the page tests), and again after this page; the store's evidence for the stories
names that last run.

```
surf v1 — 9 file(s), 14 scenario(s), valid
95 scenario(s) (14 authored), 28 refusal(s), written to .out/suite.json
  95 scenarios: 95 passed, 0 failed, 0 error, 0 unsupported
the interpreter and the Rust system ran 95 scenario(s) each, of suite sha256:5fe34265b01df3ccfc43f30d4ddb41759c22407845edb2e1e5e675481bd29b9f
generated/rust matches spec/ (30 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
Cargo.lock: 22 package(s), none from outside this repository
121 test suites: 503 passed, 0 failed, 0 ignored
view-stream-check: 19 event name(s), each a message of the AsyncAPI projection
  38 passed (17.1s)
html-conformance: 9107 cases, 9095 run, 8362 passed
```

Before the wave: 93 scenarios (12 authored), 118 suites with 486 passed, `24 passed` in the
page tests, and `html-conformance: 9107 cases, 9095 run, 7121 passed`. The refusals are the
same 28 lines; the gate's planted defect (a `Navigation` storage that drops every `put`) fails
33 of the 95; the conformance crate's tests are `6 test suites: 28 passed`. Of the 8,362 cases
82 are encoding cases, 7,046 tokenizer cases and 1,234 tree construction cases: `tag body
476/476/476`, `tag formatting 8/22/22`, `tag tables 11/39/39`, `tag template 29/123/123`, `tag
foreign 6/222/222`, `tag mixed 704/1085/1085`. The 54 cases of the tags `formatting`,
`tables`, `template` and `foreign` that pass, pass through the generic rules, and so do
`quirks01.dat` 2 to 4; each later tree story measures its tag again.

## Findings, and where each goes

| finding | from | goes to |
|---|---|---|
| the parser has no bound on depth, nodes, a text node's length, attributes, the list of active formatting elements, or time; nested block start tags and nested formatting start tags are quadratic in depth; the reconstruction makes n x m nodes | `html-tree-body`, its adversary | `story:html-parser-limits` (wave 10), which already names the reconstruction; no crate of the product depends on `surf-html` before `story:html-document-port` (wave 11) |
| a `go` to an address with a space in its query ends `FetchFailed` `Unreachable`: `crates/fetch` refuses an address with a byte outside 0x21 to 0x7e, and a `go` address is not passed through the URL parser, which would percent-encode it (read in the source, not run) | `view-detail-panel` | backlog: `story:agent-go-address-parsed`, a draft |
| `surf.net.MediaType` is specified without parameters; a `multipart/form-data` body needs its `boundary` | `agent-request-body-spec` | `story:agent-forms-spec` (wave 13), to settle when it is read before its wave |
| a planted defect that drops the body from the binding's mapping passes all 95 scenarios on the interpreter: the specification stays consistent with itself; two hand-written cases catch it | `agent-request-body-spec` | accepted: `crates/surf/tests/request_method.rs` holds it |
| every stream entry rebuilds all rows of the requests list | the page's adversary | backlog: `story:view-page-memory` (a draft since wave 7), with a line added |
| the timeline is about 7,000 px tall with 300 marks in one segment | the unit of the three page findings | backlog: `story:view-page-memory`, the same line |
| the contrast of the panel's colours in the dark scheme was not measured; widths below 1,000 px were not looked at | `view-detail-panel` | `story:view-acceptance` (wave 15), to settle when it is read before its wave |
| `tag template` was 30 half-way through the unit and is 29; inferred, not proven: `template.dat` case 6, whose end tag `template` goes to "any other end tag", which stops at a special element | `html-tree-body`, the unit of the three tree findings | `story:html-tree-template` (wave 12) |
| `story:html-tree-formatting` names "any other end tag" as its own line; `html-tree-body` wrote it | the reading before the wave | `story:html-tree-formatting` (wave 9), corrected when it is read before its wave |
| `crates/html-conformance/src/dump.rs` recurses over the tree's depth | `html-tree-body` | accepted: test tooling over vendored data; `story:html-parser-limits` bounds the depth |
| the named character reference state's error order (from wave 7) | `html-tree-body` | closed: fixed in the tokenizer, with a unit test |
| the tree builder passes the last start tag to the tokenizer (from wave 6) | `html-tree-body` | closed: one tokenizer serves a whole parse |

## After this wave

Wave 9 of [the wave plan](agent-waves.md): `agent-fetch-post`, `html-actionable-elements` and
`html-tree-formatting`, each read against `main` first.
