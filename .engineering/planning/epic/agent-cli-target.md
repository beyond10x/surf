---
format: aep.planning-md/3
id: epic:agent-cli-target
kind: epic
status: active
title: 'Agent target: a session command line for AI agents'
summary: An agent opens a session, gets a token, and drives the browser through a specified command line that answers with a dense text representation of what the page offers.
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 9}
---
## Context

The operator asked on 2026-11-20 and 2026-11-21 for a command line made for AI agents; his words
are quoted in `AGENTS.md` ("Implementation targets"). He set the order "a design first, then an
epic, and then a breakdown to stories" and accepted `docs/design/agent-interface.md` as the
basis for the epics on 2026-11-21 (`approval-record:agent-design-accepted-2026-11-21`). This
epic is rewritten from that design; the design is the reference for every detail not repeated
here.

What exists (`main` at `04e5c97`): `surf <url>` loads one page over plain HTTP in one process
and exits. A `Document` in the specification is an identity and an address. Nothing outlives a
process run. A first draft of a session domain is on branch `spec/agent-session` (`b48bc87`); it
validates and does not pass the gate, and `docs/plan/agent-cli-draft-1.md` holds its
measurements and the decisions taken on its markers.

## Outcome

An AI agent drives surf through a command line, `surf-agent`:

- A daemon, `surfd`, one per user, owns the list of sessions, starts and supervises one process
  per session, passes each call to the session it names and keeps the events the sessions
  publish (design, section 3, as amended on 2026-11-21). It holds no page.
- `open` starts a session and prints its identity and a token once. Every later call names the
  session and proves the token. One process per session holds the state; the command line is a
  stateless client that talks to `surfd` over a Unix socket in a directory only the user can
  read, and starts `surfd` when none is running.
- The agent's actions are commands of a new `agent` component in the ESS specification, mapped
  by bindings to the commands the five existing components specify. The command line is one
  front for that component.
- Every command answers in one line-oriented text format with four blocks: `page` (where the
  session is), `path` (the actions that led here), `changed` (what the last action changed,
  structurally) and `now` (every actionable element, one line each, with a handle, its kind,
  whether it is visible, its label and its state or target). Every value that comes from a
  page, a server or an address is quoted and escaped.
- What a page holds, on request: `--as tree|markdown` on `open <url>`, `go <url>` and `look` (and
  on `surf <url>`) prints the document's tree as an outline or its content as Markdown in a `text`
  block, every line of page text behind `>` (`docs/design/page-as-text.md`).
- The commands: `open`, `go`, `look`, `close`; `follow`, `back`, `forward`; `fill`, `pick`,
  `press`; `log`.

## Scope: steps 1, 3, 4 and 5 of the design's section 9

1. Session and command line over today's specification: the `surf.agent` domain revised to the
   decisions of `docs/plan/agent-cli-draft-1.md`, the session process, the socket and the token,
   `open`, `go`, `look`, `close`, the answer format with `page` and `path`, the `ess-cli/2`
   document and a parser written here that a test holds to it.
3. Links: `now` lists them; `follow`, `back`, `forward`; redirects in `surf.net`.
4. Forms: controls in `now`; `fill`, `pick`, `press`; `POST` and request bodies in `surf.net`.
5. Change and log: `changed` in every answer, text changes, `log`.

Also in scope, as a first step: the `surf` binary moves into a package of its own
(`crates/surf-cli`, `story:runner-surf-cli-package`), so that every later story that changes the
binary writes one place.

What needs what, at step level (there is no edge between the epics: three parser stories depend
on stories of this epic, and the later steps of this epic depend on parser stories):

- Step 1, the redirect model, request bodies and `POST` need nothing from `epic:html-parser`.
- Listing links and `follow` (step 3), controls and forms (step 4) and the `changed` block (step
  5) need the parsed page: `story:html-document-spec` for the rows and
  `story:html-document-port` for the tree and the filled rows.

Before a script engine exists, a document's tree does not change after parsing. What an agent
changes on a page is control state (value, checkedness, selectedness), which the specification
holds in rows. So the `changed` block of an action inside one document holds control-state
lines and added or changed text only where a new document arrived
(`docs/plan/agent-plan-revision-1.md`, D2).

## Out of scope

- The HTML parser and the node tree (`epic:html-parser`).
- Recording, scenarios and the runner (`epic:e2e-scenario-runner`).
- `--detail` and every detailed line (`epic:detailed-mode`, after a script engine).
- Visibility beyond the `hidden` attribute, scripts, HTTPS, the Chrome DevTools Protocol. Until
  visibility is computed, an element line says `?` or what the `hidden` attribute gives, and a
  `note` line says so.
- Deferred, each with a backlog draft tagged `deferred`: hashing the token, paging a long
  answer (`--max-lines`, `look --from <handle>`), a caller-chosen `tab_id` on `OpenTab`,
  `CloseWindow`, `--as` on `follow`, `back`, `forward`, `fill`, `pick` and `press`
  (`story:page-text-on-other-commands`). One exception to the paging item: the `text` block of
  page text is paged by `--from-line <n>` (`story:page-text-look`), with a `more` line of its
  own; `--max-lines` and `look --from <handle>` for `now` stay deferred.

## Constraints

- Specs first: each behaviour is in `spec/` with a green `task check` before its code.
- No third-party crate in anything that ships. Both ESS command-line generators write code that
  needs third-party crates (measured, `docs/plan/agent-cli-draft-1.md`), so the parser and the
  text format are written here.
- The token is never an argument word; it is read from the environment or a file. No secret
  value is: `fill <handle> -` reads the value from standard input (a decision of the
  decomposition, `story:agent-fill-pick`; the operator can overrule it).
- A token marked `one_time_response` stops Rust generation for the whole model (measured), so
  the session process issues and checks the token and the specification says so.
- A handle is assigned in document order and never drawn at random.
- No Python anywhere.

## Acceptance

`slot task check` exits 0 on `main`, and in that run four things hold, each shown by a line a
command prints:

1. The walk test of `story:agent-change-and-log` passes by name: an agent with nothing but
   `surf-agent` opens a session against a server the test starts, goes to a page with links and
   a form, follows a link by its handle, goes back with `back`, fills and submits the form, reads
   in `changed` what the submit changed and in `path` the actions it took, and closes the
   session.
2. The wrong-token test of `story:agent-session-process` passes by name: a call that names the
   session with another token is refused with exit status 1 and changes nothing.
3. Both conformance reports (`.out/report.json` and `.out/conform-rust.json`) hold, for each of
   the agent commands `OpenSession`, `Go`, `CloseSession`, `Follow`, `GoBack`, `GoForward`,
   `Fill`, `Pick` and `Press`, at least one passed scenario for every outcome the command
   declares, and `task conform-totals` prints one total for both runs that is larger than the
   51 of `main` at `1c979b4`.
4. The test `go_as_markdown_prints_the_page` of `story:page-text-look` passes by name:
   `go <url> --as markdown` to the `Offers` page of `docs/design/page-as-text.md` section 1
   prints `text markdown 3 of 3 lines from 1, 62 of 62 bytes` and its three body lines, each
   behind `>`.
