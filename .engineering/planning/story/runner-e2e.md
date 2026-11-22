---
format: aep.planning-md/3
id: story:runner-e2e
kind: story
status: active
title: Record, convert and run a login session end to end in the gate
summary: The epic's acceptance case as a test task check runs, and the documentation of the runner.
relations:
- decomposes: epic:e2e-scenario-runner
- depends_on: story:runner-record-session
- depends_on: story:runner-run
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 4}
---
## Context

The epic is accepted on one end-to-end case: an agent explores a page with a form through
`surf-agent` with `--record`; `surf scenario from` turns the recording into a scenario file;
`surf run` on that file passes twice in a row with the same output, with the server stopped;
with the recorded response of the form's page changed so that the form's button has another
label, `surf run` fails, names the step, and prints the expected and the actual line (epic,
Acceptance). Each piece is tested on its own by `runner-record-session`, `runner-scenario-from`
and `runner-run`; this story owns the case itself as a test in the gate, and the sentences that
say the runner exists.

Stands on `epic:agent-cli-target` (through `agent-change-and-log`): the `surf-agent` binary able
to `open`, `follow`, `fill` (with `-`), `press` and `close` against a server a test starts.

## Change

- `crates/surf-cli/tests/record_and_run.rs`, the test `record_and_run`: it starts a server on
  `127.0.0.1` with `std::net::TcpListener` serving a home page with a link to a login page that
  holds a text control, a password control and a button labelled "Sign in", the form posting to a
  path the server answers with 303 to a landing page. It drives the `surf-agent` binary, found
  beside `env!("CARGO_BIN_EXE_surf")` (a missing binary fails the test naming the path; `task
  test` runs `cargo test --workspace`, which builds every member's binaries before it runs a
  test, to be confirmed by this story), with `open --record <file> <home>`, `follow`, `fill`,
  `fill <handle> -` with the password on standard input, `press`, `close`; runs `surf scenario
  from`; stops the server and checks that a connect to its port is refused; runs `surf run` twice
  with `SURF_VAR_PASSWORD_1` set and asserts exit 0 both times and byte-identical output, whose
  last step passed at the landing page's address; replaces "Sign in" by "Log in" in the recorded
  body of the login page only; runs `surf run` again and asserts exit 1, a `fail` line naming the
  `press` step, an `expected` line containing `button "Sign in"` and an `actual` line containing
  `button` and `"Log in"`.

Surfaces: `crates/surf-cli/tests/record_and_run.rs`, `crates/surf-cli/tests/main.rs` (one `mod` line). Follows `runner-run` on
`crates/surf-cli/tests/`.

## For AGENTS.md and README.md

- `AGENTS.md`, "Implementation targets", after the paragraph on `epic:e2e-scenario-runner`: "Since
  it, surf records an agent session (`surf-agent open --record`), turns it into a scenario (`surf
  scenario from`) and runs it in one process with the network replayed (`surf run`), for pages
  that work without scripts, over plain HTTP. The test `record_and_run` in `crates/surf-cli` is
  the case the epic was accepted on."
- `README.md`, section "Record and run a scenario", opening sentence: "Send an agent over a site
  with `surf-agent open --record`, and keep what it did as a test that runs without the agent and
  without the site."

## Acceptance

`slot task check` exits 0, and its `.out/test.log` shows `record_and_run` passed: two replayed
runs with the server stopped printed identical output and exited 0, and the run after the button
was relabelled exited 1 with a `fail` line for the `press` step, an `expected` line containing
`"Sign in"` and an `actual` line containing `"Log in"`.

## Out of scope

HTTPS, scripts, detailed lines, a speed target, the Chrome DevTools Protocol (epic, Out of
scope).
