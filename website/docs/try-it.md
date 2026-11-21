---
title: Try it
description: Download the surf binary and load one page.
---

# Try it

surf comes as one pre-built binary. It loads one `http:` address and prints what happened; it
shows no page.

## Install

Each [release](https://github.com/beyond10x/surf/releases) has a `surf` binary for Linux
(x86_64 and aarch64, statically linked, any distribution) and macOS (Apple silicon and Intel),
and a `SHA256SUMS` file. Pick your target: `x86_64-unknown-linux-musl`,
`aarch64-unknown-linux-musl`, `aarch64-apple-darwin` or `x86_64-apple-darwin`.

```console
$ version=0.1.0 target=x86_64-unknown-linux-musl
$ base=https://github.com/beyond10x/surf/releases/download/$version
$ curl -fsSLO $base/surf-$version-$target.tar.gz -fsSLO $base/SHA256SUMS
$ grep " surf-$version-$target.tar.gz\$" SHA256SUMS | sha256sum --check
$ tar -xzf surf-$version-$target.tar.gz
$ sudo install surf-$version-$target/surf /usr/local/bin/surf
```

On macOS use `shasum -a 256 --check` in place of `sha256sum --check`. The macOS binaries are
not signed or notarised: a file downloaded with `curl` runs as it is; one downloaded with a
browser needs `xattr -d com.apple.quarantine surf` first. There is no Windows binary yet.

## Load a page

`surf <url>` loads one address over plain HTTP:

```console
$ surf http://example.org/
```

A run on 2026-11-20 against a local server:

```console
$ surf http://127.0.0.1:41977/index.html
navigation f6eef140-16d8-458c-bdfb-8999c52d7b31 Committed http://127.0.0.1:41977/index.html
request f6eef140-16d8-458c-bdfb-8999c52d7b31 Responded http://127.0.0.1:41977/index.html
document 9ed741bc-a98b-428b-9585-c90c379a8e4d http://127.0.0.1:41977/index.html
```

One line per thing that happened, in a format a script can rely on. The exit status is 0 when the
page loaded and 1 when it did not. [The `surf` command](reference/surf-command.md) gives the full
output format, the exit statuses and the limits.

## Working on surf itself

Building from source and running the checks is for people who change surf.
[How it is built](how-it-is-built.md) says what you need and what the checks do.
