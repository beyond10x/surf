---
format: aep.planning-md/3
id: review-result:agent-plan-critic-acceptance-agent-round-1
kind: review-result
status: active
title: Plan critic (acceptance), round 1, epic agent-cli-target
relations:
- reviews: epic:agent-cli-target
revision: 1
---
needs-revision

agent-session-spec's acceptance already holds on main today. Five specification stories put no bound on the gate's refusal count, and several security promises appear only in Change prose, outside the acceptance.

**Findings**

- epic:agent-cli-target — The acceptance joins three separate outcomes in one sentence: the walk, the wrong token refused, and "every agent command is a scenario of the specification". The third gives no count and names no scenario per command, so the epic can be half met and neither done nor not done. — .engineering/planning/epic/agent-cli-target.md:79
- story:agent-session-spec — The acceptance is already true on main: 10 `refused:` lines (at most 26), `REFUSALS_MAX` 10 equal to that count, 50 scenarios passing on both runners, and the planted defect failing 21 (AGENTS.md:264-273). So it names nothing that shows the agent domain landed, such as a scenario total above 50 that includes `surf.agent` scenarios, or `task cli` writing `.out/cli/binding.json`. — .engineering/planning/story/agent-session-spec.md:109
- story:agent-page-path-spec — The acceptance sets no limit on the `refused:` count and no floor on the scenario total. `REFUSALS_MAX` may rise by any amount as long as AGENTS.md lists each refusal. — .engineering/planning/story/agent-page-path-spec.md:73
- story:agent-links-spec — Same defect: no limit on the `refused:` count and no floor on the scenario total. "Every outcome of `Follow`" names no count or scenario file, so which scenarios count is the implementor's call. — .engineering/planning/story/agent-links-spec.md:80
- story:agent-history-spec — Same defect: no limit on the `refused:` count and no floor on the scenario total. "Every outcome of the traversal" names no count or scenario file. — .engineering/planning/story/agent-history-spec.md:91
- story:agent-forms-spec — Same defect: no limit on the `refused:` count and no floor on the scenario total. "Every outcome of `Fill`, `Pick`, `Press` and of the document commands they invoke" names no count or scenario file. — .engineering/planning/story/agent-forms-spec.md:90
- story:agent-request-body-spec — The acceptance sets no limit on the `refused:` count. `REFUSALS_MAX` may rise by any amount as long as each refusal is listed. — .engineering/planning/story/agent-request-body-spec.md:57
- story:agent-url-parser — The printed `urltestdata:` counts have no floor. A truncated data file, or a skip rule that marks most cases "needs domain to ASCII", passes. To settle it, the total should equal the vendored file's case count, and the skipped count should be at most a stated number of cases with non-ASCII hosts. — .engineering/planning/story/agent-url-parser.md:58
- story:agent-answer-format — "One read-back test per forging case" does not say which fields each case is rendered into (address, media type, title, path argument, reason). A suite that puts every forging value into one field passes while the others print raw. — .engineering/planning/story/agent-answer-format.md:66
- story:agent-session-process — The directory refusals are only in Change prose: a symbolic link, a directory owned by another user, a mode other than 0700 after creation, and an existing session directory (lines 59-60). None is among the numbered cases the acceptance counts, so it passes without them. — .engineering/planning/story/agent-session-process.md:102
- story:agent-cli-client — "One test per exit status" is met by four tests. The non-UUID session refused with exit 2 (the guard against a path outside the directory, line 49) can be missing, and so can the close, stall and over-long-path cases that each give 3. — .engineering/planning/story/agent-cli-client.md:74
- story:agent-follow-links — Only the label has a forging case (line feed and `"`). The page title and an unresolved link target are also page-derived, and they can print unescaped while the acceptance passes. — .engineering/planning/story/agent-follow-links.md:72
- story:agent-change-and-log — No case puts a line feed or `"` into a `+ text`, `~ … <before> -> <after>` or `changed page` value, although each carries page or server text. — .engineering/planning/story/agent-change-and-log.md:60

**What I read:** 19 of 19 artifacts given (`epic:agent-cli-target` and 18 `story:agent-*`), each in full with `aep plan artifact show`. Also `aep plan artifact kinds`, `aep plan artifact lifecycle story`, `aep plan artifact list --kind story --status draft`, `Taskfile.yml`, AGENTS.md "The gate's numbers", and grep for the acceptance lines in `.engineering/planning/`.

**What I could not establish**

- Whether `.out/report.json` names each scenario, so a reviewer can find "every outcome of X" without opening the suite. I ran nothing.
- The constant-time token comparison (agent-session-process) has no observable check. It is a review item for the security lane.
- agent-change-and-log calls its walk "the epic acceptance", but its acceptance leaves out `follow` by handle, `fill`, `path`, and the wrong-token refusal. Out of my lane (scope).
- Without scripts, nothing in this epic can produce the same-document change kinds `+ <element>`, `- <handle>`, `~ label`, `~ target` and `~ visibility`. So their tests can only run on states the test builds by hand. Out of my lane (design).

```findings
- file: .engineering/planning/epic/agent-cli-target.md
  line: 79
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The acceptance joins three separate outcomes in one sentence (the walk, the wrong token refused, every agent command a scenario), and the third gives no count and names no scenario per command."
- file: .engineering/planning/story/agent-session-spec.md
  line: 109
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "The acceptance is already true on main (10 refused lines at most 26, REFUSALS_MAX 10 equal to that count, 50 scenarios passing on both runners, planted defect failing 21), so it names nothing that shows the agent domain landed, such as a scenario total above 50 with surf.agent scenarios or task cli writing .out/cli/binding.json."
- file: .engineering/planning/story/agent-page-path-spec.md
  line: 73
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The acceptance sets no limit on the refused: count and no floor on the scenario total, so REFUSALS_MAX may rise by any amount as long as AGENTS.md lists each refusal."
- file: .engineering/planning/story/agent-links-spec.md
  line: 80
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The acceptance sets no limit on the refused: count and no floor on the scenario total, and 'every outcome of Follow' names no count or scenario file."
- file: .engineering/planning/story/agent-history-spec.md
  line: 91
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The acceptance sets no limit on the refused: count and no floor on the scenario total, and 'every outcome of the traversal' names no count or scenario file."
- file: .engineering/planning/story/agent-forms-spec.md
  line: 90
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The acceptance sets no limit on the refused: count and no floor on the scenario total, and 'every outcome of Fill, Pick, Press and of the document commands they invoke' names no count or scenario file."
- file: .engineering/planning/story/agent-request-body-spec.md
  line: 57
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The acceptance sets no limit on the refused: count, so REFUSALS_MAX may rise by any amount as long as each refusal is listed."
- file: .engineering/planning/story/agent-url-parser.md
  line: 58
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "The printed urltestdata counts have no floor, so a truncated data file or a skip rule that marks most cases as needing domain to ASCII passes; the total should equal the vendored file's case count and the skipped count should be at most a stated number."
- file: .engineering/planning/story/agent-answer-format.md
  line: 66
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'One read-back test per forging case' does not say which fields each case is rendered into, so a suite that puts every forging value into one field passes while the others print raw."
- file: .engineering/planning/story/agent-session-process.md
  line: 102
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "The directory refusals (a symbolic link, another owner, a mode other than 0700 after creation, an existing session directory) are only in Change prose (lines 59-60) and none is among the numbered cases the acceptance counts, so it passes without them."
- file: .engineering/planning/story/agent-cli-client.md
  line: 74
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "'One test per exit status' is met by four tests, so the non-UUID session refused with exit 2 (the guard against a path outside the directory, line 49) and the close, stall and over-long-path cases that each give 3 can be missing."
- file: .engineering/planning/story/agent-follow-links.md
  line: 72
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Only the label has a forging case (line feed and quote), so the page title and an unresolved link target, both page-derived, can print unescaped while the acceptance passes."
- file: .engineering/planning/story/agent-change-and-log.md
  line: 60
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "No case puts a line feed or quote into a + text, ~ before -> after or changed page value, although each carries page or server text."
```
