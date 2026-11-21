---
format: aep.planning-md/3
id: story:runner-record-session
kind: story
status: active
title: 'Record a session: actions, changes, exchanges, secrets as references'
summary: The session process writes every action, its changed and detailed lines and every exchange, passwords as variables.
relations:
- decomposes: epic:e2e-scenario-runner
- depends_on: story:runner-record-option
- depends_on: story:runner-replay-transport
- depends_on: story:agent-change-and-log
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 5}
---
## Context

With `--record` the session process has an open recording holding its header
(`story:runner-record-option`). This story fills it as the session runs: every action and what it
changed, and every exchange, so that `surf scenario from` and a replayed `surf run` have all they
need. No detailed lines are written: before scripts a document's tree does not change, and the
detailed mode belongs to `epic:detailed-mode` (`story:runner-decisions`, D12, D13). A recording
holds page content and what the agent typed, so password values are written as variable
references (D6).

**Which actions are written.** One `action` block per row of the action log
(`agent-page-path-spec`), `open` and `close` included (D9): `open <url>` is the two rows `open`
and `go`, as the log has them. `surf scenario from` later makes no step of `open` and `close`.

**Where a typed value comes from.** The action log never stores a value the agent typed
(`agent-page-path-spec`: "An action never stores a value the agent typed"). So the recording does
not read it from the log: the session core hands the recording hook the input of the agent
command it is about to issue (`surf.agent.Fill` and its value, as the client sent it over the
wire; for `fill <handle> -` that is what `surf-agent` read from standard input). The hook writes
it into the `action` block, as a reference when the control's kind is `password`; the log row is
written as `agent-page-path-spec` says, without it.

Stands on `epic:agent-cli-target` (through the edge to `agent-change-and-log`): the action log
and the point in `crates/session` where each answer is produced; `follow`, `back`, `forward`,
`fill` (with `-`), `pick`, `press`; for the element an action names, its handle, kind and
accessible name as the `now` line shows them; and the transport the session core takes as a
parameter. On `epic:html-parser` (through `html-document-port`): the element's node and
ancestors in the tree read through `Browser::document_tree`, for its tree path.

## Change

- `crates/session`: with a recording open, the transport is wrapped in the `RecordingTransport`
  of `crates/scenario` (`runner-replay-transport`), told the sequence number of the current
  action. After each action an `action` block is appended: command, handle and locator (kind,
  accessible name, tree path) of the element it acted on, its input, the answer without its head
  and token lines as `surf_answer::render_blocks` writes it, and one `at <handle> <tree path>`
  line per handle of that answer's `now` block, all through the writer of `surf-scenario` in the
  grammar `runner-formats` gives (`docs/formats/surf-recording.md`). When one command adds two
  action rows (`open <url>`: `open`, then `go`), the answer is written once, in the block of the
  last of them. Reads (`look`, `log`) are not actions and are not written. The block is
  flushed before the answer is sent, so a recording of a session that is killed holds every
  answered action.
- A value typed into a `password` control is registered as a secret: the action writes
  `${PASSWORD_<n>}`, numbered by first appearance, and the transport writes the reference in
  bodies. The value is never written to the file.
- No token is written (the `token` line of `open`'s answer is left out with the head line); the
  session identity only in the header (D7).
- `crates/session/tests/`, against a server the test starts on `127.0.0.1` with a home page
  linking to a login form, the form posting to a path the server answers with 303 to a landing
  page:
  1. `recording_of_a_login_session`: `open --record <file> <home>`, `follow` the login link,
     `fill` the user name, `fill <handle> -` the password `s3cret &pass=1` on standard input,
     `press` the button, `close`. The recording parses with `surf_scenario`, holds the action
     blocks `open`, `go`, `follow`, `fill`, `fill`, `press`, `close` in that order, one exchange
     per request with its `during` number, the `POST` exchange with the landing page as its
     response address, and `${PASSWORD_1}`; its bytes contain neither `s3cret &pass=1` nor
     `s3cret+%26pass%3D1`, nor the token `open` printed, nor a line starting `surf 1 session`.
  2. `killed_session_leaves_its_answered_actions`: the session process killed right after the
     answer to `follow` leaves a recording that parses and holds the action blocks `open`, `go`
     and `follow`.

Surfaces: `crates/session/`, possibly `crates/scenario/src/` for a writer helper (after
`runner-replay-transport` there). Follows `runner-record-option` on `crates/session`.

## Acceptance

`cargo test -p surf-session` passes with `recording_of_a_login_session`, whose recording holds
the seven action blocks in order and `${PASSWORD_1}` and contains neither `s3cret &pass=1` nor
`s3cret+%26pass%3D1` in its bytes, nor the token `open` printed, nor a line starting
`surf 1 session`, and with `killed_session_leaves_its_answered_actions`.

## Out of scope

The scenario (`runner-scenario-from`), running (`runner-run`), detailed lines and `--detail`
(`epic:detailed-mode`).

## Domain relations

- Recording to action, one to one with the action log: rests on `surf.agent.Session → Action`,
  which `agent-page-path-spec` writes as a `relations:` entry (`owns`, `many`); no `ess/1`
  document on `main` holds it yet.
- Exchange to `surf.net.Request`, one to one: inferable (inferred from
  `crates/surf/src/lib.rs:473-481`; see `runner-replay-transport`).
- Variable to password value: `story:runner-decisions`, D6.
