# Agent interface plan, wave 5 of 22: the entries of a session, and the first of the parser

One wave of the [wave plan](agent-waves.md). Three stories, each in its own worktree, each
implemented by a sub-agent on Opus; two small units beside them. The coordinating session wrote
no code. Every commit is the organization bot's.

After this wave a session process sends, with every answer, what it did for it (the action,
the events, the answer), and `surfd` keeps that in a bounded log for the live view. The HTML
parser decides a document's encoding and decodes it, and its tokenizer passes every markup
case of the vendored test data. The parser still builds no tree, and no command of `surf-agent`
prints anything it did not print before.

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `agent-session-events` | `crates/session/src/entries.rs`; the counted block of entries on the link and the rule for an answer entry in `crates/surfd/src/link.rs`; the log in `crates/surfd/src/log.rs`; the late answer in `crates/surfd/src/daemon.rs`; `docs/view-stream.md` | 15 cases in `crates/session/tests/events.rs`, 5 in `crates/surfd/tests/log.rs`, 3 written by the adversary; 5 planted defects, each red | `298d57f` |
| `html-input-stream` | `crates/html/src/encoding.rs`: the sniffing, the label table (228 labels, 40 encodings) and the decoders of UTF-8, UTF-16LE, UTF-16BE and windows-1252 | 13 tests; `tag encoding 75/82/82` (was 0 of 82); 4 planted defects, each red | `e9f2204` |
| `html-tokenizer-markup` | `crates/html/src/input.rs` and `crates/html/src/tokenizer/machine.rs`: the input stream and the states of tags, attributes, comments, DOCTYPE, processing instructions and CDATA sections | 8 unit tests; `tag markup 1979/1979/1979` (was 1), `tag text 57/328/328` (was 4), `tag charref 2053/4739/4739` (was 0); 4 planted defects, each red | `24df933` |
| the fixtures of the conformance check (before the wave's units, alone) | `crates/html-conformance/tests/check.rs`: the two fixture cases that are meant to fail now fail for any parser | 17 tests before and after, no assertion changed | `c73d0f5` |
| the crate documentation (after the HTML merges, alone) | the section "State" of `crates/html/src/lib.rs` | `cargo doc` with warnings denied; the crate's tests keep their counts | `9685efa` |

Each unit's claim was checked by the coordinator before its commit: the unit's packages were
tested again with warnings denied, and for the two HTML units the conformance task and
`--list-failing` were run again. The session unit was then given to an adversary on Opus, which
attacks a green unit and writes failing tests; its three tests are kept in the tree.

## The stories were measured before the wave

All three stories were read against `main` by a reviewer first, as in waves 3 and 4. None could
be met as written: 13 places in `agent-session-events`, 7 in `html-input-stream`, 5 in
`html-tokenizer-markup`. The bodies were revised and each story got a typed scope, so that
`aep plan artifact waves` puts the three in one wave with no file in two of them (`26a8b54`).

Decided before the wave:

- **The entries travel in a block of their own, before the answer, at most 8 MiB**
  (`link::ENTRIES_MAX`). Inside the 4 MiB limit of the answer text one `go` to a long address
  would not have fitted.
- **All 17 events of the model become entries**, the five of `surf.agent` included. A field is
  written as `spec/` names it; an absent optional field is `null`.
- **`encoding` is the name the sniffing chose, also when surf has no decoder for it.** The
  decoder used and the confidence are fields of their own. With that the measured 75 of 82
  holds for ISO-8859-2 and EUC-JP cases too.
- **The token types and the signature of `tokenize` stay as the conformance runner reads
  them.** The hook for CDATA outside the HTML namespace is left to `html-tree-foreign`.
- **Neither HTML unit writes `crates/html-conformance/tests/check.rs`.** Its fixtures failed
  only because the parser was empty, and both units would have turned six of its tests red in a
  file neither story lists. A unit of its own changed the fixtures first.

## What went wrong, and what was decided

- **The entries of an action answered late were lost.** `surfd` gave up on an answer after 60
  seconds and never read the answer, so the entries it carried did not reach the log. The
  adversary found it with a test. `surfd` now keeps the connection and reads the rest for at most 600
  seconds (`LATE_ANSWER_TIMEOUT`) while the turn stays taken. The client still gets
  `did refused session-timeout`. A session that does not answer in that time, or whose late
  answer breaks the link, is killed and listed `died`.
- **`decode` accepted an answer entry that held the token line**, or that was unlike the
  answer's own lines. A broken session process could have put a token into the log the live
  view serves. `link::check_answer_entry` now refuses both, in `encode` and in `decode`, and
  `surfd` treats it as a broken link. Found by the adversary with two tests.
- **A count above the expected one does not fail the check.** It prints a `raise:` line and
  passes; `AGENTS.md` said it fails, and both HTML stories relied on that. A unit could have
  left the expected files below what it measured, with a green gate. The stories now name the
  lines of the expected files and ask for a run without a `raise:` line (the gate's log on the
  wave branch has none), and `AGENTS.md` is corrected with this wave.
- **The crate documentation of `surf-html` said that every entry point returns an empty
  result.** Two units made that false in files neither owned whole. It was rewritten on the
  wave branch after both merges, from the source.
- **An input-stream error has no place in the standard's order of tokenizer errors.** The unit
  took the order from the test data: the error is reported when its character becomes the next
  input character, and a look-ahead does not report it.
- **`worktree finish --discard-cache` refuses a tree whose build output holds fixture
  repositories** (`target/tmp/check`). The trees were finished with `--archive` as well.

## The gate on the wave branch

`slot task check` in the runner's form (`CI=true GITHUB_ACTIONS=true CARGO_INCREMENTAL=0
RUSTC_WRAPPER=""`), exit 0, on `wave/a5-events-html` after the five units:

```
surf v1 — 9 file(s), 9 scenario(s), valid
83 scenario(s) (9 authored), 27 refusal(s), written to .out/suite.json
  83 scenarios: 83 passed, 0 failed, 0 error, 0 unsupported
the interpreter and the Rust system ran 83 scenario(s) each, of suite sha256:4d9ad0c31479f50662514ef890cfa8bf01b2eaee94669f5df38269e13feda726
generated/rust matches spec/ (30 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
Cargo.lock: 21 package(s), none from outside this repository
102 test suites: 407 passed, 0 failed, 0 ignored
html-conformance: 9107 cases, 9095 run, 4164 passed
```

Before the wave: 97 suites, 363 passed, and `html-conformance: 9107 cases, 9095 run, 5 passed`.
The scenarios, the refusals and the suite digest are unchanged: no unit of this wave changed
`spec/`.

## Findings, and where each goes

| finding | from | goes to |
|---|---|---|
| no test drives a late answer that never comes or that breaks the link: it needs a wait of the late timeout and a stand-in that answers after it | `agent-session-events` | backlog, with the stand-in session's next change |
| while a late answer is pending, a further request to that session waits for its turn and is refused after 60 seconds | `agent-session-events` | accepted: the session is busy, and the refusal says so |
| not attacked: an entry of `close` kept after the log ended the session (`crates/surfd/src/log.rs`), and the backlog of a subscriber between two reads | the adversary | `story:view-surfd-serves` (wave 6), the first reader of the log |
| `docs/view-stream.md` defines `dropped` as dropped "by the bound per session", and its own example counts the entries of a log the bound over all sessions dropped | the reviewer of `agent-session-events` | `story:view-surfd-serves`, which serves that field |
| encoding cases 48 to 54 of `encoding/tests1.dat` need the encoding changed while parsing | `html-input-stream` | `story:html-tree-body`; `encoding::decode_with` is its entry |
| a surrogate inside a name, a value, a comment or an identifier becomes U+FFFD, because those token fields are strings; no case covers it | `html-tokenizer-markup` | accepted, marked by a comment at the place |
| the RCDATA, RAWTEXT, script data and PLAINTEXT states return no token; `&` comes out as a character | `html-tokenizer-markup` | `story:html-tokenizer-text` (wave 6) and `story:html-character-references` (wave 7) |
| the planning store's transition lines carry a person's name as the actor, in a public tree | the coordinator | open: the store writes the actor it is given; a question for the operator, not changed in this wave |

## After this wave

Wave 6 of [the wave plan](agent-waves.md): `html-document-spec`, `html-tokenizer-text` and
`view-surfd-serves`, each read against `main` first.
