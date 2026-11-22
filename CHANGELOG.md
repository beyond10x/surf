# Changelog

What changed in each release of surf. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). The history starts at the first
release; what came before it is in the git log and in `docs/plan/`.

## [Unreleased]

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
  that address shows the sessions (below).
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
- A redirect can no longer send a request into the machine's own network. Since redirects are
  followed (above, in no release yet), a public site could redirect a `go` or a `surf` load to a
  loopback or private address, and surf connected there. Now, when a load starts at a public
  address, a redirect to a loopback, private, link-local, unique-local, shared (100.64.0.0/10)
  or unspecified address, or to an IPv6 address that embeds one (IPv4-mapped, IPv4-compatible,
  NAT64 `64:ff9b::/96`, 6to4 `2002::/16`), also through a name, fails with the new reason
  `ForbiddenDestination` (`page failed ForbiddenDestination`, `fetch-failed … ForbiddenDestination`,
  exit status 1), with no connection to it. The name is resolved once per request and the
  address checked is the one connected to. A load that starts at a local address, and its
  redirects, work as before.
- `surf-agent` no longer sends its token to a socket it has not checked. In `0.2.0` it
  connected to `$XDG_RUNTIME_DIR/surf/surfd.sock` and wrote the request, token included, with no
  check of who owned the directories or the socket. Now every command first checks that
  `$XDG_RUNTIME_DIR` and `surf/` in it are directories of the caller, not symbolic links, with
  no access for group and others, and that the socket is a socket of the caller; otherwise it
  exits 3, names the path and what is wrong, and sends nothing. The peer of the connection is
  not checked: the standard library offers no stable way to read its credentials.
- `surfd` started by hand no longer writes the live view's token on standard error. Since the
  live view (above, in no release yet) it printed the address with its token, so a log of it
  gave every session's view to whoever read the log. Now it prints the path of
  `$XDG_RUNTIME_DIR/surf/view-url`, which holds the address (mode 0600); `surfd
  --print-view-url` prints the address with its token for a person who asks for it.

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
