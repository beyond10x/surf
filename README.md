<p align="center">
  <img src="docs/logo.svg" width="112" height="112" alt="surf">
</p>

# surf

**A web browser written from scratch, where the specification comes first.**

![spec: ESS 0.96.0, valid](docs/badges/spec.svg)
![conformance: 83 of 83 scenarios](docs/badges/conformance.svg)
![tests: 407 passed](docs/badges/tests.svg)
![third-party crates shipped: 0](docs/badges/dependencies.svg)
![unsafe code: forbidden](docs/badges/unsafe.svg)
![standard sections marked complete: 0](docs/badges/complete.svg)
![roadmap: 5 of 22 waves delivered](docs/badges/roadmap.svg)

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

## Install

surf comes as pre-built binaries. A Linux archive (x86_64 and aarch64, statically linked) holds
`surf`, `surf-agent`, `surfd` and `surf-session`; install the four into one directory. A macOS
archive (Apple silicon and Intel) holds `surf`: the agent binaries read the caller's user id from
`/proc/self` and need `XDG_RUNTIME_DIR`, and macOS has neither. Each
[release](https://github.com/beyond10x/surf/releases) has the four archives and a `SHA256SUMS`
file.

```console
$ version=0.2.0 target=x86_64-unknown-linux-musl
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

## Driving surf from an agent

On Linux, `surf-agent` drives a browser session one call at a time, for an AI agent or a
script. `surf-agent open <url>` opens a session, loads the address and prints the session's
identity and a token, once. Export the two as `SURF_SESSION` and `SURF_TOKEN`; then `go <url>`,
`look` and `close` take the session as their subject. Each call answers in a few plain lines. A
run of `go`:

```console
$ surf-agent go http://example.org/
surf 1 session cef9d223-7fdb-4151-85af-6c6d71d4fece open
did go "http://example.org/"
page 200 "text/html" "http://example.org/"
path 2 of 2
  1 open
  2 go "http://example.org/"
```

What it does not do yet:

- Only `http:` addresses load: no TLS.
- Redirects are not followed: a redirect status is delivered as the response.
- No JavaScript and no rendering.
- Nothing of the page's content is shown yet: an answer holds the status, the media type and the
  address.
- The agent binaries run on Linux only.

The commands, the token, the exit statuses and the answer, one by one:
[Drive surf from an agent](https://beyond10x.github.io/surf/docs/drive-from-an-agent).
Quoting lowers the risk of page text steering an agent; it does not remove it.

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
   are allowed. The checks fail if the lock file lists an outside package. One exception is
   planned: audited cryptographic primitives for TLS.
5. **No unsafe code.** Every hand-written crate forbids it.
6. **Text from the web stays text.** Anything from a page, a server or an address is escaped
   before it is printed, so a page cannot forge a line of surf's output.
7. **A passing test must be able to fail.** Tests are tried against planted defects. A storage
   that drops every write fails 33 of the 83 scenarios.
8. **Small.** A standard is followed for what surf does, not implemented whole.

## What works today

- **A page load, end to end.** `surf <url>` opens a window and a tab, starts a navigation,
  fetches the address over `http:` and creates a document for an HTML response.
- **Six components in one process**: `agent`, `shell`, `navigator`, `net`, `document` and
  `renderer`.
- **Its own HTTP/1.1 client**, with one deadline for the whole exchange (40 seconds at most)
  and hard limits on header and body size.
- **83 specified scenarios**, each passing twice: once against the specification's own
  interpreter and once against the Rust code.
- **407 tests**, built with warnings treated as errors.

New with the fifth wave of the current plan, not in a release yet. Nothing a user of
`surf-agent` sees has changed:

- **What a session did, kept for the live view.** With each answer, a session's process sends
  the daemon what it did: the actions, the events they caused and the answer, without the token.
  `surfd` keeps these in memory only, bounded per session and over all sessions. Nothing shows
  them yet; the live view reads them in a later wave.
- **The HTML parser decides the encoding.** It sniffs a document's encoding as the HTML
  Standard says (a byte order mark, else a `meta` element in the first 1,024 bytes, else
  windows-1252) and decodes UTF-8, UTF-16 and windows-1252. It passes 75 of the 82 encoding
  cases.
- **The tokenizer reads markup.** Tags, attributes, comments, DOCTYPEs, processing
  instructions and CDATA sections are tokenized, and all 1,979 markup cases of the test data
  pass. Text inside elements such as `<script>` and `<textarea>` and character references are
  not read yet, and the parser builds no tree, so an answer still shows no page content.

New with the fourth wave of the current plan, shipped in release `0.2.0`:

- **The agent commands.** On Linux, `surf-agent open`, `go`, `look` and `close` run against a
  real daemon and a real session process, and load real pages over `http:`. Each session has
  its own process and its own token; a call with the wrong token learns nothing but the refusal,
  and after each answer the session holds no page body. An answer holds the status, the media
  type and the address, not yet the page's content, because no page is parsed yet.
- **A response names its address.** In the specification, a response says which address it came
  from after the last redirect: the navigation commits there and the session shows it, while the
  request keeps the address asked for. A redirect that arrives after 20 followed ones is a
  failure of its own, `TooManyRedirects`. The transport does not follow redirects yet, so
  today a redirect status is delivered as the response.
- **A gate for the HTML parser's tests.** The harness's counts are written down per file, tag
  and section, and `task check` fails when a count falls below them, when a case stops running
  without a reason, or when an expected count is lowered. The parser was empty then and passed
  5 of 9,095 cases; after the fifth wave it passes 4,164, and the gate holds that.
- **Four binaries in one release.** A Linux archive holds `surf`, `surf-agent`, `surfd` and
  `surf-session`; a macOS archive holds `surf`. A smoke test runs the unpacked archive before a
  release is cut.

New with the third wave, the parts the agent commands and the live view build on:

- **The daemon, `surfd`.** It keeps the list of sessions (open, closed or died), passes each
  request to the session's own process and the answer back, and exits when it has been idle.
- **The live view's page.** Nine plain files, with no framework, nothing from another origin
  and every value shown as text, fitting screens from 360 px to 1,400 px wide. One command from
  a source checkout, the `prototype` example of the `surf-view` crate, serves it over three
  scripted sessions and prints an address to open in a browser on the same machine. It shows
  example data only; real sessions come with later waves.
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
  passes 4,164: 75 encoding cases and 4,089 tokenizer cases. The parser builds no tree yet.

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

The current plan has 58 stories in 22 steps, called waves. Waves 1 to 5 are delivered. The
plan is [`docs/plan/agent-waves.md`](docs/plan/agent-waves.md) and its design is
[`docs/design/agent-interface.md`](docs/design/agent-interface.md).

Release `0.2.0`, which follows wave 4, adds the agent binaries beside `surf`: `surf-agent`,
`surfd` and `surf-session`, for Linux only at first. The releases are listed at
<https://github.com/beyond10x/surf/releases>. After it, work moves to the script engine, a
project of its own. Wave 5 followed; the waves after it are planned and have no date.

| after wave | what you can do |
|---|---|
| 4 (delivered) | Drive a session: `surf-agent open`, `go`, `look`, `close`. A daemon keeps the sessions, one process and one token per session. |
| 7 | Watch every session live in a local web page: where it is, what it did, what changed. Redirects are followed. |
| 11 | Read a real page's links, buttons and form fields, from an HTML parser held to the html5lib tests. |
| 18 | Follow links, fill and submit forms, go back and forward. Each answer says what changed. |
| 22 | Record an agent's session, turn it into a test, and replay it against the recorded network. |

The next planned steps are the live view on real sessions (it shows example data only today),
following redirects, and the HTML parser on real pages (an answer shows no page content yet).
TLS and `https:` are on the list below. None of them has a date.

Next after that, wanted and not planned in detail yet:

- **TLS 1.3**, and `https:` with it. Soon: most of the web is not reachable without it.
- **Screenshots.** That takes CSS, layout, painting and text rendering, all written here.
- **A JavaScript engine**, built from first principles as a project of its own that surf
  embeds.
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

To build surf from source you need [`ess`](https://github.com/beyond10x/ess) 0.96.0,
[`task`](https://taskfile.dev) and a Rust toolchain (built with 1.99.1); `task build` builds
the four binaries. That is for working on surf, not for using it.

One command checks everything:

```console
$ task check
surf v1 — 9 file(s), 9 scenario(s), valid
83 scenario(s) (9 authored), 27 refusal(s), written to .out/suite.json
  83 scenarios: 83 passed, 0 failed, 0 error, 0 unsupported
83 scenario(s) (9 authored), 27 refusal(s), written to .out/suite.json
  83 scenarios: 83 passed, 0 failed, 0 error, 0 unsupported
the interpreter and the Rust system ran 83 scenario(s) each, of suite sha256:4d9ad0c31479f50662514ef890cfa8bf01b2eaee94669f5df38269e13feda726
generated/rust matches spec/ (30 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
Cargo.lock: 21 package(s), none from outside this repository
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.00s
102 test suites: 407 passed, 0 failed, 0 ignored
html-conformance: 9107 cases, 9095 run, 4164 passed
```

It validates the specification, runs the 83 scenarios against the interpreter built into `ess`
and against the Rust code, checks that the generated code still matches the specification and
that the lock file holds no outside package, then builds and tests, and last holds the HTML
parser's test counts where they are. The scenario runner in
[`conformance/`](conformance/) is the one place a third-party crate is used; nothing that
ships depends on it.

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
