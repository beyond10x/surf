---
title: Try it
description: Build surf from source and load one page.
---

# Try it

surf is built from source. It loads one `http:` address and prints what happened; it shows no
page.

## What you need

- A Rust toolchain with `cargo` (surf is built with 1.99.1).
- [`task`](https://taskfile.dev).
- [`ess`](https://github.com/beyond10x/ess) 0.96.0.

## Build

```console
$ git clone https://github.com/beyond10x/surf
$ cd surf
$ task build
```

`task build` builds the binary into `target/debug/surf`.

## Load a page

`surf <url>` loads one address over plain HTTP:

```console
$ target/debug/surf http://example.org/
```

A run on 2026-11-20 against a local server:

```console
$ target/debug/surf http://127.0.0.1:41977/index.html
navigation f6eef140-16d8-458c-bdfb-8999c52d7b31 Committed http://127.0.0.1:41977/index.html
request f6eef140-16d8-458c-bdfb-8999c52d7b31 Responded http://127.0.0.1:41977/index.html
document 9ed741bc-a98b-428b-9585-c90c379a8e4d http://127.0.0.1:41977/index.html
```

One line per thing that happened, in a format a script can rely on. The exit status is 0 when the
page loaded and 1 when it did not. [The `surf` command](reference/surf-command.md) gives the full
output format, the exit statuses and the limits.

## Run the checks

```console
$ task check
```

[How it is built](how-it-is-built.md) says what it checks.
