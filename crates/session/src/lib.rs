//! One agent session: the core, a [`Session`] over any [`surf::Transport`], and the process
//! [`process::run`] that the binary `surf-session` is, which `surfd` starts per session and
//! alone connects to (design section 3 of `docs/design/agent-interface.md`).
//!
//! # The core
//!
//! [`Session::new`] takes the session's identity, chosen by the caller (`surfd` draws it), and a
//! transport, and draws the session's token from `surf-secret`. [`Session::answer`] takes one
//! request of [`surf_agent::wire`] and returns a [`Reply`]: the exit status, the typed answer of
//! `docs/agent-answer.md` and the status record of [`surfd::link`]. Nothing in it needs a socket
//! or a process, so the scenario runner uses it as it is.
//!
//! What each command issues, as the model asks (`spec/domains/agent.yaml`, the comments above
//! `OpenSession` and `RecordAction`), each on its component's port, then [`surf::Browser::run`]:
//!
//! - `open [<url>]`: `surf.shell.OpenWindow`, `OpenTab` in it, `surf.agent.OpenSession` sent as
//!   `Opener` (the caller naming no session), then `RecordAction` 1 `open`; with an address, then
//!   `Go` sent as the session's holder and `RecordAction` 2 `go`. Two actions, one answer, which
//!   carries the `token` line.
//! - `go <url>`: `Go` as the holder; when it is `requested`, `RecordAction` `go`.
//! - `look`: the views `SessionState` and `SessionLog`; no action.
//! - `follow <handle>` (`arg handle`): `Follow` as the holder with the link's resolved address,
//!   its handle and its label; when it is `requested`, `RecordAction` `follow` with all three
//!   ("The links" below).
//! - `fill`, `pick` ("The controls" below) and `press <handle>` ("Pressing a button" below).
//! - `back` and `forward`: `GoBack` and `GoForward` as the holder; when it is `requested`,
//!   `RecordAction` `back` or `forward` ("Back and forward" below).
//! - `log [<n>]` (`arg count`): the views `SessionState` and `SessionLog`; no action ("The log"
//!   below).
//! - `close`: `CloseSession` as the holder; when it is `closed`, `RecordAction` `close`.
//!
//! An accepted command's `RecordAction` is issued before the run that delivers what the command
//! published, so that the log holds the action before the events it caused (decision D19).
//! [`Session`] holds the sequence number: the last one recorded.
//!
//! # The page as text
//!
//! `open <url>`, `go <url>` and `look` take `arg form` (`tree` or `markdown`, the client's
//! `--as`) and, beside it, `arg line` (a number from 1, `--from-line`; 1 when absent). The answer
//! then ends in `note text: no-css` and the `text` block of `docs/agent-answer.md`:
//! [`surf_answer::text_page`] over [`surf_page_text::outline`] or
//! [`surf_page_text::markdown::markdown`] of the tree `surf::Browser` keeps for the document of
//! the session's current navigation (`Browser::document_tree`, read only; nothing is fetched
//! again). For `open <url>` and `go <url>` that is the document the load committed. With no such
//! document (`page none`, a failed load, a response that is not HTML) the answer ends in `note
//! text: no-document` and has no block. A refusal carries neither. The call records what it
//! records without the arguments: the same action, the same events, the same `path`; only the
//! answer is longer, and the exit status is the one the call has without them. A Markdown block
//! whose nesting passed the prefix bound adds `note text: deep <n>` after `note text: no-css`.
//!
//! The session renders a document once per form, in time linear in the tree, and keeps the
//! lines of the current document for the looks that page through it: a look reads the lines it
//! shows and one more, so reading a page to its end costs the page once, not once per look. It
//! keeps the current document's text only, and drops it with every request but `look` (a `go`,
//! a `close`, and each later command that changes the document or what its text shows) and when
//! the current document is another one.
//!
//! # The links
//!
//! After each command the session reads its current document (the one "The page as text" names).
//! The `page` line and the status record carry its title (`document.title`), unless it is empty
//! or longer than [`TITLE_CHARS_MAX`] characters. An answer with such a document holds `note
//! parser: <parts>` when [`surf::NOT_YET_PARSED`] is not empty; one of an open session without
//! page text, when the document has links or controls, the `now` block and `note visibility:
//! hidden attribute only` (`docs/agent-answer.md`). A call with `--as` has no `now` block.
//!
//! The links are the rows of `surf.document.Elements` of kind `link` in position order, each
//! with the handle [`surf_page::handles`] gives its node (`l1`, `l2`, … in document order,
//! hidden elements included, starting at 1 for each document), the row's label and target (the
//! `href` as written), and `hid` when the element or an ancestor has the HTML `hidden`
//! attribute, read from the tree: `aria-hidden`, a closed `dialog` or `details` and a popover
//! leave a link `?`, though the row's `hidden` flag is wider. The target is resolved with
//! `surf-url` against [`surf_page::base_url::document_base_url`]; a line shows its path, query
//! and fragment when it has the document's origin and a path that does not start with `//`
//! (which would read as another host), and otherwise the whole address without its userinfo,
//! so that scheme, host and port come first; or the `href` as written and `unresolved`. The
//! label is cut at [`surf_answer::LABEL_CHARS_MAX`]
//! characters, the target at [`surf_answer::TARGET_CHARS_MAX`]. The block shows the first lines
//! that fit [`NOW_BYTES_MAX`] bytes, and its header `now <shown> of <total>` counts them all.
//! The session computes the links once per document and keeps them while it is current.
//!
//! `follow <handle>` is refused `closed` on a closed session, `unknown-handle` when the current
//! document has no element of that handle (or there is none), `wrong-kind` for a control,
//! `hidden` for a link `hid` names, and `unresolved` for a target that does not resolve; each
//! with status 1 and the state. The `path` line of a `follow` is `<n> follow <handle>
//! "<label>"`, the label cut as in the `now` block.
//!
//! # The controls
//!
//! The `now` block lists every actionable row, links and controls together, in position order:
//! a control's handle is `f<n>` or `b<n>` from [`surf_page::handles`], its visibility as for a
//! link, and in place of a target its state (`docs/agent-answer.md`, "The now block"), read from
//! the control's row and, for a select, from its option rows: the value of a text, textarea or
//! file control quoted and cut at [`surf_answer::VALUE_CHARS_MAX`]; `(set)` or `(empty)` for a
//! password, whose value is in no line; the label of a select's selected option (the first of
//! them in position order); then `disabled`, `checked` (a checkbox or radio button), `required`
//! and `readonly`.
//!
//! `fill <handle> <value>` and `pick <handle> <option>` are checked in this order, each refusal
//! with status 1 and the state: `closed`; `unknown-handle`; `wrong-kind` (`fill` takes `text`,
//! `password` and `textarea`, `pick` takes `select`, `checkbox` and `radio`); `disabled`;
//! `hidden`, from the row's `hidden` flag, which is wider than `hid` and is what `SetValue`
//! refuses; then for `fill` `readonly`, and for `pick` `unknown-option` (a select without an
//! option of that label, then of that value; a word other than `on` or `off` for a checkbox, or
//! than `on` for a radio button) and `disabled` for the option picked (an option is hidden only
//! where its select is, which is refused `hidden` before). A checkbox already in the state asked
//! for is left as it is: `did pick "<handle>"`, status 0, no action. The value of a `text` or
//! `password` control is sent with its line feeds and carriage returns removed, as the HTML
//! Standard sanitizes a single-line `input`; a `textarea` keeps them. Otherwise the session sends
//! `Fill` or `Pick` as the holder, records the action (`<n> fill
//! <handle>`, `<n> pick <handle>`: the handle bare, never the value or the option) and runs the
//! system, in which the bindings set the value, check the control (a radio button unchecks the
//! others of its group) or select the option. The answer is `did fill "<handle>"` or `did pick
//! "<handle>"` with the state. The value of a `fill` is in no row of the session, no line of an
//! answer and no entry: the `FillRequested` entry gives its length (`"<n> bytes"`).
//!
//! # Following a link
//!
//! Before any navigation, and so with the page and the path as they were, `follow` refuses a
//! resolved target, with status 1 and the state:
//!
//! - longer than [`surf_fetch::MAX_ADDRESS_BYTES`] bytes: `address-too-long`, the bound of an
//!   address a redirect leads to ("What it keeps");
//! - with a scheme other than `http`, the one scheme the transport fetches (`javascript:`,
//!   `mailto:`, `data:`, `file:`, `https:`): `unsupported-scheme`. `go` still navigates to such
//!   an address and its page line says `failed UnsupportedScheme`;
//! - from a document whose address is not local by its text, a target whose host is
//!   (`host_is_local`: an IP address [`surf_fetch::is_local`] names, the rule a redirect's
//!   destination is checked by, or `localhost` or a name under it): `forbidden-destination`. A
//!   document at another name counts as public, as the session does not know where its fetch
//!   connected.
//!
//! A target that is another name is resolved only by the fetch. From a document that counts as
//! public, the session tells the transport so ([`fetch`], [`fetch_through`]), and the fetch
//! applies to the first request the rule it applies to a redirect ([`surf_fetch::Origin`]): a
//! host that resolves to a local address, any one of those resolved, is refused before a
//! connection is made. The session runs the navigation before it records the action; when the
//! fetch refused its first request, it records nothing and answers `did refused
//! forbidden-destination` with the page, the path and the values as they were, as after its own
//! check: the page and the current document are read as before that navigation for as long as
//! it is the session's last. `go` with a typed address is not checked so. The same holds for the
//! request of a `press` ("Pressing a button").
//!
//! # Pressing a button
//!
//! The line of a button with a form owner ends in `submits F<n> <method> "<action>"`: the form's
//! handle ([`surf_page::form_handles`]), and the method and action of [`surf::Submits`], which
//! [`surf::Browser::submits`] reads for every button of the document with the submitter's
//! `formmethod` and `formaction`; the action is shown as a link's target is ("The links"), and
//! an action that does not parse as `submits F<n> <method> unresolved`, with none of its text,
//! which could hold a user name and a password. The action of the form and each distinct
//! `formaction` are resolved once per answer, however many buttons share them.
//!
//! `press <handle>` is checked in this order, each refusal with status 1 and the state, before
//! any command is sent, so with no submission event, no request and the path as it was:
//! `closed`; `unknown-handle`; `wrong-kind` for anything but a button; `disabled`; `hidden`;
//! `no-default-action` for a button that is not a submit button (`type=button`, `type=reset`);
//! then, for a button with a form owner, from [`surf::Browser::plan_submission`], the plan the
//! submission step itself follows ([`surf::plan_submission`]), so the address checked is the
//! address requested: `password-in-query` for a `GET` whose entry list holds a non-empty password
//! value; and for the address it would request (a `GET`'s with its query written) the three
//! refusals of "Following a link", `address-too-long`, `unsupported-scheme` and
//! `forbidden-destination`. Otherwise the session sends `Press` as the holder, records `<n> press
//! <handle>` and runs the system: the submission step submits the form and the tab navigates
//! (`did press "<handle>"`, the page it committed), or ends the submission with a
//! `SubmissionAbandoned` the session answers with its word, status 1, the action recorded:
//! `no-form-owner`, `invalid-controls`, `invalid-action`, `unsupported-enctype`,
//! `dialog-method`; `NotSubmitButton`, `UnsupportedScheme` and `AddressTooLong`, refused before,
//! map to `no-default-action`, `unsupported-scheme` and `address-too-long`.
//!
//! # Back and forward
//!
//! The `agent` component reads no history, so the session reads it before it sends anything:
//! the row of the navigator's view `Navigations` in the tab of the session whose `standing` is
//! `back` (for `back`) or `forward` (for `forward`). Each refusal has status 1 and the state,
//! with no event, no request and the path as it was: `closed`; `at-start` or `at-end` when there
//! is no such row; `post-entry` when its `method` is `Post`, an entry the navigator would not
//! load again (`TraverseHistory`), since surf has no cache. Otherwise the session sends `GoBack`
//! or `GoForward` as the holder and runs the system: the navigator loads the entry's address
//! with `GET`.
//!
//! The session's own checks of a target ("Following a link") are not made again: they read the
//! address text, and an entry's address passed them when the entry was made. What a name
//! resolves to is not settled by that, so the fetch's rule is applied again (decisions D279 and
//! D283): the run goes out under the rule of a page that counts as public when a public page or
//! a public server chose the entry's address, by link, form or redirect: an entry a `follow` or
//! a `press` made from such a page, and an entry a redirect moved to from an address that counts
//! as public, whatever command started it. Only an entry the agent's own address reached without
//! a redirect, or by a redirect from a local address, goes out under the rule of `go`, whatever
//! page the session is on. A first request that rule refuses is
//! answered `did refused forbidden-destination`, status 1, with nothing recorded and the page
//! and the path as they were ([`Kept`]), as for a `follow`. Otherwise the session records `<n>
//! back` or `<n> forward`, and the answer is `did back` or `did forward` with the page it
//! committed, or `page failed`, the tab's current entry left where it was.
//!
//! # What changed
//!
//! The session keeps what its last answer that showed the session's state showed ([`Shown`]):
//! the navigation its page came from (the view `Sessions`, or the one before a refused first
//! request, [`Kept`]), the address of its `page` line, its current document, and for each
//! control of that document, in document order, its handle, its kind, its row's value and
//! checkedness and the label of its selected option (the first selected by position), each
//! whole. Before the first answer it keeps no navigation, no address and no control. Every
//! answer that shows the state (`state_reply`: actions, reads and refusals with the state)
//! compares that with the same reading now and holds the `changed` block, then keeps the new
//! reading; a refusal without the token, a `usage` refusal of `open` or of a request that does
//! not decode, and a status-3 answer show no state, have no block and keep nothing.
//!
//! - Another navigation: `changed page`, the address of the old `page` line and of the new one,
//!   or `none` for `page none`. Then the texts of the new document that the old one did not
//!   hold: the lines [`surf_page_text::markdown::markdown`] renders of each tree
//!   (`Browser::document_tree`, which keeps the trees of every document of the session) but
//!   the empty ones, each once, in order; the first [`surf_answer::CHANGED_TEXTS_MAX`] (10)
//!   cut at [`surf_answer::CHANGED_TEXT_CHARS_MAX`] (120) characters, and the count of the
//!   others. Without a new document (a failed load, a response that is not HTML) there are
//!   none.
//! - The same navigation: one line per control whose state differs, in document order: its value
//!   for a text, textarea, file or password control (a password's `(set)` or `(empty)`, never the
//!   value), its checkedness for a checkbox or a radio button, the label of its selected option
//!   for a select. Before scripts nothing but a control's state changes within a document
//!   (decision D2 of `story:agent-change-and-log`), so nothing else is compared.
//!
//! An action changes at most two controls: `fill` and `pick` one, a radio button turned on the
//! one of its group the parser left checked as well (`surf_page::forms`, at most one checked per
//! group), so the block of the same navigation is at most two lines.
//!
//! # The log
//!
//! `log` takes `arg count`, a number from 1 in decimal digits; any other argument, or a count
//! that is not one, is `did refused usage`, status 2. It records nothing. Its answer is that of
//! `look` with the `path` block of every action of the view `SessionLog`, or of the last
//! `count`, oldest first: as many of the newest as fit [`LOG_BYTES_MAX`] bytes of lines,
//! `path <shown> of <total>` counting them all.
//!
//! # Refusals
//!
//! - A request other than `open` whose `session` is not this identity, or whose token does not
//!   match the session's (compared by [`surf_secret::Token::matches`] only), or that comes before
//!   the session is open: `did refused token`, status 1, the head line and the `did` line only.
//!   Nothing is acted on and nothing of the session's state is in the answer.
//! - From the holder, a command this session does not know, an argument name it does not know,
//!   `go` without `arg url`, `arg form` other than `tree` or `markdown`, `arg line` without `arg
//!   form` or not a number from 1, `arg count` of `log` not a number from 1: `did refused
//!   usage`, status 2, with the current page and path.
//! - `open` after the first, `open` with a session, a token or an argument but `url`, `form` and
//!   `line`, `open` with `arg form` and no `arg url` or with a `form` or `line` refused above, and
//!   a request that does not decode ([`Session::malformed`]): `did refused usage`, status 2, the
//!   head line and the `did` line only, because it carries no token.
//!
//! The refusals that carry no proof of the token (the first and the third above) are the same
//! in every state of the session: the head line says `unknown`, and the status record holds the
//! state `unknown` ([`link::SessionState::Unknown`]) with no address and no title. So a process
//! that reaches the session's socket without the token reads nothing of the session, not even
//! the address of its page, and `surfd` keeps what it knew of the session before the request.
//! - `go`, `follow`, `fill`, `pick`, `press`, `back`, `forward` or `close` on a closed session:
//!   `did refused closed`, status 1, with the state. `follow` or `press` without `arg handle`,
//!   `fill` without `arg handle` and `arg value`, `pick` without `arg handle` and `arg option`,
//!   or any of them with another argument, and `back`, `forward` or `close` with any argument:
//!   `did refused usage`.
//!
//! # The token
//!
//! The 32 bytes stay in this process's memory, as a [`surf_secret::Token`]; they are written once,
//! as the `token` line of the answer to `open`, and nowhere else: not in the status record, not
//! in another answer, not in a `Debug` form ([`Session`]'s writes `Token(..)`).
//!
//! # What it keeps
//!
//! The answer lines hold the page line, the `did` line and at most [`PATH_SHOWN`] path lines,
//! seven addresses, and since `story:agent-change-and-log` the two of a `changed page` line,
//! nine. An address has one of three sources: a request of at most
//! [`surf_agent::wire::REQUEST_MAX`] bytes, which quotes it as the answer does; the address a
//! response came from after redirects, at most [`surf_fetch::MAX_ADDRESS_BYTES`] bytes, the
//! same 64 KiB, before quoting; or the target of a link a `follow` took, which a page writes
//! and `follow` refuses over [`surf_fetch::MAX_ADDRESS_BYTES`] bytes ("Following a link"), and
//! which a redirect after it bounds the same way. Quoting writes a `"`, a `\` and each byte outside
//! printable ASCII as four bytes (`\x5c` for a `\`), so a quoted address is at most four times
//! its length and its two quotes; `surf-url` leaves a `\` in the query or fragment of a
//! response URL as it is, so the quoted form of one comes to up to four times
//! [`surf_fetch::MAX_ADDRESS_BYTES`], at most 262,146 bytes. Nine addresses of at most 262,146
//! bytes each are 2,359,314 bytes. The `did` line of `follow` holds the handle as typed, from a
//! request like an address, and a `follow` path line holds a label of at most 60 characters
//! (at most 965 bytes quoted), so the nine still bound them. The title adds at most
//! [`TITLE_CHARS_MAX`] characters, at most 16,386 bytes quoted. The rest of the `changed` block
//! adds at most 10 `+ text` lines of 120 characters, at most 495 bytes each quoted, and a `more`
//! line; or at most two `~` lines ("What changed"), each with two values of at most 1,024
//! characters, under 8,300 bytes each quoted. A `text` block adds at most
//! [`surf_answer::TEXT_BYTES_MAX`] (32,768) bytes of body lines, its header, a `more` line and a
//! `note text:` line, each under 200 bytes; or, in an answer without text, the `now` block adds
//! at most [`NOW_BYTES_MAX`] (1,048,576) bytes and `note visibility:` and `note parser:` lines
//! under 200 bytes each. So an answer stays below [`surfd::link::ANSWER_TEXT_MAX`], 4,194,304
//! bytes: about 3,441,000 at most. The answer of `log` has no `did` line and in place of the five
//! path lines at most [`LOG_BYTES_MAX`] (1,048,576) bytes of them, so it stays below 3,000,000.
//!
//! A status record whose quoted address does not fit [`surfd::link::RECORD_MAX`] (65,536 bytes,
//! with the record's other lines) is sent without the address, and `surfd` lists the session
//! without it; the page line of the answer still holds it. An address asked through the wire
//! comes to about that limit at most, as the request that carries it is at most 64 KiB and
//! quotes it the same way; the address a response came from after redirects comes to up to
//! four times it, so a server can cause a record without an address: a redirect to an address
//! of about 65,500 bytes or more, or to a shorter one whose query holds backslashes (case 24 of
//! `tests/redirect.rs`: an address of 65,536 bytes whose query is backslashes quotes to 262,071
//! bytes).
//!
//! A response body (at most `surf-fetch`'s body limit, 16 MiB) is held only while the command
//! that loaded it runs: the system carries it in two events and two binding invocations, and
//! [`Session::answer`] drops everything the system delivered once the reply is made
//! ([`surf::Browser::forget_delivered`]; [`Session::held`] counts what is left, nothing between
//! commands). No answer needs a body: the page line is read from the `SessionState` row (status,
//! media type, address, failure reason). What stays across commands are the components' rows,
//! none of which holds a body: per `go` one action, one navigation, one request and, for an
//! HTML page, one document, each with its address and identities; and the session's row.
//! Since `story:html-document-port` an HTML page also leaves its document's title, one element
//! row per actionable element, and its node tree, which `surf::Browser` keeps for the life of
//! the session (each tree within `surf-html`'s limits; no bound over all documents yet). Since
//! the fix round of round 1 a call with `--as` also leaves the lines of the current document in
//! the forms asked for ("The page as text"), until the next request that is not a `look`.
//! Since `story:agent-change-and-log` the session keeps what its last answer showed ("What
//! changed"): two addresses and identities, and the handle, value and selected label of each
//! control of the current document, values the element and option rows hold as well; it is
//! replaced at each answer that shows the state.
//!
//! # The entries
//!
//! Each reply carries the entries `surfd` keeps in its log (`surfd::link`, "The entries";
//! `story:agent-session-events`), read inside [`Session::answer`] before its
//! `forget_delivered`. For each action the request recorded: the action entry, from the view
//! `SessionLog`; then one event entry ([`entries::event_line`]) for every event the system
//! published for it, in the order `System::published` lists them. An action's events end at
//! the run after its `RecordAction`, so `open <url>` gives action `open`, the events up to and
//! including that run, action `go`, the remaining events. Then one answer entry, the rendered
//! lines without the `token` line ([`Reply::to_link`]). A request that records no action
//! (`look`, a refusal) has no entry.
//!
//! The entries of one request through the wire stay below [`surfd::link::ENTRIES_MAX`],
//! 8,388,608 bytes, whatever the page holds. The answer entry is the answer lines, at most
//! [`surfd::link::ANSWER_TEXT_MAX`] (4,194,304 bytes; it holds at most nine addresses, `did`,
//! `page`, [`PATH_SHOWN`] path lines and the two of `changed page`, each at most 262,146 bytes
//! once quoted, "What it keeps"), and its `answer-entry <m>` line. The action and event entries take at most
//! [`ACTION_AND_EVENT_BYTES_MAX`], 4,194,240 bytes, the rest: the action entries (at most two,
//! `open <url>` records `open` and `go`, each with at most one address) are written whole and
//! their bytes, with 64 bytes for a left-out line after each, are set aside first; an event
//! entry that would take the event entries past what is left is left out, and so is every
//! row entry (`surf.document.ElementAdded`, `surf.document.FormAdded` and
//! `surf.document.OptionAdded`, one per row the parser step adds) after the first
//! [`ELEMENT_ENTRIES_MAX`] (256) of the request. After an action's events, one line
//! `event surf.agent.EventsLeftOut count="<n>"` ([`entries::left_out_line`]) counts the entries
//! left out of them; it is absent when none was. So a request carries at most 256 row entries,
//! and `surfd`'s bounds of a session's log (10,000 entries) and of a reader's queue (1,024)
//! are not reached by one page.
//! An event field holds no body, only its length. The fields that hold text from a page, the
//! title of `surf.document.DocumentParsed` and the label of `surf.agent.ActionRecorded` and of
//! `surf.agent.NavigationRequested` (the label of the link a `follow` took), each hold at most
//! [`entries::TITLE_ENTRY_BYTES`] (1,024) bytes of it, at most 4,098 bytes quoted, with the
//! field `title_bytes` or `label_bytes` when it was cut. The `handle` of an action's events,
//! which the client supplies, is cut the same way, with `handle_bytes`. An element's target and
//! value are in no entry, and its label only in those two (`surf.document.ElementAdded` carries
//! two identities and a position). An address, at most
//! [`surf_agent::wire::REQUEST_MAX`] bytes when it comes from a request and at most
//! [`surf_fetch::MAX_ADDRESS_BYTES`] bytes, the same 64 KiB, when it is the address a response
//! came from after redirects or the target a `follow` took, is at most 262,146 bytes once
//! quoted, in at most nine action and event lines of one `open <url>`, `go` or `follow`
//! (`DocumentParsed` is the ninth), 2,359,314 bytes,
//! with a few hundred bytes of other fields per line: below the 4,194,240, so no event of the
//! page load itself is left out.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod entries;
pub mod process;

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io;

use surf::{MemoryBrowser, NodeId, Transport, Tree, UnmetObligation};
use surf_agent::wire::{self, Request};
use surf_answer::{
    Answer, CHANGED_TEXT_CHARS_MAX, CHANGED_TEXTS_MAX, Change, Changed, ControlChange, Cut, Did,
    Element, Form, Head, LABEL_CHARS_MAX, Note, Now, Page, Path, PathEntry, RefusalReason,
    SessionState as HeadState, State, TARGET_CHARS_MAX, Target, TextNote, VALUE_CHARS_MAX, Value,
    Visibility,
};
use surf_ports::{Caller, ComponentPorts};
use surf_secret::Token;
use surf_types::agent::{
    ActionCommand, CloseSession, CloseSessionOutcome, Fill, FillOutcome, Follow, FollowOutcome, Go,
    GoBack, GoBackOutcome, GoForward, GoForwardOutcome, GoOutcome, OpenSession, OpenSessionOutcome,
    Pick, PickOutcome, Press, PressOutcome, RecordAction, RecordActionOutcome, SessionId,
    SessionState, SessionStateView,
};
use surf_types::document::{
    DocumentId, DocumentState, Documents, ElementId, ElementKind, Elements, Options,
};
use surf_types::navigation::{NavigationState, Standing};
use surf_types::net::{Method, RequestId};
use surf_types::primitives::Uuid;
use surf_types::shell::{OpenTab, OpenTabOutcome, OpenWindow, OpenWindowOutcome};
use surf_types::url::Url;
use surfd::link;

/// The most actions the `path` block shows: the last five (design section 5.1).
pub const PATH_SHOWN: usize = 5;

/// The most bytes the `path` block of a `log` answer takes, its lines counted high ("The log"
/// above): 1,048,576. The newest actions are kept. A deliberate bound the design does not
/// state, as for [`NOW_BYTES_MAX`].
pub const LOG_BYTES_MAX: usize = 1 << 20;

/// The bytes of a `path` line beside its quoted field and its handle: two spaces, a sequence
/// number of up to 20 digits, a command word of up to seven letters, the spaces, `...` and the
/// line feed, counted high.
const PATH_LINE_FIXED: usize = 40;

/// The most bytes the lines of the `now` block take, its header and line feeds included ("The
/// links" above): 1,048,576. A deliberate bound the design does not state; paging a long block
/// is `story:agent-answer-paging`.
pub const NOW_BYTES_MAX: usize = 1 << 20;

/// The most characters of a title the `page` line and the status record hold ("The links"
/// above): a longer one is left out of both.
pub const TITLE_CHARS_MAX: usize = 1_024;

/// The bytes of a `now` line beside its handle and its two quoted fields: two spaces, the kind
/// `link`, the visibility word, `...` after each field, ` -> `, ` unresolved`, separators and
/// the line feed, counted high.
const NOW_LINE_FIXED: usize = 48;

/// The bytes a control's `now` line takes beside [`NOW_LINE_FIXED`], its handle, its label and
/// its quoted value: a kind word of up to eight letters, ` = `, `(empty)`, `...` after the value
/// and the four flags with their spaces, counted high.
const CONTROL_LINE_FIXED: usize = 64;

/// The bytes of a button's `submits` part beside its form handle and its quoted action:
/// ` submits `, a method word of up to six letters, the spaces, `...` and ` unresolved`, counted
/// high.
const SUBMITS_FIXED: usize = 32;

/// The most row entries (`surf.document.ElementAdded`, `FormAdded` and `OptionAdded`) one request
/// carries ("The entries" above): the first 256 of the request; the others are left out and
/// counted.
pub const ELEMENT_ENTRIES_MAX: usize = 256;

/// The most bytes the action and event entries of one request take together, left-out lines
/// included ("The entries" above): [`surfd::link::ENTRIES_MAX`] less
/// [`surfd::link::ANSWER_TEXT_MAX`], the most the answer entry holds, and 64 bytes for its
/// `answer-entry <m>` line: 4,194,240 bytes.
pub const ACTION_AND_EVENT_BYTES_MAX: usize = link::ENTRIES_MAX - link::ANSWER_TEXT_MAX - 64;

/// The most bytes of a left-out line ([`entries::left_out_line`]) with its line feed: the line
/// with the 20 digits of the largest count is 61 bytes.
const LEFT_OUT_LINE_MAX: usize = 64;

/// The transport of the binary: [`surf::fetch_http`] at `surf-fetch`'s default limits, through
/// [`surf::answer`]. An address that is not `http:` is `UnsupportedScheme` and nothing is read;
/// so is a `Get` with a body or a media type, before any connection. A `Post` is sent with its
/// body and media type (an agent's `go` navigates with `Get`). Redirects are followed inside the
/// one fetch, up to 20, and the answer carries the address of the last response. The first
/// request of a `follow` or a `press` from a page that counts as public goes to public addresses
/// only ("Following a link" above).
pub fn fetch(request: &surf::RequestStarted) -> surf::Answer {
    fetch_through(request, &surf_fetch::OsNetwork)
}

/// [`fetch`] over `network`, which resolves names and makes connections: a test gives a name
/// the addresses it chooses without the host's resolver.
pub fn fetch_through(
    request: &surf::RequestStarted,
    network: &dyn surf_fetch::Network,
) -> surf::Answer {
    let from_public = FIRST_REQUEST.with(|first| first.from_public.replace(false));
    let origin = if from_public {
        surf_fetch::Origin::PublicPage
    } else {
        surf_fetch::Origin::Given
    };
    let dialing = Dialing {
        network,
        dialed: Cell::new(false),
    };
    let result = surf::fetch_http_from(request, &surf_fetch::Limits::default(), &dialing, origin);
    if from_public
        && !dialing.dialed.get()
        && matches!(result, Err(surf_fetch::Failure::ForbiddenDestination(_)))
    {
        FIRST_REQUEST.with(|first| first.refused.set(true));
    }
    surf::answer(result)
}

/// What the session tells [`fetch`] about the run it starts, and what the fetch tells it back.
/// The session and its transport run on one thread, the transport inside the session's run.
struct FirstRequest {
    /// The next request is the first of a `follow` or a `press` from a page that counts as
    /// public; the fetch that reads it clears it.
    from_public: Cell<bool>,
    /// That request was refused `ForbiddenDestination` before any connection.
    refused: Cell<bool>,
}

thread_local! {
    static FIRST_REQUEST: FirstRequest = const {
        FirstRequest {
            from_public: Cell::new(false),
            refused: Cell::new(false),
        }
    };
}

/// A [`surf_fetch::Network`] that notes whether a connection was attempted.
struct Dialing<'a> {
    network: &'a dyn surf_fetch::Network,
    dialed: Cell<bool>,
}

impl surf_fetch::Network for Dialing<'_> {
    fn resolve(
        &self,
        host: &str,
        port: u16,
        timeout: std::time::Duration,
    ) -> Result<Vec<std::net::SocketAddr>, surf_fetch::Failure> {
        self.network.resolve(host, port, timeout)
    }

    fn connect(
        &self,
        address: &std::net::SocketAddr,
        timeout: std::time::Duration,
    ) -> io::Result<std::net::TcpStream> {
        self.dialed.set(true);
        self.network.connect(address, timeout)
    }
}

/// What one request comes to: the exit status, the answer lines as a typed answer, and the
/// status record of the link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    /// The exit status of the call, 0 to 3.
    pub status: u8,
    /// A sentence for standard error, when the session could not act.
    pub reason: Option<String>,
    /// The answer, in the grammar of `docs/agent-answer.md`.
    pub answer: Answer,
    /// `closed` once the session is closed, `unknown` for a refusal that carries no proof of
    /// the token ("Refusals" above), `open` otherwise.
    pub state: link::SessionState,
    /// The address of the page the session shows, if any.
    pub address: Option<String>,
    /// The action and event entries of the request, in their order ("The entries" above);
    /// [`Reply::to_link`] adds the answer entry after them. None when it recorded no action.
    pub entries: Vec<link::Entry>,
}

impl Reply {
    /// The answer of the link: the rendered lines, the status record and the entries, the last
    /// of them the answer entry, which is the rendered lines without the `token` line. The
    /// record's title is the one of the `page` line. The address is left out of the record when the answer would not fit the link with it; an
    /// answer the renderer refuses becomes status 3 with the head line alone, which is then its
    /// answer entry too.
    pub fn to_link(&self) -> link::Answer {
        let (status, reason, text) = match surf_answer::render(&self.answer) {
            Ok(text) => (self.status, self.reason.clone(), text),
            Err(_) => (
                3,
                Some("the session could not write its answer".to_string()),
                surf_answer::render(&Answer::new(self.answer.head.clone())).unwrap_or_default(),
            ),
        };
        let mut entries = self.entries.clone();
        if !entries.is_empty() {
            let kept = link::without_token(&text);
            if kept.is_empty() {
                entries.clear();
            } else {
                entries.push(link::Entry::answer(&kept));
            }
        }
        let answer = link::Answer {
            status,
            reason,
            state: self.state,
            address: self.address.as_deref().map(link::Quoted::new),
            title: match &self.answer.page {
                Some(Page::Loaded {
                    title: Some(title), ..
                }) => Some(link::Quoted::new(title)),
                _ => None,
            },
            entries,
            text,
        };
        match answer.encode() {
            Err(link::LinkError::TooLarge) if answer.address.is_some() => link::Answer {
                address: None,
                ..answer
            },
            _ => answer,
        }
    }
}

/// Why the session could not act on an accepted request: the system answered with an unmet
/// obligation, or with an outcome the session's own state rules out.
enum Stop {
    Unmet(UnmetObligation),
    Unexpected(&'static str),
}

impl From<UnmetObligation> for Stop {
    fn from(obligation: UnmetObligation) -> Self {
        Stop::Unmet(obligation)
    }
}

impl fmt::Display for Stop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stop::Unmet(obligation) => write!(f, "the session's system stopped: {obligation}"),
            Stop::Unexpected(what) => write!(f, "the session's system answered {what}"),
        }
    }
}

/// The lines of one rendering of a document, in one form, kept for the looks that page through
/// it ("The page as text" above).
struct Lines {
    lines: Vec<String>,
    /// The bytes of all the lines as body lines, as [`surf_answer::text_page`] counts them.
    bytes: u64,
    /// [`surf_page_text::markdown::Markdown::deep`]; 0 for the outline.
    deep: u64,
}

impl Lines {
    /// The whole rendering of `tree` in `form`: one walk of the tree.
    fn render(form: Form, tree: &surf::Tree) -> Lines {
        let (lines, deep): (Vec<String>, u64) = match form {
            Form::Tree => (surf_page_text::outline(tree).collect(), 0),
            Form::Markdown => {
                let markdown = surf_page_text::markdown::markdown(tree);
                let deep = u64::try_from(markdown.deep()).unwrap_or(u64::MAX);
                (markdown.collect(), deep)
            }
        };
        // Read from line `u64::MAX` on, the page keeps no line and counts every one.
        let bytes = surf_answer::text_page(form, lines.iter().cloned(), u64::MAX).bytes_total;
        Lines { lines, bytes, deep }
    }

    /// The page of the lines from line `from`, as [`surf_answer::text_page`] over all of them
    /// gives it: the page reads at most one line past the most it shows, so a look costs the
    /// lines it shows and not the whole rendering.
    fn page(&self, form: Form, from: u64) -> surf_answer::Text {
        let first = from.max(1);
        let start = usize::try_from(first - 1)
            .unwrap_or(usize::MAX)
            .min(self.lines.len());
        let shown = self.lines[start..]
            .iter()
            .take(surf_answer::TEXT_LINES_MAX + 1)
            .cloned();
        let mut text = surf_answer::text_page(form, shown, 1);
        text.first = first;
        text.total = u64::try_from(self.lines.len()).unwrap_or(u64::MAX);
        text.bytes_total = self.bytes;
        text
    }
}

/// The text of the current document, in each form a call asked for: the session renders a
/// document once per form, and drops it with every request but `look` (which changes no
/// document) and when the current document is another one.
struct Rendered {
    document: DocumentId,
    tree: Option<Lines>,
    markdown: Option<Lines>,
}

impl Rendered {
    fn of(document: DocumentId) -> Rendered {
        Rendered {
            document,
            tree: None,
            markdown: None,
        }
    }

    fn form(&self, form: Form) -> Option<&Lines> {
        match form {
            Form::Tree => self.tree.as_ref(),
            Form::Markdown => self.markdown.as_ref(),
        }
    }

    fn form_mut(&mut self, form: Form) -> &mut Option<Lines> {
        match form {
            Form::Tree => &mut self.tree,
            Form::Markdown => &mut self.markdown,
        }
    }
}

/// One actionable element of the current document, a link or a control, as the `now` block
/// shows it and `follow`, `fill` and `pick` take it ("The links" and "The controls" above). What
/// does not change while the document lives; a control's state is read from its row at each
/// answer.
#[derive(Clone)]
struct Link {
    /// `l<n>`, `b<n>` or `f<n>`, as [`surf_page::handles`] numbers it.
    handle: String,
    /// The element's row.
    element_id: ElementId,
    /// The element's kind.
    kind: ElementKind,
    /// The label of the element's row, the whole accessible name (at most
    /// [`surf_page::LABEL_BYTES_MAX`] bytes).
    label: String,
    /// The element or an ancestor has the `hidden` attribute.
    hidden: bool,
    /// A link's target resolved against the document base URL; `None` when it does not
    /// resolve, and for a control.
    resolved: Option<surf_url::Url>,
    /// What the `now` line of a link shows: the target's path, query and fragment when it has
    /// the document's origin, the whole target otherwise, the `href` as written when
    /// unresolved; empty for a control.
    shown: String,
}

/// The actionable elements of the current document, kept while it is current.
struct Links {
    document: DocumentId,
    links: Vec<Link>,
    /// The document's address has a host that is not local by its text ([`host_is_local`]):
    /// `follow` refuses a local target from it ("Following a link" above).
    from_public: bool,
}

/// One session: its identity, its token, the system it drives and its sequence number.
pub struct Session<T> {
    id: String,
    session_id: SessionId,
    token: Token,
    caller: Caller,
    browser: MemoryBrowser<T>,
    sequence: i64,
    opened: bool,
    /// The sequence numbers the request being answered recorded, in order.
    recorded: Vec<i64>,
    /// For each of them, how many events `System::published` held after the run that followed
    /// its `RecordAction`: where its events end.
    marks: Vec<usize>,
    /// The text of the current document, once a call asked for it.
    rendered: Option<Rendered>,
    /// The links of the current document, once an answer showed them.
    links: Option<Links>,
    /// The state before a `follow` or a `press` whose first request the fetch refused, kept
    /// while that navigation is the session's last ([`Session::run_from`]).
    kept: Option<Kept>,
    /// The navigations whose first request went out under the rule of a page that counts as
    /// public: a `follow` or a `press` from such a page ([`Session::run_from`]). An entry of the
    /// history has the identity of the navigation that made it, so `back` and `forward` fetch
    /// it again under the same rule (decision D279); an entry made by `go` is not here. A
    /// traversal under the rule is noted too; it makes no entry, so it is never looked up.
    from_public: Vec<RequestId>,
    /// What the last answer that showed the session's state showed, for the `changed` block of
    /// the next ("What changed" above).
    shown: Shown,
}

/// The page state an answer showed ("What changed" above): the load its page came from, the
/// address of its `page` line, its document and the state of each control of that document.
#[derive(Clone, Debug, Default)]
struct Shown {
    navigation: Option<RequestId>,
    address: Option<String>,
    document: Option<DocumentId>,
    controls: Vec<ShownControl>,
}

/// The state of one control: its handle, its kind, and from its rows its value, its checkedness
/// and the label of its selected option, each whole.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ShownControl {
    handle: String,
    kind: ElementKind,
    value: Option<String>,
    checked: bool,
    selected: Option<String>,
}

/// What the session showed before a navigation whose first request the fetch refused
/// `ForbiddenDestination`: its navigation, that navigation's state and the session's row. While
/// the refused navigation is the session's last, the page, the current document and the path
/// are read as they were, as after a `forbidden-destination` of the session's own checks.
#[derive(Clone, Debug)]
struct Kept {
    refused: Option<RequestId>,
    navigation: Option<RequestId>,
    navigation_state: Option<NavigationState>,
    row: Option<SessionStateView>,
}

impl<T> fmt::Debug for Session<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("id", &self.id)
            .field("token", &self.token)
            .field("sequence", &self.sequence)
            .field("opened", &self.opened)
            .finish_non_exhaustive()
    }
}

impl<T: Transport> Session<T> {
    /// A session of the identity `identity`, a UUID in lower-case text form, over `transport`,
    /// with a token drawn from the operating system's random source. Not open yet: the first
    /// `open` opens it. `InvalidInput` for an identity of another form; the error of the random
    /// source when it cannot be read.
    pub fn new(identity: &str, transport: T) -> io::Result<Session<T>> {
        if wire::session_id(identity).as_deref() != Some(identity) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the identity is not a UUID in lower-case text form",
            ));
        }
        let token = Token::draw()?;
        let caller = Caller::new();
        let browser =
            MemoryBrowser::from_ports(ComponentPorts::with_caller(caller.clone()), transport);
        Ok(Session {
            id: identity.to_string(),
            session_id: SessionId(Uuid(identity.to_string())),
            token,
            caller,
            browser,
            sequence: 0,
            opened: false,
            recorded: Vec::new(),
            marks: Vec::new(),
            rendered: None,
            links: None,
            kept: None,
            from_public: Vec::new(),
            shown: Shown::default(),
        })
    }

    /// The session's identity.
    pub fn identity(&self) -> &str {
        &self.id
    }

    /// Whether the model holds the session closed.
    pub fn is_closed(&self) -> bool {
        self.row()
            .is_some_and(|row| row.state == SessionState::Closed)
    }

    /// How much of its record the system behind the session holds ([`surf::Browser::held`]).
    /// Between commands it is nothing: [`Session::answer`] drops what the system delivered.
    pub fn held(&self) -> surf::Held {
        self.browser.held()
    }

    /// How many lines of the current document's text the session keeps between requests, in
    /// every form it rendered: 0 when it keeps none. Read-only, for the tests: the kept text is
    /// dropped by every request but `look` ([`Session::answer`]).
    pub fn kept_text_lines(&self) -> usize {
        self.rendered.as_ref().map_or(0, |kept| {
            kept.tree.as_ref().map_or(0, |lines| lines.lines.len())
                + kept.markdown.as_ref().map_or(0, |lines| lines.lines.len())
        })
    }

    /// The reply to `request`, with its entries ("The entries" above), which are read from what
    /// the system published while it was answered. Then everything the system delivered is
    /// dropped ([`surf::Browser::forget_delivered`]): the next answer reads only the views.
    pub fn answer(&mut self, request: &Request) -> Reply {
        let start = self.browser.system().published().len();
        self.recorded.clear();
        self.marks.clear();
        if request.command != "look" {
            self.rendered = None;
        }
        let mut reply = self.reply_to(request);
        reply.entries = self.entries(start);
        self.browser.forget_delivered();
        reply
    }

    /// The action and event entries of the request being answered: for each action it
    /// recorded, the action entry from the view `SessionLog`, then one event entry per event
    /// the system published from `start` up to the action's mark; the last action takes every
    /// event published after the mark before it. None when it recorded no action, or when an
    /// action it recorded is not in the view.
    fn entries(&self, start: usize) -> Vec<link::Entry> {
        if self.recorded.is_empty() {
            return Vec::new();
        }
        let system = self.browser.system();
        let published = system.published();
        let log = system
            .agent
            .session_log(self.session_id.clone())
            .unwrap_or_default();
        let mut actions = Vec::with_capacity(self.recorded.len());
        for sequence in &self.recorded {
            let row = log.iter().find(|row| row.sequence == *sequence);
            let Some(action) = row.and_then(|row| link::Entry::line(&entries::action_line(row)))
            else {
                return Vec::new();
            };
            actions.push(action);
        }
        // What the event entries may take: the bytes of the request's entries beside the answer
        // entry, less its action entries and a left-out line after each.
        let reserved: usize = actions
            .iter()
            .map(|action| action.text.len() + LEFT_OUT_LINE_MAX)
            .sum();
        let mut room = ACTION_AND_EVENT_BYTES_MAX.saturating_sub(reserved);
        let mut elements = 0;
        let mut out = Vec::new();
        let mut from = start.min(published.len());
        for (at, action) in actions.into_iter().enumerate() {
            out.push(action);
            let end = if at + 1 == self.recorded.len() {
                published.len()
            } else {
                self.marks
                    .get(at)
                    .copied()
                    .unwrap_or(published.len())
                    .clamp(from, published.len())
            };
            let mut left_out = 0u64;
            for event in &published[from..end] {
                let element = matches!(
                    event,
                    surf::SystemEvent::ElementAdded(_)
                        | surf::SystemEvent::FormAdded(_)
                        | surf::SystemEvent::OptionAdded(_)
                );
                if element && elements >= ELEMENT_ENTRIES_MAX {
                    left_out += 1;
                    continue;
                }
                let Some(entry) = link::Entry::line(&entries::event_line(event)) else {
                    continue;
                };
                if entry.text.len() > room {
                    left_out += 1;
                    continue;
                }
                room -= entry.text.len();
                elements += usize::from(element);
                out.push(entry);
            }
            if left_out > 0
                && let Some(line) = link::Entry::line(&entries::left_out_line(left_out))
            {
                out.push(line);
            }
            from = end;
        }
        out
    }

    fn reply_to(&mut self, request: &Request) -> Reply {
        if request.command == "open" {
            return self.open(request);
        }
        if !self.holds(request) {
            return self.bare(RefusalReason::Token);
        }
        self.caller.set(Some(self.session_id.clone()));
        let result = match request.command.as_str() {
            "go" => match (
                request.unknown_arg(&["url", "form", "line"]),
                request.arg("url"),
                as_text(request),
            ) {
                (None, Some(url), Ok(text)) => {
                    self.go(url).map(|reply| self.with_text(reply, text))
                }
                _ => Ok(self.with_state(RefusalReason::Usage)),
            },
            "look" => match (request.unknown_arg(&["form", "line"]), as_text(request)) {
                (None, Ok(text)) => {
                    let reply = self.state_reply(0, None);
                    Ok(self.with_text(reply, text))
                }
                _ => Ok(self.with_state(RefusalReason::Usage)),
            },
            "follow" => match (request.unknown_arg(&["handle"]), request.arg("handle")) {
                (None, Some(handle)) => self.follow(handle),
                _ => Ok(self.with_state(RefusalReason::Usage)),
            },
            "fill" => match (
                request.unknown_arg(&["handle", "value"]),
                request.arg("handle"),
                request.arg("value"),
            ) {
                (None, Some(handle), Some(value)) => self.fill(handle, value),
                _ => Ok(self.with_state(RefusalReason::Usage)),
            },
            "pick" => match (
                request.unknown_arg(&["handle", "option"]),
                request.arg("handle"),
                request.arg("option"),
            ) {
                (None, Some(handle), Some(option)) => self.pick(handle, option),
                _ => Ok(self.with_state(RefusalReason::Usage)),
            },
            "press" => match (request.unknown_arg(&["handle"]), request.arg("handle")) {
                (None, Some(handle)) => self.press(handle),
                _ => Ok(self.with_state(RefusalReason::Usage)),
            },
            "log" => match (request.unknown_arg(&["count"]), log_count(request)) {
                (None, Ok(count)) => {
                    let mut reply = self.state_reply(0, None);
                    reply.answer.path = Some(self.path_of(count, LOG_BYTES_MAX));
                    Ok(reply)
                }
                _ => Ok(self.with_state(RefusalReason::Usage)),
            },
            "back" if request.unknown_arg(&[]).is_none() => self.traverse(Standing::Back),
            "forward" if request.unknown_arg(&[]).is_none() => self.traverse(Standing::Forward),
            "close" if request.unknown_arg(&[]).is_none() => self.close(),
            _ => Ok(self.with_state(RefusalReason::Usage)),
        };
        result.unwrap_or_else(|stop| self.stopped(&stop))
    }

    /// `reply` with the page as text when the call asked for it (`--as`, "The page as text"
    /// above): `note text: no-css` and the `text` block of the current document, or `note text:
    /// no-document` when there is none. A refusal (a status other than 0) is left as it is.
    fn with_text(&mut self, mut reply: Reply, text: Option<AsText>) -> Reply {
        let Some(AsText { form, from_line }) = text else {
            return reply;
        };
        if reply.status != 0 {
            return reply;
        }
        // An answer with page text has no `now` block (`docs/design/page-as-text.md`).
        reply.answer.now = None;
        reply.answer.notes.retain(|note| *note != Note::Visibility);
        let Some(document) = self.current_document() else {
            reply.answer.notes.push(Note::Text(TextNote::NoDocument));
            return reply;
        };
        let mut kept = self
            .rendered
            .take()
            .filter(|kept| kept.document == document)
            .unwrap_or_else(|| Rendered::of(document));
        if kept.form(form).is_none() {
            let Some(tree) = self.browser.document_tree(&kept.document) else {
                reply.answer.notes.push(Note::Text(TextNote::NoDocument));
                return reply;
            };
            let lines = Lines::render(form, tree);
            *kept.form_mut(form) = Some(lines);
        }
        if let Some(lines) = kept.form(form) {
            reply.answer.notes.push(Note::Text(TextNote::NoCss));
            if lines.deep > 0 {
                reply
                    .answer
                    .notes
                    .push(Note::Text(TextNote::Deep(lines.deep)));
            }
            reply.answer.text = Some(lines.page(form, from_line));
        }
        self.rendered = Some(kept);
        reply
    }

    /// The document of the session's current navigation: the navigation is committed and its
    /// document parsed. `None` before the first navigation, after a failed one, and for a
    /// response that is not HTML.
    fn current_document(&self) -> Option<DocumentId> {
        self.current_row().map(|row| row.document_id)
    }

    /// The row of [`Session::current_document`]: its identity, address and title.
    fn current_row(&self) -> Option<Documents> {
        let system = self.browser.system();
        let (navigation, navigation_state) = self.navigation()?;
        if navigation_state != Some(NavigationState::Committed) {
            return None;
        }
        let navigation = navigation?;
        let document = system
            .document
            .documents()
            .ok()?
            .into_iter()
            .find(|document| {
                document.navigation_id == navigation && document.state == DocumentState::Parsed
            })?;
        self.browser
            .document_tree(&document.document_id)
            .map(|_| document)
    }

    /// The links of the current document, computed once per document ("The links" above);
    /// `None` without a current document.
    fn links(&mut self) -> Option<&[Link]> {
        let row = self.current_row()?;
        if self
            .links
            .as_ref()
            .is_none_or(|kept| kept.document != row.document_id)
        {
            let tree = self.browser.document_tree(&row.document_id)?;
            let mut rows = self
                .browser
                .system()
                .document
                .elements(row.document_id.clone())
                .unwrap_or_default();
            rows.sort_by_key(|element| element.position);
            let nodes = self
                .browser
                .element_nodes(&row.document_id)
                .unwrap_or_default();
            let links = document_links(tree, &row.url.0, &rows, nodes);
            let from_public =
                !surf_url::Url::parse(&row.url.0, None).is_ok_and(|page| host_is_local(&page));
            self.links = Some(Links {
                document: row.document_id,
                links,
                from_public,
            });
        }
        self.links.as_ref().map(|kept| kept.links.as_slice())
    }

    /// The title of the current document for the `page` line and the status record: `None`
    /// when it has none, an empty one, or one over [`TITLE_CHARS_MAX`] characters.
    fn title(&self) -> Option<String> {
        self.current_row()?
            .title
            .filter(|title| !title.is_empty() && title.chars().nth(TITLE_CHARS_MAX).is_none())
    }

    /// `follow <handle>`: refused `closed` on a closed session, `unknown-handle` when the
    /// current document has no link of that handle, `hidden` when the link or an ancestor has
    /// the `hidden` attribute, `unresolved` when its target does not resolve,
    /// `address-too-long`, `unsupported-scheme` or `forbidden-destination` for a target
    /// "Following a link" names; otherwise `Follow` as the holder with the resolved address,
    /// and when it is `requested`, the run ([`Session::run_from`]): `forbidden-destination` when
    /// the fetch refused its first request, otherwise `RecordAction` `follow` with the address,
    /// the handle and the label.
    fn follow(&mut self, handle: &str) -> Result<Reply, Stop> {
        if self.is_closed() {
            return Ok(self.with_state(RefusalReason::Closed));
        }
        let Some(link) = self
            .links()
            .and_then(|links| links.iter().find(|link| link.handle == handle))
            .cloned()
        else {
            return Ok(self.with_state(RefusalReason::UnknownHandle));
        };
        let from_public = self.links.as_ref().is_none_or(|kept| kept.from_public);
        if link.kind != ElementKind::Link {
            return Ok(self.with_state(RefusalReason::WrongKind));
        }
        if link.hidden {
            return Ok(self.with_state(RefusalReason::Hidden));
        }
        let Some(target) = link.resolved.as_ref() else {
            return Ok(self.with_state(RefusalReason::Unresolved));
        };
        let url = target.href();
        if url.len() > surf_fetch::MAX_ADDRESS_BYTES {
            return Ok(self.with_state(RefusalReason::AddressTooLong));
        }
        if target.scheme() != "http" {
            return Ok(self.with_state(RefusalReason::UnsupportedScheme));
        }
        if from_public && host_is_local(target) {
            return Ok(self.with_state(RefusalReason::ForbiddenDestination));
        }
        let outcome = self.browser.agent_mut().follow(Follow {
            session_id: self.session_id.clone(),
            url: Url(url.clone()),
            handle: link.handle.clone(),
            label: link.label.clone(),
        })?;
        match outcome {
            FollowOutcome::Requested { .. } => {
                if self.run_from(from_public)? {
                    return Ok(self.with_state(RefusalReason::ForbiddenDestination));
                }
                self.record_and_run(
                    ActionCommand::Follow,
                    Some(&url),
                    Some((&link.handle, Some(&link.label))),
                )?;
                Ok(self.state_reply(
                    0,
                    Some(Did::Action {
                        command: "follow".to_string(),
                        argument: Some(handle.to_string()),
                    }),
                ))
            }
            FollowOutcome::Closed { .. } => Ok(self.with_state(RefusalReason::Closed)),
            FollowOutcome::NotHolder { .. } => Err(Stop::Unexpected("not-holder to the holder")),
            FollowOutcome::NoSuchSession { .. } => {
                Err(Stop::Unexpected("no such session to the open session"))
            }
        }
    }

    /// The `now` block of the current document, with each control's state read from its row
    /// now ("The controls" above); `None` without a current document or an actionable element.
    fn now(&mut self) -> Option<Now> {
        let states = self.control_states()?;
        self.links().and_then(|links| now_block(links, &states))
    }

    /// The state of each control of the current document, from the rows of its elements and of
    /// every select's options.
    fn control_states(&self) -> Option<HashMap<String, State>> {
        let row = self.current_row()?;
        let document = &self.browser.system().document;
        let rows = document.elements(row.document_id.clone()).ok()?;
        let options = document.options().ok()?;
        let mut states = control_states(&rows, &options);
        let page = surf_url::Url::parse(&row.url.0, None).ok();
        for (element_id, found) in self.browser.submits(&row.document_id).ok()? {
            if let Some(state) = states.get_mut(&element_id.0.0) {
                state.submits = Some(submits_part(&found, page.as_ref()));
            }
        }
        Some(states)
    }

    /// The row of the control `handle` of the current document, checked as "The controls"
    /// above orders the refusals for `command`: `unknown-handle`, `wrong-kind` (a kind not in
    /// `kinds`), `disabled`, then `hidden` (the row's flag, which the document's command reads).
    fn control(&mut self, handle: &str, kinds: &[ElementKind]) -> Result<Elements, RefusalReason> {
        let Some(link) = self
            .links()
            .and_then(|links| links.iter().find(|link| link.handle == handle))
            .cloned()
        else {
            return Err(RefusalReason::UnknownHandle);
        };
        let row = self
            .current_row()
            .and_then(|row| {
                self.browser
                    .system()
                    .document
                    .elements(row.document_id)
                    .ok()
            })
            .and_then(|rows| {
                rows.into_iter()
                    .find(|row| row.element_id == link.element_id)
            })
            .ok_or(RefusalReason::UnknownHandle)?;
        if !kinds.contains(&row.kind) {
            return Err(RefusalReason::WrongKind);
        }
        if row.disabled {
            return Err(RefusalReason::Disabled);
        }
        if row.hidden || link.hidden {
            return Err(RefusalReason::Hidden);
        }
        Ok(row)
    }

    /// `fill <handle> <value>`: refused `closed` on a closed session; `unknown-handle`,
    /// `wrong-kind` for a control that is not `text`, `password` or `textarea`, `disabled`,
    /// `hidden` and `readonly` from the control's row; otherwise `Fill` as the holder, and when
    /// it is `requested`, `RecordAction` `fill` with the handle alone and a run, in which the
    /// binding gives the control the value (`SetValue`). The value of a `text` or `password`
    /// control is sent without its line breaks ([`sanitized`]). The value is in no row of the
    /// session and no line of the answer.
    fn fill(&mut self, handle: &str, value: &str) -> Result<Reply, Stop> {
        if self.is_closed() {
            return Ok(self.with_state(RefusalReason::Closed));
        }
        let row = match self.control(
            handle,
            &[
                ElementKind::Text,
                ElementKind::Password,
                ElementKind::Textarea,
            ],
        ) {
            Ok(row) if row.readonly => return Ok(self.with_state(RefusalReason::Readonly)),
            Ok(row) => row,
            Err(reason) => return Ok(self.with_state(reason)),
        };
        let outcome = self.browser.agent_mut().fill(Fill {
            session_id: self.session_id.clone(),
            element_id: row.element_id,
            value: sanitized(row.kind, value),
            handle: handle.to_string(),
        })?;
        match outcome {
            FillOutcome::Requested { .. } => {
                self.record_and_run(ActionCommand::Fill, None, Some((handle, None)))?;
                Ok(self.acted("fill", handle))
            }
            FillOutcome::Closed { .. } => Ok(self.with_state(RefusalReason::Closed)),
            FillOutcome::NotHolder { .. } => Err(Stop::Unexpected("not-holder to the holder")),
            FillOutcome::NoSuchSession { .. } => {
                Err(Stop::Unexpected("no such session to the open session"))
            }
        }
    }

    /// `pick <handle> <option>`: refused `closed` on a closed session; `unknown-handle`,
    /// `wrong-kind` for a control that is not a `select`, `checkbox` or `radio`, `disabled` and
    /// `hidden` from the control's row; `unknown-option` for a select without an option of that
    /// label, then value, and for a word other than `on` or `off` on a checkbox or `on` on a
    /// radio button; `disabled` for a disabled option. A checkbox already in the state asked for
    /// is left as it is: `did pick`, status 0, no action. Otherwise `Pick` as the holder (with the
    /// option for a select), and when it is `requested`, `RecordAction` `pick` with the handle
    /// alone and a run, in which the binding checks the control (`Check`) or selects the option
    /// (`SelectOption`).
    fn pick(&mut self, handle: &str, word: &str) -> Result<Reply, Stop> {
        if self.is_closed() {
            return Ok(self.with_state(RefusalReason::Closed));
        }
        let row = match self.control(
            handle,
            &[
                ElementKind::Select,
                ElementKind::Checkbox,
                ElementKind::Radio,
            ],
        ) {
            Ok(row) => row,
            Err(reason) => return Ok(self.with_state(reason)),
        };
        let option_id = match row.kind {
            ElementKind::Select => {
                let mut options: Vec<Options> = self
                    .browser
                    .system()
                    .document
                    .options()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|option| option.select_id == row.element_id)
                    .collect();
                options.sort_by_key(|option| option.position);
                let found = options
                    .iter()
                    .find(|option| option.label == word)
                    .or_else(|| options.iter().find(|option| option.value == word));
                match found {
                    None => return Ok(self.with_state(RefusalReason::UnknownOption)),
                    Some(option) if option.disabled => {
                        return Ok(self.with_state(RefusalReason::Disabled));
                    }
                    Some(option) => Some(option.option_id.clone()),
                }
            }
            ElementKind::Checkbox => match word {
                "on" | "off" if row.checked == (word == "on") => {
                    return Ok(self.acted("pick", handle));
                }
                "on" | "off" => None,
                _ => return Ok(self.with_state(RefusalReason::UnknownOption)),
            },
            _ => match word {
                "on" => None,
                _ => return Ok(self.with_state(RefusalReason::UnknownOption)),
            },
        };
        let outcome = self.browser.agent_mut().pick(Pick {
            session_id: self.session_id.clone(),
            element_id: row.element_id,
            option_id,
            handle: handle.to_string(),
        })?;
        match outcome {
            PickOutcome::Requested { .. } => {
                self.record_and_run(ActionCommand::Pick, None, Some((handle, None)))?;
                Ok(self.acted("pick", handle))
            }
            PickOutcome::Closed { .. } => Ok(self.with_state(RefusalReason::Closed)),
            PickOutcome::NotHolder { .. } => Err(Stop::Unexpected("not-holder to the holder")),
            PickOutcome::NoSuchSession { .. } => {
                Err(Stop::Unexpected("no such session to the open session"))
            }
        }
    }

    /// `press <handle>` ("Pressing a button" above): refused `closed` on a closed session;
    /// `unknown-handle`, `wrong-kind` for an element that is not a button, `disabled` and
    /// `hidden` from its row; then, from the [`surf::Plan`] of the press, which computes the
    /// address the submission step would request, `no-default-action` for a plain or a reset
    /// button, `password-in-query` for a `GET` whose entry list holds a filled password, and
    /// `address-too-long`, `unsupported-scheme` and `forbidden-destination` for that address.
    /// Otherwise `Press` as the holder, and when it is `requested`, the run in which the
    /// submission starts and ends ([`Session::run_from`]): `forbidden-destination` when the fetch
    /// refused its first request; otherwise `RecordAction` `press` with the handle alone. With a
    /// navigation, `did press "<handle>"` and the page it landed on; abandoned, the word of its
    /// reason ([`end_reason`]), status 1.
    fn press(&mut self, handle: &str) -> Result<Reply, Stop> {
        if self.is_closed() {
            return Ok(self.with_state(RefusalReason::Closed));
        }
        let row = match self.control(handle, &[ElementKind::Button]) {
            Ok(row) => row,
            Err(reason) => return Ok(self.with_state(reason)),
        };
        let Some(plan) = self.browser.plan_submission(&row.element_id)? else {
            return Ok(self.with_state(RefusalReason::NoDefaultAction));
        };
        if !matches!(
            plan.button,
            surf_page::forms::ButtonType::Submit | surf_page::forms::ButtonType::Image
        ) {
            return Ok(self.with_state(RefusalReason::NoDefaultAction));
        }
        let from_public = self.links.as_ref().is_none_or(|kept| kept.from_public);
        if let Some(submits) = &plan.submits {
            if submits.method == surf::FormMethod::Get && plan.password {
                return Ok(self.with_state(RefusalReason::PasswordInQuery));
            }
            if let (Some(address), Ok(action)) = (&plan.address, &submits.action) {
                if address.len() > surf_fetch::MAX_ADDRESS_BYTES {
                    return Ok(self.with_state(RefusalReason::AddressTooLong));
                }
                if action.scheme() != "http" {
                    return Ok(self.with_state(RefusalReason::UnsupportedScheme));
                }
                if from_public && host_is_local(action) {
                    return Ok(self.with_state(RefusalReason::ForbiddenDestination));
                }
            }
        }
        let outcome = self.browser.agent_mut().press(Press {
            session_id: self.session_id.clone(),
            element_id: row.element_id,
            handle: handle.to_string(),
        })?;
        match outcome {
            PressOutcome::Requested { .. } => {
                let start = self.browser.system().published().len();
                if self.run_from(from_public)? {
                    return Ok(self.with_state(RefusalReason::ForbiddenDestination));
                }
                self.record_and_run(ActionCommand::Press, None, Some((handle, None)))?;
                let abandoned = self
                    .browser
                    .system()
                    .published()
                    .get(start..)
                    .unwrap_or(&[])
                    .iter()
                    .find_map(|event| match event {
                        surf::SystemEvent::SubmissionAbandoned(abandoned) => Some(abandoned.reason),
                        _ => None,
                    });
                Ok(match abandoned {
                    Some(reason) => self.with_state(end_reason(reason)),
                    None => self.acted("press", handle),
                })
            }
            PressOutcome::Closed { .. } => Ok(self.with_state(RefusalReason::Closed)),
            PressOutcome::NotHolder { .. } => Err(Stop::Unexpected("not-holder to the holder")),
            PressOutcome::NoSuchSession { .. } => {
                Err(Stop::Unexpected("no such session to the open session"))
            }
        }
    }

    /// `back` (`step` [`Standing::Back`]) or `forward` ([`Standing::Forward`]): refused `closed`
    /// on a closed session; otherwise the history of the session's tab is read from the
    /// navigator's view `Navigations` before anything is issued ("Back and forward" above): no
    /// entry at `step` is refused `at-start` or `at-end`, an entry made by `POST` `post-entry`.
    /// Otherwise `GoBack` or `GoForward` as the holder, and when it is `requested`, the run
    /// ([`Session::run_from`]) under the rule of the request that made the entry (decision
    /// D279): `forbidden-destination` when the fetch refused it, otherwise `RecordAction` `back`
    /// or `forward`.
    fn traverse(&mut self, step: Standing) -> Result<Reply, Stop> {
        let (command, at_edge, word) = match step {
            Standing::Forward => (ActionCommand::Forward, RefusalReason::AtEnd, "forward"),
            _ => (ActionCommand::Back, RefusalReason::AtStart, "back"),
        };
        if self.is_closed() {
            return Ok(self.with_state(RefusalReason::Closed));
        }
        let from_public = match self.entry_at(step)? {
            None => return Ok(self.with_state(at_edge)),
            Some((_, Method::Post)) => return Ok(self.with_state(RefusalReason::PostEntry)),
            Some((entry, Method::Get)) => {
                self.from_public.contains(&entry) || self.redirected_by_public(&entry)?
            }
        };
        let not_holder = Stop::Unexpected("not-holder to the holder");
        let no_session = Stop::Unexpected("no such session to the open session");
        let session_id = self.session_id.clone();
        let closed = match step {
            Standing::Forward => match self
                .browser
                .agent_mut()
                .go_forward(GoForward { session_id })?
            {
                GoForwardOutcome::Requested { .. } => false,
                GoForwardOutcome::Closed { .. } => true,
                GoForwardOutcome::NotHolder { .. } => return Err(not_holder),
                GoForwardOutcome::NoSuchSession { .. } => return Err(no_session),
            },
            _ => match self.browser.agent_mut().go_back(GoBack { session_id })? {
                GoBackOutcome::Requested { .. } => false,
                GoBackOutcome::Closed { .. } => true,
                GoBackOutcome::NotHolder { .. } => return Err(not_holder),
                GoBackOutcome::NoSuchSession { .. } => return Err(no_session),
            },
        };
        if closed {
            return Ok(self.with_state(RefusalReason::Closed));
        }
        if self.run_from(from_public)? {
            return Ok(self.with_state(RefusalReason::ForbiddenDestination));
        }
        self.record_and_run(command, None, None)?;
        Ok(self.state_reply(
            0,
            Some(Did::Action {
                command: word.to_string(),
                argument: None,
            }),
        ))
    }

    /// The identity and the method of the entry at `step` in the history of the session's tab,
    /// from the navigator's view `Navigations`; `None` when the tab has no entry there.
    fn entry_at(&self, step: Standing) -> Result<Option<(RequestId, Method)>, Stop> {
        let system = self.browser.system();
        let Some(tab_id) = system
            .agent
            .sessions()?
            .into_iter()
            .find(|row| row.session_id == self.session_id)
            .map(|row| row.tab_id)
        else {
            return Err(Stop::Unexpected("no row of the open session"));
        };
        Ok(system
            .navigator
            .navigations()?
            .into_iter()
            .find(|row| row.tab_id == tab_id && row.standing == Some(step))
            .map(|row| (row.navigation_id, row.method)))
    }

    /// Whether a redirect moved the navigation that made the entry `entry` from an address that
    /// counts as public ("Following a link" above) to another one (decision D283): its request
    /// in the view `Requests` asked for an address other than the one the entry committed at,
    /// and that address's host is not local by its text. The fetch keeps neither the addresses
    /// between the first and the last of a chain of redirects nor where it connected, so the
    /// first address of the chain stands for the one that redirected.
    fn redirected_by_public(&self, entry: &RequestId) -> Result<bool, Stop> {
        let system = self.browser.system();
        let Some(committed) = system
            .navigator
            .navigations()?
            .into_iter()
            .find(|row| row.navigation_id == *entry)
            .map(|row| row.url)
        else {
            return Ok(false);
        };
        let Some(asked) = system
            .net
            .requests()?
            .into_iter()
            .find(|row| row.request_id == *entry)
            .map(|row| row.url)
        else {
            return Ok(false);
        };
        Ok(asked != committed
            && surf_url::Url::parse(&asked.0, None).is_ok_and(|asked| !host_is_local(&asked)))
    }

    /// Status 0 with `did <command> "<handle>"` and the state.
    fn acted(&mut self, command: &str, handle: &str) -> Reply {
        self.state_reply(
            0,
            Some(Did::Action {
                command: command.to_string(),
                argument: Some(handle.to_string()),
            }),
        )
    }

    /// The reply to a request that did not decode: `did refused usage`, status 2, the head line
    /// and the `did` line only.
    pub fn malformed(&self) -> Reply {
        self.bare(RefusalReason::Usage)
    }

    /// Whether `request` names this session and carries its token, once it is open.
    fn holds(&self, request: &Request) -> bool {
        let named = request.session.as_deref().and_then(wire::session_id);
        self.opened
            && named.as_deref() == Some(self.id.as_str())
            && request
                .token
                .as_ref()
                .is_some_and(|token| token.matches(&self.token))
    }

    fn open(&mut self, request: &Request) -> Reply {
        let url = request.arg("url");
        let text = as_text(request);
        if self.opened
            || request.session.is_some()
            || request.token.is_some()
            || request.unknown_arg(&["url", "form", "line"]).is_some()
            || text.is_err()
            || (url.is_none() && text.is_ok_and(|text| text.is_some()))
        {
            return self.bare(RefusalReason::Usage);
        }
        match self.open_session(url) {
            Ok(()) => {
                let mut reply = self.state_reply(
                    0,
                    Some(Did::Action {
                        command: "open".to_string(),
                        argument: url.map(str::to_string),
                    }),
                );
                reply.answer.token = Some(self.token.to_hex());
                self.with_text(reply, text.unwrap_or_default())
            }
            Err(stop) => self.stopped(&stop),
        }
    }

    fn open_session(&mut self, url: Option<&str>) -> Result<(), Stop> {
        let window_id = match self.browser.shell_mut().open_window(OpenWindow {})? {
            OpenWindowOutcome::Opened { window_opened } => window_opened.window_id,
        };
        self.browser.run()?;
        let tab_id = match self.browser.shell_mut().open_tab(OpenTab {
            window_id: window_id.clone(),
        })? {
            OpenTabOutcome::Opened { tab_opened } => tab_opened.tab_id,
            OpenTabOutcome::NoSuchWindow { .. } => {
                return Err(Stop::Unexpected("no such window to the window it opened"));
            }
        };
        self.browser.run()?;
        self.caller.set(None);
        let outcome = self.browser.agent_mut().open_session(OpenSession {
            session_id: self.session_id.clone(),
            window_id,
            tab_id,
        })?;
        match outcome {
            OpenSessionOutcome::Opened { .. } => {}
            OpenSessionOutcome::Exists { .. } => {
                return Err(Stop::Unexpected("that the session exists before its open"));
            }
        }
        self.opened = true;
        self.record_and_run(ActionCommand::Open, None, None)?;
        self.caller.set(Some(self.session_id.clone()));
        if let Some(url) = url {
            match self.request_go(url)? {
                GoOutcome::Requested { .. } => {}
                _ => return Err(Stop::Unexpected("a refusal of go to the session it opened")),
            }
        }
        Ok(())
    }

    /// `Go` as the holder; when it is `requested`, records it and runs.
    fn request_go(&mut self, url: &str) -> Result<GoOutcome, Stop> {
        let outcome = self.browser.agent_mut().go(Go {
            session_id: self.session_id.clone(),
            url: Url(url.to_string()),
        })?;
        if let GoOutcome::Requested { .. } = outcome {
            self.record_and_run(ActionCommand::Go, Some(url), None)?;
        }
        Ok(outcome)
    }

    fn go(&mut self, url: &str) -> Result<Reply, Stop> {
        match self.request_go(url)? {
            GoOutcome::Requested { .. } => Ok(self.state_reply(
                0,
                Some(Did::Action {
                    command: "go".to_string(),
                    argument: Some(url.to_string()),
                }),
            )),
            GoOutcome::Closed { .. } => Ok(self.with_state(RefusalReason::Closed)),
            GoOutcome::NotHolder { .. } => Err(Stop::Unexpected("not-holder to the holder")),
            GoOutcome::NoSuchSession { .. } => {
                Err(Stop::Unexpected("no such session to the open session"))
            }
        }
    }

    fn close(&mut self) -> Result<Reply, Stop> {
        let outcome = self.browser.agent_mut().close_session(CloseSession {
            session_id: self.session_id.clone(),
        })?;
        match outcome {
            CloseSessionOutcome::Closed { .. } => {
                self.record_and_run(ActionCommand::Close, None, None)?;
                Ok(self.state_reply(
                    0,
                    Some(Did::Action {
                        command: "close".to_string(),
                        argument: None,
                    }),
                ))
            }
            CloseSessionOutcome::WrongState { .. } => Ok(self.with_state(RefusalReason::Closed)),
            CloseSessionOutcome::NotHolder { .. } => {
                Err(Stop::Unexpected("not-holder to the holder"))
            }
            CloseSessionOutcome::NoSuchSession { .. } => {
                Err(Stop::Unexpected("no such session to the open session"))
            }
        }
    }

    /// [`Session::record`], then the run that delivers what the command published, then the
    /// mark of where the action's events end.
    fn record_and_run(
        &mut self,
        command: ActionCommand,
        url: Option<&str>,
        element: Option<(&str, Option<&str>)>,
    ) -> Result<(), Stop> {
        self.record(command, url, element)?;
        self.browser.run()?;
        self.marks.push(self.browser.system().published().len());
        Ok(())
    }

    /// `RecordAction` with the next sequence number, on the `agent` port; `element` is the
    /// handle of the element the action took and its label, for an action on one.
    fn record(
        &mut self,
        command: ActionCommand,
        url: Option<&str>,
        element: Option<(&str, Option<&str>)>,
    ) -> Result<(), Stop> {
        let sequence = self.sequence + 1;
        let outcome = self.browser.agent_mut().record_action(RecordAction {
            session_id: self.session_id.clone(),
            sequence,
            command,
            url: url.map(|url| Url(url.to_string())),
            handle: element.map(|(handle, _)| handle.to_string()),
            label: element.and_then(|(_, label)| label.map(str::to_string)),
        })?;
        match outcome {
            RecordActionOutcome::Recorded { .. } => {
                self.sequence = sequence;
                self.recorded.push(sequence);
                Ok(())
            }
            RecordActionOutcome::NoSuchSession { .. } => Err(Stop::Unexpected(
                "no such session to the action of the open session",
            )),
        }
    }

    /// The session's row of the view `SessionState`, if it is open or was; while a navigation
    /// whose first request the fetch refused is the last, its address, navigation state,
    /// status, media type and failure reason are those before it ([`Kept`]).
    fn row(&self) -> Option<SessionStateView> {
        let row = self
            .browser
            .system()
            .agent
            .session_state_view(self.session_id.clone())
            .ok()?
            .into_iter()
            .next()?;
        match self.kept_now().and_then(|kept| kept.row.as_ref()) {
            Some(kept) => Some(SessionStateView {
                state: row.state,
                ..kept.clone()
            }),
            None => Some(row),
        }
    }

    /// The session's navigation and its state from the view `Sessions`, or those before a
    /// refused first request ([`Kept`]); `None` when the view has no row of the session.
    fn navigation(&self) -> Option<(Option<RequestId>, Option<NavigationState>)> {
        let row = self
            .browser
            .system()
            .agent
            .sessions()
            .ok()?
            .into_iter()
            .find(|row| row.session_id == self.session_id)?;
        Some(match &self.kept {
            Some(kept) if kept.refused == row.navigation_id => {
                (kept.navigation.clone(), kept.navigation_state)
            }
            _ => (row.navigation_id, row.navigation_state),
        })
    }

    /// [`Session::kept`] while the navigation it was kept for is the session's last.
    fn kept_now(&self) -> Option<&Kept> {
        let kept = self.kept.as_ref()?;
        let last = self
            .browser
            .system()
            .agent
            .sessions()
            .ok()?
            .into_iter()
            .find(|row| row.session_id == self.session_id)?
            .navigation_id;
        (kept.refused == last).then_some(kept)
    }

    /// The run of a `follow`, a `press`, a `back` or a `forward` before anything is recorded,
    /// with the session's transport told whether the first request goes out under the rule of a
    /// page that counts as public ("Following a link" above, "Back and forward"): `true` when the
    /// fetch refused that request `ForbiddenDestination` before any connection. Then the state
    /// before the run is kept ([`Kept`]) and the caller records nothing, as for a refusal of its
    /// own checks. Otherwise, under that rule, the navigation it started is noted in
    /// [`Session::from_public`], so that the entry it makes keeps the rule.
    fn run_from(&mut self, from_public: bool) -> Result<bool, Stop> {
        let (navigation, navigation_state) = self.navigation().unwrap_or((None, None));
        let row = self.row();
        FIRST_REQUEST.with(|first| {
            first.from_public.set(from_public);
            first.refused.set(false);
        });
        let ran = self.browser.run();
        let refused = FIRST_REQUEST.with(|first| {
            first.from_public.set(false);
            first.refused.replace(false)
        });
        ran?;
        if refused {
            let refused = self
                .browser
                .system()
                .agent
                .sessions()
                .ok()
                .and_then(|rows| {
                    rows.into_iter()
                        .find(|row| row.session_id == self.session_id)
                })
                .and_then(|row| row.navigation_id);
            self.kept = Some(Kept {
                refused,
                navigation,
                navigation_state,
                row,
            });
        } else if from_public
            && let Some(started) = self
                .browser
                .system()
                .agent
                .sessions()
                .ok()
                .and_then(|rows| {
                    rows.into_iter()
                        .find(|row| row.session_id == self.session_id)
                })
                .and_then(|row| row.navigation_id)
            && !self.from_public.contains(&started)
        {
            self.from_public.push(started);
        }
        Ok(refused)
    }

    fn head(&self) -> Head {
        let state = match self.row().map(|row| row.state) {
            Some(SessionState::Closed) => HeadState::Closed,
            Some(SessionState::Open) | None => HeadState::Open,
        };
        Head {
            session: self.id.clone(),
            state,
        }
    }

    /// The status record's state and address.
    fn record_fields(&self) -> (link::SessionState, Option<String>) {
        match self.row() {
            Some(row) => (
                match row.state {
                    SessionState::Closed => link::SessionState::Closed,
                    SessionState::Open => link::SessionState::Open,
                },
                row.url.map(|url| url.0),
            ),
            None => (link::SessionState::Open, None),
        }
    }

    /// The head line with the state `unknown`, `did refused <reason>` and nothing else; the
    /// status of the reason. The status record holds nothing of the session either: the state
    /// [`link::SessionState::Unknown`], no address and no title, so that `surfd` keeps what it
    /// knew before the request ("Refusals" above).
    fn bare(&self, reason: RefusalReason) -> Reply {
        let mut answer = Answer::new(Head {
            session: self.id.clone(),
            state: HeadState::Unknown,
        });
        answer.did = Some(Did::Refused(reason));
        Reply {
            status: refusal_status(reason),
            reason: None,
            answer,
            state: link::SessionState::Unknown,
            address: None,
            entries: Vec::new(),
        }
    }

    /// `did refused <reason>` with the page and the path.
    fn with_state(&mut self, reason: RefusalReason) -> Reply {
        self.state_reply(refusal_status(reason), Some(Did::Refused(reason)))
    }

    /// The head line, `did` when there is one, the page and path from the views, the `changed`
    /// block against the state the previous answer showed ("What changed" above), and for the
    /// current document of an open session its `note parser:` line, its `now` block and `note
    /// visibility:` ("The links" above). The title of the current document is on a loaded
    /// `page` line.
    fn state_reply(&mut self, status: u8, did: Option<Did>) -> Reply {
        let mut answer = Answer::new(self.head());
        answer.did = did;
        let row = self.row();
        answer.page = Some(row.as_ref().map_or(Page::None, page));
        if let Some(Page::Loaded { title, .. }) = &mut answer.page {
            *title = self.title();
        }
        answer.path = Some(self.path());
        let shown = self.shown_now(answer.page.as_ref());
        answer.changed = Some(self.changed(&shown));
        self.shown = shown;
        if self.current_document().is_some() {
            let parts = self.browser.parser_not_yet_parsed();
            if !parts.is_empty() {
                answer.notes.push(Note::Parser(
                    parts.iter().map(|part| part.to_string()).collect(),
                ));
            }
            if !self.is_closed()
                && let Some(now) = self.now()
            {
                answer.now = Some(now);
                answer.notes.push(Note::Visibility);
            }
        }
        let (state, address) = self.record_fields();
        Reply {
            status,
            reason: None,
            answer,
            state,
            address,
            entries: Vec::new(),
        }
    }

    /// The last [`PATH_SHOWN`] actions of the log, oldest first, and their total.
    fn path(&self) -> Path {
        self.path_of(Some(PATH_SHOWN), usize::MAX)
    }

    /// The last `last` actions of the log, or all of them for `None`, oldest first, as many of
    /// the newest of them as fit `bytes` bytes of lines counted high ([`PATH_LINE_FIXED`]), and
    /// their total ("The log" above).
    fn path_of(&self, last: Option<usize>, bytes: usize) -> Path {
        let mut log = self
            .browser
            .system()
            .agent
            .session_log(self.session_id.clone())
            .unwrap_or_default();
        log.sort_by_key(|row| row.sequence);
        let total = log.len();
        let mut room = bytes;
        let mut first = total.saturating_sub(last.unwrap_or(total));
        for (at, row) in log.iter().enumerate().skip(first).rev() {
            let argument = match (&row.handle, &row.label, &row.url) {
                (Some(_), Some(label), _) => {
                    surf_answer::quote(&Cut::of(label, LABEL_CHARS_MAX).kept).len()
                }
                (None, _, Some(url)) => surf_answer::quote(&url.0).len(),
                _ => 0,
            };
            let line = PATH_LINE_FIXED + row.handle.as_ref().map_or(0, String::len) + argument;
            if line > room {
                first = at + 1;
                break;
            }
            room -= line;
        }
        let entries = log
            .into_iter()
            .skip(first)
            .map(|row| {
                let sequence = u64::try_from(row.sequence).unwrap_or_default();
                let command = command_word(row.command).to_string();
                match row.handle {
                    // An action on an element: its handle and its label, cut as the `now` block
                    // cuts it.
                    Some(handle) => {
                        let label = row.label.map(|label| Cut::of(&label, LABEL_CHARS_MAX));
                        PathEntry {
                            sequence,
                            command,
                            handle: Some(handle),
                            argument_cut: label.as_ref().is_some_and(|label| label.cut),
                            argument: label.map(|label| label.kept),
                        }
                    }
                    None => PathEntry {
                        sequence,
                        command,
                        handle: None,
                        argument: row.url.map(|url| url.0),
                        argument_cut: false,
                    },
                }
            })
            .collect();
        Path {
            total: u64::try_from(total).unwrap_or(u64::MAX),
            entries,
        }
    }

    /// What an answer with the `page` line `page` shows now ("What changed" above).
    fn shown_now(&mut self, page: Option<&Page>) -> Shown {
        let address = match page {
            Some(Page::Loaded { address, .. } | Page::Failed { address, .. }) => {
                Some(address.clone())
            }
            _ => None,
        };
        Shown {
            navigation: self.navigation().and_then(|(navigation, _)| navigation),
            address,
            document: self.current_document(),
            controls: self.controls_shown(),
        }
    }

    /// The state of each control of the current document, in document order, from its row and,
    /// for a select, from its option rows; none without a current document.
    fn controls_shown(&mut self) -> Vec<ShownControl> {
        let Some(row) = self.current_row() else {
            return Vec::new();
        };
        let document = &self.browser.system().document;
        let mut rows: HashMap<String, Elements> = document
            .elements(row.document_id)
            .unwrap_or_default()
            .into_iter()
            .map(|row| (row.element_id.0.0.clone(), row))
            .collect();
        let options = document.options().unwrap_or_default();
        let selected: HashMap<String, String> = selected_labels(&options)
            .into_iter()
            .map(|(select, label)| (select.to_string(), label.to_string()))
            .collect();
        let Some(links) = self.links() else {
            return Vec::new();
        };
        links
            .iter()
            .filter(|link| !matches!(link.kind, ElementKind::Link | ElementKind::Button))
            .filter_map(|link| {
                let row = rows.remove(&link.element_id.0.0)?;
                Some(ShownControl {
                    handle: link.handle.clone(),
                    kind: row.kind,
                    value: row.value,
                    checked: row.checked,
                    selected: selected.get(&link.element_id.0.0).cloned(),
                })
            })
            .collect()
    }

    /// The `changed` block from [`Session::shown`] to `now` ("What changed" above): `changed
    /// page` when they show the pages of two loads, otherwise a line per control whose state
    /// changed.
    fn changed(&self, now: &Shown) -> Changed {
        let before = &self.shown;
        if before.navigation != now.navigation {
            let (texts, more) = self.new_texts(before.document.as_ref(), now.document.as_ref());
            return Changed::Page {
                from: before.address.clone(),
                to: now.address.clone(),
                texts,
                more,
            };
        }
        let earlier: HashMap<&str, &ShownControl> = before
            .controls
            .iter()
            .map(|control| (control.handle.as_str(), control))
            .collect();
        Changed::Controls(
            now.controls
                .iter()
                .filter_map(|control| {
                    let change = control_change(earlier.get(control.handle.as_str())?, control)?;
                    Some(ControlChange {
                        handle: control.handle.clone(),
                        change,
                    })
                })
                .collect(),
        )
    }

    /// The texts of the document `new` that the document `old` does not hold: the lines of the
    /// Markdown rendering of each ([`surf_page_text::markdown::markdown`]) but the empty ones,
    /// each once, in order; the first [`CHANGED_TEXTS_MAX`] cut at [`CHANGED_TEXT_CHARS_MAX`]
    /// characters, and how many more there are.
    fn new_texts(&self, old: Option<&DocumentId>, new: Option<&DocumentId>) -> (Vec<Cut>, u64) {
        let Some(tree) = new.and_then(|document| self.browser.document_tree(document)) else {
            return (Vec::new(), 0);
        };
        let held: HashSet<String> = old
            .and_then(|document| self.browser.document_tree(document))
            .map(|old| markdown_texts(old).collect())
            .unwrap_or_default();
        let mut seen = HashSet::new();
        let mut texts = Vec::new();
        let mut more = 0u64;
        for line in markdown_texts(tree) {
            if held.contains(&line) || !seen.insert(line.clone()) {
                continue;
            }
            if texts.len() < CHANGED_TEXTS_MAX {
                texts.push(Cut::of(&line, CHANGED_TEXT_CHARS_MAX));
            } else {
                more += 1;
            }
        }
        (texts, more)
    }

    /// Status 3 with the reason, the head line alone.
    fn stopped(&self, stop: &Stop) -> Reply {
        let (state, address) = self.record_fields();
        Reply {
            status: 3,
            reason: Some(stop.to_string()),
            answer: Answer::new(self.head()),
            state,
            address,
            entries: Vec::new(),
        }
    }
}

/// The exit status of a refusal, as the reason table of `docs/agent-answer.md` gives it.
fn refusal_status(reason: RefusalReason) -> u8 {
    match reason {
        RefusalReason::Token
        | RefusalReason::Closed
        | RefusalReason::UnknownHandle
        | RefusalReason::Hidden
        | RefusalReason::Unresolved
        | RefusalReason::AddressTooLong
        | RefusalReason::UnsupportedScheme
        | RefusalReason::ForbiddenDestination
        | RefusalReason::Disabled
        | RefusalReason::WrongKind
        | RefusalReason::UnknownOption
        | RefusalReason::Readonly
        | RefusalReason::NoDefaultAction
        | RefusalReason::PasswordInQuery
        | RefusalReason::NoFormOwner
        | RefusalReason::InvalidControls
        | RefusalReason::InvalidAction
        | RefusalReason::UnsupportedEnctype
        | RefusalReason::DialogMethod
        | RefusalReason::AtStart
        | RefusalReason::AtEnd
        | RefusalReason::PostEntry => 1,
        RefusalReason::Usage => 2,
        RefusalReason::NoSuchSession
        | RefusalReason::SessionDied
        | RefusalReason::SessionTimeout => 3,
    }
}

/// What `--as` asked for: the form and the first line ("The page as text" above).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AsText {
    form: Form,
    from_line: u64,
}

/// The `arg form` and `arg line` of `request` (`surf_agent::wire`): `None` without `form`; `Err`
/// for a form other than `tree` or `markdown`, for `line` without `form`, and for a `line` that is
/// not a number from 1 in decimal digits. Without `line` the first line is 1.
fn as_text(request: &Request) -> Result<Option<AsText>, ()> {
    let line = request.arg("line");
    let Some(form) = request.arg("form") else {
        return if line.is_none() { Ok(None) } else { Err(()) };
    };
    let form = match form {
        "tree" => Form::Tree,
        "markdown" => Form::Markdown,
        _ => return Err(()),
    };
    let from_line = match line {
        None => 1,
        Some(digits) => digits
            .bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| digits.parse::<u64>().ok())
            .flatten()
            .filter(|line| *line >= 1)
            .ok_or(())?,
    };
    Ok(Some(AsText { form, from_line }))
}

/// The word of an action's command on a path line, and its variant name in `spec/`.
fn command_word(command: ActionCommand) -> &'static str {
    match command {
        ActionCommand::Open => "open",
        ActionCommand::Go => "go",
        ActionCommand::Close => "close",
        ActionCommand::Follow => "follow",
        ActionCommand::Fill => "fill",
        ActionCommand::Pick => "pick",
        ActionCommand::Press => "press",
        ActionCommand::Back => "back",
        ActionCommand::Forward => "forward",
    }
}

/// The `page` line of a `SessionState` row: `page none` before the first navigation and while
/// one has not ended, the status, media type and address of a committed one, the reason and
/// address of a failed one. The reason is the name of the `surf.net.FailureReason` variant, its
/// `Debug` form, so a variant a specification story adds is written without a change here.
fn page(row: &SessionStateView) -> Page {
    let Some(url) = &row.url else {
        return Page::None;
    };
    match row.navigation_state {
        Some(NavigationState::Committed) => {
            let status = row
                .status
                .as_ref()
                .and_then(|status| u16::try_from(status.0).ok());
            match (status, &row.media_type) {
                (Some(status), Some(media_type)) => Page::Loaded {
                    status,
                    media_type: media_type.0.clone(),
                    address: url.0.clone(),
                    title: None,
                },
                _ => Page::None,
            }
        }
        Some(NavigationState::Failed) => match row.failure_reason {
            Some(reason) => Page::Failed {
                reason: format!("{reason:?}"),
                address: url.0.clone(),
            },
            None => Page::None,
        },
        _ => Page::None,
    }
}

/// The actionable elements of a document from its element rows (`rows`, in position order) and
/// the node of each (`nodes`, [`surf::Browser::element_nodes`]): the handle
/// [`surf_page::handles`] gives its node, the row's kind and label, whether it or an ancestor
/// has the `hidden` attribute, and for a link its target resolved against the document base URL
/// of `tree` at `address`.
fn document_links(
    tree: &Tree,
    address: &str,
    rows: &[Elements],
    nodes: &[(surf_types::document::ElementId, NodeId)],
) -> Vec<Link> {
    let handles: HashMap<NodeId, String> = surf_page::handles(tree).into_iter().collect();
    let hidden = hidden_by_attribute(tree);
    let page = surf_url::Url::parse(address, None).ok();
    let base = page
        .as_ref()
        .map(|page| surf_page::base_url::document_base_url(tree, page));
    let node_of: HashMap<&str, NodeId> = nodes
        .iter()
        .map(|(element, node)| (element.0.0.as_str(), *node))
        .collect();
    let mut links = Vec::new();
    for row in rows {
        let href = row
            .target
            .as_ref()
            .filter(|_| row.kind == ElementKind::Link);
        if row.kind == ElementKind::Link && href.is_none() {
            continue;
        }
        let Some(node) = node_of.get(row.element_id.0.0.as_str()) else {
            continue;
        };
        let Some(handle) = handles.get(node) else {
            continue;
        };
        let resolved = href.and_then(|href| surf_url::Url::parse(href, base.as_ref()).ok());
        let shown = match (&resolved, href) {
            (Some(target), _) => shown_target(target, page.as_ref()),
            (None, Some(href)) => href.clone(),
            (None, None) => String::new(),
        };
        links.push(Link {
            handle: handle.clone(),
            element_id: row.element_id.clone(),
            kind: row.kind,
            label: row.label.clone(),
            hidden: hidden.contains(node),
            resolved,
            shown,
        });
    }
    links
}

/// The nodes that are or are under an HTML element with the `hidden` attribute: the one rule of
/// `hid` and of the refusal `hidden` (`story:agent-follow-links`). `aria-hidden`, a closed
/// `dialog` or `details` and a popover do not count. One walk with its own stack.
fn hidden_by_attribute(tree: &Tree) -> HashSet<NodeId> {
    let mut hidden = HashSet::new();
    let mut pending = vec![(tree.root(), false)];
    while let Some((node, above)) = pending.pop() {
        let here = above
            || matches!(
                tree.data(node),
                surf_html::NodeData::Element { namespace: surf_html::Namespace::Html, attributes, .. }
                    if attributes
                        .iter()
                        .any(|a| a.namespace.is_none() && a.local_name == "hidden")
            );
        if here {
            hidden.insert(node);
        }
        pending.extend(tree.children(node).iter().map(|&child| (child, here)));
    }
    hidden
}

/// A resolved target as the `now` line writes it: its path, query and fragment when it has the
/// origin of `page` and its path does not start with `//`, which reads as another host; the
/// whole address without its userinfo otherwise, so that scheme, host and port come first and
/// a cut at [`TARGET_CHARS_MAX`] keeps them whole (a host alone over the limit aside).
fn shown_target(target: &surf_url::Url, page: Option<&surf_url::Url>) -> String {
    let path = target.serialize_path();
    if !page.is_some_and(|page| page.origin().same_origin(&target.origin()))
        || path.starts_with("//")
    {
        return without_userinfo(target);
    }
    let mut shown = path;
    if let Some(query) = target.query() {
        shown.push('?');
        shown.push_str(query);
    }
    if let Some(fragment) = target.fragment() {
        shown.push('#');
        shown.push_str(fragment);
    }
    shown
}

/// The URL serializer's output for `url` without the userinfo: `user:password@` follows
/// `<scheme>://` and is left out; a URL without one is its `href`.
fn without_userinfo(url: &surf_url::Url) -> String {
    let mut href = url.href();
    let (username, password) = (url.username(), url.password());
    if url.host().is_some() && !(username.is_empty() && password.is_empty()) {
        let start = url.scheme().len() + "://".len();
        let length = username.len()
            + if password.is_empty() {
                0
            } else {
                1 + password.len()
            }
            + 1;
        href.replace_range(start..start + length, "");
    }
    href
}

/// Whether the host of `url` is local by its text: an IP address [`surf_fetch::is_local`]
/// names (the rule of a redirect's destination), or the name `localhost` or one under it, which
/// the operating system resolves to a loopback address. Any other name is not: where it leads
/// is known only when the fetch resolves it.
fn host_is_local(url: &surf_url::Url) -> bool {
    match url.host() {
        Some(surf_url::Host::Ipv4(address)) => {
            surf_fetch::is_local(std::net::IpAddr::V4(std::net::Ipv4Addr::from(*address)))
        }
        Some(surf_url::Host::Ipv6(pieces)) => {
            surf_fetch::is_local(std::net::IpAddr::V6(std::net::Ipv6Addr::from(*pieces)))
        }
        Some(surf_url::Host::Domain(name)) => {
            let name = name.strip_suffix('.').unwrap_or(name);
            name == "localhost" || name.ends_with(".localhost")
        }
        _ => false,
    }
}

/// The `now` block of `links`: a line per element in document order while the lines fit
/// [`NOW_BYTES_MAX`], the label cut at [`LABEL_CHARS_MAX`] characters, a link's target at
/// [`TARGET_CHARS_MAX`] and a control's value at [`VALUE_CHARS_MAX`]; a control's state is its
/// entry in `states`, and nothing when it has none. `None` for a document without an actionable
/// element.
fn now_block(links: &[Link], states: &HashMap<String, State>) -> Option<Now> {
    if links.is_empty() {
        return None;
    }
    let total = u64::try_from(links.len()).unwrap_or(u64::MAX);
    let mut bytes = format!("now {total} of {total}\n").len();
    let mut elements = Vec::new();
    for link in links {
        let label = Cut::of(&link.label, LABEL_CHARS_MAX);
        let fixed = NOW_LINE_FIXED + link.handle.len() + surf_answer::quote(&label.kept).len();
        let (line, target) = if link.kind == ElementKind::Link {
            let target = Cut::of(&link.shown, TARGET_CHARS_MAX);
            let line = fixed + surf_answer::quote(&target.kept).len();
            let target = if link.resolved.is_some() {
                Target::Resolved(target)
            } else {
                Target::Unresolved(target)
            };
            (line, target)
        } else {
            let state = states
                .get(&link.element_id.0.0)
                .cloned()
                .unwrap_or_default();
            let value = match &state.value {
                Some(Value::Text(value)) => surf_answer::quote(&value.kept).len(),
                _ => 0,
            };
            let submits = state.submits.as_ref().map_or(0, |submits| {
                SUBMITS_FIXED
                    + submits.form.len()
                    + submits
                        .action
                        .as_ref()
                        .map_or(0, |action| surf_answer::quote(&action.kept).len())
            });
            (
                fixed + CONTROL_LINE_FIXED + value + submits,
                Target::State(state),
            )
        };
        if bytes + line > NOW_BYTES_MAX {
            break;
        }
        bytes += line;
        elements.push(Element {
            handle: link.handle.clone(),
            kind: kind_word(link.kind).to_string(),
            visibility: if link.hidden {
                Visibility::Hidden
            } else {
                Visibility::Unknown
            },
            label,
            target,
        });
    }
    Some(Now { total, elements })
}

/// The value `fill` sends for a control of `kind`: for `text` and `password`, the kinds of a
/// single-line `input`, `value` with every U+000A and U+000D removed, the value sanitization
/// algorithm of the HTML Standard's Text, Search, Telephone, URL, Email and Password states
/// ("strip newlines"); for a `textarea`, `value` as given. It is done here, before `Fill`,
/// because the specification's `SetValue` stores its input as it is: a field's `sets:` has no
/// operation on a string. So the value `FillRequested` counts, the row holds and a submission
/// sends is the one the field holds.
fn sanitized(kind: ElementKind, value: &str) -> String {
    match kind {
        ElementKind::Text | ElementKind::Password => value
            .chars()
            .filter(|c| !matches!(c, '\n' | '\r'))
            .collect(),
        _ => value.to_string(),
    }
}

/// The word of a submission's end reason ("Pressing a button" above): one per reason;
/// `UnsupportedScheme` and `AddressTooLong`, which the session refuses before it presses, give
/// the words of those refusals.
fn end_reason(reason: surf::SubmissionEndReason) -> RefusalReason {
    use surf::SubmissionEndReason as End;

    match reason {
        End::NoFormOwner => RefusalReason::NoFormOwner,
        End::NotSubmitButton => RefusalReason::NoDefaultAction,
        End::InvalidControls => RefusalReason::InvalidControls,
        End::InvalidAction => RefusalReason::InvalidAction,
        End::UnsupportedScheme => RefusalReason::UnsupportedScheme,
        End::UnsupportedEnctype => RefusalReason::UnsupportedEnctype,
        End::DialogMethod => RefusalReason::DialogMethod,
        End::AddressTooLong => RefusalReason::AddressTooLong,
    }
}

/// The `submits` part of a button line (`story:agent-press-submit`): the form's handle
/// `F<n>`, the method, and the action shown by the rules of a link's target
/// ([`shown_target`]), cut at [`TARGET_CHARS_MAX`], or `unresolved` alone.
fn submits_part(found: &surf::Submits, page: Option<&surf_url::Url>) -> surf_answer::Submits {
    // An action that does not parse is shown as `unresolved` with none of its text, which
    // could hold a userinfo ("As decided after the adversary's pass", decision 2).
    surf_answer::Submits {
        form: format!("F{}", found.form + 1),
        method: found.method.word().to_string(),
        action: found
            .action
            .as_ref()
            .ok()
            .map(|action| Cut::of(&shown_target(action, page), TARGET_CHARS_MAX)),
    }
}

/// The kind word of a `now` line: the variant name of `surf.document.ElementKind`.
fn kind_word(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::Link => "link",
        ElementKind::Button => "button",
        ElementKind::Text => "text",
        ElementKind::Password => "password",
        ElementKind::Checkbox => "checkbox",
        ElementKind::Radio => "radio",
        ElementKind::File => "file",
        ElementKind::Select => "select",
        ElementKind::Textarea => "textarea",
    }
}

/// The state of each control of a document as a `now` line writes it ("The controls" above):
/// from its row (`rows`) and, for a select, the label of its first selected option by position
/// (`options`, the rows of every select).
fn control_states(rows: &[Elements], options: &[Options]) -> HashMap<String, State> {
    let selected = selected_labels(options);
    rows.iter()
        .filter(|row| row.kind != ElementKind::Link)
        .map(|row| {
            let text = |value: &str| Some(Value::Text(Cut::of(value, VALUE_CHARS_MAX)));
            let value = match row.kind {
                ElementKind::Text | ElementKind::Textarea | ElementKind::File => {
                    text(row.value.as_deref().unwrap_or(""))
                }
                ElementKind::Password => Some(
                    if row.value.as_deref().is_some_and(|value| !value.is_empty()) {
                        Value::Set
                    } else {
                        Value::Empty
                    },
                ),
                ElementKind::Select => selected
                    .get(row.element_id.0.0.as_str())
                    .and_then(|label| text(label)),
                _ => None,
            };
            let state = State {
                value,
                disabled: row.disabled,
                checked: row.checked
                    && matches!(row.kind, ElementKind::Checkbox | ElementKind::Radio),
                required: row.required,
                readonly: row.readonly,
                submits: None,
            };
            (row.element_id.0.0.clone(), state)
        })
        .collect()
}

/// The label of the selected option of each select, by the select's element identity: of its
/// selected options (`options`, the rows of every select) the first by position.
fn selected_labels(options: &[Options]) -> HashMap<&str, &str> {
    let mut selected: HashMap<&str, (i64, &str)> = HashMap::new();
    for option in options.iter().filter(|option| option.selected) {
        let entry = selected
            .entry(option.select_id.0.0.as_str())
            .or_insert((option.position, option.label.as_str()));
        if option.position < entry.0 {
            *entry = (option.position, option.label.as_str());
        }
    }
    selected
        .into_iter()
        .map(|(select, (_, label))| (select, label))
        .collect()
}

/// The `~` line of a control from `before` to `after`, or `None` when its state is the same
/// ("What changed" above): `value` for a text, textarea, file or password control, a password's
/// written `(set)` or `(empty)`; `checked` for a checkbox or a radio button; `selected` for a
/// select, `""` for none selected.
fn control_change(before: &ShownControl, after: &ShownControl) -> Option<Change> {
    match after.kind {
        ElementKind::Text | ElementKind::Textarea | ElementKind::File
            if before.value != after.value =>
        {
            let value = |value: &Option<String>| {
                Value::Text(Cut::of(value.as_deref().unwrap_or(""), VALUE_CHARS_MAX))
            };
            Some(Change::Value {
                before: value(&before.value),
                after: value(&after.value),
            })
        }
        ElementKind::Password if before.value != after.value => {
            let value = |value: &Option<String>| match value.as_deref() {
                Some(value) if !value.is_empty() => Value::Set,
                _ => Value::Empty,
            };
            Some(Change::Value {
                before: value(&before.value),
                after: value(&after.value),
            })
        }
        ElementKind::Checkbox | ElementKind::Radio if before.checked != after.checked => {
            Some(Change::Checked {
                before: before.checked,
                after: after.checked,
            })
        }
        ElementKind::Select if before.selected != after.selected => {
            let label =
                |label: &Option<String>| Cut::of(label.as_deref().unwrap_or(""), VALUE_CHARS_MAX);
            Some(Change::Selected {
                before: label(&before.selected),
                after: label(&after.selected),
            })
        }
        _ => None,
    }
}

/// The texts of a document's tree ("What changed" above): the lines of its Markdown rendering
/// but the empty ones.
fn markdown_texts(tree: &Tree) -> impl Iterator<Item = String> + '_ {
    surf_page_text::markdown::markdown(tree).filter(|line| !line.trim().is_empty())
}

/// The `arg count` of `log` ("The log" above): `None` without it; `Err` for one that is not a
/// number from 1 in decimal digits. A number past the platform's `usize` counts as every action.
fn log_count(request: &Request) -> Result<Option<usize>, ()> {
    let Some(digits) = request.arg("count") else {
        return Ok(None);
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(());
    }
    match digits.trim_start_matches('0') {
        "" => Err(()),
        number => Ok(Some(number.parse().unwrap_or(usize::MAX))),
    }
}
