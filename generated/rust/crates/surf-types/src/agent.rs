// generated from surf v1
// contract digest 8a6a9f1bd1962c0655a4273e57621bd9c11144463771cf0d9887841440044e63
// do not edit: regenerate with `ess synthesize`

//! Agent — `surf.agent`.
//!
//! A browser session an AI agent drives from a command line: opened once, acted on with a token, read back as text, closed. Every action of the agent is mapped onto the commands the other components already accept; this domain holds only what the agent's session adds to them. The `agent` component holds it.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// The states of `surf.agent.Action`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Action<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionState {
    /// `Recorded`.
    Recorded,
}

/// ActionCommand — `surf.agent.ActionCommand`: one of a closed set of names.
///
/// Which command of the agent an action was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionCommand {
    /// `open`.
    Open,
    /// `go`.
    Go,
    /// `follow`.
    Follow,
    /// `fill`.
    Fill,
    /// `pick`.
    Pick,
    /// `press`.
    Press,
    /// `back`.
    Back,
    /// `forward`.
    Forward,
    /// `close`.
    Close,
}

/// ActionId — `surf.agent.ActionId`: a distinct wrapper around `Uuid`.
///
/// The identity of one recorded action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionId(pub crate::primitives::Uuid);

/// FillCall — `surf.agent.FillCall`.
///
/// The handle of the control to fill in a session, its value, and the token that proves the caller holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FillCall {
    /// `session_id` — `String`.
    pub session_id: String,
    /// `token` — `surf.agent.SessionToken`.
    pub token: SessionToken,
    /// `handle` — `String`.
    pub handle: String,
    /// `value` — `String`.
    pub value: String,
}

/// FollowCall — `surf.agent.FollowCall`.
///
/// The handle of the link to follow in a session, and the token that proves the caller holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FollowCall {
    /// `session_id` — `String`.
    pub session_id: String,
    /// `token` — `surf.agent.SessionToken`.
    pub token: SessionToken,
    /// `handle` — `String`.
    pub handle: String,
}

/// GoCall — `surf.agent.GoCall`.
///
/// The address to load into a session, the token that proves the caller holds it, and the form to print the page in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoCall {
    /// `session_id` — `String`.
    pub session_id: String,
    /// `token` — `surf.agent.SessionToken`.
    pub token: SessionToken,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `form` — `Optional<surf.agent.PageForm>`.
    pub form: Option<PageForm>,
    /// `from_line` — `Optional<Integer>`.
    pub from_line: Option<i64>,
}

/// LogCall — `surf.agent.LogCall`.
///
/// Which session `log` reads, how many of its last actions to list if not all, and the token that proves the caller holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogCall {
    /// `session_id` — `String`.
    pub session_id: String,
    /// `token` — `surf.agent.SessionToken`.
    pub token: SessionToken,
    /// `count` — `Optional<Integer>`.
    pub count: Option<i64>,
}

/// LookCall — `surf.agent.LookCall`.
///
/// Which session `look` reads, the token that proves the caller holds it, and the form to print the page in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookCall {
    /// `session_id` — `String`.
    pub session_id: String,
    /// `token` — `surf.agent.SessionToken`.
    pub token: SessionToken,
    /// `form` — `Optional<surf.agent.PageForm>`.
    pub form: Option<PageForm>,
    /// `from_line` — `Optional<Integer>`.
    pub from_line: Option<i64>,
}

/// OpenCall — `surf.agent.OpenCall`.
///
/// The input of `open`, the address to load first if one is given, the form to print it in, a file of extra trust anchors, and whether the session runs its pages' scripts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCall {
    /// `url` — `Optional<surf.url.Url>`.
    pub url: Option<crate::url::Url>,
    /// `form` — `Optional<surf.agent.PageForm>`.
    pub form: Option<PageForm>,
    /// `from_line` — `Optional<Integer>`.
    pub from_line: Option<i64>,
    /// `ca_file` — `Optional<String>`.
    pub ca_file: Option<String>,
    /// `scripts` — `Optional<Boolean>`.
    pub scripts: Option<bool>,
}

/// OpenedSession — `surf.agent.OpenedSession`.
///
/// The answer of `open`; the only answer that carries the token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedSession {
    /// `session_id` — `String`.
    pub session_id: String,
    /// `token` — `surf.agent.SessionToken`.
    pub token: SessionToken,
}

/// PageForm — `surf.agent.PageForm`: one of a closed set of names.
///
/// The form in which an answer prints the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageForm {
    /// `tree`.
    Tree,
    /// `markdown`.
    Markdown,
}

/// PickCall — `surf.agent.PickCall`.
///
/// The handle of the choice to change in a session, the option to pick, and the token that proves the caller holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickCall {
    /// `session_id` — `String`.
    pub session_id: String,
    /// `token` — `surf.agent.SessionToken`.
    pub token: SessionToken,
    /// `handle` — `String`.
    pub handle: String,
    /// `option` — `String`.
    pub option: String,
}

/// PressCall — `surf.agent.PressCall`.
///
/// The handle of the button to press in a session, and the token that proves the caller holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PressCall {
    /// `session_id` — `String`.
    pub session_id: String,
    /// `token` — `surf.agent.SessionToken`.
    pub token: SessionToken,
    /// `handle` — `String`.
    pub handle: String,
}

/// The states of `surf.agent.Session`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Session<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// `Closed`.
    Closed,
    /// `Open`.
    Open,
}

/// SessionCall — `surf.agent.SessionCall`.
///
/// Which session a call acts on, and the token that proves the caller holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCall {
    /// `session_id` — `String`.
    pub session_id: String,
    /// `token` — `surf.agent.SessionToken`.
    pub token: SessionToken,
}

/// SessionId — `surf.agent.SessionId`: a distinct wrapper around `Uuid`.
///
/// The identity of a session, drawn by `open`; not secret, it appears in listings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(pub crate::primitives::Uuid);

/// SessionReadout — `surf.agent.SessionReadout`.
///
/// A row of `SessionState`, as every call but `open` answers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionReadout {
    /// `session_id` — `String`.
    pub session_id: String,
    /// `state` — `surf.agent.Session.State`.
    pub state: SessionState,
    /// `url` — `Optional<surf.url.Url>`.
    pub url: Option<crate::url::Url>,
    /// `navigation_state` — `Optional<surf.navigation.Navigation.State>`.
    pub navigation_state: Option<crate::navigation::NavigationState>,
    /// `status` — `Optional<surf.net.Status>`.
    pub status: Option<crate::net::Status>,
    /// `media_type` — `Optional<surf.net.MediaType>`.
    pub media_type: Option<crate::net::MediaType>,
    /// `failure_reason` — `Optional<surf.net.FailureReason>`.
    pub failure_reason: Option<crate::net::FailureReason>,
}

/// SessionToken — `surf.agent.SessionToken`: a distinct wrapper around `String`.
///
/// The secret an agent presents to act on its session, 64 lower-case hexadecimal digits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionToken(pub String);

/// What Action — `surf.agent.Action` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Action<S>`], and at a boundary by [`ActionSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionData {
    /// The identity: `action_id` — `surf.agent.ActionId`.
    pub action_id: ActionId,
    /// `session_id` — `surf.agent.SessionId`.
    ///
    /// Carries `actions`: `surf.agent.Session` owns many `surf.agent.Action`.
    pub session_id: SessionId,
    /// `sequence` — `Integer`.
    pub sequence: i64,
    /// `command` — `surf.agent.ActionCommand`.
    pub command: ActionCommand,
    /// `url` — `Optional<surf.url.Url>`.
    pub url: Option<crate::url::Url>,
    /// `handle` — `Optional<String>`.
    pub handle: Option<String>,
    /// `label` — `Optional<String>`.
    pub label: Option<String>,
}

/// The states of `surf.agent.Action`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](action_state::Marker), so [`Action<S>`](Action) can only ever rest in a real state.
pub mod action_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `Action`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ActionState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::ActionState = super::ActionState::Recorded;
    }
}

/// Action — `surf.agent.Action` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ActionSnapshot`]
/// and [`ActionSnapshot::refine`].
pub struct Action<S: action_state::Marker> {
    data: ActionData,
    state: core::marker::PhantomData<S>,
}

impl<S: action_state::Marker> Action<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ActionState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ActionData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ActionData {
        self.data
    }
}

impl Action<action_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: ActionData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `surf.agent.Action` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ActionSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ActionState,
    /// What it holds.
    pub data: ActionData,
}

/// An `Action` in whichever declared state it was found.
pub enum AnyAction {
    /// Resting in `Recorded`.
    Recorded(Action<action_state::Recorded>),
}

impl ActionSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ActionState` cannot spell one.
    pub fn refine(self) -> AnyAction {
        match self.state {
            ActionState::Recorded => AnyAction::Recorded(Action {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyAction {
    /// The state, as the runtime value.
    pub fn state(&self) -> ActionState {
        match self {
            Self::Recorded(_) => ActionState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ActionSnapshot {
        match self {
            Self::Recorded(instance) => ActionSnapshot {
                state: ActionState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What Session — `surf.agent.Session` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Session<S>`], and at a boundary by [`SessionSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionData {
    /// The identity: `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `window_id` — `surf.shell.WindowId`.
    ///
    /// Carries `window`: `surf.agent.Session` references one `surf.shell.Window`.
    pub window_id: crate::shell::WindowId,
    /// `tab_id` — `surf.shell.TabId`.
    ///
    /// Carries `tab`: `surf.agent.Session` references one `surf.shell.Tab`.
    pub tab_id: crate::shell::TabId,
    /// `holder` — `surf.agent.SessionId`.
    ///
    /// Carries `holder`: `surf.agent.Session` references one `surf.agent.Session`.
    pub holder: SessionId,
    /// `navigation_id` — `Optional<surf.net.RequestId>`.
    ///
    /// Carries `navigation`: `surf.agent.Session` references one `surf.navigation.Navigation`.
    pub navigation_id: Option<crate::net::RequestId>,
    /// `url` — `Optional<surf.url.Url>`.
    pub url: Option<crate::url::Url>,
    /// `navigation_state` — `Optional<surf.navigation.Navigation.State>`.
    pub navigation_state: Option<crate::navigation::NavigationState>,
    /// `status` — `Optional<surf.net.Status>`.
    pub status: Option<crate::net::Status>,
    /// `media_type` — `Optional<surf.net.MediaType>`.
    pub media_type: Option<crate::net::MediaType>,
    /// `failure_reason` — `Optional<surf.net.FailureReason>`.
    pub failure_reason: Option<crate::net::FailureReason>,
    /// `scripts` — `Boolean`.
    pub scripts: bool,
}

/// The states of `surf.agent.Session`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](session_state::Marker), so [`Session<S>`](Session) can only ever rest in a real state.
pub mod session_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Closed {}
        impl Sealed for super::Open {}
    }

    /// A declared state of `Session`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SessionState;
    }

    /// `Closed`. Terminal: an instance may rest here forever.
    pub struct Closed;

    impl Marker for Closed {
        const STATE: super::SessionState = super::SessionState::Closed;
    }

    /// `Open`. Where a new instance starts.
    pub struct Open;

    impl Marker for Open {
        const STATE: super::SessionState = super::SessionState::Open;
    }
}

/// Session — `surf.agent.Session` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Open`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SessionSnapshot`]
/// and [`SessionSnapshot::refine`].
pub struct Session<S: session_state::Marker> {
    data: SessionData,
    state: core::marker::PhantomData<S>,
}

impl<S: session_state::Marker> Session<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SessionState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SessionData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SessionData {
        self.data
    }
}

impl Session<session_state::Open> {
    /// A new instance, resting in `Open` — the only state the lifecycle starts one in.
    pub fn new(data: SessionData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Session<session_state::Open> {
    /// `close` — `Open` → `Closed`. Taken by the `closed` outcome of `surf.agent.CloseSession`.
    pub fn close(self) -> Session<session_state::Closed> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `surf.agent.Session` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SessionSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SessionState,
    /// What it holds.
    pub data: SessionData,
}

/// An `Session` in whichever declared state it was found.
pub enum AnySession {
    /// Resting in `Closed`.
    Closed(Session<session_state::Closed>),
    /// Resting in `Open`.
    Open(Session<session_state::Open>),
}

impl SessionSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SessionState` cannot spell one.
    pub fn refine(self) -> AnySession {
        match self.state {
            SessionState::Closed => AnySession::Closed(Session {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            SessionState::Open => AnySession::Open(Session {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySession {
    /// The state, as the runtime value.
    pub fn state(&self) -> SessionState {
        match self {
            Self::Closed(_) => SessionState::Closed,
            Self::Open(_) => SessionState::Open,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SessionSnapshot {
        match self {
            Self::Closed(instance) => SessionSnapshot {
                state: SessionState::Closed,
                data: instance.into_data(),
            },
            Self::Open(instance) => SessionSnapshot {
                state: SessionState::Open,
                data: instance.into_data(),
            },
        }
    }
}

/// Close a session — the input of `surf.agent.CloseSession`.
///
/// Close a session and the tab it holds.
///
/// Everything it can result in is [`CloseSessionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseSession {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
}

/// Everything `surf.agent.CloseSession` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseSessionOutcome {
    /// `not-holder` — when the existing subject's stored fields satisfy `holder != caller.session_id`.
    ///
    /// The caller's token is another session's.
    NotHolder {
        /// Why it was refused: `surf.agent.NotSessionHolder`.
        error: NotSessionHolder,
    },
    /// `no-such-session` — for an identity no record carries.
    ///
    /// No session has this identity.
    NoSuchSession {
        /// Why it was refused: `surf.agent.SessionNotFound`.
        error: SessionNotFound,
    },
    /// `closed` — otherwise.
    ///
    /// The session is Closed and its tab is to be closed.
    Closed {
        /// The `surf.agent.SessionClosed` this outcome publishes.
        session_closed: SessionClosed,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The session was already Closed.
    WrongState {
        /// Why it was refused: `surf.agent.SessionStateConflict`.
        error: SessionStateConflict,
    },
}

/// Fill a control — the input of `surf.agent.Fill`.
///
/// Give a text control of the session's page a value.
///
/// Everything it can result in is [`FillOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fill {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: crate::document::ElementId,
    /// `value` — `String`.
    pub value: String,
    /// `handle` — `String`.
    pub handle: String,
}

/// Everything `surf.agent.Fill` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FillOutcome {
    /// `not-holder` — when the existing subject's stored fields satisfy `holder != caller.session_id`.
    ///
    /// The caller's token is another session's.
    NotHolder {
        /// Why it was refused: `surf.agent.NotSessionHolder`.
        error: NotSessionHolder,
    },
    /// `no-such-session` — for an identity no record carries.
    ///
    /// No session has this identity.
    NoSuchSession {
        /// Why it was refused: `surf.agent.SessionNotFound`.
        error: SessionNotFound,
    },
    /// `closed` — when the existing subject is in Closed.
    ///
    /// The session is Closed; nothing was requested.
    Closed {
        /// Why it was refused: `surf.agent.SessionStateConflict`.
        error: SessionStateConflict,
    },
    /// `requested` — otherwise.
    ///
    /// The document is asked to give the control the value.
    Requested {
        /// The `surf.agent.FillRequested` this outcome publishes.
        fill_requested: FillRequested,
    },
}

/// Follow a link — the input of `surf.agent.Follow`.
///
/// Start loading a link's target into the session's tab.
///
/// Everything it can result in is [`FollowOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Follow {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `handle` — `String`.
    pub handle: String,
    /// `label` — `String`.
    pub label: String,
    /// `initiator` — `Optional<String>`.
    pub initiator: Option<String>,
}

/// Everything `surf.agent.Follow` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowOutcome {
    /// `not-holder` — when the existing subject's stored fields satisfy `holder != caller.session_id`.
    ///
    /// The caller's token is another session's.
    NotHolder {
        /// Why it was refused: `surf.agent.NotSessionHolder`.
        error: NotSessionHolder,
    },
    /// `no-such-session` — for an identity no record carries.
    ///
    /// No session has this identity.
    NoSuchSession {
        /// Why it was refused: `surf.agent.SessionNotFound`.
        error: SessionNotFound,
    },
    /// `closed` — when the existing subject is in Closed.
    ///
    /// The session is Closed; nothing was requested.
    Closed {
        /// Why it was refused: `surf.agent.SessionStateConflict`.
        error: SessionStateConflict,
    },
    /// `requested` — otherwise.
    ///
    /// The navigator is asked to load the link's target into the session's tab.
    Requested {
        /// The `surf.agent.NavigationRequested` this outcome publishes.
        navigation_requested: NavigationRequested,
    },
}

/// Go to an address — the input of `surf.agent.Go`.
///
/// Start loading an address into the session's tab.
///
/// Everything it can result in is [`GoOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Go {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
}

/// Everything `surf.agent.Go` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoOutcome {
    /// `not-holder` — when the existing subject's stored fields satisfy `holder != caller.session_id`.
    ///
    /// The caller's token is another session's.
    NotHolder {
        /// Why it was refused: `surf.agent.NotSessionHolder`.
        error: NotSessionHolder,
    },
    /// `no-such-session` — for an identity no record carries.
    ///
    /// No session has this identity.
    NoSuchSession {
        /// Why it was refused: `surf.agent.SessionNotFound`.
        error: SessionNotFound,
    },
    /// `closed` — when the existing subject is in Closed.
    ///
    /// The session is Closed; nothing was requested.
    Closed {
        /// Why it was refused: `surf.agent.SessionStateConflict`.
        error: SessionStateConflict,
    },
    /// `requested` — otherwise.
    ///
    /// The navigator is asked to load the address into the session's tab.
    Requested {
        /// The `surf.agent.NavigationRequested` this outcome publishes.
        navigation_requested: NavigationRequested,
    },
}

/// Go back — the input of `surf.agent.GoBack`.
///
/// Ask for the entry before the current one in the session's tab.
///
/// Everything it can result in is [`GoBackOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoBack {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
}

/// Everything `surf.agent.GoBack` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoBackOutcome {
    /// `not-holder` — when the existing subject's stored fields satisfy `holder != caller.session_id`.
    ///
    /// The caller's token is another session's.
    NotHolder {
        /// Why it was refused: `surf.agent.NotSessionHolder`.
        error: NotSessionHolder,
    },
    /// `no-such-session` — for an identity no record carries.
    ///
    /// No session has this identity.
    NoSuchSession {
        /// Why it was refused: `surf.agent.SessionNotFound`.
        error: SessionNotFound,
    },
    /// `closed` — when the existing subject is in Closed.
    ///
    /// The session is Closed; nothing was requested.
    Closed {
        /// Why it was refused: `surf.agent.SessionStateConflict`.
        error: SessionStateConflict,
    },
    /// `requested` — otherwise.
    ///
    /// The navigator is asked for the entry before the current one.
    Requested {
        /// The `surf.agent.TraversalRequested` this outcome publishes.
        traversal_requested: TraversalRequested,
    },
}

/// Go forward — the input of `surf.agent.GoForward`.
///
/// Ask for the entry after the current one in the session's tab.
///
/// Everything it can result in is [`GoForwardOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoForward {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
}

/// Everything `surf.agent.GoForward` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoForwardOutcome {
    /// `not-holder` — when the existing subject's stored fields satisfy `holder != caller.session_id`.
    ///
    /// The caller's token is another session's.
    NotHolder {
        /// Why it was refused: `surf.agent.NotSessionHolder`.
        error: NotSessionHolder,
    },
    /// `no-such-session` — for an identity no record carries.
    ///
    /// No session has this identity.
    NoSuchSession {
        /// Why it was refused: `surf.agent.SessionNotFound`.
        error: SessionNotFound,
    },
    /// `closed` — when the existing subject is in Closed.
    ///
    /// The session is Closed; nothing was requested.
    Closed {
        /// Why it was refused: `surf.agent.SessionStateConflict`.
        error: SessionStateConflict,
    },
    /// `requested` — otherwise.
    ///
    /// The navigator is asked for the entry after the current one.
    Requested {
        /// The `surf.agent.TraversalRequested` this outcome publishes.
        traversal_requested: TraversalRequested,
    },
}

/// Note a committed navigation — the input of `surf.agent.NoteNavigationCommitted`.
///
/// Record on the session showing a navigation that it committed, with its address, status and media type.
///
/// Everything it can result in is [`NoteNavigationCommittedOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteNavigationCommitted {
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `status` — `surf.net.Status`.
    pub status: crate::net::Status,
    /// `media_type` — `surf.net.MediaType`.
    pub media_type: crate::net::MediaType,
}

/// Everything `surf.agent.NoteNavigationCommitted` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteNavigationCommittedOutcome {
    /// `noted` — otherwise.
    ///
    /// The session showing the navigation, if any, shows it Committed at its address with its status and media type.
    Noted {
        /// The `surf.agent.SessionNavigationChanged` this outcome publishes.
        session_navigation_changed: SessionNavigationChanged,
    },
}

/// Note an ended navigation — the input of `surf.agent.NoteNavigationEnded`.
///
/// Record on the session showing a navigation how the navigation ended.
///
/// Everything it can result in is [`NoteNavigationEndedOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteNavigationEnded {
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
    /// `state` — `surf.navigation.Navigation.State`.
    pub state: crate::navigation::NavigationState,
}

/// Everything `surf.agent.NoteNavigationEnded` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteNavigationEndedOutcome {
    /// `noted` — otherwise.
    ///
    /// The session showing the navigation, if any, shows the state it ended in.
    Noted {
        /// The `surf.agent.SessionNavigationChanged` this outcome publishes.
        session_navigation_changed: SessionNavigationChanged,
    },
}

/// Note a failed navigation — the input of `surf.agent.NoteNavigationFailed`.
///
/// Record on the session showing a navigation that it failed, and why.
///
/// Everything it can result in is [`NoteNavigationFailedOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteNavigationFailed {
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
    /// `reason` — `surf.net.FailureReason`.
    pub reason: crate::net::FailureReason,
}

/// Everything `surf.agent.NoteNavigationFailed` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteNavigationFailedOutcome {
    /// `noted` — otherwise.
    ///
    /// The session showing the navigation, if any, shows it Failed with its reason.
    Noted {
        /// The `surf.agent.SessionNavigationChanged` this outcome publishes.
        session_navigation_changed: SessionNavigationChanged,
    },
}

/// Note a started navigation — the input of `surf.agent.NoteNavigationStarted`.
///
/// Record on the session holding the tab that a navigation started in it.
///
/// Everything it can result in is [`NoteNavigationStartedOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteNavigationStarted {
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
}

/// Everything `surf.agent.NoteNavigationStarted` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteNavigationStartedOutcome {
    /// `noted` — otherwise.
    ///
    /// The session holding the tab, if any, shows the navigation as Started.
    Noted {
        /// The `surf.agent.SessionNavigationChanged` this outcome publishes.
        session_navigation_changed: SessionNavigationChanged,
    },
}

/// Open a session — the input of `surf.agent.OpenSession`.
///
/// Open a session over a window and tab of its own.
///
/// Everything it can result in is [`OpenSessionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenSession {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `window_id` — `surf.shell.WindowId`.
    pub window_id: crate::shell::WindowId,
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `scripts` — `Boolean`.
    pub scripts: bool,
}

/// Everything `surf.agent.OpenSession` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenSessionOutcome {
    /// `opened` — otherwise.
    ///
    /// The session is Open.
    Opened {
        /// The `surf.agent.SessionOpened` this outcome publishes.
        session_opened: SessionOpened,
    },
    /// `exists` — for an identity a record already carries.
    ///
    /// A session already has this identity; it is unchanged.
    Exists {
        /// Why it was refused: `surf.agent.SessionExists`.
        error: SessionExists,
    },
}

/// Pick a choice — the input of `surf.agent.Pick`.
///
/// Toggle a checkbox, check a radio button or select an option of the session's page.
///
/// Everything it can result in is [`PickOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pick {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: crate::document::ElementId,
    /// `option_id` — `Optional<surf.document.OptionId>`.
    pub option_id: Option<crate::document::OptionId>,
    /// `handle` — `String`.
    pub handle: String,
}

/// Everything `surf.agent.Pick` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickOutcome {
    /// `not-holder` — when the existing subject's stored fields satisfy `holder != caller.session_id`.
    ///
    /// The caller's token is another session's.
    NotHolder {
        /// Why it was refused: `surf.agent.NotSessionHolder`.
        error: NotSessionHolder,
    },
    /// `no-such-session` — for an identity no record carries.
    ///
    /// No session has this identity.
    NoSuchSession {
        /// Why it was refused: `surf.agent.SessionNotFound`.
        error: SessionNotFound,
    },
    /// `closed` — when the existing subject is in Closed.
    ///
    /// The session is Closed; nothing was requested.
    Closed {
        /// Why it was refused: `surf.agent.SessionStateConflict`.
        error: SessionStateConflict,
    },
    /// `requested` — otherwise.
    ///
    /// The document is asked to change the control's choice.
    Requested {
        /// The `surf.agent.PickRequested` this outcome publishes.
        pick_requested: PickRequested,
    },
}

/// Press a button — the input of `surf.agent.Press`.
///
/// Activate a button of the session's page, which submits its form.
///
/// Everything it can result in is [`PressOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Press {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: crate::document::ElementId,
    /// `handle` — `String`.
    pub handle: String,
}

/// Everything `surf.agent.Press` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PressOutcome {
    /// `not-holder` — when the existing subject's stored fields satisfy `holder != caller.session_id`.
    ///
    /// The caller's token is another session's.
    NotHolder {
        /// Why it was refused: `surf.agent.NotSessionHolder`.
        error: NotSessionHolder,
    },
    /// `no-such-session` — for an identity no record carries.
    ///
    /// No session has this identity.
    NoSuchSession {
        /// Why it was refused: `surf.agent.SessionNotFound`.
        error: SessionNotFound,
    },
    /// `closed` — when the existing subject is in Closed.
    ///
    /// The session is Closed; nothing was requested.
    Closed {
        /// Why it was refused: `surf.agent.SessionStateConflict`.
        error: SessionStateConflict,
    },
    /// `requested` — otherwise.
    ///
    /// The document is asked to activate the button.
    Requested {
        /// The `surf.agent.PressRequested` this outcome publishes.
        press_requested: PressRequested,
    },
}

/// Record an action — the input of `surf.agent.RecordAction`.
///
/// Add an accepted action to its session's log.
///
/// Everything it can result in is [`RecordActionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordAction {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `sequence` — `Integer`.
    pub sequence: i64,
    /// `command` — `surf.agent.ActionCommand`.
    pub command: ActionCommand,
    /// `url` — `Optional<surf.url.Url>`.
    pub url: Option<crate::url::Url>,
    /// `handle` — `Optional<String>`.
    pub handle: Option<String>,
    /// `label` — `Optional<String>`.
    pub label: Option<String>,
}

/// Everything `surf.agent.RecordAction` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordActionOutcome {
    /// `no-such-session` — when no `surf.agent.Session` carries the identity `input.session_id` names.
    ///
    /// No session has this identity; nothing is recorded.
    NoSuchSession {
        /// Why it was refused: `surf.agent.SessionNotFound`.
        error: SessionNotFound,
    },
    /// `recorded` — otherwise.
    ///
    /// The action is a row of its session's log.
    Recorded {
        /// The `surf.agent.ActionRecorded` this outcome publishes.
        action_recorded: ActionRecorded,
    },
}

/// ActionRecorded — the event `surf.agent.ActionRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRecorded {
    /// `action_id` — `surf.agent.ActionId`.
    pub action_id: ActionId,
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `sequence` — `Integer`.
    pub sequence: i64,
    /// `command` — `surf.agent.ActionCommand`.
    pub command: ActionCommand,
    /// `url` — `Optional<surf.url.Url>`.
    pub url: Option<crate::url::Url>,
    /// `handle` — `Optional<String>`.
    pub handle: Option<String>,
    /// `label` — `Optional<String>`.
    pub label: Option<String>,
}

/// EventsLeftOut — the event `surf.agent.EventsLeftOut`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventsLeftOut {
    /// `count` — `Integer`.
    pub count: i64,
}

/// FillRequested — the event `surf.agent.FillRequested`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FillRequested {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: crate::document::ElementId,
    /// `value` — `String`.
    pub value: String,
    /// `handle` — `Optional<String>`.
    pub handle: Option<String>,
}

/// NavigationRequested — the event `surf.agent.NavigationRequested`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationRequested {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `handle` — `Optional<String>`.
    pub handle: Option<String>,
    /// `label` — `Optional<String>`.
    pub label: Option<String>,
    /// `initiator` — `Optional<String>`.
    pub initiator: Option<String>,
}

/// PickRequested — the event `surf.agent.PickRequested`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickRequested {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: crate::document::ElementId,
    /// `option_id` — `Optional<surf.document.OptionId>`.
    pub option_id: Option<crate::document::OptionId>,
    /// `handle` — `Optional<String>`.
    pub handle: Option<String>,
}

/// PressRequested — the event `surf.agent.PressRequested`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PressRequested {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: crate::document::ElementId,
    /// `handle` — `Optional<String>`.
    pub handle: Option<String>,
}

/// SessionClosed — the event `surf.agent.SessionClosed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionClosed {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `window_id` — `surf.shell.WindowId`.
    pub window_id: crate::shell::WindowId,
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
}

/// SessionNavigationChanged — the event `surf.agent.SessionNavigationChanged`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionNavigationChanged {
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
    /// `state` — `surf.navigation.Navigation.State`.
    pub state: crate::navigation::NavigationState,
    /// `sessions` — `Integer`.
    pub sessions: i64,
}

/// SessionOpened — the event `surf.agent.SessionOpened`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionOpened {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `window_id` — `surf.shell.WindowId`.
    pub window_id: crate::shell::WindowId,
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
}

/// TraversalRequested — the event `surf.agent.TraversalRequested`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraversalRequested {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `step` — `surf.navigation.Standing`.
    pub step: crate::navigation::Standing,
}

/// The declared error `surf.agent.NotSessionHolder`.
///
/// The caller's token is not this session's, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotSessionHolder {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
}

/// The declared error `surf.agent.SessionExists`.
///
/// A session already has this identity, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionExists {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
}

/// The declared error `surf.agent.SessionNotFound`.
///
/// No session has this identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionNotFound {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
}

/// The declared error `surf.agent.SessionStateConflict`.
///
/// The session is not in a state this command acts from, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStateConflict {
    /// `state` — `surf.agent.Session.State`.
    pub state: SessionState,
}

/// Session log — one row of the view `surf.agent.SessionLog`.
///
/// Projects `surf.agent.Action` at `read_your_writes` consistency, containing instances where `session_id == param.session_id`.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLog {
    /// `action_id` — `surf.agent.ActionId`.
    pub action_id: ActionId,
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `sequence` — `Integer`.
    pub sequence: i64,
    /// `command` — `surf.agent.ActionCommand`.
    pub command: ActionCommand,
    /// `url` — `Optional<surf.url.Url>`.
    pub url: Option<crate::url::Url>,
    /// `handle` — `Optional<String>`.
    pub handle: Option<String>,
    /// `label` — `Optional<String>`.
    pub label: Option<String>,
}

/// Session state — one row of the view `surf.agent.SessionState`.
///
/// Projects `surf.agent.Session` at `read_your_writes` consistency, containing instances where `session_id == param.session_id`.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStateView {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `state` — `surf.agent.Session.State`.
    pub state: SessionState,
    /// `url` — `Optional<surf.url.Url>`.
    pub url: Option<crate::url::Url>,
    /// `navigation_state` — `Optional<surf.navigation.Navigation.State>`.
    pub navigation_state: Option<crate::navigation::NavigationState>,
    /// `status` — `Optional<surf.net.Status>`.
    pub status: Option<crate::net::Status>,
    /// `media_type` — `Optional<surf.net.MediaType>`.
    pub media_type: Option<crate::net::MediaType>,
    /// `failure_reason` — `Optional<surf.net.FailureReason>`.
    pub failure_reason: Option<crate::net::FailureReason>,
}

/// Sessions — one row of the view `surf.agent.Sessions`.
///
/// Projects `surf.agent.Session` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sessions {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `state` — `surf.agent.Session.State`.
    pub state: SessionState,
    /// `window_id` — `surf.shell.WindowId`.
    pub window_id: crate::shell::WindowId,
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `navigation_id` — `Optional<surf.net.RequestId>`.
    pub navigation_id: Option<crate::net::RequestId>,
    /// `url` — `Optional<surf.url.Url>`.
    pub url: Option<crate::url::Url>,
    /// `navigation_state` — `Optional<surf.navigation.Navigation.State>`.
    pub navigation_state: Option<crate::navigation::NavigationState>,
    /// `status` — `Optional<surf.net.Status>`.
    pub status: Option<crate::net::Status>,
    /// `media_type` — `Optional<surf.net.MediaType>`.
    pub media_type: Option<crate::net::MediaType>,
    /// `failure_reason` — `Optional<surf.net.FailureReason>`.
    pub failure_reason: Option<crate::net::FailureReason>,
    /// `scripts` — `Boolean`.
    pub scripts: bool,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `surf.agent.CloseSession` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait CloseSessionBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.agent.CloseSession`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn close_session(&mut self, input: super::CloseSession) -> Result<super::CloseSessionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.agent.Fill` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait FillBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.agent.Fill`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn fill(&mut self, input: super::Fill) -> Result<super::FillOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.agent.Follow` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait FollowBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.agent.Follow`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn follow(&mut self, input: super::Follow) -> Result<super::FollowOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.agent.Go` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait GoBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.agent.Go`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn go(&mut self, input: super::Go) -> Result<super::GoOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.agent.GoBack` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait GoBackBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.agent.GoBack`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn go_back(&mut self, input: super::GoBack) -> Result<super::GoBackOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.agent.GoForward` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait GoForwardBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.agent.GoForward`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn go_forward(&mut self, input: super::GoForward) -> Result<super::GoForwardOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.agent.NoteNavigationCommitted` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a set outcome over filtered rows (`instances:`), in `noted`.
    ///
    /// Contract: given `surf.agent.NoteNavigationCommitted` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `noted` otherwise, changes every `surf.agent.Session` where `navigation_id == input.navigation_id`, setting `url`, `navigation_state`, `status`, `media_type`, emits `surf.agent.SessionNavigationChanged`.
    pub trait NoteNavigationCommittedBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.agent.NoteNavigationCommitted`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn note_navigation_committed(&mut self, input: super::NoteNavigationCommitted) -> Result<super::NoteNavigationCommittedOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.agent.NoteNavigationEnded` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a set outcome over filtered rows (`instances:`), in `noted`.
    ///
    /// Contract: given `surf.agent.NoteNavigationEnded` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `noted` otherwise, changes every `surf.agent.Session` where `navigation_id == input.navigation_id`, setting `navigation_state`, emits `surf.agent.SessionNavigationChanged`.
    pub trait NoteNavigationEndedBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.agent.NoteNavigationEnded`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn note_navigation_ended(&mut self, input: super::NoteNavigationEnded) -> Result<super::NoteNavigationEndedOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.agent.NoteNavigationFailed` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a set outcome over filtered rows (`instances:`), in `noted`.
    ///
    /// Contract: given `surf.agent.NoteNavigationFailed` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `noted` otherwise, changes every `surf.agent.Session` where `navigation_id == input.navigation_id`, setting `navigation_state`, `failure_reason`, emits `surf.agent.SessionNavigationChanged`.
    pub trait NoteNavigationFailedBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.agent.NoteNavigationFailed`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn note_navigation_failed(&mut self, input: super::NoteNavigationFailed) -> Result<super::NoteNavigationFailedOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.agent.NoteNavigationStarted` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a set outcome over filtered rows (`instances:`), in `noted`.
    ///
    /// Contract: given `surf.agent.NoteNavigationStarted` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `noted` otherwise, changes every `surf.agent.Session` where `tab_id == input.tab_id`, setting `navigation_id`, `url`, `navigation_state`, `status`, `media_type`, `failure_reason`, emits `surf.agent.SessionNavigationChanged`.
    pub trait NoteNavigationStartedBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.agent.NoteNavigationStarted`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn note_navigation_started(&mut self, input: super::NoteNavigationStarted) -> Result<super::NoteNavigationStartedOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.agent.OpenSession` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait OpenSessionBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.agent.OpenSession`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn open_session(&mut self, input: super::OpenSession) -> Result<super::OpenSessionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.agent.Pick` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait PickBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.agent.Pick`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn pick(&mut self, input: super::Pick) -> Result<super::PickOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.agent.Press` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait PressBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.agent.Press`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn press(&mut self, input: super::Press) -> Result<super::PressOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.agent.RecordAction` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordActionBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.agent.RecordAction`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_action(&mut self, input: super::RecordAction) -> Result<super::RecordActionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `surf.agent.SessionLog` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait SessionLogQuery {
        /// Serves `surf.agent.SessionLog` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn session_log(&self, session_id: crate::agent::SessionId) -> Result<Vec<super::SessionLog>, crate::obligation::UnmetObligation>;
    }

    /// The query `surf.agent.SessionState` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait SessionStateViewQuery {
        /// Serves `surf.agent.SessionState` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn session_state_view(&self, session_id: crate::agent::SessionId) -> Result<Vec<super::SessionStateView>, crate::obligation::UnmetObligation>;
    }

    /// The query `surf.agent.Sessions` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait SessionsQuery {
        /// Serves `surf.agent.Sessions` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn sessions(&self) -> Result<Vec<super::Sessions>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl NoteNavigationCommittedBehavior for Unimplemented {
        fn note_navigation_committed(&mut self, _input: super::NoteNavigationCommitted) -> Result<super::NoteNavigationCommittedOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "surf.agent.NoteNavigationCommitted" })
        }
    }

    impl NoteNavigationEndedBehavior for Unimplemented {
        fn note_navigation_ended(&mut self, _input: super::NoteNavigationEnded) -> Result<super::NoteNavigationEndedOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "surf.agent.NoteNavigationEnded" })
        }
    }

    impl NoteNavigationFailedBehavior for Unimplemented {
        fn note_navigation_failed(&mut self, _input: super::NoteNavigationFailed) -> Result<super::NoteNavigationFailedOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "surf.agent.NoteNavigationFailed" })
        }
    }

    impl NoteNavigationStartedBehavior for Unimplemented {
        fn note_navigation_started(&mut self, _input: super::NoteNavigationStarted) -> Result<super::NoteNavigationStartedOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "surf.agent.NoteNavigationStarted" })
        }
    }
}
