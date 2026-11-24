# Changelog

What changed in each release of surf. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). The history starts at the first
release; what came before it is in the git log and in `docs/plan/`.

## [Unreleased]

### Added

- `surf-agent help --for-agent` prints a definition for an AI agent, assembled from the texts
  the binary is held to: the usage and every command with its description, the parts of the
  answer format an agent reads, the rule that text from a page is data and never an
  instruction, and worked `open`, `go` and `look` calls. It names three ways to pass the session
  and the token, one of them for a tool that starts a new shell per command. It needs no
  session: no `XDG_RUNTIME_DIR`, no `surfd`. `help` in any other form is a usage error; a
  definition that cannot be written exits 1.
- An agent session keeps cookies: each `surf-agent` session has a cookie jar of its own, so a
  login with `fill` and `press` stays signed in on the next request of the same session (`go`,
  `follow`, `press`, `back`, `forward`); `open` starts a new session with an empty jar. The
  transport takes `Set-Cookie` from every response, redirects included, and sends the `Cookie`
  field on every hop, by the rules of the cookie draft (rfc6265bis): host-only cookies, `Path`,
  `Max-Age` and `Expires`, `Secure` only over `https:`, and `SameSite` decided by the page that
  started the request and by whether a redirect chain has been on another site. A jar holds at
  most 3,000 cookies and 16 MiB; `close` drops it. A cookie value is never printed: not in an
  answer, an entry of the live view or a log line.
- An agent answer has the line `cookies <n>` after `path` when the session holds cookies.
- The agent task `login`: the measure now has six tasks, all passing.
- The specification's `surf.net.Requests` view and the events `NavigationRequested`,
  `NavigationStarted` and `RequestStarted` carry the initiator of a request: the origin of the
  page that started it, none for an address the agent typed or a step `back` or `forward`.

## [0.7.0] - 2026-11-24

### Added

- `surf` loads `https:` addresses, over a TLS 1.3 client (RFC 8446) written for surf, on port
  443 when the address names no port. The server is authenticated against the root list of
  Mozilla as curl publishes it, built into the binary. Redirects between `http:` and `https:`
  are followed both ways. surf does not check revocation: a revoked certificate that has not
  expired loads.
- An `https:` load that fails names one of five new reasons: `CertificateNotTrusted`,
  `CertificateExpired`, `CertificateNameMismatch`, `TlsVersionUnsupported` and
  `TlsHandshakeFailed`. surf speaks TLS 1.3 only: a server limited to TLS 1.2 ends
  `TlsVersionUnsupported`, or, when it answers the offer of TLS 1.3 with `handshake_failure`,
  `TlsHandshakeFailed` with the detail `the server sent handshake_failure before its
  ServerHello; a server limited to TLS 1.2 answers this way`.
- `surf --ca-file <path> <url>` adds the CA certificates of a PEM file to the trust anchors of
  that load: at most 32 KiB and 64 certificates, every one a CA. A file it refuses exits 2,
  naming the path and the error, before any connection.
- `surf --anchors [--ca-file <path>]` prints the number of trust anchors read from the bundled
  list, the list's date and its sha256,
  `anchors <n> bundled <yyyy-mm-dd> sha256:<hex>`, and with a file `anchors <m> file <path>`.
- An agent session loads `https:`: `go` and `open` an `https:` address, `follow` and `press` an
  `https:` target.
- `surf-agent open [<url>] --ca-file <path>` gives the session the CA certificates of one PEM
  file as trust anchors beside the bundled ones, for every `https:` load of the session and for
  its life. A file it refuses exits 2 before `surfd` is started, and no session is made.

### Changed

- `follow` and `press` no longer refuse an `https:` target; a target of any scheme other than
  `http:` and `https:` is still refused `unsupported-scheme`.
- The binaries are built with ring 0.17.14, an outside library, for the cryptographic
  primitives of TLS, and the four crates it needs; `NOTICE` names them and the root list.
  Building surf from source needs a C compiler, which ring compiles its C and assembly with.
- On macOS, `surf` now also links `/usr/lib/libiconv.2.dylib`, a library of the system, beside
  `/usr/lib/libSystem.B.dylib`; nothing needs to be installed for it.

## [0.6.0] - 2026-11-23

### Added

- The live view's panel "Now" lists a page's buttons and form fields beside its links, in the
  groups `links`, `buttons` and `fields`: a field with its kind, label, value and the flags
  `disabled`, `checked`, `required` and `readonly`; a button with the form, method and action
  it submits to. A password shows `(empty)`, the length a `fill` typed (`12 bytes`, from the
  `FillRequested` entry), or `(set)` when no `fill` of that page typed it, never its value.
- The live view has a panel `Changed by <action>` below "Now" (`Changed by fill f2`): what the
  action at the playhead changed, from the `changed` block of its answer. On the same page one
  line per control, such as `f1 value "" -> "ada"`, `f3 checked true -> false` or
  `f2 value (empty) -> 12 bytes`; after a load `page "<old>" -> "<new>"`, the new page's
  `+ text` lines and `+ text more <n>`. It shows `nothing changed` for `changed 0` and
  `no changed block` for an answer without one. The value the agent typed into a password is in
  neither the page, the stream nor a session's detail; text a server sends back is page text
  and shows in the `+ text` lines.
- The view's `answer` entry gains the member `changed`, `{"kind":"controls","lines":[…]}` or
  `{"kind":"page","from","to","texts","more"}`, and each element of its `now` member the member
  `state`: a control's value, its flags and what a button submits, `null` for a link. Format:
  [`docs/view-stream.md`](docs/view-stream.md).

### Changed

- In the view's `now` member, a control or a button has its `label` and `label_cut`, which were
  `null`; its `text` is still the line as the answer writes it. No member is removed or
  renamed.

## [0.5.0] - 2026-11-23

### Added

- `surf-agent follow <handle>` follows a link of the current page. An answer without `--as`
  lists the page's links in a `now` block, one line each, such as `l1 link ? "Opening hours" ->
  "/about/hours.html"`: the handle, `hid` when the `hidden` attribute hides the link and `?`
  otherwise (visibility is not computed, as `note visibility: hidden attribute only` says), the
  label cut at 60 characters, and the target resolved against the page's base URL, as a path on
  the page's own origin, or as written with `unresolved`. `follow` of a handle the page has not,
  of a hidden link or of an unresolved target exits 1 with `did refused unknown-handle`,
  `hidden` or `unresolved`. A followed link's `path` line is `3 follow l1 "Opening hours"`.
  Before any navigation, and so with the page left as it was, `follow` also exits 1 with `did
  refused address-too-long` for a target over 65,536 bytes, `unsupported-scheme` for a scheme
  other than `http:` (`javascript:`, `mailto:`, `data:`, `file:`, `https:`), and
  `forbidden-destination` from a page that counts as public (below). A target is never shown with
  its `user:password@`, and a path on the page's own origin that starts with `//` is shown as
  the whole address. Format: [`docs/agent-answer.md`](docs/agent-answer.md).
- `surf-agent fill <handle> <value>` types into a text field, a password field or a textarea,
  and `fill <handle> -` reads the value from standard input, for a secret: other users can read
  a command's words. `surf-agent pick <handle> <option>` selects an option of a select by its
  label, then its value, and `pick <handle> on|off` checks or unchecks a box, `on` a radio button
  (the others of its group are unchecked). The `now` block lists the form fields beside the
  links, such as `f1 text ? "User name" = "ada" required`: the value quoted, `(set)` or
  `(empty)` for a password, whose value is never printed, the selected option's label for a
  select, and the flags `disabled`, `checked`, `required` and `readonly`. The answer is `did fill
  "f1"` and the path line `4 fill f1`, never the value. `fill` and `pick` exit 1 with `did
  refused` and `unknown-handle`, `wrong-kind`, `disabled`, `hidden`, `readonly` or
  `unknown-option`; `follow` of a form field is `wrong-kind`.
- `surf-agent press <handle>` submits the form of a submit button with `GET` or `POST`, and the
  answer shows the page the server answered with: `did press "b1"`, the path line `5 press b1`.
  The line of a button with a form says what it submits, such as `b1 button ? "Sign in" submits
  F1 post "/session"`: the form's handle, the method and the action, shown as a link's target
  is; the button's `formmethod` and `formaction` count. Before anything is sent `press` exits 1
  with `did refused no-default-action` for a button that is not a submit button,
  `password-in-query` for a `GET` form whose password field holds a value, and
  `address-too-long`, `unsupported-scheme` or `forbidden-destination` for the address the form
  would be submitted to; a submission that ends without a request exits 1 with
  `no-form-owner`, `invalid-controls`, `invalid-action`, `unsupported-enctype` or
  `dialog-method`.
- `fill` of a text or password field drops the line feeds and carriage returns of the value, as
  a browser sanitizes a single-line field; a textarea keeps them. `pick` of a radio button with
  no form unchecks the other radio buttons of the page with its name and no form. `surf-agent`
  takes `--` as the end of options (`fill f1 -- -5`, `pick f2 -- "-- choose --"`; `fill f1 --
  -` fills one dash), and a value from standard input too long for its request is refused with
  its size on the wire; a value of up to 16,000 bytes always fits.
- The `surf` library takes a submit button's `formaction` and `formmethod`: an empty
  `formaction` submits to the page's own address, and a `formmethod` of an unknown value is
  `GET`.
- Of two radio buttons of one group that a page marks `checked`, only the last is checked, as
  the HTML Standard parses it, in the `now` block and in a submission.
- The `page` line of an agent answer ends with the page's title, and the live view's session
  list shows it. A title longer than 1,024 characters is left out.
- The `surf` library reads a page's forms: each form, each control's `name` and form, and the
  options of each select with the one selected, as rows of the session log and the live view
  (`FormAdded`, `OptionAdded`). It builds a form's submission the way the HTML Standard does
  for `application/x-www-form-urlencoded`: a `GET` replaces the query of the form's action, a
  `POST` sends the body. A submission that cannot be sent ends with its reason, such as a
  required field or a required select left empty, a `multipart/form-data` form, or an address
  over 65,536 bytes (`AddressTooLong`), which is not requested. `surf-agent press` uses it.
- `surf-agent back` and `surf-agent forward` move through the pages of the session, one step,
  and load the page again with `GET`; the answer is `did back` or `did forward` with that page,
  and the path line `6 back`. Before any request they exit 1 with `did refused at-start` on the
  first page, `at-end` on the last, and `post-entry` for a page that is the answer to a form
  `POST`, which surf does not send again. A `go`, `follow` or `press` after `back` that loads a
  page drops the pages after the current one.
- Every answer that shows the session's state has a `changed` block after `path`: what the
  last action changed. On the same page, `changed <n>` and one line per control whose state
  changed, such as `~ f1 value "" -> "ada"`, `~ f2 value (empty) -> (set)`, `~ f3 checked false
  -> true` or `~ f5 selected "Red" -> "Green"` (a select lists all its selected options);
  `changed 0` when nothing changed. After a load, `changed page "<old>" -> "<new>"` (`none`
  before the first page) and up to 10 `+ text` lines with the texts of the new page that the
  old one did not hold, each cut at 120 characters, then `+ text more <n>`.
- `surf-agent log [<n>]` lists every action of the session, or the last `<n>`, in the `path`
  block, without taking an action. Format: [`docs/agent-answer.md`](docs/agent-answer.md).
- A page that counts as public cannot send the session to a loopback or private address:
  `follow` and `press` from such a page, and `back` and `forward` to a page that such a page or
  a redirect after a public address chose, exit 1 with `did refused forbidden-destination` when
  the address is or resolves to one, before any connection to it. To load a local address, type
  it with `go`.
- The live view has a panel "Now": the links of the answer at the playhead, grouped by kind,
  each with its handle, label, target and visibility word, a hidden one marked, and the
  visibility note. A `follow` shows its handle on the timeline. A bidirectional control in a
  label, a target, a title or an address shows as a mark that names it (`U+202E`), so a page
  cannot reorder what a person reads in the view. The view's `answer` entry
  gains the member `now`, and each action of its `path` the members `handle` and
  `argument_cut`. Format: [`docs/view-stream.md`](docs/view-stream.md).

### Changed

- The specification, and the code generated from it, is checked with ESS 0.97.4 instead of 0.97.3.
- The session log keeps the first 256 rows a page adds, forms, links, controls and options
  together, and counts the rest in one `EventsLeftOut` line, as it did for links and controls.
- `surf <url>` no longer prints a password: the last field of the `element` line of a
  `password` control is `(set)` when it holds a value and `(empty)` when it does not, unquoted.
  Every other kind is printed as before. Output: [`docs/surf-command.md`](docs/surf-command.md).
- The label of a link, button or field leaves out the value of a password input inside it: a
  `<button>Code <input type=password value=…></button>` is named `Code`.
- The session log of `surf-agent` cuts the `label` of a recorded action to its first 1,024 bytes
  and adds `label_bytes`, the whole label's length, when it cut it, as it already did for a
  page's title. It cuts the `handle` of an action's events, which the client supplies, the
  same way, with `handle_bytes`.
- The event `surf.agent.NavigationRequested` in the session log and the live view has two more
  fields, `handle` and `label`, both empty after a `go`. They hold the handle and the label of
  the link a `follow` takes (the label cut as above), and `FillRequested`, `PickRequested` and
  `PressRequested` the `handle` of the control.
- The event `surf.net.ResponseReceived` in the session log and the live view has the field
  `method` after `url`: the method of the request the response answers.
- The live view's page writes `live` only when its stream of events is open and the load that
  the stream's opening started is drawn. Until then it reads `connecting` over the first load,
  and after a lost stream `reconnecting` until the next opening's load is drawn; a keep-alive or
  an event before that draw writes no word, and neither does a load whose stream has closed.

## [0.4.0] - 2026-11-22

### Added

- `surf <url>` parses each `text/html` response with the HTML parser and prints, after the
  `document` line, the page's title, one `element` line for each link, button and form field
  (its position, kind, the flags among `hidden`, `disabled`, `checked` and `required`, its label,
  and its target or value), and `note limit <document-id> <limit>` when a limit stopped the
  parse. Each title, label, target and value is a quoted field, escaped as an address is, with `"`
  written as `\x22`. A real page's document now ends `Parsed`, with its title and its elements.
  The lines: [`README.md`](README.md), "A first look".
- `surf <url> --as tree` and `surf <url> --as markdown` print, after the rows, the page as an
  outline of its tree or as Markdown: `note text: no-css`, a `text` header, and every line of
  the page behind `>`, with no bound on the number of lines, since the parser's limits bound the
  tree. Without an HTML document the last line is `note text: no-document`. The exit status is
  the one without `--as`; `--as` with another form, or given twice, is a usage error (exit
  status 2). Output: [`docs/surf-command.md`](docs/surf-command.md).
- `surf-agent open <url>`, `go <url>` and `look` take `--as tree|markdown` and
  `--from-line <n>`: the answer ends with a `text` block of the current page, at most 200 lines
  and 32,768 bytes, then a `more` line that names the `look --as <form> --from-line <m>` call for
  the rest. The page is not fetched again, and the call records what it records without `--as`.
  `--as` on `open` without an address, and `--from-line` without `--as`, are usage errors (exit
  status 2). An answer to a call without `--as` is unchanged. The lines of the block are the one
  place an answer holds characters outside ASCII: UTF-8 is written as it is, and control,
  invisible, bidirectional and private-use characters as `\x` and two hexadecimal digits per
  byte. Format: [`docs/agent-answer.md`](docs/agent-answer.md), section "Text".
- The two forms of page text, described in
  [`docs/design/page-as-text.md`](docs/design/page-as-text.md): the outline has one line per
  element and per text, with some of the attributes, `hid <n>` for a hidden part, and the
  handles `@l1`, `@b1`, `@f1` of the links, buttons and fields. The Markdown has headings,
  paragraphs, links with their handles, lists, quotes, code, images, form controls with their
  labels, landmarks, and tables as GitHub tables; hidden parts are left out. Both are computed
  from the tree without CSS. Page text is escaped, so a page can write neither a line of the
  answer nor a link or a handle of surf's. A Markdown line holds at most 1,024 bytes, a table row
  excepted, and its prefix at most 128 bytes: lists and quotes nested deeper are written at that
  indentation, and `note text: deep <n>` says so.
- The HTML parser builds the tree in every insertion mode of the HTML Standard snapshot it is
  pinned to: tables, `select`, `template` and foreign content (SVG and MathML), and it runs the
  fragment parsing algorithm. All 9,095 cases of the test data that run pass, of 9,107, among
  them 1,967 tree construction cases. Two parts differ from the standard, and no case of the
  test data reaches them: a `template` with a `for` attribute keeps its content, and a `select`
  copies its selected `option` into its `selectedcontent` once, at the end of the parse.
  [`docs/design/html-parser.md`](docs/design/html-parser.md).
- The HTML parser (`surf-html`) stops at six limits on untrusted input: 16 MiB of input, a depth
  of 512 open elements, 1,048,576 nodes, 1,024 attributes on one element, 64 MiB of stored bytes
  (the strings the tree holds, and 80 bytes for each attribute) and 134,217,728 (2^27) steps,
  the entries tree construction visits in its scans. A parse that reaches one stops, keeps the
  tree built so far and names the limit; a longer input is parsed as its first 16 MiB. The
  specification's `surf.document.ParseLimit` gains `Steps` for the sixth. What each limit counts
  and where it stops: [`docs/design/html-parser.md`](docs/design/html-parser.md), section 7.
- `surf-page` gives each link, button and form field a label, its accessible name by the
  Accessible Name and Description Computation 1.2 and the HTML Accessibility API Mappings,
  computed over the tree without CSS: from `aria-labelledby`, `aria-label`, the field's `label`
  elements, the content, `value`, `alt`, `title` or `placeholder`, in the order the element's
  kind takes them. A label is cut at 1,024 bytes and then marked as cut. The work for all labels
  of one page is bounded (16,777,216 units); past it a name comes from the element's own
  attributes only. The rules, and where surf deviates from the standards:
  [`docs/design/html-parser.md`](docs/design/html-parser.md), section 5.
- The specification holds following a link and filling and submitting a form: the commands
  `surf.agent.Follow`, `Fill`, `Pick` and `Press`, and in `surf.document` forms, options and
  submissions, with the commands that set a field's value, check a checkbox or a radio button
  (which unchecks the others of its group), select an option and submit a form, whose request
  navigates the tab. The scenarios hold them. No command uses them yet: `surf-agent` has `open`,
  `go`, `look` and `close` only, so a handle in the page text cannot be followed, filled or
  pressed, and a parsed page's forms and options are not stored.
- A new crate, `agent-tasks` (`surf-agent-tasks`), measures four tasks on fixture sites served
  on 127.0.0.1, each a question that a scripted agent answers with `surf-agent` alone.
  `task check` prints its counts after the tests: 2 of the 4 pass (`article` and `table`), and
  `link` and `search` wait for the commands that follow a link, fill a field and submit a form
  ([`crates/agent-tasks/status.txt`](crates/agent-tasks/status.txt)).
  `task agent-tasks-archive DIR=<dir>` measures the binaries of an unpacked release archive, and
  the notes of a release carry its line ([`docs/releasing.md`](docs/releasing.md)).
  `task agent-tasks-model MODEL=<name>` puts the tasks to a model, outside the gate. The crate is
  not shipped.
- The design of TLS 1.3 for `https:` addresses is written down in
  [`docs/design/tls.md`](docs/design/tls.md). No command loads an `https:` address yet.

### Changed

- A session's log, and so the live view's stream, holds what parsing a page publishes:
  `surf.document.DocumentParsed` with the title, cut at 1,024 bytes (the field `title_bytes`
  then gives its whole length), and one `surf.document.ElementAdded` per link, button and field,
  at most 256 of them per call. A line `event surf.agent.EventsLeftOut count="<n>"` after an
  action's events counts the entries left out. No label, target or value of an element is in
  the log. [`docs/view-stream.md`](docs/view-stream.md).
- `surf-agent`, `surfd` and `surf-session` pass the UTF-8 of page text between them, which the
  binaries of `0.3.0` refuse: install the four binaries of one release together, as before.
- The specification is checked, and the code generated from it, with ESS 0.97.3 instead of
  0.96.0. Building surf from source needs `ess` 0.97.3 ([`README.md`](README.md), "How it is
  built").
- The integration tests of each crate build as one test binary over `tests/main.rs`, with each
  former test file a module of it. A file's tests are selected by their module:
  `cargo test -p surf-session --test main e2e::` instead of `--test e2e`. `task check` holds
  this layout (`taskfiles/test-layout.sh`).
- The links to swell, the script engine surf will embed, in `README.md` and on the website go to
  swell's documentation site, <https://beyond10x.github.io/swell/>, instead of its repository.

### Fixed

- `task test` no longer fails with exit status 141 after every test passed. Printing its ten
  slowest tests ended the sort early once the list of tests outgrew a pipe's buffer; the list
  now comes from `taskfiles/slowest-tests.sh`.
- `task agent-tasks-model` counts the model's `surf-agent` calls where `sh` is dash (Debian,
  Ubuntu). Its script wrote the program that logs each call with `echo`, which in dash turns
  the program's backslashes into control characters: every task was written `0 calls`, and
  `task check` failed there. `task check` now runs those tests under an `sh` whose `echo` does
  as dash's (`task sh-echo`).

## [0.3.0] - 2026-11-22

### Added

- `surfd` keeps, in memory only, a log of what each session did: each action, the events it
  caused and the answer, without the token. It is bounded (10,000 entries and 16 MiB per
  session, 64 MiB over all sessions) and is what the live view's stream serves (below). No
  `surf-agent` command prints anything new.
- `surfd` serves the live view's stream: the list of its sessions, each session's kept log, and
  server-sent events as the sessions act, in the routes of
  [`docs/view-stream.md`](docs/view-stream.md). It is read-only (`GET` only, no route changes
  anything) and listens on 127.0.0.1 only, on a port the system picks. The address and a token
  drawn at start are in `$XDG_RUNTIME_DIR/surf/view-url` (mode 0600), removed when `surfd`
  exits; the routes of data need the token. No session's token is in the stream. The page at
  that address shows the sessions (below). A `surfd` started from a shell prints the path of
  that file on standard error, not the token; `surfd --print-view-url` prints the address with
  its token there instead.
- The HTML parser (`surf-html`) decides a document's encoding by the HTML Standard's sniffing
  algorithm and decodes UTF-8, UTF-16LE, UTF-16BE and windows-1252, and its tokenizer reads
  tags, attributes, comments, DOCTYPEs, processing instructions and CDATA sections. No command
  uses it.
- The tokenizer also reads text: the RCDATA, RAWTEXT, script data and PLAINTEXT states, the
  content of elements such as `<textarea>` and `<script>`. All 328 text cases of the test data
  pass.
- The tokenizer reads named and numeric character references, in text and in attribute values,
  from the HTML Standard's `entities.json` (2,231 names). All 4,739 character reference cases of
  the test data pass. No command runs the tokenizer on a page yet.
- The specification holds the elements of a document an agent can act on: the entity
  `Element`, the commands `AddElement` and `FinishParsing`, and the view `Elements`. A document
  is now parsed in steps: `ParseDocument` leaves it `Parsing`, and `FinishParsing` sets its
  title and moves it to `Parsed`. No parser fills them yet: a real page's document stays
  `Parsing`.
- Redirects are followed: `surf`, `surf-agent go` and `surf-agent open <url>` follow up to 20
  redirects (301, 302, 303, 307 and 308 with a `Location`) inside one fetch, within the same
  40 s. The `page` line of an answer, and the `navigation` and `document` lines of `surf`, hold
  the address the fetch ended at; the `did go` line and the `request` line hold the address
  asked for. A redirect after the twentieth fails the load with `TooManyRedirects`. A redirect
  to an address that is not `http:` fails with `UnsupportedScheme`.
- The live view's page, at the address in `$XDG_RUNTIME_DIR/surf/view-url`, shows the sessions
  of `surfd` as they act: a timeline of the selected session's actions with a playhead, the page
  line of the action at the playhead, "Latest" to follow the newest action, the arrow keys to
  move one action, and the selection kept in the address's fragment. After the view refuses its
  token it stops, asks nothing again and reads `offline`.
- The HTML parser builds a document's tree with the HTML Standard's insertion modes outside
  tables, templates and foreign content, and a `meta` that names another encoding parses the
  bytes again with it. No command runs the parser on a page yet.
- The live view's page has a detail panel: the right column shows the action at the playhead or,
  after a click on a mark of the timeline, that event with its fields, and the session's
  requests, one row per request (the address asked for, then status, media type and size, or
  `failed` and the reason, or `pending`). The selected event is kept in the address's fragment
  (`&event=<number>`). The requests list and the selection scroll inside themselves.
- The HTML parser handles misnested formatting tags with the HTML Standard's adoption agency
  algorithm. All 22 `formatting` tree cases of the test data pass. No command runs the parser
  on a page yet.
- A new crate, `surf-page`, reads a document's title and its links, buttons and form fields out
  of the parser's tree, each with its kind, its target or value, and whether it is hidden,
  disabled, checked or required. No command uses it yet.
- The fetch library behind `surf` and `surf-agent` (`surf-fetch`) sends a `POST` with a body and
  its media type (up to 16 MiB of body by default), and after a 301, 302 or 303 to a `POST`
  continues as a `GET` without the body, as the Fetch Standard says; a 307 or 308 repeats the
  `POST`. No command sends a `POST` yet: `surf` and `surf-agent go` still send a `GET` with no
  body.

### Changed

- After a call ends with `did refused session-timeout`, `surfd` goes on reading that session's
  answer for up to 10 minutes, and a further call to the session waits for it. A session that
  does not answer in that time is ended and listed `died`.
- `surfd` and `surf-session` talk a changed link: install the four binaries of one release
  together, as before.
- The events `surf.navigation.NavigationStarted` and `surf.net.RequestStarted` in a session's
  log, and so in the live view's stream, carry three more fields: `method`, `body` (its length,
  `<n> bytes`) and `body_type`. `go` and `open` send `GET` without a body as before, so their
  `method` is `Get` and `body` and `body_type` have no value (`null` in the stream).

### Security

- A call refused for its token, and a request the session process cannot read, no longer
  carries the session's address. In `0.2.0` such a refusal on the session process's own socket
  held the address of the session's page, which can hold a secret in its query or fragment;
  `surfd` did not pass it on, so only another process of the same user could read it. Now the
  refusal holds the head line, with the state `unknown`, and the `did` line, nothing else, and
  `surfd` keeps what it knew of the session.
- A redirect cannot send a request into the machine's own network. Redirects are followed since
  this release (above); before this fix, which came before the release, a public site could
  redirect a `go` or a `surf` load to a loopback or private address, and surf connected there.
  When a load starts at a public address, a redirect to a loopback, private, link-local,
  unique-local, shared (100.64.0.0/10) or unspecified address, or to an IPv6 address that
  embeds one (IPv4-mapped, IPv4-compatible, NAT64 `64:ff9b::/96`, 6to4 `2002::/16`), also
  through a name, fails with the new reason
  `ForbiddenDestination` (`page failed ForbiddenDestination`, `fetch-failed … ForbiddenDestination`,
  exit status 1), with no connection to it. The name is resolved once per request and the
  address checked is the one connected to. A load that starts at a local address, and its
  redirects, work as before.
- `surf-agent` no longer sends its token to a socket it has not checked. In `0.2.0` it
  connected to `$XDG_RUNTIME_DIR/surf/surfd.sock` and wrote the request, token included, with no
  check of who owned the directories or the socket. Now every command first checks that
  `$XDG_RUNTIME_DIR` and `surf/` in it are directories of the caller, not symbolic links, with
  no access for group and others, and that the socket is a socket of the caller; otherwise it
  exits 3, names the path and what is wrong, and sends nothing. A link is refused in every
  spelling of the path: a trailing `/`, a doubled `/` and a `.` component are removed before the
  check, and a `..` component or a relative path is refused. The peer of the connection is not
  checked: the standard library offers no stable way to read its credentials.
- `surfd` started by hand does not write the live view's token on standard error. The live view
  is new in this release (above); before this fix, which came before the release, `surfd`
  printed the address with its token, so a log of it gave every session's view to whoever read
  the log. It prints the path of `$XDG_RUNTIME_DIR/surf/view-url`, which holds the address
  (mode 0600); `surfd --print-view-url` prints the address with its token for a person who asks
  for it.

## [0.2.0] - 2026-11-21

### Added

- `surf-agent open [<url>]`, `go <url>`, `look` and `close` drive a browser session from the
  command line, one call at a time, for an AI agent or a script. `open` prints the session's
  identity and its token once; later calls name them in `SURF_SESSION` and `SURF_TOKEN`. Each
  session runs in a process of its own, `surf-session`, under the daemon `surfd`, which
  `surf-agent open` starts. Every call answers in the line format of
  [`docs/agent-answer.md`](docs/agent-answer.md). How to use it:
  [Drive surf from an agent](https://beyond10x.github.io/surf/docs/drive-from-an-agent).
- The Linux archives hold `surf`, `surf-agent`, `surfd` and `surf-session`. Install the four
  into one directory: `surf-agent` starts `surfd` from its own directory, and `surfd` starts
  `surf-session` from its own.
- The agent binaries are for Linux only: they read the caller's user id from `/proc/self` and
  need `XDG_RUNTIME_DIR`, and macOS has neither. The macOS archives hold `surf`, as before.
- What the agent commands do not do yet: only `http:` addresses load (no `https:`); a redirect is
  not followed, so after a redirect status the response is delivered as it is, with the status
  and the address asked for; there is no JavaScript and no rendering; an answer shows the
  status, the media type and the address of the page and nothing of its content, because no page
  is parsed yet (no title, no elements).

### Security

- The session's token is compared in constant time, and the session process keeps it in no
  command line, environment or file. A call with a wrong token, or for another session, gets the
  head line and the refusal only. After each answer the session holds no response body, so a
  page an agent is sent to cannot grow the session's memory by its size.

## [0.1.0] - 2026-11-21

The first release.

### Added

- `surf <url>` loads one `http:` address and prints one line per navigation, request, document
  and failed fetch, with exit status 0 when the navigation commits. Output format, exit statuses
  and limits: [`docs/surf-command.md`](docs/surf-command.md).
- Pre-built `surf` binaries for Linux (x86_64 and aarch64, statically linked) and macOS (x86_64
  and aarch64), each archive with `README.md`, `LICENSE` and `NOTICE`, and a `SHA256SUMS`
  file. How a release is cut:
  [`docs/releasing.md`](docs/releasing.md).
