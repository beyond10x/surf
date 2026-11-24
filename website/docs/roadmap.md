---
title: Roadmap
description: What surf can do after which step of the current plan, and what comes after it.
---

# Roadmap

The current plan has 58 stories in 22 steps, called waves. Waves 1 to 10 are delivered.

Release `0.2.0`, which follows wave 4, adds the agent binaries beside `surf`: `surf-agent`,
`surfd` and `surf-session`, for Linux only at first. The
[releases page](https://github.com/beyond10x/surf/releases) lists the releases. After it, work
moves to the script engine, [swell](https://beyond10x.github.io/swell/), a project of its own.
Waves 5 to 9 followed, and release `0.3.0` follows wave 9: it adds the live view, redirects and the session log. Wave 10 and round 1, which builds the stories of wave 11 and some of the waves after it, followed, and release `0.4.0` follows round 1: it adds the HTML parser on real pages and the page as text. Round 2 followed, and release `0.5.0` follows it: it adds following links, filling and submitting forms, going back and forward, and what changed. The live view's control states followed, and release `0.6.0` follows them: the live view shows a page's controls and what an action changed. TLS 1.3 followed, and release `0.7.0` follows it: `surf` and an agent session load `https:` addresses. The waves after them are planned, and they and the
list under "After the plan" have no date.

| after wave | what you can do |
|---|---|
| 4 (delivered) | Drive a session: `surf-agent open`, `go`, `look`, `close`. A daemon keeps the sessions, one process and one token per session. |
| 7 (delivered) | Watch every session live in a local web page: where it is and what it did, action by action. Redirects are followed. |
| 8 (delivered) | Select an action or an event in the live view and read its fields, beside the session's requests. |
| 11 (delivered, round 1) | Read a real page's links, buttons and form fields, from an HTML parser held to the html5lib tests. |
| 18 (delivered, round 2) | Follow links, fill and submit forms, go back and forward. Each answer says what changed. |
| 22 | Record an agent's session, turn it into a test, and replay it against the recorded network. |

Round 2 delivered following links, filling and submitting forms, going back and forward, the
`changed` block and `log`; release `0.5.0` ships them. TLS 1.3 and `https:` followed:
`surf` and an agent session's `go`, `open`, `follow` and `press` load `https:`, and a session
takes an extra anchor file with `open --ca-file`; release `0.7.0` ships them. None of the next
steps has a date.

The agent commands exist today: see [Drive surf from an agent](drive-from-an-agent.md). An
agent's answer looks like this today:

```text
surf 1 session cef9d223-7fdb-4151-85af-6c6d71d4fece open
did go "http://example.org/"
page 200 "text/html" "http://example.org/" "Example Domain"
path 2 of 2
  1 open
  2 go "http://example.org/"
changed page none -> "http://example.org/"
  + text "# Example Domain"
  + text "This domain is for use in documentation examples without needing permission. Avoid use in operations."
  + text "[Learn more](<https://iana.org/domains/example>) [@l1]"
now 1 of 1
  l1 link ? "Learn more" -> "https://iana.org/domains/example"
note visibility: hidden attribute only
```

Since release `0.4.0`, `--as tree` or `--as markdown` adds the page as text, with the handles
of its links, buttons and fields. Since round 2 the `page` line shows the page's title.

## After the plan

Wanted, and not planned in detail yet:

- **Screenshots.** That takes CSS, layout, painting and text rendering, all written here.
- **A JavaScript engine**: [swell](https://beyond10x.github.io/swell/), an ECMAScript engine
  written from scratch in Rust as a project of its own, which surf will embed.
- **One operating-system process per component.**
- **The Chrome DevTools Protocol**, so existing test tools can drive surf.
