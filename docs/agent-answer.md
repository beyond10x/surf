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

This document states the blocks of step 1 of design section 9 (`docs/design/agent-interface.md`)
and the `now` block of its step 3 (`agent-follow-links`), which lists links, with the control
and button lines of its step 4 (`agent-fill-pick`), the `submits` part of a button line
(`agent-press-submit`), and of its step 5 (`agent-change-and-log`) the `changed` block and the
answer of `log`.

## Example

```
surf 1 session 7f3a open
did go "http://example.org/login"
page 200 "text/html" "http://example.org/login" "Sign in"
path 2 of 2
  1 open
  2 go "http://example.org/login"
changed page none -> "http://example.org/login"
  + text "Sign in to read the archive."
note parser: tables,foreign-content
```

## What an answer holds today

The example above shows the whole format. The binaries of release `0.7.0` (`surf-agent`,
`surfd`, `surf-session`) print fewer lines:

- the head line, `surf 1 session <id> <state>`;
- `token <64 digits>`, in the answer of `open` only;
- `did <command>`, `did <command> "<argument>"` or `did refused <reason>`;
- `page <status> "<media type>" "<address>" ["<title>"]` for a loaded page, with the title of
  an HTML document that has one, `page failed <reason> "<address>"` for a failed load, or
  `page none` before the first load;
- `path <shown> of <total>` and its indented lines, `<n> follow <handle> "<label>"` for a
  `follow`; every action in the answer of `log`;
- since `agent-cookie-jar`, `cookies <n>` in every answer that shows the session's state while
  its jar holds a cookie (section "Cookies");
- since `agent-change-and-log`, the `changed` block in every answer that shows the session's
  state (section "Changed");
- without `--as`, for an HTML page with links, the `now` block of its links and `note
  visibility: hidden attribute only` (section "Now");
- with `--as`, the `note text:` lines and the `text` block (section "Text").

Since `agent-follow-links` a loaded page shows its title; before it, none. A title longer than
1,024 characters is left out. No `note parser:` line is printed: `surf_html::NOT_YET_PARSED`
is empty, since the parser builds the tree in every insertion mode.

An answer of today's binaries, to `go`:

```
surf 1 session 7ffade4a-4d71-491c-a6c5-b69681d68d07 open
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
  5. `cookies`
  6. `changed` and its indented lines
  7. `now` and its indented lines
  8. `note` lines, any number
  9. `text`, its body lines and its `more` line, which is the last line of the answer
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

A field with a length limit (a label of the `now` block and of a `path` line, 60 characters,
design section 5.2; a target of the `now` block, 1,024 characters; the text of a `+ text` line of
the `changed` block, 120 characters, design section 5.3) is
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
| `unknown-handle` | the current document has no element of that handle, or there is none | 1 |
| `hidden` | the element or an ancestor has the `hidden` attribute | 1 |
| `unresolved` | the link's target does not resolve against the document base URL | 1 |
| `address-too-long` | the link's resolved target is longer than 65,536 bytes, the bound of an address `surf-fetch` follows | 1 |
| `unsupported-scheme` | the link's resolved target has a scheme a session does not follow: any but `http` and `https` (`javascript:`, `mailto:`, `data:`, `file:`, `ftp:`) | 1 |
| `forbidden-destination` | the page was loaded from a public address and the link's target is a local one: a loopback, private, link-local, unique-local, shared or unspecified IP address, an IPv6 address that embeds one, or the name `localhost` or one under it | 1 |
| `disabled` | the control, or the option `pick` names, is disabled | 1 |
| `wrong-kind` | the element is not of a kind the command acts on: `fill` takes `text`, `password` and `textarea`, `pick` takes `select`, `checkbox` and `radio` | 1 |
| `unknown-option` | the select has no option of that label or value, or the word is not `on` or `off` for a checkbox, or not `on` for a radio button | 1 |
| `readonly` | the control has the `readonly` attribute | 1 |
| `no-default-action` | the button is not a submit button (`type=button`, `type=reset`): without a script engine pressing it does nothing | 1 |
| `password-in-query` | the form submits with `GET` and a password control of it holds a value, which the address would carry | 1 |
| `no-form-owner` | the submission ended: the submit button belongs to no form | 1 |
| `invalid-controls` | the submission ended: a control of the form is `required` and empty | 1 |
| `invalid-action` | the submission ended: the action does not parse as an address | 1 |
| `unsupported-enctype` | the submission ended: the form `POST`s as `multipart/form-data` or `text/plain`, which surf does not encode | 1 |
| `dialog-method` | the submission ended: the form's method is `dialog` | 1 |
| `at-start` | `back`: the session's history has no page before the current one | 1 |
| `at-end` | `forward`: the session's history has no page after the current one | 1 |
| `post-entry` | `back` or `forward`: the page it would go to is the answer to a form `POST`, which surf does not send again | 1 |

`address-too-long`, `unsupported-scheme` and `forbidden-destination` are given for the address a
`press` would submit to as for a link's target. `back` and `forward` are refused
`forbidden-destination` for a page a `follow` or a `press` reached from a public page, or a
redirect led to after the load connected first to a public address, when its name now
resolves to a local address (section "Now", `back` and `forward`).

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
reason, a word of `[A-Za-z]+`, and the address. The media type, the address and the title are
quoted fields. The reason is a variant of `surf.net.FailureReason`:

| reason | when |
|---|---|
| `Unreachable` | no connection could be made, or it ended before a response was complete; for `https:` also a server that never finishes the handshake, a connection closed before the handshake ended or after it without `close_notify`, and an error of the socket |
| `UnsupportedScheme` | the address, or the `Location` of a redirect, has a scheme other than `http:` and `https:` |
| `MalformedResponse` | bytes arrived that are not an HTTP response |
| `TooManyRedirects` | a redirect arrived after 20 redirects of the same request were followed |
| `ForbiddenDestination` | a redirect led from a public address to a loopback or private one, as `docs/surf-command.md`, "Limits", says |
| `CertificateNotTrusted` | the server's certificate has no path to a trust anchor, or a certificate on its path is refused: unreadable, a signature that does not verify, an issuer that is no CA, a path too long, a limit of the path search, a key usage, an unknown critical extension, a name constraint, no `serverAuth`, a weak algorithm or a weak key |
| `CertificateExpired` | a certificate on the path is outside its validity, either side |
| `CertificateNameMismatch` | no `subjectAltName` entry of the server's certificate matches the host (no Common Name is read), or the host is one no certificate can match |
| `TlsVersionUnsupported` | the server offers at best TLS 1.2: it sent `protocol_version` before its ServerHello, or its ServerHello selects TLS 1.2 or below |
| `TlsHandshakeFailed` | the TLS connection failed, in the handshake or after it, with any other alert or violation of the protocol: after the handshake an alert the server sent or a record that does not verify (`bad_record_mac`); in it a server that sent `handshake_failure` before its ServerHello (a server limited to TLS 1.2 answers this way), a downgrade value in the ServerHello, a `supported_versions` other than TLS 1.3, an oversized handshake message |

surf does not check revocation: a revoked certificate that has not expired loads.

### Path

```
path <shown> of <total>
  <n> <command>
  <n> <command> "<argument>"
  <n> <command> <handle> "<label>"
```

The last actions of the session, oldest first: `<shown>` of the `<total>` the session has taken,
`<shown>` not above `<total>`. Exactly `<shown>` lines follow, each indented by two spaces:
the action's sequence number, its command, a word of `[a-z]+`, and its argument as a quoted
field when it had one. An action on an element names the element's handle, a word of
`[a-z0-9]+` written bare (the session made it), and, for `follow`, the link's label as the
`now` block writes it, cut at 60 characters: `3 follow l4 "Sign in"` (design section 5.1). A
`fill` or a `pick` names the handle alone, never the value typed or the option picked:
`4 fill f1`, `5 pick f3`; so does a `press`, `6 press b1`. `back` and `forward` have neither
argument nor handle: `7 back`, `8 forward`.

The block shows the last 5 actions; the answer of `log` shows more (section "Log").

### Cookies

```
cookies <n>
```

The cookies the session's jar holds when the answer is made, expired ones removed first: one
line directly after the `path` block, `<n>` from 1, and no line when the jar holds none. It
counts them and names none: no cookie's name or value is ever in an answer. A session keeps its
cookies in memory, for itself, until it ends (`crates/cookie`, `surf-cookie`).

```
path 2 of 2
  1 open
  2 go "http://127.0.0.1:8080/login"
cookies 1
```

### Changed

```
changed <n>
  ~ <handle> value "<before>" -> "<after>"
  ~ <handle> value (empty) -> (set)
  ~ <handle> checked <before> -> <after>
  ~ <handle> selected "<before>" -> "<after>"
changed page "<old address>" -> "<new address>"
  + text "<text>"
  + text more <n>
```

What the last action changed: the session compares the state its previous answer showed with
the state after the action (design section 5.3, `agent-change-and-log`). Every answer of the
session that shows the session's state has the block, reads (`look`, `log`) and refusals
included, `changed 0` when nothing changed. An answer that shows no state has none (decision
D291): the head line and `did` line of a `token` refusal, of a `usage` refusal of `open` and of
a request not in the format of the wire (section "Exit status and refusals"), the head line
alone of status 3, and the answers `surfd` gives
without the session process (`no-such-session`, `session-died`, `session-timeout`, and the
`did refused closed` of a session whose process has ended). Such an answer leaves the state the
next one compares with as it was.

- Same page, when the previous answer and this one show the page of the same load: `changed
  <n>` and one line per control of the current document whose state changed, in document order,
  exactly `<n>` lines. `value` for a `text`, `textarea`, `file` or `password` control, each
  value as the `now` block writes it: quoted and cut at 1,024 characters, for a password
  `(set)` or `(empty)` and never the value. `checked` for a checkbox or a radio button, `true` or
  `false`. `selected` for a select whose selected options changed, compared by their places in
  the select, not by their labels (decision D298): each side the labels of all its selected
  options in document order, joined by `, ` in the one quoted string and cut at 1,024
  characters, `""` when none was selected. A `select multiple` with Red and Green selected that
  a `pick` leaves with Red alone gives `~ f1 selected "Red, Green" -> "Red"`. A pick that moves
  the selection between two options of one label gives a line whose two sides read the same:
  the line says that the selection moved. A radio button turned on gives one
  line for each button of its group that changed: `~ f6 checked true -> false` and `~ f7 checked
  false -> true`. Values are compared whole, so a change past the first 1,024 characters gives a
  line whose two values read the same. Before scripts an action changes no element of a page,
  only the state of a control (decision D2 of the story), so the block holds no other line;
  elements that appear or go, and a label, target, visibility or text that changes within one
  page, wait for the script engine and the detailed mode (design sections 5.3 and 5.4).
- Another page, when a load happened since the previous answer (`open <url>`, `go`, a `follow`,
  a `press` that loaded a page, `back`, `forward`): `changed page`, the address of the previous
  answer's `page` line and of this one's, each a quoted field, or `none` for `page none`; a
  failed load counts with its address. Then one `+ text` line per text of the new page that the
  old one did not hold, in order: the texts are the lines of the page's Markdown rendering
  (`surf-page-text`, the lines of `look --as markdown`), empty lines left out, each once; a line
  the old page also had is not new. At most 10 lines, each text quoted and cut at 120
  characters (`surf_answer::quote_cut`); then `+ text more <n>` counts the new texts not shown.
  A page without a document (a failed load, a response that is not HTML) has no texts. A text
  is quoted as the Markdown line is, its own escapes included (rule 2 of section "Text"), so a
  `\` of the line is `\x5c` in the field.

A refusal changes nothing, so its block is `changed 0`; so is the block of a `forbidden-destination`
the fetch refused, which leaves the page as it was.

### Now

```
now <shown> of <total>
  <handle> link <visibility> "<label>" -> "<target>"
  <handle> link <visibility> "<label>" -> "<target>" unresolved
  <handle> <kind> <visibility> "<label>" [= <value>] [disabled] [checked] [required] [readonly]
  <handle> button <visibility> "<label>" [disabled] submits F<n> <method> ("<action>" / unresolved)
note visibility: hidden attribute only
```

The actionable elements of the current document, one line each, in document order (design
section 5.2): since `agent-follow-links` its links, since `agent-fill-pick` its controls and
buttons too. Only in an answer without page text (no `--as`), and only when the document has an
actionable element; `note visibility: hidden attribute only` follows it among the notes.
`<shown>` of the `<total>` elements are listed, the first ones: the lines stop before the block
passes 1,048,576 bytes (paging is `agent-answer-paging`).

- `<handle>`: `l<n>` for a link, `b<n>` for a button, `f<n>` for every other control, each
  letter numbered from 1 in document order over the actionable elements, hidden ones included,
  as the outline and the Markdown number them; a new document starts at 1 again, and the same
  pages give the same handles.
- `<kind>`: `link`, `button`, `text`, `password`, `checkbox`, `radio`, `select`, `textarea` or
  `file`. A `link` line has a target and no value; every other line has no target.
- `<visibility>`: `hid` when the element or an ancestor has the `hidden` attribute, `?`
  otherwise: visibility is not computed, and `aria-hidden`, a closed `dialog` or `details` and
  a popover leave a link `?`. `vis` is not written until visibility is computed.
- `<label>`: the accessible name, cut at 60 characters (section "Quoting").
- `<target>`: the `href` resolved against the document base URL (the first `base` element with
  an `href`, else the document's address), written as its path, query and fragment when it has
  the page's origin and whole otherwise; a path that starts with `//`, which would read as
  another host, is written whole too. An `href` that does not resolve is written as the page
  wrote it, followed by `unresolved`. Cut at 1,024 characters. A whole target never shows a
  userinfo (`user:password@`): scheme, host and port come first and stay whole when the target
  is cut, and `follow` still goes to the whole address, userinfo included.
- `= <value>`: the control's current value as a quoted field cut at 1,024 characters, for a
  `text`, `textarea` and `file` control (a file control's is always `""`); for a `password`
  never its value but `(set)` when it holds one and `(empty)` when it does not; for a `select`
  the label of its selected option, absent when none is selected. A checkbox, a radio button
  and a button have no value.
- the flags, each written when it holds, in this order: `disabled`; `checked`, a checked
  checkbox or radio button; `required`; `readonly`, which `fill` refuses.
- `submits F<n> <method> "<action>"`, after the flags, on the line of a button that has a form
  owner: what a `press` of it submits. `F<n>` is the form's handle, the forms of the document
  numbered from 1 in tree order (`surf_page::form_handles`); `<method>` is `get`, `post` or
  `dialog`, a word of `[a-z]+`; `<action>` is the address it submits to, written as a link's
  `<target>` is, cut at 1,024 characters. The method and the action are the button's own
  `formmethod` and `formaction` when a submit button has them, else the form's `method` and
  `action`; an empty action is the document's address, and a method other than `post` or
  `dialog` is `get`. An action that does not resolve is written as `unresolved` alone, with
  none of its text, which could hold a user name and a password. A button without a form owner
  has no `submits` part.

`fill <handle> <value>` gives a `text`, `password` or `textarea` control the value, a `text` or
`password` control without its line feeds and carriage returns (the HTML Standard's value
sanitization of a single-line `input`), a `textarea` with them; `pick
<handle> <option>` selects the option of a `select` whose label is the word, else the first
whose value is, turns a `checkbox` `on` or `off`, or a `radio` button `on`, which turns the
others of its group off: the radio buttons of the page with its name and its form, or with its
name and no form. A `pick` that asks for the state a checkbox already has records no
action and answers `did pick` with status 0. Each is refused, with status 1 and nothing changed:
`unknown-handle`, then `wrong-kind`, `disabled`, `hidden` (the `hidden` attribute, or what else
hides the control: `aria-hidden`, a closed `dialog` or `details`, a popover), `readonly` for
`fill`, and `unknown-option`, or the picked option's `disabled`, for `pick`. The `did` line
names the handle as typed, `did fill "f1"`. `surf-agent fill <handle> -` reads the value from
standard input; a value of up to 16,000 bytes always fits its request of 65,536 bytes, where a
byte outside printable ASCII takes 4. `--` ends the options: `fill f1 -- -5` fills `-5`.

`follow <handle>` takes a handle of this block. It is refused `unknown-handle` for a handle the
current document has not, `hidden` for a `hid` link and `unresolved` for an unresolved one;
and, before any navigation and with the page left as it was, `address-too-long`,
`unsupported-scheme` or `forbidden-destination` for a target the table of `did refused` names.

`press <handle>` takes a `button` of this block and submits its form; the answer shows the page
the server answered with, and the `did` line names the handle as typed, `did press "b1"`. Before
any submission starts, with the page left as it was and no request sent, it is refused, status
1: `unknown-handle`, `wrong-kind` for any other kind, `disabled`, `hidden`; `no-default-action`
for a button that is not a submit button; `password-in-query` for a `GET` form whose password
control holds a value; and `address-too-long`, `unsupported-scheme` or `forbidden-destination`
for the address it would submit to: the action, and for `GET` the action with its query
replaced by the form's entries. A submission that starts and then ends without a request is
refused with the word of its reason, the `press` in the path: `no-form-owner`,
`invalid-controls`, `invalid-action`, `unsupported-enctype`, `dialog-method`.

`back` and `forward` take no argument and move the session one page through its history:
`did back`, `did forward`, and in the path `5 back`, `6 forward`. The answer shows the page
the step loaded, fetched again with `GET`, or `page failed` when the load failed, which leaves
the session's place in its history where it was; the handles start at 1 again. Before any
request, with the page left as it was, each is refused, status 1: `at-start` for `back` on the
first page, `at-end` for `forward` on the last, and `post-entry` for a page that is the answer
to a form `POST`. A page a `POST` was redirected to (301, 302, 303) was fetched with `GET` and
is gone back to like any other. A `go`, `follow` or `press` after `back` that loads a page
drops the pages after the current one; one that fails or is refused drops none.

The session's own checks of a target (`address-too-long`, `unsupported-scheme`, the address
text of `forbidden-destination`) are not made again: a page of the history was loaded once, at
an address that passed them. What a name resolves to can change, though. A page whose address
a public page or a public server chose is fetched again under the rule of a page loaded from a
public address: one a `follow` or a `press` reached from such a page, and one a redirect led to
after the load connected first to a public address, whichever command started the load. Where
the load connected decides, not the text of the address: a name that resolved to a local
address counts as local. When its name now resolves to a local address, `back` or `forward` is
refused `forbidden-destination`, status 1, before any connection, with the page and the path
as they were. A page the agent's own address reached without a redirect, or by a redirect
after a first connection to a local address, is fetched again as `go` fetches it, from
whatever page the session is on.

The answer of a closed session has no `now` block.

### Note

```
note parser: <parts>
note visibility: hidden attribute only
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
- `note visibility: hidden attribute only`: the `now` block says `hid` for an element the
  `hidden` attribute hides and `?` for every other (section "Now"). With the `now` block only.
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

### Log

`log [<n>]` is a read: it records no action, and its answer has no `did` line. It is the answer
of `look` with a longer `path` block: every action of the session, oldest first, or the last
`<n>` of them (a number from 1; on the wire `arg count`), `path <shown> of <total>` as before.
The lines stop before the block passes 1,048,576 bytes, the newest kept, so `<shown>` can be
below what was asked. `log 0`, a word that is not a number and any other argument are refused
`usage`, status 2.

```
surf 1 session 7f3a open
page 200 "text/html" "http://example.org/" "Home"
path 7 of 7
  1 open
  2 go "http://example.org/"
  3 follow l1 "Next"
  4 back
  5 forward
  6 go "http://example.org/"
  7 follow l1 "Next"
changed 0
```

## Grammar

The notation is ABNF (RFC 5234), with one change: every string in double quotes matches
exactly as written, in its case (RFC 7405 writes this `%s"..."`). `SP` is U+0020, `LF` is
U+000A, `DQUOTE` is U+0022, `DIGIT` is `0` to `9`, `ALPHA` is `A` to `Z` and `a` to `z`.

```
answer        = head-line [token-line] [did-line] [page-line] [path-block] [cookies-line]
                [changed-block] [now-block] *note-line [text-block]

head-line     = "surf" SP "1" SP "session" SP session-id SP state LF
session-id    = 1*(DIGIT / lower / "-")
state         = "open" / "closed" / "died" / "unknown"

token-line    = "token" SP 64hex LF

did-line      = "did" SP (did-refused / did-action) LF
did-refused   = "refused" SP reason
did-action    = command [SP quoted]           ; command is not "refused"
reason        = "token" / "closed" / "usage" / "no-such-session"
              / "session-died" / "session-timeout" / "unknown-handle" / "hidden"
              / "unresolved" / "address-too-long" / "unsupported-scheme"
              / "forbidden-destination" / "disabled" / "wrong-kind"
              / "unknown-option" / "readonly" / "no-default-action"
              / "password-in-query" / "no-form-owner" / "invalid-controls"
              / "invalid-action" / "unsupported-enctype" / "dialog-method"
              / "at-start" / "at-end" / "post-entry"

page-line     = "page" SP (page-none / page-failed / page-loaded) LF
page-none     = "none"
page-failed   = "failed" SP failure SP quoted
page-loaded   = number SP quoted SP quoted [SP quoted]
                                              ; status 0 to 65535, media type, address, title
failure       = 1*ALPHA

path-block    = path-line *path-entry         ; exactly <shown> path-entry lines
path-line     = "path" SP number SP "of" SP number LF
                                              ; <shown> of <total>, <shown> not above <total>
path-entry    = SP SP number SP command [SP quoted / SP handle [SP cut-quoted]] LF
                                              ; with a handle: a label of at most 60
handle        = 1*(lower / DIGIT)

cookies-line  = "cookies" SP number LF        ; number from 1

changed-block = changed-line *change-entry    ; exactly <n> change-entry lines
              / changed-page *text-entry [text-more]
                                              ; at most 10 text-entry lines; text-more only
                                              ; after 10 of them
changed-line  = "changed" SP number LF        ; <n>
change-entry  = SP SP "~" SP handle SP (change-value / change-checked / change-selected) LF
change-value  = "value" SP value SP "->" SP value
value         = cut-quoted / "(set)" / "(empty)"
                                              ; at most 1,024
change-checked = "checked" SP boolean SP "->" SP boolean
boolean       = "true" / "false"
change-selected = "selected" SP cut-quoted SP "->" SP cut-quoted
                                              ; each at most 1,024
changed-page  = "changed" SP "page" SP address SP "->" SP address LF
address       = quoted / "none"
text-entry    = SP SP "+" SP "text" SP cut-quoted LF
                                              ; at most 120
text-more     = SP SP "+" SP "text" SP "more" SP number LF
                                              ; number at least 1

now-block     = now-line *now-entry           ; exactly <shown> now-entry lines
now-line      = "now" SP number SP "of" SP number LF
                                              ; <shown> of <total>, <shown> not above <total>
now-entry     = link-entry / control-entry
link-entry    = SP SP handle SP "link" SP visibility SP cut-quoted SP "->" SP cut-quoted
                [SP "unresolved"] LF          ; label at most 60, target at most 1,024
control-entry = SP SP handle SP kind SP visibility SP cut-quoted
                [SP "=" SP (cut-quoted / "(set)" / "(empty)")]
                [SP "disabled"] [SP "checked"] [SP "required"] [SP "readonly"]
                [SP submits] LF
                                              ; kind not "link", value at most 1,024
submits       = "submits" SP form-handle SP command SP (cut-quoted / "unresolved")
                                              ; method, then action at most 1,024
form-handle   = "F" %x31-39 *DIGIT
kind          = 1*lower
visibility    = "vis" / "hid" / "?"

note-line     = "note" SP (note-parser / note-visibility / note-text) LF
note-parser   = "parser:" SP part *("," part)
part          = 1*(lower / "-")
note-visibility = "visibility:" SP "hidden attribute only"
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
state of the session as far as the one answering knows it (`page`, `path`, `changed`, the
notes).
A `token` refusal prints the head line and the `did` line only: a caller without the token
learns nothing of the session. So does a `usage` refusal of `open` (such as a second `open`
to the same session process) and of a request that is not in the format of the wire: neither carries
the session's token. The head line of these three has the state `unknown`, whatever the state of
the session, and the session process sends `surfd` no address and no title with them, so that
`surfd` keeps what it knew of the session.

### Extra trust anchors: `open --ca-file`

`surf-agent open [<url>] --ca-file <path>` gives the session the CA certificates of one PEM file
as trust anchors beside the bundled ones, for every `https:` load of the session and for its
life (`docs/design/tls.md` item 8); a session opened without it trusts the bundled anchors only,
and the anchors of one session are never those of another. The client reads the file before it
connects to `surfd`: at most 32,768 bytes and 64 blocks, every block a `CERTIFICATE`, every
certificate a CA. The text of its certificate blocks, from each BEGIN line to its END line,
travels in the `open` request (`arg anchors`); the lines before, between and after the blocks
are not sent, and need not be UTF-8. `surfd` passes the request to the session process over the
session socket; neither the path nor the blocks are an argument or in the environment of a
process.

A file the client refuses is an invalid input: exit status 2, nothing on standard output, and on
standard error one of these lines, followed by the usage:

- `surf-agent: the --ca-file file cannot be read: <kind>`: no file at the path, or one the caller
  may not read; `<kind>` is the kind of the error of the system, such as `entity not found`.
- `surf-agent: the --ca-file file is not a regular file`: a directory, a device or a FIFO. The
  path is looked at before it is opened, so a FIFO is refused and not waited on.
- `surf-agent: the --ca-file file changed while it was read`: the file opened is not the file
  looked at.
- `surf-agent: the --ca-file file is refused: <error>`: `surf-tls` does not take the file.
  `<error>` is the `AnchorError` of `surf-tls` (`NoCertificate`, `NotACertificate`, `BadPem`,
  `BadCertificate`, `NotACa`, `TooLarge`, `TooMany`) by its kind and the index of the block at
  fault, such as `NotACa: certificate 0 is not a CA`, `NotACertificate: block 0 is not a
  CERTIFICATE` or `BadCertificate: certificate 0 is refused: UnexpectedTag` (the kind the
  certificate reader names).
- `surf-agent: the request is longer than 65536 bytes`: `surf-tls` takes the file, but the
  quoted text of its blocks does not fit an `open` request (`docs/design/tls.md` item 8). The
  quoting writes a line feed, a carriage return or a tab as 4 bytes, so only blocks full of them
  get here, such as copies of a root with one base64 character per line; lines outside the
  blocks are not sent and never lead here.

No line repeats the path or a byte of the file: not the label of a block, and not a byte of a
certificate the reader cannot read. No request is sent, `surfd` is not started, and no session
is made. A session process that gets anchors it refuses in an `open` request (a client other
than `surf-agent`) answers `did refused usage`, status 2, as for any `usage` refusal of `open`.

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
