// generated from surf v1
// contract digest f176714dd8b7999f1b6166f2e2d3e020c7ce9478056f6babb4bb5edef3c071ff
// do not edit: regenerate with `ess synthesize`

//! Agent — `surf.agent`.
//!
//! A browser session an AI agent drives from a command line: opened once, acted on with a token, read back as text, closed. Every action of the agent is mapped onto the commands the other components already accept; this domain holds only what the agent's session adds to them. The `agent` component holds it.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// GoCall — `surf.agent.GoCall`.
///
/// The address to load into a session, and the token that proves the caller holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoCall {
    /// `session_id` — `String`.
    pub session_id: String,
    /// `token` — `surf.agent.SessionToken`.
    pub token: SessionToken,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
}

/// OpenCall — `surf.agent.OpenCall`.
///
/// The input of `open`, the address to load first if one is given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCall {
    /// `url` — `Optional<surf.url.Url>`.
    pub url: Option<crate::url::Url>,
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
}

/// SessionToken — `surf.agent.SessionToken`: a distinct wrapper around `String`.
///
/// The secret an agent presents to act on its session, 64 lower-case hexadecimal digits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionToken(pub String);

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

/// NavigationRequested — the event `surf.agent.NavigationRequested`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationRequested {
    /// `session_id` — `surf.agent.SessionId`.
    pub session_id: SessionId,
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
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

    /// The behaviour `surf.agent.NoteNavigationStarted` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a set outcome over filtered rows (`instances:`), in `noted`.
    ///
    /// Contract: given `surf.agent.NoteNavigationStarted` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `noted` otherwise, changes every `surf.agent.Session` where `tab_id == input.tab_id`, setting `navigation_id`, `url`, `navigation_state`, emits `surf.agent.SessionNavigationChanged`.
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

    impl NoteNavigationEndedBehavior for Unimplemented {
        fn note_navigation_ended(&mut self, _input: super::NoteNavigationEnded) -> Result<super::NoteNavigationEndedOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "surf.agent.NoteNavigationEnded" })
        }
    }

    impl NoteNavigationStartedBehavior for Unimplemented {
        fn note_navigation_started(&mut self, _input: super::NoteNavigationStarted) -> Result<super::NoteNavigationStartedOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "surf.agent.NoteNavigationStarted" })
        }
    }
}
