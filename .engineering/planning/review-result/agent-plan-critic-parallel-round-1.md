---
format: aep.planning-md/3
id: review-result:agent-plan-critic-parallel-round-1
kind: review-result
status: active
title: Plan critic (parallel safety), round 1, the three agent interface epics
relations:
- reviews: epic:agent-cli-target
- reviews: epic:html-parser
- reviews: epic:e2e-scenario-runner
revision: 1
---
needs-revision

Seven pairs can run at the same time and collide, and none of them is admitted with a `depends_on` edge. The specification chain itself is clean: every writer of `spec/**`, `generated/rust/**`, `conformance/src/*`, `crates/ports`, `REFUSALS_MAX`, `crates/answer`, `crates/page` and every `crates/html` count file sits in one ordered chain.

**What I read:** 47 stories and 86 story-to-story `depends_on` edges. Commands: `aep plan artifact list --kind story --status draft`, `aep plan artifact graph`, `aep plan artifact waves --kind story --status draft` (all 47 unassessed, no scope recorded), every body under `.engineering/planning/story/`, plus `AGENTS.md`, `crates/surf/src/{lib,main}.rs`, `crates/surf/Cargo.toml` and `crates/fetch/src/lib.rs`. I worked out which stories are ordered (directly or through other stories) with awk over the edges, so no Python.
- **Surfaces:** 45 cited, 2 inferred (`runner-record-option`, `runner-record-session` say "paths are that epic's"; I resolved them to `crates/agent` and `crates/session`), 0 unplaceable. Every story names a surface.
- **Acceptance reads:** 18 traced to a producer in the set; 13 have a `depends_on` path to it, 5 do not.

## Findings

1. **story:runner-surf-cli-package** — moves `crates/surf/src/main.rs`, the four binary tests, `crates/surf/Cargo.toml` and the `AGENTS.md` Layout entry of `crates/surf` while `agent-request-body-spec`, `agent-fetch-post`, `agent-fetch-redirects`, `agent-form-port` and `html-document-port` write the same files with no `depends_on` edge either way, and its body names only the POST and redirect stories — `.engineering/planning/story/runner-surf-cli-package.md:47`
   - Quoted: "Collides with any story of `epic:agent-cli-target` that changes `crates/surf` (the transport for POST, redirects)."
   - Evidence: `agent-fetch-post.md:28,35`, `agent-request-body-spec.md:44-46` (main.rs interim refusal), `agent-fetch-redirects.md:36` (a `surf <url>` test needs `CARGO_BIN_EXE_surf`), `agent-form-port.md:53`, `html-document-port.md:42-44,52-55`. Its acceptance, "the same total number of tests as before the move", reads a count those stories change.
   - The `Cargo.toml` overlap for `agent-form-port` and `html-document-port` is inferred: they add surf-url, surf-html and surf-page.
   - To settle: either add edges whose reason is `crates/surf/src/main.rs` and `crates/surf/tests`, or have those five stories put binary changes in `crates/surf-cli`. Either way, name `html-document-port` and `agent-request-body-spec` in the body.

2. **story:runner-detail-option** — writes `cli/surf-agent.yaml`, the `crates/agent` parser and `crates/session` as `runner-record-option` (`--record`) and `runner-record-session` (the session process answer path) do, and `README.md` as `runner-e2e` does, with no `depends_on` edge either way and no body naming the other — `.engineering/planning/story/runner-detail-option.md:45`
   - Quoted: "Surfaces written: `cli/surf-agent.yaml`, `crates/agent/`, `crates/session/`, …"
   - Evidence: `runner-record-option.md:31-40,45-48`, `runner-record-session.md:39-45,59-60`, `runner-e2e.md:43-48`.
   - To settle: an edge whose reason is `cli/surf-agent.yaml` and `crates/session`, or a split of the surface.

3. **story:html-conformance-gate** — writes the Layout and Commands sections of `AGENTS.md` that `agent-session-process` also writes, with no `depends_on` edge either way, so the two land in one wave (the third) unordered — `.engineering/planning/story/html-conformance-gate.md:43`
   - Quoted: "`AGENTS.md`: "Layout" names `crates/html` … "Commands" names `html-conformance`".
   - Evidence: `agent-session-process.md:78-79,97-98`. Both add entries to the same lists of crates and commands.
   - To settle: an edge whose reason is `AGENTS.md`, or one story owning both entries.

4. **story:html-conformance-gate** — writes `Taskfile.yml` (task check) and `AGENTS.md` The gate's numbers that `agent-page-path-spec` also writes (`REFUSALS_MAX` and the refusal list), with no `depends_on` edge either way — `.engineering/planning/story/html-conformance-gate.md:38`
   - Evidence: `agent-page-path-spec.md:63-64,66-69`. The two land in waves 2 and 3 only by depth; the gate becomes ready as soon as the harness lands.
   - To settle: an edge whose reason is `Taskfile.yml`, or a split of the surface.

5. **story:html-actionable-elements** — its acceptance runs `template_contents_are_not_listed` and `select_value_follows_selectedness`, which read template contents and select parsing that `html-tree-template` and `html-tree-tables` produce, and no `depends_on` edge runs to either — `.engineering/planning/story/html-actionable-elements.md:42`
   - Evidence: lines 42-44 and 51. The producers are `html-tree-template.md:26` ("template contents as a fragment owned by the `template` element") and `html-tree-tables.md:31-32` ("the `select` rules"). `html-tree-body.md:29-32` implements neither in select nor in template.
   - Cut by depth, this story lands two to three waves before its producers.
   - To settle: edges to the two tree stories, or drop those two tests here.

6. **story:agent-form-port** — its select option and selectedness tests, and `agent-fill-pick`'s pick of a select option on a served page, read the select parsing `html-tree-tables` produces, and no story of the agent epic depends on `html-tree-tables` — `.engineering/planning/story/agent-form-port.md:35`
   - Evidence: `agent-fill-pick.md:36,46-47`, `html-tree-tables.md:31-32`.
   - To settle: a `depends_on` edge to `html-tree-tables`.

7. **story:agent-request-body-spec** (inferred surface) — changes `Transport::fetch` (and `agent-fetch-post` changes the surf-fetch entry point) that the session process transport in `crates/session` calls, names no `crates/session` surface, and no `depends_on` edge orders either against `agent-follow-links`, which writes `crates/session` — `.engineering/planning/story/agent-request-body-spec.md:43`
   - Evidence: `agent-fetch-post.md:23-24`; `agent-session-process.md:44,72-73`; `crates/surf/src/lib.rs:83-89` (the closure `Transport` impl); `crates/surf/src/main.rs:36-47` (today's transport pattern); `agent-follow-links.md:67-68`.
   - To settle: name `crates/session` in both stories and add an edge, or split.

## Every unordered pair that writes one file

Unordered means no `depends_on` path runs between the two stories in either direction.

**Same lines or same section (the findings):**
- `runner-surf-cli-package` with:
  - `agent-fetch-post`: main.rs, `crates/surf/tests`
  - `agent-request-body-spec`: main.rs
  - `agent-fetch-redirects`: `crates/surf/tests`, AGENTS.md Layout
  - `agent-form-port`: `crates/surf/tests`, `crates/surf/Cargo.toml` (inferred)
  - `html-document-port`: main.rs, tests, Cargo.toml (inferred), README, AGENTS.md Layout entry of `crates/surf`
  - `html-conformance-gate`: AGENTS.md Layout
- `runner-detail-option` with:
  - `runner-record-option`: `cli/surf-agent.yaml`, `crates/agent`, `crates/session`
  - `runner-record-session`: `crates/agent`, `crates/session`
  - `runner-e2e`: README
- `html-conformance-gate` with:
  - `agent-session-process`: AGENTS.md Layout and Commands
  - `agent-page-path-spec`: Taskfile.yml, AGENTS.md The gate's numbers
- `agent-follow-links` with `agent-request-body-spec` and `agent-fetch-post`: `crates/session` (inferred)

**Different sections of AGENTS.md or README (one file, separate hunks):**
- `runner-surf-cli-package` with `html-document-spec`, `agent-links-spec`, `agent-forms-spec`, `agent-history-spec`, `agent-change-and-log` (AGENTS.md).
- `runner-surf-cli-package` with `agent-follow-links`, `agent-fill-pick`, `agent-press-submit`, `agent-back-forward`, `agent-change-and-log`, `runner-detail-option` (README).
- `agent-session-process` with `html-document-spec` (AGENTS.md Layout/Commands/Implementation targets against Components/The gate's numbers).

**Cargo.lock:** 36 unordered pairs touch it. All are covered by the rule in `AGENTS.md:225-226` ("`Cargo.lock` is the coordinator's too: regenerated on the wave branch"), so none is a finding.

## Widest sets, waves cut by depth

| When | Stories | Count | Collision |
|---|---|---|---|
| Start | `agent-answer-format`, `agent-session-spec`, `agent-url-parser`, `html-decisions`, `runner-decisions` | 5 | Only Cargo.lock (coordinator's) |
| After wave 1 | `agent-cli-client`, `agent-page-path-spec`, `html-conformance-harness` | 3 | None |
| After wave 2 | `agent-session-process`, `html-conformance-gate` | 2 | AGENTS.md (finding 3) |
| After wave 3 | `html-document-spec`, `html-input-stream`, `html-tokenizer-markup`, `runner-surf-cli-package` | 4 | AGENTS.md in different sections only; `input-stream` and `tokenizer-markup` are disjoint, as `html-input-stream.md:21` says |

## Count files and numbers in AGENTS.md

No count file is raised by two parallel stories:
- `expected/tokenizer.txt`: markup, then text, then charref, in order.
- `expected/tree-construction.txt`: body, formatting, tables, template, foreign, complete, in order.
- `expected/encoding.txt`: the gate creates it, then `html-input-stream` raises it.

`REFUSALS_MAX` and the scenario totals are written only by the specification chain, in this order: `agent-session-spec`, `agent-page-path-spec`, `html-document-spec`, `agent-links-spec`, `agent-request-body-spec`, `agent-forms-spec`, `agent-history-spec`, `runner-record-option`. The one parallel writer of "The gate's numbers" is `html-conformance-gate`, which writes no number into it (finding 4).

## What I could not establish

- Whether `html-document-spec`'s "changed only as far as the regenerated interface needs" reaches `crates/surf/src/main.rs`. If it does, it joins finding 1.
- Whether `crates/session` builds its own surf-fetch transport (finding 7 rests on that).
- Read-only overlaps with no file written by both: `runner-run` against `runner-record-session` and against `runner-detail-option`, on the session core interface.
- Out of my lane (scope): `agent-links-spec` (`Answer::Response`) and `html-document-port` (a new `Browser` parameter) change interfaces that `crates/session` uses without naming it. Both are ordered, so neither is a concurrency defect.

```findings
- file: .engineering/planning/story/runner-surf-cli-package.md
  line: 47
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "moves crates/surf/src/main.rs, the four binary tests, crates/surf/Cargo.toml and the AGENTS.md Layout entry of crates/surf while agent-request-body-spec, agent-fetch-post, agent-fetch-redirects, agent-form-port and html-document-port write the same files with no depends_on edge either way, and its body names only the POST and redirect stories (Cargo.toml overlap for agent-form-port and html-document-port inferred)"
- file: .engineering/planning/story/runner-detail-option.md
  line: 45
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "writes cli/surf-agent.yaml, the crates/agent parser and crates/session as runner-record-option (--record) and runner-record-session (the session process answer path) do, and README.md as runner-e2e does, with no depends_on edge either way and no body naming the other (record-option and record-session paths inferred from 'paths are that epic's')"
- file: .engineering/planning/story/html-conformance-gate.md
  line: 43
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "writes the Layout and Commands sections of AGENTS.md that agent-session-process also writes, with no depends_on edge either way, so the two land in one wave (the third) unordered"
- file: .engineering/planning/story/html-conformance-gate.md
  line: 38
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "writes Taskfile.yml (task check) and AGENTS.md The gate's numbers that agent-page-path-spec also writes (REFUSALS_MAX and the refusal list), with no depends_on edge either way"
- file: .engineering/planning/story/html-actionable-elements.md
  line: 42
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance runs template_contents_are_not_listed and select_value_follows_selectedness, which read template contents and select parsing that html-tree-template and html-tree-tables produce, and no depends_on edge runs to either"
- file: .engineering/planning/story/agent-form-port.md
  line: 35
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its select option and selectedness tests, and agent-fill-pick's pick of a select option on a served page, read the select parsing html-tree-tables produces, and no story of the agent epic depends on html-tree-tables"
- file: .engineering/planning/story/agent-request-body-spec.md
  line: 43
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "changes Transport::fetch (and agent-fetch-post changes the surf-fetch entry point) that the session process transport in crates/session calls, names no crates/session surface, and no depends_on edge orders either against agent-follow-links, which writes crates/session (inferred surface)"
```
