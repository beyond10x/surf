# Agent interface plan, wave 9 of 22: a POST in the fetch crate, the actionable elements, and misnested formatting

One wave of the [wave plan](agent-waves.md). Three stories, each in its own worktree, each
implemented by a sub-agent on Opus; three units of fixes beside them, one after each
adversary's pass. The coordinating session wrote no code. Every commit is the organization
bot's.

After this wave `surf-fetch` sends a `POST` with a body and its media type and changes the
method on a redirect as the Fetch Standard says, and `surf::fetch_http` and the session's
transport send a `Post` in place of refusing it. No command sends one yet: a user of the
commands sees no change until the form stories. The tree builder runs the adoption agency
algorithm and the "Noah's Ark" clause, so misnested formatting gives the standard's tree:
1,300 of the 1,809 tree construction cases of the vendored test data pass, every case tagged
`formatting` among them. A new crate, `surf-page`, reads the title and the actionable elements
out of a tree. No parser runs on a real page yet: a real page's document stays `Parsing` until
`html-document-port`, and no crate of the product depends on `surf-html` or `surf-page`.

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `agent-fetch-post` | `crates/fetch`: `send(&Request, &Limits)` beside `fetch(address, limits)`, `Request` (address, method, body, media type), `Method` (`Get`, `Post`), `Limits::max_request_body_bytes` (16,777,216); a `Post` writes `Content-Length` always and `Content-Type` when it has a media type; a 301, 302 or 303 after a `Post` continues as `Get` without the body and its two fields, a 307 or 308 repeats them; a body over the limit, a media type that is not `type/subtype` of tokens and a `Get` with a body end `Unreachable` before the host is resolved. `surf::fetch_http` and `crates/session` send a `Post` through it | 105 tests in `surf-fetch` (91 before), 13 of them in `tests/post.rs`; the memory case measured 16,785,434 bytes against the stated 16,917,505; `request_method.rs` 5, `core.rs` 10; 10 planted defects, 9 red (see below) | `fa2b756` |
| `html-actionable-elements` | `crates/page` (package `surf-page`, new, depends on `surf-html` only): `title`, `actionable_elements` with `ActionableElement { node, kind, target, value, hidden, disabled, checked, required }` and nine kinds, `is_hidden` | 21 tests in `tests/elements.rs`; `Cargo.lock: 23 package(s)`; 12 planted defects, each red | `82d638e` |
| `html-tree-formatting` | `crates/html/src/tree_builder/formatting.rs` (new): the adoption agency algorithm with the step numbers of the snapshot; `modes.rs`: the `a` and `nobr` start tags and the end tags of the fourteen formatting elements; `tree_builder.rs`: Noah's Ark; `tree.rs`: one crate-private operation that moves all children of a node; `NOT_YET_PARSED` is `tables-and-select`, `template`, `foreign-content` | `tag formatting 22/22/22` (8), `tag mixed 755/1085/1085` (704), `tag template 30/123/123` (29), `html-conformance: 9107 cases, 9095 run, 8428 passed` (8,362); 44 tests in `surf-html`; 10 planted defects, each red | `fbe6164` |
| four findings in `surf-fetch` (after the adversary's pass) | `crates/fetch/src/lib.rs`: the documentation of a server that answers and closes, the memory bound while a body is written, `Debug` of `Request` by hand; three new cases in `tests/post.rs`, one in `tests/adversary_redirect_memory.rs` | 109 tests in `surf-fetch`; the four new cases green 20 times each; 4 planted defects, each red | `6ea5a5c` |
| four findings in `surf-page` (after the adversary's pass) | `crates/page/src/lib.rs`, `tests/elements.rs`, one paragraph of `docs/design/html-parser.md`: `aria-hidden` compared without case, a closed `dialog` and the content of a closed `details` hidden, `is_hidden` without a scan of the document's start, the deep-tree test on nested `span` | the adversary's case, red before; 24 tests; 6 planted defects, each red | `fd96719` |
| two findings in the tree builder (after the adversary's pass) | `crates/html/src/tree_builder/formatting.rs`, `modes.rs`, `tree_builder.rs`: the scans of the stack per formatting end tag and per `a` start tag; three tests and six assertions in `tests/tree_formatting.rs` | the conformance counts unchanged, line for line; 47 tests in `surf-html`; 9 planted defects, each red | `4b4f653` |

Each unit's claim was checked by the coordinator before its commit: the unit's packages were
tested again with warnings denied, with a count of each named test; `cargo doc`; for the tree
builder the conformance task and `cargo fmt --check`; for `surf-page` its dependency tree and
`lock-check`; for the fix units also `cargo clippy`. Each of the three stories was then given
to an adversary on Opus, which attacks a green unit and writes failing tests.

## The stories were measured before the wave

All three stories were read against `main` by a reviewer first, as in waves 3 to 8. None could
be met as written: 13 places in `agent-fetch-post`, 12 in `html-actionable-elements`, 9 in
`html-tree-formatting`. The bodies were revised and each story got a typed scope, so that
`aep plan artifact waves` puts the three in one wave with no file in two of them (`f175a84`).

Decided before the wave:

- **`send` is added beside `fetch`.** `fetch(address, limits)` stays as the `Get` without a
  body, so its 52 calls in the tests do not change. The request body is borrowed from the
  caller and not copied; the stated sum while a response body is read stays 16,917,505 bytes
  at the defaults.
- **A request the crate will not send ends `Unreachable` before any connection**: a body over
  `max_request_body_bytes` (16 MiB by default, a choice), a media type that is not
  `type/subtype` of tokens (which refuses CR, LF and a parameter), a `Get` with a body.
- **The redirect rule is the Fetch Standard's**: a 303 changes a method other than `Get`, a
  301 or 302 changes a `Post`, and a method once changed stays `Get` for the rest of the
  chain. Request headers stay out.
- **`html-actionable-elements` writes a new crate and no file of `crates/html`**: the readers
  it needs exist. `actionable_elements` computes the inherited flags in one walk with its own
  stack; a walk up the parents per element measured quadratic in a probe.
- **The tests of `surf-page` use no table, template, SVG, MathML or misnested formatting**, so
  that the order of the merges with `html-tree-formatting` does not matter.
- **The adoption agency algorithm was prototyped in a scratch copy before the unit started.**
  The story's acceptance names every count the prototype measured (the three `tag` lines, the
  total, 16 file lines of the expected file), and the unit reached each. "Any other end tag"
  is `html-tree-body`'s and was written in wave 8; the story no longer names it.
- **Two findings of the reading go to `story:html-tree-template`** (wave 12): a `template`
  start tag in the head is popped in place of `head`, so `body` becomes a child of `head`;
  and the `template` case that stopped passing in wave 8.

## What went wrong, and what was decided

- **`surf-page` followed the standards over the story in three places.** An `input` in the
  Email state with `multiple` has its value split on commas and each part stripped; the
  story's one rule for URL and Email would have stripped the whole. `aria-hidden` on the
  document element or on the body element is not read (WAI-ARIA 1.3: user agents must not
  expose the hidden state there). The third, that `aria-hidden` is compared with `true`
  exactly, was the unit's reading and did not hold: see the next item.
- **`aria-hidden="TRUE"` did not hide.** The adversary's failing case. WAI-ARIA maps
  true/false values in HTML to enumerated attributes, which match their keywords without
  regard to ASCII case, and ARIA in HTML says browsers do so. Decided: compared without case,
  nothing stripped.
- **A closed `dialog`'s buttons were listed as visible.** The rule for "hidden" in the design
  note stopped at one section of the HTML Standard's user-agent style sheet. Decided: the two
  further rules that need no style computation are added, `dialog:not([open])` and the
  content of a `details` without `open` but for its first `summary`; the note's paragraph
  says so. Decided not to add: the fallback content of `video`, `audio` and `object` (surf
  plays no media and loads no object, so the fallback is what is shown); content of a
  `select` that is not an `option` (the parser does not build a `select` yet).
- **The deep-tree test of `surf-page` stood on a tree the standard does not give**: 20,000
  nested `select`, which hold only until `html-tree-tables` closes a `select` inside a
  `select`. It nests `span` now.
- **`is_hidden` scanned the start of the document on every call.** With 20,000 comments
  between `head` and `body`, a call for every node took 6.7 to 9.1 s in a debug build; 5.3 to
  8.4 ms after the change. Nothing calls it for every node yet; `html-accessible-name` will.
- **The story's step numbers were not the snapshot's.** The snapshot the design note names
  inserts sanitizer and redirection steps; the story's "step 4.14" is 4.19 to 4.23 there.
  The code carries the snapshot's numbers, and the steps that cannot act in this parser are
  comments.
- **The adversary found no input that breaks the algorithm**: every step compared with the
  snapshot, 44 hostile inputs, 8,000 seeded inputs with tree invariants, 40,000 random inputs
  compared with another parser (no tree differs). It found three planted defects that no
  test and no conformance count catches (a node not in the list left on the stack, an inner
  loop's clone without its attributes, Noah's Ark comparing attributes in their order), and
  six that only the counts catch. Each has a test or an assertion now.
- **A formatting end tag on a deep stack got slower.** The step that asks whether the last
  node is an ancestor of the place it is inserted at walks to the root, and scanned the whole
  stack there: 1 MiB of 60,000 nested `object` and misnested `a` took 38.0 s in a release
  build against 1.23 s at the base of the wave (the adversary's measurement). The scan is
  gone (50.4 s before the change and 34.6 s after, in the fix unit's own runs on a loaded
  machine); the walk is the standard's and stays. Decided: nothing is bounded in this wave;
  the depth limit of `story:html-parser-limits` (wave 10) bounds the stack, and that story
  gains this input.
- **The crate documentation of `surf-fetch` was wrong about a server that answers and
  closes.** It said the fetch ends `Unreachable`. The adversary's case got the server's 413
  in 50 of 50 runs with a body of 7 bytes: a body that fits the socket's buffers is written
  without an error. The code stays; the documentation states both outcomes and that a caller
  may rely on neither.
- **One planted defect of the fetch unit stayed green**: with the check of `send` that
  refuses a `Get` with a body removed, no test failed, and the body would have followed a
  `GET` head without a `Content-Length`. A case holds it now. The unit reported the gap and
  it went to the adversary by name, with two more: the memory case missed a copy dropped
  before the response is read (a new case bounds the whole fetch of a 4 MiB body sent twice
  at 73,728 bytes; 8,761 measured), and the media type check takes the token characters of
  RFC 9110 where RFC 6838 wants a letter or digit first (no byte 0x00 to 0xff in a media
  type changes the head; decided: unchanged).
- **`Request` printed its body.** It derived `Debug`, so `{:?}` printed the bytes of a form
  with its passwords. `Debug` is written by hand and prints the body's length.

## The gate on the wave branch

`slot task check` in the runner's form (`CI=true GITHUB_ACTIONS=true CARGO_INCREMENTAL=0
RUSTC_WRAPPER=""`), exit 0, on `wave/a9-post-actionable-formatting` after the six units, at
`a9532a1`. It ran first after the three stories, at `90dabd8` (126 suites, 543 passed), and
again after this page; the store's evidence for the stories names that last run.

```
surf v1 — 9 file(s), 14 scenario(s), valid
95 scenario(s) (14 authored), 28 refusal(s), written to .out/suite.json
  95 scenarios: 95 passed, 0 failed, 0 error, 0 unsupported
the interpreter and the Rust system ran 95 scenario(s) each, of suite sha256:5fe34265b01df3ccfc43f30d4ddb41759c22407845edb2e1e5e675481bd29b9f
generated/rust matches spec/ (30 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
Cargo.lock: 23 package(s), none from outside this repository
126 test suites: 553 passed, 0 failed, 0 ignored
view-stream-check: 19 event name(s), each a message of the AsyncAPI projection
  38 passed (15.8s)
html-conformance: 9107 cases, 9095 run, 8428 passed
```

Before the wave: 121 suites with 503 passed, `Cargo.lock: 22 package(s)`, and
`html-conformance: 9107 cases, 9095 run, 8362 passed`. The scenarios, the refusals and the
suite's hash are those of wave 8: the wave changed no file of `spec/`. The gate's planted
defect (a `Navigation` storage that drops every `put`) fails 33 of the 95; the conformance
crate's tests are `6 test suites: 28 passed`. Of the 8,428 cases 82 are encoding cases, 7,046
tokenizer cases and 1,300 tree construction cases: `tag body 476/476/476`, `tag formatting
22/22/22`, `tag tables 11/39/39`, `tag template 30/123/123`, `tag foreign 6/222/222`, `tag
mixed 755/1085/1085`. The 47 cases of the tags `tables`, `template` and `foreign` that pass,
pass without rules of their own: through the generic rules, and `template.dat` case 84
through the adoption agency algorithm. Each later tree story measures its tag again.

## Findings, and where each goes

| finding | from | goes to |
|---|---|---|
| each formatting end tag walks up to eight times from the common ancestor to the Document, and scans and shifts a stack that is long above the formatting element: 34.6 s and 2.5 s for two inputs of 1 MiB in a release build | `html-tree-formatting`, its adversary, the unit of the two tree findings | `story:html-parser-limits` (wave 10), with the two inputs as lines for its adversarial cases |
| the list of active formatting elements grows without bound when the tags differ in attributes, and Noah's Ark scans it for each formatting start tag (30,000 nested `b` with different ids: 7.9 s in a prototype's release build); each end tag adds at most 32 elements | the reading before the wave, `html-tree-formatting` | `story:html-parser-limits` (wave 10) |
| nested block start tags parse quadratically (20,000 `fieldset` with a `legend`: 55.4 s in a debug build) | the adversary of `surf-page` | `story:html-parser-limits` (wave 10), which names nested blocks already |
| `is_hidden` walks up from its node on each call, and two shapes cost more than the node's depth: a `body` with `aria-hidden` true after many comments, a `summary` after many children of a closed `details`; a node outside the document is judged by the ancestors it has | `html-actionable-elements`, the unit of the four page findings | `story:html-accessible-name` (wave 10), to settle when it is read before its wave |
| content of a `select` that is not an `option` is listed as visible | the adversary of `surf-page` | `story:html-tree-tables` (wave 11), to settle when it is read before its wave |
| `aria-hidden` content can be visibly rendered; its rows are flagged hidden | the adversary of `surf-page` | accepted: the design note's decision; the row is kept and flagged |
| a `template` start tag in the head is popped in place of `head`, so `body` becomes a child of `head` | the reading of `html-actionable-elements` | `story:html-tree-template` (wave 12), in its body since `f175a84` |
| a `template` that is not on the stack of open elements is not found by the ancestor check of the adoption agency algorithm; not reached without scripts | `html-tree-formatting` | accepted: stated in the code |
| an error answer that a server sends before it closes on a large request body is lost as `Unreachable`, against the SHOULD of RFC 9112 section 9.5; a server that answers and holds the connection without reading a 16 MiB body blocks the fetch until the read timeout | `agent-fetch-post`, its adversary | backlog: `story:fetch-early-answer`, a draft |
| after a `Post` that a redirect turned into a `GET`, the `Requests` row and the events still say `Post` with the form's media type; the specification says the request keeps its identity and its address and does not name the method | the adversary of `agent-fetch-post` | `story:agent-forms-spec` (wave 13), to settle when it is read before its wave |
| a derived `Debug` prints a request body in the generated types (`Fetch`, `Navigate`, `RequestStarted`, `NavigationStarted`, the outcomes and invocations that hold them) and a response body in `surf_fetch::Response` and `surf::Answer` | the unit of the four fetch findings | `story:agent-forms-spec` (wave 13), the first story that puts a form's values into a body |
| the media type check takes the token characters of RFC 9110; RFC 6838 wants a letter or digit first | `agent-fetch-post`, its adversary | accepted: no accepted value changes the request's head |
| `cargo fmt -p surf-fetch -- --check` prints diffs in five test files of earlier reviews; the gate does not run it | the unit of the four fetch findings | accepted: `AGENTS.md` names the five beside the files of this kind in other packages |
| no command sends a `Post`: nothing of `agent-fetch-post` is reachable by a user of the commands | the reading before the wave | `story:agent-forms-spec` and the form stories after it |
| "any other end tag" named as `html-tree-formatting`'s own line (from wave 8) | the reading before the wave | closed: the story was corrected in `f175a84` |

## After this wave

Wave 10 of [the wave plan](agent-waves.md): `html-accessible-name` and `html-parser-limits`,
each read against `main` first.
