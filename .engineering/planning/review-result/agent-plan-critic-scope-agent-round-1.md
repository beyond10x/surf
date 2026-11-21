---
format: aep.planning-md/3
id: review-result:agent-plan-critic-scope-agent-round-1
kind: review-result
status: active
title: Plan critic (scope), round 1, epic agent-cli-target
relations:
- reviews: epic:agent-cli-target
revision: 1
---
needs-revision

All promises are claimed except one design field that a story drops without saying so; one story adds a command form nobody asked for; and the epic's exclusion text disagrees with where the plan put `--detail`.

**What I read:** 22 artifacts:
- the epic, `epic:agent-cli-target`
- its 18 `story:agent-*`
- `story:runner-detail-option`, plus grep of five other runner stories and `story:html-document-spec`
- `docs/design/agent-interface.md` (whole), `docs/plan/agent-cli-draft-1.md` sections State and Decisions, `AGENTS.md` Goals

Commands: `aep plan artifact show|list|graph` and `cat -n` on `.engineering/planning/{epic,story}/*.md`. I listed the epic's promises before reading any story: 31 from the epic and 32 from design sections 3 to 6, 63 in total. 60 trace to an item. Three of those 60 are claimed by stories of other epics:
- `--detail` by `runner-detail-option`
- document title and elements by `html-document-spec`
- the accessible-name label by `html-accessible-name`

The other 3 promises are dealt with below: one is finding 1, two are named omissions.

## Findings

1. **story:agent-page-path-spec — design section 6's `Action` field "outcome" is quoted in Context but the Change list leaves it out without saying so — .engineering/planning/story/agent-page-path-spec.md:46**
   - Context line 19 quotes "`Action` rows owned by a session (sequence number, command, input, outcome)" (design line 218).
   - The Change at lines 46-50 gives the action: session, sequence number, command, argument. It explains why refused commands are not recorded, but not why "outcome" (for example, whether the navigation committed or failed) goes.
   - An implementor reading the design as the reference cannot tell whether to add the field.
   - Settle: the story either adds `outcome` or says in one sentence that it is dropped and why.

2. **story:agent-fill-pick — adds `fill <handle> -` (value from standard input), which no sentence of the epic or design sections 3 to 6 asks for — .engineering/planning/story/agent-fill-pick.md:26**
   - The story's own words: "So `fill` also reads the value from standard input (`fill <handle> -`), and `README.md` says to use that for a secret."
   - The epic lists `fill` (line 42), and design section 4 gives only `fill <handle> <text>`. The epic's argument-word rule covers only the token (Constraints).
   - This new form changes `cli/surf-agent.yaml`, and the epic's acceptance walk in `agent-change-and-log:50` relies on it.
   - Settle: either the epic gains a sentence for it (a Constraint "a secret value is never an argument word"), or the story records it as a reversible decision the operator can overrule.

3. **epic:agent-cli-target — Out of scope excludes only "the detailed mode's style changes", so by its own text the epic still owes `--detail`; the plan gives it to another epic — .engineering/planning/epic/agent-cli-target.md:61**
   - Line 18 says "the design is the reference for every detail not repeated here", and design section 4 (line 111) puts `--detail` on every command.
   - The coordinator gave the option to `story:runner-detail-option` (`epic:e2e-scenario-runner`).
   - `agent-change-and-log.md:67-68` still says "neither this epic nor `epic:e2e-scenario-runner` claims the option", and that is now false.
   - Settle: the epic's Out of scope names `--detail` and the non-style detailed lines as `epic:e2e-scenario-runner`'s, and the parenthetical in `agent-change-and-log` is corrected.

## What I could not establish, or left to others

These are omissions a story names with a reason, so they are not findings. But no artifact carries them forward:
- **The token hash.** Design line 83: "keeps only a hash of it". `agent-session-process.md:28-33` defers it until the cryptographic crate of `AGENTS.md` Goals is chosen.
- **`--max-lines` and `look --from`.** Design lines 112 and 174. `agent-follow-links.md:79` excludes them, citing design section 10 question 3, and no decision-blocker tracks that question.
- **Two follow-up stories.** Draft-1 decisions 6 and 10 promise "a story of its own" for a caller-chosen `tab_id` and "a `CloseWindow` story". Neither exists; `agent-session-spec.md:119-120` names both out of scope.

The operator decides whether these need a carrier.

Outside my lane:
- The false parenthetical also sits in `runner-detailed-changes.md:58-59` (design lane).
- `agent-form-port` writes `crates/page` and `crates/surf`, which belong to `epic:html-parser` (parallel-safety lane).
- `agent-url-parser` adds behaviour with no specification change, against "Specs first" (design lane).

```findings
- file: .engineering/planning/story/agent-page-path-spec.md
  line: 46
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Change list defines the Action entity without the field 'outcome' that design section 6 promises and Context line 19 quotes, and says nothing about dropping it"
- file: .engineering/planning/story/agent-fill-pick.md
  line: 26
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story adds 'fill <handle> -' (value from standard input), a command-line form no sentence of the epic or design sections 3 to 6 asks for, and records it as no overrulable decision"
- file: .engineering/planning/epic/agent-cli-target.md
  line: 61
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Out of scope excludes only the detailed mode's style changes, so the epic by its own text still owes --detail, which the plan gives to story:runner-detail-option of epic:e2e-scenario-runner, and agent-change-and-log says no epic claims it"
```
