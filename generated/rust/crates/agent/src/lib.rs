// generated from surf v1
// contract digest a781938fbf4be70c3e62196db4afd2b86d2f53f3955885c5d3e606169c3ba178
// do not edit: regenerate with `ess synthesize`

//! agent — the `agent` component of `surf` v1.
//!
//! The sessions an AI agent opens and drives from the command line.
//!
//! The component's outer surface exactly as the specification declares it: accepted commands as
//! handlers, declared views as queries, published events as a typed outbox. The behaviour behind
//! every handler is an implementation obligation — see the `PLAN.md` beside this workspace — and
//! until one is satisfied, its stub answers with a typed refusal naming what is owed.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// An event this component declares it publishes, on its way to the system's transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishedEvent {
    /// `surf.agent.ActionRecorded`.
    ActionRecorded(surf_types::agent::ActionRecorded),
    /// `surf.agent.FillRequested`.
    FillRequested(surf_types::agent::FillRequested),
    /// `surf.agent.NavigationRequested`.
    NavigationRequested(surf_types::agent::NavigationRequested),
    /// `surf.agent.PickRequested`.
    PickRequested(surf_types::agent::PickRequested),
    /// `surf.agent.PressRequested`.
    PressRequested(surf_types::agent::PressRequested),
    /// `surf.agent.SessionClosed`.
    SessionClosed(surf_types::agent::SessionClosed),
    /// `surf.agent.SessionNavigationChanged`.
    SessionNavigationChanged(surf_types::agent::SessionNavigationChanged),
    /// `surf.agent.SessionOpened`.
    SessionOpened(surf_types::agent::SessionOpened),
}

/// agent — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct Agent<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> Agent<B> {
    /// A new port over the given obligation implementations.
    pub fn new(behaviors: B) -> Self {
        Self {
            behaviors,
            outbox: Vec::new(),
        }
    }

    /// Hands over everything published since the last drain, in publication order.
    ///
    /// The system's transport calls this; anything else reading it is taking events the transport
    /// will then never deliver.
    pub fn drain_outbox(&mut self) -> Vec<PublishedEvent> {
        core::mem::take(&mut self.outbox)
    }
}

impl<B> Agent<B>
where
    B: surf_types::agent::obligations::CloseSessionBehavior + surf_types::agent::obligations::FillBehavior + surf_types::agent::obligations::FollowBehavior + surf_types::agent::obligations::GoBehavior + surf_types::agent::obligations::NoteNavigationCommittedBehavior + surf_types::agent::obligations::NoteNavigationEndedBehavior + surf_types::agent::obligations::NoteNavigationFailedBehavior + surf_types::agent::obligations::NoteNavigationStartedBehavior + surf_types::agent::obligations::OpenSessionBehavior + surf_types::agent::obligations::PickBehavior + surf_types::agent::obligations::PressBehavior + surf_types::agent::obligations::RecordActionBehavior + surf_types::agent::obligations::SessionLogQuery + surf_types::agent::obligations::SessionStateViewQuery + surf_types::agent::obligations::SessionsQuery,
{
    /// Accepts `surf.agent.CloseSession`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn close_session(&mut self, input: surf_types::agent::CloseSession) -> Result<surf_types::agent::CloseSessionOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.close_session(input)?;
        match &outcome {
            surf_types::agent::CloseSessionOutcome::NotHolder { .. } => {}
            surf_types::agent::CloseSessionOutcome::NoSuchSession { .. } => {}
            surf_types::agent::CloseSessionOutcome::Closed { session_closed, .. } => {
                self.outbox.push(PublishedEvent::SessionClosed(session_closed.clone()));
            }
            surf_types::agent::CloseSessionOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `surf.agent.Fill`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn fill(&mut self, input: surf_types::agent::Fill) -> Result<surf_types::agent::FillOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.fill(input)?;
        match &outcome {
            surf_types::agent::FillOutcome::NotHolder { .. } => {}
            surf_types::agent::FillOutcome::NoSuchSession { .. } => {}
            surf_types::agent::FillOutcome::Closed { .. } => {}
            surf_types::agent::FillOutcome::Requested { fill_requested, .. } => {
                self.outbox.push(PublishedEvent::FillRequested(fill_requested.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.agent.Follow`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn follow(&mut self, input: surf_types::agent::Follow) -> Result<surf_types::agent::FollowOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.follow(input)?;
        match &outcome {
            surf_types::agent::FollowOutcome::NotHolder { .. } => {}
            surf_types::agent::FollowOutcome::NoSuchSession { .. } => {}
            surf_types::agent::FollowOutcome::Closed { .. } => {}
            surf_types::agent::FollowOutcome::Requested { navigation_requested, .. } => {
                self.outbox.push(PublishedEvent::NavigationRequested(navigation_requested.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.agent.Go`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn go(&mut self, input: surf_types::agent::Go) -> Result<surf_types::agent::GoOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.go(input)?;
        match &outcome {
            surf_types::agent::GoOutcome::NotHolder { .. } => {}
            surf_types::agent::GoOutcome::NoSuchSession { .. } => {}
            surf_types::agent::GoOutcome::Closed { .. } => {}
            surf_types::agent::GoOutcome::Requested { navigation_requested, .. } => {
                self.outbox.push(PublishedEvent::NavigationRequested(navigation_requested.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.agent.NoteNavigationCommitted`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn note_navigation_committed(&mut self, input: surf_types::agent::NoteNavigationCommitted) -> Result<surf_types::agent::NoteNavigationCommittedOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.note_navigation_committed(input)?;
        match &outcome {
            surf_types::agent::NoteNavigationCommittedOutcome::Noted { session_navigation_changed, .. } => {
                self.outbox.push(PublishedEvent::SessionNavigationChanged(session_navigation_changed.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.agent.NoteNavigationEnded`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn note_navigation_ended(&mut self, input: surf_types::agent::NoteNavigationEnded) -> Result<surf_types::agent::NoteNavigationEndedOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.note_navigation_ended(input)?;
        match &outcome {
            surf_types::agent::NoteNavigationEndedOutcome::Noted { session_navigation_changed, .. } => {
                self.outbox.push(PublishedEvent::SessionNavigationChanged(session_navigation_changed.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.agent.NoteNavigationFailed`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn note_navigation_failed(&mut self, input: surf_types::agent::NoteNavigationFailed) -> Result<surf_types::agent::NoteNavigationFailedOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.note_navigation_failed(input)?;
        match &outcome {
            surf_types::agent::NoteNavigationFailedOutcome::Noted { session_navigation_changed, .. } => {
                self.outbox.push(PublishedEvent::SessionNavigationChanged(session_navigation_changed.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.agent.NoteNavigationStarted`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn note_navigation_started(&mut self, input: surf_types::agent::NoteNavigationStarted) -> Result<surf_types::agent::NoteNavigationStartedOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.note_navigation_started(input)?;
        match &outcome {
            surf_types::agent::NoteNavigationStartedOutcome::Noted { session_navigation_changed, .. } => {
                self.outbox.push(PublishedEvent::SessionNavigationChanged(session_navigation_changed.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.agent.OpenSession`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn open_session(&mut self, input: surf_types::agent::OpenSession) -> Result<surf_types::agent::OpenSessionOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.open_session(input)?;
        match &outcome {
            surf_types::agent::OpenSessionOutcome::Opened { session_opened, .. } => {
                self.outbox.push(PublishedEvent::SessionOpened(session_opened.clone()));
            }
            surf_types::agent::OpenSessionOutcome::Exists { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `surf.agent.Pick`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn pick(&mut self, input: surf_types::agent::Pick) -> Result<surf_types::agent::PickOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.pick(input)?;
        match &outcome {
            surf_types::agent::PickOutcome::NotHolder { .. } => {}
            surf_types::agent::PickOutcome::NoSuchSession { .. } => {}
            surf_types::agent::PickOutcome::Closed { .. } => {}
            surf_types::agent::PickOutcome::Requested { pick_requested, .. } => {
                self.outbox.push(PublishedEvent::PickRequested(pick_requested.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.agent.Press`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn press(&mut self, input: surf_types::agent::Press) -> Result<surf_types::agent::PressOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.press(input)?;
        match &outcome {
            surf_types::agent::PressOutcome::NotHolder { .. } => {}
            surf_types::agent::PressOutcome::NoSuchSession { .. } => {}
            surf_types::agent::PressOutcome::Closed { .. } => {}
            surf_types::agent::PressOutcome::Requested { press_requested, .. } => {
                self.outbox.push(PublishedEvent::PressRequested(press_requested.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.agent.RecordAction`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_action(&mut self, input: surf_types::agent::RecordAction) -> Result<surf_types::agent::RecordActionOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_action(input)?;
        match &outcome {
            surf_types::agent::RecordActionOutcome::NoSuchSession { .. } => {}
            surf_types::agent::RecordActionOutcome::Recorded { action_recorded, .. } => {
                self.outbox.push(PublishedEvent::ActionRecorded(action_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Serves `surf.agent.SessionLog` at `read_your_writes` consistency, from the owed projection.
    pub fn session_log(&self, session_id: surf_types::agent::SessionId) -> Result<Vec<surf_types::agent::SessionLog>, surf_types::obligation::UnmetObligation> {
        self.behaviors.session_log(session_id)
    }

    /// Serves `surf.agent.SessionState` at `read_your_writes` consistency, from the owed projection.
    pub fn session_state_view(&self, session_id: surf_types::agent::SessionId) -> Result<Vec<surf_types::agent::SessionStateView>, surf_types::obligation::UnmetObligation> {
        self.behaviors.session_state_view(session_id)
    }

    /// Serves `surf.agent.Sessions` at `read_your_writes` consistency, from the owed projection.
    pub fn sessions(&self) -> Result<Vec<surf_types::agent::Sessions>, surf_types::obligation::UnmetObligation> {
        self.behaviors.sessions()
    }
}
