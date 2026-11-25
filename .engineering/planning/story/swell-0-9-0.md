---
format: aep.planning-md/3
id: story:swell-0-9-0
kind: story
status: draft
title: surf takes swell 0.9.0, runs the microtask checkpoint after each script, and maps swell's new error kind and host error
summary: The tag moves to 0.9.0 in one place with the lock, the allow-list, the test constants, NOTICE and the tag sentences; the runner performs HTML's microtask checkpoint after each script under the document's totals and clock and clears the queue after a limit; AggregateError gets its word and a wildcard arm whose swallowing a test notices; HostError::MemoryBudget joins surf_defect!(); clock_advances runs; the design note is revised at 0.9.0, with requirement 11 held.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-engine-dependency
- depends_on: story:webidl-bindings
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
  path: crates/script/tests/bindings.rs
- confidence: cited
  path: crates/script/tests/engine.rs
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
revision: 11
---
## Context

surf depends on swell by a git tag, `0.8.0`, written once in `crates/script/Cargo.toml:15`. swell
released `0.9.0` on 2026-11-24, commit `d49a09bcc44fbdd155f705f913a90b676518afbc`
(`git -C <swell clone> rev-parse '0.9.0^{commit}'`). Every swell citation below is read with
`git show 0.9.0:<path>` or `git show 0.8.0:<path>` in a clone of
`https://github.com/beyond10x/swell` and written `<path>:<line> @ <tag>`; every surf citation is
`main` at `9934c89`.

What a change of tag is followed by, from `story:scripts-engine-dependency`, "As built"
(`.engineering/planning/story/scripts-engine-dependency.md:329-331`, branch `plan/round-2`):
`Cargo.lock`, six lines of `taskfiles/lock-allow.txt`, the constant `SWELL` in
`crates/surf/tests/lock_check.rs`, `NOTICE` and the citations `@ 0.8.0`. Read on `main`, the tag
or its commit is also written outside those places:

- `crates/surf/tests/lock_check.rs`: the constant `SWELL` (`:21-22`) and the literals `swell 0.8.0`
  (`:378`, `:381`, `:428`), `swell-num 0.8.0` (`:537`), `?tag=0.8.0#` (`:391`) and the commit prefix
  `#0c18` (`:398`). At the commit of `0.9.0` the replacement of `:398` finds nothing to replace,
  so the case `a_swell_package_at_another_commit_fails_naming_it` would hand the check the listed
  source unchanged.
- `crates/surf/tests/adversary.rs`: a second constant `SWELL` (`:12`), the commit as a literal
  (`:216`) and `0.8.0` in sources, messages and lock fragments (`:127-135`, `:207`, `:221-228`,
  `:237`, `:257`, `:267-268`, `:301-308`, `:377`, `:393-398`).
- `NOTICE:54`, `AGENTS.md:355` and `:1873`, `README.md:254`, `website/docs/principles.md:27`.

`Cargo.lock` holds 58 packages, 25 from outside this repository (`grep -c '^\[\[package\]\]'
Cargo.lock` prints 58, `grep -c '^source = ' Cargo.lock` prints 25). swell `0.9.0` has the same
eight packages in its root lock, and `swell` takes the same five by path
(`crates/swell/Cargo.toml:13-17 @ 0.9.0`); its workspace version is `0.9.0` (`Cargo.toml:8 @
0.9.0`), and its root manifest sets `overflow-checks = true` in the same four profiles
(`Cargo.toml:10-20 @ 0.9.0`). Both repositories pin rustc 1.99.1 (`rust-toolchain.toml`, and the
same file `@ 0.9.0`). So the counts stay 58 and 25 and only the six packages' version and source
change.

What `0.9.0` changes for a host, from `CHANGELOG.md:154-214 @ 0.9.0` ("### Host interface"):

- "Breaking: `ErrorKind` is `#[non_exhaustive]`. A `match` over it in a host needs a wildcard
  arm" (`:178-180`), and "New: `ErrorKind::AggregateError`. [...] `ErrorKind::ALL` keeps its seven
  entries" (`:181-183`). `ErrorKind::name()` answers `"AggregateError"` for it
  (`crates/swell/src/host/mod.rs:69-81 @ 0.9.0`).
- "Breaking: `HostError::MemoryBudget` is a new variant, and `HostError` is not
  `#[non_exhaustive]` [...] `queue_microtask` answers it when the memory budget refuses the growth
  of the microtask queue, with nothing queued" (`:209-212`). The only place the engine answers it
  is `enqueue_microtask` (`crates/swell/src/host/jobs.rs:207 @ 0.9.0`; `git grep -n
  'HostError::MemoryBudget' 0.9.0 -- crates/swell/src` lists no other answer outside tests and
  doc comments), whose only callers are `Agent::queue_microtask` and `Cx::queue_microtask`
  (`crates/swell/src/host/agent.rs:713-716`, `crates/swell/src/host/mod.rs:1034-1046 @ 0.9.0`), and
  the table of misuse names `queue_microtask` alone (`docs/design/host-interface.md:651 @ 0.9.0`).
  surf calls no `queue_microtask`.
- New: `Agent::perform_microtask_checkpoint`, `Agent::queue_microtask`, `pending_microtasks`,
  `clear_microtasks`, `CheckpointReport` (`:156-160`); `PromiseState`, `Agent::promise_state`,
  `promise_is_handled`, `mark_promise_as_handled` (`:171-176`); slice 10 delivered, so promises,
  `async` functions, async generators, `for await` and `Array.fromAsync` run (`:30-35`, `:40-119`);
  the engine calls `host_promise_rejection_tracker` (`:96-101`).
- `Date` over the host's clock: the constructor, `Date.now()`, `getTime`, `setTime`, `valueOf`;
  `Date.UTC`, `Date.parse` and the other 14 setters throw a `TypeError`; the getters of the fields
  do not exist; `Math` but `max` does not exist (`:23-29`, `:224-231`).
- "A buffer whose size a script controls and that the engine holds while it calls into script
  [...] is charged to `memory_bytes` before it is made and credited when it is dropped, or is not
  made" (`:145-150`), held by HOST-215 (`conformance/host/requirements.txt:215 @ 0.9.0`, `test
  swell::budgets::host_215_a_buffer_held_across_a_call_into_script_is_charged`). swell's list of
  open findings drops the two copies it listed at `0.8.0` (`docs/plan/second-batch.md:284-285 @
  0.8.0`) and says "Closed after the batch [...] by
  `story:copies-held-across-a-call-into-script-are-charged` (HOST-215)"
  (`docs/plan/second-batch.md:302-304 @ 0.9.0`). This is requirement 11 of the note
  (`docs/design/scripts.md:1271`).
- `HOST-147` no longer names a `Map`; the key order of a `Map` is `HOST-214`, `pending slice 9`
  (`:199-201`; `conformance/host/requirements.txt:214 @ 0.9.0`).

How the checkpoint works at `0.9.0` (`crates/swell/src/host/agent.rs:738-773 @ 0.9.0`;
`docs/design/host-interface.md:1174-1223 @ 0.9.0`): one outermost entry with its own step budget
of `steps_per_entry`, one step per entry taken off the queue; it answers `Normal(CheckpointReport)`
only once the queue is empty; a termination (`StepBudget`, `MemoryBudget`, `Interrupted`) ends the
whole checkpoint and leaves every entry not started queued; the interrupt is read at its start,
before each entry and inside a job, and cleared; `last_entry_steps()` then reads the steps of the
whole checkpoint (HOST-193, HOST-197; `agent.rs:738-751 @ 0.9.0`), and the checkpoint clears the
error sites at its start (`agent.rs:754 @ 0.9.0`), so after it `last_entry_steps()` and
`last_entry_error_site()` read the checkpoint, no longer the script; it never answers `Throw` (a
throw there faults the agent, `agent.rs:765-769 @ 0.9.0`). A promise job that ends in a throw "ran
to its end" (`crates/swell/src/host/jobs.rs:359`, `:408-411 @ 0.9.0`): the checkpoint goes on, and
`host_clean_up_after_job` receives `Err(value)` (HOST-82). `clear_microtasks` drops every queued
entry unrun, credits its charge, runs nothing and spends no step (`agent.rs:726-736 @ 0.9.0`,
HOST-200). swell's note leaves the clear to the host: "The engine drops none of them: HTML stops a
script without clearing the microtask queue, and only the host knows whether the page goes on"
(`docs/design/host-interface.md:1206-1211 @ 0.9.0`). Generic, timeout and cleanup jobs are pending:
HOST-85 and HOST-86 `pending slice 14`, HOST-87 `pending slice 16`
(`conformance/host/requirements.txt:85-87 @ 0.9.0`), and `git grep -n
'host_enqueue_generic_job\|host_enqueue_timeout_job\|host_enqueue_finalization_registry_cleanup_job'
0.9.0 -- crates/swell/src` lists only their declarations and comments, no call. The module comment
of `hooks.rs` still says no engine path makes a `Job` "before slices 10 and 11"
(`crates/swell/src/host/hooks.rs:7-9 @ 0.9.0`); `hooks.rs` is the same file at both tags (`git diff
--quiet 0.8.0 0.9.0 -- crates/swell/src/host/hooks.rs` exits 0), so that sentence was not
revised when slice 10 was delivered, and this story does not rest on it. The rejection tracker's
default does nothing (`hooks.rs:255-262 @ 0.9.0`).

The HTML Standard (multipage `webappapis.html`, "Last Updated 24 November 2026", read on
2026-11-24): "run a classic script" (`#run-a-classic-script`) takes a parse error as the script's
error to rethrow (step 6), reports an abrupt completion (step 8.3) and calls "clean up after
running script" on both paths (steps 8 and 9); "clean up after running script"
(`#clean-up-after-running-script`) step 3: "If the JavaScript execution context stack is now empty,
perform a microtask checkpoint." "Perform a microtask checkpoint" (`#perform-a-microtask-checkpoint`)
runs the queue until it is empty, then "notify about rejected promises" (step 4).
HostEnqueuePromiseJob (`#hostenqueuepromisejob`) step 2.4: "If result is an abrupt completion,
then report an exception". HostPromiseRejectionTracker
(`#the-hostpromiserejectiontracker-implementation`) appends a rejected promise to the global's
about-to-be-notified list, and "notify about rejected promises" fires `unhandledrejection`, after
which "the user agent may report p.[[PromiseResult]] to a developer console".

What the plan already says about the checkpoint: the epic, "The gaps" (`epic/scripts-in-a-page.md:299-311`,
branch `plan/round-2`): surf "drains the queue with `agent.perform_microtask_checkpoint()`, which
surf calls after each script", and calls `Agent::clear_microtasks()` "when a limit ended a
checkpoint, so that a page that floods the queue cannot spend the next scripts' budgets";
`story:scripts-document-port`, in its section "Waits for" before this story took the work over
(its revision 32, lines 158-161): "the microtask checkpoint after each script, its steps counted
in that script's and the document's totals, and its end taken as the script's end. Open until
then: which call of the host interface runs the jobs, and what a job that ends at a limit makes
of the record of a script that already ended `ran`." This story answers both; that section no
longer holds them.

Where surf meets the changed types, on `main`:

- `crates/script/src/runner.rs:23-32`, the macro `surf_defect!()`, names six `HostError` variants;
  seven patterns `HostError::Faulted | surf_defect!()` (`:515`, `:562`, `:585`, `:592`, `:625`,
  `:643`, `:656`) are exhaustive at `0.8.0` and are not at `0.9.0`. The macro's doc comment
  (`runner.rs:16-22`) reads "swell's `HostError` at `0.8.0` but `Faulted`" and "Each is a defect of
  surf's own calls that a page cannot cause". `story:webidl-bindings` (the branch
  `impl/webidl-bindings`, not on `main`; it moves) keeps seven such patterns in `runner.rs`; its
  `bindings.rs` reads every `HostError` as a defect without a `match` (`fn defect`,
  `bindings.rs:461-465` on that branch), and names `WellKnownSymbol` and `HostIntrinsic` variants
  only as arguments, in no `match`.
- On that branch `Runner::evaluate` (`runner.rs:612-647`) runs the script (`run_script`,
  `:680-714`, which reads the error summary and site of a `threw` through `error` and `site`,
  `:716-747`), passes its end through `Runner::document_end` (`:649-677`, decision 10 of `story:webidl-bindings`: an interrupted end
  with a limit the bindings recorded becomes that limit's word; after any other end a recorded
  limit and its interrupt are withdrawn), then adds `last_entry_steps()`, then applies the memory
  rule. A DOM write over a parser limit records the limit and sets the interrupt "so that the entry
  ends `Terminated(Interrupted)` at swell's next read of the flag, also under a `catch`"
  (`bindings.rs:329-333`, `end_with_limit`).
- `runner.rs:572` on `main`, `_ => {}`, is a wildcard over surf's own `ScriptEnd`, not over a
  swell enum.
- `runner.rs:704-716`, `error_kind`, names the seven `ErrorKind` variants with no wildcard; its doc
  comment says a later kind "stops the build here". "As built" of `story:scripts-engine-dependency`
  (`:352-354`): "`surf-script` names every variant of swell's error types, with no catch-all arm".
  `#[non_exhaustive]` ends that for `ErrorKind`: outside swell a `match` over it must have a
  wildcard arm. The note decided the arm (`docs/design/scripts.md:451-454`): "the tag change that
  brings it adds the word `AggregateError` to this list and to `ScriptErrorKind`, and a wildcard
  arm maps a kind surf does not know yet to `Error`".
- `runner.rs:188-197`: the comment "No engine path makes a job or a load request before swell's
  slices 10 and 11 [...]: promises and modules are refused at the tag" is false at `0.9.0` for
  promises.
- `crates/script/tests/engine.rs:339-378`: three tests ignored, `clock_advances` (`HOST-125`),
  `seeds_fixed` (`HOST-26`), `time_zone` (`HOST-127`). At `0.9.0` `HOST-125` and `HOST-126` have a
  test (`conformance/host/requirements.txt:125-126 @ 0.9.0`); `HOST-26` and `HOST-127` are
  `pending slice 4` (`:26`, `:127 @ 0.9.0`). `Date.now()` "ask[s] the host's clock hook once
  [...] floor[s] the answer to milliseconds" (`CHANGELOG.md:24-25 @ 0.9.0`), and the test clock
  starts at a whole millisecond and advances 1,000,000 ns a call (`runner.rs:55-61`), so
  `clock_advances` reads `b - a` as 1. `Math.random` and `getTimezoneOffset` do not exist at
  `0.9.0` (`CHANGELOG.md:228-231 @ 0.9.0`).
- `crates/script/tests/adversary.rs:94-120`, `every_error_kind_keeps_its_word`, lists the seven
  kinds.

The swell citations in surf's code that move with the tag (`git diff --numstat 0.8.0 0.9.0 --
<path>`; a file the diff does not list keeps its lines): `crates/syntax/src/parser/statement.rs`
changed, and the refusal of `class` moved from `:218` to `:221 @ 0.9.0`
(`Keyword::Class => return self.refuse("class", 8)`); `crates/syntax/src/lib.rs` changed, the
sentence cited at `:16` is still at `:16 @ 0.9.0`; `crates/syntax/src/error.rs`,
`crates/swell/src/script.rs`, `crates/swell/src/interp/budget.rs`, `crates/swell/src/host/limits.rs`,
`crates/swell/src/host/hooks.rs`, `crates/swell/tests/support/stack.rs`,
`crates/swell/tests/limits_scripts.rs` and `crates/swell/tests/eval_stack.rs` are the same at both
tags.

Requirement 11 (`docs/design/scripts.md:1271`, `absent` at `0.8.0`): the note left its detail out
until a tag closed it (Q13, `:1074-1077`; the coordinating agent's decision of 2026-11-24, epic
"Decisions taken"). The detail at `0.8.0`, each line readable at that tag: the separator of
`Array.prototype.join` copied by `string_units` (`crates/swell/src/builtins/array/mod.rs:323-335
@ 0.8.0`; `crates/swell/src/object/string.rs:46-53 @ 0.8.0`, which charges nothing) and held across
`Get` and `ToString` of each element; the String an object operand's `toString` returns, held by
`sort` without a comparator when that operand comes first
(`crates/swell/src/builtins/array/mutation.rs:758-759`, `:789-801 @ 0.8.0`); and the argument lists
of the `call` and `new` instructions (`crates/swell/src/interp/dispatch.rs:1390-1392`,
`:1469-1471 @ 0.8.0`). The probe of the note's Q13 (the program `swell-limits` with its modes
`join`, `sort-xy` and `sort-yx`, run by `m3-copies.sh` under
`prlimit --as=2147483648:2147483648 --core=0:0` with Q4's limits) is not in the repository.
Between the commit swell merged the charged buffers at (`f27c52e`, an ancestor of `0.9.0`: `git
merge-base --is-ancestor f27c52e 0.9.0` exits 0) and the tag, no file under `crates/` changed
(`git diff --stat f27c52e 0.9.0` lists none). The probe was built against the tree of the tag
(`git archive 0.9.0`) and run on 2026-11-24, 19:45 CET, after the tag was published: each of the
three modes ended with exit status 0; the script that asks for the copy ends `throw RangeError
"reentry_depth exceeded"` (3,607, 5,154 and 4,130 steps) and the doubling after it `terminated
StepBudget`; `heap_bytes()` reads 13,052,701, 13,054,051 and 13,054,051; the highest `VmHWM` is
37,512, 37,716 and 37,740 kB, under an address-space bound of 2 GiB that ended the same three
modes with status 134 at `0.8.0` (the note's Q13). The coordinating agent hands the run's output
lines to the unit as a file; the unit does not build the probe again.

## Decisions taken

1. The tag is `0.9.0`, commit `d49a09bcc44fbdd155f705f913a90b676518afbc`, written once:
   `swell = { git = "https://github.com/beyond10x/swell", tag = "0.9.0" }` in
   `crates/script/Cargo.toml` (the line `CHANGELOG.md:258-260 @ 0.9.0` gives). In the same commit:
   `Cargo.lock` (the six packages at version `0.9.0` with the source
   `git+https://github.com/beyond10x/swell?tag=0.9.0#d49a09bcc44fbdd155f705f913a90b676518afbc`),
   the six lines of `taskfiles/lock-allow.txt` (`<name> 0.9.0 <source> none`, Q15), and the test
   constants of decision 2. No package is added or removed; the counts stay those of the base.
2. The test files name the tag, the version and the commit once each. In
   `crates/surf/tests/lock_check.rs` and in `crates/surf/tests/adversary.rs`, two constants hold
   them: `SWELL`, the source with tag and commit (on `main` its value is on one line,
   `lock_check.rs:22` and `adversary.rs:12`), and one constant beside it that holds the version.
   Every other literal of the swell version, tag or commit in those files (Context, first two
   items) is built from the two; the case "at another commit" (`lock_check.rs:398` on `main`)
   changes the first hex digit of the commit `SWELL` holds, whatever it is, and the commit of
   `adversary.rs:216` is read out of `SWELL`; so the next tag changes the two constants and
   nothing else. `swell 0.8.1` in `adversary.rs:127-143` (a version beside the listed one)
   becomes the listed version with its patch number raised by one, built from the version
   constant. The comment at `adversary.rs:213` ("Held: swell 0.8.0 from every other spelling of
   a source fails, naming it.") is reworded without a version. One literal stays: `?tag=0.7.0#`
   in `lock_check.rs:391`, the "another tag" of `a_swell_package_of_another_tag_fails_naming_it`,
   which names no listed version; the string it replaces is built from the version constant.
3. `NOTICE:54` names the tag `0.9.0`; swell's `NOTICE` is the same file at both tags (`git diff
   --quiet 0.8.0 0.9.0 -- NOTICE` exits 0), so the quoted text stays. `AGENTS.md:355` and `:1873`,
   `README.md:254` and `website/docs/principles.md:27` name `0.9.0`.
4. The error kinds. `ScriptErrorKind` gains `AggregateError`, word `"AggregateError"`, mapped from
   `ErrorKind::AggregateError`. `error_kind` keeps one arm per named kind and gains one wildcard
   arm, `_ => ScriptErrorKind::Error`, as the note decided (`docs/design/scripts.md:451-454`). The
   arm's comment names what it swallows: every kind swell adds after `AggregateError`, none at
   `0.9.0` (`crates/swell/src/host/mod.rs:46-55 @ 0.9.0`); the test below that notices such a
   kind; and what that test cannot notice: a kind that has no global constructor whose name ends
   in `Error` reaches the wildcard unnoticed. `ErrorKind::ALL` cannot serve the test: it keeps
   seven entries without `AggregateError` (`CHANGELOG.md:183 @ 0.9.0`). The test reads the realm
   instead: it lists the own properties of the global object whose names end in `Error`, throws
   `new <name>([])` for each (an `AggregateError` needs an iterable first argument, and `[]` is the
   message `""` for the others), and compares the end's kind by its word,
   `ScriptEnd::Threw(error)` with `error.kind.word()` equal to `<name>`
   (`ScriptErrorKind::word`, `runner.rs:223-236` on `main`), not the end's text: the seven other
   kinds end `threw <name> "" 1:1` (`new <name>([])` sets the message `""`, which `ScriptError`'s
   `Display` prints, `runner.rs:337-349`), `URIError` among them, and `new AggregateError([])` sets
   no message and ends `threw AggregateError 1:1`. A kind that reaches the wildcard reads `Error` and the
   test fails naming it. This is the one wildcard arm over a swell enum in `crates/script`; the
   rule of "As built" holds for every other enum, `HostError` included, which is not
   `#[non_exhaustive]` (`CHANGELOG.md:209-210 @ 0.9.0`).
5. `HostError::MemoryBudget` joins `surf_defect!()` (`runner.rs:23-32` on `main`). One reading
   for every call surf makes: at `0.9.0` only `queue_microtask` answers it
   (`crates/swell/src/host/jobs.rs:207 @ 0.9.0`, `docs/design/host-interface.md:651 @ 0.9.0`),
   and surf calls no `queue_microtask`, so from surf's calls it is a defect of surf's own calls,
   read as `HostError::Faulted` is, like the six variants the macro names. The macro's doc comment
   (`runner.rs:16-22` on `main`, "swell's `HostError` at `0.8.0` but `Faulted`") is rewritten for
   `0.9.0`: it names the seven variants, says why `MemoryBudget` is among them, and says that it
   leaves the macro with the first call of `queue_microtask` in surf, where surf decides its
   reading. No arm of its own at any call; the seven patterns `HostError::Faulted |
   surf_defect!()` are exhaustive again, and the build is the check.
6. The microtask checkpoint, HTML's "clean up after running script", inside `Runner::evaluate`,
   in this order:
   - The script's own end is settled first, as on the branch of `story:webidl-bindings`
     (Context): `run_script` answers the end and reads the error summary and site of a `threw` or
     `refused`; `Runner::document_end` reads the end with the rule of that story's decision 10
     (and withdraws a stale interrupt); `last_entry_steps()` is added to the step total. The
     checkpoint overwrites what `last_entry_steps()` and `last_entry_error_site()` answer
     (`agent.rs:738-751`, `:754 @ 0.9.0`), so these reads come before it.
   - Then, after a script that ended `ran`, `threw` or `refused` (a parse error is run through
     "run a classic script" too, steps 6 and 8.3), the runner calls
     `Agent::perform_microtask_checkpoint()`. After `limit:` or `faulted` it calls none. A script
     `not run` is not run and gets no checkpoint. A limit a DOM write records ends the script
     "also under a `catch`" (`bindings.rs:329-333` on the branch; decision 9 of
     `story:webidl-bindings` as ruled in its fix round 1, held by
     `no_catch_and_no_finally_runs_after_a_limit`, `tests/bindings.rs:675-679` on the branch), so
     a script that reaches a DOM limit ends `limit: <word>`, gets no checkpoint, and its queue is
     cleared (below).
   - Its budget is swell's: one outermost entry with a step budget of its own, `steps_per_entry`
     (HOST-193). After it the runner adds `last_entry_steps()` to its step total (HOST-197), so
     the document's total of Q4 counts the jobs; the wall-clock bound of the runner is the same
     clock, and its interrupt ends a checkpoint as it ends a script (HOST-198). The script count
     does not rise for the checkpoint.
   - Its end passes through the same rule of `document_end`. `Normal(_)` leaves the script's end
     as it was. `Terminated(StepBudget)`, `Terminated(MemoryBudget)` and `Terminated(Interrupted)`
     replace it with `limit: steps-per-script`, `limit: memory` and `limit: wall-clock` (or, for
     an interrupt with a recorded DOM limit, that limit's word, so a job's DOM write over a parser
     limit ends the script `limit: nodes` or its word), and `Terminated(EngineFault)` with
     `faulted`: the checkpoint is a step of running that script ("run a classic script", step 9),
     and Q7 holds one end per script element. `Throw`, which the checkpoint never answers
     (`agent.rs:765-769 @ 0.9.0`), is read as `faulted`, with that citation.
   - After any end `limit: <word>`, of the script or of its checkpoint, the runner calls
     `Agent::clear_microtasks()` before anything else: jobs a script queued before its limit are
     that script's work, and run by the next script's checkpoint they would spend its budget, the
     epic's reason for the clear (`epic/scripts-in-a-page.md:308-310`). This is wider than HTML
     and than swell's note, which keeps the queue after a script stopped ("HTML stops a script
     without clearing the microtask queue", `docs/design/host-interface.md:1206-1211 @ 0.9.0`):
     a deviation, named in the note's Q2 paragraph (decision 11) with this reason, and the epic's
     lines `:308-310` read "when a limit ended a script or its checkpoint". After `limit: memory`
     the runner clears first and then applies Q4's rule (one `collect()`, `not run: memory` after a
     second), so the collection finds the jobs' charge credited (HOST-200).
   - At a `script` end tag the queue is therefore empty: no script code runs between two
     scripts of a document (the first version has no timer and no event), each checkpoint ends with
     the queue empty or is followed by the clear. So step 1 of the end tag (the note's Q2, row 1)
     is taken in effect, with no call of its own.
7. A job's throw and an unhandled rejection are not recorded (the coordinating agent's decision
   of 2026-11-24; the two other forms, a job's throw as the script's end and a count per script
   in the record, need a question to swell or a change of the specification under its synthesis
   bound, and no story holds either): `host_clean_up_after_job` and
   `host_promise_rejection_tracker` keep swell's defaults, a script whose job threw or whose
   promise stays rejected ends as the script itself ended, and the note names the deviation from
   HTML's "report an exception" (`#hostenqueuepromisejob` step 2.4) and "notify about rejected
   promises" in Q2. A script element may so spend two entries' steps, its own and its
   checkpoint's (HOST-193); the document's step total and the wall clock bound the sum, and
   `steps_per_entry` is not halved (the coordinating agent's decision of 2026-11-24).
8. Generic, timeout and cleanup jobs and load requests stay dropped. The comment at
   `runner.rs:188-190` says so with HOST-85 and HOST-86 `pending slice 14` and HOST-87 `pending
   slice 16` (`conformance/host/requirements.txt:85-87 @ 0.9.0`), not with `hooks.rs:7-9`, whose
   sentence is stale at `0.9.0` (Context), and no longer says promises are refused.
9. `#[ignore = "HOST-125"]` goes from `clock_advances`; `seeds_fixed` (`HOST-26`) and `time_zone`
   (`HOST-127`) stay ignored with their ids, both `pending slice 4` at `0.9.0`. The module comment
   of `engine.rs` (`:1`, `:8-13`) names `0.9.0` and two ignored tests. The swell citations in
   `runner.rs` and `engine.rs` read `@ 0.9.0` with the lines of Context.
10. Requirement 11 at `0.9.0` is `held`: by HOST-215 and its test, and by the probe's run on the
    tag (Context, last paragraph). The unit writes the run's output lines into the note as they
    are in the file handed over (no path of a machine is in them) and builds no probe.
11. The design note, `docs/design/scripts.md`. The unit rewrites the sentences the tag makes
    false and cites them `@ 0.9.0`; every other citation stays `@ 0.8.0`, read at that tag and
    true there (the coordinating agent's decision of 2026-11-24: 119 citations, of which the
    statement of most does not change):
    - the head (`:12-15`, `:28`): "swell is read at its tag `0.9.0`", commit `d49a09b...`, on one
      line; a citation names its tag, and one still written `@ 0.8.0` was read at that tag and is
      true there;
    - Q2, row 1 (`:123`): "taken in effect: the runner performs a checkpoint after each script
      and clears the queue after a limit, so the queue is empty at every end tag (decision 6 of
      `story:swell-0-9-0`)"; a paragraph after the table states decisions 6 and 7 with the HTML
      sections of Context, and names three deviations from HTML, each in a sentence that begins
      "Deviation from HTML": a job's exception is not reported (decision 7), a rejection no
      handler took is not notified (decision 7), and the queue is cleared after a limit, which
      HTML and swell's note do not do, with the reason of decision 6;
    - Q4: the cell of `memory_bytes` (`:300`) drops the sentence on the uncharged copies, citing
      HOST-215; the row "after `Terminated(MemoryBudget)`" (`:317`) gains the clear of decision 6;
      the row of `steps_per_entry` says that a script element spends at most two entries, the
      script and its checkpoint (decision 7);
    - Q6 (`:403-426`): HOST-125 and HOST-126 `test`, the six others `pending slice 4`, HOST-147
      without `Map` and HOST-214 `pending slice 9`, HOST-209 to HOST-213 new and `pending slice 4`;
      its `probe:` line is run again at `0.9.0`;
    - Q7 (`:446-454`): "eight kinds", `AggregateError` among them, the wildcard of decision 4,
      what it cannot notice, and its test; the note's copy of the specification's enum
      `surf.document.ScriptErrorKind` (`:529-540`) gains `{name: AggregateError, summary:
      "AggregateError"}` before `Value`, as `story:script-error-kind-aggregate` writes it into
      `spec/domains/document.yaml` (this story owns the note);
    - Q11: the sentences that call requirement 11 open (`:898-903`, `:908`, `:920`, the cell at
      `:958`) say it is held at `0.9.0`; the bound values stay, which
      `story:scripts-document-port` decides (its decision 3);
    - Q13 (`:1018-1086`): the tag `0.9.0` with its `probe:` line (commit, 8 packages, no `source`
      line), the detail of requirement 11 at `0.8.0` (Context, last paragraph) and its close at
      `0.9.0`, the probe's output lines of decision 10 as a `probe:` line, the closing of slice 10
      and of `Date`'s first wave; the decision text: `story:swell-0-9-0` takes `0.9.0`, and
      `story:scripts-document-port` takes no tag;
    - Q18 and the table (`:1239-1271`): the counts at `0.9.0` on one line, "215 lines, 148
      `test`, 67 `pending`"; "status at `0.9.0`" where `:1242`, `:1247` and the table's head
      `:1259` say "status at `0.8.0`"; rows 1 to 10 `held` with their lines `@ 0.9.0` (row 7:
      `docs/design/host-interface.md:2241 @ 0.9.0` and `Taskfile.yml:117 @ 0.9.0`); row 11 `held`
      with HOST-215, `conformance/host/requirements.txt:215 @ 0.9.0` and the probe of decision 10.

## Change

- `crates/script/Cargo.toml`, `Cargo.lock`, `taskfiles/lock-allow.txt`: decision 1, in the first
  commit, with the test constants of decision 2 (`crates/surf/tests/lock_check.rs`,
  `crates/surf/tests/adversary.rs`).
- `crates/script/src/runner.rs`: decisions 4 to 8.
- `crates/script/tests/engine.rs`, `crates/script/tests/adversary.rs`: decision 9 and the cases of
  acceptance 3 and 4; `crates/script/tests/bindings.rs` (the test file of
  `story:webidl-bindings`, on its branch): the cases `dom_limit_drops_the_jobs` (acceptance 3) and
  the `then` variant of `limit_end` (acceptance 4).
- `NOTICE`, `AGENTS.md`, `README.md:254`, `website/docs/principles.md:27`: decision 3, and the
  numbers of `surf-script`'s tests in `AGENTS.md` (`:356-357` on `main`).
- `docs/design/scripts.md`: decisions 10 and 11.
- In the last commit, the public surfaces of acceptance 8 (`README.md`, `AGENTS.md`,
  `docs/badges/make.sh`, `docs/badges/tests.svg`, `website/docs/status.md`,
  `website/docs/what-works.md`, `website/docs/how-it-is-built.md`).

## Acceptance

1. `slot task lock-check` prints
   `Cargo.lock: <n> package(s), <m> from outside this repository, each on the allow-list` with the
   `<n>` and `<m>` of the story's base; `git grep -n -F -e 'tag=0.8.0' -e 'tag = "0.8.0"' -e
   0c1827128b57845e203713e87a0b0949fc8d63be -- Cargo.lock taskfiles crates NOTICE` prints nothing;
   `git grep -c -F 'tag=0.9.0#d49a09bcc44fbdd155f705f913a90b676518afbc' -- taskfiles/lock-allow.txt`
   counts 6; `git grep -n -F 'tag 0.8.0' -- NOTICE` prints nothing and
   `git grep -c -F 'tag 0.9.0' -- NOTICE` counts 1.
2. Every case of `crates/surf/tests/lock_check.rs` and `crates/surf/tests/adversary.rs` passes, the
   swell cases among them, and `a_swell_package_at_another_commit_fails_naming_it` fails the check
   with a source whose commit differs from the listed one in its first hex digit;
   `git grep -n -E '0\.[89]\.0|d49a09b|0c1827' -- crates/surf/tests/lock_check.rs
   crates/surf/tests/adversary.rs` prints two lines per file, the value line of `SWELL` and the
   line of the version constant (decision 2; on `main` it prints 6 lines of `lock_check.rs` and 25
   of `adversary.rs`); `?tag=0.7.0#` at the case "another tag" is the one other version literal
   left, and it names no listed version.
3. `crates/script/tests/main.rs`, module `engine`, each case with limits of the test's own and
   under 5 s:
   - `clock_advances` runs and passes; `git grep -n '#\[ignore' -- crates/script/tests` prints two
     lines, `HOST-26` and `HOST-127`;
   - `aggregate_error`: `throw new AggregateError([], "m")` ends `threw AggregateError "m" 1:1`;
   - `every_error_constructor_keeps_its_word`: the script
     `throw new Error(Object.getOwnPropertyNames(globalThis).filter(function (n) { return /Error$/.test(n); }).join(","))`
     names the constructors; for each name, `throw new <name>([])` ends `ScriptEnd::Threw(error)`
     with `error.kind.word()` equal to the name; the list holds `AggregateError` and the seven
     others; a name whose kind reads another word fails the test naming it;
   - `checkpoint_runs_after_the_script`: `var x = 0; Promise.resolve().then(function () { x = 1; });`
     ends `ran`, then `if (x !== 1) throw new Error("x")` ends `ran`;
   - `async_function_runs`: `(async function () { await null; globalThis.y = 2; })();` ends `ran`,
     then `if (y !== 2) throw new Error("y")` ends `ran`;
   - `checkpoint_after_threw`: `Promise.resolve().then(function () { globalThis.z = 3; }); throw 1`
     ends `threw value`, then `if (z !== 3) throw new Error("z")` ends `ran`;
   - `job_throw_keeps_the_end`: `Promise.resolve().then(function () { throw new Error("j"); });`
     ends `ran`, and `Promise.reject(new Error("r"));` ends `ran` (decision 7);
   - `job_flood_ends_at_the_step_limit`: `function f() { Promise.resolve().then(f); } f();` ends
     `limit: steps-per-script`; the runner's step total after it is greater than `steps_per_entry`
     and at most twice `steps_per_entry`; a following `1` ends `ran`;
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
4. `crates/script/tests/main.rs`, module `adversary`: `every_error_kind_keeps_its_word` holds eight
   kinds, `AggregateError` thrown as `new AggregateError([], "m")` ending
   `threw AggregateError "m" 1:1`. In `crates/script/tests/bindings.rs` (the test file of
   `story:webidl-bindings`), two cases beside its `limit_end` (`:655` on the branch), each in its
   test document whose `createElement` answers `DomEnd::Limit(Nodes)`:
   - `dom_limit_drops_the_jobs`: a script queues a job that sets a global
     (`Promise.resolve().then(function () { globalThis.q = 1; })`), then calls `createElement`; it
     ends `limit: nodes`; `1` ends `ran`; a third script,
     `if (typeof q !== "undefined") throw new Error("q")`, ends `ran`;
   - the `then` variant of `limit_end`: the call to `createElement` is made inside a `then`
     callback; the script ends `limit: nodes`, and `1` after it ends `ran`.
5. `git grep -n -F '_ => ScriptErrorKind::Error' -- crates/script/src/runner.rs` prints one line,
   inside `error_kind`, under a comment that names what it swallows, what the test of acceptance 3
   cannot notice, and that test; the other wildcard of that file, `_ => {}` over surf's own
   `ScriptEnd` (`runner.rs:572` on `main`), is not over a swell enum. `git grep -n -E
   '\| HostError::MemoryBudget' -- crates/script/src/runner.rs` prints one line, inside the macro
   `surf_defect!`; the seven patterns `HostError::Faulted | surf_defect!()` build without another
   arm (acceptance 7).
6. The coordinating agent hands over the probe's output as one file of 18 lines (per mode one
   `exit 0` line and five `stdout:` lines), sha256
   `78fa5309cf44a42b2de1d6b4ad0abaed7e2d120ee974e5ce67c605ad40fc16da` (`sha256sum`), the identity
   of what the unit receives. `docs/design/scripts.md` holds its lines as a `probe:` line in Q13,
   byte for byte: `git grep -c -F 'one more doubling, to 2^24: terminated StepBudget' --
   docs/design/scripts.md` counts 3; Q18's row 11 reads `held` with HOST-215.
   `git grep -n -F 'status at `0.9.0`' -- docs/design/scripts.md` prints three lines, the two
   sentences and the table's head that read `0.8.0` at `:1242`, `:1247` and `:1259` on `main`;
   `git grep -n -F 'swell is read at its tag `0.9.0`' -- docs/design/scripts.md` prints the line of
   the note's head; Q2's row 1 no longer reads "left out".
7. `slot task check` exits 0; `git diff --stat` names only the files of the Change and the public
   surfaces of acceptance 8.
8. Public surfaces: in its last commit, after merging `main`, the story changes every public surface
   its change makes false, each number from the `slot task check` log of its tip. After that
   commit `git grep -n -F -e '<old>' -- README.md website/docs docs/badges AGENTS.md` prints nothing
   for each old string, read on the story's base with `git grep -n` before the first commit and
   listed in the report: the swell tag strings `release tag (`0.8.0`)` (`README.md:254`,
   `website/docs/principles.md:27` on `main`), `swell 0.8.0` (`AGENTS.md:355`, `:1873`), the test
   counts of the repository and of `surf-script` (`README.md:11`, `:285`, `docs/badges/make.sh:38`,
   `docs/badges/tests.svg`, `website/docs/status.md:26`, `website/docs/what-works.md:17-18`,
   `README.md:650`, `website/docs/how-it-is-built.md:106`, `AGENTS.md:356-357` on `main`), and the
   gate blocks (`README.md:585-626`, `website/docs/how-it-is-built.md:40-81` on `main`) hold that
   log's lines. The lock's counts do not change (acceptance 1).
9. Decisions 8, 9 and 11 in the text: `git grep -c -F '@ 0.8.0' -- crates/script/src/runner.rs
   crates/script/tests/engine.rs` prints nothing (2 and 10 lines on `main`); `git grep -n -F
   'requirements.txt:85-87 @ 0.9.0' -- crates/script/src/runner.rs` prints the comment of decision
   8; in `docs/design/scripts.md`, `git grep -n -F 'eight kinds'` prints a line of Q7 (none on
   `main`), `git grep -n -F '215 lines, 148 `test`, 67 `pending`'` prints one line of Q18,
   `git grep -c -F 'Deviation from HTML'` counts 3 (none on `main`), and `git grep -n -F
   '{name: AggregateError, summary: "AggregateError"}'` prints one line, in the note's copy of
   the enum.
10. The report lists what no test of this story holds at `0.9.0`: the reading of a checkpoint's
    `Throw` and of its `Terminated(EngineFault)` as `faulted`, that the script count does not rise
    for a checkpoint, and that the clear comes before the collection after `limit: memory`.
11. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s
    (`.config/nextest.toml`); no model call and no network request in a test (cargo fetches swell
    before the tests); every build and test runs through `slot`, and a job in the pool ends within
    10 minutes or names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes (the
    first cold build of swell `0.9.0` through the cargo shim may name it); no Python; no generator
    with a committed output is added; a test over 5 s is named with its time.

## Out of scope

The specification's word for `AggregateError` (`story:script-error-kind-aggregate`); recording a
job's throw or an unhandled rejection (decision 7; no story holds it); the writer of a script's end
and the session (`story:scripts-document-port`); the bound values of Q11; host microtasks queued by
surf (`queue_microtask`), timers, events and generic jobs; `Math.random` and the time zone
(`pending slice 4` at `0.9.0`); any change in swell. The other additions of `0.9.0` that no Rust
file of surf names on `main` or on the branch `impl/webidl-bindings` (`git grep -n -E
'AsyncIterator|PromiseCapability|new_promise_capability|PromiseState|promise_state|promise_is_handled|mark_promise_as_handled'
-- 'crates/*.rs'` prints nothing): `WellKnownSymbol::AsyncIterator`, `PromiseCapability` and
`new_promise_capability`, `PromiseState` and the promise reads `promise_state` and
`promise_is_handled`, and `mark_promise_as_handled`; the epic cites `new_promise_capability` for
a later story (`epic/scripts-in-a-page.md:300`).

## Placement

After `story:webidl-bindings` is on `main` (both change `crates/script/src/runner.rs` and
`crates/script/tests/`; that story's rule of decision 10 is the one decision 6 composes with).
Beside `story:script-error-kind-aggregate`: no file in common but the public surfaces (`README.md`,
`website/docs/how-it-is-built.md`); whichever merges second merges `main` first and takes its
numbers from its own log. Before `story:scripts-parse-run`, which depends on this story (it writes
`crates/script/src/runner.rs` and `docs/design/scripts.md`, and measures Q4 on `0.9.0`). Beside
`story:conformance-lock-and-lints` (draft, `epic:headless-rust-target`), which writes `AGENTS.md`,
`website/docs/status.md` and `crates/surf/tests`: whichever merges second merges `main` first.
`story:scripts-document-port` depends on this story and takes no tag of its own. The texts that cite
`0.8.0` as the tag of a statement and follow it: `story:scripts-dom-reading` (`:77`),
`story:scripts-dom-mutation` (`:80`, `crates/swell/src/host/mod.rs:577 @ 0.8.0` is `:604 @ 0.9.0`;
`:105`) and the epic's "Corrections of 2026-11-24" (`epic/scripts-in-a-page.md:960-969`).

## Why `epic:scripts-in-a-page`

The epic runs page scripts on a swell tag that holds the security review's fixes; `0.9.0` charges
the copies the review's condition left open (requirement 11), and a page's promises need the
checkpoint this story runs.
