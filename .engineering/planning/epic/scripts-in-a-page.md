---
format: aep.planning-md/3
id: epic:scripts-in-a-page
kind: epic
status: draft
title: 'Scripts in a page: surf runs a page''s inline scripts with swell'
summary: surf embeds the organization's script engine swell and runs a page's inline classic scripts while parsing, against a minimal DOM and under limits per page, so that what a script writes is what surf and surf-agent show.
relations:
- depends_on: story:html-document-port
revision: 5
---
## Context

`AGENTS.md` ("Implementation targets"), the operator on 2026-11-21: the JavaScript engine "could
also be just, I guess, its own new project [...] It should be separate, I guess, from Surf itself
[...] Requirements are for sure influenced by Surf and its needs". So "the script engine is a
repository of its own that surf embeds, with its requirements written from surf's needs". That
repository is swell (`https://github.com/beyond10x/swell`), an ECMAScript engine written from
scratch in Rust, "written to be embedded by surf" (swell `README.md`).

The order, decision D77 of 2026-11-22 (option A), given for the operator by the session
`host-operator`: surf starts embedding swell after wave 11 of `docs/plan/agent-waves.md`, as this
epic, planned from now beside the running waves, with no build slot for it until the HTML parser
delivers a real page's document (`story:html-document-port`, wave 11).

What exists in surf: a `script` element is parsed and never run. `docs/design/html-parser.md`
item 9 ("The script hooks") says, at the `script` end tag in the "text" insertion mode, "pops the
element and returns to the original insertion mode; nothing is prepared or run, the parser never
pauses", and parses every page with parser scripting disabled true (`surf_html::ParseOptions`,
default `scripting_disabled: true`), so `noscript` content is markup in the tree.
`story:html-document-port` says: "a page that builds its content with script shows only what its
markup holds". `spec/components.yaml` holds the marker "no component for script execution [...]
None is declared until its first command is." `epic:detailed-mode` waits for this epic: "nothing
mutates a document's tree before one exists".

What is read of swell, every figure from its published branch, `origin/main` at `6a43e18`
(2026-11-21), whose last release tag `0.2.0` is commit `29f674c`:

- It runs scripts of the ES5 grammar under a step budget and a memory budget; the first two of
  nineteen delivery slices are delivered; modules are slice 11 "and are not parsed yet"; next is
  slice 3, the core library (`README.md`). Not in 0.2.0: regular expression literals, `let` and
  `const`, arrow functions, classes, promises, and most builtin methods (`JSON.parse`, `Date`,
  `Math` but `max`); "A script that uses one is refused with a `SyntaxError` that names the
  delivery slice that brings it" (`CHANGELOG.md`, 0.2.0, "Limits of this release").
- "Not a browser. The DOM and everything else a web page sees beyond the language belong to the
  program that embeds swell" (`README.md`). Web IDL's conversions and the DOM are surf's
  (`docs/design/engine.md`, "Not in scope").
- The host interface is `docs/design/host-interface.md`: 164 numbered requirements `HOST-<n>`;
  `conformance/host/requirements.txt` gives 80 of them a test and 84 `pending slice <t>`.
- engine.md decision 9: "What surf must have before it can use the engine at all: a mutable DOM
  [...], Web IDL bindings for at least `Window`, `Document`, `Node`, `Element` and `EventTarget`,
  the HTML event loop with its task queues and the microtask checkpoint, and script processing in
  the parser". And: "A script that touches no DOM and uses only ES5 can run after slice 4 [...] A
  script on a real page becomes realistic after slice 18 on swell's side".

## Outcome

A page that surf loads runs its inline classic scripts while it is parsed, and what the scripts
wrote is what surf shows. Two examples, each served by a test on 127.0.0.1:

1. `<title>Before</title><script>document.title = "Written by script"</script>`: with scripts on,
   `surf-agent look` answers `page 200 "text/html" "<address>" "Written by script"`, and
   `surf <url>` prints `title <document-id> Written by script`. With scripts off both show
   `Before`, as today.
2. `<ul id=l></ul><script>var a = document.createElement("a"); a.setAttribute("href", "/next");
   a.textContent = "Next"; document.getElementById("l").appendChild(a);</script>`: `surf <url>`
   prints an `element` row of kind `link`, label `Next`, target `/next`, which the markup does not
   hold. `surf-agent` lists it once the session lists links (`story:agent-follow-links`).

In detail:

- surf depends on swell in the form the operator chooses (decision S1 below), and `lock-check`
  states that dependency exactly.
- At a `script` end tag the tree builder hands a parser-inserted inline classic script to a hook;
  surf's hook evaluates it with swell against the live tree, then parsing goes on over the tree
  the script left.
- A minimal DOM, a named list of members (decision S4), bound to the `surf-html` tree.
- Each script's end is recorded with its document (ran, threw, refused by swell, ended by a
  limit, not run and why), and a page whose script fails still shows everything else.
- Limits per script, per document and in wall time, each ending in a defined outcome.
- The html5lib cases tagged `needs-script` that need no `document.write` run and pass.

## In and out of the first version

In: inline classic scripts (no `src`, `type` absent or a JavaScript MIME type essence), run
synchronously at their end tag in document order, in one realm per document whose global is a
`Window` host object; the minimal DOM; per-page limits; parser scripting enabled for a document
whose scripts run, and its effect on `noscript`.

Out, each with its reason:

- `async`, `defer` and module scripts (`type=module`): swell does not parse modules yet (slice 11,
  `README.md`), and `async`/`defer` need the end of parsing with its deferred scripts and events
  (`html-parser.md` item 9, `the-end scripts-and-events`). A `nomodule` classic script is skipped,
  as the HTML Standard says (from memory, to be confirmed by `scripts-decisions`).
- External scripts (`src`): a fetch per script is a new request kind for `surf.net` (subresource,
  not navigation), its own story and its own security review; parser-blocking fetches also need
  the parser pause.
- Timers and the event loop: no task queue exists in surf, and swell's job hooks and the microtask
  checkpoint are slice 10, all pending (HOST-77 to HOST-84). Scripts run only while parsing, so
  D2 (`docs/plan/agent-plan-revision-1.md`: no mutation after parsing) still holds afterwards.
- DOM events and handlers (`addEventListener`, `onclick`): they need the event loop, and an agent's
  action firing script is a new path in `surf.agent`; after this epic.
- `document.write`: it needs the insertion point and a re-entrant tokenizer
  (`the-input-byte-stream insertion-point`); the 3 html5lib cases that use it stay not run.
- The Web API surface beyond the named minimal DOM: `fetch`, `XMLHttpRequest`, storage, cookies,
  `location`, `history`, `navigator`, `console`, `innerHTML` (fragment parsing from script). Each
  is a capability a script gains and is decided on its own later.
- Anything that needs layout or style: `getComputedStyle`, `offsetWidth`, `style`. surf computes
  no style (`spec/domains/render.yaml` marker).

## The dependency rule

`AGENTS.md` ("Goals", decided by the operator on 2026-11-20, option C): "No third-party crates in
anything that ships" and "The root `Cargo.lock` proves the rest: it lists no package from outside
this repository". `task lock-check` fails on every root `Cargo.lock` package with a `source` line,
that is one from a registry or a git repository; "A path dependency outside the repository has no
`source` line and is not caught."

swell is the organization's own and itself ships no third-party code: its root `Cargo.lock` at
`6a43e18` lists 7 packages, none with a `source` line; HOST-1 ("name no package from outside the
repository in the root `Cargo.lock`") has a test there (`source_scan::host_1_...`). A host would
depend on the crate `swell`, which brings `swell-compiler`, `swell-syntax`, `swell-num` and
`swell-unicode`, each by path (`crates/*/Cargo.toml` at `6a43e18`); `swell-test262` and
`swell-unicode-gen` are tooling, and swell's `conformance/ess` workspace, the one place it allows
third-party crates, is outside its root workspace and not reached. `swell` has the features
`collect-every-safe-point` and `fault-injection`, test modes, which surf does not enable. The
crates are not on a registry; swell's `CHANGELOG.md` (0.2.0) says to "depend on the repository at
the tag: `swell = { git = "https://github.com/beyond10x/swell", tag = "0.2.0" }`".

So no third-party crate enters, but the sentence "lists no package from outside this repository"
stops being true in forms a and b below, and `lock-check` must change in form a. This is a change
of architecture and the operator's decision (S1). Two things hold in every form: swell's
`[profile.*] overflow-checks = true` (its root `Cargo.toml`) does not apply inside surf's build,
because cargo reads profiles from the root workspace only (from memory), so surf sets
`overflow-checks = true` for the swell packages in the profiles it ships; and swell is Apache-2.0
with a `NOTICE`, which surf's release archives carry beside its own (Apache-2.0 section 4(d), from
memory).

## Security

Scripts are untrusted input, like the bytes the parser reads.

What swell offers, by its committed documents at `6a43e18`, each with a test there unless marked:

- No ambient authority: the engine crates use no clock, environment, file, network, process or
  random state (HOST-2), forbid unsafe code and spawn no thread (HOST-1); "swell has no other way
  to reach files, the network, time or randomness" (`README.md`). A script reaches only what the
  host defines.
- Limits per agent (decision 2): `steps_per_entry`, `memory_bytes` (the agent's heap over its
  life), `frame_depth`, `reentry_depth`, `parser_nesting`, `string_length`, `bigint_bits`; 0 is a
  limit of zero. Each ends in an outcome the host reads (HOST-6 to HOST-20, HOST-22); a nested
  entry spends from the outermost entry's budget (HOST-8); after a budget or an interrupt the
  agent stays usable (HOST-70).
- An `InterruptHandle` that another thread may set (HOST-90 to HOST-96), for a wall-clock bound.
- Misuse is a defined answer: stale or foreign handles answer `Err` (HOST-30, HOST-31, HOST-45); no
  entry point panics for any argument (HOST-3, a fuzz test); a broken invariant faults the agent
  for good (HOST-48, HOST-160).
- `host_ensure_can_compile_strings` lets the host refuse `eval` and `Function` (HOST-116, HOST-117).
- Determinism: a run is a function of limits, seeds, calls and hook answers (HOST-133).
- Its own stated gap (`CHANGELOG.md` 0.2.0): "the comparisons of that scan are not charged to the
  step budget: a script with very large objects runs longer than its step count says". So the step
  budget alone does not bound time; surf's wall-clock interrupt does.

What surf must add:

- One agent per document, created at its first script and dropped with the document, so no state
  passes from one page to the next. An `Agent` is not `Send` (HOST-136): the agent, and with it
  the parse of that document, runs on one thread whose stack is at least
  `Limits::min_stack_bytes()` (HOST-17). swell measured 14,672,640 bytes for `reentry_depth` 1,000
  and `parser_nesting` 1,000 in a debug build (`CHANGELOG.md` 0.2.0); surf's own profile has
  debug information off (`.cargo/config.toml`), which swell has not measured.
- Limits per page, proposed for `scripts-decisions` to confirm and measure: the agent's limits,
  a step total per document summed from `last_entry_steps()`, a number of scripts run per
  document, a wall-clock bound per document through the interrupt handle, and every DOM write
  held to the parser's limits (`html-parser.md` item 7: nodes, depth, attributes, stored bytes).
  A limit reached ends that script; parsing goes on, and the document records which limit.
- What a script may reach: the minimal DOM of its own document and nothing else. No host function
  for the network, the session, its token, storage, other documents or time beyond the clock hook
  (`host_system_utc_epoch_nanoseconds` is a required hook, HOST-125, pending slice 4).
- The process boundary: surf runs one `surf-session` process per session (`AGENTS.md`, "Agent
  target"). The first version runs scripts inside it (decision S2), so an engine defect that
  escaped the language would reach that session's process, never another session's. The process
  per component of the isolation target stays blocked on beyond10x/ess#792.
- Scripts off by default until the operator decides otherwise (decision S3).

## The gaps

Requirements of swell's host interface surf needs first, by `conformance/host/requirements.txt`
at `6a43e18`:

- With a test there: HOST-1 to HOST-3 (authority, panics), HOST-6 to HOST-20 and HOST-22
  (budgets and limits), HOST-28 to HOST-37 (roots, strings, sources, outcomes), HOST-45, HOST-47,
  HOST-48, HOST-160 (misuse), HOST-49 to HOST-54 and HOST-161 (realms with a host global, scripts),
  HOST-69 to HOST-72 (termination), HOST-90 to HOST-96 (interrupt), HOST-102 (reads),
  HOST-103 to HOST-111 (host functions, host objects with brand, host data and `trace`),
  HOST-116 to HOST-118 (compiling strings), HOST-124, HOST-129, HOST-132, HOST-133, HOST-136.
- Pending, needed soon after: HOST-155 (`globalThis` is the setup's `this`, slice 3);
  HOST-125 to HOST-128 (clock and time zone, slice 4); HOST-26, HOST-27, HOST-144, HOST-147
  (`Math.random` from the seeds, slice 4); HOST-112 to HOST-114 (internal-method overrides,
  slice 9), without which a live `HTMLCollection` or a `WindowProxy` cannot be built, so the first
  version returns static snapshots (a deliberate deviation); HOST-77 to HOST-84, HOST-100,
  HOST-101, HOST-121, HOST-148, HOST-149 (promise jobs and the checkpoint, slice 10).

Requirements from surf's needs, which no requirement of `host-interface.md` at `6a43e18` states
(searched for "line number", "column", "position", "charge" and "weak handle"; each is surf's
proposal to the swell session):

1. A parse error and an uncaught thrown error can be read with the line and column of the
   script's source where each arose, so that surf can report where a page's script failed.
2. Every operation whose work grows with the size of its operands, the property scan named in
   0.2.0's limits included, charges steps in proportion, so that `steps_per_entry` bounds an
   entry's time.
3. The host can charge and credit bytes it holds for the agent (DOM nodes and text behind host
   data) against `memory_bytes`, so that a script creating nodes without end ends in
   `Terminated(MemoryBudget)`.
4. The host can hold a reference to a host object that does not keep it alive and reads as gone
   once it is collected, so that a node-to-wrapper map keeps one wrapper per node without a
   `Root` per node (inferred from engine.md decision 9, "Rooting").
5. A read returns an error object's kind and its own `message` data property without running
   code or calling a hook, so that surf can record a thrown value after its run ended.
6. HOST-17, HOST-152 and HOST-163 hold under the embedding program's build profiles (release, and
   debug information off), each shown by a test in that profile.
7. Each release's `CHANGELOG.md` section names every change to a signature or a requirement of
   the host interface, so that surf moves its pinned version knowingly.

## What must exist in surf first

- The document: `story:html-document-port` (wave 11) gives `Browser` a parser step that parses
  each committed HTML body and keeps the tree per document; this epic runs scripts inside that
  step, before its ending command, so the element rows are computed from the tree the scripts
  left. `story:html-tree-tables` (wave 11) for the foster-parenting case below.
- The tree builder at a `script` end tag: today it pops and returns (`crates/html/src/
  tree_builder.rs`, module doc). It must, when parser scripting is enabled, take the steps of the
  "text" insertion mode the first version keeps (pop, return to the original insertion mode,
  prepare the script element: classic, inline, parser-inserted, not `nomodule`), call a hook with
  the tree, and go on with its stack of open elements and formatting list over the tree the hook
  left, including nodes a script removed. `Tree`'s writes are `pub(crate)` today
  (`crates/html/src/tree.rs`); the hook needs a public write surface bounded by the limits.
  The `html-sections` line `tree-construction script-processing` stays open for the steps not
  taken (parser pause, pending parsing-blocking script, nesting level beyond 1).
- The scripting flag: `ParseOptions.scripting_disabled` exists and the conformance harness
  already runs `#script-on` cases with it false (`crates/html-conformance/src/runner.rs`), so the
  flag changes no html5lib expectation. A document whose scripts run is parsed with it false:
  `noscript` in "in head" and in body is then a raw text element, its content one text node, so
  links inside it give no element row; and `surf-page`'s hidden rule, which says "`noscript` is not
  hidden, because surf parses with scripting disabled" (`html-parser.md` item 5), takes `noscript`
  as hidden in that mode.
- The html5lib expectations: 7 cases are `not-run:needs-script` (`crates/html-conformance/
  expected/tree-construction.txt` and `encoding.txt`). Four need only a DOM:
  `scripted_adoption01.dat` (`getElementById`, the `id` setter), `scripted_ark.dat`
  (`getElementsByTagName`, an index, `setAttribute`), `scripted_foster01.dat` 2 cases
  (`querySelector("table")`, `documentElement`, `remove`, `appendChild` on the document). They
  become run and pass. Three need `document.write` (`scripted_webkit01.dat` 2,
  `encoding/scripted/tests1.dat` 1) and stay not run.

## Candidate stories, in dependency order

Drafts for the decomposition, not yet artifacts:

1. `scripts-decisions`: a design note `docs/design/scripts.md` settles the first version's
   boundary, the minimal DOM member list with each member's section of the DOM Standard, the
   steps of the `script` end tag taken and left out, the limits with measured values, seeds and
   clock, the specification shape of a script's end, and the requirements handed to swell; it
   writes the answers to S1 to S4 as proposals. Writes only `docs/design/scripts.md`. A planning
   story: it may run before wave 11 ends.
2. `scripts-engine-dependency`: surf depends on swell in the form S1 chose; `lock-check` states it;
   a new crate `crates/script` (`surf-script`) creates an agent with surf's limits and evaluates a
   source with no DOM, tested on an endless loop, an endless allocation and an interrupt. Touches
   root `Cargo.toml`, `Cargo.lock`, `Taskfile.yml` (`lock-check`), `crates/script/` (new),
   `NOTICE`.
3. `scripts-spec`: `surf.document` records each script's end; the session gains the scripting
   choice of S3 (`surf.agent`, `cli/surf-agent.yaml`); `task generate`. Touches
   `spec/domains/document.yaml`, `spec/domains/agent.yaml`, `cli/surf-agent.yaml`,
   `spec/scenarios/`, `generated/rust/`.
4. `html-script-hook`: the tree builder calls a hook at a parser-inserted inline classic script's
   end tag when scripting is enabled, and `Tree` gains a public, bounded write surface. Touches
   `crates/html/src/tree_builder.rs`, `crates/html/src/tree_builder/modes.rs`,
   `crates/html/src/tree.rs`, `crates/html/src/lib.rs`.
5. `scripts-minimal-dom`: `surf-script` binds the global `Window`, `Document`, `Node` and `Element`
   members of the list to a `surf-html` tree as branded host objects, each write held to the
   limits. Touches `crates/script/`. Needs 2 and 4.
6. `scripts-html-conformance`: the harness runs the four DOM-only `needs-script` cases through
   `surf-script` and the expected counts rise. Touches `crates/html-conformance/` (`src/`,
   `expected/`, `Cargo.toml`) and the `html-sections` block of `docs/design/html-parser.md`.
   Needs 5 and `story:html-tree-tables`.
7. `scripts-document-port`: `Browser`'s parser step runs a document's scripts on an agent thread
   with the per-page limits and a wall-clock interrupt when the session's choice says so, writes
   each script's end, and `surf <url>` and `surf-agent look` show the result (the two examples
   above). Touches `crates/surf/src/lib.rs`, `crates/surf-cli/`, `crates/session/`,
   `crates/page/src/lib.rs` (the `noscript` rule), `conformance/src/target.rs`,
   `docs/agent-answer.md`. Needs 3, 5 and `story:html-document-port`.
8. `scripts-hostile-pages`: pages with an endless loop, endless allocation, endless node creation,
   deep recursion, 10,000 script elements and a script over the source limit each end with a
   recorded limit and the rest of the page parsed, every test under 5 s. Touches
   `crates/surf/tests/`, `crates/surf-cli/tests/`. Needs 7.

At the same time: 2, 3 and 4 after 1 (disjoint files; 2 waits for S1); 6 and 7 after 5 (disjoint
crates; 6 also waits for `html-tree-tables`, 7 for `html-document-port`).

## Open decisions for the operator

S1. The form of the dependency on swell (a change of architecture).
- a. A git dependency on a release tag, as swell's `CHANGELOG.md` says. Cost: `lock-check` gains
  an allow-list of exact sources (`git+https://github.com/beyond10x/swell?tag=<tag>#<commit>`) and
  the "Goals" sentence becomes "no package from outside the beyond10x organization"; a cold build
  and CI fetch from the forge, as `conformance/` already does for ess; a version moves by one line
  and the lock.
- b. A vendored copy of the five crates at a tag under `vendor/swell/`, with a `SOURCE` file of
  checksums as `crates/html-conformance/data/` has, checked by the gate. Cost: about 1.1 MB of
  source without tests (the five crates hold 1,932,533 bytes in 627 files at `6a43e18`, 824,571
  of them in 540 test files), a refresh per swell release, a check that nobody edits it;
  `lock-check` stays as it is.
- c. A path dependency on a sibling checkout. Cost: a fresh clone and CI cannot build, and
  `lock-check` would not see it. Not recommended.
Default: a.

S2. Where a page's scripts run.
- a. In the session process, one agent and one thread per document. Cost: an engine escape reaches
  that session's process; no new process kind. swell's engine.md open item D recommends the
  engine and the DOM in one process.
- b. A script process per session or per document holding the DOM, the session process holding
  the rows. Cost: a new process kind, a protocol and its tests; the tree crosses a boundary.
- c. The engine in its own process, the DOM in the session process, one message per DOM access.
  Cost: a message on most statements (engine.md item D, not measured).
Default: a.

S3. When scripts run.
- a. Always, for every HTML document. Cost: pages whose scripts swell refuses lose their
  `noscript` fallback, and every answer changes for such pages.
- b. Per session, an option of `surf-agent open` and of `surf`, off by default. Cost: one input in
  the specification and the command line, both modes tested.
- c. On by default, with an option to turn them off. Cost as b, and a's loss by default.
Default: b, until swell's delivered slices run real pages' scripts.

S4. The DOM of the first version.
- a. A named list (about 25 members of `Window`, `Document`, `Node`, `Element`: those of the two
  examples and the four html5lib cases, plus `textContent`, `createTextNode`, `insertBefore`,
  `removeChild`, `getAttribute`), collections as static snapshots, a deviation until HOST-112 to
  HOST-114. Cost: real pages that use more throw `TypeError`, recorded.
- b. The whole `Node`, `Element` and `Document` interfaces from their Web IDL. Cost: a Web IDL
  reader and binding generator first, and live collections waiting for swell slice 9.
- c. The named list, with no collection member until swell slice 9. Cost: `scripted_ark.dat`
  stays not run.
Default: a.

## Out of scope

Everything the first version leaves out above; style, layout and rendering; workers (swell
engine.md open item B); a script engine other than swell; any change in swell's repository, which
is the swell session's, given the requirements above.

## Constraints

- Specs first: the specification changes (`scripts-spec`) are green before the code that fills
  them. No third-party crate; swell enters only in the form S1 chose. `#![forbid(unsafe_code)]`.
- A script never hangs, panics or exhausts the session process: every limit ends in a defined,
  recorded outcome, and the rest of the page is parsed.
- Tests start their servers in Rust on 127.0.0.1; no test over 60 s, the test step within 120 s;
  a test over 5 s is named with its time. No Python.

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. `lock-check` passes and names the swell dependency in the form S1 chose.
2. `task html-conformance` prints the four DOM-only `needs-script` cases as run and passed, and
   `not-run:needs-script` counts 3 cases, each needing `document.write`.
3. Named tests over the two examples of "Outcome": with scripts on, `surf <url>` prints the title
   the script wrote and the element row of the link it added, and `surf-agent look` prints that
   title in its page line; with scripts off, both print what the markup holds.
4. Each page of `scripts-hostile-pages` ends with its limit recorded and the page's other elements
   present, each test under 5 s.
5. Both conformance reports hold at least one passed scenario for every outcome of the commands
   that record a script's end.

## Decisions taken

- S1, the form of the dependency: option a, a git dependency on a release tag of swell. The operator, 2026-11-22 04:51 CET, relayed by the session `host-operator`: "A: git dep on a swell tag". `lock-check` allows that one source by its exact address and tag, and the sentence on dependencies becomes "no third-party crate; one crate of the organization".
- S2, where a page's scripts run: option b, a script process of its own per session that holds the DOM; the session process holds the rows. The operator, 2026-11-22 about 04:55 CET, relayed by the session `host-operator`: "B: separate script process". This is not the default the epic named (a). What follows from it, to be written into the sections above when S3 and S4 are answered, in one revision: the sentence in "Security" that the first version runs scripts inside the session process is replaced (an engine defect that escaped the language reaches the script process, which holds no token and no socket of the session); a new process kind (a fifth binary in the release archives, or a mode of `surf-session`: a point of `scripts-decisions`), a protocol between the session process and the script process with its specification and tests, and the lifetime rules (started with the first document that has a script, ended with the session, a crash reported as a failure of the script and not of the session); the candidate stories gain one for the process and its protocol before `scripts-minimal-dom`; the reduced environment, restricted privileges and resource limits of the section "From the security review of 2026-11-22" apply to the script process first.
- S3, when scripts run: option b of the list above (put to the operator as "A"), per session, an option of `surf-agent open` and of `surf`, off by default, both modes tested. The operator, 2026-11-22 05:12 CET, relayed by the session `host-operator`: "A: option, off by default". It was the default the epic named.

## From the security review of 2026-11-22

A security review of surf and swell of 2026-11-22 names two conditions for running page scripts. The operator forwarded it: "Fix Swell's resource limits before Surf runs untrusted JavaScript."

- surf runs page scripts only on a release tag of swell in which parsing and compilation are charged to the memory budget before they allocate, and property operations (enumeration of string and object keys, property lookup) charge their work and their temporary storage before they do it. The swell session has the findings and plans them as stories; the tag that holds the fixes is named here when it exists.
- Before a session process runs a script, its environment is reduced to what it needs (today it inherits the caller's environment but two variables of surf), its privileges of the operating system are restricted, and it has resource limits of its own. This is part of the candidate story `scripts-hostile-pages` or a story before it.
