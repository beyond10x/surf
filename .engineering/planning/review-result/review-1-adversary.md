---
format: aep.planning-md/3
id: review-result:review-1-adversary
kind: review-result
status: active
title: Adversary, review 1, waves 1 to 4 of epic headless-rust-target
relations:
- reviews: story:net-fetch-failure-reasons
- reviews: story:generated-rust-workspace
- reviews: story:http-fetch-library
- reviews: story:memory-ports
- reviews: story:headless-cli
revision: 2
---
unit: review-1 (adversary), tree `surf-r1-adv` at `3aaa3aa` plus 4 untracked test files
verdict: red
cases: executed 60→75, red 3
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: 1 directory, `$HOME/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/waves/review-1/adversary/` (contents in part 6)
needs-coordinator: no

**1. What I changed**

`git --no-pager diff --stat` is empty: no tracked file changed. `git status --short`:
```
?? crates/fetch/tests/review1_fetch.rs
?? crates/surf/tests/review1_cli.rs
?? crates/surf/tests/review1_generated_check.rs
?? crates/surf/tests/review1_library.rs
```
All four are test files. Nothing is committed. The lease `adv-r1` is taken and released.

**Findings**

| id | severity | origin | file:line | what was measured | what reaches it | test |
|---|---|---|---|---|---|---|
| F1 | warning | introduced | `Taskfile.yml:88` | `generated-check` lists only regular files (`find … -type f`), so it never sees a symbolic link. With `generated/rust/crates/net/build.rs` added as a link to a file outside, the check exits 0 and prints `generated/rust matches spec/ (27 artifact(s) and .ess-output)`. In that same copy, `cargo build -p net --locked` printed `warning: net@1.0.0: hand-written build script ran`. A regular `build.rs` at the same path is caught. | Committing a symbolic link (git stores links). The check stays green even though its own summary says "any other file in generated/rust" fails, and AGENTS.md says only `task generate` changes that directory. A linked directory under `generated/rust/crates/` would match the workspace member glob the same way; I inferred that and did not run it. | `review1_a_symlinked_build_script_in_a_generated_crate_fails_naming_it` (red) |
| F2 | note | introduced | `crates/fetch/src/lib.rs:616` | `max - body.len() + 1` overflows when `max_body_bytes == usize::MAX` and no body byte arrived with the head. The test build panics with `attempt to add with overflow`. The crate documentation (`lib.rs:14`) says "no input makes this crate panic". In a release build the value wraps, which makes the read 1 byte at a time. | Nothing in the repository: `Limits` is public, but the binary uses the default 16 MiB. Verdict INFEASIBLE for that reason. Fix: `max.saturating_sub(body.len()).saturating_add(1)`. | `review1_body_limit_at_usize_max_reads_a_body_ended_by_close_without_panicking` (red) |
| F3 | note | introduced | `crates/surf/src/main.rs:46` | When standard output is `/dev/full`, a Committed load prints `surf: standard output: No space left on device (os error 28)` on standard error and still exits 0. Every row is lost. | `surf <url>` with output to a full disk or a closed pipe. README.md's exit-status table does not cover this case. | `review1_rows_that_cannot_be_written_do_not_exit_0` (red) |
| F4 | note | introduced | `crates/surf/tests/library.rs:111` | In a scratch copy I disabled the `Pending` check at `crates/surf/src/lib.rs:472` (`if !pending && false`). All 6 tests in `library.rs` stayed green, `each_started_request_is_handed_to_the_transport_once` included. My two cases went red (`review1_library.rs:65` and `:85`). | Two paths in the public API rely on that check: `Browser::new` over a system whose request is already answered, and two `Fetch` commands for one request issued through `net_mut()`. The code is correct; the unit's suite would not notice if it broke. | `review1_a_system_whose_request_was_answered_is_not_fetched_again_by_a_new_browser`, `review1_two_fetches_of_one_request_reach_the_transport_once` (both green now, both red on the mutant) |

**2. The cases I added**

Each file was run alone, before the suite.

- `crates/fetch/tests/review1_fetch.rs`
  - A header section exactly at `max_header_bytes` is a response, and 1 byte under the limit is MalformedResponse. Green.
  - A body exactly at the limit is a response, by length, by close and chunked. Green.
  - A body 1 byte past the limit is MalformedResponse, by length, by close and chunked. Green.
  - Media type for edge spellings: trailing space, tab, upper case, `;charset`, quoted `;`, trailing `;`, a `0xff` byte, `text /html`, an empty value. Green.
  - `usize::MAX` body limit. Red. My first version of this case sent `abcd` and was green, because the body arrived in the same read as the head. I changed it to an empty body, which reaches the overflow. Red output, run alone:
    ```
    thread '…usize_max…' panicked at crates/fetch/src/lib.rs:616:22:
    attempt to add with overflow
    test result: FAILED. 0 passed; 1 failed; … 4 filtered out
    ```
- `crates/surf/tests/review1_library.rs`: the two transport-once paths and `escape` over `\0 \t \r \n \x1b \x7f \u{85} \u{9f} \\`. All 3 green.
- `crates/surf/tests/review1_cli.rs`
  - `TEXT/HTML ;Charset=UTF-8 ` (trailing space) gives exit 0 and one `document` row. Green.
  - Two arguments, or one argument that is not UTF-8, give exit 2, nothing on standard output and the usage line on standard error. Green.
  - Output to `/dev/full`. Red:
    ```
    assertion `left != right` failed: stderr "surf: standard output: No space left on device (os error 28)\n"
      left: Some(0)
    ```
- `crates/surf/tests/review1_generated_check.rs`. Each case copies `Taskfile.yml`, `spec/` and `generated/rust` into `target/tmp` and runs `task generated-check` there.
  - The unchanged copy passes. Green.
  - One byte flipped in each committed file under `generated/rust`, one at a time, `state.json` included: the check fails and names the file every time. This is story `generated-rust-workspace`'s Acceptance tested directly. Green.
  - A regular `build.rs` in a generated crate is caught. Green.
  - A linked `build.rs` is not caught. Red:
    ```
    assertion `left != right` failed: generated/rust matches spec/ (27 artifact(s) and .ess-output)
      left: Some(0)
    ```

**3. Suite run, after the cases existed**

- `slot task test`: exit 201. cargo stops at the first failing test binary, `review1_cli`.
- `RUSTFLAGS='-D warnings' cargo test --workspace --locked --no-fail-fast`: exit 101, `error: 3 targets failed: review1_cli, review1_generated_check, review1_fetch`.
  - 30 test binaries: 72 passed, 3 failed, 0 ignored, so 75 executed.
  - Without my 4 files: 26 test binaries and 60 tests. That matches the gate line on wave 4's page and the counts in AGENTS.md (36 fetch, 12 ports, 12 surf).

**4. Judgement findings**

None beyond F1 to F4. They cover the tree above.

**5. Attacked and found sound**

- `generated-check`:
  - An extra file in `.ess-output` and a changed byte in `state.json` both fail: ess refuses the record.
  - A generated file replaced by a link is reported `missing`.
  - An executable bit on a generated file, and a regular file named `generated/rust/target`, pass the check, but neither changes the build.
- `task generate` over a hand-edited file refuses and names the file, as its summary says.
- `crates/surf`:
  - The exit status follows `navigation_state()`.
  - Only `main.rs:40` writes to standard output, and only rows. No generated crate prints.
  - Every failure detail on standard error is a static string, a value quoted with `{:?}`, or printable ASCII.
  - Exit 1 means Failed whenever `surf-fetch` is the transport, because `Unanswered` cannot come from it.
- `crates/ports`:
  - `list` order after delete and re-put is asserted in `exercise`.
  - The identity text form is checked against literals.
  - `OsRandom` uses `read_exact`, so a short read is retried and end of file becomes an unmet obligation.
  - The `compile_fail` test fails for the stated reason (missing `NavigationStorage`).
  - No ports type holds another component's tables.
- `crates/fetch`:
  - Status codes outside 100 to 599 are rejected, and 1xx responses are skipped.
  - Media type matches `surf.net.MediaType` for every spelling I tried, so the binding `== "text/html"` fires for a real server's `Text/HTML; charset=…`.
  - Header and body limits hold exactly at the bound.
- The acceptance statements of `http-fetch-library`, `memory-ports` and `headless-cli` hold: the cases are present and named, and `cargo tree` shows no outside crate.

Not covered: concurrency (the binary is single-threaded), and the parser under hostile input (the security reviewer's lane).

**6. Paths written outside the worktree**

All under `$HOME/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/waves/review-1/adversary/`:
- Tree copies: `gc-base/`, `gc-symlink-build/` and `mut-guard/` (these two hold a cargo `target/`), `gc-mode-bit/`, `gc-ess-output-extra/`, `gc-state-byte/`, `gc-target-file/`, `gc-symlink-same/`, `gc-symlink-cargo-config/`, `gen-edit/`
- Script: `probe-generated-check.sh`
- Logs: `run-fetch.log`, `run-mut-guard.log`, `run-review1_cli.log`, `run-review1_generated_check.log`, `run-review1_library.log`, `suite.log`, `suite-nff.log`, `symlink-build-cargo.log`

Inside the worktree, `target/tmp/review1-generated-check/` holds three copies. No processes are left running.

```findings
- file: Taskfile.yml
  line: 88
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: generated-check lists only regular files, so a symlinked build.rs committed into a generated crate passes the gate while cargo runs it as that crate's build script.
- file: crates/fetch/src/lib.rs
  line: 616
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: max_body_bytes of usize::MAX overflows max - body.len() + 1 on an empty body read until close and panics in debug builds, contrary to the crate's no-panic claim; no caller in the repository sets it.
- file: crates/surf/src/main.rs
  line: 46
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: when standard output cannot be written the binary reports it on standard error and still exits 0 for a committed load, so a run whose rows were all lost reads as success.
- file: crates/surf/tests/library.rs
  line: 111
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the unit's transport-once test stays green with the Pending check in Browser::answer disabled, so the suite does not guard the property it names.
```
