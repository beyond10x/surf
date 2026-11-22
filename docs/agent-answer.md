# The agent answer

Every command of the agent command line answers in this format, reads included. It is line
text, printable ASCII only but for the body lines of the `text` block (section "Text"). The reader is a language model, and page text is untrusted input to
it, so no value from a page, a server or an address can start a line, end one, or look like a
field of the format. This lowers the risk of a page steering an agent; it does not remove it.

The crate `crates/answer` (`surf-answer`) is the one implementation: typed values for each
block, `render` from an answer to its lines, `quote` for every outside value, and the reverse,
`unquote` and `parse`. `surfd`, the live view and the scenario runner read an answer only
through `unquote` and `parse` and write no parser of their own (decisions D20 and D26 of
`docs/plan/agent-plan-revision-2.md`). The golden files under `crates/answer/tests/golden/` are
answers in this grammar, byte for byte.

This document states the blocks of step 1 of design section 9 (`docs/design/agent-interface.md`).
`now` and `changed` are defined by later stories (`agent-follow-links`, `agent-fill-pick`,
`agent-press-submit`, `agent-back-forward`, `agent-change-and-log`), each of which adds its block
here and in the crate.

## Example

```
surf 1 session 7f3a open
did go "http://example.org/login"
page 200 "text/html" "http://example.org/login" "Sign in"
path 2 of 2
  1 open
  2 go "http://example.org/login"
note parser: tables,foreign-content
```

## What an answer holds today

The example above shows the whole format. The binaries of release `0.3.0` (`surf-agent`,
`surfd`, `surf-session`) parse no page yet, so they print fewer lines:

- the head line, `surf 1 session <id> <state>`;
- `token <64 digits>`, in the answer of `open` only;
- `did <command>`, `did <command> "<argument>"` or `did refused <reason>`;
- `page <status> "<media type>" "<address>"` for a loaded page, `page failed <reason>
  "<address>"` for a failed load, or `page none` before the first load;
- `path <shown> of <total>` and its indented lines.

A loaded page shows no title, even when the document has one, and no `note parser:` line is
printed. Both arrive with the HTML parser (`story:html-document-port`). The two names the
section "Note" refers to, `surf_html::NOT_YET_PARSED` and `Browser::parser_not_yet_parsed`, do
not exist yet.

An answer of today's binaries, to `go`:

```
surf 1 session 7ffade4a-4d71-491c-a6c5-b69681d68d07 open
did go "http://example.org/missing"
page 404 "text/html" "http://example.org/missing"
path 3 of 3
  1 open
  2 go "http://example.org/"
  3 go "http://example.org/missing"
```

## Lines

- An answer is one or more lines. Every line ends in a line feed (U+000A), the last one too.
  No line is empty, and no line holds a carriage return.
- Every byte of a line is from U+0020 to U+007E. The one exception is a body line of the
  `text` block: it holds UTF-8 characters above U+007F too, but those rule 2 of section "Text"
  names.
- The first line is the head line. The blocks follow in this order, each at most once unless
  it says otherwise, each left out when the answer has nothing for it:
  1. `token`
  2. `did`
  3. `page`
  4. `path` and its indented lines
  5. `note` lines, any number
  6. `text`, its body lines and its `more` line, which is the last line of the answer
- A line is made of words of the format, numbers and quoted fields, separated by one space.
  Only a quoted field holds a value from a page, a server or an address. A word is checked
  against its character set below, and the renderer refuses an answer with a word outside it
  rather than write it.
- A number is decimal, without a sign and without a leading zero (`0`, `7`, `200`).

## Quoting

A value from a page, a server or an address is written as a quoted field:

- a double quote `"`, then the value, then a double quote;
- inside the quotes, `"`, `\` and every character outside U+0020 to U+007E are written as `\x`
  and two lower-case hexadecimal digits per byte of the character's UTF-8 encoding
  (`"` is `\x22`, `\` is `\x5c`, a line feed `\x0a`, U+2028 `\xe2\x80\xa8`);
- every other character is written as itself.

So a quoted field never holds a `"` other than its two quotes, and it ends at the first `"`
after the opening one.

A field with a length limit (a label of the `now` block, 60 characters, design section 5.2) is
cut before escaping: a value longer than the limit keeps its first characters up to the limit,
counted as Unicode scalar values, so the cut falls on a character boundary. The kept part is
quoted as above, and the field then ends in `...` after the closing quote:
`"Sign in to read the archive of everything this site has ever"...` (60 characters kept). The mark stands
outside the quotes so that a value ending in dots is not read as cut. A value within its limit
is written as a field without a limit is. No field of step 1 has a limit.

Reading a field back (`unquote`, and `unquote_cut` for a field with a limit) gives the exact
value, or for a cut field the kept part and the mark that it was cut. It refuses anything the
quoting function cannot have written: no opening quote, no closing quote, anything after the
closing quote, a `\` not followed by `x` and two hexadecimal digits, an upper-case
hexadecimal digit, a byte outside U+0020 to U+007E between the quotes, escaped bytes that are
not UTF-8, a character escaped that is written as itself (`\x41`), and a cut field whose kept
part is not exactly its limit or an uncut one longer than it.

## The blocks

### Head

```
surf 1 session <id> <state>
```

`1` is the format version. `<id>` is the session identity, a word of `[0-9a-z-]+` (a UUID in
text form, lower case). `<state>` is one of:

| state | meaning |
|---|---|
| `open` | the session is open |
| `closed` | the session was closed |
| `died` | the session process ended under `surfd` without `close` |
| `unknown` | an identity `surfd` does not list |

### Token

```
token <64 lower-case hexadecimal digits>
```

In the answer of `open` only. `surfd` does not keep it (decision D21).

### Did

```
did <command>
did <command> "<argument>"
did refused <reason>
```

The action the answer is for. `<command>` is a word of `[a-z]+` other than `refused`; the
argument, when the command was given one, is a quoted field (an address is one). Or the
refusal of the action, with a reason from this list:

| reason | when | exit status |
|---|---|---|
| `token` | the token does not belong to the session | 1 |
| `closed` | the session is closed | 1 |
| `usage` | the request is not one of the command | 2 |
| `no-such-session` | `surfd` lists no session of that identity | 3 |
| `session-died` | the session process ended without `close` | 3 |
| `session-timeout` | the session process did not answer in time | 3 |

A reason not in the list is refused by the parser. A story that adds a reason adds it here, to
`RefusalReason` and to a test.

### Page

```
page none
page <status> "<media type>" "<address>"
page <status> "<media type>" "<address>" "<title>"
page failed <reason> "<address>"
```

`page none` before the first load. A loaded page: its status, a number from 0 to 65535; its
media type, from the server; its address; and its title when it has one. A failed load: the
reason, a word of `[A-Za-z]+` (the variants of `surf.net.FailureReason`: `Unreachable`,
`UnsupportedScheme`, `MalformedResponse`, `TooManyRedirects` and `ForbiddenDestination`, a
redirect from a public address to a loopback or private one, as `docs/surf-command.md`, "Limits",
says), and the address. The media type, the
address and the title are quoted fields.

### Path

```
path <shown> of <total>
  <n> <command>
  <n> <command> "<argument>"
```

The last actions of the session, oldest first: `<shown>` of the `<total>` the session has taken,
`<shown>` not above `<total>`. Exactly `<shown>` lines follow, each indented by two spaces:
the action's sequence number, its command, a word of `[a-z]+`, and its argument as a quoted
field when it had one.

### Note

```
note parser: <parts>
note text: no-css
note text: no-document
note text: deep <n>
```

A `note` line says what the answer cannot show yet. The list is fixed, and a story that adds a
note adds it here:

- `note parser: <parts>`: the page was parsed by a tree builder with parts not implemented
  yet. `<parts>` is the names of those parts, as `surf_html::NOT_YET_PARSED` gives them
  (`html-document-port` exposes the list as `Browser::parser_not_yet_parsed`), joined by `,`
  without spaces, at least one. Each name is a word of `[a-z-]+`; the renderer refuses
  another (decision D31).
- `note text: no-css`: the `text` block is computed from the document's tree without CSS, so
  text a style sheet hides is shown, in source order.
- `note text: no-document`: page text was asked for, but the load failed or the page is not
  HTML; the answer has no `text` block.
- `note text: deep <n>`: the Markdown of the `text` block wrote up to `<n>` levels of nested
  lists and quotes, at least 1, at the indentation of the level above them and without their
  markers, because the prefix of their lines would pass 128 bytes
  (`surf_page_text::markdown::PREFIX_BYTES_MAX`). The answer has it when
  `surf_page_text::markdown::Markdown::deep` is not 0.

### Text

```
text <form> <shown> of <total> lines from <first>, <bytes shown> of <bytes total> bytes
>
> <line>
more <n> lines: look --as <form> --from-line <m>
```

Page text in bulk: the lines of one rendering of the page, `tree` (the outline of the document's
tree, `docs/design/page-as-text.md` section 5) or `markdown` (its content as Markdown, sections
3 and 4 of that note). Only in the answer to a call that asks for it (`--as`), after the `note`
lines. The rules are those of section 3 of that note; this section copies them, and the note is
the one place of rule 2.

- The header names the form, how many lines are shown of how many the rendering has, the
  number of the first line shown (counted from 1), and the bytes of the body lines shown of the
  bytes of all of them. Exactly `<shown>` body lines follow, each `>` alone (an empty line) or
  `> ` and the line. Bytes count a body line with its prefix and its line feed. No line surf
  writes for itself starts with `>`, so a line without it is surf's wherever it stands. With
  the prefix the block is a Markdown block quote.
- Characters (rule 2, decision D1 B). A body line holds printable ASCII and UTF-8 characters,
  but these are written as `\x` and two lower-case hexadecimal digits per UTF-8 byte: U+0000 to
  U+001F, U+007F to U+009F; U+00AD, U+034F, U+061C, U+115F to U+1160, U+17B4 to U+17B5, U+180B
  to U+180F, U+200B to U+200F, U+2028 to U+202E, U+2060 to U+206F, U+3164, U+FE00 to U+FE0F,
  U+FEFF, U+FFA0, U+FFF0 to U+FFFB, U+1BCA0 to U+1BCA3, U+1D173 to U+1D17A; U+E000 to U+F8FF
  and U+F0000 to U+10FFFF; U+40000 to U+EFFFF; U+FDD0 to U+FDEF and U+nFFFE, U+nFFFF of planes
  0 to 3; U+FF02, U+FF08, U+FF09, U+FF1C, U+FF1E, U+FF20, U+FF3B, U+FF3C, U+FF3D, U+FF5C. A
  combining mark of U+0300 to U+036F, U+1AB0 to U+1AFF, U+1DC0 to U+1DFF, U+20D0 to U+20FF or
  U+FE20 to U+FE2F is escaped when it starts a run of page text (the first code point after a
  character surf wrote: the `> ` prefix, a `[`, `(<`, a quote, a cell wall) and kept inside a
  run. U+FFFD is kept. `surf_answer::escaped_in_text` and `escaped_at_run_start` hold the lists.
  The renderer refuses a body line that holds a line feed or a character `escaped_in_text`
  names, or whose first character `escaped_at_run_start` names.
- Escape units. A unit is one character, a `\` with the character it escapes, or the `\x`
  escapes of one character. Every wrap, cut and `[cut]` falls between units, so no line ends in
  a bare `\` and `[cut]` is never escaped.
- What a body line holds: for `tree`, one line of the outline, its texts and attribute values
  written by `surf_answer::quote_text` or `quote_text_cut` (as `quote`, with the characters
  of rule 2; note section 5); for `markdown`, one line of Markdown, page text escaped by note
  sections 3 and 4.
- Bounds: at most 200 body lines and 32,768 bytes of them, whichever is met first. When lines
  are left, `more <n> lines: look --as <form> --from-line <m>` ends the block and the answer:
  `<m>` is `<first>` plus `<shown>`, `<n>` the lines from there to the end. `--from-line` counts
  the lines of the form asked for (it is not `--from <handle>` of `agent-answer-paging`). Past
  the end the block is `text <form> 0 of <total> lines from <m>, 0 of <bytes total> bytes`. A
  line that alone is over the byte bound is shown alone, cut at the last unit that leaves room
  for `[cut]`, and ends in `[cut]`.
- The version stays 1: no outside value starts a line, the block is only in an answer to a
  call that asks for it, after the `note` lines, and `more` is the last line. An answer to a
  call without `--as` is the same as before the block existed.

`surf_answer::text_page` makes one page of a rendering's lines, reading them one at a time;
`surf_answer::render_text_whole` writes every line behind `>` without the bounds and without
`more`, for `surf <url> --as`.

### Later blocks

`now` (the actionable elements, one line each) and `changed` (what the last action changed) are
defined by later stories. The `log` answer is defined by `agent-change-and-log`.

## Grammar

The notation is ABNF (RFC 5234), with one change: every string in double quotes matches
exactly as written, in its case (RFC 7405 writes this `%s"..."`). `SP` is U+0020, `LF` is
U+000A, `DQUOTE` is U+0022, `DIGIT` is `0` to `9`, `ALPHA` is `A` to `Z` and `a` to `z`.

```
answer        = head-line [token-line] [did-line] [page-line] [path-block] *note-line
                [text-block]

head-line     = "surf" SP "1" SP "session" SP session-id SP state LF
session-id    = 1*(DIGIT / lower / "-")
state         = "open" / "closed" / "died" / "unknown"

token-line    = "token" SP 64hex LF

did-line      = "did" SP (did-refused / did-action) LF
did-refused   = "refused" SP reason
did-action    = command [SP quoted]           ; command is not "refused"
reason        = "token" / "closed" / "usage" / "no-such-session"
              / "session-died" / "session-timeout"

page-line     = "page" SP (page-none / page-failed / page-loaded) LF
page-none     = "none"
page-failed   = "failed" SP failure SP quoted
page-loaded   = number SP quoted SP quoted [SP quoted]
                                              ; status 0 to 65535, media type, address, title
failure       = 1*ALPHA

path-block    = path-line *path-entry         ; exactly <shown> path-entry lines
path-line     = "path" SP number SP "of" SP number LF
                                              ; <shown> of <total>, <shown> not above <total>
path-entry    = SP SP number SP command [SP quoted] LF

note-line     = "note" SP (note-parser / note-text) LF
note-parser   = "parser:" SP part *("," part)
part          = 1*(lower / "-")
note-text     = "text:" SP ("no-css" / "no-document" / "deep" SP number)
                                              ; deep: at least 1

text-block    = text-line *body-line [more-line]
                                              ; exactly <shown> body-line lines; more-line
                                              ; when <first> + <shown> is not above <total>
text-line     = "text" SP form SP number SP "of" SP number SP "lines" SP "from" SP number ","
                SP number SP "of" SP number SP "bytes" LF
                                              ; <shown> of <total>, <first>,
                                              ; <bytes shown> of <bytes total>
form          = "tree" / "markdown"
body-line     = ">" [SP 1*text-char] LF
text-char     = %x20-7E / <a UTF-8 character above U+007F not named by rule 2>
more-line     = "more" SP number SP "lines:" SP "look" SP "--as" SP form SP "--from-line" SP
                number LF

command       = 1*lower
number        = "0" / (%x31-39 *DIGIT)
quoted        = DQUOTE *(plain / escaped) DQUOTE
cut-quoted    = quoted ["..."]                ; a field with a length limit
plain         = %x20-21 / %x23-5B / %x5D-7E   ; printable ASCII but " and \
escaped       = "\x" hex hex                  ; one byte of the UTF-8 encoding
hex           = DIGIT / %x61-66               ; lower case only
lower         = %x61-7A
```

The grammar is not all of it: an `escaped` sequence stands only for a character that `plain`
does not hold, and the bytes of a field's escapes are UTF-8 (section "Quoting"). In the `text`
block, `<shown>` is at most `<total>` and at most 200, `<bytes shown>` is the bytes of the body
lines (at most `<bytes total>` and 32,768), `<first>` is at least 1, the lines shown end at or
before `<total>`, and the first `text-char` of a body line is not a mark `escaped_at_run_start`
names (section "Text").

## Exit status and refusals

Design section 4: 0 the action was taken; 1 the action was refused (unknown handle, hidden or
disabled control, closed session, wrong token); 2 usage; 3 the session process cannot be reached
or stopped. The table of `did refused` gives the status of each reason.

A refusal still prints the answer: the head line, then `did refused <reason>`, then the current
state of the session as far as the one answering knows it (`page`, `path`, the notes).
A `token` refusal prints the head line and the `did` line only: a caller without the token
learns nothing of the session. So does a `usage` refusal of `open` (such as a second `open`
to the same session process) and of a request that is not in the format of the wire: neither carries
the session's token. The head line of these three has the state `unknown`, whatever the state of
the session, and the session process sends `surfd` no address and no title with them, so that
`surfd` keeps what it knew of the session.

## Parsing

`parse` reads lines back to the typed answer. It accepts exactly what `render` writes: parsing
what `render` wrote gives the value back, and rendering a parsed answer gives the same lines
byte for byte. It refuses, with the number of the line, an answer that does not start with its
head line, a line outside the grammar or out of the order of the blocks, a refusal reason not in
the list, and a quoted field `unquote` refuses. A text without a final line feed is refused at
its last line.

## Adding a block

A story that adds a block (`now`, `changed`, a note, a `log` answer) does all of these in one
change:

1. its lines here: the form in the block's section, its place in the order of section "Lines",
   and its rules in section "Grammar";
2. its typed value: a field of `Answer` in `crates/answer/src/lib.rs`, `Option` for a block that
   appears at most once, `Vec` for one that repeats, or a new variant of `Note`;
3. its rendering: a `render_<block>` function called from `render` in the block's place, every
   outside value through `quote` or `quote_cut`, every word checked against its set;
4. its parse: a `parse_<block>` function in the list `BLOCKS`, in the same place, that reads its
   lines when the next line is one of its own and leaves the lines alone otherwise, and accepts
   exactly what its rendering writes;
5. its tests: a golden file under `crates/answer/tests/golden/`, a golden test, a round-trip
   case, and every new outside field added to the forging matrix in `tests/forging.rs`.
