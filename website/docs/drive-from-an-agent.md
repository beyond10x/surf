---
title: Drive surf from an agent
description: "surf-agent open, go, look and close: a browser session for an AI agent or a script, one call at a time."
---

import AgentTerminal from '@site/src/components/AgentTerminal';
import agentSession from '@site/data/surf-agent.terminal.json';

# Drive surf from an agent

`surf-agent` drives a browser session one call at a time. An AI agent, or a script, opens a
session, sends it to an address, reads what it holds and closes it. Every call prints a short
answer in plain lines, written to be read by a language model.

<AgentTerminal
  id="agent-session"
  session={agentSession}
  caption="An agent session recorded on 2026-11-22 against a local copy of this site on 127.0.0.1:43117, with the token replaced."
/>

It runs on Linux only: the agent binaries read the caller's user id from `/proc/self` and need
`XDG_RUNTIME_DIR`, and macOS has neither.

## What you need

- The four binaries of a Linux archive, `surf`, `surf-agent`, `surfd` and `surf-session`,
  installed together into one directory ([Try it](try-it.md#install)). `surf-agent` starts the
  daemon `surfd` from its own directory, and `surfd` starts one `surf-session` process per
  session from its own. When one of them is missing, `surf-agent open` exits 3 at once and names
  the program it cannot start.
- `XDG_RUNTIME_DIR` set to a directory only you can use. A desktop or SSH login on a systemd
  system sets it (`/run/user/<uid>`). Elsewhere, set it to a short directory of yours with mode
  0700: the session's socket lives under it, and a socket path may have 107 bytes at most.

## The four commands

| command | what it does |
|---|---|
| `surf-agent open [<url>]` | open a session, load the address if one is given, print the session's identity and token |
| `surf-agent go <url>` | load an address into the session's tab |
| `surf-agent look` | print the session's state |
| `surf-agent close` | close the session and print its final state |

`open` starts `surfd` when it is not running. `surfd` keeps the list of sessions and exits ten
minutes after its last session ended.

## The session and its token

`open` prints the session's identity on its first line and its token on the second:

```text
surf 1 session <id> open
token <64 hexadecimal digits>
```

The session's own process draws the token from the operating system's random source when the
session opens. It is shown once: `surfd` does not keep it, and no command prints it again, so a
session whose token is lost can no longer be driven. Every later call names the session and its
token in two environment variables:

- `SURF_SESSION`: the identity;
- `SURF_TOKEN`: the token.

`--session <id>` and `--token-file <path>` take their place. A token file is a regular file of
yours that group and others cannot read (mode 0600). The token is never an argument word,
because every local user can read a process's arguments.

## A session

With the two values exported, each later call answers like this. A run on 2026-11-21, the
session opened without an address:

```console
$ surf-agent go http://example.org/
surf 1 session cef9d223-7fdb-4151-85af-6c6d71d4fece open
did go "http://example.org/"
page 200 "text/html" "http://example.org/"
path 2 of 2
  1 open
  2 go "http://example.org/"
```

Later in the same session, after a `go` to an unreachable address and one to an `https:`
address:

```console
$ surf-agent close
surf 1 session cef9d223-7fdb-4151-85af-6c6d71d4fece closed
did close
page failed UnsupportedScheme "https://example.org/"
path 5 of 5
  1 open
  2 go "http://example.org/"
  3 go "http://127.0.0.1:1/"
  4 go "https://example.org/"
  5 close
```

A load that fails is still an action taken: that `go` exits 0, and the `page` line says
`failed` and why.

A script can take the two values from the answer of `open`:

```console
$ answer=$(surf-agent open http://example.org/)
$ export SURF_SESSION=$(printf '%s\n' "$answer" | sed -n 's/^surf 1 session \(.*\) open$/\1/p')
$ export SURF_TOKEN=$(printf '%s\n' "$answer" | sed -n 's/^token //p')
$ surf-agent look
```

## The answer

Every answer starts with its head line, `surf 1 session <id> <state>`, then the action it is
for (`did`), the page (`page`: `none` before the first load, the status, media type and address
of a loaded page, or `failed` with the reason) and the session's last actions (`path`). Every
value that comes from a page, a server or an address stands in double quotes, with every
character that could start a line or look like a field escaped. The format, line by line, with
its grammar:
[`docs/agent-answer.md`](https://github.com/beyond10x/surf/blob/main/docs/agent-answer.md).

Quoting lowers the risk of page text steering an agent; it does not remove it.

## Exit statuses

| status | meaning |
|---|---|
| 0 | the action was taken |
| 1 | the action was refused: the token does not belong to the session (`did refused token`), or the session is closed (`did refused closed`) |
| 2 | usage: a word, the session or the token is missing or malformed |
| 3 | `surfd` lists no such session (`did refused no-such-session`), the session process ended (`did refused session-died`) or did not answer in time (`did refused session-timeout`), or `surfd` cannot be reached or started |

A refusal by `surfd` or the session prints an answer too, with the reason on its `did` line. When
`surf-agent` itself cannot go on (a usage error, or no `surfd` to reach or start), it prints no
answer and says why on standard error.

## Limits

- Only `http:` addresses load. No `https:` yet.
- Release `0.2.0` follows no redirect. Since release `0.3.0`, a `go`
  follows up to 20 redirects inside one fetch: the `page` line holds the address it ended at,
  the `did go` line the address asked for, and a redirect after the twentieth fails with
  `TooManyRedirects`.
- No JavaScript, and no rendering: the answer says what loaded, not what a page shows.
- Linux only.
