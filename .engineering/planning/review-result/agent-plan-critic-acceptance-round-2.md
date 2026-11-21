---
format: aep.planning-md/3
id: review-result:agent-plan-critic-acceptance-round-2
kind: review-result
status: active
title: Plan critic (acceptance), round 2, the four agent interface epics
relations:
- reviews: epic:agent-cli-target
- reviews: epic:html-parser
- reviews: epic:e2e-scenario-runner
- reviews: epic:live-view
revision: 1
---
needs-revision

The round-1 acceptance findings were answered in substance, but the daemon stories and the view stories contradict each other in three places: the order of entries in the kept log, whether a typed value reaches the log, and how many cases close `agent-session-events`. Under each pair, at most one of the two acceptances can pass.

**What I read:** 61 artifacts: the 4 epics, the 23 stories of `epic:agent-cli-target` (the 4 `deferred` ones left out, `runner-surf-cli-package` included), the 18 `html-*`, the 8 `runner-*` and the 8 `view-*` stories. I read 32 of them in full: the 4 new or revised daemon and client stories, all 8 `view-*`, `agent-session-spec`, `agent-page-path-spec`, `agent-cli-spec`, `agent-url-parser`, `agent-answer-format`, `agent-follow-links`, `agent-change-and-log`, `agent-fill-pick`, `agent-press-submit`, `html-conformance-gate`, `html-conformance-harness`, `html-document-spec` and the 4 epics. For the other 29 I read the Acceptance section and the Change parts it points to.
Commands: `aep plan artifact list|show|graph`, the 3 round-1 acceptance reports, `docs/plan/agent-plan-revision-1.md`, and on `main` `AGENTS.md` "The gate's numbers", `Taskfile.yml`, `crates/surf/tests/` and `spec/domains/document.yaml`. I also computed each story's ancestors from its `depends_on` edges.

**Round 1, by group**
- agent 1, the epic's acceptance: answered. It now asks for a passed scenario for every declared outcome and a total above the 51 of `1c979b4` (`epic/agent-cli-target.md:112-116`).
- agent 2, `agent-session-spec`: answered (at least 120 scenarios, at most 26 refusals, `:90-94`).
- agent 3 to 7, the floors and refusal limits of the specification stories: answered for `page-path`, `links`, `history`, `forms`, `request-body` and `redirect`. Each now has a floor, a refusal limit and named scenarios. The same defect is new in `html-document-spec` (finding 10).
- agent 8, `agent-url-parser`: answered (`<total>` equals the count in `SOURCE`; skipped at most 15 percent, only for hosts outside ASCII).
- agent 9, `agent-answer-format`: answered (10 values × 5 fields, 50 read-backs).
- agent 10, the directory refusals of `agent-session-process`: partly answered. They moved to `agent-surfd-supervisor`, where cases 15 to 18 cover the `surf` directory, but the two refusals of a session directory are in no case (finding 8).
- agent 11, `agent-cli-client`: answered (24 cases, among them a non-UUID session, a close without answer, a stall and an over-long path).
- agent 12 and 13, the forging cases of `follow-links` and `change-and-log`: answered (cases 11-14 and 10-12).
- html 1, the epic: answered (pass = run, and run plus not-run = exist).
- html 2 to 6, the gate story: answered. The section map and the open requirements live in the note, the `tag` lines are printed, and `--base` with its test refuses a lowered expected count.
- html 7 to 19: answered. The harness test per tag exists, and every parser story names its `tag … n/n/n` lines under `HTML_BASE=<base>`, which applies the rule that no count may drop. `character-references` reads "complete or incomplete with a reason". `tree-foreign` and `tree-complete` are split by tag. `parser-limits` has its numbers and its 10 s and 30 s bounds, and `accessible-name` names its 13 tests.
- runner 1 to 6: answered (`WouldBlock`, the grammar documents and coverage test, golden output, `PATH` emptied, `"Sign in"` and `"Log in"`, `s3cret &pass=1` in both spellings).
- runner 7 and 8: the two stories left this set under D3.

**New findings**

1. `story:agent-session-events` — case 1 and the Change put a command's events before its action record, but the D17 case requires "the action row, the events and one answer entry, in that order". `view-surfd-serves` case 13 and `view-first-page` case 6 ("events after that action") read the action first, so the two kept-log orders cannot both pass — `.engineering/planning/story/agent-session-events.md:66` against `:112-113`
   Settle: one order, stated once in this story, and the view stories' cases follow it.
2. `story:agent-session-events` — "passes with the 12 numbered cases" leaves out the "one more named case" D17 adds, so the story closes without the answer entry that the `now` and `changed` panels read — `.engineering/planning/story/agent-session-events.md:95`
   Settle: number the D17 case 13 and make the acceptance read 13.
3. `story:agent-session-events` — case 10 needs "a page title" in a record. No event on `main` carries a title (`spec/domains/document.yaml`), and `html-document-port`, the first story that fills one, comes after this story through `agent-fetch-redirects`, so the case cannot be written when the story runs — `.engineering/planning/story/agent-session-events.md:79`
   Settle: put the line feed, `"` and bidirectional control into an address field (`RequestStarted` `url`).
4. `story:agent-session-events` — case 7 ("the oldest records are dropped until the kept ones fit") names no kept count or first kept number, so a log that drops every record passes — `.engineering/planning/story/agent-session-events.md:75`
   Settle: give the record sizes and the expected first kept number and dropped count, as case 6 does.
5. `story:view-controls-and-changes` — case 4 expects the panel to show `f1 value "" -> "ada"`. `agent-session-events` keeps "a typed value or a password … only as its length" in the very answer entry the panel reads, so the two acceptances cannot both pass. `view-decisions` item 10 hides only the password — `.engineering/planning/story/view-controls-and-changes.md:58` against `agent-session-events.md:37-39,111` and `view-decisions.md:116`
   Settle: decide whether a non-password typed value reaches the kept log, and make case 4, the D17 note and item 10 say the same.
6. `story:view-surfd-serves` — case 13 ("two `action` entries in order, the events of each after it") and case 14 ("that go's events … then its `action` and `answer` entries") need opposite orders from the same log — `.engineering/planning/story/view-surfd-serves.md:89-92`
   Settle: both cases follow the one order fixed under finding 1.
7. `story:agent-surfd-supervisor` — cases 1, 6, 8, 9, 10 and 11 check "the list", which is `surfd::Sessions`, "readable in process". The tests run a built `surfd`, and the wire has no list request, so the opened time and address in case 1 cannot be observed and the implementor and the reviewer will pick different ways to read them — `.engineering/planning/story/agent-surfd-supervisor.md:77,104-105`
   Settle: name how a test reads the list (an in-process `surfd`, or a stated request), or assert only what an answer prints.
8. `story:agent-surfd-supervisor` — the refusals "a session directory is a symbolic link" and "a session directory exists before `surfd` creates it" are in Change but in none of the 21 counted cases. This is the rest of the round-1 directory finding — `.engineering/planning/story/agent-surfd-supervisor.md:69-70`
   Settle: add a case for each.
9. `story:view-http-server` — no case covers the write limit of item 8 ("10 s to write"), so a client that requests a JSON route and never reads holds a connection until the limit of 16 is reached, and the story still passes — `.engineering/planning/story/view-http-server.md:87`
   Settle: add a case: with the write limit set to 1 s, a client that does not read a large answer is closed within 2 s and a new connection is answered.
10. `story:html-document-spec` — the acceptance has no limit on the `refused:` count and no floor on the scenario total, although Change raises `REFUSALS_MAX`. This is round 1's specification-story defect, in the one specification story it did not read — `.engineering/planning/story/html-document-spec.md:105-108`
    Settle: "the `refused:` count at most N above the count recorded before it, `REFUSALS_MAX` equal to it", and a total floor.
11. `story:agent-cli-spec` — "the scenario total still 50 passed on both runners" is false on `main` from the start, which has 51 since `1c979b4` (the epic's own baseline), so the acceptance can only pass if a scenario is removed — `.engineering/planning/story/agent-cli-spec.md:74`
    Settle: 51, or "unchanged from its base".
12. `story:runner-surf-cli-package` — it lists the test binaries `cli`, `output_streams`, `review1_cli` and `review1_output`. On `main`, `crates/surf/tests/review2_output.rs` also runs the binary, and deleting it meets the acceptance — `.engineering/planning/story/runner-surf-cli-package.md:64-65`
    Settle: add `review2_output`.
13. `story:runner-scenario-from` — "plus the named refusal and file-mode tests" names no test, and Change gives none, so the count is the implementor's — `.engineering/planning/story/runner-scenario-from.md:66-67`
    Settle: list the test names.
14. `epic:e2e-scenario-runner` — the acceptance tells a story and names no command or test that prints the result. The other three epics name `slot task check` and their tests — `.engineering/planning/epic/e2e-scenario-runner.md:72-76`
    Settle: "`slot task check` exits 0 with `record_and_run` passed in `.out/test.log`", as `runner-e2e` already states.
15. `story:view-decisions` — "one example for each of the 6 routes", but no list of the 6 routes exists here or in the epic (the epic names 3 data routes and "the page's files") — `.engineering/planning/story/view-decisions.md:166`
    Settle: name the 6 paths.

**Not established, and whose lane**
- The "27 artifacts of `main`" in `agent-cli-spec`: I ran no generation. This is for whoever implements it.
- The token comparisons in constant time (session and view) and the `peer_cred` check, which depends on whether it is stable, have no observable check. Security lane.
- Out of my lane (design): the supervisor keeps title and address "already quoted and escaped", and the view writes them again as JSON, so the view may show escaped text. `view-decisions` item 7 (1,000 entries, 1 MiB) differs from the bound `agent-session-events` sets (10,000 records, 16 MiB), though item 7 applies only if that story leaves the bound open.

```findings
- file: .engineering/planning/story/agent-session-events.md
  line: 66
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "case 1 and the Change put a command's events before its action record, but the D17 case requires the action row, the events and the answer entry in that order, and view-surfd-serves case 13 and view-first-page case 6 read the action first, so the two kept-log orders cannot both pass"
- file: .engineering/planning/story/agent-session-events.md
  line: 95
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "'passes with the 12 numbered cases' leaves out the one more named case D17 adds, so the story closes without the answer entry the now and changed panels read"
- file: .engineering/planning/story/agent-session-events.md
  line: 79
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "case 10 needs a page title in a record, but no event on main carries a title and html-document-port, the first story that fills one, comes after this story through agent-fetch-redirects, so the case cannot be written when the story runs"
- file: .engineering/planning/story/agent-session-events.md
  line: 75
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "case 7 names no kept count or first kept number, so a log that drops every record passes"
- file: .engineering/planning/story/view-controls-and-changes.md
  line: 58
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "case 4 expects the panel to show f1 value \"\" -> \"ada\" while agent-session-events keeps a typed value only as its length in the answer entry the panel reads, so the two acceptances cannot both pass"
- file: .engineering/planning/story/view-surfd-serves.md
  line: 89
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "case 13 (action entries, then the events of each after it) and case 14 (the go's events, then its action and answer entries) need opposite orders from the same log"
- file: .engineering/planning/story/agent-surfd-supervisor.md
  line: 104
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "cases 1, 6, 8, 9, 10 and 11 check the session list, which is readable only in process, while the tests run a built surfd and the wire has no list request, so the opened time and address of case 1 cannot be observed"
- file: .engineering/planning/story/agent-surfd-supervisor.md
  line: 69
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the refusals of a session directory that is a symbolic link or exists before surfd creates it are in the Change and in none of the 21 counted cases"
- file: .engineering/planning/story/view-http-server.md
  line: 87
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "no case covers the write limit of item 8, so a client that requests a JSON route and never reads holds a connection until the limit of 16 is reached, and the story still passes"
- file: .engineering/planning/story/html-document-spec.md
  line: 105
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance has no limit on the refused: count and no floor on the scenario total, although the Change raises REFUSALS_MAX"
- file: .engineering/planning/story/agent-cli-spec.md
  line: 74
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "'the scenario total still 50 passed on both runners' is false on main from the start, which has 51 since 1c979b4, so the acceptance can only pass if a scenario is removed"
- file: .engineering/planning/story/runner-surf-cli-package.md
  line: 64
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the listed test binaries leave out review2_output, which on main also runs the surf binary, so deleting it meets the acceptance"
- file: .engineering/planning/story/runner-scenario-from.md
  line: 66
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'plus the named refusal and file-mode tests' names no test and the Change gives none, so the count is the implementor's"
- file: .engineering/planning/epic/e2e-scenario-runner.md
  line: 72
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance tells a story and names no command or test that prints the result, where the other epics name slot task check and their tests"
- file: .engineering/planning/story/view-decisions.md
  line: 166
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'one example for each of the 6 routes' counts routes that no list names, here or in the epic"
```
