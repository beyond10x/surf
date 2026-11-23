<p align="center">
  <img src="docs/logo.svg" width="112" height="112" alt="surf">
</p>

# surf

**A web browser written from scratch, where the specification comes first.**

![spec: ESS 0.97.4, valid](docs/badges/spec.svg)
![conformance: 196 of 196 scenarios](docs/badges/conformance.svg)
![tests: 1769 passed](docs/badges/tests.svg)
![third-party crates shipped: 0](docs/badges/dependencies.svg)
![unsafe code: forbidden](docs/badges/unsafe.svg)
![standard sections marked complete: 0](docs/badges/complete.svg)
![roadmap: 10 of 22 waves delivered](docs/badges/roadmap.svg)

surf is a small browser written in Rust. It does not wrap Chromium, WebKit or Gecko, and nothing
that ships depends on a third-party crate. Every behaviour is written down as a specification
before any code exists, and a test suite generated from that specification runs against the
real code on every change.

It is young. Today it loads a page from the command line and tells you what happened, and on
Linux an agent can open a session and send it to a page. It is growing into two things:

- a browser that an AI agent drives from the command line, where each call answers in a few
  dense lines of text: where you are, what you can do here, what just changed;
- a fast, repeatable end-to-end test runner that replays what such an agent explored, without
  a server.

surf has no JavaScript yet. The script engine it will embed is
[swell](https://beyond10x.github.io/swell/), an ECMAScript (JavaScript) engine written from
scratch in Rust, a project of its own that is developed beside surf. A page that surf loads
runs no script today.

## Install

surf comes as pre-built binaries. A Linux archive (x86_64 and aarch64, statically linked) holds
`surf`, `surf-agent`, `surfd` and `surf-session`; install the four into one directory. A macOS
archive (Apple silicon and Intel) holds `surf`: the agent binaries read the caller's user id from
`/proc/self` and need `XDG_RUNTIME_DIR`, and macOS has neither. Each
[release](https://github.com/beyond10x/surf/releases) has the four archives and a `SHA256SUMS`
file.

```console
$ version=0.5.0 target=x86_64-unknown-linux-musl
$ base=https://github.com/beyond10x/surf/releases/download/$version
$ curl -fsSLO $base/surf-$version-$target.tar.gz -fsSLO $base/SHA256SUMS
$ grep " surf-$version-$target.tar.gz\$" SHA256SUMS | sha256sum --check
$ tar -xzf surf-$version-$target.tar.gz
$ sudo install surf-$version-$target/surf* /usr/local/bin/
```

The other Linux target is `aarch64-unknown-linux-musl`. On macOS, with `aarch64-apple-darwin` or
`x86_64-apple-darwin`, use `shasum -a 256 --check` and install the one binary:
`sudo install surf-$version-$target/surf /usr/local/bin/surf`. The macOS binaries are not signed
or notarised. There is no Windows binary yet.

## A first look

```console
$ surf http://127.0.0.1:41977/index.html
navigation f6eef140-16d8-458c-bdfb-8999c52d7b31 Committed http://127.0.0.1:41977/index.html
request f6eef140-16d8-458c-bdfb-8999c52d7b31 Responded http://127.0.0.1:41977/index.html
document 9ed741bc-a98b-428b-9585-c90c379a8e4d http://127.0.0.1:41977/index.html
```

One line per thing that happened, in a format a script can rely on. The exit status says
whether the page loaded. The full output format and the exit statuses are in
[`docs/surf-command.md`](docs/surf-command.md).

For a parsed HTML page the output also holds its title, its links, buttons and form fields, and
the limit that stopped the parse, if one did:

```text
title <document-id> "<title>"
element <document-id> <position> <kind> <flags> "<label>" "<target or value>"
note limit <document-id> <limit>
```

A title, label, target and value are quoted fields, written like an address with `"` as `\x22`;
`<flags>` is the set flags among `hidden,disabled,checked,required` joined by `,`, or `-`; the
last field is `-` for a row with neither target nor value.

`surf <url> --as markdown` prints the page as Markdown, every line behind `>`.

## Driving surf from an agent

On Linux, `surf-agent` drives a browser session one call at a time, for an AI agent or a
script. `surf-agent open <url>` opens a session, loads the address and prints the session's
identity and a token, once. Export the two as `SURF_SESSION` and `SURF_TOKEN`; then `go <url>`,
`look`, `follow <handle>`, `fill <handle> <value>`, `pick <handle> <option>`, `press <handle>`,
`back`, `forward`, `log` and `close` take the session as their subject. Each call answers in a
few plain lines. `now`
lists every link with a handle such as `l3`, every form field with one such as `f1` and every
button with one such as `b1`; `follow l3` goes there. `?` means visibility is not computed; only the `hidden` attribute is known.

`fill f1 ada` types into a field; for a secret use `fill f2 -` and write the value to standard
input: an argument word is readable by other users. `pick` chooses an option or checks a box.
A password is never printed: `(set)` or `(empty)`. A value of up to 16,000 bytes from standard
input always fits its request. A text field drops the line breaks of its value, as a browser
does; a textarea keeps them. `--` ends the options, for a value or an option that starts with a
dash: `fill f1 -- -5`, `pick f2 -- "-- choose --"`.

`press b1` submits the form of a submit button; the answer shows the page the server answered
with. A button that only a script would act on is refused.

`back` and `forward` move through the pages of the session. A page reached by a form `POST`
cannot be gone back or forward to until surf has a cache.

A page that counts as public cannot send the session to a loopback or private address: `follow`
and `press` from such a page are refused `forbidden-destination` when the address is or resolves
to one, and so are `back` and `forward` to a page that such a page, or a redirect after a public
address, chose. To go to a local address, type it with `go`.

`changed` says what the last action changed: the value of a control, or the new page and its new
texts. `log` lists every action.

A run of `go`:

```console
$ surf-agent go http://example.org/
surf 1 session cef9d223-7fdb-4151-85af-6c6d71d4fece open
did go "http://example.org/"
page 200 "text/html" "http://example.org/" "Example Domain"
path 2 of 2
  1 open
  2 go "http://example.org/"
changed page none -> "http://example.org/"
  + text "# Example Domain"
  + text "This domain is for use in documentation examples without needing permission. Avoid use in operations."
  + text "[Learn more](<https://iana.org/domains/example>) [@l1]"
now 1 of 1
  l1 link ? "Learn more" -> "https://iana.org/domains/example"
note visibility: hidden attribute only
```

What it does not do yet:

- Only `http:` addresses load: no TLS.
- Release `0.2.0` delivers a redirect status as the response. Since release `0.3.0`, a `go`
  follows redirects.
- No JavaScript and no rendering.
- An answer holds the status, the media type, the address and the title. Since release
  `0.4.0`, `--as tree` or `--as markdown` adds the page as text.
- A form is submitted with `GET` or `POST` as `application/x-www-form-urlencoded` only: a
  `multipart/form-data` form is refused (`unsupported-enctype`), and so is a `GET` form whose
  password field holds a value (`password-in-query`). `press` refuses the address a form would
  submit to as `follow` refuses a link's target. The commands are `open`, `go`, `look`,
  `follow`, `fill`, `pick`, `press`, `back`, `forward`, `log` and `close`. Visibility is not
  computed: `now` says `hid` for the `hidden` attribute and `?` for every other element.
- Before scripts, `changed` within one page holds the state of controls only.
- `follow` refuses, before any navigation, a link to a scheme other than `http:`
  (`unsupported-scheme`), a target over 65,536 bytes (`address-too-long`) and, from a page on a
  public address, a target on a loopback or private address or `localhost`
  (`forbidden-destination`). The page stays.
- The agent binaries run on Linux only.

The commands, the token, the exit statuses and the answer, one by one:
[Drive surf from an agent](https://beyond10x.github.io/surf/docs/drive-from-an-agent).
Quoting lowers the risk of page text steering an agent; it does not remove it.

## Watching sessions live

New with the seventh wave, shipped in release `0.3.0`. While `surfd` runs, a page in your browser
shows its sessions as they act. Open a session with `surf-agent open <url>`, then open the
address in `$XDG_RUNTIME_DIR/surf/view-url` in a browser on the same machine:

```console
$ surf-agent open http://example.org/
$ cat "$XDG_RUNTIME_DIR/surf/view-url"
http://127.0.0.1:<port>/#<token>
```

The page lists the sessions, open, closed or died. For the selected session it shows a timeline
of its actions with a playhead: the page line is that of the action at the playhead, "Latest"
follows the newest action, and the arrow keys move the playhead one action. The selection is
kept in the address, so a reload shows the same action.

The word at the top says whether the page follows: `connecting` until its stream of events is
open and what it loaded then is shown, `live` with the time of the last update while it follows,
`reconnecting` from a lost stream until the next one is open and its load shown, and `offline`
after the view refused the token.

Since the eighth wave, shipped in release `0.3.0`, the right column shows the action at the playhead
or, after a click on a mark of the timeline, that event with its fields, and the session's
requests, one row per request. The selected event is kept in the address too. The requests
list and the selection scroll inside themselves.

Since round 2 a panel "Now" under the page line lists what the agent could do at the playhead:
the links of that action's answer with their handles, labels, targets and visibility (`hid`
for a link the `hidden` attribute hides, `?` where visibility is not computed), and the page's
title shows in the session list and on the page line. A `follow` shows its handle on the
timeline (`follow l1`). Labels, targets and titles are shown as text; no target is a link. A
bidirectional control (U+202A to U+202E, U+2066 to U+2069) in a value from a session shows as a
mark that names it (`U+202E`), so it cannot reorder what a person reads.

The view only reads: nothing on the page changes a session. It listens on 127.0.0.1 only. The
token in the address is valid until `surfd` exits, and the browser may keep that address in its
history. A `surfd` you start yourself writes one line on standard error, the path of
`view-url`, and not the token; `surfd --print-view-url` writes the address with its token
there instead.

## What surf is

- A browser built from first principles. URL parsing, HTTP, HTML parsing and everything after
  them are written here, each against its standard.
- A set of components that share no state. Each keeps its own data, accepts commands and
  publishes events. Nothing reaches into another component's memory.
- A headless tool first. Its first interface is a command line, for people and for agents.
- An experiment: how far does a browser get with no runtime dependencies at all?

## What surf is not

- Not a browser for daily use. There is no window, no rendering, no JavaScript and no `https:`
  yet.
- Not a fork of an existing engine, and not a shell around one.
- Not complete, and it says where. A part of a standard is called complete only when all of it
  is specified and the standard's own tests for it pass. Nothing carries that mark yet, which
  is what the grey badge above means.
- Not in a hurry to grow. A feature is added when a planned step needs it, not because a
  standard lists it.

## Principles

1. **Specification first.** No behaviour exists in code that [`spec/`](spec/) does not state.
   A gap is reported, not filled in quietly.
2. **Complete means complete.** Work stays marked incomplete until it passes the standard's own
   tests. Skipped cases are counted and listed by name.
3. **Isolation over speed.** A component is a security boundary. A design that needs two
   components to share memory is refused.
4. **No third-party code in what ships.** The Rust standard library and the operating system
   are allowed. Every outside package of the lock file is on the allow-list
   `taskfiles/lock-allow.txt` (19 of its 49 packages), and the lock check fails on one that is
   not. One exception:
   audited cryptographic primitives for TLS come from ring 0.17.14, reviewed as a dependency in
   the rustls audit of Cure53, 2020 (<https://cure53.de/pentest-report_rustls.pdf>). Only the
   crate `surf-tls` may depend on it, and the checks pin its packages by name, version and
   checksum.
5. **No unsafe code.** Every hand-written crate forbids it. The exceptions are in tests: three
   counting allocators that measure what a fetch holds, what a parse allocates and what the
   TLS code allocates.
6. **Text from the web stays text.** Anything from a page, a server or an address is escaped
   before it is printed, so a page cannot forge a line of surf's output.
7. **A passing test must be able to fail.** Tests are tried against planted defects. Over the
   whole suite (`task conform-slow`), a navigation storage that drops every write fails 50 of
   the 196 scenarios; the gate runs each of the 29 planted defects over the first 3 scenarios
   that notice it.
8. **Small.** A standard is followed for what surf does, not implemented whole.

## What works today

- **A page load, end to end.** `surf <url>` opens a window and a tab, starts a navigation,
  fetches the address over `http:` and creates a document for an HTML response.
- **Six components in one process**: `agent`, `shell`, `navigator`, `net`, `document` and
  `renderer`.
- **Its own HTTP/1.1 client**, with one deadline for the whole exchange (40 seconds at most)
  and hard limits on header and body size.
- **196 specified scenarios**, each passing twice: once against the specification's own
  interpreter and once against the Rust code.
- **1,769 tests** (1,224, and 545 of the TLS crates in a step of their own), built with
  warnings treated as errors, and 59 tests of the live view's page in a browser.

New with round 2, shipped in release `0.5.0`:

- **An agent can act on a page.** `surf-agent follow <handle>` loads a link's target, `fill`
  types into a field, `pick` chooses an option or ticks a box, and `press` presses a button and
  submits its form with `GET` or `POST`. `back` and `forward` move through the session's pages.
  The `page` line shows the page's title.
- **Each answer says what changed.** The `changed` block lists the controls whose value,
  checked state or selection the last action changed, or the new page and its new texts, at
  most ten lines and a count of the rest. `log [<n>]` lists the session's actions.
- **A public page cannot send the session to your network.** `follow` and `press` from a page
  that counts as public, and `back` and `forward` to a page that such a page or a redirect after
  a public address chose, are refused `forbidden-destination` when the address is or resolves to
  a loopback or private one. To load a local address, type it with `go`.
- **The live view shows a page's links.** Its panel "Now" lists the links of the answer at the
  playhead with their handles, labels and targets, and a `follow` shows its handle on the
  timeline.

New with round 1 after the tenth wave of the current plan, shipped in release `0.4.0`:

- **A page as text, for an agent.** A parsed page can be written as the outline of its tree
  (one line per element and text, with `hid` for hidden parts and the handles `@l1`, `@b1`,
  `@f1` of its links, buttons and fields) or as Markdown (headings, paragraphs, links, lists,
  quotes, code, images, form controls with their labels, landmarks and tables). Every piece of
  page text is escaped, so a page can write neither a line of the answer nor a link or a handle
  of surf's; control, invisible and bidirectional characters are shown as bytes. A link stays
  on one line with its handle, and the Markdown grows with the page, not with its nesting. The
  answer format has a `text` block for it, every line behind `>`, at most 200 lines and 32 KiB
  at a time.

Fixed after a security review, shipped in release `0.3.0`:

- **A redirect cannot lead into your network.** When a load starts at a public address, a
  redirect to a loopback, private, link-local, unique-local, shared (100.64.0.0/10) or
  unspecified address, or to an IPv6 address that embeds one (IPv4-mapped, IPv4-compatible,
  NAT64 or 6to4), also one reached through a name, fails with `ForbiddenDestination` before any
  connection to it. The name is resolved once per request, and the address checked is the one
  connected to. Loading a local address on purpose works as before, and so do the redirects of a
  load that starts there.
- **A refused call learns nothing of the session.** A request without the right token, or one
  that cannot be read, gets the refusal alone: the head line says `unknown`, and no address or
  title of the session is in the reply, also on the session's own socket.
- **`surf-agent` checks where it sends the token.** Before it connects, it checks that
  `$XDG_RUNTIME_DIR` and `surf/` in it are directories of yours, not symbolic links in any
  spelling of the path, with no access for group and others, and that `surfd.sock` is a socket
  of yours; otherwise it exits 3, names the path and sends nothing.
- **`surfd` keeps the view's token off its standard error.** Started by hand, it prints where the
  live view's address is kept (`$XDG_RUNTIME_DIR/surf/view-url`), not the address with its
  token; `surfd --print-view-url` prints the address for a person who asks for it.

New with the tenth wave of the current plan, shipped in release `0.4.0`. Nothing a `surf-agent`
command prints has changed:

- **The HTML parser has limits on untrusted input.** Input bytes (16 MiB), depth (512), nodes
  (1,048,576), attributes of one element (1,024), stored bytes (64 MiB) and steps (2^27). A
  parse that meets one stops, keeps what it built and names the limit.
- **Each link and form field has a label**, its accessible name, computed over the tree without
  CSS, cut at 1,024 bytes and within a bound of work per page. Since round 1, `surf` prints it
  on each `element` line.

New with the ninth wave of the current plan, shipped in release `0.3.0`. Nothing a `surf-agent`
command prints has changed:

- **The HTTP client can send a `POST`** with a body and its media type, and after a redirect
  it does what the Fetch Standard says: a 301, 302 or 303 continues as `GET` without the body,
  a 307 or 308 sends the method and the body again. A body over 16 MiB, or a media type with
  parameters, is refused before anything is sent. `go` and `open` send `GET`; since round 2,
  `press` sends a form's `POST`.
- **Misnested formatting tags give the standard's tree.** The HTML parser runs the HTML
  Standard's adoption agency algorithm. All 22 `formatting` tree cases of the test data pass,
  and 1,300 of the 1,809 tree construction cases.
- **A page's links and form fields, read from the tree.** A new crate, `surf-page`, reads a
  document's title and its links, buttons and form fields out of the parser's tree, each with
  its kind, its target or value, and whether it is hidden, disabled, checked or required. Only
  its tests run it: no command runs the parser on a page yet, so an answer still shows no page
  content.

New with the eighth wave of the current plan, shipped in release `0.3.0`:

- **The live view's detail panel.** The right column of the page shows the action at the
  playhead or, after a click on a mark, that event with its fields, and the session's requests,
  one row per request: the address asked for, then status, media type and size, or `failed`
  and the reason, or `pending`. The selected event is kept in the address (see "Watching
  sessions live" above).
- **A request has a method and a body, in the specification.** A navigation and a request carry
  a method (`Get` or `Post`), an optional body and its media type, and the live view's events
  `NavigationStarted` and `RequestStarted` show them. `go` and `open` send `GET` as before. A
  request with any other method was refused before anything was sent; since the ninth wave
  the client sends a `POST` (above), and since round 2 `press` does.
- **The HTML parser builds a document tree** with the HTML Standard's rules outside tables,
  templates and foreign content (SVG and MathML). All 476 `body` tree cases of the test data
  pass, and all 82 encoding cases: a `meta` that names another encoding parses the page again
  with it. The parser had no limits then (they came with the tenth wave), and no command runs it
  on a page, so an answer still shows no page content.

New with the seventh wave of the current plan, shipped in release `0.3.0`:

- **Redirects are followed.** A `go` or an `open` follows up to 20 redirects (301, 302, 303,
  307 and 308 with a `Location`) inside one fetch. The `page` line shows the address the fetch
  ended at, and the `did go` line the address asked for. One more redirect after the twentieth
  fails the load with `TooManyRedirects`. Only `http:` loads: a redirect to `https:` fails with
  `UnsupportedScheme`. A redirect from a public address to a loopback or private one fails with
  `ForbiddenDestination` (above).
- **The live view's page shows real sessions**, with a playhead over each session's actions
  (see "Watching sessions live" above). After a refused token the page stops and reads
  `offline`. It is tested in a browser as a step of `task check`.
- **The tokenizer reads character references**, named (`&amp;`) and numeric (`&#38;`), from
  the HTML Standard's own table of 2,231 names. All 4,739 character reference cases of the test
  data pass. No command runs the tokenizer on a page yet, so an answer still shows no page
  content.

New with the sixth wave of the current plan, shipped in release `0.3.0`. Nothing a `surf-agent`
command prints has changed:

- **What sessions do, served live.** `surfd` serves a read-only stream of what its sessions
  do: the list of sessions, each session's actions, events and answers, and each new one as it
  happens. It listens on 127.0.0.1 only; while it runs, the address and the token to use are in
  `$XDG_RUNTIME_DIR/surf/view-url`. The routes are in
  [`docs/view-stream.md`](docs/view-stream.md). Since the seventh wave a page shows them
  (above).
- **The tokenizer reads text.** Text inside elements such as `<script>` and `<textarea>` is
  tokenized (the RCDATA, RAWTEXT, script data and PLAINTEXT states), and all 328 text cases of
  the test data pass. Character references are read since the seventh wave.
- **A document's elements in the specification.** The specification holds the elements of a
  page an agent can act on (links, buttons and form fields) and parses a document in steps:
  started, one element at a time in tree order, then finished with its title. The scenarios
  hold it. No parser fills it yet, so a real page's document stays unfinished and an answer
  still shows no page content.

New with the fifth wave of the current plan, shipped in release `0.3.0`. Nothing a user of
`surf-agent` sees has changed:

- **What a session did, kept for the live view.** With each answer, a session's process sends
  the daemon what it did: the actions, the events they caused and the answer, without the token.
  `surfd` keeps these in memory only, bounded per session and over all sessions. Since the
  sixth wave `surfd` serves them (above).
- **The HTML parser decides the encoding.** It sniffs a document's encoding as the HTML
  Standard says (a byte order mark, else a `meta` element in the first 1,024 bytes, else
  windows-1252) and decodes UTF-8, UTF-16 and windows-1252. It passed 75 of the 82 encoding
  cases; since the eighth wave it passes all 82.
- **The tokenizer reads markup.** Tags, attributes, comments, DOCTYPEs, processing
  instructions and CDATA sections are tokenized, and all 1,979 markup cases of the test data
  pass. Text inside elements such as `<script>` and `<textarea>` is read since the sixth wave
  and character references since the seventh; the parser builds a tree since the eighth, and
  no command runs it on a page, so an answer still shows no page content.

New with the fourth wave of the current plan, shipped in release `0.2.0`:

- **The agent commands.** On Linux, `surf-agent open`, `go`, `look` and `close` run against a
  real daemon and a real session process, and load real pages over `http:`. Each session has
  its own process and its own token; a call with the wrong token learns nothing but the refusal,
  and after each answer the session holds no page body. An answer holds the status, the media
  type and the address, not yet the page's content, because no page is parsed yet.
- **A response names its address.** In the specification, a response says which address it came
  from after the last redirect: the navigation commits there and the session shows it, while the
  request keeps the address asked for. A redirect that arrives after 20 followed ones is a
  failure of its own, `TooManyRedirects`. Since the seventh wave the transport follows
  redirects (above).
- **A gate for the HTML parser's tests.** The harness's counts are written down per file, tag
  and section, and `task check` fails when a count falls below them, when a case stops running
  without a reason, or when an expected count is lowered. The parser was empty then and passed
  5 of 9,095 cases; after the ninth wave it passes 8,428, and the gate holds that.
- **Four binaries in one release.** A Linux archive holds `surf`, `surf-agent`, `surfd` and
  `surf-session`; a macOS archive holds `surf`. A smoke test runs the unpacked archive before a
  release is cut.

New with the third wave, the parts the agent commands and the live view build on:

- **The daemon, `surfd`.** It keeps the list of sessions (open, closed or died), passes each
  request to the session's own process and the answer back, and exits when it has been idle.
- **The live view's page.** Nine plain files (ten since the eighth wave, eleven since round
  2), with no framework, nothing from another origin and every value shown as text, fitting
  screens from 360 px to
  1,400 px wide. One command from a source checkout, the `prototype` example of the `surf-view` crate, serves it over three
  scripted sessions and prints an address to open in a browser on the same machine. It shows
  example data only; since the seventh wave `surfd` serves the page over real sessions (above).
- **An agent's actions in the model.** A session keeps a log of what the agent did, in order
  (an `open` with an address is two actions, `open` and `go`), and shows the status, media type
  and failure reason of its navigation. The specification and the Rust code hold both, with
  scenarios.

New with the second wave:

- **Agent sessions in the specification.** The `agent` component keeps the sessions an AI agent
  opens: it can open one, send it to an address, and close it. A session follows its
  navigation from started to committed, failed or cancelled, and closing a session closes its
  tab. The Rust code runs it, and the scenarios hold it.
- **The agent command line, `surf-agent`**: its parser follows the command-line document
  [`cli/surf-agent.yaml`](cli/surf-agent.yaml), the session token is read from the environment
  or a file only you can read, and it talks to a daemon over a Unix socket. With the fourth
  wave it drives real sessions.
- **The live view's web server.** It answers `GET` requests only, from a fixed set of routes,
  checks the `Host` header and a view token, and streams events with a bounded queue per
  reader ([`docs/view-stream.md`](docs/view-stream.md)). Its page is new with the third wave.
- **A test harness for the HTML parser.** It holds 9,107 cases from html5lib-tests and
  web-platform-tests and counts how many a parser passes. Today it runs 9,095 of them and
  passes 9,095: 82 encoding cases, 7,046 tokenizer cases and 1,967 tree construction cases.

New with the first wave:

- **A URL parser** following the WHATWG URL Standard. It passes 872 of the 904 cases of the
  cross-browser URL test data. The other 32 need internationalised host names and are listed
  as skipped.
- **The agent answer format**: a renderer and a parser that reads every answer back unchanged,
  tested with 50 hostile values.
- **Secret handling**: random numbers from the operating system, a comparison that does not
  leak where two secrets differ, and the session token.
- **Design notes** for the HTML parser, the scenario runner and the live view.

## Roadmap

The current plan has 58 stories in 22 steps, called waves. Waves 1 to 10 are delivered. The
plan is [`docs/plan/agent-waves.md`](docs/plan/agent-waves.md) and its design is
[`docs/design/agent-interface.md`](docs/design/agent-interface.md).

Release `0.2.0`, which follows wave 4, adds the agent binaries beside `surf`: `surf-agent`,
`surfd` and `surf-session`, for Linux only at first. The releases are listed at
<https://github.com/beyond10x/surf/releases>. After it, work moves to the script engine,
[swell](https://beyond10x.github.io/swell/), a project of its own. Waves 5 to 9 followed, and release `0.3.0` follows wave 9: it adds the live view, redirects and the session log. Wave 10 and round 1, which builds the stories of wave 11 and some of the waves after it, followed, and release `0.4.0` follows round 1: it adds the HTML parser on real pages and the page as text. Round 2 followed, and release `0.5.0` follows it: it adds following links, filling and submitting forms, going back and forward, and what changed. The waves after them are planned and have no date.

| after wave | what you can do |
|---|---|
| 4 (delivered) | Drive a session: `surf-agent open`, `go`, `look`, `close`. A daemon keeps the sessions, one process and one token per session. |
| 7 (delivered) | Watch every session live in a local web page: where it is and what it did, action by action. Redirects are followed. |
| 8 (delivered) | Select an action or an event in the live view and read its fields, beside the session's requests. |
| 11 (delivered, round 1) | Read a real page's links, buttons and form fields, from an HTML parser held to the html5lib tests. |
| 18 (delivered, round 2) | Follow links, fill and submit forms, go back and forward. Each answer says what changed. |
| 22 | Record an agent's session, turn it into a test, and replay it against the recorded network. |

Round 2 delivered following links, filling and submitting forms, going back and forward, the
`changed` block and `log`; release `0.5.0` ships them. TLS and `https:` are on the list below.
None of the next steps has a date.

Next after that, wanted and not planned in detail yet:

- **TLS 1.3**, and `https:` with it. Soon: most of the web is not reachable without it.
- **Screenshots.** That takes CSS, layout, painting and text rendering, all written here.
- **A JavaScript engine**: [swell](https://beyond10x.github.io/swell/), an ECMAScript engine
  written from scratch in Rust as a project of its own, which surf will embed.
- One operating-system process per component.
- The Chrome DevTools Protocol, so existing test tools can drive surf.

## How it is built

The design lives in [`spec/`](spec/) as an [ESS](https://github.com/beyond10x/ess)
specification. `ess` generates the components from it into
[`generated/rust/`](generated/rust/), and generates the scenarios that hold the code to it.
Hand-written crates under [`crates/`](crates/) supply what the generated code leaves open:
talking to the network, keeping each component's data, the command lines and the session process.

| component | what it does |
|---|---|
| `agent` | the sessions an AI agent opens and drives from the command line |
| `shell` | the windows and tabs the user sees and acts on |
| `navigator` | decides what each tab loads and tracks every load to its end |
| `net` | fetches resources; the only component that talks to the network |
| `document` | parses response bodies into documents |
| `renderer` | styles, lays out and paints a document into a frame (specified, not rendering yet) |

To build surf from source you need [`ess`](https://github.com/beyond10x/ess) 0.97.4,
[`task`](https://taskfile.dev) and a Rust toolchain (built with 1.99.1); `task build` builds
the four binaries. `task check` also needs [`cargo-nextest`](https://nexte.st) 0.9.143 or later.
That is for working on surf, not for using it.

One command checks everything:

```console
$ task check
surf v1 — 9 file(s), 37 scenario(s), valid
196 scenario(s) (37 authored), 56 refusal(s), written to .out/suite.json
  196 scenarios: 196 passed, 0 failed, 0 error, 0 unsupported
196 scenario(s) (37 authored), 56 refusal(s), written to .out/suite.json
conform-noticed: conformance/tests/noticed.txt matches its sources
conform-rust: the narrowed suites of 29 planted defects in conformance/target/noticed/6d99491696333b9a70c2b009b42634243f65bbe23190765d981e5e6ab8c09ee1 are current
Summary [  10.969s] 39 tests run: 39 passed, 0 skipped
  196 scenarios: 196 passed, 0 failed, 0 error, 0 unsupported
conformance step: 17.4 s of its budget of 120 s
the interpreter and the Rust system ran 196 scenario(s) each, of suite sha256:dcc7e7c2c534eb6f98ef03cbd07da620efd16fbe16e8d448de697919b0a56bf3
generated/rust matches spec/ (30 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
10 CLI artifact(s), generated in .out/cli
Cargo.lock: 49 package(s), 19 from outside this repository, each on the allow-list
test-layout: 23 test directories, 148 test files, one test binary each
tls-certs: 33 cases, certs/ matches its sources
tls-flights: 6 recordings match SOURCE, the recorder matches DIGEST
tls-flights: self-test: a changed copy of aes128-x25519-ecdsa.bin fails naming it
doc-tests-check: no doc test outside crates/ports
    Finished `dev` profile [unoptimized] target(s) in 13.34s
1224 tests: 1224 passed, 0 failed, 0 ignored (cargo-nextest 1222, doc tests 2)
test step: 45.2 s of its budget of 120 s
545 tests: 545 passed, 0 failed, 0 ignored (cargo-nextest 545)
tls test step: 20.4 s of its budget of 120 s
agent-tasks: 4 tasks, 4 pass, 0 fail, 0 skipped, lane gate, binaries target/debug
sh-echo: Summary [   6.596s] 40 tests run: 40 passed, 104 skipped
view-stream-check: 23 event name(s), each a message of the AsyncAPI projection
  59 passed (1.9m)
html-conformance: 9107 cases, 9095 run, 9095 passed
x509-limbo: 5143 cases, 5089 run, 5060 passed
tls-x509-conformance: 14 s
forge-watch-test: 6 case(s), each decided as expected by taskfiles/forge-watch.sh
planning-only-test: 9 case(s), each decided as expected by taskfiles/planning-only.sh
test-layout-test: 39 case(s), each decided as expected by taskfiles/test-layout.sh
sh-echo-test: 2 case(s), each decided as expected by taskfiles/sh-echo.sh
doc-tests-check-test: 14 case(s), each decided as expected by taskfiles/doc-tests-check.sh
```

It validates the specification, runs the 196 scenarios against the interpreter built into `ess`
and against the Rust code, checks that the generated code still matches the specification and
that every package in the lock file from outside the repository is on its allow-list, then
builds and tests, the TLS crates in a step of their own, checks that every event name of the
live view's stream is an event of the specification, runs the live view's page tests in a
browser, holds the HTML parser's and the X.509 checker's test counts where they are, and last
tests the gate's own scripts. The tests run under cargo-nextest; a test still running after 60
seconds is ended and fails, each test step fails when it takes over 120 seconds, and the block
leaves out the ten slowest tests each step also prints, the agent tasks' rows after their
counts and the rows of the two conformance counts. It is the run of 2026-11-23 at a load
average of 96 to 132 on 20 CPUs. The page tests' line, `59 passed (1.9m)`, has its slowest
test (`an entry over 1 MiB`) at 27.3 s; `59 passed (1.6m)` had it at 16.0 s at a load average
of 111 to 117; `59 passed (3.0m)` took the task 220 s at a load
average of 126 to 127; `53 passed` took 1.1m at a load average of 55 to 65; `47 passed` took
2.5m at a load average of 110 to 140, and the 38 of before took 15.8 s.
The page tests need Node and
Chromium's headless shell; with Node there, install the shell once with `npm --prefix view-tests ci`, then `npx --prefix view-tests playwright
install --only-shell chromium`. Third-party crates are used in two places: the scenario runner
in [`conformance/`](conformance/), with a lock of its own, and `surf-tls`, whose `ring` and the
packages it needs are the 19 outside packages of the root lock, each on the allow-list
`taskfiles/lock-allow.txt`; the lock check fails on one that is not. Nothing that ships depends
on either.

`task open` lists the design questions that are still open.

## Where things are

| path | what it holds |
|---|---|
| [`spec/`](spec/) | the specification |
| [`generated/rust/`](generated/rust/) | the components, generated from `spec/`, never edited by hand |
| [`crates/`](crates/) | the hand-written crates |
| [`conformance/`](conformance/) | the runner that holds the Rust code to the scenarios |
| [`docs/surf-command.md`](docs/surf-command.md) | the `surf` command: output format, exit statuses, limits |
| [`docs/design/`](docs/design/) | accepted designs |
| [`docs/plan/`](docs/plan/) | the plans, their waves and reviews |
| [`AGENTS.md`](AGENTS.md) | the working rules, written for the coding agents that build surf |

The badges at the top are files in this repository ([`docs/badges/`](docs/badges/)), written
from the numbers `task check` prints. They are not fetched from a service.

## Licence

Apache-2.0: see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE). You may use, change and share surf.
If you do, keep the notice and say where it came from: beyond10x, this repository.
