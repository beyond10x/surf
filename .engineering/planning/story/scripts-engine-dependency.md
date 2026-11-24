---
format: aep.planning-md/3
id: story:scripts-engine-dependency
kind: story
status: draft
title: surf depends on swell by a release tag, lock-check allows exactly that source, and surf-script runs an agent under surf's limits
summary: A git dependency on the tag the note names; the allow-list and its rule extended to swell's packages and surf-script; overflow checks, NOTICE and the Goals sentence; surf-script evaluates a source with no DOM under the step, memory and wall-clock limits.
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
  path: crates/script
- confidence: cited
  path: crates/surf/tests/lock_check.rs
- confidence: cited
  path: taskfiles/lock-allow.txt
- confidence: cited
  path: taskfiles/lock-check.sh
revision: 2
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
  release): its root `Cargo.lock` lists 8 packages, none with a `source` line; `swell` depends by
  path on `swell-compiler`, `swell-num`, `swell-regexp`, `swell-syntax` and `swell-unicode`
  (`crates/swell/Cargo.toml`), six packages for a host where the epic counts five; its root
  `Cargo.toml` sets `overflow-checks = true` in the profiles dev, release, test and bench; it has
  a `NOTICE` ("swell / Copyright 2026 beyond10x"). Read in a clone at the tag.

## Waits for the note

`story:scripts-decisions` must settle, before this story starts:

- waits for the note: Q13, the tag pinned, its commit and its package list;
- waits for the note: Q15, the allow-list line of a package from git (the lock may carry no
  `checksum` for it) and the rule naming which crate may depend on which listed package;
- waits for the note: Q14, that `surf-script` alone depends on swell (and who may depend on
  `surf-script`), so the lock rule can say it;
- waits for the note: Q4 and Q5, surf's default limits and the stack size the runner's thread
  gets, measured in surf's profiles;
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
3. `[profile.release.package.<name>] overflow-checks = true` in the root `Cargo.toml` for each
   swell package (cargo reads profiles from the root workspace only; Q17). The dev and test
   profiles keep cargo's default, which checks overflow.
4. `surf-script`'s first public surface, no DOM: limits (surf's defaults from Q4), a runner that
   creates one agent and runs on a thread of its own with the stack of Q5, `evaluate(source) ->
   ScriptEnd`, totals per runner (steps summed from `last_entry_steps()`, scripts run), a
   wall-clock bound through swell's `InterruptHandle` set by a timer thread, and `ScriptEnd`: ran,
   threw (kind, message, line and column when the tag has `Agent::last_entry_error_site`),
   refused by swell (its `SyntaxError` message), ended by a limit (which), not run (why). After
   `Terminated(MemoryBudget)` it calls `collect()` once; a second one in the same runner runs no
   further script (epic line 840).
5. Tests run in one binary, `crates/script/tests/main.rs`, module `engine`. Each test makes its
   own runner (a small realm, measured; the time is in the report); nothing heavier is built per
   test.

## Change

- `Cargo.toml` (root): the release profile settings of decision 3.
- `crates/script/` (new; package `surf-script`; `swell = { git =
  "https://github.com/beyond10x/swell", tag = "<tag>" }`, no feature turned on, neither
  `collect-every-safe-point` nor `fault-injection`; `#![forbid(unsafe_code)]`; one test binary):
  decision 4.
- `Cargo.lock`: the swell packages with their `source` lines, and `surf-script`.
- `taskfiles/lock-allow.txt`, `taskfiles/lock-check.sh` (its head states the new line form and
  rule), `crates/surf/tests/lock_check.rs`: decision 2 and its tests.
- `Taskfile.yml`: the `desc` and `summary` of `lock-check` (lines 283-301) name swell and
  `surf-script` beside ring and `surf-tls`.
- `AGENTS.md`, "Goals": the sentence of the exception names swell, "one crate of the
  organization", and the root lock's proof counts its packages.
- `NOTICE`: swell's notice, its address and tag, Apache-2.0 (section 4(d), Q17).

## Acceptance

`slot task check` exits 0. The folder's bounds hold: the test step ends within 120 s and no test
runs over 60 s (`.config/nextest.toml:5`); no model call and no network request in a test (cargo
fetches swell before the tests, as it fetches ring); nothing expensive is made per test (a runner
per test is a realm of a few objects, its time measured and reported); no Python; no generator with a committed output is added; a test over 5 s
is named with its time. A build or test job in the pool ends within 10 minutes or names its time
(`SLOT_TIMEOUT` for the first cold build of swell through the cargo shim, at most 30 minutes).

1. `slot task lock-check` prints `Cargo.lock: <n> package(s), <m> from outside this repository,
   each on the allow-list`, with `<m>` 19 plus the number of swell packages of the tag (six at
   `0.8.0`).
2. `crates/surf/tests/lock_check.rs`, new cases, each failing with a message naming the package:
   a swell package of another tag; of the same tag at another commit; from another address; a
   crate other than `surf-script` depending on `swell`; `surf-script` depending on `ring`. The
   ring cases there pass unchanged, and `the_unchanged_lock_passes` passes on the new lock.
3. `crates/script/tests/main.rs`, module `engine`, named cases: `1 + 1` ends `Ran`; `throw new
   Error("x")` ends `Threw` with kind `Error` and message `x`; a construct the tag refuses ends
   `Refused` with swell's message; `while (true) {}` ends with the step limit and the runner runs
   `1` afterwards; an endless allocation ends with the memory limit, the runner collects once and
   runs `1`, and a second memory end stops further scripts; a deep recursion ends with
   `frame_depth`; a script under a wall-clock bound of 100 ms that loops ends `Interrupted` within
   1 s; the runner's step total and script count rise per script. Each test under 5 s.
4. Outside the gate, recorded in the report: `slot cargo build --release -p surf-script -v`
   shows `-C overflow-checks=on` on the rustc line of each swell package.
5. `grep -c swell NOTICE` prints at least 1; `grep -n swell AGENTS.md` shows the "Goals"
   sentence.
6. `git diff --stat` names only the files of the Change.

## Out of scope

The DOM and the bindings (`story:webidl-bindings`), a page's scripts (`story:scripts-document-port`),
the tag with the review's fixes, and any change in swell.

## Placement

After `story:scripts-decisions`. Beside `story:webidl-reader` (both write `Cargo.lock`: the second
to merge takes the other's entries), `story:scripts-spec` and `story:session-process-hardening`.
`taskfiles/lock-check.sh` is also written by the draft `story:conformance-lock-and-lints`, and
`Taskfile.yml` by the active `story:test-step-holds-with-tls` and `story:agent-cookie-public-suffixes`:
never in the same wave.

## Why `epic:scripts-in-a-page`

S1's form of the dependency and the first code that runs swell inside surf, as the epic's third
candidate.
