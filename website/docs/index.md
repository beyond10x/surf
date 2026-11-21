---
title: What surf is
slug: /
description: surf is a small web browser written from scratch in Rust, where the specification comes first.
---

# What surf is

surf is a small web browser written in Rust. It does not wrap Chromium, WebKit or Gecko, and
nothing that ships depends on a third-party crate. Every behaviour is written down as a
specification before any code exists, and a test suite generated from that specification runs
against the real code on every change.

It is young. Today it loads a page from the command line and tells you what happened. It is
growing into two things:

- a browser that an AI agent drives from the command line, where each call answers in a few
  dense lines of text: where you are, what you can do here, what just changed;
- a fast, repeatable end-to-end test runner that replays what such an agent explored, without a
  server.

:::caution[Not for daily use]

surf has no window, no rendering, no JavaScript and no `https:` yet. It loads one `http:` address
from the command line and prints what happened.

:::

## What surf is

- **A browser built from first principles.** URL parsing, HTTP, HTML parsing and everything after
  them are written in the repository, each against its standard.
- **A set of components that share no state.** Each keeps its own data, accepts commands and
  publishes events. Nothing reaches into another component's memory.
- **A headless tool first.** Its first interface is a command line, for people and for agents.
- **An experiment:** how far does a browser get with no runtime dependencies at all?

## What surf is not

- **Not a browser for daily use.** There is no window, no rendering, no JavaScript and no
  `https:` yet.
- **Not a fork of an existing engine**, and not a shell around one.
- **Not complete, and it says where.** A part of a standard is called complete only when all of
  it is specified and the standard's own tests for it pass. Nothing carries that mark yet.
- **Not in a hurry to grow.** A feature is added when a planned step needs it, not because a
  standard lists it.

## Where to go next

- [Try it](try-it.md): build surf and load a page.
- [Principles](principles.md): the rules surf is built by.
- [What works today](what-works.md) and the [roadmap](roadmap.md).
- [How it is built](how-it-is-built.md): the specification, the components and the gate.
- [The `surf` command](reference/surf-command.md): output format, exit statuses, limits.
