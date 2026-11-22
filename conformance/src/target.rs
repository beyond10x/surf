//! The `surf` system as a [`ConformanceTarget`]: the runner's commands, views, events and binding
//! invocations, read from and written to the generated typed API by hand.
//!
//! Each scenario gets a [`Browser`] of its own, assembled when the scenario begins and dropped when
//! it ends. A command is issued on its component's port, and [`Browser::run`] then delivers what it
//! published through the generated bindings before the answer returns. The transport is
//! [`Unanswered`]: the suite ends a request itself, with `surf.net.DeliverResponse` or
//! `surf.net.FailRequest`, so no request is ever answered from the network. The parser step is
//! [`Unparsed`]: the suite ends a parse itself, with `surf.document.AddElement` and
//! `surf.document.FinishParsing`, so no body is ever parsed by `surf-html`.
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
    ConformanceTarget, DeclaredErrorValue, EntitySetupRequest, EventObservationRequest,
    ExternalOutcomeControl, ImplementationIdentity, InvocationObservationRequest, ObservedEvent,
    ObservedInvocation, RedeliveryRequest, ScenarioContext, SemanticCommandRequest,
    SemanticCommandResult, SemanticViewRequest, SemanticViewResult, TargetError, ViewRow,
};
use ess_domain::command::OutcomeName;
use ess_primitives::consistency::ConsistencyToken;
use ess_primitives::node::Node;
use surf::{
    AgentBehaviors, Answer, Browser, DocumentBehaviors, NavigatorBehaviors, NetBehaviors, Parse,
    Parser, ParsingStarted, RendererBehaviors, RequestStarted, ShellBehaviors, SystemEvent,
    Transport, UnmetObligation,
};
use surf_ports::{AgentPorts, Caller, DocumentPorts};
use surf_system::BindingInvocation;
use surf_types::agent::{
    ActionCommand, CloseSession, CloseSessionOutcome, Fill, FillOutcome, Follow, FollowOutcome, Go,
    GoOutcome, NoteNavigationCommitted, NoteNavigationCommittedOutcome, NoteNavigationEnded,
    NoteNavigationEndedOutcome, NoteNavigationFailed, NoteNavigationFailedOutcome,
    NoteNavigationStarted, NoteNavigationStartedOutcome, OpenSession, OpenSessionOutcome, Pick,
    PickOutcome, Press, PressOutcome, RecordAction, RecordActionOutcome, SessionData, SessionId,
    SessionSnapshot, SessionState,
};
use surf_types::behaviour::{ElementStorage, ListOptionStorage, SessionStorage, SubmissionStorage};
use surf_types::document::{
    AbandonSubmission, AbandonSubmissionOutcome, AddElement, AddElementOutcome, AddForm,
    AddFormOutcome, AddOption, AddOptionOutcome, Check, CheckOutcome, DocumentId, DocumentState,
    ElementData, ElementId, ElementKind, ElementSnapshot, ElementState, EncodingName,
    FinishParsing, FinishParsingOutcome, FormId, FormState, ListOptionData, ListOptionSnapshot,
    OptionId, OptionState, ParseDocument, ParseDocumentOutcome, ParseLimit, SelectOption,
    SelectOptionOutcome, SetValue, SetValueOutcome, StartSubmission, StartSubmissionOutcome,
    SubmissionData, SubmissionEndReason, SubmissionId, SubmissionSnapshot, SubmissionState,
    SubmitForm, SubmitFormOutcome,
};
use surf_types::navigation::{
    CancelNavigation, CancelNavigationOutcome, CommitNavigation, CommitNavigationOutcome,
    FailNavigation, FailNavigationOutcome, Navigate, NavigateOutcome, NavigationState,
};
use surf_types::net::{
    DeliverResponse, DeliverResponseOutcome, FailRequest, FailRequestOutcome, FailureReason, Fetch,
    FetchOutcome, MediaType, Method, RequestId, RequestState, Status,
};
use surf_types::primitives::Uuid;
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
    fn fetch(&mut self, _request: &RequestStarted) -> Answer {
        Answer::Unanswered
    }
}

/// A parser step that never answers: every document stays `Parsing` until the suite ends its
/// parse itself, with `surf.document.AddElement` and `surf.document.FinishParsing`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Unparsed;

impl Parser for Unparsed {
    fn parse(&mut self, _started: &ParsingStarted) -> Parse {
        Parse::Unanswered
    }
}

/// A [`Browser`] over component ports `A`, `D`, `N`, `Ne`, `R` and `S`, with the
/// [`Unanswered`] transport: what the function a [`SurfTarget`] is built with returns. The
/// target replaces its parser step with [`Unparsed`] before the scenario's first command.
pub type ConformanceBrowser<A, D, N, Ne, R, S> = Browser<A, D, N, Ne, R, S, Unanswered>;

/// The browser a scenario runs: a [`ConformanceBrowser`] with the parser step [`Unparsed`].
type Running<A, D, N, Ne, R, S> = Browser<A, D, N, Ne, R, S, Unanswered, Unparsed>;

type Assemble<A, D, N, Ne, R, S> = Box<dyn Fn(&Setup) -> ConformanceBrowser<A, D, N, Ne, R, S>>;

type Open<A, D, N, Ne, R, S> = RefCell<Option<Scenario<A, D, N, Ne, R, S>>>;

/// What a scenario's system is assembled with beyond fresh ports: the [`Caller`] the target sends
/// every command as, and the entities the scenario establishes before its first command
/// (`setup:` in an authored scenario).
///
/// `surf.agent.Session`, `surf.document.Element`, `surf.document.Option` and
/// `surf.document.Submission` can be established: an authored scenario arranges the controls an
/// agent acts on, and a submission the submission step ends, because a scenario cannot name the
/// identity a binding's command mints. The other entities are reached through commands in every
/// scenario of the suite.
#[derive(Debug, Default)]
pub struct Setup {
    caller: Caller,
    sessions: Vec<SessionSnapshot>,
    elements: Vec<ElementSnapshot>,
    options: Vec<ListOptionSnapshot>,
    submissions: Vec<SubmissionSnapshot>,
}

impl Setup {
    /// The `agent` component's ports: answering with this setup's caller, and holding the
    /// sessions it established, in the order they were.
    pub fn agent_ports(&self) -> AgentPorts {
        let mut ports = AgentPorts::with_caller(self.caller.clone());
        for session in &self.sessions {
            SessionStorage::put(&mut ports, session.clone());
        }
        ports
    }

    /// The `document` component's in-memory ports, holding the elements, options and
    /// submissions this setup established.
    pub fn document_ports(&self) -> DocumentPorts {
        self.document_entities(DocumentPorts::new())
    }

    /// `ports` with the elements, options and submissions this setup established put into them,
    /// each kind in the order it was established: for a test that wraps the `document`
    /// component's ports.
    pub fn document_entities<P>(&self, mut ports: P) -> P
    where
        P: ElementStorage + ListOptionStorage + SubmissionStorage,
    {
        for element in &self.elements {
            ElementStorage::put(&mut ports, element.clone());
        }
        for option in &self.options {
            ListOptionStorage::put(&mut ports, option.clone());
        }
        for submission in &self.submissions {
            SubmissionStorage::put(&mut ports, submission.clone());
        }
        ports
    }
}

/// The `surf` system as a conformance target: a fresh [`ConformanceBrowser`] per scenario, from
/// the function it was built with.
pub struct SurfTarget<A, D, N, Ne, R, S> {
    identity: ImplementationIdentity,
    assemble: Assemble<A, D, N, Ne, R, S>,
    scenario: Open<A, D, N, Ne, R, S>,
}

struct Scenario<A, D, N, Ne, R, S> {
    browser: Running<A, D, N, Ne, R, S>,
    /// Who the next command is sent as, set from each command's caller before it is issued, and
    /// the sessions established so far.
    setup: Setup,
    writes: u64,
}

impl<A, D, N, Ne, R, S> SurfTarget<A, D, N, Ne, R, S> {
    /// A target named `identity` that assembles each scenario's system with `assemble`.
    ///
    /// `assemble` is handed the scenario's [`Setup`]; the `agent` component's ports it builds
    /// come from [`Setup::agent_ports`], so they answer the caller the target sends each command
    /// as and hold the sessions the scenario established, and the `document` component's hold
    /// what [`Setup::document_entities`] puts into them. A scenario that establishes an entity
    /// has its system assembled again, before its first command.
    pub fn new(
        identity: ImplementationIdentity,
        assemble: impl Fn(&Setup) -> ConformanceBrowser<A, D, N, Ne, R, S> + 'static,
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

/// A refusal whose error names one element: a control that is missing, of another kind,
/// disabled or hidden.
fn refused_element(
    outcome: &'static str,
    error: &str,
    element_id: &ElementId,
) -> Result<Answered, TargetError> {
    refused(
        outcome,
        error,
        vec![("element_id", values::uuid(&element_id.0))],
    )
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

/// One of the six components, as the publisher of an event or the receiver of a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Component {
    Agent,
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
        "surf.agent" => Component::Agent,
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
        SystemEvent::ActionRecorded(_)
        | SystemEvent::FillRequested(_)
        | SystemEvent::NavigationRequested(_)
        | SystemEvent::PickRequested(_)
        | SystemEvent::PressRequested(_)
        | SystemEvent::SessionClosed(_)
        | SystemEvent::SessionNavigationChanged(_)
        | SystemEvent::SessionOpened(_) => Component::Agent,
        SystemEvent::CheckednessSet(_)
        | SystemEvent::DocumentParsed(_)
        | SystemEvent::ElementAdded(_)
        | SystemEvent::FormAdded(_)
        | SystemEvent::FormSubmitted(_)
        | SystemEvent::OptionAdded(_)
        | SystemEvent::OptionSelected(_)
        | SystemEvent::ParsingStarted(_)
        | SystemEvent::SubmissionAbandoned(_)
        | SystemEvent::SubmissionStarted(_)
        | SystemEvent::ValueSet(_) => Component::Document,
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
        BindingInvocation::CommitOnResponse(_)
        | BindingInvocation::FailOnFetchFailed(_)
        | BindingInvocation::NavigateOnFormSubmitted(_)
        | BindingInvocation::NavigateOnNavigationRequested(_) => Component::Navigator,
        BindingInvocation::FetchOnNavigationStarted(_) => Component::Net,
        BindingInvocation::CheckOnPickRequested(_)
        | BindingInvocation::ParseOnNavigationCommitted(_)
        | BindingInvocation::SelectOnPickRequested(_)
        | BindingInvocation::SetValueOnFillRequested(_)
        | BindingInvocation::SubmitOnPressRequested(_) => Component::Document,
        BindingInvocation::CloseTabOnSessionClosed(_) => Component::Shell,
        BindingInvocation::NoteOnNavigationCancelled(_)
        | BindingInvocation::NoteOnNavigationCommitted(_)
        | BindingInvocation::NoteOnNavigationFailed(_)
        | BindingInvocation::NoteOnNavigationStarted(_) => Component::Agent,
    }
}

/// The name of the event whose delivery made a binding's invocation, as `SystemEvent::name`
/// gives it.
fn trigger(invocation: &BindingInvocation) -> &'static str {
    match invocation {
        BindingInvocation::CheckOnPickRequested(_)
        | BindingInvocation::SelectOnPickRequested(_) => "surf.agent.PickRequested",
        BindingInvocation::SetValueOnFillRequested(_) => "surf.agent.FillRequested",
        BindingInvocation::SubmitOnPressRequested(_) => "surf.agent.PressRequested",
        BindingInvocation::NavigateOnFormSubmitted(_) => "surf.document.FormSubmitted",
        BindingInvocation::CloseTabOnSessionClosed(_) => "surf.agent.SessionClosed",
        BindingInvocation::CommitOnResponse(_) => "surf.net.ResponseReceived",
        BindingInvocation::FailOnFetchFailed(_) => "surf.net.FetchFailed",
        BindingInvocation::FetchOnNavigationStarted(_)
        | BindingInvocation::NoteOnNavigationStarted(_) => "surf.navigation.NavigationStarted",
        BindingInvocation::NavigateOnNavigationRequested(_) => "surf.agent.NavigationRequested",
        BindingInvocation::NoteOnNavigationCancelled(_) => "surf.navigation.NavigationCancelled",
        BindingInvocation::NoteOnNavigationCommitted(_)
        | BindingInvocation::ParseOnNavigationCommitted(_) => "surf.navigation.NavigationCommitted",
        BindingInvocation::NoteOnNavigationFailed(_) => "surf.navigation.NavigationFailed",
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
/// starts with. A later event of that component is the command's only if no binding invoked it;
/// one a binding invoked it for is not, provided the event that binding reacted to came from
/// another component and reached the log after that run, so the binding's events sit behind it.
/// The `agent` component is invoked back so: `Go` publishes `NavigationRequested`, the
/// navigator's `NavigationStarted` follows, and `note-on-navigation-started` invokes the agent
/// again. The [`Unanswered`] transport issues no command. Where that does not hold, the log
/// cannot be split, and the answer is an error.
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
    let direct = gained
        .iter()
        .take_while(|event| publisher(event) == component)
        .count();
    let mut back = invoked
        .iter()
        .filter(|invocation| invoked_on(invocation) == component)
        .peekable();
    let invoked_back = back.peek().is_some();
    for invocation in back {
        let cause = trigger(invocation);
        let delivered = gained[direct..]
            .iter()
            .any(|event| event.name() == cause && publisher(event) != component);
        if !delivered {
            return Err(format!(
                "a binding invoked {invocation:?} on the component `{command}` was issued on, and \
                 no `{cause}` of another component reached the log after the command's own \
                 events, so the log does not tell the command's own events from that one's"
            ));
        }
    }
    if !invoked_back
        && let Some(later) = gained[direct..]
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

fn navigation_state_named(name: &str) -> Option<NavigationState> {
    [
        NavigationState::Cancelled,
        NavigationState::Committed,
        NavigationState::Failed,
        NavigationState::Started,
    ]
    .into_iter()
    .find(|state| navigation_state(*state) == name)
}

fn session_state(state: SessionState) -> &'static str {
    match state {
        SessionState::Closed => "Closed",
        SessionState::Open => "Open",
    }
}

fn action_command(command: ActionCommand) -> &'static str {
    match command {
        ActionCommand::Open => "open",
        ActionCommand::Go => "go",
        ActionCommand::Follow => "follow",
        ActionCommand::Fill => "fill",
        ActionCommand::Pick => "pick",
        ActionCommand::Press => "press",
        ActionCommand::Close => "close",
    }
}

fn action_command_named(name: &str) -> Option<ActionCommand> {
    [
        ActionCommand::Open,
        ActionCommand::Go,
        ActionCommand::Follow,
        ActionCommand::Fill,
        ActionCommand::Pick,
        ActionCommand::Press,
        ActionCommand::Close,
    ]
    .into_iter()
    .find(|command| action_command(*command) == name)
}

/// An optional value: `null` where it is absent.
fn optional<T>(value: Option<T>, present: impl FnOnce(T) -> Node) -> Node {
    value.map_or(Node::Null, present)
}

/// `fields` without the ones whose value is `null`: an optional input the suite sends as `null`
/// is read as absent, the same as one it leaves out.
fn present(fields: &BTreeMap<String, Node>) -> BTreeMap<String, Node> {
    fields
        .iter()
        .filter(|(_, node)| !matches!(node, Node::Null))
        .map(|(field, node)| (field.clone(), node.clone()))
        .collect()
}

/// A `surf.agent.Session` from an entity setup: its identity, every declared field and its
/// state. A field the entity does not declare, a required field that is missing, a value of
/// another type and a state it does not declare are each an adapter failure naming it. An
/// optional field may be absent or `null`.
fn session_snapshot(
    identity: &Node,
    fields: &BTreeMap<String, Node>,
    state: &impl std::fmt::Display,
) -> Result<SessionSnapshot, TargetError> {
    let reading = "establishing a `surf.agent.Session`";
    let wrong = |what: String| TargetError::unavailable(reading, what);
    let text = |name: &str, node: &Node| match node {
        Node::Text(value) => Ok(value.clone()),
        other => Err(wrong(format!(
            "`{name}` is not text: {}",
            ess_conformance::report::quote(other)
        ))),
    };
    let required = |name: &str| {
        fields
            .get(name)
            .ok_or_else(|| wrong(format!("`{name}` is missing")))
            .and_then(|node| text(name, node))
    };
    let optional = |name: &str| match fields.get(name) {
        None | Some(Node::Null) => Ok(None),
        Some(node) => text(name, node).map(Some),
    };
    const DECLARED: [&str; 9] = [
        "window_id",
        "tab_id",
        "holder",
        "navigation_id",
        "url",
        "navigation_state",
        "status",
        "media_type",
        "failure_reason",
    ];
    if let Some(undeclared) = fields
        .keys()
        .find(|name| !DECLARED.contains(&name.as_str()))
    {
        return Err(wrong(format!("undeclared field `{undeclared}`")));
    }
    let navigation_state = match optional("navigation_state")? {
        None => None,
        Some(name) => Some(
            navigation_state_named(&name)
                .ok_or_else(|| wrong(format!("`navigation_state` is no state: {name}")))?,
        ),
    };
    let status = match fields.get("status") {
        None | Some(Node::Null) => None,
        Some(node @ Node::Number(number)) => Some(Status(number.as_i64().ok_or_else(|| {
            wrong(format!(
                "`status` is not an integer: {}",
                ess_conformance::report::quote(node)
            ))
        })?)),
        Some(other) => {
            return Err(wrong(format!(
                "`status` is not an integer: {}",
                ess_conformance::report::quote(other)
            )));
        }
    };
    let failure_reason = match optional("failure_reason")? {
        None => None,
        Some(name) => Some(
            failure_reason_named(&name)
                .ok_or_else(|| wrong(format!("`failure_reason` is no reason: {name}")))?,
        ),
    };
    let state = match state.to_string().as_str() {
        "Open" => SessionState::Open,
        "Closed" => SessionState::Closed,
        other => return Err(wrong(format!("`{other}` is no state of a session"))),
    };
    Ok(SessionSnapshot {
        state,
        data: SessionData {
            session_id: SessionId(Uuid(text("identity", identity)?)),
            window_id: WindowId(Uuid(required("window_id")?)),
            tab_id: TabId(Uuid(required("tab_id")?)),
            holder: SessionId(Uuid(required("holder")?)),
            navigation_id: optional("navigation_id")?.map(|id| RequestId(Uuid(id))),
            url: optional("url")?.map(Url),
            navigation_state,
            status,
            media_type: optional("media_type")?.map(MediaType),
            failure_reason,
        },
    })
}

/// The `session_id` the caller of `command` carries, read from the runner's caller attributes.
///
/// Only `surf.agent.SessionHolder` declares an attribute, `session_id`. An attribute of another
/// name, or one that is not text, is an adapter failure: the target cannot send the command as
/// that caller, and does not send it as someone else.
fn caller_session_id(
    command: &str,
    caller: Option<&BTreeMap<String, Node>>,
) -> Result<Option<SessionId>, TargetError> {
    let Some(attributes) = caller else {
        return Ok(None);
    };
    let sending = || format!("sending `{command}` as its caller");
    let mut session_id = None;
    for (attribute, value) in attributes {
        match (attribute.as_str(), value) {
            ("session_id", Node::Text(text)) => {
                session_id = Some(SessionId(Uuid(text.clone())));
            }
            _ => {
                return Err(TargetError::unsupported(
                    sending(),
                    format!(
                        "the surf system authenticates no caller attribute `{attribute}` = {}",
                        ess_conformance::report::quote(value)
                    ),
                ));
            }
        }
    }
    Ok(session_id)
}

fn method(method: Method) -> &'static str {
    match method {
        Method::Get => "Get",
        Method::Post => "Post",
    }
}

fn method_named(name: &str) -> Option<Method> {
    [Method::Get, Method::Post]
        .into_iter()
        .find(|candidate| method(*candidate) == name)
}

/// The method, the body and its media type of a request, by declared field name; `null` for an
/// absent body or media type.
fn request_fields(
    request_method: Method,
    body: Option<&Vec<u8>>,
    body_type: Option<&MediaType>,
) -> [(&'static str, Node); 3] {
    [
        ("method", values::text(method(request_method))),
        ("body", optional(body, |body| values::bytes(body))),
        (
            "body_type",
            optional(body_type, |media_type| values::text(&media_type.0)),
        ),
    ]
}

/// The method of a request, its body and the body's media type.
type RequestParts = (Method, Option<Vec<u8>>, Option<MediaType>);

/// The method, the body and its media type of the input of `Navigate` or `Fetch`, whose fields
/// without `null` are `present`: an absent body or media type and one sent as `null` read the
/// same.
fn request_input(
    input: &mut Input<'_>,
    present: &BTreeMap<String, Node>,
) -> Result<RequestParts, TargetError> {
    let request_method = input.variant("method", method_named)?;
    let body = if present.contains_key("body") {
        Some(input.bytes("body")?)
    } else {
        None
    };
    let body_type = if present.contains_key("body_type") {
        Some(MediaType(input.text("body_type")?))
    } else {
        None
    };
    Ok((request_method, body, body_type))
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
        FailureReason::TooManyRedirects => "TooManyRedirects",
        FailureReason::ForbiddenDestination => "ForbiddenDestination",
    }
}

fn failure_reason_named(name: &str) -> Option<FailureReason> {
    [
        FailureReason::Unreachable,
        FailureReason::UnsupportedScheme,
        FailureReason::MalformedResponse,
        FailureReason::TooManyRedirects,
        FailureReason::ForbiddenDestination,
    ]
    .into_iter()
    .find(|reason| failure_reason(*reason) == name)
}

fn document_state(state: DocumentState) -> &'static str {
    match state {
        DocumentState::Parsed => "Parsed",
        DocumentState::Parsing => "Parsing",
    }
}

/// An element's kind as `spec/` spells it, in lower case.
fn element_kind(kind: ElementKind) -> &'static str {
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

fn element_kind_named(name: &str) -> Option<ElementKind> {
    [
        ElementKind::Link,
        ElementKind::Button,
        ElementKind::Text,
        ElementKind::Password,
        ElementKind::Checkbox,
        ElementKind::Radio,
        ElementKind::File,
        ElementKind::Select,
        ElementKind::Textarea,
    ]
    .into_iter()
    .find(|kind| element_kind(*kind) == name)
}

fn element_state(state: ElementState) -> &'static str {
    match state {
        ElementState::Added => "Added",
    }
}

fn form_state(state: FormState) -> &'static str {
    match state {
        FormState::Added => "Added",
    }
}

fn option_state(state: OptionState) -> &'static str {
    match state {
        OptionState::Added => "Added",
    }
}

fn submission_state(state: SubmissionState) -> &'static str {
    match state {
        SubmissionState::Abandoned => "Abandoned",
        SubmissionState::Started => "Started",
        SubmissionState::Submitted => "Submitted",
    }
}

fn submission_state_named(name: &str) -> Option<SubmissionState> {
    [
        SubmissionState::Abandoned,
        SubmissionState::Started,
        SubmissionState::Submitted,
    ]
    .into_iter()
    .find(|state| submission_state(*state) == name)
}

fn end_reason(reason: SubmissionEndReason) -> &'static str {
    match reason {
        SubmissionEndReason::NoFormOwner => "NoFormOwner",
        SubmissionEndReason::NotSubmitButton => "NotSubmitButton",
        SubmissionEndReason::InvalidControls => "InvalidControls",
        SubmissionEndReason::InvalidAction => "InvalidAction",
        SubmissionEndReason::UnsupportedScheme => "UnsupportedScheme",
        SubmissionEndReason::UnsupportedEnctype => "UnsupportedEnctype",
        SubmissionEndReason::DialogMethod => "DialogMethod",
    }
}

fn end_reason_named(name: &str) -> Option<SubmissionEndReason> {
    [
        SubmissionEndReason::NoFormOwner,
        SubmissionEndReason::NotSubmitButton,
        SubmissionEndReason::InvalidControls,
        SubmissionEndReason::InvalidAction,
        SubmissionEndReason::UnsupportedScheme,
        SubmissionEndReason::UnsupportedEnctype,
        SubmissionEndReason::DialogMethod,
    ]
    .into_iter()
    .find(|reason| end_reason(*reason) == name)
}

/// The text field `name` of `input`, whose fields without `null` are `present`; `None` where it
/// is absent or `null`.
fn optional_text(
    input: &mut Input<'_>,
    present: &BTreeMap<String, Node>,
    name: &'static str,
) -> Result<Option<String>, TargetError> {
    if present.contains_key(name) {
        input.text(name).map(Some)
    } else {
        Ok(None)
    }
}

/// The identity of an entity setup, as text.
fn identity_text(entity: &str, identity: &Node) -> Result<String, TargetError> {
    match identity {
        Node::Text(value) => Ok(value.clone()),
        other => Err(TargetError::unavailable(
            format!("establishing a `{entity}`"),
            format!(
                "the identity is not text: {}",
                ess_conformance::report::quote(other)
            ),
        )),
    }
}

/// The state `state` of an entity setup, read by `named`; an adapter failure naming it where the
/// entity declares no such state.
fn established_state<T>(
    entity: &str,
    state: &impl std::fmt::Display,
    named: fn(&str) -> Option<T>,
) -> Result<T, TargetError> {
    let name = state.to_string();
    named(&name).ok_or_else(|| {
        TargetError::unavailable(
            format!("establishing a `{entity}`"),
            format!("`{name}` is no state of the entity"),
        )
    })
}

/// The entities a scenario can establish without a command.
const ESTABLISHED: [&str; 4] = [
    "surf.agent.Session",
    "surf.document.Element",
    "surf.document.Option",
    "surf.document.Submission",
];

/// Appends `snapshot` to `held` unless an entity of its identity, read by `id`, is held already;
/// that identity where one is.
fn hold<T>(held: &mut Vec<T>, snapshot: T, id: fn(&T) -> &Uuid) -> Option<String> {
    if held.iter().any(|other| id(other) == id(&snapshot)) {
        return Some(id(&snapshot).0.clone());
    }
    held.push(snapshot);
    None
}

/// A `surf.document.Element` from an entity setup: its identity, every declared field and its
/// state. A field the entity does not declare, a required field that is missing, a value of
/// another type and a state it does not declare are each an adapter failure naming it. An
/// optional field may be absent or `null`.
fn element_snapshot(
    identity: &Node,
    fields: &BTreeMap<String, Node>,
    state: &impl std::fmt::Display,
) -> Result<ElementSnapshot, TargetError> {
    const ENTITY: &str = "surf.document.Element";
    let present = present(fields);
    let mut input = Input::new(ENTITY, &present);
    let data = ElementData {
        element_id: ElementId(Uuid(identity_text(ENTITY, identity)?)),
        document_id: DocumentId(input.uuid("document_id")?),
        position: input.integer("position")?,
        kind: input.variant("kind", element_kind_named)?,
        label: input.text("label")?,
        target: optional_text(&mut input, &present, "target")?,
        value: optional_text(&mut input, &present, "value")?,
        hidden: input.boolean("hidden")?,
        disabled: input.boolean("disabled")?,
        checked: input.boolean("checked")?,
        required: input.boolean("required")?,
        name: optional_text(&mut input, &present, "name")?,
        form_id: optional_text(&mut input, &present, "form_id")?.map(|id| FormId(Uuid(id))),
    };
    input.finish()?;
    let state = established_state(ENTITY, state, |name| {
        (name == element_state(ElementState::Added)).then_some(ElementState::Added)
    })?;
    Ok(ElementSnapshot { state, data })
}

/// A `surf.document.Option` from an entity setup, read as [`element_snapshot`] reads an element.
fn option_snapshot(
    identity: &Node,
    fields: &BTreeMap<String, Node>,
    state: &impl std::fmt::Display,
) -> Result<ListOptionSnapshot, TargetError> {
    const ENTITY: &str = "surf.document.Option";
    let mut input = Input::new(ENTITY, fields);
    let data = ListOptionData {
        option_id: OptionId(Uuid(identity_text(ENTITY, identity)?)),
        select_id: ElementId(input.uuid("select_id")?),
        position: input.integer("position")?,
        value: input.text("value")?,
        label: input.text("label")?,
        selected: input.boolean("selected")?,
        disabled: input.boolean("disabled")?,
        hidden: input.boolean("hidden")?,
    };
    input.finish()?;
    let state = established_state(ENTITY, state, |name| {
        (name == option_state(OptionState::Added)).then_some(OptionState::Added)
    })?;
    Ok(ListOptionSnapshot { state, data })
}

/// A `surf.document.Submission` from an entity setup, read as [`element_snapshot`] reads an
/// element.
fn submission_snapshot(
    identity: &Node,
    fields: &BTreeMap<String, Node>,
    state: &impl std::fmt::Display,
) -> Result<SubmissionSnapshot, TargetError> {
    const ENTITY: &str = "surf.document.Submission";
    let mut input = Input::new(ENTITY, fields);
    let data = SubmissionData {
        submission_id: SubmissionId(Uuid(identity_text(ENTITY, identity)?)),
        submitter: ElementId(input.uuid("submitter")?),
    };
    input.finish()?;
    let state = established_state(ENTITY, state, submission_state_named)?;
    Ok(SubmissionSnapshot { state, data })
}

fn parse_limit(limit: ParseLimit) -> &'static str {
    match limit {
        ParseLimit::InputBytes => "InputBytes",
        ParseLimit::Depth => "Depth",
        ParseLimit::Nodes => "Nodes",
        ParseLimit::Attributes => "Attributes",
        ParseLimit::StoredBytes => "StoredBytes",
        ParseLimit::Steps => "Steps",
    }
}

fn parse_limit_named(name: &str) -> Option<ParseLimit> {
    [
        ParseLimit::InputBytes,
        ParseLimit::Depth,
        ParseLimit::Nodes,
        ParseLimit::Attributes,
        ParseLimit::StoredBytes,
        ParseLimit::Steps,
    ]
    .into_iter()
    .find(|limit| parse_limit(*limit) == name)
}

/// The payload of an event on the system's log, by declared field name.
fn payload(event: &SystemEvent) -> Vec<(&'static str, Node)> {
    use values::{bytes, integer, text, uuid};
    match event {
        SystemEvent::ActionRecorded(e) => vec![
            ("action_id", uuid(&e.action_id.0)),
            ("session_id", uuid(&e.session_id.0)),
            ("sequence", integer(e.sequence)),
            ("command", text(action_command(e.command))),
            ("url", optional(e.url.as_ref(), |url| text(&url.0))),
            ("handle", optional(e.handle.as_deref(), text)),
            ("label", optional(e.label.as_deref(), text)),
        ],
        SystemEvent::FillRequested(e) => vec![
            ("session_id", uuid(&e.session_id.0)),
            ("element_id", uuid(&e.element_id.0)),
            ("value", text(&e.value)),
        ],
        SystemEvent::NavigationRequested(e) => vec![
            ("session_id", uuid(&e.session_id.0)),
            ("tab_id", uuid(&e.tab_id.0)),
            ("url", text(&e.url.0)),
        ],
        SystemEvent::PickRequested(e) => vec![
            ("session_id", uuid(&e.session_id.0)),
            ("element_id", uuid(&e.element_id.0)),
            (
                "option_id",
                optional(e.option_id.as_ref(), |id| uuid(&id.0)),
            ),
        ],
        SystemEvent::PressRequested(e) => vec![
            ("session_id", uuid(&e.session_id.0)),
            ("element_id", uuid(&e.element_id.0)),
        ],
        SystemEvent::SessionClosed(e) => vec![
            ("session_id", uuid(&e.session_id.0)),
            ("window_id", uuid(&e.window_id.0)),
            ("tab_id", uuid(&e.tab_id.0)),
        ],
        SystemEvent::SessionNavigationChanged(e) => vec![
            ("navigation_id", uuid_of(&e.navigation_id)),
            ("state", text(navigation_state(e.state))),
            ("sessions", integer(e.sessions)),
        ],
        SystemEvent::SessionOpened(e) => vec![
            ("session_id", uuid(&e.session_id.0)),
            ("window_id", uuid(&e.window_id.0)),
            ("tab_id", uuid(&e.tab_id.0)),
        ],
        SystemEvent::DocumentParsed(e) => vec![
            ("document_id", uuid(&e.document_id.0)),
            ("url", text(&e.url.0)),
            ("navigation_id", uuid_of(&e.navigation_id)),
            ("title", text(&e.title)),
            ("encoding", text(&e.encoding.0)),
            (
                "unsupported_encoding",
                optional(e.unsupported_encoding.as_ref(), |name| text(&name.0)),
            ),
            (
                "stopped_by",
                optional(e.stopped_by, |limit| text(parse_limit(limit))),
            ),
        ],
        SystemEvent::ElementAdded(e) => vec![
            ("element_id", uuid(&e.element_id.0)),
            ("document_id", uuid(&e.document_id.0)),
            ("position", integer(e.position)),
        ],
        SystemEvent::ParsingStarted(e) => vec![
            ("document_id", uuid(&e.document_id.0)),
            ("body", bytes(&e.body)),
        ],
        SystemEvent::CheckednessSet(e) => vec![
            ("element_id", uuid(&e.element_id.0)),
            ("checked", values::boolean(e.checked)),
        ],
        SystemEvent::FormAdded(e) => vec![
            ("form_id", uuid(&e.form_id.0)),
            ("document_id", uuid(&e.document_id.0)),
            ("position", integer(e.position)),
        ],
        SystemEvent::FormSubmitted(e) => {
            let mut fields = vec![
                ("submission_id", uuid(&e.submission_id.0)),
                ("tab_id", uuid(&e.tab_id.0)),
                ("url", text(&e.url.0)),
            ];
            fields.extend(request_fields(
                e.method,
                e.body.as_ref(),
                e.body_type.as_ref(),
            ));
            fields
        }
        SystemEvent::OptionAdded(e) => vec![
            ("option_id", uuid(&e.option_id.0)),
            ("select_id", uuid(&e.select_id.0)),
            ("position", integer(e.position)),
        ],
        SystemEvent::OptionSelected(e) => vec![
            ("option_id", uuid(&e.option_id.0)),
            ("select_id", uuid(&e.select_id.0)),
        ],
        SystemEvent::SubmissionAbandoned(e) => vec![
            ("submission_id", uuid(&e.submission_id.0)),
            ("reason", text(end_reason(e.reason))),
        ],
        SystemEvent::SubmissionStarted(e) => vec![
            ("submission_id", uuid(&e.submission_id.0)),
            ("submitter", uuid(&e.submitter.0)),
        ],
        SystemEvent::ValueSet(e) => vec![("element_id", uuid(&e.element_id.0))],
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
            ("reason", text(failure_reason(e.reason))),
        ],
        SystemEvent::NavigationStarted(e) => {
            let mut fields = vec![
                ("navigation_id", uuid_of(&e.navigation_id)),
                ("tab_id", uuid(&e.tab_id.0)),
                ("url", text(&e.url.0)),
            ];
            fields.extend(request_fields(
                e.method,
                e.body.as_ref(),
                e.body_type.as_ref(),
            ));
            fields
        }
        SystemEvent::FetchFailed(e) => vec![
            ("request_id", uuid_of(&e.request_id)),
            ("url", text(&e.url.0)),
            ("reason", text(failure_reason(e.reason))),
        ],
        SystemEvent::RequestStarted(e) => {
            let mut fields = vec![
                ("request_id", uuid_of(&e.request_id)),
                ("url", text(&e.url.0)),
            ];
            fields.extend(request_fields(
                e.method,
                e.body.as_ref(),
                e.body_type.as_ref(),
            ));
            fields
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
                ("url", text(&input.url.0)),
                ("status", integer(input.status.0)),
                ("media_type", text(&input.media_type.0)),
                ("body", bytes(&input.body)),
            ],
        ),
        BindingInvocation::FailOnFetchFailed(input) => (
            "fail-on-fetch-failed",
            "surf.navigation.FailNavigation",
            vec![
                ("navigation_id", uuid_of(&input.navigation_id)),
                ("reason", text(failure_reason(input.reason))),
            ],
        ),
        BindingInvocation::FetchOnNavigationStarted(input) => {
            ("fetch-on-navigation-started", "surf.net.Fetch", {
                let mut fields = vec![
                    ("request_id", uuid_of(&input.request_id)),
                    ("url", text(&input.url.0)),
                ];
                fields.extend(request_fields(
                    input.method,
                    input.body.as_ref(),
                    input.body_type.as_ref(),
                ));
                fields
            })
        }
        BindingInvocation::ParseOnNavigationCommitted(input) => (
            "parse-on-navigation-committed",
            "surf.document.ParseDocument",
            vec![
                ("url", text(&input.url.0)),
                ("body", bytes(&input.body)),
                ("navigation_id", uuid_of(&input.navigation_id)),
                ("tab_id", values::uuid(&input.tab_id.0)),
            ],
        ),
        BindingInvocation::NavigateOnFormSubmitted(input) => {
            ("navigate-on-form-submitted", "surf.navigation.Navigate", {
                let mut fields = vec![
                    ("tab_id", values::uuid(&input.tab_id.0)),
                    ("url", text(&input.url.0)),
                ];
                fields.extend(request_fields(
                    input.method,
                    input.body.as_ref(),
                    input.body_type.as_ref(),
                ));
                fields
            })
        }
        BindingInvocation::SetValueOnFillRequested(input) => (
            "set-value-on-fill-requested",
            "surf.document.SetValue",
            vec![
                ("element_id", values::uuid(&input.element_id.0)),
                ("value", text(&input.value)),
            ],
        ),
        BindingInvocation::CheckOnPickRequested(input) => (
            "check-on-pick-requested",
            "surf.document.Check",
            vec![("element_id", values::uuid(&input.element_id.0))],
        ),
        BindingInvocation::SelectOnPickRequested(input) => (
            "select-on-pick-requested",
            "surf.document.SelectOption",
            vec![("option_id", values::uuid(&input.option_id.0))],
        ),
        BindingInvocation::SubmitOnPressRequested(input) => (
            "submit-on-press-requested",
            "surf.document.StartSubmission",
            vec![("element_id", values::uuid(&input.element_id.0))],
        ),
        BindingInvocation::NavigateOnNavigationRequested(input) => (
            "navigate-on-navigation-requested",
            "surf.navigation.Navigate",
            {
                let mut fields = vec![
                    ("tab_id", values::uuid(&input.tab_id.0)),
                    ("url", text(&input.url.0)),
                ];
                fields.extend(request_fields(
                    input.method,
                    input.body.as_ref(),
                    input.body_type.as_ref(),
                ));
                fields
            },
        ),
        BindingInvocation::CloseTabOnSessionClosed(input) => (
            "close-tab-on-session-closed",
            "surf.shell.CloseTab",
            vec![("tab_id", values::uuid(&input.tab_id.0))],
        ),
        BindingInvocation::NoteOnNavigationStarted(input) => (
            "note-on-navigation-started",
            "surf.agent.NoteNavigationStarted",
            vec![
                ("tab_id", values::uuid(&input.tab_id.0)),
                ("navigation_id", uuid_of(&input.navigation_id)),
                ("url", text(&input.url.0)),
            ],
        ),
        BindingInvocation::NoteOnNavigationCommitted(input) => (
            "note-on-navigation-committed",
            "surf.agent.NoteNavigationCommitted",
            vec![
                ("navigation_id", uuid_of(&input.navigation_id)),
                ("url", text(&input.url.0)),
                ("status", integer(input.status.0)),
                ("media_type", text(&input.media_type.0)),
            ],
        ),
        BindingInvocation::NoteOnNavigationFailed(input) => (
            "note-on-navigation-failed",
            "surf.agent.NoteNavigationFailed",
            vec![
                ("navigation_id", uuid_of(&input.navigation_id)),
                ("reason", text(failure_reason(input.reason))),
            ],
        ),
        BindingInvocation::NoteOnNavigationCancelled(input) => (
            "note-on-navigation-cancelled",
            "surf.agent.NoteNavigationEnded",
            vec![
                ("navigation_id", uuid_of(&input.navigation_id)),
                ("state", text(navigation_state(input.state))),
            ],
        ),
    }
}

impl<A, D, N, Ne, R, S> SurfTarget<A, D, N, Ne, R, S>
where
    A: AgentBehaviors,
    D: DocumentBehaviors,
    N: NavigatorBehaviors,
    Ne: NetBehaviors,
    R: RendererBehaviors,
    S: ShellBehaviors,
{
    /// Runs `operation` on the open scenario.
    fn with<T>(
        &self,
        operation: impl FnOnce(&mut Scenario<A, D, N, Ne, R, S>) -> Result<T, TargetError>,
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
        browser: &mut Running<A, D, N, Ne, R, S>,
        command: &str,
        fields: &BTreeMap<String, Node>,
    ) -> Result<Answered, TargetError> {
        use values::{integer, uuid};
        let mut input = Input::new(command, fields);
        let invoking = format!("invoking `{command}`");
        let unmet = unmet(&invoking);
        Ok(match command {
            "surf.agent.OpenSession" => {
                let session_id = SessionId(input.uuid("session_id")?);
                let window_id = WindowId(input.uuid("window_id")?);
                let tab_id = TabId(input.uuid("tab_id")?);
                input.finish()?;
                match browser
                    .agent_mut()
                    .open_session(OpenSession {
                        session_id,
                        window_id,
                        tab_id,
                    })
                    .map_err(unmet)?
                {
                    OpenSessionOutcome::Opened { session_opened } => {
                        took("opened", SystemEvent::SessionOpened(session_opened))
                    }
                    OpenSessionOutcome::Exists { error } => refused(
                        "exists",
                        "surf.agent.SessionExists",
                        vec![("session_id", uuid(&error.session_id.0))],
                    )?,
                }
            }
            "surf.agent.Go" => {
                let session_id = SessionId(input.uuid("session_id")?);
                let url = Url(input.text("url")?);
                input.finish()?;
                match browser
                    .agent_mut()
                    .go(Go { session_id, url })
                    .map_err(unmet)?
                {
                    GoOutcome::Requested {
                        navigation_requested,
                    } => took(
                        "requested",
                        SystemEvent::NavigationRequested(navigation_requested),
                    ),
                    GoOutcome::NotHolder { error } => refused(
                        "not-holder",
                        "surf.agent.NotSessionHolder",
                        vec![("session_id", uuid(&error.session_id.0))],
                    )?,
                    GoOutcome::NoSuchSession { error } => refused(
                        "no-such-session",
                        "surf.agent.SessionNotFound",
                        vec![("session_id", uuid(&error.session_id.0))],
                    )?,
                    GoOutcome::Closed { error } => refused(
                        "closed",
                        "surf.agent.SessionStateConflict",
                        vec![("state", values::text(session_state(error.state)))],
                    )?,
                }
            }
            "surf.agent.Follow" => {
                let session_id = SessionId(input.uuid("session_id")?);
                let url = Url(input.text("url")?);
                let handle = input.text("handle")?;
                let label = input.text("label")?;
                input.finish()?;
                match browser
                    .agent_mut()
                    .follow(Follow {
                        session_id,
                        url,
                        handle,
                        label,
                    })
                    .map_err(unmet)?
                {
                    FollowOutcome::Requested {
                        navigation_requested,
                    } => took(
                        "requested",
                        SystemEvent::NavigationRequested(navigation_requested),
                    ),
                    FollowOutcome::NotHolder { error } => refused(
                        "not-holder",
                        "surf.agent.NotSessionHolder",
                        vec![("session_id", uuid(&error.session_id.0))],
                    )?,
                    FollowOutcome::NoSuchSession { error } => refused(
                        "no-such-session",
                        "surf.agent.SessionNotFound",
                        vec![("session_id", uuid(&error.session_id.0))],
                    )?,
                    FollowOutcome::Closed { error } => refused(
                        "closed",
                        "surf.agent.SessionStateConflict",
                        vec![("state", values::text(session_state(error.state)))],
                    )?,
                }
            }
            "surf.agent.Fill" => {
                let session_id = SessionId(input.uuid("session_id")?);
                let element_id = ElementId(input.uuid("element_id")?);
                let value = input.text("value")?;
                let handle = input.text("handle")?;
                input.finish()?;
                match browser
                    .agent_mut()
                    .fill(Fill {
                        session_id,
                        element_id,
                        value,
                        handle,
                    })
                    .map_err(unmet)?
                {
                    FillOutcome::Requested { fill_requested } => {
                        took("requested", SystemEvent::FillRequested(fill_requested))
                    }
                    FillOutcome::NotHolder { error } => refused(
                        "not-holder",
                        "surf.agent.NotSessionHolder",
                        vec![("session_id", uuid(&error.session_id.0))],
                    )?,
                    FillOutcome::NoSuchSession { error } => refused(
                        "no-such-session",
                        "surf.agent.SessionNotFound",
                        vec![("session_id", uuid(&error.session_id.0))],
                    )?,
                    FillOutcome::Closed { error } => refused(
                        "closed",
                        "surf.agent.SessionStateConflict",
                        vec![("state", values::text(session_state(error.state)))],
                    )?,
                }
            }
            "surf.agent.Pick" => {
                let present = present(fields);
                let mut input = Input::new(command, &present);
                let session_id = SessionId(input.uuid("session_id")?);
                let element_id = ElementId(input.uuid("element_id")?);
                let option_id =
                    optional_text(&mut input, &present, "option_id")?.map(|id| OptionId(Uuid(id)));
                let handle = input.text("handle")?;
                input.finish()?;
                match browser
                    .agent_mut()
                    .pick(Pick {
                        session_id,
                        element_id,
                        option_id,
                        handle,
                    })
                    .map_err(unmet)?
                {
                    PickOutcome::Requested { pick_requested } => {
                        took("requested", SystemEvent::PickRequested(pick_requested))
                    }
                    PickOutcome::NotHolder { error } => refused(
                        "not-holder",
                        "surf.agent.NotSessionHolder",
                        vec![("session_id", uuid(&error.session_id.0))],
                    )?,
                    PickOutcome::NoSuchSession { error } => refused(
                        "no-such-session",
                        "surf.agent.SessionNotFound",
                        vec![("session_id", uuid(&error.session_id.0))],
                    )?,
                    PickOutcome::Closed { error } => refused(
                        "closed",
                        "surf.agent.SessionStateConflict",
                        vec![("state", values::text(session_state(error.state)))],
                    )?,
                }
            }
            "surf.agent.Press" => {
                let session_id = SessionId(input.uuid("session_id")?);
                let element_id = ElementId(input.uuid("element_id")?);
                let handle = input.text("handle")?;
                input.finish()?;
                match browser
                    .agent_mut()
                    .press(Press {
                        session_id,
                        element_id,
                        handle,
                    })
                    .map_err(unmet)?
                {
                    PressOutcome::Requested { press_requested } => {
                        took("requested", SystemEvent::PressRequested(press_requested))
                    }
                    PressOutcome::NotHolder { error } => refused(
                        "not-holder",
                        "surf.agent.NotSessionHolder",
                        vec![("session_id", uuid(&error.session_id.0))],
                    )?,
                    PressOutcome::NoSuchSession { error } => refused(
                        "no-such-session",
                        "surf.agent.SessionNotFound",
                        vec![("session_id", uuid(&error.session_id.0))],
                    )?,
                    PressOutcome::Closed { error } => refused(
                        "closed",
                        "surf.agent.SessionStateConflict",
                        vec![("state", values::text(session_state(error.state)))],
                    )?,
                }
            }
            "surf.agent.CloseSession" => {
                let session_id = SessionId(input.uuid("session_id")?);
                input.finish()?;
                match browser
                    .agent_mut()
                    .close_session(CloseSession { session_id })
                    .map_err(unmet)?
                {
                    CloseSessionOutcome::Closed { session_closed } => {
                        took("closed", SystemEvent::SessionClosed(session_closed))
                    }
                    CloseSessionOutcome::NotHolder { error } => refused(
                        "not-holder",
                        "surf.agent.NotSessionHolder",
                        vec![("session_id", uuid(&error.session_id.0))],
                    )?,
                    CloseSessionOutcome::NoSuchSession { error } => refused(
                        "no-such-session",
                        "surf.agent.SessionNotFound",
                        vec![("session_id", uuid(&error.session_id.0))],
                    )?,
                    CloseSessionOutcome::WrongState { error } => refused(
                        "wrong-state",
                        "surf.agent.SessionStateConflict",
                        vec![("state", values::text(session_state(error.state)))],
                    )?,
                }
            }
            "surf.agent.NoteNavigationStarted" => {
                let tab_id = TabId(input.uuid("tab_id")?);
                let navigation_id = RequestId(input.uuid("navigation_id")?);
                let url = Url(input.text("url")?);
                input.finish()?;
                match browser
                    .agent_mut()
                    .note_navigation_started(NoteNavigationStarted {
                        tab_id,
                        navigation_id,
                        url,
                    })
                    .map_err(unmet)?
                {
                    NoteNavigationStartedOutcome::Noted {
                        session_navigation_changed,
                    } => took(
                        "noted",
                        SystemEvent::SessionNavigationChanged(session_navigation_changed),
                    ),
                }
            }
            "surf.agent.NoteNavigationCommitted" => {
                let navigation_id = RequestId(input.uuid("navigation_id")?);
                let url = Url(input.text("url")?);
                let status = Status(input.integer("status")?);
                let media_type = MediaType(input.text("media_type")?);
                input.finish()?;
                match browser
                    .agent_mut()
                    .note_navigation_committed(NoteNavigationCommitted {
                        navigation_id,
                        url,
                        status,
                        media_type,
                    })
                    .map_err(unmet)?
                {
                    NoteNavigationCommittedOutcome::Noted {
                        session_navigation_changed,
                    } => took(
                        "noted",
                        SystemEvent::SessionNavigationChanged(session_navigation_changed),
                    ),
                }
            }
            "surf.agent.NoteNavigationFailed" => {
                let navigation_id = RequestId(input.uuid("navigation_id")?);
                let reason = input.variant("reason", failure_reason_named)?;
                input.finish()?;
                match browser
                    .agent_mut()
                    .note_navigation_failed(NoteNavigationFailed {
                        navigation_id,
                        reason,
                    })
                    .map_err(unmet)?
                {
                    NoteNavigationFailedOutcome::Noted {
                        session_navigation_changed,
                    } => took(
                        "noted",
                        SystemEvent::SessionNavigationChanged(session_navigation_changed),
                    ),
                }
            }
            "surf.agent.RecordAction" => {
                let present = present(fields);
                let mut input = Input::new(command, &present);
                let session_id = SessionId(input.uuid("session_id")?);
                let sequence = input.integer("sequence")?;
                let command = input.variant("command", action_command_named)?;
                let url = if present.contains_key("url") {
                    Some(Url(input.text("url")?))
                } else {
                    None
                };
                let handle = if present.contains_key("handle") {
                    Some(input.text("handle")?)
                } else {
                    None
                };
                let label = if present.contains_key("label") {
                    Some(input.text("label")?)
                } else {
                    None
                };
                input.finish()?;
                match browser
                    .agent_mut()
                    .record_action(RecordAction {
                        session_id,
                        sequence,
                        command,
                        url,
                        handle,
                        label,
                    })
                    .map_err(unmet)?
                {
                    RecordActionOutcome::Recorded { action_recorded } => {
                        took("recorded", SystemEvent::ActionRecorded(action_recorded))
                    }
                    RecordActionOutcome::NoSuchSession { error } => refused(
                        "no-such-session",
                        "surf.agent.SessionNotFound",
                        vec![("session_id", uuid(&error.session_id.0))],
                    )?,
                }
            }
            "surf.agent.NoteNavigationEnded" => {
                let navigation_id = RequestId(input.uuid("navigation_id")?);
                let state = input.variant("state", navigation_state_named)?;
                input.finish()?;
                match browser
                    .agent_mut()
                    .note_navigation_ended(NoteNavigationEnded {
                        navigation_id,
                        state,
                    })
                    .map_err(unmet)?
                {
                    NoteNavigationEndedOutcome::Noted {
                        session_navigation_changed,
                    } => took(
                        "noted",
                        SystemEvent::SessionNavigationChanged(session_navigation_changed),
                    ),
                }
            }
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
                let present = present(fields);
                let mut input = Input::new(command, &present);
                let tab_id = TabId(input.uuid("tab_id")?);
                let url = Url(input.text("url")?);
                let (method, body, body_type) = request_input(&mut input, &present)?;
                input.finish()?;
                match browser
                    .navigator_mut()
                    .navigate(Navigate {
                        tab_id,
                        url,
                        method,
                        body,
                        body_type,
                    })
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
                let url = Url(input.text("url")?);
                let status = Status(input.integer("status")?);
                let media_type = MediaType(input.text("media_type")?);
                let body = input.bytes("body")?;
                input.finish()?;
                match browser
                    .navigator_mut()
                    .commit_navigation(CommitNavigation {
                        navigation_id,
                        url,
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
                let reason = input.variant("reason", failure_reason_named)?;
                input.finish()?;
                match browser
                    .navigator_mut()
                    .fail_navigation(FailNavigation {
                        navigation_id,
                        reason,
                    })
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
                let present = present(fields);
                let mut input = Input::new(command, &present);
                let request_id = RequestId(input.uuid("request_id")?);
                let url = Url(input.text("url")?);
                let (method, body, body_type) = request_input(&mut input, &present)?;
                input.finish()?;
                match browser
                    .net_mut()
                    .fetch(Fetch {
                        request_id,
                        url,
                        method,
                        body,
                        body_type,
                    })
                    .map_err(unmet)?
                {
                    FetchOutcome::Started { request_started } => {
                        took("started", SystemEvent::RequestStarted(request_started))
                    }
                }
            }
            "surf.net.DeliverResponse" => {
                let request_id = RequestId(input.uuid("request_id")?);
                let url = Url(input.text("url")?);
                let status = Status(input.integer("status")?);
                let media_type = MediaType(input.text("media_type")?);
                let body = input.bytes("body")?;
                input.finish()?;
                match browser
                    .net_mut()
                    .deliver_response(DeliverResponse {
                        request_id,
                        url,
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
                let navigation_id = RequestId(input.uuid("navigation_id")?);
                let tab_id = TabId(input.uuid("tab_id")?);
                input.finish()?;
                match browser
                    .document_mut()
                    .parse_document(ParseDocument {
                        url,
                        body,
                        navigation_id,
                        tab_id,
                    })
                    .map_err(unmet)?
                {
                    ParseDocumentOutcome::Started { parsing_started } => {
                        took("started", SystemEvent::ParsingStarted(parsing_started))
                    }
                }
            }
            "surf.document.AddElement" => {
                let present = present(fields);
                let mut input = Input::new(command, &present);
                let document_id = DocumentId(input.uuid("document_id")?);
                let position = input.integer("position")?;
                let kind = input.variant("kind", element_kind_named)?;
                let label = input.text("label")?;
                let target = if present.contains_key("target") {
                    Some(input.text("target")?)
                } else {
                    None
                };
                let value = if present.contains_key("value") {
                    Some(input.text("value")?)
                } else {
                    None
                };
                let hidden = input.boolean("hidden")?;
                let disabled = input.boolean("disabled")?;
                let checked = input.boolean("checked")?;
                let required = input.boolean("required")?;
                let name = optional_text(&mut input, &present, "name")?;
                let form_id =
                    optional_text(&mut input, &present, "form_id")?.map(|id| FormId(Uuid(id)));
                input.finish()?;
                match browser
                    .document_mut()
                    .add_element(AddElement {
                        document_id,
                        position,
                        kind,
                        label,
                        target,
                        value,
                        hidden,
                        disabled,
                        checked,
                        required,
                        name,
                        form_id,
                    })
                    .map_err(unmet)?
                {
                    AddElementOutcome::Added { element_added } => {
                        took("added", SystemEvent::ElementAdded(element_added))
                    }
                    AddElementOutcome::NoSuchDocument { error } => refused(
                        "no-such-document",
                        "surf.document.DocumentNotFound",
                        vec![("document_id", uuid(&error.document_id.0))],
                    )?,
                    AddElementOutcome::AlreadyParsed { error } => refused(
                        "already-parsed",
                        "surf.document.DocumentStateConflict",
                        vec![("state", values::text(document_state(error.state)))],
                    )?,
                }
            }
            "surf.document.FinishParsing" => {
                let present = present(fields);
                let mut input = Input::new(command, &present);
                let document_id = DocumentId(input.uuid("document_id")?);
                let title = input.text("title")?;
                let encoding = EncodingName(input.text("encoding")?);
                let unsupported_encoding = if present.contains_key("unsupported_encoding") {
                    Some(EncodingName(input.text("unsupported_encoding")?))
                } else {
                    None
                };
                let stopped_by = if present.contains_key("stopped_by") {
                    Some(input.variant("stopped_by", parse_limit_named)?)
                } else {
                    None
                };
                input.finish()?;
                match browser
                    .document_mut()
                    .finish_parsing(FinishParsing {
                        document_id,
                        title,
                        encoding,
                        unsupported_encoding,
                        stopped_by,
                    })
                    .map_err(unmet)?
                {
                    FinishParsingOutcome::Parsed { document_parsed } => {
                        took("parsed", SystemEvent::DocumentParsed(document_parsed))
                    }
                    FinishParsingOutcome::NoSuchDocument { error } => refused(
                        "no-such-document",
                        "surf.document.DocumentNotFound",
                        vec![("document_id", uuid(&error.document_id.0))],
                    )?,
                    FinishParsingOutcome::WrongState { error } => refused(
                        "wrong-state",
                        "surf.document.DocumentStateConflict",
                        vec![("state", values::text(document_state(error.state)))],
                    )?,
                }
            }
            "surf.document.AddForm" => {
                let present = present(fields);
                let mut input = Input::new(command, &present);
                let document_id = DocumentId(input.uuid("document_id")?);
                let position = input.integer("position")?;
                let action = optional_text(&mut input, &present, "action")?;
                let method = optional_text(&mut input, &present, "method")?;
                let enctype = optional_text(&mut input, &present, "enctype")?;
                input.finish()?;
                match browser
                    .document_mut()
                    .add_form(AddForm {
                        document_id,
                        position,
                        action,
                        method,
                        enctype,
                    })
                    .map_err(unmet)?
                {
                    AddFormOutcome::Added { form_added } => {
                        took("added", SystemEvent::FormAdded(form_added))
                    }
                    AddFormOutcome::NoSuchDocument { error } => refused(
                        "no-such-document",
                        "surf.document.DocumentNotFound",
                        vec![("document_id", uuid(&error.document_id.0))],
                    )?,
                    AddFormOutcome::AlreadyParsed { error } => refused(
                        "already-parsed",
                        "surf.document.DocumentStateConflict",
                        vec![("state", values::text(document_state(error.state)))],
                    )?,
                }
            }
            "surf.document.AddOption" => {
                let select_id = ElementId(input.uuid("select_id")?);
                let position = input.integer("position")?;
                let value = input.text("value")?;
                let label = input.text("label")?;
                let selected = input.boolean("selected")?;
                let disabled = input.boolean("disabled")?;
                let hidden = input.boolean("hidden")?;
                input.finish()?;
                match browser
                    .document_mut()
                    .add_option(AddOption {
                        select_id,
                        position,
                        value,
                        label,
                        selected,
                        disabled,
                        hidden,
                    })
                    .map_err(unmet)?
                {
                    AddOptionOutcome::Added { option_added } => {
                        took("added", SystemEvent::OptionAdded(option_added))
                    }
                    AddOptionOutcome::NoSuchSelect { error } => refused_element(
                        "no-such-select",
                        "surf.document.ElementNotFound",
                        &error.element_id,
                    )?,
                    AddOptionOutcome::NotASelect { error } => refused_element(
                        "not-a-select",
                        "surf.document.ControlKindConflict",
                        &error.element_id,
                    )?,
                }
            }
            "surf.document.SetValue" => {
                let element_id = ElementId(input.uuid("element_id")?);
                let value = input.text("value")?;
                input.finish()?;
                match browser
                    .document_mut()
                    .set_value(SetValue { element_id, value })
                    .map_err(unmet)?
                {
                    SetValueOutcome::Set { value_set } => {
                        took("set", SystemEvent::ValueSet(value_set))
                    }
                    SetValueOutcome::NoSuchElement { error } => refused_element(
                        "no-such-element",
                        "surf.document.ElementNotFound",
                        &error.element_id,
                    )?,
                    SetValueOutcome::NotTextEntry { error } => refused_element(
                        "not-text-entry",
                        "surf.document.ControlKindConflict",
                        &error.element_id,
                    )?,
                    SetValueOutcome::Disabled { error } => refused_element(
                        "disabled",
                        "surf.document.ControlDisabled",
                        &error.element_id,
                    )?,
                    SetValueOutcome::Hidden { error } => {
                        refused_element("hidden", "surf.document.ControlHidden", &error.element_id)?
                    }
                }
            }
            "surf.document.Check" => {
                let element_id = ElementId(input.uuid("element_id")?);
                input.finish()?;
                match browser
                    .document_mut()
                    .check(Check { element_id })
                    .map_err(unmet)?
                {
                    CheckOutcome::RadioChecked { checkedness_set } => took(
                        "radio-checked",
                        SystemEvent::CheckednessSet(checkedness_set),
                    ),
                    CheckOutcome::Unchecked { checkedness_set } => {
                        took("unchecked", SystemEvent::CheckednessSet(checkedness_set))
                    }
                    CheckOutcome::Checked { checkedness_set } => {
                        took("checked", SystemEvent::CheckednessSet(checkedness_set))
                    }
                    CheckOutcome::NoSuchElement { error } => refused_element(
                        "no-such-element",
                        "surf.document.ElementNotFound",
                        &error.element_id,
                    )?,
                    CheckOutcome::NotCheckable { error } => refused_element(
                        "not-checkable",
                        "surf.document.ControlKindConflict",
                        &error.element_id,
                    )?,
                    CheckOutcome::Disabled { error } => refused_element(
                        "disabled",
                        "surf.document.ControlDisabled",
                        &error.element_id,
                    )?,
                    CheckOutcome::Hidden { error } => {
                        refused_element("hidden", "surf.document.ControlHidden", &error.element_id)?
                    }
                }
            }
            "surf.document.SelectOption" => {
                let option_id = OptionId(input.uuid("option_id")?);
                input.finish()?;
                match browser
                    .document_mut()
                    .select_option(SelectOption { option_id })
                    .map_err(unmet)?
                {
                    SelectOptionOutcome::Selected { option_selected } => {
                        took("selected", SystemEvent::OptionSelected(option_selected))
                    }
                    SelectOptionOutcome::NoSuchOption { error } => refused(
                        "no-such-option",
                        "surf.document.OptionNotFound",
                        vec![("option_id", uuid(&error.option_id.0))],
                    )?,
                    SelectOptionOutcome::Disabled { error } => refused_element(
                        "disabled",
                        "surf.document.ControlDisabled",
                        &error.element_id,
                    )?,
                    SelectOptionOutcome::Hidden { error } => {
                        refused_element("hidden", "surf.document.ControlHidden", &error.element_id)?
                    }
                }
            }
            "surf.document.StartSubmission" => {
                let element_id = ElementId(input.uuid("element_id")?);
                input.finish()?;
                match browser
                    .document_mut()
                    .start_submission(StartSubmission { element_id })
                    .map_err(unmet)?
                {
                    StartSubmissionOutcome::Started { submission_started } => took(
                        "started",
                        SystemEvent::SubmissionStarted(submission_started),
                    ),
                    StartSubmissionOutcome::NoSuchElement { error } => refused_element(
                        "no-such-element",
                        "surf.document.ElementNotFound",
                        &error.element_id,
                    )?,
                    StartSubmissionOutcome::NotAButton { error } => refused_element(
                        "not-a-button",
                        "surf.document.ControlKindConflict",
                        &error.element_id,
                    )?,
                    StartSubmissionOutcome::Disabled { error } => refused_element(
                        "disabled",
                        "surf.document.ControlDisabled",
                        &error.element_id,
                    )?,
                    StartSubmissionOutcome::Hidden { error } => {
                        refused_element("hidden", "surf.document.ControlHidden", &error.element_id)?
                    }
                }
            }
            "surf.document.SubmitForm" => {
                let present = present(fields);
                let mut input = Input::new(command, &present);
                let submission_id = SubmissionId(input.uuid("submission_id")?);
                let tab_id = TabId(input.uuid("tab_id")?);
                let url = Url(input.text("url")?);
                let (method, body, body_type) = request_input(&mut input, &present)?;
                input.finish()?;
                match browser
                    .document_mut()
                    .submit_form(SubmitForm {
                        submission_id,
                        tab_id,
                        url,
                        method,
                        body,
                        body_type,
                    })
                    .map_err(unmet)?
                {
                    SubmitFormOutcome::Submitted { form_submitted } => {
                        took("submitted", SystemEvent::FormSubmitted(form_submitted))
                    }
                    SubmitFormOutcome::NoSuchSubmission { error } => refused(
                        "no-such-submission",
                        "surf.document.SubmissionNotFound",
                        vec![("submission_id", uuid(&error.submission_id.0))],
                    )?,
                    SubmitFormOutcome::WrongState { error } => refused(
                        "wrong-state",
                        "surf.document.SubmissionStateConflict",
                        vec![("state", values::text(submission_state(error.state)))],
                    )?,
                }
            }
            "surf.document.AbandonSubmission" => {
                let submission_id = SubmissionId(input.uuid("submission_id")?);
                let reason = input.variant("reason", end_reason_named)?;
                input.finish()?;
                match browser
                    .document_mut()
                    .abandon_submission(AbandonSubmission {
                        submission_id,
                        reason,
                    })
                    .map_err(unmet)?
                {
                    AbandonSubmissionOutcome::Abandoned {
                        submission_abandoned,
                    } => took(
                        "abandoned",
                        SystemEvent::SubmissionAbandoned(submission_abandoned),
                    ),
                    AbandonSubmissionOutcome::NoSuchSubmission { error } => refused(
                        "no-such-submission",
                        "surf.document.SubmissionNotFound",
                        vec![("submission_id", uuid(&error.submission_id.0))],
                    )?,
                    AbandonSubmissionOutcome::WrongState { error } => refused(
                        "wrong-state",
                        "surf.document.SubmissionStateConflict",
                        vec![("state", values::text(submission_state(error.state)))],
                    )?,
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

    /// The rows of one view, by projected field name, for `params`, its parameters by declared
    /// name. A parameter the view does not declare is an adapter failure naming it.
    fn rows(
        browser: &Running<A, D, N, Ne, R, S>,
        view: &str,
        params: &BTreeMap<String, Node>,
    ) -> Result<Vec<ViewRow>, TargetError> {
        use values::{integer, text, uuid};
        let reading = format!("reading `{view}`");
        let unmet = unmet(&reading);
        let system = browser.system();
        let mut params = Input::new(view, params);
        if view == "surf.agent.SessionState" {
            let session_id = SessionId(params.uuid("session_id")?);
            params.finish()?;
            let rows = system
                .agent
                .session_state_view(session_id)
                .map_err(unmet)?
                .into_iter()
                .map(|row| {
                    fields(vec![
                        ("session_id", uuid(&row.session_id.0)),
                        ("state", text(session_state(row.state))),
                        ("url", optional(row.url.as_ref(), |url| text(&url.0))),
                        (
                            "navigation_state",
                            optional(row.navigation_state, |state| text(navigation_state(state))),
                        ),
                        ("status", optional(row.status, |status| integer(status.0))),
                        (
                            "media_type",
                            optional(row.media_type.as_ref(), |media_type| text(&media_type.0)),
                        ),
                        (
                            "failure_reason",
                            optional(row.failure_reason, |reason| text(failure_reason(reason))),
                        ),
                    ])
                })
                .collect();
            return Ok(rows);
        }
        if view == "surf.agent.SessionLog" {
            let session_id = SessionId(params.uuid("session_id")?);
            params.finish()?;
            let rows = system
                .agent
                .session_log(session_id)
                .map_err(unmet)?
                .into_iter()
                .map(|row| {
                    fields(vec![
                        ("action_id", uuid(&row.action_id.0)),
                        ("session_id", uuid(&row.session_id.0)),
                        ("sequence", integer(row.sequence)),
                        ("command", text(action_command(row.command))),
                        ("url", optional(row.url.as_ref(), |url| text(&url.0))),
                        ("handle", optional(row.handle.as_deref(), text)),
                        ("label", optional(row.label.as_deref(), text)),
                    ])
                })
                .collect();
            return Ok(rows);
        }
        if view == "surf.document.Elements" {
            let document_id = DocumentId(params.uuid("document_id")?);
            params.finish()?;
            let rows = system
                .document
                .elements(document_id)
                .map_err(unmet)?
                .into_iter()
                .map(|row| {
                    fields(vec![
                        ("element_id", uuid(&row.element_id.0)),
                        ("document_id", uuid(&row.document_id.0)),
                        ("position", integer(row.position)),
                        ("kind", text(element_kind(row.kind))),
                        ("label", text(&row.label)),
                        (
                            "target",
                            optional(row.target.as_ref(), |target| text(target)),
                        ),
                        ("value", optional(row.value.as_ref(), |value| text(value))),
                        ("hidden", values::boolean(row.hidden)),
                        ("disabled", values::boolean(row.disabled)),
                        ("checked", values::boolean(row.checked)),
                        ("required", values::boolean(row.required)),
                        ("name", optional(row.name.as_deref(), text)),
                        ("form_id", optional(row.form_id.as_ref(), |id| uuid(&id.0))),
                    ])
                })
                .collect();
            return Ok(rows);
        }
        params.finish()?;
        let rows: Vec<Vec<(&'static str, Node)>> = match view {
            "surf.agent.Sessions" => system
                .agent
                .sessions()
                .map_err(unmet)?
                .into_iter()
                .map(|row| {
                    vec![
                        ("session_id", uuid(&row.session_id.0)),
                        ("state", text(session_state(row.state))),
                        ("window_id", uuid(&row.window_id.0)),
                        ("tab_id", uuid(&row.tab_id.0)),
                        (
                            "navigation_id",
                            optional(row.navigation_id.as_ref(), uuid_of),
                        ),
                        ("url", optional(row.url.as_ref(), |url| text(&url.0))),
                        (
                            "navigation_state",
                            optional(row.navigation_state, |state| text(navigation_state(state))),
                        ),
                        ("status", optional(row.status, |status| integer(status.0))),
                        (
                            "media_type",
                            optional(row.media_type.as_ref(), |media_type| text(&media_type.0)),
                        ),
                        (
                            "failure_reason",
                            optional(row.failure_reason, |reason| text(failure_reason(reason))),
                        ),
                    ]
                })
                .collect(),
            "surf.document.Documents" => system
                .document
                .documents()
                .map_err(unmet)?
                .into_iter()
                .map(|row| {
                    vec![
                        ("document_id", uuid(&row.document_id.0)),
                        ("url", text(&row.url.0)),
                        ("navigation_id", uuid_of(&row.navigation_id)),
                        ("tab_id", uuid(&row.tab_id.0)),
                        ("state", text(document_state(row.state))),
                        ("title", optional(row.title.as_ref(), |title| text(title))),
                        (
                            "encoding",
                            optional(row.encoding.as_ref(), |name| text(&name.0)),
                        ),
                        (
                            "unsupported_encoding",
                            optional(row.unsupported_encoding.as_ref(), |name| text(&name.0)),
                        ),
                        (
                            "stopped_by",
                            optional(row.stopped_by, |limit| text(parse_limit(limit))),
                        ),
                    ]
                })
                .collect(),
            "surf.document.Controls" => system
                .document
                .controls()
                .map_err(unmet)?
                .into_iter()
                .map(|row| {
                    vec![
                        ("element_id", uuid(&row.element_id.0)),
                        ("document_id", uuid(&row.document_id.0)),
                        ("state", text(element_state(row.state))),
                        ("kind", text(element_kind(row.kind))),
                        ("value", optional(row.value.as_deref(), text)),
                        ("hidden", values::boolean(row.hidden)),
                        ("disabled", values::boolean(row.disabled)),
                        ("checked", values::boolean(row.checked)),
                        ("name", optional(row.name.as_deref(), text)),
                        ("form_id", optional(row.form_id.as_ref(), |id| uuid(&id.0))),
                    ]
                })
                .collect(),
            "surf.document.Forms" => system
                .document
                .forms()
                .map_err(unmet)?
                .into_iter()
                .map(|row| {
                    vec![
                        ("form_id", uuid(&row.form_id.0)),
                        ("document_id", uuid(&row.document_id.0)),
                        ("state", text(form_state(row.state))),
                        ("position", integer(row.position)),
                        ("action", optional(row.action.as_deref(), text)),
                        ("method", optional(row.method.as_deref(), text)),
                        ("enctype", optional(row.enctype.as_deref(), text)),
                    ]
                })
                .collect(),
            "surf.document.Options" => system
                .document
                .options()
                .map_err(unmet)?
                .into_iter()
                .map(|row| {
                    vec![
                        ("option_id", uuid(&row.option_id.0)),
                        ("select_id", uuid(&row.select_id.0)),
                        ("state", text(option_state(row.state))),
                        ("position", integer(row.position)),
                        ("value", text(&row.value)),
                        ("label", text(&row.label)),
                        ("selected", values::boolean(row.selected)),
                        ("disabled", values::boolean(row.disabled)),
                        ("hidden", values::boolean(row.hidden)),
                    ]
                })
                .collect(),
            "surf.document.Submissions" => system
                .document
                .submissions()
                .map_err(unmet)?
                .into_iter()
                .map(|row| {
                    vec![
                        ("submission_id", uuid(&row.submission_id.0)),
                        ("submitter", uuid(&row.submitter.0)),
                        ("state", text(submission_state(row.state))),
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
                        ("method", text(method(row.method))),
                        (
                            "body_type",
                            optional(row.body_type.as_ref(), |media_type| text(&media_type.0)),
                        ),
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

impl<A, D, N, Ne, R, S> ConformanceTarget for SurfTarget<A, D, N, Ne, R, S>
where
    A: AgentBehaviors,
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
        let setup = Setup::default();
        *self.scenario.borrow_mut() = Some(Scenario {
            browser: (self.assemble)(&setup).with_parser(Unparsed),
            setup,
            writes: 0,
        });
        Ok(())
    }

    fn establish_entity(&self, request: EntitySetupRequest) -> Result<(), TargetError> {
        self.with(|scenario| {
            let entity = request.entity.to_string();
            let establishing = format!("establishing a `{entity}`");
            if !ESTABLISHED.contains(&entity.as_str()) {
                return Err(TargetError::unsupported(
                    establishing,
                    format!(
                        "the surf system establishes only {} without a command",
                        ESTABLISHED.join(", ")
                    ),
                ));
            }
            if scenario.writes > 0 || !scenario.browser.system().published().is_empty() {
                return Err(TargetError::unsupported(
                    establishing,
                    "an entity is established before the scenario's first command, and one ran",
                ));
            }
            let (identity, fields, state) = (&request.identity, &request.fields, &request.state);
            let setup = &mut scenario.setup;
            let duplicate = match entity.as_str() {
                "surf.agent.Session" => hold(
                    &mut setup.sessions,
                    session_snapshot(identity, fields, state)?,
                    |held| &held.data.session_id.0,
                ),
                "surf.document.Element" => hold(
                    &mut setup.elements,
                    element_snapshot(identity, fields, state)?,
                    |held| &held.data.element_id.0,
                ),
                "surf.document.Option" => hold(
                    &mut setup.options,
                    option_snapshot(identity, fields, state)?,
                    |held| &held.data.option_id.0,
                ),
                _ => hold(
                    &mut setup.submissions,
                    submission_snapshot(identity, fields, state)?,
                    |held| &held.data.submission_id.0,
                ),
            };
            if let Some(identity) = duplicate {
                return Err(TargetError::unavailable(
                    establishing,
                    format!("a `{entity}` `{identity}` is already established"),
                ));
            }
            scenario.browser = (self.assemble)(&scenario.setup).with_parser(Unparsed);
            Ok(())
        })
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
            let caller = &scenario.setup.caller;
            caller.set(caller_session_id(&command, request.caller.as_ref())?);
            let answered = Self::issue(&mut scenario.browser, &command, &request.input);
            caller.set(None);
            let answered = answered?;
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
                &request.params,
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
    use std::collections::BTreeMap;

    use super::reported_events;
    use ess_primitives::node::Node;
    use surf::{Browser, SystemEvent};
    use surf_ports::{
        AgentPorts, ComponentPorts, DocumentPorts, NavigatorPorts, NetPorts, RendererPorts,
        ShellPorts,
    };
    use surf_system::BindingInvocation;
    use surf_types::agent::{
        NavigationRequested, NoteNavigationStarted, SessionId, SessionNavigationChanged,
    };
    use surf_types::behaviour::Generated;
    use surf_types::document::{DocumentId, ParsingStarted};
    use surf_types::navigation::{
        CommitNavigation, Navigate, NavigationCancelled, NavigationCommitted, NavigationStarted,
        NavigationState,
    };
    use surf_types::net::{
        Fetch, MediaType, Method, RequestId, RequestStarted, ResponseReceived, Status,
    };
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
            method: Method::Get,
            body: None,
            body_type: None,
        })
    }

    fn request_started() -> SystemEvent {
        SystemEvent::RequestStarted(RequestStarted {
            request_id: navigation_id(),
            url: Url(ADDRESS.to_owned()),
            method: Method::Get,
            body: None,
            body_type: None,
        })
    }

    fn fetch() -> BindingInvocation {
        BindingInvocation::FetchOnNavigationStarted(Fetch {
            request_id: navigation_id(),
            url: Url(ADDRESS.to_owned()),
            method: Method::Get,
            body: None,
            body_type: None,
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
            url: Url(ADDRESS.to_owned()),
            status: Status(200),
            media_type: MediaType("text/html".to_owned()),
            body: Vec::new(),
        })
    }

    fn parsing_started() -> SystemEvent {
        SystemEvent::ParsingStarted(ParsingStarted {
            document_id: DocumentId(Uuid("3f1d5b7e-0000-4000-8000-000000000030".to_owned())),
            body: Vec::new(),
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
                &[received(), committed(), parsing_started()],
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

    const SESSION: &str = "3f1d5b7e-0000-4000-8000-000000000040";

    fn requested() -> SystemEvent {
        SystemEvent::NavigationRequested(NavigationRequested {
            session_id: SessionId(Uuid(SESSION.to_owned())),
            tab_id: TabId(Uuid(TAB.to_owned())),
            url: Url(ADDRESS.to_owned()),
        })
    }

    fn noted_started() -> SystemEvent {
        SystemEvent::SessionNavigationChanged(SessionNavigationChanged {
            navigation_id: navigation_id(),
            state: NavigationState::Started,
            sessions: 1,
        })
    }

    fn navigate() -> BindingInvocation {
        BindingInvocation::NavigateOnNavigationRequested(Navigate {
            tab_id: TabId(Uuid(TAB.to_owned())),
            url: Url(ADDRESS.to_owned()),
            method: Method::Get,
            body: None,
            body_type: None,
        })
    }

    fn note_started() -> BindingInvocation {
        BindingInvocation::NoteOnNavigationStarted(NoteNavigationStarted {
            tab_id: TabId(Uuid(TAB.to_owned())),
            navigation_id: navigation_id(),
            url: Url(ADDRESS.to_owned()),
        })
    }

    /// Story agent-session-spec: `Go` publishes `NavigationRequested`, and the navigator's
    /// `NavigationStarted` invokes the `agent` component again. That event is the binding's, not
    /// the command's: it sits behind the other component's event it reacted to.
    #[test]
    fn an_event_a_binding_invoked_back_after_another_components_event_is_not_the_commands() {
        assert_eq!(
            reported_events(
                "surf.agent.Go",
                &[requested()],
                &[requested(), started(), noted_started(), request_started()],
                &[navigate(), fetch(), note_started()],
            ),
            Ok(vec![requested()])
        );
    }

    /// The same invocation back into the component, with the event it reacts to missing from the
    /// log, leaves the log unsplit.
    #[test]
    fn a_binding_invoked_back_without_its_cause_on_the_log_is_an_error() {
        let answer = reported_events(
            "surf.agent.Go",
            &[requested()],
            &[requested(), noted_started()],
            &[navigate(), note_started()],
        );
        assert!(answer.is_err(), "{answer:?}");
    }

    type MemoryTarget = super::SurfTarget<
        Generated<AgentPorts>,
        Generated<DocumentPorts>,
        Generated<NavigatorPorts>,
        Generated<NetPorts>,
        Generated<RendererPorts>,
        Generated<ShellPorts>,
    >;

    /// The `NavigationStarted` and the `Fetch` invocation of `Navigate` with `fields`, over a
    /// fresh system.
    fn navigated(fields: &[(&str, Node)]) -> (NavigationStarted, Fetch) {
        let mut browser = Browser::from_ports(ComponentPorts::new(), super::Unanswered)
            .with_parser(super::Unparsed);
        let fields: BTreeMap<String, Node> = fields
            .iter()
            .map(|(name, node)| ((*name).to_owned(), node.clone()))
            .collect();
        let answered = MemoryTarget::issue(&mut browser, "surf.navigation.Navigate", &fields)
            .unwrap_or_else(|error| panic!("{error:?}"));
        browser.run().expect("run");
        let [SystemEvent::NavigationStarted(started)] = answered.events.as_slice() else {
            panic!("{:?}", answered.events);
        };
        let fetches: Vec<Fetch> = browser
            .system()
            .invocations()
            .iter()
            .filter_map(|invocation| match invocation {
                BindingInvocation::FetchOnNavigationStarted(fetch) => Some(fetch.clone()),
                _ => None,
            })
            .collect();
        let [fetch] = fetches.as_slice() else {
            panic!("{fetches:?}");
        };
        (started.clone(), fetch.clone())
    }

    /// Story agent-request-body-spec: an optional input the suite sends as `null` reads as one it
    /// leaves out, for the body and the media type of `Navigate`; a present one is passed on.
    #[test]
    fn a_navigate_body_sent_as_null_reads_as_one_left_out() {
        let required = [
            ("tab_id", Node::Text(TAB.to_owned())),
            ("url", Node::Text(ADDRESS.to_owned())),
        ];
        let absent = navigated(&[
            required[0].clone(),
            required[1].clone(),
            ("method", Node::Text("Get".to_owned())),
        ]);
        let null = navigated(&[
            required[0].clone(),
            required[1].clone(),
            ("method", Node::Text("Get".to_owned())),
            ("body", Node::Null),
            ("body_type", Node::Null),
        ]);
        let present = navigated(&[
            required[0].clone(),
            required[1].clone(),
            ("method", Node::Text("Post".to_owned())),
            ("body", Node::Text("YT0x".to_owned())),
            (
                "body_type",
                Node::Text("application/x-www-form-urlencoded".to_owned()),
            ),
        ]);
        for (started, fetch) in [&absent, &null] {
            assert_eq!(
                (started.method, &started.body, &started.body_type),
                (Method::Get, &None, &None)
            );
            assert_eq!(
                (fetch.method, &fetch.body, &fetch.body_type),
                (Method::Get, &None, &None)
            );
        }
        let (started, fetch) = &present;
        let form = Some(MediaType("application/x-www-form-urlencoded".to_owned()));
        assert_eq!(
            (started.method, &started.body, &started.body_type),
            (Method::Post, &Some(b"a=1".to_vec()), &form)
        );
        assert_eq!(
            (fetch.method, &fetch.body, &fetch.body_type),
            (Method::Post, &Some(b"a=1".to_vec()), &form)
        );
    }

    /// A second event of the commanding component behind another one's, with no binding back
    /// into it, is still an error.
    #[test]
    fn a_later_event_of_the_commanding_component_with_no_binding_back_is_an_error() {
        let answer = reported_events(
            "surf.agent.Go",
            &[requested()],
            &[requested(), started(), noted_started()],
            &[navigate()],
        );
        assert!(answer.is_err(), "{answer:?}");
    }
}
