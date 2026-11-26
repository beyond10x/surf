---
format: aep.planning-md/3
id: story:swell-0-12-0
kind: story
status: draft
title: Pin swell at its tag 0.12.0, where an interrupt set by any host function ends the run
summary: Moves surf's pin of the script engine swell from tag 0.10.0 to 0.12.0 (0.11.0 gives Date, Math and Number; 0.12.0 reads the interrupt flag after every host function, getters, setters, conversions, iterators and promise jobs included). No signature surf calls changes and surf has no cx.call; the change shows in tests and notes. One test is red on 0.10.0 and green on 0.12.0, one is rewritten, time_zone loses its ignore, and the notes of Q2, Q4, Q6, Q13 and Q18 are rewritten.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:swell-0-9-0
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
  path: crates/script/tests/adversary_6.rs
- confidence: cited
  path: crates/script/tests/adversary_checkpoint.rs
- confidence: cited
  path: crates/script/tests/adversary_checkpoint_2.rs
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
revision: 22
---
## Context

surf depends on swell by a git tag. The code of `story:swell-0-9-0`, commit `f463e24` on the
branch `impl/swell-0-10-0`, writes the tag `0.10.0` once, in `crates/script/Cargo.toml:16`. That
commit is implemented and reviewed and is not on `main` yet (`main` is `a14e3b0`, tag `0.8.0`).
This story starts from `f463e24`. Every surf citation below is `<path>:<line>` at `f463e24`.
Every swell citation is `<path>:<line> @ 0.12.0`, read with `git show <tag>:<path>` in a clone
of `https://github.com/beyond10x/swell`. A statement that no file or command shows is marked
"not shown".

swell has released two tags since `0.10.0`, both dated 2026-11-25: `0.11.0`, commit
`3785379a9eb75d25b76e842746cdb7391a768159`, and `0.12.0`, commit
`911a83df281b3e788bc1c7eac41f95ed49227e6e` (`git rev-parse '<tag>^{commit}'`,
`git log -1 --format=%cs <tag>`). This story pins `0.12.0`, which holds `0.11.0`. Its id says
what it does: the tag moves, and one behaviour of the engine that surf's tests and notes
describe changes with it.

### What the two tags hold

`CHANGELOG.md:14-150 @ 0.12.0` has 24 items under Added, Changed, Fixed and Host interface
(11 in `[0.12.0]`, `:21-80`; 13 in `[0.11.0]`, `:92-150`). None is marked "Breaking": `grep -n
'Breaking:' CHANGELOG.md` at the tag finds the first such line at `:350`, in `[0.9.0]`. Item by
item, with surf's use and the grep behind it, the 24 are in the table of the facts file. In
short:

- 14 of the 24 touch no file of surf. The eight Added items of `[0.12.0]` (`:23-61`) are the
  shell binary `swell`, its release assets, the test262 digest, the compare page, the engine
  catalog, the zoo import, `site-data` and two designs. The shell is the package `swell-cli`,
  which `swell` does not depend on (`crates/swell/Cargo.toml:12-17 @ 0.12.0`), and surf's lock
  has no such package (`git grep -c 'swell-cli' f463e24 -- Cargo.lock` prints nothing). The
  other six: the second item of the interrupt fix (`:72-74`), which concerns `cx.call`, and
  surf has no `cx.call` (below); `Number` formatting and `Number.is*` (`:118-123`); `parseInt`
  and `parseFloat` (`:124-127`); the realm-size note (`:135-139`); and the change of `HOST-211`
  (`:143-146`).
- 10 touch a file of surf, none of them its Rust logic, only comments, tests and notes: the
  interrupt fix (`:65-71`) and `HOST-221` (`:78-80`); the six Added items of `Date` and `Math`
  that `docs/design/scripts.md` Q6 says do not exist (`:94-117`); the placeholders of `Date`
  that are gone (`:131-134`); and the six requirements that gain a test (`:147-150`).

The host interface does not change. `git diff --stat 0.10.0 0.12.0 -- crates/swell/src/host
crates/swell/src/lib.rs crates/swell/src/value.rs` lists nothing, and `git diff 0.10.0 0.12.0 --
conformance/host/surface.txt` changes the header (`# swell 0.12.0`), the hash of `HOST-211` and
adds the line of `HOST-221`. So no signature surf calls changes and no `match` of surf over a
swell enum needs a new arm (`ErrorKind` has the eight kinds it had, `crates/swell/src/host/mod.rs`
is unchanged). `conformance/host/requirements.txt @ 0.12.0` has 221 lines, 159 `test` and 62
`pending` (`awk '{c[$2]++}'` over the file prints `pending=62 test=159`; the same at `0.10.0`
prints 220 lines, 152 and 68, the numbers of `docs/design/scripts.md:1439`). Seven lines change
status or are new: `HOST-127`, `HOST-128`, `HOST-135`, `HOST-209`, `HOST-210`, `HOST-211`
and `HOST-221` (`git diff 0.10.0 0.12.0 -- conformance/host/requirements.txt`).

The lock does not change in count. swell's root lock at `0.12.0` has nine packages with no
`source` line (`git show 0.12.0:Cargo.lock | grep -c '^\[\[package\]\]'` prints 9, `grep -c
'^source'` prints 0): the eight of `0.10.0` and `swell-cli`. `swell` takes the same five crates
by path (`crates/swell/Cargo.toml:13-17 @ 0.12.0`), none of its manifests changes between the
tags (`git diff --stat 0.10.0 0.12.0 -- 'crates/*/Cargo.toml'` lists only the new
`crates/cli/Cargo.toml`), and the workspace version is `0.12.0` (`Cargo.toml:8 @ 0.12.0`, the
one line of the root manifest that changed). `rust-toolchain.toml @ 0.12.0` pins rustc 1.99.1,
as surf's does (`rust-toolchain.toml` at `f463e24`), and swell's `NOTICE` is the same at both tags
(`git diff --stat 0.10.0 0.12.0 -- NOTICE` lists nothing). `Cargo.lock` at `f463e24` holds 58
packages, 25 from outside the repository (`grep -c '^\[\[package\]\]'` prints 58, `grep -c
'^source = '` prints 25); the six swell packages keep their names, so the counts stay and
only version and source change. The numbers are those of `f463e24`; the base after
`story:swell-0-9-0` is merged is re-read there (not shown).

### Where the version is written

At `f463e24`, `git grep -n -E '0\.10\.0|b9d3d46'` outside `.engineering` and the website's own
lock lists these places that state swell's version, tag or commit:

- `crates/script/Cargo.toml:16`, the dependency line.
- `Cargo.lock:350-351`, `:362-363`, `:371-372`, `:376-377`, `:384-385`, `:393-394`: per package
  a `version = "0.10.0"` line and a `source = "git+...?tag=0.10.0#b9d3d46..."` line, six
  packages.
- `taskfiles/lock-allow.txt:8-13`, six lines `<name> 0.10.0 <source> none`.
- `crates/surf/tests/lock_check.rs:21-25` and `crates/surf/tests/adversary.rs:11-15`: the
  constants `SWELL` (source with tag and commit) and `SWELL_VERSION`. No other literal of the
  tag, version or commit is left in the two files (the grep above lists only these four
  lines), as `story:swell-0-9-0` decided.
- `NOTICE:54`, `AGENTS.md:366` and `:1901`, `README.md:254`, `website/docs/principles.md:27`.
- `docs/design/scripts.md`: the sentences that name `0.10.0` as the pinned tag, not as the tag
  of a citation: `:13`, `:17-18`, `:31`, `:140`, `:526`, `:546`, the block of Q13 `:1149-1160`
  with its probe `:1162`, `:1273`, `:1276`, the count line of Q18 `:1439`, and the lines
  `:1443`, `:1448`, `:1452`, `:1456` and `:1470` that say "status at `0.10.0`".
- `crates/script/src/runner.rs`: `:21`, `:26-27`, `:55`, `:208`, `:230`, `:641`, `:709`,
  `:765-768`, `:810-818`, `:885`, `:947`, `:1075`, `:1088-1089`; `crates/script/tests/engine.rs`:
  `:1`, `:10`, `:29-37`, `:652`, `:721`, `:742`, `:778`, `:794`;
  `crates/script/tests/adversary_6.rs:170`, `adversary_checkpoint.rs:1`, `:157`,
  `run_end.rs:8`. Each is a doc comment or a citation `@ 0.10.0`; a citation stays when the
  file it names is the same at both tags.

A citation names the tag it was read at, and it stays when the sentence is still true there.
`git diff --quiet 0.10.0 0.12.0 -- <path>` says which swell files differ. The swell files that
surf's tree cites (35 paths, read with `git grep -h -o -E 'crates/(swell|syntax|compiler|num|
regexp|unicode)/[A-Za-z0-9_/.-]+\.rs' f463e24`) are the same at both tags but five:
`crates/swell/src/interp/budget.rs`, `crates/swell/src/object/builtin_function.rs`,
`crates/swell/tests/limits_scripts.rs`, `crates/swell/tests/frontend_ledger.rs` and
`crates/swell/tests/support/host.rs`. The last two are cited only `@ 0.8.0` (`scripts.md:382`,
`:391`, `:439`, `:1181`) and stay: a citation names its tag. `builtin_function.rs` is cited
`:497 @ 0.8.0` in the generated copy (`bindings.rs:443`, `prelude.rs:372`) and stays. Three
citations `@ 0.10.0` land on other lines at `0.12.0`: `interp/budget.rs:58` (`engine.rs:37`) is
`:59 @ 0.12.0`; `interp/budget.rs:34-41` (`runner.rs:768`, `docs/design/scripts.md:329`) is
`:35-42 @ 0.12.0`; `limits_scripts.rs:126` (`engine.rs:721`) is `:157 @ 0.12.0` (the test
`host_17_deepest_reentry_parses_at_full_nesting`). They stay as they are while the sentence they
support is true at `0.10.0`; a sentence the unit rewrites cites the new line `@ 0.12.0`.

### The interrupt flag, read in full

What swell changed. At `0.10.0` the flag was read after a builtin or host function that a
`Call` or `New` instruction ran (`Agent::check_interrupt_after`). At `0.12.0` the same check
ends `Agent::run_host_function`, the one place every host function runs, whoever called it
(`crates/swell/src/object/builtin_function.rs:828-832`, `:863-881 @ 0.12.0`; the doc comment of
the module says so, `crates/swell/src/interp/budget.rs:10-15 @ 0.12.0`). So a host function that
sets the flag and answers `Ok` or a throw, reached as a getter, a setter, a conversion
(`valueOf`, `toString`), an iterator's `next` or a promise job (a thenable's `then`, a
reaction handler), ends the run or the microtask checkpoint in `Terminated(Interrupted)` when it
returns, also when it is the script's last operation, and the thrown value reaches no `catch`
(`CHANGELOG.md:65-71 @ 0.12.0`; `HOST-221`, `docs/design/host-interface.md:1339 @ 0.12.0`, with
the test `crates/swell/tests/interrupt.rs:548-660 @ 0.12.0`). The check reads the flag and spends
no step (`interp/budget.rs:35-42 @ 0.12.0`). An answer that is already a termination keeps its
reason and the flag is not read (`interp/budget.rs:44-53 @ 0.12.0`), and a run that was
terminated before the host function returned ignores what it returned
(`builtin_function.rs:861-862 @ 0.12.0`). An embedder that calls a host function with `cx.call`
gets `Err(Abrupt::Terminated(Termination::Interrupted))` where it got the function's `Ok`
(`CHANGELOG.md:72-74 @ 0.12.0`).

Where surf sets the interrupt, at `f463e24`:

- `bindings::end_with_limit` (`crates/script/src/bindings.rs:445-452`; the generated copy is
  `crates/webidl/src/prelude.rs:374-381`) records a limit, sets the flag
  (`bindings.rs:447`), reads it at once with `Cx::charge_steps(0)` (`:448`) and answers the
  termination it got, or `Terminated(Interrupted)` itself. It has one caller, the end of every
  member that answers `DomEnd::Limit` (`bindings.rs:814`). Whichever member that is, a getter,
  a setter or a method, the function answers a termination, so at `0.12.0` the check after it finds
  the run terminated and reads nothing (`builtin_function.rs:862`, `budget.rs:49-51 @ 0.12.0`).
  No production host function of surf sets the flag and then answers `Ok` or a throw. So the
  fix changes nothing a page can reach through surf's own members.
- The wall-clock thread (`crates/script/src/runner.rs:512`) sets it from another thread once
  the deadline passed. swell now also reads it when any host function returns, so a run can end
  at the next host return and not at the next call or loop jump. The end word is
  `limit: wall-clock` either way (`runner.rs:1070`).
- The runner withdraws it after every script (`runner.rs:675`).
- Test slices set it: the getter of `Window.document` in `crates/script/tests/run_end.rs:48-50`,
  `adversary_checkpoint_2.rs:49`, `adversary_checkpoint.rs:323` (a getter that also throws), and
  the method `getElementById("interrupt")` in `adversary_5.rs:98`. The arm `Wait`
  (`adversary_checkpoint_2.rs:50-56`) clears the flag inside the getter, so the new check finds
  it clear. These are the only cases in which surf has a host function that sets the flag and
  answers something other than a termination.

Calls from Rust into script or into a host function, at `f463e24`:
`git grep -n -F -e 'cx.call' -e '.call(' f463e24 -- crates/script/src
crates/webidl/src/prelude.rs` prints nothing, so surf has no `cx.call`. What surf calls:
`Agent::evaluate_script` (`runner.rs:855`), `Agent::perform_microtask_checkpoint`
(`runner.rs:821`), `Agent::collect` (`runner.rs:988`),
`Cx::construct` of the realm's `Array` with no argument (`bindings.rs:1162`), `Cx::get` of
data properties of its own prototypes and its global (`bindings.rs:621`, `:1161`, `:1177`,
`:1239`, `runner.rs:954`, `:1052`), and the conversions `to_string` (`bindings.rs:838`),
`to_number` (`:909`, `runner.rs:960`) and `to_object` (`:411`, `:873`). A conversion may run a
page's `toString` or `valueOf`; if that is a host function of surf that sets the flag, it is
`end_with_limit`, which answers a termination. What each does with an `Err` of the new kind:
`bindings::engine` passes the abrupt completion on (`bindings.rs:580-582`), and
`runner.rs:1066-1073` reads `Termination::Interrupted` as `limit: wall-clock`. The one place that
names the pattern is the read of a thrown value, `runner.rs:960-963`, where `to_number(undefined)`
runs no host function, so the new check is not reached there.

What the runner does with the flag at the end of a script, from `story:swell-0-9-0`
(`runner.rs:632-669`, `:763-793`): after the script's run it asks whether the clock ran out or
the flag is set (`run_ended`, `:769-771`) and, if so, ends the script `limit: wall-clock`; it asks
again before it reads a thrown value (`thrown`, `:884-890`) and after a checkpoint that emptied
the queue (`checkpoint`, `:820-830`); the read of a thrown value ends in `Err(RunEnded)` when the
flag is set between the question and the read (`:944-948`, `:960-963`). At `0.10.0` a getter of
the bindings that set the flag and threw ended its run with the flag still set, and a job whose
function was a host function left it set when the checkpoint answered `Normal`
(`runner.rs:808-814`, `docs/design/scripts.md:331-346`). At `0.12.0` neither happens: swell reads
the flag when the getter or the job returns. The clock's thread sets the flag only after the
deadline passed (`runner.rs:508-513`), so `clock.ran_out()` is already true for a flag it sets;
a flag that a host function of surf's tests sets is read by swell when that function returns.
The runner's questions stay as the second line `story:swell-0-9-0` made them, because the
thread can set the flag after swell's last read and before the runner returns: the clock half
decides that case, and no deterministic test can make the flag half decide (decision 5).

Sentences of surf's tree about the old behaviour, each marked false, true or can go at `0.12.0`
(the table with lines is in the facts file):

- False: `runner.rs:808-814` ("when that function is a script function, and not when it is a
  host function", and "a last job whose function is a host function [...] leaves the flag set
  when the checkpoint answers `Normal`"; the clause on the wall clock's thread stays true).
  `docs/design/scripts.md:331-334` ("a getter of the bindings that sets it, and then throws or
  is followed by a `throw`, meets no read of it") and `:340-346` (the same sentence as
  `runner.rs:808-814`). `crates/script/tests/run_end.rs:1-10` (the module comment: "swell reads
  the flag at the next call, loop jump or return of script code, so a throw right after the
  getter ends the run with the flag still set"), `:158-160` ("swell's read at the script's return
  takes the flag"). `adversary_checkpoint_2.rs:154-159` ("nothing reads the flag after a host
  function a job called") and `:254-258` ("a property get reads no flag after it").
  `adversary_checkpoint.rs:308-317` (a getter that sets the flag and throws as the model of
  the clock's thread) and `:382-387` (the read of the record "takes the flag").
- True: `runner.rs:641-646`, `:709-710`, `:885-887` (the two second-line checks never decide
  an end alone; only the stamp `0.10.0` and the closing clause "decide if a later tag changes
  where the engine reads the flag" are rewritten, because that tag is this one); `runner.rs:763-768`
  and `:944-948`; `docs/design/scripts.md:316-321`, `:323-330`, `:348-353`;
  `adversary_3.rs:311-317` (it describes `0.8.0` and `0.9.1`); `adversary_6.rs:170-174` (a
  method that throws, `HOST-216`); `adversary_checkpoint.rs:125-128`; `run_end.rs:208-210` (a job
  whose function is a script function); `.engineering/planning/story/scripts-dom-reading.md:467-468`
  ("an interrupt is taken only after a host call, so one call whose time is not bounded by its
  steps runs past the wall clock": the flag is still read when the call returns).
- Can go: the stamp "as swell `0.10.0` reads it" (`adversary_6.rs:170`).
- Test names that claim what no longer holds:
  `a_host_function_as_the_last_job_leaves_the_flag_to_the_runner`
  (`adversary_checkpoint_2.rs:161`) and
  `an_interrupt_the_record_read_takes_still_ends_the_script_at_the_wall_clock`
  (`adversary_checkpoint.rs:389`). Both stay green at `0.12.0`, by reading (below), and both are
  renamed.

The tests that set the flag through a getter or a job, read at `0.12.0` (nobody has run them
there; decision 7 says how the implementor finds out):

- `run_end.rs:161-193` (three cases, `Arm::Set`, expected `(Limit(WallClock), false, Ran)`),
  `:197-206` (`Arm::LimitThrow`, a termination from `end_with_limit`), `:211-221` (a job that is
  a script function), `adversary_checkpoint_2.rs:160-177`, `:183-199`, `:204-220`, `:225-252`,
  and `adversary_checkpoint.rs:388-408`: each ends at the same word and flag as before, by
  another route (at the getter, not at the return). Green by reading.
- `adversary_checkpoint_2.rs:259-274`
  (`a_step_limit_with_the_interrupt_set_leaves_no_flag_for_the_next_script`): the script reads
  `G.document` first, the getter sets the flag and answers `Ok`, and the new check ends the run in
  `Terminated(Interrupted)` before the step limit (1,000 steps, `:265`) is reached, so the end is
  `limit: wall-clock` where the test expects `limit: steps-per-script` (`:269`). Red by reading.
- No other test of surf differs between the tags. The case of `HOST-221` for a getter that is
  followed by `try`/`catch` code does not exist in surf; decision 6 adds it.

### `Date`, `Math` and `Number`

`docs/design/scripts.md` Q6 states what `Date` and `Math` are at `0.10.0`: "the other setters,
`Date.UTC` and `Date.parse` throw a `TypeError`" (`:528`), "`Math.random` and all of `Math` but
`max`, and the getters of a date's fields, do not exist" (`:536-537`), the six requirements
`pending slice 4` (`:536`, `:538-542`), and the table rows `:509-512`. At `0.12.0` these are false
except `Math.random`: `Date` has its time zone, 17 getters, 14 setters and text
(`CHANGELOG.md:94-106`), `Math` has its constants and functions (`:107-117`), `Number` its
formatting (`:118-123`), `parseInt` and `parseFloat` are globals (`:124-127`), and the placeholders
of `0.9.0` are gone (`:131-134`). `Math.random` is a later story of slice 4
(`crates/swell/src/builtins/math.rs:24 @ 0.12.0`; `HOST-26`, `HOST-27`, `HOST-144`, `HOST-147`,
`HOST-212` and `HOST-213` are `pending slice 4`, `conformance/host/requirements.txt:26-27`,
`:144`, `:147`, `:212-213 @ 0.12.0`). No surf test states the placeholder behaviour:
`git grep -n -E 'Invalid Date|Date\.UTC|Date\.parse|toGMTString|placeholder' --
crates/script crates/webidl/src` prints nothing, and the Date and Math strings in
`crates/script/tests/engine.rs` are `:340-378`, the three ignored or clock tests. The docs that
state it are `docs/design/scripts.md:509-546`, `:1205`, `:1210-1212`.

The hooks `Date` uses are swell's defaults and surf keeps them: `system_time_zone_identifier`
answers `"UTC"`, `get_named_time_zone_offset_nanoseconds` answers 0 and
`get_named_time_zone_epoch_nanoseconds` has a default
(`crates/swell/src/host/hooks.rs:310-345 @ 0.12.0`, the same file as at `0.10.0`), and
`crates/script/src/runner.rs:225-227` says surf overrides none. So a page reads the time zone
UTC: surf reveals no time zone of its host, the choice of Q6 (`docs/design/scripts.md:510`).
`HOST-127` has a test at `0.12.0` that runs `new Date(0).getTimezoneOffset()` with swell's
provided hooks (`crates/swell/tests/date_text.rs:98-116 @ 0.12.0`), which is the test
`engine::time_zone` of surf (`engine.rs:370-378`), ignored with `#[ignore = "HOST-127"]` today.
`engine::seeds_fixed` (`:351-368`, `HOST-26`) stays ignored.

What a page can now spend: swell charges the new functions to the budgets itself ("every hook
call is charged one step before it is made", `CHANGELOG.md:96-97`; `Date.parse` "charges one
step per code unit past 32", `:104-106`; `Math.hypot` charges "one step per argument and its list
to the memory budget", `:112-113`). surf's limits are those of Q4 and need no new row. The new
builtins also make the realm larger: swell's own tests had to be changed to hold at any size
of the realm, and tests that compared against a room fitted to one size now read their room
from agents of their own (`CHANGELOG.md:135-139 @ 0.12.0`). Four tests of surf set
`memory_bytes` to 1 MiB
(`engine.rs:166`, `:631`, `adversary.rs:203`, `bindings.rs:959`) and two to 4 MiB
(`adversary_4.rs:644`, `adversary_checkpoint.rs:26`). Whether any of them turns red because the
realm grew is not shown (decision 8).

### `HostObjectSpec`

swell's design lists two more fields, `overrides` and `is_html_dda`, and a type parameter:
`pub struct HostObjectSpec<H: Host>` with `error_kind`, `overrides`, `is_html_dda`
(`docs/design/host-interface.md:1768-1776 @ 0.12.0`). The code at `0.12.0` has four fields and no
parameter (`crates/swell/src/host/host_object.rs:134-144 @ 0.12.0`), and its module comment says
`overrides` (slice 9) and `is_html_dda` (slice 18) "are added to [`HostObjectSpec`] by the slices
of their requirements" (`host_object.rs:7-9 @ 0.12.0`). The sentence "a breaking change of the
interface: every `HostObjectSpec` literal names it" is swell's note on `error_kind`, which is
already added (`docs/design/host-interface.md:1818-1819 @ 0.12.0`). `overrides` is not in
`0.12.0`, and the list of surf's literals is in Out of scope.

swell's `main` after `0.12.0` is twelve commits ahead (`git log --oneline 0.12.0..origin/main`;
the clone's local `main` branch is stale and is not used). Its newest are the merges that make
the crates `#![no_std]` (`d0d00a55` "swell-num builds without std", `bdbda15b` "Build unicode,
syntax, compiler and regexp without std"). That work has no tag, and this story does not take it.

## Decisions taken

1. The tag is `0.12.0`, commit `911a83df281b3e788bc1c7eac41f95ed49227e6e`, written once:
   `swell = { git = "https://github.com/beyond10x/swell", tag = "0.12.0" }` in
   `crates/script/Cargo.toml:16`. All six swell packages move together, in one commit with
   the lock and the other places that state the version: `Cargo.lock` (the six packages at
   version `0.12.0`, source
   `git+https://github.com/beyond10x/swell?tag=0.12.0#911a83df281b3e788bc1c7eac41f95ed49227e6e`,
   at the lines of Context), `taskfiles/lock-allow.txt:8-13` (`<name> 0.12.0 <source> none`),
   and the constants of decision 2. No package is added or removed: `swell-cli` is in swell's
   own lock only, and `swell` does not depend on it, so surf's lock keeps its counts (Context).
2. The test files name the tag, the version and the commit once each, as `story:swell-0-9-0`
   decided: `SWELL` and `SWELL_VERSION` in `crates/surf/tests/lock_check.rs:21-25` and
   `crates/surf/tests/adversary.rs:11-15`. The unit changes those two constants in each file and no
   other line; every other literal in the two files is built from them, so the grep of
   acceptance 2 shows the count of lines it showed before.
3. The places of Context that name the tag in prose change with it: `NOTICE:54`,
   `AGENTS.md:366` and `:1901`, `README.md:254`, `website/docs/principles.md:27` name `0.12.0`;
   swell's `NOTICE` is the same file at both tags, so the quoted text stays. The sentence of
   `AGENTS.md:365-369` about `surf-script` changes the tag and its list of ignored tests: at
   `f463e24` it says 3 ignored, "HOST-125, HOST-26 and HOST-127" (`:367-368`), while the code has
   two `#[ignore` lines (`git grep -n -F '#[ignore' f463e24 -- crates/script/tests` prints
   `engine.rs:353` and `:372`), so it names `HOST-26` only after this story (decision 9). The
   base, after `story:swell-0-9-0` merged, is re-read for the figure (not shown).
   `story:swell-0-9-0` and `story:scripts-dom-reading` own the sentence's other words.
4. The runner's logic does not change. No signature surf calls changes (Context), the code
   compiles unchanged, `end_with_limit` keeps its `Cx::charge_steps(0)` (a termination is read
   the same way with or without the fix) and surf has no `cx.call`. What changes in
   `crates/script/src/runner.rs` is comments: a stamp `0.10.0` is re-cited `@ 0.12.0` only
   where its sentence is false or is rewritten, and the doc comment of `Runner::checkpoint`
   (`:804-819`), whose clause about a host function as a job's function is false, is rewritten:
   the checkpoint reads the flag at its start, before each job and after every function a job
   ran, a host function included (`HOST-221`); the clause about the clock's thread stays. The
   stamp in `:641-646`, `:709-710` and `:885-887` ("at swell `0.10.0` they never decide an end
   alone", "decide if a later tag changes where the engine reads the flag") says `0.12.0`
   and what the tag changed.
5. The runner's second-line questions stay: the clock and flag halves of `run_ended`, the
   question before the read of a thrown value, the question after the checkpoint, and the
   `Err(RunEnded)` branch. Reason: the clock's thread sets the flag from another thread at any
   moment after the deadline (`runner.rs:508-513`), so a flag can be set after swell's last read
   and before the runner returns, at any tag. After this tag no deterministic test makes the flag
   half decide: a host function of surf's tests that sets the flag is read by swell when it
   returns. The doc comments (`runner.rs:641-646`, `:709-710`, `:885-887`,
   `docs/design/scripts.md:348-353`) say so, and the report lists these branches as held by no
   test (acceptance 10). Question 1 of the questions file asks whether to delete the flag
   halves instead; the default is to keep them.
6. A case for the behaviour of the tag that surf's tests rely on, the smallest that is red on
   `0.10.0` and green on `0.12.0`: in `crates/script/tests/run_end.rs`, which has the slice
   (`Arm::Throw`, `:31-32`, `:49-52`) and the helper `ends` (`:142-156`), the test
   `a_getter_that_sets_the_interrupt_and_throws_ends_the_script_at_the_getter`. The script is
   `var G = this; G.document.getElementById("throw"); try { G.document; } catch (e) { G.job =
   1; }`, and the helper's second script is `NO_JOB` (`:139-140`), which throws when `job` is
   defined. Expected `(ScriptEnd::Limit(Reason::WallClock), false, ScriptEnd::Ran)`. At
   `0.12.0` the getter ends the run before the `catch` runs (`HOST-221`, "both when the read is
   followed by `try`/`catch` code that stores a global", `host-interface.md:1339 @ 0.12.0`),
   `job` stays undefined, and the second script runs. At `0.10.0` nothing reads the flag between
   the getter's throw and the script's return, so the `catch` stores `job`, swell reads the flag
   at the return, the first script ends `limit: wall-clock` with the flag clear, and the second
   script ends `threw` (by reading, `docs/design/scripts.md:331-334`, `run_end.rs:1-10`). The
   test names no clock and no wall time, only the event (`AGENTS.md:1965-1967`). Its doc
   comment names `HOST-221`.
   The second case is a rewrite, not an addition:
   `a_step_limit_with_the_interrupt_set_leaves_no_flag_for_the_next_script`
   (`adversary_checkpoint_2.rs:259`) keeps its claim, which is the design note's rule that a
   flag set while the clock had not run out leaves the next script to run, but reaches it a
   way `0.12.0` allows: a new arm `Steps` of the slice (`:25-33`), armed by
   `getElementById("steps")`, whose getter calls `cx.engine().charge_steps(u64::MAX)` and drops
   the answer (the pattern of `adversary_4.rs:221`), then `cx.interrupt().interrupt()`, then
   answers `Ok`. The run is terminated by the step budget before the getter returns, so swell
   does not read the flag (`builtin_function.rs:862`, `budget.rs:49-51 @ 0.12.0`) and the flag
   stays set; the script ends `limit: steps-per-script`, the runner withdraws the flag and the
   next script runs. Expected `(Limit(StepsPerScript), false, 0, Ran)`, the tuple of the test
   today; the same at both tags. The comment at `:254-258` loses "a property get reads no flag
   after it".
   The renames of the Context (`adversary_checkpoint_2.rs:161` to
   `a_host_function_as_the_last_job_ends_the_checkpoint_at_the_wall_clock`,
   `adversary_checkpoint.rs:389` to
   `an_interrupt_set_at_the_end_of_a_script_ends_it_at_the_wall_clock`) change the names and
   the doc comments and no assertion. The test slice's doc comments of
   `adversary_checkpoint.rs:308-317`, `run_end.rs:1-10` and `:158-160` are rewritten for
   `0.12.0`.
7. The order that proves the red, kept in the report. First, on the base with the pin at
   `0.10.0`: write the new test of decision 6 and run it; it must fail with the second script
   `threw`, and the rewritten step-limit test must pass. If the new test is green at `0.10.0`
   the unit stops and reports, and keeps no test that is not red there. Then move the pin and
   run `slot cargo nextest run -p surf-script` before any other edit: the report names every
   red test and the reading above (one expected, the step-limit test) says which. Then the
   renames and comments. A test that is red for a reason the reading did not give is a finding,
   not an edit.
8. A test of surf whose memory room was fitted to the realm of `0.10.0` and that is red at
   `0.12.0` is fixed by raising its `memory_bytes` to the next whole MiB that passes, with the
   reading of `live_bytes()` after the realm printed and named in the report. A bound is never
   raised for any other reason, and the report names each test it touched. Not shown: that any
   of the six tests of Context is red. Question 3 of the questions file gives the other way.
9. `engine::time_zone` loses its `#[ignore = "HOST-127"]` and passes (`HOST-127` has a test at
   `0.12.0`; the draft `story:scripts-seeds-and-time-zone` held both ignores for one later tag,
   see Placement and Question 4); `engine::seeds_fixed` keeps `#[ignore = "HOST-26"]`
   (`Math.random` is not in `0.12.0`). The module comment of `engine.rs` (`:1`, `:9-14`) names
   `0.12.0` and one ignored
   test. If `time_zone` does not pass, the unit reports the script, the end and the reading
   and keeps the ignore with the reason.
10. The design note, `docs/design/scripts.md`: the unit rewrites the sentences the tag makes
    false and cites them `@ 0.12.0`; every other citation stays at the tag it was read at.
    The edits:
    - the head (`:13`, `:17-18`) and the table row (`:31`): "swell is read at its tag
      `0.12.0`", commit `911a83d...`, on one line;
    - Q4, the paragraph on a DOM write over a parser limit (`:312-321`): an interrupt set by a
      host function that then throws or answers `Ok`, reached by a getter, a setter, a
      conversion, an iterator or a promise job, ends the run in `Terminated(Interrupted)`
      (`HOST-221`); the paragraphs `:323-353` are rewritten as the Context says (the three
      false sentences, the second-line paragraph with its stamp);
    - Q6 (`:505-546`): the rows `:509-512` and the paragraphs `:526-542` say that `Date` has its
      zone, getters, setters and text, `Math` all but `random`, `Number` its formatting, and that
      `HOST-127`, `HOST-128`, `HOST-135`, `HOST-209`, `HOST-210`, `HOST-211` have a test and the
      other six ids are `pending slice 4`; the `probe:` line (`:544`) and the Settled line
      (`:546`) are run again at `0.12.0`;
    - Q13 (`:1147-1160`, `:1162`, `:1273-1276`): the tag `0.12.0`, its commit, `0.10.0` and
      `0.11.0` as the tags it holds, the root lock of nine packages with no `source` line, six
      taken by a host, and `swell-cli` named beside `swell-test262` and `swell-unicode-gen` as a
      package a host does not take. The `probe:` line `:1162` is read again at `0.12.0` (git
      reads, no build) and its output is written; the block of `0.8.0` (`:1164-1168`) stays;
    - Q18 (`:1205`, `:1210-1212`, `:1438-1456`, `:1470-1479`): the sentence that calls `Date`
      open; the count line "221 lines, 159 `test`, 62 `pending`"; "status at `0.12.0`" in the
      heads; the rows 1 to 11 stay `held` (no row names a requirement that changed:
      `/usr/bin/grep -o -E 'HOST-(127|128|135|209|210|211|221)'` over `:1458-1468` prints
      nothing) and a bullet for `HOST-221` is added to the list of requirements new at `0.9.1`
      and later (`conformance/host/requirements.txt:221 @ 0.12.0`).
    The copies of requirement 11 and the measurement of `0.9.0` (`:1214-1244`) stay as
    `story:swell-0-9-0` left them. The files of the copies
    (`crates/swell/src/builtins/array/mod.rs`, `builtins/array/mutation.rs`, `object/string.rs`,
    `interp/dispatch.rs`, `interp/spread.rs`,
    `heap/string.rs`, `host/mod.rs`) are the same at `0.12.0` (`git diff --stat 0.10.0 0.12.0 --
    crates/swell/src/heap crates/swell/src/host/mod.rs crates/swell/src/interp/dispatch.rs
    crates/swell/src/interp/spread.rs crates/swell/src/builtins/array crates/swell/src/object`
    lists only `object/builtin_function.rs`). The engine sources that differ are eight files
    (`git diff --stat 0.10.0 0.12.0 -- crates/swell/src`): four builtins, `interp/budget.rs`,
    `object/builtin_function.rs` and two of `realm/`. The probe's scripts set no interrupt and
    call none of the new builtins, so the reading stays by reading; the realm is larger, which
    decision 8 covers. Not shown: a run of the probe on `0.12.0`.
11. What the new requirements mean for surf, one line each (the tests of the six requirements are
    `CHANGELOG.md:147-150 @ 0.12.0`). `HOST-221`: used by decision 6 and recorded in Q4 and in the
    Q18 list. `HOST-127` (the provided zone hooks): held by `engine::time_zone` (decision 9).
    `HOST-128` (an offset of one hour), `HOST-209` (an offset out of range) and `HOST-210` (a host
    with no instant): they concern a host that overrides the zone hooks; surf keeps the defaults
    (`runner.rs:225-227`), so they are recorded in Q6 and no test is added. `HOST-211` (the step
    charged to each zone hook call) applies to surf's defaults too and is recorded in Q6.
    `HOST-135` (the bytes of `Math` on every platform) is recorded in Q6.

## Change

- `crates/script/Cargo.toml`, `Cargo.lock`, `taskfiles/lock-allow.txt`: decision 1, in the first
  commit, with the constants of decision 2 (`crates/surf/tests/lock_check.rs`,
  `crates/surf/tests/adversary.rs`).
- `crates/script/tests/run_end.rs`: the new test of decision 6, the comments of decision 6.
  `crates/script/tests/adversary_checkpoint_2.rs`: the rewrite and the rename of decision 6.
  `crates/script/tests/adversary_checkpoint.rs`: the rename and the comments of decision 6.
  `crates/script/tests/adversary_6.rs`: the stamp of decision 6. `crates/script/tests/engine.rs`:
  decision 9 and the stamps of the sentences it rewrites. `crates/script/tests/adversary.rs`,
  `bindings.rs`, `adversary_4.rs`: only if decision 8 applies.
- `crates/script/src/runner.rs`: decision 4, comments only.
- `NOTICE`, `AGENTS.md`, `README.md`, `website/docs/principles.md`: decision 3.
- `docs/design/scripts.md`: decision 10 and 11.
- In the last commit, on its own tip, the public numbers (acceptance 9): `README.md`,
  `AGENTS.md`, `docs/badges/make.sh`, `docs/badges/tests.svg`, `website/docs/status.md`,
  `website/docs/what-works.md`, `website/docs/how-it-is-built.md`.

## Acceptance

1. `slot task lock-check` prints
   `Cargo.lock: <n> package(s), <m> from outside this repository, each on the allow-list`
   with the `<n>` and `<m>` of the story's base (58 and 25 at `f463e24`, re-read at the base);
   `git grep -n -F -e 'tag=0.10.0' -e 'tag = "0.10.0"' -e b9d3d46fa7ac4abc982f2248e903e6301e277a9f
   -- Cargo.lock taskfiles crates NOTICE` prints nothing; `git grep -c -F
   'tag=0.12.0#911a83df281b3e788bc1c7eac41f95ed49227e6e' -- Cargo.lock taskfiles/lock-allow.txt`
   counts 6 in each file; `git grep -c -F 'tag 0.12.0' -- NOTICE` counts 1 (`NOTICE:54` at the
   base) and the same with `tag 0.10.0` prints nothing.
2. `slot cargo nextest run -p surf --test lock_check --test adversary` exits 0 within the bounds
   of acceptance 10: every case passes, the swell cases among them. `git grep -n -F -e '0.10.0'
   -e b9d3d46 -- crates/surf/tests/lock_check.rs crates/surf/tests/adversary.rs` prints nothing,
   and `git grep -c -F 911a83d -- crates/surf/tests/lock_check.rs crates/surf/tests/adversary.rs`
   counts 1 in each file.
3. In `crates/script/tests/run_end.rs` the test of decision 6 passes at `0.12.0`, and the report
   says it failed at `0.10.0` and quotes the end of the second script (`threw`) from that run
   (decision 7). `git grep -n -F
   'a_getter_that_sets_the_interrupt_and_throws_ends_the_script_at_the_getter' --
   crates/script/tests` prints one line, and `git grep -c -F 'HOST-221' --
   crates/script/tests/run_end.rs` counts at least 1 (its doc comment).
4. `slot cargo nextest run -p surf-script --test run_end --test adversary_checkpoint --test
   adversary_checkpoint_2 --test adversary_3 --test adversary_6` exits 0 within the bounds of
   acceptance 10, and these greps print what is said: `git grep -n -F 'a property get reads no
   flag after it' -- crates/script/tests` prints nothing (`adversary_checkpoint_2.rs:254` at
   the base); `git grep -n -F -e
   'a_host_function_as_the_last_job_leaves_the_flag_to_the_runner' -e
   'an_interrupt_the_record_read_takes_still_ends' -- crates/script/tests` prints nothing, and
   the two new names of decision 6 print one line each; `git grep -n -F 'not after a job whose'
   -- crates/script/tests` prints nothing (`run_end.rs:8` at the base). The rewritten step-limit
   test passes at both tags (decision 7).
5. `slot cargo nextest run -p surf-script --test engine` exits 0 within the bounds of acceptance
   10: `engine::time_zone` runs and passes; `git grep -n -F '#[ignore' -- crates/script/tests`
   prints one line, `HOST-26`; `git grep -n -F '0.10.0' -- crates/script/tests/engine.rs |
   /usr/bin/grep -v -F '@ 0.10.0'` prints nothing (the stamps of `:1` and `:10` name `0.12.0`;
   the lines that remain are citations `@ 0.10.0` of files that are the same at both tags).
6. `git grep -n -F -e 'and not when it is a host function' -e 'meets no read of it' --
   crates/script/src/runner.rs docs/design/scripts.md` prints nothing (`runner.rs:811` and
   `docs/design/scripts.md:333`, `:343` at the base). `git grep -c -F 'HOST-221' --
   docs/design/scripts.md` counts at least 2 (Q4 and the list of Q18). `git grep -n -F '221
   lines, 159 `test`, 62 `pending`' -- docs/design/scripts.md` prints one line, and the same
   with `220 lines, 152 `test`, 68 `pending`` prints nothing. `git grep -n -F 'swell is read at
   its tag `0.12.0`' -- docs/design/scripts.md` prints the head (`:13` at the base for the
   `0.10.0` form). `git grep -n -F -e 'Date.parse` throw a `TypeError`' -e 'fields, do not
   exist' -- docs/design/scripts.md` prints nothing (`:528`, `:537` at the base). `git grep -c
   -F "rev-parse '0.12.0^{commit}'" -- docs/design/scripts.md` counts 1 (the probe of Q13).
7. `slot task check` exits 0, and `git diff --stat main...HEAD`, run after the last merge of
   `main` into the unit's branch, names only the files of the Change and the public surfaces
   of acceptance 9.
8. The prose that names the tag, the version or the ignored tests: the strings `release tag
   (`0.10.0`)` (`README.md:254`, `website/docs/principles.md:27`) and `swell 0.10.0`
   (`AGENTS.md:366`, `:1901`) name `0.12.0`; the sentences on ignored tests of `surf-script`
   (`README.md:285-286`, `website/docs/what-works.md:17-18`, `AGENTS.md:367-368`,
   `website/docs/status.md:31`) say one test is ignored, `HOST-26`. At `f463e24` the code has two
   `#[ignore` lines and these sentences say 3; the base's figure is re-read (not shown).
9. Public surfaces: in the story's last commit, on its own tip, after merging `main`, the story
   changes every public surface its change makes false, each number from the `slot task check`
   log of that tip: the test counts of the repository and of `surf-script` (`README.md:11`,
   `:285-286`, `:661`, `docs/badges/make.sh:38`, `docs/badges/tests.svg:1,2,10`,
   `website/docs/status.md:31`, `website/docs/what-works.md:17-18`,
   `website/docs/how-it-is-built.md:105`, `AGENTS.md:367-369`, all at `f463e24`) and the gate
   blocks (`README.md:596-645`, `website/docs/how-it-is-built.md:40-89`) hold that log's lines.
   The old strings are read on the story's base with `git grep -n` before the first commit and
   listed in the report; after the commit `git grep -n -F -e '<old>' -- README.md website/docs
   docs/badges AGENTS.md` prints nothing for each, except a string that is also a line of the
   new log, which the report names. At `f463e24` the figures are 2,397 tests (`README.md:285`)
   and 1845 tests with 3 ignored (`README.md:626`); the figures of the base are not shown.
10. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s
    (`.config/nextest.toml`); no model call and no network request in a test (cargo fetches
    swell before the tests); every build and test runs through `slot`, with none of `--local`,
    `--here`, `SLOT_NO_REMOTE` or `SLOT_SHIM_OFF`; a job in the pool ends within 10 minutes or
    names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes (the first cold
    build of swell `0.12.0` through the cargo shim may name it), and a job that timed out is not
    started again unchanged; a script that measures or waits starts without the `slot`
    prefix; no Python in anything that stays in the repository; no generator with a committed
    output is added; a test over 5 s is named with its time. The new test of decision 6 and the
    rewritten test are timed before they are kept, and each time is in the report.
11. The report lists what no test of this story holds at `0.12.0`: the branch `Err(RunEnded)`
    of `Runner::exception_text` (`runner.rs:893`, `:961`), the question before the read of a
    thrown value (`:888`), and the flag halves of the questions before and after the
    checkpoint (`:783`, `:822`): they decide only for a flag set after swell's last read, which
    no deterministic test makes. It also lists that the new test was run red at `0.10.0`
    (decision 7) or, if it was not, that the story stopped; which of the tests of Context were
    red at `0.12.0` before any edit; that the realm's growth was or was not found to turn a
    memory test red; and that the measurement of the copies (`docs/design/scripts.md:1214-1244`)
    was read, not run, on `0.12.0` (decision 10).
12. The public-numbers step is the story's last: no commit after it changes a file that a
    count or a gate block depends on, and its tip is the one that `slot task check` is run on.

## Out of scope

`overrides` and `is_html_dda` on `HostObjectSpec`, which swell's design lists and
`0.12.0` does not have (Context). The later story needs the list of surf's literals, at
`f463e24`, each naming `prototype`, `brand`, `data` and `error_kind`: 13 literals,
`crates/script/src/bindings.rs:351`, `:663`, `:1141`, `:1331`, `:1444`, `:1524`;
`crates/webidl/src/prelude.rs:284`, `:593`, `:1049`, `:1228`, `:1312`, `:1377` (the
generated copy of the six of `bindings.rs`); `crates/script/tests/engine.rs:263`.
The mentions of the type that change if swell makes it generic: the import
(`bindings.rs:80`, `prelude.rs:15`, `engine.rs:24`) and the return type of `window_spec`
(`bindings.rs:1523`, `prelude.rs:1376`). swell's `main` after `0.12.0` (the `#![no_std]` crates)
is not taken. `Math.random` and the six requirements `pending slice 4`; the shell binary
`swell` and its release assets; the stale doc comments of the generated code that
`story:scripts-utf16-length` (draft) rewrites (the cite `object/builtin_function.rs:497 @ 0.8.0`
in `bindings.rs:443` and `prelude.rs:372`); the second member form of the interrupt (a setter, a
conversion or an iterator in a test slice, Question 2);
deleting the flag halves of the runner's questions (Question 1); a provided time zone or a
clock hook other than UTC and the system clock.

## Placement

After `story:swell-0-9-0` is on `main`: this story's base is `main` after it, and its
decisions 3, 6 and 10 edit what that story wrote (the tag strings, the two constants, the
runner's doc comments and the tests of the checkpoint). `story:swell-0-9-0` is `active` in the
store; nothing in this story starts before it merges.

Stories not implemented that have a manifest or `Cargo.lock` in scope:
`story:scripts-dom-reading` (active: `Cargo.lock`, `crates/script/Cargo.toml`),
`story:scripts-document-port` (proposed: `Cargo.lock`) and `story:scripts-html-conformance`
(proposed: `Cargo.lock`). The reading story adds `surf-url` right after `surf-secret` in
`crates/script/Cargo.toml` and in the entry of `surf-script` in `Cargo.lock`; this story changes
line 16 and the swell entries, so the second merge may conflict in those two files and keeps both
changes. Whichever of the two merges second merges `main` first and runs `slot task check` on
the merged tree.

Other stories in work that scope a file this story edits: `story:scripts-dom-reading` and
`story:scripts-dom-mutation` (proposed) and `story:scripts-parse-run` (proposed) scope
`crates/script/src/runner.rs`; `story:scripts-parse-run` also `crates/script/tests/engine.rs`
and the public surfaces; `story:scripts-document-port`, `story:scripts-dom-attributes`,
`story:scripts-dom-selectors`, `story:scripts-dom-mutation`, `story:scripts-dom-reading` and
`story:scripts-parse-run` scope `docs/design/scripts.md`; `story:conformance-lock-and-lints`
(draft), `story:agent-tasks-left-process-wait`, `story:test-step-holds-with-tls` and
`story:tests-without-wall-time-bounds` (active) scope `AGENTS.md`;
`story:agent-cookie-public-suffixes` (active) scopes `NOTICE`; `story:scripts-dom-reading`,
`story:scripts-parse-run`, `story:scripts-document-port`, `story:scripts-html-conformance` and
`story:conformance-lock-and-lints` scope public surfaces. `story:tests-without-wall-time-bounds`
scopes `crates/script/tests/adversary_4.rs` and `adversary_5.rs`; this story reads
`adversary_4.rs:221` and `adversary_5.rs:98` and edits neither unless decision 8 applies. Each
overlap is settled as the story before it did: the second merge merges `main` first, keeps both
sets of edits, and takes the counts from its own log.

`story:scripts-seeds-and-time-zone` (draft, depends on `story:swell-0-9-0`) un-ignores
`seeds_fixed` (`HOST-26`) and `time_zone` (`HOST-127`) on "the first tag whose requirements
list has no `pending slice 4` line for HOST-26 and HOST-127". `0.12.0` is not that tag:
`HOST-26` is still `pending slice 4` (`conformance/host/requirements.txt:26 @ 0.12.0`), while
`HOST-127` is `test`. Decision 9 takes `time_zone` here (Question 4 puts the fork). After this
story that draft has `seeds_fixed` left: its first Change item, its acceptance 1 (an empty
`#[ignore` grep) and its title name two tests and are not rewritten here. `story:scripts-parse-run`
measures Q4 on the tag the pin story pins; it reads the tag this story leaves, `0.12.0`, and its
text is not rewritten here.

## Why `epic:scripts-in-a-page`

The epic runs page scripts on a swell tag that holds the security review's fixes and on an
engine whose behaviour surf's notes can state. `0.12.0` is the newest tag. It moves the interrupt
to every host function, which the wall-clock bound and the parser limits rely on and which
surf's tests had modelled as a limit of the engine (a getter's flag left set), and it gives pages
`Date`, `Math` and `Number` that the note still calls absent. Pinning it now keeps the note
true and the tests honest before the DOM stories that regenerate `bindings.rs` add more
host functions that can set the flag.
