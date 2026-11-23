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
| 9 (delivered, [page](agent-wave-9.md)) | `agent-fetch-post`, `html-actionable-elements`, `html-tree-formatting` |
| 10 (delivered, [page](agent-wave-10.md)) | `html-accessible-name`, `html-parser-limits`; then `one-test-binary-per-crate`, alone, on `main` after the merges of release 0.3.0, `fix/tests-hold-under-load` and this wave; round 1 (the rounds after wave 10) starts after its merge |
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

## Rounds after wave 10

The operator, 2026-11-22, on the twelve waves after wave 10: "why don't we, you know, just
implement all of those like in one go? I mean, we have tests and we should be able to, you
know, just spit this out in one big one shot". The table above is cut by depth: one chain of
twelve `depends_on` edges (`html-document-port`, `agent-links-spec`, `agent-forms-spec`,
`agent-form-port`, `agent-fill-pick`, `agent-press-submit`, `agent-back-forward`,
`agent-change-and-log`, `runner-formats`, `runner-record-option`, `runner-record-session`,
`runner-e2e`) gives twelve waves, each with its own branch, gate, record and publication. The
edges are real (a port needs its specification, a command needs the document), but a unit can
build a whole chain in order: what must come first is built first inside the unit.

The coordinating agent's decision (D78): after wave 10 the 24 stories of waves 11 to 22 are
built in three rounds. A round is one wave branch, one gate, one record, one review and one
publication; its stories keep their identities, statuses and acceptance in the store.

| round | units, each one sub-agent building its stories in the order given | review after it |
|---|---|---|
| 1 | parser: `html-tree-tables`, `html-tree-template`, `html-tree-foreign`, `html-tree-complete`. Text: `page-text-answer-block`, `page-text-outline`, `page-text-markdown`, `page-text-tables`. Document: `html-document-port`, `page-text-look`, `page-text-surf-command`, `agent-links-spec`, `agent-forms-spec` | the parser on real pages |
| 2 | commands: `values-and-labels-before-fill`, `agent-follow-links`, `agent-form-port`, `agent-fill-pick`, `agent-press-submit`, `agent-history-spec`, `agent-back-forward`, `agent-change-and-log`. View, from when `agent-follow-links` is in its base: `view-now-panel`, `view-acceptance` | links, forms, history |
| 3 | runner: `runner-formats`, `runner-record-option`, `runner-replay-transport`, `runner-scenario-from`, `runner-record-session`, `runner-run`, `runner-e2e`. View: `view-controls-and-changes` | the runner |

Rules of a round, beside the rules of a wave:

- Before a round starts, each of its stories is scoped and the units of the round must not
  collide (`aep plan artifact waves`); two units that write the same file become one unit or
  run one after the other.
- The parser unit of round 1 is held to the vendored html5lib-tests: the line
  `html-conformance: … passed` of the gate rises with each story and never falls.
- Round 1 has no specification unit of its own (corrected on 2026-11-22 after the critic panel
  on the page text drafts): `agent-links-spec` depends on `html-document-port`, every run of
  `task generate` rewrites all files under `generated/` with one contract digest, and
  `agent-history-spec` depends on `agent-form-port`, a story of round 2. So the document unit
  makes every specification change of round 1, one after the other, and `agent-history-spec`
  is built by the commands unit of round 2 after `agent-form-port`. Scenarios whose code comes
  in round 2 are counted with `REFUSALS_MAX`, as in a specification unit of a wave.
- Units of round 1 hand over at three points: `page-text-answer-block` starts on the round's
  base and `page-text-outline` follows it in the text unit (wave 10 holds the parser's limits
  and the labels); `page-text-tables` waits for the parser
  unit's `html-tree-tables`; `page-text-look` waits for `page-text-answer-block`,
  `page-text-outline` and `page-text-markdown`, and `page-text-surf-command` for
  `page-text-tables`. The coordinator merges the finished story's commits into the waiting
  unit's branch.
- Round 2, decided on 2026-11-22: its nine stories were revised that day against round 1 as
  built (two scopers on `main` at `c5c2895`), and `values-and-labels-before-fill` was drafted
  from the "Left open" lines of round 1's As built sections. The commands unit is one sub-agent
  that builds its stories one after the other in the row's order, `values-and-labels-before-fill`
  first, and commits each when it is green. `agent-fill-pick` and `agent-press-submit` come
  before `agent-history-spec` because the round's measure is `agent-tasks`, whose tasks `link`
  and `search` wait for `agent-follow-links` and for `agent-fill-pick` with
  `agent-press-submit` (`crates/agent-tasks/status.txt`); no `depends_on` edge forbids that
  order. The view unit (`view-now-panel`, then `view-acceptance`) starts when
  `agent-follow-links` is in its base and runs beside the commands unit: no command story after
  `agent-follow-links` writes `crates/surfd/src/view.rs` (a story that adds a refusal writes
  `crates/surfd/src/daemon.rs`, `agent-change-and-log` writes `crates/surfd/tests/`).
- `agent-tasks` runs beside round 1 as a fourth unit and starts when `page-text-answer-block`
  is in its base (its judge tests build answers with `surf_answer::text_page`).
- Every command of a round follows the pool's rules: every build and test runs through the
  build pool; the pool ends a job after 10 minutes, and a command that needs longer names its
  time with `slot --timeout <t>` or `SLOT_TIMEOUT=<t>`, at most 30 minutes; a script that
  measures or waits starts without the `slot` prefix. A unit runs the tests of the packages it
  touched; the coordinator runs the gate once after the round's units, as
  `slot --timeout 25m task check-runner`.
- A unit commits on its branch after each story whose tests are green, so that a story that
  fails leaves the stories before it usable.
- The test budget holds for a round as for a wave: no test over 60 s, the test step within
  120 s, a unit times its tests before it reports.
- A release can follow each round: the list "What works after which wave" names the same
  three points (after wave 11, 18 and 22).
- The website plays a recorded agent session (`website/data/surf-agent.terminal.json`, on
  "Drive surf from an agent"). The recording of release `0.3.0` shows no title and no content,
  because no command parses a page. The operator, 2026-11-22 (D79, option A): keep it for
  `0.3.0` and record it again when `html-document-port` has landed. Round 1 ends with that
  recording, made with the binaries of the round's last commit, and with the caption under the
  terminal changed to what an answer then holds. It was recorded again with release `0.4.0` on
  2026-11-22; an answer still carries no title, because the session sets none
  (`story:agent-follow-links` sets it), and the next recording follows that story.

### TLS beside the rounds

`epic:tls-https` and its thirteen stories are active since 2026-11-22 (four plan critics, seven
rounds). They are not a round of their own:

- Nine stories write only `crates/tls*`, `taskfiles/tls-*`, the lock check and its test, and
  `.cargo/config.toml`. They are built beside rounds 1 and 2, in this order: `tls-decisions`
  alone (a design note and a build probe on the forge); `tls-lock-allow`, after
  `one-test-binary-per-crate`; then `tls-x509`, `tls-test-certs` and `tls-record` side by side;
  then `tls-x509-path`, `tls-anchors` and `tls-handshake` side by side; then `tls-test-server`.
- `tls-spec`, `tls-fetch-https`, `tls-session-anchors` and `tls-release` are one unit, in that
  order. It starts at the first boundary between two rounds after the nine are merged, and
  never beside a unit that writes `spec/`, `generated/`, `crates/fetch`, `crates/session`,
  `crates/agent`, `crates/surfd`, `crates/surf/src/lib.rs`, `conformance/src/target.rs` or
  `docs/agent-answer.md`.
- The coordinator writes for the nine what they share: the `mod` lines of `crates/tls/src/lib.rs`
  and `tests/main.rs`, the root `Taskfile.yml`, `Cargo.lock`, `README.md` and `AGENTS.md`.
- The ClientHello offers the three TLS 1.3 suites and nothing else (RFC 8446 section 9.3). A
  server limited to TLS 1.2 fails with `TlsVersionUnsupported` when it reads
  `supported_versions`, and with `TlsHandshakeFailed` and a detail that says so when it does
  not. A TLS 1.2 epic follows this one and is not drafted yet.

## What an agent needs from surf

The operator, 2026-11-22: "I wonder what is needed to actually make [surf] useful for agents,
because right now I think it isn't." At release `0.3.0` that holds: an answer names a page's
status, media type and address and nothing of its content, and only `http:` loads.

The coordinating agent's decision (D85): the work after wave 10 is ordered by this list. Each
item says what an agent can do after it and where it is planned.

1. Read a page. The title and the element rows (`html-document-port`), the page as a tree and
   as Markdown (the stories `page-text-*` of round 1). Round 1. A long answer in parts
   (`agent-answer-paging`) is deferred and comes after round 1 (the coordinating agent's
   decision D129, 2026-11-22: the text block of round 1 is bounded by its own limits, so no
   answer needs paging yet).
2. Reach the web. TLS 1.3 and `https:`: the README's own line is "most of the web is not
   reachable without it". It was on the list after the plan with no date. An epic is drafted
   beside round 1 (branch `plan/useful-for-agents`), with the cryptographic primitives from
   audited code as principle 4 of the README plans it. It is built beside round 2: it writes
   `crates/fetch` and new crates, round 2 writes the session and the client.
3. Move through a site. Links, forms, history: round 2.
4. Stay signed in. Cookies: not planned (`spec/domains/net.yaml` lists them as unmapped). A
   story after the forms.
5. Pages that build their content by script: `epic:scripts-in-a-page`, after round 1, on a tag
   of the script engine that holds the fixes of the security review of 2026-11-22.
6. Be called without reading the manual: a skill or tool definition for agents that ships with
   a release. Not planned; a draft on the same planning branch.

Item 1 in stories (D88), drafted on 2026-11-22 with the design note `docs/design/page-as-text.md`
(branch `plan/page-as-tree-and-markdown`): `page-text-answer-block` (the `text` block of the
answer), `page-text-outline`, `page-text-markdown` and `page-text-tables` (a new crate
`crates/page-text`, built one after the other by one unit of round 1, beside the parser unit),
then `page-text-look` (`surf-agent look --as tree|markdown`) and `page-text-surf-command`
(`surf <url> --as`). The planner put the last two into round 2 because they write files of the
session, the client and `crates/surf-cli/src/main.rs`. They move into round 1 as the document
unit's stories after `html-document-port`, which writes the same files: the recording that ends
round 1 (D79) shows a page's content only with them.

Round 3 (the runner) serves surf's own tests and no agent's task. It moves behind items 2 and 4.

The measure is a set of tasks, drafted as a story on the planning branch: an agent that has
`surf-agent` and nothing else answers a question whose answer is on a named page (read an
article, read a table, go from an index to a page by a link, send a search form, a page behind
`https:`). Each release record says how many of them pass. Release `0.3.0` passes none: no
answer holds page content.

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
  wave branch, from the units' reports (D7, D16, D30). In a round, a specification unit runs
  `slot task suite REFUSALS_MAX=<n>` and the tests of its packages; the coordinator writes the
  last `<n>` of the round into `Taskfile.yml` and runs the gate.
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
