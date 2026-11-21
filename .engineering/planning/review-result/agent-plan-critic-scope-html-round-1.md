---
format: aep.planning-md/3
id: review-result:agent-plan-critic-scope-html-round-1
kind: review-result
status: active
title: Plan critic (scope), round 1, epic html-parser
relations:
- reviews: epic:html-parser
revision: 1
---
needs-revision

The 18 stories cover the parsing algorithm the epic takes on: tokenizer, tree construction, the fragment algorithm, encoding sniffing, character references, limits and the conformance gate. Two warnings remain. One story quietly rewrites an Outcome sentence of the epic, and one runner need for the tree is claimed by no html story.

**Findings**

1. `story:html-decisions` — item 1 replaces an Outcome sentence of the epic and does not say it is doing so. The epic promises "The node tree is reachable by the `agent` component's port inside the session process" (`epic/html-parser.md:39`). Item 1 proposes that "the `agent` component's port receives what it needs as command input or events, never the tree itself" (`story/html-decisions.md:51`). `story:html-document-port` then claims only "the node tree reachable inside the session process", without the port. Open question 1 asks where the tree is modelled, not who can reach it, so this is not one of the questions the epic left open. Two ways to settle it: item 1 names this as a change to Outcome bullet 4 that the operator must confirm, or the epic sentence is changed to read "through `Browser` in the session process".

2. `story:html-document-port` — a runner need that no html story claims. `story:runner-detailed-changes` needs "a node identity that stays the same for a node across changes within one document, so two states can be compared node by node" (`story/runner-detailed-changes.md:30`), and its tests "build the two states directly" (line 27). `html-document-port` says "No mutation path exists" (`story/html-document-port.md:40`). `html-tree-body` exposes only the operations tree construction uses (`story/html-tree-body.md:24-27`). No html story offers either a way to change a tree or a public way to build two trees that share node identities. To settle it, `html-document-port` (or `html-tree-body`) either states and delivers that API, or says the tree never changes within a document, so the runner story can drop the need.

**What I read**

- 31 artifacts: `epic:html-parser`, all 18 `story:html-*` in full, `epic:agent-cli-target`, `epic:e2e-scenario-runner` and `story:runner-detailed-changes` in full. I searched 9 more agent and runner stories for what they take from html.
- Commands: `aep plan artifact show`, `aep plan artifact list --kind story --status draft`, `aep plan artifact graph`, `aep plan artifact kinds`, `aep plan artifact relations`, and `rg` on `.engineering/planning/`.
- The epic makes 21 promises (8 Outcome, 4 open questions, 5 Constraints, 4 Acceptance). 20 trace to a story; 1 is narrowed (finding 1).
- The other two epics need 3 things from this one. The tree in process and the specification's rows are delivered. Document order and stable identity are delivered while a document does not change; within a changing document the need is not claimed (finding 2).

**Coverage of the standard**

- Every part of the parsing algorithm the epic takes on is claimed by a story: preprocessing, the tokenizer states (split into markup, text and character-reference stories), insertion modes, the adoption agency algorithm, tables and `select`, `template`, foreign content and fragments, BOM sniffing, the prescan and the default encoding, the named and numeric character references, and the limits.
- Every part left out is named by a story's "Out of scope" or by the decision story:
  - the transport-layer `charset`
  - changing the encoding during parsing
  - other decoders
  - `scripted/`
  - declarative shadow roots
  - "the end" and the script hooks
  - serialization outside the test format
- No story reaches beyond the epic. The fields html-document-spec adds (the navigation identity, the encoding, the stop reason) trace to the agent epic's needs and to the `UNMAPPED:` marker. The fragment algorithm is justified by the conformance data.
- No two stories claim the same outcome.

**What I could not establish**

- Whether html5lib-tests holds data outside `tokenizer/`, `tree-construction/` and `encoding/` (for example `xmlViolation` tests or `serializer/`) that a section's completeness mark depends on. The inventory of `html-decisions` will show it.
- Whether comparing the tree but not `#errors` for tree construction makes a "passing" case too weak. That is the acceptance critic's lane.
- The `select` selectedness rule is computed in both `html-actionable-elements` and `agent-form-port`, so it may be built twice. That is the design critic's lane.
- `html-document-port` waits only for the stories up to `html-tree-formatting`, so pages built with tables are parsed wrongly until `html-tree-tables` lands. That is the design or parallel-safety lane.
- Finding 2 is also a consistency question across three epics (detailed changes over a tree that never changes). That part belongs to the design critic.

```findings
- file: .engineering/planning/story/html-decisions.md
  line: 51
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "item 1 proposes that the agent component's port receives 'never the tree itself', which replaces the epic Outcome 'The node tree is reachable by the `agent` component's port inside the session process' (epic/html-parser.md:39) without naming it as a change to the Outcome for the operator to confirm"
- file: .engineering/planning/story/html-document-port.md
  line: 40
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "runner-detailed-changes needs 'a node identity that stays the same for a node across changes within one document' and builds two states directly (story/runner-detailed-changes.md:30), and no html story claims either a mutation path or a public way to build trees with shared node identities, while this story states 'No mutation path exists'"
```
