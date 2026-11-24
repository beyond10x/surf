---
format: aep.planning-md/3
id: story:scripts-decisions
kind: story
status: draft
title: 'Settle the first version of page scripts in a design note: boundary, limits, the Web IDL selection, the session''s bounds and the engine''s requirements'
summary: docs/design/scripts.md answers the 18 questions the epic leaves open, each by a measured value, a table or a cited section, records the operator's answers to S5 and S6, and hands requirements 1 to 10 to the script engine's session.
relations:
- decomposes: epic:scripts-in-a-page
scope:
- confidence: cited
  path: docs/design/scripts.md
revision: 2
---
## Context

`epic:scripts-in-a-page` leaves its open questions to one design note (epic, "Candidate stories",
item 1, lines 408-414, and the sentences that name `scripts-decisions` at lines 113, 225, 262,
507, 650 and 702). `story:scripts-engine-dependency`, `story:scripts-spec` and
`story:session-process-hardening` start on the note; `story:html-script-hook` and
`story:webidl-bindings` follow it through them. The note writes no code and changes no
specification.

Every `file:line` below is read on `main` of 2026-11-24 (the planning branch this story was
drafted on carries older code). Where the epic and the tree disagree, the tree wins, and the note
works from the tree:

- `task lock-check` already holds an allow-list: `taskfiles/lock-allow.txt` (19 lines, the
  packages of ring) is matched on name, version, source and checksum, and no crate but `surf-tls`
  may depend on a listed package (`taskfiles/lock-check.sh:7-9`, `:28-30`, `:157`;
  `Taskfile.yml:282-303`). The epic (lines 149-151) describes the check before ring.
- `AGENTS.md` "Goals" (`AGENTS.md:27-43`) already names one exception, ring; the sentence the
  epic rewrites (epic lines 170-172) is that one.
- The root `Cargo.toml:3` reads `members = ["crates/*", "generated/rust/crates/*"]`: a new crate
  under `crates/` joins the workspace without a changed line (epic lines 419 and 427 name a
  shared `members` line).
- The lane `slow` exists (`Taskfile.yml:612-617`: `tls-certs-slow`, `tls-flights-slow`,
  `conform-slow`); the epic (line 760) calls it new.
- At swell `0.8.0` (2026-11-24, commit `0c1827128b57845e203713e87a0b0949fc8d63be`) the crate
  `swell` depends by path on `swell-compiler`, `swell-num`, `swell-regexp`, `swell-syntax` and
  `swell-unicode` (`crates/swell/Cargo.toml` at the tag): six packages a host takes, where the
  epic (lines 155-157, 168) counts five at `6a43e18`.
- `surf <url>` prints `title <document-id> "<title>"`, quoted (`docs/surf-command.md:28`); the
  epic (line 65) writes it unquoted.
- The six `not-run:needs-script` tree-construction cases are files of web-platform-tests
  (`crates/html-conformance/expected/tree-construction.txt:25-28`,
  `wpt/html/syntax/parsing/resources/scripted_*.dat`); only the encoding case is html5lib's
  (`expected/encoding.txt:2`).
- Three test-only counting allocators are `unsafe` (`crates/fetch`, `crates/html`, `crates/tls`;
  `AGENTS.md:276-280`); the epic (lines 258-259) names one.

## Decisions taken

1. The note is `docs/design/scripts.md`. One section per question below, headed
   `## Q<n>. <title>` with the number and title of this story, in this order. Each section ends
   with one line that starts `Settled: ` and names how: a value with the command that measured it
   and its output line, a table in the section, or a cited section of a standard with its address
   and the date or commit of the text read. An answer the note cannot give is a line that starts
   `Open: ` naming the story it stops; no `Open:` line may name `scripts-engine-dependency`,
   `scripts-spec` or `session-process-hardening`.
2. Measurements are made by probe programs in the session scratch directory, never committed, in
   Rust or POSIX sh (no Python). A probe that builds runs through `slot` and names its time when it
   needs more than 10 minutes (`slot --timeout`, at most 30 minutes); a probe that only measures,
   times or waits runs without `slot`. The note names each probe by its command.
3. swell is read at a tag in a clone of `https://github.com/beyond10x/swell`, read only. A change
   in swell is the engine session's; the note hands it requirements, it does not write them there.
4. S5 and S6 are the operator's. The coordinating agent puts each to the operator with its default
   (a and a, epic lines 504-532); the note records the answer, or "S5 taken on default a" and "S6
   taken on default a" after 15 minutes without one (the organization rule "A default proceeds").
5. The note records the answer of `decision-blocker:script-end-record` (Q7); clearing the blocker
   is the operator's.

## Domain relations

- `surf.document.Document` and the record of one script's end: requires-stakeholder-input, filed
  as `decision-blocker:script-end-record`; Q7 proposes the answer.
- `surf.document.Element` to `surf.document.Document`, many to one: inferable, `spec/domains/
  document.yaml`, entity `surf.document.Element`, relation `document` (line 297). An element a
  script made is a row of this relation like a parsed one.
- `surf.agent.Session` and its scripting option, one value per session set by `open` and ending
  with it: inferable (inferred from the field `ca_file` of `surf.agent.OpenCall`,
  `spec/domains/agent.yaml:102-119`, an `ess/1` field and comment, no `relations:` entry; the
  operator's S3 and S7 make it one option per session, off by default).

## The questions

Q1. Which script elements run. Settled when a table holds one row per case: no `type`; an empty
`type`; each JavaScript MIME type essence; `type=module`; any other `type`; a `language`
attribute; `src` present, with and without text; `nomodule`; `async` or `defer` on an inline
script; a script inside `template` contents; an SVG `script` in foreign content
(`docs/design/html-parser.md:627`); a script element a script created or moved. Each row says
"runs" or "not run: <reason>", citing the step of "prepare the script element" of the HTML
Standard (section number, anchor, the standard's date or commit). This confirms or corrects the
sentence on `nomodule` (epic line 113).

Q2. The steps of the `script` end tag. Settled when a numbered table lists each step of 13.2.6.4.8
for an end tag named `script` and each step of "prepare the script element", each "taken" or
"left out: <reason>" (the insertion point, a script nesting level above 1, the pending
parsing-blocking script, spinning the event loop, `document.write`). Today the end tag pops the
element and returns to the original insertion mode (`crates/html/src/tree_builder/modes.rs:
1120-1141`). The section also gives the hook's signature that `story:html-script-hook` writes:
what it receives (the tree, the script element) and what it returns, and what the tree builder
does with an open element or a formatting element the hook removed or moved; and which part of
the `html-sections` line `tree-construction script-processing` (`docs/design/html-parser.md:539`)
stays open.

Q3. Parser scripting enabled. Settled when the section states that a document whose scripts run
is parsed with `scripting_disabled: false` (`crates/html/src/lib.rs:180-195`, default `true`),
what that changes for `noscript` in head and in body (the insertion modes cited), the new
sentence of `surf-page`'s hidden rule, which today says `noscript` is not hidden because surf parses
with scripting disabled (`crates/page/src/lib.rs:56`, `docs/design/html-parser.md:318`), and the
count of `#script-on` cases `task html-conformance` prints (`crates/html-conformance/src/
runner.rs:121` runs them with the flag false).

Q4. Limits, with values. Settled when a table gives, for each limit, its value, the probe command
and output line it rests on, and the outcome recorded when it is reached: swell's agent limits
(`steps_per_entry`, `memory_bytes`, `frame_depth`, `reentry_depth`, `parser_nesting`,
`string_length`, `bigint_bits`; epic lines 199-205); a step total per document summed from
`last_entry_steps()`; a number of scripts run per document; the bytes of one script's source; a
wall-clock bound per document through the interrupt handle; and the parser limits every DOM
write is held to (`crates/html/src/limits.rs:16-47`: input bytes, depth 512, nodes 1,048,576,
attributes 1,024, stored bytes 64 MiB, steps 2^27 by default). The basis: steps per second on
this host in the test profile and in release, the heap of a realm of the bindings' size (from
the engine's constants of requirement 10 where the tag has them, else from a probe with 300 host
functions and 40 host objects). Also: what follows `Terminated(MemoryBudget)` (epic line 840:
`collect()` once, then no further script in that document), and whether `eval` and `Function`
are refused through `host_ensure_can_compile_strings` (HOST-116, HOST-117; epic line 210).

Q5. The thread and the stack. Settled when `Limits::min_stack_bytes()` for the limits of Q4 is
measured by a probe in surf's test profile (`debug = false`, `.cargo/config.toml`) and in release
(swell measured 14,672,640 bytes in a debug build for depths of 1,000, epic lines 222-224), and
the section names the thread a scripted document is parsed on and its stack size (the session
serves its requests on its main thread today, `crates/session/src/process.rs:144-148`), and when
the agent is dropped: at the end of the document's parse, or with the document (scripts run only
while parsing, epic lines 117-119; HOST-136: an `Agent` is not `Send`).

Q6. Seeds, clock and time zone. Settled when each hook is named with its HOST id and what it
answers in a session and in a test: the clock `host_system_utc_epoch_nanoseconds` (HOST-125), a
value that advances between two calls (epic line 846; a test advances it by a fixed amount per
call); the time-zone hooks (HOST-126 to HOST-128); the seeds of `Math.random` (HOST-26, HOST-27,
HOST-144, HOST-147), with their source in a session and fixed values in a test; and whether the
tag of Q13 has each.

Q7. The specification of a script's end and of the option. Settled when the section holds the
YAML it proposes for `spec/domains/document.yaml` and `spec/domains/agent.yaml`, and states:
- the record of a script's end and its relation to `surf.document.Document` (cardinality, owner,
  when it is written, that it never outlives its document): the answer of
  `decision-blocker:script-end-record`;
- the closed list of ends: ran; threw (kind, message, line, column); refused by swell (its
  `SyntaxError` message, which names the delivery slice); ended by a limit (which); not run (the
  reasons of Q1, and after a limit of the document); and whether a document parsed without
  scripts records its script elements at all;
- the command and event that write it, which process issues the command, and what becomes of
  the marker "no component for script execution" (`spec/components.yaml:429-430`);
- the option: its field on `surf.agent.OpenCall` (`spec/domains/agent.yaml:107-119`), its type
  and spelling under `ess-cli/2`, where each input field has exactly one argument mapping and a
  Boolean takes an explicit `true` or `false` (ESS `docs/design/cli-presentation-binding.md:
  175-178`), so a bare `--scripts` is not a form the binding takes; how it reaches the session
  (as `arg anchors` does: the `open` request on the socket, `crates/session/src/process.rs:
  17-19`, `crates/surfd/src/daemon.rs:1154-1157`); whether `surf.agent.Session` records it;
- what an answer prints of a script's end (a line of `docs/agent-answer.md`, its grammar, every
  text from the page quoted as untrusted), or that it prints nothing.

Q8. The Web IDL selection. Settled when a table holds one row per member, `| <interface> |
<member> | <slice> | <reason or deviation> |`, for the 146 members of `EventTarget`, `Node`,
`Element` and `Document` (epic lines 631-642) and the 68 of the interfaces they return (epic lines
644-650), the slice one of `reading`, `attributes`, `mutation`, `selectors`, `throws`; over the
146 the counts are 73, 14, 29, 7 and 23 (epic lines 681-715), and the 68 are assigned by the same
rules. Each deviation stands beside its member: the 9 members that return a snapshot for a live
collection, `[SameObject]` on `childNodes` and `children`, an element's wrapper being an
`Element`, `TrustedType` read as `DOMString`, the global `this` being the `Window` (no
`WindowProxy`). The section also gives the grammar of `crates/webidl/selection.txt`, which
`story:webidl-bindings` writes from this table.

Q9. The selector subset. Settled when the section gives the grammar of the subset (type,
universal, `#id`, `.class`, attribute presence and equality, compounds, descendant and child
combinators, lists; epic lines 701-705), each production cited to its section of Selectors Level
4, the case rules in an HTML document cited, and which input throws `NotSupportedError` (valid,
outside the subset) and which `SyntaxError` (invalid).

Q10. The session's environment. Settled when the section gives the allow-list of variables
`surf-session` is started with (it may be empty) and, for each, the line of code that reads it.
For this story, `git grep -n 'env::var\|var_os' main -- 'crates/*/src/*.rs'
'crates/*/src/**/*.rs'` on 2026-11-24 finds reads in `crates/agent-tasks`, `crates/agent`
(the client), `crates/surfd/src/main.rs` and `crates/view-fixture`, none in `crates/session`,
`crates/surf`, `crates/fetch` or `crates/tls`; the note runs it again on its base. The section
also decides, each with its reason, on the variables the standard library and the C library read
(`RUST_MIN_STACK`, `RUST_BACKTRACE`, the resolver's `LOCALDOMAIN`, `RES_OPTIONS`, `HOSTALIASES`,
`TZ`), and how the tests' `Config::session_env` (`crates/surfd/src/daemon.rs:83`) still add
theirs.

Q11. S5, and the bounds of the operating system. Settled when the section records the
operator's answer to S5 (default a) and, under a: the two programs by absolute path and their
presence on this host (`command -v setpriv prlimit` on 2026-11-24: `/usr/bin/setpriv`,
`/usr/bin/prlimit`, util-linux 2.42.4) and on the forge's runner image (a workflow run's output,
or the image's package list cited); each bound with its value and the measurement it rests on,
the peak of a session (`VmHWM` in `/proc/<pid>/status`, open descriptors counted in
`/proc/<pid>/fd`) on a named set of pages: address space, CPU seconds, open files, core size 0,
`NoNewPrivs`; the number of processes only with care, because `RLIMIT_NPROC` counts every process
of the user, so a value below the user's count stops the session from starting a thread; whether
every session runs under them or only one whose `open` asks for scripts, and in the second case
how `surfd` learns it (it passes the `open` request on unchanged today,
`crates/surfd/src/daemon.rs:1154-1157`); that both programs replace themselves by `exec`, so the
pid `surfd` holds is the session's (from memory today: confirmed by a probe); and what `open`
answers when a program is missing.

Q12. S6, and the source of HTML's definitions. Settled when the section records the operator's
answer to S6 (default a) and the file `story:webidl-bindings` vendors for it: the HTML Standard
at a commit snapshot (`https://html.spec.whatwg.org/commit-snapshots/<commit>/`, its size
measured with `wc -c`) or another extraction of its IDL, with address, commit and licence; and,
under a, the definitions the reader must read for the four members (`Window`, and the partial
`Document` that holds `title`).

Q13. The tag of swell. Settled when the section names the tag `story:scripts-engine-dependency`
pins, with its commit and the package list of its root `Cargo.lock` (at `0.8.0`, the newest
release on 2026-11-24: 8 packages, none with a `source` line; the host takes the six above, and
`swell-test262` and `swell-unicode-gen` are tooling), and states whether a tag holds the fixes
of "From the security review of 2026-11-22" (epic lines 598-603), read from its `CHANGELOG.md` and
`conformance/host/requirements.txt` with the HOST ids that state them. "No tag yet" is an answer;
it stops only `story:scripts-document-port`.

Q14. Where swell is linked. Settled when the section gives the crate graph: `surf-script` alone
depends on swell and only `surf-session` on `surf-script`, so `surf`, `surf-agent` and `surfd`
link no swell (S7); the scripting parser step reaches the session through `Browser::with_parser`
(`crates/surf/src/lib.rs:1563`), not through a change of `HtmlParser` (`:415`); and the check that
holds the graph in the gate.

Q15. `lock-check` for a git source. Settled when the section gives the allow-list line for a
package from git (read from a lock cargo writes for a probe that depends on the tag: whether it
has a `checksum` line), the rule that names which crate of this repository may depend on which
listed package (today only `surf-tls` on any, `taskfiles/lock-check.sh:28-30`), and the new
sentence of `AGENTS.md` "Goals" and of the `lock-check` description.

Q16. What a recorded error holds. Settled when the section names the fields read after a script
threw or was refused, the kind and own `message` of the error (requirement 5) and the line and
column of `Agent::last_entry_error_site` (HOST-172 to HOST-174), the most bytes kept of each text,
and whether and how they reach an answer.

Q17. The epic's statements from memory. Settled when each is confirmed or corrected with a
citation: `nomodule` skips a classic script (epic lines 112-113, with Q1); cargo reads profiles
from the root workspace only, so surf sets `overflow-checks = true` for swell's packages (epic
lines 180-183; swell's root `Cargo.toml` at `0.8.0` sets it in all four profiles); Apache-2.0
section 4(d) and swell's `NOTICE` (epic lines 183-184); `TrustedType` read as `DOMString` (epic
lines 668-669).

Q18. Requirements 1 to 10 for the engine. Settled when the table of the next section stands in the
note with, for each row, its HOST ids and its status at the tag of Q13, read from
`conformance/host/requirements.txt` there.

## Requirements 1 to 10, as the epic's last section revised them

| n | requirement | how the engine states it | batch | first surf story that needs it |
|---|---|---|---|---|
| 1 | the line and column where a parse error or an uncaught throw arose | `Agent::last_entry_error_site`: line, column, and whether the code was dynamic or host defined (HOST-172 to HOST-174) | H3 | `scripts-document-port` |
| 2 | every operation whose work grows with its operands charges steps in proportion, the property scan of 0.2.0 included; parsing and compilation charged to the memory budget before they allocate; property operations charge their work and temporary storage first (the review's first condition) | string and object charging, and the rest of 2 | H2, H3 | `scripts-document-port` (the tag condition) |
| 3 | the host charges and credits bytes and charges steps on the engine's one ledger through `Cx` | `charge_host_bytes(n)` answers a `HostBytes`; dropping it credits, also in a sweep; no free-standing credit; surf asked for a charge changed in place; the bindings charge steps in proportion to a member's work | H2 | `scripts-dom-mutation` |
| 4 | withdrawn as a need of surf | surf keeps one wrapper per node strongly for the document's life, bounded by the node limit; the engine writes down what a kept handle does | H2 | `scripts-dom-reading` |
| 5 | an error's kind and own `message` read without running code or a hook | read from the error's own slot; `HostObjectSpec` gains `error_kind`, for `DOMException` | H3 | `webidl-bindings` |
| 6 | HOST-17, HOST-152 and HOST-163 hold under the embedder's profiles | shown in the engine's own development and release profiles; surf tests its own profile | H1 | `scripts-engine-dependency` |
| 7 | each release names every change to a signature or requirement of the host interface | a committed snapshot of the host interface with a gate check | H1 | `scripts-engine-dependency` |
| 8 | a host function's [[Prototype]] named or set | `Cx::set_prototype_of`, `Cx::get_prototype_of` | H1 | `webidl-bindings` |
| 9 | a closed list of intrinsics and well-known symbols | two closed enumerations: `%Error.prototype%`, `%Function.prototype%`, `%Array.prototype.values%`, `entries`, `keys`, `forEach`; `@@toStringTag`, `@@unscopables`, `@@iterator` | H2, H3 | `webidl-bindings` |
| 10 | stated costs of a host function, a host object and a property | constants `HOST_FUNCTION_LIVE_BYTES`, `HOST_OBJECT_LIVE_BYTES`, `PROPERTY_LIVE_BYTES`, `BINDINGS_REALM_HEAP_BYTES` and a test of 300 host functions and 40 host objects; surf's test reads the constants | H3 | `webidl-bindings` |

Beside them, as the engine narrowed HOST-70 (HOST-188 to HOST-190), and slice 10 with surf's
requests (HOST-192, HOST-193, HOST-197 to HOST-200), which the first version does not need: no
promise exists before slice 10, so no microtask checkpoint is run; the note says so.

## Change

`docs/design/scripts.md` (new), and nothing else.

## Acceptance

1. `grep -c '^## Q[0-9]' docs/design/scripts.md` prints 18, and `grep -c '^Settled: '
   docs/design/scripts.md` prints 18; `grep '^Open: ' docs/design/scripts.md` prints no line
   that names `scripts-engine-dependency`, `scripts-spec` or `session-process-hardening`.
2. Q8's table has 214 member rows; counting its slice column over the 146 rows of the four
   interfaces gives 73 `reading`, 14 `attributes`, 29 `mutation`, 7 `selectors` and 23 `throws`
   (each count by a `grep -c` the section names, run by the reviewer).
3. The requirements table has 10 rows, each with its HOST ids and its status at the tag of Q13.
4. Q11 and Q12 each name the operator's answer or "taken on default a"; Q7 states the answer the
   blocker `decision-blocker:script-end-record` asks for.
5. Every measured value names its probe command and the output line it was read from; no probe
   file is in the change.
6. `slot task check` exits 0 on the story branch. The folder's bounds hold: the change adds no
   test, so the test step keeps its time within 120 s and no test runs over 60 s; no model call,
   no network request in a test; nothing is built per test; no Python is committed or run; no generator with a committed output is added; no build or test job is started for the note but its probes, each ending within 10 minutes or naming its time with `slot --timeout`.
7. `git diff --stat` of the story names only `docs/design/scripts.md`.

## Out of scope

Code, specification, the vendored IDL (`story:webidl-reader`), any change in swell's repository,
and the questions the epic puts after the first version (external, module, `async` and `defer`
scripts, timers, events, `document.write`).

## Why `epic:scripts-in-a-page`

It is the epic's first candidate story, and the one every other story of the epic but
`webidl-reader` waits on.
