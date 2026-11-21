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
- **77 specified scenarios**, each passing twice: once against the specification's own
  interpreter and once against the Rust code.
- **280 tests**, built with warnings treated as errors.

New with the second wave of the current plan. Nothing here lets you do something new with the
`surf` binary yet; these are the parts the next waves build on:

- **Agent sessions in the specification.** The `agent` component keeps the sessions an AI agent
  opens: it can open one, send it to an address, and close it. A session follows its navigation
  from started to committed, failed or cancelled, and closing a session closes its tab. The Rust
  code runs it, and the scenarios hold it.
- **The agent command line, `surf-agent`**: its parser follows the command-line document
  [`cli/surf-agent.yaml`](https://github.com/beyond10x/surf/blob/main/cli/surf-agent.yaml), the
  session token is read from the environment or a file only you can read, and it talks to a
  daemon over a Unix socket. That daemon, `surfd`, comes next, so `surf-agent` cannot open a
  session yet; its tests run against a stand-in.
- **The live view's web server.** It answers `GET` requests only, from a fixed set of routes,
  checks the `Host` header and a view token, and streams events with a bounded queue per reader
  ([`docs/view-stream.md`](https://github.com/beyond10x/surf/blob/main/docs/view-stream.md)). It
  serves no page yet; the page comes next.
- **A test harness for the HTML parser.** It holds 9,107 cases from html5lib-tests and
  web-platform-tests and counts how many a parser passes. The parser is still empty, so today it
  runs 9,095 of them and passes 5, which is one tokenizer test with an empty input.

New with the first wave:

- **A URL parser** following the WHATWG URL Standard. It passes 872 of the 904 cases of the
  cross-browser URL test data. The other 32 need internationalised host names and are listed as
  skipped.
- **The agent answer format**: a renderer and a parser that reads every answer back unchanged,
  tested with 50 hostile values. The command that prints it arrives with wave 4 of the
  [roadmap](roadmap.md).
- **Secret handling**: random numbers from the operating system, a comparison that does not leak
  where two secrets differ, and the session token.

## What it does not do yet

- No window and no rendering. The `renderer` component is specified; it does not render yet.
- No JavaScript.
- No `https:`. Only `http:` addresses load.
- No standard section is marked complete.
