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
//! - `close`: `CloseSession` as the holder; when it is `closed`, `RecordAction` `close`.
//!
//! An accepted command's `RecordAction` is issued before the run that delivers what the command
//! published, so that the log holds the action before the events it caused (decision D19).
//! [`Session`] holds the sequence number: the last one recorded.
//!
//! # Refusals
//!
//! - A request other than `open` whose `session` is not this identity, or whose token does not
//!   match the session's (compared by [`surf_secret::Token::matches`] only), or that comes before
//!   the session is open: `did refused token`, status 1, the head line and the `did` line only.
//!   Nothing is acted on and nothing of the session's state is in the answer.
//! - From the holder, a command this session does not know, an argument name it does not know,
//!   `go` without `arg url`: `did refused usage`, status 2, with the current page and path.
//! - `open` after the first, `open` with a session, a token or an argument but `url`, and a
//!   request that does not decode ([`Session::malformed`]): `did refused usage`, status 2, the
//!   head line and the `did` line only, because it carries no token.
//! - `go` or `close` on a closed session: `did refused closed`, status 1, with the state.
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
//! each address at most four times its length once quoted; an address comes from a request of at
//! most [`surf_agent::wire::REQUEST_MAX`] bytes or is the address a response came from after
//! redirects, at most [`surf_fetch::MAX_ADDRESS_BYTES`] bytes, the same 64 KiB, so an answer
//! stays far below
//! [`surfd::link::ANSWER_TEXT_MAX`]. A status record whose quoted address does not fit
//! [`surfd::link::RECORD_MAX`] is sent without the address.
//!
//! A response body (at most `surf-fetch`'s body limit, 16 MiB) is held only while the command
//! that loaded it runs: the system carries it in two events and two binding invocations, and
//! [`Session::answer`] drops everything the system delivered once the reply is made
//! ([`surf::Browser::forget_delivered`]; [`Session::held`] counts what is left, nothing between
//! commands). No answer needs a body: the page line is read from the `SessionState` row (status,
//! media type, address, failure reason). What stays across commands are the components' rows,
//! none of which holds a body: per `go` one action, one navigation, one request and, for an
//! HTML page, one document, each with its address and identities; and the session's row.
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
//! The entries of one request through the wire stay far below [`surfd::link::ENTRIES_MAX`]: an
//! event field holds no body, only its length, and an address, at most
//! [`surf_agent::wire::REQUEST_MAX`] bytes long when it comes from a request and at most
//! [`surf_fetch::MAX_ADDRESS_BYTES`] bytes, the same 64 KiB, when it is the address a response
//! came from after redirects, is in at most eight event and action lines of
//! one `open <url>` or `go`, and the answer entry holds at most seven addresses (`did`, `page`
//! and [`PATH_SHOWN`] path lines): about 1 MiB.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod entries;
pub mod process;

use std::fmt;
use std::io;

use surf::{MemoryBrowser, Transport, UnmetObligation};
use surf_agent::wire::{self, Request};
use surf_answer::{
    Answer, Did, Head, Page, Path, PathEntry, RefusalReason, SessionState as HeadState,
};
use surf_ports::{Caller, ComponentPorts};
use surf_secret::Token;
use surf_types::agent::{
    ActionCommand, CloseSession, CloseSessionOutcome, Go, GoOutcome, OpenSession,
    OpenSessionOutcome, RecordAction, RecordActionOutcome, SessionId, SessionState,
    SessionStateView,
};
use surf_types::navigation::NavigationState;
use surf_types::primitives::Uuid;
use surf_types::shell::{OpenTab, OpenTabOutcome, OpenWindow, OpenWindowOutcome};
use surf_types::url::Url;
use surfd::link;

/// The most actions the `path` block shows: the last five (design section 5.1).
pub const PATH_SHOWN: usize = 5;

/// The transport of the binary: `surf-fetch` at its default limits, through [`surf::answer`].
/// An address that is not `http:` is `UnsupportedScheme` and nothing is read; redirects are
/// followed inside the one fetch, up to 20, and the answer carries the address of the last
/// response.
pub fn fetch(address: &str) -> surf::Answer {
    surf::answer(surf_fetch::fetch(address, &surf_fetch::Limits::default()))
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
    /// `closed` once the session is closed, `open` otherwise.
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
    /// address is left out of the record when the answer would not fit the link with it; an
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
            title: None,
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

    /// The reply to `request`, with its entries ("The entries" above), which are read from what
    /// the system published while it was answered. Then everything the system delivered is
    /// dropped ([`surf::Browser::forget_delivered`]): the next answer reads only the views.
    pub fn answer(&mut self, request: &Request) -> Reply {
        let start = self.browser.system().published().len();
        self.recorded.clear();
        self.marks.clear();
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
        let mut out = Vec::new();
        let mut from = start.min(published.len());
        for (at, sequence) in self.recorded.iter().enumerate() {
            let row = log.iter().find(|row| row.sequence == *sequence);
            let Some(action) = row.and_then(|row| link::Entry::line(&entries::action_line(row)))
            else {
                return Vec::new();
            };
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
            out.extend(
                published[from..end]
                    .iter()
                    .filter_map(|event| link::Entry::line(&entries::event_line(event))),
            );
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
            "go" => match (request.unknown_arg(&["url"]), request.arg("url")) {
                (None, Some(url)) => self.go(url),
                _ => Ok(self.with_state(RefusalReason::Usage)),
            },
            "look" if request.unknown_arg(&[]).is_none() => Ok(self.state_reply(0, None)),
            "close" if request.unknown_arg(&[]).is_none() => self.close(),
            _ => Ok(self.with_state(RefusalReason::Usage)),
        };
        result.unwrap_or_else(|stop| self.stopped(&stop))
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
        if self.opened
            || request.session.is_some()
            || request.token.is_some()
            || request.unknown_arg(&["url"]).is_some()
        {
            return self.bare(RefusalReason::Usage);
        }
        let url = request.arg("url");
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
                reply
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
        self.record_and_run(ActionCommand::Open, None)?;
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
            self.record_and_run(ActionCommand::Go, Some(url))?;
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
                self.record_and_run(ActionCommand::Close, None)?;
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
    fn record_and_run(&mut self, command: ActionCommand, url: Option<&str>) -> Result<(), Stop> {
        self.record(command, url)?;
        self.browser.run()?;
        self.marks.push(self.browser.system().published().len());
        Ok(())
    }

    /// `RecordAction` with the next sequence number, on the `agent` port.
    fn record(&mut self, command: ActionCommand, url: Option<&str>) -> Result<(), Stop> {
        let sequence = self.sequence + 1;
        let outcome = self.browser.agent_mut().record_action(RecordAction {
            session_id: self.session_id.clone(),
            sequence,
            command,
            url: url.map(|url| Url(url.to_string())),
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

    /// The head line and `did refused <reason>` and nothing else; the status of the reason.
    fn bare(&self, reason: RefusalReason) -> Reply {
        let mut answer = Answer::new(self.head());
        answer.did = Some(Did::Refused(reason));
        let (state, address) = self.record_fields();
        Reply {
            status: refusal_status(reason),
            reason: None,
            answer,
            state,
            address,
            entries: Vec::new(),
        }
    }

    /// `did refused <reason>` with the page and the path.
    fn with_state(&self, reason: RefusalReason) -> Reply {
        self.state_reply(refusal_status(reason), Some(Did::Refused(reason)))
    }

    /// The head line, `did` when there is one, and the page and path from the views.
    fn state_reply(&self, status: u8, did: Option<Did>) -> Reply {
        let mut answer = Answer::new(self.head());
        answer.did = did;
        let row = self.row();
        answer.page = Some(row.as_ref().map_or(Page::None, page));
        answer.path = Some(self.path());
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
            .map(|row| PathEntry {
                sequence: u64::try_from(row.sequence).unwrap_or_default(),
                command: command_word(row.command).to_string(),
                argument: row.url.map(|url| url.0),
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
        RefusalReason::Token | RefusalReason::Closed => 1,
        RefusalReason::Usage => 2,
        RefusalReason::NoSuchSession
        | RefusalReason::SessionDied
        | RefusalReason::SessionTimeout => 3,
    }
}

/// The word of an action's command on a path line, and its variant name in `spec/`.
fn command_word(command: ActionCommand) -> &'static str {
    match command {
        ActionCommand::Open => "open",
        ActionCommand::Go => "go",
        ActionCommand::Close => "close",
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
