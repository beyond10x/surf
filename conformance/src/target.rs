//! The `surf` system as a [`ConformanceTarget`]: the runner's commands, views, events and binding
//! invocations, read from and written to the generated typed API by hand.
//!
//! Each scenario gets a [`Browser`] of its own, assembled when the scenario begins and dropped when
//! it ends. A command is issued on its component's port, and [`Browser::run`] then delivers what it
//! published through the generated bindings before the answer returns. The transport is
//! [`Unanswered`]: the suite ends a request itself, with `surf.net.DeliverResponse` or
//! `surf.net.FailRequest`, so no request is ever answered from the network.
//!
//! What the target reports, and where it reads it:
//!
//! | answer | read from |
//! |---|---|
//! | a command's outcome and error | the typed outcome the port returned |
//! | a command's direct events | what [`System::published`](surf::System::published) gained while it ran, up to the first binding delivery; the scenario errors where that differs from the event the outcome carries |
//! | a view's rows | the component's generated query |
//! | events, eventually | [`System::published`](surf::System::published), the system's log |
//! | invocations | [`System::invocations`](surf::System::invocations), the record the pump keeps |
//!
//! Every command answers a consistency token, `write-<n>` for its `n`th command of the scenario.
//! A view is read after [`Browser::run`] has returned, so it already shows every write, and a read
//! demanding any token this scenario issued is answered at once.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::str::FromStr;

use ess_conformance::scenario::{CommandRef, ErrorRef, EventRef, OutcomeRef};
use ess_conformance::target::{
    ConformanceTarget, DeclaredErrorValue, EventObservationRequest, ExternalOutcomeControl,
    ImplementationIdentity, InvocationObservationRequest, ObservedEvent, ObservedInvocation,
    RedeliveryRequest, ScenarioContext, SemanticCommandRequest, SemanticCommandResult,
    SemanticViewRequest, SemanticViewResult, TargetError, ViewRow,
};
use ess_domain::command::OutcomeName;
use ess_primitives::consistency::ConsistencyToken;
use ess_primitives::node::Node;
use surf::{
    Answer, Browser, DocumentBehaviors, NavigatorBehaviors, NetBehaviors, RendererBehaviors,
    ShellBehaviors, SystemEvent, Transport, UnmetObligation,
};
use surf_system::BindingInvocation;
use surf_types::document::{ParseDocument, ParseDocumentOutcome};
use surf_types::navigation::{
    CancelNavigation, CancelNavigationOutcome, CommitNavigation, CommitNavigationOutcome,
    FailNavigation, FailNavigationOutcome, Navigate, NavigateOutcome, NavigationState,
};
use surf_types::net::{
    DeliverResponse, DeliverResponseOutcome, FailRequest, FailRequestOutcome, FailureReason, Fetch,
    FetchOutcome, MediaType, RequestId, RequestState, Status,
};
use surf_types::render::{PaintFrame, PaintFrameOutcome};
use surf_types::shell::{
    CloseTab, CloseTabOutcome, OpenTab, OpenTabOutcome, OpenWindow, OpenWindowOutcome, TabId,
    TabState, WindowId,
};
use surf_types::url::Url;

use crate::values::{self, Input};

/// A transport that never answers: every request stays `Pending` until the suite ends it.
#[derive(Debug, Clone, Copy, Default)]
pub struct Unanswered;

impl Transport for Unanswered {
    fn fetch(&mut self, _address: &str) -> Answer {
        Answer::Unanswered
    }
}

/// A [`Browser`] over component ports `D`, `N`, `Ne`, `R` and `S`, with the [`Unanswered`]
/// transport.
pub type ConformanceBrowser<D, N, Ne, R, S> = Browser<D, N, Ne, R, S, Unanswered>;

type Assemble<D, N, Ne, R, S> = Box<dyn Fn() -> ConformanceBrowser<D, N, Ne, R, S>>;

type Open<D, N, Ne, R, S> = RefCell<Option<Scenario<D, N, Ne, R, S>>>;

/// The `surf` system as a conformance target: a fresh [`ConformanceBrowser`] per scenario, from
/// the function it was built with.
pub struct SurfTarget<D, N, Ne, R, S> {
    identity: ImplementationIdentity,
    assemble: Assemble<D, N, Ne, R, S>,
    scenario: Open<D, N, Ne, R, S>,
}

struct Scenario<D, N, Ne, R, S> {
    browser: ConformanceBrowser<D, N, Ne, R, S>,
    writes: u64,
}

impl<D, N, Ne, R, S> SurfTarget<D, N, Ne, R, S> {
    /// A target named `identity` that assembles each scenario's system with `assemble`.
    pub fn new(
        identity: ImplementationIdentity,
        assemble: impl Fn() -> ConformanceBrowser<D, N, Ne, R, S> + 'static,
    ) -> Self {
        Self {
            identity,
            assemble: Box::new(assemble),
            scenario: RefCell::new(None),
        }
    }
}

/// What one command answered: its outcome, the declared error a refusing branch carries, and the
/// event the outcome carries, which [`reported_events`] holds against the system's log.
struct Answered {
    outcome: &'static str,
    error: Option<DeclaredErrorValue>,
    events: Vec<SystemEvent>,
}

fn took(outcome: &'static str, event: SystemEvent) -> Answered {
    Answered {
        outcome,
        error: None,
        events: vec![event],
    }
}

fn refused(
    outcome: &'static str,
    error: &str,
    fields: Vec<(&'static str, Node)>,
) -> Result<Answered, TargetError> {
    let mut value = DeclaredErrorValue::new(named::<ErrorRef>(error)?);
    for (field, node) in fields {
        value = value.with(field, node);
    }
    Ok(Answered {
        outcome,
        error: Some(value),
        events: Vec::new(),
    })
}

/// One of the five components, as the publisher of an event or the receiver of a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Component {
    Document,
    Navigator,
    Net,
    Renderer,
    Shell,
}

/// The component a command of this name is issued on, by its domain.
fn receiver(command: &str) -> Option<Component> {
    let domain = command.rsplit_once('.')?.0;
    Some(match domain {
        "surf.document" => Component::Document,
        "surf.navigation" => Component::Navigator,
        "surf.net" | "surf.url" => Component::Net,
        "surf.render" => Component::Renderer,
        "surf.shell" => Component::Shell,
        _ => return None,
    })
}

/// The component whose outbox `event` came from.
fn publisher(event: &SystemEvent) -> Component {
    match event {
        SystemEvent::DocumentParsed(_) => Component::Document,
        SystemEvent::NavigationCancelled(_)
        | SystemEvent::NavigationCommitted(_)
        | SystemEvent::NavigationFailed(_)
        | SystemEvent::NavigationStarted(_) => Component::Navigator,
        SystemEvent::FetchFailed(_)
        | SystemEvent::RequestStarted(_)
        | SystemEvent::ResponseReceived(_) => Component::Net,
        SystemEvent::FramePainted(_) => Component::Renderer,
        SystemEvent::TabClosed(_) | SystemEvent::TabOpened(_) | SystemEvent::WindowOpened(_) => {
            Component::Shell
        }
    }
}

/// The component a binding's invocation issued its command on.
fn invoked_on(invocation: &BindingInvocation) -> Component {
    match invocation {
        BindingInvocation::CommitOnResponse(_) | BindingInvocation::FailOnFetchFailed(_) => {
            Component::Navigator
        }
        BindingInvocation::FetchOnNavigationStarted(_) => Component::Net,
        BindingInvocation::ParseOnNavigationCommitted(_) => Component::Document,
    }
}

fn names(events: &[SystemEvent]) -> String {
    if events.is_empty() {
        return "no event".to_owned();
    }
    events
        .iter()
        .map(|event| format!("{event:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The events a command of `command` published itself, read from `gained`, what the system's log
/// gained while the command and [`Browser::run`] after it ran, and checked against `answered`, the
/// events its typed outcome carries. `invoked` is what the bindings invoked meanwhile.
///
/// Before the command the system was quiescent, so the pump's first collection puts exactly the
/// commanding component's outbox on the log, and every binding delivery appends after it. The
/// command's own events are therefore the run of the commanding component's events `gained`
/// starts with, provided nothing later in `gained` comes from that component: no binding invoked
/// a command on it, and no later event is its. The [`Unanswered`] transport issues no command.
/// Where that does not hold, the log cannot be split, and the answer is an error.
///
/// `Err` also when the outcome and the log disagree: an event the outcome carries that the log
/// lacks, an event on the log behind a refusal, or other values.
fn reported_events(
    command: &str,
    answered: &[SystemEvent],
    gained: &[SystemEvent],
    invoked: &[BindingInvocation],
) -> Result<Vec<SystemEvent>, String> {
    let component = receiver(command)
        .ok_or_else(|| format!("`{command}` is issued on no component of the surf system"))?;
    if let Some(invocation) = invoked
        .iter()
        .find(|invocation| invoked_on(invocation) == component)
    {
        return Err(format!(
            "a binding invoked {invocation:?} on the component `{command}` was issued on, so the \
             log does not tell the command's own events from that one's"
        ));
    }
    let direct = gained
        .iter()
        .take_while(|event| publisher(event) == component)
        .count();
    if let Some(later) = gained[direct..]
        .iter()
        .find(|event| publisher(event) == component)
    {
        return Err(format!(
            "{later:?} reached the log after a binding delivery, from the component `{command}` \
             was issued on, and no binding invoked that component"
        ));
    }
    let logged = &gained[..direct];
    if logged != answered {
        return Err(format!(
            "the typed outcome carries {} and the system's log gained {}",
            names(answered),
            names(logged)
        ));
    }
    Ok(logged.to_vec())
}

fn named<T: FromStr>(name: &str) -> Result<T, TargetError>
where
    T::Err: std::fmt::Display,
{
    name.parse().map_err(|error: T::Err| {
        TargetError::unavailable(format!("naming `{name}`"), error.to_string())
    })
}

fn unmet(operation: &str) -> impl Fn(UnmetObligation) -> TargetError + '_ {
    move |obligation| TargetError::unavailable(operation, obligation.to_string())
}

fn uuid_of(value: &RequestId) -> Node {
    values::uuid(&value.0)
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

fn tab_state(state: TabState) -> &'static str {
    match state {
        TabState::Closed => "Closed",
        TabState::Open => "Open",
    }
}

fn failure_reason(reason: FailureReason) -> &'static str {
    match reason {
        FailureReason::Unreachable => "Unreachable",
        FailureReason::UnsupportedScheme => "UnsupportedScheme",
        FailureReason::MalformedResponse => "MalformedResponse",
    }
}

fn failure_reason_named(name: &str) -> Option<FailureReason> {
    [
        FailureReason::Unreachable,
        FailureReason::UnsupportedScheme,
        FailureReason::MalformedResponse,
    ]
    .into_iter()
    .find(|reason| failure_reason(*reason) == name)
}

/// The payload of an event on the system's log, by declared field name.
fn payload(event: &SystemEvent) -> Vec<(&'static str, Node)> {
    use values::{bytes, integer, text, uuid};
    match event {
        SystemEvent::DocumentParsed(e) => {
            vec![
                ("document_id", uuid(&e.document_id.0)),
                ("url", text(&e.url.0)),
            ]
        }
        SystemEvent::NavigationCancelled(e) => vec![
            ("navigation_id", uuid_of(&e.navigation_id)),
            ("tab_id", uuid(&e.tab_id.0)),
        ],
        SystemEvent::NavigationCommitted(e) => vec![
            ("navigation_id", uuid_of(&e.navigation_id)),
            ("tab_id", uuid(&e.tab_id.0)),
            ("url", text(&e.url.0)),
            ("status", integer(e.status.0)),
            ("media_type", text(&e.media_type.0)),
            ("body", bytes(&e.body)),
        ],
        SystemEvent::NavigationFailed(e) => vec![
            ("navigation_id", uuid_of(&e.navigation_id)),
            ("tab_id", uuid(&e.tab_id.0)),
            ("url", text(&e.url.0)),
        ],
        SystemEvent::NavigationStarted(e) => vec![
            ("navigation_id", uuid_of(&e.navigation_id)),
            ("tab_id", uuid(&e.tab_id.0)),
            ("url", text(&e.url.0)),
        ],
        SystemEvent::FetchFailed(e) => vec![
            ("request_id", uuid_of(&e.request_id)),
            ("url", text(&e.url.0)),
            ("reason", text(failure_reason(e.reason))),
        ],
        SystemEvent::RequestStarted(e) => {
            vec![
                ("request_id", uuid_of(&e.request_id)),
                ("url", text(&e.url.0)),
            ]
        }
        SystemEvent::ResponseReceived(e) => vec![
            ("request_id", uuid_of(&e.request_id)),
            ("url", text(&e.url.0)),
            ("status", integer(e.status.0)),
            ("media_type", text(&e.media_type.0)),
            ("body", bytes(&e.body)),
        ],
        SystemEvent::FramePainted(e) => vec![
            ("frame_id", uuid(&e.frame_id.0)),
            ("document_id", uuid(&e.document_id.0)),
            ("width", integer(e.width)),
            ("height", integer(e.height)),
        ],
        SystemEvent::TabClosed(e) => {
            vec![
                ("tab_id", uuid(&e.tab_id.0)),
                ("window_id", uuid(&e.window_id.0)),
            ]
        }
        SystemEvent::TabOpened(e) => {
            vec![
                ("tab_id", uuid(&e.tab_id.0)),
                ("window_id", uuid(&e.window_id.0)),
            ]
        }
        SystemEvent::WindowOpened(e) => vec![("window_id", uuid(&e.window_id.0))],
    }
}

fn fields(pairs: Vec<(&'static str, Node)>) -> BTreeMap<String, Node> {
    pairs
        .into_iter()
        .map(|(field, node)| (field.to_owned(), node))
        .collect()
}

fn observed(
    event: &SystemEvent,
    correlation: &ess_primitives::ids::CorrelationId,
) -> Result<ObservedEvent, TargetError> {
    let mut observed = ObservedEvent::new(named::<EventRef>(event.name())?);
    observed.payload = fields(payload(event));
    Ok(observed.in_activity(correlation.clone()))
}

/// The binding, the command it invoked and the input it passed, by declared field name.
fn invocation(
    invoked: &BindingInvocation,
) -> (&'static str, &'static str, Vec<(&'static str, Node)>) {
    use values::{bytes, integer, text};
    match invoked {
        BindingInvocation::CommitOnResponse(input) => (
            "commit-on-response",
            "surf.navigation.CommitNavigation",
            vec![
                ("navigation_id", uuid_of(&input.navigation_id)),
                ("status", integer(input.status.0)),
                ("media_type", text(&input.media_type.0)),
                ("body", bytes(&input.body)),
            ],
        ),
        BindingInvocation::FailOnFetchFailed(input) => (
            "fail-on-fetch-failed",
            "surf.navigation.FailNavigation",
            vec![("navigation_id", uuid_of(&input.navigation_id))],
        ),
        BindingInvocation::FetchOnNavigationStarted(input) => (
            "fetch-on-navigation-started",
            "surf.net.Fetch",
            vec![
                ("request_id", uuid_of(&input.request_id)),
                ("url", text(&input.url.0)),
            ],
        ),
        BindingInvocation::ParseOnNavigationCommitted(input) => (
            "parse-on-navigation-committed",
            "surf.document.ParseDocument",
            vec![("url", text(&input.url.0)), ("body", bytes(&input.body))],
        ),
    }
}

impl<D, N, Ne, R, S> SurfTarget<D, N, Ne, R, S>
where
    D: DocumentBehaviors,
    N: NavigatorBehaviors,
    Ne: NetBehaviors,
    R: RendererBehaviors,
    S: ShellBehaviors,
{
    /// Runs `operation` on the open scenario.
    fn with<T>(
        &self,
        operation: impl FnOnce(&mut Scenario<D, N, Ne, R, S>) -> Result<T, TargetError>,
    ) -> Result<T, TargetError> {
        let mut open = self.scenario.borrow_mut();
        let scenario = open.as_mut().ok_or_else(|| {
            TargetError::unavailable("reaching the system", "no scenario is open")
        })?;
        operation(scenario)
    }

    /// Issues one command on its component's port.
    #[allow(clippy::too_many_lines)]
    fn issue(
        browser: &mut ConformanceBrowser<D, N, Ne, R, S>,
        command: &str,
        fields: &BTreeMap<String, Node>,
    ) -> Result<Answered, TargetError> {
        use values::{integer, uuid};
        let mut input = Input::new(command, fields);
        let invoking = format!("invoking `{command}`");
        let unmet = unmet(&invoking);
        Ok(match command {
            "surf.shell.OpenWindow" => {
                input.finish()?;
                match browser
                    .shell_mut()
                    .open_window(OpenWindow {})
                    .map_err(unmet)?
                {
                    OpenWindowOutcome::Opened { window_opened } => {
                        took("opened", SystemEvent::WindowOpened(window_opened))
                    }
                }
            }
            "surf.shell.OpenTab" => {
                let window_id = WindowId(input.uuid("window_id")?);
                input.finish()?;
                match browser
                    .shell_mut()
                    .open_tab(OpenTab { window_id })
                    .map_err(unmet)?
                {
                    OpenTabOutcome::Opened { tab_opened } => {
                        took("opened", SystemEvent::TabOpened(tab_opened))
                    }
                    OpenTabOutcome::NoSuchWindow { error } => refused(
                        "no-such-window",
                        "surf.shell.WindowNotFound",
                        vec![("window_id", uuid(&error.window_id.0))],
                    )?,
                }
            }
            "surf.shell.CloseTab" => {
                let tab_id = TabId(input.uuid("tab_id")?);
                input.finish()?;
                match browser
                    .shell_mut()
                    .close_tab(CloseTab { tab_id })
                    .map_err(unmet)?
                {
                    CloseTabOutcome::Closed { tab_closed } => {
                        took("closed", SystemEvent::TabClosed(tab_closed))
                    }
                    CloseTabOutcome::NoSuchTab { error } => refused(
                        "no-such-tab",
                        "surf.shell.TabNotFound",
                        vec![("tab_id", uuid(&error.tab_id.0))],
                    )?,
                    CloseTabOutcome::WrongState { error } => refused(
                        "wrong-state",
                        "surf.shell.TabStateConflict",
                        vec![("state", values::text(tab_state(error.state)))],
                    )?,
                }
            }
            "surf.navigation.Navigate" => {
                let tab_id = TabId(input.uuid("tab_id")?);
                let url = Url(input.text("url")?);
                input.finish()?;
                match browser
                    .navigator_mut()
                    .navigate(Navigate { tab_id, url })
                    .map_err(unmet)?
                {
                    NavigateOutcome::Started { navigation_started } => took(
                        "started",
                        SystemEvent::NavigationStarted(navigation_started),
                    ),
                }
            }
            "surf.navigation.CancelNavigation" => {
                let navigation_id = RequestId(input.uuid("navigation_id")?);
                input.finish()?;
                match browser
                    .navigator_mut()
                    .cancel_navigation(CancelNavigation { navigation_id })
                    .map_err(unmet)?
                {
                    CancelNavigationOutcome::Cancelled {
                        navigation_cancelled,
                    } => took(
                        "cancelled",
                        SystemEvent::NavigationCancelled(navigation_cancelled),
                    ),
                    CancelNavigationOutcome::NoSuchNavigation { error } => refused(
                        "no-such-navigation",
                        "surf.navigation.NavigationNotFound",
                        vec![("navigation_id", uuid_of(&error.navigation_id))],
                    )?,
                    CancelNavigationOutcome::WrongState { error } => refused(
                        "wrong-state",
                        "surf.navigation.NavigationStateConflict",
                        vec![("state", values::text(navigation_state(error.state)))],
                    )?,
                }
            }
            "surf.navigation.CommitNavigation" => {
                let navigation_id = RequestId(input.uuid("navigation_id")?);
                let status = Status(input.integer("status")?);
                let media_type = MediaType(input.text("media_type")?);
                let body = input.bytes("body")?;
                input.finish()?;
                match browser
                    .navigator_mut()
                    .commit_navigation(CommitNavigation {
                        navigation_id,
                        status,
                        media_type,
                        body,
                    })
                    .map_err(unmet)?
                {
                    CommitNavigationOutcome::Committed {
                        navigation_committed,
                    } => took(
                        "committed",
                        SystemEvent::NavigationCommitted(navigation_committed),
                    ),
                    CommitNavigationOutcome::NoSuchNavigation { error } => refused(
                        "no-such-navigation",
                        "surf.navigation.NavigationNotFound",
                        vec![("navigation_id", uuid_of(&error.navigation_id))],
                    )?,
                    CommitNavigationOutcome::WrongState { error } => refused(
                        "wrong-state",
                        "surf.navigation.NavigationStateConflict",
                        vec![("state", values::text(navigation_state(error.state)))],
                    )?,
                }
            }
            "surf.navigation.FailNavigation" => {
                let navigation_id = RequestId(input.uuid("navigation_id")?);
                input.finish()?;
                match browser
                    .navigator_mut()
                    .fail_navigation(FailNavigation { navigation_id })
                    .map_err(unmet)?
                {
                    FailNavigationOutcome::Failed { navigation_failed } => {
                        took("failed", SystemEvent::NavigationFailed(navigation_failed))
                    }
                    FailNavigationOutcome::NoSuchNavigation { error } => refused(
                        "no-such-navigation",
                        "surf.navigation.NavigationNotFound",
                        vec![("navigation_id", uuid_of(&error.navigation_id))],
                    )?,
                    FailNavigationOutcome::WrongState { error } => refused(
                        "wrong-state",
                        "surf.navigation.NavigationStateConflict",
                        vec![("state", values::text(navigation_state(error.state)))],
                    )?,
                }
            }
            "surf.net.Fetch" => {
                let request_id = RequestId(input.uuid("request_id")?);
                let url = Url(input.text("url")?);
                input.finish()?;
                match browser
                    .net_mut()
                    .fetch(Fetch { request_id, url })
                    .map_err(unmet)?
                {
                    FetchOutcome::Started { request_started } => {
                        took("started", SystemEvent::RequestStarted(request_started))
                    }
                }
            }
            "surf.net.DeliverResponse" => {
                let request_id = RequestId(input.uuid("request_id")?);
                let status = Status(input.integer("status")?);
                let media_type = MediaType(input.text("media_type")?);
                let body = input.bytes("body")?;
                input.finish()?;
                match browser
                    .net_mut()
                    .deliver_response(DeliverResponse {
                        request_id,
                        status,
                        media_type,
                        body,
                    })
                    .map_err(unmet)?
                {
                    DeliverResponseOutcome::Responded { response_received } => took(
                        "responded",
                        SystemEvent::ResponseReceived(response_received),
                    ),
                    DeliverResponseOutcome::NoSuchRequest { error } => refused(
                        "no-such-request",
                        "surf.net.RequestNotFound",
                        vec![("request_id", uuid_of(&error.request_id))],
                    )?,
                    DeliverResponseOutcome::WrongState { error } => refused(
                        "wrong-state",
                        "surf.net.RequestStateConflict",
                        vec![("state", values::text(request_state(error.state)))],
                    )?,
                }
            }
            "surf.net.FailRequest" => {
                let request_id = RequestId(input.uuid("request_id")?);
                let reason = input.variant("reason", failure_reason_named)?;
                input.finish()?;
                match browser
                    .net_mut()
                    .fail_request(FailRequest { request_id, reason })
                    .map_err(unmet)?
                {
                    FailRequestOutcome::Failed { fetch_failed } => {
                        took("failed", SystemEvent::FetchFailed(fetch_failed))
                    }
                    FailRequestOutcome::NoSuchRequest { error } => refused(
                        "no-such-request",
                        "surf.net.RequestNotFound",
                        vec![("request_id", uuid_of(&error.request_id))],
                    )?,
                    FailRequestOutcome::WrongState { error } => refused(
                        "wrong-state",
                        "surf.net.RequestStateConflict",
                        vec![("state", values::text(request_state(error.state)))],
                    )?,
                }
            }
            "surf.document.ParseDocument" => {
                let url = Url(input.text("url")?);
                let body = input.bytes("body")?;
                input.finish()?;
                match browser
                    .document_mut()
                    .parse_document(ParseDocument { url, body })
                    .map_err(unmet)?
                {
                    ParseDocumentOutcome::Parsed { document_parsed } => {
                        took("parsed", SystemEvent::DocumentParsed(document_parsed))
                    }
                }
            }
            "surf.render.PaintFrame" => {
                let document_id = surf_types::document::DocumentId(input.uuid("document_id")?);
                let width = input.integer("width")?;
                let height = input.integer("height")?;
                input.finish()?;
                match browser
                    .renderer_mut()
                    .paint_frame(PaintFrame {
                        document_id,
                        width,
                        height,
                    })
                    .map_err(unmet)?
                {
                    PaintFrameOutcome::Painted { frame_painted } => {
                        took("painted", SystemEvent::FramePainted(frame_painted))
                    }
                    PaintFrameOutcome::InvalidViewport { error } => refused(
                        "invalid-viewport",
                        "surf.render.InvalidViewport",
                        vec![
                            ("width", integer(error.width)),
                            ("height", integer(error.height)),
                        ],
                    )?,
                }
            }
            other => {
                return Err(TargetError::unsupported(
                    format!("invoking `{other}`"),
                    "the surf system accepts no command of that name",
                ));
            }
        })
    }

    /// The rows of one view, by projected field name.
    fn rows(
        browser: &ConformanceBrowser<D, N, Ne, R, S>,
        view: &str,
    ) -> Result<Vec<ViewRow>, TargetError> {
        use values::{integer, text, uuid};
        let reading = format!("reading `{view}`");
        let unmet = unmet(&reading);
        let system = browser.system();
        let rows: Vec<Vec<(&'static str, Node)>> = match view {
            "surf.document.Documents" => system
                .document
                .documents()
                .map_err(unmet)?
                .into_iter()
                .map(|row| {
                    vec![
                        ("document_id", uuid(&row.document_id.0)),
                        ("url", text(&row.url.0)),
                    ]
                })
                .collect(),
            "surf.navigation.Navigations" => system
                .navigator
                .navigations()
                .map_err(unmet)?
                .into_iter()
                .map(|row| {
                    vec![
                        ("navigation_id", uuid_of(&row.navigation_id)),
                        ("tab_id", uuid(&row.tab_id.0)),
                        ("url", text(&row.url.0)),
                        ("state", text(navigation_state(row.state))),
                    ]
                })
                .collect(),
            "surf.net.Requests" => system
                .net
                .requests()
                .map_err(unmet)?
                .into_iter()
                .map(|row| {
                    vec![
                        ("request_id", uuid_of(&row.request_id)),
                        ("url", text(&row.url.0)),
                        ("state", text(request_state(row.state))),
                    ]
                })
                .collect(),
            "surf.render.Frames" => system
                .renderer
                .frames()
                .map_err(unmet)?
                .into_iter()
                .map(|row| {
                    vec![
                        ("frame_id", uuid(&row.frame_id.0)),
                        ("document_id", uuid(&row.document_id.0)),
                        ("width", integer(row.width)),
                        ("height", integer(row.height)),
                    ]
                })
                .collect(),
            "surf.shell.Tabs" => system
                .shell
                .tabs()
                .map_err(unmet)?
                .into_iter()
                .map(|row| {
                    vec![
                        ("tab_id", uuid(&row.tab_id.0)),
                        ("window_id", uuid(&row.window_id.0)),
                        ("state", text(tab_state(row.state))),
                    ]
                })
                .collect(),
            "surf.shell.Windows" => system
                .shell
                .windows()
                .map_err(unmet)?
                .into_iter()
                .map(|row| vec![("window_id", uuid(&row.window_id.0))])
                .collect(),
            other => {
                return Err(TargetError::unsupported(
                    format!("reading `{other}`"),
                    "the surf system declares no view of that name",
                ));
            }
        };
        Ok(rows.into_iter().map(fields).collect())
    }
}

impl<D, N, Ne, R, S> ConformanceTarget for SurfTarget<D, N, Ne, R, S>
where
    D: DocumentBehaviors,
    N: NavigatorBehaviors,
    Ne: NetBehaviors,
    R: RendererBehaviors,
    S: ShellBehaviors,
{
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        Ok(self.identity.clone())
    }

    fn begin_scenario(&self, _scenario: &ScenarioContext) -> Result<(), TargetError> {
        *self.scenario.borrow_mut() = Some(Scenario {
            browser: (self.assemble)(),
            writes: 0,
        });
        Ok(())
    }

    fn end_scenario(&self, _scenario: &ScenarioContext) -> Result<(), TargetError> {
        *self.scenario.borrow_mut() = None;
        Ok(())
    }

    fn execute_command(
        &self,
        request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        self.with(|scenario| {
            let command = request.command.to_string();
            let logged = scenario.browser.system().published().len();
            let recorded = scenario.browser.system().invocations().len();
            let answered = Self::issue(&mut scenario.browser, &command, &request.input)?;
            let delivering = format!("delivering what `{command}` published");
            scenario.browser.run().map_err(unmet(&delivering))?;
            let system = scenario.browser.system();
            let reading = format!("reading what `{command}` published");
            let (Some(gained), Some(invoked)) = (
                system.published().get(logged..),
                system.invocations().get(recorded..),
            ) else {
                return Err(TargetError::unavailable(
                    reading,
                    "the system's log or invocation record lost entries",
                ));
            };
            let events = reported_events(&command, &answered.events, gained, invoked)
                .map_err(|disagreement| TargetError::unavailable(reading, disagreement))?;
            scenario.writes += 1;
            let token =
                ConsistencyToken::new(format!("write-{}", scenario.writes)).map_err(|error| {
                    TargetError::unavailable("issuing a consistency token", error.to_string())
                })?;
            let outcome = OutcomeName::new(answered.outcome).map_err(|error| {
                TargetError::unavailable(
                    format!("naming `{}`", answered.outcome),
                    error.to_string(),
                )
            })?;
            let mut result = SemanticCommandResult::took(OutcomeRef::new(
                named::<CommandRef>(&command)?,
                outcome,
            ))
            .with_consistency(token);
            result.error = answered.error;
            for event in &events {
                result = result.emitting(observed(event, &request.correlation)?);
            }
            Ok(result)
        })
    }

    fn query_view(&self, request: SemanticViewRequest) -> Result<SemanticViewResult, TargetError> {
        self.with(|scenario| {
            let view = request.view.to_string();
            if !request.params.is_empty() {
                return Err(TargetError::unavailable(
                    format!("reading `{view}`"),
                    "the surf system declares no view parameter",
                ));
            }
            if let Some(token) = request.consistency.token() {
                let issued = token
                    .as_str()
                    .strip_prefix("write-")
                    .and_then(|number| number.parse::<u64>().ok())
                    .is_some_and(|number| (1..=scenario.writes).contains(&number));
                if !issued {
                    return Err(TargetError::unavailable(
                        format!("reading `{view}` no older than `{token}`"),
                        "this scenario issued no such token",
                    ));
                }
            }
            Ok(SemanticViewResult::of(Self::rows(
                &scenario.browser,
                &view,
            )?))
        })
    }

    fn observe_events(
        &self,
        request: EventObservationRequest,
    ) -> Result<Vec<ObservedEvent>, TargetError> {
        self.with(|scenario| {
            let event = request.event.to_string();
            scenario
                .browser
                .system()
                .published()
                .iter()
                .filter(|published| published.name() == event)
                .map(|published| observed(published, &request.correlation))
                .collect()
        })
    }

    fn observe_invocations(
        &self,
        request: InvocationObservationRequest,
    ) -> Result<Vec<ObservedInvocation>, TargetError> {
        let binding = request.binding.to_string();
        let command = request.command.to_string();
        self.with(|scenario| {
            Ok(scenario
                .browser
                .system()
                .invocations()
                .iter()
                .map(invocation)
                .filter(|(by, invoked, _)| *by == binding && *invoked == command)
                .map(|(_, _, input)| ObservedInvocation {
                    binding: request.binding.clone(),
                    command: request.command.clone(),
                    input: fields(input),
                })
                .collect())
        })
    }

    fn configure_external_outcome(
        &self,
        request: ExternalOutcomeControl,
    ) -> Result<(), TargetError> {
        Err(TargetError::unsupported(
            format!("forcing `{}`", request.force),
            "the surf specification declares no external outcome",
        ))
    }

    fn redeliver_event(&self, request: RedeliveryRequest) -> Result<(), TargetError> {
        Err(TargetError::unsupported(
            format!("delivering `{}` again", request.event),
            "every surf binding is at_most_once, and the surf library hands out no mutable system \
             to redeliver through",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::reported_events;
    use surf::SystemEvent;
    use surf_system::BindingInvocation;
    use surf_types::document::{DocumentId, DocumentParsed};
    use surf_types::navigation::{
        CommitNavigation, NavigationCancelled, NavigationCommitted, NavigationStarted,
    };
    use surf_types::net::{Fetch, MediaType, RequestId, RequestStarted, ResponseReceived, Status};
    use surf_types::primitives::Uuid;
    use surf_types::shell::TabId;
    use surf_types::url::Url;

    const NAVIGATION: &str = "3f1d5b7e-0000-4000-8000-000000000010";
    const TAB: &str = "3f1d5b7e-0000-4000-8000-000000000020";
    const ADDRESS: &str = "https://example.org/";

    fn navigation_id() -> RequestId {
        RequestId(Uuid(NAVIGATION.to_owned()))
    }

    fn cancelled() -> SystemEvent {
        SystemEvent::NavigationCancelled(NavigationCancelled {
            navigation_id: navigation_id(),
            tab_id: TabId(Uuid(TAB.to_owned())),
        })
    }

    fn started() -> SystemEvent {
        SystemEvent::NavigationStarted(NavigationStarted {
            navigation_id: navigation_id(),
            tab_id: TabId(Uuid(TAB.to_owned())),
            url: Url(ADDRESS.to_owned()),
        })
    }

    fn request_started() -> SystemEvent {
        SystemEvent::RequestStarted(RequestStarted {
            request_id: navigation_id(),
            url: Url(ADDRESS.to_owned()),
        })
    }

    fn fetch() -> BindingInvocation {
        BindingInvocation::FetchOnNavigationStarted(Fetch {
            request_id: navigation_id(),
            url: Url(ADDRESS.to_owned()),
        })
    }

    fn received() -> SystemEvent {
        SystemEvent::ResponseReceived(ResponseReceived {
            request_id: navigation_id(),
            url: Url(ADDRESS.to_owned()),
            status: Status(200),
            media_type: MediaType("text/html".to_owned()),
            body: Vec::new(),
        })
    }

    fn committed() -> SystemEvent {
        SystemEvent::NavigationCommitted(NavigationCommitted {
            navigation_id: navigation_id(),
            tab_id: TabId(Uuid(TAB.to_owned())),
            url: Url(ADDRESS.to_owned()),
            status: Status(200),
            media_type: MediaType("text/html".to_owned()),
            body: Vec::new(),
        })
    }

    fn commit() -> BindingInvocation {
        BindingInvocation::CommitOnResponse(CommitNavigation {
            navigation_id: navigation_id(),
            status: Status(200),
            media_type: MediaType("text/html".to_owned()),
            body: Vec::new(),
        })
    }

    fn parsed() -> SystemEvent {
        SystemEvent::DocumentParsed(DocumentParsed {
            document_id: DocumentId(Uuid("3f1d5b7e-0000-4000-8000-000000000030".to_owned())),
            url: Url(ADDRESS.to_owned()),
        })
    }

    /// The review-2 mutant `REVIEW2_NO_CANCELLED_EVENT`: the outcome says `cancelled` and carries
    /// the event, and the port never put it on the log.
    #[test]
    fn an_outcome_event_missing_from_the_log_is_an_error() {
        let answer = reported_events("surf.navigation.CancelNavigation", &[cancelled()], &[], &[]);
        assert!(answer.is_err(), "{answer:?}");
    }

    /// The review-2 mutant `REVIEW2_EVENT_ON_REFUSAL`: a refusal carries no event, and the port
    /// published one anyway.
    #[test]
    fn an_event_on_the_log_behind_a_refusal_is_an_error() {
        let answer = reported_events("surf.navigation.CommitNavigation", &[], &[cancelled()], &[]);
        assert!(answer.is_err(), "{answer:?}");
    }

    /// The log holds other values than the outcome carries.
    #[test]
    fn a_logged_event_other_than_the_outcome_carries_is_an_error() {
        let mut logged = started();
        if let SystemEvent::NavigationStarted(started) = &mut logged {
            started.url = Url("https://example.org/elsewhere".to_owned());
        }
        let answer = reported_events(
            "surf.navigation.Navigate",
            &[started()],
            &[logged, request_started()],
            &[fetch()],
        );
        assert!(answer.is_err(), "{answer:?}");
    }

    /// What a binding's command published is the other component's, observed eventually, and
    /// not among the command's own events.
    #[test]
    fn the_direct_events_end_where_the_bindings_deliveries_begin() {
        assert_eq!(
            reported_events(
                "surf.navigation.Navigate",
                &[started()],
                &[started(), request_started()],
                &[fetch()],
            ),
            Ok(vec![started()])
        );
        assert_eq!(
            reported_events(
                "surf.net.DeliverResponse",
                &[received()],
                &[received(), committed(), parsed()],
                &[commit()],
            ),
            Ok(vec![received()])
        );
    }

    /// A refusal that published nothing reports nothing.
    #[test]
    fn a_refusal_with_nothing_logged_reports_no_event() {
        assert_eq!(
            reported_events("surf.navigation.CancelNavigation", &[], &[], &[]),
            Ok(Vec::new())
        );
    }

    /// A binding that invoked a command of the commanding component could publish into the run
    /// the direct events are read from, so the log cannot be split and the adapter says so.
    #[test]
    fn a_binding_back_into_the_commanding_component_is_an_error() {
        let answer = reported_events(
            "surf.net.DeliverResponse",
            &[received()],
            &[received(), committed()],
            &[commit(), fetch()],
        );
        assert!(answer.is_err(), "{answer:?}");
    }

    /// A command the adapter cannot place in a component is an error, not an empty answer.
    #[test]
    fn a_command_of_no_component_is_an_error() {
        let answer = reported_events("surf.elsewhere.Act", &[], &[], &[]);
        assert!(answer.is_err(), "{answer:?}");
    }
}
