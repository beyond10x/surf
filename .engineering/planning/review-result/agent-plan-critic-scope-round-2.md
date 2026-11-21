---
format: aep.planning-md/3
id: review-result:agent-plan-critic-scope-round-2
kind: review-result
status: active
title: Plan critic (scope), round 2, the four agent interface epics
relations:
- reviews: epic:agent-cli-target
- reviews: epic:html-parser
- reviews: epic:e2e-scenario-runner
- reviews: epic:live-view
revision: 2
---
needs-revision

Every round-1 scope finding is answered. Seven new findings remain at the daemon seam: two that `agent-session-events` cannot deliver as written, and five more needs of the live view and the runner that no story claims.

**What I read:** 4 epics and 56 stories in the set (22 agent including `runner-surf-cli-package`, 18 html, 8 runner, 8 view). I read 14 stories whole: the four daemon stories, `agent-answer-format`, the 8 `view-*` and `runner-record-option`. For the others I read the diffs since `79be686`, their relations, and grep on their bodies. I also read the design (whole), `agent-plan-revision-1.md`, the `AGENTS.md` diff and the three round-1 scope reports. Commands: `aep plan artifact show|list|graph`, `git diff 79be686 497dedd`, `cat -n`/`grep` on `.engineering/planning/` (worktree `$HOME/.local/state/worktree/trees/b10x/surf/surf-plan-agent`).

**Promises:**
- 116 extracted, 116 traced to a story or a named deferred carrier.
- By epic: agent 28 from the epic plus 22 from design section 3; parser 21; runner 23; live view 22.
- Not re-traced from scratch: the parser and runner promises that did not change since round 1. For those I checked that the stories behind them changed only as the decisions D1–D18 say.
- Six of the traced promises cannot be delivered through the seam as written (findings 1–6).

**Round-1 scope findings:**
- scope-agent 1 (`outcome` field): answered. `agent-page-path-spec.md:28` drops it and says why.
- scope-agent 2 (`fill <handle> -`): answered. The epic Constraints carry it, and `agent-fill-pick.md:29` records it as D13, which the operator can overrule.
- scope-agent 3 (`--detail`): answered. The epic's Out of scope names `epic:detailed-mode`, and `agent-change-and-log.md:28` is corrected.
- scope-agent "not established" (token hash, paging, `tab_id`, `CloseWindow`): answered. Four drafts are tagged `deferred`.
- scope-html 1 (who can reach the tree): answered. The epic Outcome is rewritten (D1), matching `html-decisions.md:51-57`.
- scope-html 2 (tree mutation): answered. D2 says the tree does not change, and `html-document-port.md:36` states it.
- scope-runner 1 and 2 (`runner-detail-option`): answered. Both detail stories now decompose `epic:detailed-mode`.
- scope-runner 3 (response address): answered. `runner-formats.md:37,50` holds it, and `runner-replay-transport.md:57` tests it.

**New findings:**

1. `story:agent-session-events` — the third kind of entry (D17) is "the typed answer (`surf-answer`'s value, before it is rendered to text)", and the story says "nothing changes in how a session publishes". But the link carries only "the exit status, the answer lines and a status record … printable ASCII only", and `surf-answer` only renders. So no story carries the typed answer to `surfd` or reads it back there. — `.engineering/planning/story/agent-session-events.md:108-110`; `agent-surfd-supervisor.md:87-90`; `agent-answer-format.md:55-56`
   - Without this, `view-surfd-serves`, `view-now-panel` and `view-controls-and-changes` would each write their own parser of answer text inside `surfd`.
   - Settle: this story claims an encoding of the typed answer on the link (and its count in Acceptance), or says that `surfd` keeps the lines and names who parses them.

2. `story:agent-session-events` — the rule "a typed value or a password appears only as its length" also removes the values of controls that are not passwords. `view-controls-and-changes` case 4 needs exactly that value (`f1 value "" -> "ada"`), and so does the epic's "state or target". — `.engineering/planning/story/agent-session-events.md:111`; `view-controls-and-changes.md:58`
   - Settle: the rule for the answer entry is narrowed to password values and request bodies, as the answer already prints them.

3. `story:agent-session-events` — kept records carry "a per-session sequence number from 1", and `Daemon::subscribe()` gives records "from then on". The view server's source needs "a subscription from an entry identity … and says when the identity asked for is older than what is kept" on `/events`, which spans all sessions. Neither story gives a position across all sessions or a way to subscribe from one. — `.engineering/planning/story/agent-session-events.md:56-60`; `view-http-server.md:45-47`
   - Settle: this story gives a log-wide entry identity and subscribe-after-identity with the "older than kept" signal, or `view-surfd-serves` claims building them.

4. `story:view-surfd-serves` — several values reach `surfd` already quoted with the `\x` escapes of `surf-answer`:
   - the session list's address and title, "already quoted and escaped by the session (kept as text, never read)";
   - event fields and the action argument, "written by the quoting function of `surf-answer`".

   No story writes the reverse, which the view's JSON of exact values needs. — `.engineering/planning/story/view-surfd-serves.md:59-61`; `agent-surfd-supervisor.md:79-80`; `agent-session-events.md:48-50`
   - Settle: this story claims an unquote function (in `surf-answer`) and the supervisor's "never read" is relaxed for it, or the link carries the raw values.

5. `story:view-surfd-serves` — it needs "The random source and the constant-time comparison the session token uses (`story:agent-session-process`), called here for the view token, not written twice". That cannot work: `surf-session` depends on `surfd`, so `surfd` cannot call into it. And `view-http-server` already writes its own comparison. — `.engineering/planning/story/view-surfd-serves.md:39-40`; `agent-session-process.md:42-44`; `view-http-server.md:50-51`
   - Settle: re-point the need to `view-http-server`'s comparison and to the random source `surfd` already reads for identities (`agent-surfd-supervisor.md:35-36`), or move both into a crate `surfd` can depend on.

6. `story:agent-session-events` — its rule "events of commands added by later stories (each maps the events it uses)" names no owner. Only `agent-fill-pick` and `agent-press-submit` take it on. These stories add events and claim no mapping:
   - `agent-redirect-spec` / `agent-fetch-redirects`: `url` on `ResponseReceived` and `NavigationCommitted`;
   - `agent-links-spec` / `agent-follow-links`: `Follow`;
   - `agent-history-spec` / `agent-back-forward`: `GoBack`, `GoForward`.

   Those events would reach the view `unmapped`, with no field, while `view-detail-panel` case 2 reads `url` of `ResponseReceived`. — `.engineering/planning/story/agent-session-events.md:101-102`; `agent-redirect-spec.md:41`; `view-detail-panel.md:41-42`
   - Settle: each of those stories claims mapping its events, or this story maps fields generically from the generated types.

7. `story:runner-record-option` — the story still has `surf-agent` pass `--record` "to the session process `open` starts", which the daemon move made false. No story claims any of the following:
   - carrying `--record` over the wire and the link;
   - resolving a relative path against the client's directory (`surfd`'s working directory is not the client's);
   - the promised refusal ("refuses `open` with exit 2 and starts no session"), which can no longer hold once `surfd` has started the process.

   — `.engineering/planning/story/runner-record-option.md:49-54`
   - Settle: revise the story for `surfd`. The client sends an absolute path; the refusal is the session process's `open` answer, and `surfd` lists no session for it.

**What I could not establish, or left to other lanes:**
- Acceptance lane:
  - The extra case in `agent-session-events`'s D17 addendum is not in its Acceptance count ("12 numbered cases", line 95).
  - The session directory's mode 0700 and its "exists before" refusal moved to `agent-surfd-supervisor` Change (lines 67-73) with no numbered case. Before the move they were cases 3 and 7 of `agent-session-process`.
- Design/parallel lane (D16):
  - `view-surfd-serves.md:104` and `view-first-page.md:114` write their tasks into `Taskfile.yml`, against D16 (`taskfiles/<name>.yml`).
  - `view-detail-panel` needs `url` on `ResponseReceived` from `agent-redirect-spec`, and no edge orders the two.
- Accepted as named, not a finding: the epic's "`ess-http/1` binding where ESS accepts one" has no writing story if measurement (b) finds a binding usable. `view-decisions.md:172-173` says it will name a later story then.

```findings
- file: .engineering/planning/story/agent-session-events.md
  line: 108
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the D17 answer entry is the typed surf-answer value 'before it is rendered to text' and 'nothing changes in how a session publishes', but the link carries only printable answer lines (agent-surfd-supervisor.md:87-90) and surf-answer only renders, so no story claims carrying the typed answer to surfd or reading it there"
- file: .engineering/planning/story/agent-session-events.md
  line: 111
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the rule 'a typed value or a password appears only as its length' removes control values from the answer entry that view-controls-and-changes case 4 shows (f1 value \"\" -> \"ada\", view-controls-and-changes.md:58) and the live view epic promises as each element's state"
- file: .engineering/planning/story/agent-session-events.md
  line: 56
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "records carry only a per-session sequence number and subscribe() starts 'from then on', while the view server's source needs a subscription from an entry identity across all sessions with an 'older than kept' signal for /events and Last-Event-ID (view-http-server.md:45-47), and no story claims it"
- file: .engineering/planning/story/view-surfd-serves.md
  line: 59
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "address, title, event fields and action argument reach surfd already quoted with surf-answer's \\x escapes (agent-surfd-supervisor.md:79-80 'kept as text, never read'; agent-session-events.md:48-50), and no story writes the reverse the view's JSON of exact values needs"
- file: .engineering/planning/story/view-surfd-serves.md
  line: 39
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the need to call agent-session-process's random source and constant-time comparison for the view token cannot be delivered because surf-session depends on surfd (agent-session-process.md:42-44), and view-http-server already writes its own comparison (view-http-server.md:50-51)"
- file: .engineering/planning/story/agent-session-events.md
  line: 101
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'each maps the events it uses' is carried only by agent-fill-pick and agent-press-submit; the redirect, follow and history stories map none of the events or fields they add (url on ResponseReceived, Follow, GoBack, GoForward), so those reach the view unmapped while view-detail-panel case 2 reads url of ResponseReceived"
- file: .engineering/planning/story/runner-record-option.md
  line: 49
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story passes --record 'to the session process open starts', which under surfd is false; carrying the path through wire and link, resolving a relative path against the client's directory, and refusing open with exit 2 and no session after surfd started the process are claimed by no story"
```
