---
format: aep.planning-md/3
id: review-result:agent-plan-critic-design-round-1
kind: review-result
status: active
title: Plan critic (design), round 1, the three agent interface epics
relations:
- reviews: epic:agent-cli-target
- reviews: epic:html-parser
- reviews: epic:e2e-scenario-runner
revision: 1
---
needs-revision

The set has no cycle and the epics fit together. The answer-format grammar, `crates/page`, `spec/domains/document.yaml` and form rows each have one owner or are ordered by an edge. The detailed mode and the scenario formats have problems where the epics meet, and two of the coordinator's shared-file edges make the critical path longer than it needs to be.

**Findings**

1. story:runner-formats — Its grammars embed the answer's element lines and `changed` lines, which agent-follow-links, agent-fill-pick and agent-change-and-log define later. agent-change-and-log even decides a `changed` form the design does not have. runner-formats has no depends_on edge to agent-change-and-log; add one, or make the recording carry answer lines as opaque quoted text. — `.engineering/planning/story/runner-formats.md:24-29` ("Until that document exists the implementor reads the design, section 5"), `.engineering/planning/story/agent-change-and-log.md:28-31` ("Decided here, where design section 5.3 leaves it open")
2. story:runner-formats — The `exchange` block matches requests on method and body and records `status <code> <media type>` with no response URL. The method and body come from agent-request-body-spec and the post-redirect response URL from agent-links-spec, and nothing orders runner-formats after either. As written, a replayed POST→303 commits at the wrong address. Add depends_on agent-request-body-spec (which already comes after agent-links-spec) and a response-URL field. — `.engineering/planning/story/runner-formats.md:41-42`, `.engineering/planning/story/agent-links-spec.md:34-35` ("a response has a URL of its own"), `.engineering/planning/story/agent-request-body-spec.md:43`
3. story:runner-detail-option — The acceptance asks `fill --detail` to show "the attribute-level line for the same control". No story changes the tree on `fill`: it writes the spec row, and html-document-port gives the tree no mutation path, so this acceptance cannot pass. Either rewrite the acceptance to a change a session can produce before scripts, or name the story that mirrors control state into the tree. — `.engineering/planning/story/runner-detail-option.md:50-52`, `.engineering/planning/story/html-document-port.md:39-41` ("No mutation path exists"), `.engineering/planning/story/runner-detailed-changes.md:26-27`
4. story:runner-detail-option — It writes a second grammar and a second renderer for the detailed line (`docs/agent-answer.md`, `crates/answer`). runner-formats and runner-detailed-changes already define them (`docs/formats`, `crates/scenario`), and the two `note` texts already differ. Name one owner; the other refers to it. — `.engineering/planning/story/runner-detail-option.md:39-43` ("detail: attributes, classes and text only"), `.engineering/planning/story/runner-detailed-changes.md:40-42` (`note detail: computed style not available`)
5. story:runner-record-session — It and runner-detail-option both wire runner-detailed-changes' function into the session core, once per action, with no edge between them. One story owns the hook in `crates/session` and the other depends on it. — `.engineering/planning/story/runner-record-session.md:39-45`, `.engineering/planning/story/runner-detail-option.md:36-38`
6. story:runner-surf-cli-package — It moves the `surf` binary and its binary tests out of package `surf`, and is ordered only after agent-session-process. Four other stories write `crates/surf/src/main.rs` or binary tests whose acceptance runs `cargo test -p surf`: agent-request-body-spec, agent-fetch-post, agent-fetch-redirects and html-document-port. Whichever order the waves pick, someone's acceptance names the wrong package. Two ways to settle it:
   - Move first: drop the edge to agent-session-process, since agent-session-process.md:25-26 already answers which crate holds the core, and add edges from the four stories.
   - Move last: add edges from it to the four. — `.engineering/planning/story/runner-surf-cli-package.md:48-49` ("Collides with any story of `epic:agent-cli-target` that changes `crates/surf`"), `.engineering/planning/story/agent-fetch-redirects.md:44-46`
7. story:agent-links-spec — It holds two things: `Follow` and the net/navigation redirect model. Its edge to html-document-port exists for the shared `crates/surf` and `conformance/src`. That edge puts agent-fetch-redirects, agent-request-body-spec and agent-fetch-post on the 23-story critical path behind the whole parser, though none of them reads the node tree. The options are to split out the redirect and response-URL half, ordered before html-document-spec (about 4 steps shorter), or to keep this order knowingly. — `.engineering/planning/story/agent-links-spec.md:12`, `:25` ("needs nothing from `epic:html-parser`"), `:74-76`; `aep plan artifact graph`
8. story:agent-session-spec — It carries `cli/surf-agent.yaml` and the `cli` task, and agent-cli-client is their only reader. So the client waits on the hardest story in the plan (5 synthesis failures still to resolve in the model). Split the command-line document and task into a story that agent-cli-client depends on. — `.engineering/planning/story/agent-session-spec.md:75-79`, `:89-93`, `.engineering/planning/story/agent-cli-client.md:22-24`
9. story:html-conformance-gate — Its depends_on agent-session-spec exists only for the shared `Taskfile.yml` and `AGENTS.md`. It puts the whole 13-story parser chain behind the largest agent story. The options are to reverse the edge (agent-session-spec after the small gate story) or to split the `AGENTS.md` edits. — `.engineering/planning/story/html-conformance-gate.md:11`, `:38-46`
10. epic:agent-cli-target — It says `depends_on: epic:html-parser`, yet three parser stories depend on agent stories:
    - html-conformance-gate → agent-session-spec
    - html-document-spec → agent-page-path-spec
    - html-document-port → agent-session-process

    The epic edge states an order its own stories contradict. Restate it at step level (steps 3–5 need html-document-port) or remove it. — `.engineering/planning/epic/agent-cli-target.md:9`, `.engineering/planning/story/html-document-port.md:13`
11. epic:html-parser — Its Outcome says "The node tree is reachable by the `agent` component's port inside the session process". html-decisions proposes the opposite, and html-document-port, agent-follow-links and agent-change-and-log build the opposite: the session process reads the tree through `Browser::document_tree`. Rewrite the epic line to match the decision. — `.engineering/planning/epic/html-parser.md:39`, `.engineering/planning/story/html-decisions.md:50-52`
12. story:agent-form-port — It works out the document base URL and resolves the form action in `crates/surf`. agent-follow-links works out the same base URL from the tree in `crates/session`, and no edge links the two, so it gets written twice. One story owns the base URL (for example a function in `crates/page`) and the other depends on it. — `.engineering/planning/story/agent-form-port.md:41-42`, `.engineering/planning/story/agent-follow-links.md:31-33`
13. story:agent-forms-spec — It adds option rows with selectedness and a command that sets selectedness. It does not say what happens to the select element's `value` field, which html-document-spec specifies and html-actionable-elements computes from selectedness. The selected option then lives in two fields with two writers. Either the command updates the element's value, or `select` loses the value field. — `.engineering/planning/story/agent-forms-spec.md:61-63`, `.engineering/planning/story/html-actionable-elements.md:22`, `:42`, `.engineering/planning/story/agent-fill-pick.md:35`

**Ordering of the first ten**

As drafted:
1. html-decisions
2. runner-decisions
3. agent-session-spec
4. agent-answer-format
5. agent-url-parser
6. html-conformance-harness
7. agent-cli-client
8. agent-page-path-spec
9. html-conformance-gate
10. html-tokenizer-markup

html-input-stream and agent-session-process come next.

Why this order:
- Slots 1 and 3 are the heads of the two chains that make up the critical path (html-decisions → … → runner-e2e is 23 long).
- runner-decisions goes early because it needs an operator approval-record, and waiting for that approval should overlap with other work.
- Slots 4 and 5 are leaf libraries with no inbound edges. They fill parallel slots without touching shared files.
- Slots 6–10 are each the first story whose predecessors are done.

If finding 7 is taken, the redirect spec story goes in at slot 8 and the three net stories leave the critical path.

**What I read**

I read 47 stories, 3 epics, `docs/design/agent-interface.md` and the critic procedure and rubric. Commands: `aep plan artifact relations`, `graph`, `validate` (valid), `waves`, plus `tsort` over the 94 story-level depends_on edges (no cycle) and an awk longest-path pass. I walked all 86 in-set story edges and both epic edges, and went outside the set to the first plan's 8 edges. I traced 22 acceptance reads to a producer in the set; edges record 17 of them. The other 5 are findings 1, 2, 3 and 6 (two reads).

**What I could not establish**

- Whether the 13-step parser chain could be shorter. Its edges give shared files as the reason (`tokenizer.rs`, `tree_builder.rs`), so I did not report it.
- Parallel-safety lane:
  - agent-page-path-spec and html-conformance-gate both write `Taskfile.yml` and `AGENTS.md`, with no edge between them.
  - runner-record-option, runner-detail-option and runner-record-session all write `cli/surf-agent.yaml` and `crates/session`, and none of them is ordered against another.
- Acceptance/scope lane: some sentences are out of date.
  - "neither epic claims `--detail`": agent-change-and-log.md:67-68, runner-detailed-changes.md:58-59.
  - html-document-spec.md:94 says "URL parsing, which no epic of this set takes on", but agent-url-parser takes it on.
- Whether agent-session-spec is too large for one implementor pass. Size is outside my lane except where it hides two things in one story (finding 8).

```findings
- file: .engineering/planning/story/runner-formats.md
  line: 24
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its grammars embed the answer's element and changed lines that agent-follow-links, agent-fill-pick and agent-change-and-log define later (agent-change-and-log deciding a changed form the design does not have), and no depends_on edge runs from it to agent-change-and-log"
- file: .engineering/planning/story/runner-formats.md
  line: 41
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the exchange block keys on method and body and records no response URL, while method and body come from agent-request-body-spec and the post-redirect response URL from agent-links-spec, and no edge orders runner-formats after them"
- file: .engineering/planning/story/runner-detail-option.md
  line: 50
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance expects fill --detail to show an attribute-level line, but no story changes the tree on fill and html-document-port gives the tree no mutation path, so the acceptance reads state nothing produces"
- file: .engineering/planning/story/runner-detail-option.md
  line: 39
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it writes a second grammar and renderer for the detailed line in docs/agent-answer.md and crates/answer beside those runner-formats and runner-detailed-changes define, with a different note text, so the detailed line has two owners"
- file: .engineering/planning/story/runner-record-session.md
  line: 39
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it and runner-detail-option both wire the detailed-line computation into the session core per action with no edge between them, so one story must own the hook and the other depend on it"
- file: .engineering/planning/story/runner-surf-cli-package.md
  line: 48
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "it moves the surf binary and its binary tests out of package surf with no edge to agent-request-body-spec, agent-fetch-post, agent-fetch-redirects or html-document-port, whose bodies and acceptances name crates/surf/src/main.rs and cargo test -p surf, so the binary has two homes depending on wave order"
- file: .engineering/planning/story/agent-links-spec.md
  line: 12
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it holds Follow and the net redirect model in one story, and its shared-file edge to html-document-port puts agent-fetch-redirects, agent-request-body-spec and agent-fetch-post, which never read the tree, on the critical path behind the whole parser; split the redirect half before html-document-spec or keep the order knowingly"
- file: .engineering/planning/story/agent-session-spec.md
  line: 89
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it carries cli/surf-agent.yaml and task cli whose only reader is agent-cli-client, so the client waits on the hardest story of the plan; the command-line document and task are a separate story"
- file: .engineering/planning/story/html-conformance-gate.md
  line: 11
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its depends_on agent-session-spec, recorded for shared Taskfile.yml and AGENTS.md, puts the 13-story parser chain behind the largest agent story; reversing the edge or splitting the AGENTS.md edits are the two options"
- file: .engineering/planning/epic/agent-cli-target.md
  line: 9
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the epic depends_on epic:html-parser while html-conformance-gate, html-document-spec and html-document-port depend on agent-cli-target stories, so the epic edge states an order its stories contradict"
- file: .engineering/planning/epic/html-parser.md
  line: 39
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the outcome says the node tree is reachable by the agent component's port, while html-decisions proposes and html-document-port, agent-follow-links and agent-change-and-log build the session process reading it through Browser::document_tree"
- file: .engineering/planning/story/agent-form-port.md
  line: 41
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it computes the document base URL in crates/surf while agent-follow-links computes the same base URL in crates/session with no edge between them, so one story must own the base URL and the other depend on it"
- file: .engineering/planning/story/agent-forms-spec.md
  line: 61
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it adds option selectedness and a command that sets it but does not say whether that command updates the select element's value field that html-document-spec specifies and html-actionable-elements computes, so the selected option has two fields and two writers"
```
