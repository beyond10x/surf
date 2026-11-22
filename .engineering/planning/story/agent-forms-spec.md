---
format: aep.planning-md/3
id: story:agent-forms-spec
kind: story
status: active
title: Specify filling, picking and submitting a form, from the agent to the navigation
summary: surf.document gains form rows, form owners, options and commands that fill, pick and submit; surf.agent gains Fill, Pick and Press mapped to them; a submission navigates with POST or GET.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-fetch-post
- depends_on: story:html-document-spec
- depends_on: story:agent-links-spec
scope:
- confidence: cited
  path: conformance/src
- confidence: cited
  path: crates/ports
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: generated/rust
- confidence: cited
  path: spec/components.yaml
- confidence: cited
  path: spec/domains/agent.yaml
- confidence: cited
  path: spec/domains/document.yaml
- confidence: cited
  path: spec/ess-inputs.yaml
- confidence: cited
  path: spec/scenarios
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 5}
---
## Context

Step 4 of design section 9; design section 4: `fill` gives a control a value, `pick` changes a
select, checkbox or radio control, `press` activates a button (form submission before scripts).
These act on controls the `document` component owns, so they are its commands, and the agent
commands map to them by bindings (design section 3). A submission navigates the tab of the
document with the method and body of `agent-request-body-spec`.

What `epic:html-parser` leaves to this epic (`html-document-spec`, Out of scope: "form owners,
`select` options and anything the agent's `fill`, `pick` and `press` need"): forms, the form
owner and `name` of a control, the options of a select. Needs from `epic:html-parser`: the
document and element entities, the element identity type and the command shape of
`docs/design/html-parser.md` item 2 (`html-decisions`, `html-document-spec`); this story writes
`spec/domains/document.yaml` after `html-document-spec`.

Decision D11: a select stores no value. The stored fact is the selectedness of each option; what
an answer prints as the value of a select is derived from it. So the command for a select sets
selectedness only. Decision D2: control state (value, checkedness, selectedness) lives in these
rows, not in the tree, which nothing changes before scripts.

The entry list and its encoding are not computed in the model (ESS has no encoder): a submission
is a command that starts it and one that ends it, and the step between is `agent-form-port`.
Standards, from memory, to be confirmed: HTML Standard "form owner", "constructing the entry
list", "form submission algorithm", "radio button group", the `select` selectedness setting
algorithm; URL Standard "application/x-www-form-urlencoded". Only that encoding is taken on.

## Domain relations

- `surf.document.Document → surf.shell.Tab`: each document is shown in the tab of the navigation it
  was parsed for. Inferable: `spec/domains/navigation.yaml`, `surf.navigation.Navigation` relation
  `tab` (`references`, `one`, via `tab_id`), and the document-to-navigation link of
  `html-document-spec`. The document stores a copy of `tab_id`, mapped from `NavigationCommitted`,
  because the `document` component reads no navigator rows (`AGENTS.md`, "Rules").
- `surf.navigation.Navigation → surf.net.Request`: one to one (`spec/domains/navigation.yaml`,
  identity), carrying method and body since `agent-request-body-spec`.
- New, in no `ess/1` document; cardinality and ownership are the HTML Standard's, which
  `AGENTS.md` ("Goals") makes the source where a standard covers a behaviour; written here as
  `relations:` entries: form → document, many forms per document, owned by it; control → form
  owner, zero or one, `references`; option → select, one to many, owned by the select (or a list
  field of it if ESS takes that more simply; the story says which).

## Change

- `spec/domains/document.yaml`: the form entity (`action`, `method`, `enctype` as written, position
  in document order); on the element: `name` and the form owner; the options of a select (value,
  label, selectedness), and no stored value for a select; on the document: `tab_id`. Commands that
  set the value of a text-entry control, set the checkedness of a checkbox or radio (the others of
  its radio button group unchecked), set the selectedness of an option, and start a submission from
  a submitter; each refuses an element that does not exist, a control of another kind, a disabled
  control and a hidden control. The command that ends a submission carries the tab, the resolved
  action URL, the method, the body and its media type, or a reason it ended without one; its event
  invokes `surf.navigation.Navigate`.
- `spec/domains/agent.yaml`: `Fill(session_id, element, value, handle)`,
  `Pick(session_id, element, option, handle)`, `Press(session_id, element, handle)`, each with the
  refusals of `Go`, each recorded as an `Action` holding the handle and never the value.
- `spec/components.yaml`: bindings from the agent events to the document commands and from the
  submission event to `Navigate`; `parse-on-navigation-committed` maps `tab_id`.
- Authored scenarios, each named after its case: `agent-fill-sets-value.yaml`,
  `agent-pick-unchecks-radio-group.yaml`, `agent-pick-selects-option.yaml`,
  `document-refuses-disabled-control.yaml`, `document-refuses-hidden-control.yaml`,
  `agent-press-submits-post.yaml` (an ended submission invokes `Navigate` with `Post`, the body and
  `application/x-www-form-urlencoded`), `agent-press-submission-ends-without-body.yaml`.
- `task generate`; `crates/ports` stores the new rows; `conformance/src/`; `crates/surf/src/lib.rs`
  only where the regenerated interface forces it, with no submission step. `REFUSALS_MAX` is not written here (decision D30): the unit runs the tests of the packages it touched, `<n>` is the measured `refused:` count (`.out/synthesize.log` after `slot task suite REFUSALS_MAX=<n>`), and its report gives `<n>` and each new refusal with its cause; the coordinator writes the last `<n>` of the round on the wave branch and runs the gate once after the round's units, as `slot --timeout 25m task check-runner`.

Surfaces written: `spec/domains/document.yaml`, `spec/domains/agent.yaml`, `spec/components.yaml`,
`spec/scenarios/`, `spec/ess-inputs.yaml`, `generated/rust/`, `crates/ports/`,
`crates/surf/src/lib.rs` (only if forced), `conformance/src/`, `crates/session/src/lib.rs`
(`command_word` matches every `ActionCommand`).

Bounds for every command of this story. Every build and test runs through the build pool:
`cargo` is wrapped by the pool's shim, every other command that builds or tests gets the `slot`
prefix. The pool ends a job after 10 minutes; a command that needs longer names its time with
`slot --timeout <t>` or `SLOT_TIMEOUT=<t>`, at most 30 minutes; a `timeout` wrapped around the
command extends nothing. A script that measures or waits starts without the `slot` prefix; the
builds and test runs inside it carry it. No test runs over 60 s and the test step ends within
120 s. No Python.

## For AGENTS.md (handed over in the unit report, decision D7)

- "Components": the `document` row gains the form, option and control-state commands; the `agent`
  row gains `Fill`, `Pick`, `Press`; the page-load paragraph gains the submission binding.
- "The gate's numbers": the new totals and each new refusal with its cause.

## Acceptance

On the unit's branch after this story, `slot task conform REFUSALS_MAX=<n>` prints a scenario
total at least 20 above the total recorded before it with `0 failed, 0 error, 0 unsupported`, and
`.out/report.json` (the report `task conform` writes with `--report-out`) lists the seven
authored files of Change as passed; `slot task conform-rust REFUSALS_MAX=<n>`
and then `task conform-totals` show the same total on both runners; and `.out/synthesize.log`
holds `<n>` `refused:` lines, at most 12 above the count recorded before it, the unit report
giving `<n>` and each new refusal with its cause (decision D30). With `REFUSALS_MAX: <n>`
committed in `Taskfile.yml` by the coordinator, `<n>` this story's, the last of the round, the
gate exits 0, run once by the coordinator after the round's units as
`slot --timeout 25m task check-runner`.

## Out of scope

Filling the rows from a page and computing the entry list (`agent-form-port`); the session commands
(`agent-fill-pick`, `agent-press-submit`); `multipart/form-data`, `text/plain`; file upload; a
button that does anything without a form (scripts).

## Found before this story's wave

From wave 9 (`docs/plan/agent-wave-9.md`), for this story's own reading before its wave.

- After a `Post` that a 301, 302 or 303 turned into a `GET`, the `Requests` row and the
  events of the request still say `Post` with the form's media type. The comment of
  `spec/domains/net.yaml` says a redirected request keeps its identity and its address; it
  does not name the method. Decide what the row means (the request as started, or as last
  sent) and say it in `spec/`.
- A derived `Debug` prints a request body in the generated types (`Fetch`, `Navigate`,
  `RequestStarted`, `NavigationStarted`, and the outcomes and invocations that hold them), so
  any `{:?}` of one prints a form's values, passwords included. `surf_fetch::Request` prints
  the body's length since wave 9; `surf_fetch::Response` and `surf::Answer` print a response
  body, which can repeat the form. Before a form's values reach a body, say where a body may
  be printed and how the generated types are kept from it (the session's log writes a body as
  `<n> bytes` already).
- No command sends a `Post` yet: `surf_fetch::send`, `surf::fetch_http` and the session's
  transport can. The 16 MiB default of `Limits::max_request_body_bytes` is a choice no
  measurement backs.
- The media type check of `surf_fetch::send` takes `type/subtype` of RFC 9110 tokens and
  refuses a parameter; `surf.net.MediaType` is specified without parameters (the finding of
  wave 8 about `multipart/form-data` and its `boundary` stands).
