# Design: page scripts, their limits, the session's bounds and the swell tag

Decision note of `story:scripts-decisions` (`epic:scripts-in-a-page`), written on 2026-11-24. It
answers the story's questions Q1 to Q7, Q10, Q11 and Q13 to Q18, one section each; Q8 and Q12
moved to `story:webidl-bindings` and Q9 to `story:scripts-dom-selectors`, and their numbers stay
unused so that every "Q<n>" another story cites names the same question. Each section ends with
one line `Settled:` that names what settles it: a table of the section, a `probe:` line, or a
cited text with its date or commit. A later story that finds a value wrong says so in its
report, and the coordinating agent revises the note; Q4, Q5 and Q11 hold starting values that
`story:scripts-parse-run` and `story:scripts-document-port` measure again.

Every `file:line` of surf is read at the base of this note, `main` at `29945f5`. Every fact about
swell is read at its tag `0.8.0`, commit `0c1827128b57845e203713e87a0b0949fc8d63be` (2026-11-24),
with `git show 0.8.0:<path>` and `git grep -n <pattern> 0.8.0 -- <paths>`, and is written
`<path>:<line> @ 0.8.0`.

The texts read, all fetched on 2026-11-24:

| text | version read |
|---|---|
| WHATWG HTML Standard, `multipage/scripting.html` and `multipage/parsing.html` | "Last Updated 23 November 2026", snapshot `546d834701d171e3b164da8463d62878deb1e954`; sha256 `349b242ee2b82d6a86860dcde6e73a27b58ef500bfd4f149684214b910033ff8` and `30eb9a58baa2492aaa7e7e6efe4a73f90b31334a76ad6270958e623d23be4145` |
| WHATWG MIME Sniffing Standard | "Last Updated 19 October 2026", snapshot `ba4b3c385b9765ddfe4a994e2bb89e0ab96068e2` |
| WHATWG DOM Standard | commit snapshot `e4f4f4f0ace83fe6c52ab68e047351ce7f2dcd69`, "Last Updated 15 November 2026" (the snapshot `story:webidl-reader` vendors) |
| W3C Trusted Types | Editor's Draft, 18 November 2026, `https://w3c.github.io/trusted-types/dist/spec/` |
| The Cargo Book, "Profiles" | `https://doc.rust-lang.org/cargo/reference/profiles.html`, sha256 `8ec081e473b74d6b73b5b587ed6e2e7e9f586b9788c905e5949c5bfae2c1d51a` |
| Apache License 2.0 | `https://www.apache.org/licenses/LICENSE-2.0.txt`, sha256 `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30` |
| GitHub's runner images | `actions/runner-images` `README.md` and `images/ubuntu/Ubuntu2604-Readme.md` at `main` (sha256 `7d8b4625300a10d13cc52db90c92462b1df475fce80247dffca2101e92926d78`, `57a912f70eb971f6d88fa55efbaa926ea45acd8f051f7469cb0663ab10b85d1d`); Ubuntu's package pages of `util-linux` in `resolute` |
| swell | tag `0.8.0`, commit `0c1827128b57845e203713e87a0b0949fc8d63be`, `https://github.com/beyond10x/swell` |

Section names of the HTML Standard below are its anchors on `https://html.spec.whatwg.org/multipage/`;
the numbers (4.12.1.1, 13.2.6.4.8) are those of the snapshot above. Steps of "prepare the script
element" are numbered as the snapshot numbers its ordered list: 38 top-level steps, a bulleted
list inside step 10 not counted.

The probes are programs that are never committed: a cargo package under `.out/probes/swell-limits/`
with a `[workspace]` table of its own and swell as a git dependency at `0.8.0`, built with `slot
cargo build --locked --manifest-path .out/probes/swell-limits/Cargo.toml` (and `--release`) and
run as `.out/probes/swell-limits/target/<profile>/swell-limits-probe <mode>`; and two POSIX sh
scripts, `.out/probes/session-bounds.sh` and `.out/probes/session-ends.sh`, which measure and wait and so start without `slot` (`sh
.out/probes/<name>.sh`); the builds inside them take a slot. They ran on two machines on
2026-11-24, both x86_64 Linux with kernel 6.18, rustc 1.99.1 of `rust-toolchain.toml` and
`.cargo/config.toml` read from the tree's root (so `debug = false` in the test profile): "the
build node", where the pool ran the first round's jobs (the parse sweep of Q4, the session probe
of Q11, `task html-conformance` of Q3), and "the workstation", where the second round's binaries
and scripts ran and every `git` probe and reading of `/proc/sys` was made. Each `probe:` line
below names its machine. The
probe of `swell-limits` runs with the limits of Q4 (its line `limits Limits { steps_per_entry:
10000000, ... }`) and, in release, `overflow-checks = true` (Q17). A probe's output lines below
are quoted from its log; a number without a `probe:` line or a citation is marked "chosen", with
its reason.

## Q1. Which script elements run

A `script` element runs when the steps of "prepare the script element" (4.12.1.1,
`#prepare-the-script-element`) reach step 38.3, "immediately execute the script element", for an
element the parser inserted. The first version takes the steps that decide whether an inline
classic script runs, and stops, with one reason word, where the standard would fetch, queue or
hand the element to a later part the first version leaves out (Q2). The tree builder decides the
row from the element as the parser made it and calls the hook of Q2 with "run" or with the word.

| case | first version | step of "prepare the script element" |
|---|---|---|
| no `type`, no `language` | runs | step 10, third bullet: type string `text/javascript`; step 11: classic |
| `type=""` | runs | step 10, first bullet: `text/javascript` |
| `type` one of the 16 JavaScript MIME type essences of MIME Sniffing 4.6 (`application/ecmascript`, `application/javascript`, `application/x-ecmascript`, `application/x-javascript`, `text/ecmascript`, `text/javascript`, `text/javascript1.0` to `text/javascript1.5`, `text/jscript`, `text/livescript`, `text/x-ecmascript`, `text/x-javascript`), in any ASCII case, with or without leading and trailing ASCII whitespace | runs | step 11: "a JavaScript MIME type essence match", also after stripping whitespace |
| `type="text/javascript; charset=utf-8"` (a parameter) | not run: type | step 15: the whole string is no essence match |
| `type=module` (any ASCII case) | not run: module | step 12; modules are swell's slice 11 (`docs/design/engine.md:2486 @ 0.8.0`), refused until then with a `SyntaxError` (`CHANGELOG.md:104-106 @ 0.8.0`) |
| `type=importmap` | not run: importmap | step 13 |
| `type=speculationrules` | not run: speculationrules | step 14 |
| any other `type` (a data block, `text/template`, `application/json`) | not run: type | step 15, "Otherwise, return" |
| no `type`, `language=""` | runs | step 10, second bullet |
| no `type`, `language=javascript` (any value whose `text/<value>` is an essence) | runs | step 10, "text/" and the attribute's value; step 11 |
| no `type`, `language=vbscript` (any other value) | not run: type | step 10, then step 15 |
| `type` and `language` both present | as its `type` alone | step 10: "always ignored if there is a `type` attribute" |
| `src` present, element empty | not run: src | step 35 fetches; the first version fetches no script |
| `src` present, element with text | not run: src | step 35; the text is never run (step 36 is for no `src` only) |
| `src=""` | not run: src | step 35.3 (an `error` event, which the first version does not fire) |
| no `src` and an empty text (no child text, or text nodes that are all empty) | not run: empty | step 8: "source text is the empty string, then return" |
| `nomodule` on a classic script | not run: nomodule | step 21: "If el has a nomodule content attribute and its type is "classic", then return" |
| `nomodule` on a module script | not run: module | step 12 comes first; step 21's note: "specifying nomodule on a module script has no effect" |
| `async` or `defer` on an inline classic script | runs, at its end tag | steps 16 and 37 to 38: for an inline classic script neither attribute is read; step 38.3 runs it |
| `event` and `for` attributes, `for` not `window` or `event` not `onload`/`onload()` | not run: event | step 24.4 and 24.5 |
| `event=onload for=window` | runs | step 24 does not return |
| a script in `template` contents | not run: disconnected | step 9: the contents are a document fragment, not connected |
| an SVG `script` in foreign content (`docs/design/html-parser.md:627`), however it is closed: its own end tag, self-closing, `</svg>`, an HTML end tag, a breakout start tag, `</script>` while an SVG child is the current node, or open at the end of the input; one record, when it leaves the stack of open elements or the parse ends, whichever comes first | not run: svg | not this algorithm: foreign content's own `script` end tag ("in foreign content", 13.2.6.5), which processes an SVG script element; the first version runs HTML script elements only. A MathML `script` is no script element and gets no record |
| a script element a script created and inserted into the document | not run: inserted | the DOM's insertion steps reach "prepare the script element" for an element that is not parser-inserted; the first version runs only parser-inserted elements, and the DOM write that connects one records it (Q7) |
| a parser-inserted script element a script moved after it ran | no second record | step 1: "already started is true, then return" |
| a `script` element open when the parser stopped: at the end of the input (`<script>x = 1` with no end tag), or because a limit stopped the parse inside it | not run: eof | 13.2.6.4.8, "An end-of-file token": "If the current node is a script element, then set its already started to true"; it is popped and never prepared. A limit stops the parse before the end tag, and "stop parsing" pops it the same way |
| a parser-inserted element not run for `empty`, `disconnected` or `type` that a script later gives text, a usable `type` or a connected place | no second record; its first record stays | these return at steps 8, 9 and 15, before step 17 sets already started; step 3 has set the parser document to null "so that ... another script can later mutate them and cause them to run again", and a browser runs it then; the first version runs scripts only at the parser's end tag (a deviation, below) |
| a parser-inserted element not run for `event`, `nomodule` or `src`, later changed or moved by a script | no second record | these return at steps 24, 21 and 35, after step 17 set already started, so step 1 returns on every later prepare: a browser never runs them again either |
| any element in a document whose session has no scripts | not run, no record | step 20 ("scripting is disabled"): no hook is called, and Q7 records nothing |

reasons: empty, src, type, module, importmap, speculationrules, disconnected, nomodule, event, svg, inserted, eof

The deviations of the first version from the HTML Standard, each a row above or a step of Q2:
a script element a script inserts is not run (`inserted`); a parser-inserted element not run for
`empty`, `disconnected` or `type` is not run later when a script changes it or moves it (no second
record); a module, an import map and speculation rules are not run (`module`, `importmap`,
`speculationrules`); an SVG `script` is
not run (`svg`); external scripts are not fetched (`src`); Content Security Policy is not read
(steps 22 and 23); `document.write`, `document.currentScript` and the `load` and `error` events
of a script element do not exist (Q2).

The epic's sentence "A `nomodule` classic script is skipped, as the HTML Standard says (from
memory, to be confirmed)" (epic, "In and out of the first version") is confirmed by step 21 (Q17).
Content Security Policy (step 23) is not read: surf reads no `Content-Security-Policy` field, so
no inline script is blocked by one (Q2).

Settled: the table above, its `reasons:` line and the list of deviations, each row citing HTML Standard 4.12.1.1 `#prepare-the-script-element` or 13.2.6.4.8 `#parsing-main-incdata` ("Last Updated 23 November 2026", snapshot `546d834701d171e3b164da8463d62878deb1e954`) and MIME Sniffing 4.6 `#javascript-mime-type` (snapshot `ba4b3c385b9765ddfe4a994e2bb89e0ab96068e2`).

## Q2. The steps of the `script` end tag

Today the "text" insertion mode pops the element at any end tag and returns to the original
insertion mode (`crates/html/src/tree_builder/modes.rs:1120-1141`; the module doc says the same,
`crates/html/src/tree_builder.rs:28-29`).

The steps of 13.2.6.4.8 (`#parsing-main-incdata`), "An end tag whose tag name is "script"", while
parser scripting is enabled and a hook is set. The standard writes them as paragraphs without
numbers; the numbers below are this note's, and its 5 and 6 are one paragraph of the standard:

| n | step | first version |
|---|---|---|
| 1 | perform a microtask checkpoint if no speculative parser and the execution context stack is empty | left out: no promise and no job exist before swell's slice 10 (HOST-192 to HOST-200 are `pending slice 10`, `conformance/host/requirements.txt:192-200 @ 0.8.0`), so the queue is always empty |
| 2 | let script be the current node | taken |
| 3 | pop the current node | taken |
| 4 | switch to the original insertion mode | taken |
| 5 | old insertion point := insertion point | left out: the insertion point exists only for `document.write`, which the first version does not have |
| 6 | insertion point := just before the next input character | left out: the insertion point |
| 7 | increment the script nesting level | left out: without `document.write` the parser is never invoked again inside a script, so the level would only be 0 or 1; a script nesting level above 1 does not arise |
| 8 | prepare the script element (if no speculative parser) | taken: the steps of the second table, then the hook |
| 9 | decrement the nesting level; at zero, clear the parser pause flag | left out: the parser never pauses; the hook returns before parsing goes on |
| 10 | restore the insertion point | left out: the insertion point |
| 11 | a pending parsing-blocking script: pause when nested, queue a task in the fragment case, else execute the pending parsing-blocking scripts (which spins the event loop) | left out: no pending parsing-blocking script exists, because external scripts are not run (step 35) and no style sheet blocks scripts (step 38.2, surf loads none); spinning the event loop does not arise |

The steps of "prepare the script element" (4.12.1.1) for an element of the first table:

| n | step | first version |
|---|---|---|
| 1 | already started: return | taken |
| 2 to 5 | parser document, intended document | taken: one document per parse |
| 6 | force async | left out: read only by external and module scripts (step 37.2) |
| 7 | source text := child text content | taken |
| 8 | no `src` and empty source text: return | taken: `empty` |
| 9 | not connected: return | taken: `disconnected` |
| 10 to 15 | the script block's type string and the element's type | taken: `type`, `module`, `importmap`, `speculationrules` |
| 16 to 19 | parser document back, already started, preparation-time document | taken |
| 20 | scripting disabled: return | taken: no hook is called in a document parsed with scripting disabled |
| 21 | `nomodule` on a classic script: return | taken: `nomodule` |
| 22, 23 | Content Security Policy for inline behaviour | left out: surf reads no CSP; a deliberate deviation, named in `scripts-spec`'s comment |
| 24 | `event` and `for` | taken: `event` |
| 25 to 34 | encoding, CORS, nonce, integrity, referrer policy, fetch priority, options, settings | left out: read only for a fetch |
| 35 | `src` present: fetch | left out: `src` |
| 36.1 | base URL | left out: no script of the first version fetches |
| 36.2 classic | create a classic script from the source text, mark as ready | taken: swell's `parse_script` (`crates/swell/src/host/agent.rs:575 @ 0.8.0`) |
| 37 | external classic or module: the async, in-order and deferred lists | left out: not reached |
| 38.1, 38.3 | execute the script element now | taken: `evaluate_script` (`crates/swell/src/host/agent.rs:634 @ 0.8.0`) |
| 38.2 | a style sheet that is blocking scripts: pending parsing-blocking script | left out: surf loads no style sheet |

Of "execute the script element" (`#execute-the-script-element`), steps 1, 2 and 6.3 ("Run the
classic script") are taken; 3 (rendering), 4 and 8 (the `error` and `load` events), 5 and 7 (the
ignore-destructive-writes counter of `document.write`) and 6.1 to 6.4's `currentScript` are left
out (`document.currentScript` is not one of the four HTML members of S6).

`document.write` and everything that serves it (the insertion point, nesting above 1, the parser
pause) stay out of the first version.

The hook `story:html-script-hook` writes, in `surf-html`:

```rust
/// What the tree builder calls at a `script` end tag of a parser-inserted HTML script element
/// while parser scripting is enabled (`ParseOptions::scripting_disabled` false), after steps 2
/// to 4 of 13.2.6.4.8 and the steps of "prepare the script element" the note's Q2 takes.
pub trait ScriptHook {
    /// Run the element's script to its end. The call may write the tree through its public
    /// write surface; it returns when the script ended, however it ended.
    fn run(&mut self, tree: &mut Tree, script: NodeId);
    /// The element is not run, for the reason of Q1's line `reasons:`.
    fn skip(&mut self, tree: &Tree, script: NodeId, reason: NotRun);
}

/// One word of Q1's line `reasons:` each, but `inserted`, which no end tag reports.
pub enum NotRun { Empty, Src, Type, Module, Importmap, Speculationrules, Disconnected, Nomodule, Event, Svg, Eof }
```

The tree builder calls `skip` with `Eof` for a `script` element that is the current node when
the end-of-file token arrives in the "text" insertion mode.

It receives the tree and the element; it returns nothing: the hook keeps each end itself, and
the parse never stops because of a script (a parser limit that a script's write reached ends that
script, Q4, and parsing goes on). The tree builder calls `skip` for an SVG `script` at its end tag
in foreign content with `Svg`. After either call the tree builder goes on with its stack of open
elements and its list of active formatting elements as they were, over the tree the hook left:
they hold node identities, and a node a script removed stays in the tree's arena
(`crates/html/src/limits.rs:34-35` counts "nodes removed from their parent"). So an element the
script removed from the tree still receives what the markup after it holds, as the standard's
stack of open elements keeps it (`<div><script></script>x</div>` with the `div` removed puts `x`
into the removed `div`), and a moved element receives it where it now is. The "appropriate place
for inserting a node" and foster parenting read the node's parent at the time of the insertion,
as the standard does.

The `html-sections` line `open tree-construction script-processing`
(`docs/design/html-parser.md:539`) stays open, and `story:html-script-hook` writes its text as:
"The script end tag in the text insertion mode: the insertion point, a script nesting level above
1, the parser pause and the pending parsing-blocking script, and `document.write`." The line
`open parsing-main-inforeign svg-script-processing` (`docs/design/html-parser.md:627`) stays as
it is.

Settled: the two tables of this section, the table of "execute the script element", the hook's signature and the `html-sections` line, each step citing HTML Standard 13.2.6.4.8 `#parsing-main-incdata` and 4.12.1.1 `#prepare-the-script-element` (snapshot `546d834701d171e3b164da8463d62878deb1e954`).

## Q3. Parser scripting enabled

A document whose session runs scripts is parsed with `scripting_disabled: false`
(`crates/html/src/lib.rs:180-195`, default `true`); every other document is parsed as today. The
flag is "parser scripting disabled" of 13.2.4.5 (`#other-parsing-state-flags`).

What it changes for `noscript`:

- In head, 13.2.6.4.4 (`#parsing-main-inhead`): "A start tag whose tag name is "noscript", if
  parser scripting disabled is false" is listed with `noframes` and `style`: "Follow the generic
  raw text element parsing algorithm" (`crates/html/src/tree_builder/modes.rs:343-349`). With the
  flag true it is "Insert an HTML element for the token. Switch the insertion mode to "in head
  noscript"" (`modes.rs:350-356`, 13.2.6.4.5).
- In body, 13.2.6.4.7 (`#parsing-main-inbody`): "A start tag whose tag name is "noembed"" and
  "A start tag whose tag name is "noscript", if parser scripting disabled is false": the generic
  raw text element parsing algorithm (`modes.rs:951-955`). With the flag true it is an ordinary
  element and its content is markup.

So in a scripted document a `noscript` element holds one text node, the markup inside it is not
parsed, and links or controls written inside it give no element row.

`surf-page`'s hidden rule says today: "`noscript` is not hidden: surf parses with scripting
disabled" (`crates/page/src/lib.rs:56-57`; `docs/design/html-parser.md:318`). The new sentence,
which `story:scripts-document-port` writes into both: "`noscript` is not hidden in a document
parsed with scripting disabled, where its content is markup; in a document whose scripts run, it
is parsed with scripting enabled, holds one text node, and is hidden, with everything in it."

The `#script-on` cases: 14 tree-construction cases carry `#script-on`
(`crates/html-conformance/data/wpt/html/syntax/parsing/resources/`: `tests16.dat` 6,
`scripted_foster01.dat` 2, `scripted_webkit01.dat` 2, `scripted_adoption01.dat` 1,
`scripted_ark.dat` 1, `tests5.dat` 1, `webkit02.dat` 1), and the runner parses each with the flag
false (`crates/html-conformance/src/runner.rs:121`). `task html-conformance` prints no count of
its own for them: the 8 of `tests16.dat`, `tests5.dat` and `webkit02.dat` run and pass inside
their files' counts (197/197/197, 17/17/17 and 49/49/49,
`crates/html-conformance/expected/tree-construction.txt:39`, `:53`, `:62`), and the 6 of the four
`scripted_*.dat` files are the 6 `not-run:needs-script` cases (`:25-28`). On `29945f5`, on the build node, `slot
task html-conformance` printed `html-conformance: 9107 cases, 9095 run, 9095 passed` and `tag
not-run 0/0/12`. The flag changes no expectation: the harness sets it already.

Settled: the flag, the two insertion modes (HTML Standard 13.2.6.4.4 `#parsing-main-inhead` and 13.2.6.4.7 `#parsing-main-inbody`, snapshot `546d834701d171e3b164da8463d62878deb1e954`), the new sentence of the hidden rule, and the count of 14 `#script-on` cases, 8 run and passed and 6 not run, beside `html-conformance: 9107 cases, 9095 run, 9095 passed`.

## Q4. Limits, with values

One agent per document (Q5). The agent's seven limits are fixed when it is created
(`Limits`, `crates/swell/src/host/limits.rs:12-28 @ 0.8.0`; no call changes them after
`Agent::new`), so a limit of the document is held between two scripts: after each script surf
adds `last_entry_steps()` to the document's total and counts the script, and a later script
starts only while each total is under its limit. A DOM write over a parser limit ends its script
where it is reached: the bindings note the limit's word in the host value, set the
`InterruptHandle` (`crates/swell/src/host/agent.rs:359 @ 0.8.0`) and read it at once with
`Cx::charge_steps`, which ends the run in `Terminated(Interrupted)` (HOST-177,
`conformance/host/requirements.txt:177 @ 0.8.0`) and clears the flag; the termination is sticky
for the run (HOST-72), so no `catch` and no `finally` of the script runs after the limit. The
session records the noted word, not `wall-clock`, whatever the entry answered, but for `limit:
memory` and `faulted`, which keep their own word; the note is cleared and the interrupt withdrawn
after the entry (`Runner::document_end`, `bindings::end_with_limit`).

The basis, from the probe `swell-limits` with the limits of Q4 (Q5 names the stack), run on the
workstation on 2026-11-24 unless a line says the build node. `R` below is `.out/probes/swell-limits/target/release/swell-limits-probe`,
`T` the same in the test profile (`target/debug/`):

- the step budget of Q4 reached by `var i = 0; while (true) { i = i + 1; }`: after 0.464 s in
  release and 7.358 s in the test profile, `last_entry_steps 10000000` in both;
- steps per second of the same loop ended by the interrupt after 1 s, three runs each:
  19,978,480 to 21,133,215 in release, 1,372,567 to 1,405,663 in the test profile;
- `heap_bytes()` after `create_realm` with no setup: 202,424 bytes; the engine's ceiling for a
  realm of 300 host functions and 40 host objects, `BINDINGS_REALM_HEAP_BYTES`, is 506,112 bytes
  (`crates/swell/src/host/limits.rs:95 @ 0.8.0`; the probe printed the same constant), so a realm
  with the bindings takes at most 708,536 bytes, 1.1 % of `memory_bytes`;
- parsing under `memory_bytes` of 64 MiB, in release (the first round's run, on the build node): a source of
  `var a<i> = <i>;` lines parsed at 524,260 bytes (0.106 s, the script record then 11,713,890
  bytes) and ended `Terminated(MemoryBudget)` at 1,048,540 bytes; a source of plain statements
  with a comment parsed at 1,048,572 bytes (0.088 s);
- a script's parse spends no step: "It runs no code, calls no hook and spends no step"
  (`crates/swell/src/host/agent.rs:569-570 @ 0.8.0`), and the probe read `last_entry_steps 0`
  after parsing the 524,260-byte source. So the steps of a document never count the parse of its
  scripts; only `eval` and `Function` charge a step per code unit of their source
  (`crates/swell/tests/frontend_ledger.rs:172-176 @ 0.8.0`). The parse of a script is bounded by
  the bytes of its source, by `memory_bytes` (the frontend charges each growth before it is made,
  `crates/syntax/src/ledger.rs:1-7 @ 0.8.0`) and by the wall clock: `parse_script` reads the
  interrupt at its start (`agent.rs:571-573 @ 0.8.0`), and the lexer reads it at least once per
  `POLL_INTERVAL` code units, the walks after the parser once per `POLL_INTERVAL` nodes
  (`crates/syntax/src/ledger.rs:21-23 @ 0.8.0`); the probe's interrupt, set 0.020 s into the parse
  of the 524,260-byte source, ended it `terminated Interrupted` after 0.025 s in release and 0.021
  s in the test profile.

Each value says where it is from: "swell's test value" (`crates/swell/tests/support/host.rs:70-80
@ 0.8.0`), "measured" (the output line beside it), "the parser's" (surf's own limit), or "chosen",
with the reason.

| limit | value | probe command | output line | outcome when reached |
|---|---|---|---|---|
| `steps_per_entry` (one script, with every nested entry) | 10,000,000, chosen: about half a second of work per script in release | `R limits` and `T limits` | `step budget 10000000: terminated StepBudget after 0.464 s, last_entry_steps 10000000` (release); `... after 7.358 s ...` (test profile, where the wall clock of 5 s ends a script first; a test gives limits of its own) | the script: `limit: steps-per-script` |
| `memory_bytes` (the agent's heap over the document's life) | 67,108,864 (64 MiB), swell's test value (`host.rs:73 @ 0.8.0`) | `R limits` | `realm heap_bytes 202424`; `constants ... BINDINGS_REALM_HEAP_BYTES 506112` (the constant from the first round, on the build node) | the script: `limit: memory`; then the row "after `Terminated(MemoryBudget)`". It does not bound the copies swell lists as open (`docs/plan/second-batch.md:284-285 @ 0.8.0`): the address-space backstop of Q11 is their only bound, and requirement 11 asks swell to charge them (Q13, Q18) |
| `frame_depth` | 10,000, swell's test value (`host.rs:74 @ 0.8.0`) | `T limits` | `recursion frames on a stack of min_stack_bytes: throw RangeError "frame_depth exceeded"` | a `RangeError` the script may catch; uncaught, the script: `threw RangeError` |
| `reentry_depth` | 256, swell's test value (`host.rs:75 @ 0.8.0`) | `T limits` | `recursion json-replacer on a stack of min_stack_bytes: throw RangeError "reentry_depth exceeded"` | a `RangeError`; uncaught: `threw RangeError` |
| `parser_nesting` | 1,000, swell's test value (`host.rs:76 @ 0.8.0`) | `T limits` | `min_stack_bytes 4006656` (the parser's need at 1,000 levels is inside it, Q5) | a `SyntaxError` naming `parser_nesting` (`crates/swell/src/host/limits.rs:22 @ 0.8.0`), not a refusal (`crates/swell/src/host/agent.rs:266-275 @ 0.8.0`): `threw SyntaxError` |
| `string_length` | 16,777,216 (2^24), chosen: one String at most 32 MiB of UTF-16, half of `memory_bytes` (swell's tests use 2^30, `host.rs:77 @ 0.8.0`) | `R limits` | `limits Limits { steps_per_entry: 10000000, memory_bytes: 67108864, frame_depth: 10000, reentry_depth: 256, parser_nesting: 1000, string_length: 16777216, bigint_bits: 1048576 }` | a `RangeError`; uncaught: `threw RangeError`. The engine applies it to a String its own operations create; the bindings' glue applies it to a String a DOM member answers, before `Cx::new_string`, with the engine's message (`the string would be longer than string_length`). It does not bound the copies swell lists as open (Q13) |
| `bigint_bits` | 1,048,576 (2^20), swell's test value (`host.rs:78 @ 0.8.0`) | `R limits` | the same `limits` line | a `RangeError`; uncaught: `threw RangeError` |
| steps per document, summed from `last_entry_steps()` | 50,000,000, chosen: five scripts at `steps_per_entry` | `R limits` | `loop run 0: true after 1.000 s, 19980802 steps, 19978480 steps/s` (2.4 to 2.5 s for the sum at those rates) | the script whose steps take the sum to it keeps its own end; each later script: `not run: steps-per-document` |
| scripts run per document | 1,000, chosen: it bounds the records of Q7, at most 1,000 per document; the steps, memory and wall clock bound the work already | none: a choice, not a measurement | none: `story:scripts-parse-run` measures again whether a page with more scripts reaches it | the 1,001st script element and each later one: `not run: scripts-per-document` |
| bytes of one script's source (UTF-8, as the tree holds it) | 524,288 (512 KiB), measured: the largest power of two of the sweep that parsed | `R limits` (the first round's parse sweep, on the build node) | `parse var source of 524260 bytes, 27324 statements, memory_bytes 64 MiB: parsed in 0.106 s, heap_bytes after 11713890`, and `parse var source of 1048540 bytes, 53538 statements, memory_bytes 64 MiB: terminated MemoryBudget in 0.155 s, heap_bytes after 202424` | that script, not parsed: `limit: source-bytes`; later scripts run |
| wall clock per document, from its first script's start, through the interrupt handle | 5 s, chosen: twice the release time of the document's step total, and far below `surfd`'s 60 s for an answer (`crates/surfd/src/daemon.rs:34`) | `R limits` | `parse of 524260 bytes, interrupt set after 0.020 s: terminated Interrupted after 0.025 s` (the clock also ends a parse); `loop run 1: true after 1.000 s, 20780416 steps, 20777341 steps/s` | the running script: `limit: wall-clock`; each later script: `not run: wall-clock`. The interrupt is taken between member calls, not inside one: one call of a walking member of the bindings is linear in the nodes it visits and in the bytes and attributes it reads, which the parser's node bound (`max_nodes`) and its bound of stored bytes (`max_stored_bytes`) bound, and in the code units of a String argument it copies, which `string_length` bounds |
| parser: input bytes (`crates/html/src/limits.rs:17-21`) | 16,777,216, the parser's | none: a script adds no input | `crates/html/src/limits.rs:103` (`max_input_bytes: 16 * 1024 * 1024`) | not reached by a script: the first version has no `document.write`; parsing stops as today |
| parser: depth | 512, the parser's | none: the parser's own limit | `crates/html/src/limits.rs:104` (`max_depth: 512`) | a write that would nest an element deeper: refused, the script: `limit: depth` |
| parser: nodes | 1,048,576, the parser's | none: the parser's own limit | `crates/html/src/limits.rs:105` (`max_nodes: 1024 * 1024`) | a write that would make one more node: refused, the script: `limit: nodes` |
| parser: attributes per element | 1,024, the parser's | none: the parser's own limit | `crates/html/src/limits.rs:106` (`max_attributes: 1024`) | `limit: attributes` |
| parser: stored bytes | 67,108,864, the parser's | none: the parser's own limit | `crates/html/src/limits.rs:107` (`max_stored_bytes: 64 * 1024 * 1024`) | `limit: stored-bytes` |
| parser: steps | 134,217,728 (2^27), the parser's | none: the parser's own limit | `crates/html/src/limits.rs:108` (`max_steps: 1 << 27`) | counts tree construction's scans, not a script's writes; read inside each token, before each entry of a scan of the stack of open elements or of the list of active formatting elements and before each write to the tree, so the token that passes the limit is processed only up to the next read; parsing stops (`stopped_by` `Steps`), and no later script element is reached |
| `eval` and `Function` | allowed, chosen: `host_ensure_can_compile_strings` answers `Ok` (the default, `crates/swell/src/host/hooks.rs:176-181 @ 0.8.0`) | `R limits` and `T limits`, with a hook that answers `Throw` | `eval refused: throw not an error`; `Function refused: throw not an error` in both profiles (the hook's thrown value is what `eval` throws, HOST-116 and HOST-117, `conformance/host/requirements.txt:116-117 @ 0.8.0`) | ran: `eval` charges a step per code unit of its source before the parse (`crates/swell/tests/frontend_ledger.rs:172-176 @ 0.8.0`) and its tree to `memory_bytes` (`:127 @ 0.8.0`), and they reach nothing an inline script does not. HTML refuses them only under a Content Security Policy, which surf does not read (Q2) |
| after `Terminated(MemoryBudget)` | `collect()` once, chosen; a second in the same document stops it | `T limits` | `memory 4 MiB, array growth: terminated MemoryBudget`; `state after memory budget: Alive`; `collect: Ok(true)`; `next entry after collect: normal` | the script: `limit: memory`; after the first such end in a document surf calls `collect()` once (HOST-188 to HOST-190, `conformance/host/requirements.txt:188-190 @ 0.8.0`); the second script of that document that ends `limit: memory`, next to the first or not, makes each later script `not run: memory`; so does a `limit: memory` that ends a refill of a kept `[SameObject]` collection after its first define (all or nothing for a step the object refuses; an end of the run inside a refill ends the document's scripts), after the same one collection |

The DOM a script makes is charged to `memory_bytes` (requirement 3, HOST-175 to HOST-177): each
wrapper is a host object on the engine's heap, `HOST_OBJECT_LIVE_BYTES` (256) bytes plus its
data's size (`crates/swell/src/host/limits.rs:80-83 @ 0.8.0`), counted by `heap_bytes()`; and for
each node a script creates the bindings charge, with `Cx::charge_host_bytes`
(`crates/swell/src/host/mod.rs:577 @ 0.8.0`), the node's term of the tree's heap bound, 3 ×
`NODE_BYTES` (408 bytes), plus twice the bytes it stores (`crates/html/src/limits.rs:45-51`),
before the write, and keep the `HostBytes` until the agent is dropped. So a script that makes
nodes ends at `limit: memory` or at a parser limit, whichever comes first. Measured on the
reading slice (`story:scripts-dom-reading`, 1,000 wrappers grew `heap_bytes()` by 889,447 bytes
against 465,772 for 1,000 calls that made none): a wrapper with its `NodeData` takes 423 bytes,
so 158,649 wrappers alone fill `memory_bytes`.

A rule for surf's own bindings: the host interface's reads of a String's units, `Cx::to_utf16`
and `Cx::to_rust_string_lossy` (`crates/swell/src/host/mod.rs:504-513 @ 0.8.0`, over
`scoped_to_utf16` at `:205-213 @ 0.8.0`), copy without a charge. So a generated binding converts
every argument (Web IDL's conversions may run script) before it copies any String out of the
engine, or charges the copy with `Cx::charge_host_bytes` before it makes it and drops the
`HostBytes` with the copy; no uncharged copy of surf's is alive while script runs.
`story:webidl-bindings` holds the rule with a test. A `DOMException` holds its `name` and
`message` as the engine's Strings, which its host data lists in `trace`: they are on swell's heap
and charged there, and surf keeps no copy of them.

A test gives limits of its own, as swell's tests do (`crates/swell/tests/support/host.rs:70-80 @
0.8.0`): the values above are the session's.

probe: (workstation) .out/probes/swell-limits/target/release/swell-limits-probe limits -> step budget 10000000: terminated StepBudget after 0.464 s, last_entry_steps 10000000

Settled: the table above, five columns, no cell empty, each value marked as swell's test value, measured, the parser's or chosen, from the probe `swell-limits` run with Q4's limits (test profile and release, the `probe:` line above) and the cited lines; the charge of the DOM a script makes; starting values that `story:scripts-parse-run` measures again with the bindings and the DOM.

## Q5. The thread and the stack

`Limits::min_stack_bytes()` is a constant function of the limits
(`crates/swell/src/host/limits.rs:61-70 @ 0.8.0`): 64 KiB, plus the parser's need at
`parser_nesting`, plus `reentry_depth + 1` times 14 KiB (`:46`, `:53`). It does not depend on the
build profile; what the profile changes is whether the stack it names suffices, which the probe
shows by running the deepest paths on a thread of exactly that size:

probe: (workstation) .out/probes/swell-limits/target/debug/swell-limits-probe limits -> min_stack_bytes 4006656
probe: (workstation) .out/probes/swell-limits/target/release/swell-limits-probe limits -> min_stack_bytes 4006656

In both profiles, on a thread of 4,006,656 bytes, a getter that reads itself and a
`JSON.stringify` replacer that calls `JSON.stringify` each ended `throw RangeError
"reentry_depth exceeded"`, and a recursion of 100,000 calls ended `throw RangeError "frame_depth
exceeded"`, with no overflow of the stack. The epic's 14,672,640 bytes ("swell measured ... in a
debug build", epic, "Security") is this formula at `reentry_depth` 1,000 and `parser_nesting`
1,000 (65,536 + 256,768 + 1,001 × 14,336); swell measured 11,276,288 bytes at that depth with
full debug information (`crates/swell/src/host/limits.rs:30-46 @ 0.8.0`).

The thread: today `surf-session` serves its requests on its main thread
(`crates/session/src/process.rs:144-148`), whose stack is the process's stack limit (`ulimit -s`
10,240 KiB on the build host of 2026-11-24). A document whose session runs scripts is parsed on
a thread of its own, made for that parse with `std::thread::Builder::stack_size` of
`min_stack_bytes()` of the document's limits plus a margin for surf's own frames; the request
thread waits for it.

The margin: `min_stack_bytes()` holds swell's frames only (`STACK_BYTES_PER_REENTRY` was measured
with swell's own test host functions, `crates/swell/src/host/limits.rs:30-46 @ 0.8.0`). Below
the first entry run the tree builder, the hook and the parser step; at every re-entry through a
binding the generated binding's frames come in addition. The starting margin is chosen until it
is measured: `(reentry_depth + 1)` × 14,336 bytes, as much again per re-entry as swell's own room,
plus 1 MiB for the frames below the first entry: 257 × 14,336 + 1,048,576 = 4,732,928 bytes, so
the thread of a scripted document has 4,006,656 + 4,732,928 = 8,739,584 bytes for Q4's limits.
`story:scripts-parse-run` measures the margin with a test that runs the deepest nesting the
limits allow on a thread of exactly that size, with the first version's bindings and no binding
made for the test: a call of a binding with a `DOMString` argument whose argument object's
`toString` calls the same binding again, `reentry_depth` times, the innermost `toString` calling
`eval` of a source nested to `parser_nesting`. A generated binding calls no script by itself;
its conversion of the argument (`Cx::to_string`, `crates/swell/src/host/mod.rs:887-898 @ 0.8.0`)
calls the object's `toString` (`crates/swell/src/ops/conversion.rs:132 @ 0.8.0`), and that call
is a re-entry from a host function, checked against `reentry_depth`
(`crates/swell/src/object/ecmascript_function.rs:147 @ 0.8.0`,
`crates/swell/src/interp/mod.rs:9-16,129-130 @ 0.8.0`). The margin is then that test's need
plus a fifth, as swell rounds its own. A stack that overflows ends the session: Rust's handler
of the guard page prints a line to the standard error, which `surfd` closes, and aborts, so
`surfd` reaps a `SIGABRT`, not a `SIGSEGV`:

probe: (workstation) /usr/bin/prlimit --core=0:0 -- .out/probes/swell-limits/target/release/swell-limits-probe overflow -> thread '<unknown>' (...) has overflowed its stack; fatal runtime error: stack overflow, aborting; overflow: exit 134 signal ABRT

The answer then reads as Q11's for an end without a bound word (`note ended: signal SIGABRT`). A document of a session without scripts is parsed on the request thread, as today. An
`Agent` is not `Send` (HOST-136, `conformance/host/requirements.txt:136 @ 0.8.0`), so the agent
lives on that thread only: it is created at the document's first script element that runs, and
dropped at the end of the document's parse, on that thread, after the tree is taken out of the
host value; the thread then ends and hands the tree and the script ends back. Scripts run only
while parsing (no timer, no event), so nothing needs the agent later, and no state passes from
one document to the next.

Settled: the three `probe:` lines above (test profile, release, and the end of an overflow), the thread of `min_stack_bytes()` plus the chosen margin of 4,732,928 bytes per scripted document, which `story:scripts-parse-run` measures, and the agent dropped at the end of the document's parse; starting values without bindings or DOM, which `story:scripts-parse-run` measures again.

## Q6. Seeds, clock and time zone

| hook | HOST ids | in a session | in a test |
|---|---|---|---|
| `host_system_utc_epoch_nanoseconds` (`crates/swell/src/host/hooks.rs:308 @ 0.8.0`) | HOST-125 (what `Date.now()` returns), HOST-126 (the clamp to ±8.64 × 10^21) | the system clock: nanoseconds since the Unix epoch of `SystemTime::now()`, so it advances between two calls | a fixed start that advances by 1,000,000 ns (1 ms) on each call, so that a busy wait on `Date.now()` ends (epic, "Answers of the script engine's session", `testharness.js` lines 1257 and 1260) |
| `system_time_zone_identifier` (`hooks.rs:312-314 @ 0.8.0`) | HOST-127 | the provided default, `"UTC"`: a session reveals no time zone of its host | the same |
| `get_named_time_zone_offset_nanoseconds` (`hooks.rs:318-324 @ 0.8.0`) | HOST-128 | the provided default, 0 | the same |
| `Seeds::hash_key`, `Seeds::random_seed` (`crates/swell/src/host/limits.rs:100-106 @ 0.8.0`; `Math.random` and hash order) | HOST-26, HOST-27, HOST-144, HOST-147 (`random_seed` and its uses, pending slice 4); HOST-145 (`hash_key` keys every hash map of the engine, `test`, `conformance/host/requirements.txt:145 @ 0.8.0`; `docs/design/host-interface.md:359 @ 0.8.0`, slice 1) | 48 bytes drawn per agent from `/dev/urandom` through `surf_secret::fill_random` (`crates/secret/src/lib.rs:32`), the source the session's token comes from | fixed: `hash_key` `[7; 16]` and `random_seed` `[11; 32]`, swell's own test values (`crates/swell/tests/support/host.rs:103 @ 0.8.0`, `seeds()`) |

The story names HOST-126 to HOST-128 as the time-zone hooks; at the tag HOST-126 is the clamp of
the clock's answer and HOST-127 and HOST-128 are the time zone
(`docs/design/host-interface.md:1933-1939 @ 0.8.0`).

The eight ids of the probe below are `pending slice 4` at the tag, and `Date` and all of `Math`
but `max` do not exist in 0.8.0 (`CHANGELOG.md:112-113 @ 0.8.0`): the clock, the time zone and
`random_seed` are written now, because the trait requires the clock (`hooks.rs:305-308 @
0.8.0`) and `Agent::new` takes both seeds, and a script first reaches them once surf takes a tag
with slice 4. `hash_key` is in use at 0.8.0 already: every hash map of the engine is keyed by it
(HOST-145, held by a test), so the order of its maps differs from session to session and is
fixed in a test.

probe: (workstation) git -C <swell clone> show 0.8.0:conformance/host/requirements.txt | awk '$1 ~ /^HOST-(26|27|125|126|127|128|144|147)$/' -> HOST-26 pending slice 4 (and the seven others, each `pending slice 4`)

Settled: the table above and the `probe:` line, which reads the eight HOST ids of the clock, the time zone and `random_seed` at the tag `0.8.0` of Q13 as `pending slice 4`; HOST-145 (`hash_key`) is held by a test at the tag.

## Q7. The specification of a script's end and of the option

The answer of `decision-blocker:script-end-record`, one line per item of its "The undecided
relation":

cardinality: many per document; one record per script element the parser reached in a document parsed with scripts (one closed by the end of the input included, `not run: eof`), and one per script element a script inserted into it (`not run: inserted`); never a second record for one element; each with its position in the order the records were written, from 1.
owner: the document owns its records: a relation `scripts` of `surf.document.Document`, `owns`, many, via `document_id`, as `forms` is (`spec/domains/document.yaml:183`).
written: while the document is `Parsing`, by `surf.document.AddScriptEnd`, issued by the parser step after the parse and before `FinishParsing`, in the order the ends happened.
outlives its document: no; a record lives and goes with its document.
without scripts: no record; a document parsed without scripts records none of its script elements.

This confirms the answer the decomposer would take (blocker, "The answer the decomposer would
take"), with one addition: the records of script elements a script inserted.

The closed list of ends, every word of Q1's line `reasons:` and of Q4's column `outcome when
reached` among them:

- `ran`.
- `threw <kind> "<message>" <line>:<column>`: an uncaught throw, or a parse error that is not a
  refusal. `<kind>` is one of the seven `ErrorKind` variants (`Error`, `EvalError`, `RangeError`,
  `ReferenceError`, `SyntaxError`, `TypeError`, `URIError`;
  `crates/swell/src/host/mod.rs:39-48 @ 0.8.0`), or `value` when the thrown value is not an
  error object (`error_summary` answers `None`); the fields as Q16 bounds them. swell announces
  the variant `AggregateError` and `#[non_exhaustive]` on `ErrorKind` for the release that
  brings `Promise.any` (HOST-201, `CHANGELOG.md:87-95 @ 0.8.0`, `pending slice 10`): the tag
  change that brings it adds the word `AggregateError` to this list and to `ScriptErrorKind`,
  and a wildcard arm maps a kind surf does not know yet to `Error`.
- `refused "<message>" <line>:<column>`: the parse answered `Parsed::Errors` and the agent's
  `refusals()` rose (`crates/swell/src/host/agent.rs:266-275 @ 0.8.0`): a construct of a later
  delivery slice, whose `SyntaxError` message names the slice (`CHANGELOG.md:104-106 @ 0.8.0`).
- `limit: steps-per-script`, `limit: memory`, `limit: source-bytes`, `limit: wall-clock`,
  `limit: nodes`, `limit: depth`, `limit: attributes`, `limit: stored-bytes`.
- `faulted`: the entry ended `Terminated(EngineFault)`; the agent is faulted for good
  (`website/docs/embedding.md:65 @ 0.8.0`), and each later script of the document is `not run:
  faulted`.
- `not run: empty`, `not run: src`, `not run: type`, `not run: module`, `not run: importmap`,
  `not run: speculationrules`, `not run: disconnected`, `not run: nomodule`, `not run: event`,
  `not run: svg`, `not run: inserted`, `not run: eof` (Q1), and `not run: scripts-per-document`, `not run:
  steps-per-document`, `not run: wall-clock`, `not run: memory`, `not run: faulted` (Q4 and the
  fault).

The proposed YAML for `spec/domains/document.yaml` (types, entity, command, event, view):

```yaml
types:
  # Minted when the record is written, as a form's identity is.
  - name: surf.document.ScriptId
    kind: newtype
    of: Uuid
    naming:
      summary: The identity of the record of one script element's end.

  - name: surf.document.ScriptEnd
    kind: enum
    variants:
      - name: Ran
        summary: The script ran to its end.
      - name: Threw
        summary: The script threw and did not catch it, or its source did not parse.
      - name: Refused
        summary: swell refused a construct of a later delivery slice.
      - name: Limit
        summary: A limit ended the script; the reason names it.
      - name: Faulted
        summary: The engine found its own invariant broken.
      - name: NotRun
        summary: The script was not run; the reason says why.
    naming:
      summary: How one script element ended.

  # The words of `docs/design/scripts.md`, Q1's line `reasons:` and Q4's column `outcome when
  # reached`. Reached through `Limit` or `NotRun` only.
  - name: surf.document.ScriptReason
    kind: enum
    variants:
      - {name: Empty, summary: "empty"}
      - {name: Src, summary: "src"}
      - {name: Type, summary: "type"}
      - {name: Module, summary: "module"}
      - {name: Importmap, summary: "importmap"}
      - {name: Speculationrules, summary: "speculationrules"}
      - {name: Disconnected, summary: "disconnected"}
      - {name: Nomodule, summary: "nomodule"}
      - {name: Event, summary: "event"}
      - {name: Svg, summary: "svg"}
      - {name: Inserted, summary: "inserted"}
      - {name: Eof, summary: "eof"}
      - {name: StepsPerScript, summary: "steps-per-script"}
      - {name: Memory, summary: "memory"}
      - {name: SourceBytes, summary: "source-bytes"}
      - {name: WallClock, summary: "wall-clock"}
      - {name: Nodes, summary: "nodes"}
      - {name: Depth, summary: "depth"}
      - {name: Attributes, summary: "attributes"}
      - {name: StoredBytes, summary: "stored-bytes"}
      - {name: ScriptsPerDocument, summary: "scripts-per-document"}
      - {name: StepsPerDocument, summary: "steps-per-document"}
      - {name: Faulted, summary: "faulted"}
    naming:
      summary: Why a script was not run, or which limit ended it.

  - name: surf.document.ScriptErrorKind
    kind: enum
    variants:
      - {name: Error, summary: "Error"}
      - {name: EvalError, summary: "EvalError"}
      - {name: RangeError, summary: "RangeError"}
      - {name: ReferenceError, summary: "ReferenceError"}
      - {name: SyntaxError, summary: "SyntaxError"}
      - {name: TypeError, summary: "TypeError"}
      - {name: UriError, summary: "URIError"}
      - {name: Value, summary: "a thrown value that is no error object"}
    naming:
      summary: The kind of what a script threw.

entities:
  # One script element's end (docs/design/scripts.md, Q7). Written once, while its document is
  # `Parsing`, in the order the ends happened; never outlives its document.
  - name: surf.document.Script
    naming:
      summary: The end of one script element of a document.
    identity:
      name: script_id
      type: surf.document.ScriptId
    fields:
      - name: document_id
        type: surf.document.DocumentId
      - name: position
        type: Integer
      - name: end
        type: surf.document.ScriptEnd
      - name: reason
        type: Optional<surf.document.ScriptReason>
      - name: error_kind
        type: Optional<surf.document.ScriptErrorKind>
      # At most 1,024 characters (Q16); text from the page, untrusted.
      - name: message
        type: Optional<String>
      - name: line
        type: Optional<Integer>
      - name: column
        type: Optional<Integer>
      # The line and column count in `eval` or `Function` code (swell's `ErrorSite::dynamic`).
      - name: dynamic
        type: Optional<Boolean>
    lifecycle:
      initial: Added
      states:
        - name: Added
          summary: The record is a row of its document; it never changes.
      terminal: [Added]

  # added to surf.document.Document, relations:
      - name: scripts
        kind: owns
        target: surf.document.Script
        cardinality: many
        via: document_id

errors:
  - {name: surf.document.InvalidScriptEnd, fields: [{name: end, type: surf.document.ScriptEnd}]}
  - {name: surf.document.InvalidScriptPosition, fields: [{name: position, type: Integer}]}

commands:
  - name: surf.document.AddScriptEnd
    naming:
      wire: add-script-end
      display: Add a script's end
      summary: Record how one script element of a document being parsed ended.
    refs: ["whatwg-html:prepare-the-script-element", "whatwg-html:execute-the-script-element"]
    input:
      - {name: document_id, type: surf.document.DocumentId}
      - {name: position, type: Integer}
      - {name: end, type: surf.document.ScriptEnd}
      - {name: reason, type: Optional<surf.document.ScriptReason>}
      - {name: error_kind, type: Optional<surf.document.ScriptErrorKind>}
      - {name: message, type: Optional<String>}
      - {name: line, type: Optional<Integer>}
      - {name: column, type: Optional<Integer>}
      - {name: dynamic, type: Optional<Boolean>}
    outcomes:
      # The closed list above is the rule; Q16 bounds the error's fields.
      - name: invalid-end
        when:
          any:
            - all: [{any: [end == Ran, end == Faulted]}, {any: [defined(reason), defined(error_kind), defined(message), defined(line), defined(column), defined(dynamic)]}]
            - all: [{any: [end == Limit, end == NotRun]}, {any: [missing(reason), defined(error_kind), defined(message), defined(line), defined(column), defined(dynamic)]}]
            - all: [{any: [end == Threw, end == Refused]}, {any: [defined(reason), missing(error_kind)]}]
            - all: [end == Limit, defined(reason), reason != StepsPerScript, reason != Memory, reason != SourceBytes, reason != WallClock, reason != Nodes, reason != Depth, reason != Attributes, reason != StoredBytes]
            - all: [end == NotRun, defined(reason), {any: [reason == StepsPerScript, reason == SourceBytes, reason == Nodes, reason == Depth, reason == Attributes, reason == StoredBytes]}]
            - all: [end == Threw, defined(error_kind), error_kind == Value, defined(message)]
            - all: [end == Refused, defined(error_kind), error_kind != SyntaxError]
            - all: [defined(message), message.count > 1024]
        error: surf.document.InvalidScriptEnd
        payload: {surf.document.InvalidScriptEnd: {end: input.end}}
      - name: invalid-position
        when:
          any:
            - position < 1
            - all: [defined(line), line < 1]
            - all: [defined(line), line >= 4294967296]
            - all: [defined(column), column < 1]
            - all: [defined(column), column >= 4294967296]
        error: surf.document.InvalidScriptPosition
        payload: {surf.document.InvalidScriptPosition: {position: input.position}}
      # no-such-document and already-parsed as `AddForm` has them.
      - name: added
        creates: surf.document.Script
        instance: script_id
        sets: {document_id: input.document_id, position: input.position, end: input.end, reason: input.reason, error_kind: input.error_kind, message: input.message, line: input.line, column: input.column, dynamic: input.dynamic}
        emits: [surf.document.ScriptEndAdded]
        payload:
          surf.document.ScriptEndAdded:
            script_id: {generated: true}
            document_id: input.document_id
            position: input.position
            end: input.end
        summary: The script's end is a row of its document.

events:
  - name: surf.document.ScriptEndAdded
    fields:
      - {name: script_id, type: surf.document.ScriptId}
      - {name: document_id, type: surf.document.DocumentId}
      - {name: position, type: Integer}
      - {name: end, type: surf.document.ScriptEnd}

views:
  - name: surf.document.Scripts
    source: surf.document.Script
    consistency: read_your_writes
    params:
      - {name: document_id, type: surf.document.DocumentId}
    filter: document_id == param.document_id
    order_by: [position asc]
    fields:
      - {name: script_id, type: surf.document.ScriptId}
      - {name: document_id, type: surf.document.DocumentId}
      - {name: position, type: Integer}
      - {name: end, type: surf.document.ScriptEnd}
      - {name: reason, type: Optional<surf.document.ScriptReason>}
      - {name: error_kind, type: Optional<surf.document.ScriptErrorKind>}
      - {name: message, type: Optional<String>}
      - {name: line, type: Optional<Integer>}
      - {name: column, type: Optional<Integer>}
      - {name: dynamic, type: Optional<Boolean>}
    naming:
      wire: scripts
      display: Scripts
```

The view `surf.document.Scripts` reads every stored field of a document's records, in the order
the scripts ended, as `surf.document.Elements` reads its elements: each entity of the domain has a
view, and without one no scenario sees the record.

The command refuses what this list and Q16 exclude, and adds nothing then: `invalid-end`, an end
with a reason or an error the list does not give it (`Ran` and `Faulted` carry nothing, `Limit` a
limit's word, `NotRun` a not-run word, `Threw` a kind and, for `value`, no message, `Refused` its
`SyntaxError`; only `Threw` and `Refused` carry a kind, a message, a line, a column or `dynamic`;
no message over 1,024 characters); `invalid-position`, a position below 1 or a line or column
outside 1 to 2^32 - 1, whose error names the end's position whichever of the three was outside.
The input's refusals answer before the document's existence and its state.
A second end at one position of one document is not refused by the specification; the session
writes one end per position, and the refusal comes back when the specification language
synthesizes it within the suite's time.

The command is the document component's, issued by its parser step in the session process
(`surf-session`), as `AddElement` and `AddForm` are; no other process runs page script (S7). The
event carries no page text, so no message reaches the live view's stream.

The marker "no component for script execution, storage (cookies, cache, history) or input events"
(`spec/components.yaml:429-430`) loses "script execution": a page's scripts run inside the
document component's parser step, and their one command is `surf.document.AddScriptEnd`. The new
text: "UNMAPPED: no component for storage (cookies, cache, history) or input events. None is
declared until its first command is. Script execution is part of the document component's parser
step (`docs/design/scripts.md`, Q7)."

The option, proposed for `spec/domains/agent.yaml` and `cli/surf-agent.yaml`:

```yaml
  # surf.agent.OpenCall, fields:
      # Whether the session runs its pages' inline classic scripts (`docs/design/scripts.md`);
      # absent is false. Only `open` takes it (S3, S7).
      - name: scripts
        type: Optional<Boolean>

  # surf.agent.OpenSession, input; and surf.agent.Session, fields, set by `opened`:
      - name: scripts
        type: Boolean

  # cli/surf-agent.yaml, path [open], arguments:
      - field: scripts
        source: {kind: option, long: scripts}
```

Under `ess-cli/2` each input field has exactly one argument mapping and "Boolean values use
explicit `true`/`false`" (ESS `docs/design/cli-presentation-binding.md:176-177` at ess `0.97.4`,
the pin of `spec/ess-inputs.yaml`), so the spelling is `surf-agent open --scripts true <url>`;
`--scripts false` and no option are the default. A bare `--scripts` is not a form the binding
takes. The option reaches the session as `arg anchors` does: the client sends `arg scripts
"true"` in the `open` request on the socket (`crates/agent/src/wire.rs:32-45`), `surfd` passes the
request on unchanged (`crates/surfd/src/daemon.rs:1337-1340`) after reading the argument from the
decoded request (`daemon.rs:1045-1051`, for Q11), and the session takes it from there
(`crates/session/src/process.rs:18-20`), never from its arguments or environment.
`surf.agent.Session` records it (`scripts: Boolean`), one value per session, set by `open` and
ending with it.

What an answer prints: in a session with the option, after the `page` line, one line

```
scripts <n> ran <a> threw <b> refused <c> limit <d> faulted <e> not-run <f>
```

for the current document, then at most 10 lines, one per end that is not `ran`, in position
order: `  <position> threw <kind> "<message>" <line>:<column>`, `  <position> refused "<message>"
<line>:<column>`, `  <position> limit <word>`, `  <position> faulted`, `  <position> not-run
<word>`, with `+ scripts more <m>` when more remain, as the `changed` block bounds its lines
(`docs/agent-answer.md:325-326`). A message is page text, untrusted: quoted and cut at 120
characters with `surf_answer::quote_cut`; `<line>:<column>` is written `eval <line>:<column>`
when `dynamic` is true. A session without the option prints no `scripts` line.
`story:scripts-document-port` writes the line into `docs/agent-answer.md`.

Settled: the five lines `cardinality:`, `owner:`, `written:`, `outlives its document:` and `without scripts:` (the answer of `decision-blocker:script-end-record`), the closed list of ends, the proposed YAML, the command `AddScriptEnd` and the marker, the option `--scripts true` with `arg scripts`, and the `scripts` line of an answer.

## Q10. The session's environment

On `29945f5`, `surfd` started `surf-session` with its own environment less `SURF_TOKEN` and
`SURF_SESSION`, plus `Config::session_env`; it now starts it with that variable list alone
(`crates/surfd/src/daemon.rs:1392-1408`, `:103`).

probe: (workstation) git grep -n 'env::var\|var_os' -- 'crates/*/src/*.rs' 'crates/*/src/**/*.rs' -> crates/surfd/src/main.rs:36:    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")

The probe on `29945f5` prints eight lines: `crates/agent-tasks/src/process.rs:157`, `report.rs:89`
and `:102` (test tooling), `crates/agent/src/cli.rs:344-346` (the client: `XDG_RUNTIME_DIR`,
`SURF_SESSION`, `SURF_TOKEN`), `crates/surfd/src/main.rs:36` (the daemon) and
`crates/view-fixture/src/main.rs:348` (a fixture). None is in `crates/session`, `crates/surf`,
`crates/fetch`, `crates/tls`, `crates/html`, `crates/page`, `crates/cookie` or another crate the
session links but the client's `cli.rs`, which the session does not call (it uses
`surf_agent::wire`).

The allow-list is empty: `surfd` starts the session with `Command::env_clear()`, then adds
`Config::session_env`, which the binary leaves empty (`daemon.rs:151`). Each variable the
standard library or the C library reads:

- `RUST_MIN_STACK`: read by the standard library for the stack of a thread made without a size.
  Dropped: the session's threads keep the default, and the scripted parse names its stack (Q5).
- `RUST_BACKTRACE`: read on a panic. Dropped: the session's standard error is closed
  (`Stdio::null()`, `daemon.rs:1406`), so a backtrace goes nowhere.
- `LOCALDOMAIN`, `RES_OPTIONS`, `HOSTALIASES`: read by the C library's resolver, which
  `surf-fetch` reaches through `std::net::ToSocketAddrs` (`crates/fetch/src/lib.rs:78-80`).
  Dropped: a caller's variable would steer the session's name resolution; the session resolves
  with the system's `/etc/resolv.conf` and `/etc/hosts`.
- `TZ`: read by the C library's local time. Dropped: no code of the session formats local time,
  and the script clock answers UTC with an offset of 0 (Q6).
- Every other variable (`HOME`, `PATH`, `LANG`, `XDG_RUNTIME_DIR` among them): no code of the
  session reads one (the probe), the session starts no program, and `setpriv` and `prlimit` are
  started by absolute path (Q11).

The tests that start `surfd` in process still add theirs: `Config::session_env` is applied
after `env_clear()`, so `crates/session/tests/common/mod.rs:545` (`XDG_RUNTIME_DIR`),
`crates/surfd/tests/common/mod.rs:322` (`STAND_IN_DIR`) and `crates/view-fixture/src/main.rs:156`
(`XDG_RUNTIME_DIR`) keep working. The tests that run the built `surfd`, whose
`Config::session_env` is empty, found the session by `XDG_RUNTIME_DIR` in its environment; they
now find it by its socket path argument, a path under the case's runtime directory
(`processes_among`, `crates/session/tests/common/mod.rs:127`), and the stand-in of
`crates/surfd/tests` sets `STAND_IN_DIR` on itself when it has none.

Settled: the empty allow-list and the decisions on the six variables, from the `probe:` line above run on `29945f5`.

## Q11. S5 and the bounds of the operating system

S5 was decided by the operator session on 2026-11-24, about 11:00 CET, option a, quoted in the
epic's "Decisions taken": "S5 = A (`setpriv --no-new-privs` and `prlimit`, a session with scripts
is refused when either is missing; `forbid(unsafe_code)` stays). Reason: it meets the review's
second condition with no rule change. Record in the design note that seccomp and Landlock are
absent and that B needs the owner's word."

So, under option a: no seccomp filter, no Landlock rule set, no namespaces and no
`PR_SET_DUMPABLE`; `#![forbid(unsafe_code)]` stays in every shipped crate
(`crates/session/src/main.rs:4`, `crates/surfd/src/main.rs:10`). Option b, `unsafe` in one
audited module that calls `setrlimit` and `prctl`, needs the owner's word.

What stays open under option a. A session whose page script leaves the language (a defect of
swell or of surf's bindings that lets a page run code of its choosing in the session process)
still runs as the user, with the bounds below and no new privileges. It can read and write every
file the user can; connect to 127.0.0.1, to the local network and to every socket of the user,
those of `surfd` and of the user's other sessions included; and send a signal to every process of
the user. Whether it can trace another process, or be traced and read, is the kernel's ptrace
policy: on the workstation on 2026-11-24 Yama's `/proc/sys/kernel/yama/ptrace_scope` read `1`
(a process may trace its descendants only, and the session starts none); the build node's and
the forge's values are not known. Because `PR_SET_DUMPABLE` is not cleared, a process allowed to trace the session (under
Yama 1 its ancestors, `surfd` and what started it; under 0 every process of the user) can read
its memory, its token and its pages included. Closing any of these needs seccomp, Landlock,
namespaces or `prctl`, which is option b and the owner's word.

The two programs, by absolute path: `/usr/bin/setpriv` and `/usr/bin/prlimit`, both of util-linux.

probe: (build node, inside .out/probes/session-bounds.sh) command -v setpriv prlimit; setpriv --version; prlimit --version -> programs /usr/bin/setpriv /usr/bin/prlimit; setpriv from util-linux 2.42.4; prlimit from util-linux 2.42.4

The forge's runner: the check workflow, which runs the tests, runs on `ubuntu-latest`
(`.github/workflows/check.yml:22`), which `actions/runner-images` maps to Ubuntu 26.04 (its
`README.md:23`); the image's description, `images/ubuntu/Ubuntu2604-Readme.md` (OS 26.04.1 LTS,
image 20261116.202.1, line 10), lists selected apt packages from line 243 and does not name
util-linux. Ubuntu's package page of `util-linux` in `resolute` (26.04 LTS), version
2.41.3-3ubuntu2.2, marks it `essential`, and its file list holds `/usr/bin/prlimit` and
`/usr/bin/setpriv`. `story:session-process-hardening` checks `command -v setpriv prlimit` in its
first forge run.

Both programs replace themselves by `exec`, so the process `surfd` holds is the session's:

probe: (build node, inside .out/probes/session-bounds.sh) setpriv --no-new-privs prlimit --nofile=64:64 --core=0:0 --cpu=5:5 --as=1073741824:1073741824 -- sleep 3 & (the pid of `$!` read after 1 s) -> exec pid <the pid of $!> comm sleep NoNewPrivs: 1

Today's session without scripts, measured on the build node on 2026-11-24 (the first round ran
the script as a pool job; by the folder's rules a script that measures starts without `slot`, as
`sh .out/probes/session-bounds.sh`), started by `surfd`
with a private `XDG_RUNTIME_DIR`, on the pages of `crates/agent-tasks/sites` served by
`agent-tasks-serve` on 127.0.0.1 (`/article/article.html`, `/link/index.html`,
`/table/prices.html`, `/search/search.html`, `/login/login.html`,
`/search/results?q=lighthouse`), in the test profile:

probe: (build node) slot --timeout 20m sh .out/probes/session-bounds.sh -> session after /search/results?q=lighthouse: VmPeak: 208736 kB;VmSize: 143460 kB;VmHWM: 6260 kB;VmRSS: 6260 kB;Threads: 3;NoNewPrivs: 0; fds 8 cpu_ticks 1

Right after `open`, the session read `VmHWM: 5608 kB`, 6 descriptors and 3 threads; after the six
pages `VmHWM` was 6,260 kB, the descriptors 8, the CPU time 1 tick of 100 per second. Its limits
were the build node's: no CPU, address-space or core limit, 65,536 open files, no process limit.
The same `surf-session` started by hand under both programs answered `ready`, read `NoNewPrivs:
1` and showed each limit in `/proc/<pid>/limits`, and ended with status 0.

How a process ends at each bound, as `surfd` sees it when it reaps the session (the standard
error, where Rust writes its line, is closed by `surfd`, `crates/surfd/src/daemon.rs:1406`):

probe: (workstation) sh .out/probes/session-ends.sh -> cpu soft 1 hard 1, spin: exit 137 signal KILL; cpu soft 1 hard 2, spin: exit 152 signal XCPU; as 268435456, alloc: exit 134 signal ABRT; stderr: memory allocation of 33554432 bytes failed; nofile 16, files: exit 0; stdout: files: open number 14 failed: Too many open files (os error 24)

With soft and hard equal the kernel sends `SIGKILL` at the CPU bound; with the soft one below
the hard one it sends `SIGXCPU` at the soft one, which the session does not handle, so it ends
there. So the CPU bound is written `--cpu=<n>:<n+1>`. The open-files bound does not end the
session; the real `surf-session`, started by `surfd` through a wrapper that bounds its
descriptors, answered loads it could not open a socket for as unreachable and stayed open:

probe: (workstation) NOFILES="8 10 64" LINES_SHOWN=8 sh .out/probes/session-ends.sh -> nofile 10: go /link/index.html exit 0, session fds after 8: ... did go "http://127.0.0.1:<port>/link/index.html"|page failed Unreachable "http://127.0.0.1:<port>/link/index.html"|...

At 64 the same four pages loaded (`page 200` each) with 8 descriptors open after each `go`; at 8,
the number the session holds between requests, every `go` answered ``surf-agent: the session's
system stopped: unmet obligation: context answer `surf.net.RequestId` ...`` with exit status 3
(a defect of that answer, outside this note: it names no bound); at 7 `open` itself failed. A
stack that overflows ends in `SIGABRT` (Q5). Core files: the workstation's core handler stored
the cores of two early probe runs made without `--core=0:0` and none of the nine later runs made
with it (`coredumpctl list`: `COREFILE present`, then `none`).

What a session holds over its life, the terms of the address-space bound:

- the session process without scripts after `open`: `VmPeak` 208,736 kB, measured above;
- every document's node tree and rows, for the life of the session: `surf::Browser` keeps the
  tree of each parsed document ("its node tree, which `surf::Browser` keeps for the life of the
  session (each tree within `surf-html`'s limits; no bound over all documents yet)",
  `crates/session/src/lib.rs:351-353`). One tree is at most about 536 MiB of heap at the parser's
  default limits (`crates/html/src/limits.rs:45-51`, "inferred from the bound, not measured"); the
  number of trees has no bound until `story:session-memory-bound` is built. The records of Q7 add
  at most 1,000 per document, each message at most 1,024 characters (Q16);
- the script engine's state of a document: only while that document is parsed. The agent is made
  at the document's first script and dropped at the end of its parse, on the parse's thread
  (Q5); a document the session left holds no realm and no heap of the engine, only its tree and
  rows;
- while a scripted document is parsed, one at a time (the request thread waits): the thread's
  stack, 8,739,584 bytes (Q5); the C library's malloc arena of that thread, which glibc reserves
  as up to 64 MiB of address space on x86_64 (the review's figure, from knowledge, not measured
  here; the probe `swell-limits` went from `VmPeak: 5840 kB` before its script thread to `VmPeak:
  212024 kB` at its end, with several agents and four short threads of its own); `memory_bytes`,
  64 MiB, which holds the agent's heap, the wrappers and the charge of the nodes a script makes
  (Q4);
- what swell holds without a charge: swell lists as open that "`string_units` and
  `string_units_in` copy without a memory charge" and that "The `call` and `new` instructions
  copy their argument list uncharged" (`docs/plan/second-batch.md:284-285 @ 0.8.0`), and that
  "The code of the last error site's script stays held without a charge until the next entry"
  (`:290 @ 0.8.0`). These copies are not bounded by `memory_bytes`, and the address-space
  backstop is their only bound. Requirement 11 (Q18) asks swell to charge every copy that is
  alive while the engine calls into script. The measurement and the requirement's detail were
  handed to swell's session on 2026-11-24 and come back into this note with the tag that closes
  it.

So the sum has no finite value today: the trees of all documents and swell's uncharged copies are
not bounded by anything of the session. The address-space bound of a scripted session is
therefore a chosen backstop, 4 GiB (4,294,967,296 bytes). It is the bound a session started by a
test of the stories before `story:scripts-document-port` runs under; those stories build and test
on swell `0.8.0` (Q13). After how much use it is reached by honest pages: the room for trees is 4
GiB less the session (208,736 kB), the thread (8,739,584 bytes), its arena (64 MiB) and
`memory_bytes` (64 MiB), about 3,750 MiB: about 7 documents whose trees reach the parser's limits
(536 MiB each, inferred), or about 35,000 documents of the size of the agent tasks' pages, at the
109 kB of `VmHWM` each of the six pages above added (652 kB over six pages; resident memory, not
address space, so an estimate). The agent then reads the session's end with `note ended: signal
SIGABRT`.

The backstop is real memory of the host, not a reservation: while requirement 11 is open, one
page in one document can make a session hold it. `surfd` on `main` has no bound on the number of
open sessions: it counts the session processes not yet reaped (`live`, `crates/surfd/src/daemon.rs:358-359`,
`:1450`) only to wait for them when it stops (`:575`). So k scripted sessions on hostile pages can hold k × 4 GiB of
the host at once. A bound on the number of scripted sessions is a question for
`story:scripts-document-port`.

The value `story:scripts-document-port` ships with: it takes the swell tag that holds requirement
11 (Q13), so swell's copies are charged to `memory_bytes`. With `story:session-memory-bound`
keeping the tree of the current document only, the sum for one tab is 208,736 kB + 2 × 536 MiB
(the current tree and the one being parsed) + 8,739,584 bytes + 64 MiB (the arena) + 64 MiB
(`memory_bytes`) + 64 MiB (an allowance, chosen, for what swell lists as held without a charge
beside the copies, `docs/plan/second-batch.md:290-291 @ 0.8.0`), about 1,480 MiB; 2 GiB
(2,147,483,648 bytes) holds it, and `story:scripts-document-port` ships with `--as=2147483648`.
The sum holds for one tab only: `story:session-memory-bound` keeps "the tree of the current
document of each tab", so each further tab adds a tree of up to 536 MiB, and a session with tabs
needs the bound raised by that much per tab or a bound over all tabs. If
`story:session-memory-bound` is not built before `story:scripts-document-port`, that story keeps
the 4 GiB backstop and says so.

What `story:session-memory-bound` must give for the sum to hold: a stated number of kept trees
per session (and the rows of their documents, Q7's records among them) that counts the document
being parsed, and the statement that a document the session left holds no state of the script
engine (Q5 gives it: the agent is dropped at the end of the parse).

How long a command may run: `surfd` waits 60 s for a session's answer and then answers `did
refused session-timeout` (`SESSION_ANSWER_TIMEOUT`, `crates/surfd/src/daemon.rs:34`); it goes on
reading a late answer for 600 s and then kills the session with `SIGKILL`
(`LATE_ANSWER_TIMEOUT`, `daemon.rs:36-39`; `Process::kill`, `:424-432`). That is the first bound
on one command, 660 s of wall time at most, and inside it Q4's 5 s of wall clock per document
bounds the scripts of a document. The CPU bound is the backstop over a session's whole life:
3,600 s soft and 3,601 s hard, chosen: 720 documents whose scripts each run to the 5 s wall clock
of Q4 (a script runs on one thread, so at most about 5 CPU seconds each), while the six pages
measured above took 1 tick (0.01 s) together. At the bound the agent reads `note ended: signal
SIGXCPU bound cpu`.

| bound | sessions | rule | starting value | the session's end at the bound, as `surfd` sees it | measured again by |
|---|---|---|---|---|---|
| address space (`--as`) | with scripts only | the terms above; no finite sum until `story:session-memory-bound` and requirement 11 are closed | 4,294,967,296 (4 GiB), chosen as a backstop for the stories before `story:scripts-document-port`; reached after the use given above; that story ships with 2,147,483,648 (2 GiB), the sum above | `SIGABRT` (the probe's `as 268435456, alloc: exit 134 signal ABRT`) | `story:scripts-document-port` |
| CPU seconds (`--cpu`) | with scripts only | the CPU time of a session's whole life, behind `surfd`'s 660 s per command and Q4's 5 s per document | 3,600 soft, 3,601 hard, chosen: 720 documents at Q4's wall clock | `SIGXCPU` at the soft value (`cpu soft 1 hard 2, spin: exit 152 signal XCPU`) | `story:scripts-document-port` |
| open files (`--nofile`) | with scripts only | eight times the descriptors the session held after each page; not for sessions without scripts, because no measure covers every test and agent task (at 10 the second page failed though 8 were open after the first) | 64, chosen (the probe: 8 after each page; four pages loaded at 64) | no end: a load answers `page failed Unreachable`; at 8, the descriptors the session holds between requests, `go` answers `the session's system stopped` with exit status 3; at 7 `open` fails | `story:scripts-document-port` |
| core size (`--core`) | every session | no core file: a core of a session holds its token and pages | 0 | no end; the workstation's core handler stored no core (`COREFILE none`) | `story:session-process-hardening` (a test reads it) |
| no new privileges (`setpriv --no-new-privs`) | every session | it cannot end a session that works today | `NoNewPrivs: 1` | no end | `story:session-process-hardening` (a test reads it) |
| processes (`RLIMIT_NPROC`) | none | not set: it counts every process of the user, not the session's own, and a value below the user's count would stop the session from starting a thread; `ulimit -u` was `unlimited` on the build node | not set | not set | `story:scripts-document-port`, if a later review asks for one |

The start, in `Config` with these shipped values (a test can name others), in two forms:

- a session without scripts: `/usr/bin/setpriv --no-new-privs /usr/bin/prlimit --core=0:0 --
  <session program> <identity> <socket>`;
- a session with scripts: `/usr/bin/setpriv --no-new-privs /usr/bin/prlimit
  --as=4294967296:4294967296 --cpu=3600:3601 --nofile=64:64 --core=0:0 -- <session program>
  <identity> <socket>`.

Which sessions: the decision on S5 binds a session with scripts, and only such a session gets the
address-space, CPU and open-files bounds. A session without scripts gets what cannot end a session
that works today: no new privileges and no core file. When either program is missing at `surfd`'s
start, a session without scripts starts as today, without either program, and an `open` that
asks for scripts is refused. `surfd` learns the option from the `open` request it already decodes
(`route`, `crates/surfd/src/daemon.rs:1045-1051`), reading `arg scripts` before it starts the
process; the request is still passed on unchanged.

The answers, each line literal:

- `open` with `--scripts true` while a program is missing, or when the start through both ended
  by an exit before the session wrote its first byte: the action line `did refused
  scripts-unavailable`, exit status 3 (like `session-died`, the cause is outside the request), and
  a new row of the table in `docs/agent-answer.md` ("`setpriv` or `prlimit` is missing, or the
  start through them (`setpriv`, `prlimit`, the session program) ended by an exit, not a
  signal, before the session wrote its first byte, and a session with scripts runs only under
  both").
- A session `surfd` started with the CPU bound (a session with scripts) that ended by
  `SIGXCPU`, the signal of that bound: the head reads `died`, the action `did refused
  session-died`, as today, and the answer gains the note line `note ended: signal SIGXCPU bound
  cpu` (the form `note ended: signal <NAME> bound <word>`). The bound's word is there when
  `surfd` set that bound for the session and the signal is the one the bound sends, and never
  otherwise: a session without the CPU bound that ended by `SIGXCPU` reads `note ended: signal
  SIGXCPU`, and `surfd` sets no file-size bound, so no line names one. The line cannot tell who
  sent the signal.
- A session that ended by another signal (an allocation over the address space or a stack
  overflow, both `SIGABRT`): the same head and action, and the note line `note ended: signal
  SIGABRT` (the form `note ended: signal <NAME>`). A session ended by a signal `surfd` did not
  send, `SIGKILL` from another process included (the kernel's out-of-memory killer sends it),
  answers `note ended: signal SIGKILL`.
- A session that `surfd` killed itself (the `killed` flag of its process, set by `Process::kill`,
  `crates/surfd/src/daemon.rs:420`, `:430`; after the late read of 600 s, at its stop, or
  after any other kill the `note ended` item of `docs/agent-answer.md` lists) keeps today's
  answer: after the late read the command that waited was answered `did refused
  session-timeout` after 60 s (`docs/agent-answer.md:178`), and a later command reads the head
  `died` and the action `did refused session-died` (`daemon.rs:1690-1692`: a killed process is
  listed `died`; `:1020`). No `note ended` line is written for it, unless the session had
  already ended by a signal `surfd` did not send when `surfd` came to kill it.
- A session that ended with a status and no signal: no `note ended` line.

The live view lists the session as died with the signal. `story:session-process-hardening`
builds it.

Settled: S5 = a as decided on 2026-11-24 and quoted above, with what stays open under it; the two programs by absolute path; the table of bounds, which sessions get each, their starting values (chosen, with the use after which each is reached; the 4 GiB backstop for the stories before `story:scripts-document-port`, 2 GiB from it on), that the backstop is real memory of the host and that `surfd` bounds no number of sessions and the end at each bound from the `probe:` lines above; the two forms of the start; the literal answer lines; the forge's image is cited (Ubuntu 26.04, `util-linux` essential with both programs), and `story:session-process-hardening` checks it in its first forge run.

## Q13. The tag of swell

The tag `story:scripts-engine-dependency` pins: `0.8.0`, commit
`0c1827128b57845e203713e87a0b0949fc8d63be`, released 2026-11-24, the newest tag on that day
(`0.8.0`, `0.7.0`, `0.6.0` by date). Its root `Cargo.lock` lists 8 packages and no `source`
line: `swell`, `swell-compiler`, `swell-num`, `swell-regexp`, `swell-syntax`, `swell-unicode`,
`swell-test262`, `swell-unicode-gen` (`Cargo.lock` @ 0.8.0). A host depends on `swell` alone,
which brings the other five of the first six by path (`crates/swell/Cargo.toml:12-17 @ 0.8.0`);
`swell-test262` and `swell-unicode-gen` are tooling. The dependency line is the one swell's
changelog gives: `swell = { git = "https://github.com/beyond10x/swell", tag = "0.8.0" }`
(`CHANGELOG.md:135-137 @ 0.8.0`).

probe: (workstation) git -C <swell clone> rev-parse '0.8.0^{commit}'; git show 0.8.0:Cargo.lock | grep -c '^\[\[package\]\]'; git show 0.8.0:Cargo.lock | grep -c '^source'; git merge-base --is-ancestor 84522ad 0.8.0 -> 0c1827128b57845e203713e87a0b0949fc8d63be; lock packages: 8 source lines: 0; 84522ad is an ancestor of 0.8.0

The two conditions of "From the security review of 2026-11-22" (epic, that section), each read at
the tag:

1. Parsing and compilation charged to the memory budget before they allocate: held. The ledger of
   the frontend: "the lexer, the parser, the early errors, the scope analysis and the compile step
   ... charge every growth of their structures before it is made, credit what they drop, and read
   the host's interrupt" (`crates/syntax/src/ledger.rs:1-7 @ 0.8.0`); "Charge `bytes` before the
   growth they pay for, or refuse and charge nothing" (`:39 @ 0.8.0`). The changelog of 0.4.0:
   "The parser's tree, the scope analysis, the compile step's scratch and the code are charged to
   `memory_bytes` before they grow" (`CHANGELOG.md:437-443 @ 0.8.0`, in
   the section of 0.4.0). HOST-168 and HOST-169 have a test (`conformance/host/requirements.txt:168-169 @
   0.8.0`), and so do `eval` and `Function` (`crates/swell/tests/frontend_ledger.rs:127 @ 0.8.0`).
   Exceptions swell lists as open (`docs/plan/second-batch.md:280-293 @ 0.8.0`, "Findings left
   open"): "Compile scratch is credited at the end of the step; the errors of a failed parse are
   uncharged" (`:291 @ 0.8.0`); "A statement node is 136 bytes for an empty block" (`:288 @
   0.8.0`); "The code of the last error site's script stays held without a charge until the next
   entry" (`:290 @ 0.8.0`).
2. Property operations charge their work and temporary storage before they do it: held for the
   parts the review names, enumeration of string and object keys and lookup by a String key. A
   key list is charged, steps and bytes, before it is built, and the `for`-`in` list before it
   grows (`CHANGELOG.md:444-449 @ 0.8.0`; `docs/design/engine.md:765-780 @ 0.8.0`); comparing
   two Strings and a String key longer than 32 code units charge a step per code unit past 32
   before the work (`CHANGELOG.md:454-455 @ 0.8.0`), HOST-181 and HOST-182 with a test
   (`conformance/host/requirements.txt:181-182 @ 0.8.0`). A lookup through the prototype chain
   charges a step per object (`crates/swell/src/object/ordinary.rs:330 @ 0.8.0`, `:350 @ 0.8.0`)
   and has no test of its own. Exceptions swell lists as open: "The `call` and `new`
   instructions copy their argument list uncharged" and "`string_units` and `string_units_in`
   copy without a memory charge" (`docs/plan/second-batch.md:284-285 @ 0.8.0`; a story after
   slice 6). Two texts at the tag still say the property scan is uncharged and are out of date:
   `CHANGELOG.md:768-769 @ 0.8.0` (the 0.2.0 section) and `website/src/pages/index.tsx:159 @
   0.8.0`; the 0.4.0 section says that scan "is gone" (`CHANGELOG.md:612-613 @ 0.8.0`).

What `0.8.0` leaves open for page scripts, beside those exceptions: the override table (HOST-112,
HOST-114, `pending slice 9`, `conformance/host/requirements.txt:112 @ 0.8.0`, `:114 @ 0.8.0`), so
live collections and `WindowProxy` stay deviations; promises, jobs and the microtask checkpoint
(slice 10, HOST-192 to HOST-200); `Date` and `Math.random` (slice 4, Q6); and of what came after
ES5 everything but generators and the sixth slice (`CHANGELOG.md:98-113 @ 0.8.0`).

A `[SameObject]` collection (`childNodes`, `children`) is one snapshot object per node, refreshed
when its getter is read after a write to a child list: all or nothing for a step the object
refuses; an end of the run inside a refill ends the document's scripts. When the script froze or
sealed it, made it non-extensible or made one of its indices non-configurable so that the
refresh cannot complete, the object stays the old snapshot, whole, and the getter throws a
`TypeError`, where a browser refuses that change on the collection itself. When the memory
limit ends the run after the refill's first define, nothing is put back (that would need the
Strings and wrappers the limit refused): the script ends `limit: memory` and each later script
of the document `not run: memory`, so no script reads the object again. An `HTMLCollection`
has no named properties (`collection["id"]` is `undefined`, `namedItem` answers), a deviation
until a later story builds them.

The sibling members find a node's index among its parent's children through a hint of two
positions for each of the last eight parents; three readers on one parent miss it and scan at
each read (measured on the reading slice: 500 children read by three cursors in turn cost
180,250 steps, and 5,000 children end at the step limit), which is charged and never answers a
wrong sibling; a later story decides the tree's own sibling positions.

Of those exceptions, swell lists as open that "`string_units` and `string_units_in` copy without
a memory charge" and that "The `call` and `new` instructions copy their argument list uncharged"
(`docs/plan/second-batch.md:284-285 @ 0.8.0`). These copies are not bounded by `memory_bytes`;
in a session the address-space backstop of Q11 is their only bound, and a session that reaches it
ends in `SIGABRT`, not in `limit: memory`. Requirement 11 (Q18) asks swell to charge every copy
that is alive while the engine calls into script. The measurement and the requirement's detail
were handed to swell's session on 2026-11-24 and come back into this note with the tag that
closes it. surf's limits are not lowered for it.

Decided by the coordinating agent on 2026-11-24, reversible: `story:scripts-engine-dependency`
pins swell `0.8.0`, and every story before `story:scripts-document-port` builds and tests on it,
its sessions under the 4 GiB backstop of Q11. `story:scripts-document-port`, the story that lets a
page's scripts run in a session, waits for the swell tag that holds requirement 11 and takes that
tag in its first commit. The reason: until then the only bound on the copies swell does not charge
is the address-space backstop.

Settled: decided on 2026-11-24 (reversible): tag `0.8.0`, commit `0c1827128b57845e203713e87a0b0949fc8d63be`, 8 packages in its root `Cargo.lock` with no `source` line, six taken by a host, pinned by `story:scripts-engine-dependency` for every story before `story:scripts-document-port`, which waits for the swell tag that holds requirement 11 and takes it in its first commit; both conditions held with the exceptions swell lists (`docs/plan/second-batch.md:284-291 @ 0.8.0`); open for later tags the override table, slice 10, `Date` and `Math.random`; the `probe:` line above gives the `git` commands.

## Q14. Where swell is linked

The crate graph of the first version:

- `swell` (and its five crates) ← `surf-script` (`crates/script`, new): the one crate of this
  repository that depends on swell.
- `surf-script` ← nobody when `story:scripts-engine-dependency` lands; `surf-html-conformance`
  from `story:scripts-html-conformance`; `surf-session` from `story:scripts-document-port`. Each
  joins the closed list in its own change, and the lock rule of Q15 holds the list.
- `surf-script` depends on `surf-html` for the tree and its write surface, and on `surf-url` for
  the document's address (`DocumentInfo`, the base URL of `baseURI`).
- `surf`, `surf-agent` and `surfd` link no swell (S7). `surf-session` depends on `surf`, `surfd`
  and `surf-agent` (`crates/session/Cargo.toml:20-33`), not the other way round; `surfd` depends
  on `surf-agent`, `surf-answer`, `surf-secret` and `surf-view` only
  (`crates/surfd/Cargo.toml`, `[dependencies]`).

The scripting parser step reaches the session's `Browser` through `Browser::with_parser`
(`crates/surf/src/lib.rs:1563`), which takes any parser step: `surf-session` builds a step of its
own that calls `surf_script` for a session with the option and `surf::HtmlParser`
(`crates/surf/src/lib.rs:415`) for one without. `HtmlParser` is not changed, and `crates/surf`
gains no dependency on `surf-script`, so `surf <url>` cannot run a page script.

Settled: the crate graph and the closed list (nobody, then `surf-html-conformance`, then `surf-session`), held by the lock rule of Q15, with the step reaching `Browser::with_parser` (`crates/surf/src/lib.rs:1563`).

## Q15. `lock-check` for a git source

A lock cargo writes for a package from git has no `checksum` line. The probe's lock, written by
cargo for the dependency `swell = { git = "https://github.com/beyond10x/swell", tag = "0.8.0" }`,
holds six such packages:

probe: (workstation) grep -c '^checksum' .out/probes/swell-limits/Cargo.lock; grep -c '^source = "git+https://github.com/beyond10x/swell?tag=0.8.0#0c1827128b57845e203713e87a0b0949fc8d63be"$' .out/probes/swell-limits/Cargo.lock -> 0; 6

So the allow-list line of a package from git keeps the four fields of today's format
(`taskfiles/lock-check.sh:7-9`) with `none` in the place of the checksum, the word the check
already prints for a package without one (`lock-check.sh:150`):

```
swell 0.8.0 git+https://github.com/beyond10x/swell?tag=0.8.0#0c1827128b57845e203713e87a0b0949fc8d63be none
```

and the same for `swell-compiler`, `swell-num`, `swell-regexp`, `swell-syntax` and
`swell-unicode`, six lines. A package of another address, tag or commit, or one with a checksum
where the list says `none`, fails as today.

The rule of who may depend on what: today only `surf-tls` may depend on a listed package
(`lock-check.sh:28-30`, `:157`). It becomes a second file, `taskfiles/lock-owners.txt`, one line
`<package> <crate>` per crate of this repository that may name the package among its
dependencies, sorted, no comments:

```
ring surf-tls
swell surf-script
```

with no line for `surf-script` while nobody may depend on it. A crate of this repository fails when it depends on a listed package
of `lock-allow.txt` or on `surf-script` without a line of its own, `Cargo.lock: <crate> depends
on <package>; lock-owners.txt does not allow it`; a listed package may still depend on another
listed package (ring's on theirs, `swell` on its five). `story:scripts-html-conformance` adds
`surf-script surf-html-conformance`, `story:scripts-document-port` adds `surf-script
surf-session`: the closed list of Q14.

The new sentence of `AGENTS.md` "Goals" (`AGENTS.md:30-43`), in place of "One exception: ..."
and the last item: "Two exceptions. Audited cryptographic primitives come from ring 0.17.14,
used by `surf-tls` alone; and one crate of the organization, the script engine swell, by a git
dependency on a release tag, used by `surf-script` alone. `taskfiles/lock-allow.txt` names each
of their packages by name, version, source and checksum (`none` for a package from git), and
`taskfiles/lock-owners.txt` names the crates of this repository that may depend on each (`task
lock-check`)." The `lock-check` description in `Taskfile.yml` (`Taskfile.yml:283`): "Fail,
naming each package, when the root Cargo.lock holds one from outside this repository that is not
on taskfiles/lock-allow.txt, or a crate of this repository depends on a listed package or on
surf-script without a line of taskfiles/lock-owners.txt."

Settled: the allow-list line with `none` for a package from git, read from the probe's lock (`probe:` line above), the file `taskfiles/lock-owners.txt` and its rule, the closed list of Q14 in it, and the two sentences.

## Q16. What a recorded error holds

After a script threw, the session reads, without running code:

- the kind and the own `message` through `Agent::error_summary` (`crates/swell/src/host/agent.rs:663
  @ 0.8.0`): `ErrorSummary { kind, message: Option<Vec<u16>> }`
  (`crates/swell/src/host/outcome.rs:120-124 @ 0.8.0`), the kind "the constructor that made it
  recorded, or the host's `HostObjectSpec::error_kind`, never its prototype or `name`"
  (`outcome.rs:116-118 @ 0.8.0`; HOST-179, `conformance/host/requirements.txt:179 @ 0.8.0`;
  requirement 5); `None` when the thrown value is no error object, recorded as kind `value` with
  no message;
- the line and column through `Agent::last_entry_error_site` (`agent.rs:684 @ 0.8.0`):
  `ErrorSite { line, column, dynamic, host_defined }`, "`line` and `column` count from 1, a column
  in UTF-16 code units" (`outcome.rs:91-103 @ 0.8.0`; HOST-172 to HOST-174,
  `conformance/host/requirements.txt:172-174 @ 0.8.0`; requirement 1). After a parse that
  answered `Parsed::Errors` the site is that of the first error; the error itself is the first
  root of the list.

A refusal is read the same way (its `SyntaxError`, kind and message, and the site).

The most kept: the message is decoded from UTF-16, unpaired surrogates replaced, and cut to its
first 1,024 characters (Unicode scalar values), the bound the answer gives a field's value
(`docs/agent-answer.md:304`); the cut is marked by the answer's quoting rules, not stored. The
line and column are integers below 2^32. Nothing else of the error is read: no `stack`, no
`name`, no own property but `message`. A `DOMException` the script did not catch, made by the
bindings or by `new DOMException(...)`, is recorded with kind `Error` and the message `<name>:
<message>` (`NotSupportedError: Node.appendChild is not supported: ...`), read from the
exception's own data without running script and cut to the same 1,024 characters.

How they reach an answer: through the record of Q7 (`error_kind`, `message`, `line`, `column`,
`dynamic`) and the `scripts` lines of Q7, where the message is quoted as untrusted page text and
cut to 120 characters (`surf_answer::quote_cut`, as `docs/agent-answer.md:325-326`).

Settled: the fields (kind, message, line, column, `dynamic`) and their sources, the bound of 1,024 characters for a message and 120 in an answer, and the way to an answer through Q7's record and `scripts` lines.

## Q17. The epic's statements from memory

- "A `nomodule` classic script is skipped" (epic, "In and out of the first version"):
  confirmed. HTML Standard 4.12.1.1, step 21 of "prepare the script element": "If el has a
  nomodule content attribute and its type is "classic", then return" (snapshot
  `546d834701d171e3b164da8463d62878deb1e954`, "Last Updated 23 November 2026").
- "cargo reads profiles from the root workspace only", so surf sets `overflow-checks = true` for
  swell's packages (epic, "The dependency rule"): confirmed for the manifests, corrected for
  config files. cargo reads no profile from a dependency's manifest. The Cargo Book, "Profiles":
  "Cargo only looks at the profile settings in the Cargo.toml manifest at the root of the
  workspace. Profile settings defined in dependencies will be ignored." (fetched 2026-11-24,
  sha256 above). A cargo config file overrides the root manifest's profiles (The Cargo Book,
  "Configuration", `[profile]`: it "can be used to globally change profile settings, and
  override settings specified in Cargo.toml"), so `task lock-check` also reads the tree's
  `.cargo/config.toml`; a config file above the tree, `--config` and `CARGO_PROFILE_*` are
  outside what a check of the tree sees (`story:scripts-engine-dependency`, fix round 1). The
  check also refuses, in the tree's config files, any rustflags and a line holding `overflow`
  other than `overflow-checks = true`, and, in `Taskfile.yml` and the files under `taskfiles/`
  and `.github/workflows/`, a line naming overflow checks in either spelling or rustflags with
  an argument file. It ends there: a setting reached through another file, a `rustc` or
  wrapper named in a config file, an ancestor's config file and the environment of whoever
  starts the build are held by review (fix rounds 2 and 3).
  swell's root `Cargo.toml` sets `overflow-checks = true` in `dev`, `release`, `test` and `bench`
  (`Cargo.toml:10-20 @ 0.8.0`), which surf's build does not read; surf's dev and test profiles
  check overflow by default, and its release profile sets it for the six swell packages
  (`[profile.release.package.<name>] overflow-checks = true`), as the probe's release build did
  for the whole profile.
- Apache-2.0 section 4(d) and swell's `NOTICE` (epic, "The dependency rule"): confirmed. "(d) If
  the Work includes a "NOTICE" text file as part of its distribution, then any Derivative Works
  that You distribute must include a readable copy of the attribution notices contained within
  such NOTICE file" (Apache License 2.0, `LICENSE-2.0.txt:107-110`, fetched 2026-11-24). swell is
  `license = "Apache-2.0"` (`Cargo.toml:7 @ 0.8.0`) with a `NOTICE` ("swell / Copyright 2026
  beyond10x", `NOTICE:1-2 @ 0.8.0`), so surf's release archives carry it beside surf's own.
- "`TrustedType` ... read as `DOMString`, a named deviation" (epic, "The DOM from Web IDL"):
  corrected in its form. `TrustedType` is `typedef (TrustedHTML or TrustedScript or
  TrustedScriptURL) TrustedType;` (Trusted Types, Editor's Draft 18 November 2026, section 4
  "Integrations"), and the DOM Standard uses it only in `(TrustedType or DOMString) value` of
  `setAttribute` and `setAttributeNS` (DOM Standard, 4.9 "Interface Element", snapshot
  `e4f4f4f0ace83fe6c52ab68e047351ce7f2dcd69`). Reading the typedef as `DOMString` would make the
  union `(DOMString or DOMString)`; the reader reads the whole union `(TrustedType or DOMString)`
  as `DOMString`. No value a script can make is a `TrustedHTML`, `TrustedScript` or
  `TrustedScriptURL` (none is generated), so every value converts as the union would convert it;
  it stays a named deviation in the selection. A deviation of every `DOMString` argument of the
  bindings, named once in the head of `selection.txt`: a lone surrogate reaches the slice as
  U+FFFD, because the tree holds Rust strings, where Web IDL's `DOMString` keeps every code unit.
  `DOMException`'s `name` and `message` reach no slice and keep every code unit.

Settled: each of the four statements confirmed or corrected with its citation: HTML Standard 4.12.1.1 step 21 (snapshot `546d834701d171e3b164da8463d62878deb1e954`), The Cargo Book "Profiles" (fetched 2026-11-24), Apache License 2.0 section 4(d) with `NOTICE:1-2 @ 0.8.0`, and Trusted Types (Editor's Draft 18 November 2026) with DOM Standard 4.9 (snapshot `e4f4f4f0ace83fe6c52ab68e047351ce7f2dcd69`).

## Q18. Requirements 1 to 10 for the engine

The table of the next section holds requirements 1 to 10 as the epic's last section revised
them, each with its HOST ids and its status at `0.8.0`, read from
`conformance/host/requirements.txt` @ 0.8.0, whose line n holds HOST-n (208 lines, 114 `test`,
94 `pending`). Rows 1 to 10 are `held`. Requirement 7 is a rule of the host interface, not a
numbered requirement: its row cites the rule and the gate task that holds it. Row 11 is new: it
asks swell to charge every copy that is alive while the engine calls into script, which
swell lists as open (Q13); its status at `0.8.0` is `absent`, and `story:scripts-document-port` waits for the tag that holds it.

Beside them: swell narrowed HOST-70 (HOST-188 to HOST-190, each with a test,
`conformance/host/requirements.txt:188-190 @ 0.8.0`), which Q4's row "after
`Terminated(MemoryBudget)`" uses; and slice 10 (HOST-192 to HOST-200) is `pending slice 10` at the tag (`CHANGELOG.md:81-86 @ 0.8.0`). The first version
does not need it: no promise exists before slice 10, so no microtask checkpoint is run (Q2, step
1).

Settled: the table of the next section, rows 1 to 10 each with its status and its line of `conformance/host/requirements.txt` at `0.8.0`, and row 11, the new requirement for swell, `absent` at `0.8.0`, which `story:scripts-document-port` waits for (Q13).

## Requirements 1 to 10, handed to the script engine's session, and requirement 11

| n | requirement | HOST ids | status at `0.8.0` | `requirements.txt` @ 0.8.0 |
|---|---|---|---|---|
| 1 | the line and column where a parse error or an uncaught throw arose: `Agent::last_entry_error_site` | HOST-172, HOST-173, HOST-174 | held | `conformance/host/requirements.txt:172-174 @ 0.8.0` (`test swell::scripts::host_172_a_parse_error_names_its_line_and_column` and the two after it) |
| 2 | every operation whose work grows with its operands charges steps in proportion; parsing and compilation charged to the memory budget before they allocate; property operations charge their work and temporary storage first | HOST-168, HOST-169, HOST-181, HOST-182 | held (the exceptions swell lists are in Q13) | `conformance/host/requirements.txt:168-169 @ 0.8.0`, `:181-182 @ 0.8.0` |
| 3 | the host charges and credits bytes and charges steps on the engine's one ledger: `charge_host_bytes` answers a `HostBytes`, dropping it credits; `Cx::charge_steps` | HOST-175, HOST-176, HOST-177 | held | `conformance/host/requirements.txt:175-177 @ 0.8.0` |
| 4 | withdrawn as a need of surf; the engine states what a kept handle does | HOST-178 | held | `conformance/host/requirements.txt:178 @ 0.8.0` (`test swell::host_objects::host_178_a_kept_handle_is_weak`) |
| 5 | an error's kind and own `message` read without running code; `HostObjectSpec::error_kind` | HOST-179 | held | `conformance/host/requirements.txt:179 @ 0.8.0` |
| 6 | HOST-17, HOST-152 and HOST-163 hold under the embedder's profiles: shown in the engine's own development and release profiles; surf tests its own profile | HOST-17, HOST-152, HOST-163 | held | `conformance/host/requirements.txt:17 @ 0.8.0`, `:152 @ 0.8.0`, `:163 @ 0.8.0` |
| 7 | each release names every change to a signature or requirement of the host interface: a committed snapshot of the interface with a gate check | none: a rule, `docs/design/host-interface.md:2123-2139 @ 0.8.0` | held | no line: the rule is held by `host-surface-check` (`Taskfile.yml:107 @ 0.8.0`) against `conformance/host/surface.txt` @ 0.8.0 |
| 8 | a host function's [[Prototype]] named or set: `Cx::set_prototype_of`, `Cx::get_prototype_of` | HOST-180 | held | `conformance/host/requirements.txt:180 @ 0.8.0` |
| 9 | a closed list of intrinsics and well-known symbols: `%Error.prototype%`, `%Function.prototype%`, `%Array.prototype.values%`, `entries`, `keys`, `forEach`; `@@toStringTag`, `@@unscopables`, `@@iterator` | HOST-183, HOST-184, HOST-185 | held | `conformance/host/requirements.txt:183-185 @ 0.8.0` |
| 10 | stated costs of a host function, a host object and a property, and a test of 300 host functions and 40 host objects: `HOST_FUNCTION_LIVE_BYTES`, `HOST_OBJECT_LIVE_BYTES`, `PROPERTY_LIVE_BYTES`, `BINDINGS_REALM_HEAP_BYTES` | HOST-186, HOST-187 | held | `conformance/host/requirements.txt:186-187 @ 0.8.0` |
| 11 | new, from this note: no copy that is alive while the engine calls into script is left without a charge. Every copy the engine makes of a String's units or of an argument list and holds while it calls back into script is charged to `memory_bytes` before it is made and credited when it is dropped. A test: a page that holds such copies across calls into script ends in `Terminated(MemoryBudget)` with `heap_bytes()` at most `memory_bytes` | none yet | absent | no line at the tag; swell lists the copies as open (`docs/plan/second-batch.md:284-285 @ 0.8.0`) |
