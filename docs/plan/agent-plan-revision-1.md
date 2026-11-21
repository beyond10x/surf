# The agent interface plan: revision after critic round 1

Round 1 of the plan critic panel read `epic:agent-cli-target`, `epic:html-parser` and
`epic:e2e-scenario-runner` with their 47 draft stories at `c6a66a2`. Eight critics, eight
`needs-revision`, 68 findings (22 blockers). Each report is stored as written:

| critic | epics | findings | record |
|---|---|---|---|
| scope | agent | 3 | `review-result:agent-plan-critic-scope-agent-round-1` |
| scope | parser | 2 | `review-result:agent-plan-critic-scope-html-round-1` |
| scope | runner | 3 | `review-result:agent-plan-critic-scope-runner-round-1` |
| acceptance | agent | 13 | `review-result:agent-plan-critic-acceptance-agent-round-1` |
| acceptance | parser | 19 | `review-result:agent-plan-critic-acceptance-html-round-1` |
| acceptance | runner | 8 | `review-result:agent-plan-critic-acceptance-runner-round-1` |
| design | all three | 13 | `review-result:agent-plan-critic-design-round-1` |
| parallel safety | all three | 7 | `review-result:agent-plan-critic-parallel-round-1` |

A finding inside one epic is revised by that epic's decomposer. A finding at the seam between
epics needs one answer for all of them; those answers are below. They are the coordinating
agent's, taken on 2026-11-21, reversible, and the operator can overrule each.

## Decisions

**D1. Who reads the node tree.** The session process reads it through `Browser`
(`Browser::document_tree`). No component's port receives the tree: a component holds its own
rows, and the tree is the `document` component's. `epic:html-parser`'s Outcome line is
rewritten to say so. (scope-html 1, design 11)

**D2. A document's tree does not change before scripts exist.** Parsing builds it; nothing
mutates it. What an agent changes on a page is control state: a control's value, checkedness
and selectedness. The HTML Standard keeps that state apart from the attributes, and so does
surf: it lives in the specification's control rows, not in the tree. So before the script
engine, the `changed` block of a same-document action holds control-state lines and nothing
else, and "a node identity stable across changes within one document" has nothing to be stable
across. (scope-html 2, scope-runner 2, design 3)

**D3. The detailed mode waits for scripts.** A class gained, an attribute set, a style changed:
each needs a script (or CSS) to happen. Before that, a detailed mode could only be tested on
states a test builds by hand, and two critics found its stories asserting changes no story
produces. `story:runner-detailed-changes` and `story:runner-detail-option` leave
`epic:e2e-scenario-runner` and this plan. They move to a new draft `epic:detailed-mode`, which
comes after the script engine. The recording format reserves a block kind for detailed lines so
that it does not have to change version when they arrive. `epic:agent-cli-target`'s Out of
scope names `--detail` and points there. This removes the double owner of the detailed line's
grammar, the two stories wiring one hook into the session core, and their file collisions.
(scope-agent 3, scope-runner 1 and 2, acceptance-runner 7 and 8, design 3, 4 and 5,
parallel 2)

**D4. Redirects leave the links story.** `story:agent-links-spec` held `Follow` and the
redirect model. The redirect half (redirects inside one request, the response address,
`TooManyRedirects`) becomes a story of its own, ordered before any story that needs the node
tree. `agent-fetch-redirects`, `agent-request-body-spec` and `agent-fetch-post` follow it and
no longer wait for the parser. (design 7, parallel 7)

**D5. The command-line document leaves the session specification story.**
`cli/surf-agent.yaml` and the task `cli` become a story of their own that
`agent-cli-client` depends on, so the client does not wait for the hardest story of the plan.
(design 8)

**D6. The `surf` binary moves first.** `story:runner-surf-cli-package` (the binary into its own
package `crates/surf-cli`) depends on `runner-decisions` only and runs early. Every story that
writes the binary or its tests depends on it and names `crates/surf-cli`:
`agent-fetch-redirects`, `agent-request-body-spec`, `agent-fetch-post`, `agent-form-port`,
`html-document-port`. The story moves to `epic:agent-cli-target`, whose stories are the first
to need it. (design 6, parallel 1)

**D7. `AGENTS.md` and `README.md` are the coordinator's in a wave of more than one unit**, as
the root `Cargo.lock` already is. A unit writes the sentences it needs for them into its report
under a heading "For AGENTS.md" or "For README.md", naming the section; the coordinator writes
them on the wave branch. A story still says which sections it needs changed. In a wave of one
unit the unit writes them itself, as in the first plan. (parallel 3, and the many pairs the
parallel critic lists as "different sections")

**D8. `Taskfile.yml` stays ordered by edges.** `html-conformance-gate` keeps its edge to
`agent-session-spec` and gains one to `agent-page-path-spec`. Both edges cost no wave: the gate
story sits at depth 3 behind `html-decisions` and `html-conformance-harness` anyway. The risk
the design critic names is accepted knowingly: if `agent-session-spec` slips, the parser chain
waits at its third story. (design 9, parallel 4)

**D9. The epic-level edge goes.** `epic:agent-cli-target depends_on epic:html-parser` is
removed; three parser stories depend on agent stories, so the edge stated an order the stories
contradict. The epic says at step level what needs what. (design 10)

**D10. One owner of the document base URL.** A function in `crates/page`, written by
`agent-follow-links`; `agent-form-port` depends on that story and calls it. (design 12)

**D11. A select element stores no value.** The stored fact is each option's selectedness. What
an answer prints as a select's value is derived from it. `html-document-spec` and
`html-actionable-elements` drop the stored value for `select`; `agent-forms-spec`'s command
sets selectedness only. (design 13)

**D12. Parser tests where the parser is.** `html-actionable-elements` keeps its place after
`html-tree-body`. Its two cases that read `select` and `template` parsing move to
`html-tree-tables` and `html-tree-template`, each of which adds its case to `crates/page/tests`
and depends on `html-actionable-elements`. `agent-form-port` depends on `html-tree-tables`.
`html-document-port` stays where it is, behind `html-tree-formatting` and `html-parser-limits`:
real pages are parsed by an incomplete tree builder until the tree chain ends, the answer says
so in a `note` line, and the conformance gate's counts say how incomplete. (parallel 5 and 6,
the scope and acceptance critics' notes on `html-document-port`)

**D13. `fill <handle> -` stays**, recorded as a decision the operator can overrule, and the
epic gains the constraint it serves: a secret value is never an argument word. (scope-agent 2)

**D14. The recording carries the response address.** The `exchange` block of the recording
format gains the address the response was served from, and `runner-formats` depends on
`agent-request-body-spec` and on `agent-change-and-log`, whose lines its grammars embed.
(scope-runner 3, design 1 and 2)

**D15. Deferred things get a carrier.** One backlog draft each, tagged `deferred`, in no wave:
hashing the token (waits for the choice of the cryptographic crate), paging a long answer
(`--max-lines`, `look --from`), a caller-chosen `tab_id` on `OpenTab`, and `CloseWindow`.
(scope-agent, "what I could not establish")

## What the revisers do

Each decomposer revises its own epic's stories to its critics' findings and to the decisions
above, with `aep plan artifact body`, `relate` and `unrelate`, and creates the stories D4, D5
and D15 name. The acceptance findings are theirs in full: every acceptance names a number or a
named case that a command prints, and none can be met by a stub, by lowering an expected count,
or because it is already true on `main`.

Round 2 of the panel reads the revised set. The panel has two rounds; what round 2 still finds
goes to the operator with the set.

## After round 1: the daemon and the live view

On 2026-11-21 the operator proposed `surfd` and a live view, the design was amended (sections
3 and 12), and he accepted the whole plan for implementation
(`approval-record:agent-plan-accepted-2026-11-21`). The stories changed again before round 2:
`story:agent-surfd-supervisor` and `story:agent-session-events` are new, the session process
and the client were revised, and `epic:live-view` has eight stories (`view-*`). Three more
decisions of the coordinating agent:

**D16. New tasks get a file of their own.** A story that adds a task writes it to
`taskfiles/<name>.yml`; the root `Taskfile.yml` includes it. In a wave of more than one unit
the root `Taskfile.yml` (its include list, the order of `check`, `REFUSALS_MAX`) is written by
the coordinator from the units' reports, like `AGENTS.md`, `README.md` and the root
`Cargo.lock` (D7). The specification stories, which change `REFUSALS_MAX`, already sit in one
chain. Reason: the view stories and the agent stories both add tasks at the same depth, and
ordering them with edges would put the view behind the network stories or the reverse.

**D17. The kept log holds each action's answer.** `story:agent-session-events` keeps a third
kind of entry: the typed answer of each action, which is what gives the live view its `now` and
`changed` panels.

**D18. The page's own files are served without the view token.** They hold no data. The token
arrives in the fragment of the address `surfd` prints, which a browser sends to no server, and
every route that answers with data requires it. `epic:live-view`'s constraint says so.
