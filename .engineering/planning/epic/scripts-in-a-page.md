---
format: aep.planning-md/3
id: epic:scripts-in-a-page
kind: epic
status: draft
title: 'Scripts in a page: surf runs a page''s inline scripts with swell'
summary: surf embeds the organization's script engine swell and runs a page's inline classic scripts while parsing, in the session process, against a DOM generated from Web IDL and under limits per page, when the session's option asks for it, so that what a script writes is what surf and surf-agent show.
relations:
- depends_on: story:html-document-port
revision: 38
---
## Context

`AGENTS.md` ("Implementation targets"), the operator on 2026-11-21: the JavaScript engine "could
also be just, I guess, its own new project [...] It should be separate, I guess, from Surf itself
[...] Requirements are for sure influenced by Surf and its needs". So "the script engine is a
repository of its own that surf embeds, with its requirements written from surf's needs". That
repository is swell (`https://github.com/beyond10x/swell`), an ECMAScript engine written from
scratch in Rust, "written to be embedded by surf" (swell `README.md`).

The order, decision D77 of 2026-11-22 (option A), given for the operator by the operator
session: surf starts embedding swell after wave 11 of `docs/plan/agent-waves.md`, as this
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

A page that a session of `surfd` loads runs its inline classic scripts while it is parsed, inside
the session's own process, and what the scripts wrote is what the session shows. Two examples,
each served by a test on 127.0.0.1:

1. `<title>Before</title><script>document.title = "Written by script"</script>`: in a session
   opened with the scripting option, `surf-agent look` answers `page 200 "text/html" "<address>"
   "Written by script"`. In a session without it (the default) it shows `Before`, as today, and
   `surf <url>` always prints `title <document-id> Before`.
2. `<ul id=l></ul><script>var a = document.createElement("a"); a.setAttribute("href", "/next");
   a.textContent = "Next"; document.getElementById("l").appendChild(a);</script>`: in a session
   with the option, the document holds an `Element` row of kind `link`, label `Next`, target
   `/next`, which the markup does not hold (a test of `surf-session`'s core reads it);
   `surf-agent` lists it once the session lists links (`story:agent-follow-links`).

In detail:

- surf depends on swell by a git dependency on a release tag (S1), and `lock-check` allows that
  one source by its exact address, tag and commit, and refuses every other source.
- Scripts run only in a session of `surfd` whose `open` asked for them: `surf-agent open
  --scripts` (the spelling is `scripts-spec`'s), off by default (S3, S7). Both modes are tested.
  `surf <url>` and `surf run` run no page script.
- The parser, the DOM and the engine are in one process, `surf-session` (S2, option a). At a
  `script` end tag the tree builder calls a hook
  with the tree; surf's hook evaluates the script with swell against that tree, on the parsing
  thread, and parsing goes on over the tree the script left.
- The DOM is generated from Web IDL (S4, option b): the whole `Node`, `Element` and `Document`
  interfaces of the DOM Standard, with what they inherit and include, are generated by a binding
  generator from vendored IDL. Each member either has its behaviour or throws a `DOMException`
  named `NotSupportedError` whose message names the member; the list of which is which is
  committed and counted ("The DOM from Web IDL").
- Each script's end is recorded with its document (ran, threw, refused by swell, ended by a
  limit, not run and why), and a page whose script fails still shows everything else.
- Limits per script, per document and in wall time, each ending in a defined outcome; the
  session process runs with a reduced environment and the operating-system bounds the operator's
  answer to S5 allows.
- The html5lib cases tagged `needs-script` that need no `document.write` run and pass.

## In and out of the first version

In: inline classic scripts (no `src`, `type` absent or a JavaScript MIME type essence), run
synchronously at their end tag in document order, in one realm per document whose global is a
`Window` host object; the generated interfaces of "The DOM from Web IDL", each member with its
behaviour or its named `NotSupportedError`; per-page limits; the scripting option of S3 on
`surf-agent open` only, off by default; parser scripting enabled for a document whose scripts
run, and its effect on `noscript`; the reduced environment of the session process.

Out, each with its reason:

- Scripts in `surf <url>` and `surf run`: by the operator's pick S7, scripts run only in sessions
  of `surfd`. `surf <url>` shows a page without its scripts until it is a thin client of `surfd`;
  nothing is built for that here.

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
- Events: `EventTarget`'s three members are generated and throw `NotSupportedError`; event
  handlers, `Event` and dispatch need the event loop, and an agent's action firing script is a
  new path in `surf.agent`; after this epic.
- Ranges, traversal, XPath, attribute nodes, token lists, mutation observers, custom elements and
  shadow trees: the members of the three interfaces that return or need them are generated and
  throw `NotSupportedError` (the list is in "The DOM from Web IDL"); their interfaces are not
  generated.
- Live collections: until swell's slice 9 (HOST-112 to HOST-114, internal-method overrides) a
  member that returns a live `NodeList` or `HTMLCollection` returns a snapshot with indexed own
  properties, a named deviation recorded beside each such member; selectors beyond the subset of
  `scripts-dom-selectors` throw `NotSupportedError`, a named deviation.
- `document.write` and the rest of HTML's `Document`, `Window` beyond `document`, `window` and
  `self`, and the element interfaces (`HTMLElement`, `HTMLAnchorElement`, ...): an element's
  wrapper is an `Element`, a named deviation. Which HTML definitions are generated is S6.
  `document.write` also needs the insertion point and a re-entrant tokenizer
  (`the-input-byte-stream insertion-point`); the 3 html5lib cases that use it stay not run.
- `WindowProxy`: it needs the internal-method overrides of slice 9; the global `this` is the
  `Window` itself, a named deviation.
- The Web API surface beyond the DOM: `fetch`, `XMLHttpRequest`, storage, cookies, `location`,
  `history`, `navigator`, `console`, `innerHTML` (fragment parsing from script). Each is a
  capability a script gains and is decided on its own later.
- Anything that needs layout or style: `getComputedStyle`, `offsetWidth`, `style`. surf computes
  no style (`spec/domains/render.yaml` marker).
- A separate script process: S2 was answered a (one process); see "Decisions taken".

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

The form chosen (S1, option a): that git dependency, on a release tag. What changes with it:

- `lock-check` holds an allow-list of exact sources and accepts a `source` line only when it is
  `git+https://github.com/beyond10x/swell?tag=<tag>#<commit>` with the tag and commit the list
  names, for each of the five swell packages; every other `source` line still fails, and so does
  a swell package from another address, tag or commit. Its tests cover both sides.
- The sentence of "Goals" becomes "no third-party crate; one crate of the organization" (the
  operator's pick, "Decisions taken"), and the `lock-check` description in `Taskfile.yml` says
  the same.
- A cold build and CI fetch swell from the forge, as `conformance/` already does for ess. A
  version moves by one line in `Cargo.toml` and the lock, and the allow-list's tag and commit in
  the same change.
- Which tag: page scripts run only on a tag that holds the fixes of "From the security review of
  2026-11-22". `0.2.0` may enter for tests of the engine without a page; the tag that runs a
  page's script is named in that section when it exists.

Two things hold in this form: swell's `[profile.*] overflow-checks = true` (its root
`Cargo.toml`) does not apply inside surf's build, because cargo reads profiles from the root
workspace only (from memory), so surf sets `overflow-checks = true` for the swell packages in the
profiles it ships; and swell is Apache-2.0 with a `NOTICE`, which surf's release archives carry
beside its own (Apache-2.0 section 4(d), from memory).

The vendored Web IDL of "The DOM from Web IDL" is data in a tooling crate, not a dependency, and
changes nothing of this rule.

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
  entry spends from the outermost entry's budget (HOST-8); after a step budget or an interrupt
  the agent stays usable; after a memory budget it stays alive and runs the next entry once the
  host made room (HOST-70 as narrowed on 2026-11-22, HOST-188 to HOST-190; "Answers of the
  script engine's session").
- An `InterruptHandle` that another thread may set (HOST-90 to HOST-96), for a wall-clock bound.
- Misuse is a defined answer: stale or foreign handles answer `Err` (HOST-30, HOST-31, HOST-45); no
  entry point panics for any argument (HOST-3, a fuzz test); a broken invariant faults the agent
  for good (HOST-48, HOST-160).
- `host_ensure_can_compile_strings` lets the host refuse `eval` and `Function` (HOST-116, HOST-117).
- Determinism: a run is a function of limits, seeds, calls and hook answers (HOST-133).
- Its own stated gap (`CHANGELOG.md` 0.2.0): "the comparisons of that scan are not charged to the
  step budget: a script with very large objects runs longer than its step count says". So the step
  budget alone does not bound time; surf's wall-clock interrupt does. The security review of
  2026-11-22 adds the charges swell must make first (its section below).

What surf must add inside the process:

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
- What a script may reach: the generated DOM of its own document and nothing else. A member that
  is not implemented throws `NotSupportedError` and touches nothing; no generated member reaches
  the network, the session, its token, storage, other documents or time beyond the clock hook
  (`host_system_utc_epoch_nanoseconds` is a required hook, HOST-125, pending slice 4). The
  generator refuses an IDL definition that is not in the committed selection, so a new member
  enters only by a reviewed change of that selection.

The process boundary (S2, option a): the parser, the DOM and the engine run in `surf-session`,
one process per session (`AGENTS.md`, "Agent target"), and nowhere else. What a
script reaches if it escaped the language, today, in `surf-session`:

- the environment `surfd` passes on: its own, which is the caller's, without `SURF_TOKEN` and
  `SURF_SESSION` (`crates/surfd/src/daemon.rs`, the session's `Command`), so any secret the caller
  had in its environment;
- the session's token (a hash of it once `story:agent-token-hash` is done) and the pipes and
  socket to `surfd`: it can send any answer and any log entries for its own session, within
  `surfd`'s bounds (a request of 64 KiB, an answer of 4 MiB, entries of 8 MiB, 60 s a call);
- the network, which the session needs for its pages, as the user;
- the user's files and the user's other processes (signals; tracing as far as the kernel's
  ptrace policy allows, not checked).

What bounds it: one process per session, so never another session's memory, token, pages or
socket; the live view reads a session's text only through `surf_answer::parse` and
`surf_answer::unquote`. Nothing at the level of the operating system bounds it yet.

The reduced environment, restricted privileges and resource limits of "From the security review
of 2026-11-22" apply to `surf-session` itself (`story:session-process-hardening`). What the
standard library and the operating system give without a third-party crate and without `unsafe`
(`#![forbid(unsafe_code)]` holds in every crate; the only `unsafe` of the repository is a test's
counting allocator, `AGENTS.md`, `crates/fetch`):

- A reduced environment: `Command::env_clear` and an allow-list of the variables the session needs
  (`scripts-decisions` lists them), with `current_dir("/")` and
  `process_group(0)` as today. Standard library; nothing else needed.
- Reading, not setting: `/proc/self/limits` and the `NoNewPrivs` and `Seccomp` lines of
  `/proc/self/status` show what applies, as the agent binaries already read the caller's user id
  from `/proc/self` (Linux only, `AGENTS.md`, "Publishing"). A test can prove a limit this way.
- Not possible under these rules: `setrlimit`/`prlimit` (address space, CPU time, open files,
  processes, core size), `prctl` (`PR_SET_NO_NEW_PRIVS`, `PR_SET_DUMPABLE`), seccomp, Landlock and
  namespaces each need a system call the standard library does not expose; `CommandExt::pre_exec`
  is `unsafe` itself; `CommandExt::uid` needs a privilege `surfd` does not have. The options are
  S5: a helper program the session is started through, `unsafe` in one audited place, or none.

Scripts are off by default (S3): a session runs them only when `open` asked for it. No other
process runs page script (S7): `surf <url>` and `surf run` run in the caller's own process,
which nothing reduces, and so run none.

## The gaps

Requirements of swell's host interface surf needs first, by `conformance/host/requirements.txt`
at `6a43e18`:

- With a test there: HOST-1 to HOST-3 (authority, panics), HOST-6 to HOST-20 and HOST-22
  (budgets and limits), HOST-28 to HOST-37 (roots, strings, sources, outcomes), HOST-45, HOST-47,
  HOST-48, HOST-160 (misuse), HOST-49 to HOST-54 and HOST-161 (realms with a host global, scripts),
  HOST-69 to HOST-72 (termination), HOST-90 to HOST-96 (interrupt), HOST-102 (reads),
  HOST-103 to HOST-111 (host functions, host objects with brand, host data and `trace`: the
  generated interface objects, prototypes, accessors and wrappers are made of these),
  HOST-116 to HOST-118 (compiling strings), HOST-124, HOST-129, HOST-132, HOST-133, HOST-136.
- Pending, needed soon after: HOST-155 (`globalThis` is the setup's `this`, slice 3);
  HOST-125 to HOST-128 (clock and time zone, slice 4); HOST-26, HOST-27, HOST-144, HOST-147
  (`Math.random` from the seeds, slice 4); HOST-112 to HOST-114 (internal-method overrides,
  slice 9), without which a live `NodeList` or `HTMLCollection`, a named property and a
  `WindowProxy` cannot be built, so the first version returns snapshots (the named deviation of
  "The DOM from Web IDL"); HOST-77 to HOST-84, HOST-100, HOST-101, HOST-121, HOST-148, HOST-149
  (promise jobs and the checkpoint, slice 10).
- Slice 10 as swell planned it on 2026-11-23 (its `design/slice-10`, all pending), with what surf
  asked for: the host makes a promise with `cx.new_promise_capability()`, settles it through
  `agent.call`, and drains the queue with `agent.perform_microtask_checkpoint()`, which surf
  calls after each script; `Agent::queue_microtask` takes a realm (HOST-192); a job that queues
  jobs without end ends the checkpoint with a termination (HOST-193). On surf's request:
  `last_entry_steps()` after a checkpoint reads the steps of the whole checkpoint, also after a
  termination (HOST-197), for the step total per document; the interrupt ends a checkpoint at
  its start, before each entry and inside a job (HOST-198), for the wall-clock bound; a
  checkpoint that did not empty the queue leaves its entries queued and charged, and the next
  checkpoint runs them (HOST-199); `Agent::clear_microtasks()` drops every queued entry unrun
  and credits its charge (HOST-200). surf calls it when a limit ended a checkpoint, so that a
  page that floods the queue cannot spend the next scripts' budgets. Dropping the agent with
  entries queued calls nothing of the host.

Read at `6a43e18` and not a requirement: `new_host_function` and `define_property_or_throw` are
methods of `Cx` only, not of the setup closure's `HookCx` (`crates/swell/src/host/mod.rs`), so
the bindings make the `Window` host object in the setup and define the interface objects and the
global's members in a `with_cx` right after `create_realm`, before the first script; no script
can observe the difference.

Requirements from surf's needs, which no requirement of `host-interface.md` at `6a43e18` states
(searched for "line number", "column", "position", "charge", "weak handle", "well-known",
"unscopables" and "set_prototype"; each is surf's proposal to the session working on swell). The first
seven are checked against S2 = a and S4 = b; 8 to 10 are new with S4:

1. A parse error and an uncaught thrown error can be read with the line and column of the
   script's source where each arose, so that surf can report where a page's script failed.
   Holds as before.
2. Every operation whose work grows with the size of its operands, the property scan named in
   0.2.0's limits included, charges steps in proportion, so that `steps_per_entry` bounds an
   entry's time. Holds; the security review's first condition asks the same of parsing,
   compilation and property operations.
3. The host can charge and credit bytes it holds for the agent (DOM nodes and text behind host
   data) against `memory_bytes`, so that a script creating nodes without end ends in
   `Terminated(MemoryBudget)`. Holds: the tree lives in the host, outside swell's heap, in the
   same process as the agent (S2 = a), so one budget can count both.
4. The host can hold a reference to a host object that does not keep it alive and reads as gone
   once it is collected, so that a node-to-wrapper map keeps one wrapper per node without a
   `Root` per node (inferred from engine.md decision 9, "Rooting"). Needed more with S4: every
   generated member that returns a node must return the same object for the same node
   (`a.parentNode === a.parentNode`). Until it exists, a `Root` per wrapper kept until the
   document is dropped, bounded by the parser's node limit.
5. A read returns an error object's kind and its own `message` data property without running
   code or calling a hook, so that surf can record a thrown value after its run ended. Holds; a
   thrown `DOMException` is a host object whose name surf reads from its own host data.
6. HOST-17, HOST-152 and HOST-163 hold under the embedding program's build profiles (release, and
   debug information off), each shown by a test in that profile. Holds: the parsing thread of
   `surf-session` and of `surf` is the stack HOST-17 sizes.
7. Each release's `CHANGELOG.md` section names every change to a signature or a requirement of
   the host interface, so that surf moves its pinned version knowingly. Holds, and weighs more:
   the generator writes code against that interface.
8. The host can make a host function whose [[Prototype]] it names, or set the [[Prototype]] of a
   host function it made, so that an interface object's [[Prototype]] is its parent's interface
   object (`Element`'s is `Node`, Web IDL "interface object").
9. The host can name, from a closed list, the well-known symbols `@@toStringTag`,
   `@@unscopables` and `@@iterator` and the intrinsics `%Error.prototype%`,
   `%Function.prototype%` and `%Array.prototype.values%`, so that the bindings give class
   strings, `[Unscopable]`, an iterable `NodeList` and a `DOMException` whose prototype chain
   reaches `%Error.prototype%`. Section 7 rejected an accessor for every intrinsic; this is a
   named list. When swell delivers symbols: not checked.
10. The bytes and steps a host function, a host object and a property definition cost are
    stated and charged, and a test measures a realm with 300 host functions and 40 host objects
    (the size of the generated bindings), so that surf can set `memory_bytes` and the step budget
    per document with the bindings' cost inside them.

## What must exist in surf first

- The document: `story:html-document-port` (wave 11) gives `Browser` a parser step that parses
  each committed HTML body and keeps the tree per document, in the session process (and in
  `surf` for the command line, which runs no script, S7). S2 = a keeps that: this epic runs
  scripts inside that step in a session's process, on the same thread as the parse, before its
  ending command, so the element rows are computed from the tree the scripts left. `story:html-tree-tables` (wave 11) for the foster-parenting case below.
- The tree builder at a `script` end tag: today it pops and returns (`crates/html/src/
  tree_builder.rs`, module doc). It must, when parser scripting is enabled, take the steps of the
  "text" insertion mode the first version keeps (pop, return to the original insertion mode,
  prepare the script element: classic, inline, parser-inserted, not `nomodule`), call a hook with
  the tree, and go on with its stack of open elements and formatting list over the tree the hook
  left, including nodes a script removed. The parser's pause at the end tag is that call: the
  hook runs the script to its end on the parsing thread and returns, so the tokenizer and the tree
  builder keep their state in place and nothing crosses a process. `Tree`'s writes are
  `pub(crate)` today (`crates/html/src/tree.rs`); the generated DOM needs a public write surface
  bounded by the limits (`story:html-parser-limits`, wave 10). The `html-sections` line
  `tree-construction script-processing` stays open for the steps not taken (pending
  parsing-blocking script, nesting level beyond 1, the insertion point).
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
  become run and pass; they need the reading, attribute, mutation and selector slices. Three need
  `document.write` (`scripted_webkit01.dat` 2, `encoding/scripted/tests1.dat` 1) and stay not
  run.
- The session process's start: `surfd` starts `surf-session` with its own environment less two
  variables (`crates/surfd/src/daemon.rs`). Before a session runs a script, that start is reduced
  (`story:session-process-hardening`). The open stories `agent-surfd-session-limit`,
  `agent-surfd-idle-exit-race` and `agent-token-hash` touch the same crates.
- Nothing of Web IDL exists in surf: the reader, the generator and the vendored IDL are new
  (`crates/webidl`).

## Candidate stories, in dependency order

Drafts for the decomposition, not yet artifacts. Each: outcome; `depends_on`; what it writes;
when it can start. "Now" means while the waves running today go on: it touches no file an open
story of those waves writes.

1. `scripts-decisions`: a design note `docs/design/scripts.md` settles the first version's
   boundary, the steps of the `script` end tag taken and left out, the limits with measured
   values, seeds and clock, the specification shape of a script's end and of the option, the
   Web IDL selection (each member's slice or its `NotSupportedError` reason, each deviation, the
   selector subset), the session's environment allow-list, the answers to S5 and S6, and
   requirements 1 to 10 handed to the session working on swell. Depends on nothing. Writes only
   `docs/design/scripts.md`. **Can start now: the first story a unit takes.**
2. `webidl-reader`: the DOM Standard's IDL Index (commit `e4f4f4f`) and Web IDL's `DOMException`
   are vendored with `SOURCE` files and digests; `surf-webidl` reads them into a model and refuses
   every construct it does not know by line; a test counts the 146 members and 2 constructors of
   the table. Depends on nothing (the sources are pinned here); the HTML excerpt of S6 joins once
   S6 is answered. Writes `crates/webidl/` (new), the root `Cargo.toml` (`members`) and
   `Cargo.lock`. **Can start now, beside 1.**
3. `scripts-engine-dependency`: surf depends on swell by a git tag (S1); `lock-check` allows that
   one source and its tests show another failing; the "Goals" sentence; `NOTICE`;
   `overflow-checks` for the swell packages; a new crate `crates/script` (`surf-script`) runs an
   agent with surf's limits on a thread of the measured stack and evaluates a source with no
   DOM, tested on an endless loop, an endless allocation and an interrupt. Depends on 1. Writes
   the root `Cargo.toml`, `Cargo.lock`, `Taskfile.yml` (`lock-check`), `crates/script/` (new),
   `NOTICE`, `AGENTS.md` ("Goals"). Beside 2 once the one shared line of `members` is merged by
   the first of them.
4. `scripts-spec`: `surf.document` records each script's end; the session gains the scripting
   option of S3 on `open` only (S7; `surf.agent`, `cli/surf-agent.yaml`: `open --scripts`);
   `task generate`. Depends on 1. Writes `spec/domains/document.yaml`,
   `spec/domains/agent.yaml`, `cli/surf-agent.yaml`, `spec/scenarios/`, `generated/rust/`.
   Beside 2, 3, 5; after the open
   agent specification stories that write `spec/domains/agent.yaml` and `cli/surf-agent.yaml`
   (`agent-forms-spec`, `agent-history-spec`, `agent-links-spec`, `runner-record-option`), or
   scoped against them.
5. `session-process-hardening`: `surfd` starts `surf-session` with a cleared environment and the
   allow-list, and with the bounds of the operating system that S5 allows; a test reads the
   running session's `/proc/<pid>/environ`, `limits` and `status`. Depends on 1. Writes
   `crates/surfd/src/daemon.rs`, `crates/surfd/tests/`, and under S5 b `crates/session/src/`.
   Needs no swell and no parser; after `agent-surfd-session-limit` and
   `agent-surfd-idle-exit-race`, which write `daemon.rs`.
6. `webidl-bindings`: the binary `bindings` writes `crates/script/src/bindings.rs` from the model
   and `crates/webidl/selection.txt`: interface objects, prototype chains, brands and their
   inheritance table, accessors, operations with Web IDL's conversions, and a DOM trait whose
   default throws `NotSupportedError` naming the member; the digest header, the fast check in
   `task check` and the lane `slow`. Its tests are the slice of the members that throw: every
   member exists with Web IDL's property attributes, a foreign brand is refused, and each of the
   23 members and 2 constructors throws its named error. Depends on 2 and 3. Writes
   `crates/webidl/src/`, `crates/webidl/selection.txt`, `crates/script/src/bindings.rs`,
   `crates/script/src/lib.rs`, `taskfiles/webidl.yml`, `Taskfile.yml`.
7. `html-script-hook`: the tree builder calls a hook at a parser-inserted inline classic
   script's end tag when scripting is enabled, and goes on over the tree the hook left; `Tree`
   gains a public write surface bounded by the parser's limits. Depends on 1,
   `story:html-tree-tables` and `story:html-parser-limits` (the same files). Writes
   `crates/html/src/tree_builder.rs`, `crates/html/src/tree_builder/modes.rs`,
   `crates/html/src/tree.rs`, `crates/html/src/lib.rs`. Beside 6; waits for those wave stories.
8. `scripts-dom-reading`: the 73 reading members and their kin on the returned interfaces, the
   `Window` global (`document`, `window`, `self`), `document.title`'s getter, one wrapper per
   node, and the snapshot deviation; tested member by member over a tree from
   `parse_document`, with no hook. Depends on 6. Writes `crates/script/src/dom/` (the wrapper,
   reading and window modules), `crates/script/tests/`.
9. `scripts-dom-mutation`: the 29 mutating members, the setters of `nodeValue`, `textContent`
   and `document.title`, the constructors of `Text`, `Comment` and `DocumentFragment`, each with
   the DOM Standard's exceptions and held to the limits. Depends on 7 and 8. Writes
   `crates/script/src/dom/mutation.rs` and its tests.
10. `scripts-dom-attributes`: the 14 attribute members, held to the attribute limits. Depends on 7
    and 8. Writes `crates/script/src/dom/attributes.rs` and its tests. Beside 9 and 11.
11. `scripts-dom-selectors`: the selector subset for the 7 selector members, `NotSupportedError`
    outside it. Depends on 8. Writes `crates/script/src/dom/selectors.rs` and its tests. Beside 9
    and 10.
12. `scripts-html-conformance`: the harness runs the four DOM-only `needs-script` cases through
    `surf-script` and the expected counts rise. Depends on 9, 10, 11 and
    `story:html-tree-tables`. Writes `crates/html-conformance/` (`src/`, `expected/`,
    `Cargo.toml`) and the `html-sections` block of `docs/design/html-parser.md`. Beside 13.
13. `scripts-document-port`: in a session opened with the option, `Browser`'s parser step runs a
    document's scripts in the session process on the parsing thread, with the per-page limits
    and a wall-clock interrupt, writes each script's end, and the session shows the two examples
    of "Outcome" (`surf-agent look`, and the document's rows); `surf <url>` keeps running no
    script, tested; the `noscript` rule. Depends on 4, 5, 7, 9, 10,
    `story:html-document-port`, and a swell tag with the review's fixes. Writes
    `crates/surf/src/lib.rs`, `crates/session/`, `crates/page/src/lib.rs`,
    `conformance/src/target.rs`, `docs/agent-answer.md`, and only a test in
    `crates/surf-cli/tests/`. **Waits for
    `story:html-document-port`.**
14. `scripts-hostile-pages`: pages with an endless loop, endless allocation, endless node
    creation, a tree built deep by `appendChild`, deep recursion, 10,000 script elements, an
    attribute over the limit and a script over the source limit each end with a recorded limit
    and the rest of the page parsed, every test under 5 s; and a session at its process limits
    ends as a failure of the script, not of `surfd`. Depends on 13. Writes `crates/surf/tests/`,
    `crates/session/tests/`. **Waits for `story:html-document-port`**
    (through 13).

At the same time: 1 and 2 now; after 1, 3, 4 and 5 beside 2; 6 after 2 and 3, beside 7; after 8,
the slices 9, 10 and 11 beside each other (disjoint modules of `crates/script/src/dom/`); 12 and
13 beside each other (disjoint crates).

## Open decisions for the operator

S1 to S4 and S7 are answered ("Decisions taken"). Two remain, each with its default, which is taken
after 15 minutes without an answer (the organization folder's rule "A default proceeds"); none blocks
`scripts-decisions` or `webidl-reader`.

S5. How the session process gets restricted privileges and resource limits, under
`#![forbid(unsafe_code)]` and with no third-party crate ("Security").
- a. A helper: `surfd` starts `surf-session` through the system's `setpriv --no-new-privs` and
  `prlimit` (util-linux) with the limits of `scripts-decisions`, and refuses to start a session
  with scripts when either is missing. Cost: two system programs at run time (on the forge's
  runner image: not checked); no seccomp, no Landlock, no `PR_SET_DUMPABLE`; no rule changes.
- b. `unsafe` in one audited module of `surf-session` that calls `setrlimit` and `prctl` from the
  C library the standard library already links (an `extern "C"` block, no crate), allowed there
  only, every other crate keeping `forbid`, each call proven by a test through `/proc/self`.
  Cost: the first `unsafe` in shipped code, an exception to a rule of this repository that needs
  the operator's word; seccomp and Landlock could follow in the same place.
- c. None: the reduced environment and the bounds inside the process only. Cost: the review's
  second condition stays unmet, so no session runs a page's script.
Default: a.

S6. Which definitions of the HTML Standard are generated beside the DOM Standard's.
- a. Four members: `Window`'s `document`, `window`, `self` and `Document`'s `title`, from the
  vendored HTML IDL, the rest of HTML's `Window` and `Document` not generated. Cost: `document.body`,
  `document.write`, `location` and the event handler attributes read `undefined` (a call throws a
  `TypeError`), not `NotSupportedError`.
- b. HTML's `Window` and partial `Document` whole, all but those four throwing
  `NotSupportedError`: 153 members of `Window` with its mixins and 126 more on `Document` (50 of
  its partials and 76 event handler attributes), counted on 2026-11-22 from webref's `html.idl`
  (extracted from the HTML Standard). Cost: about 280 members more per realm, about double the
  bindings' cost per document; the reader also takes `[LegacyUnforgeable]`, `[Replaceable]` and
  `Document`'s named getter, which needs swell's slice 9 and stays a deviation.
- c. No HTML IDL: a hand-written `Window` with `document` and a hand-written `title`. Cost: two
  members not generated, against S4's "generated from their Web IDL".
Default: a.

## Out of scope

Everything the first version leaves out above; style, layout and rendering; workers (swell
engine.md open item B); a script engine other than swell; a process of its own for scripts (S2
was answered a); `surf <url>` as a client of `surfd` (a consequence of the end design, not
planned here); running web-platform-tests files of the DOM; any change in swell's repository,
which belongs to the session working on swell, given requirements 1 to 10 of "The gaps".

## Constraints

- Specs first: the specification changes (`scripts-spec`) are green before the code that fills
  them. No third-party crate; swell enters only as the git dependency of S1, named exactly by
  `lock-check`'s allow-list. `#![forbid(unsafe_code)]` in every crate, unless the operator
  answers S5 with b, and then in every crate but that one module.
- The bindings are generated, never written or edited by hand: `crates/script/src/bindings.rs`
  changes only by the binary `bindings`, its digest check is part of `task check`, and its
  regeneration runs in the lane `slow`, not in a test. A member gets behaviour only in the
  hand-written DOM behind the generated trait, and a member enters or leaves only by a change of
  `crates/webidl/selection.txt`. Vendored files are never edited.
- A script never hangs, panics or exhausts the session process: every limit ends in a defined,
  recorded outcome, and the rest of the page is parsed. A failure of a script, and a session
  ended by its process limits, is reported as the script's, never as `surfd`'s.
- Tests start their servers in Rust on 127.0.0.1; no test over 60 s, the test step within 120 s;
  a test over 5 s is named with its time. No test makes a network request: the swell source is
  fetched by cargo before the tests, the IDL is vendored. Before a generator gains a source (a
  new IDL file, a new interface in the selection), it is timed before and after. No Python.

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. `lock-check` passes and names swell by the allow-list's address, tag and commit; its tests
   show a package of another source, tag or commit failing it. The tag is one that "From the
   security review of 2026-11-22" names as holding the fixes.
2. The bindings' digest check passes, and `task slow` regenerates `crates/script/src/bindings.rs`
   byte for byte; the reader's test counts, from the vendored IDL Index, 146 members and 2
   constructors of `EventTarget`, `Node`, `Element` and `Document`, and every one of them is in
   the selection with a slice or a `NotSupportedError` reason.
3. Named tests, member by member: each member of the reading, attribute, mutation and selector
   slices does what the DOM Standard says on a parsed tree, each deviation is tested as the
   selection names it, each of the 23 members and 2 constructors that throw throws a
   `DOMException` named `NotSupportedError` whose message names the member, and a member called
   on an object of another brand throws a `TypeError`.
4. `task html-conformance` prints the four DOM-only `needs-script` cases as run and passed, and
   `not-run:needs-script` counts 3 cases, each needing `document.write`.
5. Named tests over the two examples of "Outcome": in a session opened with the option,
   `surf-agent look` prints the title the script wrote in its page line and the document holds
   the element row of the link it added; in a session without it, and from `surf <url>` always,
   what the markup holds.
6. Each page of `scripts-hostile-pages` ends with its limit recorded and the page's other elements
   present, each test under 5 s.
7. A test reads a running `surf-session`'s `/proc/<pid>/environ` and finds the allow-list only,
   and its `/proc/<pid>/limits` and `status` show the bounds S5's answer gives.
8. Both conformance reports hold at least one passed scenario for every outcome of the commands
   that record a script's end and of the scripting option.

## Decisions taken

- S1, the form of the dependency: option a, a git dependency on a release tag of swell. The operator, 2026-11-22 04:51 CET, relayed by the operator session: "A: git dep on a swell tag". `lock-check` allows that one source by its exact address and tag, and the sentence on dependencies becomes "no third-party crate; one crate of the organization".
- S2, where a page's scripts run: option a, in the session process (`surf-session`, one per session), engine and DOM together; there is no separate script process. The operator first picked b at about 04:55 CET, 2026-11-22, relayed by the operator session: "B: separate script process". He corrected it at 05:15 CET, relayed by the operator session (spoken, verbatim; "serve", "served" and "serve D" are the transcription of surf and `surfd`): "So for swell and serve, I think one process is actually enough. Yeah, just correct this last decision. Eventually, there will be served, and has an isolated multiple and the CLI just becomes a thin client. That's the actual end design. So where serve D is just a daemon, basically." The epic's default a stands.
- S3, when scripts run: option b of the list above (put to the operator as "A"), per session, an option of `surf-agent open` and of `surf`, off by default, both modes tested. The operator, 2026-11-22 05:12 CET, relayed by the operator session: "A: option, off by default". It was the default the epic named. By S7 the option exists on `surf-agent open` only, not on `surf`.
- S4, the DOM of the first version: option b, the whole `Node`, `Element` and `Document` interfaces generated from their Web IDL. The operator, 2026-11-22 05:14 CET, relayed by the operator session: "B: full Web IDL interfaces". This is not the default the epic named (a).
- S7, `surf <url>` and page scripts: option c, scripts run only in sessions of `surfd` (`surf-agent open --scripts`); `surf <url>` gets no script option until `surf` is a thin client of `surfd`, and nothing is built for it. The operator, 2026-11-22 05:48 CET, relayed by the operator session: "C: scripts only in surfd". It was not the default the epic named (a).

## From the security review of 2026-11-22

A security review of surf and swell of 2026-11-22 names two conditions for running page scripts. The operator forwarded it: "Fix Swell's resource limits before Surf runs untrusted JavaScript."

- surf runs page scripts only on a release tag of swell in which parsing and compilation are charged to the memory budget before they allocate, and property operations (enumeration of string and object keys, property lookup) charge their work and their temporary storage before they do it. The session working on swell has the findings and plans them as stories; the tag that holds the fixes is named here when it exists.
- Before a session process runs a script, its environment is reduced to what it needs (today it inherits the caller's environment but two variables of surf), its privileges of the operating system are restricted, and it has resource limits of its own. This is part of the candidate story `scripts-hostile-pages` or a story before it.

## The DOM from Web IDL

S4, option b: the whole `Node`, `Element` and `Document` interfaces are generated from their Web
IDL by a binding generator over a Web IDL reader, both written here first.

Sources, vendored like `crates/html-conformance/data/` (each file copied unchanged, a `SOURCE`
file with address, commit, the standard's date, fetch date, licence and the sha256 of each file,
`* -text` in `.gitattributes`, nobody edits a file):

- The DOM Standard's IDL Index, `https://dom.spec.whatwg.org/` ("Last Updated 15 November 2026",
  commit `e4f4f4f0ace83fe6c52ab68e047351ce7f2dcd69`, permanent at
  `https://dom.spec.whatwg.org/commit-snapshots/e4f4f4f0ace83fe6c52ab68e047351ce7f2dcd69/`),
  fetched 2026-11-22: 673 lines once the markup is taken off.
- `DOMException` from the Web IDL Standard, `https://webidl.spec.whatwg.org/` ("Last Updated 14
  November 2026", commit snapshot `54a5f2de6a34e27a517a49370bc07d8943b0be11`): the error every
  `NotSupportedError` and every DOM algorithm's exception is.
- From the HTML Standard what S6 decides (by default `Window`'s `document`, `window`, `self` and
  `Document`'s `title`, which example 1 needs).
- Under `crates/webidl/data/whatwg/`. The licence is CC BY 4.0 as for `entities.json`; the
  generated file carries the BSD 3-Clause notice of the WHATWG IPR policy in its header, as
  `crates/html/src/charref_table.rs` does. WPT's `interfaces/dom.idl` (the copy idlharness reads)
  exists; whether it equals the vendored index: not checked.

The members, counted on 2026-11-22 by a script over that IDL Index (a member of a mixin counts on
each interface that includes it):

| interface | own | from mixins | constants | attributes | operations | constructor |
|---|---|---|---|---|---|---|
| `EventTarget` | 3 | 0 | 0 | 0 | 3 | yes |
| `Node : EventTarget` | 47 | 0 | 18 | 14 | 15 | no |
| `Element : Node` | 36 | 17 | 0 | 18 | 35 | no |
| `Document : Node` | 28 | 15 | 0 | 15 | 28 | yes |
| total | 114 | 32 | 18 | 47 | 81 | 2 |

`Element` includes `ParentNode` (10), `NonDocumentTypeChildNode` (2), `ChildNode` (4) and
`Slottable` (1); `Document` includes `NonElementParentNode` (1), `DocumentOrShadowRoot` (1),
`ParentNode` (10) and `XPathEvaluatorBase` (3). 146 members and 2 constructors. Writable
attributes: `nodeValue`, `textContent`, `id`, `className`, `slot`.

The interfaces these return or are made of are generated too, else a text node or a list could
not be handed to a script: `CharacterData` (7, and `ChildNode`, `NonDocumentTypeChildNode`: 13),
`Text` (3 with `Slottable`, constructor), `Comment` (constructor), `ProcessingInstruction` (1),
`DocumentType` (7 with `ChildNode`), `DocumentFragment` (11 with its mixins, constructor),
`NodeList` (2, `iterable<Node>`), `HTMLCollection` (3) and `DOMException` (3 attributes, 25
constants, constructor): 68 members more, assigned to the slices below by the same rules in
`scripts-decisions`. Not generated: `Attr`, `NamedNodeMap`, `DOMTokenList`, `DOMImplementation`,
the ranges, `NodeIterator`, `TreeWalker`, `Event` and its kin, `AbortController`,
`AbortSignal`, `MutationObserver`, `ShadowRoot`, XPath and `XSLTProcessor`.

What the reader must understand, by what those definitions use:

- Definitions: `interface` with inheritance, `partial interface`, `interface mixin` with
  `includes`, `callback interface` (`EventListener`, `NodeFilter`, `XPathNSResolver`, read for the
  arguments of members that throw), `callback` functions, `dictionary` with inheritance and
  default values (`GetRootNodeOptions`, `ElementCreationOptions`, `ImportNodeOptions` are
  converted; `EventListenerOptions`, `AddEventListenerOptions`, `ShadowRootInit` are read only),
  `enum`, `typedef`.
- Members: `const`, `attribute`, `readonly attribute`, `stringifier`, operations, `constructor`,
  special `getter` operations (`NodeList.item`, `HTMLCollection.item` and `namedItem`),
  `iterable<T>`.
- Arguments: `optional` with defaults (`= {}`, `= false`, `= ""`, `= null`, `= 0xFFFFFFFF`) and
  variadic (`(Node or DOMString)... nodes`). Types: `DOMString`, `USVString`, `boolean`,
  `unsigned short`, `unsigned long`, `undefined`, `object`, nullable `T?`, `sequence<T>`, unions,
  interface types. `TrustedType` is a typedef of the Trusted Types specification, which is not
  vendored: read as `DOMString`, a named deviation.
- Extended attributes, with their count in the index: `[CEReactions]` 49 (no custom elements, so
  no reaction is queued; recorded), `[Exposed]` 36, `[NewObject]` 27, `[SameObject]` 11 (the
  wrapper keeps the returned object in its host data, listed by `trace`), `[Unscopable]` 8 (an
  `@@unscopables` object on the prototype), `[LegacyNullToEmptyString]` 5, and once each
  `[PutForwards]`, `[LegacyUnenumerableNamedProperties]`, `[LegacyUnforgeable]`,
  `[Replaceable]`, `[EnforceRange]`.
- Not used by those definitions: `Promise<T>` (none in the DOM Standard's index) and overloads (no
  operation name twice in one interface). The reader refuses each, and every construct it does not
  know, with an error naming the line, so a newer IDL fails the check instead of generating wrong
  bindings.

Behaviour in the first version, by slice (146 members of the table):

- Tree reading, 73: `Node`'s 18 constants, its 14 attribute getters and `getRootNode`,
  `hasChildNodes`, `isEqualNode`, `isSameNode`, `compareDocumentPosition`, `contains`,
  `lookupPrefix`, `lookupNamespaceURI`, `isDefaultNamespace`; `Element`'s `namespaceURI`,
  `prefix`, `localName`, `tagName`, `shadowRoot` (null: no shadow trees), `assignedSlot` (null),
  the three `getElementsBy...`, `children`, `firstElementChild`, `lastElementChild`,
  `childElementCount`, `previousElementSibling`, `nextElementSibling`; `Document`'s `URL`,
  `documentURI`, `compatMode`, `characterSet`, `charset`, `inputEncoding`, `contentType`,
  `doctype`, `documentElement`, `getElementById`, the three `getElementsBy...` and the four
  reading members of `ParentNode`.
- Attributes, 14: `Element`'s `id`, `className`, `slot`, `hasAttributes`, `getAttributeNames`,
  `getAttribute`, `getAttributeNS`, `setAttribute`, `setAttributeNS`, `removeAttribute`,
  `removeAttributeNS`, `toggleAttribute`, `hasAttribute`, `hasAttributeNS`.
- Tree mutation, 29, and the setters of `nodeValue` and `textContent`: `Node`'s `normalize`,
  `cloneNode`, `insertBefore`, `appendChild`, `replaceChild`, `removeChild`; `Element`'s
  `insertAdjacentElement`, `insertAdjacentText` and the four mutating members of `ParentNode` and
  of `ChildNode`; `Document`'s seven `create...` of nodes (`createCDATASection` throws
  `NotSupportedError` in an HTML document, as the standard says), `importNode`, `adoptNode` and
  `ParentNode`'s four. Every write is held to the parser's limits.
- Selectors, 7: `querySelector` and `querySelectorAll` on both, `closest`, `matches`,
  `webkitMatchesSelector`, for a subset of Selectors Level 4 that `scripts-decisions` names
  (type, universal, `#id`, `.class`, attribute presence and equality, compounds, descendant and
  child combinators, lists); a valid selector outside it throws `NotSupportedError`, a named
  deviation (a browser would not throw).
- Generated to throw `NotSupportedError`, naming the member and its reason, 23 and both
  constructors: events (`addEventListener`, `removeEventListener`, `dispatchEvent`,
  `createEvent`, `new EventTarget()`), attribute nodes and token lists (`attributes`, `classList`,
  `getAttributeNode`, `getAttributeNodeNS`, `setAttributeNode`, `setAttributeNodeNS`,
  `removeAttributeNode`, `createAttribute`, `createAttributeNS`), shadow trees (`attachShadow`),
  custom elements (`customElementRegistry` on both), ranges and traversal (`createRange`,
  `createNodeIterator`, `createTreeWalker`), `implementation`, XPath (`createExpression`,
  `createNSResolver`, `evaluate`) and `new Document()`. Mutation observers and `document.write`
  are not members of these interfaces: `MutationObserver` is not generated, and `document.write`
  is HTML's (S6).
- Deviations, each named beside its member in the selection: the 9 members that return a live
  collection (`childNodes`; `children` and the three `getElementsBy...` on `Element` and on
  `Document`) return a snapshot with
  indexed own properties until swell's slice 9; `[SameObject]` holds for `childNodes` and
  `children` only as the same snapshot object refreshed when it is read; an element's wrapper is
  an `Element`, not `HTMLAnchorElement` and its kin.

How the bindings meet swell's host interface (`docs/design/host-interface.md` sections 7, 14,
15, and `crates/swell/src/host/` at `6a43e18`):

- An interface object is a host function (`new_host_function`, `constructor: true`; for an
  interface without a constructor its steps throw a `TypeError`, as Web IDL says). An interface
  prototype object is a host object (`new_host_object`) whose `Prototype::Object` is its parent's
  prototype, the chain ending at `Prototype::ObjectPrototypeOf(realm)` (`EventTarget.prototype`).
- Attributes are accessor properties and operations data properties, both defined with
  `define_property_or_throw` and a `PropertyDescriptor` whose `get`, `set` or `value` is a host
  function; constants are data properties on the interface object and its prototype.
- A node's wrapper is a host object with one brand per concrete interface (`Brand(u32)`,
  HOST-108) and host data naming the node in the `surf-html` tree, which the host holds
  (`Cx::host`, `host_mut`). The brand check of an inherited member reads `brand()`, accepts it
  when the generated table says that interface inherits the member's, and then reads
  `host_data` with that brand; another brand or an object made by script throws a `TypeError`.
  The single brand per object of HOST-108 suffices; the table is the generator's.
- A member's steps are one generated `HostFn` (a function pointer, decision 14): it converts the
  arguments with Web IDL's conversions over `to_string`, `to_number` and the others, checks the
  brand, and calls a method of a Rust trait the hand-written DOM implements; the trait's default
  for every method throws `NotSupportedError`, so a member without behaviour needs no hand-written
  code.
- What the host interface does not give, each a requirement in "The gaps": the [[Prototype]] of a
  host function (an interface object's is its parent's interface object), the well-known symbols
  (`@@toStringTag`, `@@unscopables`, `@@iterator`) and the intrinsics Web IDL names beside
  `%Object.prototype%` (`%Error.prototype%` for `DOMException`, `%Array.prototype.values%` for an
  iterable `NodeList`); section 7 rejected "an accessor for any intrinsic of a realm", so the
  request is for that named list only. And a measured cost: one realm per document gets 137
  function objects for the four interfaces of the table alone (81 operations, 47 getters, 5
  setters, 4 interface objects), more for the others.

Where the generator runs: a tooling crate `crates/webidl` (`surf-webidl`, standard library only,
not shipped, in the root workspace) holds the reader, the vendored IDL, the committed selection
(`crates/webidl/selection.txt`: which definitions are generated, each member's slice or its
`NotSupportedError` reason, each deviation) and a binary `bindings` that writes
`crates/script/src/bindings.rs`. That file is committed, never edited by hand, and names in its
header the sha256 of each source (the vendored files, the selection, the generator's sources).
The fast check, part of `task check`, recomputes the digests with `sha256sum` and compares them
with the header; it generates nothing. A new lane `slow` (`task slow`, run when those sources
changed) regenerates and compares the bytes with the committed file. This is the first form of
the folder rule "An expensive product is made once".

web-platform-tests: `dom/nodes` at `master` on 2026-11-22 holds 359 entries, 232 of them `.html`
(GitHub's contents listing). Each loads `/resources/testharness.js` by `src`, an external script,
which the first version does not run; `testharness.js` (5,209 lines) uses `let`, `const` or arrow
functions on 58 lines, which swell 0.2.0 refuses; `Node-appendChild.html` also needs an `iframe`
and `document.body`. So no web-platform-tests file of the DOM runs in the first version. Whether
some `dom/nodes` file is ES5 and single-document, and could run with an ES5 stand-in for `test`
and the `assert_*` functions at the commit `crates/html-conformance` pins (`13f8051`): not
checked; a later story. The first version's acceptance is surf's own named tests per member and
the four html5lib cases.

## Size and order

The two picks make the epic larger: a Web IDL reader and a binding generator come before any
member has behaviour, and the session process must be hardened before it runs a script. The
smallest first deliverable an agent can use under these picks: `surf-agent open --scripts <url>`
on a page whose inline script sets `document.title`, and `surf-agent look` printing the title the
script wrote in its page line (example 1), with the option off by default. It needs 11 of the 14
candidate stories: `scripts-decisions`, `webidl-reader`, `scripts-engine-dependency`,
`scripts-spec`, `session-process-hardening`, `webidl-bindings`, `html-script-hook`,
`scripts-dom-reading`, `scripts-dom-mutation`, `scripts-dom-attributes` (example 2 comes in the same
story) and `scripts-document-port`, with `story:html-document-port` and a swell tag that holds the
review's fixes. After it, without blocking it: `scripts-dom-selectors`,
`scripts-html-conformance` and `scripts-hostile-pages`, which the epic's acceptance needs. Of the
11, `scripts-decisions` and `webidl-reader` start now, three more after the note, and the rest
wait on the generator, the wave stories of the parser and `story:html-document-port`.

## End design named by the operator

The operator, 2026-11-22 05:15 CET, relayed by the operator session (spoken, verbatim;
"serve", "served" and "serve D" are the transcription of surf and `surfd`): "So for swell and
serve, I think one process is actually enough. Yeah, just correct this last decision.
Eventually, there will be served, and has an isolated multiple and the CLI just becomes a thin
client. That's the actual end design. So where serve D is just a daemon, basically."

The coordinator's reading, not the operator's word: `surfd` is the daemon; it holds several
sessions isolated from each other (today: one `surf-session` process per session); the
command-line programs are thin clients of it (today `surf-agent` is one). `surf <url>` runs
everything in its own process and would become a client of `surfd`; that is a consequence of
the end design, not planned here and not drafted.

Against this epic and the plan read for it:

- S2 = a (engine, DOM and parser in the session process) matches "one process is actually
  enough", and `surfd` keeps one process per session.
- Scripts run only in sessions of `surfd` (S7, the operator's pick): `surf <url>` gets no script
  option until it is a thin client of `surfd`, so no page script runs outside a hardened session.
- `AGENTS.md` ("Implementation targets") names an isolation target of one process per component
  (blocked on beyond10x/ess#792) and then a micro VM per component. The end design isolates
  sessions inside `surfd`; the two are not reconciled in the plan. This epic depends on neither.
- `epic:e2e-scenario-runner` plans `surf run` to run a scenario in process
  (`story:runner-run`); by S7 it runs no page script, so a scenario of a page that needs its
  scripts cannot run there until `surf run` is a client of `surfd`. That epic is not changed here.

## Answers of the script engine's session

The session of the script engine placed the ten requirements of "The gaps" against its design and code on 2026-11-22 and drafted 13 stories in three batches (H1, H2, H3), built after its budget batch. Its answers, and what they change here:

- Requirement 4 (a weak host reference) is withdrawn as a need of surf. A node-to-wrapper map that does not keep the wrapper alive loses what a script set on the wrapper while its node is still in the tree. surf keeps one wrapper per node strongly for the life of the document, bounded by the parser's node limit. The engine writes down what a kept handle already does.
- Requirement 3 gains steps: the host charges and credits bytes and charges steps for its own work on the engine's one ledger, through `Cx`. The generated bindings charge steps in proportion to the work of a member (a tree walk, a text copy).
- Requirement 9 gains `%Array.prototype.entries%`, `keys` and `forEach` beside `values`: Web IDL needs them for an iterable `NodeList`. The intrinsics and the well-known symbols are two closed enumerations.
- Requirement 1 is a read on the agent of the line and column of the last entry's parse error or uncaught throw; requirement 5 reads an error's kind from its own slot and its own `message` without running code; requirement 8 is `Cx::set_prototype_of` and `get_prototype_of`; requirement 10 is stated ceilings and a measuring test of 300 host functions and 40 host objects, not an exact cost per item; requirement 7 is a committed snapshot of the host interface with a gate check; requirement 6 is shown in the engine's own development and release profiles, and surf tests its own profile.
- Order: H1 (7, 8, the first part of 9, 4) gates `webidl-bindings` together with 10; H2 (2, 3, 10, 5, 1) gates the first page script; H3 (the rest of 2 and 9, 6) comes before the tag. The tag surf pins comes after the engine's slice 6 closes with these tested.

The picks among the engine's options for the host interface (its G1, G3, G4, G5, G8, G9: option A each) were made by the coordinating agent for the operator on 2026-11-22 and are reported to him.

Later the same day the engine's session revised its 13 stories after four critics (its requirements HOST-172 to HOST-187). What changed for surf:

- Host bytes are a handle: `charge_host_bytes(n)` answers a `HostBytes` value, and dropping it credits its charge, also when host data is dropped during a sweep. There is no free-standing credit call. surf keeps one handle per node's bytes in the node's host data. surf asked for a way to change a handle's charge in place when a node's bytes change, so that a node is not charged twice for a moment.
- `HostObjectSpec` gains `error_kind`: a host object can carry the slot of an error, as Web IDL asks for `DOMException`.
- The cost ceilings are constants of the engine (`HOST_FUNCTION_LIVE_BYTES`, `HOST_OBJECT_LIVE_BYTES`, `PROPERTY_LIVE_BYTES`, `BINDINGS_REALM_HEAP_BYTES`). The bindings test reads the constants and does not copy their numbers.
- Batches as revised: H1 holds the changelog rule, the prototype calls and the stack tests; H2 the kept handle, the closed list of intrinsics, string and object charging, host bytes and steps; H3 the error summary, the cost ceilings, the string builtins, the iteration intrinsics and the error site.

Later the same day the engine's session narrowed HOST-70 (its question D28, option A; surf did not object, decision D128 of the coordinating agent). What it states now, and what surf does:

- After `Terminated(StepBudget)` or `Terminated(Interrupted)`, `state()` reads `Alive` and a following script runs. After `Terminated(MemoryBudget)`, `state()` reads `Alive` and the calls that take no memory answer: `state()`, `collect()`, reading and dropping a root the host kept, dropping the agent (HOST-188).
- An entry whose parse and script need more than the room `memory_bytes` − `heap_bytes()` ends in `Terminated(MemoryBudget)` again, with the agent still `Alive` (HOST-189). Once the host made room, by dropping roots and calling `collect()`, the script `1` answers `Normal(1)` when the room is at least 3,072 bytes (HOST-190).
- surf: after an entry that ends in `Terminated(MemoryBudget)`, surf calls `collect()` once. If the next entry of that document ends in `Terminated(MemoryBudget)` again, surf runs no further script in that document.
- Requirement 1 is built in the engine as `Agent::last_entry_error_site`, which answers the line, the column and whether the code was dynamic or host defined, after a parse error or an uncaught throw (HOST-172 to HOST-174).

On 2026-11-24, with its release 0.8.0, the engine's session read `testharness.js` for the planning of its slice 4. What it states, and what surf does:

- `testharness.js` lines 1257 and 1260 call `Date.now()` in the busy wait of `fake_set_timeout`. With a host clock that does not advance, that loop ends only at the step budget. In 0.8.0 `Date` is not defined; it comes with the first wave of slice 4, which is not built yet.
- surf: the clock hook `host_system_utc_epoch_nanoseconds` (HOST-125) answers a value that advances between two calls. A fixed clock, as a test might choose for a stable output, makes that busy wait run to the step budget; a test that fixes the clock advances it by a fixed amount per call.
- The seven `Math` calls of the harness (`floor`, `min`, `max`, `abs` twice, `ceil`: `testharness.js` lines 1708 to 1710, 1783, 1812 and 3018, and `common.js` line 5) depend on no answer of the host.
