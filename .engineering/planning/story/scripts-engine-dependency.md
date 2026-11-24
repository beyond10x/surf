---
format: aep.planning-md/3
id: story:scripts-engine-dependency
kind: story
status: proposed
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
  path: website/docs/principles.md
revision: 19
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:12Z", actor: "human:operator", revision: 19}
---
## Context

The epic's third candidate (lines 421-428) and "The dependency rule" (lines 145-187): surf depends
on swell by a git dependency on a release tag (S1, option a), `lock-check` allows that one source
by its exact address, tag and commit, and a new crate `surf-script` runs a swell agent with surf's
limits. No DOM and no page yet.

The tree on `main` of 2026-11-24, which wins where the epic differs:

- `lock-check` already holds an allow-list: `sh taskfiles/lock-check.sh Cargo.lock
  taskfiles/lock-allow.txt` (`Taskfile.yml:282-303`). The list (19 lines today, the packages of
  ring) has one line per package, `<name> <version> <source> <checksum>`
  (`taskfiles/lock-check.sh:7-9`), and no crate of this repository but `surf-tls` may depend on a
  listed package (`:28-30`, `:157`, `:160`). Its tests are `crates/surf/tests/lock_check.rs`
  (410 lines; `a_git_package_fails_naming_it` at line 168,
  `another_crate_depending_on_ring_fails_naming_it` at line 248). The epic (lines 149-151, 166-169)
  describes the check before ring; this story extends the list and its rule, it does not write a
  new check.
- `AGENTS.md` "Goals" (`AGENTS.md:27-43`) names ring as the one exception and "the 19 packages of
  ring on `taskfiles/lock-allow.txt`"; that is the sentence that gains swell.
- The root `Cargo.toml:3` has `members = ["crates/*", ...]`: `crates/script` joins with no line
  changed. The root manifest has no `[profile]` table; `.cargo/config.toml` holds only dev and
  test settings (`debug = false`, opt-levels).
- `NOTICE` exists and every release archive carries it (`.github/workflows/release.yml:207`,
  `notices=(README.md LICENSE NOTICE)`).
- swell `0.8.0` (2026-11-24, commit `0c1827128b57845e203713e87a0b0949fc8d63be`, the newest
  release): its root `Cargo.lock` lists 8 packages (`git show 0.8.0:Cargo.lock | grep -c
  '^\[\[package\]\]'` prints 8), none with a `source` line; `swell` depends by
  path on `swell-compiler`, `swell-num`, `swell-regexp`, `swell-syntax` and `swell-unicode`
  (`crates/swell/Cargo.toml:13-17 @ 0.8.0`), six packages for a host where the epic counts five;
  its root `Cargo.toml` sets `overflow-checks = true` in the profiles dev, release, test and bench
  (`Cargo.toml:10-20 @ 0.8.0`); its `NOTICE` reads "swell / Copyright 2026 beyond10x"
  (`NOTICE:1-2 @ 0.8.0`). An entry ends `Normal`, `Throw` or
  `Terminated(StepBudget | MemoryBudget | Interrupted | EngineFault)`
  (`crates/swell/src/host/outcome.rs:12-28 @ 0.8.0`); a call past `frame_depth` throws a
  `RangeError` with the message `frame_depth exceeded`
  (`crates/swell/src/interp/budget.rs:45 @ 0.8.0`); a parse error is `Parsed::Errors` with
  `SyntaxError` objects (`crates/swell/src/host/outcome.rs:32-36 @ 0.8.0`). `let` and `const` are
  in the language at the tag (`CHANGELOG.md:409-410 @ 0.8.0`,
  `crates/swell/tests/lexical_declarations.rs:1-3 @ 0.8.0`),
  and a script's top-level `let` binding stays in the global record, so the same `let` script run
  twice in one agent throws a `SyntaxError` (HOST-156,
  `crates/swell/tests/lexical_declarations.rs:38 @ 0.8.0`); the scripts below that run twice keep
  their bindings in a block. The host hooks the note decides in Q4 and Q6:
  `host_ensure_can_compile_strings` (`crates/swell/src/host/hooks.rs:176 @ 0.8.0`), the clock
  `host_system_utc_epoch_nanoseconds` (`crates/swell/src/host/hooks.rs:308 @ 0.8.0`), the time
  zone `system_time_zone_identifier` (`crates/swell/src/host/hooks.rs:310-312 @ 0.8.0`, "UTC" by
  default) and the seeds `Seeds { hash_key, random_seed }` given to `Agent::new`
  (`crates/swell/src/host/limits.rs:100-106 @ 0.8.0`); HOST-125 to HOST-128 are `pending slice 4`
  (`conformance/host/requirements.txt:125-128 @ 0.8.0`). Read with `git show 0.8.0:<path>` in a
  clone of `https://github.com/beyond10x/swell`.

## Waits for the note

`story:scripts-decisions` settles, before this story starts:

- waits for the note: Q13, the tag pinned, its commit and its package list;
- waits for the note: Q15, the allow-list line of a package from git (the lock may carry no
  `checksum` for it), the rule naming which crate may depend on which listed package, and the form
  of the closed list of crates that may depend on `surf-script`;
- waits for the note: Q14, that `surf-script` alone depends on swell and that the closed list is
  empty when this story lands;
- waits for the note: Q4 and Q5, surf's starting limits, whether `eval` and `Function` are
  refused, and the stack size the runner's thread gets, measured in surf's profiles;
- waits for the note: Q6, the clock, the seeds and the time zone, each with what it answers in a
  session and in a test;
- waits for the note: Q17, the citation for profiles read from the root workspace only, and for
  section 4(d) of Apache-2.0.

## Decisions taken

1. The tag is the note's (Q13). `0.8.0` may enter for tests without a page (epic lines 176-178);
   a page's script runs only on a tag that holds the review's fixes, which
   `story:scripts-document-port` moves to.
2. The allow-list gains the swell packages of that tag, each by name, version and the source
   `git+https://github.com/beyond10x/swell?tag=<tag>#<commit>`, in the line form Q15 settles;
   every other source still fails, and so does a swell package of another address, tag or commit.
   The rule "only `surf-tls`" becomes one rule per listed package naming the crate that may depend
   on it: ring's packages `surf-tls`, swell's `surf-script`. A listed package may still depend on
   another listed package.
3. The lock rule also holds a closed list of the crates of this repository that may depend on
   `surf-script` (S7: only a session process runs scripts). The list is empty when this story
   lands; `story:scripts-html-conformance` adds `surf-html-conformance` and
   `story:scripts-document-port` adds `surf-session`, each in its own change. Any other crate
   depending on `surf-script`, directly, fails the check naming that crate; so `surf`, `surf-agent`
   and `surfd` link no swell.
4. `[profile.release.package.<name>] overflow-checks = true` in the root `Cargo.toml` for each
   swell package (cargo reads profiles from the root workspace only; Q17). The dev and test
   profiles keep cargo's default, which checks overflow. `lock-check` reads the root manifest and
   fails naming a swell package of the lock that has no such entry.
5. `surf-script`'s first public surface, no DOM: limits (surf's starting values from Q4), a runner
   that creates one agent and runs on a thread of its own with the stack of Q5,
   `evaluate(source) -> ScriptEnd`, totals per runner (steps summed from `last_entry_steps()`,
   scripts run), a wall-clock bound through swell's `InterruptHandle` set by a timer thread, and
   `ScriptEnd`: ran, threw (kind, message, line and column when the tag has
   `Agent::last_entry_error_site`), refused by swell (its `SyntaxError` message), ended by a limit
   (which), not run (why). After `Terminated(MemoryBudget)` it calls `collect()` once; a second one
   in the same runner runs no further script (epic line 840).
6. Tests run in one binary, `crates/script/tests/main.rs`, module `engine`. Each test makes its
   own runner (a small realm, measured; the time is in the report); nothing heavier is built per
   test.
7. Requirement 6 (epic lines 341-343): the HOST-17, HOST-152 and HOST-163 cases run in surf's
   test profile in the gate; the engine shows its own release profile, and surf tests its own
   profile (epic line 824). A run of the same three in surf's release profile is recorded in the
   report, outside the gate.
8. The host hooks the note decides, each in the runner's `Host`: the clock answers a value that
   advances between two calls (in a test, by a fixed step per call), the seeds come from Q6's
   source in a session and are fixed in a test, the time zone is Q6's, and
   `host_ensure_can_compile_strings` refuses `eval` and `Function` where Q4 says so. A hook whose
   HOST id the tag lists as pending keeps its case, `#[ignore = "HOST-<id>"]`, named in the
   report.
9. The end words are the note's: Q4's column `outcome when reached` and Q7's closed list of ends.
   This story invents none; where the note spells a word otherwise than this text, the note's
   spelling holds.

## Change

- `Cargo.toml` (root): the release profile settings of decision 4.
- `crates/script/Cargo.toml` (new; package `surf-script`, `publish = false`; `swell = { git =
  "https://github.com/beyond10x/swell", tag = "<tag>" }`, no feature turned on, neither
  `collect-every-safe-point` nor `fault-injection`; `autotests = false` and one `[[test]]` `main`),
  `crates/script/src/lib.rs` (`#![forbid(unsafe_code)]`) and `crates/script/src/runner.rs`:
  decisions 5, 8 and 9.
- `crates/script/tests/main.rs` and `crates/script/tests/engine.rs`: decision 6 and 7.
- `Cargo.lock`: the swell packages with their `source` lines, and `surf-script`.
- `taskfiles/lock-allow.txt`, `taskfiles/lock-check.sh` (its head states the new line form, the
  rule per package, the closed list of `surf-script` dependents and the profile check),
  `crates/surf/tests/lock_check.rs`: decisions 2 to 4 and their tests.
- `Taskfile.yml`: the `desc` and `summary` of `lock-check` (lines 283-301) name swell,
  `surf-script` and the closed list beside ring and `surf-tls`.
- `AGENTS.md`, "Goals": the sentence of the exception names swell, "one crate of the
  organization", and the root lock's proof counts its packages.
- `NOTICE`: swell's notice, its address and tag, Apache-2.0 (section 4(d), Q17).
- In the last commit, the public surfaces of acceptance 9.

## Acceptance

1. `slot task lock-check` prints `Cargo.lock: <n> package(s), <m> from outside this repository,
   each on the allow-list`, with `<m>` 19 plus the number of swell packages of the tag (six at
   `0.8.0`).
2. `crates/surf/tests/lock_check.rs`, new cases, each failing with a message naming the package or
   crate: a swell package of another tag; of the same tag at another commit; from another address;
   a crate other than `surf-script` depending on `swell`; `surf-script` depending on `ring`;
   `surf` depending on `surf-script` while the closed list is empty (fails naming `surf`); a swell
   package of the lock without its `overflow-checks = true` entry in the root manifest (fails
   naming that package). Pass cases: a lock where a crate on a closed list written for the test
   depends on `surf-script`; the root manifest with every entry. The ring cases there pass
   unchanged, and `the_unchanged_lock_passes` passes on the new lock.
3. `crates/script/tests/main.rs`, module `engine`, named cases, each with its script text:
   - `ran`: `1 + 1` ends `Ran`;
   - `threw`: `throw new Error("x")` ends `Threw` with kind `Error` and message `x`;
   - `refused`: `1 +;` ends `Refused` with kind `SyntaxError`, the message equal to a constant of
     the test that cites that message as `<path>:<line> @ <tag>`;
   - `step_limit`: `while (true) {}` ends `limit: steps-per-entry`, and the
     runner then runs `1` to `Ran`;
   - `memory_limit`: under a memory limit of 1 MiB and a step limit large enough not to end it
     first, `{ const a = []; for (;;) a.push(new Array(1000)); }` ends `limit: memory`; the runner
     calls `collect()` once and runs `1` to `Ran`; the same script again ends `limit: memory`, and
     a further `1` ends `not run: memory`;
   - `frame_depth`: `function f(n) { return 1 + f(n + 1); } f(0)` ends `Threw` with kind
     `RangeError` and message `frame_depth exceeded` (the constant cites
     `crates/swell/src/interp/budget.rs:45 @ 0.8.0`);
   - `wall_clock`: under a wall-clock bound of 100 ms, `while (true) {}` with a step limit that
     does not end it first ends `limit: wall-clock`, the elapsed time under 10 s;
   - `totals`: the runner's step total and script count rise with each script;
   - `clock_advances`: `var a = Date.now(); var b = Date.now(); b - a` answers the test clock's
     step in milliseconds;
   - `seeds_fixed`: `Math.random()` answers the same number in two runners made with the test
     seeds, and another number in a runner made with other seeds;
   - `time_zone`: `new Date(0).getTimezoneOffset()` answers the offset of Q6's time zone (0 for
     UTC);
   - `compile_strings`: `eval("1")` and `Function("return 1")()` each end as Q4 says: `Threw`
     with kind `EvalError` where Q4 refuses them, `Ran` with the value 1 where it does not;
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
5. `grep -c swell NOTICE` prints at least 1; `grep -n swell AGENTS.md` shows the "Goals"
   sentence.
6. `slot task check` exits 0.
7. `git diff --stat` names only the files of the Change.
8. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s
   (`.config/nextest.toml:5`); no model call and no network request in a test (cargo fetches swell
   before the tests, as it fetches ring); nothing expensive is made per test (a runner per test is
   a realm of a few objects, its time measured and reported); no Python; no generator with a
   committed output is added; a test over 5 s is named with its time. A build or test job in the
   pool ends within 10 minutes or names its time (`SLOT_TIMEOUT` for the first cold build of swell
   through the cargo shim, at most 30 minutes).
9. Public surfaces: the story changes, in its last commit and after merging `main`, every
   public surface its change makes false, each number taken from the `slot task check` log of
   its tip. After that commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges`
   prints nothing for each old string below, as the story's base holds it (the strings and lines
   given are those of `main` at `29945f5`, 2026-11-24); each new number equals the one in that
   log; and the gate block, `README.md:584-625` and `website/docs/how-it-is-built.md:40-81`,
   holds that log's lines. The old strings of this story: the lock and its exception,
   `19 of its 49 packages` (`README.md:248`, `website/docs/principles.md:20`), `One exception`
   (`README.md:249`, `website/docs/principles.md:21`), `50 package(s), 19 from outside`
   (`README.md:599`, `website/docs/how-it-is-built.md:55`), `the 19 outside packages`
   (`README.md:655`, `website/docs/how-it-is-built.md:109`) and `5: ring 0.17.14 and 4 it needs`
   (`docs/badges/make.sh:39`, `docs/badges/dependencies.svg`); the test count
   `1976 passed` (`README.md:11`, `docs/badges/make.sh:38`, `docs/badges/tests.svg`),
   `1,976` and `1,424, and 552` (`README.md:285`, `website/docs/status.md:23`,
   `website/docs/what-works.md:17`).

## Out of scope

The DOM and the bindings (`story:webidl-bindings`, which adds `surf-html` to
`crates/script/Cargo.toml`), a page's scripts (`story:scripts-parse-run`,
`story:scripts-document-port`), the tag with the review's fixes, and any change in swell.

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
