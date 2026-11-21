---
format: aep.planning-md/3
id: epic:e2e-scenario-runner
kind: epic
status: active
title: surf as an end-to-end scenario runner fed by agent recordings
summary: An agent's session through the command line is recorded, turned into a scenario, and run by surf deterministically with no agent.
relations:
- depends_on: epic:agent-cli-target
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 7}
---
## Context

The operator, 2026-11-21 (quoted in full in `AGENTS.md`, "Implementation targets"): "one purpose of
surf would also be to have a super fast end-to-end scenario runner, like test browser." An
agent is sent over a site, explores, clicks, fills and submits, "and then basically from that
transcript just build a deterministically executable end-to-end test."

`docs/design/agent-interface.md`, accepted as the basis for the epics on 2026-11-21
(`approval-record:agent-design-accepted-2026-11-21`), section 7 and step 6 of section 9. This
epic is rewritten from that design. It stands on `epic:agent-cli-target` (the session, the
actions, the log, the structural change) and through it on `epic:html-parser`.

## Outcome

- **Recording.** `surf-agent open --record <file>` writes the session as it runs: each action,
  the `changed` lines of its answer, and each HTTP request and response of the session with the
  address the response was served from. The format reserves a block kind for detailed lines
  (`epic:detailed-mode`) and writes none: nothing mutates a document's tree before a script
  engine exists (`docs/plan/agent-plan-revision-1.md`, D2 and D3).
- **A scenario from a recording.** `surf scenario from <recording>` writes a scenario file a
  person or an agent can edit. Each step holds the action with a locator in place of a handle
  (kind, accessible name, and the element's path in the tree as a tiebreak) and the
  expectations after it: by default the page's address and status and the structural `changed`
  lines.
- **Running.** `surf run <scenario>` runs it in one process, with no socket and no session
  process, driving the same `agent` component. One line per step, pass or fail, and for a
  failure the expected and the actual lines. Exit 0 only when every step passed.
- **Deterministic.** The same scenario gives the same result every time: handles and locators
  are derived from the document, session identities and tokens are never compared, and the
  network is replayed from the recording by default (the transport is already a parameter of
  the `surf` library). A run can instead be live, against the site.
- **Secrets.** A value typed into a password control is written to a recording as a reference
  to a variable, supplied when the scenario runs.

## Open, to be settled by this epic's first story

- The scenario file's format, and whether it can be `ess-scenario/6` run by the ESS runner
  (design, section 10, question 2). Not explored.
- How a locator is resolved when two elements have the same kind and name.
- What a replayed run does with a request the recording does not hold.

## Out of scope

- The detailed mode: `--detail`, detailed lines in a recording, and asserting them in a
  scenario (`epic:detailed-mode`, after a script engine).
- Time and randomness under scripts: there is no script engine. Pages that need scripts are out
  of reach for the runner until it exists.
- HTTPS. Until it exists the runner reaches plain HTTP only, which in practice is a test server
  on the machine. The design names this as a prerequisite for use against a deployed site.
- The Chrome DevTools Protocol, stated by the operator as beside the point for now.
- A speed target. A number is set once there is something to measure.

## Constraints

- Specs first. No third-party crate in anything that ships. No Python anywhere.
- A recording holds page content and what an agent typed: it is written with a mode only the
  user can read.

## Acceptance

`slot task check` exits 0 on `main`, and in that run the test `record_and_run`
(`story:runner-e2e`) is reported passed in `.out/test.log`. That test shows: an agent explores a
page with a form through `surf-agent` with `--record`; `surf scenario from` turns the recording
into a scenario file; `surf run` on that file passes twice in a row with the same output, with
the server stopped (replayed network); with the recorded response of the form's page changed so
that the form's button has another label, `surf run` fails, names the step, and prints the
expected and the actual line.
