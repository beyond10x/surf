---
format: aep.planning-md/3
id: story:scripts-parse-run
kind: story
status: proposed
title: Run a page's inline scripts during its parse, with limits per document
summary: surf_script::run::parse_with_scripts joins the parser's script hook to swell through the runner, one agent per document, and adds the per-document policy (scripts per document, source size, step total, wall clock, the stop after a second memory end), each tested, every end written with the note's words, and the values of Q4 and Q5 measured again.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:html-script-hook
- depends_on: story:scripts-dom-reading
- depends_on: story:swell-0-9-0
scope:
- confidence: cited
  path: crates/script/src/lib.rs
- confidence: cited
  path: crates/script/src/run.rs
- confidence: cited
  path: crates/script/src/runner.rs
- confidence: cited
  path: crates/script/tests/parse_run.rs
- confidence: cited
  path: docs/design/scripts.md
revision: 29
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:13Z", actor: "human:operator", revision: 14}
---
## Context

The coordinating agent cut this story out of `story:scripts-dom-mutation`: the function that joins
the parser's script hook (`story:html-script-hook`) to swell, with the per-document limits, is a
product of its own, tested here rather than first in `story:scripts-hostile-pages`. The epic's
"Limits per page" (lines 225-229): the agent's limits, a step total per document summed from
`last_entry_steps()`, a number of scripts run per document, a wall-clock bound per document
through the interrupt handle, and every DOM write held to the parser's limits; "A limit reached
ends that script; parsing goes on, and the document records which limit." Scripts run only while
the document is parsed, one agent per document, dropped with it (epic lines 117-119, 218-219).
After `Terminated(MemoryBudget)` surf calls `collect()` once and runs no further script in that
document if the next entry ends so again (epic line 840).

What exists before it, and what this story only calls:

- the runner of `story:scripts-engine-dependency` (`crates/script/src/runner.rs`): the agent and
  its limits with the starting values of Q4, the thread with the stack of Q5, `ScriptEnd`, the
  step and script totals, the wall-clock timer on the interrupt handle, and the rule after
  `Terminated(MemoryBudget)` (one `collect()`, no further script after a second);
- the bindings and `DomCx` of `story:webidl-bindings`, and its rule `Runner::document_end` in
  `crates/script/src/runner.rs` that maps an interrupted entry to the limit `DomCx` recorded,
  otherwise to `limit: wall-clock`;
- swell `0.9.0` and the microtask checkpoint after each script of `story:swell-0-9-0`, its steps
  in the step total and its end taken as the script's end;
- the reading slice and the document over a parsed tree (`story:scripts-dom-reading`);
- the hook, which reports each script element it runs and each it skips with a word of Q1's line
  `reasons:` (`story:html-script-hook`, decision 1).

The end words are the closed list of ends of `docs/design/scripts.md`, Q7: `ran`;
`threw <kind> "<message>" <line>:<column>`; `refused "<message>" <line>:<column>`;
`limit: steps-per-script`, `limit: memory`, `limit: source-bytes`, `limit: wall-clock`,
`limit: nodes`, `limit: depth`, `limit: attributes`, `limit: stored-bytes`; `faulted`; and
`not run:` with each word of Q1's line `reasons:` (`empty`, `src`, `type`, `module`, `importmap`,
`speculationrules`, `disconnected`, `nomodule`, `event`, `svg`, `inserted`, `eof`) and with
`scripts-per-document`, `steps-per-document`, `wall-clock`, `memory` and `faulted`. They are the 23
variants of `surf.document.ScriptReason` behind `Limit` and `NotRun` (Q7's YAML). This story
invents none.

Found by the review of `story:html-script-hook` (2026-11-24), and ruled in that unit:

- The hook is called once for every `script` element the parser inserts: an SVG script
  with the word `svg` however it is closed, and an HTML script that is open when the
  parser stops, at the end of the input or at a limit, with `eof`. This story maps each
  word to its end and counts on exactly one call per element.
- Bytes with a hook: the only path today is `encoding::decode` and then
  `parse_document_with_hook` on the text. That path hands the parser no encoding, so a
  `meta` that names another encoding does not decode the bytes again: scripts run once,
  and the tree can differ from the one `parse_document_bytes` builds for the same bytes.
  This story decides which tree a page with such a `meta` gets when scripts are on and
  holds it with a test on one page whose `meta` names an encoding the sniffing did not
  pick.
- In every parse, with a hook or without, the parser's depth limit holds for the tree and no
  longer for the stack of open elements alone; a parse stopped that way has `stopped_by`
  `Depth`. A parse that has passed the step limit stops with `stopped_by` `Steps` and names no
  other limit (`story:html-script-hook`, "As built").
- The document facts of a page reach the DOM slices through this story. `story:scripts-dom-reading` gives `surf_script::dom` a public `DocumentInfo { address: surf_url::Url, content_type: String, encoding: String }` and `dom::slices(info: DocumentInfo) -> Slices`; the members `URL`, `documentURI`, `baseURI`, `contentType`, `characterSet`, `charset` and `inputEncoding` read it, and `compatMode` comes from the tree (`surf_html::Tree::quirks_mode`, written by the parser). This story builds the `DocumentInfo` of a page: the address as a `surf_url::Url` (the `address` of decision 1, whose type is this story's to state), the response's content type, and the name of the encoding the parse decodes with. It makes the runner at the first script element that runs, while the encoding may still be tentative (the bytes-with-a-hook bullet above); which encoding name it passes then is this story's to decide, with its own critics. The `DocumentInfo` it passes must name the address the document was fetched from, because `baseURI` resolves a `base` element's `href` against it.

## Decisions taken

1. `surf_script::run::parse_with_scripts(bytes, address, options, limits) -> ScriptedParse`: parses
   with scripting enabled on the runner's thread, makes one runner with the bindings for the
   document at its first script element that runs, runs each script element the hook hands it
   through the runner, and answers the parsed document, the list of ends in the order they happened
   (one per script element the parser reached, run or not, `eof` included, and one per element a
   script inserted, `not run: inserted`; never two for one element; Q7's `cardinality:`), and the
   per-document limit that stopped scripts, if one did. The runner and its agent are dropped at the
   end of the parse, on that thread, after the tree is taken out of the host value (Q5).
2. This story re-implements none of the runner's mechanisms: the memory end and its one
   `collect()`, the wall-clock timer, the step and script totals and the mapping of an interrupted
   entry are the runner's and `story:webidl-bindings`' (epic lines 225-229, 840), called here. Its
   own part is the per-document policy of `docs/design/scripts.md`, Q4, and its tests:
   - a skipped element the hook reports becomes the end `not run: <its word of Q1>`, `not run: eof`
     for an element the end of the input closed; an element a script inserts and connects gets the
     end `not run: inserted`, written by the DOM write that connects it through a function of
     `run.rs` the mutation slice calls;
   - scripts per document, 1,000: the 1,001st script element and each later one ends
     `not run: scripts-per-document`;
   - bytes of one script's source, 524,288 (UTF-8, as the tree holds it): a longer script is not
     parsed and ends `limit: source-bytes`; later scripts run;
   - step total per document, 50,000,000, summed from `last_entry_steps()` after each script: the
     agent's limits are fixed at `Agent::new`, so a script starts only while the total is under the
     limit; the script whose steps take the total to it keeps its own end, and each later script
     ends `not run: steps-per-document`;
   - wall clock per document, 5 s from its first script's start, through the interrupt handle: the
     running script ends `limit: wall-clock`, each later one `not run: wall-clock`;
   - after `Terminated(MemoryBudget)` the runner calls `collect()` once; a second `limit: memory` in
     the same document makes each later script `not run: memory`; after `faulted` each later script
     ends `not run: faulted`;
   - a limit of the DOM (`limit: nodes`, `limit: depth`, `limit: attributes`, `limit: stored-bytes`)
     ends only the script that reached it, through the interrupt, so a `try` does not catch it
     (Q4); a later script of the document runs. Parsing goes on to the end of the document after
     every end.
3. The agent's limits are Q4's session values: `steps_per_entry` 10,000,000, `memory_bytes`
   67,108,864, `frame_depth` 10,000, `reentry_depth` 256, `parser_nesting` 1,000, `string_length`
   16,777,216, `bigint_bits` 1,048,576; `eval` and `Function` are allowed. A test gives limits of
   its own, as Q4 says; in the test profile a script runs about 1.4 million steps per second (Q4),
   so a case of a step limit sets limits its time allows.
4. The stack margin of Q5. The thread of a scripted document has `min_stack_bytes()` (4,006,656
   bytes at Q4's limits) plus a margin, chosen at (`reentry_depth` + 1) × 14,336 + 1,048,576 =
   4,732,928 bytes, 8,739,584 bytes in all. This story measures the margin with the test Q5 names,
   `deepest_nesting_on_parse_thread`: on a thread of exactly `min_stack_bytes()` plus the margin, a
   script whose source nests to `parser_nesting`, parsed (through `eval`) inside a call that
   re-enters through a binding (a getter of a DOM wrapper that calls `eval`) `reentry_depth` times;
   the margin then becomes that test's need plus a fifth, as swell rounds its own, written in
   `crates/script/src/runner.rs` and in Q5 with its `probe:` line. A stack that overflows is not an
   end of a script: Rust's handler of the guard page aborts the process, so a session ends in
   `SIGABRT` and its client reads `note ended: signal SIGABRT` (Q5, Q11).
5. The values of Q4 were measured without bindings or DOM; this story measures them again with both
   (steps per second in the test profile and in release, the heap of a realm with the bindings,
   whether a page with more than 1,000 scripts reaches the script count) with probes in the session
   scratch directory, never committed. The measurements are taken on swell's tag `0.9.0`, which
   `story:swell-0-9-0` takes before this story. A value holds if a script at that limit ends with that
   limit's end word. Where one does not hold, the story changes it where it lives in code (the
   agent's limits, the wall-clock bound and the stack in `crates/script/src/runner.rs`; the
   per-document limits in `crates/script/src/run.rs`) and in the note's Q4 or Q5, with its `probe:`
   line.

## Change

- `crates/script/src/run.rs` (new) and `crates/script/src/lib.rs`: decisions 1 to 3, the function
  that records `not run: inserted` included.
- `crates/script/src/runner.rs`: only a value that decision 4 or 5 changes.
- `crates/script/tests/parse_run.rs`: the module `parse_run`, declared by `story:webidl-bindings`.
- `docs/design/scripts.md`: Q4 and Q5 as decisions 4 and 5 find them, only where a value changes.
- In the last commit, the public surfaces of acceptance 14.

## Acceptance

1. `scripts_per_document`: with the limit 2, a page of three inline scripts `1` ends `ran`, `ran`,
   `not run: scripts-per-document`, and the markup after the third is in the tree.
2. `source_bytes`: with the limit 100, a script of 101 bytes ends `limit: source-bytes` and a later
   script of 10 bytes ends `ran`.
3. `steps_per_document`: the test runs `{ for (let i = 0; i < 100000; i++) {} }` once and reads its
   `last_entry_steps()`; with a per-entry limit that ends none of the scripts and a document total
   of 1.5 times that count, a page of that script three times ends `ran`, `ran`,
   `not run: steps-per-document` (the second script takes the total past the limit and keeps its
   own end), and the returned limit is `steps-per-document`.
4. `wall_clock`: with a bound of 100 ms and step limits that end nothing first, `while (true) {}`
   ends `limit: wall-clock`, the elapsed time under 10 s; a later script ends
   `not run: wall-clock`; the returned limit is `wall-clock`; the markup after both is in the tree.
5. `skipped_reasons`: a page holding one element per variant of the hook's `NotRun`
   (`<script></script>`, `<script src=a.js>`, `<script type=text/template>x</script>`,
   `<script type=module>x</script>`, `<script type=importmap>{}</script>`,
   `<script type=speculationrules>{}</script>`, a script in `template` contents,
   `<script nomodule>x</script>`, `<script event=onclick for=window>x</script>`, an SVG `script`)
   and an inline classic script ends, in document order, `not run: empty`, `not run: src`,
   `not run: type`, `not run: module`, `not run: importmap`, `not run: speculationrules`,
   `not run: disconnected`, `not run: nomodule`, `not run: event`, `not run: svg`, `ran`.
6. `eof_recorded`: the page `<p>a</p><script>x = 1` ends `not run: eof`, one end, and the `p` is in
   the tree.
7. `inserted_recorded`: the test defines on the global a host function `insertScript` that creates
   a `script` element with text, appends it to `body` through the write surface and calls the
   function of decision 2; a page `<script>insertScript()</script><script>1</script>` ends `ran`,
   `not run: inserted`, `ran`, in that order of writing.
8. `memory_twice`: under a memory limit of 1 MiB, two scripts
   `{ const a = []; for (;;) a.push(new Array(1000)); }` and then `1` end `limit: memory`,
   `limit: memory`, `not run: memory`; the returned limit is `memory`.
9. `scripting_off_same_tree`: with scripting disabled, `parse_with_scripts` answers the tree
   `surf_html` parses today and no end.
10. `dom_limit_then_next_runs`: the mutation slice comes after this story, so the test defines on
    the global a host function `reachNodeLimit` that does what the generated glue of
    `story:webidl-bindings` does for `DomEnd::Limit` of the node limit; a first script
    `reachNodeLimit(); 1` ends `limit: nodes`, also inside `try { ... } catch {}`, and a later
    script `1` of the same document ends `ran`.
11. `frame_depth_on_parse_thread`: `function f(n) { return 1 + f(n + 1); } f(0)` through
    `parse_with_scripts` at the `frame_depth` of Q4 ends
    `threw RangeError "frame_depth exceeded" <line>:<column>`, without a crash.
12. `deepest_nesting_on_parse_thread`: the test of decision 4 passes on a thread of exactly
    `min_stack_bytes()` plus the margin the story ships, each re-entry ending as swell answers it
    and the process alive; the report gives the test's measured need and the margin with its
    `probe:` line, and, from a probe outside the gate, the same program on a thread below that need
    ending `exit 134 signal ABRT`.
13. Decisions 4 and 5: the report lists, per row of Q4 and for Q5's margin, `holds` or
    `<old> -> <new>` with its `probe:` line; `git diff --stat` names `docs/design/scripts.md` and
    `crates/script/src/runner.rs` only if a value changed.
14. Public surfaces: the story changes, in its last commit and after merging `main`, every public
    surface its change makes false, each number taken from the `slot task check` log of its tip.
    After that commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges` prints
    nothing for each old string below, as the story's base holds it (the strings and lines given
    are those of `main` at `29945f5`, 2026-11-24); each new number equals the one in that log; and
    the gate block, `README.md:584-625` and `website/docs/how-it-is-built.md:40-81`, holds that
    log's lines. The old strings of this story: the test count `1976 passed` (`README.md:11`,
    `docs/badges/make.sh:38`, `docs/badges/tests.svg`), `1,976` and `1,424, and 552`
    (`README.md:285`, `website/docs/status.md:23`, `website/docs/what-works.md:17`).
15. `slot task check` exits 0; `git diff --stat` names only the files of the Change.
16. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; no model
    call and no network request in a test; each case parses a page of a few lines and makes one
    agent; no Python, the probes of decisions 4 and 5 included; a probe that only measures runs
    without `slot`, one that builds through it; no generator with a committed output is added; a
    build or test job in the pool ends within 10 minutes or names its time with `slot --timeout`,
    at most 30 minutes; a test over 5 s is named (`wall_clock` waits 100 ms by design;
    `deepest_nesting_on_parse_thread` is timed and named if over 5 s).
17. `document_facts_reach_the_script`: the test's own page (no network request, no model call),
    in bytes, parsed by `parse_with_scripts` with the address `https://example.com/dir/page` and
    the content type `text/html`, has one inline script that compares `document.URL`,
    `document.baseURI`, `document.contentType` and `document.compatMode` with
    `https://example.com/dir/page`, `https://example.com/dir/page` (no `base` element),
    `text/html` and `BackCompat` (no doctype), and throws an `Error` whose message is the four
    values read, joined by a space, when one differs. The script's end is `ran`; a `threw` end
    shows the values in its record. The same page after `<!DOCTYPE html>`, its script comparing
    with `CSS1Compat`, ends `ran`. The script's end is the reader: the test reads nothing from
    the realm, which is dropped when the parse ends (decision 1). Decision 1's signature has no
    place for the content type yet: how it reaches `parse_with_scripts` (a parameter or a field
    of `options`) is stated in decision 1 when this story is revised against
    `story:scripts-dom-reading` as built, and this item names that place then.

## Out of scope

The members that write (`story:scripts-dom-mutation`, `story:scripts-dom-attributes`); the
runner's mechanisms (`story:scripts-engine-dependency`); the harness of html5lib cases
(`story:scripts-html-conformance`); a session (`story:scripts-document-port`); a page meant to
break the limits (`story:scripts-hostile-pages`).

## Placement

After `story:html-script-hook`, `story:scripts-dom-reading` and `story:swell-0-9-0`; before
`story:scripts-dom-mutation`, `story:scripts-html-conformance` and `story:scripts-document-port`,
which call `parse_with_scripts`. Beside `story:scripts-dom-attributes` and
`story:scripts-dom-selectors`, which write other files of `crates/script`.
`crates/script/src/runner.rs` is `story:scripts-engine-dependency`'s, `story:webidl-bindings`' and
`story:swell-0-9-0`'s, each merged before this story.

## Why `epic:scripts-in-a-page`

The epic's outcome runs a page's inline scripts during its parse, with limits per document; this
is that run.
