---
title: Roadmap
description: What surf can do after which step of the current plan, and what comes after it.
---

# Roadmap

The current plan has 58 stories in 22 steps, called waves. Waves 1 to 3 are delivered.

The next release, `0.2.0`, comes after wave 4. It adds the agent binaries beside `surf`:
`surf-agent`, `surfd` and `surf-session`, for Linux only at first. The steps after it, and the
list under "After the plan", are planned and have no date.

| after wave | what you can do |
|---|---|
| 4 | Drive a session: `surf-agent open`, `go`, `look`, `close`. A daemon keeps the sessions, one process and one token per session. |
| 7 | Watch every session live in a local web page: where it is, what it did, what changed. |
| 11 | Read a real page's links, buttons and form fields, from an HTML parser held to the html5lib tests. |
| 18 | Follow links, fill and submit forms, go back and forward. Each answer says what changed. |
| 22 | Record an agent's session, turn it into a test, and replay it against the recorded network. |

This is where the agent command line is heading. An agent's answer looks like this; the format
exists today, the command that prints it arrives with wave 4:

```text
surf 1 session 7f3a open
did go "http://example.org/login"
page 200 "text/html" "http://example.org/login" "Sign in"
```

## After the plan

Wanted, and not planned in detail yet:

- **TLS 1.3**, and `https:` with it. Soon: most of the web is not reachable without it.
- **Screenshots.** That takes CSS, layout, painting and text rendering, all written here.
- **A JavaScript engine**, built from first principles as a project of its own that surf embeds.
- **One operating-system process per component.**
- **The Chrome DevTools Protocol**, so existing test tools can drive surf.
