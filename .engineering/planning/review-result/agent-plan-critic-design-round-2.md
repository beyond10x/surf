---
format: aep.planning-md/3
id: review-result:agent-plan-critic-design-round-2
kind: review-result
status: active
title: Plan critic (design), round 2, the four agent interface epics
relations:
- reviews: epic:agent-cli-target
- reviews: epic:html-parser
- reviews: epic:e2e-scenario-runner
- reviews: epic:live-view
revision: 1
---
needs-revision

The set has no cycle, and the thirteen round-1 design findings are all answered. The new daemon seam is not ready: `agent-session-events` gives the kept log two orders, a per-session entry identity the view cannot resume a stream from, no encoding for the typed answer on the link, and no rule keeping the `open` token or response bodies out of the log.

**What I read.** I judged commit `497dedd`. The worktree's HEAD moved to `d382c5e` while I worked; that commit only appends Scope sections to the six depth-1 stories, and the `depends_on` edges are identical (diffed). I read these whole: the 4 daemon stories, the 8 `view-*` stories, `epic:live-view`, `agent-session-spec`, design sections 3 and 12, the revision decisions (D1–D18) and the round-1 design report. I grepped about 20 more stories at the seams. Commands: `aep plan artifact relations`, `graph`, `validate` (valid, 99 artifacts) and `waves`, plus `tsort` and an awk longest-path pass over 129 story-level `depends_on` edges. I walked outside the set (the first plan, `detailed-mode`): no cycle. I traced 21 acceptance reads to a producer in the set, and an edge records all 21, directly or through other edges.

**Round-1 findings (design)**
1. runner-formats needs the answer's lines: answered. There are edges to `agent-change-and-log` and `agent-answer-format`.
2. Response address and method/body: answered (D14). `runner-formats.md:37,50` records the response address; edge to `agent-request-body-spec`.
3–5. The detail stories: answered by D3. They moved to `epic:detailed-mode`.
6. Where the `surf` binary lives: answered (D6). All five stories depend on `runner-surf-cli-package` and name `crates/surf-cli`.
7. Redirects: answered (D4). `agent-redirect-spec` sits at depth 4, and `agent-fetch-redirects` no longer waits for the parser.
8. Command-line document: answered (D5). `agent-cli-spec` → `agent-cli-client`.
9. `html-conformance-gate` edge: answered by D8, which keeps the edge knowingly.
10. Epic-level edge: answered (D9). Only `epic:e2e-scenario-runner → epic:agent-cli-target` remains.
11. Who reads the node tree: answered. `epic/html-parser.md:39` now says `Browser::document_tree`.
12. Document base URL: answered (D10). `crates/page` `document_base_url`, written in `agent-follow-links.md:53`, called at `agent-form-port.md:50`.
13. Select value: answered (D11) in `agent-forms-spec.md:30-32`, `html-actionable-elements.md:42` and `agent-fill-pick.md:26`.

**New findings**

1. story:agent-session-events — the kept log has two orders, and one of them puts each command's events under the wrong action in the view — `agent-session-events.md:46-47`, `:112-113`; `view-first-page.md:38`; `view-surfd-serves.md:89-92`
   - Change and case 1 say "a record for every event … then one record for each action row".
   - The D17 addendum says "the action row, the events and one answer entry, in that order".
   - `view-first-page` gives "an entry belongs to the action it follows". With events first, each `go`'s events land under the previous action.
   - `view-surfd-serves` case 13 says action then events; its case 14 says events then action.
   - Settle: one order in `agent-session-events`. Cases 13 and 14 and the `docs/view-stream.md` rule follow it.

2. story:agent-session-events — D17 keeps "the typed answer (`surf-answer`'s value, before it is rendered to text)" and says "nothing changes in how a session publishes", but the link carries no typed answer — `agent-session-events.md:108-110`; `agent-surfd-supervisor.md:87-90`; `docs/design/agent-interface.md:76-77`
   - The link answer is "the exit status, the answer lines and a status record".
   - So `surfd` would have to parse rendered answer text, which holds page titles. Design section 3 forbids that: `surfd` "parses nothing a server sent".
   - No story writes an encoding of the typed answer. The `now`/`changed` blocks added later need one too.
   - Settle: `agent-session-events` owns a typed answer record on the link, generic over `surf-answer`'s blocks.

3. story:agent-session-events — the session token reaches the kept log — `agent-session-events.md:106-110`; `agent-answer-format.md:37`; `agent-surfd-supervisor.md:85-86`
   - D17 keeps "the typed answer … of each action", and the typed answer of `open` holds the `token <64 hexadecimal digits>` line.
   - `agent-surfd-supervisor` relays that line "without keeping it". This story's case 3 and the view's rule ("a session's token never reaches the view") contradict the addendum as written.
   - Settle: the kept answer entry drops the token block, with a named case.

4. story:agent-session-events — the entry identity does not fit `/events` — `agent-session-events.md:56-60`; `view-decisions.md:130,133`; `view-http-server.md:45-47`
   - It stamps "a per-session sequence number from 1". Its in-process readers are per session, and `subscribe` orders only "in each session order".
   - `docs/view-stream.md` gives every entry "an increasing identity" used as the `id:` of one `/events` stream across sessions. `view-http-server`'s source resumes "from an entry identity" with `Last-Event-ID`.
   - A per-session number cannot name a resume point across sessions, and no story assigns a daemon-wide one.
   - Settle: `agent-session-events` stamps a daemon-wide sequence and offers a reader from it, or the view documents say `/events` is per session.

5. story:agent-session-events — the response body travels and is kept in full, which kills sessions — `agent-session-events.md:37-41,47-50`; `agent-surfd-supervisor.md:89-90`; `crates/fetch/src/lib.rs:107-108`; `view-decisions.md:107-110`
   - Every event is mapped "with its fields" (case 12), so `surf.net.ResponseReceived`'s `body` goes quoted into the link answer and the log. Only a typed value or a request body becomes a length.
   - The link refuses an answer over 4 MiB and kills the session as `died`. `surf-fetch` accepts bodies up to 16 MiB. So any page of a few MiB kills its session.
   - `surfd`, which the design says "holds no page", would then hold page bytes.
   - `view-decisions` item 7 and `view-surfd-serves` (it turns a byte field into its length in `surfd`) assume a length only.
   - Settle: the session process writes byte fields as `<n> bytes`.

6. story:view-surfd-serves — the token helpers it plans to reuse sit in a crate `surfd` cannot depend on — `view-surfd-serves.md:39-40`; `agent-session-process.md:43,56-58`; `agent-surfd-supervisor.md:35-36,60-62,157-159`; `view-http-server.md:32,51`
   - It takes "the random source and the constant-time comparison the session token uses (`story:agent-session-process`), called here for the view token, not written twice".
   - Those live in `surf-session`, which depends on `surfd` (`surfd::link`). Calling them from `surfd` is a crate cycle.
   - `surfd`'s acceptance allows only `surf-agent` and `surf-answer`, yet it already needs a random source for the session identity. The only existing reader is `surf-ports` `OsRandom`.
   - `view-http-server` writes its own constant-time comparison in a `surf-view` with no dependency.
   - Result: two to three random readers and two comparisons.
   - Settle: one std-only home reachable by `surf-session`, `surfd` and `surf-view` (for example a module of `surf-agent`, written by `agent-cli-client`), or an explicit decision to keep the copies.

7. story:agent-surfd-supervisor — address and title are stored quoted, but the view needs raw values — `agent-surfd-supervisor.md:77-80`; `view-surfd-serves.md:59-61`; `view-decisions.md:130-132`; `view-first-page.md:108-110`
   - The list keeps them "already quoted and escaped by the session (kept as text, never read)".
   - `view-surfd-serves` serves them as JSON strings under a string rule that escapes the raw value, and `view-first-page` case 12 shows the address as text.
   - So either `surfd` unquotes session text or the page shows the quoted form.
   - Settle: the link's status record carries raw values for `surfd` to escape, or the view documents the quoted form.

8. story:agent-session-events — nothing bounds the kept logs across sessions — `agent-session-events.md:32-34`; `agent-surfd-supervisor.md:55`
   - The bound is per session ("10,000 records or 16 MiB"), and an ended session's log "stays as long as the session is listed".
   - The supervisor keeps up to 256 ended sessions, so with the open ones `surfd` can hold over 4 GiB.
   - Settle: a daemon-wide byte bound, or ended sessions' logs dropped first.

9. story:view-surfd-serves — it adds `view-stream-check` to the root `Taskfile.yml`. D16 says a new task goes in `taskfiles/<name>.yml`, included by the root file — `view-surfd-serves.md:66-67,104`

10. story:view-first-page — it adds `view-test` to the root `Taskfile.yml`, against D16 — `view-first-page.md:83-84,114`

11. story:agent-cli-spec — its task `cli` goes into the root `Taskfile.yml`, against D16 — `agent-cli-spec.md:51` at `497dedd`

12. story:html-conformance-gate — its task `html-conformance` goes into the root `Taskfile.yml`, against D16 — `html-conformance-gate.md:60`

**The daemon seam, point by point**
- **Crate layout.** `surf-agent` uses only the standard library and holds `wire`. `surfd` depends on `surf-agent` and `surf-answer` and holds `link`. `surf-session` depends on `surfd` (and on `surf-agent`, `surf`, `surf-ports` and the rest). `surf-view` has no dependency, and `view-surfd-serves` adds it to `surfd`. There is no cycle as drafted; finding 6 is where one would appear. Side effect: `surfd`'s own `cargo tree` acceptance stops holding once `surf-view` is added.
- **Protocol.**
  - client ↔ `surfd`: `wire`, defined by `agent-cli-client`.
  - `surfd` ↔ session: `link`, defined by `agent-surfd-supervisor`.
  - Event records on the link: added by `agent-session-events`.
  - Typed answer on the link: owned by no story (finding 2).
  - HTTP side: `docs/view-stream.md`, written by `view-decisions`.
- **Session identity.** Consistent: `surfd` draws it (`agent-surfd-supervisor.md:35`, `agent-session-spec` marker 7, `agent-session-process.md:45-46`).
- **Log bounds.** `agent-session-events` says 10,000 records or 16 MiB, plus a 1,024-record subscriber queue. `view-decisions` item 7 (1,000 entries or 1 MiB) applies only "if `agent-session-events` leaves it open", which it does not, so they do not contradict. Finding 8 is the gap.
- **Entry kinds and the typed answer (D17).** `view-decisions` names `action`, `event` and `answer`. `agent-session-events` names them only in the addendum, while its Change section and acceptance still list two kinds and 12 cases. Findings 1–3.
- **View server start and `crates/surfd/src/main.rs`.** `agent-surfd-supervisor` writes all of `crates/surfd/`. `view-surfd-serves` adds one call to `main.rs` "or the start function the supervisor names", and the supervisor names none. Edges order them, so this is no collision.
- **Random source and constant-time comparison.** Finding 6.
- **View token against session token.** Same shape (32 bytes, 64 hex digits), different values and channels (bearer header against `SURF_TOKEN`). `view-url` is mode 0600 in the 0700 directory. Consistent, apart from finding 3.

**Shortest chain to `view-first-page`**
The chain is 7 deep: `agent-cli-spec` → `agent-session-spec` → `agent-page-path-spec` → `agent-session-process` → `agent-session-events` → `view-surfd-serves` → `view-first-page`.
- Two side branches reach the same depth: `agent-cli-spec` → `agent-cli-client` → `agent-surfd-supervisor` → `agent-session-process`, and `view-decisions` → `view-http-server` → `view-surfd-serves`.
- Each edge has a need behind it: the agent component, the session domain, the action log that the timeline shows, the session process, the published records, the routes.
- No edge lengthens the chain. The three redundant ones (`view-surfd-serves` → `agent-session-process` and → `agent-surfd-supervisor`, `agent-session-events` → `agent-surfd-supervisor`) are already implied and cost nothing.
- The cost the operator should know: the first view waits on `agent-session-spec`, the hardest story of the plan, at depth 2.

**First twelve stories to run**
1. `agent-cli-spec`
2. `html-decisions`
3. `view-decisions` — early so findings 1–5 can be settled in its document before `agent-session-events` is built.
4. `agent-answer-format`
5. `runner-decisions` — it needs an operator approval; let the wait overlap other work.
6. `agent-url-parser`
7. `agent-session-spec`
8. `html-conformance-harness`
9. `agent-cli-client`
10. `view-http-server`
11. `agent-page-path-spec`
12. `agent-surfd-supervisor`

`runner-surf-cli-package` comes next. The longest chain in the set is now 22 deep (23 in round 1), from `agent-cli-spec` to `runner-e2e`.

**What I could not establish**
- Acceptance lane: `agent-session-events` acceptance still says "the 12 numbered cases" although D17 adds a thirteenth.
- Acceptance lane: the `agent-session-process` summary still says "surf-agent open starts surf-session".
- Parallel-safety lane: `view-decisions`, `view-now-panel` and `view-controls-and-changes` all write `docs/view-stream.md`. They are chained by edges, so I found no defect.
- Whether `UnixStream::peer_cred` and `File::try_lock` are stable in rustc 1.99.1 is for the implementors; I did not compile.

```findings
- file: .engineering/planning/story/agent-session-events.md
  line: 46
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the kept log has two orders (events then action row in Change and case 1, action row then events then answer in the D17 addendum), and view-first-page attributes an entry to the action it follows, so one order must be stated and view-surfd-serves cases 13 and 14 must follow it"
- file: .engineering/planning/story/agent-session-events.md
  line: 108
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "D17 keeps the typed answer and says nothing changes in how a session publishes, but the link of agent-surfd-supervisor carries only the rendered answer lines and a status record, so no story writes the typed answer's encoding on the link and surfd would have to parse page text"
- file: .engineering/planning/story/agent-session-events.md
  line: 106
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "D17 keeps the typed answer of each action, and the typed answer of open holds the token block, while agent-surfd-supervisor relays the token without keeping it and this story's case 3 finds it in no record, so the kept answer entry must drop the token block"
- file: .engineering/planning/story/agent-session-events.md
  line: 56
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "records are stamped with a per-session sequence number from 1 and read per session, while docs/view-stream.md and view-http-server resume one /events stream across sessions from a single increasing entry identity, so no story assigns the identity Last-Event-ID needs"
- file: .engineering/planning/story/agent-session-events.md
  line: 37
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "every event is mapped with its fields, so ResponseReceived's body travels quoted in the link answer whose 4 MiB limit kills the session, and is kept by surfd which holds no page, while view-decisions and view-surfd-serves expect a byte field as its length only"
- file: .engineering/planning/story/view-surfd-serves.md
  line: 39
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it calls agent-session-process's random source and constant-time comparison, which live in surf-session that depends on surfd, while surfd may depend only on surf-agent and surf-answer yet draws the session identity, and view-http-server writes its own comparison in surf-view, so one std-only home reachable by all three must be named"
- file: .engineering/planning/story/agent-surfd-supervisor.md
  line: 80
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the session list keeps address and title already quoted by the session and never read, while view-surfd-serves serves them as JSON string values under docs/view-stream.md's escaping rule, so the status record must carry raw values or the view must document the quoted form"
- file: .engineering/planning/story/agent-session-events.md
  line: 32
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the log bound is per session and an ended session's log stays while it is listed, and agent-surfd-supervisor lists up to 256 ended sessions, so surfd's memory has no daemon-wide bound"
- file: .engineering/planning/story/view-surfd-serves.md
  line: 67
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it adds the task view-stream-check to the root Taskfile.yml, where D16 puts a new task in taskfiles/<name>.yml included by the root file"
- file: .engineering/planning/story/view-first-page.md
  line: 83
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it adds the task view-test to the root Taskfile.yml, where D16 puts a new task in taskfiles/<name>.yml included by the root file"
- file: .engineering/planning/story/agent-cli-spec.md
  line: 51
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it adds the task cli to the root Taskfile.yml, where D16 puts a new task in taskfiles/<name>.yml included by the root file"
- file: .engineering/planning/story/html-conformance-gate.md
  line: 60
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it adds the task html-conformance to the root Taskfile.yml, where D16 puts a new task in taskfiles/<name>.yml included by the root file"
```
