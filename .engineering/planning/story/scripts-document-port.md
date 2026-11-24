---
format: aep.planning-md/3
id: story:scripts-document-port
kind: story
status: draft
title: A session opened with the scripting option runs a page's inline scripts and shows what they wrote
summary: The session runs each document's scripts on its parsing thread with the per-page limits and records each script's end; the epic's two examples pass, surf <url> runs none; on a swell tag that holds the security review's fixes.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-spec
- depends_on: story:session-process-hardening
- depends_on: story:html-script-hook
- depends_on: story:scripts-dom-mutation
- depends_on: story:scripts-dom-attributes
- depends_on: story:html-document-port
- depends_on: story:agent-follow-links
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: cli/surf-agent.yaml
- confidence: cited
  path: conformance/src/target.rs
- confidence: inferred
  path: crates/agent/src
- confidence: inferred
  path: crates/agent/tests/drift.rs
- confidence: cited
  path: crates/page/src/lib.rs
- confidence: inferred
  path: crates/script/Cargo.toml
- confidence: cited
  path: crates/session
- confidence: cited
  path: crates/surf-cli/tests
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: docs/agent-answer.md
- confidence: inferred
  path: taskfiles/lock-allow.txt
revision: 3
---
## Context

The epic's thirteenth candidate (lines 476-485), its outcome and two examples (lines 56-93) and
its acceptance 5 (lines 579-582). `story:html-document-port` is implemented: `Browser`'s parser
step parses every committed HTML body and keeps the tree per document
(`crates/surf/src/lib.rs:415-443`, `HtmlParser`; `Browser::with_parser` at `:1563` takes another
step). `story:agent-follow-links` is implemented, so `surf-agent` lists a link the document holds.

The tree on `main` of 2026-11-24, where the epic differs: `surf <url>` prints the title quoted,
`title <document-id> "Before"` (`docs/surf-command.md:28`), where the epic (line 65) writes it
unquoted. The session takes the extra anchors of `open --ca-file` as `arg anchors` of the `open`
request (`crates/session/src/lib.rs:275-283`, `crates/session/src/process.rs:17-19`); the
scripting option comes the same way (the note's Q7). The session serves on its main thread
(`crates/session/src/process.rs:144-148`); a scripted document is parsed on the thread of the
note's Q5. `crates/page/src/lib.rs:56`: "`noscript` is not hidden: surf parses with scripting
disabled".

Named conditions, not known today:

- a release tag of swell that holds the fixes of "From the security review of 2026-11-22" (epic
  lines 598-603): parsing and compilation charged to the memory budget before they allocate, and
  property operations charged for their work and storage first. This story starts only when the
  note's Q13, or a later reading by the coordinating agent, names such a tag; it moves the pin to
  it (one line of `crates/script/Cargo.toml`, the lock and `taskfiles/lock-allow.txt`, in one
  change);
- S5 answered a or b and `story:session-process-hardening` merged (under c no session runs a
  page's script, epic line 515-516). Written for S5 = a.

## Domain relations

- `surf.document.Element` to `surf.document.Document`, many to one: inferable,
  `spec/domains/document.yaml`, entity `surf.document.Element`, relation `document` (line 297).
  The link a script adds is an `Element` row of its document, as a parsed one is.
- The record of a script's end and its document: as `story:scripts-spec` writes it, after
  `decision-blocker:script-end-record`.
- `surf.agent.Session` and its scripting option: inferable (inferred from
  `surf.agent.OpenCall.ca_file`, `spec/domains/agent.yaml:102-119`).

## Change

- `crates/session/`: an `open` with the option runs, for each committed HTML document of the
  session, the parse of `surf-script` that runs scripts (`story:scripts-dom-mutation`) on its own
  thread, through `Browser::with_parser`, and issues the command of `story:scripts-spec` for each
  script's end; without the option, `HtmlParser` as today.
- `crates/surf/src/lib.rs`: what the parser step answers gains the script ends (Q7), so the rows
  are written from the tree the scripts left.
- `crates/page/src/lib.rs`: the hidden rule for `noscript` in a scripted document (Q3).
- `crates/agent/src/`, `cli/surf-agent.yaml`, `crates/agent/tests/drift.rs`: the client accepts
  the option and sends it in the `open` request; the `UNMAPPED:` marker and the `BINDING_ONLY`
  entry of `story:scripts-spec` go.
- `docs/agent-answer.md`: what an answer prints of a script's end, if Q7 says it prints one.
- `conformance/src/target.rs`: the target issues what the session issues.
- `crates/surf-cli/tests/`: only a test.
- `crates/script/Cargo.toml`, `Cargo.lock`, `taskfiles/lock-allow.txt`: the tag of the first
  condition.

## Acceptance

`slot task check` exits 0. The folder's bounds hold: the test step ends within 120 s and no test
runs over 60 s; the test servers start in Rust on 127.0.0.1, so no request leaves the host; no
model call; each case starts one session, nothing more per test; no Python; no generator with a committed output is added (one that were would carry a digest of its sources, a fast check in the gate and full regeneration in the lane `slow`); a build or test job in the pool ends within 10 minutes or names its time with `slot --timeout`, at most 30 minutes; a test over 5 s is
named.

1. Example 1: a page `<title>Before</title><script>document.title = "Written by script"</script>`
   served on 127.0.0.1; in a session opened with the option, `surf-agent look` prints `page 200
   "text/html" "<address>" "Written by script"`; in a session without it, the page line ends
   `"Before"`.
2. Example 2: `<ul id=l></ul><script>var a = document.createElement("a"); a.setAttribute("href",
   "/next"); a.textContent = "Next"; document.getElementById("l").appendChild(a);</script>`: in a
   session with the option, the document holds an `Element` row of kind `link`, label `Next`,
   target `/next` (a test of `surf-session`'s core), and `surf-agent` lists that link; without
   the option, it holds none.
3. `surf <url>` on the page of example 1 prints `title <document-id> "Before"` (a test in
   `crates/surf-cli/tests/`); `surf` links no swell (the note's Q14 check passes).
4. A page whose script throws still shows every other element, and its end is recorded as `threw`
   with its message, line and column as Q16 bounds them.
5. `drift`, `task cli` and `task conform-rust` pass; `task open` no longer lists the marker of
   `story:scripts-spec`.
6. `slot task lock-check` names swell by the tag of the first condition.
7. `git diff --stat` names only the files of the Change.

## Out of scope

The hostile pages (`story:scripts-hostile-pages`); scripts in `surf <url>` and `surf run` (S7);
external, module, `async` and `defer` scripts; events and timers.

## Why `epic:scripts-in-a-page`

The epic's outcome itself: a session that runs a page's inline scripts and shows what they wrote.
