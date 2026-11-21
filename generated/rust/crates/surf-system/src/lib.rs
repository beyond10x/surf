// generated from surf v1
// contract digest 19964b50f9197185811d923fbd6c3eec7ec2855a2cc03c2c60d6e4a2149578f0
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
    /// `surf.document.DocumentParsed`.
    DocumentParsed(surf_types::document::DocumentParsed),
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
            Self::DocumentParsed(_) => "surf.document.DocumentParsed",
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

impl From<document::PublishedEvent> for SystemEvent {
    fn from(event: document::PublishedEvent) -> Self {
        match event {
            document::PublishedEvent::DocumentParsed(event) => Self::DocumentParsed(event),
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
    /// `commit-on-response` invoked `surf.navigation.CommitNavigation`.
    CommitOnResponse(surf_types::navigation::CommitNavigation),
    /// `fail-on-fetch-failed` invoked `surf.navigation.FailNavigation`.
    FailOnFetchFailed(surf_types::navigation::FailNavigation),
    /// `fetch-on-navigation-started` invoked `surf.net.Fetch`.
    FetchOnNavigationStarted(surf_types::net::Fetch),
    /// `parse-on-navigation-committed` invoked `surf.document.ParseDocument`.
    ParseOnNavigationCommitted(surf_types::document::ParseDocument),
}

/// The binding `commit-on-response`: `surf.net.ResponseReceived`, read as `surf.navigation.CommitNavigation` input.
///
/// Fully determined by the specification: every input is filled from an event field — through the
/// declared crossing where one is named — from a literal the target admits, or left absent
/// where the input is optional and the binding says nothing.
pub fn commit_on_response(event: &surf_types::net::ResponseReceived) -> surf_types::navigation::CommitNavigation {
    surf_types::navigation::CommitNavigation {
        navigation_id: event.request_id.clone(),
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
pub struct System<DocumentBehaviors, NavigatorBehaviors, NetBehaviors, RendererBehaviors, ShellBehaviors> {
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

impl<DocumentBehaviors, NavigatorBehaviors, NetBehaviors, RendererBehaviors, ShellBehaviors> System<DocumentBehaviors, NavigatorBehaviors, NetBehaviors, RendererBehaviors, ShellBehaviors> {
    /// Assembles the system from its components.
    pub fn new(document: document::Document<DocumentBehaviors>, navigator: navigator::Navigator<NavigatorBehaviors>, net: net::Net<NetBehaviors>, renderer: renderer::Renderer<RendererBehaviors>, shell: shell::Shell<ShellBehaviors>) -> Self {
        Self {
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

impl<DocumentBehaviors, NavigatorBehaviors, NetBehaviors, RendererBehaviors, ShellBehaviors> System<DocumentBehaviors, NavigatorBehaviors, NetBehaviors, RendererBehaviors, ShellBehaviors>
where
    DocumentBehaviors: surf_types::document::obligations::ParseDocumentBehavior + surf_types::document::obligations::DocumentsQuery,
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
            SystemEvent::DocumentParsed(_) => {}
            SystemEvent::NavigationCancelled(_) => {}
            SystemEvent::NavigationCommitted(event) => {
                if let Err(failure) = self.deliver_parse_on_navigation_committed(event) {
                    failed = failed.or(Some(failure));
                }
            }
            SystemEvent::NavigationFailed(_) => {}
            SystemEvent::NavigationStarted(event) => {
                if let Err(failure) = self.deliver_fetch_on_navigation_started(event) {
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
