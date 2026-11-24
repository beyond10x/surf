// generated from surf v1
// contract digest ab11f6612ea9e8cac0562933f8bc6c5facdf23efa6383fb395b012d3ed91413e
// do not edit: regenerate with `ess synthesize`

//! net — the `net` component of `surf` v1.
//!
//! Fetches resources; the only component that talks to the network.
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
    /// `surf.net.FetchFailed`.
    FetchFailed(surf_types::net::FetchFailed),
    /// `surf.net.RequestStarted`.
    RequestStarted(surf_types::net::RequestStarted),
    /// `surf.net.ResponseReceived`.
    ResponseReceived(surf_types::net::ResponseReceived),
}

/// net — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct Net<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> Net<B> {
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

impl<B> Net<B>
where
    B: surf_types::net::obligations::DeliverResponseBehavior + surf_types::net::obligations::FailRequestBehavior + surf_types::net::obligations::FetchBehavior + surf_types::net::obligations::RequestsQuery,
{
    /// Accepts `surf.net.DeliverResponse`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn deliver_response(&mut self, input: surf_types::net::DeliverResponse) -> Result<surf_types::net::DeliverResponseOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.deliver_response(input)?;
        match &outcome {
            surf_types::net::DeliverResponseOutcome::NoSuchRequest { .. } => {}
            surf_types::net::DeliverResponseOutcome::Responded { response_received, .. } => {
                self.outbox.push(PublishedEvent::ResponseReceived(response_received.clone()));
            }
            surf_types::net::DeliverResponseOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `surf.net.FailRequest`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn fail_request(&mut self, input: surf_types::net::FailRequest) -> Result<surf_types::net::FailRequestOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.fail_request(input)?;
        match &outcome {
            surf_types::net::FailRequestOutcome::NoSuchRequest { .. } => {}
            surf_types::net::FailRequestOutcome::Failed { fetch_failed, .. } => {
                self.outbox.push(PublishedEvent::FetchFailed(fetch_failed.clone()));
            }
            surf_types::net::FailRequestOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `surf.net.Fetch`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn fetch(&mut self, input: surf_types::net::Fetch) -> Result<surf_types::net::FetchOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.fetch(input)?;
        match &outcome {
            surf_types::net::FetchOutcome::Started { request_started, .. } => {
                self.outbox.push(PublishedEvent::RequestStarted(request_started.clone()));
            }
        }
        Ok(outcome)
    }

    /// Serves `surf.net.Requests` at `read_your_writes` consistency, from the owed projection.
    pub fn requests(&self) -> Result<Vec<surf_types::net::Requests>, surf_types::obligation::UnmetObligation> {
        self.behaviors.requests()
    }
}
