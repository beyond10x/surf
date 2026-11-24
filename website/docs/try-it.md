---
title: Try it
description: Download surf, load one page, and find the agent binaries.
---

# Try it

surf comes as pre-built binaries. `surf` loads one `http:` address and prints what happened; it
shows no page. `https:` loads on `main`, in no release yet ([below](#https)). On Linux,
`surf-agent` drives a browser session one call at a time, for an AI agent or a script
([Drive surf from an agent](drive-from-an-agent.md)).

## Install

Each [release](https://github.com/beyond10x/surf/releases) has an archive per target and a
`SHA256SUMS` file. Pick your target: `x86_64-unknown-linux-musl`,
`aarch64-unknown-linux-musl`, `aarch64-apple-darwin` or `x86_64-apple-darwin`.

- A Linux archive (x86_64 and aarch64, statically linked, any distribution) holds four
  binaries: `surf`, `surf-agent`, `surfd` and `surf-session`. Install the four together, into
  one directory: `surf-agent` starts `surfd` from its own directory, and `surfd` starts
  `surf-session` from its own.
- A macOS archive (Apple silicon and Intel) holds `surf`: the agent binaries read the caller's
  user id from `/proc/self` and need `XDG_RUNTIME_DIR`, and macOS has neither.

On Linux:

```console
$ version=0.6.0 target=x86_64-unknown-linux-musl
$ base=https://github.com/beyond10x/surf/releases/download/$version
$ curl -fsSLO $base/surf-$version-$target.tar.gz -fsSLO $base/SHA256SUMS
$ grep " surf-$version-$target.tar.gz\$" SHA256SUMS | sha256sum --check
$ tar -xzf surf-$version-$target.tar.gz
$ sudo install surf-$version-$target/surf* /usr/local/bin/
```

On macOS, with `target=aarch64-apple-darwin` or `target=x86_64-apple-darwin`, use
`shasum -a 256 --check` in place of `sha256sum --check`, and install the one binary:
`sudo install surf-$version-$target/surf /usr/local/bin/surf`. The macOS binaries are not
signed or notarised: a file downloaded with `curl` runs as it is; one downloaded with a browser
needs `xattr -d com.apple.quarantine surf` first. There is no Windows binary yet.

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

## https

No release has `https:` yet. Built from `main`, `surf` loads an `https:` address over its own
TLS 1.3 client, and authenticates the server against the root list of Mozilla as curl
publishes it, built into the binary. `--ca-file <path>` adds the CA certificates of a PEM file
for that load, and `surf --anchors` prints the list's count, date and sha256:

```text
anchors <n> bundled <yyyy-mm-dd> sha256:<64 hex>
```

A certificate that is not trusted, has expired or names another host, a server limited to
TLS 1.2 and a failed handshake each fail the load with a reason of its own.
surf does not check revocation: a revoked certificate that has not expired loads.

## Working on surf itself

Building from source and running the checks is for people who change surf.
[How it is built](how-it-is-built.md) says what you need and what the checks do.
