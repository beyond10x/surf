// generated from surf v1
// contract digest d02edabfb4604fe93f72d6e24ef0d1805df03b1628561d9011c4f5f33d76c73d
// do not edit: regenerate with `ess synthesize`

//! The `surf` system, v1: its components assembled, its bindings wired, and its one transport.
//!
//! The transport is derived from the specification, not chosen: this model declares more
//! than one delivery guarantee, and the pump gives each reacting binding one attempt at each
//! published event — which is what `at_most_once` requires and what `at_least_once`
//! permits — attempting it again only for the one binding that asked. Published events land
//! on an append-only log, which is the system's observable record, and so is the
//! record of what each binding invoked. What no specification
//! determines — how an escalation event is filled, behaviour behind the ports — stays an
//! obligation; see the `PLAN.md` beside this workspace.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// An event on the system's log: everything any component publishes, and everything a binding
/// escalates into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemEvent {
    /// `surf.agent.ActionRecorded`.
    ActionRecorded(surf_types::agent::ActionRecorded),
    /// `surf.agent.NavigationRequested`.
    NavigationRequested(surf_types::agent::NavigationRequested),
    /// `surf.agent.SessionClosed`.
    SessionClosed(surf_types::agent::SessionClosed),
    /// `surf.agent.SessionNavigationChanged`.
    SessionNavigationChanged(surf_types::agent::SessionNavigationChanged),
    /// `surf.agent.SessionOpened`.
    SessionOpened(surf_types::agent::SessionOpened),
    /// `surf.document.DocumentParsed`.
    DocumentParsed(surf_types::document::DocumentParsed),
    /// `surf.document.ElementAdded`.
    ElementAdded(surf_types::document::ElementAdded),
    /// `surf.document.ParsingStarted`.
    ParsingStarted(surf_types::document::ParsingStarted),
    /// `surf.navigation.NavigationCancelled`.
    NavigationCancelled(surf_types::navigation::NavigationCancelled),
    /// `surf.navigation.NavigationCommitted`.
    NavigationCommitted(surf_types::navigation::NavigationCommitted),
    /// `surf.navigation.NavigationFailed`.
    NavigationFailed(surf_types::navigation::NavigationFailed),
    /// `surf.navigation.NavigationStarted`.
    NavigationStarted(surf_types::navigation::NavigationStarted),
    /// `surf.net.FetchFailed`.
    FetchFailed(surf_types::net::FetchFailed),
    /// `surf.net.RequestStarted`.
    RequestStarted(surf_types::net::RequestStarted),
    /// `surf.net.ResponseReceived`.
    ResponseReceived(surf_types::net::ResponseReceived),
    /// `surf.render.FramePainted`.
    FramePainted(surf_types::render::FramePainted),
    /// `surf.shell.TabClosed`.
    TabClosed(surf_types::shell::TabClosed),
    /// `surf.shell.TabOpened`.
    TabOpened(surf_types::shell::TabOpened),
    /// `surf.shell.WindowOpened`.
    WindowOpened(surf_types::shell::WindowOpened),
}

impl SystemEvent {
    /// The qualified name the specification declares this event under.
    pub fn name(&self) -> &'static str {
        match self {
            Self::ActionRecorded(_) => "surf.agent.ActionRecorded",
            Self::NavigationRequested(_) => "surf.agent.NavigationRequested",
            Self::SessionClosed(_) => "surf.agent.SessionClosed",
            Self::SessionNavigationChanged(_) => "surf.agent.SessionNavigationChanged",
            Self::SessionOpened(_) => "surf.agent.SessionOpened",
            Self::DocumentParsed(_) => "surf.document.DocumentParsed",
            Self::ElementAdded(_) => "surf.document.ElementAdded",
            Self::ParsingStarted(_) => "surf.document.ParsingStarted",
            Self::NavigationCancelled(_) => "surf.navigation.NavigationCancelled",
            Self::NavigationCommitted(_) => "surf.navigation.NavigationCommitted",
            Self::NavigationFailed(_) => "surf.navigation.NavigationFailed",
            Self::NavigationStarted(_) => "surf.navigation.NavigationStarted",
            Self::FetchFailed(_) => "surf.net.FetchFailed",
            Self::RequestStarted(_) => "surf.net.RequestStarted",
            Self::ResponseReceived(_) => "surf.net.ResponseReceived",
            Self::FramePainted(_) => "surf.render.FramePainted",
            Self::TabClosed(_) => "surf.shell.TabClosed",
            Self::TabOpened(_) => "surf.shell.TabOpened",
            Self::WindowOpened(_) => "surf.shell.WindowOpened",
        }
    }
}

impl From<agent::PublishedEvent> for SystemEvent {
    fn from(event: agent::PublishedEvent) -> Self {
        match event {
            agent::PublishedEvent::ActionRecorded(event) => Self::ActionRecorded(event),
            agent::PublishedEvent::NavigationRequested(event) => Self::NavigationRequested(event),
            agent::PublishedEvent::SessionClosed(event) => Self::SessionClosed(event),
            agent::PublishedEvent::SessionNavigationChanged(event) => Self::SessionNavigationChanged(event),
            agent::PublishedEvent::SessionOpened(event) => Self::SessionOpened(event),
        }
    }
}

impl From<document::PublishedEvent> for SystemEvent {
    fn from(event: document::PublishedEvent) -> Self {
        match event {
            document::PublishedEvent::DocumentParsed(event) => Self::DocumentParsed(event),
            document::PublishedEvent::ElementAdded(event) => Self::ElementAdded(event),
            document::PublishedEvent::ParsingStarted(event) => Self::ParsingStarted(event),
        }
    }
}

impl From<navigator::PublishedEvent> for SystemEvent {
    fn from(event: navigator::PublishedEvent) -> Self {
        match event {
            navigator::PublishedEvent::NavigationCancelled(event) => Self::NavigationCancelled(event),
            navigator::PublishedEvent::NavigationCommitted(event) => Self::NavigationCommitted(event),
            navigator::PublishedEvent::NavigationFailed(event) => Self::NavigationFailed(event),
            navigator::PublishedEvent::NavigationStarted(event) => Self::NavigationStarted(event),
        }
    }
}

impl From<net::PublishedEvent> for SystemEvent {
    fn from(event: net::PublishedEvent) -> Self {
        match event {
            net::PublishedEvent::FetchFailed(event) => Self::FetchFailed(event),
            net::PublishedEvent::RequestStarted(event) => Self::RequestStarted(event),
            net::PublishedEvent::ResponseReceived(event) => Self::ResponseReceived(event),
        }
    }
}

impl From<renderer::PublishedEvent> for SystemEvent {
    fn from(event: renderer::PublishedEvent) -> Self {
        match event {
            renderer::PublishedEvent::FramePainted(event) => Self::FramePainted(event),
        }
    }
}

impl From<shell::PublishedEvent> for SystemEvent {
    fn from(event: shell::PublishedEvent) -> Self {
        match event {
            shell::PublishedEvent::TabClosed(event) => Self::TabClosed(event),
            shell::PublishedEvent::TabOpened(event) => Self::TabOpened(event),
            shell::PublishedEvent::WindowOpened(event) => Self::WindowOpened(event),
        }
    }
}

/// One command a binding invoked, and the input it passed — the transport's own record.
///
/// Recorded by the pump at the moment of invocation, so what a binding actually passed is
/// observable from outside — a conformance run holds a mapping to its words with exactly this —
/// without instrumenting the component underneath.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingInvocation {
    /// `close-tab-on-session-closed` invoked `surf.shell.CloseTab`.
    CloseTabOnSessionClosed(surf_types::shell::CloseTab),
    /// `commit-on-response` invoked `surf.navigation.CommitNavigation`.
    CommitOnResponse(surf_types::navigation::CommitNavigation),
    /// `fail-on-fetch-failed` invoked `surf.navigation.FailNavigation`.
    FailOnFetchFailed(surf_types::navigation::FailNavigation),
    /// `fetch-on-navigation-started` invoked `surf.net.Fetch`.
    FetchOnNavigationStarted(surf_types::net::Fetch),
    /// `navigate-on-navigation-requested` invoked `surf.navigation.Navigate`.
    NavigateOnNavigationRequested(surf_types::navigation::Navigate),
    /// `note-on-navigation-cancelled` invoked `surf.agent.NoteNavigationEnded`.
    NoteOnNavigationCancelled(surf_types::agent::NoteNavigationEnded),
    /// `note-on-navigation-committed` invoked `surf.agent.NoteNavigationCommitted`.
    NoteOnNavigationCommitted(surf_types::agent::NoteNavigationCommitted),
    /// `note-on-navigation-failed` invoked `surf.agent.NoteNavigationFailed`.
    NoteOnNavigationFailed(surf_types::agent::NoteNavigationFailed),
    /// `note-on-navigation-started` invoked `surf.agent.NoteNavigationStarted`.
    NoteOnNavigationStarted(surf_types::agent::NoteNavigationStarted),
    /// `parse-on-navigation-committed` invoked `surf.document.ParseDocument`.
    ParseOnNavigationCommitted(surf_types::document::ParseDocument),
}

/// The binding `close-tab-on-session-closed`: `surf.agent.SessionClosed`, read as `surf.shell.CloseTab` input.
///
/// Fully determined by the specification: every input is filled from an event field — through the
/// declared crossing where one is named — from a literal the target admits, or left absent
/// where the input is optional and the binding says nothing.
pub fn close_tab_on_session_closed(event: &surf_types::agent::SessionClosed) -> surf_types::shell::CloseTab {
    surf_types::shell::CloseTab {
        tab_id: event.tab_id.clone(),
    }
}

/// The binding `commit-on-response`: `surf.net.ResponseReceived`, read as `surf.navigation.CommitNavigation` input.
///
/// Fully determined by the specification: every input is filled from an event field — through the
/// declared crossing where one is named — from a literal the target admits, or left absent
/// where the input is optional and the binding says nothing.
pub fn commit_on_response(event: &surf_types::net::ResponseReceived) -> surf_types::navigation::CommitNavigation {
    surf_types::navigation::CommitNavigation {
        navigation_id: event.request_id.clone(),
        url: event.url.clone(),
        status: event.status.clone(),
        media_type: event.media_type.clone(),
        body: event.body.clone(),
    }
}

/// The binding `fail-on-fetch-failed`: `surf.net.FetchFailed`, read as `surf.navigation.FailNavigation` input.
///
/// Fully determined by the specification: every input is filled from an event field — through the
/// declared crossing where one is named — from a literal the target admits, or left absent
/// where the input is optional and the binding says nothing.
pub fn fail_on_fetch_failed(event: &surf_types::net::FetchFailed) -> surf_types::navigation::FailNavigation {
    surf_types::navigation::FailNavigation {
        navigation_id: event.request_id.clone(),
        reason: event.reason.clone(),
    }
}

/// The binding `fetch-on-navigation-started`: `surf.navigation.NavigationStarted`, read as `surf.net.Fetch` input.
///
/// Fully determined by the specification: every input is filled from an event field — through the
/// declared crossing where one is named — from a literal the target admits, or left absent
/// where the input is optional and the binding says nothing.
pub fn fetch_on_navigation_started(event: &surf_types::navigation::NavigationStarted) -> surf_types::net::Fetch {
    surf_types::net::Fetch {
        request_id: event.navigation_id.clone(),
        url: event.url.clone(),
        method: event.method.clone(),
        body: event.body.clone(),
        body_type: event.body_type.clone(),
    }
}

/// The binding `navigate-on-navigation-requested`: `surf.agent.NavigationRequested`, read as `surf.navigation.Navigate` input.
///
/// Fully determined by the specification: every input is filled from an event field — through the
/// declared crossing where one is named — from a literal the target admits, or left absent
/// where the input is optional and the binding says nothing.
pub fn navigate_on_navigation_requested(event: &surf_types::agent::NavigationRequested) -> surf_types::navigation::Navigate {
    surf_types::navigation::Navigate {
        tab_id: event.tab_id.clone(),
        url: event.url.clone(),
        method: surf_types::net::Method::Get,
        body: None,
        body_type: None,
    }
}

/// The binding `note-on-navigation-cancelled`: `surf.navigation.NavigationCancelled`, read as `surf.agent.NoteNavigationEnded` input.
///
/// Fully determined by the specification: every input is filled from an event field — through the
/// declared crossing where one is named — from a literal the target admits, or left absent
/// where the input is optional and the binding says nothing.
pub fn note_on_navigation_cancelled(event: &surf_types::navigation::NavigationCancelled) -> surf_types::agent::NoteNavigationEnded {
    surf_types::agent::NoteNavigationEnded {
        navigation_id: event.navigation_id.clone(),
        state: surf_types::navigation::NavigationState::Cancelled,
    }
}

/// The binding `note-on-navigation-committed`: `surf.navigation.NavigationCommitted`, read as `surf.agent.NoteNavigationCommitted` input.
///
/// Fully determined by the specification: every input is filled from an event field — through the
/// declared crossing where one is named — from a literal the target admits, or left absent
/// where the input is optional and the binding says nothing.
pub fn note_on_navigation_committed(event: &surf_types::navigation::NavigationCommitted) -> surf_types::agent::NoteNavigationCommitted {
    surf_types::agent::NoteNavigationCommitted {
        navigation_id: event.navigation_id.clone(),
        url: event.url.clone(),
        status: event.status.clone(),
        media_type: event.media_type.clone(),
    }
}

/// The binding `note-on-navigation-failed`: `surf.navigation.NavigationFailed`, read as `surf.agent.NoteNavigationFailed` input.
///
/// Fully determined by the specification: every input is filled from an event field — through the
/// declared crossing where one is named — from a literal the target admits, or left absent
/// where the input is optional and the binding says nothing.
pub fn note_on_navigation_failed(event: &surf_types::navigation::NavigationFailed) -> surf_types::agent::NoteNavigationFailed {
    surf_types::agent::NoteNavigationFailed {
        navigation_id: event.navigation_id.clone(),
        reason: event.reason.clone(),
    }
}

/// The binding `note-on-navigation-started`: `surf.navigation.NavigationStarted`, read as `surf.agent.NoteNavigationStarted` input.
///
/// Fully determined by the specification: every input is filled from an event field — through the
/// declared crossing where one is named — from a literal the target admits, or left absent
/// where the input is optional and the binding says nothing.
pub fn note_on_navigation_started(event: &surf_types::navigation::NavigationStarted) -> surf_types::agent::NoteNavigationStarted {
    surf_types::agent::NoteNavigationStarted {
        tab_id: event.tab_id.clone(),
        navigation_id: event.navigation_id.clone(),
        url: event.url.clone(),
    }
}

/// The binding `parse-on-navigation-committed`: `surf.navigation.NavigationCommitted`, read as `surf.document.ParseDocument` input.
///
/// Fully determined by the specification: every input is filled from an event field — through the
/// declared crossing where one is named — from a literal the target admits, or left absent
/// where the input is optional and the binding says nothing.
pub fn parse_on_navigation_committed(event: &surf_types::navigation::NavigationCommitted) -> surf_types::document::ParseDocument {
    surf_types::document::ParseDocument {
        url: event.url.clone(),
        body: event.body.clone(),
        navigation_id: event.navigation_id.clone(),
    }
}

/// Every binding's event-payload condition (ess/22), evaluated before its transformation, its
/// invocation record and its port, as the specification orders them.
pub mod conditions {
    /// The condition of `parse-on-navigation-committed`: `event.media_type == text/html` over the `surf.navigation.NavigationCommitted` it reacts to.
    ///
    /// `Some(true)` invokes, `Some(false)` skips this binding alone, and `None` — a comparison
    /// reading an absent member — is Unknown: this binding's unmet obligation, never a skip.
    pub fn parse_on_navigation_committed(event: &surf_types::navigation::NavigationCommitted) -> Option<bool> {
        let _ = event;
        Some(&event.media_type).map(|value| &value.0).map(|value| value.as_str() == "text/html")
    }
}

/// The `surf` system: every component behind its port, and the transport between them.
///
/// The component fields are public because commands enter the system through a component's own
/// port; the log and its delivery cursor are not, because publishing happens by pumping, not by
/// writing history directly.
pub struct System<AgentBehaviors, DocumentBehaviors, NavigatorBehaviors, NetBehaviors, RendererBehaviors, ShellBehaviors> {
    /// The `agent` component.
    pub agent: agent::Agent<AgentBehaviors>,
    /// The `document` component.
    pub document: document::Document<DocumentBehaviors>,
    /// The `navigator` component.
    pub navigator: navigator::Navigator<NavigatorBehaviors>,
    /// The `net` component.
    pub net: net::Net<NetBehaviors>,
    /// The `renderer` component.
    pub renderer: renderer::Renderer<RendererBehaviors>,
    /// The `shell` component.
    pub shell: shell::Shell<ShellBehaviors>,
    invocations: Vec<BindingInvocation>,
    published: Vec<SystemEvent>,
    cursor: usize,
}

impl<AgentBehaviors, DocumentBehaviors, NavigatorBehaviors, NetBehaviors, RendererBehaviors, ShellBehaviors> System<AgentBehaviors, DocumentBehaviors, NavigatorBehaviors, NetBehaviors, RendererBehaviors, ShellBehaviors> {
    /// Assembles the system from its components.
    pub fn new(agent: agent::Agent<AgentBehaviors>, document: document::Document<DocumentBehaviors>, navigator: navigator::Navigator<NavigatorBehaviors>, net: net::Net<NetBehaviors>, renderer: renderer::Renderer<RendererBehaviors>, shell: shell::Shell<ShellBehaviors>) -> Self {
        Self {
            agent,
            document,
            navigator,
            net,
            renderer,
            shell,
            invocations: Vec::new(),
            published: Vec::new(),
            cursor: 0,
        }
    }

    /// Everything published so far, in publication order — the system's observable record.
    pub fn published(&self) -> &[SystemEvent] {
        &self.published
    }

    /// Takes every event the pump has already delivered off the log, in publication order.
    ///
    /// A long-running shell calls this after each `pump`, or the log holds every event the
    /// process ever published. A `pump` returns with every logged event delivered: each
    /// reacting binding has had its attempt, and a binding whose attempt stopped holds the event in
    /// its own held-back list, not on the log. Events published since the last `pump` stay on the
    /// log, so the next `pump` still delivers them; taking never skips a binding.
    pub fn take_published(&mut self) -> Vec<SystemEvent> {
        let delivered: Vec<SystemEvent> = self.published.drain(..self.cursor).collect();
        self.cursor = 0;
        delivered
    }

    /// Every command a binding invoked so far, in invocation order, with what it passed.
    pub fn invocations(&self) -> &[BindingInvocation] {
        &self.invocations
    }

    /// Takes the record of every command a binding invoked so far, in invocation order.
    ///
    /// The record's counterpart to [`System::take_published`], for the same long-running shell.
    pub fn take_invocations(&mut self) -> Vec<BindingInvocation> {
        core::mem::take(&mut self.invocations)
    }
}

impl<AgentBehaviors, DocumentBehaviors, NavigatorBehaviors, NetBehaviors, RendererBehaviors, ShellBehaviors> System<AgentBehaviors, DocumentBehaviors, NavigatorBehaviors, NetBehaviors, RendererBehaviors, ShellBehaviors>
where
    AgentBehaviors: surf_types::agent::obligations::CloseSessionBehavior + surf_types::agent::obligations::GoBehavior + surf_types::agent::obligations::NoteNavigationCommittedBehavior + surf_types::agent::obligations::NoteNavigationEndedBehavior + surf_types::agent::obligations::NoteNavigationFailedBehavior + surf_types::agent::obligations::NoteNavigationStartedBehavior + surf_types::agent::obligations::OpenSessionBehavior + surf_types::agent::obligations::RecordActionBehavior + surf_types::agent::obligations::SessionLogQuery + surf_types::agent::obligations::SessionStateViewQuery + surf_types::agent::obligations::SessionsQuery,
    DocumentBehaviors: surf_types::document::obligations::AddElementBehavior + surf_types::document::obligations::FinishParsingBehavior + surf_types::document::obligations::ParseDocumentBehavior + surf_types::document::obligations::DocumentsQuery + surf_types::document::obligations::ElementsQuery,
    NavigatorBehaviors: surf_types::navigation::obligations::CancelNavigationBehavior + surf_types::navigation::obligations::CommitNavigationBehavior + surf_types::navigation::obligations::FailNavigationBehavior + surf_types::navigation::obligations::NavigateBehavior + surf_types::navigation::obligations::NavigationsQuery,
    NetBehaviors: surf_types::net::obligations::DeliverResponseBehavior + surf_types::net::obligations::FailRequestBehavior + surf_types::net::obligations::FetchBehavior + surf_types::net::obligations::RequestsQuery,
    RendererBehaviors: surf_types::render::obligations::PaintFrameBehavior + surf_types::render::obligations::FramesQuery,
    ShellBehaviors: surf_types::shell::obligations::CloseTabBehavior + surf_types::shell::obligations::OpenTabBehavior + surf_types::shell::obligations::OpenWindowBehavior + surf_types::shell::obligations::TabsQuery + surf_types::shell::obligations::WindowsQuery,
{
    /// Delivers until quiescent: collects every component's outbox onto the log, then gives each
    /// binding that reacts to a logged event one attempt at it, and passes the event once every one
    /// of them has had its attempt.
    ///
    /// Delivery is tracked per binding. A binding whose attempt an unmet obligation stopped, and
    /// which permits a second run, holds the event in its own held-back list; so does a declared
    /// refusal its `on_failure: retry` answers. The next pump attempts each held event again for
    /// that binding alone, after the events published since, and no other binding receives it
    /// twice.
    ///
    /// `Err` carries the first failure among the events this pump delivered for the first time:
    /// what was published since the last pump, and what delivering it published in turn. A
    /// held-back attempt that fails again stays held and is not this pump's answer, so an event
    /// that keeps failing never fails the pump for what is published after it. A specification
    /// whose bindings feed each other without end will not quiesce, and this pump will not
    /// pretend otherwise.
    pub fn pump(&mut self) -> Result<(), surf_types::obligation::UnmetObligation> {
        self.drain()
    }

    /// Delivers one already-published occurrence to every binding that reacts to it and declares
    /// `at_least_once`, again, then pumps until quiescent. An `at_most_once` binding declared one
    /// attempt and no redelivery, so it does not run again.
    ///
    /// The duplicate a delivery guarantee of at least once explicitly permits: the occurrence is
    /// not published a second time — a second occurrence would be a different claim — but
    /// every binding that permits it runs again, and what that causes lands on the log as usual.
    pub fn redeliver(&mut self, event: &SystemEvent) -> Result<(), surf_types::obligation::UnmetObligation> {
        self.redeliver_occurrence(event)?;
        self.pump()
    }

    /// Delivers every logged event the cursor has not passed yet, collecting as it goes: each
    /// reacting binding gets one attempt, and the cursor passes the event once all of them have
    /// had it. `Err` is the first attempt that failed.
    fn drain(&mut self) -> Result<(), surf_types::obligation::UnmetObligation> {
        let mut failed = None;
        loop {
            self.collect();
            if self.cursor == self.published.len() {
                return failed.map_or(Ok(()), Err);
            }
            let event = self.published[self.cursor].clone();
            if let Err(failure) = self.deliver(&event) {
                failed = failed.or(Some(failure));
            }
            self.cursor += 1;
        }
    }

    /// Moves every component's outbox onto the log, in component order.
    fn collect(&mut self) {
        for event in self.agent.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
        for event in self.document.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
        for event in self.navigator.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
        for event in self.net.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
        for event in self.renderer.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
        for event in self.shell.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
    }

    /// Gives every binding that reacts to one logged event its attempt at it.
    ///
    /// Each binding's attempt is its own: one that stops holds the event in that binding's
    /// held-back list where it may run again, and the bindings beside it still run. `Err` is the
    /// first attempt that failed.
    fn deliver(&mut self, event: &SystemEvent) -> Result<(), surf_types::obligation::UnmetObligation> {
        let mut failed = None;
        match event {
            SystemEvent::ActionRecorded(_) => {}
            SystemEvent::NavigationRequested(event) => {
                if let Err(failure) = self.deliver_navigate_on_navigation_requested(event) {
                    failed = failed.or(Some(failure));
                }
            }
            SystemEvent::SessionClosed(event) => {
                if let Err(failure) = self.deliver_close_tab_on_session_closed(event) {
                    failed = failed.or(Some(failure));
                }
            }
            SystemEvent::SessionNavigationChanged(_) => {}
            SystemEvent::SessionOpened(_) => {}
            SystemEvent::DocumentParsed(_) => {}
            SystemEvent::ElementAdded(_) => {}
            SystemEvent::ParsingStarted(_) => {}
            SystemEvent::NavigationCancelled(event) => {
                if let Err(failure) = self.deliver_note_on_navigation_cancelled(event) {
                    failed = failed.or(Some(failure));
                }
            }
            SystemEvent::NavigationCommitted(event) => {
                if let Err(failure) = self.deliver_note_on_navigation_committed(event) {
                    failed = failed.or(Some(failure));
                }
                if let Err(failure) = self.deliver_parse_on_navigation_committed(event) {
                    failed = failed.or(Some(failure));
                }
            }
            SystemEvent::NavigationFailed(event) => {
                if let Err(failure) = self.deliver_note_on_navigation_failed(event) {
                    failed = failed.or(Some(failure));
                }
            }
            SystemEvent::NavigationStarted(event) => {
                if let Err(failure) = self.deliver_fetch_on_navigation_started(event) {
                    failed = failed.or(Some(failure));
                }
                if let Err(failure) = self.deliver_note_on_navigation_started(event) {
                    failed = failed.or(Some(failure));
                }
            }
            SystemEvent::FetchFailed(event) => {
                if let Err(failure) = self.deliver_fail_on_fetch_failed(event) {
                    failed = failed.or(Some(failure));
                }
            }
            SystemEvent::RequestStarted(_) => {}
            SystemEvent::ResponseReceived(event) => {
                if let Err(failure) = self.deliver_commit_on_response(event) {
                    failed = failed.or(Some(failure));
                }
            }
            SystemEvent::FramePainted(_) => {}
            SystemEvent::TabClosed(_) => {}
            SystemEvent::TabOpened(_) => {}
            SystemEvent::WindowOpened(_) => {}
        }
        failed.map_or(Ok(()), Err)
    }

    /// Gives one occurrence again to every binding that reacts to it and declares
    /// `at_least_once`. An `at_most_once` binding declared one attempt and no redelivery, so it
    /// is left out.
    fn redeliver_occurrence(&mut self, event: &SystemEvent) -> Result<(), surf_types::obligation::UnmetObligation> {
        let _ = event;
        Ok(())
    }

    /// `close-tab-on-session-closed`'s attempt at one `surf.agent.SessionClosed`: transform, record the invocation, invoke the
    /// acceptor's port, and answer a declared refusal with the declared policy.
    fn deliver_close_tab_on_session_closed(&mut self, event: &surf_types::agent::SessionClosed) -> Result<(), surf_types::obligation::UnmetObligation> {
        // `close-tab-on-session-closed`: at_most_once, on failure drop.
        let input = close_tab_on_session_closed(event);
        self.invocations.push(BindingInvocation::CloseTabOnSessionClosed(input.clone()));
        // `drop`: a declared refusal is given up silently, because that is what the author
        // wrote; an unmet obligation still propagates.
        let _ = self.shell.close_tab(input)?;
        Ok(())
    }

    /// `commit-on-response`'s attempt at one `surf.net.ResponseReceived`: transform, record the invocation, invoke the
    /// acceptor's port, and answer a declared refusal with the declared policy.
    fn deliver_commit_on_response(&mut self, event: &surf_types::net::ResponseReceived) -> Result<(), surf_types::obligation::UnmetObligation> {
        // `commit-on-response`: at_most_once, on failure drop.
        let input = commit_on_response(event);
        self.invocations.push(BindingInvocation::CommitOnResponse(input.clone()));
        // `drop`: a declared refusal is given up silently, because that is what the author
        // wrote; an unmet obligation still propagates.
        let _ = self.navigator.commit_navigation(input)?;
        Ok(())
    }

    /// `fail-on-fetch-failed`'s attempt at one `surf.net.FetchFailed`: transform, record the invocation, invoke the
    /// acceptor's port, and answer a declared refusal with the declared policy.
    fn deliver_fail_on_fetch_failed(&mut self, event: &surf_types::net::FetchFailed) -> Result<(), surf_types::obligation::UnmetObligation> {
        // `fail-on-fetch-failed`: at_most_once, on failure drop.
        let input = fail_on_fetch_failed(event);
        self.invocations.push(BindingInvocation::FailOnFetchFailed(input.clone()));
        // `drop`: a declared refusal is given up silently, because that is what the author
        // wrote; an unmet obligation still propagates.
        let _ = self.navigator.fail_navigation(input)?;
        Ok(())
    }

    /// `fetch-on-navigation-started`'s attempt at one `surf.navigation.NavigationStarted`: transform, record the invocation, invoke the
    /// acceptor's port, and answer a declared refusal with the declared policy.
    fn deliver_fetch_on_navigation_started(&mut self, event: &surf_types::navigation::NavigationStarted) -> Result<(), surf_types::obligation::UnmetObligation> {
        // `fetch-on-navigation-started`: at_most_once, on failure drop.
        let input = fetch_on_navigation_started(event);
        self.invocations.push(BindingInvocation::FetchOnNavigationStarted(input.clone()));
        // No declared refusal exists, so this invocation cannot fail.
        let _ = self.net.fetch(input)?;
        Ok(())
    }

    /// `navigate-on-navigation-requested`'s attempt at one `surf.agent.NavigationRequested`: transform, record the invocation, invoke the
    /// acceptor's port, and answer a declared refusal with the declared policy.
    fn deliver_navigate_on_navigation_requested(&mut self, event: &surf_types::agent::NavigationRequested) -> Result<(), surf_types::obligation::UnmetObligation> {
        // `navigate-on-navigation-requested`: at_most_once, on failure drop.
        let input = navigate_on_navigation_requested(event);
        self.invocations.push(BindingInvocation::NavigateOnNavigationRequested(input.clone()));
        // No declared refusal exists, so this invocation cannot fail.
        let _ = self.navigator.navigate(input)?;
        Ok(())
    }

    /// `note-on-navigation-cancelled`'s attempt at one `surf.navigation.NavigationCancelled`: transform, record the invocation, invoke the
    /// acceptor's port, and answer a declared refusal with the declared policy.
    fn deliver_note_on_navigation_cancelled(&mut self, event: &surf_types::navigation::NavigationCancelled) -> Result<(), surf_types::obligation::UnmetObligation> {
        // `note-on-navigation-cancelled`: at_most_once, on failure drop.
        let input = note_on_navigation_cancelled(event);
        self.invocations.push(BindingInvocation::NoteOnNavigationCancelled(input.clone()));
        // No declared refusal exists, so this invocation cannot fail.
        let _ = self.agent.note_navigation_ended(input)?;
        Ok(())
    }

    /// `note-on-navigation-committed`'s attempt at one `surf.navigation.NavigationCommitted`: transform, record the invocation, invoke the
    /// acceptor's port, and answer a declared refusal with the declared policy.
    fn deliver_note_on_navigation_committed(&mut self, event: &surf_types::navigation::NavigationCommitted) -> Result<(), surf_types::obligation::UnmetObligation> {
        // `note-on-navigation-committed`: at_most_once, on failure drop.
        let input = note_on_navigation_committed(event);
        self.invocations.push(BindingInvocation::NoteOnNavigationCommitted(input.clone()));
        // No declared refusal exists, so this invocation cannot fail.
        let _ = self.agent.note_navigation_committed(input)?;
        Ok(())
    }

    /// `note-on-navigation-failed`'s attempt at one `surf.navigation.NavigationFailed`: transform, record the invocation, invoke the
    /// acceptor's port, and answer a declared refusal with the declared policy.
    fn deliver_note_on_navigation_failed(&mut self, event: &surf_types::navigation::NavigationFailed) -> Result<(), surf_types::obligation::UnmetObligation> {
        // `note-on-navigation-failed`: at_most_once, on failure drop.
        let input = note_on_navigation_failed(event);
        self.invocations.push(BindingInvocation::NoteOnNavigationFailed(input.clone()));
        // No declared refusal exists, so this invocation cannot fail.
        let _ = self.agent.note_navigation_failed(input)?;
        Ok(())
    }

    /// `note-on-navigation-started`'s attempt at one `surf.navigation.NavigationStarted`: transform, record the invocation, invoke the
    /// acceptor's port, and answer a declared refusal with the declared policy.
    fn deliver_note_on_navigation_started(&mut self, event: &surf_types::navigation::NavigationStarted) -> Result<(), surf_types::obligation::UnmetObligation> {
        // `note-on-navigation-started`: at_most_once, on failure drop.
        let input = note_on_navigation_started(event);
        self.invocations.push(BindingInvocation::NoteOnNavigationStarted(input.clone()));
        // No declared refusal exists, so this invocation cannot fail.
        let _ = self.agent.note_navigation_started(input)?;
        Ok(())
    }

    /// `parse-on-navigation-committed`'s attempt at one `surf.navigation.NavigationCommitted`: transform, record the invocation, invoke the
    /// acceptor's port, and answer a declared refusal with the declared policy.
    fn deliver_parse_on_navigation_committed(&mut self, event: &surf_types::navigation::NavigationCommitted) -> Result<(), surf_types::obligation::UnmetObligation> {
        // `parse-on-navigation-committed`: at_most_once, on failure drop.
        // The condition first (ess/22): false skips this binding alone, and Unknown invokes
        // nothing and is this binding's unmet obligation — the bindings beside it still run.
        match conditions::parse_on_navigation_committed(event) {
            Some(true) => {}
            Some(false) => return Ok(()),
            None => return Err(surf_types::obligation::UnmetObligation { capability: "binding condition", source: "parse-on-navigation-committed" }.into()),
        }
        let input = parse_on_navigation_committed(event);
        self.invocations.push(BindingInvocation::ParseOnNavigationCommitted(input.clone()));
        // No declared refusal exists, so this invocation cannot fail.
        let _ = self.document.parse_document(input)?;
        Ok(())
    }
}
