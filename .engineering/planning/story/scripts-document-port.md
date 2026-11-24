---
format: aep.planning-md/3
id: story:scripts-document-port
kind: story
status: draft
title: A session opened with the scripting option runs a page's inline scripts and shows what they wrote
summary: A session opened with the option runs each document's scripts through parse_with_scripts and records each end; the epic's two examples, no state between pages and noscript pass; surf <url> runs none; surf-session joins the closed list; the bounds measured again with scripts; on the swell tag the coordinating agent names from Q13.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-spec
- depends_on: story:session-process-hardening
- depends_on: story:html-script-hook
- depends_on: story:scripts-dom-mutation
- depends_on: story:scripts-dom-attributes
- depends_on: story:html-document-port
- depends_on: story:agent-follow-links
- depends_on: story:scripts-parse-run
- depends_on: story:scripts-html-conformance
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: cli/surf-agent.yaml
- confidence: cited
  path: conformance/src/target.rs
- confidence: cited
  path: conformance/tests/noticed.txt
- confidence: cited
  path: crates/agent/src
- confidence: cited
  path: crates/agent/tests/drift.rs
- confidence: cited
  path: crates/page/src/lib.rs
- confidence: cited
  path: crates/script/Cargo.toml
- confidence: cited
  path: crates/session
- confidence: cited
  path: crates/surf-cli/tests
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: inferred
  path: crates/surfd/src/daemon.rs
- confidence: cited
  path: docs/agent-answer.md
- confidence: cited
  path: docs/design/scripts.md
- confidence: cited
  path: taskfiles/lock-allow.txt
- confidence: cited
  path: website/docs/drive-from-an-agent.md
- confidence: cited
  path: website/docs/index.md
- confidence: cited
  path: website/docs/status.md
- confidence: cited
  path: website/docs/what-works.md
revision: 22
---
## Context

The epic's thirteenth candidate (lines 476-485), its outcome and two examples (lines 56-93) and
its acceptance 5 (lines 579-582). `story:html-document-port` is implemented: `Browser`'s parser
step parses every committed HTML body and keeps the tree per document
(`crates/surf/src/lib.rs:415-443`, `HtmlParser`; `Browser::with_parser` at `:1563` takes another
step). `story:agent-follow-links` is implemented, so `surf-agent` lists a link the document holds.

The tree on `main` of 2026-11-24, where the epic differs: `surf <url>` prints the title quoted,
`title <document-id> "Before"` (`docs/surf-command.md:28`), where the epic (line 65) writes it
unquoted. `surf run` does not exist on `main` (no `run` command in `crates/surf-cli/src`); the
active `story:runner-run` adds it. The session takes the extra anchors of `open --ca-file` as
`arg anchors` of the `open` request (`crates/session/src/lib.rs:275-283`,
`crates/session/src/process.rs:17-19`); the scripting option comes the same way (the note's Q7).
The session serves on its main thread (`crates/session/src/process.rs:144-148`); a scripted
document is parsed on the thread of the note's Q5. `crates/page/src/lib.rs:56`: "`noscript` is
not hidden: surf parses with scripting disabled". `conformance/Cargo.toml` depends on `surf` and
`surf-ports`, and `conformance/Cargo.lock` is checked with `--locked`.

S5 is decided (option a, the operator, 2026-11-24): a session with scripts runs under `setpriv`
and `prlimit` (`story:session-process-hardening`, an edge of this story).

One named condition, not known today: a release tag of swell that holds the fixes of "From the
security review of 2026-11-22" (epic lines 598-603): parsing and compilation charged to the memory
budget before they allocate, and property operations charged for their work and storage first.
`docs/design/scripts.md` Q13 records, from swell's tree at a tag, which of the two each tag holds
and the exceptions it names; the coordinating agent decides on that record. This story starts
only when the coordinating agent has named the tag, and moves the pin to it (one line of
`crates/script/Cargo.toml`, the lock and the allow-list, in one change).

## Domain relations

- `surf.document.Element` to `surf.document.Document`, many to one: inferable,
  `spec/domains/document.yaml`, entity `surf.document.Element`, relation `document` (line 297).
  The link a script adds is an `Element` row of its document, as a parsed one is.
- The record of a script's end and its document: as `story:scripts-spec` writes it, after
  `decision-blocker:script-end-record`.
- `surf.agent.Session` and its scripting option: inferable (inferred from
  `surf.agent.OpenCall.ca_file`, `spec/domains/agent.yaml:102-119`).

## Waits for the note

- waits for the note: Q3, the rule for `noscript` in a scripted document;
- waits for the note: Q7 and Q16, what an answer prints of a script's end and what a recorded
  error holds;
- waits for the note: Q11, the bounds and their starting values, which this story measures again
  with scripts;
- waits for the note: Q13, the tag, as the coordinating agent decides on it.

## Decisions taken

1. `surf-session` gains the dependency `surf-script`; the closed list of crates that may depend
   on `surf-script` gains `surf-session` in this story's change. `crates/surf` gains no dependency
   on `surf-script` (Q14): the session hands its scripted parser step to `Browser::with_parser`,
   and what the parser step answers carries the script ends in `surf`'s own types. So
   `conformance/Cargo.toml` and `conformance/Cargo.lock` do not change.
2. With the option, every committed HTML document of the session is parsed by
   `surf_script::run::parse_with_scripts` (`story:scripts-parse-run`) on the thread of Q5, one
   agent per document, dropped as Q5 says, so no state passes from one page to the next; the
   session issues the command of `story:scripts-spec` for each end. Without the option,
   `HtmlParser` as today.
3. The bounds of Q11 are measured again with scripted sessions on the pages Q11 names (`VmHWM` in
   `/proc/<pid>/status`, the descriptors in `/proc/<pid>/fd`); where a starting value does not
   hold, the note's Q11 and the shipped value in `surfd`'s `Config` change in this story, each
   with its `probe:` line.

## Change

- `crates/session/` (its `Cargo.toml` included): decision 2.
- `crates/surf/src/lib.rs`: what the parser step answers gains the script ends (Q7), so the rows
  are written from the tree the scripts left.
- `crates/page/src/lib.rs`: the hidden rule for `noscript` in a scripted document (Q3).
- `crates/agent/src/`, `cli/surf-agent.yaml`, `crates/agent/tests/drift.rs`: the client accepts
  the option and sends it in the `open` request; the `UNMAPPED:` marker and the `BINDING_ONLY`
  entry of `story:scripts-spec` go.
- `docs/agent-answer.md`: what an answer prints of a script's end, if Q7 says it prints one.
- `conformance/src/target.rs` and `conformance/tests/noticed.txt`: the target issues what the
  session issues; the list gets its new digest from `task conform-slow WRITE=1`.
- `crates/surf-cli/tests/`: only tests.
- `crates/script/Cargo.toml`, `Cargo.lock`, `taskfiles/lock-allow.txt`: the tag of the condition;
  the file of the closed list (Q15's form): `surf-session` added.
- `docs/design/scripts.md` and `crates/surfd/src/daemon.rs`: only as decision 3 finds.
- In the last commit, the public surfaces of acceptance 12.

## Acceptance

1. Example 1: a page `<title>Before</title><script>document.title = "Written by script"</script>`
   served on 127.0.0.1; in a session opened with the option, `surf-agent look` prints `page 200
   "text/html" "<address>" "Written by script"`; in a session without it, the page line ends
   `"Before"`.
2. Example 2: `<ul id=l></ul><script>var a = document.createElement("a"); a.setAttribute("href",
   "/next"); a.textContent = "Next"; document.getElementById("l").appendChild(a);</script>`: in a
   session with the option, the document holds an `Element` row of kind `link`, label `Next`,
   target `/next` (a test of `surf-session`'s core), and `surf-agent` lists that link; without
   the option, it holds none.
3. `no_state_between_pages`: in one session with the option, page 1 runs `globalThis.x = 1;
   document.title = "one"` and page 2 runs `document.title = typeof x`; after `go` to page 2 its
   title is `undefined`.
4. `noscript_scripted`: a page with `<noscript><p>Off</p></noscript>` in body shows `Off` in a
   session without the option and follows the rule of Q3 in one with it (both sessions in one
   test).
5. `surf <url>` on the page of example 1 prints `title <document-id> "Before"` (a test in
   `crates/surf-cli/tests/`). If `story:runner-run` has merged on this story's base, `surf run`
   over a page with a script runs no page script, shown by a test in `crates/surf-cli/tests/`; if
   not, the report says so and `story:runner-run` keeps that case.
6. `slot task lock-check` passes with `surf-session` on the closed list; its case that `surf`
   depending on `surf-script` fails still passes; `git diff --stat` names neither
   `conformance/Cargo.toml` nor `conformance/Cargo.lock`.
7. A page whose script throws still shows every other element, and its end is recorded as `threw`
   with its message, line and column as Q16 bounds them.
8. `drift`, `slot task cli` and `slot task conform-rust` pass; `task open` no longer lists the
   marker of `story:scripts-spec`; `sh taskfiles/conform-noticed.sh check` passes.
9. `slot task lock-check` names swell by the tag the coordinating agent named on the record of
   `docs/design/scripts.md` Q13.
10. Decision 3: the report gives the measured `VmHWM` and descriptor count of each page beside the
    bound of Q11, with the probe.
11. `slot task check` exits 0; `git diff --stat` names only the files of the Change.
12. Public surfaces: the story names every public surface its change makes false (a badge, a
    `README.md` line, a website page, a count) and changes it in its last commit, every number
    from the log of a gate run on its tip, which the coordinating agent orders. For this story,
    lines on `main` at `29945f5` (2026-11-24): the sentences that surf has no JavaScript at
    `README.md:31-34`, `:150`, `:230`, `website/docs/index.md:23-26`, `:30`, `:48`,
    `website/docs/status.md:11`, `website/docs/what-works.md:281` and
    `website/docs/drive-from-an-agent.md:309`; the test count of `README.md:11` and `:285-286`
    and `docs/badges/make.sh:38` with `docs/badges/tests.svg`; the gate block of
    `README.md:584-625` and of `website/docs/how-it-is-built.md:40-81`.
13. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; the test
    servers start in Rust on 127.0.0.1, so no request leaves the host; no model call; each case
    starts one session, nothing more per test; no Python, the probes of decision 3 included; no
    generator with a committed output is added; `task conform-slow` runs in the lane `slow`
    because its sources changed; a build or test job in the pool ends within 10 minutes or names
    its time with `slot --timeout`, at most 30 minutes; a test over 5 s is named.

## Out of scope

The hostile pages (`story:scripts-hostile-pages`); scripts in `surf <url>` and `surf run` (S7);
external, module, `async` and `defer` scripts; events and timers.

## Placement

After `story:scripts-spec`, `story:session-process-hardening`, `story:html-script-hook`,
`story:scripts-dom-mutation`, `story:scripts-dom-attributes`, `story:scripts-parse-run` and
`story:scripts-html-conformance` (which writes `Cargo.lock`, the closed list and
`crates/script/Cargo.toml` before it), and the named condition. `crates/session`,
`crates/surf/src/lib.rs` and `conformance/` are also written by active stories outside the epic:
never in the same wave.

## Why `epic:scripts-in-a-page`

The epic's outcome itself: a session that runs a page's inline scripts and shows what they wrote.
