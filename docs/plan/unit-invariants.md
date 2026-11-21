# Repository invariants for a unit of a wave

Copied unchanged into every unit brief. A unit that finds one of these wrong says so in its
report and does not work around it.

- Work only in the worktree the brief names. Build output goes to `target/` inside that
  worktree: never set `CARGO_TARGET_DIR`, never build in another tree.
- `cargo` in the shell is a shim that queues in the shared build pool. Prefix every other command
  that compiles or tests with `slot` (`slot task check`). `slot: job N queued` means waiting. Never
  pass `-j`, never set `CARGO_BUILD_JOBS`, never bypass the pool.
- No third-party crate, as a dependency or a development dependency, unless the brief names it. A
  `Cargo.toml` written by hand holds path dependencies on crates of this repository and nothing
  else. `Cargo.lock` then lists only crates of this repository.
- `generated/rust/` is never edited by hand. It changes only by `task generate`, and only in a
  unit whose brief assigns `spec/` or `generated/rust/`.
- Specs first: no behaviour that `spec/` does not state. A gap in the specification is reported,
  not filled in code.
- A component holds its own rows and reads no other component's. Hand-written code does not give
  one component's port another component's storage.
- Every hand-written crate starts with `#![forbid(unsafe_code)]` and builds with warnings denied.
- A test that needs a server starts its own on `127.0.0.1` port 0 and stops it. No test reaches
  the network outside the machine.
- Never write under `.engineering/`. No `aep plan artifact` write verb; reads are fine.
- No `git commit`, `git add`, `git stash`, branch or worktree command. Changes are left in the
  working tree; the coordinator commits.
- Nothing under `/tmp` or `/dev/shm`. Uncommitted and untracked files go to the scratch directory
  the brief assigns.
- The shell runs as `zsh -c … < /dev/null`: no heredoc into a command that reads standard input.
  Write a script with the file-writing tool and run it by path.
- No Python anywhere: not in the repository, not in a script, a probe or a manual check, not as
  a throwaway web server. A server for a manual run is written in Rust in the scratch directory,
  or the run uses `http://example.org/`.
- Anything that runs longer than about 30 seconds runs in the background. Every process a unit
  starts is stopped before it reports, checked with `kill -0`.
- The gate for a unit is package-scoped (`cargo test -p <package>`, or `slot task <name>` where the
  brief says so). The whole gate, `slot task check`, runs once on the integration branch.
- A report quotes each suite's own summary line. It starts with the six-line header the brief
  gives.
