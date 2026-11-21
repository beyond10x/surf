//! The `surf` system assembled: the six generated components behind their ports, the generated
//! bindings, and a transport for the `net` component.
//!
//! [`Browser`] holds the generated [`System`] and a [`Transport`]. The specification leaves one
//! step to the `net` component's own transport (`spec/domains/net.yaml`, the comment above
//! `surf.net.DeliverResponse`): after `surf.net.Fetch` made a request `Pending`, something issues
//! `surf.net.DeliverResponse` or `surf.net.FailRequest`. [`Browser::run`] is that step. It pumps the
//! system, reads every `surf.net.RequestStarted` published since its last pass, asks the transport
//! for each request that is still `Pending` in the `net` component's own `Requests` view, issues
//! the answer on the `net` port and on no other, and pumps again, until no new request started.
//!
//! A [`Browser`] is built from any six component ports that meet the generated bounds:
//! [`Browser::new`] takes a [`System`] the caller assembled, and [`Browser::from_ports`] the
//! in-memory ports of `surf-ports`. [`Browser::system`] reads the views, [`System::published`] and
//! [`System::invocations`]; the `*_mut` methods issue commands on one component's port.
//!
//! The run reads the log through a cursor of its own over [`System::published`] and never takes
//! it. Nothing here calls [`System::take_published`], and no method hands out the system mutably,
//! so a request is handed to the transport once.
//!
//! [`Browser::load`] is what the `surf` binary does: `OpenWindow`, `OpenTab`, `Navigate`, run, and
//! the rows of the `Navigations`, `Requests` and `Documents` views with every published
//! `FetchFailed`, as a [`Report`]. [`exit_status`] turns its result into the binary's exit status.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt;

use agent::Agent;
use document::Document;
use navigator::Navigator;
use net::Net;
use renderer::Renderer;
use shell::Shell;
use surf_ports::{
    AgentPorts, ComponentPorts, DocumentPorts, NavigatorPorts, NetPorts, RendererPorts, ShellPorts,
};
use surf_types::behaviour::Generated;
use surf_types::document::Documents;
use surf_types::navigation::{Navigate, NavigateOutcome, NavigationState, Navigations};
use surf_types::net::{
    DeliverResponse, FailRequest, FailureReason, FetchFailed, MediaType, RequestId, RequestStarted,
    RequestState, Requests, Status,
};
use surf_types::shell::{OpenTab, OpenTabOutcome, OpenWindow, OpenWindowOutcome};
use surf_types::url::Url;

pub use surf_system::{System, SystemEvent};
pub use surf_types::obligation::UnmetObligation;

/// The exit status when the navigation is `Committed`.
pub const EXIT_COMMITTED: u8 = 0;
/// The exit status when the navigation is not `Committed`: `Failed`, or, only with a transport
/// that leaves a request unanswered, still `Started`.
pub const EXIT_NOT_COMMITTED: u8 = 1;
/// The exit status of a usage error.
pub const EXIT_USAGE: u8 = 2;
/// The exit status when the load stopped with an [`Error`], and the `surf` binary's exit status
/// when the rows could not be written to standard output.
pub const EXIT_STOPPED: u8 = 3;

/// What a [`Transport`] answers for one address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// A response: issued as `surf.net.DeliverResponse`.
    Response {
        /// The status code, such as 200 or 404.
        status: u16,
        /// The type and subtype of the response's media type in lower case, such as `text/html`.
        media_type: String,
        /// The body bytes.
        body: Vec<u8>,
    },
    /// No response, for this reason: issued as `surf.net.FailRequest`.
    Failure(FailureReason),
    /// No answer. Nothing is issued and the request stays `Pending`.
    Unanswered,
}

/// Fetches the resource an address names, for the `net` component.
///
/// A closure `FnMut(&str) -> Answer` is a transport.
pub trait Transport {
    /// Fetches `address` and answers what came of it.
    fn fetch(&mut self, address: &str) -> Answer;
}

impl<F: FnMut(&str) -> Answer> Transport for F {
    fn fetch(&mut self, address: &str) -> Answer {
        self(address)
    }
}

/// The [`Answer`] for what `surf-fetch` returned: a response, or the failure reason of the same
/// name.
pub fn answer(result: Result<surf_fetch::Response, surf_fetch::Failure>) -> Answer {
    match result {
        Ok(response) => Answer::Response {
            status: response.status,
            media_type: response.media_type,
            body: response.body,
        },
        Err(surf_fetch::Failure::Unreachable(_)) => Answer::Failure(FailureReason::Unreachable),
        Err(surf_fetch::Failure::UnsupportedScheme(_)) => {
            Answer::Failure(FailureReason::UnsupportedScheme)
        }
        Err(surf_fetch::Failure::MalformedResponse(_)) => {
            Answer::Failure(FailureReason::MalformedResponse)
        }
    }
}

/// Why a load stopped before its report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A command or a pump returned an unmet obligation of the generated code.
    Unmet(UnmetObligation),
    /// `surf.shell.OpenTab` refused the window `OpenWindow` had just opened. Ports that keep the
    /// window they were given never do this.
    Refused(String),
}

impl From<UnmetObligation> for Error {
    fn from(obligation: UnmetObligation) -> Self {
        Error::Unmet(obligation)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unmet(obligation) => obligation.fmt(f),
            Error::Refused(refusal) => f.write_str(refusal),
        }
    }
}

impl std::error::Error for Error {}

/// The rows a load ends with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The navigation the load started.
    pub navigation_id: RequestId,
    /// The rows of `surf.navigation.Navigations`.
    pub navigations: Vec<Navigations>,
    /// The rows of `surf.net.Requests`.
    pub requests: Vec<Requests>,
    /// The rows of `surf.document.Documents`.
    pub documents: Vec<Documents>,
    /// Every published `surf.net.FetchFailed`, in publication order.
    pub fetch_failures: Vec<FetchFailed>,
}

impl Report {
    /// The state of the navigation the load started, if its row is in the view.
    pub fn navigation_state(&self) -> Option<NavigationState> {
        self.navigations
            .iter()
            .find(|row| row.navigation_id == self.navigation_id)
            .map(|row| row.state)
    }

    /// The output lines, without line terminators: every navigation, then every request, every
    /// document and every `FetchFailed`, each in the order its view or the log holds it.
    ///
    /// ```text
    /// navigation <navigation-id> <state> <address>
    /// request <request-id> <state> <address>
    /// document <document-id> <address>
    /// fetch-failed <request-id> <reason> <address>
    /// ```
    ///
    /// Fields are separated by one space. The address is the rest of the line, written by
    /// [`escape`].
    pub fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for row in &self.navigations {
            lines.push(format!(
                "navigation {} {} {}",
                row.navigation_id.0.0,
                navigation_state(row.state),
                escape(&row.url.0)
            ));
        }
        for row in &self.requests {
            lines.push(format!(
                "request {} {} {}",
                row.request_id.0.0,
                request_state(row.state),
                escape(&row.url.0)
            ));
        }
        for row in &self.documents {
            lines.push(format!(
                "document {} {}",
                row.document_id.0.0,
                escape(&row.url.0)
            ));
        }
        for event in &self.fetch_failures {
            lines.push(format!(
                "fetch-failed {} {} {}",
                event.request_id.0.0,
                failure_reason(event.reason),
                escape(&event.url.0)
            ));
        }
        lines
    }
}

/// An address as one output field: printable ASCII (U+0020 to U+007E) other than the backslash
/// is written as itself; the backslash and every other character is written as `\x` and two
/// lower-case hexadecimal digits per byte of its UTF-8 encoding. That covers the control
/// characters and line terminators, Unicode's line and paragraph separators (U+2028, U+2029),
/// and every format character, such as the bidirectional controls, so the field can neither
/// split a line nor reorder one. The standard library has no table of Unicode's general
/// categories, so no character outside ASCII is let through: `surf-fetch` refuses such an
/// address anyway.
pub fn escape(address: &str) -> String {
    let mut escaped = String::with_capacity(address.len());
    for character in address.chars() {
        if character == '\\' || !matches!(character, ' '..='~') {
            let mut bytes = [0u8; 4];
            for byte in character.encode_utf8(&mut bytes).bytes() {
                escaped.push_str(&format!("\\x{byte:02x}"));
            }
        } else {
            escaped.push(character);
        }
    }
    escaped
}

fn navigation_state(state: NavigationState) -> &'static str {
    match state {
        NavigationState::Cancelled => "Cancelled",
        NavigationState::Committed => "Committed",
        NavigationState::Failed => "Failed",
        NavigationState::Started => "Started",
    }
}

fn request_state(state: RequestState) -> &'static str {
    match state {
        RequestState::Failed => "Failed",
        RequestState::Pending => "Pending",
        RequestState::Responded => "Responded",
    }
}

fn failure_reason(reason: FailureReason) -> &'static str {
    match reason {
        FailureReason::Unreachable => "Unreachable",
        FailureReason::UnsupportedScheme => "UnsupportedScheme",
        FailureReason::MalformedResponse => "MalformedResponse",
        FailureReason::TooManyRedirects => "TooManyRedirects",
    }
}

/// The exit status of a load: [`EXIT_COMMITTED`] when its navigation is `Committed`,
/// [`EXIT_NOT_COMMITTED`] when it is in any other state or has no row, and [`EXIT_STOPPED`] when
/// the load stopped with an [`Error`].
pub fn exit_status(result: &Result<Report, Error>) -> u8 {
    match result {
        Ok(report) => match report.navigation_state() {
            Some(NavigationState::Committed) => EXIT_COMMITTED,
            _ => EXIT_NOT_COMMITTED,
        },
        Err(_) => EXIT_STOPPED,
    }
}

/// The behaviours the `agent` component's port needs to be part of a [`System`].
pub trait AgentBehaviors:
    surf_types::agent::obligations::CloseSessionBehavior
    + surf_types::agent::obligations::GoBehavior
    + surf_types::agent::obligations::NoteNavigationCommittedBehavior
    + surf_types::agent::obligations::NoteNavigationEndedBehavior
    + surf_types::agent::obligations::NoteNavigationFailedBehavior
    + surf_types::agent::obligations::NoteNavigationStartedBehavior
    + surf_types::agent::obligations::OpenSessionBehavior
    + surf_types::agent::obligations::RecordActionBehavior
    + surf_types::agent::obligations::SessionLogQuery
    + surf_types::agent::obligations::SessionStateViewQuery
    + surf_types::agent::obligations::SessionsQuery
{
}

impl<B> AgentBehaviors for B where
    B: surf_types::agent::obligations::CloseSessionBehavior
        + surf_types::agent::obligations::GoBehavior
        + surf_types::agent::obligations::NoteNavigationCommittedBehavior
        + surf_types::agent::obligations::NoteNavigationEndedBehavior
        + surf_types::agent::obligations::NoteNavigationFailedBehavior
        + surf_types::agent::obligations::NoteNavigationStartedBehavior
        + surf_types::agent::obligations::OpenSessionBehavior
        + surf_types::agent::obligations::RecordActionBehavior
        + surf_types::agent::obligations::SessionLogQuery
        + surf_types::agent::obligations::SessionStateViewQuery
        + surf_types::agent::obligations::SessionsQuery
{
}

/// The behaviours the `document` component's port needs to be part of a [`System`].
pub trait DocumentBehaviors:
    surf_types::document::obligations::ParseDocumentBehavior
    + surf_types::document::obligations::DocumentsQuery
{
}

impl<B> DocumentBehaviors for B where
    B: surf_types::document::obligations::ParseDocumentBehavior
        + surf_types::document::obligations::DocumentsQuery
{
}

/// The behaviours the `navigator` component's port needs to be part of a [`System`].
pub trait NavigatorBehaviors:
    surf_types::navigation::obligations::CancelNavigationBehavior
    + surf_types::navigation::obligations::CommitNavigationBehavior
    + surf_types::navigation::obligations::FailNavigationBehavior
    + surf_types::navigation::obligations::NavigateBehavior
    + surf_types::navigation::obligations::NavigationsQuery
{
}

impl<B> NavigatorBehaviors for B where
    B: surf_types::navigation::obligations::CancelNavigationBehavior
        + surf_types::navigation::obligations::CommitNavigationBehavior
        + surf_types::navigation::obligations::FailNavigationBehavior
        + surf_types::navigation::obligations::NavigateBehavior
        + surf_types::navigation::obligations::NavigationsQuery
{
}

/// The behaviours the `net` component's port needs to be part of a [`System`].
pub trait NetBehaviors:
    surf_types::net::obligations::DeliverResponseBehavior
    + surf_types::net::obligations::FailRequestBehavior
    + surf_types::net::obligations::FetchBehavior
    + surf_types::net::obligations::RequestsQuery
{
}

impl<B> NetBehaviors for B where
    B: surf_types::net::obligations::DeliverResponseBehavior
        + surf_types::net::obligations::FailRequestBehavior
        + surf_types::net::obligations::FetchBehavior
        + surf_types::net::obligations::RequestsQuery
{
}

/// The behaviours the `renderer` component's port needs to be part of a [`System`].
pub trait RendererBehaviors:
    surf_types::render::obligations::PaintFrameBehavior + surf_types::render::obligations::FramesQuery
{
}

impl<B> RendererBehaviors for B where
    B: surf_types::render::obligations::PaintFrameBehavior
        + surf_types::render::obligations::FramesQuery
{
}

/// The behaviours the `shell` component's port needs to be part of a [`System`].
pub trait ShellBehaviors:
    surf_types::shell::obligations::CloseTabBehavior
    + surf_types::shell::obligations::OpenTabBehavior
    + surf_types::shell::obligations::OpenWindowBehavior
    + surf_types::shell::obligations::TabsQuery
    + surf_types::shell::obligations::WindowsQuery
{
}

impl<B> ShellBehaviors for B where
    B: surf_types::shell::obligations::CloseTabBehavior
        + surf_types::shell::obligations::OpenTabBehavior
        + surf_types::shell::obligations::OpenWindowBehavior
        + surf_types::shell::obligations::TabsQuery
        + surf_types::shell::obligations::WindowsQuery
{
}

/// The generated [`System`] and the transport of its `net` component.
///
/// The type parameters are the behaviours behind each component's port, in the order
/// [`System::new`] takes the ports (`agent`, `document`, `navigator`, `net`, `renderer`,
/// `shell`), and the transport.
pub struct Browser<A, D, N, Ne, R, S, T> {
    system: System<A, D, N, Ne, R, S>,
    transport: T,
    /// How far into [`System::published`] the run has read.
    cursor: usize,
}

/// A [`Browser`] over the in-memory ports of `surf-ports` and the operating system's random
/// source.
pub type MemoryBrowser<T> = Browser<
    Generated<AgentPorts>,
    Generated<DocumentPorts>,
    Generated<NavigatorPorts>,
    Generated<NetPorts>,
    Generated<RendererPorts>,
    Generated<ShellPorts>,
    T,
>;

impl<T> MemoryBrowser<T> {
    /// The system over `ports`, with `transport`.
    pub fn from_ports(ports: ComponentPorts, transport: T) -> Self {
        Browser::new(
            System::new(
                ports.agent,
                ports.document,
                ports.navigator,
                ports.net,
                ports.renderer,
                ports.shell,
            ),
            transport,
        )
    }
}

impl<A, D, N, Ne, R, S, T> Browser<A, D, N, Ne, R, S, T> {
    /// `system` with `transport`. The first [`Browser::run`] reads `system`'s log from its start.
    pub fn new(system: System<A, D, N, Ne, R, S>, transport: T) -> Self {
        Self {
            system,
            transport,
            cursor: 0,
        }
    }

    /// The system: its components' views, [`System::published`] and [`System::invocations`].
    pub fn system(&self) -> &System<A, D, N, Ne, R, S> {
        &self.system
    }

    /// The `agent` component's port. A command issued on it is published by the next run, and
    /// is sent as whoever the caller its ports answer with names.
    pub fn agent_mut(&mut self) -> &mut Agent<A> {
        &mut self.system.agent
    }

    /// The `document` component's port. A command issued on it is published by the next run.
    pub fn document_mut(&mut self) -> &mut Document<D> {
        &mut self.system.document
    }

    /// The `navigator` component's port. A command issued on it is published by the next run.
    pub fn navigator_mut(&mut self) -> &mut Navigator<N> {
        &mut self.system.navigator
    }

    /// The `net` component's port. A command issued on it is published by the next run.
    pub fn net_mut(&mut self) -> &mut Net<Ne> {
        &mut self.system.net
    }

    /// The `renderer` component's port. A command issued on it is published by the next run.
    pub fn renderer_mut(&mut self) -> &mut Renderer<R> {
        &mut self.system.renderer
    }

    /// The `shell` component's port. A command issued on it is published by the next run.
    pub fn shell_mut(&mut self) -> &mut Shell<S> {
        &mut self.system.shell
    }
}

impl<A, D, N, Ne, R, S, T> Browser<A, D, N, Ne, R, S, T>
where
    A: AgentBehaviors,
    D: DocumentBehaviors,
    N: NavigatorBehaviors,
    Ne: NetBehaviors,
    R: RendererBehaviors,
    S: ShellBehaviors,
    T: Transport,
{
    /// Runs the system until no event is undelivered and no request started since the last run
    /// is left unasked: pump, hand every `surf.net.RequestStarted` published since the last pass
    /// whose request is `Pending` to the transport once, issue its answer on the `net` port, and
    /// pump again.
    ///
    /// `Err` is the first unmet obligation of a pump or of a `net` command, returned at once.
    pub fn run(&mut self) -> Result<(), UnmetObligation> {
        loop {
            self.system.pump()?;
            let published = self.system.published();
            let started: Vec<RequestStarted> = published[self.cursor..]
                .iter()
                .filter_map(|event| match event {
                    SystemEvent::RequestStarted(started) => Some(started.clone()),
                    _ => None,
                })
                .collect();
            self.cursor = published.len();
            if started.is_empty() {
                return Ok(());
            }
            for request in started {
                self.answer(request)?;
            }
        }
    }

    /// Asks the transport for one started request, if it is still `Pending`, and issues the
    /// answer on the `net` port. A declared refusal of that command is the `net` component's
    /// outcome and is left as it is.
    fn answer(&mut self, started: RequestStarted) -> Result<(), UnmetObligation> {
        let pending =
            self.system.net.requests()?.iter().any(|row| {
                row.request_id == started.request_id && row.state == RequestState::Pending
            });
        if !pending {
            return Ok(());
        }
        match self.transport.fetch(&started.url.0) {
            Answer::Response {
                status,
                media_type,
                body,
            } => {
                // The transport follows no redirect yet (`story:agent-fetch-redirects`), so the
                // response comes from the address that was asked for.
                self.system.net.deliver_response(DeliverResponse {
                    request_id: started.request_id,
                    url: started.url,
                    status: Status(i64::from(status)),
                    media_type: MediaType(media_type),
                    body,
                })?;
            }
            Answer::Failure(reason) => {
                self.system.net.fail_request(FailRequest {
                    request_id: started.request_id,
                    reason,
                })?;
            }
            Answer::Unanswered => {}
        }
        Ok(())
    }

    /// Issues `OpenWindow`, `OpenTab` in that window and `Navigate` that tab to `address`,
    /// pumping after the first two, and returns the navigation's identity. The navigation is
    /// published by the next [`Browser::run`].
    pub fn start(&mut self, address: &str) -> Result<RequestId, Error> {
        let window_id = match self.system.shell.open_window(OpenWindow {})? {
            OpenWindowOutcome::Opened { window_opened } => window_opened.window_id,
        };
        self.system.pump()?;
        let tab_id = match self.system.shell.open_tab(OpenTab { window_id })? {
            OpenTabOutcome::Opened { tab_opened } => tab_opened.tab_id,
            OpenTabOutcome::NoSuchWindow { error } => {
                return Err(Error::Refused(format!(
                    "surf.shell.OpenTab refused: no window {}",
                    error.window_id.0.0
                )));
            }
        };
        self.system.pump()?;
        let navigation_id = match self.system.navigator.navigate(Navigate {
            tab_id,
            url: Url(address.to_owned()),
        })? {
            NavigateOutcome::Started { navigation_started } => navigation_started.navigation_id,
        };
        Ok(navigation_id)
    }

    /// The rows of the `Navigations`, `Requests` and `Documents` views and every published
    /// `FetchFailed`, for the navigation `navigation_id`.
    pub fn report(&self, navigation_id: RequestId) -> Result<Report, UnmetObligation> {
        Ok(Report {
            navigation_id,
            navigations: self.system.navigator.navigations()?,
            requests: self.system.net.requests()?,
            documents: self.system.document.documents()?,
            fetch_failures: self
                .system
                .published()
                .iter()
                .filter_map(|event| match event {
                    SystemEvent::FetchFailed(failed) => Some(failed.clone()),
                    _ => None,
                })
                .collect(),
        })
    }

    /// [`Browser::start`] at `address`, [`Browser::run`], and the [`Browser::report`] of the
    /// navigation it started.
    pub fn load(&mut self, address: &str) -> Result<Report, Error> {
        let navigation_id = self.start(address)?;
        self.run()?;
        Ok(self.report(navigation_id)?)
    }
}
