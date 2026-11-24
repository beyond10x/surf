---
format: aep.planning-md/3
id: story:scripts-engine-dependency
kind: story
status: implemented
title: surf depends on swell by a release tag, lock-check allows exactly that source, and surf-script runs an agent under surf's limits
summary: A git dependency on the tag the note names; the allow-list, its rule per package, and a closed list of crates that may depend on surf-script; overflow checks enforced by lock-check; NOTICE and the Goals sentence; surf-script evaluates a source with no DOM under the step, memory, frame-depth and wall-clock limits, with the clock, seed, time-zone and compile-strings hooks of Q4 and Q6, HOST-17, 152 and 163 tested in surf's profile.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-decisions
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: NOTICE
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/script/Cargo.toml
- confidence: cited
  path: crates/script/src/lib.rs
- confidence: cited
  path: crates/script/src/runner.rs
- confidence: cited
  path: crates/script/tests/engine.rs
- confidence: cited
  path: crates/script/tests/main.rs
- confidence: cited
  path: crates/surf/tests/lock_check.rs
- confidence: cited
  path: taskfiles/lock-allow.txt
- confidence: cited
  path: taskfiles/lock-check.sh
- confidence: cited
  path: taskfiles/lock-owners.txt
- confidence: cited
  path: website/docs/principles.md
revision: 29
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:12Z", actor: "human:operator", revision: 19}
- {from: "proposed", to: "active", at: "2026-11-24T12:08:06Z", actor: "human:operator", revision: 27, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-24T15:42:28Z", actor: "human:operator", revision: 29, decided_on: {"recorded":{"test_result":2,"verification":1}}, executor: "agent:coordinator"}
---
## Context

The epic's third candidate (lines 421-428) and "The dependency rule" (lines 145-187): surf depends
on swell by a git dependency on a release tag (S1, option a), `lock-check` allows that one source
by its exact address, tag and commit, and a new crate `surf-script` runs a swell agent with surf's
limits. No DOM and no page yet.
The tree on `main` of 2026-11-24, which wins where the epic differs:
- `lock-check` already holds an allow-list:
  `sh taskfiles/lock-check.sh Cargo.lock taskfiles/lock-allow.txt` (`Taskfile.yml:282-303`). The
  list (19 lines today, the packages of ring) has one line per package,
  `<name> <version> <source> <checksum>` (`taskfiles/lock-check.sh:7-9`), and no crate of this
  repository but `surf-tls` may depend on a listed package (`:28-30`, `:157`, `:160`). Its tests
  are `crates/surf/tests/lock_check.rs` (410 lines; `a_git_package_fails_naming_it` at line 168,
  `another_crate_depending_on_ring_fails_naming_it` at line 248). The epic (lines 149-151, 166-169)
  describes the check before ring; this story extends the list and its rule, it does not write a
  new check.
- `AGENTS.md` "Goals" (`AGENTS.md:27-43`) names ring as the one exception and "the 19 packages of
  ring on `taskfiles/lock-allow.txt`"; that is the sentence that gains swell.
- The root `Cargo.toml:3` has `members = ["crates/*", ...]`: `crates/script` joins with no line
  changed. The root manifest has no `[profile]` table; `.cargo/config.toml` holds only dev and test
  settings (`debug = false`, opt-levels).
- `NOTICE` exists and every release archive carries it (`.github/workflows/release.yml:207`,
  `notices=(README.md LICENSE NOTICE)`).
- swell `0.8.0` (2026-11-24, commit `0c1827128b57845e203713e87a0b0949fc8d63be`, the newest
  release): its root `Cargo.lock` lists 8 packages
  (`git show 0.8.0:Cargo.lock | grep -c '^\[\[package\]\]'` prints 8), none with a `source` line;
  `swell` depends by path on `swell-compiler`, `swell-num`, `swell-regexp`, `swell-syntax` and
  `swell-unicode` (`crates/swell/Cargo.toml:13-17 @ 0.8.0`), six packages for a host where the epic
  counts five; its root `Cargo.toml` sets `overflow-checks = true` in the profiles dev, release,
  test and bench (`Cargo.toml:10-20 @ 0.8.0`); its `NOTICE` reads "swell / Copyright 2026
  beyond10x" (`NOTICE:1-2 @ 0.8.0`). An entry ends `Normal`, `Throw` or
  `Terminated(StepBudget | MemoryBudget | Interrupted | EngineFault)`
  (`crates/swell/src/host/outcome.rs:12-28 @ 0.8.0`); a call past `frame_depth` throws a
  `RangeError` with the message `frame_depth exceeded`
  (`crates/swell/src/interp/budget.rs:45 @ 0.8.0`); a parse error is `Parsed::Errors` with
  `SyntaxError` objects (`crates/swell/src/host/outcome.rs:32-36 @ 0.8.0`). `let` and `const` are
  in the language at the tag (`CHANGELOG.md:409-410 @ 0.8.0`,
  `crates/swell/tests/lexical_declarations.rs:1-3 @ 0.8.0`), and a script's top-level `let` binding
  stays in the global record, so the same `let` script run twice in one agent throws a
  `SyntaxError` (HOST-156, `crates/swell/tests/lexical_declarations.rs:38 @ 0.8.0`); the scripts
  below that run twice keep their bindings in a block. The host hooks `docs/design/scripts.md`
  decides in Q4 and Q6: `host_ensure_can_compile_strings`
  (`crates/swell/src/host/hooks.rs:176 @ 0.8.0`), the clock `host_system_utc_epoch_nanoseconds`
  (`crates/swell/src/host/hooks.rs:308 @ 0.8.0`), the time zone `system_time_zone_identifier`
  (`crates/swell/src/host/hooks.rs:312-314 @ 0.8.0`, "UTC" by default) and
  `get_named_time_zone_offset_nanoseconds` (`crates/swell/src/host/hooks.rs:318-324 @ 0.8.0`, 0 by
  default) and the seeds `Seeds { hash_key, random_seed }` given to `Agent::new`
  (`crates/swell/src/host/limits.rs:100-106 @ 0.8.0`); HOST-125 to HOST-128 are `pending slice 4`,
  and `Date` and all of `Math` but `max` do not exist at the tag (`CHANGELOG.md:112-113 @ 0.8.0`,
  Q6) (`conformance/host/requirements.txt:125-128 @ 0.8.0`). Read with `git show 0.8.0:<path>` in a
  clone of `https://github.com/beyond10x/swell`.

## Decisions taken

1. The tag is `0.8.0`, commit `0c1827128b57845e203713e87a0b0949fc8d63be` (`docs/design/scripts.md`,
   Q13; decided by the coordinating agent on 2026-11-24, reversible). Every story before
   `story:scripts-document-port` builds and tests on it, its sessions under the 4 GiB backstop of
   Q11. `story:scripts-document-port` waits for the swell tag that holds requirement 11
   (`docs/design/scripts.md`, Q18) and takes that tag in its first commit; until then the
   address-space backstop is the only bound on the copies swell does not charge (Q13).
2. The allow-list gains the six packages a host takes from that tag, `swell`, `swell-compiler`,
   `swell-num`, `swell-regexp`, `swell-syntax` and `swell-unicode`
   (`crates/swell/Cargo.toml:13-17 @ 0.8.0`; `swell-test262` and `swell-unicode-gen` are tooling),
   one line each in today's four fields with `none` in the place of the checksum, the word the
   check already prints for a package without one (Q15; a lock cargo writes for a package from git
   has no `checksum` line):
   `swell 0.8.0 git+https://github.com/beyond10x/swell?tag=0.8.0#0c1827128b57845e203713e87a0b0949fc8d63be none`,
   and the same for the other five. A package of another address, tag or commit, or one with a
   checksum where the list says `none`, fails as today, and so does every other source.
3. The rule of who may depend on what becomes a second file, `taskfiles/lock-owners.txt` (Q15): one
   line `<package> <crate>` per crate of this repository that may name the package among its
   dependencies, sorted, no comments; this story writes `ring surf-tls` and `swell surf-script`,
   and no line for `surf-script`, since nobody may depend on it when this story lands (Q14). A
   crate of this repository fails when it depends on a package of `lock-allow.txt` or on
   `surf-script` without a line of its own, with
   `Cargo.lock: <crate> depends on <package>; lock-owners.txt does not allow it`; a listed package
   may still depend on another listed package (ring's on theirs, `swell` on its five).
   `story:scripts-html-conformance` adds `surf-script surf-html-conformance` and
   `story:scripts-document-port` adds `surf-script surf-session`, each in its own change: the
   closed list of Q14. So `surf`, `surf-agent` and `surfd` link no swell (S7).
4. `[profile.release.package.<name>] overflow-checks = true` in the root `Cargo.toml` for each of
   the six swell packages: cargo reads profiles from the root workspace only ("Cargo only looks at
   the profile settings in the Cargo.toml manifest at the root of the workspace", The Cargo Book,
   "Profiles"; Q17), and swell's own `overflow-checks = true` (`Cargo.toml:10-20 @ 0.8.0`) is not
   read by surf's build. The dev and test profiles keep cargo's default, which checks overflow.
   `lock-check` reads the root manifest and fails naming a swell package of the lock that has no
   such entry.
5. `surf-script`'s first public surface, no DOM: limits with Q4's session values (`steps_per_entry`
   10,000,000; `memory_bytes` 67,108,864; `frame_depth` 10,000; `reentry_depth` 256;
   `parser_nesting` 1,000; `string_length` 16,777,216; `bigint_bits` 1,048,576; per runner a step
   total of 50,000,000, 1,000 scripts, 524,288 bytes of source per script and a wall clock of 5 s
   from the first script's start); a runner that creates one agent and runs on a thread of its own
   whose stack is `min_stack_bytes()` (4,006,656 bytes at those limits) plus Q5's margin,
   (`reentry_depth` + 1) × 14,336 + 1,048,576 = 4,732,928 bytes, 8,739,584 bytes in all, which
   `story:scripts-parse-run` measures; `evaluate(source) -> ScriptEnd`; totals per runner (steps
   summed from `last_entry_steps()`, scripts run); the wall-clock bound through swell's
   `InterruptHandle` set by a timer thread; and `ScriptEnd` with Q7's closed list: `Ran`, `Threw`
   (kind one of the seven `ErrorKind` variants or `value`, message, line, column and `dynamic`,
   Q16; a wildcard arm maps a kind surf does not know yet to `Error`), `Refused` (the parse
   answered `Parsed::Errors` and `refusals()` rose), `Limit` and `NotRun` with their reason words,
   and `Faulted` (`Terminated(EngineFault)`; each later script `not run: faulted`). After
   `Terminated(MemoryBudget)` the runner calls `collect()` once; a second `limit: memory` in the
   same runner makes each later script `not run: memory` (Q4).
6. Tests run in one binary, `crates/script/tests/main.rs`, module `engine`. Each test makes its own
   runner (a small realm, measured; the time is in the report) with limits of its own, as Q4 says a
   test does; nothing heavier is built per test.
7. Requirement 6 (epic lines 341-343): the HOST-17, HOST-152 and HOST-163 cases run in surf's test
   profile in the gate; the engine shows its own release profile, and surf tests its own profile
   (epic line 824). A run of the same three in surf's release profile is recorded in the report,
   outside the gate.
8. The host hooks of Q6, each in the runner's `Host`: the clock answers, in a session, the
   nanoseconds since the Unix epoch of `SystemTime::now()`, and in a test a fixed start that
   advances by 1,000,000 ns on each call; the time zone is the provided default, `"UTC"`, with the
   offset 0, in both; the seeds are 48 bytes drawn per agent through `surf_secret::fill_random`
   (`crates/secret/src/lib.rs:32`) in a session, and swell's test values `hash_key` `[7; 16]` and
   `random_seed` `[11; 32]` in a test; `host_ensure_can_compile_strings` keeps its default and
   answers `Ok`, so `eval` and `Function` are allowed (Q4). `crates/script` gains the dependency
   `surf-secret` by path. The clock, the time zone and `random_seed` are `pending slice 4` at
   `0.8.0` (HOST-26, HOST-27, HOST-125 to HOST-128, HOST-144, HOST-147), and no script reaches them
   before a tag with slice 4: each hook is tested on the `Host` from Rust, and each case that
   reaches it from a script keeps its case with `#[ignore = "HOST-<id>"]`, named in the report.
9. The end words are Q7's closed list, with the words of Q1's line `reasons:` and of Q4's column
   `outcome when reached`; this story uses `ran`, `threw`, `refused`, `limit: steps-per-script`,
   `limit: memory`, `limit: wall-clock`, `faulted`, `not run: memory` and `not run: faulted`, and
   invents none.

## Change

- `Cargo.toml` (root): the release profile settings of decision 4.
- `crates/script/Cargo.toml` (new; package `surf-script`, `publish = false`; the dependency line
  swell's changelog gives, `swell = { git = "https://github.com/beyond10x/swell", tag = "0.8.0" }`
  (`CHANGELOG.md:135-137 @ 0.8.0`), no feature turned on, neither `collect-every-safe-point` nor
  `fault-injection`; `surf-secret` by path; `autotests = false` and one `[[test]]` `main`),
  `crates/script/src/lib.rs` (`#![forbid(unsafe_code)]`) and `crates/script/src/runner.rs`:
  decisions 5, 8 and 9.
- `crates/script/tests/main.rs` and `crates/script/tests/engine.rs`: decisions 6 and 7.
- `Cargo.lock`: the six swell packages with their `source` lines, and `surf-script`.
- `taskfiles/lock-allow.txt` (the six lines of decision 2), `taskfiles/lock-owners.txt` (new,
  decision 3), `taskfiles/lock-check.sh` (its head states the line form with `none`, the owners
  file and its rule, and the profile check), `crates/surf/tests/lock_check.rs`: decisions 2 to 4
  and their tests.
- `Taskfile.yml`: the `desc` of `lock-check` (`Taskfile.yml:283`) becomes Q15's: "Fail, naming each
  package, when the root Cargo.lock holds one from outside this repository that is not on
  taskfiles/lock-allow.txt, or a crate of this repository depends on a listed package or on
  surf-script without a line of taskfiles/lock-owners.txt."; its `summary` (lines 284-301) names
  swell, `surf-script` and the owners file beside ring and `surf-tls`.
- `AGENTS.md`, "Goals" (`AGENTS.md:30-43`): in place of "One exception: ..." and the last item,
  Q15's sentence: "Two exceptions. Audited cryptographic primitives come from ring 0.17.14, used by
  `surf-tls` alone; and one crate of the organization, the script engine swell, by a git dependency
  on a release tag, used by `surf-script` alone. `taskfiles/lock-allow.txt` names each of their
  packages by name, version, source and checksum (`none` for a package from git), and
  `taskfiles/lock-owners.txt` names the crates of this repository that may depend on each
  (`task lock-check`)."
- `NOTICE`: swell's notice ("swell / Copyright 2026 beyond10x", `NOTICE:1-2 @ 0.8.0`), its address
  and tag, Apache-2.0 (section 4(d), Q17).
- In the last commit, the public surfaces of acceptance 9.

## Acceptance

1. `slot task lock-check` prints
   `Cargo.lock: <n> package(s), 25 from outside this repository, each on the allow-list` (the 19 of
   ring and the six of swell `0.8.0`).
2. `crates/surf/tests/lock_check.rs`, new cases, each failing with a message naming the package or
   crate: a swell package of another tag; of the same tag at another commit; from another address;
   a swell line of the list with a checksum where the lock has none; a crate other than
   `surf-script` depending on `swell` and `surf` depending on `surf-script`, each with
   `Cargo.lock: <crate> depends on <package>; lock-owners.txt does not allow it`; `surf-script`
   depending on `ring`; a swell package of the lock without its `overflow-checks = true` entry in
   the root manifest (fails naming that package). Pass cases: a lock where a crate given a line
   `surf-script <crate>` in a `lock-owners.txt` written for the test depends on `surf-script`; the
   root manifest with every entry. The ring cases there pass unchanged, and
   `the_unchanged_lock_passes` passes on the new lock.
3. `crates/script/tests/main.rs`, module `engine`, named cases, each with its script text and
   limits of the test's own:
   - `ran`: `1 + 1` ends `ran`;
   - `threw`: `throw new Error("x")` ends `threw Error "x" 1:1` (kind, message, line and column as
     Q16 reads them); `throw 1` ends `threw` with the kind `value` and no message;
   - `threw_syntax`: `1 +;` ends `threw SyntaxError` with its message, line and column (a parse
     error that is not a refusal, Q7);
   - `refused`: `class A {}` ends `refused` with the message equal to a constant of the test that
     cites that message as `<path>:<line> @ 0.8.0` (classes are refused with a `SyntaxError` that
     names their delivery slice, `CHANGELOG.md:104-106 @ 0.8.0`);
   - `step_limit`: `while (true) {}` under a step limit of 1,000,000 ends
     `limit: steps-per-script`, and the runner then runs `1` to `ran`;
   - `memory_limit`: under a memory limit of 1 MiB and a step limit large enough not to end it
     first, `{ const a = []; for (;;) a.push(new Array(1000)); }` ends `limit: memory`, the runner
     calls `collect()` once, and `1` then ends `ran`; in a second runner the same script twice in a
     row ends `limit: memory` twice, and a further `1` ends `not run: memory`;
   - `frame_depth`: `function f(n) { return 1 + f(n + 1); } f(0)` ends
     `threw RangeError "frame_depth exceeded"` (the constant cites
     `crates/swell/src/interp/budget.rs:45 @ 0.8.0`);
   - `wall_clock`: under a wall-clock bound of 100 ms, `while (true) {}` with a step limit that
     does not end it first ends `limit: wall-clock`, the elapsed time under 10 s;
   - `totals`: the runner's step total and script count rise with each script;
   - `stack_size`: the runner's thread, with Q4's session limits, is made with 8,739,584 bytes,
     `min_stack_bytes()` plus the margin of decision 5;
   - `host_hooks`: from Rust, the test `Host`'s clock answers two values 1,000,000 ns apart, its
     time zone `"UTC"` with the offset 0, and its seeds `[7; 16]` and `[11; 32]`; the session
     `Host`'s clock answers a value within one second of `SystemTime::now()` and two agents get
     different seeds;
   - `clock_advances`, `seeds_fixed`, `time_zone`: the same from a script (`Date.now()`,
     `Math.random()`, `new Date(0).getTimezoneOffset()`), each `#[ignore = "HOST-<id>"]` with the
     id of Q6, since `Date` and `Math.random` do not exist at `0.8.0`;
   - `compile_strings`:
     `if (eval("1") !== 1 || Function("return 1")() !== 1) throw new Error("compile")` ends `ran`
     (Q4 allows both);
   - `host_17_reentry_at_full_nesting`, `host_152_eval_at_full_nesting`,
     `host_163_call_at_full_nesting`: the three cases of swell's `crates/swell/tests` at the tag
     (`host_17_deepest_reentry_parses_at_full_nesting`,
     `crates/swell/tests/limits_scripts.rs:126 @ 0.8.0`;
     `host_152_deepest_reentry_evals_at_full_nesting`,
     `crates/swell/tests/eval_stack.rs:173 @ 0.8.0`;
     `host_163_deepest_reentry_calls_function_with_parameters_and_body_at_full_nesting`,
     `crates/swell/tests/eval_stack.rs:201 @ 0.8.0`), run with
     surf's limits on the runner's thread with the stack of Q5, each ending without a crash.
   Each test under 5 s.
4. Recorded in the report, outside the gate: the three cases of acceptance 3's last item pass in
   surf's release profile (`slot cargo nextest run --release -p surf-script engine::host_`), and
   `slot cargo build --release -p surf-script -v` shows `-C overflow-checks=on` on the rustc line
   of each swell package.
5. `grep -c swell NOTICE` prints at least 1;
   `git grep -n -F lock-owners.txt -- AGENTS.md Taskfile.yml` shows the "Goals" sentence and the
   `desc` of `lock-check`, each as Q15 gives it.
6. `slot task check` exits 0.
7. `git diff --stat` names only the files of the Change.
8. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s
   (`.config/nextest.toml:5`); no model call and no network request in a test (cargo fetches swell
   before the tests, as it fetches ring); nothing expensive is made per test (a runner per test is
   a realm of a few objects, its time measured and reported); no Python; no generator with a
   committed output is added; a test over 5 s is named with its time. A build or test job in the
   pool ends within 10 minutes or names its time (`SLOT_TIMEOUT` for the first cold build of swell
   through the cargo shim, at most 30 minutes).
9. Public surfaces: the story changes, in its last commit and after merging `main`, every public
   surface its change makes false, each number taken from the `slot task check` log of its tip.
   After that commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges` prints
   nothing for each old string below, as the story's base holds it (the strings and lines given are
   those of `main` at `29945f5`, 2026-11-24); each new number equals the one in that log; and the
   gate block, `README.md:584-625` and `website/docs/how-it-is-built.md:40-81`, holds that log's
   lines. The old strings of this story: the lock and its exception, `19 of its 49 packages`
   (`README.md:248`, `website/docs/principles.md:20`), `One exception` (`README.md:249`,
   `website/docs/principles.md:21`), `50 package(s), 19 from outside` (`README.md:599`,
   `website/docs/how-it-is-built.md:55`), `the 19 outside packages` (`README.md:655`,
   `website/docs/how-it-is-built.md:109`) and `5: ring 0.17.14 and 4 it needs`
   (`docs/badges/make.sh:39`, `docs/badges/dependencies.svg`); the test count `1976 passed`
   (`README.md:11`, `docs/badges/make.sh:38`, `docs/badges/tests.svg`), `1,976` and
   `1,424, and 552` (`README.md:285`, `website/docs/status.md:23`,
   `website/docs/what-works.md:17`).

## Out of scope

The DOM and the bindings (`story:webidl-bindings`, which adds `surf-html` to
`crates/script/Cargo.toml`), a page's scripts (`story:scripts-parse-run`,
`story:scripts-document-port`), the swell tag that holds requirement 11 (`docs/design/scripts.md`,
Q18), which `story:scripts-document-port` takes, and any change in swell.

## Placement

After `story:scripts-decisions`. Beside `story:webidl-reader` (both write `Cargo.lock` and the
public surfaces: the second to merge takes the other's lock entries, and the coordinating agent
orders the two gate runs), `story:scripts-spec`, `story:session-process-hardening` and
`story:html-script-hook`. `taskfiles/lock-check.sh` is also written by the draft
`story:conformance-lock-and-lints`, and `Taskfile.yml` by the active
`story:test-step-holds-with-tls` and `story:agent-cookie-public-suffixes`: never in the same
wave.
`website/docs/principles.md` is also written by `story:scripts-spec`, which may run beside this
story: this story rewrites principle 4 (lines 18-25 at `29945f5`), that story the scenario count
of line 33; neither changes the other's lines, and the second to merge keeps the first's.

## Why `epic:scripts-in-a-page`

S1's form of the dependency and the first code that runs swell inside surf, as the epic's third
candidate.

## As built

The unit is on `main` at `f513b99` (the merge of `impl/scripts-engine-dependency` at `254a744`,
which holds the reviewer's tests of three passes and one merge of `main`). The crate
`crates/script` (package `surf-script`) runs 29 tests, 3 of them ignored; `surf` runs 158
(118 before), 56 of them on `lock-check`. `Cargo.lock` holds 58 packages, 25 from outside this
repository (51 and 19 before). Where it differs from the text above:

- The tag is written in one place, `crates/script/Cargo.toml`. A change of tag is followed by
  `Cargo.lock`, six lines of `taskfiles/lock-allow.txt`, the constant `SWELL` in
  `crates/surf/tests/lock_check.rs`, `NOTICE` and the citations `@ 0.8.0`.
- Acceptance 2 against decision 3: decision 3 holds.
- `NOTICE` keeps swell's attribution of the Unicode data (Apache-2.0, section 4(d)).
- `AGENTS.md`, "Goals": the story's sentence had dropped the reason for ring and the
  operator's words; the unit restored them, and the Layout lists name `crates/script`.
- `lock-check` grew beyond the story. It reads the tree's cargo config files: no multi-line
  string, no backslash, no `rustflags` in any letter case, no line with `overflow` but
  `overflow-checks = true`, no profile section for a swell package. The root manifest holds
  no multi-line string. `Taskfile.yml`, every file under `taskfiles/` but the check's own
  three, and every file under `.github/workflows/` hold no line that names the overflow
  checks in either spelling and no `rustflags` line with an `@`. A package of the lock
  without a source whose name is on the allow-list fails. A duplicate line of either list
  fails, and so does an owners line no dependency uses.
- Where that check ends, by a ruling: it holds a key or a flag written in those files. It
  does not hold a setting reached through another file, a `rustc` or wrapper named in a
  config file, an ancestor's config file or the environment of whoever starts the build.
  Review of a change to those files holds the rest. The sentence "cargo reads profiles from
  the root manifest only" was wrong (a cargo config file overrides the root manifest) and is
  corrected in `Taskfile.yml` and in the note's Q17.
- A script started after the wall-clock deadline ends `not run: wall-clock`, is not started
  and not counted. The first script's start is the clock's start.
- `surf-script` names every variant of swell's error types, with no catch-all arm, so a tag
  that adds a variant stops the build where surf must choose the word. The host errors that
  are defects of surf's own calls stay `faulted`.
- The per-runner totals are held and counted in this unit and enforced by
  `story:scripts-parse-run`.
- `agent_and_realm()` is for the bindings' setup before the first script; an entry run
  through it is outside the totals and the clock.
- Three tests are ignored at swell `0.8.0` (`clock_advances`, `seeds_fixed`, `time_zone`):
  `Date` and `Math.random` wait for swell's slice 4. Two claims have no test yet: a fault
  stays set, and a test runner hands its seeds to the agent.
- The test of the clock thread holds that no thread remains after 100 runners, read again
  for at most 2 s. No test holds the join itself: a thread that ends by itself leaves the
  thread count as a joined one does.
- The dependencies badge stays at 5: swell is the organization's own, and no binary ships
  `surf-script` yet.
