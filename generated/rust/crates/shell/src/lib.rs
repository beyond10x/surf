// generated from surf v1
// contract digest 8248095f602196a66470973ac0d75774fbdde611c3d1aa5b65ede54d807a7673
// do not edit: regenerate with `ess synthesize`

//! shell — the `shell` component of `surf` v1.
//!
//! The windows and tabs the user sees and acts on.
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
    /// `surf.shell.TabClosed`.
    TabClosed(surf_types::shell::TabClosed),
    /// `surf.shell.TabOpened`.
    TabOpened(surf_types::shell::TabOpened),
    /// `surf.shell.WindowOpened`.
    WindowOpened(surf_types::shell::WindowOpened),
}

/// shell — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct Shell<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> Shell<B> {
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

impl<B> Shell<B>
where
    B: surf_types::shell::obligations::CloseTabBehavior + surf_types::shell::obligations::OpenTabBehavior + surf_types::shell::obligations::OpenWindowBehavior + surf_types::shell::obligations::TabsQuery + surf_types::shell::obligations::WindowsQuery,
{
    /// Accepts `surf.shell.CloseTab`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn close_tab(&mut self, input: surf_types::shell::CloseTab) -> Result<surf_types::shell::CloseTabOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.close_tab(input)?;
        match &outcome {
            surf_types::shell::CloseTabOutcome::NoSuchTab { .. } => {}
            surf_types::shell::CloseTabOutcome::Closed { tab_closed, .. } => {
                self.outbox.push(PublishedEvent::TabClosed(tab_closed.clone()));
            }
            surf_types::shell::CloseTabOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `surf.shell.OpenTab`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn open_tab(&mut self, input: surf_types::shell::OpenTab) -> Result<surf_types::shell::OpenTabOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.open_tab(input)?;
        match &outcome {
            surf_types::shell::OpenTabOutcome::NoSuchWindow { .. } => {}
            surf_types::shell::OpenTabOutcome::Opened { tab_opened, .. } => {
                self.outbox.push(PublishedEvent::TabOpened(tab_opened.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.shell.OpenWindow`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn open_window(&mut self, input: surf_types::shell::OpenWindow) -> Result<surf_types::shell::OpenWindowOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.open_window(input)?;
        match &outcome {
            surf_types::shell::OpenWindowOutcome::Opened { window_opened, .. } => {
                self.outbox.push(PublishedEvent::WindowOpened(window_opened.clone()));
            }
        }
        Ok(outcome)
    }

    /// Serves `surf.shell.Tabs` at `read_your_writes` consistency, from the owed projection.
    pub fn tabs(&self) -> Result<Vec<surf_types::shell::Tabs>, surf_types::obligation::UnmetObligation> {
        self.behaviors.tabs()
    }

    /// Serves `surf.shell.Windows` at `read_your_writes` consistency, from the owed projection.
    pub fn windows(&self) -> Result<Vec<surf_types::shell::Windows>, surf_types::obligation::UnmetObligation> {
        self.behaviors.windows()
    }
}
