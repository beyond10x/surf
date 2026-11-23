---
title: Drive surf from an agent
description: "surf-agent open, go, look, follow, fill, pick, press and close: a browser session for an AI agent or a script, one call at a time."
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
  caption="An agent session recorded on 2026-11-22 with the binaries of release 0.4.0 against a local copy of this site on 127.0.0.1:43117, with the token replaced. An answer names a page's status, media type and address, still with no title; with --as markdown or --as tree it ends with the page as Markdown or as the outline of its tree, every line behind >. The agent reads the Principles page with look --as markdown (all 89 lines) and look --as tree --from-line 151 (lines 151 to 224 of 224, from the heading of its last section)."
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
  Before it connects, `surf-agent` checks that this directory and `surf/` in it are directories
  of yours, not symbolic links, with no access for group and others, and that
  `surf/surfd.sock` is a socket of yours; otherwise it exits 3, names the path and what is
  wrong, and sends nothing, so the token never reaches a socket someone else put there.

## The commands

| command | what it does |
|---|---|
| `surf-agent open [<url>]` | open a session, load the address if one is given, print the session's identity and token |
| `surf-agent go <url>` | load an address into the session's tab |
| `surf-agent look` | print the session's state |
| `surf-agent follow <handle>` | follow the link of a handle of the `now` block, such as `l3` |
| `surf-agent fill <handle> <value>` | type a value into a text field, password field or textarea, such as `f1`; `-` reads the value from standard input |
| `surf-agent pick <handle> <option>` | select an option of a select by its label or value, or `on` or `off` for a checkbox, `on` for a radio button |
| `surf-agent press <handle>` | submit the form of a submit button, such as `b1`, and load the page the server answers with |
| `surf-agent close` | close the session and print its final state |

`open <url>`, `go` and `look` also take `--as tree` or `--as markdown`: the answer then ends
with the page as the outline of its tree or as Markdown, every line behind `>`, at most 200
lines at a time
([`docs/agent-answer.md`](https://github.com/beyond10x/surf/blob/main/docs/agent-answer.md),
section "Text").

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

## Links

Since `follow` was added (after that run), the `page` line of an HTML page ends with its
title, and an answer without `--as` lists the page's links in a `now` block, each with a handle:

```text
now 3 of 3
  l1 link ? "Opening hours" -> "/about/hours.html"
  l2 link hid "Members" -> "/members"
  l3 link ? "Elsewhere" -> "http://other.example/"
note visibility: hidden attribute only
```

`now` lists every link with a handle such as `l3`; `follow l3` goes there. `?` means
visibility is not computed; only the `hidden` attribute is known, and a link it hides says
`hid`. A target on the page's own origin is written as its path; one that does not resolve is
written as the page wrote it, followed by `unresolved`. Handles are numbered in document order
and start at 1 again on each new page. `follow` of a handle the page has not, of a `hid` link
or of an unresolved target is refused with status 1 (`did refused unknown-handle`, `hidden`,
`unresolved`); a followed link's `path` line names its handle and label,
`3 follow l1 "Opening hours"`. Three more refusals come before any navigation and leave the page
as it was: a target over 65,536 bytes (`address-too-long`), a scheme surf does not fetch, such as
`javascript:`, `mailto:` or `https:` (`unsupported-scheme`), and, from a page loaded from a
public address, a target on a loopback or private address or `localhost`
(`forbidden-destination`). A target never shows a `user:password@` part; `follow` still sends it.

## Form fields

The `now` block lists a page's form fields beside its links, with handles such as `f1` (a
field) and `b1` (a button), and in place of a target the field's state:

```text
now 4 of 4
  f1 text ? "User name" = "ada" required
  f2 password ? "Password" = (set)
  f3 checkbox ? "News" checked
  f4 select ? "Colour" = "Green"
note visibility: hidden attribute only
```

The value stands in quotes; a password shows only `(set)` or `(empty)`; a select shows the
label of its selected option; the flags `disabled`, `checked`, `required` and `readonly` follow
when they hold.

`fill f1 ada` types into a field; for a secret use `fill f2 -` and write the value to standard
input: an argument word is readable by other users. `pick` chooses an option or checks a box.
A password is never printed: `(set)` or `(empty)`. A value of up to 16,000 bytes from standard
input always fits its request. A text field drops the line breaks of its value, as a browser
does; a textarea keeps them. `--` ends the options, for a value or an option that starts with a
dash: `fill f1 -- -5`, `pick f2 -- "-- choose --"`.

```sh
printf '%s\n' "$PASSWORD" | surf-agent fill f2 -
surf-agent pick f4 Red
surf-agent pick f3 off
```

The answer says `did fill "f2"`, and the `path` line `4 fill f2`: the handle, never the value.
`fill` and `pick` are refused with status 1 for a handle the page has not (`unknown-handle`), a
field of another kind (`wrong-kind`, also for `follow` of a field), a disabled field or option
(`disabled`), a hidden one (`hidden`), a read-only field (`readonly`) and an option the select
has not, or a word other than `on` and `off` (`unknown-option`).

`press b1` submits the form of a submit button; the answer shows the page the server answered
with. A button that only a script would act on is refused. The line of a submit button says
what it submits: `b1 button ? "Sign in" submits F1 post "/session"`, the form's handle, the
method and the action, the button's own `formmethod` and `formaction` when it has them.

```sh
surf-agent fill f1 ada
printf '%s\n' "$PASSWORD" | surf-agent fill f2 -
surf-agent press b1
```

The answer says `did press "b1"` with the page the form led to, and the `path` line `5 press
b1`. Before anything is sent, with the page left as it was, `press` is refused with status 1
for a button that is not a submit button (`no-default-action`), a disabled or hidden one
(`disabled`, `hidden`), a `GET` form whose password field holds a value, which the address
would carry (`password-in-query`), and an address `follow` would refuse as a link's target
(`address-too-long`, `unsupported-scheme`, `forbidden-destination`). A submission that starts
and ends without a request is refused with its reason: a button outside any form
(`no-form-owner`), an empty `required` field (`invalid-controls`), an action that is not an
address (`invalid-action`), a `multipart/form-data` or `text/plain` form, which surf does not
encode (`unsupported-enctype`), and `method="dialog"` (`dialog-method`).

A script can take the two values from the answer of `open`:

```console
$ answer=$(surf-agent open http://example.org/)
$ export SURF_SESSION=$(printf '%s\n' "$answer" | sed -n 's/^surf 1 session \(.*\) open$/\1/p')
$ export SURF_TOKEN=$(printf '%s\n' "$answer" | sed -n 's/^token //p')
$ surf-agent look
```

## The answer

Every answer starts with its head line, `surf 1 session <id> <state>`, then the action it is
for (`did`), the page (`page`: `none` before the first load, the status, media type, address and
title of a loaded page, or `failed` with the reason) and the session's last actions (`path`);
without `--as` the page's links and form fields (`now`) follow, with `--as` the `note text:`
lines and the `text` block with the page. Every
value that comes from a page, a server or an address stands in double quotes, with every
character that could start a line or look like a field escaped. The format, line by line, with
its grammar:
[`docs/agent-answer.md`](https://github.com/beyond10x/surf/blob/main/docs/agent-answer.md).

Quoting lowers the risk of page text steering an agent; it does not remove it.

## Exit statuses

| status | meaning |
|---|---|
| 0 | the action was taken |
| 1 | the action was refused: the token does not belong to the session (`did refused token`), the session is closed (`did refused closed`), or `follow` names a handle the page has not (`unknown-handle`), a hidden link (`hidden`), a target that does not resolve (`unresolved`), a target over 65,536 bytes (`address-too-long`), one with a scheme surf does not fetch (`unsupported-scheme`) or, from a public page, one on a local address (`forbidden-destination`), or `fill` or `pick` names a field of another kind (`wrong-kind`), a disabled (`disabled`) or read-only (`readonly`) one, or an option it has not (`unknown-option`), or `press` names a button that is not a submit button (`no-default-action`), a `GET` form with a filled password (`password-in-query`), or a submission that ended without a request (`no-form-owner`, `invalid-controls`, `invalid-action`, `unsupported-enctype`, `dialog-method`) |
| 2 | usage: a word, the session or the token is missing or malformed |
| 3 | `surfd` lists no such session (`did refused no-such-session`), the session process ended (`did refused session-died`) or did not answer in time (`did refused session-timeout`), or `surfd` cannot be reached or started, or the runtime directory or the socket fails `surf-agent`'s check |

A refusal by `surfd` or the session prints an answer too, with the reason on its `did` line. When
`surf-agent` itself cannot go on (a usage error, or no `surfd` to reach or start), it prints no
answer and says why on standard error.

## Limits

- Only `http:` addresses load. No `https:` yet.
- Release `0.2.0` follows no redirect. Since release `0.3.0`, a `go`
  follows up to 20 redirects inside one fetch: the `page` line holds the address it ended at,
  the `did go` line the address asked for, and a redirect after the twentieth fails with
  `TooManyRedirects`. When the first request went to a public address, a redirect to a
  loopback, private, link-local, unique-local, shared or unspecified address, or to an IPv6
  address that embeds one, fails with `ForbiddenDestination`, before any connection to it; a session that starts at a local
  address follows its redirects as before.
- `follow` loads `http:` targets only and refuses the others (`unsupported-scheme`); from a page
  on a public address it refuses a target on a local IP address or `localhost`
  (`forbidden-destination`), and it refuses a target over 65,536 bytes (`address-too-long`).
  A target that is another name is resolved by the fetch and not checked by `follow`.
- No JavaScript, and no rendering: `--as` reads the document's tree without CSS (`note text:
  no-css`), so text a style sheet hides is shown, in source order.
- Linux only.
