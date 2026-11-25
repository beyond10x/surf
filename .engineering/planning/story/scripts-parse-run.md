---
format: aep.planning-md/3
id: story:scripts-parse-run
kind: story
status: proposed
title: Run a page's inline scripts during its parse, with limits per document
summary: surf_script::run::parse_with_scripts joins the parser's script hook to swell on one thread, one runner per document, given the page's address, content type and encoding name from a new surf-html entry that restarts under the hook as parse_document_bytes does (one parse, or two when a meta changes the encoding) and equals its tree; it enforces the three limits the runner only counts (scripts, step total, source bytes) beside the runner's wall clock, memory and fault stops, the first stop of a document keeping its word for every later script, records each skipped and inserted script element with the note's words, answers Err when the host cannot make a runner, lends the parser's tree to the DOM for each script, and measures the values of Q4 and Q5 again.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:html-script-hook
- depends_on: story:scripts-dom-reading
- depends_on: story:swell-0-9-0
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/html/src/lib.rs
- confidence: cited
  path: crates/html/src/tree_builder.rs
- confidence: cited
  path: crates/html/tests/bytes_with_hook.rs
- confidence: cited
  path: crates/html/tests/main.rs
- confidence: cited
  path: crates/script/src/bindings.rs
- confidence: cited
  path: crates/script/src/lib.rs
- confidence: cited
  path: crates/script/src/run.rs
- confidence: cited
  path: crates/script/src/runner.rs
- confidence: cited
  path: crates/script/tests/engine.rs
- confidence: cited
  path: crates/script/tests/parse_run.rs
- confidence: cited
  path: crates/webidl/src/prelude.rs
- confidence: cited
  path: docs/badges/make.sh
- confidence: cited
  path: docs/badges/tests.svg
- confidence: cited
  path: docs/design/html-parser.md
- confidence: cited
  path: docs/design/scripts.md
- confidence: cited
  path: website/docs/how-it-is-built.md
- confidence: cited
  path: website/docs/status.md
- confidence: cited
  path: website/docs/what-works.md
revision: 52
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:13Z", actor: "human:operator", revision: 14}
---
## Context

This story is the function that joins the parser's script hook to swell, with the limits of one
document. It was cut out of `story:scripts-dom-mutation` so that it is built and tested before the
members that write. It is written for the tree after `impl/scripts-dom-reading` and
`impl/swell-0-10-0` are merged into `main`, in either order: the story holds whichever merges
first (Placement). A line is read at `a14e3b0` (`main`), `8086a30` (the reading unit's tip when
this was written; its scope has grown since, so that tip is not its last) or `f463e24` (the swell
unit's tip), as it says; the implementor reads each again on the merged tree and names those that
moved.

The epic's words (`.engineering/planning/epic/scripts-in-a-page.md`): one agent per document,
created at its first script and dropped with the document, on one thread (lines 225-230); "Limits
per page": the agent's limits, a step total per document summed from `last_entry_steps()`, a number
of scripts run per document, a wall-clock bound per document through the interrupt handle, and
every DOM write held to the parser's limits; "A limit reached ends that script; parsing goes on,
and the document records which limit." (lines 231-235); scripts run only while the document is
parsed (lines 123-124); after `Terminated(MemoryBudget)` surf calls `collect()` once and runs no
further script of that document if the next entry ends so again (line 873).

What exists, and what this story only calls.

The hook, on `main` (`a14e3b0`):

- `parse_document_with_hook(input: &str, options: &ParseOptions, hook: &mut dyn ScriptHook) ->
  Tree` takes decoded text and gives the parser no encoding (`crates/html/src/lib.rs:301-312`, the
  `None` at `:307`), so a `meta` never changes the encoding in that parse. `parse_document_bytes`
  takes no hook (`lib.rs:324-331`); the unit says why: its second parse after a `meta` would call
  each script twice (`.engineering/planning/story/html-script-hook.md:226-229`). This story
  accepts that, as the Standard's restart does: the scripts of the first document are gone with
  it and run again in the second. With scripting disabled, which is `ParseOptions::default()`
  (`lib.rs:244-251`), no call is made.
- Where a restart is decided: `change_the_encoding` sets `reparse_with` and calls `stop_parsing`
  (`crates/html/src/tree_builder.rs:1907-1928`), reached from `meta_changes_the_encoding`
  (`:1934-1951`) only while the encoding in use has the confidence `Tentative` (`:1936`). A
  parse is given the encoding in use by its `encoding` argument (`parse_with_hook`, `:598-610`),
  and `run` answers `Parsed.reparse_with` (`:713`). So a hooked parse given a tentative
  `InputEncoding` answers `reparse_with` today; no change of `tree_builder.rs` makes it. Only
  `parse_document_with_hook` never does, because it passes `None` (`lib.rs:307`). After a
  restart is decided, `run` still reports the script elements that were open
  (`tree_builder.rs:691-697`). Three places say that second pass takes no hook, "so its stack
  stays inside the depth limit": the comment at `tree_builder.rs:1104-1106`, the crate
  documentation at `crates/html/src/lib.rs:170` ("Limits") and
  `docs/design/html-parser.md:422-423`, all `@ a14e3b0`.
- That reason is false for every parse since the hook unit, with a hook or without. The stack of
  open elements has no bound of its own: a node is on it at most once, so the node limit bounds
  its length and the step limit bounds every read of it (`story/html-script-hook.md:334-339`;
  `docs/design/html-parser.md:414-421 @ a14e3b0`). The depth limit bounds the tree: `room_at`
  stops the parse at `Limit::Depth` when a place would be deeper (`tree_builder.rs:1226-1228`).
  A hooked second pass is bounded the same way, so the entry needs no guard for it. This proof
  carries the stack bound, and no test shows a stack hazard: acceptance 23 (f) shows that, at a
  small `max_depth`, the tree and `stopped_by` of a restarted page under a hook equal those of
  `parse_document_bytes`.
- `ScriptHook` has `run(&mut self, tree: &mut Tree, script: NodeId)` and
  `skip(&mut self, tree: &Tree, script: NodeId, reason: NotRun)`
  (`crates/html/src/tree_builder.rs:82-88`). `NotRun` has 11 variants, each with a `word()`
  (`tree_builder.rs:93-135`): `empty`, `src`, `type`, `module`, `importmap`, `speculationrules`,
  `disconnected`, `nomodule`, `event`, `svg`, `eof`. Every script element the parser inserts gets
  one call, never two and never none: an HTML `script` open when the parser stops, at the end of
  the input or at a limit, gets `skip` with `eof`; an SVG `script`, however it is closed, gets
  `skip` with `svg`; an element a script inserted gets none (`html-script-hook.md:237-250`).
- The tree's reads (`Tree::children`, `data`, `limits`, `stopped_by`) and its write
  surface are public (`crates/html/src/tree.rs:520,554,595,605,962-1561`). `Tree::document` and
  `Tree::stop_at` are crate-private (`tree.rs:273,513`), as are `tree_builder::parse*`,
  `Parsed` (`tree_builder.rs:573-583`), `InputEncoding` (`tree_builder.rs:565`) and
  `limits::cut_bytes` (`limits.rs:146`). `parse_document_bytes` is two functions: itself
  (`lib.rs:324-331`) and the private `parse_cut_bytes` (`:334-360`). Together they cut the bytes,
  decode them, parse, parse again with `Certain` after a `meta` that changes the encoding, and
  mark the tree `InputBytes` when they cut; the decoded text goes to the parser without a second
  cut.
- `encoding::decode(bytes)` sniffs and decodes; its confidence is `Tentative` unless a byte order
  mark decided; it reads the first `PRESCAN_BYTES` (1,024, `encoding.rs:361`) for a `meta`.
  `encoding::decode_with(bytes, name)` decodes again with a named encoding and the confidence
  `Certain` (`encoding.rs:402,429`). `ParsedDocument { tree, encoding }` is what
  `parse_document_bytes` returns (`lib.rs:266-274`).

The reading unit (`8086a30`):

- `surf_script::dom::DocumentInfo { address: Url, content_type: String, encoding: String }` and
  `dom::slices(info: DocumentInfo) -> Slices` (`crates/script/src/dom/mod.rs:27-34,48-56`). The
  members `URL`, `documentURI`, `baseURI`, `contentType`, `characterSet`, `charset` and
  `inputEncoding` read it; `compatMode` comes from the tree (`Tree::quirks_mode`, written by the
  parser where the initial insertion mode decides it).
- `Document { tree, limits, slices }` and `run_document(limits, mode, document, body)`
  (`crates/script/src/runner.rs:434-438,444-456`) make the runner and its realm with the bindings
  on the runner's thread; the tree sits in `DomState`, whose `tree` field is private
  (`crates/script/src/bindings.rs:169-225`). `Slices` has public fields and the test pattern of
  replacing one is the reading unit's (`story:scripts-dom-reading`, decision 14;
  `bindings.rs:522-533`).
- A kept `childNodes` or `children` object whose refill the memory limit cuts is recorded
  (`DomState::cut_refill`, `bindings.rs:217-219,1425-1432`); `Runner::evaluate` reads the record
  after `limit: memory` and treats it as a second memory end (`runner.rs:663-677`).

The swell unit (`f463e24`):

- `RunnerLimits { engine, steps_per_runner, scripts_per_runner, source_bytes, wall_clock }`
  (`runner.rs:75-93`) with `RunnerLimits::SESSION` (`runner.rs:97-103`): the agent's seven limits
  of Q4 (`SESSION_ENGINE_LIMITS`, `runner.rs:44-52`), 50,000,000 steps, 1,000 scripts, 524,288
  bytes of source, 5 s. Its comments name this story as the one that enforces
  `steps_per_runner`, `scripts_per_runner` and `source_bytes` (`runner.rs:79-90`); the runner
  counts `Runner::steps()` and `Runner::scripts()` (`runner.rs:1000-1008`).
- The runner enforces what it can alone (`Runner::evaluate`, doc comment `runner.rs:626-669`, body
  `:670-689`, `script_end` `:691-724`): the wall clock (`WallClock`, `:488-556`: the clock starts
  at the first script's start; a script after it ran out is `not run: wall-clock`), `not run:
  memory` after the second `limit: memory`, `not run: faulted`, one `collect()` after the first
  `limit: memory`, and the rule of the interrupt below.
- The microtask checkpoint (`Runner::checkpoint`, `runner.rs:820-834`) follows a script that
  ended `ran`, `threw` or `refused`; its end is the script's end; its steps are in
  `Runner::steps()`; after any `limit: <word>` the queue is cleared (`Runner::clear_microtasks`,
  `:836`, called from `evaluate`, `:670-689`). An interrupt set when a script ends is that
  script's end, and no script inherits an interrupt: `evaluate` withdraws it after every end,
  `not run` included (`runner.rs:675`, doc comment `:629-669`).
- The thread's stack: `min_stack_bytes()` 4,006,656 plus the margin
  `(reentry_depth + 1) × 14,336 + 1,048,576` = 4,732,928, in all 8,739,584 bytes
  (`runner.rs:56,59,107-120`; pinned by `crates/script/tests/engine.rs:240-250`). `run` and
  `run_document` make that thread and make the runner on it before their body runs;
  `Runner::new` is private (`runner.rs:423-486,580`).
- The words of the ends are `ScriptEnd` and its 23 `Reason` variants (`runner.rs:266-289,349-362`),
  the closed list of `docs/design/scripts.md`, Q7, and of `surf.document.ScriptReason`
  (`spec/domains/document.yaml:157-182` at `a14e3b0`). This story invents none.

`tests/parse_run.rs` is a two-line stub declared in `crates/script/tests/main.rs` (`:18` at
`a14e3b0`); this story writes it.

## Decisions taken

1. The limits of one document are held in two places. The runner holds the agent's limits, the
   wall clock, the second memory end, the fault and the cut refill, and enforces them. The caller
   enforces the three it only counts or measures: `scripts_per_runner`, `steps_per_runner` and
   `source_bytes`. One runner serves one document, so each is a limit of the document
   (`runner.rs:75-93`; the comments name this story). This story writes the caller: `run.rs`.
2. The session's values are `RunnerLimits::SESSION` and are in code already (Context); this story
   restates none of them in code. A test passes limits of its own, as Q4 says, and sets them so
   that it ends within the test bounds (Q4's rates are 19,978,480 to 21,133,215 steps per second
   in release and 1,372,567 to 1,405,663 in the test profile, `docs/design/scripts.md:369 @
   f463e24`; both are measured again with the bindings, acceptance 20).
3. The thread and its stack margin are `RunnerLimits::stack_bytes()`, in code already. This story
   measures the margin on the parse thread, whose first frames are the parser and the hook, and
   changes it only where the measure says so.
4. The agent lives on one thread only (an `Agent` is not `Send`; Q5 of `docs/design/scripts.md`).
   The parse runs on that thread; the runner is made there at the first script element that runs
   and dropped there at the end of the parse (Q5). No state passes from one document to the next.
5. One end for each script element, in the order the ends happened: the hook's one call for each
   parser-inserted element gives one end, a script element a script inserts and connects gives
   one `not run: inserted`, and an element the hook reported never gets a second record
   (Q1's rows, `docs/design/scripts.md:91-95 @ f463e24`; Q7's `cardinality:` and `written:`,
   `:553-555 @ f463e24`).
6. The content type of a page is a parameter of the call, passed unchanged to
   `DocumentInfo.content_type`. The bytes carry none, and `encoding::decode` takes no transport
   layer because `surf.net.MediaType` carries no `charset` (`encoding.rs:395-401`).
7. The document's address is the `surf_url::Url` the page was fetched from, passed unchanged to
   `DocumentInfo.address`: `baseURI` resolves a `base` element's `href` against it
   (`story:scripts-dom-reading`, decisions 12 and 13).
8. The words of the ends of a script, its checkpoint included, and the rule that no script
   inherits an interrupt are the runner's. `run.rs` reads `Runner::steps()` and
   `Runner::scripts()` and does not add steps of its own. A cut refill and a second memory end both
   make `Runner::evaluate` answer `not run: memory` afterwards; `run.rs` learns that through the
   runner's stop (decision 9) and adds no check of its own for either.
9. The first per-document stop that happened is the document's stop. The stops are `faulted`,
    `memory`, `wall-clock`, `scripts-per-document` and `steps-per-document`. Before each script
    element that would run: when a stop is recorded, the end is `not run: <its word>`, and the
    word does not change later in the document; otherwise the stops are tested in the order
    `faulted`, `memory`, `wall-clock`, `scripts-per-document`, `steps-per-document`, and the first
    that holds is recorded and is the end; only when none holds is the script's own source checked
    (`limit: source-bytes`, which ends that script alone and records no stop). What the runner
    answers today: `script_end` returns `not run: faulted` when its `faulted` field or the agent's
    state says so, then `not run: memory` when the second memory end happened, then
    `not run: wall-clock` when the clock ran out, in that order, before it runs anything
    (`runner.rs:691-704 @ f463e24`). Those three are private (`faulted` and `memory_ends` fields,
    `WallClock::ran_out` at `:529`), so `run.rs` cannot learn them without running a script: the
    Change adds `Runner::stop`. The clock can run out, or a fault can set in, between the hook's
    call of `stop()` and the check at the start of `script_end`; then `evaluate` answers
    `not run: faulted`, `memory` or `wall-clock` for a script the hook had let through, so the
    hook records that word as the stop as well. The invariant: whenever an end of the list is a
    `not run` with one of the three stop words, `stopped()` is not `None` and is that word, because
    the first stop keeps its word (the helper of the Acceptance holds it). Two things are held
    by the shared code of `Runner::stop` and `script_end` and by no test through the parse: the
    `faulted` word (no script makes swell fault on purpose,
    `crates/script/tests/engine.rs:11-12 @ f463e24`) and the order of `wall-clock` before
    `scripts-per-document` when both hold at one check (a test would need the clock to run out
    at that very check).
10. The bytes of a page parsed with a hook are decoded and parsed by `surf-html`, in one new public
    entry that restarts under the hook as `parse_document_bytes` does and makes no pre-parse.
    A page that does not change its encoding is parsed once and each of its scripts runs once. A
    page whose `meta` changes the encoding is parsed twice, as the Standard's "change the
    encoding" restarts with the new encoding and the confidence `Certain` when it cannot change
    the decoder on the fly: the first document's runner, ends and totals are dropped, and a script
    before the `meta` runs twice, once in each document. The ends, the totals and the stop that
    `parse_with_scripts` answers are those of the second parse alone, a `RunnerError` included:
    an `Err` that the first parse's hook kept is dropped with that hook when the page restarts,
    and the call answers what the second parse's hook holds, so a page whose first parse could
    not make a runner can answer `Ok` after its restart. In the first parse
    `document.characterSet` reads the tentative name, in the second the final one. There is at
    most one restart, because the second parse has the confidence `Certain`. The limits are per
    runner, so a page that restarts can use each per-document limit twice (the wall clock: two
    bounds, one for each runner). That is the rule and the bound of the whole page: at most two
    parses, so each per-document limit at most twice, and no limit of the page across the
    restart. The epic's Decisions and `story:scripts-hostile-pages` record it (a case with the
    wall clock spent before a late `meta` and again after it). The tree, `stopped_by` and every
    mark equal `parse_document_bytes`' for the same bytes when the hook writes nothing, by one
    private function that both entries call; the hook is the only difference. A script's write
    before a later `meta` can make the parse reach that `meta` differently from an unscripted
    parse; that is what the Standard does as well. The entry uses the crate's own
    `Tree::stop_at` and `limits::cut_bytes`, which are private to it (Context; `lib.rs:324-360`,
    `limits.rs:146`).
11. Scripts do not become visible to a user of surf in this story. The option and the session that
   run them are `story:scripts-document-port`'s (`scripts-document-port.md:7,149-156`); the
   README's "surf has no JavaScript yet" (`README.md:31`) and "no binary uses `surf-script` yet"
   (`README.md:253-254`) stay true, and no crate gains a dependency on `surf-script`.

## Change

`crates/script/src/run.rs` (new) and `crates/script/src/lib.rs` (`pub mod run;`, with the doc
comment of the crate). `surf_script::run` is a module and `runner::run` is re-exported as a
function of the same name (`lib.rs:19-22 @ 8086a30`); types and values are separate namespaces,
and the downstream stories cite `surf_script::run::parse_with_scripts`, so the module keeps the
name.

```rust
pub struct Page<'a> { pub address: &'a Url, pub content_type: &'a str }
pub struct Config { pub limits: RunnerLimits, pub mode: Mode }
impl Config { pub const SESSION: Config }
pub struct ScriptedParse {
    pub tree: Tree,
    pub ends: Vec<ScriptEnd>,
    pub encoding: Option<&'static str>,
    pub steps: u64,
    pub scripts: u32,
    stop: Option<Reason>,
}
impl ScriptedParse { pub fn stopped(&self) -> Option<Reason> }
pub fn parse_with_scripts(bytes: &[u8], page: &Page<'_>, options: &ParseOptions, config: &Config)
    -> Result<ScriptedParse, RunnerError>
pub fn parse_with_slices<F>(bytes: &[u8], page: &Page<'_>, options: &ParseOptions,
    config: &Config, make_slices: F) -> Result<ScriptedParse, RunnerError>
    where F: FnMut(DocumentInfo) -> Slices + Send
pub fn record_inserted(cx: &mut DomCx<'_, '_>, node: NodeId)
```

`parse_with_scripts` is `parse_with_slices` with `dom::slices`; the second is the seam for a test
that replaces a slice (the pattern of the reading unit's decision 14). `stopped()` returns the
private field `stop`, the stop recorded by decision 9, one of `scripts-per-document`,
`steps-per-document`, `wall-clock`, `memory`, `faulted`, or `None`; a stop is recorded at the
first script element handed over after it held (or from the answer of `evaluate`, decision 9), so
a stop that held only after the last script is not recorded. `steps` and `scripts` are
`Runner::steps()` and `Runner::scripts()` at the end of the parse, 0 and 0 when no runner was
made.

`parse_with_slices`, in this order:

1. With `options.scripting_disabled` true: `parse_document_bytes(bytes, options)`; its tree and
   encoding, no end, no thread, no runner.
2. Otherwise, on a thread of `config.limits.stack_bytes()`, call the entry of `surf-html` below
   with a closure that makes a hook from the encoding name it is given (called once, or twice
   when the page restarts), then drop the last runner on that thread, and hand back the tree, the
   encoding, and the ends, the totals and any `RunnerError` of the last hook alone. A
   `RunnerError` (the thread, the seeds, the agent, the realm) is a failure of the host and not
   of the page; the call answers `Err(<that error>)` after the parse, with no panic and the tree
   not answered. After its first `RunnerError` the hook keeps it and returns at once from every
   later `run`: it pushes no end, makes no second runner (an agent and a clock thread each time)
   and does not call `make_slices` again.

`make_slices` is held as follows, one form that compiles by reading. `parse_with_slices` puts it
in a `RefCell<F>` made inside the parse thread (so the cell needs no `Send`; `F: Send` is what
carries the factory to that thread), and each hook holds a `&RefCell<F>`, which is `Copy`, so
one `make_hook` closure serves both hooks. A hook borrows the cell mutably and calls
`make_slices` only at the moment it makes its runner. A `&mut F` held by the hook would not
compile: a `FnMut` closure cannot hand out a reborrow of what it captured. The implementor may
change the form. It may not change that the factory is called once for each runner, on the parse
thread, and only when a runner is made (a page with no script that runs calls it never; a page
with a restart calls it once for each parse that ran a script; acceptance 16 and 17 count it).

The entry of `surf-html`, in `crates/html/src/lib.rs`, named after `parse_document_bytes` and
`parse_document_with_hook`:

```rust
pub fn parse_document_bytes_with_hook<H: ScriptHook>(
    bytes: &[u8],
    options: &ParseOptions,
    make_hook: impl FnMut(Option<&'static str>) -> H,
) -> (ParsedDocument, H)
```

It returns the document as `parse_document_bytes` does (`ParsedDocument { tree, encoding }`) and
the last hook it made, so the caller reads what that hook collected. It does what
`parse_document_bytes` does (`lib.rs:324-360`), with a hook; the body is one private function that
both entries call, whose one difference is the closure that parses one pass (`parse_document_bytes`
parses with `tree_builder::parse`; the new entry makes a hook and parses with
`tree_builder::parse_with_hook`). In this order:

1. Cut `bytes` with `limits::cut_bytes` to `options.limits.max_input_bytes`, and decode them with
   `encoding::decode` (`lib.rs:334-335`).
2. Call `make_hook(name)` with the name the decode chose, and parse the decoded text once with the
   hook and the confidence the decode gave (`lib.rs:336-340`). The hook is called once for each
   script element of that pass. With `options.scripting_disabled` true the hook is made and never
   called.
3. When that parse answers `reparse_with` (Context): drop the hook (the caller's runner, ends and
   totals go with it, on the parse thread, before the next step), decode the cut bytes again with
   `encoding::decode_with` (`lib.rs:347-351`), call `make_hook(final name)` a second time, and
   parse the new text once with that hook and the confidence `Certain` (`lib.rs:352-358`). The
   order is drop, then make: the entry holds the hook in an `Option` and takes it out (`take()`,
   which drops it) before it calls `make_hook` again; `slot = Some(make_hook(name))` would make
   the new hook before it dropped the old one, with two runners alive. There is no third parse.
   `make_hook` is called once or twice.
4. When step 1 cut, `Tree::stop_at(Limit::InputBytes)` on the last tree, as `parse_document_bytes`
   does (`lib.rs:325-329`). The tree, `stopped_by` and every mark equal those of
   `parse_document_bytes` for the same bytes and options when the hook writes nothing.

`Config::SESSION` is `Config { limits: RunnerLimits::SESSION, mode: Mode::Session }`
(`Mode::Session` at `runner.rs:126-129 @ f463e24`).

`tree_builder.rs` needs no change for the restart (Context). The comment at `:1104-1106`, the
crate documentation at `lib.rs:170` and the sentence of `docs/design/html-parser.md:422-423`,
which say the second pass takes no hook so that its stack stays inside the depth limit, are
wrong for the new entry and, by the reason in the Context, for every parse. The Change corrects
all three to what the Context says (the pass of a restart may have a hook; the node and step
limits bound the stack, the depth limit bounds the tree) and nothing else in those places.

The hook of `run.rs` keeps the ends, the set of elements it has reported, the recorded stop and
an `Option<Runner>`. `parse_with_slices` makes one hook for each call of `make_hook`; the first
is dropped by the entry when it restarts, so one runner is alive at a time, on the parse thread,
and `ScriptedParse` is built from the last hook alone:

- `skip(tree, node, reason)`: the end `not run: <word>` for the `NotRun` word; the element joins
  the reported set.
- `run(tree, node)`: the element joins the reported set; the source is the concatenation of the
  element's text children (`Tree::children`, `Tree::data`), its length summed before anything is
  copied. Then, by decision 9: when a stop is recorded, the end is `not run: <its word>`;
  otherwise `Runner::stop()` (none when no runner exists yet) gives `faulted`, `memory` or
  `wall-clock` if one holds; else `runner.scripts()` at or over `scripts_per_runner` gives
  `scripts-per-document`; else `runner.steps()` at or over `steps_per_runner` gives
  `steps-per-document` (with no runner both are 0); the first that holds is recorded and is the
  end `not run: <word>`; if none holds, a source over `source_bytes` bytes gives
  `limit: source-bytes`, not parsed, and no stop; otherwise the runner is made if it is not yet
  (below) and `Runner::evaluate(source)` gives the end. When `evaluate` answers
  `not run: faulted`, `memory` or `wall-clock` and no stop is recorded, that word is recorded as
  the stop (decision 9, the race between `stop()` and `script_end`). Around `evaluate` the
  parser's tree is swapped into the bindings' `DomState` and out again; between scripts
  `DomState` holds a placeholder tree. After the end is pushed, each node that `record_inserted`
  listed during the script, its checkpoint included, and that the set does not hold yet, is
  pushed as `not run: inserted` and joins the set. After the first `RunnerError` the hook keeps
  it and returns at once from every later `run` (step 2 of `parse_with_slices`).
- The runner is made on the parse thread with `Runner::new` (made `pub(crate)`), a `Document`
  whose tree is the placeholder `parse_document("", options)`, whose limits are `options.limits`
  and whose slices are `make_slices(DocumentInfo { address, content_type, encoding })`, with the
  encoding name `make_hook` was given (never `None`: `encoding.rs:22-30`; `windows-1252` stands in
  for a `None` that cannot happen).
- `record_inserted(cx, node)` appends `node` to a list in the host value, each node once, at a
  cost that does not grow with the list; the mutation slice calls it from the write that connects
  a `script` element (`scripts-dom-mutation.md:362-366`).

`crates/script/src/runner.rs`: the two intra-doc links [`run`] (`:440,559 @ 8086a30`) become
[`run()`], because `run` is then also a module (`pub mod run;`) and a bare link is ambiguous;
`Runner::new` and a function that runs a closure on the runner's
thread (the part of `run_on` that makes the thread, `:457-486`) become `pub(crate)`; `ScriptHost`
gains the list of `record_inserted`; `Runner` gains `pub(crate)` access to its `Dom`, and
`pub(crate) fn stop(&mut self) -> Option<Reason>`: the first of `faulted` (the field, or the
agent's state, which then sets the field), `memory` (second memory end) and `wall-clock`
(`WallClock::ran_out`) that holds, without running a script; `script_end` calls it for the three
checks at its start (`:691-704`), so the order and the words are one piece of code. A value of the
margin changes only where decision 3 says (`:56,59`), with the pins of `engine.rs:240-250`.

`crates/html/src/lib.rs`: the entry above and the private function both entries call, and the
sentence of the crate documentation at `:170` (`@ a14e3b0`), nothing else.
`crates/html/src/tree_builder.rs`: the comment at `:1104-1106`, nothing else.
`docs/design/html-parser.md`: the sentence at `:422-423` and the paragraph it ends (`:414-423 @
a14e3b0`), which gain the hooked restart and the reason of the Context, nothing else.
`crates/html/tests/bytes_with_hook.rs`, declared in `crates/html/tests/main.rs` (one module per
file, `AGENTS.md`, Layout): acceptance 23. In `main.rs` the line comes after the three modules
the reading unit adds (`tree_state`, `adversary_tree_state`, `adversary_tree_state_2`).

`crates/script/src/bindings.rs` and `crates/webidl/src/prelude.rs`: `DomState::swap_tree(&mut self,
other: &mut Tree)`. `bindings.rs` is generated from `prelude.rs` and holds digests of its sources
(the digest list is `bindings.rs:43-60 @ 8086a30`, `selection.txt` at `:46` and `prelude.rs` at
`:59`; gates `task webidl-check` and `webidl-slow`); the story edits `prelude.rs` and writes
`bindings.rs` again with `slot cargo run -p surf-webidl --bin bindings` (the line is
`bindings.rs:34 @ 8086a30`).

`crates/script/tests/parse_run.rs`: the module of acceptance 1 to 17 and 24.
`docs/design/scripts.md`: the edits of acceptance 19 and 20, that is Q2's sentence on the step
total (`:182-183 @ f463e24`), Q5's sentence on the tree (`:497-499`), Q4's row of scripts run per
document (`:405`), the sentence of Q11's memory bullet on the records of Q7 (`:1015-1016`), the
four lines that name this story as the one that measures (`:10`, the "Settled:" line of Q4
`:444`, the sentence on the margin `:479` and the "Settled:" line of Q5 `:503`), which say
after it that the values were measured by this story on swell's tag `0.10.0` (`:10` keeps
`story:scripts-document-port` as the one that measures again), and the `probe:` lines. The new
text of the Q4 row and of the Q11 sentence says what is true after this story: only a
script that ran can carry a message, and at most 1,000 scripts run in a document
(`scripts_per_runner`), so the messages are bounded as before, each at most 1,024 characters
(Q16); an end `not run` carries one of the fixed words of Q7 and no message, there is one for each
script element the parser reached or a script connected, a node of the tree, so their number is
bounded by the parser's node limit, 1,048,576 (`tree.rs:672 @ a14e3b0`: the arena never gives a
node back). The bytes of the records are stated as a bound: at most 1,048,576 records, each
without a message of at most 256 bytes (chosen: one record of Q7, `surf.document.Script`,
`spec/domains/document.yaml:496 @ a14e3b0`, holds two 16-byte identities, a position, an end, a
reason and optional fields that are empty), so at most 268,435,456 bytes (256 MiB), beside the
536 MiB of the tree of that many nodes (Q11); the probe of acceptance 20 replaces the 256 with
the size it reads of one record. `AGENTS.md`: the entry of
`surf-script` (`:365-368` at `a14e3b0`, which both units rewrite): `run.rs`,
`parse_with_scripts`, the test counts of its log; and the entry of `surf-html` (`:873` on, the
test counts at `:922-929 @ a14e3b0`): the new `tests/bytes_with_hook.rs` and its count. In the
last commit, the public surfaces of acceptance 21.

## Acceptance

Items 1 to 17 and 24 are tests in the module `parse_run` of
`crates/script/tests/parse_run.rs`, run with
`slot cargo nextest run -p surf-script parse_run::`, make no network request and no model call,
parse a page of a few lines with their own runner, and use `Mode::Test`. Each sets
`scripting_disabled: false` in its `ParseOptions` (`ParseOptions::default()` has it true,
`lib.rs:244-251`, and then no script runs), except item 11, which sets it true. A test that needs
limits of its own sets them in `Config`. Each end is compared as `ScriptEnd` and, where a word
is named, as its `to_string()`. No test states a step count of its own. The tests of steps
(3, 4) run scripts that call no DOM member, so the charges of the reading members (the reading
unit's decision 19: 8 steps for each node of a collection made or refilled, one step for 16
bytes of a String answer) do not enter them; a test with a member reads the count from
`ScriptedParse.steps`.
One helper, `assert_stop_word(&ScriptedParse)`, holds the invariant of decision 9 and is called at
the end of every test of the module that answers `Ok`: for each end that is `not run:
faulted`, `not run: memory` or `not run: wall-clock`, `stopped()` is not `None` and its word is
the end's word, so a race that leaves a stop unrecorded fails whichever test it happens in. Items
18 to 22 are checks by command, 23 is in `crates/html/tests/bytes_with_hook.rs`, and 25 is the
folder's bounds.

1. `scripts_per_document`: with `scripts_per_runner` 2, `<script>1</script><script>1</script>
   <script>1</script><p>after</p>` ends `ran`, `ran`, `not run: scripts-per-document`; the `p` is
   in the tree; `stopped()` is `scripts-per-document`.
2. `source_bytes`: with `source_bytes` 100, a script of 101 bytes ends `limit: source-bytes` and a
   later script of 10 bytes ends `ran`; `stopped()` is `None`.
3. `steps_per_document`: let N be `steps` of a page of one script `{ for (let i = 0; i < 100000;
   i++) {} }`, whose `steps_per_entry` ends none of the scripts below. With `steps_per_runner`
   1.5 N, a page of that script three times ends `ran`, `ran`, `not run: steps-per-document`
   (the second script takes the total past the limit and keeps its own end); `stopped()` is
   `steps-per-document`.
4. `checkpoint_steps_count`: let T be `steps` of a page of one script that queues a promise job
   whose body is the finite loop `for (let i = 0; i < 100000; i++) {}`, and U that of the same
   script without the job, U < T; the loop count is chosen so that T is below `steps_per_entry`,
   and the test asserts it (T from a run with `steps_per_entry` at its session value). With
   `steps_per_runner` T, the page of the job script twice ends `ran`, `not run:
   steps-per-document`, and the page of the plain script twice ends `ran`, `ran`.
5. `parse_wall_clock`: with `wall_clock` 100 ms and `steps_per_entry` `u64::MAX`, `while (true) {}`
   ends `limit: wall-clock` after 100 ms (a lower bound, read from the clock) and before 10 s (a
   wait's limit of 100 times the test's own 100 ms, over the 10 times that `AGENTS.md:1965-1967
   @ a14e3b0` asks for; no tighter upper bound is asserted); a later script ends
   `not run: wall-clock`; `stopped()` is `wall-clock`; the markup after both is in the tree. With
   the first case's `wall_clock` and `steps_per_entry` and `source_bytes` 100, the page
   `while (true) {}`, a script of 101 bytes and `1` ends `limit: wall-clock`,
   `not run: wall-clock`, `not run: wall-clock`: a script too long that follows a clock that ran
   out is `not run: wall-clock`, not `limit: source-bytes`.
6. `skipped_reasons`: one page holding `<script></script>`, `<script src=a.js></script>`,
   `<script type=text/template>x</script>`, `<script type=module>x</script>`,
   `<script type=importmap>{}</script>`, `<script type=speculationrules>{}</script>`,
   `<template><script>x</script></template>`, `<script nomodule>x</script>`,
   `<script event=onclick for=window>x</script>`, `<svg><script>x</script></svg>` and
   `<script>1</script>` ends, in document order, `not run: empty`, `src`, `type`, `module`,
   `importmap`, `speculationrules`, `disconnected`, `nomodule`, `event`, `svg`, then `ran`.
7. `eof_recorded`: `<p>a</p><script>x = 1` ends `not run: eof`, one end, and the `p` is in the
   tree.
8. `inserted_recorded`: the test replaces the slice `mutation` through `parse_with_slices` with
   one whose `document_create_element` makes an HTML element with `Tree::create_element_ns` and
   whose `node_append_child` appends with `Tree::append_child` and calls `record_inserted`. The
   page `<script>var s = document.createElement("script"); document.documentElement.appendChild(s);
   </script><script>1</script>` ends `ran`, `not run: inserted`, `ran`, in that order, and the
   inserted element is in the returned tree. Appending the same element twice gives one
   `not run: inserted`. The page `<script id=a></script><script>
   document.documentElement.appendChild(document.getElementById("a"));</script>` ends
   `not run: empty`, `ran`: an element the hook reported gets no second record.
9. `script_sees_the_tree_so_far`: `<p id=a>x</p><script>if (!document.getElementById("a") ||
   document.getElementById("b")) throw new Error("tree");</script><p id=b>y</p>` ends `ran`, and
   both `p` are in the returned tree, so the parser's tree is back with the parser after the
   script.
10. `memory_twice`: M is the least `memory_bytes` at which the realm with the bindings is made,
    measured by the test: it tries 256 KiB, 512 KiB, 1 MiB and 2 MiB in that order and takes the
    first at which the page `1` ends `ran` (a realm that is not made answers `Err`; the heap of a
    realm with the bindings is 202,424 to 506,112 bytes, `docs/design/scripts.md:370-371 @
    f463e24`), and fails with a message when none of them makes it. With `memory_bytes` M, the
    scripts `{ const a = []; for (;;) a.push(new Array(1000)); }` twice and then `1` end
    `limit: memory`, `limit: memory`, `not run: memory`; `stopped()` is `memory`. With
    `scripts_per_runner` 2 as well, the third script still ends `not run: memory`, not
    `not run: scripts-per-document` (the order of decision 9).
11. `scripting_off_same_tree`: with `scripting_disabled` true, `parse_with_scripts` answers
    the tree and encoding `parse_document_bytes` answers for the same bytes, no end, `steps` 0.
12. `dom_limit_then_next_runs`: the test replaces `mutation` with a slice whose
    `document_create_element` answers `Err(DomEnd::Limit(Limit::Nodes))`. A first script
    `document.createElement("div"); 1` ends `limit: nodes`, also inside `try { ... } catch {}`,
    and a later script `1` of the same page ends `ran` (no script inherits the interrupt the
    limit set).
13. `frame_depth_on_parse_thread`: `function f(n) { return 1 + f(n + 1); } f(0)` through
    `parse_with_scripts` with the session's limits ends with a word that starts
    `threw RangeError "frame_depth exceeded"`, and the process is alive.
14. `deepest_nesting_on_parse_thread`: on a thread of exactly `min_stack_bytes()` plus the margin
    the story ships, a script calls the binding `document.getElementById` with an argument
    object whose `toString` calls the same binding again, `reentry_depth` times, and the
    innermost `toString` calls `eval` of a source nested to `parser_nesting` (Q5's test; the
    runner-level cases of `engine.rs:725-` use host functions of the test, no parser frames and
    no generated binding). Each re-entry ends as swell answers it, the end is a word of the closed
    list, and the process is alive. The report gives the test's measured need, the margin with its
    `probe:` line, and, from a probe outside the gate, the same program on a thread below that
    need ending `exit 134 signal ABRT`. `Config` has no stack knob (`stack_bytes()` derives from
    the limits), so the probe gets its thread without a committed change: it builds the same test
    from a copy of the tree made with `git archive` under `.out/probes/` (not the story's tree),
    in which `STACK_MARGIN_FIXED` and `STACK_MARGIN_PER_REENTRY` (`runner.rs:56,59 @ f463e24`)
    are set to 0, so that the thread is `min_stack_bytes()` alone, below the need; the copy is
    deleted after the run and nothing of it is committed. The margin is the need plus a fifth, as
    swell rounds its own; where it changes, `runner.rs` and `engine.rs:240-250` change with it.
15. `document_facts_reach_the_script`: bytes of a page without a doctype and with one inline
    script, parsed with the address `https://example.com/dir/page` and the content type
    `text/html`, ending `ran` when `document.URL`, `document.baseURI`, `document.contentType`,
    `document.compatMode` and `document.characterSet` read `https://example.com/dir/page`,
    `https://example.com/dir/page`, `text/html`, `BackCompat` and `windows-1252`, and `threw` with
    the five values read otherwise; the same page after `<!DOCTYPE html>` reads `CSS1Compat`. The
    page `<!DOCTYPE html><base href=/other/><script>` and a script that throws unless
    `document.baseURI` is `https://example.com/other/` and `document.URL` is still
    `https://example.com/dir/page`, with the same address, ends `ran` (the `href` resolves
    against the address, reading decision 13). The page `<!DOCTYPE html><script>` and the same
    kind of script, behind the bytes 0xEF 0xBB 0xBF, reads `document.characterSet` `UTF-8`, and
    its end is `ran`.
16. `meta_restarts_once_one_agent_at_a_time`: bytes `<!DOCTYPE html><script>1</script><!--`,
    1,100 bytes of `x`, `--><meta charset=windows-1251><title>`, the bytes 0xCF 0xF0 0xE8,
    `</title>` and a script that throws unless `document.characterSet` is `windows-1251`, parsed
    with `parse_with_slices`. The test's slices factory is `FnMut`: it counts its calls, records
    the `DocumentInfo.encoding` it is given and the number of runners alive when it is called, and
    its slices carry a guard whose drop lowers that number. The factory is called twice, with
    `windows-1252` and then `windows-1251`; the number alive is 0 at both calls, so the first
    runner and its slices were dropped before the second was made, on the parse thread (the
    guard's drop records the thread). The ends answered are the second parse's, two `ran`; the
    script after the `meta` read `windows-1251`, so its end is `ran`; `steps` and `scripts` are
    the second runner's (`scripts` 2); the title is U+041F U+0440 U+0438;
    `ScriptedParse.encoding` is `windows-1251`; the tree and `stopped_by` equal
    `parse_document_bytes`'. The page `<!DOCTYPE html><meta charset=windows-1251>
    <script>1</script>` (a `meta` inside the first 1,024 bytes, which the sniffing picks) and the
    page 0xEF 0xBB 0xBF `<!DOCTYPE html><script>1</script>` (a byte order mark) each call the
    factory once, with `windows-1251` and `UTF-8`, and end `ran`. The page above with 200 bytes
    of `y` after the second script and `max_input_bytes` in `options.limits` set 100 bytes below
    its length (the cut falls after both scripts) ends the same, and the tree and its
    `stopped_by`, `InputBytes`, equal `parse_document_bytes`'.
17. `runner_cannot_be_made`: a page of three scripts, `<script>1</script>` three times, parsed
    with `parse_with_slices` and a `Config` whose `limits.engine.memory_bytes` is 0 (the other
    limits are `RunnerLimits::SESSION`'s), the factory counting its calls. The call answers
    `Err(RunnerError::Realm(_))`, and that is the assertion. Swell `0.10.0` answers
    `Terminated(MemoryBudget)` when a realm is made over a budget of 0 (its own test,
    `crates/swell/tests/realms.rs:20-36 @ 0.10.0`, calls `create_realm(None, None)` with 0 and
    1,000); `Runner::new` passes a setup closure for the window and maps any outcome of the
    realm but a normal one to `Realm` (`runner.rs:598-606 @ f463e24`), a failure inside the
    closure as an `EngineFault` termination. So whether the text of the error holds
    `MemoryBudget` depends on the budget failing before the closure runs: the report says what
    the text holds, and no test asserts it. The call does not panic and answers no tree; the
    factory counter reads 1 (the hook kept the first error and returned at once from the second
    and third script, which made no runner); and a second call on the same thread with
    `Config::SESSION` answers `Ok` with three `ran` for the same page, so the thread and the
    process stay usable. If swell makes a runner with a budget of 0, the implementor takes the
    next cause in this order and says which in the report: `memory_bytes` 1,000, then a
    `reentry_depth` large enough that `stack_bytes()` cannot be mapped
    (`Err(RunnerError::Thread(_))`; under overcommit the stack may map lazily, so this one is
    tried last).
18. The checks of the generator. `slot task webidl-check` exits 0 and
    `ESS_TOOLCHAIN=0.97.4 slot --timeout 30m task webidl-slow` finds `bindings.rs` the bytes the
    binary writes; the report names the time `webidl-slow` took, and the job is not started again
    unchanged if it ends at the limit. Both pass on the merged base and at the story's tip: they
    are guards, and no command at the tip shows a transition. The transition is shown by the
    commit order, which the report names: the commit that adds `swap_tree` to `prelude.rs`
    leaves `webidl-check` failing on the digest of `prelude.rs` (`bindings.rs:59 @ 8086a30`),
    and the commit that writes `bindings.rs` again, which comes next, makes it exit 0.
19. The notes of `docs/design/scripts.md`. The base is the merged one: `main` with both
    units merged, where the note holds Q2's sentence of the swell unit and Q5's of the reading
    unit (at `f463e24` the lines below; Q5's phrase is at `:403` at `8086a30` and `83e6b8a`).
    Each phrase sits on one line of the file there, each command prints the lines named on that
    base and prints nothing after the story's change:
    - `git grep -n -F 'no caller enforces it yet' -- docs/design/scripts.md` prints `:183`: Q2's
      "no caller enforces it yet; `story:scripts-document-port` does" names this story instead;
    - `git grep -n -F 'after the tree is taken out of the' -- docs/design/scripts.md` prints
      `:498` (the phrase wraps, and `host value` starts the next line, so the command stops
      before it): Q5 says the tree is the parser's and goes back to it after each script, and
      its test is the test of acceptance 14;
    - `git grep -n -F 'at most 1,000 per document' -- docs/design/scripts.md` prints two lines,
      Q4's row "scripts run per document" (`:405`) and the sentence of Q11's memory bullet on
      the records of Q7 (`:1016`); after the change both say what the Change gives: the messages
      bounded by the scripts that ran, the records bounded by the parser's nodes, and the
      bytes as a bound.
    - ``git grep -n -E 'scripts-parse-run` (and `story:scripts-document-port` )?(measures|measure
      again)' -- docs/design/scripts.md`` prints five lines on that base (`:10`, `:405`, `:444`,
      `:479`, `:503 @ f463e24`; each phrase is on one line there, the sentence of `:479-480`
      wrapping after it) and nothing after the change: the lines say that this story measured
      the values, in the past tense, and `:405` carries the probe's value in place of "measures
      again".
    After the change, `git grep -n -F "bounded by the parser's nodes" -- docs/design/scripts.md`
    prints those two places (the phrase kept on one line in each).
20. The re-measured values of Q4 and Q5, measured with the bindings and the DOM on swell's tag
    `0.10.0` by probes that start without `slot` (acceptance 25) and live in `.out/probes/`: steps
    per second in the test profile and in release, the heap of a realm with the bindings, whether
    a page of more than 1,000 scripts reaches the script count, the size of one record of Q7
    without a message (acceptance 19, Q11), and Q5's margin. The carrier of each is a `probe:`
    line that stands alone, as the existing ones do (`:442`, `:454 @ f463e24`), beneath the Q4
    table or in Q5's margin paragraph; the line names its row, says `holds` or
    `<old> -> <new>` and gives the command and its output; where a value changed, the row
    changes with it. The six lines, by measure:
    - the rate in the test profile and the rate in release: two lines beneath the Q4 table, each
      naming the row "steps per document, summed from `last_entry_steps()`";
    - the heap of a realm with the bindings: a line naming the row `memory_bytes`;
    - whether a page of more than 1,000 scripts reaches the count: a line naming the row
      "scripts run per document";
    - the size of one record: a second line naming that row, which says it bounds the records of
      Q7. The probe prints `size_of::<Script<script_state::Added>>()` of `surf_types::document`
      (`generated/rust/crates/surf-types/src/document.rs:954 @ a14e3b0`, a struct of one
      `ScriptData` and a `PhantomData`, so the size of `ScriptData`, `:897-920`: two identities,
      a position, an end, an optional reason, error kind, message, line, column and dynamic). It
      is the size of the value in memory, the heap of a message not included; the row the
      session stores may differ, and the line says that. The chosen 256 bytes of the Change
      stay until the line gives its number;
    - Q5's margin: a line in Q5's margin paragraph.
    After the commits,
    `git diff -U0 "$(git merge-base main HEAD)..HEAD" -- docs/design/scripts.md |
    /usr/bin/grep -c '^+probe:'` prints at least 6, one for each of the six measures, and the
    report lists them as `holds` or `<old> -> <new>`.
21. Public surfaces: in its last commit, after merging `main`, the story changes every public
    surface its change makes false, each number taken from the `slot task check` log of its tip.
    The rule is: each number on the lines below that the log of its tip gives otherwise. The
    lines are those `git grep -n -E 'passed|tests|test files' -- README.md website/docs
    docs/badges` finds on its base, and at `a14e3b0` they are: the test count `2394 passed`
    (`README.md:11`, `docs/badges/make.sh:38`, `docs/badges/tests.svg:1,2,10`); the sums
    `2,397 tests`, `1,845` and `2,394` (`README.md:285`, `website/docs/what-works.md:17`); the
    row `2,394 passed and 3 ignored: 1,842 passed` (`website/docs/status.md:31`); the line
    `1845 tests: 1842 passed` of the gate block (`README.md:626`,
    `website/docs/how-it-is-built.md:70`); the line `26 test directories, 230 test files`
    (`README.md:612`, `how-it-is-built.md:56`), since the story adds two test files; and the
    paragraph that names the gate run, with its `1845 tests` and `230 test files`
    (`README.md:658-661`, `how-it-is-built.md:102-105`). The line `552 tests: 552 passed` of the
    TLS step (`README.md:628`, `how-it-is-built.md:72`) is not changed by this story and stays,
    unless the log's number differs. After the commit, `git grep -n -F '<old>'` over those files
    prints nothing for each old string the log changes, and each new number equals the log's.
    "After merging `main`" is the order against the stories that write the same lines
    (Placement).
22. The gate and the guards of the merge. `slot task check` exits 0 (true on the merged base: a
    guard); `git diff --stat "$(git merge-base main HEAD)..HEAD"` names only files of this
    story's scope list (the front matter), and the `git diff --name-only` of the same range
    holds no path that list lacks. The reading unit's test of a
    refill the memory limit cuts (at `8086a30`,
    `adversary_dom_reading_2::a_refill_ended_by_the_memory_limit_leaves_one_list`, by the name it
    has on the merged tree) passes: it is the check that the merged `Runner::evaluate` kept the
    block that reads the record of a cut refill into `memory_ends` (`runner.rs:663-677 @
    8086a30`), whichever unit merged first. Each edit of the Change shows by a command that is
    true after it and false before:
    - `git grep -n -F 'takes no hook' -- crates/html/src/lib.rs crates/html/src/tree_builder.rs
      docs/design/html-parser.md` prints nothing (`lib.rs:170`, `tree_builder.rs:1106` and
      `html-parser.md:423` at `a14e3b0`; `:1105` at `8086a30` and `83e6b8a`);
    - `git grep -n -F 'parse_with_scripts' -- AGENTS.md` and
      `git grep -n -F 'bytes_with_hook' -- AGENTS.md` each print a line;
    - `git grep -n -F 'fn swap_tree' -- crates/webidl/src/prelude.rs
      crates/script/src/bindings.rs` prints one line in each file;
    - ``git grep -n -F '[`run`]' -- crates/script/src/runner.rs`` prints nothing.
23. `bytes_with_hook`, module of `crates/html/tests/bytes_with_hook.rs`, run with
    `slot cargo nextest run -p surf-html bytes_with_hook::`; the hook is a recording hook that
    writes nothing. (a) On the page of acceptance 16 (a `meta` after the first 1,024 bytes names
    `windows-1251`, which the sniffing did not pick, with one script before and one after it),
    `make_hook` is called twice, the first time with the tentative name (`windows-1252`) and the
    second time with `windows-1251`, each before the first call of its hook; the entry returns the
    second hook, which saw each of the two script elements once; the first hook saw the script
    before the `meta` only; the tree, `stopped_by` and `encoding` equal `parse_document_bytes`'.
    (b) On a page with a `meta` in its first 1,024 bytes and on a page with a byte order mark,
    `make_hook` is called once, and the tree equals `parse_document_bytes`'. (c) On the page of
    (a) with `max_input_bytes` below its length, the tree and its mark (`InputBytes`) equal
    `parse_document_bytes`'. (d) With `scripting_disabled` true the hooks are made and never
    called, and the tree equals `parse_document_bytes`'. (e) The page of (a) with a second
    `<meta charset=koi8-r>` right after the first: the restarted parse meets a `meta` that names a
    third encoding and does not restart again, so `make_hook` is called twice and no third time,
    the second name is `windows-1251`, and the tree and `encoding` equal `parse_document_bytes`'.
    (f) A page with a script, a comment of 1,100 bytes, a `meta` naming `windows-1251` and then
    `<div>` nested one level past `options.limits.max_depth`, which the test sets to 64 (a test
    of a limit sets a small limit): `make_hook` is called twice, the tree and `stopped_by`
    (`Depth`) equal `parse_document_bytes`', and the call returns without a crash. The case
    shows that the restarted parse stops at the depth limit with the tree and the mark of the
    parse without a hook; it does not prove the stack bound, which the proof of the Change
    carries (the node limit bounds the stack's length, the step limit the reads).
24. `stop_word_holds`, in `parse_run`, the decision 9 cases not in 5 and 10. A page where the
    script count is reached and the clock runs out later keeps `not run: scripts-per-document`
    for every later script. The test lets the bound's own clock run out between two checks, with
    no sleep and no spinning script (a script that spins ends `limit: wall-clock` and records
    the wall clock first): `scripts_per_runner` 1; the page is `<script>1</script>`,
    `<script>1</script>`, a filler of `<p></p>` and `<script>1</script>`. The test sizes the
    filler by measurement: it doubles the number of `<p></p>` until one parse of the filler with
    scripting disabled takes 200 ms or more, and sets `wall_clock` to a quarter of the time it
    measured (50 ms or more), so that the check before the second script, which follows the first
    script by far less than that, finds the clock still running. The test asserts no upper time
    bound: it asserts the words only. If the host preempts it so that the clock runs out before
    the second script (that end reads `not run: wall-clock`, or the first end is not `ran`), the
    attempt does not count and the test tries again with `wall_clock` at half and then at three
    quarters of the measured time, which the filler still passes; only when all three attempts
    end that way does it fail, with a message that says the host preempted it three times and
    gives the three bounds. The filler is capped at 131,072 `<p></p>` (917,504 bytes, 131,072
    nodes), and the test never raises the parser's limits (node limit 1,048,576, input limit
    16 MiB, `limits.rs:100-108 @ a14e3b0`). The arithmetic: the note's bound for the heap of a
    tree is 2 times the stored bytes plus 3 times 136 bytes for each node
    (`crates/html/src/limits.rs:45-51 @ a14e3b0`); the filler stores one byte for each node, so
    it is at most 2 x 131,072 + 408 x 131,072 = 53,739,520 bytes, 51.25 MiB, under 64 MiB
    (the tree's own bound at the defaults is about 536 MiB). If one parse of the filler at the
    cap takes under 200 ms, the test fails with a message that names the size reached (the
    count of `<p></p>`, the nodes) and the time of that parse. The ends of the attempt that
    counts are `ran`, `not run: scripts-per-document`,
    `not run: scripts-per-document`, and `stopped()` is `scripts-per-document`. The
    test prints the time it took, and is named in the report if that is over 5 s.
25. The folder's bounds hold. The test step ends within 120 s and no test runs over 60 s; a test
    over 5 s is named in the report with its time (`parse_wall_clock` waits 100 ms by design; the
    deepest-nesting test and `stop_word_holds` are timed). No model call and no network request
    in a test; each case parses a page of a few lines and makes one runner, and no expensive
    product is made per test (the filler of item 24 is sized once for that test). No Python in
    anything that stays in the repository, the probes included. A script that only measures or
    waits starts without `slot`; a build or a test goes through it. A job in the pool ends within
    10 minutes or names its time with `slot --timeout`, at most 30 minutes, and is not started
    again unchanged. A probe lives in `.out/probes/` of the managed worktree, which git ignores
    (`AGENTS.md:78 @ a14e3b0`), and is not committed. A brief to a sub-agent that runs tests names
    the 60 s bound and the 120 s step budget.

## Out of scope

The members that write (`story:scripts-dom-mutation`, `story:scripts-dom-attributes`,
`story:scripts-dom-selectors`); the runner's mechanisms (`story:scripts-engine-dependency`,
`story:swell-0-9-0`); the harness of html5lib cases (`story:scripts-html-conformance`); the option,
the session and what a user sees (`story:scripts-document-port`); a page meant to break the limits
(`story:scripts-hostile-pages`).

## Placement

After `story:html-script-hook` (on `main`), `story:scripts-dom-reading` and `story:swell-0-9-0`;
before `story:scripts-dom-mutation`, `story:scripts-html-conformance` and
`story:scripts-document-port`, which call `parse_with_scripts` (`scripts-dom-mutation.md:797`,
`scripts-html-conformance.md:54`, `scripts-document-port.md:149-156`) and, for the first, also
`record_inserted`. The document-port story passes the page's address and content type in `Page`,
the conformance story the case's UTF-8 bytes behind a UTF-8 byte order mark (so the entry decodes
them as UTF-8 with confidence `Certain` and no `meta` restarts the parse; `encoding.rs:396-406,
454-462 @ a14e3b0`, `scripts-html-conformance.md:58-65` at the plan tip), `about:blank`,
`text/html` and `Config::SESSION`, and both read `Err(RunnerError)` as a failure of the host.
The records of one document are bounded by the parser's nodes, not by 1,000 (acceptance 19);
`story:scripts-document-port` and `story:scripts-hostile-pages` are told so in their texts.

The merge order of the two units does not matter: the story holds whichever merges first.
`Runner::evaluate` at `8086a30` holds the block that reads the reading unit's record of a cut
refill into `memory_ends` (`runner.rs:663-677`); at `f463e24` (`:670-689`) it does not. The unit
that merges second keeps the block, and the check that it did is the reading unit's test of
acceptance 22. The reading story's scope has grown since `8086a30` (three more test files); that
tip is not its last.

What the story shares with other open stories, by file. Where it says "no edge", no story names
the other as a dependency, and the merge is the ordinary one: the story that merges second takes
the merged file and resolves what overlaps.

- `crates/script/src/runner.rs`: this story, `story:scripts-dom-reading` (where `ScriptHost` is
  made, `:171,585 @ 8086a30`), `story:swell-0-9-0` (`evaluate`) and the draft
  `story:scripts-job-errors` (`host_clean_up_after_job`, `scripts-job-errors.md:49`), which
  depends on `story:swell-0-9-0` and has no edge to this story, and the draft
  `story:scripts-event-loop-and-timers`, which takes `HostError::MemoryBudget` out of
  `surf_defect!()` there (`scripts-event-loop-and-timers.md:30-31`) and is ordered after this
  story through `story:scripts-document-port`. This story reads the merged file.
- `crates/script/tests/engine.rs`: this story (the pins at `:240-250`), `story:swell-0-9-0`
  (a dependency: three ignored tests at `:341-378` and the cases of its acceptance 3,
  `swell-0-9-0.md:296,658`) and the draft `story:scripts-seeds-and-time-zone` (removes the
  `#[ignore]` lines, `scripts-seeds-and-time-zone.md:16,37`, a different region). No new edge;
  the story that merges second takes the merged file.
- `crates/script/src/bindings.rs` is generated from `crates/webidl/selection.txt`,
  `crates/webidl/src/prelude.rs` and the generator (`crates/webidl/src/`, the files listed at
  `bindings.rs:51-60 @ 8086a30`); its header holds their digests. Six stories write that set,
  the reading unit merged before this story: `story:scripts-dom-reading` (`generate.rs`,
  `prelude.rs`, `bindings.rs`; `scripts-dom-reading.md:646-660`), this story (`prelude.rs`,
  `DomState::swap_tree`), `story:scripts-dom-attributes` (proposed; `prelude.rs`,
  `strings_value`, `live` and `peak`; `scripts-dom-attributes.md:497-503,897-899`; no edge to
  this story, which it names as the sharer), `story:scripts-utf16-length` (`prelude.rs`,
  `string_length`; `scripts-utf16-length.md:18-41`), `story:scripts-dom-collection-names`
  (`selection.txt`; `scripts-dom-collection-names.md:17`) and `story:html-open-namespace`
  (draft; `selection.txt`, where the note of the deviation on `setAttributeNS` goes,
  `html-open-namespace.md:19,61`; it depends on `story:scripts-dom-attributes` and has no edge
  to this story); the swell unit leaves the comments of `prelude.rs` out
  (`swell-0-9-0.md:327-331`) and does not write the set. The drafts have no edge to this
  one. No new edge. The rule:
  `bindings.rs` is never merged by hand; the story that merges second writes it again from the
  merged sources with `slot cargo run -p surf-webidl --bin bindings`, and `slot task webidl-check`
  (the digests) and `webidl-slow` (the bytes) in its gate hold it (acceptance 18).
- The public-surface lines of acceptance 21 (`README.md:285`, `docs/badges/make.sh:38`,
  `docs/badges/tests.svg`, `website/docs/status.md`, `what-works.md`) are rewritten in the last
  commit of this story and of `story:scripts-dom-attributes` and `story:scripts-dom-selectors`
  too (`scripts-dom-attributes.md:64-72`, `scripts-dom-selectors.md:72-80`), which also write
  other files of `crates/script`; `story:scripts-dom-mutation` rewrites them, `status.md:31` and
  `what-works.md:17` in its acceptance 22 (`scripts-dom-mutation.md:981-999`; its scope list
  names none of them), and `story:scripts-html-conformance` and `story:scripts-document-port` have
  `website/docs/status.md` and `what-works.md` in their scope. Those three depend on this
  story, so the order exists: this story merges before them and each takes the log of its own
  tip. For the others, no edge: acceptance 21's "after merging `main`" is the order, and the
  last to merge takes the numbers of its own log. The entry of `surf-script` in `AGENTS.md`
  (`:365-368 @ a14e3b0`) is edited by this story, by `story:scripts-dom-attributes` (`AGENTS.md`
  in its scope, `scripts-dom-attributes.md:597`) and by `story:scripts-dom-selectors`: no edge;
  the one that merges later takes `main` first and re-reads the test count from its log.
- `AGENTS.md` and `website/docs/status.md` are in the scope of `story:conformance-lock-and-lints`
  (draft); `AGENTS.md` is also in that of `story:session-argument-checks` (draft),
  `story:test-step-holds-with-tls` (active) and `story:tls-tests-as-tables` (draft, which changes
  the TLS count behind `552` in `README.md:285` and `what-works.md:17`). No edge; the same rule:
  the last to merge re-reads the counts from its own log.
- `crates/html/src/lib.rs`: the other story that changes it is `story:scripts-dom-reading`
  (status active; one line, the `pub use` of `QuirksMode`, at `8086a30`), which also changes
  `tree_builder.rs` and adds three modules to `crates/html/tests/main.rs`; it is a dependency of
  this story, so it is merged before. Every other story whose scope names `lib.rs` is
  implemented. This story reads `lib.rs`, `tree_builder.rs` and `main.rs` as the reading unit
  leaves them.
- `crates/html/src/tree_builder.rs`: the draft `story:html-parser-hook-cost` changes it and
  `tree.rs`, not `lib.rs`; its scope is inferred, and the comment at `:1104-1106` belongs to the
  doc comment of `stop_parsing`, which says "The pops are not steps". No edge; the one-comment
  hunk is resolved by whichever merges second. This story changes `tree.rs` not at all.
- `docs/design/html-parser.md` (the paragraph at `:414-423 @ a14e3b0`): also in the scope of
  `story:scripts-document-port`, `story:scripts-html-conformance`, `story:select-content-rows`
  and `story:html-parser-hook-cost`. No edge; the story that merges second resolves a conflict
  in that paragraph, if the other edited it.
- `docs/design/scripts.md`: Q2, Q4, Q5 and Q11, which acceptance 19 and 20 edit, are also
  edited by the two units (the Q4 rows "after `Terminated(MemoryBudget)`" and "wall clock", the
  reading unit's `[SameObject]` paragraph beside the swell unit's rewrites of Q2 and Q4); this
  story edits the merged text, and the lines it cites at `f463e24` move. Other open stories
  edit the file too, with no edge beyond the dependencies named:
  `story:scripts-document-port` (scope; Q11's bound values; it depends on this story, so the
  order exists), `story:scripts-dom-attributes` (scope; `:322-332`, `:902-904`),
  `story:scripts-dom-mutation` (Q1, Q4, `scripts-dom-mutation.md:733-738`; depends on this story),
  `story:scripts-job-errors` (draft; Q2, the checkpoint deviation, `scripts-job-errors.md:19`,
  the note this story edits at `:182-183 @ f463e24`), `story:scripts-seeds-and-time-zone`
  (draft; the `Math.random` and time-zone rows, `:32,40`), `story:session-memory-bound` (draft;
  Q11, `:70`) and `story:scripts-html-conformance` (Q3 and Q15, `:56,68`). The second to merge
  resolves Q2, Q4, Q5 and Q11 on the merged text.

## Why `epic:scripts-in-a-page`

The epic's outcome runs a page's inline scripts during its parse, with limits per document; this
is that run.
