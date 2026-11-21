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
scope:
- confidence: cited
  path: conformance/src/target.rs
- confidence: cited
  path: conformance/src/values.rs
- confidence: cited
  path: conformance/tests/planted_defect.rs
- confidence: cited
  path: conformance/tests/review2_planted_defects.rs
- confidence: cited
  path: conformance/tests/two_of_each_entity.rs
- confidence: cited
  path: crates/ports/src/lib.rs
- confidence: cited
  path: crates/ports/src/ports.rs
- confidence: cited
  path: crates/ports/tests/ports.rs
- confidence: cited
  path: crates/session/src/entries.rs
- confidence: cited
  path: crates/session/tests/events.rs
- confidence: cited
  path: crates/session/tests/memory.rs
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: crates/surf/tests/forget_delivered.rs
- confidence: cited
  path: crates/surf/tests/held.rs
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
  path: spec/scenarios/page-load-commits-and-parses.yaml
- confidence: cited
  path: spec/scenarios/parse-stopped-at-a-limit.yaml
- confidence: cited
  path: spec/scenarios/parsed-page-with-title-links-and-form.yaml
- confidence: cited
  path: spec/scenarios/parsed-page-without-title.yaml
revision: 30
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:12Z", actor: "human:timo", revision: 5}
---
## Context

`spec/domains/document.yaml` holds a `surf.document.Document` as an identity and an address,
created by `surf.document.ParseDocument` from any bytes in its one state `Parsed`, with an
`UNMAPPED:` marker for the node tree, parse errors and the character encoding. `epic:html-parser`
asks for the observable part of a parsed page in that domain: the document's title and its
actionable elements (links and form controls) as rows owned by the document, with kind, label,
target or value, and the flags `hidden`, `disabled`, `checked`, `required`. Specs first
(`AGENTS.md`, "Rules"): these rows are in `spec/` with a green `task check` before any code fills
them.

`docs/design/html-parser.md` (story `html-decisions`) answers what this story writes: the tree
stays behind the port and only the session process reads it, through `Browser` (item 1); one
command that starts parsing, one command per actionable element, then one ending command, issued
by the parser step the way the transport issues `DeliverResponse` (item 2); the encoding recorded
(item 4); the link target stored as the `href` value as written, the navigation identity on the
document, the nine kinds, and no stored value for a `select` element (item 6); the limit that
stopped parsing (item 7). Every name follows the HTML Standard's terms where it has one
(`AGENTS.md`, "Rules").

Decisions that bind this story (`docs/plan/agent-plan-revision-1.md`): D11, a `select` element
row stores no value; what is stored is each option's selectedness, in option rows that
`story:agent-forms-spec` adds. Resolving a link target is not this story's:
`story:agent-url-parser` writes the URL parser and `story:agent-follow-links` resolves targets
against the document base URL (D10). D21 and D22 (`docs/plan/agent-plan-revision-2.md`): no
password value and no bytes in the session's log. D27: an event or event field added after
`story:agent-session-events` is mapped in `crates/session` with a named case. D30: this story
writes no line of the root `Taskfile.yml`.

The model was drafted and measured before the wave, on ess 0.96.0, in a copy of `spec/` of
`main` at `d437dc6` (the reviser's scratch `wave6/html-document-spec/final/spec`, not committed;
the coordinator hands it to the unit). Each run used the Taskfile's own commands (`validate`,
`conform synthesize`, `conform run --target interpreted`, `generate synthesize --target rust`):

| spec | validate | synthesis | interpreter | Rust generation |
|---|---|---|---|---|
| `main` | `9 scenario(s), valid` | `83 scenario(s) (9 authored), 27 refusal(s)` | `83 passed` | `151 capabilities: 143 generated, 4 obligation(s), 4 refused` |
| this story, `AddElement` without guards | `12 scenario(s), valid`, no warning | `91 scenario(s) (12 authored), 27 refusal(s)` | `91 passed` | `167 capabilities: 159 generated, 4 obligation(s), 4 refused` |
| this story as written below | `12 scenario(s), valid`, no warning | `93 scenario(s) (12 authored), 28 refusal(s)` | `93 passed, 0 failed, 0 error, 0 unsupported` | `167 capabilities: 159 generated, 4 obligation(s), 4 refused`, 30 artifacts |

The 4 obligations (`NoteNavigation*`) and the 4 refused capabilities (`needs_caller_identity`)
are those of `main`; this story adds none. What the measurement corrects in the design note:
`AddElement` can refuse an unknown document and a document already `Parsed` with no obligation.
Item 2 measured only the row-set guard (`when_related: {entity:, where:}`); the `via:` form that
`surf.agent.RecordAction` already uses (`no-such-session`) is generated and synthesized, and so is
`when_related: {via: input.document_id, predicate: state == Parsed}`. The gap item 2 accepted is
closed. Its price is one refusal: `ESS-SYNTH-014` for `AddElement/added`, because the guards leave
synthesis one element where the ordered view `Elements` needs two, as for `RecordAction/recorded`;
the authored scenario `parsed-page-with-title-links-and-form` covers it.

Also measured: an outcome that emits no event is refused by `validate` (`ESS-COMMAND-007`,
`empty_change`), so `AddElement` must publish an event and the session log gets one entry per
element once `story:html-document-port` fills the rows.

## Domain relations

None of these is declared by an `ess/1` document today; this story writes each into
`spec/domains/document.yaml` as a typed entry, and `ess specify validate` checks it.

- `surf.document.Document` -> `surf.document.Element`: one-to-many, zero allowed; an element
  belongs to one document and never outlives it (the document's lifecycle has no state that
  removes rows, and no command deletes an element). Stated by
  `docs/design/agent-interface.md` section 6 ("as entities owned by a document") and the epic's
  Outcome, accepted in `approval-record:agent-design-accepted-2026-11-21`. Written on the element:
  `relations: [{name: document, kind: references, target: surf.document.Document, cardinality:
  one, via: document_id}]`; the two guards of `AddElement` resolve through it.
- `surf.document.Document` -> `surf.navigation.Navigation` by `surf.net.RequestId`
  (`navigation_id`): zero or one document per navigation, the navigation is not owned by the
  document. Written on the document as `relations: [{name: navigation, kind: references, target:
  surf.navigation.Navigation, cardinality: one, via: navigation_id}]`, mapped by
  `parse-on-navigation-committed` from `NavigationCommitted.navigation_id` (item 6).
- The node tree and the element-to-node link are not modelled (item 1), so no relation is
  written for them; the comment that replaces the node-tree part of the `UNMAPPED:` marker says
  where they live.

## Change

- `spec/domains/document.yaml`, as drafted:
  - Types: `surf.document.ElementId` (newtype of `Uuid`, minted by the component like
    `surf.agent.ActionId`); `surf.document.ElementKind`, an enum of the nine kinds of item 6
    spelled as the answer prints them: `link`, `button`, `text`, `password`, `checkbox`, `radio`,
    `file`, `select`, `textarea`; `surf.document.EncodingName` (newtype of `String`, the name as
    the Encoding Standard writes it); `surf.document.ParseLimit`, an enum of the five limits of
    item 7: `InputBytes`, `Depth`, `Nodes`, `Attributes`, `StoredBytes`.
  - `Document`: fields `url`, `navigation_id: surf.net.RequestId`, `title: Optional<String>`,
    `encoding: Optional<EncodingName>`, `unsupported_encoding: Optional<EncodingName>`,
    `stopped_by: Optional<ParseLimit>` (the four optional ones absent while `Parsing`); lifecycle
    `Parsing` (initial) and `Parsed` (terminal), transition `finish`. `title` is what
    `document.title` returns, the empty string when the document has no `title` element.
    `encoding` is the encoding the text was decoded with (`surf_html::encoding::Decoded::
    decoded_with`); `unsupported_encoding` is the name the sniffing chose when surf has no decoder
    for it (`Decoded::encoding` when it differs), a deviation from 13.2.3.3 said in a comment.
  - `Element`: `document_id`, `position: Integer` (from 1, tree order), `kind`, `label: String`
    (the whole accessible name), `target: Optional<String>` (a link's `href` as written),
    `value: Optional<String>` (`text`, `password`, `textarea`, `file`; none for `select`, D11),
    and the four `Boolean` flags; one state `Added`.
  - `ParseDocument` (input `url`, `body`, `navigation_id`): outcome `started` creates the document
    in `Parsing` and emits `surf.document.ParsingStarted {document_id, body}`, as `RequestStarted`
    carries the address to the transport. Its outcome `parsed` and its emission of `DocumentParsed`
    go.
  - `AddElement` (input `document_id`, `position`, `kind`, `label`, `target`, `value` and the four
    flags): outcomes `no-such-document` (`when_related: {via: input.document_id, exists: false}`,
    error `DocumentNotFound`), `already-parsed` (`when_related: {via: input.document_id,
    predicate: state == Parsed}`, error `DocumentStateConflict`), `added` (creates the row, emits
    `surf.document.ElementAdded {element_id, document_id, position}`). The event carries no
    label, target or value, so no page text and no password value reaches the session log by it
    (D21). A second element at a position already taken is not refused; a comment says the parser
    step numbers them once.
  - `FinishParsing` (input `document_id`, `title: String`, `encoding`, `unsupported_encoding`,
    `stopped_by`; `refs:` `whatwg-html:stop-parsing`, `whatwg-html:document.title`): outcomes
    `no-such-document` (`unknown_instance`), `parsed` (moves `finish`, sets the four fields, emits
    `surf.document.DocumentParsed {document_id, url, navigation_id, title, encoding,
    unsupported_encoding, stopped_by}`), `wrong-state` (`DocumentStateConflict`).
  - No actor is granted `AddElement` or `FinishParsing`; a comment says the parser step issues
    them, one `AddElement` per actionable element in tree order, then `FinishParsing`.
  - Views: `Documents` gains `navigation_id`, `state`, `title`, `encoding`,
    `unsupported_encoding`, `stopped_by`; `Elements` (param `document_id`, `order_by: [position
    asc]`) lists every field of a document's elements.
  - The `UNMAPPED:` marker is replaced by a comment for what item 1 decided (the tree behind the
    port) and one `UNMAPPED:` marker for parse errors only.
- `spec/domains/agent.yaml`: the text of the marker "the session's document" (`Session`
  relations) only, whose premise ("`DocumentParsed` ... names no navigation") stops being true: the
  document now carries `navigation_id`; what stays open is that the `agent` component stores no
  document, and the session process finds it in `Documents` by `navigation_id`
  (`story:agent-follow-links`). The marker stays a marker.
- `spec/components.yaml`: the `document` component accepts `AddElement` and `FinishParsing` and
  publishes `ParsingStarted`, `ElementAdded` and `DocumentParsed`; `parse-on-navigation-committed`
  maps `navigation_id: event.navigation_id`.
- `spec/scenarios/`: `page-load-commits-and-parses.yaml` asserts the document with
  `navigation_id: {$instance: load}`, `state: Parsing`, `title: null`; three new authored
  scenarios, listed in `spec/ess-inputs.yaml`:
  - `parsed-page-with-title-links-and-form`: title `Shop`, three links and a form of a text
    field, a password field, a checkbox, a radio button, a file control, a `select` with no
    value, a textarea and a submit button: eleven elements, every `ElementKind` once or more, added
    in tree order except the third link, which is added first; asserts 11 rows, `ranked: true`,
    the first and last row, and each row's fields. It covers `AddElement/added` (`ESS-SYNTH-014`).
  - `parsed-page-without-title`: title `""`, encoding `windows-1252`, no element row.
  - `parse-stopped-at-a-limit`: five documents, each stopped by another `ParseLimit`; the first
    keeps the element added before the limit and records `unsupported_encoding: EUC-KR`.
  An event payload in a scenario does not list an absent optional field: `null` there fails on the
  interpreter (`ESS-CF-PAYLOAD`, "was not carried"); in a view assertion `null` passes.
- `generated/rust/`: regenerated by `task generate`, nothing by hand. The generated
  `TryContext` gains `try_generate_surf_document_element_id`, the `document` component the
  storage trait `ElementStorage`, and the `System` bound three traits (`AddElementBehavior`,
  `FinishParsingBehavior`, `ElementsQuery`). The root `Cargo.lock` does not change (measured
  with `cargo check --locked`).
- `crates/ports/src/ports.rs`: `DocumentPorts` stores `Element` rows (`ElementStorage`); the
  `context!` macro assigns an `ElementId` like a `DocumentId`; `AgentPorts` answers it as unmet.
  `crates/ports/src/lib.rs`: the table of the crate documentation (`Document`, `Element`) and the
  bound of `document_port` (`ElementStorage`). `crates/ports/tests/ports.rs`: the `DocumentData`
  of the storage test gains the five new fields (it no longer compiles without them), and the
  identity tests cover `ElementId`.
- `crates/surf/src/lib.rs`: `DocumentBehaviors` gains the three bounds (without them nothing in
  the crate compiles); `event_body` counts the body of `ParsingStarted`, and the module
  documentation says a body is in five places of the record, not four. No parsing is added: a
  real page's document stays `Parsing` until `story:html-document-port`. The output of the
  `surf` binary (`crates/surf-cli`) does not change. New test `crates/surf/tests/held.rs`,
  `a_parsed_body_is_held_five_times_until_delivered`: after `run` over one HTML response of `n`
  bytes, `held().body_bytes` is `5 * n`; after `forget_delivered`, 0.
- `crates/session/src/entries.rs`: `event_line` maps `ParsingStarted` (`document_id`, `body` as
  `<n> bytes`), `ElementAdded` (`element_id`, `document_id`, `position`) and the new fields of
  `DocumentParsed` (`navigation_id`, `title` quoted, `encoding`, `unsupported_encoding` and
  `stopped_by` as optional values, the limit by its variant name); the module documentation lists
  `ParsingStarted.body` among the byte fields. Without this, a `go` writes
  `event surf.document.ParsingStarted unmapped` and `case_04_two_sessions_at_once_keep_their_own_entries`
  fails (measured).
- `crates/session/tests/events.rs`: `case_17_every_event_maps_with_all_its_fields` fails as soon
  as `spec/` changes (measured: the covered variants and the events of `spec/` differ). `covered()`
  gains the two events and the new `DocumentParsed` fields, `expected_value` the new fields, the
  count assertion reads 19 (its message names this story), `BYTES_FIELDS` gains
  `surf.document.ParsingStarted.body`. The named case `document_events_map_with_their_fields`
  (in this file, beside the helpers it needs) maps each of the three document events with known
  values (a title holding `"`, a backslash and `é`; `unsupported_encoding` present and absent;
  `stopped_by` `Depth` and absent) and compares the record field by field through
  `surf_answer::unquote`.
- `conformance/src/target.rs`: the two commands and their outcomes, `ParseDocument`'s new input
  and outcome, the two events in `publisher` and the event mapping, the `Elements` view with its
  parameter, the new `Documents` fields, `ElementKind` spelled lower case, `ParseLimit` and
  `Document.State`. `conformance/src/values.rs`: a `Boolean` reader and writer (it has none).
- `conformance/tests/planted_defect.rs`, `review2_planted_defects.rs`, `two_of_each_entity.rs`:
  each wraps `DocumentPorts` and implements `TryContext`, so each gains
  `try_generate_surf_document_element_id` and an `ElementStorage` pass-through (the mutant and the
  planted storage apply their policy to it). `two_of_each_entity.rs` adds
  `SameIdentity(Element)` and `FirstRowOnly(Element)`; `parsed-page-with-title-links-and-form`
  holds eleven elements, so both must fail at least one scenario. The test names the gate greps
  do not change.

Writes: `spec/domains/document.yaml`, `spec/domains/agent.yaml` (one marker's text),
`spec/components.yaml`, `spec/ess-inputs.yaml`, `spec/scenarios/` (one changed, three new),
`generated/rust/`, `crates/ports/src/ports.rs`, `crates/ports/src/lib.rs`,
`crates/ports/tests/ports.rs`, `crates/surf/src/lib.rs`, `crates/surf/tests/held.rs` (new),
`crates/session/src/entries.rs`, `crates/session/tests/events.rs`, `conformance/src/target.rs`,
`conformance/src/values.rs`, `conformance/tests/planted_defect.rs`,
`conformance/tests/review2_planted_defects.rs`, `conformance/tests/two_of_each_entity.rs`. Not the
root `Taskfile.yml`, not `AGENTS.md`, not `docs/view-stream.md`, no file of `crates/surfd`,
`crates/view` or `crates/html`.

For the root `Taskfile.yml`, handed to the coordinator in the report (D30): `REFUSALS_MAX: 28`.

For `docs/view-stream.md`, handed to the coordinator, written after this story and
`story:view-surfd-serves` are merged (that story's goldens are copied from the document's
examples): the "Event names" block gains `surf.document.ParsingStarted` and
`surf.document.ElementAdded` (the document's rule: "A story that maps a new event adds its name
here"). The examples stay as they are; they show `DocumentParsed` with two fields, and a real `go`
now publishes `ParsingStarted` in that place until `story:html-document-port`.

For `AGENTS.md`, handed over in the report (D7), with the numbers this story measures:
- "Components", the `document` row: "`document` | `surf.document` | `Document`, `Element` |
  `ParseDocument`, `AddElement`, `FinishParsing`".
- "Components", the paragraph after the table: "`NavigationCommitted` invokes `ParseDocument`
  when its media type is `text/html`; the document is `Parsing` and `ParsingStarted` carries the
  body. The parser step of `crates/surf` (`story:html-document-port`) issues `AddElement` once
  per actionable element in tree order, then `FinishParsing`, which sets the title and moves the
  document to `Parsed`. Until then a real page's document stays `Parsing`."
- "Layout": `spec/scenarios/` holds twelve authored files, the three new ones named; the
  `crates/ports` entry: `DocumentPorts` stores `Document` and `Element`.
- "The gate's numbers": `surf v1 — 9 file(s), 12 scenario(s), valid`, `93 scenario(s) (12
  authored), 28 refusal(s)`, both run lines `93 scenarios: 93 passed, 0 failed, 0 error, 0
  unsupported`, the planted-defect counts the unit measures, and the bullet: "One more is
  `ESS-SYNTH-014`: the outcome `surf.document.AddElement/added`. The view `Elements` declares an
  order, and the guards of `AddElement` leave synthesis one element. `parsed-page-with-title-links-and-form.yaml`
  covers it."

## Acceptance

- `slot task check REFUSALS_MAX=28` exits 0 and prints `surf v1 — 9 file(s), 12 scenario(s),
  valid`, `93 scenario(s) (12 authored), 28 refusal(s), written to .out/suite.json`, and on both
  runners `93 scenarios: 93 passed, 0 failed, 0 error, 0 unsupported`; `.out/conform.log` holds
  `passed surf.document/authored/parsed-page-with-title-links-and-form`, `... parsed-page-without-title`
  and `... parse-stopped-at-a-limit`.
- `grep '^refused:' .out/synthesize.log` prints the 27 lines of `main` and one more,
  `refusal[ESS-SYNTH-014]: outcome surf.document.AddElement/added has no scenario`.
- `.out/generated-check/fresh.log` (written by `task generated-check` inside `task check`) holds
  `167 capabilities: 159 generated, 4 obligation(s), 4 refused` and `30 artifact(s)`: no
  obligation and no refused capability beyond those of `main`.
- `slot task open` prints a line of `spec/domains/document.yaml` naming parse errors and none
  naming the tree of nodes.
- `cargo test -p surf-session` passes with `document_events_map_with_their_fields`,
  `case_17_every_event_maps_with_all_its_fields` (19 events) and `byte_fields_are_lengths`
  (three byte fields); `cargo test -p surf` passes with
  `a_parsed_body_is_held_five_times_until_delivered`; `cargo test --manifest-path
  conformance/Cargo.toml` passes with the per-entity tests planting `SameIdentity(Element)` and
  `FirstRowOnly(Element)`, and the unit reports how many scenarios each fails.
- Reverting only the two guards of `AddElement` gives `91 scenario(s) (12 authored), 27
  refusal(s)` (the measured variant), so the refusal and the two scenarios are the guards'.

## Out of scope

Filling the rows from a real page and the parser step (story `html-document-port`); form owners,
option rows and anything the agent's `fill`, `pick` and `press` need (`story:agent-forms-spec`);
resolving a link target (`story:agent-follow-links` with `story:agent-url-parser`); the title on
the agent's `page` line and the session status record (`story:agent-follow-links`). Scripts: the
specification says nothing about scripts. For `story:html-document-port` to meet: every element
becomes one `ElementAdded` entry in the session log, and `crates/surfd` bounds a session's log at
10,000 entries (`log::ENTRIES_PER_SESSION`) and one answer's entries at 8 MiB
(`link::ENTRIES_MAX`), while the node limit allows far more elements.
