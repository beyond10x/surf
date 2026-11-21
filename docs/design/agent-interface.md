# Design: the agent interface and the scenario runner

Status: **accepted** by the operator on 2026-11-21 ("A" to "is this design the basis for the
epics?", then, after reading it: "I'm accepting it"). Written by the coordinating agent. The operator set
the order: "a design first, then an epic, and then a breakdown to stories". This is the design.
`epic:agent-cli-target` and `epic:e2e-scenario-runner` are drafts that record what was asked;
they are rewritten from this document once it is accepted.

Every name of a standard below is written from memory and is checked when its slice is
specified. Where this document proposes, it says "proposed". Where something was measured, it
says where.

## 1. What was asked

The operator's words are quoted in `AGENTS.md` ("Implementation targets") and in the two draft
epics. In short:

1. A command line made for AI agents. An agent opens a session, gets a token, and every later
   call names the session.
2. Every answer is a dense text representation of what can be done on the page: where it can
   navigate, which buttons can be pressed, which links exist and which of them are visible.
3. An answer also shows what led here (where the agent has been) and what the last action
   changed on the page, structurally.
4. A detailed mode, for recording, shows each change itself: a class gained, a font changed.
5. From the transcript of an agent's session, a deterministic end-to-end test is built, and surf
   runs it fast. "One purpose of surf would also be to have a super fast end-to-end scenario
   runner, like test browser."
6. Spec first. The command line is specified, and the agent's action surface is mapped to the
   core operations of the browser.
7. A wish, stated as beside the point for now: support for "this protocol from Google" (read
   as the Chrome DevTools Protocol).

## 2. What exists (`main` at `2bcfe46`)

- `surf <url>` loads one page over plain HTTP in one process and exits.
- The specification has five components. A `Document` is an identity and an address: no node
  tree, no link, no control, no style (`spec/domains/document.yaml`, the `UNMAPPED:` marker).
- No script engine, no CSS, no layout, no HTTPS, no redirect handling, no request method other
  than `GET`, no request body.
- Nothing outlives one process run.
- A first draft of a session domain is on branch `spec/agent-session`
  (`docs/plan/agent-cli-draft-1.md`). It validates and does not pass the gate. Its measurements
  stand: both ESS generators for a command line write code that needs third-party crates, and
  `ess-cli/2` prints JSON only.

So most of this design stands on things that do not exist yet. Section 9 orders them.

## 3. The parts

Amended on 2026-11-21 after the operator proposed a daemon: "What if there is a surfd which is
the surf daemon which is what owns or maintains the active browser sessions and [the] CLI [...]
would just be some thin client over it and there would also be some web UI showing all the
active sessions on that daemon and we're using real-time events [of] ESS to basically push to a
nice UI what's currently going on semantically, not like on a HTML tag basis." The first
version of this section had no daemon and rejected one that hosts every session.

```
agent ──runs──> surf-agent <command>       person ──browser──> http://127.0.0.1:<port>/
                    │ Unix socket                                   │ HTTP and server-sent events
                    ▼                                               ▼
                  surfd                    one per user: the list of sessions, routing,
                    │                      the event relay, the live view
                    │ starts and supervises; one socket per session
                    ▼
               session process             one per session: the page, the `agent` component
                    │ in process
                    ▼
     agent component ── bindings ──> shell, navigator, net, document, renderer
```

- **`surf-agent`** is a client. It parses its arguments, reads the token, sends one request to
  `surfd` and prints the answer. It holds no state. `open` starts `surfd` when none is running.
- **`surfd`** is the daemon, one per user. It owns the list of sessions (identity, state, when
  opened, current address), starts one session process per session and supervises it, passes
  each request to the session it names, keeps the events each session publishes, and serves the
  live view (section 12). It holds no page and parses no page, no header and no body: a page
  that breaks its session process has not reached `surfd` or another session. The only text of
  a session it reads is the answer grammar, through `surf-answer`'s parser, when the live view
  asks for an answer as data (`docs/plan/agent-plan-revision-2.md`, D20).
- **The session process** holds one session: the assembled system (`crates/surf`, `Browser`)
  plus the new `agent` component, the action log, and the last page state it showed. It
  publishes its events to `surfd` as they happen.
- **The `agent` component** is specified in ESS like the other five. Its commands are the
  agent's action surface. Bindings map its events to the core commands
  (`surf.navigation.Navigate`, later the document's interaction commands). This is the mapping
  the operator asked for, and it is the only place the action surface exists: the command line
  is one front for it, the scenario runner a second, and any other protocol would be a third.

Why a process per session under the daemon (decided, reversible): page state will include a
script heap, which cannot be written to a file between calls; and one process per session is
the isolation the goals ask for (later a micro VM per session, the same boundary). Rejected: a
state file read and written by each call (stops working with the first script), and a daemon
that hosts every session in its own process (one compromised page reaches every session).
`surfd` is the part that does not touch pages, so that it can be the part that knows them all.

What `surfd` adds to the risk, and the answer: it is one more listener and it sees every
session's events, which carry page text (titles, labels). It listens on a Unix socket in a
directory only the user can read and, for the live view, on 127.0.0.1 only. Section 12 says
what guards the HTTP side.

### Session directory, token, socket (proposed)

- Directory `$XDG_RUNTIME_DIR/surf/`, mode 0700, holding `surfd`'s socket and one directory per
  session with that session's socket, which only `surfd` connects to. No
  `XDG_RUNTIME_DIR`: refuse, do not fall back to a shared directory.
- The session identity is drawn by `open` and is not secret. It appears in listings.
- The token is 32 bytes from the operating system's random source, 64 hexadecimal digits,
  printed once by `open`. The session process keeps only a hash of it and compares in constant
  time.
- `surf-agent` never takes the token as an argument word, because other local users can read a
  process's arguments. It reads `SURF_TOKEN` from the environment or a file named by
  `--token-file`. (The draft's `--token-stdin` and `--token-prompt` are dropped: an agent has no
  terminal.)
- The session process also checks the peer's user id on the socket. This needs a system call
  the standard library does not expose in stable Rust; whether it can be done without `unsafe`
  and without a crate is open, and if not, the directory mode and the token carry it.
- Idle timeout, proposed 30 minutes, then the session closes itself.

## 4. The commands

Proposed. Few and short, because an agent pays for every word.

| command line | agent command (ESS) | core operation it maps to | slice |
|---|---|---|---|
| `open [<url>]` | `OpenSession`, then `Go` | `OpenWindow`, `OpenTab`; `Navigate` | 1 |
| `go <url>` | `Go` | `Navigate` | 1 |
| `look` | view `SessionState` | none (a read) | 1 |
| `close` | `CloseSession` | `CloseTab` | 1 |
| `back`, `forward` | `GoBack`, `GoForward` | `Navigate` to a history entry | 3 |
| `follow <handle>` | `Follow` | `Navigate` to the link's target | 3 |
| `fill <handle> <text>` | `Fill` | the document's control gets a value | 4 |
| `pick <handle> <option>` | `Pick` | a select, checkbox or radio control changes | 4 |
| `press <handle>` | `Press` | a button's activation: form submission, later a script | 4 |
| `log [<n>]` | view `SessionLog` | none (a read) | 6 |

Every command, reads included, answers in the format of section 5. Options on every command:
`--detail` (section 5.4), `--max-lines <n>`.

Exit status: 0 the action was taken; 1 the action was refused (unknown handle, hidden or
disabled control, closed session); 2 usage; 3 the session process cannot be reached or stopped.
A refusal still prints an answer: the reason, then the current state.

## 5. The answer

One format for every command. Line-oriented text, printable ASCII only: every value that comes
from a page, a server or an address is escaped the way `crates/surf` `escape` does since review
1 and written inside double quotes. This matters more here than for a person: the reader is a
language model, and page text is untrusted input to it. The format never lets page text start a
line or look like a field of the format.

### 5.1 Example (proposed grammar, slice 4 and 6 present)

```
surf 1 session 7f3a open
did fill f1
page 200 text/html http://example.org/login "Sign in"
path 3 of 3
  1 go http://example.org/
  2 follow l4 "Sign in"
  3 fill f1
changed 1
  ~ f1 value "" -> "ada"
now 5
  l1 link vis "Home" -> /
  l2 link hid "Admin" -> /admin
  f1 text vis "User name" = "ada"
  f2 password vis "Password" = (set)
  b1 button vis "Sign in" submits F1 post /session
```

- Line 1: format version, session, session state.
- `did`: the action this answer is for, or `refused <reason>`.
- `page`: status, media type, address, title. `page none` before the first load; `page failed
  <reason> <address>` after a failed one.
- `path`: the last actions of the session, oldest first, with the total. Default 5 lines;
  `log` prints all.
- `changed`: what the action changed, section 5.3.
- `now`: every actionable element, one line each, in document order.

### 5.2 An element line

`<handle> <kind> <visibility> <label> <state or target>`

- **kind**: `link`, `button`, `text`, `password`, `checkbox`, `radio`, `select`, `textarea`,
  `file`. Later: an element a script made clickable.
- **visibility**: `vis`, `hid`, or `?` while visibility is not computed (before slice 5 the
  `hidden` attribute is all that is known, and the answer says so in a line `note visibility:
  hidden attribute only`). The completeness rule of `AGENTS.md` applies: the format never claims
  `vis` on a basis the specification does not have.
- **label**: the element's accessible name (WAI-ARIA accessible name computation, from memory):
  its text, its `label`, `aria-label`, `alt`, `title`, `placeholder`, in the standard's order.
  Cut at 60 characters with `…` written as `...`.
- **state or target**: a link's target, resolved and then shortened to a path when it is on the
  same origin; a control's value; `disabled`; `checked`; `required`. A password's value is never
  printed: `(set)` or `(empty)`.
- Elements that exist and cannot be used (hidden, disabled) are listed, flagged. That is the
  "technically available" against "visible" distinction the operator named.

A page with 400 links would cost an agent 400 lines. Proposed: `--max-lines`, default 120;
past it the answer ends with `more <n> elements: look --from <handle>`. Whether to rank by
visibility or keep document order is open (section 10).

### 5.3 What changed

Computed by the session process between the page state of the previous answer and the state
after the action, over the node tree.

- Same document: one line per actionable element that appeared (`+`), disappeared (`-`) or
  changed (`~ <handle> <what> <before> -> <after>`), where `what` is `label`, `value`, `target`,
  `visibility`, `enabled`, `checked`.
- New document: `changed page <old address> -> <new address>`, and no per-element lines: every
  element is new, and listing them repeats `now`.
- Text that is not part of an actionable element (a message "Wrong password" appearing) is what
  an agent most needs after a submit, and it is not an element line. Proposed: `+ text "Wrong
  password"` for text nodes added or changed, cut at 120 characters, at most 10 lines.

### 5.4 Detailed mode

`--detail` adds, under `changed`, every difference of the node tree and not only of actionable
elements: attribute set or removed, class added or removed, text changed, node added, removed
or moved, and, once style is computed, a computed style property that changed
(`~ b1 class +active`, `~ h1 style font-size 16px -> 20px`). It is the mode a recording uses.
It needs the node tree (slice 2) for attributes and classes and computed style (slice 5 and
after) for styles; before that it prints what it has and a `note` line saying what is missing.

### 5.5 Handles

A handle is a kind letter and a number (`l4`, `f1`, `b1`).

- Assigned by the session, in document order, the first time an element is shown, and kept for
  as long as that node is in the document. A node added later gets the next number. A new
  document starts again at 1.
- Deterministic: the same actions on the same pages give the same handles, because nothing in
  the assignment is random or timed.
- A handle is a name inside one session. A recording does not store handles alone (section 7).

## 6. In the specification

Proposed split, following the rule the first target already uses (the specification states what
is observable; work outside the model is a port):

- **`surf.agent`** (new): `Session` (Open, Closed), `Action` rows owned by a session (sequence
  number, command, input, outcome) which are the log, and the commands of section 4. Views:
  `SessionState`, `SessionLog`. The draft on `spec/agent-session` is the start, revised by the
  decisions in `docs/plan/agent-cli-draft-1.md`.
- **`surf.document`** grows the observable part of a page: the document's title and, as
  entities owned by a document, its actionable elements with the fields of section 5.2. Whether
  the whole node tree becomes entities is open (section 10); the detailed mode needs to observe
  it, and a specification that holds every node of every page is a large model.
- **The HTML parser** is a port of the `document` component, as the HTTP client is a port of
  `net`: `ParseDocument` stays the command, and what it creates is computed by hand-written
  code. The parser is held to the WHATWG HTML parsing algorithm by that standard's own test
  data (the html5lib tests, from memory), as test tooling. It is called complete only when it
  passes them.
- **`surf.net`** grows what form submission needs: a method, a request body, redirects.
- **The answer format** is specified where a format can be: its grammar in this repository
  (`docs/` or a schema), with golden tests. ESS has no text rendering of a view row (measured).
- **The command line** is specified as an `ess-cli/2` document that `ess` validates. The parser
  is written here from it, standard library only, with a test that walks the document and the
  parser side by side so they cannot drift.

Known obstacles from the first draft, to be solved in slice 1: a token marked
`one_time_response` stops Rust generation for the whole model, so the token is checked by the
session process and the specification says so; synthesized scenarios need a caller-chosen
session identity.

## 7. Recording and running scenarios

- **A recording** is what the session process writes as the session runs: each action, what it
  changed, and every exchange with the network (`docs/design/scenario-runner.md`; the detailed
  mode's lines are not part of it before scripts exist, revision 1, D3).
  `surf-agent open --record <file>` writes it as the session runs.
- **A scenario** is made from a recording by a separate step (`surf scenario from <recording>`),
  so a person or an agent can edit what it asserts. Each step holds:
  - the action, with a **locator** in place of a handle: kind, accessible name, and the
    element's path in the tree as a tiebreak. A scenario survives a link being added above the
    one it uses; a handle would not.
  - the expectations after it. Proposed default: the page's address and status, and the
    `changed` lines of the normal mode (structural). The detailed lines are kept in the
    recording and are asserted only where someone marks them. Asserting every style and class
    fails on the first cosmetic change of the site.
- **Running** is `surf run <scenario>`: one process, no socket, no session process, the same
  `agent` component driven directly. Output: one line per step, pass or fail, and for a failure
  the expected and actual `changed` lines. Exit 0 only when every step passed.
- **Determinism**, what it takes:
  - handles and locators as above; session identity and token never compared;
  - the network: a scenario runs **live** (against the site, which can change) or **replayed**
    (the recording also stores each request and response, and the run answers from them through
    the transport, which `crates/surf` already takes as a parameter). Proposed: both, chosen per
    run; replayed is the default for a test in a gate.
  - time and randomness, once scripts run: a fixed clock and a seeded source in a run. Not
    needed before the script engine.
- **Format** of the scenario file: open (section 10). `ess-scenario/6` drives ESS commands and
  asserts on views, which is what a step is; whether a recorded session can be written as one
  and run by the ESS runner is not explored.
- **Speed**: no window, no painting, no layout a step does not need, no process start per step.
  A number is set when there is something to measure.

## 8. Security

- Page content is untrusted input to a language model. The format's answer to that is section
  5: quoting, escaping, cuts on length, and no page text outside quotes. It lowers the risk of
  a page steering the agent; it does not remove it, and the documentation for agents says so.
- The token and the socket: section 3.
- One process per session; later the isolation target's boundary around it.
- `file:` and other local schemes are refused for an agent session unless a flag allows them.
- A recording can hold what the agent typed, passwords included. Proposed: values of password
  controls are written as a reference to a variable (`${PASSWORD_1}`), supplied at run time.

## 9. Order (proposed)

What must exist, in the order it unblocks the next thing:

1. **Session and command line over today's specification**: `surfd`, `open`, `go`, `look`, `close`, the
   session process, the answer format with `page` and `path`. No elements yet.
2. **HTML parsing and the node tree**: the parser port, its conformance data, the document's
   title and elements in the specification.
3. **Links**: `now` lists them; `follow`, `back`, `forward`; redirects in `net`.
4. **Forms**: controls in `now`; `fill`, `pick`, `press`; `POST` and request bodies in `net`.
5. **Change and log**: `changed` in every answer, `log`, text changes.
6. **Recording and the runner**: `--record`, `surf scenario from`, `surf run`, replayed network.
7. **Visibility**: `hidden`, then CSS parsing and the cascade for `display` and `visibility`,
   then layout for the viewport.
8. **Detailed mode** in full: computed style changes.
9. **Scripts**: the engine (its own design). Until it exists, surf drives pages that work
   without scripts and no others. A site built as a script application shows an empty page.

Steps 1 to 6 give the whole loop the operator described (send an agent, record, replay) on
pages that work without scripts. Steps 7 to 9 widen what pages it works on.

This is more than one epic. Proposed: `agent-cli-target` = steps 1, 3, 4, 5; a new epic for
step 2 (the parser is the largest single piece and has its own standard and tests);
`e2e-scenario-runner` = step 6; steps 7, 8 and 9 each their own epic later. The live view
(section 12) is `epic:live-view`; it needs step 1 and grows with steps 2 to 5. Since the plan's
revision (`docs/plan/agent-plan-revision-1.md`, D3) the detailed mode, step 8, comes after the
script engine, step 9: nothing mutates a document's tree before it.

## 10. Open

1. Whether the whole node tree is modelled in ESS or only the actionable elements, with the
   tree behind the parser port (section 6).
2. The scenario file format, and whether it can be `ess-scenario/6` (section 7). Answered in
   `docs/design/scenario-runner.md`, Q1: surf's own line-oriented formats.
3. Ranking and paging of long pages in `now` (section 5.2).
4. Checking the socket peer's user id without `unsafe` and without a crate (section 3).
5. Whether the answer is also offered as JSON. Proposed: no, until something asks for it.
6. The Chrome DevTools Protocol. Not explored. It would be a third front on the `agent`
   component's commands. WebDriver BiDi (W3C, from memory) is the standard with the same aim.
7. HTTPS. A site an agent is sent to test is served over HTTPS in practice. TLS is its own
   piece of work under decision C (protocol written here, audited primitives from a crate), and
   it is a prerequisite for the scenario runner being useful outside a local test server.

## 11. Decisions in this draft that the operator can overrule

- A daemon, `surfd`, that supervises one process per session and holds no page itself; the
  command line talks to `surfd` over a Unix socket.
- A live view served by `surfd` on 127.0.0.1, read-only, behind a view token (section 12).
- The token is read from the environment or a file, never from an argument.
- The parser and the dense text format are written here; the `ess-cli/2` document specifies the
  command line and a test holds the parser to it.
- The HTML parser is a port, held to the standard's test data, not modelled in ESS.
- A scenario stores locators, not handles, and asserts structural changes by default.
- Replayed network is the default for a scenario in a gate.
- Scripts come last; before them only pages that work without scripts are in reach.

## 12. The live view (added 2026-11-21)

The operator: "some web UI showing all the active sessions on that daemon and we're using
real-time events [of] ESS to basically push to a nice UI what's currently going on
semantically, not like on a HTML tag basis. UI wise you could just orient yourself with the SDC
project and the live view". The reference is the SDC run inspector (`sdc/ui` in the
organization's folder; read: its `README.md`, `server.mjs` and `screenshots/wide-live.png`). It
is proposed here; `epic:live-view` holds the work.

### What it shows

Semantic, in the specification's terms, never a tree of tags:

- **Sessions** (left): every session `surfd` knows, with its state, its current address and
  title, and how long ago its last action was. Closed sessions stay listed, greyed, until
  `surfd` drops them.
- **The session** (centre), for the one selected:
  - the `page` line: status, media type, address, title, or the failure and its reason;
  - what can be done now: the actionable elements the agent's `now` block lists, grouped by
    kind (links, buttons, fields), each with its handle, label, state or target, and whether it
    is visible;
  - what the last action changed: the `changed` block.
- **Timeline** (bottom): the session's actions in order (`go`, `follow`, `fill`, `press`,
  `back`), each a segment; under it the events of the components as they happened
  (`NavigationStarted`, `RequestStarted`, `ResponseReceived`, `NavigationCommitted`,
  `DocumentParsed`, and what the agent domain adds). A playhead: the centre shows the session as
  it was after the selected action. "Latest" follows live.
- **Detail** (right): the selected action or event with its fields; the session's requests with
  status and size; for a recorded session, a link to the recording.

Taken from the SDC inspector: a list on the left, the subject in the centre, a timeline with a
playhead at the bottom, a detail panel on the right, a "live" marker with the time of the last
update, links that carry the selection in the address, plain JavaScript without a framework and
without a build step.

### How it gets its data

- The events are the specification's events. Each session process publishes what its system
  published (`System::published()`, in order) and the rows of its action log to `surfd`, which
  keeps a bounded log per session.
- `surfd` serves `GET /api/sessions`, `GET /api/sessions/<id>` (the same views `look` and `log`
  read, plus the kept events) and `GET /events`, a server-sent-event stream of everything that
  happens from then on. The page loads the state once and then follows the stream.
- The shape of the stream is specified, not invented beside the specification: ESS projects a
  system's events as an AsyncAPI document (`ess generate --kind asyncapi`; not run in this
  repository yet, and what it gives for components that are `in_process` is to be measured by
  the first story). The HTTP routes are specified as an `ess-http/1` binding if ESS accepts one
  for views that are only read; if not, they are specified here in `docs/`.
- `surfd` serves the page's files itself. The server is written here with the standard library:
  HTTP/1.1, `GET` only, a fixed set of routes, server-sent events. No Node, no third-party
  crate, nothing fetched from the network by the page.

### What guards it

- 127.0.0.1 only, on a port `surfd` picks and prints. A request whose `Host` is not that
  address and port is refused (DNS rebinding).
- A view token, drawn by `surfd` and printed with the address, required on every route that
  answers with data (the page's own files hold none and are served without it, revision 1,
  D18). Without
  it any program of any local user that can reach the port reads every session's page text.
- Read-only. The first version of the view cannot act on a session; no route changes anything.
  Acting from the view would be a later decision, as the SDC inspector's `--write` is.
- Page text is untrusted here too. The page writes it as text and never as markup, and the
  server sends a content security policy that allows no inline script and no outside origin.
- The token of a session never reaches the view or the event stream.

### What it costs

Two stories more in `epic:agent-cli-target` (`surfd` as supervisor and router; sessions
publishing their events to it) and a new epic of about six (the specified event stream; the
HTTP and event-stream server; the page with the session list; the session view; the timeline
and live follow; tests that drive the page). The parser and runner epics do not change. The
view shows links and controls only once the parser epic has delivered them; before that it shows
sessions, pages, actions and events.
