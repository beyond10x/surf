---
title: What works today
description: What surf does today, and what it does not do yet.
---

# What works today

- **A page load, end to end.** `surf <url>` opens a window and a tab, starts a navigation, fetches
  the address over `http:` and creates a document for an HTML response.
- **Six components in one process**: `agent`, `shell`, `navigator`, `net`, `document` and
  `renderer`.
- **Its own HTTP/1.1 client**, with one deadline for the whole exchange (40 seconds at most) and
  hard limits on header and body size.
- **93 specified scenarios**, each passing twice: once against the specification's own
  interpreter and once against the Rust code.
- **486 tests**, built with warnings treated as errors, and 24 tests of the live view's page in a
  browser.

New with the seventh wave of the current plan, not in a release yet:

- **Redirects are followed.** A `go` or an `open` follows up to 20 redirects (301, 302, 303, 307
  and 308 with a `Location`) inside one fetch. The `page` line shows the address the fetch ended
  at, and the `did go` line the address asked for. One more redirect after the twentieth fails
  the load with `TooManyRedirects`. Only `http:` loads: a redirect to `https:` fails with
  `UnsupportedScheme`.
- **The live view's page shows real sessions.** While `surfd` runs, open the address in
  `$XDG_RUNTIME_DIR/surf/view-url` in a browser on the same machine. The page lists the
  sessions, open, closed or died; for the selected session it shows a timeline of its actions
  with a playhead: the page line is that of the action at the playhead, "Latest" follows the
  newest action, and the arrow keys move the playhead one action. The selection is kept in the
  address. The view only reads; after a refused token the page stops and reads `offline`. The
  token in the address is valid until `surfd` exits, and the browser may keep that address in
  its history. The page is tested in a browser as a step of `task check`.
- **The tokenizer reads character references**, named (`&amp;`) and numeric (`&#38;`), from the
  HTML Standard's own table of 2,231 names. All 4,739 character reference cases of the test data
  pass. No command runs the tokenizer on a page yet, so an answer still shows no page content.

New with the sixth wave of the current plan, not in a release yet. Nothing a `surf-agent`
command prints has changed:

- **What sessions do, served live.** `surfd` serves a read-only stream of what its sessions do:
  the list of sessions, each session's actions, events and answers, and each new one as it
  happens. It listens on 127.0.0.1 only; while it runs, the address and the token to use are in
  `$XDG_RUNTIME_DIR/surf/view-url`. The routes are in
  [`docs/view-stream.md`](https://github.com/beyond10x/surf/blob/main/docs/view-stream.md). Since
  the seventh wave a page shows them (above).
- **The tokenizer reads text.** Text inside elements such as `<script>` and `<textarea>` is
  tokenized (the RCDATA, RAWTEXT, script data and PLAINTEXT states), and all 328 text cases of
  the test data pass. Character references are read since the seventh wave.
- **A document's elements in the specification.** The specification holds the elements of a page
  an agent can act on (links, buttons and form fields) and parses a document in steps: started,
  one element at a time in tree order, then finished with its title. The scenarios hold it. No
  parser fills it yet, so a real page's document stays unfinished and an answer still shows no
  page content.

New with the fifth wave of the current plan, not in a release yet. Nothing a user of
`surf-agent` sees has changed:

- **What a session did, kept for the live view.** With each answer, a session's process sends
  the daemon what it did: the actions, the events they caused and the answer, without the token.
  `surfd` keeps these in memory only, bounded per session and over all sessions. Since the sixth
  wave `surfd` serves them (above).
- **The HTML parser decides the encoding.** It sniffs a document's encoding as the HTML Standard
  says (a byte order mark, else a `meta` element in the first 1,024 bytes, else windows-1252) and
  decodes UTF-8, UTF-16 and windows-1252. It passes 75 of the 82 encoding cases.
- **The tokenizer reads markup.** Tags, attributes, comments, DOCTYPEs, processing instructions
  and CDATA sections are tokenized, and all 1,979 markup cases of the test data pass. Text inside
  elements such as `<script>` and `<textarea>` is read since the sixth wave and character
  references since the seventh; the parser builds no tree, so an answer still shows no page
  content.

New with the fourth wave of the current plan, shipped in release `0.2.0` (the
[releases page](https://github.com/beyond10x/surf/releases) lists them):

- **The agent commands.** On Linux, `surf-agent open`, `go`, `look` and `close` run against a
  real daemon and a real session process, and load real pages over `http:`. Each session has its
  own process and its own token; a call with the wrong token learns nothing but the refusal, and
  after each answer the session holds no page body. An answer holds the status, the media type
  and the address, not yet the page's content, because no page is parsed yet. How to use them:
  [Drive surf from an agent](drive-from-an-agent.md).
- **A response names its address.** In the specification, a response says which address it came
  from after the last redirect: the navigation commits there and the session shows it, while the
  request keeps the address asked for. A redirect that arrives after 20 followed ones is a
  failure of its own, `TooManyRedirects`. Since the seventh wave the transport follows redirects
  (above).
- **A gate for the HTML parser's tests.** The harness's counts are written down per file, tag and
  section, and `task check` fails when a count falls below them, when a case stops running
  without a reason, or when an expected count is lowered. The parser was empty then and passed 5
  of 9,095 cases; after the seventh wave it passes 7,121, and the gate holds that.
- **Four binaries in one release.** A Linux archive holds `surf`, `surf-agent`, `surfd` and
  `surf-session`; a macOS archive holds `surf`. A smoke test runs the unpacked archive before a
  release is cut. See [Try it](try-it.md#install).

New with the third wave, the parts the agent commands and the live view build on:

- **The daemon, `surfd`.** It keeps the list of sessions (open, closed or died), passes each
  request to the session's own process and the answer back, and exits when it has been idle.
- **The live view's page.** Nine plain files, with no framework, nothing from another origin and
  every value shown as text, fitting screens from 360 px to 1,400 px wide. One command from a
  source checkout, the `prototype` example of the `surf-view` crate, serves it over three
  scripted sessions and prints an address to open in a browser on the same machine. It shows
  example data only; since the seventh wave `surfd` serves the page over real sessions (above).
- **An agent's actions in the model.** A session keeps a log of what the agent did, in order (an
  `open` with an address is two actions, `open` and `go`), and shows the status, media type and
  failure reason of its navigation. The specification and the Rust code hold both, with
  scenarios.

New with the second wave:

- **Agent sessions in the specification.** The `agent` component keeps the sessions an AI agent
  opens: it can open one, send it to an address, and close it. A session follows its navigation
  from started to committed, failed or cancelled, and closing a session closes its tab. The Rust
  code runs it, and the scenarios hold it.
- **The agent command line, `surf-agent`**: its parser follows the command-line document
  [`cli/surf-agent.yaml`](https://github.com/beyond10x/surf/blob/main/cli/surf-agent.yaml), the
  session token is read from the environment or a file only you can read, and it talks to a
  daemon over a Unix socket. With the fourth wave it drives real sessions.
- **The live view's web server.** It answers `GET` requests only, from a fixed set of routes,
  checks the `Host` header and a view token, and streams events with a bounded queue per reader
  ([`docs/view-stream.md`](https://github.com/beyond10x/surf/blob/main/docs/view-stream.md)). Its
  page is new with the third wave.
- **A test harness for the HTML parser.** It holds 9,107 cases from html5lib-tests and
  web-platform-tests and counts how many a parser passes. Today it runs 9,095 of them and passes
  7,121: 75 encoding cases and 7,046 tokenizer cases. The parser builds no tree yet.

New with the first wave:

- **A URL parser** following the WHATWG URL Standard. It passes 872 of the 904 cases of the
  cross-browser URL test data. The other 32 need internationalised host names and are listed as
  skipped.
- **The agent answer format**: a renderer and a parser that reads every answer back unchanged,
  tested with 50 hostile values. `surf-agent` prints it since the fourth wave.
- **Secret handling**: random numbers from the operating system, a comparison that does not leak
  where two secrets differ, and the session token.

## What it does not do yet

- No window and no rendering. The `renderer` component is specified; it does not render yet.
- No JavaScript.
- No `https:`. Only `http:` addresses load.
- Release `0.2.0` delivers a redirect status as the response; redirects are followed since the
  seventh wave, not in a release yet.
- No page content in an agent's answer: status, media type and address only.
- The agent binaries run on Linux only.
- No standard section is marked complete.
