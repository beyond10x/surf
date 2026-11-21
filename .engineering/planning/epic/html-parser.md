---
format: aep.planning-md/3
id: epic:html-parser
kind: epic
status: active
title: HTML parsing and the node tree
summary: A hand-written HTML parser held to the WHATWG standard's test data, and the observable part of a page in the specification.
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 4}
---
## Context

`docs/design/agent-interface.md` (accepted as the basis for the epics on 2026-11-21,
`approval-record:agent-design-accepted-2026-11-21`), section 9, step 2. The agent command line
lists links and controls and reports what changed on a page. All of that needs what a page
holds, and the specification has none of it: a `surf.document.Document` is an identity and an
address, and the `UNMAPPED:` marker in `spec/domains/document.yaml` names the node tree as
missing. `ParseDocument` creates a document from any bytes and looks at none of them.

The design proposes, and the operator can overrule: the parser is a port of the `document`
component, as the HTTP client is a port of `net`. The specification states what is observable;
the parsing algorithm is the WHATWG HTML Standard's, and the parser is held to it by that
standard's own test data.

## Outcome

- A hand-written HTML parser, standard library only, that turns the bytes of a response into a
  node tree: the tokenizer and the tree construction of the WHATWG HTML Standard, with its
  error recovery, and the character encoding decided the way the standard says for the cases
  taken on.
- The parser is run against the standard's conformance data (the html5lib tests for the
  tokenizer and for tree construction; named from memory, to be confirmed by the first story)
  as test tooling. The counts are in the gate: how many cases exist, how many run, how many
  pass. A section of the standard is called complete only when every case for it passes
  (`AGENTS.md`, "Goals").
- The specification's `surf.document` domain gains the observable part of a parsed page: the
  document's title and its actionable elements (links and form controls) as rows owned by the
  document, with the fields the answer format prints: kind, label, target or value, and the
  flags `hidden`, `disabled`, `checked`, `required`.
- The node tree is reachable inside the session process through `Browser`
  (`Browser::document_tree`), so that the session process can list and label what a page holds.
  No component's port receives the tree: it is the `document` component's
  (`docs/plan/agent-plan-revision-1.md`, D1). The tree is built by parsing and is not mutated
  before a script engine exists (D2).

## Open, to be settled by this epic's first story

- Whether the whole node tree is modelled in ESS or only the actionable elements, with the tree
  behind the parser port (design, section 10, question 1). The proposal is the second.
- How the conformance data enters the repository: it is test tooling with its own licence, and
  nothing that ships may depend on it.
- Which encodings are taken on first. UTF-8 is the minimum; the Encoding Standard names the
  rest.
- What "label" is before CSS exists: the accessible name computation reads text, `label`,
  `aria-label`, `alt`, `title` and `placeholder`; parts of it depend on what is rendered.

## Out of scope

- Scripts: `script` elements are parsed as elements and never run. `document.write`, the
  parser's script-related states that need an engine, and custom elements wait for the script
  engine.
- CSS, layout, rendering, and visibility beyond the `hidden` attribute.
- Serializing a tree back to HTML, except where the conformance data's format needs it.

## Constraints

- Specs first. No third-party crate in the parser. `#![forbid(unsafe_code)]`.
- The bytes come from an untrusted server: every limit ends parsing with a defined result, none
  is a hang, an unbounded allocation or a panic. Depth of the tree and size of the input have
  limits, stated like the limits of `crates/fetch`.
- No Python anywhere, also not to read or convert the conformance data.

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. `task html-conformance` prints, for the tokenizer data, the tree-construction data and the
   encoding cases, a number of passed cases equal to the number of cases run, and the number run
   plus the cases the decision note names as not run (each with its reason) equals the number
   that exist in the data.
2. Every section is printed `complete n/n`, or `incomplete` with a reason that is not a failing
   case (a requirement that needs a script engine, or a test of the standard that cannot run
   here). No section is `complete` while the decision note lists an open requirement or a test
   that does not run for it.
3. `surf <url>` on a page with a title, three links and a form, served by the test, prints the
   title and one row per actionable element (a named test).
4. Both conformance reports hold at least one passed scenario for every outcome of the
   `surf.document` commands that create a document and its elements.
