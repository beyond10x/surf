// generated from surf v1
// contract digest d86950609f0648d1fd4f49718d20e544490bed20fa1a6c742579023bd4f50f16
// do not edit: regenerate with `ess synthesize`

//! navigator — the `navigator` component of `surf` v1.
//!
//! Decides what each tab loads and tracks every load to its end.
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
    /// `surf.navigation.NavigationCancelled`.
    NavigationCancelled(surf_types::navigation::NavigationCancelled),
    /// `surf.navigation.NavigationCommitted`.
    NavigationCommitted(surf_types::navigation::NavigationCommitted),
    /// `surf.navigation.NavigationFailed`.
    NavigationFailed(surf_types::navigation::NavigationFailed),
    /// `surf.navigation.NavigationStarted`.
    NavigationStarted(surf_types::navigation::NavigationStarted),
}

/// navigator — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct Navigator<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> Navigator<B> {
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

impl<B> Navigator<B>
where
    B: surf_types::navigation::obligations::CancelNavigationBehavior + surf_types::navigation::obligations::CommitNavigationBehavior + surf_types::navigation::obligations::FailNavigationBehavior + surf_types::navigation::obligations::NavigateBehavior + surf_types::navigation::obligations::NavigationsQuery,
{
    /// Accepts `surf.navigation.CancelNavigation`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn cancel_navigation(&mut self, input: surf_types::navigation::CancelNavigation) -> Result<surf_types::navigation::CancelNavigationOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.cancel_navigation(input)?;
        match &outcome {
            surf_types::navigation::CancelNavigationOutcome::NoSuchNavigation { .. } => {}
            surf_types::navigation::CancelNavigationOutcome::Cancelled { navigation_cancelled, .. } => {
                self.outbox.push(PublishedEvent::NavigationCancelled(navigation_cancelled.clone()));
            }
            surf_types::navigation::CancelNavigationOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `surf.navigation.CommitNavigation`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn commit_navigation(&mut self, input: surf_types::navigation::CommitNavigation) -> Result<surf_types::navigation::CommitNavigationOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.commit_navigation(input)?;
        match &outcome {
            surf_types::navigation::CommitNavigationOutcome::NoSuchNavigation { .. } => {}
            surf_types::navigation::CommitNavigationOutcome::Committed { navigation_committed, .. } => {
                self.outbox.push(PublishedEvent::NavigationCommitted(navigation_committed.clone()));
            }
            surf_types::navigation::CommitNavigationOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `surf.navigation.FailNavigation`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn fail_navigation(&mut self, input: surf_types::navigation::FailNavigation) -> Result<surf_types::navigation::FailNavigationOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.fail_navigation(input)?;
        match &outcome {
            surf_types::navigation::FailNavigationOutcome::NoSuchNavigation { .. } => {}
            surf_types::navigation::FailNavigationOutcome::Failed { navigation_failed, .. } => {
                self.outbox.push(PublishedEvent::NavigationFailed(navigation_failed.clone()));
            }
            surf_types::navigation::FailNavigationOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `surf.navigation.Navigate`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn navigate(&mut self, input: surf_types::navigation::Navigate) -> Result<surf_types::navigation::NavigateOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.navigate(input)?;
        match &outcome {
            surf_types::navigation::NavigateOutcome::Started { navigation_started, .. } => {
                self.outbox.push(PublishedEvent::NavigationStarted(navigation_started.clone()));
            }
        }
        Ok(outcome)
    }

    /// Serves `surf.navigation.Navigations` at `read_your_writes` consistency, from the owed projection.
    pub fn navigations(&self) -> Result<Vec<surf_types::navigation::Navigations>, surf_types::obligation::UnmetObligation> {
        self.behaviors.navigations()
    }
}
