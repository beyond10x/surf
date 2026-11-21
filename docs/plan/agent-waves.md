# The agent interface plan: waves

The second plan of surf: `epic:agent-cli-target`, `epic:html-parser`,
`epic:e2e-scenario-runner` and `epic:live-view`, 58 stories, accepted by the operator on
2026-11-21 (`approval-record:agent-plan-accepted-2026-11-21`) and moved to `active` after two
rounds of the plan critic panel (`docs/plan/agent-plan-revision-1.md`,
`docs/plan/agent-plan-revision-2.md`). The first plan is `docs/plan/waves.md`.

Not in this plan: `epic:detailed-mode` and its two stories (after the script engine), four
drafts tagged `deferred`, and the backlog drafts `story:invalid-address-ending` and
`story:random-source-failure-cause`.

## How the waves are cut

A story's depth is one more than the longest chain of `depends_on` edges below it among the
stories not yet implemented. A wave is the stories of one depth. Before a wave starts, each of
its stories is scoped (`story-scoper`, typed scope in the store) and
`aep plan artifact waves --kind story --status active` must report the wave without a
collision; a story that collides moves to the next wave. The table below is the cut by depth on
2026-11-21 at 58 active stories; it moves when a story slips.

| wave | stories |
|---|---|
| 1 (delivered, [page](agent-wave-1.md)) | `agent-answer-format`, `agent-cli-spec`, `agent-secret-primitives`, `agent-url-parser`, `html-decisions`, `runner-decisions`, `view-decisions` |
| 2 | `agent-cli-client`, `agent-session-spec`, `html-conformance-harness`, `view-http-server` |
| 3 | `agent-page-path-spec`, `agent-surfd-supervisor`, `runner-surf-cli-package` (moved from wave 2 after D32: it shares `crates/surf/Cargo.toml` with `agent-session-spec`) |
| 4 | `agent-redirect-spec`, `agent-session-process`, `html-conformance-gate` |
| 5 | `agent-session-events`, `html-input-stream`, `html-tokenizer-markup` |
| 6 | `html-document-spec`, `html-tokenizer-text`, `view-surfd-serves` |
| 7 | `agent-fetch-redirects`, `html-character-references`, `view-first-page` |
| 8 | `agent-request-body-spec`, `html-tree-body`, `view-detail-panel` |
| 9 | `agent-fetch-post`, `html-actionable-elements`, `html-tree-formatting` |
| 10 | `html-accessible-name`, `html-parser-limits` |
| 11 | `html-document-port`, `html-tree-tables` |
| 12 | `agent-links-spec`, `html-tree-template` |
| 13 | `agent-follow-links`, `agent-forms-spec`, `html-tree-foreign` |
| 14 | `agent-form-port`, `html-tree-complete`, `view-now-panel` |
| 15 | `agent-fill-pick`, `agent-history-spec`, `view-acceptance` |
| 16 | `agent-press-submit` |
| 17 | `agent-back-forward` |
| 18 | `agent-change-and-log` |
| 19 | `runner-formats`, `view-controls-and-changes` |
| 20 | `runner-record-option`, `runner-replay-transport`, `runner-scenario-from` |
| 21 | `runner-record-session`, `runner-run` |
| 22 | `runner-e2e` |

## What works after which wave

- After wave 4: `surf-agent open`, `go`, `look` and `close` against `surfd` and one session
  process per session. The operator gets a "to try" list here.
- After wave 7: the first live view (`story:view-first-page`): the sessions, the page line and
  the timeline, following live. The coordinator opens it in Brave with the address in
  `$XDG_RUNTIME_DIR/surf/view-url`.
- After wave 11: a real page's elements come from the HTML parser (`story:html-document-port`).
- After wave 18: links, forms, history and the `changed` block.
- After wave 22: record a session, make a scenario from it, run it against the replayed network.

## Rules of a wave in this plan

Beside `AGENTS.md` "Planning and waves" and `docs/plan/unit-invariants.md`:

- In a wave of more than one unit, `AGENTS.md`, `README.md`, the root `Taskfile.yml`
  (`REFUSALS_MAX` included) and the root `Cargo.lock` are written by the coordinator on the
  wave branch, from the units' reports (D7, D16, D30). A specification unit runs its gate as
  `slot task check REFUSALS_MAX=<n>`.
- Every unit is implemented by a sub-agent on Opus; the coordinating session writes no code.
- One adversary or security review per larger piece, not per unit: after wave 4 (the daemon,
  the session process, the token), after wave 7 (the view's server and page), after wave 11
  (the parser on real pages), after wave 18 and after wave 22. A review's findings are fixed
  before the next wave starts.
- A decision story (`html-decisions`, `runner-decisions`, `view-decisions`) ends in a design
  note. The operator's word of 2026-11-21, "accept all of them, put it into stories or epics,
  whatever, and implement it", is read by the coordinating agent as covering the notes this
  plan's decision stories write; each note's decisions are reported to him when it lands, and
  he can overrule each.
