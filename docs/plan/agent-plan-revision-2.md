# The agent interface plan: revision after critic round 2

Round 2 of the plan critic panel read the four epics (`epic:agent-cli-target`,
`epic:html-parser`, `epic:e2e-scenario-runner`, `epic:live-view`) and their 57 draft stories at
`497dedd`. Four critics, four `needs-revision`, 46 findings (14 blockers). Every round-1 finding
was found answered, except the two session-directory refusals, which have no numbered case yet.
Each report is stored as written:

| critic | findings | blockers | record |
|---|---|---|---|
| scope | 7 | 2 | `review-result:agent-plan-critic-scope-round-2` |
| acceptance | 15 | 6 | `review-result:agent-plan-critic-acceptance-round-2` |
| design | 12 | 5 | `review-result:agent-plan-critic-design-round-2` |
| parallel safety | 12 | 1 | `review-result:agent-plan-critic-parallel-round-2` |

Where the findings are: 11 of the 14 blockers are in `story:agent-session-events` and the view
stories that read what it keeps. That is the seam round 2 was sent to look at: the daemon
stories and the view stories were written by two agents that could not see each other's drafts.
The parser and runner stories drew acceptance corrections and the `Taskfile.yml` rule, nothing
about their shape.

The panel has two rounds, so there is no third. The operator accepted the plan for
implementation before round 2 (`approval-record:agent-plan-accepted-2026-11-21`); the decisions
below are the coordinating agent's, taken on 2026-11-21, reversible, and he can overrule each.
The decomposers revise the stories to them and to the findings; the coordinator checks each
finding against the revised text before the stories move to `active`.

## Decisions

**D19. One order in the kept log.** For each action: the action entry, then the events of that
action in the order the components published them, then the answer entry. An entry belongs to
the action entry it follows. `story:agent-session-events` states it once; every case in the view
stories follows it. (acceptance 1 and 6, design 1)

**D20. The answer travels as its lines, and `surf-answer` reads them back.** The link between a
session process and `surfd` keeps carrying the rendered answer lines; there is no second
encoding of an answer. `surf-answer` gains the reverse direction: `unquote` for one quoted
value and `parse` from lines to the typed answer, with the property that parsing what `render`
wrote gives the value back, for every golden file and for the forging matrix.
`story:agent-answer-format` writes both for its blocks; every later story that adds a block adds
its parse and its round-trip case. `surfd` keeps the lines of each answer as the answer entry
and does not read them when it keeps them; the view routes call `surf_answer::parse` to write
JSON. D17's "typed answer" is this: the answer's lines, read by one parser. Reason: a second
encoding would have to be extended by every story that adds a block, and nothing would hold it
equal to the first; a parser held to the renderer by a round trip also proves that the grammar
is unambiguous, which the forging cases need anyway. The design's sentence "parses nothing a
server sent" is amended in section 3: `surfd` parses no page, no header and no body; the only
text of a session it reads is the answer grammar, through `surf-answer`. (scope 1, design 2)

**D21. The token is not kept.** The answer entry of `open` omits the `token` line. A named case
finds the token in no entry and in no route's answer. (design 3)

**D22. What the log holds of a value.** A field of bytes (a response body, a request body) is
written by the session process as its length, `<n> bytes`, never as the bytes: the link's
answer limit is 4 MiB and a body may be 16 MiB, and `surfd` holds no page. A password's value is
its length, as the answer prints it. Every other control value is kept as the answer prints it,
which is what the `changed` panel shows. (scope 2, acceptance 5, design 5)

**D23. One identity across sessions.** `surfd` stamps every entry it keeps with a number that
increases across all sessions, beside the per-session number, and offers a reader from a number:
entries after it, or the statement that the number is older than what is kept.
`story:agent-session-events` owns both; `/events` and `Last-Event-ID` use them. (scope 3,
design 4)

**D24. One bound over all logs.** Beside the bound per session, `surfd` keeps at most 64 MiB of
entries over all sessions. Above it, the logs of ended sessions go first, the longest-ended
first, and such a session's detail says that its log was dropped. `story:view-decisions` item 7
follows the bounds of `story:agent-session-events`. (design 8)

**D25. One home for secrets.** A new story at depth 1, `story:agent-secret-primitives`: the
crate `crates/secret` (`surf-secret`, standard library only) with the read of the operating
system's random source, the comparison in constant time, and the 32-byte token with its form
of 64 hexadecimal digits. `agent-surfd-supervisor`, `agent-session-process` and
`view-http-server` depend on it and write none of the three themselves. `surf-ports`
`OsRandom` stays as it is in this plan; `story:random-source-failure-cause` (backlog) notes
that it moves onto `surf-secret`. (scope 5, design 6)

**D26. Text on the link stays quoted.** The link is printable ASCII: an address, a title and an
event field arrive quoted by `surf-answer`'s quoting function, and `surfd` keeps them so. It
reads them only through `surf_answer::unquote`, in the view routes, which then write the exact
value as a JSON string. (scope 4, design 7)

**D27. Who maps an event.** `story:agent-session-events` gains an edge to
`story:agent-redirect-spec` and maps every event and field on `main` when it starts. A story
that adds an event or an event field after that maps it in `crates/session`, with a named case,
and depends on `agent-session-events`: the stories of `Follow`, of `GoBack` and `GoForward`,
and `story:html-document-spec`. (scope 6, parallel 1)

**D28. `--record` under `surfd`.** The client sends an absolute path. The refusal of a path
that cannot be recorded to is the session process's answer to `open`, with exit 2, and `surfd`
lists no session for it. (scope 7)

**D29. How a test reads the session list.** Cases that assert the list run `surfd` in the test
process, through its library. Cases that run the built `surfd` assert only what an answer
prints. (acceptance 7)

**D30. No unit writes the root `Taskfile.yml` in a wave of more than one unit**, and that
includes `REFUSALS_MAX`. A specification story runs its gate as
`slot task check REFUSALS_MAX=<n>` (measured: `task -n conform REFUSALS_MAX=99` prints
`-le 99`), reports `<n>` and each new refusal, and the coordinator writes the number on the
wave branch. A new task goes to `taskfiles/<name>.yml` (D16). (design 9 to 12, parallel 3 to
12)

**D31. The parser's "not yet parsed" names.** Each entry of `surf_html::NOT_YET_PARSED`
matches `[a-z-]+`, as the answer's renderer requires. A golden test that holds a `note parser`
line builds it from the constant, so a parser story that shortens the list breaks no other
story's test. (parallel 2, and the parallel critic's note on the entries with spaces)

## Findings inside one story

The acceptance findings that name one story are the decomposers' in full: the case numbering
and the cases without numbers in `agent-session-events`, the two session-directory refusals in
`agent-surfd-supervisor`, the write limit in `view-http-server`, the refusal limit and the
floor in `html-document-spec`, `51` in `agent-cli-spec`, `review2_output` in
`runner-surf-cli-package`, the test names in `runner-scenario-from`, the six routes in
`view-decisions`. `epic:e2e-scenario-runner`'s acceptance is the coordinator's edit: it names
`slot task check` and the test `record_and_run`.

## After round 2: found in wave 1

**D32. The `agent` component arrives with its commands.** `story:agent-cli-spec` was to add a
component `agent` that owns `surf.agent` and accepts nothing, and the sixth port with it. ess
0.96.0 generates for every component a port with a field `behaviors`, which only command and
view handlers read (`ess-synth/src/rust/port.rs:158`); for a component with neither, nothing
reads it, and the gate builds with warnings denied (`error: field 'behaviors' is never read`,
`generated/rust/crates/agent/src/lib.rs:28`). So `story:agent-cli-spec` lands the domain
`surf.agent` (types only) owned by `net`, as `net` owns `surf.url`, with the command-line
document and its task; `story:agent-session-spec`, which gives the component its commands, adds
the component, moves the domain to it and assembles the sixth port. The first attempt, with
the hand-written assembly measured green under `-A dead_code` (140 tests, 51 of 51 scenarios),
is kept on the branch `park/agent-sixth-port` (`9e43dcf`) for that story to start from.
Rejected: allowing dead code in the gate (weakens it for every crate); waiting for an ESS
release (the operator has not yet said whether ESS issues are filed). The defect is recorded as
an ESS finding in the wave page.
