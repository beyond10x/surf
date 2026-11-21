---
title: What works today
description: What surf does today, and what it does not do yet.
---

# What works today

- **A page load, end to end.** `surf <url>` opens a window and a tab, starts a navigation, fetches
  the address over `http:` and creates a document for an HTML response.
- **Five components in one process**: `shell`, `navigator`, `net`, `document` and `renderer`.
- **Its own HTTP/1.1 client**, with one deadline for the whole exchange (40 seconds at most) and
  hard limits on header and body size.
- **51 specified scenarios**, each passing twice: once against the specification's own
  interpreter and once against the Rust code.
- **201 tests**, built with warnings treated as errors.

New with the first wave of the current plan:

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
