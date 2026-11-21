---
format: aep.planning-md/3
id: review-result:agent-plan-critic-parallel-round-2
kind: review-result
status: active
title: Plan critic (parallel safety), round 2, the four agent interface epics
relations:
- reviews: epic:agent-cli-target
- reviews: epic:html-parser
- reviews: epic:e2e-scenario-runner
- reviews: epic:live-view
revision: 1
---
needs-revision

No two stories write one file without being ordered, except the root `Taskfile.yml`, but 10 stories still write that file themselves against D16, and two pairs collide through generated or shared state they don't name.

**What I read:** 57 stories and the 118 `depends_on` edges between them, plus the eight round-1 reports, the D1 to D18 decisions, `AGENTS.md` "Planning and waves", `Cargo.toml`, `crates/surf/src/main.rs` and `spec/domains/net.yaml`. Commands: `aep plan artifact list --kind story --status draft`, `aep plan artifact show review-result:agent-plan-critic-parallel-round-1`, `aep plan artifact graph`, `aep plan artifact waves --kind story --status draft`, and every body under `.engineering/planning/story/`. I worked out depth, which stories are ordered (directly or through others) and the surface overlaps with awk.
- **The store moved while I read.** The brief pins `497dedd`, but the worktree HEAD is now `d382c5e` ("scope of the six first-wave stories"). That commit adds Scope sections to the six depth-0 stories. I judged the bodies; `d382c5e` changes nothing below except `agent-cli-spec`.
- **Surfaces:** 57 cited, 0 inferred, 0 unplaceable. Every story names a surface.
- **Acceptance reads:** 53 traced to a producer in the set. 47 have a `depends_on` path to it; 6 do not (findings 1 and 2).

**Round-1 findings, all seven answered:**
1. `runner-surf-cli-package` against the binary writers: **answered.** D6 gives edges from `agent-fetch-redirects`, `agent-request-body-spec`, `agent-fetch-post`, `agent-form-port` and `html-document-port`, and the body names them (`runner-surf-cli-package.md:17-21`). There is a new edge to `agent-cli-spec` for `crates/surf/Cargo.toml`, and the acceptance no longer reads a test total.
2. `runner-detail-option`: **answered by removal.** D3 moved it to `epic:detailed-mode`, which is not in this set.
3. `html-conformance-gate` and `agent-session-process` on `AGENTS.md`: **answered by D7.** Both hand their sentences over (`html-conformance-gate.md:34`, `agent-session-process.md:85`).
4. `html-conformance-gate` and `agent-page-path-spec` on `Taskfile.yml`: **answered by an edge** (D8, `html-conformance-gate.md:32-34`). The gate now contradicts D16 instead (finding 3).
5. `html-actionable-elements` reading select and template parsing: **answered.** The two cases moved to `html-tree-tables.md:41` and `html-tree-template.md:39`, each with an edge to `html-actionable-elements`.
6. `agent-form-port` needs `html-tree-tables`: **answered** by the edge in `agent-form-port.md:15`. `agent-fill-pick` reaches it through `agent-form-port`.
7. `agent-request-body-spec` and `agent-fetch-post` on `crates/session`: **answered.** Both name it (`agent-request-body-spec.md:25-27,59`, `agent-fetch-post.md:22-24,45`), and `agent-follow-links` is behind both through `html-document-port`.

## New findings

**1. `story:agent-session-events` — its case 12 reads the set of `SystemEvent` variants, which `html-document-spec` changes in the same wave with no edge between them.** Citations: `.engineering/planning/story/agent-session-events.md:83`, `.engineering/planning/story/html-document-spec.md:72`.
- Quoted: "12. every variant of `SystemEvent` on main maps to a record with its fields, none `unmapped`" and "every event published on main when this story starts is mapped (a test lists them)" (`:54`).
- What is wrong: `html-document-spec` adds "the commands, outcomes and events of item 2". Both stories are at depth 4, so both are in wave 5. Which one merges first decides whether case 12 can hold.
- `agent-redirect-spec` is also unordered against it. It changes the fields of `ResponseReceived` and `NavigationCommitted` (`agent-redirect-spec.md:41,47`), and only the depth cut puts it in the wave before.
- What would settle it: an edge from `agent-session-events` to `html-document-spec`, or a merge of the two.
  - With the edge, `view-first-page` moves to wave 8.
  - With the reverse edge (`html-document-spec` behind `agent-session-events`), it stays in wave 7.
  - Neither lengthens the plan's longest chain (`runner-e2e`, depth 21).

**2. `story:agent-follow-links` (inferred) — its answer golden texts read `NOT_YET_PARSED`, which `html-tree-foreign` changes in the same wave, and `html-tree-tables` and `html-tree-template` are unordered against it too.** Citations: `.engineering/planning/story/agent-follow-links.md:56`, `.engineering/planning/story/html-tree-foreign.md:39`.
- Quoted: "`note parser: <parts>` when `Browser::parser_not_yet_parsed` is not empty" (`:56`), together with the golden cases at `:72` and the session goldens at `agent-session-process.md:65`.
- What is wrong: both stories are at depth 12, so both are in wave 13. `html-tree-foreign` removes the last entry of the list and runs no `crates/session` test. So the wave branch's `task check` fails on goldens written against the older list.
- Inferred: that the golden texts hold the note line. They do if the test pages are `text/html`, which link pages are.
- What would settle it:
  - an edge from `agent-follow-links` to `html-tree-foreign`, which costs one wave on the longest chain; or
  - golden tests that build the note line from `surf_html::NOT_YET_PARSED` instead of a literal.

**3. Ten stories write the root `Taskfile.yml` themselves, against D16.** Each sits in a wave of more than one unit when cut by depth. What would settle each: a new task goes into `taskfiles/<name>.yml`, and the root include line, the line in `check` and `REFUSALS_MAX` go into the unit's report for the coordinator.

| Story | Quoted | Citation |
|---|---|---|
| `agent-session-spec` | "`Taskfile.yml`: `REFUSALS_MAX` equal to the measured count." | `:75` |
| `agent-page-path-spec` | "`Taskfile.yml` `REFUSALS_MAX`" | `:62` |
| `html-document-spec` | "`Taskfile.yml` `REFUSALS_MAX` raised only with each new refusal listed" | `:87` |
| `agent-request-body-spec` | "`Taskfile.yml` (only if the `refused:` count rises)" | `:59` |
| `agent-links-spec` | "`Taskfile.yml` `REFUSALS_MAX`" | `:47` |
| `agent-forms-spec` | "`Taskfile.yml`" | `:78` |
| `agent-history-spec` | "`Taskfile.yml` `REFUSALS_MAX`" | `:84` |
| `html-conformance-gate` | "`Taskfile.yml`: a task `html-conformance`…; `check` runs it after `test`" | `:60` |
| `view-surfd-serves` | "`Taskfile.yml` (the task of item 2 and its line in `check`)" | `:104` |
| `view-first-page` | "`Taskfile.yml`: a task `view-test`…, added to `check` after `test`" | `:83` |

`agent-cli-spec` is not in the table. Its Scope section at `d382c5e` moves its task to `taskfiles/cli.yml` and hands the root file to the coordinator (`agent-cli-spec.md:127,146-151`). Its Change and Surfaces lines (`:84,91`) still say `Taskfile.yml`.

## Unordered pairs that write one file
- Only `Taskfile.yml`: 12 pairs, each of `html-conformance-gate`, `html-document-spec`, `agent-request-body-spec`, `agent-links-spec`, `agent-forms-spec` and `agent-history-spec` with `view-surfd-serves` and with `view-first-page`. All of them are D16's to resolve, once the stories follow it.
- Every other shared surface is ordered by a `depends_on` path: `crates/surfd/**`, `crates/session/**`, `crates/agent/**`, `crates/answer/**`, `crates/view/**`, `crates/surf/src/lib.rs`, `crates/surf-cli/**`, `crates/page/**`, `cli/surf-agent.yaml`, `docs/agent-answer.md`, `docs/view-stream.md`, `spec/**`, `generated/rust/**`, `conformance/src/**`, `crates/ports/**`, and the parser's `expected/tokenizer.txt`, `tree-construction.txt` and `encoding.txt`.
- The root `Cargo.toml` takes members by the glob `crates/*` (`Cargo.toml:3`), so new crates do not collide there.

## Waves cut by depth

| Wave | Widest set that can run at once | What collides |
|---|---|---|
| 1 (6) | `agent-answer-format`, `agent-cli-spec`, `agent-url-parser`, `html-decisions`, `runner-decisions`, `view-decisions` | No file. Only `Cargo.lock` and `AGENTS.md`, both the coordinator's. |
| 2 (5) | `agent-cli-client`, `agent-session-spec`, `html-conformance-harness`, `runner-surf-cli-package`, `view-http-server` | No file. `agent-session-spec` writes `REFUSALS_MAX` itself (finding 3). |
| 3 (2) | `agent-page-path-spec`, `agent-surfd-supervisor` | No file. `agent-page-path-spec` writes `REFUSALS_MAX` itself (finding 3). |
| 4 (3) | `agent-redirect-spec`, `agent-session-process`, `html-conformance-gate` | No file. The gate writes `Taskfile.yml` itself (finding 3). `agent-redirect-spec` regenerates code under `crates/session` and says so (`agent-redirect-spec.md:31`). |
| 5 (4) | `agent-session-events`, `html-document-spec`, `html-input-stream`, `html-tokenizer-markup` | `agent-session-events` against `html-document-spec` (finding 1). The two parser stories are disjoint, as `html-input-stream.md:21` says. |
| 6 (3) | `agent-fetch-redirects`, `html-tokenizer-text`, `view-surfd-serves` | No file. `view-surfd-serves` writes `Taskfile.yml` itself (finding 3). |

**`view-first-page` lands in wave 7,** beside `agent-request-body-spec` and `html-character-references`. Wave 7 is the first wave where two units, `agent-request-body-spec` and `view-first-page`, both say they write the root `Taskfile.yml`.

Its longest chain: `agent-cli-spec` (wave 1) → `agent-cli-client` (2) → `agent-surfd-supervisor` (3) → `agent-session-process` (4) → `agent-session-events` (5) → `view-surfd-serves` (6) → `view-first-page` (7). Two shorter chains join it: `agent-session-spec` (2) → `agent-page-path-spec` (3) → `agent-session-process`, and `view-decisions` (1) → `view-http-server` (2) → `view-surfd-serves`.

## What I could not establish
- Whether `agent-session-spec`, which changes `crates/surf/src/lib.rs` "only where … forces it", reaches `crates/surf/src/main.rs` or `crates/surf/tests/`. Those are the files `runner-surf-cli-package` moves in the same wave 2. The `agent-cli-spec` scope expects `main.rs:48` to stay as it is.
- Out of my lane (design or acceptance): the `NOT_YET_PARSED` entries contain spaces ("`misnested formatting elements`, `tables and select`…", `html-tree-body.md:44`), but the renderer refuses any part outside `[a-z-]+` (`agent-answer-format.md:52`).
- Out of my lane (design): `surfd` receives rendered "answer lines" over its link (`agent-surfd-supervisor.md:89`). D17 keeps "the typed answer … before it is rendered" while "nothing changes in how a session publishes" (`agent-session-events.md:108-110`). Nothing in the set parses the lines back or carries the typed value.
- Out of my lane (scope): `view-acceptance` writes a page-test count into "The gate's numbers" (`view-acceptance.md:51`), and the unordered `view-controls-and-changes` changes that count. The coordinator writes `AGENTS.md` under D7, so it is not a file collision.

```findings
- file: .engineering/planning/story/agent-session-events.md
  line: 83
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its case 12 reads the set of SystemEvent variants, which html-document-spec changes in the same wave with no edge between them; agent-redirect-spec, which changes event fields, is unordered against it too (html-document-spec.md:72, agent-redirect-spec.md:41)"
- file: .engineering/planning/story/agent-follow-links.md
  line: 56
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its answer golden texts read NOT_YET_PARSED, which html-tree-foreign changes in the same wave, and html-tree-tables and html-tree-template are unordered against it too; inferred: that the golden texts hold the note line"
- file: .engineering/planning/story/agent-session-spec.md
  line: 75
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "writes REFUSALS_MAX in the root Taskfile.yml itself, which D16 gives to the coordinator in a wave of more than one unit"
- file: .engineering/planning/story/agent-page-path-spec.md
  line: 62
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "writes REFUSALS_MAX in the root Taskfile.yml itself, which D16 gives to the coordinator in a wave of more than one unit"
- file: .engineering/planning/story/html-document-spec.md
  line: 87
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "writes REFUSALS_MAX in the root Taskfile.yml itself, which D16 gives to the coordinator in a wave of more than one unit"
- file: .engineering/planning/story/agent-request-body-spec.md
  line: 59
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "writes the root Taskfile.yml itself, which D16 gives to the coordinator, in wave 7 beside view-first-page, which also writes it"
- file: .engineering/planning/story/agent-links-spec.md
  line: 47
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "writes REFUSALS_MAX in the root Taskfile.yml itself, which D16 gives to the coordinator in a wave of more than one unit"
- file: .engineering/planning/story/agent-forms-spec.md
  line: 78
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "writes the root Taskfile.yml itself, which D16 gives to the coordinator in a wave of more than one unit"
- file: .engineering/planning/story/agent-history-spec.md
  line: 84
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "writes REFUSALS_MAX in the root Taskfile.yml itself, which D16 gives to the coordinator in a wave of more than one unit"
- file: .engineering/planning/story/html-conformance-gate.md
  line: 60
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "adds the task html-conformance to the root Taskfile.yml itself instead of taskfiles/html-conformance.yml, against D16"
- file: .engineering/planning/story/view-surfd-serves.md
  line: 104
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "adds the task view-stream-check and its line in check to the root Taskfile.yml itself instead of taskfiles/<name>.yml, against D16"
- file: .engineering/planning/story/view-first-page.md
  line: 83
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "adds the task view-test and its line in check to the root Taskfile.yml itself instead of taskfiles/view-test.yml, against D16, in wave 7 beside agent-request-body-spec, which also writes it"
```
