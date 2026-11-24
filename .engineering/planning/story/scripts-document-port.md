---
format: aep.planning-md/3
id: story:scripts-document-port
kind: story
status: proposed
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
- confidence: inferred
  path: crates/answer/src/lib.rs
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
  path: docs/design/html-parser.md
- confidence: cited
  path: docs/design/scripts.md
- confidence: cited
  path: taskfiles/lock-allow.txt
- confidence: cited
  path: taskfiles/lock-owners.txt
- confidence: cited
  path: website/docs/drive-from-an-agent.md
- confidence: cited
  path: website/docs/index.md
- confidence: cited
  path: website/docs/status.md
- confidence: cited
  path: website/docs/what-works.md
revision: 30
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:14Z", actor: "human:operator", revision: 25}
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

S5 is decided (option a, the operator session, 2026-11-24): a session with scripts runs under
`setpriv` and `prlimit`, in the form with scripts of `docs/design/scripts.md`, Q11
(`story:session-process-hardening`, an edge of this story).

The tag of swell: every story before this one builds and tests on `0.8.0` (Q13). The copies swell
makes without a memory charge, which it lists as open
(`docs/plan/second-batch.md:284-285 @ 0.8.0`), are not bounded by `memory_bytes`; the address-space
bound of Q11 is their only bound. Requirement 11 (`docs/design/scripts.md`, Q18) asks swell to
charge every copy that is alive while the engine calls into script; its status at `0.8.0` is
`absent`. Q13's decision (the coordinating agent, 2026-11-24, reversible): this story, the one that
lets a page's scripts run in a session, waits for the swell tag that holds requirement 11 and takes
that tag in its first commit.

Found while `story:scripts-spec` was built (2026-11-24): the session's cap of 256 rows of entries
per request does not count the event `ScriptEndAdded`. A page may add up to 1,000 script records
(Q4), so this story decides whether those events count against the cap or have a bound of their
own, and holds the answer with a test.

Found by the reviews of `story:session-process-hardening` and `story:scripts-engine-dependency`
(2026-11-24), each for this story to hold with a test:

- No test starts the built `surf-session` under the bounds of a session with scripts: until this
  story the session refuses `open` with `arg scripts` as usage, so every case with scripts ran the
  test stand-in. This story's first test of the option starts the built session through `surfd`
  and reads `/proc/<pid>/limits` and `NoNewPrivs` of it.
- Three tests of `surf-script` are ignored at swell `0.8.0` (`clock_advances`, `seeds_fixed`,
  `time_zone`; `Date` and `Math.random` are `pending slice 4`). They hide one claim: that the
  runner hands its mode's seeds to the agent. The commit that takes a swell tag with those
  builtins removes the `#[ignore]` lines.
- `surf-script` matches swell's error types arm by arm, so a tag that adds a variant stops the
  build where surf must choose the word.

## Domain relations

- `surf.document.Element` to `surf.document.Document`, many to one: inferable,
  `spec/domains/document.yaml`, entity `surf.document.Element`, relation `document` (line 297).
  The link a script adds is an `Element` row of its document, as a parsed one is.
- The record of a script's end and its document: as `story:scripts-spec` writes it, after
  `decision-blocker:script-end-record`.
- `surf.agent.Session` and its scripting option: inferable (inferred from
  `surf.agent.OpenCall.ca_file`, `spec/domains/agent.yaml:102-119`).

## Waits for

- waits for a release of swell: the tag that holds requirement 11 (`docs/design/scripts.md`, Q18),
  as Q13 decides; this story starts when that tag exists and takes it in its first commit. The note
  records the tag when it comes back from swell (Q11, Q13).
- with that tag, promises and jobs exist (swell's slice 10 is closed in the same release, as
  swell's session said on 2026-11-24). The note leaves out step 1 of HTML's steps at a script's
  end tag, "perform a microtask checkpoint", because "no promise and no job exist before swell's
  slice 10" (`docs/design/scripts.md`, Q2, row 1, and Q18). So the first commit that takes the
  tag also decides and writes into the note: the microtask checkpoint after each script, its
  steps counted in that script's and the document's totals, and its end taken as the script's
  end. Open until then: which call of the host interface runs the jobs, and what a job that
  ends at a limit makes of the record of a script that already ended `ran`.
- with that tag, the host interface's error type gains a variant (swell's session, 2026-11-24);
  a `match` of surf over the host's errors takes it in.

## Decisions taken

1. `surf-session` gains the dependency `surf-script`; `taskfiles/lock-owners.txt` gains the line
   `surf-script surf-session` in this story's change (Q14, Q15). `crates/surf` gains no dependency
   on `surf-script` (Q14): the session hands its scripted parser step to `Browser::with_parser`,
   and what the parser step answers carries the script ends in `surf`'s own types. So
   `conformance/Cargo.toml` and `conformance/Cargo.lock` do not change.
2. With the option, every committed HTML document of the session is parsed by
   `surf_script::run::parse_with_scripts` (`story:scripts-parse-run`) with
   `scripting_disabled: false` (Q3), on the thread of Q5, one agent per document, made at its first
   script element that runs and dropped at the end of its parse, so no state passes from one page
   to the next; the session issues `surf.document.AddScriptEnd` (`story:scripts-spec`) once per
   end, after the parse and before `FinishParsing`, in the order the ends happened (Q7's
   `written:`). Without the option, `HtmlParser` as today, and no record.
3. The bounds of a session with scripts (Q11). This story ships the address space `--as=2147483648`
   (2 GiB): with the swell tag of requirement 11 the copies are charged to `memory_bytes`, and with
   `story:session-memory-bound` keeping the tree of the current document only, Q11's sum for one
   tab is 208,736 kB (the session) + 2 × 536 MiB (the current tree and the one being parsed) +
   8,739,584 bytes (the thread) + 64 MiB (its malloc arena) + 64 MiB (`memory_bytes`) + 64 MiB (an
   allowance for what swell holds without a charge beside the copies), about 1,480 MiB. If
   `story:session-memory-bound` is not built before this story, it keeps the 4 GiB backstop
   (`--as=4294967296`) and says so in its report and in Q11. The sum holds for one tab; each
   further tab adds a tree of up to 536 MiB, and this story writes into Q11 which it ships: the
   bound raised per tab, or a bound over all tabs. The CPU bound `--cpu=3600:3601` and the
   open-files bound `--nofile=64:64` stay. The bounds are measured again with scripted sessions on
   the pages Q11 names (`VmHWM` and `VmPeak` in `/proc/<pid>/status`, the descriptors in
   `/proc/<pid>/fd`, the CPU ticks); where a value does not hold, Q11 and the shipped value in
   `surfd`'s `Config` change in this story, each with its `probe:` line.
4. How many sessions with scripts may be open at once, the question Q11 leaves to this story:
   `surfd` on `main` bounds no number of open sessions (it counts the processes not yet reaped only
   to wait for them when it stops, `crates/surfd/src/daemon.rs:219-220`, `:1240`, `:420`), so k
   scripted sessions on hostile pages can hold k times the address-space bound of the host at once.
   This story answers it: it writes into Q11 the number of sessions with scripts `surfd` holds open
   at once, with that number times the address-space bound set against the memory of the host, and
   `surfd` refuses an `open --scripts true` past it with the refusal Q11 then names; a session
   without scripts is not counted. Where `story:agent-surfd-session-limit` (a bound on open
   sessions of every kind, active on 2026-11-24) has merged by then, the bound for scripted
   sessions is written beside its bound.
5. `noscript` (Q3): in a document parsed with scripting enabled a `noscript` element holds one text
   node and is hidden with everything in it. The sentence of `surf-page`'s hidden rule becomes
   Q3's, in `crates/page/src/lib.rs:56-57` and `docs/design/html-parser.md:318`: "`noscript` is not
   hidden in a document parsed with scripting disabled, where its content is markup; in a document
   whose scripts run, it is parsed with scripting enabled, holds one text node, and is hidden, with
   everything in it."
6. What an answer prints (Q7, Q16): in a session with the option, after the `page` line, one line
   `scripts <n> ran <a> threw <b> refused <c> limit <d> faulted <e> not-run <f>` for the current
   document, then at most 10 lines, one per end that is not `ran`, in position order:
   `  <position> threw <kind> "<message>" <line>:<column>`,
   `  <position> refused "<message>" <line>:<column>`, `  <position> limit <word>`,
   `  <position> faulted`, `  <position> not-run <word>`, with `+ scripts more <m>` when more
   remain, as the `changed` block bounds its lines (`docs/agent-answer.md:325-326`). A message is
   page text, untrusted: kept to its first 1,024 characters in the record (Q16), quoted and cut at
   120 characters with `surf_answer::quote_cut` in the answer; `<line>:<column>` is written
   `eval <line>:<column>` when `dynamic` is true. A session without the option prints no `scripts`
   line. This story writes the line into `docs/agent-answer.md`, and `surf-answer` parses and
   renders it.
7. The tag: the first commit of this story moves swell from `0.8.0` to the tag that holds
   requirement 11 (`docs/design/scripts.md`, Q18): the dependency line of
   `crates/script/Cargo.toml`, the six lines of `taskfiles/lock-allow.txt` (address, tag and
   commit, `none` for the checksum, Q15) and `Cargo.lock`, in one change, and the note records that
   tag (Q11, Q13).

## Change

- `crates/script/Cargo.toml`, `Cargo.lock`, `taskfiles/lock-allow.txt`: the tag of decision 7, in
  the first commit.
- `crates/session/` (its `Cargo.toml` included): decisions 1 and 2; `taskfiles/lock-owners.txt`:
  the line `surf-script surf-session`.
- `crates/surf/src/lib.rs`: what the parser step answers gains the script ends (Q7), so the rows
  are written from the tree the scripts left.
- `crates/page/src/lib.rs` and `docs/design/html-parser.md`: the hidden rule for `noscript` of
  decision 5.
- `crates/agent/src/`, `cli/surf-agent.yaml`, `crates/agent/tests/drift.rs`: the client accepts
  `--scripts true` and sends `arg scripts "true"` in the `open` request; the `UNMAPPED:` marker and
  the `BINDING_ONLY` entry of `story:scripts-spec` go.
- `docs/agent-answer.md`, `crates/answer/src/lib.rs` and its tests: the `scripts` lines of decision
  6.
- `conformance/src/target.rs` and `conformance/tests/noticed.txt`: the target issues what the
  session issues; the list gets its new digest from `task conform-slow WRITE=1`.
- `crates/surf-cli/tests/`: only tests.
- `crates/surfd/src/daemon.rs`: the shipped address-space bound of decision 3 and the bound of
  decision 4; `docs/design/scripts.md`: Q11 as decisions 3 and 4 find it, and the tag of decision
  7.
- In the last commit, the public surfaces of acceptance 14.

## Acceptance

1. Example 1: a page `<title>Before</title><script>document.title = "Written by script"</script>`
   served on 127.0.0.1; in a session opened with the option, `surf-agent look` prints
   `page 200 "text/html" "<address>" "Written by script"`; in a session without it, the page line
   ends `"Before"`.
2. Example 2:
   `<ul id=l></ul><script>var a = document.createElement("a"); a.setAttribute("href", "/next"); a.textContent = "Next"; document.getElementById("l").appendChild(a);</script>`:
   in a session with the option, the document holds an `Element` row of kind `link`, label `Next`,
   target `/next` (a test of `surf-session`'s core), and `surf-agent` lists that link; without the
   option, it holds none.
3. `no_state_between_pages`: in one session with the option, page 1 runs
   `globalThis.x = 1; document.title = "one"` and page 2 runs `document.title = typeof x`; after
   `go` to page 2 its title is `undefined`.
4. `noscript_scripted`: a page with `<noscript><p>Off</p></noscript>` in body shows `Off` in a
   session without the option, and in a session with it shows neither `Off` nor any row of the
   `noscript` element (it is hidden with everything in it, decision 5), both sessions in one test;
   `git grep -n -F` of the first words of Q3's sentence prints a line of `crates/page/src/lib.rs`
   and one of `docs/design/html-parser.md`.
5. `surf <url>` on the page of example 1 prints `title <document-id> "Before"` (a test in
   `crates/surf-cli/tests/`). `surf run`: either the test `surf_run_runs_no_script` in
   `crates/surf-cli/tests/` exists and passes (a page with a script, the script's effect absent),
   or on the story's base `git grep -n 'surf run' -- crates/surf-cli/src` prints nothing (the usage
   line of `crates/surf-cli/src/main.rs:28` at `29945f5` names no `run`), and the report names the
   case for `story:runner-run`.
6. `slot task lock-check` passes with the line `surf-script surf-session` in
   `taskfiles/lock-owners.txt`; its case that `surf` depending on `surf-script` fails still passes;
   `git diff --stat` names neither `conformance/Cargo.toml` nor `conformance/Cargo.lock`.
7. A page whose script throws (`<script>throw new Error("boom")</script>` among other elements)
   still shows every other element; its end is recorded `threw` with the kind `Error`, the message
   `boom`, its line and column (Q16), and the answer holds
   `scripts 1 ran 0 threw 1 refused 0 limit 0 faulted 0 not-run 0` and
   `  1 threw Error "boom" <line>:<column>`.
8. `scripts_lines`: a page of `<script>1</script>`,
   `<script>throw new Error(Array(201).join("x"))</script>`, `<script>class A {}</script>`,
   `<script>eval("throw new Error(1)")</script>`, ten `<script src=a.js></script>` and a last
   `<script>x = 1` closed by the end of the input: the answer prints
   `scripts 15 ran 1 threw 2 refused 1 limit 0 faulted 0 not-run 11`, then the ten ends of
   positions 2 to 11 in position order (position 2's message cut by `surf_answer::quote_cut` at 120
   characters, position 4's site written `eval <line>:<column>`, positions 5 to 11 `not-run src`),
   then `+ scripts more 4`; the record of position 15 is `not run: eof`. The same page in a session
   without the option prints no `scripts` line.
9. `drift`, `slot task cli` and `slot task conform-rust` pass; `task open` no longer lists the
   marker of `story:scripts-spec`; `sh taskfiles/conform-noticed.sh check` passes.
10. Decision 7: `slot task lock-check` names swell by the tag that holds requirement 11
    (`docs/design/scripts.md`, Q18); `git show --stat` of the branch's first commit names
    `crates/script/Cargo.toml`, `Cargo.lock` and `taskfiles/lock-allow.txt`;
    `git grep -n -F 0.8.0 -- crates/script/Cargo.toml taskfiles/lock-allow.txt` prints nothing.
11. Decision 3: `/proc/<pid>/limits` of a session opened with the option shows `Max address space`
    2147483648 soft and hard (4294967296 where the report says `story:session-memory-bound` was not
    built before this story), `Max cpu time` 3600 and 3601, `Max open files` 64; on each page of
    Q11, `VmHWM` and `VmPeak` stay below the address-space bound and the descriptors below 64, or
    the story changes that bound in Q11 and in `Config` and names the page that forced it; the
    report gives each value with its `probe:` line, and Q11 names the bound with more than one tab.
12. Decision 4: with the number of sessions with scripts set to 2 in the test's `Config`, a third
    `open --scripts true` is refused with the refusal Q11 names and starts no process, an `open`
    without the option still opens a session, and after one session with scripts is closed an
    `open --scripts true` opens one; Q11 holds the shipped number and its reason.
13. `slot task check` exits 0; `git diff --stat` names only the files of the Change.
14. Public surfaces: the story changes, in its last commit and after merging `main`, every public
    surface its change makes false, each number taken from the `slot task check` log of its tip.
    After that commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges` prints
    nothing for each old string below, as the story's base holds it (the strings and lines given
    are those of `main` at `29945f5`, 2026-11-24); each new number equals the one in that log; and
    the gate block, `README.md:584-625` and `website/docs/how-it-is-built.md:40-81`, holds that
    log's lines. The old strings of this story: the sentences that surf runs no script,
    `no JavaScript yet` (`README.md:31`, `:230`, `website/docs/index.md:23`, `:30`, `:48`,
    `website/docs/status.md:11`), `runs no script today` (`README.md:34`,
    `website/docs/index.md:26`), `No JavaScript and no rendering` (`README.md:150`),
    `- No JavaScript.` (`website/docs/what-works.md:281`) and `No JavaScript, and no rendering`
    (`website/docs/drive-from-an-agent.md:309`); the test count `1976 passed` (`README.md:11`,
    `docs/badges/make.sh:38`, `docs/badges/tests.svg`), `1,976` and `1,424, and 552`
    (`README.md:285`, `website/docs/status.md:23`, `website/docs/what-works.md:17`).
15. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; the test
    servers start in Rust on 127.0.0.1, so no request leaves the host; no model call; each case
    starts one session, nothing more per test; no Python, the probes of decision 3 included; no
    generator with a committed output is added; `task conform-slow` runs in the lane `slow` because
    its sources changed; a build or test job in the pool ends within 10 minutes or names its time
    with `slot --timeout`, at most 30 minutes; a test over 5 s is named.

## Out of scope

The hostile pages (`story:scripts-hostile-pages`); scripts in `surf <url>` and `surf run` (S7);
external, module, `async` and `defer` scripts; events and timers.

## Placement

After `story:scripts-spec`, `story:session-process-hardening`, `story:html-script-hook`,
`story:scripts-dom-mutation`, `story:scripts-dom-attributes`, `story:scripts-parse-run` and
`story:scripts-html-conformance` (which writes `Cargo.lock`, `taskfiles/lock-owners.txt` and
`crates/script/Cargo.toml` before it), and the swell tag that holds requirement 11
(`docs/design/scripts.md`, Q18). `story:session-memory-bound` is not an edge: built before this
story, it lets the story ship the 2 GiB bound of decision 3; otherwise the story keeps the 4 GiB
backstop and says so. `crates/session`, `crates/surf/src/lib.rs` and `conformance/` are also
written by active stories outside the epic: never in the same wave.

## Why `epic:scripts-in-a-page`

The epic's outcome itself: a session that runs a page's inline scripts and shows what they wrote.
