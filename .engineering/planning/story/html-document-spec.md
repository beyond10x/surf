---
format: aep.planning-md/3
id: story:html-document-spec
kind: story
status: active
title: 'Specify the parsed page: title and actionable elements'
summary: surf.document gains the title, encoding, stop reason, navigation and the actionable element rows, with authored scenarios green in the gate.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-decisions
- depends_on: story:html-conformance-gate
- depends_on: story:agent-page-path-spec
- depends_on: story:agent-redirect-spec
- depends_on: story:agent-session-events
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 5}
---
## Context

`spec/domains/document.yaml` holds a `surf.document.Document` as an identity and an address,
created by `surf.document.ParseDocument` from any bytes, with an `UNMAPPED:` marker for the node
tree, parse errors and the character encoding. `epic:html-parser` asks for the observable part
of a parsed page in that domain: the document's title and its actionable elements (links and
form controls) as rows owned by the document, with kind, label, target or value, and the flags
`hidden`, `disabled`, `checked`, `required`. Specs first (`AGENTS.md`, "Rules"): these rows are
in `spec/` with a green `task check` before any code fills them.

`docs/design/html-parser.md` (story `html-decisions`) answers what this story writes: the tree
stays behind the port and only the session process reads it, through `Browser` (item 1); the
command shape, measured against ess 0.96.0 (item 2: one command that starts parsing, one command
per actionable element, then one ending command, issued by the parser step the way the transport
issues `DeliverResponse`; of the eight shapes measured it is the only one with no obligation and
no new refusal); the encoding recorded (item 4); the link target stored as
the `href` value as written, the navigation identity on the document, the nine kinds, and no
stored value for a `select` element (item 6); the limit that stopped parsing (item 7). Every name
follows the HTML Standard's terms where it has one (`AGENTS.md`, "Rules").

Decisions that bind this story (`docs/plan/agent-plan-revision-1.md`): D11, a `select` element
row stores no value; what is stored is each option's selectedness, in option rows that
`story:agent-forms-spec` adds. Resolving a link target is not this story's: `story:agent-url-parser`
writes the URL parser and `story:agent-follow-links` resolves targets against the document base
URL (D10).

Ordering: the specification chain is ordered by edges. This story follows
`story:agent-page-path-spec` (which writes `spec/`, `generated/rust/` and `conformance/src/`
before it), `story:html-conformance-gate`, and `story:agent-session-events`
(`docs/plan/agent-plan-revision-2.md`, D27): "A story that adds an event or an event field after
that maps it in `crates/session`, with a named case, and depends on `agent-session-events`".
This story adds events and event fields, so it maps them there. It writes no line of the root
`Taskfile.yml`, `REFUSALS_MAX` included (D30): it runs its gate as
`slot task check REFUSALS_MAX=<n>` and reports `<n>` and each new refusal.

## Domain relations

None of these is declared by an `ess/1` document today; this story writes each into
`spec/domains/document.yaml` as a typed entry, and `ess specify validate` checks it.

- `surf.document.Document` -> the actionable element entity this story introduces: one-to-many,
  zero allowed; an element belongs to one document and never outlives it (the document's
  lifecycle has no end state that removes rows, and no command deletes an element). Not
  inferable from `ess/1` on `main`: stated by `docs/design/agent-interface.md` section 6 ("as
  entities owned by a document") and the epic's Outcome, accepted as the basis for the epics in
  `approval-record:agent-design-accepted-2026-11-21`. Written on the element as the note measured
  it (item 2, variants `list-rel` and `per-element-plain-rel`, "valid, no warning"):
  `relations: [{name: document, kind: references, target: surf.document.Document, cardinality:
  one, via: document_id}]`; without it `validate` warns `ESS-ENTITY-019`.
- `surf.document.Document` -> `surf.navigation.Navigation` by `surf.net.RequestId`: zero or one
  document per navigation, the navigation is not owned by the document. Not inferable from
  `ess/1`: the binding `parse-on-navigation-committed` in `spec/components.yaml` maps only `url`
  and `body` today; proposed by story `html-decisions` (item 6).
- The node tree and the element-to-node link are not modelled (item 1), so no relation is
  written for them; the comment that replaces the `UNMAPPED:` marker says where they live.

## Change

- `spec/domains/document.yaml`, in the shape of item 2, each name chosen here in the HTML
  Standard's terms:
  1. `ParseDocument` (input `url`, `body` and the navigation's `surf.net.RequestId`) creates the
     document in a new state `Parsing` and publishes an event that carries the document's
     identity and the body, as `RequestStarted` carries the address to the transport.
  2. One command per actionable element, issued in document order, creating that element's row:
     its identity, its document's identity, its position in document order, its kind (an
     enumeration of the nine kinds of item 6), label, target (links: the `href` as written),
     value (text-like controls, `password`, `textarea`, `file`; none for `select`, D11) and the
     four flags `hidden`, `disabled`, `checked`, `required`. It declares no refusal: in ess
     0.96.0 it cannot refuse an unknown document, or one already `Parsed`, without becoming an
     obligation (a `when_related` guard, variant `per-element`) or being refused by `validate`
     (the document as its subject, variant `per-element-subject`). A comment beside the command
     says so, cites item 2, and states why the gap is accepted: no actor is granted the
     command, and only the parser step issues it, in order, before the ending command.
  3. One ending command that moves the document from `Parsing` to `Parsed` and sets the title
     (as the standard's `document.title` gives it), the encoding used, the encoding name not
     honoured if any, and the limit that stopped parsing if any, and publishes the event that
     the document is parsed.

  The `Documents` view gains those fields and a view lists a document's elements in document
  order. No obligation remains: every capability of the three commands is generated. The
  `UNMAPPED:` marker is replaced by a comment for what item 1 decided; a marker stays for parse
  errors, which item 1 leaves out.
- `spec/components.yaml`: the `document` component accepts the per-element and the ending
  command and publishes the new events; `parse-on-navigation-committed` maps the navigation identity.
- `spec/scenarios/`: `page-load-commits-and-parses.yaml` updated to the new shape, and three new
  authored scenarios: `parsed-page-with-title-links-and-form` (a title, three links and a form
  with a text field, a password field, a checkbox, a `select` with no value and a submit
  button), `parsed-page-without-title`, `parse-stopped-at-a-limit`. `spec/ess-inputs.yaml` lists
  them.
- `generated/rust/` regenerated by `task generate`; `conformance/src/target.rs` and
  `conformance/src/values.rs` spell the new commands, outcomes, views and events; `crates/ports/
  src/ports.rs` stores the new entity; `crates/surf/src/lib.rs` changed only as far as the
  regenerated interface needs to compile, with no parsing added. The binary (`crates/surf-cli`)
  is not changed.
- `crates/session`: the function of `story:agent-session-events` that maps `SystemEvent` to a
  record maps every event and event field this story adds (D27), each value written by the
  quoting function of `surf-answer` (D26) and a field of bytes, such as a body, written as
  `<n> bytes` (D22); so the case of that story that every `SystemEvent` variant maps, none
  `unmapped`, still holds. A named case in `crates/session/tests/`:
  `document_events_map_with_their_fields`, which publishes each new event with known values and
  compares the record field by field.
- `REFUSALS_MAX` is not written (D30). The gate runs as `slot task check REFUSALS_MAX=<n>`, with
  `<n>` the `refused:` count `task suite` prints; the report gives `<n>` and each new refusal with
  its reason, and the coordinator writes the number on the wave branch. No binding is added (the
  per-element and ending commands are issued by the parser step, not by a binding), and the note
  measured the shape with no new refusal (variant `per-element-nav`: `10 refusal(s)` before and
  after), so the Acceptance holds the count equal. The floor of 8 more scenarios is the 5 the
  same variant added in synthesis (51 to 56) and the 3 new authored scenarios.

Writes: `spec/domains/document.yaml`, `spec/components.yaml`, `spec/ess-inputs.yaml`,
`spec/scenarios/`, `generated/rust/`, `conformance/src/target.rs`, `conformance/src/values.rs`,
`crates/ports/src/ports.rs`, `crates/surf/src/lib.rs`, `crates/session/` (the event mapping and
`tests/`). Not the root `Taskfile.yml`.

For the root `Taskfile.yml`, handed to the coordinator in the report (D30): `REFUSALS_MAX: <n>`.

For `AGENTS.md`, handed over in the report (D7), with the numbers this story measures:
- "Components", the `document` row: "`document` | `surf.document` | `Document`, <the element
  entity> | `ParseDocument`, <the per-element command>, <the ending command>".
- "Components", the paragraph after the table: "`NavigationCommitted` invokes `ParseDocument`
  when its media type is `text/html`; the parser step of `crates/surf` issues <the per-element
  command> once per actionable element in document order, then <the ending command>, which sets
  the title and moves the document to `Parsed`."
- "The gate's numbers": the synthesis line `<n> scenario(s) (<a> authored), <r> refusal(s)`,
  both run lines `<n> scenarios: <n> passed, 0 failed, 0 error, 0 unsupported`, and one bullet
  per new refusal with its reason.

## Acceptance

`slot task check REFUSALS_MAX=<n>` exits 0 with the scenario total at least 8 above the total
recorded before it and equal on both runners with `0 failed, 0 error, 0 unsupported`, the three
authored scenarios named in Change among the passed, the `refused:` count equal to the count
recorded before it and `<n>` equal to it; `.out/generate.log` holds the summary line `<c>
capabilities: <g> generated, 0 obligation(s), 2 refused`; `slot task open` no longer lists the
node-tree marker of `spec/domains/document.yaml`; and `cargo test -p surf-session` passes
`document_events_map_with_their_fields`.

## Out of scope

Filling the rows from a real page (story `html-document-port`); form owners, option rows and
anything the agent's `fill`, `pick` and `press` need (`story:agent-forms-spec`); resolving a link
target (`story:agent-follow-links` with `story:agent-url-parser`). Scripts: the specification
says nothing about scripts.
