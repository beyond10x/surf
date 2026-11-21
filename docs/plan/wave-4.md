# Wave 4 of 5: `surf <url>` runs the page load

One wave of the [wave plan](waves.md). N is one.

## Unit

| | |
|---|---|
| story | `story:headless-cli` |
| objective | epic `headless-rust-target`: one command loads a page over plain HTTP through the five generated components and prints what the page load left behind |
| scope | cited: `crates/surf`, `README.md`, `AGENTS.md` |
| acceptance | `cargo test -p surf` passes with the seven numbered cases, each under a name that says its case, and `cargo tree -p surf --edges normal` lists only crates of this repository |
| move to `implemented` needs | no required evidence; `test_result` and `verification` are recorded |

## The check

1. Implementable in this tree: yes. Standard library only; the tests start their own servers on
   `127.0.0.1`.
2. Surfaces that overlap: none inside the wave (one unit). Because it is alone, the root
   `Cargo.lock` is assigned to the unit in this wave.
3. Blast radius: one new crate and two project files.

Dependencies, all `implemented`: `story:net-fetch-failure-reasons` (wave 1),
`story:generated-rust-workspace` (wave 2), `story:http-fetch-library` and `story:memory-ports`
(wave 3, gated at `aa52c47`). `aep plan artifact blocked` on 2026-11-20: "nothing is blocked".

## Pre-flight numbers (2026-11-20)

- free disk on `/`: 60 G (`df -h /`); 95 G, 86 G and 67 G at the three earlier pre-flights. The
  build pool evicts below 40 G. This plan's trees held under 100 M of build output each.
- the whole gate on a tree with no build directory: under 10 s of work once it has a build slot;
  a wait of 134 s for a slot was observed in wave 3
- model budget: not asked; N is 1

## After this wave: the first adversary pass

By the operator's decision the adversary attacks once here, over everything merged in waves 1 to
4, before wave 5 starts. Its page is [review-1.md](review-1.md), written when it runs.

## Commits this wave makes

The opening store commit (this page), one unit commit, the merge of the unit into the wave
branch, the closing store commit, and the merge into `main`. Corrections that answer the
adversary pass are committed on a branch of their own and recorded on the review page.

## Record

Kept current while the wave runs.

| | |
|---|---|
| wave branch | `wave/4-headless-cli` |
| unit branch | `impl/headless-cli` |
| integration tree | `~/.local/state/worktree/trees/b10x/surf/surf-w4-int` (managed id `surf-w4-int`, lease `coord-b84c1d1b-86a1-4880-9b86-d11fa8d6a813`) |
| unit tree | `~/.local/state/worktree/trees/b10x/surf/surf-w4-cli` (managed id `surf-w4-cli`) |
| build directory | `target/` inside each tree |
| scratch | `~/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/waves/wave-4/headless-cli/` |
| stage | delivered: unit `dd32b9c`, merge `8867e2a`, gate exit 0 |

## Result

- Unit: green on the first return. Claim VERIFIED by the coordinator in the unit tree: `cargo
  test -p surf` exit 0 with 6 + 6 tests and a test for each of cases 1 to 7; `cargo tree -p surf
  --edges normal` lists only crates of this repository; `surf ftp://example.org/x` printed a
  `Failed` navigation, a `Failed` request and `fetch-failed … UnsupportedScheme`, exit 1; `surf`
  with no argument printed a usage line, exit 2. The implementor ran it against a local server:
  `navigation … Committed`, `request … Responded`, one `document` row, exit 0.
- Gate, once, on merge `8867e2a`: `slot task check` exit 0. Five steps, none skipped: `surf v1 —
  8 file(s), 2 scenario(s), valid`; `50 scenario(s) (2 authored), 10 refusal(s)`; `50 scenarios:
  50 passed, 0 failed, 0 error, 0 unsupported`; `generated/rust matches spec/ (27 artifact(s) and
  .ess-output)`; cargo build `Finished`; `26 test suites: 60 passed, 0 failed, 0 ignored`.
  Executed tests: 48 before the wave, 60 after.
- Exit status 3 also covers one case the story does not name: a command refusing what the run
  itself just created (`OpenTab` refusing the window `OpenWindow` opened).
- Named by the implementor as a small race: test case 3 binds port 0, closes it and then runs
  the binary; another process could take the port in between. 11 runs passed.
- Free disk when the unit returned: 92 G. It had been 46 to 52 G during the wave. Measured, not
  explained: another task of this session (the tree for beyond10x/ess#792) held a 58 G build
  directory, and it still held it when free disk was back at 92 G, so other sessions freed the
  space. (Corrected in review 1: this line first named that directory as the cause, which
  nothing measured.)
- Cost: one `aep:implementor` on Opus, 152,483 tokens, 66 tool uses, 1,080 s.
- Filed to the backlog: none.
