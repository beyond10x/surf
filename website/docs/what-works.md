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
- **83 specified scenarios**, each passing twice: once against the specification's own
  interpreter and once against the Rust code.
- **363 tests**, built with warnings treated as errors.

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
  failure of its own, `TooManyRedirects`. The transport does not follow redirects yet, so today a
  redirect status is delivered as the response.
- **A gate for the HTML parser's tests.** The harness's counts are written down per file, tag and
  section, and `task check` fails when a count differs, when a case stops running without a
  reason, or when an expected count is lowered. The parser is still empty, so it passes 5 of
  9,095 cases, and the gate holds exactly that.
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
  example data only; real sessions come with later waves.
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
  web-platform-tests and counts how many a parser passes. The parser is still empty, so today it
  runs 9,095 of them and passes 5, which is one tokenizer test with an empty input.

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
- No redirects: a redirect status is delivered as the response, not followed.
- No page content in an agent's answer: status, media type and address only.
- The agent binaries run on Linux only.
- No standard section is marked complete.
