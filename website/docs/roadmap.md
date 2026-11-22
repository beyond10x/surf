---
title: Roadmap
description: What surf can do after which step of the current plan, and what comes after it.
---

# Roadmap

The current plan has 58 stories in 22 steps, called waves. Waves 1 to 10 are delivered.

Release `0.2.0`, which follows wave 4, adds the agent binaries beside `surf`: `surf-agent`,
`surfd` and `surf-session`, for Linux only at first. The
[releases page](https://github.com/beyond10x/surf/releases) lists the releases. After it, work
moves to the script engine, [swell](https://github.com/beyond10x/swell), a project of its own.
Waves 5 to 9 followed, and release `0.3.0` follows wave 9: it adds the live view, redirects and the session log. The waves after them are planned, and they and the
list under "After the plan" have no date.

| after wave | what you can do |
|---|---|
| 4 (delivered) | Drive a session: `surf-agent open`, `go`, `look`, `close`. A daemon keeps the sessions, one process and one token per session. |
| 7 (delivered) | Watch every session live in a local web page: where it is and what it did, action by action. Redirects are followed. |
| 8 (delivered) | Select an action or an event in the live view and read its fields, beside the session's requests. |
| 11 | Read a real page's links, buttons and form fields, from an HTML parser held to the html5lib tests. |
| 18 | Follow links, fill and submit forms, go back and forward. Each answer says what changed. |
| 22 | Record an agent's session, turn it into a test, and replay it against the recorded network. |

The next planned steps are the HTML parser on real pages (an answer shows no page content yet),
first the tree rules for tables, templates and foreign content, and the parser behind the document.
TLS and `https:` are on the list below. None of them has a date.

The agent commands exist today: see [Drive surf from an agent](drive-from-an-agent.md). An
agent's answer looks like this today:

```text
surf 1 session cef9d223-7fdb-4151-85af-6c6d71d4fece open
did go "http://example.org/"
page 200 "text/html" "http://example.org/"
path 2 of 2
  1 open
  2 go "http://example.org/"
```

Once the HTML parser reads real pages, the `page` line gains the page's title, and the answer
gains what you can do on the page.

## After the plan

Wanted, and not planned in detail yet:

- **TLS 1.3**, and `https:` with it. Soon: most of the web is not reachable without it.
- **Screenshots.** That takes CSS, layout, painting and text rendering, all written here.
- **A JavaScript engine**: [swell](https://github.com/beyond10x/swell), an ECMAScript engine
  written from scratch in Rust as a project of its own, which surf will embed.
- **One operating-system process per component.**
- **The Chrome DevTools Protocol**, so existing test tools can drive surf.
