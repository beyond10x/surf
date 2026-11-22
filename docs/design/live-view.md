# The live view: what ESS gives it, and the decisions it is built on

Written 2026-11-21 by `story:view-decisions` (`epic:live-view`), on `main` at `7a6298b`, with
ess 0.96.0. The coordinating agent's decisions, reversible; the operator can overrule each item,
and each item says what he would be overruling. The routes and the stream these items lead to
are `docs/view-stream.md`. Design section 12 (`docs/design/agent-interface.md`) is the proposal
this note measures.

Every command below ran in this repository's worktree; scratch output is under
`.out/view-measure/`, which git ignores. Nothing under `spec/` or `generated/` changed.

## Measurements

### (a) The AsyncAPI and schema projections

```console
$ ess generate --path spec --kind asyncapi --out .out/view-measure/asyncapi
asyncapi/document.yaml — 7658 byte(s)
asyncapi/navigator.yaml — 16408 byte(s)
asyncapi/net.yaml — 10814 byte(s)
asyncapi/renderer.yaml — 3414 byte(s)
asyncapi/shell.yaml — 5859 byte(s)
5 artifact(s), written to .out/view-measure/asyncapi
$ ess generate --path spec --kind schema --out .out/view-measure/schema
…
53 artifact(s), written to .out/view-measure/schema
```

- The AsyncAPI projection is YAML, `asyncapi: 3.0.0`, one document per component. The schema
  projection is JSON, `"$schema": "https://json-schema.org/draft/2020-12/schema"`, one file per
  command, entity, error, event and type (12 under `events/`).
- Every component is `reached_by: in_process`, and every event still appears as a message: one
  channel per event, its address the qualified name (`x-ess-address-source: qualified-name`).
  Counted over the five documents, `grep -h 'x-ess-event:' | sort | uniq -c` lists all 12 events
  of `spec/`; `grep -c 'action: send'` gives 12 publishing operations (net 3, navigator 4,
  shell 3, document 1, renderer 1), one per event, each in the document of the component that
  publishes it. The projection says why
  nothing else is there: "The specification declares no transport, so each address below is a
  name and not a topic on a named broker."
- Field types in the payload schemas (`net.yaml`, `event.surf.net.ResponseReceived`):
  `surf.net.RequestId` is `type: string`, `format: uuid` with the UUID pattern; `surf.url.Url` is
  `type: string`; `surf.net.Status` is `type: integer`; the `body` of `ResponseReceived` (and of
  `surf.navigation.NavigationCommitted`) is `type: string`, `contentEncoding: base64`. Every
  payload is `additionalProperties: false` with every field `required`.

### (b) An `ess-http/1` binding for the read views

The binding, on a copy of `spec/` under `.out/view-measure/spec/`
(`.out/view-measure/http-views.yaml`):

```yaml
type: ess-http/1
specification:
  system: surf
  version: v1
  source_digest: sha256:dac2ef7448ba14b90fd4de262ec4155271c6b76114b4fd648a167b75924d0e6a
routes:
  - view: surf.net.Requests
    method: get
    path: /api/requests
  - view: surf.navigation.Navigations
    method: get
    path: /api/navigations
```

With the components left `in_process`, it is refused:

```console
$ ess specify http validate --path .out/view-measure/spec --binding .out/view-measure/http-views.yaml
.out/view-measure/http-views.yaml was refused:
error[ESS-HTTP-013]: routes[0].view: no `reached_by: network` component owns the domain of view `surf.net.Requests`, so it has no HTTP surface to route
error[ESS-HTTP-013]: routes[1].view: no `reached_by: network` component owns the domain of view `surf.navigation.Navigations`, so it has no HTTP surface to route
```

On a second copy, `.out/view-measure/spec-network/`, with `net` and `navigator` changed to
`reached_by: network` and nothing else, the same binding (with that copy's digest) validates:
`.out/view-measure/http-views-network.yaml — 2 route(s), 0 open, valid`. The gate does not:

```console
$ ess specify validate --path .out/view-measure/spec-network
refusal[ESS-AUTHOR-040]: `surf.navigation/authored/page-load-commits-and-parses` …
  `surf.net.DeliverResponse` is accepted by a served component and no declared actor is granted it, so every caller is refused it and it cannot run
…
surf v1 — 8 file(s), 3 scenario(s), 4 scenario refusal(s)        (exit 1)
$ ess verify conform synthesize --path .out/view-measure/spec-network --scenarios .out/view-measure/spec-network --out …
(the same four ESS-AUTHOR-040 refusals; exit 1, no suite written)
```

| | `spec/` as it is | `net`, `navigator` on the network |
|---|---|---|
| `validate` | `surf v1 — 8 file(s), 3 scenario(s), valid` | exit 1, 4 scenario refusals (`ESS-AUTHOR-040`) in all 3 authored scenarios |
| `synthesize`, with the authored scenarios (the gate's `task suite`) | `51 scenario(s) (3 authored), 10 refusal(s)` | exit 1, no suite |
| `synthesize`, without them | `48 scenario(s) (0 authored), 10 refusal(s)` | `25 scenario(s) (0 authored), 10 refusal(s)`, and 5 notes withholding the outcome scenarios of every command no actor is granted |

What `--kind openapi` writes: with `spec/` as it is, 5 documents with 12 paths, all of commands
(`/net/commands/fetch` and the like). With the network copy, the two views gain paths
(`/net/views/requests`, `/navigation/views/navigations`). With the network copy and `--http`, the bound views move to the
binding's paths (`/api/requests`, `/api/navigations`, `x-ess-route-source: binding`) and every
command path stays.

What a binding could not say even where it validates: the routes of the view are over `surfd`'s
session list and its kept logs, which are bookkeeping of the supervisor and not rows of `spec/`
(`story:agent-surfd-supervisor`); and a binding of 0.96.0 as measured routes views and commands,
not a token check or a server-sent event stream. The ESS project has stories for both in its own
plan (`an-http-binding-declares-how-a-caller-authenticates`,
`an-event-channel-may-be-bound-to-server-sent-events`); whether 0.96.0 holds any of them was not
measured.

### (c) Test tooling on this machine

- `node --version`: `v22.23.3`; `npm --version`: `12.2.0`; `/usr/bin/npx` present.
- `~/.cache/ms-playwright` holds `chromium-1228`, `chromium-1254`,
  `chromium_headless_shell-1228`, `chromium_headless_shell-1254`, `ffmpeg-1011`, `ffmpeg-1013`.
  The SDC inspector pins `@playwright/test` `1.65.0`; that version's `browsers.json` names
  `chromium` and `chromium-headless-shell` revision `1254`, both present.
- A directory with only `package.json` pinning `@playwright/test` `1.65.0`, in the scratch
  directory: `npm install --offline` exits 0 (`added 3 packages`); after removing
  `node_modules`, `npm ci --offline` exits 0 (`added 3 packages in 318ms`). The user's npm cache
  holds the pin; no network was used.
- No WebDriver: `chromedriver` and `geckodriver` are not on the path. `firefox --version`:
  `Mozilla Firefox 159.0`. Playwright 1.65.0 drives Firefox only with its own build, which is not
  in the cache; launching the installed one, `firefox.launch({executablePath:
  '/usr/bin/firefox'})`, answers `browserType.launch: Failed to launch the browser process.`
- A task file with a task whose commands are `npm ci --offline --no-audit --no-fund` and
  `npx playwright test --workers=2`, run as `slot task --taskfile <file> view-test`: exit 0,
  `6 passed (3.2s)`, 4.4 s in all. A command under `slot` already runs inside a job of the pool:
  `slot run env` prints `SLOT_JOB=12161` and the pool's jobserver in `MAKEFLAGS`, so the task's
  commands need no `slot` of their own.
- A nested `slot`: the same task with `slot npx playwright test --workers=2` also passed, but
  `slot run sh -c 'slot run env'` prints `SLOT_JOB=12162`: the inner `slot` is a second job,
  queued in the pool while the first holds its slot. It passed because the pool had room; on a
  full pool a job waits on another job of its own.

### (d) The fragment, the history and the `Referer`

A Playwright test in the scratch directory loads `http://127.0.0.1:47999/#<64 digits>` (answered
by `context.route`, no server), runs the page path of item 6 (token into `sessionStorage`, the
fragment removed with `history.replaceState`, one `fetch` with `Authorization: Bearer`, one
`new EventSource('/events', {headers: {Authorization: 'Bearer x'}})`), reloads, and records every
request's headers, `Page.getNavigationHistory` and the profile's `History` database (read with
`sqlite3`). Six runs: three browsers, each without a `Referrer-Policy` and with `no-referrer`.

| | headless shell 1254 (158.0.8127.18) | Chromium 1254 (158.0.8127.18) | Brave 156.0.8078.80 (`/usr/bin/brave`) |
|---|---|---|---|
| `location.href` after `replaceState` and after the reload | `http://127.0.0.1:47999/` | the same | the same |
| session history after the reload | `about:blank`, `http://127.0.0.1:47999/` | the same | the same |
| token in `sessionStorage` after the reload | yes | yes | yes |
| `Referer` of `probe.js`, requested while the fragment was still in the address, and of the `fetch` | `http://127.0.0.1:47999/`, no fragment; none with `no-referrer` | the same | the same |
| `Authorization` on the `fetch` | `Bearer <64 digits>` | the same | the same |
| `Authorization` on the `EventSource` request | none | none | none |
| the profile's `History` database | none written | `http://127.0.0.1:47999/` and `http://127.0.0.1:47999/#<64 digits>` | the same as Chromium |

So: the fragment never reaches a request or a `Referer`; after `replaceState` it is gone from the
address and from the session history; the profile history of a full browser keeps the first
address with its fragment. `EventSource` sends no `Authorization` even when one is passed to it,
which confirms the reason for reading `/events` with `fetch`. Firefox: not measured, see (c).

## Decisions

### 1. What ESS specifies

- **Decision.** The event names and field names the stream carries are those of `spec/` and are
  not redeclared. (a) gives a message for every in-process event, so the AsyncAPI projection is
  the reference for the names: an event's name, and its fields by their names in declaration
  order. It is not the schema of the view's values: in the view every field value is a string,
  the exact text the session process quoted and `surf_answer::unquote` reads (D26), and a byte
  field is `<n> bytes` (D22), where the projection has an integer for `Status` and base64 for
  `Bytes`. `docs/view-stream.md` says so.
- **Rests on.** (a): `ess generate --path spec --kind asyncapi --out .out/view-measure/asyncapi`
  printed `5 artifact(s)`; the five documents hold 12 channels, one per event of `spec/`, with
  every component `in_process`; `body` is `contentEncoding: base64`.
- **Rejected.** `spec/domains/*.yaml` as the reference (the story's fallback): not needed, the
  projection has every event. Writing the view's JSON to the projection's payload schemas: it
  would put the bytes of a body into the log, which D22 forbids.
- **Overrule.** The operator can ask for typed values (a number for `status`); that changes the
  `event` entry of `docs/view-stream.md` and `story:view-surfd-serves`, and needs a type per field
  in `surfd`, which reads no specification today.

### 2. How the stream is held to it

- **Decision.** A task `view-stream-check` in `taskfiles/view-stream-check.yml`, part of `check`
  (written by `story:view-surfd-serves`). It generates the AsyncAPI projection into
  `.out/view-stream-check/`, reads the names under "Event names" in `docs/view-stream.md` (one
  fenced block, one name per line), and fails with one line per name that no
  `x-ess-event: <name>` line of the projection holds. A story that maps a new event adds its
  name there (D27).
- **Rests on.** A trial of exactly this, a shell script in the scratch directory
  (`ess generate … --kind asyncapi`, `awk` for the block, `grep -q -x "    x-ess-event: $name"`):
  over this `docs/view-stream.md`,
  `view-stream-check: 12 event name(s), each a message of the AsyncAPI projection`, exit 0; over
  a copy with `surf.agent.ActionTaken` added,
  `…/view-stream.md: surf.agent.ActionTaken: no message in the AsyncAPI projection`, exit 1.
- **Rejected.** Checking every qualified name anywhere in the document: type names
  (`surf.net.RequestId`) are not events and would need a second list. Checking field names: the
  mapping test of `story:agent-session-events` (its case 17) already holds every field to the
  generated events.
- **Overrule.** The list can move into a file of its own if the operator prefers the document
  free of it.

### 3. The routes

- **Decision.** Specified in `docs/view-stream.md`. No `ess-http/1` binding, and no later story
  writes one: (b) found none usable. `ess generate ui` is not used: both its targets bring a
  framework, a build step or a crate.
- **Rests on.** (b): `ess specify http validate --path .out/view-measure/spec --binding
  .out/view-measure/http-views.yaml` refuses the binding with the components `in_process`
  (`ESS-HTTP-013`, "no `reached_by: network` component owns the domain of view"); with `net` and
  `navigator` on the network it validates, but `ess specify validate` exits 1 with 4
  `ESS-AUTHOR-040` refusals, `task suite` writes no suite (51 scenarios to none), and the
  synthesized scenarios alone fall from 48 to 25. The view's routes are over `surfd`'s session
  list, which is not in `spec/`, and need a token check and a stream the binding does not route.
  `ess generate ui`: not run; the story's reading of `ess generate ui --help` stands.
- **Rejected.** Moving components to the network for the view: it changes every count of the
  gate and still cannot describe `/api/sessions`. AGENTS.md already records the same reach as
  blocked on beyond10x/ess#792.
- **Overrule.** When ESS can bind a read surface of an `in_process` system, with a token and an
  event stream, the routes can move into `spec/`; the measurement of (b) is then repeated.

### 4. Test tooling for the page

- **Decision.** Playwright, `@playwright/test` pinned at `1.65.0` like the SDC inspector,
  Chromium only (its headless shell, revision 1254), in a directory `view-tests/` of its own:
  `package.json`, a committed `package-lock.json`, `playwright.config.mjs`. A task `view-test` in
  `taskfiles/view-test.yml`, part of `check`, runs `npm ci --prefer-offline --no-audit --no-fund`
  and `npx playwright test --workers=2`, with no second `slot`: the root task is run as
  `slot task check`, and its commands are already in a job. Node and Playwright are gate tooling
  like `ess` and `task`; nothing built for `surfd` depends on them. A Rust fixture binary starts
  what a test needs: a page server on `std::net::TcpListener`, `surfd` with a private
  `XDG_RUNTIME_DIR` under `.out/`, the sessions. No Python, no Node server.
- **Rests on.** (c): `npm ci --offline` printed `added 3 packages in 318ms`, exit 0: the pin
  installs from the user's cache without the network; `chromium_headless_shell-1254`, the
  revision 1.65.0 names, is in the cache; `slot task --taskfile <file> view-test` printed
  `6 passed (3.2s)`, exit 0; `slot run sh -c 'slot run env'` printed a second job number: a
  nested `slot` queues a second job. (d) ran on the same tooling.
- **Rejected.** WebDriver: no `chromedriver` or `geckodriver` on the machine, and a client for it
  is more tooling to choose. Firefox: Playwright needs its own Firefox build, which is not in the
  cache and would be a download. Brave in the gate: Playwright drives it (d), but it is the
  operator's browser, updated on its own, not a pinned build; it stays for a manual look. A second
  `slot` inside the task: measured to queue a second job.
- **Overrule.** Another tool, or Firefox beside Chromium (a download of Playwright's Firefox
  build into the user cache).

### 5. Page files

- **Decision.** Compiled into `surfd` with `include_str!` from `crates/view/web/`. The binary is
  then the whole program: there is no install path to find, and no file that another program
  could replace between the test and the run. The cost, a rebuild for each page change, is
  accepted.
- **Rests on.** Not measured: nothing of the page exists to build. `include_str!` is the
  standard library's.
- **Rejected.** Reading the files from a directory at run time: an install path, and a file
  another program of the user could change under `surfd`.
- **Overrule.** A development switch that reads `crates/view/web/` from disk would make page work
  faster; it is not planned.

### 6. Port and token

- **Decision.** 127.0.0.1 and a port the operating system picks (bind to port 0): no fixed port
  on a machine with many users and other local servers. A view token drawn at each start with
  `surf-secret` (`story:agent-secret-primitives`, decision D25): 32 bytes from the operating
  system's random source, written as 64 hexadecimal digits, compared in constant time by
  `surf-secret`'s comparison. `surfd` writes `http://127.0.0.1:<port>/#<token>` to
  `$XDG_RUNTIME_DIR/surf/view-url`, mode 0600, removed at exit, because `surfd` is usually
  started by `surf-agent open` and nobody sees its output. Since 2026-11-22
  (`story:security-review-fixes`) its standard error gets only the path of that file, so that a
  log of a `surfd` started by hand holds no token; `surfd --print-view-url` prints the address
  with its token there for a person who asks for it. The token is in
  the fragment: a browser does not send a fragment in a request or a `Referer`. The page keeps
  it in `sessionStorage`, removes it from the address with `history.replaceState`, and sends it
  as `Authorization: Bearer <token>` on every `/api/` request. It reads `/events` with `fetch`
  and a stream reader (the server-sent-event framing is kept), because `EventSource` sends no
  header. The page's own files (`/` and the files the page loads) are served without the token:
  they hold no data (D18). Accepted and said in `README.md` (`story:view-first-page`'s section):
  the browser may keep the first address, fragment included, in the history of the user's own
  profile until `surfd` exits.
- **Rests on.** (d), `slot npx playwright test --workers=2` in the scratch directory, `6 passed
  (3.4s)`, in three browsers: no fragment in any request or `Referer`; the fragment
  gone from the address and the session history after `replaceState` and a reload; the token
  still in `sessionStorage`; no `Authorization` on an `EventSource` request. And the accepted
  cost, measured: Chromium 158 and Brave 156 keep `http://127.0.0.1:47999/#<64 digits>` in the
  profile's `History` database. The token is valid only until `surfd` exits, and the history is
  the user's own.
- **Rejected.** The token in the query, which lands in the history and in every request line; a
  cookie, because cookies are not separated by port, so any other server on 127.0.0.1 would
  receive it.
- **Overrule.** If the history entry is not acceptable: a page served at a token-free address
  that asks for the token once (pasted from `view-url`), at the cost of a step for the person.

### 7. The kept log

- **Decision.** The view follows the bounds of `story:agent-session-events` and decides none:
  per session at most 10,000 records or 16 MiB, whichever is reached first, the oldest dropped
  first and counted; over all sessions at most 64 MiB; above it the logs of ended sessions go
  first, the longest-ended first, and such a session's detail says that its log was dropped
  (D24). `docs/view-stream.md` gives the session detail `dropped` (the count) and `log_dropped`
  (the whole log), and the stream answers a number older than what is kept with `reset` (D23).
- **Rests on.** Not measured: the log does not exist yet. The numbers are
  `story:agent-session-events`'s.
- **Rejected.** A bound of the view's own: two bounds on one log would disagree.
- **Overrule.** The numbers are the session-events story's to change.

### 8. Server limits

- **Decision.** Request line and headers at most 8 KiB and 50 lines; 5 s to receive a request;
  16 open connections; 8 open streams; per stream a queue of 256 entries or 1 MiB, past which
  that stream is closed and the reader resumes with `Last-Event-ID`; a comment line every 15 s;
  10 s to write one answer or one stream write, past which the connection is closed. Each a field
  of a limits value, so tests shorten them. `docs/view-stream.md` gives the status of each and
  the order of the checks.
- **Rests on.** Not measured: there is no server yet. The comment line every 15 s is the SDC
  inspector's (`setInterval(… ': ping\n\n' …, 15000)` in its `server.mjs`). A stream also passes
  through `surfd`'s queue of 1,024 entries per reader (`story:agent-session-events`); whichever
  fills first closes it, and the reader resumes the same way.
- **Rejected.** Persistent connections for anything but the stream: one page, few requests.
- **Overrule.** Any number.

### 9. Fonts and assets

- **Decision.** System font stacks; no font file and no image ships. The content security policy
  allows neither (`default-src 'none'`, no `font-src` or `img-src`).
- **Rests on.** Not measured. The SDC inspector vendors two font families (`web/fonts/`); the view
  does not take them.
- **Rejected.** Vendored fonts: third-party files in the binary, against the epic's constraint.
- **Overrule.** A font under a licence the operator accepts, at the cost of `font-src 'self'`.

### 10. What the view holds of a value

- **Decision.** In the words of decision D22, and D21: a field of bytes (a response body, a
  request body) is written by the session process as its length, `<n> bytes`, never as the
  bytes; a password's value is its length, as the answer prints it; every other control value is
  kept as the answer prints it, which is what the `changed` panel shows. The token is not kept:
  the answer entry of `open` omits the `token` line, and no route and no stream event holds a
  session's token. Stated as a rule of `docs/view-stream.md` ("The entries").
- **Rests on.** Not measured: decisions D21 and D22. (a) shows what the rule avoids: the
  projection carries a body as base64 of every byte.
- **Rejected.** The body in the view, even cut: `surfd` holds no page (design section 3).
- **Overrule.** D21 and D22 are the coordinating agent's decisions of critic round 2.

## Where this differs from the drafted stories

- `story:view-http-server`: its case 21 lists the five security lines; `docs/view-stream.md` adds
  `Date` on every answer and `WWW-Authenticate: Bearer` on a 401 (RFC 9110), and fixes the order
  of the head's lines. Its source reader yields `(number, event name, data)`; the `session`
  events fit that shape (they carry the newest number) but are live only, never replayed.
- `story:view-surfd-serves`: its string matrix (case 9) includes U+007F and asks for no raw
  control character; the string rule of this story's text stops at U+001F, so
  `docs/view-stream.md` also escapes U+007F to U+009F. Its detail `unreadable` count is in the
  detail shape.
- `story:agent-session-events`: it names the `body` of `surf.net.ResponseReceived` as the byte
  field; `surf.navigation.NavigationCommitted` has a `body` of `Bytes` too (`spec/domains/navigation.yaml`),
  and the examples write it as `<n> bytes` as well.
