# The live view's routes and event stream

What `surfd` answers on its view port: the routes, the status codes, the headers, the JSON of each
route and the server-sent events of `/events`. `story:view-http-server` and
`story:view-surfd-serves` are held to this document byte for byte; each fenced example below is
meant to be copied into a golden file. The decisions behind it, and the measurements they rest
on, are in `docs/design/live-view.md`.

Written 2026-11-21 by `story:view-decisions`, before any of it is implemented. It follows the
decisions of `docs/plan/agent-plan-revision-1.md` (D17, D18) and
`docs/plan/agent-plan-revision-2.md` (D19 to D24, D26) in their words where it repeats them.
Standards are cited from memory: RFC 9110 and RFC 9112 (HTTP), the HTML Standard's server-sent
events section, Content Security Policy Level 3, Referrer Policy. surf claims no section of them
complete.

## Rules over every answer

- `surfd` listens on 127.0.0.1 only, on a port the operating system picks (item 6 of the design
  note). In this document the port is 41873.
- HTTP/1.1, `GET` only. One request per connection; every answer carries `Connection: close`, the
  stream included. The stream's body ends when the connection closes (RFC 9112, message body
  length, from memory), so it has no `Content-Length` and no chunked coding.
- Every line of an answer's head ends in CR LF and the head ends with an empty line. The examples
  show each line without its CR. A request's head is read the same way: a line that ends in a
  bare LF is answered 400.
- The head of every answer holds these lines, in this order, and no other:
  1. the status line, `HTTP/1.1 <code> <reason>`, with the reason of the table below;
  2. `Content-Type`;
  3. `Content-Length`, the body's length in bytes (absent on the stream only);
  4. `Allow: GET` on a 405 only; `WWW-Authenticate: Bearer` on a 401 only (RFC 9110 requires
     the challenge on every 401);
  5. the five security lines, exactly:

     ```text
     Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'
     X-Content-Type-Options: nosniff
     Referrer-Policy: no-referrer
     Cache-Control: no-store
     Cross-Origin-Resource-Policy: same-origin
     ```

  6. `Date: <IMF-fixdate>`, the time the answer was written (RFC 9110 requires it of a server
     with a clock). A test compares every byte of an answer except the 29 bytes of this value;
  7. `Connection: close`.
- No answer carries an `Access-Control-` line, a `Set-Cookie` line or a `Server` line.
- The token: every route under `/api/` and `/events` requires the view token, sent as exactly
  `Authorization: Bearer <64 hexadecimal digits, lower case>`, compared by `surf-secret`'s
  comparison in constant time. The token is never read from the query, a cookie or another
  header. The page's own files are served without it: they hold no data (D18).
- No session's token is in any answer or any event (D21): the answer entry of `open` omits the
  `token` line, and no JSON shape below has a member for it.

## The route table

A request's target is matched byte for byte against these paths: no percent-decoding, no `.` or
`..` resolution, and no route takes a query, so a target holding `?` matches none.

| path | token | answers | content type of the 200 |
|---|---|---|---|
| `/` | no | 200 | `text/html; charset=utf-8` |
| each page file, `/<name>.js` and `/<name>.css` | no | 200 | `text/javascript; charset=utf-8`, `text/css; charset=utf-8` |
| `/api/sessions` | yes | 200, 401 | `application/json` |
| `/api/sessions/<id>` | yes | 200, 401, 404 | `application/json` |
| `/events` | yes | 200, 401, 503 | `text/event-stream` |
| any other target | | 404 | |

- The page files are the list `crates/view/src/assets.rs` holds (`story:view-first-page`): `/`
  answers `index.html`; `/view.css`, `/view.js` and the other modules answer under their own
  names. `/index.html` is not a route.
- `<id>` is a session identity in its text form: 36 characters, lower-case hexadecimal digits in
  groups of 8, 4, 4, 4 and 12 joined by `-`. A target with anything else there matches no route.

## Status codes and their order

Every refusal has a JSON body `{"error":"<word>"}` with `Content-Type: application/json`, and
names no session.

| code | reason | word | when |
|---|---|---|---|
| 200 | `OK` | | the route answers |
| 400 | `Bad Request` | `bad-request` | the request is not well-formed (below) |
| 401 | `Unauthorized` | `unauthorized` | a route that needs the token and the token is missing, malformed or wrong |
| 403 | `Forbidden` | `host` | `Host` is not `127.0.0.1:<port>` |
| 404 | `Not Found` | `not-found` | no route matches, or `<id>` names no listed session |
| 405 | `Method Not Allowed` | `method` | any method but `GET` |
| 431 | `Request Header Fields Too Large` | `too-large` | the request line and headers are over 8 KiB or 50 lines |
| 503 | `Service Unavailable` | `busy` | past the connection limit, or past the stream limit on `/events` |
| 505 | `HTTP Version Not Supported` | `version` | a version `HTTP/<digit>.<digit>` other than `HTTP/1.1` |

The checks run in this order; the first that fails answers, and nothing after it runs:

1. Connection limit: past 16 open connections, the new one is answered 503 without its request
   being read, then closed.
2. Receive limit: a request line and headers not complete within 5 s close the connection with
   no answer.
3. Size: request line and headers over 8,192 bytes (from the first byte through the empty line)
   or over 50 header lines: 431, then the connection closes.
4. Syntax: the request line is not `<method> SP <target> SP <version>`: 400. The version is
   `HTTP/<digit>.<digit>` and not `HTTP/1.1`: 505; any other version text: 400. A header line
   without `:`, or starting with a space or a tab: 400. A target not starting with `/` (absolute
   form, `*`): 400. `Content-Length` or `Transfer-Encoding` present: 400. `Host` absent or given
   twice: 400. `Authorization` given twice: 400.
5. Method: anything but `GET`, `HEAD` included: 405 with `Allow: GET`. A deliberate deviation:
   RFC 9110 asks a general-purpose server to support `HEAD`; this one serves one page to one
   reader and supports `GET` alone.
6. `Host`: not exactly `127.0.0.1:<port>` (`localhost`, another port, any name): 403. This is
   the guard against DNS rebinding.
7. Route: no route matches the target: 404. The table above is public, so a 404 before the
   token says nothing a reader of this document does not know.
8. Token, on `/api/` and `/events`: 401 with `WWW-Authenticate: Bearer`.
9. Session: `/api/sessions/<id>` naming no listed session: 404.
10. Stream limit: past 8 open streams, `/events` answers 503.

Other limits, each a field of the server's limits value so a test can shorten it: an answer or
one stream write not finished within 10 s closes the connection; per stream at most 256 queued
entries or 1 MiB, past which that stream is closed (below).

## The JSON

### Writing

- No whitespace outside strings. Members in the order this document gives them. Numbers in
  decimal without a sign, a fraction or an exponent. Absent values are `null`, never left out.
- A string is the exact value: an address, a title, an action's argument and an event field
  arrive quoted by `surf-answer` and are read through `surf_answer::unquote` before they are
  written (D26); no `\x` escape of the link is left in them. Then these are written as `\u` and
  four lower-case hexadecimal digits: `"`, `\`, every character below U+0020, U+007F to U+009F,
  U+2028, U+2029, `<`, `>` and `&`. Every other character is written as itself in UTF-8. So no
  line feed, carriage return, line or paragraph separator ever breaks a `data:` line, and no
  body holds a raw `<`, `>` or `&`.
- A value that `surf_answer::unquote` or `surf_answer::parse` refuses is written as `null`, the
  entry is kept, and the session detail counts it in `unreadable`.
- Times are milliseconds since the Unix epoch, as `surfd` stamped them.

The string rule on one value, a title holding `a"b\c<d>&e`, a line feed, U+2028 and `é`:

```json
"a\u0022b\u005cc\u003cd\u003e\u0026e\u000a\u2028é"
```

### The page line

The `page` line of an answer, as `surf_answer::parse` reads it (`docs/agent-answer.md`), in one
of three forms:

- `{"kind":"none"}` for `page none`;
- `{"kind":"loaded","status":<n>,"media_type":"<text>","address":"<text>","title":"<text>"}`,
  `title` `null` when the answer has none;
- `{"kind":"failed","reason":"<reason>","address":"<text>"}`.

Since `story:agent-follow-links` the session writes a loaded page's title on its page line (at
most 1,024 characters; a longer title is left out and `title` is `null`), and the session
summary's `title` is the same title. The page line of the three-link page of the page tests'
fixture (`crates/view-fixture/pages/now/links.html`), served on 127.0.0.1:41000:

```json
{"kind":"loaded","status":200,"media_type":"text/html","address":"http://127.0.0.1:41000/now/links.html","title":"Three links"}
```

### A session summary

`{"id","state","address","title","opened_at","last_action_at","ended_at","page"}`:

- `id`: the session identity;
- `state`: `open`, `closed` or `died`;
- `address`, `title`: as the session last reported them to `surfd`, or `null`;
- `opened_at`; `last_action_at`, the `kept_at` of its newest action entry, or `null`; `ended_at`,
  or `null` while it is `open`;
- `page`: the page line of its newest kept answer entry, or `null` when none is kept.

### The session list, the 200 of `/api/sessions`

`{"newest":<n>,"sessions":[<summary>,…]}`: `newest` is the highest number across sessions kept
so far (0 before the first entry), and `sessions` every session `surfd` lists, oldest `opened_at`
first.

### The session detail, the 200 of `/api/sessions/<id>`

`{"session":<summary>,"dropped":<n>,"log_dropped":<bool>,"unreadable":<n>,"entries":[<entry>,…]}`:

- `dropped`: entries of this session dropped by any bound: one by one, the oldest first, and
  every kept one when its log was dropped whole;
- `log_dropped`: `true` when the bound over all sessions dropped this session's log whole
  (D24); `entries` is then empty;
- `entries`: the kept entries in their order (below).

### The entries

The order (D19): for each action, the action entry, then the events of that action in the order
the components published them, then the answer entry. An entry belongs to the action entry it
follows. A command that records no action (`look`, `log`, a refused command) has no entry. An
action whose answer has not arrived has no entry yet: its entries arrive with its answer, a late
one included (`LATE_ANSWER_TIMEOUT`, 600 s), with the time `surfd` kept them; a session killed
before it answered ends `died`, and that action has no entry.

Every entry starts with the same five members:
`{"number","session","index","kept_at","kind",…}`: `number` is the entry's number across all
sessions (D23), from 1 for each run of `surfd`, the `id:` of the stream; `session` the session
identity; `index` its number within the session, from 1; `kept_at` the time `surfd` kept it;
`kind` one of the three below.

- `action`: then `"sequence"` (the action's sequence number), `"command"` and `"argument"` (the
  exact text, or `null` when the action has none).
- `event`: then `"name"`, the qualified event name of `spec/`; `"unmapped"`, `true` when the
  session process does not map this event yet (`story:agent-session-events`), its `fields` then
  `{}`; and `"fields"`, an object with every field of the event by its `spec/` name, in the order
  the `ess/1` document declares them. Each value is a string: the exact value
  `surf_answer::unquote` reads from the quoted text the log keeps (D26). A field of bytes is the
  text `<n> bytes`, never the bytes (D22).
- `answer`: then `"page"`, the page line, `"path"`,
  `{"shown":<n>,"total":<n>,"actions":[{"sequence","command","handle","argument","argument_cut"},…]}`,
  `"now"` and `"changed"` (below), each as `surf_answer::parse` reads it from the answer's lines
  (D20), or `null` when the answer has none. In a `path` action, `handle` is the handle of the
  element the action took (`follow l1`, `fill f2`), `null` for an action on no element;
  `argument` is the address, or with a handle the element's label, cut at 60 characters, and
  `argument_cut` is `true` when the label was cut there (the answer writes `...` after it); a
  `fill`, a `pick` and a `press` have the handle and no argument. `changed` was added after `now`
  by `story:view-controls-and-changes`; no member is removed or renamed.

### The `now` member

The `now` block of an answer (`docs/agent-answer.md`, "Now"): what can be done on the current
document, the first elements in document order. `null` when the answer has no `now` block: before
the first page, when the session has no current HTML document or it has no link, control or
button, for a closed session, and in an answer with page text (`--as`). A refused `follow`,
`fill`, `pick` or `press` records no action, so it adds no entry.

`{"shown":<n>,"total":<n>,"elements":[<element>,…],"visibility_note":"<text>"}`:

- `shown`, `total`: the block's head, `now <shown> of <total>`; the session lists the first
  `<shown>` of the document's `<total>` elements, the block within 1,048,576 bytes;
- `elements`: one per line of the block, in its order;
- `visibility_note`: `"hidden attribute only"` when the answer holds `note visibility: hidden
  attribute only`, otherwise `null`. It says that the visibility words are not computed: `hid`
  when the element or an ancestor has the `hidden` attribute, `?` otherwise; `aria-hidden`, a
  closed `dialog` or `details` and a popover leave an element `?`.

An element,
`{"handle","kind","visibility","label","label_cut","target","target_cut","unresolved","text","state"}`:

- `handle`: `l<n>` for a link, `b<n>` for a button, `f<n>` for every other control, numbered
  over the document's actionable elements;
- `kind`: the kind word of the line: `link`, `button`, or the control's kind (`text`,
  `password`, `checkbox`, `radio`, `select`, `textarea`, `file`);
- `visibility`: the answer's word, `vis`, `hid` or `?`;
- `label`: the accessible name cut at 60 characters, with `label_cut`;
- of a link: `target`, the address resolved against the document base URL, written as its path,
  query and fragment when it has the page's origin and whole otherwise, or, when it does not
  resolve, the `href` as the page wrote it with `unresolved` `true`; cut at 1,024 characters,
  with `target_cut`. `text` and `state` are `null`;
- of a control or a button (`story:view-controls-and-changes`): `target`, `target_cut` and
  `unresolved` are `null`; `text` is the line as the answer writes it, without its indentation,
  its quoted fields still quoted; and `state` is what follows the label on the line,
  `{"value","disabled","checked","required","readonly","submits"}`:
  - `value`: the `= <value>` of the line as the answer prints it, `{"kind","text","cut"}`: a
    quoted value (a text control's value, the label of a select's selected option) is
    `{"kind":"text","text":"<value>","cut":<bool>}`, cut at 1,024 characters; a password is
    `{"kind":"set","text":null,"cut":null}` or `{"kind":"empty","text":null,"cut":null}`, as the
    answer prints `(set)` and `(empty)`, never its value; `null` for a checkbox, a radio button,
    a button and a select with no option selected;
  - `disabled`, `checked` (a checkbox or a radio button), `required`, `readonly`: `true` when the
    line writes the flag;
  - `submits`: of a submit button with a form owner, `{"form","method","action","action_cut"}`:
    the form's handle (`F1`), the method word (`get`, `post`, `dialog`), and the action shown as
    a link's target is, cut at 1,024 characters; `action` and `action_cut` are `null` for
    `unresolved`. `null` for every other control.

The view writes every one of these values as text (a label, a target, a value and an action
come from the page); no target or action is a link the page follows. The stream carries each
value unchanged; the page shows a bidirectional control in it (U+202A to U+202E, U+2066 to
U+2069) as a mark that names it (`U+202E`), so a value cannot reorder what a person reads, and
does so in every text that came from a session: a label, a target, a value, a title, an
address. Other format characters (U+200E, U+200F, joiners, a soft hyphen) are shown as they are.

### The `changed` member

The `changed` block of an answer (`docs/agent-answer.md`, "Changed"): what the action changed,
from the state the session's previous answer showed. `null` when the answer has none (an answer
that shows no state, and the entries of a run before `story:agent-change-and-log`). One of two
shapes, by `kind`:

- `{"kind":"controls","lines":[<line>,…]}`: `changed <n>` and its `~` lines, the same page;
  `lines` is empty for `changed 0`. A line, `{"handle","what","before","after"}`: the control's
  handle; `what` is `value`, `checked` or `selected`; for `value`, `before` and `after` are each
  a value as the `now` member writes it (a quoted value, or a password's `set` or `empty`, never
  its value); for `checked`, `true` or `false`; for `selected`, a value of kind `text`, the labels
  of the selected options joined by `, `, `""` when none was selected;
- `{"kind":"page","from","to","texts":[{"text","cut"},…],"more":<n>}`: `changed page <old> ->
  <new>`, another page; `from` and `to` the addresses of the previous answer's page and this
  one's, `null` for `none`; `texts` the `+ text` lines, each the text of the new page cut at 120
  characters with `cut`; `more` the count of `+ text more <n>`, 0 without that line.

The answer has no line for an element that appears or goes (`+` and `-` lines wait for the
script engine and the detailed mode, `docs/agent-answer.md`), so neither has this member.

What the view holds of a value, in the words of D22 and D21: a field of bytes (a response body, a
request body) is written by the session process as its length, `<n> bytes`, never as the bytes.
"A password's value is its length, as the answer prints it. Every other control value is kept
as the answer prints it, which is what the `changed` panel shows." The answer prints a
password's value as `(set)` or `(empty)`, never the value, and the event
`surf.agent.FillRequested` holds the length of what a `fill` typed, `<n> bytes`, never its text.
So the length the page shows for a password, in the panel "Now" and in the panel "Changed by
<action>", comes from the `FillRequested` entry, not from the answer: the `value` of the newest
`FillRequested` entry of that handle since the answer that loaded the document (`12 bytes`), or
`(set)` when no `fill` of that document typed it. The token is not kept: the answer entry of
`open` omits the `token` line, and no route and no stream event holds a session's token.

### Event names

The event names this document and the stream use, one per line. `task view-stream-check` (item 2
of the design note) reads this block and fails naming each name the AsyncAPI projection of
`spec/` lacks. A story that maps a new event adds its name here.

```text
surf.shell.WindowOpened
surf.shell.TabOpened
surf.shell.TabClosed
surf.navigation.NavigationStarted
surf.navigation.NavigationCommitted
surf.navigation.NavigationFailed
surf.navigation.NavigationCancelled
surf.navigation.TraversalStarted
surf.net.RequestStarted
surf.net.ResponseReceived
surf.net.FetchFailed
surf.document.ParsingStarted
surf.document.FormAdded
surf.document.ElementAdded
surf.document.OptionAdded
surf.document.DocumentParsed
surf.render.FramePainted
surf.agent.ActionRecorded
surf.agent.NavigationRequested
surf.agent.SessionClosed
surf.agent.SessionNavigationChanged
surf.agent.SessionOpened
surf.agent.TraversalRequested
```

One more name reaches the log and the stream: `surf.agent.EventsLeftOut` (field `count`), which
the session process writes after an action's events when it left `count` of them out of the
request's entries (`crates/session/src/lib.rs`, "The entries"). `spec/` declares it, but no
command publishes it, so the AsyncAPI projection has no message for it and it stands outside the
block the check reads.

## The stream, `GET /events`

The body is server-sent events (HTML Standard, event stream format). Each line ends in LF; each
event ends with an empty line. An event is three lines:

```text
id: <the number across all sessions>
event: entry | session | reset
data: <one JSON line>
```

- `entry`: `data` is the entry exactly as the session detail writes it, `id` its `number`.
- `session`: `data` is a session summary; `id` is the number of the entry that changed it, or,
  for a change without an entry (a session listed for the first time, or `closed` or `died` when
  its process ends), the newest number at that moment.
- `reset`: `data` is `{"newest":<n>}` and `id` that same number: the reader's `Last-Event-ID` is
  older than anything kept, so the reader loads the session list and its selected session again
  and goes on from `<n>`.
- A comment line, `: keep-alive` and an empty line, every 15 s.

Where a stream starts:

- Without `Last-Event-ID`: after the newest number at the time it opens, live only.
- With `Last-Event-ID: <n>`, `<n>` decimal digits of at most 20, not above the newest number, not
  more than 256 numbers below it (the queue of a stream, `Limits::queue_entries`), and no entry
  above `<n>` dropped: every kept entry above `<n>` in number order (the reader from a number,
  D23), then live.
- With any other `Last-Event-ID` (older than what is kept, more than 256 numbers below the
  newest, which is answered as one older than what is kept, above the newest, which is what a
  restarted `surfd` sees, or not a number): `reset` first, then live after the newest number.

Live, in number order: each entry as it is kept, in the order of D19, and after each answer entry
the `session` event of its session with the same `id`. `session` events are sent live only and
are not replayed from a number, so a page loads `/api/sessions` again after each reconnect; it
opens the stream with `Last-Event-ID` set to `newest` of `/api/sessions` or the last `id:` it
read, and ignores an entry whose `number` it already holds.

A stream is closed, and its reader resumes with `Last-Event-ID`, when its queue in the server
passes 256 entries or 1 MiB, when `surfd`'s queue for it passes 1,024 entries
(`story:agent-session-events`), or when one write takes longer than 10 s. An event over 1 MiB
alone is no reason to close: it waits until the queue is empty and is then queued and written
alone, so the queue holds at most 1 MiB or that one event; its size is bounded by `surfd`'s log.

## Examples

The run behind every example: view port 41873, view token
`0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef`, a page server on
127.0.0.1:41000 that serves `/a.html` with 5,000 bytes of `text/html`, without a title and
without a link, so its answers have `title` and `now` `null`. Session `5f0c2a9e-…`
ran `open` without an address and then `go http://127.0.0.1:41000/a.html` (numbers 1 to 17).
Session `0b7d4e21-…` then ran `open`, `go http://127.0.0.1:9/`, where nothing listens, and
`close` (numbers 18 to 38). A request shows the lines the server reads; a browser sends more
headers, which the server ignores within the limits. A body is shown as its bytes: a JSON body
has no line feed at its end, and a page file ends where the file does. The `Content-Length` of
each example was counted with `wc -c` over the body, not by hand. These examples were written
before the `changed` block (`story:agent-change-and-log`), and their answers hold
`"changed":null` and their links `"state":null`; an answer of today's session holds the block,
as the two examples of the login form below show.

### `GET /`

The page files are the ones the server is configured with; in this example `index.html` is the
36 bytes `<!doctype html>`, a line feed, `<title>surf</title>` and a line feed.

```http
GET / HTTP/1.1
Host: 127.0.0.1:41873
```

```http
HTTP/1.1 200 OK
Content-Type: text/html; charset=utf-8
Content-Length: 36
Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'
X-Content-Type-Options: nosniff
Referrer-Policy: no-referrer
Cache-Control: no-store
Cross-Origin-Resource-Policy: same-origin
Date: Sat, 21 Nov 2026 02:00:00 GMT
Connection: close

<!doctype html>
<title>surf</title>
```

### `GET /view.js`

`view.js` configured as the 11 bytes `export {};` and a line feed.

```http
GET /view.js HTTP/1.1
Host: 127.0.0.1:41873
```

```http
HTTP/1.1 200 OK
Content-Type: text/javascript; charset=utf-8
Content-Length: 11
Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'
X-Content-Type-Options: nosniff
Referrer-Policy: no-referrer
Cache-Control: no-store
Cross-Origin-Resource-Policy: same-origin
Date: Sat, 21 Nov 2026 02:00:00 GMT
Connection: close

export {};
```

### `GET /api/sessions`

```http
GET /api/sessions HTTP/1.1
Host: 127.0.0.1:41873
Authorization: Bearer 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

```http
HTTP/1.1 200 OK
Content-Type: application/json
Content-Length: 603
Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'
X-Content-Type-Options: nosniff
Referrer-Policy: no-referrer
Cache-Control: no-store
Cross-Origin-Resource-Policy: same-origin
Date: Sat, 21 Nov 2026 02:00:00 GMT
Connection: close

{"newest":38,"sessions":[{"id":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","state":"open","address":"http://127.0.0.1:41000/a.html","title":null,"opened_at":1795226400000,"last_action_at":1795226401500,"ended_at":null,"page":{"kind":"loaded","status":200,"media_type":"text/html","address":"http://127.0.0.1:41000/a.html","title":null}},{"id":"0b7d4e21-9c3a-4f58-b6e2-7a1d9c8e5f30","state":"closed","address":"http://127.0.0.1:9/","title":null,"opened_at":1795226401990,"last_action_at":1795226403000,"ended_at":1795226403010,"page":{"kind":"failed","reason":"Unreachable","address":"http://127.0.0.1:9/"}}]}
```

### `GET /api/sessions/<id>`

The first session: its `open` (entries 1 to 6: the action, `surf.shell.WindowOpened`,
`surf.shell.TabOpened`, `surf.agent.SessionOpened`, `surf.agent.ActionRecorded`, and the answer
with the page line `page none`) and its `go` (entries 7 to 17: the action, nine events, the
answer), each event in the order the session's system published it. An absent optional field
(`url` of the `ActionRecorded` of `open`; `handle` and `label` of every `ActionRecorded` and of
the `NavigationRequested` of a `go`, which takes no link; `body` and `body_type` of the
`NavigationStarted` and the `RequestStarted` of a `go`, which sends `Get` without a body) is
`null`: every field `spec/` declares is written, present or not. A `follow` writes the link's
handle and label there.

```http
GET /api/sessions/5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69 HTTP/1.1
Host: 127.0.0.1:41873
Authorization: Bearer 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

```http
HTTP/1.1 200 OK
Content-Type: application/json
Content-Length: 5578
Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'
X-Content-Type-Options: nosniff
Referrer-Policy: no-referrer
Cache-Control: no-store
Cross-Origin-Resource-Policy: same-origin
Date: Sat, 21 Nov 2026 02:00:00 GMT
Connection: close

{"session":{"id":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","state":"open","address":"http://127.0.0.1:41000/a.html","title":null,"opened_at":1795226400000,"last_action_at":1795226401500,"ended_at":null,"page":{"kind":"loaded","status":200,"media_type":"text/html","address":"http://127.0.0.1:41000/a.html","title":null}},"dropped":0,"log_dropped":false,"unreadable":0,"entries":[{"number":1,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":1,"kept_at":1795226400010,"kind":"action","sequence":1,"command":"open","argument":null},{"number":2,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":2,"kept_at":1795226400011,"kind":"event","name":"surf.shell.WindowOpened","unmapped":false,"fields":{"window_id":"2c4e6a80-1b3d-4f57-9a2c-4e6a8b0d2f41"}},{"number":3,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":3,"kept_at":1795226400012,"kind":"event","name":"surf.shell.TabOpened","unmapped":false,"fields":{"tab_id":"7e9a1c3e-5a7c-4e91-b3d5-7f9b1d3f5a72","window_id":"2c4e6a80-1b3d-4f57-9a2c-4e6a8b0d2f41"}},{"number":4,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":4,"kept_at":1795226400013,"kind":"event","name":"surf.agent.SessionOpened","unmapped":false,"fields":{"session_id":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","window_id":"2c4e6a80-1b3d-4f57-9a2c-4e6a8b0d2f41","tab_id":"7e9a1c3e-5a7c-4e91-b3d5-7f9b1d3f5a72"}},{"number":5,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":5,"kept_at":1795226400014,"kind":"event","name":"surf.agent.ActionRecorded","unmapped":false,"fields":{"action_id":"e5a7c9e1-3b5d-4f71-9a3c-5e7a9c1e3b51","session_id":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","sequence":"1","command":"open","url":null,"handle":null,"label":null}},{"number":6,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":6,"kept_at":1795226400015,"kind":"answer","page":{"kind":"none"},"path":{"shown":1,"total":1,"actions":[{"sequence":1,"command":"open","handle":null,"argument":null,"argument_cut":false}]},"now":null,"changed":null},{"number":7,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":7,"kept_at":1795226401500,"kind":"action","sequence":2,"command":"go","argument":"http://127.0.0.1:41000/a.html"},{"number":8,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":8,"kept_at":1795226401501,"kind":"event","name":"surf.agent.NavigationRequested","unmapped":false,"fields":{"session_id":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","tab_id":"7e9a1c3e-5a7c-4e91-b3d5-7f9b1d3f5a72","url":"http://127.0.0.1:41000/a.html","handle":null,"label":null}},{"number":9,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":9,"kept_at":1795226401502,"kind":"event","name":"surf.agent.ActionRecorded","unmapped":false,"fields":{"action_id":"e5a7c9e1-3b5d-4f71-9a3c-5e7a9c1e3b52","session_id":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","sequence":"2","command":"go","url":"http://127.0.0.1:41000/a.html","handle":null,"label":null}},{"number":10,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":10,"kept_at":1795226401503,"kind":"event","name":"surf.navigation.NavigationStarted","unmapped":false,"fields":{"navigation_id":"c1e3a5c7-e9b1-4d3f-8a5c-7e9b1d3f5a73","tab_id":"7e9a1c3e-5a7c-4e91-b3d5-7f9b1d3f5a72","url":"http://127.0.0.1:41000/a.html","method":"Get","body":null,"body_type":null}},{"number":11,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":11,"kept_at":1795226401504,"kind":"event","name":"surf.agent.SessionNavigationChanged","unmapped":false,"fields":{"navigation_id":"c1e3a5c7-e9b1-4d3f-8a5c-7e9b1d3f5a73","state":"Started","sessions":"1"}},{"number":12,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":12,"kept_at":1795226401505,"kind":"event","name":"surf.net.RequestStarted","unmapped":false,"fields":{"request_id":"c1e3a5c7-e9b1-4d3f-8a5c-7e9b1d3f5a73","url":"http://127.0.0.1:41000/a.html","method":"Get","body":null,"body_type":null}},{"number":13,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":13,"kept_at":1795226401506,"kind":"event","name":"surf.net.ResponseReceived","unmapped":false,"fields":{"request_id":"c1e3a5c7-e9b1-4d3f-8a5c-7e9b1d3f5a73","url":"http://127.0.0.1:41000/a.html","method":"Get","status":"200","media_type":"text/html","body":"5000 bytes"}},{"number":14,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":14,"kept_at":1795226401507,"kind":"event","name":"surf.navigation.NavigationCommitted","unmapped":false,"fields":{"navigation_id":"c1e3a5c7-e9b1-4d3f-8a5c-7e9b1d3f5a73","tab_id":"7e9a1c3e-5a7c-4e91-b3d5-7f9b1d3f5a72","url":"http://127.0.0.1:41000/a.html","status":"200","media_type":"text/html","body":"5000 bytes"}},{"number":15,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":15,"kept_at":1795226401508,"kind":"event","name":"surf.agent.SessionNavigationChanged","unmapped":false,"fields":{"navigation_id":"c1e3a5c7-e9b1-4d3f-8a5c-7e9b1d3f5a73","state":"Committed","sessions":"1"}},{"number":16,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":16,"kept_at":1795226401509,"kind":"event","name":"surf.document.ParsingStarted","unmapped":false,"fields":{"document_id":"d2f4b6d8-0a2c-4e4a-9b6d-8f0a2c4e6b84","body":"5000 bytes"}},{"number":17,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":17,"kept_at":1795226401510,"kind":"answer","page":{"kind":"loaded","status":200,"media_type":"text/html","address":"http://127.0.0.1:41000/a.html","title":null},"path":{"shown":2,"total":2,"actions":[{"sequence":1,"command":"open","handle":null,"argument":null,"argument_cut":false},{"sequence":2,"command":"go","handle":null,"argument":"http://127.0.0.1:41000/a.html","argument_cut":false}]},"now":null,"changed":null}]}
```

The second session after the bound over all sessions dropped its log (D24), body only (264
bytes); with no entry kept, its `last_action_at` and `page` are `null`, as the summary's rule
gives them:

```json
{"session":{"id":"0b7d4e21-9c3a-4f58-b6e2-7a1d9c8e5f30","state":"closed","address":"http://127.0.0.1:9/","title":null,"opened_at":1795226401990,"last_action_at":null,"ended_at":1795226403010,"page":null},"dropped":21,"log_dropped":true,"unreadable":0,"entries":[]}
```

### `GET /events`

A reader that last read number 37 reconnects; entry 38, the answer of the second session's
`close`, is replayed, and 15 s later the comment line follows.

```http
GET /events HTTP/1.1
Host: 127.0.0.1:41873
Authorization: Bearer 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
Last-Event-ID: 37
```

```http
HTTP/1.1 200 OK
Content-Type: text/event-stream
Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'
X-Content-Type-Options: nosniff
Referrer-Policy: no-referrer
Cache-Control: no-store
Cross-Origin-Resource-Policy: same-origin
Date: Sat, 21 Nov 2026 02:00:00 GMT
Connection: close

id: 38
event: entry
data: {"number":38,"session":"0b7d4e21-9c3a-4f58-b6e2-7a1d9c8e5f30","index":21,"kept_at":1795226403004,"kind":"answer","page":{"kind":"failed","reason":"Unreachable","address":"http://127.0.0.1:9/"},"path":{"shown":3,"total":3,"actions":[{"sequence":1,"command":"open","handle":null,"argument":null,"argument_cut":false},{"sequence":2,"command":"go","handle":null,"argument":"http://127.0.0.1:9/","argument_cut":false},{"sequence":3,"command":"close","handle":null,"argument":null,"argument_cut":false}]},"now":null,"changed":null}

: keep-alive

```

### `GET /api/sessions` without the token

```http
GET /api/sessions HTTP/1.1
Host: 127.0.0.1:41873
```

```http
HTTP/1.1 401 Unauthorized
Content-Type: application/json
Content-Length: 24
WWW-Authenticate: Bearer
Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'
X-Content-Type-Options: nosniff
Referrer-Policy: no-referrer
Cache-Control: no-store
Cross-Origin-Resource-Policy: same-origin
Date: Sat, 21 Nov 2026 02:00:00 GMT
Connection: close

{"error":"unauthorized"}
```

### The three entry kinds

`action`, entry 7 (182 bytes):

```json
{"number":7,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":7,"kept_at":1795226401500,"kind":"action","sequence":2,"command":"go","argument":"http://127.0.0.1:41000/a.html"}
```

`event`, entry 13, with `body` as its length (340 bytes):

```json
{"number":13,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":13,"kept_at":1795226401506,"kind":"event","name":"surf.net.ResponseReceived","unmapped":false,"fields":{"request_id":"c1e3a5c7-e9b1-4d3f-8a5c-7e9b1d3f5a73","url":"http://127.0.0.1:41000/a.html","method":"Get","status":"200","media_type":"text/html","body":"5000 bytes"}}
```

`answer`, entry 17 (489 bytes):

```json
{"number":17,"session":"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69","index":17,"kept_at":1795226401510,"kind":"answer","page":{"kind":"loaded","status":200,"media_type":"text/html","address":"http://127.0.0.1:41000/a.html","title":null},"path":{"shown":2,"total":2,"actions":[{"sequence":1,"command":"open","handle":null,"argument":null,"argument_cut":false},{"sequence":2,"command":"go","handle":null,"argument":"http://127.0.0.1:41000/a.html","argument_cut":false}]},"now":null,"changed":null}
```

An `answer` entry with a `now` member, from another run: session `3d5f7b91-…` ran `open` and
`go http://127.0.0.1:41000/now/links.html`, the three-link page of the page tests' fixture
(`crates/view-fixture/pages/now/links.html`): a link to `next.html`, one inside an element with
the `hidden` attribute, and one whose `href` is `http://[`, which does not resolve. Its answer
entry, number 14 (1,101 bytes):

```json
{"number":14,"session":"3d5f7b91-2a4c-4e6f-8b0d-1f3a5c7e9b20","index":14,"kept_at":1795226500014,"kind":"answer","page":{"kind":"loaded","status":200,"media_type":"text/html","address":"http://127.0.0.1:41000/now/links.html","title":"Three links"},"path":{"shown":2,"total":2,"actions":[{"sequence":1,"command":"open","handle":null,"argument":null,"argument_cut":false},{"sequence":2,"command":"go","handle":null,"argument":"http://127.0.0.1:41000/now/links.html","argument_cut":false}]},"now":{"shown":3,"total":3,"elements":[{"handle":"l1","kind":"link","visibility":"?","label":"Next page","label_cut":false,"target":"/now/next.html","target_cut":false,"unresolved":false,"text":null,"state":null},{"handle":"l2","kind":"link","visibility":"hid","label":"Members","label_cut":false,"target":"/now/members.html","target_cut":false,"unresolved":false,"text":null,"state":null},{"handle":"l3","kind":"link","visibility":"?","label":"Broken","label_cut":false,"target":"http://[","target_cut":false,"unresolved":true,"text":null,"state":null}],"visibility_note":"hidden attribute only"},"changed":null}
```

The same session's `follow l1` then has this action in the `path` of its answer:

```json
{"sequence":3,"command":"follow","handle":"l1","argument":"Next page","argument_cut":false}
```

Two `answer` entries with controls, buttons and a `changed` member, from another run (the golden
files `crates/view/tests/golden/forms-go-answer.json` and `forms-fill-answer.json`, which a test
holds equal to these blocks): session `8e2b4d6f-…` ran `open` and a `go` to
`http://127.0.0.1:41000/forms/login.html`, the login form of the page tests' fixture
(`crates/view-fixture/pages/forms/login.html`: a text control, a password, a checked checkbox, a
select with `Editor` selected, and a submit button that posts to `answer.html`). Its answer
entry, number 26 (2,678 bytes), whose `changed` is the new page and its texts:

```json
{"number":26,"session":"8e2b4d6f-1a3c-4e5f-9b7d-2c4e6f8a0b13","index":26,"kept_at":1795226600026,"kind":"answer","page":{"kind":"loaded","status":200,"media_type":"text/html","address":"http://127.0.0.1:41000/forms/login.html","title":"Sign in"},"path":{"shown":2,"total":2,"actions":[{"sequence":1,"command":"open","handle":null,"argument":null,"argument_cut":false},{"sequence":2,"command":"go","handle":null,"argument":"http://127.0.0.1:41000/forms/login.html","argument_cut":false}]},"now":{"shown":5,"total":5,"elements":[{"handle":"f1","kind":"text","visibility":"?","label":"Name","label_cut":false,"target":null,"target_cut":null,"unresolved":null,"text":"f1 text ? \u0022Name\u0022 = \u0022\u0022","state":{"value":{"kind":"text","text":"","cut":false},"disabled":false,"checked":false,"required":false,"readonly":false,"submits":null}},{"handle":"f2","kind":"password","visibility":"?","label":"Password","label_cut":false,"target":null,"target_cut":null,"unresolved":null,"text":"f2 password ? \u0022Password\u0022 = (empty) required","state":{"value":{"kind":"empty","text":null,"cut":null},"disabled":false,"checked":false,"required":true,"readonly":false,"submits":null}},{"handle":"f3","kind":"checkbox","visibility":"?","label":"Remember me","label_cut":false,"target":null,"target_cut":null,"unresolved":null,"text":"f3 checkbox ? \u0022Remember me\u0022 checked","state":{"value":null,"disabled":false,"checked":true,"required":false,"readonly":false,"submits":null}},{"handle":"f4","kind":"select","visibility":"?","label":"Role","label_cut":false,"target":null,"target_cut":null,"unresolved":null,"text":"f4 select ? \u0022Role\u0022 = \u0022Editor\u0022","state":{"value":{"kind":"text","text":"Editor","cut":false},"disabled":false,"checked":false,"required":false,"readonly":false,"submits":null}},{"handle":"b1","kind":"button","visibility":"?","label":"Sign in","label_cut":false,"target":null,"target_cut":null,"unresolved":null,"text":"b1 button ? \u0022Sign in\u0022 submits F1 post \u0022/forms/answer.html\u0022","state":{"value":null,"disabled":false,"checked":false,"required":false,"readonly":false,"submits":{"form":"F1","method":"post","action":"/forms/answer.html","action_cut":false}}}],"visibility_note":"hidden attribute only"},"changed":{"kind":"page","from":null,"to":"http://127.0.0.1:41000/forms/login.html","texts":[{"text":"# Sign in","cut":false},{"text":"Name [@f1 text: Name]","cut":false},{"text":"Password [@f2 password required: Password]","cut":false},{"text":"[@f3 checkbox checked: Remember me] Remember me","cut":false},{"text":"Role [@f4 select: Role]","cut":false},{"text":"[@b1 button: Sign in]","cut":false}],"more":0}}
```

Then `fill f2 -`, the password typed on standard input. Its answer entry, number 31 (2,512
bytes): the password is `(set)` in `now` and its line in `changed` is `(empty)` to `(set)`; the
entries of that action hold its length only, in the `FillRequested` entry (`"value":"12
bytes"`), which is where the page reads the length it shows.

```json
{"number":31,"session":"8e2b4d6f-1a3c-4e5f-9b7d-2c4e6f8a0b13","index":31,"kept_at":1795226601031,"kind":"answer","page":{"kind":"loaded","status":200,"media_type":"text/html","address":"http://127.0.0.1:41000/forms/login.html","title":"Sign in"},"path":{"shown":3,"total":3,"actions":[{"sequence":1,"command":"open","handle":null,"argument":null,"argument_cut":false},{"sequence":2,"command":"go","handle":null,"argument":"http://127.0.0.1:41000/forms/login.html","argument_cut":false},{"sequence":3,"command":"fill","handle":"f2","argument":null,"argument_cut":false}]},"now":{"shown":5,"total":5,"elements":[{"handle":"f1","kind":"text","visibility":"?","label":"Name","label_cut":false,"target":null,"target_cut":null,"unresolved":null,"text":"f1 text ? \u0022Name\u0022 = \u0022\u0022","state":{"value":{"kind":"text","text":"","cut":false},"disabled":false,"checked":false,"required":false,"readonly":false,"submits":null}},{"handle":"f2","kind":"password","visibility":"?","label":"Password","label_cut":false,"target":null,"target_cut":null,"unresolved":null,"text":"f2 password ? \u0022Password\u0022 = (set) required","state":{"value":{"kind":"set","text":null,"cut":null},"disabled":false,"checked":false,"required":true,"readonly":false,"submits":null}},{"handle":"f3","kind":"checkbox","visibility":"?","label":"Remember me","label_cut":false,"target":null,"target_cut":null,"unresolved":null,"text":"f3 checkbox ? \u0022Remember me\u0022 checked","state":{"value":null,"disabled":false,"checked":true,"required":false,"readonly":false,"submits":null}},{"handle":"f4","kind":"select","visibility":"?","label":"Role","label_cut":false,"target":null,"target_cut":null,"unresolved":null,"text":"f4 select ? \u0022Role\u0022 = \u0022Editor\u0022","state":{"value":{"kind":"text","text":"Editor","cut":false},"disabled":false,"checked":false,"required":false,"readonly":false,"submits":null}},{"handle":"b1","kind":"button","visibility":"?","label":"Sign in","label_cut":false,"target":null,"target_cut":null,"unresolved":null,"text":"b1 button ? \u0022Sign in\u0022 submits F1 post \u0022/forms/answer.html\u0022","state":{"value":null,"disabled":false,"checked":false,"required":false,"readonly":false,"submits":{"form":"F1","method":"post","action":"/forms/answer.html","action_cut":false}}}],"visibility_note":"hidden attribute only"},"changed":{"kind":"controls","lines":[{"handle":"f2","what":"value","before":{"kind":"empty","text":null,"cut":null},"after":{"kind":"set","text":null,"cut":null}}]}}
```

### The three stream events

`entry`, as it arrives live after the second session's `close` (553 bytes, the empty line
included):

```text
id: 38
event: entry
data: {"number":38,"session":"0b7d4e21-9c3a-4f58-b6e2-7a1d9c8e5f30","index":21,"kept_at":1795226403004,"kind":"answer","page":{"kind":"failed","reason":"Unreachable","address":"http://127.0.0.1:9/"},"path":{"shown":3,"total":3,"actions":[{"sequence":1,"command":"open","handle":null,"argument":null,"argument_cut":false},{"sequence":2,"command":"go","handle":null,"argument":"http://127.0.0.1:9/","argument_cut":false},{"sequence":3,"command":"close","handle":null,"argument":null,"argument_cut":false}]},"now":null,"changed":null}

```

`session`, when the second session's process has ended after `close` and it is listed `closed`
(299 bytes):

```text
id: 38
event: session
data: {"id":"0b7d4e21-9c3a-4f58-b6e2-7a1d9c8e5f30","state":"closed","address":"http://127.0.0.1:9/","title":null,"opened_at":1795226401990,"last_action_at":1795226403000,"ended_at":1795226403010,"page":{"kind":"failed","reason":"Unreachable","address":"http://127.0.0.1:9/"}}

```

`reset`, to a reader that sends `Last-Event-ID: 99` while the newest number is 38 (41 bytes):

```text
id: 38
event: reset
data: {"newest":38}

```
