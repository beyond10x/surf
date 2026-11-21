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
| 2 (delivered, [page](agent-wave-2.md)) | `agent-cli-client`, `agent-session-spec`, `html-conformance-harness`, `view-http-server` |
| 3 (delivered, [page](agent-wave-3.md)) | `agent-page-path-spec`, `agent-surfd-supervisor`, `runner-surf-cli-package` (moved from wave 2 after D32: it shares `crates/surf/Cargo.toml` with `agent-session-spec`), `view-page-prototype` (added in wave 2: the page on example data, so that it can be opened before `surfd` exists) |
| 4 (delivered, [page](agent-wave-4.md)) | `agent-redirect-spec`, `agent-session-process`, `html-conformance-gate`; then `release-agent-binaries`, alone |
| 5 (delivered, [page](agent-wave-5.md)) | `agent-session-events`, `html-input-stream`, `html-tokenizer-markup` |
| 6 (delivered, [page](agent-wave-6.md)) | `html-document-spec`, `html-tokenizer-text`, `view-surfd-serves` |
| 7 (delivered, [page](agent-wave-7.md)) | `agent-fetch-redirects`, `html-character-references`, `view-first-page` |
| 8 (delivered, [page](agent-wave-8.md)) | `agent-request-body-spec`, `html-tree-body`, `view-detail-panel` |
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

- After wave 3: the view's page on example data (`story:view-page-prototype`), served by an
  example program. The coordinator opens it in Brave.
- After wave 4: `surf-agent open`, `go`, `look` and `close` against `surfd` and one session
  process per session. The operator gets a "to try" list here.
  **Checkpoint** (the operator, 2026-11-21): release `0.2.0` follows this wave, with `surf`,
  `surf-agent`, `surfd` and `surf-session` (`story:release-agent-binaries`, after
  `agent-session-process`). Work then moves to the script engine, its own repository; the waves
  after 4 stay planned here and are not scheduled.
- After wave 7: the first live view (`story:view-first-page`): the sessions, the page line and
  the timeline, following live. The coordinator opens it in Brave with the address in
  `$XDG_RUNTIME_DIR/surf/view-url`.
- After wave 11: a real page's elements come from the HTML parser (`story:html-document-port`).
- After wave 18: links, forms, history and the `changed` block.
- After wave 22: record a session, make a scenario from it, run it against the replayed network.

## Accepted on 2026-11-21, behind the planned waves

The operator accepted every draft on 2026-11-21 ("Accept all your drafts"). These stories
are active in the store and are taken up after the planned waves, or earlier where a wave
touches their files anyway: `agent-answer-paging`, `agent-close-window`,
`agent-open-tab-caller-id`, `agent-token-hash`, `invalid-address-ending`,
`random-source-failure-cause`, `runner-detail-option`, `runner-detailed-changes` (with
`epic:detailed-mode`), and four from the wave records: `agent-file-open-race`,
`agent-surfd-session-limit`, `agent-surfd-idle-exit-race`, `view-stream-string-example`.
`release-agent-binaries` runs after wave 4, alone (the checkpoint above).

## ESS findings, settled on 2026-11-21

The wave records list findings "for ESS" that waited for the operator's word. He gave it on
2026-11-21, with the limit that only defects are filed: a documented boundary of ESS is not
an issue. Each of the 15 collected findings (12 from this repository, 3 from the script
engine's) was reduced to a model written for it and run on ess 0.96.0.

Filed in `beyond10x/ess` by the organization bot, each reproduction run a second time by the
coordinator:

- [#793](https://github.com/beyond10x/ess/issues/793): the Rust target writes a `behaviors`
  field that nothing reads for a component with no command and no view.
- [#794](https://github.com/beyond10x/ess/issues/794): synthesis sends the step that arranges
  a state as a caller that the command's caller guard refuses, and the interpreter fails the
  scenario.
- [#795](https://github.com/beyond10x/ess/issues/795): ESS-SYNTH-014 names a scenario the
  suite holds and passes, and a `when_related` refusal drops the order check.

Defects that ESS already tracks as active stories in its own planning store, not filed again:
a binding that maps a required event field into an `Optional` command input is refused by the
Rust target (`rust-binding-wraps-a-plain-copy-into-an-optional-member`), and the generated
Rust fails `cargo clippy -- -D warnings`
(`the-rust-crate-layout-is-rustfmt-and-clippy-clean`).

Limits, each documented or refused with a clear text, not filed: the `ess-cli/2` sources and
argument types, `ess-http/1` with in-process components, a one-time secret type deriving
`Debug` (ESS has no construct that marks a type secret), a caller without the attribute a
guard reads (refused at validation as `ESS-AUTHOR-014`), no scenario for a new enum variant
that no branch reads, `refs:` with a provider name ESS does not resolve, and from the script
engine: ESS-SYNTH-023 on records the system creates, `refs:` refusing a URL, and `Binary64`
on the Rust target.

Not reproduced: the refusal of a command applied per element. The `via:` form synthesizes
(8 scenarios, 0 refusals); only the row-set form of `when_related` gives ESS-SYNTH-001.

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
