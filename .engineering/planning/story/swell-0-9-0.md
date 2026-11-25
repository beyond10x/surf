---
format: aep.planning-md/3
id: story:swell-0-9-0
kind: story
status: active
title: surf takes swell 0.10.0, runs the microtask checkpoint after each script, and maps swell's new error kind and host error
summary: The tag moves to 0.10.0, which holds 0.9.0 and 0.9.1, in one place with the lock, the allow-list, the test constants, NOTICE and the tag sentences; the runner performs HTML's microtask checkpoint after each script under the document's totals and clock and clears the queue after a limit; AggregateError gets its word and a wildcard arm whose swallowing a test notices; HostError::MemoryBudget joins surf_defect!(); clock_advances runs; the test that swell 0.9.1's interrupt fix breaks is rewritten; the design note is revised at 0.10.0, with requirement 11 held and HOST-216 to HOST-220 recorded.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-engine-dependency
- depends_on: story:webidl-bindings
- depends_on: story:script-error-kind-aggregate
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: NOTICE
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/script/Cargo.toml
- confidence: cited
  path: crates/script/src/runner.rs
- confidence: cited
  path: crates/script/tests/adversary.rs
- confidence: cited
  path: crates/script/tests/adversary_3.rs
- confidence: cited
  path: crates/script/tests/adversary_6.rs
- confidence: cited
  path: crates/script/tests/adversary_checkpoint.rs
- confidence: cited
  path: crates/script/tests/adversary_checkpoint_2.rs
- confidence: cited
  path: crates/script/tests/bindings.rs
- confidence: cited
  path: crates/script/tests/engine.rs
- confidence: cited
  path: crates/script/tests/run_end.rs
- confidence: cited
  path: crates/surf/tests/adversary.rs
- confidence: cited
  path: crates/surf/tests/lock_check.rs
- confidence: cited
  path: docs/badges/make.sh
- confidence: cited
  path: docs/badges/tests.svg
- confidence: cited
  path: docs/design/scripts.md
- confidence: cited
  path: taskfiles/lock-allow.txt
- confidence: cited
  path: website/docs/how-it-is-built.md
- confidence: cited
  path: website/docs/principles.md
- confidence: cited
  path: website/docs/status.md
- confidence: cited
  path: website/docs/what-works.md
revision: 29
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T03:16:34Z", actor: "human:operator", revision: 16}
- {from: "proposed", to: "active", at: "2026-11-25T10:35:48Z", actor: "human:operator", revision: 23, executor: "agent:coordinator"}
---
## Context

surf depends on swell by a git tag, `0.8.0`, written once in `crates/script/Cargo.toml:16`. swell
has released three tags since: `0.9.0` on 2026-11-24, commit
`d49a09bcc44fbdd155f705f913a90b676518afbc`; `0.9.1` on 2026-11-25, commit
`4e5c98914be5f42ce2908b56c30cae71f5395d24`; and `0.10.0` on 2026-11-25, commit
`b9d3d46fa7ac4abc982f2248e903e6301e277a9f` (`git -C <swell clone> rev-parse '<tag>^{commit}'`).
This story pins `0.10.0`, which holds the other two. Its id keeps the name of the first tag it
was written for. swell's note of `0.10.0` calls the release additive: "Nothing else changes"
beyond the two items of "What `0.9.1` and `0.10.0` add" below (`CHANGELOG.md:13-27 @ 0.10.0`).
Every swell citation below is read with `git show <tag>:<path>` in a clone of
`https://github.com/beyond10x/swell` and written `<path>:<line> @ <tag>`; every surf citation is
`main` at `77e8f3f`.

What `main` holds that an earlier text of this story took for a branch: `story:webidl-bindings`,
`story:script-error-kind-aggregate` and `story:html-script-hook` are `implemented` and merged. So
the generated bindings (`crates/script/src/bindings.rs`, from `crates/webidl/src/prelude.rs`),
`Runner::document_end` and the function `end_with_limit` (`crates/script/src/bindings.rs:445-452`)
are on `main`, the specification's enum `surf.document.ScriptErrorKind` has `AggregateError`
(`spec/domains/document.yaml:200`), and the parser's script hook exists. `story:scripts-dom-reading`
is `active` and not on `main`; it keeps the pin at `0.8.0` and regenerates `bindings.rs`.

What a change of tag is followed by, from `story:scripts-engine-dependency`, "As built"
(`.engineering/planning/story/scripts-engine-dependency.md:328-331`): `Cargo.lock`, six lines of
`taskfiles/lock-allow.txt`, the constant `SWELL` in `crates/surf/tests/lock_check.rs`, `NOTICE`
and the citations `@ 0.8.0`. Read on `main`, the tag or its commit is also written outside those
places:

- `crates/surf/tests/lock_check.rs`: the constant `SWELL` (`:21-22`) and the literals `swell 0.8.0`
  (`:381`, `:428`), `swell-num 0.8.0` (`:537`), `?tag=0.8.0#` (`:391`) and the commit prefix
  `#0c18` (`:398`), and `0.8.0` in the comment at `:378`. At the commit of `0.10.0` the
  replacement of `:398` finds nothing to replace, so the case
  `a_swell_package_at_another_commit_fails_naming_it` would hand the check the listed source
  unchanged.
- `crates/surf/tests/adversary.rs`: a second constant `SWELL` (`:11-12`), the commit as a literal
  (`:216`) and `0.8.0` in sources, messages and lock fragments (`:127-143`, `:207`, `:213`,
  `:221-237`, `:257`, `:267-268`, `:301-308`, `:377`, `:393-398`). `git grep -c -E
  '0\.(8|9|10)\.[0-9]|0c1827|b9d3d46|d49a09b' -- crates/surf/tests/adversary.rs` counts 28 lines
  and the same pattern counts 6 lines in `lock_check.rs`.
- `NOTICE:54`, `AGENTS.md:366` and `:1901`, `README.md:254`, `website/docs/principles.md:27`.
  These strings also hold surf's own release numbers (`0.8.0` and `0.9.0` are surf releases as
  well), so a search for the tag names `swell` or the words `release tag` beside it.

`Cargo.lock` holds 58 packages, 25 from outside this repository (`grep -c '^\[\[package\]\]'
Cargo.lock` prints 58, `grep -c '^source = ' Cargo.lock` prints 25). swell `0.10.0` has the same
eight packages in its root lock, with only the version lines changed (`git diff 0.9.0 0.10.0 --
Cargo.lock` shows eight lines `version = "0.9.0"` replaced by `"0.10.0"`; `git show
0.10.0:Cargo.lock | grep -c '^source'` prints 0), and `swell` takes the same five by path
(the five `path = ...` lines under `[dependencies]`, `crates/swell/Cargo.toml:13-17 @ 0.10.0`).
Its workspace version is `0.10.0` (`Cargo.toml:8 @ 0.10.0`) and its root manifest sets
`overflow-checks = true` in the same four profiles (`Cargo.toml:10-20 @ 0.10.0`). Both
repositories pin rustc 1.99.1 (`rust-toolchain.toml`, and the same file `@ 0.10.0`). swell's
`NOTICE` is the same file at `0.8.0` and `0.10.0` (`git diff --quiet 0.8.0 0.10.0 -- NOTICE`
exits 0). So the counts stay 58 and 25 and only the six packages' version and source change.

What `0.9.0` changes for a host, from `CHANGELOG.md:187-264 @ 0.10.0` ("### Host interface" of
`0.9.0`):

- "Breaking: `ErrorKind` is `#[non_exhaustive]`. A `match` over it in a host needs a wildcard
  arm" (`:211-213`), and "New: `ErrorKind::AggregateError`. [...] `ErrorKind::ALL` keeps its seven
  entries" (`:214-216`). `ErrorKind::name()` answers `"AggregateError"` for it
  (`crates/swell/src/host/mod.rs:70-81 @ 0.10.0`; the enum is `:39-55`, `ALL` `:59-67`).
- "Breaking: `HostError::MemoryBudget` is a new variant, and `HostError` is not
  `#[non_exhaustive]` [...] `queue_microtask` answers it when the memory budget refuses the growth
  of the microtask queue, with nothing queued" (`:242-245`; the variant is
  `crates/swell/src/host/outcome.rs:67-68 @ 0.10.0`). The only place the engine answers it is
  `enqueue_microtask` (`crates/swell/src/host/jobs.rs:207 @ 0.10.0`; `git grep -n
  'HostError::MemoryBudget' 0.10.0 -- crates/swell/src` lists no other answer outside tests and
  doc comments), whose only callers are `Agent::queue_microtask` and `Cx::queue_microtask`
  (`crates/swell/src/host/agent.rs:713-716`, `crates/swell/src/host/mod.rs:1050-1062 @ 0.10.0`),
  and the table of misuse names `queue_microtask` alone (`docs/design/host-interface.md:654 @
  0.10.0`). surf calls no `queue_microtask`.
- New: `Agent::perform_microtask_checkpoint`, `Agent::queue_microtask`, `pending_microtasks`,
  `clear_microtasks`, `CheckpointReport` (`:189-193`); `PromiseState`, `Agent::promise_state`,
  `promise_is_handled`, `mark_promise_as_handled` (`:204-209`); slice 10 delivered, so promises,
  `async` functions, async generators, `for await` and `Array.fromAsync` run; the engine calls
  `host_promise_rejection_tracker` (`:130`).
- `Date` over the host's clock: the constructor, `Date.now()`, `getTime`, `setTime`, `valueOf`;
  `Date.UTC`, `Date.parse` and the other 14 setters throw a `TypeError`; the getters of the fields
  do not exist; `Math` but `max` does not exist (`:56-62`, `:257-264`).
- "A buffer whose size a script controls and that the engine holds while it calls into script
  [...] is charged to `memory_bytes` before it is made and credited when it is dropped, or is not
  made" (`:178-183`), held by HOST-215 (`conformance/host/requirements.txt:215 @ 0.10.0`, `test
  swell::budgets::host_215_a_buffer_held_across_a_call_into_script_is_charged`). swell's list of
  open findings no longer holds the two copies it listed at `0.8.0`
  (`docs/plan/second-batch.md:284-285 @ 0.8.0`) and says "Closed after the batch [...] by
  `story:copies-held-across-a-call-into-script-are-charged` (HOST-215)"
  (`docs/plan/second-batch.md:302-304 @ 0.10.0`). This is requirement 11 of the note
  (`docs/design/scripts.md:1282`).
- `HOST-147` no longer names a `Map`; the key order of a `Map` is `HOST-214`, `pending slice 9`
  (`:232-234`; `conformance/host/requirements.txt:214 @ 0.10.0`).

What `0.9.1` and `0.10.0` add, from `CHANGELOG.md:13-45 @ 0.10.0`, and what this story does with
each (decision 12):

- The interrupt fix of `0.9.1` (`:34-38`): the flag is read after a builtin or host function
  called by a call or a construction, with or without spread arguments, answers by a throw, as
  after a return. A set flag ends the run in `Terminated(Interrupted)`, and "the thrown value
  reaches no `catch`". The code is `Agent::check_interrupt_after`
  (`crates/swell/src/interp/budget.rs:47-52 @ 0.10.0`) at the four call sites
  (`crates/swell/src/interp/dispatch.rs:1428-1429`, `:1559-1560`,
  `crates/swell/src/interp/spread.rs:151-152`, `:208-209 @ 0.10.0`). It reads the flag and spends
  no step (`budget.rs:35-41`), and an answer that is already a termination keeps its reason and
  is not checked (`budget.rs:44-46`). With no interrupt set the run is the run of `0.9.0`.
- `HOST-216` and `HOST-217` (`conformance/host/requirements.txt:216-217 @ 0.10.0`, each with a
  test in `swell::interrupt`): a host function that sets the interrupt and then throws ends the
  run in `Terminated(Interrupted)`, with the flag clear after and `x` still `undefined` in
  `try { f() } catch (e) { x = 1 }` (`docs/design/host-interface.md:1335 @ 0.10.0`); a host
  function that answers `Err(Abrupt::Terminated(Termination::Interrupted))` with the flag clear
  ends the run the same way, the agent stays `Alive` and a following script `1` answers
  `Normal(1)` (`:1337`).
- `HOST-218` has a test and `HOST-219` is `pending slice 12`
  (`conformance/host/requirements.txt:218-219 @ 0.10.0`): a String or a Symbol that host data
  lists in its `trace` with `Tracer::value` survives `collect`, as an object does, and a BigInt
  will once slice 12 makes one (`docs/design/host-interface.md:1828-1830 @ 0.10.0`; the
  signature is `:1743`).
- `to_utf16_len(&self, string: StringRef) -> Result<usize, HostError>` on `Cx` and `HookCx`
  answers the length of a String in code units without a copy, lone surrogates included
  (`crates/swell/src/host/mod.rs:548-550 @ 0.10.0`; `docs/design/host-interface.md:446`),
  held by `HOST-220` (`conformance/host/requirements.txt:220 @ 0.10.0`, with a test).
- swell's committed snapshot of its host interface lists the difference. `git diff 0.8.0 0.10.0 --
  conformance/host/surface.txt` removes one line besides the header and the requirement hashes,
  the line of `ErrorKind` without `#[non_exhaustive]`, and adds the new items. No signature that
  surf calls changes. The new items that are enum variants: `ErrorKind::AggregateError`,
  `HostError::MemoryBudget` and `WellKnownSymbol::AsyncIterator`
  (`crates/swell/src/host/host_object.rs:116-118 @ 0.10.0`). `WellKnownSymbol` and `HostIntrinsic`
  were `#[non_exhaustive]` at `0.8.0` already and no `match` over them exists in surf; the
  generated code names `WellKnownSymbol::ToStringTag`, `::Unscopables` and `::Iterator` as
  arguments only (`crates/script/src/bindings.rs:1304-1313`). `PromiseState` is a new enum,
  `Termination`, `Outcome`, `Abrupt` and `Parsed` have the variants they had.

How the checkpoint works at `0.10.0` (`crates/swell/src/host/agent.rs:751-773 @ 0.10.0`;
`docs/design/host-interface.md:1197-1226 @ 0.10.0`): one outermost entry with its own step budget
of `steps_per_entry`, one step per entry taken off the queue; it answers `Normal(CheckpointReport)`
only once the queue is empty; a termination (`StepBudget`, `MemoryBudget`, `Interrupted`) ends the
whole checkpoint and leaves every entry not started queued; the interrupt is read at its start,
before each entry and inside a job, and cleared; `last_entry_steps()` then reads the steps of the
whole checkpoint (HOST-193, HOST-197; `agent.rs:751-761`), and the checkpoint clears the error
sites at its start (`agent.rs:754`), so after it `last_entry_steps()` and
`last_entry_error_site()` read the checkpoint, no longer the script; it never answers `Throw` (a
throw there faults the agent, `agent.rs:765-769`); on a faulted agent it answers
`Err(HostError::Faulted)` (`agent.rs:752`). A promise job that ends in a throw "ran to its end"
(`crates/swell/src/host/jobs.rs:358-359`, `:408-411 @ 0.10.0`): the checkpoint goes on, and
`host_clean_up_after_job` receives `Err(value)` (HOST-82). `clear_microtasks` drops every queued
entry unrun, credits its charge, runs nothing and spends no step (`agent.rs:726-736 @ 0.10.0`,
HOST-200). swell's note leaves the clear to the host: "The engine drops none of them: HTML stops a
script without clearing the microtask queue, and only the host knows whether the page goes on"
(`docs/design/host-interface.md:1211-1212 @ 0.10.0`). Generic, timeout and cleanup jobs are
pending: HOST-85 and HOST-86 `pending slice 14`, HOST-87 `pending slice 16`
(`conformance/host/requirements.txt:85-87 @ 0.10.0`), and `git grep -n
'host_enqueue_generic_job\|host_enqueue_timeout_job\|host_enqueue_finalization_registry_cleanup_job'
0.10.0 -- crates/swell/src` lists only their declarations (`crates/swell/src/host/hooks.rs:246`,
`:249`, `:253`), their default bodies in the test host of that file and doc-test stubs, and no
call. The module comment of `hooks.rs` still says no engine path makes a `Job` "before slices 10
and 11" (`crates/swell/src/host/hooks.rs:7-9 @ 0.10.0`); `hooks.rs` is the same file at `0.8.0` and
`0.10.0` (`git diff --stat 0.8.0 0.10.0 -- crates/swell/src/host/hooks.rs` lists nothing), so that
sentence was not revised when slice 10 was delivered, and this story does not rest on it. The
rejection tracker's default does nothing (`hooks.rs:255-262 @ 0.10.0`).

The HTML Standard (multipage `webappapis.html`, "Last Updated 24 November 2026"; read for revision
17 of this story on 2026-11-24 and carried here, not read again): "run a classic script"
(`#run-a-classic-script`) takes a parse error as the script's error to rethrow (step 6), reports
an abrupt completion (step 8.3) and calls "clean up after running script" on both paths (steps 8
and 9); "clean up after running script" (`#clean-up-after-running-script`) step 3: "If the
JavaScript execution context stack is now empty, perform a microtask checkpoint." "Perform a
microtask checkpoint" (`#perform-a-microtask-checkpoint`) runs the queue until it is empty, then
"notify about rejected promises" (step 4). HostEnqueuePromiseJob (`#hostenqueuepromisejob`) step
2.4: "If result is an abrupt completion, then report an exception". HostPromiseRejectionTracker
(`#the-hostpromiserejectiontracker-implementation`) appends a rejected promise to the global's
about-to-be-notified list, and "notify about rejected promises" fires `unhandledrejection`, after
which "the user agent may report p.[[PromiseResult]] to a developer console".

What the plan already says about the checkpoint: the epic, "The gaps"
(`.engineering/planning/epic/scripts-in-a-page.md:302-314`): surf "drains the queue with
`agent.perform_microtask_checkpoint()`, which surf calls after each script", and calls
`Agent::clear_microtasks()` "when a limit ended a script or its checkpoint, so that a page that
floods the queue cannot spend the next scripts' budgets" (`:311-313`);
`story:scripts-document-port`, in its section "Waits for" before this story took the work over:
"the microtask checkpoint after each script, its steps counted in that script's and the
document's totals, and its end taken as the script's end. Open until then: which call of the host
interface runs the jobs, and what a job that ends at a limit makes of the record of a script that
already ended `ran`." This story answers both; that section no longer holds them. The epic and
two stories (`story:scripts-parse-run`, `story:scripts-document-port`) named `0.9.0` as the tag
this story takes; those lines were corrected with this revision to `0.10.0` (Placement, last
paragraph).

Where surf meets the changed types, on `main`:

- `crates/script/src/runner.rs:21-37`, the macro `surf_defect!()`, names six `HostError`
  variants; seven patterns `HostError::Faulted | surf_defect!()` (`:588`, `:644`, `:705`, `:712`,
  `:747`, `:803`, `:816`) are exhaustive at `0.8.0` and are not at `0.10.0`. The macro's doc
  comment (`runner.rs:21-27`) reads "swell's `HostError` at `0.8.0` but `Faulted`" and "Each is a
  defect of surf's own calls that a page cannot cause". `crates/script/src/bindings.rs` reads
  every `HostError` as a defect without a `match` (the function `defect`, `:573-577`, and `engine`,
  `:579-583`), and so does the generated copy in `crates/webidl/src/prelude.rs:505-512`; neither
  names a `HostError` variant.
- `Runner::evaluate` (`runner.rs:618-657`) runs the script (`Runner::run_script`, `:698`, which
  reads the error summary and site of a `threw` through `Runner::error`, `:737`, and
  `Runner::site`, `:795`), passes its end through `Runner::document_end` (`:668`), then adds the
  steps, `thrown_steps` when `Runner::error` read them before the read of a `DOMException`'s data
  or else `last_entry_steps()` (`:636-646`), then applies the memory rule (`:647-655`). The rule
  of `document_end` as built: a limit the bindings recorded during the entry is the script's
  end, in the words of Q4, whatever swell answered, except `limit: memory` and `faulted`, which
  keep their own word; the record is cleared and the interrupt withdrawn after every script that
  recorded one (`runner.rs:656-692`). A DOM write over a parser limit goes through
  `bindings::end_with_limit` (`crates/script/src/bindings.rs:445-452`): it records the limit,
  sets the agent's interrupt, reads it at once with `Cx::charge_steps(0)` and answers that
  termination, `Terminated(Interrupted)`, so that "no `catch` and no `finally` of the page runs
  after the limit" (its doc comment); should the charge answer `Ok`, the function answers
  `Terminated(Interrupted)` itself. The test `no_catch_and_no_finally_runs_after_a_limit`
  (`crates/script/tests/bindings.rs:799`) holds it.
- `runner.rs:654`, `_ => {}`, is a wildcard over surf's own `ScriptEnd`, not over a swell enum.
- `runner.rs:222-247`, the enum `ScriptErrorKind`, has the seven kinds and `Value`; its doc comment
  says "one of swell's seven `ErrorKind` variants". `Runner::error` matches it with no wildcard
  and no `AggregateError` (`:752-767`). `runner.rs:895-908`, `error_kind`, names the seven
  `ErrorKind` variants with no wildcard; its doc comment says a later kind "stops the build
  here". "As built" of `story:scripts-engine-dependency`
  (`.engineering/planning/story/scripts-engine-dependency.md:352-354`): "`surf-script` names every
  variant of swell's error types, with no catch-all arm". `#[non_exhaustive]` ends that for
  `ErrorKind`: outside swell a `match` over it must have a wildcard arm. The note decided the arm
  (`docs/design/scripts.md:455-459`): "the tag change that brings it adds the word
  `AggregateError` to this list and to `ScriptErrorKind`, and a wildcard arm maps a kind surf does
  not know yet to `Error`". The note's list and its copy of the specification's enum still have
  seven kinds (`docs/design/scripts.md:452-454`, `:534-545`), while the specification has eight
  (`spec/domains/document.yaml:192-202`): `story:script-error-kind-aggregate` wrote the
  specification and left the note and the runner to this story (its "As built", last item).
- `runner.rs:201-203`: the comment "No engine path makes a job or a load request before swell's
  slices 10 and 11 [...]: promises and modules are refused at the tag" is false at `0.10.0` for
  promises.
- `crates/script/tests/engine.rs:341-378`: three tests ignored, `clock_advances` (`HOST-125`),
  `seeds_fixed` (`HOST-26`), `time_zone` (`HOST-127`). At `0.10.0` `HOST-125` and `HOST-126` have a
  test (`conformance/host/requirements.txt:125-126 @ 0.10.0`); `HOST-26` and `HOST-127` are
  `pending slice 4` (`:26`, `:127`). `Date.now()` "ask[s] the host's clock hook once
  [...] floor[s] the answer to milliseconds" (`CHANGELOG.md:56-59 @ 0.10.0`), and the test clock
  starts at a whole millisecond and advances 1,000,000 ns a call (`runner.rs:55-61`), so
  `clock_advances` reads `b - a` as 1. `Math.random` and `getTimezoneOffset` do not exist at
  `0.10.0` (`CHANGELOG.md:260-264`).
- `crates/script/tests/adversary.rs:96-120`, `every_error_kind_keeps_its_word`, lists the seven
  kinds.
- `crates/script/tests/adversary_6.rs:170-185`, the test of a record read that an interrupt ends,
  rests on the behaviour `0.9.1` fixed (decision 11 names it and replaces it). The
  test slice for `document.getElementById("interrupt")` sets the agent's interrupt and then
  throws a `DOMException` (`crates/script/tests/adversary_5.rs:95-98`); the test's doc comment
  says the script "ends `threw`" and the test asserts `Ok(None)` for the record of that end. That
  is the case of `HOST-216`: at `0.10.0` the run ends in `Terminated(Interrupted)`, which the
  runner reads as `limit: wall-clock` (`runner.rs:886-893`), and the helper `record`
  (`adversary_6.rs:15-20`) turns such an end into `Err("limit: wall-clock")`. By reading, the test
  fails at `0.10.0`; nobody has run it (decision 11). Its sibling comment at
  `crates/script/tests/adversary_3.rs:312-316` says "the engine reads the interrupt only after a
  host call that returned normally", which `0.9.1` made false for a throw.
- A test comment states what `0.10.0` makes stale: `crates/script/tests/adversary_5.rs:226` says
  "Swell's host interface @ 0.8.0, section 4: a handle listed in the `trace` of host data the
  engine reaches is strong", for Strings that `HOST-218` now holds with a test.
- Four doc comments in the generated code state what `0.9.1` and `0.10.0` make stale
  (`crates/webidl/src/prelude.rs`, copied to `crates/script/src/bindings.rs`): `ExceptionData`
  says that a String listed with `Tracer::value` lives "from the sentence and the signature, not
  from a requirement" (`prelude.rs:236-246`), which `HOST-218` now is; `end_with_limit` says the
  answer `Terminated(Interrupted)` of a host function "works at the tag but swell does not promise
  it there" (`prelude.rs:365-380`), which `HOST-217` now promises; `string_length` reads the length
  through `ToObject(string).length` (`prelude.rs:782-794`), which `to_utf16_len` (`HOST-220`)
  replaces without an engine call; `exception_text` says an interrupt "set after the throw" ends
  the read (`prelude.rs:310-313`), which stays true. The generator's output carries digests of
  its sources; a change of these comments is a change of `prelude.rs`, a regeneration with the
  task `webidl-check` and a rewrite of `bindings.rs`, which `story:scripts-dom-reading` rewrites
  as well. Decision 12 leaves these four and the test comment out of this story.

The swell citations in surf's code that move with the tag (`git diff --stat 0.8.0 0.10.0 --
<path>`; a file the diff does not list keeps its lines): `crates/syntax/src/parser/statement.rs`
changed, and the refusal of `class` moved from `:218` to `:221 @ 0.10.0`
(`Keyword::Class => return self.refuse("class", 8)`); `crates/swell/src/interp/budget.rs` changed
in `0.9.1`, and the `RangeError` "frame_depth exceeded" moved from `:45` to `:58 @ 0.10.0`;
`crates/syntax/src/lib.rs` changed, the sentence cited at `:16` is still at `:16 @ 0.10.0`;
`crates/swell/src/object/builtin_function.rs` changed, and the line
`Err(Abrupt::Terminated(termination)) => Err(self.terminate(termination))`, cited by the
generated code, moved from `:497` to `:876 @ 0.10.0`; `crates/syntax/src/error.rs`,
`crates/swell/src/script.rs`, `crates/swell/src/host/limits.rs`,
`crates/swell/src/host/hooks.rs`, `crates/swell/tests/support/stack.rs`,
`crates/swell/tests/limits_scripts.rs` and `crates/swell/tests/eval_stack.rs` are the same at
`0.8.0` and `0.10.0`.

Requirement 11 (`docs/design/scripts.md:1282`, `absent` at `0.8.0`): the note left its detail out
until a tag closed it (Q13, `:1074-1080`; decided on 2026-11-24, epic "Decisions taken"). The
detail at `0.8.0`, each line readable at that tag: the separator of `Array.prototype.join` copied
by `string_units` (`crates/swell/src/builtins/array/mod.rs:323-335 @ 0.8.0`;
`crates/swell/src/object/string.rs:46-53 @ 0.8.0`, which charges nothing) and held across `Get`
and `ToString` of each element; the String an object operand's `toString` returns, held by `sort`
without a comparator when that operand comes first
(`crates/swell/src/builtins/array/mutation.rs:758-759`, `:789-801 @ 0.8.0`); and the argument
lists of the `call` and `new` instructions (`crates/swell/src/interp/dispatch.rs:1390-1392`,
`:1469-1471 @ 0.8.0`). The probe of the note's Q13 (the program `swell-limits` with its modes
`join`, `sort-xy` and `sort-yx`, run by `m3-copies.sh` under
`prlimit --as=2147483648:2147483648 --core=0:0` with Q4's limits) is not in the repository.
Between the commit swell merged the charged buffers at (`f27c52e`, an ancestor of `0.9.0`: `git
merge-base --is-ancestor f27c52e 0.9.0` exits 0) and the tag `0.9.0`, no file under `crates/`
changed. The probe was built against the tree of the tag `0.9.0` (`git archive 0.9.0`) and run on
2026-11-24, 19:45 CET, after that tag was published: each of the three modes ended with exit
status 0; the script that asks for the copy ends `throw RangeError "reentry_depth exceeded"`
(3,607, 5,154 and 4,130 steps) and the doubling after it `terminated StepBudget`; `heap_bytes()`
reads 13,052,701, 13,054,051 and 13,054,051; the highest `VmHWM` the process prints itself (its
`stdout:` lines) is 37,512, 37,716 and 37,740 kB, and the highest the sampler beside it read (the
`exit 0` line of each mode) is 32,032, 33,116 and 33,656 kB, under an address-space bound of 2 GiB
that ended the same three modes with status 134 at `0.8.0` (the note's Q13). The unit is handed
the run's output lines as a file and does not build the probe again.

Those lines stay the measurement of this story at `0.10.0`, by reading and not by a new run. The
files of swell's engine that differ between `0.9.0` and `0.10.0` are
`crates/swell/src/heap/string.rs` (a method `len`), `crates/swell/src/host/mod.rs` (the method
`to_utf16_len`), `crates/swell/src/interp/budget.rs`, `crates/swell/src/interp/dispatch.rs` and
`crates/swell/src/interp/spread.rs` (`check_interrupt_after`, which reads the flag and spends no
step); `git diff 0.9.0 0.10.0 --stat -- crates/` lists these five and the tests of the three
items. A script of the probe sets no interrupt, so the changed reads find the flag clear and the
steps, heap sizes and ends it printed are those of `0.9.0`. The `VmHWM` values belong to the
process that ran on `0.9.0`.

## Decisions taken

1. The tag is `0.10.0`, commit `b9d3d46fa7ac4abc982f2248e903e6301e277a9f`, written once:
   `swell = { git = "https://github.com/beyond10x/swell", tag = "0.10.0" }` in
   `crates/script/Cargo.toml` (line 16; swell's changelog gives that form for `0.9.0` at
   `CHANGELOG.md:291-293 @ 0.10.0`, and `0.10.0` is written the same way). In the same commit:
   `Cargo.lock` (the six packages at version `0.10.0` with the source
   `git+https://github.com/beyond10x/swell?tag=0.10.0#b9d3d46fa7ac4abc982f2248e903e6301e277a9f`),
   the six lines of `taskfiles/lock-allow.txt` (`:8-13`; `<name> 0.10.0 <source> none`, Q15), and
   the test constants of decision 2. No package is added or removed; the counts stay those of the
   base.
2. The test files name the tag, the version and the commit once each. In
   `crates/surf/tests/lock_check.rs` and in `crates/surf/tests/adversary.rs`, two constants hold
   them: `SWELL`, the source with tag and commit (on `main` its value is on one line,
   `lock_check.rs:22` and `adversary.rs:12`), and one constant beside it that holds the version.
   Every other literal of the swell version, tag or commit in those files (Context, first two
   items) is built from the two; the case "at another commit" (`lock_check.rs:398` on `main`)
   changes the first hex digit of the commit `SWELL` holds, whatever it is, and the commit of
   `adversary.rs:216` is read out of `SWELL`; so the next tag changes the two constants and
   nothing else. `swell 0.8.1` in `adversary.rs:127-143` (a version beside the listed one)
   becomes the listed version with its patch number raised by one, `0.10.1`, built from the
   version constant. The comments at `lock_check.rs:378` and `adversary.rs:127` and `:213` are
   reworded without a version. One literal stays: `?tag=0.7.0#` in `lock_check.rs:391`, the
   "another tag" of `a_swell_package_of_another_tag_fails_naming_it`, which names no listed
   version; the string it replaces is built from the version constant.
3. `NOTICE:54` names the tag `0.10.0`; swell's `NOTICE` is the same file at both tags (Context),
   so the quoted text stays. `AGENTS.md:366` and `:1901`, `README.md:254` and
   `website/docs/principles.md:27` name `0.10.0`. The sentence of `AGENTS.md:366-369` about
   `surf-script` is stale beyond the tag ("no DOM yet", "29 tests", the three test files it
   lists): the unit rewrites it for what `crates/script` is on the tree the unit merges into
   (after `story:scripts-dom-reading` it holds `surf-url` and no "no DOM yet") and for the numbers
   of the gate log (acceptance 8).
4. The error kinds. `ScriptErrorKind` gains `AggregateError`, word `"AggregateError"`, mapped from
   `ErrorKind::AggregateError`, and `Runner::error` lists it with the kinds whose message is kept
   as read (`runner.rs:760-766`). `error_kind` keeps one arm per named kind and gains one wildcard
   arm, `_ => ScriptErrorKind::Error`, as the note decided (`docs/design/scripts.md:455-459`). The
   doc comments of the enum (`runner.rs:222-224`) and of `error_kind` (`:895-897`) are rewritten.
   The arm's comment names what it swallows: every kind swell adds after `AggregateError`, none at
   `0.10.0` (`crates/swell/src/host/mod.rs:39-55 @ 0.10.0`); the test below that notices such a
   kind; and what that test cannot notice: a kind that has no global constructor whose name ends
   in `Error` reaches the wildcard unnoticed. `ErrorKind::ALL` cannot serve the test: it keeps
   seven entries without `AggregateError` (`CHANGELOG.md:216 @ 0.10.0`;
   `crates/swell/src/host/mod.rs:57-67`). The test reads the realm instead: it lists the own
   properties of the global object whose names end in `Error`, throws `new <name>([])` for each
   (an `AggregateError` needs an iterable first argument, and `[]` is the message `""` for the
   others), and compares the end's kind by its word, `ScriptEnd::Threw(error)` with
   `error.kind.word()` equal to `<name>` (`ScriptErrorKind::word`, `runner.rs:238-248`), not the
   end's text: the seven other kinds end `threw <name> "" 1:1` (`new <name>([])` sets the message
   `""`, which `ScriptError`'s `Display` prints, `runner.rs:352-365`), `URIError` among them, and
   `new AggregateError([])` sets no message and ends `threw AggregateError 1:1`. A kind that
   reaches the wildcard reads `Error` and the test fails naming it. This is the one wildcard arm
   over a swell enum in `crates/script`; the rule of "As built" holds for every other enum,
   `HostError` included, which is not `#[non_exhaustive]` (`CHANGELOG.md:242-243 @ 0.10.0`).
   `WellKnownSymbol::AsyncIterator` and the other new variants break no `match` (Context).
5. `HostError::MemoryBudget` joins `surf_defect!()` (`runner.rs:28-37` on `main`). One reading
   for every call surf makes: at `0.10.0` only `queue_microtask` answers it
   (`crates/swell/src/host/jobs.rs:207 @ 0.10.0`, `docs/design/host-interface.md:654 @ 0.10.0`),
   and surf calls no `queue_microtask`, so from surf's calls it is a defect of surf's own calls,
   read as `HostError::Faulted` is, like the six variants the macro names. The macro's doc comment
   (`runner.rs:21-27`, "swell's `HostError` at `0.8.0` but `Faulted`") is rewritten for `0.10.0`:
   it names the seven variants, says why `MemoryBudget` is among them, and says that it leaves
   the macro with the first call of `queue_microtask` in surf, where surf decides its reading.
   That call is `queueMicrotask` of `story:scripts-event-loop-and-timers`, whose unit takes
   `HostError::MemoryBudget` out of the macro and handles it at the call (Out of scope).
   No arm of its own at any call; the seven patterns `HostError::Faulted | surf_defect!()` are
   exhaustive again, and the build is the check.
6. The microtask checkpoint, HTML's "clean up after running script", inside `Runner::evaluate`,
   in this order:
   - The script's own end is settled first, as `evaluate` does on `main`: `run_script` answers
     the end and reads the error summary and site of a `threw` or `refused`; `Runner::document_end`
     reads the end with the rule of Context (a limit the bindings recorded is the end whatever
     swell answered, except `limit: memory` and `faulted`; the interrupt is withdrawn); the steps
     of the script (`thrown_steps` or `last_entry_steps()`) are added to the step total. The
     checkpoint overwrites what `last_entry_steps()` and `last_entry_error_site()` answer
     (`agent.rs:754 @ 0.10.0`), so these reads come before it.
   - Then, after a script that ended `ran`, `threw` or `refused` (a parse error is run through
     "run a classic script" too, steps 6 and 8.3), the runner calls
     `Agent::perform_microtask_checkpoint()`. After `limit:` or `faulted` it calls none. A script
     `not run` is not run and gets no checkpoint. A limit a DOM write records ends the script
     whatever the script catches (`bindings::end_with_limit`, held by the test
     `no_catch_and_no_finally_runs_after_a_limit` of `crates/script/tests/bindings.rs`), so a
     script that reaches a DOM limit ends `limit: <word>`, gets no checkpoint, and its queue is
     cleared (below).
   - Its budget is swell's: one outermost entry with a step budget of its own, `steps_per_entry`
     (HOST-193). After it the runner adds `last_entry_steps()` to its step total (HOST-197), so
     the document's total of Q4 counts the jobs; the wall-clock bound of the runner is the same
     clock, and its interrupt ends a checkpoint as it ends a script (HOST-198). The script count
     does not rise for the checkpoint.
   - Its end passes through the same rule of `document_end`. `Normal(_)` leaves the script's end
     as it was. `Terminated(StepBudget)`, `Terminated(MemoryBudget)` and `Terminated(Interrupted)`
     replace it with `limit: steps-per-script`, `limit: memory` and `limit: wall-clock` (or, when
     a job recorded a DOM limit, that limit's word by the rule of `document_end`, so a job's DOM
     write over a parser limit ends the script `limit: nodes` or its word), and
     `Terminated(EngineFault)` with `faulted`: the checkpoint is a step of running that script
     ("run a classic script", step 9), and Q7 holds one end per script element. `Throw`, which
     the checkpoint never answers (`agent.rs:765-769 @ 0.10.0`), and `Err(HostError::Faulted)`
     or `Err(surf_defect!())`, which it answers on a faulted agent (`agent.rs:752`), are read as
     `faulted`, with those citations.
   - After any end `limit: <word>`, of the script or of its checkpoint, the runner calls
     `Agent::clear_microtasks()` before anything else: jobs a script queued before its limit are
     that script's work, and run by the next script's checkpoint they would spend its budget, the
     epic's reason for the clear (`.engineering/planning/epic/scripts-in-a-page.md:311-313`).
     This is wider than HTML and than swell's note, which keeps the queue after a script stopped
     ("HTML stops a script without clearing the microtask queue",
     `docs/design/host-interface.md:1211-1212 @ 0.10.0`): a deviation, named in the note's Q2
     paragraph (decision 13) with this reason; the epic's lines read "when a limit ended a script
     or its checkpoint". After `limit: memory` the runner clears first and then applies Q4's rule
     (one `collect()`, `not run: memory` after a second), so the collection finds the jobs'
     charge credited (HOST-200).
   - At a `script` end tag the queue is therefore empty: no script code runs between two
     scripts of a document (the first version has no timer and no event), each checkpoint ends
     with the queue empty or is followed by the clear. So step 1 of the end tag (the note's Q2,
     row 1) is taken in effect, with no call of its own.
7. A job's throw and an unhandled rejection are not recorded by this story. The two other forms,
   a job's throw as the script's end and a count per script in the record, need a question to
   swell or a change of the specification under its synthesis bound, and this story takes
   neither; `story:scripts-job-errors` (draft) holds them. So `host_clean_up_after_job` and
   `host_promise_rejection_tracker` keep swell's defaults, a script whose job threw or whose
   promise stays rejected ends as the script itself ended, and the note names the deviation
   from HTML's "report an exception" (`#hostenqueuepromisejob` step 2.4) and "notify about
   rejected promises" in Q2. A script element may so spend two entries' steps, its own and its
   checkpoint's (HOST-193). The document's step total and the wall clock bound the sum, so
   `steps_per_entry` is not halved: it stays one number, the one of Q4.
8. Generic, timeout and cleanup jobs and load requests stay dropped. The comment at
   `runner.rs:201-203` says so with HOST-85 and HOST-86 `pending slice 14` and HOST-87 `pending
   slice 16` (`conformance/host/requirements.txt:85-87 @ 0.10.0`), not with `hooks.rs:7-9`, whose
   sentence is stale at `0.10.0` (Context), and no longer says promises are refused.
9. `#[ignore = "HOST-125"]` goes from `clock_advances` (`engine.rs:341-342`); `seeds_fixed`
   (`HOST-26`) and `time_zone` (`HOST-127`) stay ignored with their ids, both `pending slice 4`
   at `0.10.0`; they run when a swell tag holds slice 4, which `story:scripts-seeds-and-time-zone`
   (draft) waits for. The module comment of `engine.rs` (`:1`, `:8-13`) names `0.10.0` and two
   ignored tests. The swell citations in `runner.rs` (`:51`, `:202`) and `engine.rs` (`:28-36`,
   `:399`, `:468`, `:489`, `:525`, `:541`) read `@ 0.10.0` with the lines of Context
   (`statement.rs:221`, `budget.rs:58`) or the lines they have (the files that did not change),
   and the doc comment of `ScriptHost`'s `Host` impl names HOST-85 to HOST-87 as decision 8 says.
   `crates/script/tests/adversary_3.rs:490` ("swell 0.8.0 has no `Reflect`") is true at
   `0.10.0` as well (`CHANGELOG.md:263-264`: no `Reflect`) and is reworded without a version.
10. Requirement 11 at `0.10.0` is `held`: by HOST-215 and its test, and by the probe's run on the
    tag `0.9.0` that Context carries to `0.10.0`. The unit writes the 18 lines of the file handed
    over into the note verbatim, each whole on its own line, in one fenced block in Q13 (no path
    of a machine is in them), and builds no probe. Acceptance 6 compares the note with the file.
11. The interrupt fix of `0.9.1` and the test it breaks. The runner needs no change for it, and
    `end_with_limit` keeps its `Cx::charge_steps(0)` (a termination is read the same way with or
    without the fix). The test `a_record_read_that_an_interrupt_ends_gives_no_text`
    (`adversary_6.rs:170-185`) is replaced by the test
    `interrupt_set_before_a_throw_ends_at_wall_clock_and_the_next_record_is_whole`, which holds
    what `HOST-216` says of surf's slice: the
    script `document.getElementById("interrupt");` ends `limit: wall-clock`, the interrupt flag
    is clear after it, and the next script, `throw new DOMException("m", "N");`, ends `threw Error
    "N: m" 1:1` with the record whole. The read-failure branch of `Runner::exception_text` (an
    interrupt set between the throw and the read) stays reachable: `HOST-216` covers a host
    function reached by a call or a construction, and a getter of a slice that sets the
    interrupt and throws leaves the run ended by a throw with the flag set. Decision 14 rules
    what the runner does with it. The
    comments at `adversary_3.rs:312-316` and in the doc comment of the rewritten test are
    reworded for `0.10.0`.
12. What the new requirements mean for surf, one line each. `HOST-216` and `HOST-217`: used by
    decision 11 and recorded in the note's Q4, in the paragraph on a DOM write over a parser limit.
    `HOST-218`: recorded in the note's Q4 beside the sentence on `DOMException` data
    (`docs/design/scripts.md:338-341`), where the Strings listed in `trace` now rest on a
    requirement. `HOST-219`: recorded in Q18 as `pending slice 12`; surf has no BigInt in a host
    object's data. `HOST-220`: recorded in Q18 and left to `story:scripts-utf16-length` (draft),
    which replaces the function `string_length` of `crates/webidl/src/prelude.rs` by
    `Cx::to_utf16_len` and rewrites the stale comments of Context (the four of the generated
    code and the one of `crates/script/tests/adversary_5.rs:226`); that change regenerates
    `bindings.rs` and belongs after `story:scripts-dom-reading` is merged, because both rewrite
    the generated file. This story edits neither `crates/webidl/src/prelude.rs` nor
    `crates/script/src/bindings.rs`. On `string_length`: swell states that a String the host
    makes with `Cx::new_string` is bounded by the memory budget and not by `string_length`
    (`docs/design/host-interface.md:165-168 @ 0.10.0`); `story:scripts-dom-reading` checks a
    length in its own glue, and the pin changes nothing there.
13. The design note, `docs/design/scripts.md`. The unit rewrites the sentences the tag makes
    false and cites them `@ 0.10.0`; every other citation stays `@ 0.8.0`, read at that tag and
    true there. A citation names the tag it was read at, and rewriting a true one would only
    cost a re-read of each (`git grep -o -F "@ 0.8.0" -- docs/design/scripts.md | wc -l` prints
    114 on `main`). The edits, each with the fixed text acceptance 6 and 9 check:
    - the head (`:12-15`, `:28`, `:36`): "swell is read at its tag `0.10.0`", commit
      `b9d3d46...`, on one line; a citation names its tag, and one still written `@ 0.8.0` was
      read at that tag and is true there;
    - Q2, row 1 (`:123`; its third cell reads "left out: no promise and no job exist" on
      `main`) reads "taken in effect: the runner performs a checkpoint after each script and
      clears the queue after a limit, so the queue is empty at every end tag (decision 6 of
      `story:swell-0-9-0`)", so its first words are `taken in effect: the runner performs a
      checkpoint after each script`; a paragraph after the table states decisions 6 and 7 with
      the HTML sections of Context, and names three deviations from HTML, each in a sentence
      that begins "Deviation from HTML": a job's exception is not reported (decision 7), a
      rejection no handler took is not notified (decision 7), and the queue is cleared after a
      limit, which HTML and swell's note do not do, with the reason of decision 6;
    - Q4: the cells of `memory_bytes` (`:303`) and `string_length` (`:307`) drop the sentence "It
      does not bound the copies swell lists as open" (only that sentence of `:307`: the reading
      story reads the `RangeError` text of that row and edits none of it), and the first says
      that the copies swell held uncharged at `0.8.0` are charged at `0.10.0` (HOST-215); the
      row "after `Terminated(MemoryBudget)`" (`:320`) says
      "`Agent::clear_microtasks()` first, then `collect()` once" (decision 6); the row of
      `steps_per_entry` (`:302`) says "at most two entries, the script and its checkpoint"
      (decision 7); the paragraph on a DOM write over a parser limit (`:255-265`) says that an
      interrupt set by a host function that then throws ends the run the same way (HOST-216)
      and that a host function may answer `Terminated(Interrupted)` itself (HOST-217); the
      sentence on `DOMException` data (`:338-341`) cites HOST-218;
    - Q6 (`:412-431`): HOST-125 and HOST-126 `test`, the six others `pending slice 4`, HOST-147
      without `Map` and HOST-214 `pending slice 9`, HOST-209 to HOST-213 new and `pending slice
      4`; its `probe:` line is run again at `0.10.0`;
    - Q7 (`:451-459`): "eight kinds", `AggregateError` among them, the wildcard of decision 4,
      what it cannot notice, and its test; the note's copy of the specification's enum
      `surf.document.ScriptErrorKind` (`:534-545`) gains `{name: AggregateError, summary:
      "AggregateError"}` before `Value`, as `story:script-error-kind-aggregate` wrote it into
      `spec/domains/document.yaml:200` (this story owns the note);
    - Q11: the sentences that call requirement 11 open (`:902-909`, `:917`, `:925`, the cell at
      `:963`) say it is held at `0.10.0`; so "while requirement 11 is open" (`:925`) and "and
      requirement 11 are closed" (`:963`) go, and the sentence "Requirement 11 (Q18) asks swell
      to charge every copy" goes at `:908` and `:1079`, and the same words at `:1257` are
      rewritten as held; the bound values stay, which `story:scripts-document-port` decides
      (its decision 3);
    - Q13 (`:1023-1091`): the tag `0.10.0` with its `probe:` line, which reads
      `probe: (workstation) git -C <swell clone> rev-parse '0.10.0^{commit}'` and the other
      commands of Context (commit, 8 packages, no `source` line), the commit
      `b9d3d46fa7ac4abc982f2248e903e6301e277a9f` in the text and in the `Settled:` line, the
      detail of requirement 11 at `0.8.0` (Context, last two paragraphs) and its close at
      `0.9.0`, the closing of slice 10 and of `Date`'s first wave; the decision text:
      `story:swell-0-9-0` takes `0.10.0`, and `story:scripts-document-port` takes no tag; and
      the 18 lines of the file handed over (decision 10), verbatim, each whole on its own line,
      in one fenced block under a `probe:` line that names the three modes and the bound;
    - Q18 and the table (`:1250-1282`): the counts at `0.10.0` on one line, "220 lines, 152
      `test`, 68 `pending`"; "status at `0.10.0`" where `:1253`, `:1258` and the table's head
      `:1270` say "status at `0.8.0`"; rows 1 to 10 `held` with their lines `@ 0.10.0` (row 7:
      `docs/design/host-interface.md:2252 @ 0.10.0` and `Taskfile.yml:117 @ 0.10.0`); row 11
      `held` with HOST-215, `conformance/host/requirements.txt:215 @ 0.10.0` and the probe of
      decision 10; a paragraph after the table lists HOST-216 to HOST-220 as decision 12 says,
      each with its line `conformance/host/requirements.txt:216-220 @ 0.10.0`.

14. What the review of the unit's first tip found, as rules. Each narrows the decision it names
    and wins where the two differ.
    - An interrupt that is set when a script ends is the script's end (decisions 6 and 11).
      After the script's own run answered, whatever it answered, the runner decides the end
      before it reads a thrown value and before the checkpoint: when the wall clock ran out or
      the interrupt is set, the script ends with the limit (`limit: wall-clock`, or the limit
      the bindings recorded), no text of a thrown value is read, no checkpoint runs and the
      microtask queue is cleared. A read of a thrown value that an interrupt ends all the same
      gives the same end and never a record `threw` with no message. The reason: the read's
      first engine call takes a set interrupt, and a checkpoint that starts after it runs the
      page's jobs past the wall clock, up to one `steps_per_entry`.
    - The clear of the queue after a script's own `limit: memory`, its place before the
      collection, and the collection itself are each held by a test.
    - Q4 of the design note names the two code-from-string paths new at `0.9.0`,
      `%AsyncFunction%` and `%AsyncGeneratorFunction%`, with the hook they pass; Q6 says that
      a page can time its own run against the wall-clock bound with `Date.now()`; Q2 says what
      bounds a script and its checkpoint at this tip (the wall clock and each entry's
      `steps_per_entry`) and names `story:scripts-document-port` for the document's step total.
    - Whether `HOST-216` means a host accessor too is swell's to rule and to fix. surf's rule
      above does not wait for a swell tag and stays when one comes.

15. What the review of the unit's second tip found, as rules. Each narrows decision 14 and wins
    where the two differ.
    - No script inherits an interrupt. After every script's end, whatever its end word, the
      runner withdraws a set interrupt in one place before it returns, and no way out of a
      script returns around it (a limit, `faulted`, `not run`, a parse error). The end word of
      a limit stays what swell or the bindings answered. A clock that ran out is known by the
      clock and not by the flag: the next script of the document is `not run: wall-clock` when
      the clock ran out, and runs when the flag was set while the clock had not run out.
    - Where the checkpoint reads the flag at `0.10.0`: at its start and before it takes each
      job off the queue; after a job's function answered when that function is a script
      function, and not when it is a host function. The design note and the runner's comments
      say so with the lines of the tag.
    - Two checks of decision 14 never decide an end alone at `0.10.0`: the check before the
      read of a thrown value, and the flag half of the check before the checkpoint. They stay
      as the second line: they decide when a later tag changes where the engine reads the
      flag, and the first keeps the rule that no text is read after an interrupt. The note
      and the two comments name them so.
    - The clear of the queue after `limit: wall-clock` and the four changes of the first fix
      round that no test held are each held by a test.

## Change

- `crates/script/Cargo.toml`, `Cargo.lock`, `taskfiles/lock-allow.txt`: decision 1, in the first
  commit, with the test constants of decision 2 (`crates/surf/tests/lock_check.rs`,
  `crates/surf/tests/adversary.rs`).
- `crates/script/src/runner.rs`: decisions 4 to 8 and 9.
- `crates/script/tests/engine.rs`: decision 9 and the cases of acceptance 3.
  `crates/script/tests/adversary.rs`: the eight-kind case `every_error_kind_keeps_its_word`
  (acceptance 4). `crates/script/tests/bindings.rs`: the cases `dom_limit_drops_the_jobs`
  and the `then` variant of `limit_end` (acceptance 4). `crates/script/tests/adversary_6.rs`:
  the test of decision 11. `crates/script/tests/adversary_3.rs`: decisions 9 and 11 (a comment
  and a version in a comment; no `TestWrap` changes).
- `NOTICE`, `AGENTS.md`, `README.md:254`, `website/docs/principles.md:27`: decision 3, and the
  entry of `surf-script` in `AGENTS.md` (`:365-369` on `main`).
- `docs/design/scripts.md`: decisions 10, 12 and 13, with the 18 probe lines in one fenced block
  in Q13.
- In the last commit, the public surfaces of acceptance 8 (`README.md`, `AGENTS.md`,
  `docs/badges/make.sh`, `docs/badges/tests.svg`, `website/docs/status.md`,
  `website/docs/what-works.md`, `website/docs/how-it-is-built.md`), among them the paragraph that
  names the gate run (the one that holds "It is the gate run of 2026-11-25 on `58d720b`",
  `README.md:658` to `:678`, `website/docs/how-it-is-built.md:102` to `:123` on `main`): its
  commit and numbers are rewritten from the unit's own log by the rule of the gate blocks,
  and a sentence another story added to it stays.

## Acceptance

1. `slot task lock-check` prints
   `Cargo.lock: <n> package(s), <m> from outside this repository, each on the allow-list` with the
   `<n>` and `<m>` of the story's base (58 and 25); `git grep -n -F -e 'tag=0.8.0' -e 'tag =
   "0.8.0"' -e 0c1827128b57845e203713e87a0b0949fc8d63be -- Cargo.lock taskfiles crates NOTICE`
   prints nothing; `git grep -c -F 'tag=0.10.0#b9d3d46fa7ac4abc982f2248e903e6301e277a9f' --
   Cargo.lock taskfiles/lock-allow.txt` counts 6 in each file; `git grep -n -F 'tag 0.8.0' --
   NOTICE` prints nothing and `git grep -c -F 'tag 0.10.0' -- NOTICE` counts 1.
2. `slot cargo nextest run -p surf lock_check:: adversary::` (package `surf`, `crates/surf`) exits
   0, within the bounds of acceptance 11 (no test over 60 s, the step within 120 s): every case of
   `crates/surf/tests/lock_check.rs` and `crates/surf/tests/adversary.rs` passes, the swell cases
   among them, and `a_swell_package_at_another_commit_fails_naming_it` fails the check
   with a source whose commit differs from the listed one in its first hex digit;
   `git grep -n -E '0\.(8|9|10)\.[0-9]|0c1827|b9d3d46|d49a09b' -- crates/surf/tests/lock_check.rs
   crates/surf/tests/adversary.rs` prints two lines per file, the value line of `SWELL` and the
   line of the version constant (decision 2; on `main` it prints 6 lines of `lock_check.rs` and 28
   of `adversary.rs`); `?tag=0.7.0#` at the case "another tag" is the one other version literal
   left, and it names no listed version.
3. `slot cargo nextest run -p surf-script engine::` (package `surf-script`, `crates/script`; one
   test binary, `crates/script/tests/main.rs`) exits 0 within the bounds of acceptance 11, with
   these cases in module `engine`, each with limits of the test's own and under 5 s. A script
   below that names `globalThis` or another builtin that no test of surf has used yet (no test
   of surf uses `globalThis` on `main`; swell holds it with `HOST-155`, a test at `0.10.0`,
   `conformance/host/requirements.txt:155 @ 0.10.0`) is first run on its own; if a builtin is
   missing, the case is written with a `var` at the top level instead, and the report says so.
   The kind of an end is compared as `crates/script/tests/engine.rs:101-107` compares it for
   `throw 1`: `let ScriptEnd::Threw(thrown) = &end else { panic!(...) };` and then
   `assert_eq!(thrown.kind, ScriptErrorKind::<kind>)`.
   - `clock_advances` runs and passes; `git grep -n -F '#[ignore' -- crates/script/tests` prints two
     lines, `HOST-26` and `HOST-127`;
   - `aggregate_error`: `throw new AggregateError([], "m")` ends `threw AggregateError "m" 1:1`;
   - `every_error_constructor_keeps_its_word`: the script
     `throw new Error(Object.getOwnPropertyNames(globalThis)
     .filter(function (n) { return /Error$/.test(n); }).join(","))`
     names the constructors; for each name, `throw new <name>([])` ends `ScriptEnd::Threw(error)`
     with `error.kind.word()` equal to the name; the list holds `AggregateError` and the seven
     others; a name whose kind reads another word fails the test naming it;
   - `checkpoint_runs_after_the_script`:
     `var x = 0; Promise.resolve().then(function () { x = 1; });` ends `ran`,
     then `if (x !== 1) throw new Error("x")` ends `ran`;
   - `async_function_runs`: `(async function () { await null; globalThis.y = 2; })();` ends `ran`,
     then `if (y !== 2) throw new Error("y")` ends `ran`;
   - `checkpoint_after_threw`: `Promise.resolve().then(function () { globalThis.z = 3; }); throw 1`
     ends `ScriptEnd::Threw` of kind `ScriptErrorKind::Value` (the comparison above), then
     `if (z !== 3) throw new Error("z")` ends `ran`;
   - `job_throw_keeps_the_end`: `Promise.resolve().then(function () { throw new Error("j"); });`
     ends `ran`, and `Promise.reject(new Error("r"));` ends `ran` (decision 7);
   - `job_flood_ends_at_the_step_limit`: `function f() { Promise.resolve().then(f); } f();` ends
     `limit: steps-per-script`; a following `1` ends `ran`. What swell promises of the steps:
     a checkpoint spends at most `steps_per_entry`, and `last_entry_steps()` reads the steps of
     the whole checkpoint also after a termination (`crates/swell/src/host/agent.rs:738-749 @
     0.10.0`, HOST-197, whose example reads exactly `steps_per_entry` after `StepBudget`). The
     test asserts that `runner.steps()` after the script is at most the script's own steps plus
     `steps_per_entry`. The lower bound is not known before a first run: the implementor reads
     the script's steps and the checkpoint's steps off the first run (a print in the test, or a
     probe in the session scratch directory, never committed), and asserts the total equals
     their sum; when the checkpoint's steps are under `steps_per_entry`, that sum is the lower
     bound, and the report names both numbers;
   - `limit_clears_the_queue`: three scripts. The first,
     `Promise.resolve().then(function () { globalThis.w = 1; }); while (true) {}`, ends
     `limit: steps-per-script`; the second, `1`, ends `ran` (its checkpoint would run a job left in
     the queue); the third, `if (typeof w !== "undefined") throw new Error("w")`, ends `ran`;
   - `checkpoint_wall_clock`: under a wall-clock bound of 100 ms and a step limit that does not end
     it first, `Promise.resolve().then(function () { while (true) {} });` ends `limit: wall-clock`,
     the elapsed time under 10 s, and a later script ends `not run: wall-clock`;
   - `checkpoint_memory`: under a memory limit of 1 MiB,
     `Promise.resolve().then(function () { var a = []; for (;;) a.push(new Array(1000)); });` ends
     `limit: memory`, `collections()` reads 1, and `1` then ends `ran`.
4. `slot cargo nextest run -p surf-script adversary:: adversary_6:: bindings::` exits 0 within the
   bounds of acceptance 11. In module `adversary` (`crates/script/tests/adversary.rs`, the only
   file of this story with a case of acceptance 4 besides `bindings.rs` and `adversary_6.rs`):
   `every_error_kind_keeps_its_word` holds eight kinds, `AggregateError` thrown as
   `new AggregateError([], "m")` ending `threw AggregateError "m" 1:1`. Module `adversary_6`: the
   test `interrupt_set_before_a_throw_ends_at_wall_clock_and_the_next_record_is_whole` (decision
   11) passes with the ends and the flag it names (the script ends `limit: wall-clock`;
   `is_set()` reads false after it; the next script ends with the record `Ok(Some("N: m"))`),
   and its doc comment names `HOST-216`; `git grep -n -F
   'a_record_read_that_an_interrupt_ends_gives_no_text' -- crates/script/tests` prints nothing
   and the same with the new name prints one line. In `crates/script/tests/bindings.rs`, two
   cases beside its test `limit_end`
   (`:775`), each in its test document whose `createElement` answers `DomEnd::Limit(Nodes)`:
   - `dom_limit_drops_the_jobs`: a script queues a job that sets a global
     (`Promise.resolve().then(function () { globalThis.q = 1; })`), then calls `createElement`; it
     ends `limit: nodes`; `1` ends `ran`; a third script,
     `if (typeof q !== "undefined") throw new Error("q")`, ends `ran`;
   - the `then` variant of `limit_end`: the call to `createElement` is made inside a `then`
     callback; the script ends `limit: nodes`, and `1` after it ends `ran`.
5. `git grep -n -F '_ => ScriptErrorKind::Error' -- crates/script/src/runner.rs` prints one line,
   inside `error_kind`, under a comment that names what it swallows, what the test of acceptance 3
   cannot notice, and that test, so `git grep -n -F 'every_error_constructor_keeps_its_word' --
   crates/script/src/runner.rs` prints one line (the comment names the test); the other wildcard
   of that file, `_ => {}` over surf's own
   `ScriptEnd` (`runner.rs:654` on `main`), is not over a swell enum. `git grep -n -F
   '| HostError::MemoryBudget' -- crates/script/src/runner.rs` prints one line, inside the macro
   `surf_defect!`; the seven patterns `HostError::Faulted | surf_defect!()` build without another
   arm (acceptance 7).
6. The probe's output is a file of 18 lines (per mode one `exit 0` line and five `stdout:` lines;
   `sort -u` of it prints 18 lines, so each is distinct), sha256
   `78fa5309cf44a42b2de1d6b4ad0abaed7e2d120ee974e5ce67c605ad40fc16da` (`sha256sum`). The file is
   not in the repository: the implementor's brief gives its path, and this check names it by its
   hash. `docs/design/scripts.md` holds the 18 lines verbatim, each whole on its own line, in
   one fenced block in Q13: with `<file>` the handed file, `/usr/bin/grep -c -x -F -f <file>
   docs/design/scripts.md` prints 18 (every line of the file is a whole line of the note, and
   no line occurs twice), and as a second check `git grep -c -F 'one more doubling, to 2^24:
   terminated StepBudget' -- docs/design/scripts.md` counts 3 (0 on `main`). Q18's row 11 reads
   `held` with HOST-215: `git grep -c -F 'HOST-215' -- docs/design/scripts.md` counts at least 2
   (the cell of `memory_bytes` and row 11; 0 on `main`). `git grep -n -F 'status at `0.10.0`'
   -- docs/design/scripts.md` prints three lines, the two sentences and the table's head that
   read `0.8.0` at `:1253`, `:1258` and `:1270` on `main`, and `git grep -c -F 'status at
   `0.8.0`' -- docs/design/scripts.md` prints nothing (3 on `main`); `git grep -n -F 'swell is
   read at its tag `0.10.0`' -- docs/design/scripts.md` prints the line of the note's head
   (nothing on `main`). Q2's row 1: `git grep -n -F 'left out: no promise and no job exist' --
   docs/design/scripts.md` prints nothing (one line on `main`, `:123`), and `git grep -c -F
   'taken in effect: the runner performs a checkpoint after each script' --
   docs/design/scripts.md` counts 1 (0 on `main`).
7. `slot task check` exits 0; `git diff --stat main...HEAD`, run after the last merge of `main`
   into the unit's branch, names only the files of the Change and the public surfaces of
   acceptance 8.
8. Public surfaces: in its last commit, after merging `main`, the story changes every public surface
   its change makes false, each number from the `slot task check` log of its tip. After that
   commit `git grep -n -F -e '<old>' -- README.md website/docs docs/badges AGENTS.md` prints nothing
   for each old string, read on the story's base with `git grep -n` before the first commit and
   listed in the report: the swell tag strings `release tag (`0.8.0`)` (`README.md:254`,
   `website/docs/principles.md:27` on `main`), `swell 0.8.0` (`AGENTS.md:366`, `:1901`), the test
   counts of the repository and of `surf-script` (`README.md:11`, `:285-286`,
   `docs/badges/make.sh:38`, `docs/badges/tests.svg`, `website/docs/status.md:31`,
   `website/docs/what-works.md:17-18`, `README.md:626`, `website/docs/how-it-is-built.md:70`,
   `AGENTS.md:367-369` on `main`), and the gate blocks (`README.md:596-645`,
   `website/docs/how-it-is-built.md:40-89` on `main`) hold that log's lines. An old string that
   is also a line of the new log (a count or a line that did not change) is left out of the
   "prints nothing" list, and the report names each such string. The paragraph that names the
   gate run keeps a sentence the other story added (Change). The sentence "3 of
   `surf-script` are ignored until swell reaches what they call" becomes 2 ignored tests with
   their ids, and the entry of `surf-script` in `AGENTS.md` no longer says "no DOM yet". The
   lock's counts do not change (acceptance 1).
9. Decisions 8, 9 and 13 in the text: `git grep -c -F '@ 0.8.0' -- crates/script/src/runner.rs
   crates/script/tests/engine.rs` prints nothing (2 and 10 lines on `main`); `git grep -n -F
   'requirements.txt:85-87 @ 0.10.0' -- crates/script/src/runner.rs` prints the comment of
   decision 8; `git grep -n -F 'eight kinds' -- docs/design/scripts.md` prints a line of Q7
   (nothing on `main`); `git grep -n -F '220 lines, 152 `test`, 68 `pending`' --
   docs/design/scripts.md` prints one line of Q18 (nothing on `main`); `git grep -c -F
   'requirements.txt:216-220 @ 0.10.0' -- docs/design/scripts.md` counts 1; `git grep -o -F
   'Deviation from HTML' -- docs/design/scripts.md | wc -l` prints 3, counting occurrences (0 on
   `main`); and `git grep -n -F '{name: AggregateError, summary: "AggregateError"}' --
   docs/design/scripts.md` prints one line, in the note's copy of the enum (nothing on `main`).
   The other edits of decision 13, each false on `main` (`git grep -c -F '<text>' 77e8f3f --
   docs/design/scripts.md` prints nothing for each positive text and 1 to 3 for each negative
   one): `git grep -c -F 'HOST-216' -- docs/design/scripts.md` counts at least 2 (Q4 and Q18),
   the same for `HOST-217` and `HOST-218`, and at least 1 for `HOST-219`, `HOST-220`, `HOST-209`
   and `HOST-214` (Q18 and Q6); `git grep -c -F '`Agent::clear_microtasks()` first, then
   `collect()` once' -- docs/design/scripts.md` counts 1 (the row after
   `Terminated(MemoryBudget)`); `git grep -c -F 'at most two entries, the script and its
   checkpoint' -- docs/design/scripts.md` counts 1 (the row of `steps_per_entry`);
   `git grep -c -F 'b9d3d46fa7ac4abc982f2248e903e6301e277a9f' -- docs/design/scripts.md` counts at
   least 1 and `git grep -c -F "rev-parse '0.10.0^{commit}'" -- docs/design/scripts.md` counts 1
   (Q13); and these print nothing, with their counts on `main` in parentheses: `'It does not
   bound the copies swell lists as open'` (2), `'while requirement 11 is open'` (1), `'and
   requirement 11 are closed'` (1) and `'asks swell to charge every copy'` (3).
10. The report lists what no test of this story holds at `0.10.0`: the reading of a checkpoint's
    `Throw` and of its `Terminated(EngineFault)` as `faulted`. The script count is held by
    `engine::totals`; the clear after `limit: memory`, the clear before the collection and the
    read of a thrown value that an interrupt ends are held by acceptance 12. It also lists that
    the test
    `a_record_read_that_an_interrupt_ends_gives_no_text` was read, not run, before the unit
    replaced it with `interrupt_set_before_a_throw_ends_at_wall_clock_and_the_next_record_is_whole`
    (decision 11), and the comments that decision 12 leaves stale (the four of the generated code
    and `crates/script/tests/adversary_5.rs:226`).
11. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s
    (`.config/nextest.toml`); no model call and no network request in a test (cargo fetches swell
    before the tests); every build and test runs through `slot`, and a job in the pool ends within
    10 minutes or names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes (the
    first cold build of swell `0.10.0` through the cargo shim may name it), and a job that timed
    out is not started again unchanged; a script that measures or waits starts without the `slot`
    prefix; no Python in anything that stays in the repository; no generator with a committed
    output is added; a test over 5 s is named with its time.

12. The review's rules (decision 14), in the module `adversary_checkpoint` of
    `crates/script/tests`.
    a. A script whose last operation sets the interrupt and throws ends `limit: wall-clock`; no
       job it queued runs; the queue is empty for what follows. The same for a thrown value
       that is no Error, for a normal end with one queued job, and for a script whose entry
       recorded a limit of the bindings and threw (that limit is the end).
    b. After a script's own `limit: memory` no job it queued runs, and memory held only by a
       queued job's closure is free after the collection.
    c. A checkpoint raises no script count; an interrupt inside a job ends the checkpoint and
       the next script's checkpoint runs.
    d. Endless jobs (an `async` loop, a thenable that queues itself, an endless `for await`)
       end `limit: steps-per-script` within two times `steps_per_entry` and leave no job.
    e. A thrown value with accessors on `message`, `errors`, `stack` or `constructor` ends
       with a record and runs no accessor; an `AggregateError` of 100,000 entries records its
       message.
    f. `new (async function () {}).constructor("…")` and the async generator form end as Q4's
       row says for `Function` in a session.

13. The second review's rules (decision 15), in the modules `adversary_checkpoint_2` and
    `run_end` of `crates/script/tests`.
    a. A script that ends `limit: steps-per-script` with the interrupt set is followed by a
       script of the same document that runs; the flag is not set when the next script starts.
    b. A script that ends `limit: wall-clock` by the rule of decision 14 leaves no job for a
       later script.
    c. A clock that ran out before its thread set the flag runs no job of the script, and ends
       the script after its last job.
    d. A host function as the last job of a checkpoint leaves the flag set when the checkpoint
       returns; the runner ends the script at the wall clock.
    e. With the one withdrawal removed, the case of a fails; with the clear after
       `limit: wall-clock` removed, the case of b fails.

## Out of scope

The specification's word for `AggregateError` (`story:script-error-kind-aggregate`, merged);
recording a job's throw or an unhandled rejection (decision 7; `story:scripts-job-errors`, draft);
the writer of a script's end and the session (`story:scripts-document-port`); the bound values of
Q11; host microtasks queued by surf (`queue_microtask`), timers, events and generic jobs, which
`story:scripts-event-loop-and-timers` takes (its `:28` lists `queueMicrotask`; its unit takes
`HostError::MemoryBudget` out of `surf_defect!()` and handles it at the call, decision 5);
`Math.random` and the time zone (`pending slice 4` at `0.10.0`; `story:scripts-seeds-and-time-zone`,
draft, waits for a swell tag that holds the slice); any change in swell; the use of
`to_utf16_len` (`HOST-220`) in the generated code and the stale comments of Context (decision
12; `story:scripts-utf16-length`, draft). The other additions of `0.9.0` that no Rust file of
surf names on `main` (`git grep -n -E
'AsyncIterator|PromiseCapability|new_promise_capability|PromiseState|promise_state'
-- 'crates/*.rs'` and the same with `promise_is_handled|mark_promise_as_handled` prints
nothing): `WellKnownSymbol::AsyncIterator`, `PromiseCapability` and `new_promise_capability`,
`PromiseState` and the promise reads `promise_state` and `promise_is_handled`, and
`mark_promise_as_handled`; the epic cites `new_promise_capability` for a later story
(`.engineering/planning/epic/scripts-in-a-page.md:303`). `HOST-219` (a BigInt listed in `trace`,
`pending slice 12`) concerns a type no surf host data holds.

## Placement

After `story:webidl-bindings` and `story:script-error-kind-aggregate`, both on `main`; this story
has the edge `depends_on: story:script-error-kind-aggregate` because decisions 4 and 13 copy that
story's enum line. `story:html-script-hook` (on `main`) is an ordering fact with no edge. Before
`story:scripts-parse-run`, which depends on this story (it writes `crates/script/src/runner.rs`
and `docs/design/scripts.md`, and measures Q4 on the tag this story pins), and before
`story:scripts-document-port` and `story:scripts-event-loop-and-timers`, which depend on it too
and take no tag of their own. `story:scripts-event-loop-and-timers` re-reads the queue clearing
of decision 6 when timers and events exist. Beside `story:conformance-lock-and-lints` (draft,
`epic:headless-rust-target`), which writes `AGENTS.md`, `website/docs/status.md` and
`crates/surf/tests`: whichever merges second merges `main` first and runs `slot task check` on the
merged tree.

Beside `story:scripts-dom-reading` (active, `depends_on: story:webidl-bindings` and
`story:html-script-hook`): no edge. The two are built in separate trees and merged one after the
other. They write the same five files, `AGENTS.md`, `Cargo.lock`, `crates/script/Cargo.toml`,
`crates/script/tests/bindings.rs` and `docs/design/scripts.md`, and both rewrite the public
surfaces (`README.md`, `docs/badges/make.sh`, `docs/badges/tests.svg`,
`website/docs/how-it-is-built.md`, `website/docs/status.md`, `website/docs/what-works.md`).
Whichever merges second merges `main` first, runs `slot task check` on the merged tree and takes
its numbers from its own log. The reading story adds the dependency `surf-url` right after
`surf-secret` in `crates/script/Cargo.toml` and in the entry of `surf-script` in `Cargo.lock`;
this story changes the pin on the next line of `crates/script/Cargo.toml` (line 16) and the swell
entries of `Cargo.lock`, so the second merge may conflict in those two files and keeps both
changes. The reading story regenerates `crates/script/src/bindings.rs` and rewrites
`crates/webidl/src/prelude.rs`; this story edits neither (decision 12).

Five overlaps and how the second merge settles each. `crates/script/src/runner.rs`: this story
edits its macro, `evaluate`, `document_end`, `error`, `error_kind` and the comments at `:51` and
`:201-203`; the reading unit edits the lines that make `ScriptHost` (`ScriptHost::new` gains
the agent's `string_length`, and the one call of it in `Runner::new`) and no line of these, so
the second merge keeps both. `AGENTS.md:365-369` is one paragraph that
both rewrite: the second merge keeps `0.10.0` and `surf-url`, has no "no DOM yet", and takes the
test counts from its own log after merging `main`. `crates/script/tests/adversary_3.rs` is a file
this story edits (a comment and a version in a comment, decisions 9 and 11); the reading story's
acceptance 12d says its unit changes none of `adversary_3.rs`, `adversary_4.rs` and
`adversary_5.rs`, which is read against that unit's own base, so the two do not conflict. The
paragraph that names the gate run, in `README.md` and `website/docs/how-it-is-built.md`: the
reading story adds a sentence to it; this story rewrites the paragraph's commit and numbers from
its own gate log by the rule of the gate blocks and keeps a sentence the other story added.
`docs/design/scripts.md`: the reading story edits line 1102 only (`surf-script` depends on
`surf-html` for the tree, which gains `surf-url`) and reads lines 307 and 320 and the rows of
Q18; this story edits the Q2, Q4, Q6, Q7, Q11, Q13 and Q18 passages of decision 13, none of them
line 1102, and of line 307 only its last sentence, so the edits do not overlap. Line cites of
the note in other planning texts move with this story's edits; the reading story's cites are
read at its base.

The texts of the plan that named `0.9.0` as the tag this story takes were corrected with this
revision: ten edits, five in the epic, two in `story:scripts-parse-run`, two in
`story:scripts-document-port`, and one sentence in `story:scripts-event-loop-and-timers`
(decisions 5 and 6), each now naming `0.10.0`. None is listed here. A text that states what
`0.9.0` holds or cites `0.8.0` or `0.9.0` as the tag of a statement true at that tag stays
(`story:script-error-kind-aggregate` `:36-38`, `:87`, `:184` and `:193`, the epic's `:121`,
`story:scripts-dom-reading` `:77`, `story:scripts-dom-mutation` `:86` and `:110-115`, and the epic's
"Corrections of 2026-11-24").

## Why `epic:scripts-in-a-page`

The epic runs page scripts on a swell tag that holds the security review's fixes; `0.9.0`
charged the copies the review's condition left open (requirement 11), `0.9.1` makes an interrupt
end a run also when the function that was running throws, which the wall clock and the parser
limits rely on, and a page's promises need the checkpoint this story runs. The epic's rule is
that surf runs page scripts on the tag that holds requirement 11
(`.engineering/planning/epic/scripts-in-a-page.md:606-612`, Q18 of the note), and `0.10.0` is
the newest tag that does, so surf pins it.
