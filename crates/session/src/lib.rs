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
//! page text, when the document has links, the `now` block and `note visibility: hidden
//! attribute only` (`docs/agent-answer.md`). A call with `--as` has no `now` block.
//!
//! The links are the rows of `surf.document.Elements` of kind `link` in position order, each
//! with the handle [`surf_page::handles`] gives its node (`l1`, `l2`, … in document order,
//! hidden elements included, starting at 1 for each document), the row's label and target (the
//! `href` as written), and `hid` when the element or an ancestor has the HTML `hidden`
//! attribute, read from the tree: `aria-hidden`, a closed `dialog` or `details` and a popover
//! leave a link `?`, though the row's `hidden` flag is wider. The target is resolved with
//! `surf-url` against [`surf_page::base_url::document_base_url`]; a line shows its path, query
//! and fragment when it has the document's origin and the whole address otherwise, or the
//! `href` as written and `unresolved`. The label is cut at [`surf_answer::LABEL_CHARS_MAX`]
//! characters, the target at [`surf_answer::TARGET_CHARS_MAX`]. The block shows the first lines
//! that fit [`NOW_BYTES_MAX`] bytes, and its header `now <shown> of <total>` counts them all.
//! The session computes the links once per document and keeps them while it is current.
//!
//! `follow <handle>` is refused `closed` on a closed session, `unknown-handle` when the current
//! document has no link of that handle (or there is none), `hidden` for a link `hid` names, and
//! `unresolved` for a target that does not resolve; each with status 1 and the state. The `path`
//! line of a `follow` is `<n> follow <handle> "<label>"`, the label cut as in the `now` block.
//!
//! # Refusals
//!
//! - A request other than `open` whose `session` is not this identity, or whose token does not
//!   match the session's (compared by [`surf_secret::Token::matches`] only), or that comes before
//!   the session is open: `did refused token`, status 1, the head line and the `did` line only.
//!   Nothing is acted on and nothing of the session's state is in the answer.
//! - From the holder, a command this session does not know, an argument name it does not know,
//!   `go` without `arg url`, `arg form` other than `tree` or `markdown`, `arg line` without `arg
//!   form` or not a number from 1: `did refused usage`, status 2, with the current page and path.
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
//! - `go`, `follow` or `close` on a closed session: `did refused closed`, status 1, with the
//!   state. `follow` without `arg handle` or with another argument: `did refused usage`.
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
//! seven addresses. An address comes from a request of at most
//! [`surf_agent::wire::REQUEST_MAX`] bytes, which quotes it as the answer does, or is the
//! address a response came from after redirects, at most [`surf_fetch::MAX_ADDRESS_BYTES`]
//! bytes, the same 64 KiB, before quoting. Quoting writes a `"`, a `\` and each byte outside
//! printable ASCII as four bytes (`\x5c` for a `\`), so a quoted address is at most four times
//! its length and its two quotes; `surf-url` leaves a `\` in the query or fragment of a
//! response URL as it is, so the quoted form of one comes to up to four times
//! [`surf_fetch::MAX_ADDRESS_BYTES`], at most 262,146 bytes. Seven addresses of at most 262,146
//! bytes each are 1,835,022 bytes. The `did` line of `follow` holds the handle as typed, from a
//! request like an address, and a `follow` path line holds a label of at most 60 characters
//! (at most 965 bytes quoted), so the seven still bound them. The title adds at most
//! [`TITLE_CHARS_MAX`] characters, at most 16,386 bytes quoted. A `text` block adds at most
//! [`surf_answer::TEXT_BYTES_MAX`] (32,768) bytes of body lines, its header, a `more` line and a
//! `note text:` line, each under 200 bytes; or, in an answer without text, the `now` block adds
//! at most [`NOW_BYTES_MAX`] (1,048,576) bytes and `note visibility:` and `note parser:` lines
//! under 200 bytes each. So an answer stays below [`surfd::link::ANSWER_TEXT_MAX`], 4,194,304
//! bytes.
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
//! [`surfd::link::ANSWER_TEXT_MAX`] (4,194,304 bytes; it holds at most seven addresses, `did`,
//! `page` and [`PATH_SHOWN`] path lines, each at most 262,146 bytes once quoted, "What it
//! keeps"), and its `answer-entry <m>` line. The action and event entries take at most
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
//! came from after redirects, is at most 262,146 bytes once quoted, in at most nine action and
//! event lines of one `open <url>` or `go` (`DocumentParsed` is the ninth), 2,359,314 bytes,
//! with a few hundred bytes of other fields per line: below the 4,194,240, so no event of the
//! page load itself is left out.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod entries;
pub mod process;

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io;

use surf::{MemoryBrowser, NodeId, Transport, Tree, UnmetObligation};
use surf_agent::wire::{self, Request};
use surf_answer::{
    Answer, Cut, Did, Element, Form, Head, LABEL_CHARS_MAX, Note, Now, Page, Path, PathEntry,
    RefusalReason, SessionState as HeadState, TARGET_CHARS_MAX, Target, TextNote, Visibility,
};
use surf_ports::{Caller, ComponentPorts};
use surf_secret::Token;
use surf_types::agent::{
    ActionCommand, CloseSession, CloseSessionOutcome, Follow, FollowOutcome, Go, GoOutcome,
    OpenSession, OpenSessionOutcome, RecordAction, RecordActionOutcome, SessionId, SessionState,
    SessionStateView,
};
use surf_types::document::{DocumentId, DocumentState, Documents, ElementKind};
use surf_types::navigation::NavigationState;
use surf_types::primitives::Uuid;
use surf_types::shell::{OpenTab, OpenTabOutcome, OpenWindow, OpenWindowOutcome};
use surf_types::url::Url;
use surfd::link;

/// The most actions the `path` block shows: the last five (design section 5.1).
pub const PATH_SHOWN: usize = 5;

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
/// one fetch, up to 20, and the answer carries the address of the last response.
pub fn fetch(request: &surf::RequestStarted) -> surf::Answer {
    surf::answer(surf::fetch_http(request, &surf_fetch::Limits::default()))
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

/// One link of the current document, as the `now` block shows it and `follow` takes it ("The
/// links" above).
#[derive(Clone)]
struct Link {
    /// `l<n>`, as [`surf_page::handles`] numbers it.
    handle: String,
    /// The label of the element's row, the whole accessible name (at most
    /// [`surf_page::LABEL_BYTES_MAX`] bytes).
    label: String,
    /// The element or an ancestor has the `hidden` attribute.
    hidden: bool,
    /// The target resolved against the document base URL; `None` when it does not resolve.
    resolved: Option<surf_url::Url>,
    /// What the `now` line shows: the target's path, query and fragment when it has the
    /// document's origin, the whole target otherwise, the `href` as written when unresolved.
    shown: String,
}

/// The links of the current document, kept while it is current.
struct Links {
    document: DocumentId,
    links: Vec<Link>,
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
        let row = system
            .agent
            .sessions()
            .ok()?
            .into_iter()
            .find(|row| row.session_id == self.session_id)?;
        if row.navigation_state != Some(NavigationState::Committed) {
            return None;
        }
        let navigation = row.navigation_id?;
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
            self.links = Some(Links {
                document: row.document_id,
                links,
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
    /// the `hidden` attribute, `unresolved` when its target does not resolve; otherwise
    /// `Follow` as the holder with the resolved address, and when it is `requested`,
    /// `RecordAction` `follow` with the address, the handle and the label.
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
        if link.hidden {
            return Ok(self.with_state(RefusalReason::Hidden));
        }
        let Some(url) = link.resolved.as_ref().map(surf_url::Url::href) else {
            return Ok(self.with_state(RefusalReason::Unresolved));
        };
        let outcome = self.browser.agent_mut().follow(Follow {
            session_id: self.session_id.clone(),
            url: Url(url.clone()),
            handle: link.handle.clone(),
            label: link.label.clone(),
        })?;
        match outcome {
            FollowOutcome::Requested { .. } => {
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

    /// The session's row of the view `SessionState`, if it is open or was.
    fn row(&self) -> Option<SessionStateView> {
        self.browser
            .system()
            .agent
            .session_state_view(self.session_id.clone())
            .ok()?
            .into_iter()
            .next()
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

    /// The head line, `did` when there is one, the page and path from the views, and for the
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
        if self.current_document().is_some() {
            let parts = self.browser.parser_not_yet_parsed();
            if !parts.is_empty() {
                answer.notes.push(Note::Parser(
                    parts.iter().map(|part| part.to_string()).collect(),
                ));
            }
            if !self.is_closed()
                && let Some(now) = self.links().and_then(now_block)
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
        let mut log = self
            .browser
            .system()
            .agent
            .session_log(self.session_id.clone())
            .unwrap_or_default();
        log.sort_by_key(|row| row.sequence);
        let total = log.len();
        let entries = log
            .into_iter()
            .skip(total.saturating_sub(PATH_SHOWN))
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
        | RefusalReason::Unresolved => 1,
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

/// The links of a document from its element rows (`rows`, in position order) and the node of
/// each (`nodes`, [`surf::Browser::element_nodes`]): the handle [`surf_page::handles`] gives its
/// node, the row's label and target, whether it or an ancestor has the `hidden` attribute, and
/// the target resolved against the document base URL of `tree` at `address`.
fn document_links(
    tree: &Tree,
    address: &str,
    rows: &[surf_types::document::Elements],
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
        let Some(href) = row
            .target
            .as_ref()
            .filter(|_| row.kind == ElementKind::Link)
        else {
            continue;
        };
        let Some(node) = node_of.get(row.element_id.0.0.as_str()) else {
            continue;
        };
        let Some(handle) = handles.get(node) else {
            continue;
        };
        let resolved = surf_url::Url::parse(href, base.as_ref()).ok();
        let shown = match &resolved {
            Some(target) => shown_target(target, page.as_ref()),
            None => href.clone(),
        };
        links.push(Link {
            handle: handle.clone(),
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
/// origin of `page`, the whole address otherwise.
fn shown_target(target: &surf_url::Url, page: Option<&surf_url::Url>) -> String {
    if !page.is_some_and(|page| page.origin().same_origin(&target.origin())) {
        return target.href();
    }
    let mut shown = target.serialize_path();
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

/// The `now` block of `links`: a line per link in document order while the lines fit
/// [`NOW_BYTES_MAX`], the label cut at [`LABEL_CHARS_MAX`] characters and the target at
/// [`TARGET_CHARS_MAX`]; `None` for a document without links.
fn now_block(links: &[Link]) -> Option<Now> {
    if links.is_empty() {
        return None;
    }
    let total = u64::try_from(links.len()).unwrap_or(u64::MAX);
    let mut bytes = format!("now {total} of {total}\n").len();
    let mut elements = Vec::new();
    for link in links {
        let label = Cut::of(&link.label, LABEL_CHARS_MAX);
        let target = Cut::of(&link.shown, TARGET_CHARS_MAX);
        let line = NOW_LINE_FIXED
            + link.handle.len()
            + surf_answer::quote(&label.kept).len()
            + surf_answer::quote(&target.kept).len();
        if bytes + line > NOW_BYTES_MAX {
            break;
        }
        bytes += line;
        elements.push(Element {
            handle: link.handle.clone(),
            kind: "link".to_string(),
            visibility: if link.hidden {
                Visibility::Hidden
            } else {
                Visibility::Unknown
            },
            label,
            target: if link.resolved.is_some() {
                Target::Resolved(target)
            } else {
                Target::Unresolved(target)
            },
        });
    }
    Some(Now { total, elements })
}
