// generated from surf v1
// contract digest 96684badf51201e308af373a0097db9fb63321aefd9a08fd216a1e28745125c0
// do not edit: regenerate with `ess synthesize`

//! What the specification fully determines, generated: the behaviour of every command the plan
//! lists as generated, written against ports the implementor supplies.
//!
//! Storage is a port: one trait per entity, get, put and delete of a snapshot by identity. ess
//! preserves the trait; generated network entries supply an ephemeral store. `Context` carries the caller's attributes,
//! every identity and value the model says the implementation assigns, and the answer to each
//! `external:` branch. [`Generated`] implements every generated `…Behavior` trait over those ports
//! and forwards every behaviour and query the plan still owes to them, so it is a complete bundle
//! for every component port. To replace one generated behaviour, write a bundle of your own that
//! implements that trait and delegates the rest to a `Generated`.
//!
//! An `Err` from a generated behaviour is the typed refusal naming the command: the model declares
//! no outcome for the request (a guard is undecidable over it, or no declared branch answers it),
//! or — as `entity invariant` — the declared outcome would leave an entity breaking an invariant.

use crate::obligation::UnmetObligation;

/// Where `surf.agent.Action` is stored — a port the implementor provides.
///
/// Keyed by the identity `action_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait ActionStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::agent::ActionId) -> Option<crate::agent::ActionSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::agent::ActionSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::agent::ActionId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::agent::ActionSnapshot>;
}

/// Where `surf.agent.Session` is stored — a port the implementor provides.
///
/// Keyed by the identity `session_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait SessionStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::agent::SessionId) -> Option<crate::agent::SessionSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::agent::SessionSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::agent::SessionId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::agent::SessionSnapshot>;
}

/// Where `surf.document.Document` is stored — a port the implementor provides.
///
/// Keyed by the identity `document_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait DocumentStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::document::DocumentId) -> Option<crate::document::DocumentSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::document::DocumentSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::document::DocumentId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::document::DocumentSnapshot>;
}

/// Where `surf.document.Element` is stored — a port the implementor provides.
///
/// Keyed by the identity `element_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait ElementStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::document::ElementId) -> Option<crate::document::ElementSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::document::ElementSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::document::ElementId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::document::ElementSnapshot>;
}

/// Where `surf.navigation.Navigation` is stored — a port the implementor provides.
///
/// Keyed by the identity `navigation_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait NavigationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::net::RequestId) -> Option<crate::navigation::NavigationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::navigation::NavigationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::net::RequestId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::navigation::NavigationSnapshot>;
}

/// Where `surf.net.Request` is stored — a port the implementor provides.
///
/// Keyed by the identity `request_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait RequestStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::net::RequestId) -> Option<crate::net::RequestSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::net::RequestSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::net::RequestId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::net::RequestSnapshot>;
}

/// Where `surf.render.Frame` is stored — a port the implementor provides.
///
/// Keyed by the identity `frame_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait FrameStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::render::FrameId) -> Option<crate::render::FrameSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::render::FrameSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::render::FrameId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::render::FrameSnapshot>;
}

/// Where `surf.shell.Tab` is stored — a port the implementor provides.
///
/// Keyed by the identity `tab_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait TabStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::shell::TabId) -> Option<crate::shell::TabSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::shell::TabSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::shell::TabId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::shell::TabSnapshot>;
}

/// Where `surf.shell.Window` is stored — a port the implementor provides.
///
/// Keyed by the identity `window_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait WindowStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::shell::WindowId) -> Option<crate::shell::WindowSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::shell::WindowSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::shell::WindowId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::shell::WindowSnapshot>;
}

/// What the specification leaves to the implementor's context — a port the implementor provides.
///
/// The caller's attributes, the values the model says the implementation assigns, and the answer
/// to each `external:` branch.
pub trait Context {
    /// The authenticated caller's `session_id`, or `None` where the caller carries none.
    fn caller_session_id(&self) -> Option<crate::agent::SessionId>;

    /// A new `surf.agent.ActionId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_surf_agent_action_id(&mut self) -> crate::agent::ActionId;

    /// A new `surf.document.DocumentId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_surf_document_document_id(&mut self) -> crate::document::DocumentId;

    /// A new `surf.document.ElementId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_surf_document_element_id(&mut self) -> crate::document::ElementId;

    /// A new `surf.net.RequestId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_surf_net_request_id(&mut self) -> crate::net::RequestId;

    /// A new `surf.render.FrameId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_surf_render_frame_id(&mut self) -> crate::render::FrameId;

    /// A new `surf.shell.TabId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_surf_shell_tab_id(&mut self) -> crate::shell::TabId;

    /// A new `surf.shell.WindowId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_surf_shell_window_id(&mut self) -> crate::shell::WindowId;
}

/// Context answers that may be unavailable, without fabricated values.
/// Existing `Context` implementations receive the blanket adapter.
pub trait TryContext {
/// The caller's `session_id`, or a named unavailable answer.
fn try_caller_session_id(&self) -> Result<Option<crate::agent::SessionId>, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_surf_agent_action_id(&mut self) -> Result<crate::agent::ActionId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_surf_document_document_id(&mut self) -> Result<crate::document::DocumentId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_surf_document_element_id(&mut self) -> Result<crate::document::ElementId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_surf_net_request_id(&mut self) -> Result<crate::net::RequestId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_surf_render_frame_id(&mut self) -> Result<crate::render::FrameId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_surf_shell_tab_id(&mut self) -> Result<crate::shell::TabId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_surf_shell_window_id(&mut self) -> Result<crate::shell::WindowId, UnmetObligation>;
}

impl<T: Context + ?Sized> TryContext for T {
fn try_caller_session_id(&self) -> Result<Option<crate::agent::SessionId>, UnmetObligation> { Ok(Context::caller_session_id(self)) }
fn try_generate_surf_agent_action_id(&mut self) -> Result<crate::agent::ActionId, UnmetObligation> { Ok(Context::generate_surf_agent_action_id(self)) }
fn try_generate_surf_document_document_id(&mut self) -> Result<crate::document::DocumentId, UnmetObligation> { Ok(Context::generate_surf_document_document_id(self)) }
fn try_generate_surf_document_element_id(&mut self) -> Result<crate::document::ElementId, UnmetObligation> { Ok(Context::generate_surf_document_element_id(self)) }
fn try_generate_surf_net_request_id(&mut self) -> Result<crate::net::RequestId, UnmetObligation> { Ok(Context::generate_surf_net_request_id(self)) }
fn try_generate_surf_render_frame_id(&mut self) -> Result<crate::render::FrameId, UnmetObligation> { Ok(Context::generate_surf_render_frame_id(self)) }
fn try_generate_surf_shell_tab_id(&mut self) -> Result<crate::shell::TabId, UnmetObligation> { Ok(Context::generate_surf_shell_tab_id(self)) }
fn try_generate_surf_shell_window_id(&mut self) -> Result<crate::shell::WindowId, UnmetObligation> { Ok(Context::generate_surf_shell_window_id(self)) }
}

/// An unavailable runtime context answer, rather than a new planned capability.
pub fn unmet_context(source: &'static str) -> UnmetObligation { UnmetObligation { capability: "context answer", source } }

/// Every generated behaviour of this workspace, over the ports `P` supplies.
///
/// `P` implements the storage trait of each entity a generated behaviour reads or writes,
/// `TryContext` (or its legacy `Context` blanket adapter) where one asks it anything, and every `…Behavior` and `…Query` trait the plan still
/// owes; `Generated<P>` forwards those to it.
pub struct Generated<P> {
    /// The storage and context ports, and every behaviour or query still owed.
    pub ports: P,
}

impl<P> Generated<P> {
    /// The generated behaviours, over `ports`.
    pub fn new(ports: P) -> Self {
        Self { ports }
    }
}

/// `surf.agent.CloseSession`, generated: every outcome is one the specification fully determines.
impl<P> crate::agent::obligations::CloseSessionBehavior for Generated<P>
where
    P: TryContext + SessionStorage,
{
    fn close_session(&mut self, input: crate::agent::CloseSession) -> Result<crate::agent::CloseSessionOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if SessionStorage::get(&self.ports, &input.session_id).is_none() {
            return Ok(crate::agent::CloseSessionOutcome::NoSuchSession { error: crate::agent::SessionNotFound { session_id: input.session_id.clone() } });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = SessionStorage::get(&self.ports, &input.session_id) else {
            return Ok(crate::agent::CloseSessionOutcome::NoSuchSession { error: crate::agent::SessionNotFound { session_id: input.session_id.clone() } });
        };
        let _ = &held;
        // `not-holder`: selected by the addressed row.
        if decided(equal(Some(&held.data.holder).map(|value| &value.0).map(|value| value.0.clone()), self.ports.try_caller_session_id()?.as_ref().map(|value| &value.0).map(|value| value.0.clone())).map(|value| !value), "surf.agent.CloseSession")? {
            return Ok(crate::agent::CloseSessionOutcome::NotHolder { error: crate::agent::NotSessionHolder { session_id: input.session_id.clone() } });
        }
        // `closed`: the default.
        let Some(held) = SessionStorage::get(&self.ports, &input.session_id) else {
            return Ok(crate::agent::CloseSessionOutcome::NoSuchSession { error: crate::agent::SessionNotFound { session_id: input.session_id.clone() } });
        };
        let _ = &held;
        let held_state = held.state;
        let before = held.data.clone();
        let moved = match held.refine() {
            crate::agent::AnySession::Open(instance) => crate::agent::AnySession::Closed(instance.close()),
            _ => return Ok(crate::agent::CloseSessionOutcome::WrongState { error: crate::agent::SessionStateConflict { state: held_state } }),
        };
        let next = moved.snapshot();
        let answer = crate::agent::CloseSessionOutcome::Closed { session_closed: crate::agent::SessionClosed { session_id: input.session_id.clone(), window_id: before.window_id.clone(), tab_id: before.tab_id.clone() } };
        SessionStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `surf.agent.Go`, generated: every outcome is one the specification fully determines.
impl<P> crate::agent::obligations::GoBehavior for Generated<P>
where
    P: TryContext + SessionStorage,
{
    fn go(&mut self, input: crate::agent::Go) -> Result<crate::agent::GoOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if SessionStorage::get(&self.ports, &input.session_id).is_none() {
            return Ok(crate::agent::GoOutcome::NoSuchSession { error: crate::agent::SessionNotFound { session_id: input.session_id.clone() } });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = SessionStorage::get(&self.ports, &input.session_id) else {
            return Ok(crate::agent::GoOutcome::NoSuchSession { error: crate::agent::SessionNotFound { session_id: input.session_id.clone() } });
        };
        let _ = &held;
        // `not-holder`: selected by the addressed row.
        if decided(equal(Some(&held.data.holder).map(|value| &value.0).map(|value| value.0.clone()), self.ports.try_caller_session_id()?.as_ref().map(|value| &value.0).map(|value| value.0.clone())).map(|value| !value), "surf.agent.Go")? {
            return Ok(crate::agent::GoOutcome::NotHolder { error: crate::agent::NotSessionHolder { session_id: input.session_id.clone() } });
        }
        // `closed`: selected by the addressed row.
        if decided(Some(matches!(held.state, crate::agent::SessionState::Closed)), "surf.agent.Go")? {
            return Ok(crate::agent::GoOutcome::Closed { error: crate::agent::SessionStateConflict { state: held.state } });
        }
        // `requested`: the default.
        let Some(held) = SessionStorage::get(&self.ports, &input.session_id) else {
            return Ok(crate::agent::GoOutcome::NoSuchSession { error: crate::agent::SessionNotFound { session_id: input.session_id.clone() } });
        };
        let _ = &held;
        let before = held.data.clone();
        let next = held;
        let answer = crate::agent::GoOutcome::Requested { navigation_requested: crate::agent::NavigationRequested { session_id: input.session_id.clone(), tab_id: before.tab_id.clone(), url: input.url.clone() } };
        SessionStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

impl<P: crate::agent::obligations::NoteNavigationCommittedBehavior> crate::agent::obligations::NoteNavigationCommittedBehavior for Generated<P> {
    fn note_navigation_committed(&mut self, input: crate::agent::NoteNavigationCommitted) -> Result<crate::agent::NoteNavigationCommittedOutcome, UnmetObligation> {
        crate::agent::obligations::NoteNavigationCommittedBehavior::note_navigation_committed(&mut self.ports, input)
    }
}

impl<P: crate::agent::obligations::NoteNavigationEndedBehavior> crate::agent::obligations::NoteNavigationEndedBehavior for Generated<P> {
    fn note_navigation_ended(&mut self, input: crate::agent::NoteNavigationEnded) -> Result<crate::agent::NoteNavigationEndedOutcome, UnmetObligation> {
        crate::agent::obligations::NoteNavigationEndedBehavior::note_navigation_ended(&mut self.ports, input)
    }
}

impl<P: crate::agent::obligations::NoteNavigationFailedBehavior> crate::agent::obligations::NoteNavigationFailedBehavior for Generated<P> {
    fn note_navigation_failed(&mut self, input: crate::agent::NoteNavigationFailed) -> Result<crate::agent::NoteNavigationFailedOutcome, UnmetObligation> {
        crate::agent::obligations::NoteNavigationFailedBehavior::note_navigation_failed(&mut self.ports, input)
    }
}

impl<P: crate::agent::obligations::NoteNavigationStartedBehavior> crate::agent::obligations::NoteNavigationStartedBehavior for Generated<P> {
    fn note_navigation_started(&mut self, input: crate::agent::NoteNavigationStarted) -> Result<crate::agent::NoteNavigationStartedOutcome, UnmetObligation> {
        crate::agent::obligations::NoteNavigationStartedBehavior::note_navigation_started(&mut self.ports, input)
    }
}

/// `surf.agent.OpenSession`, generated: every outcome is one the specification fully determines.
impl<P> crate::agent::obligations::OpenSessionBehavior for Generated<P>
where
    P: SessionStorage,
{
    fn open_session(&mut self, input: crate::agent::OpenSession) -> Result<crate::agent::OpenSessionOutcome, UnmetObligation> {
        let _ = &input;
        // `exists`: an identity a record already carries, before any branch is taken.
        if SessionStorage::get(&self.ports, &input.session_id).is_some() {
            return Ok(crate::agent::OpenSessionOutcome::Exists { error: crate::agent::SessionExists { session_id: input.session_id.clone() } });
        }
        // `opened`: the default.
        let identity: crate::agent::SessionId = input.session_id.clone();
        let data = crate::agent::SessionData {
            session_id: identity.clone(),
            window_id: input.window_id.clone(),
            tab_id: input.tab_id.clone(),
            holder: input.session_id.clone(),
            navigation_id: None,
            url: None,
            navigation_state: None,
            status: None,
            media_type: None,
            failure_reason: None,
        };
        let answer = crate::agent::OpenSessionOutcome::Opened { session_opened: crate::agent::SessionOpened { session_id: identity.clone(), window_id: input.window_id.clone(), tab_id: input.tab_id.clone() } };
        SessionStorage::put(&mut self.ports, crate::agent::AnySession::Open(crate::agent::Session::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `surf.agent.RecordAction`, generated: every outcome is one the specification fully determines.
impl<P> crate::agent::obligations::RecordActionBehavior for Generated<P>
where
    P: TryContext + ActionStorage + SessionStorage,
{
    fn record_action(&mut self, input: crate::agent::RecordAction) -> Result<crate::agent::RecordActionOutcome, UnmetObligation> {
        let _ = &input;
        // `when_related:` reads the `surf.agent.Session` row `input.session_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.session_id);
        let related = reference.and_then(|identity| SessionStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-session`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::agent::RecordActionOutcome::NoSuchSession { error: crate::agent::SessionNotFound { session_id: input.session_id.clone() } });
        }
        // `recorded`: the default.
        let identity: crate::agent::ActionId = self.ports.try_generate_surf_agent_action_id()?;
        let data = crate::agent::ActionData {
            action_id: identity.clone(),
            session_id: input.session_id.clone(),
            sequence: input.sequence.clone(),
            command: input.command.clone(),
            url: input.url.clone(),
        };
        let answer = crate::agent::RecordActionOutcome::Recorded { action_recorded: crate::agent::ActionRecorded { action_id: identity.clone(), session_id: input.session_id.clone(), sequence: input.sequence.clone(), command: input.command.clone(), url: input.url.clone() } };
        ActionStorage::put(&mut self.ports, crate::agent::AnyAction::Recorded(crate::agent::Action::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `surf.document.AddElement`, generated: every outcome is one the specification fully determines.
impl<P> crate::document::obligations::AddElementBehavior for Generated<P>
where
    P: TryContext + DocumentStorage + ElementStorage,
{
    fn add_element(&mut self, input: crate::document::AddElement) -> Result<crate::document::AddElementOutcome, UnmetObligation> {
        let _ = &input;
        // `when_related:` reads the `surf.document.Document` row `input.document_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.document_id);
        let related = reference.and_then(|identity| DocumentStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-document`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::document::AddElementOutcome::NoSuchDocument { error: crate::document::DocumentNotFound { document_id: input.document_id.clone() } });
        }
        // `already-parsed`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(equal(Some(&related.state).map(|value| match value { crate::document::DocumentState::Parsed => "Parsed", crate::document::DocumentState::Parsing => "Parsing" }.to_owned()), Some("Parsed".to_owned())), "surf.document.AddElement")? {
            return Ok(crate::document::AddElementOutcome::AlreadyParsed { error: crate::document::DocumentStateConflict { state: crate::document::DocumentState::Parsed } });
        }
        }
        // `added`: the default.
        let identity: crate::document::ElementId = self.ports.try_generate_surf_document_element_id()?;
        let data = crate::document::ElementData {
            element_id: identity.clone(),
            document_id: input.document_id.clone(),
            position: input.position.clone(),
            kind: input.kind.clone(),
            label: input.label.clone(),
            target: input.target.clone(),
            value: input.value.clone(),
            hidden: input.hidden.clone(),
            disabled: input.disabled.clone(),
            checked: input.checked.clone(),
            required: input.required.clone(),
        };
        let answer = crate::document::AddElementOutcome::Added { element_added: crate::document::ElementAdded { element_id: identity.clone(), document_id: input.document_id.clone(), position: input.position.clone() } };
        ElementStorage::put(&mut self.ports, crate::document::AnyElement::Added(crate::document::Element::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `surf.document.FinishParsing`, generated: every outcome is one the specification fully determines.
impl<P> crate::document::obligations::FinishParsingBehavior for Generated<P>
where
    P: DocumentStorage,
{
    fn finish_parsing(&mut self, input: crate::document::FinishParsing) -> Result<crate::document::FinishParsingOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if DocumentStorage::get(&self.ports, &input.document_id).is_none() {
            return Ok(crate::document::FinishParsingOutcome::NoSuchDocument { error: crate::document::DocumentNotFound { document_id: input.document_id.clone() } });
        }
        // `parsed`: the default.
        let Some(held) = DocumentStorage::get(&self.ports, &input.document_id) else {
            return Ok(crate::document::FinishParsingOutcome::NoSuchDocument { error: crate::document::DocumentNotFound { document_id: input.document_id.clone() } });
        };
        let _ = &held;
        let held_state = held.state;
        let before = held.data.clone();
        let moved = match held.refine() {
            crate::document::AnyDocument::Parsing(instance) => crate::document::AnyDocument::Parsed(instance.finish()),
            _ => return Ok(crate::document::FinishParsingOutcome::WrongState { error: crate::document::DocumentStateConflict { state: held_state } }),
        };
        let mut next = moved.snapshot();
        next.data.title = Some(input.title.clone());
        next.data.encoding = Some(input.encoding.clone());
        next.data.unsupported_encoding = input.unsupported_encoding.clone();
        next.data.stopped_by = input.stopped_by.clone();
        let answer = crate::document::FinishParsingOutcome::Parsed { document_parsed: crate::document::DocumentParsed { document_id: input.document_id.clone(), url: before.url.clone(), navigation_id: before.navigation_id.clone(), title: input.title.clone(), encoding: input.encoding.clone(), unsupported_encoding: input.unsupported_encoding.clone(), stopped_by: input.stopped_by.clone() } };
        DocumentStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `surf.document.ParseDocument`, generated: every outcome is one the specification fully determines.
impl<P> crate::document::obligations::ParseDocumentBehavior for Generated<P>
where
    P: TryContext + DocumentStorage,
{
    fn parse_document(&mut self, input: crate::document::ParseDocument) -> Result<crate::document::ParseDocumentOutcome, UnmetObligation> {
        let _ = &input;
        // `started`: the default.
        let identity: crate::document::DocumentId = self.ports.try_generate_surf_document_document_id()?;
        let data = crate::document::DocumentData {
            document_id: identity.clone(),
            url: input.url.clone(),
            navigation_id: input.navigation_id.clone(),
            title: None,
            encoding: None,
            unsupported_encoding: None,
            stopped_by: None,
        };
        let answer = crate::document::ParseDocumentOutcome::Started { parsing_started: crate::document::ParsingStarted { document_id: identity.clone(), body: input.body.clone() } };
        DocumentStorage::put(&mut self.ports, crate::document::AnyDocument::Parsing(crate::document::Document::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `surf.navigation.CancelNavigation`, generated: every outcome is one the specification fully determines.
impl<P> crate::navigation::obligations::CancelNavigationBehavior for Generated<P>
where
    P: NavigationStorage,
{
    fn cancel_navigation(&mut self, input: crate::navigation::CancelNavigation) -> Result<crate::navigation::CancelNavigationOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if NavigationStorage::get(&self.ports, &input.navigation_id).is_none() {
            return Ok(crate::navigation::CancelNavigationOutcome::NoSuchNavigation { error: crate::navigation::NavigationNotFound { navigation_id: input.navigation_id.clone() } });
        }
        // `cancelled`: the default.
        let Some(held) = NavigationStorage::get(&self.ports, &input.navigation_id) else {
            return Ok(crate::navigation::CancelNavigationOutcome::NoSuchNavigation { error: crate::navigation::NavigationNotFound { navigation_id: input.navigation_id.clone() } });
        };
        let _ = &held;
        let held_state = held.state;
        let before = held.data.clone();
        let moved = match held.refine() {
            crate::navigation::AnyNavigation::Started(instance) => crate::navigation::AnyNavigation::Cancelled(instance.cancel()),
            _ => return Ok(crate::navigation::CancelNavigationOutcome::WrongState { error: crate::navigation::NavigationStateConflict { state: held_state } }),
        };
        let next = moved.snapshot();
        let answer = crate::navigation::CancelNavigationOutcome::Cancelled { navigation_cancelled: crate::navigation::NavigationCancelled { navigation_id: input.navigation_id.clone(), tab_id: before.tab_id.clone() } };
        NavigationStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `surf.navigation.CommitNavigation`, generated: every outcome is one the specification fully determines.
impl<P> crate::navigation::obligations::CommitNavigationBehavior for Generated<P>
where
    P: NavigationStorage,
{
    fn commit_navigation(&mut self, input: crate::navigation::CommitNavigation) -> Result<crate::navigation::CommitNavigationOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if NavigationStorage::get(&self.ports, &input.navigation_id).is_none() {
            return Ok(crate::navigation::CommitNavigationOutcome::NoSuchNavigation { error: crate::navigation::NavigationNotFound { navigation_id: input.navigation_id.clone() } });
        }
        // `committed`: the default.
        let Some(held) = NavigationStorage::get(&self.ports, &input.navigation_id) else {
            return Ok(crate::navigation::CommitNavigationOutcome::NoSuchNavigation { error: crate::navigation::NavigationNotFound { navigation_id: input.navigation_id.clone() } });
        };
        let _ = &held;
        let held_state = held.state;
        let before = held.data.clone();
        let moved = match held.refine() {
            crate::navigation::AnyNavigation::Started(instance) => crate::navigation::AnyNavigation::Committed(instance.commit()),
            _ => return Ok(crate::navigation::CommitNavigationOutcome::WrongState { error: crate::navigation::NavigationStateConflict { state: held_state } }),
        };
        let mut next = moved.snapshot();
        next.data.url = input.url.clone();
        let answer = crate::navigation::CommitNavigationOutcome::Committed { navigation_committed: crate::navigation::NavigationCommitted { navigation_id: input.navigation_id.clone(), tab_id: before.tab_id.clone(), url: input.url.clone(), status: input.status.clone(), media_type: input.media_type.clone(), body: input.body.clone() } };
        NavigationStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `surf.navigation.FailNavigation`, generated: every outcome is one the specification fully determines.
impl<P> crate::navigation::obligations::FailNavigationBehavior for Generated<P>
where
    P: NavigationStorage,
{
    fn fail_navigation(&mut self, input: crate::navigation::FailNavigation) -> Result<crate::navigation::FailNavigationOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if NavigationStorage::get(&self.ports, &input.navigation_id).is_none() {
            return Ok(crate::navigation::FailNavigationOutcome::NoSuchNavigation { error: crate::navigation::NavigationNotFound { navigation_id: input.navigation_id.clone() } });
        }
        // `failed`: the default.
        let Some(held) = NavigationStorage::get(&self.ports, &input.navigation_id) else {
            return Ok(crate::navigation::FailNavigationOutcome::NoSuchNavigation { error: crate::navigation::NavigationNotFound { navigation_id: input.navigation_id.clone() } });
        };
        let _ = &held;
        let held_state = held.state;
        let before = held.data.clone();
        let moved = match held.refine() {
            crate::navigation::AnyNavigation::Started(instance) => crate::navigation::AnyNavigation::Failed(instance.fail()),
            _ => return Ok(crate::navigation::FailNavigationOutcome::WrongState { error: crate::navigation::NavigationStateConflict { state: held_state } }),
        };
        let next = moved.snapshot();
        let answer = crate::navigation::FailNavigationOutcome::Failed { navigation_failed: crate::navigation::NavigationFailed { navigation_id: input.navigation_id.clone(), tab_id: before.tab_id.clone(), url: before.url.clone(), reason: input.reason.clone() } };
        NavigationStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `surf.navigation.Navigate`, generated: every outcome is one the specification fully determines.
impl<P> crate::navigation::obligations::NavigateBehavior for Generated<P>
where
    P: TryContext + NavigationStorage,
{
    fn navigate(&mut self, input: crate::navigation::Navigate) -> Result<crate::navigation::NavigateOutcome, UnmetObligation> {
        let _ = &input;
        // `started`: the default.
        let identity: crate::net::RequestId = self.ports.try_generate_surf_net_request_id()?;
        let data = crate::navigation::NavigationData {
            navigation_id: identity.clone(),
            tab_id: input.tab_id.clone(),
            url: input.url.clone(),
        };
        let answer = crate::navigation::NavigateOutcome::Started { navigation_started: crate::navigation::NavigationStarted { navigation_id: identity.clone(), tab_id: input.tab_id.clone(), url: input.url.clone(), method: input.method.clone(), body: input.body.clone(), body_type: input.body_type.clone() } };
        NavigationStorage::put(&mut self.ports, crate::navigation::AnyNavigation::Started(crate::navigation::Navigation::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `surf.net.DeliverResponse`, generated: every outcome is one the specification fully determines.
impl<P> crate::net::obligations::DeliverResponseBehavior for Generated<P>
where
    P: RequestStorage,
{
    fn deliver_response(&mut self, input: crate::net::DeliverResponse) -> Result<crate::net::DeliverResponseOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if RequestStorage::get(&self.ports, &input.request_id).is_none() {
            return Ok(crate::net::DeliverResponseOutcome::NoSuchRequest { error: crate::net::RequestNotFound { request_id: input.request_id.clone() } });
        }
        // `responded`: the default.
        let Some(held) = RequestStorage::get(&self.ports, &input.request_id) else {
            return Ok(crate::net::DeliverResponseOutcome::NoSuchRequest { error: crate::net::RequestNotFound { request_id: input.request_id.clone() } });
        };
        let _ = &held;
        let held_state = held.state;
        let moved = match held.refine() {
            crate::net::AnyRequest::Pending(instance) => crate::net::AnyRequest::Responded(instance.respond()),
            _ => return Ok(crate::net::DeliverResponseOutcome::WrongState { error: crate::net::RequestStateConflict { state: held_state } }),
        };
        let next = moved.snapshot();
        let answer = crate::net::DeliverResponseOutcome::Responded { response_received: crate::net::ResponseReceived { request_id: input.request_id.clone(), url: input.url.clone(), status: input.status.clone(), media_type: input.media_type.clone(), body: input.body.clone() } };
        RequestStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `surf.net.FailRequest`, generated: every outcome is one the specification fully determines.
impl<P> crate::net::obligations::FailRequestBehavior for Generated<P>
where
    P: RequestStorage,
{
    fn fail_request(&mut self, input: crate::net::FailRequest) -> Result<crate::net::FailRequestOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if RequestStorage::get(&self.ports, &input.request_id).is_none() {
            return Ok(crate::net::FailRequestOutcome::NoSuchRequest { error: crate::net::RequestNotFound { request_id: input.request_id.clone() } });
        }
        // `failed`: the default.
        let Some(held) = RequestStorage::get(&self.ports, &input.request_id) else {
            return Ok(crate::net::FailRequestOutcome::NoSuchRequest { error: crate::net::RequestNotFound { request_id: input.request_id.clone() } });
        };
        let _ = &held;
        let held_state = held.state;
        let before = held.data.clone();
        let moved = match held.refine() {
            crate::net::AnyRequest::Pending(instance) => crate::net::AnyRequest::Failed(instance.fail()),
            _ => return Ok(crate::net::FailRequestOutcome::WrongState { error: crate::net::RequestStateConflict { state: held_state } }),
        };
        let next = moved.snapshot();
        let answer = crate::net::FailRequestOutcome::Failed { fetch_failed: crate::net::FetchFailed { request_id: input.request_id.clone(), url: before.url.clone(), reason: input.reason.clone() } };
        RequestStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `surf.net.Fetch`, generated: every outcome is one the specification fully determines.
impl<P> crate::net::obligations::FetchBehavior for Generated<P>
where
    P: RequestStorage,
{
    fn fetch(&mut self, input: crate::net::Fetch) -> Result<crate::net::FetchOutcome, UnmetObligation> {
        let _ = &input;
        // `started`: the default.
        let identity: crate::net::RequestId = input.request_id.clone();
        let data = crate::net::RequestData {
            request_id: identity.clone(),
            url: input.url.clone(),
            method: input.method.clone(),
            body_type: input.body_type.clone(),
        };
        let answer = crate::net::FetchOutcome::Started { request_started: crate::net::RequestStarted { request_id: identity.clone(), url: input.url.clone(), method: input.method.clone(), body: input.body.clone(), body_type: input.body_type.clone() } };
        RequestStorage::put(&mut self.ports, crate::net::AnyRequest::Pending(crate::net::Request::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `surf.render.PaintFrame`, generated: every outcome is one the specification fully determines.
impl<P> crate::render::obligations::PaintFrameBehavior for Generated<P>
where
    P: TryContext + FrameStorage,
{
    fn paint_frame(&mut self, input: crate::render::PaintFrame) -> Result<crate::render::PaintFrameOutcome, UnmetObligation> {
        let _ = &input;
        // `invalid-viewport`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(any(&[compare_numbers(Some(&input.width).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_le), compare_numbers(Some(&input.height).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_le)]), "surf.render.PaintFrame")? {
            return Ok(crate::render::PaintFrameOutcome::InvalidViewport { error: crate::render::InvalidViewport { width: input.width.clone(), height: input.height.clone() } });
        }
        // `painted`: the default.
        let identity: crate::render::FrameId = self.ports.try_generate_surf_render_frame_id()?;
        let data = crate::render::FrameData {
            frame_id: identity.clone(),
            document_id: input.document_id.clone(),
            width: input.width.clone(),
            height: input.height.clone(),
        };
        if let Some(broken) = data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::render::PaintFrameOutcome::Painted { frame_painted: crate::render::FramePainted { frame_id: identity.clone(), document_id: input.document_id.clone(), width: input.width.clone(), height: input.height.clone() } };
        FrameStorage::put(&mut self.ports, crate::render::AnyFrame::Painted(crate::render::Frame::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `surf.shell.CloseTab`, generated: every outcome is one the specification fully determines.
impl<P> crate::shell::obligations::CloseTabBehavior for Generated<P>
where
    P: TabStorage,
{
    fn close_tab(&mut self, input: crate::shell::CloseTab) -> Result<crate::shell::CloseTabOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if TabStorage::get(&self.ports, &input.tab_id).is_none() {
            return Ok(crate::shell::CloseTabOutcome::NoSuchTab { error: crate::shell::TabNotFound { tab_id: input.tab_id.clone() } });
        }
        // `closed`: the default.
        let Some(held) = TabStorage::get(&self.ports, &input.tab_id) else {
            return Ok(crate::shell::CloseTabOutcome::NoSuchTab { error: crate::shell::TabNotFound { tab_id: input.tab_id.clone() } });
        };
        let _ = &held;
        let held_state = held.state;
        let before = held.data.clone();
        let moved = match held.refine() {
            crate::shell::AnyTab::Open(instance) => crate::shell::AnyTab::Closed(instance.close()),
            _ => return Ok(crate::shell::CloseTabOutcome::WrongState { error: crate::shell::TabStateConflict { state: held_state } }),
        };
        let next = moved.snapshot();
        let answer = crate::shell::CloseTabOutcome::Closed { tab_closed: crate::shell::TabClosed { tab_id: input.tab_id.clone(), window_id: before.window_id.clone() } };
        TabStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `surf.shell.OpenTab`, generated: every outcome is one the specification fully determines.
impl<P> crate::shell::obligations::OpenTabBehavior for Generated<P>
where
    P: TryContext + TabStorage + WindowStorage,
{
    fn open_tab(&mut self, input: crate::shell::OpenTab) -> Result<crate::shell::OpenTabOutcome, UnmetObligation> {
        let _ = &input;
        // `when_related:` reads the `surf.shell.Window` row `input.window_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.window_id);
        let related = reference.and_then(|identity| WindowStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-window`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::shell::OpenTabOutcome::NoSuchWindow { error: crate::shell::WindowNotFound { window_id: input.window_id.clone() } });
        }
        // `opened`: the default.
        let identity: crate::shell::TabId = self.ports.try_generate_surf_shell_tab_id()?;
        let data = crate::shell::TabData {
            tab_id: identity.clone(),
            window_id: input.window_id.clone(),
        };
        let answer = crate::shell::OpenTabOutcome::Opened { tab_opened: crate::shell::TabOpened { tab_id: identity.clone(), window_id: input.window_id.clone() } };
        TabStorage::put(&mut self.ports, crate::shell::AnyTab::Open(crate::shell::Tab::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `surf.shell.OpenWindow`, generated: every outcome is one the specification fully determines.
impl<P> crate::shell::obligations::OpenWindowBehavior for Generated<P>
where
    P: TryContext + WindowStorage,
{
    fn open_window(&mut self, input: crate::shell::OpenWindow) -> Result<crate::shell::OpenWindowOutcome, UnmetObligation> {
        let _ = &input;
        // `opened`: the default.
        let identity: crate::shell::WindowId = self.ports.try_generate_surf_shell_window_id()?;
        let data = crate::shell::WindowData {
            window_id: identity.clone(),
        };
        let answer = crate::shell::OpenWindowOutcome::Opened { window_opened: crate::shell::WindowOpened { window_id: identity.clone() } };
        WindowStorage::put(&mut self.ports, crate::shell::AnyWindow::Open(crate::shell::Window::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `surf.agent.SessionLog`, generated: every row is one the specification fully determines from the stored `surf.agent.Action`s.
impl<P> crate::agent::obligations::SessionLogQuery for Generated<P>
where
    P: ActionStorage,
{
    fn session_log(&self, param_0: crate::agent::SessionId) -> Result<Vec<crate::agent::SessionLog>, UnmetObligation> {
        let mut admitted = ActionStorage::list(&self.ports);
        // `filter:` shows a row where it holds; false or unknown hides it.
        admitted.retain(|held| equal(Some(&held.data.session_id).map(|value| &value.0).map(|value| value.0.clone()), Some(&param_0).map(|value| &value.0).map(|value| value.0.clone())) == Some(true));
        // `order_by:`, stably: rows equal on every key keep the port's order.
        admitted.sort_by(|left, right| number_order(&Some(&left.data.sequence).map(|value| value.to_string()).unwrap_or_default(), &Some(&right.data.sequence).map(|value| value.to_string()).unwrap_or_default()));
        Ok(admitted
            .into_iter()
            .map(|held| crate::agent::SessionLog {
                action_id: held.data.action_id,
                session_id: held.data.session_id,
                sequence: held.data.sequence,
                command: held.data.command,
                url: held.data.url,
            })
            .collect())
    }
}

/// `surf.agent.SessionState`, generated: every row is one the specification fully determines from the stored `surf.agent.Session`s.
impl<P> crate::agent::obligations::SessionStateViewQuery for Generated<P>
where
    P: SessionStorage,
{
    fn session_state_view(&self, param_0: crate::agent::SessionId) -> Result<Vec<crate::agent::SessionStateView>, UnmetObligation> {
        let mut admitted = SessionStorage::list(&self.ports);
        // `filter:` shows a row where it holds; false or unknown hides it.
        admitted.retain(|held| equal(Some(&held.data.session_id).map(|value| &value.0).map(|value| value.0.clone()), Some(&param_0).map(|value| &value.0).map(|value| value.0.clone())) == Some(true));
        Ok(admitted
            .into_iter()
            .map(|held| crate::agent::SessionStateView {
                session_id: held.data.session_id,
                state: held.state,
                url: held.data.url,
                navigation_state: held.data.navigation_state,
                status: held.data.status,
                media_type: held.data.media_type,
                failure_reason: held.data.failure_reason,
            })
            .collect())
    }
}

/// `surf.agent.Sessions`, generated: every row is one the specification fully determines from the stored `surf.agent.Session`s.
impl<P> crate::agent::obligations::SessionsQuery for Generated<P>
where
    P: SessionStorage,
{
    fn sessions(&self) -> Result<Vec<crate::agent::Sessions>, UnmetObligation> {
        let admitted = SessionStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::agent::Sessions {
                session_id: held.data.session_id,
                state: held.state,
                window_id: held.data.window_id,
                tab_id: held.data.tab_id,
                navigation_id: held.data.navigation_id,
                url: held.data.url,
                navigation_state: held.data.navigation_state,
                status: held.data.status,
                media_type: held.data.media_type,
                failure_reason: held.data.failure_reason,
            })
            .collect())
    }
}

/// `surf.document.Documents`, generated: every row is one the specification fully determines from the stored `surf.document.Document`s.
impl<P> crate::document::obligations::DocumentsQuery for Generated<P>
where
    P: DocumentStorage,
{
    fn documents(&self) -> Result<Vec<crate::document::Documents>, UnmetObligation> {
        let admitted = DocumentStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::document::Documents {
                document_id: held.data.document_id,
                url: held.data.url,
                navigation_id: held.data.navigation_id,
                state: held.state,
                title: held.data.title,
                encoding: held.data.encoding,
                unsupported_encoding: held.data.unsupported_encoding,
                stopped_by: held.data.stopped_by,
            })
            .collect())
    }
}

/// `surf.document.Elements`, generated: every row is one the specification fully determines from the stored `surf.document.Element`s.
impl<P> crate::document::obligations::ElementsQuery for Generated<P>
where
    P: ElementStorage,
{
    fn elements(&self, param_0: crate::document::DocumentId) -> Result<Vec<crate::document::Elements>, UnmetObligation> {
        let mut admitted = ElementStorage::list(&self.ports);
        // `filter:` shows a row where it holds; false or unknown hides it.
        admitted.retain(|held| equal(Some(&held.data.document_id).map(|value| &value.0).map(|value| value.0.clone()), Some(&param_0).map(|value| &value.0).map(|value| value.0.clone())) == Some(true));
        // `order_by:`, stably: rows equal on every key keep the port's order.
        admitted.sort_by(|left, right| number_order(&Some(&left.data.position).map(|value| value.to_string()).unwrap_or_default(), &Some(&right.data.position).map(|value| value.to_string()).unwrap_or_default()));
        Ok(admitted
            .into_iter()
            .map(|held| crate::document::Elements {
                element_id: held.data.element_id,
                document_id: held.data.document_id,
                position: held.data.position,
                kind: held.data.kind,
                label: held.data.label,
                target: held.data.target,
                value: held.data.value,
                hidden: held.data.hidden,
                disabled: held.data.disabled,
                checked: held.data.checked,
                required: held.data.required,
            })
            .collect())
    }
}

/// `surf.navigation.Navigations`, generated: every row is one the specification fully determines from the stored `surf.navigation.Navigation`s.
impl<P> crate::navigation::obligations::NavigationsQuery for Generated<P>
where
    P: NavigationStorage,
{
    fn navigations(&self) -> Result<Vec<crate::navigation::Navigations>, UnmetObligation> {
        let admitted = NavigationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::navigation::Navigations {
                navigation_id: held.data.navigation_id,
                tab_id: held.data.tab_id,
                url: held.data.url,
                state: held.state,
            })
            .collect())
    }
}

/// `surf.net.Requests`, generated: every row is one the specification fully determines from the stored `surf.net.Request`s.
impl<P> crate::net::obligations::RequestsQuery for Generated<P>
where
    P: RequestStorage,
{
    fn requests(&self) -> Result<Vec<crate::net::Requests>, UnmetObligation> {
        let admitted = RequestStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::net::Requests {
                request_id: held.data.request_id,
                url: held.data.url,
                method: held.data.method,
                body_type: held.data.body_type,
                state: held.state,
            })
            .collect())
    }
}

/// `surf.render.Frames`, generated: every row is one the specification fully determines from the stored `surf.render.Frame`s.
impl<P> crate::render::obligations::FramesQuery for Generated<P>
where
    P: FrameStorage,
{
    fn frames(&self) -> Result<Vec<crate::render::Frames>, UnmetObligation> {
        let admitted = FrameStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::render::Frames {
                frame_id: held.data.frame_id,
                document_id: held.data.document_id,
                width: held.data.width,
                height: held.data.height,
            })
            .collect())
    }
}

/// `surf.shell.Tabs`, generated: every row is one the specification fully determines from the stored `surf.shell.Tab`s.
impl<P> crate::shell::obligations::TabsQuery for Generated<P>
where
    P: TabStorage,
{
    fn tabs(&self) -> Result<Vec<crate::shell::Tabs>, UnmetObligation> {
        let admitted = TabStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::shell::Tabs {
                tab_id: held.data.tab_id,
                window_id: held.data.window_id,
                state: held.state,
            })
            .collect())
    }
}

/// `surf.shell.Windows`, generated: every row is one the specification fully determines from the stored `surf.shell.Window`s.
impl<P> crate::shell::obligations::WindowsQuery for Generated<P>
where
    P: WindowStorage,
{
    fn windows(&self) -> Result<Vec<crate::shell::Windows>, UnmetObligation> {
        let admitted = WindowStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::shell::Windows {
                window_id: held.data.window_id,
            })
            .collect())
    }
}

/// The typed refusal of a request the model declares no outcome for.
fn undeclared(source: &'static str) -> UnmetObligation {
    let capability = "command behaviour";
    UnmetObligation { capability, source }
}

/// A guard's truth, where it has one: Unknown selects no branch, so the model declares no outcome.
fn decided(truth: Option<bool>, command: &'static str) -> Result<bool, UnmetObligation> {
    truth.ok_or_else(|| undeclared(command))
}

/// Three-valued disjunction: true wins, then Unknown.
fn any(truths: &[Option<bool>]) -> Option<bool> {
    if truths.contains(&Some(true)) {
        Some(true)
    } else if truths.contains(&None) {
        None
    } else {
        Some(false)
    }
}

/// Equality of two read values; an unread one is Unknown.
fn equal<T: PartialEq>(left: Option<T>, right: Option<T>) -> Option<bool> {
    Some(left? == right?)
}

/// A decimal rendering as its sign, its whole digits and its fraction digits, without the zeros
/// that do not change its value; `None` where it is not a plain decimal.
fn number_parts(text: &str) -> Option<(bool, String, String)> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    if !whole.bytes().chain(fraction.bytes()).all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let whole = whole.trim_start_matches('0').to_owned();
    let fraction = fraction.trim_end_matches('0').to_owned();
    let zero = whole.is_empty() && fraction.is_empty();
    Some((negative && !zero, whole, fraction))
}

/// Compares two decimal renderings exactly; an unread or unparsable one is Unknown.
fn compare_numbers(
    left: Option<String>,
    right: Option<String>,
    accepts: fn(core::cmp::Ordering) -> bool,
) -> Option<bool> {
    let (left, right) = (number_parts(&left?)?, number_parts(&right?)?);
    let magnitude = left
        .1
        .len()
        .cmp(&right.1.len())
        .then_with(|| left.1.cmp(&right.1))
        .then_with(|| left.2.cmp(&right.2));
    let ordering = match (left.0, right.0) {
        (false, false) => magnitude,
        (true, true) => magnitude.reverse(),
        (true, false) => core::cmp::Ordering::Less,
        (false, true) => core::cmp::Ordering::Greater,
    };
    Some(accepts(ordering))
}

/// Two decimal renderings by their exact value; text that is no number by its bytes.
fn number_order(left: &str, right: &str) -> core::cmp::Ordering {
    let (Some(left), Some(right)) = (number_parts(left), number_parts(right)) else {
        return left.as_bytes().cmp(right.as_bytes());
    };
    let magnitude = left
        .1
        .len()
        .cmp(&right.1.len())
        .then_with(|| left.1.cmp(&right.1))
        .then_with(|| left.2.cmp(&right.2));
    match (left.0, right.0) {
        (false, false) => magnitude,
        (true, true) => magnitude.reverse(),
        (true, false) => core::cmp::Ordering::Less,
        (false, true) => core::cmp::Ordering::Greater,
    }
}
