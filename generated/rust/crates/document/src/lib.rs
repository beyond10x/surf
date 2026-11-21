// generated from surf v1
// contract digest 19964b50f9197185811d923fbd6c3eec7ec2855a2cc03c2c60d6e4a2149578f0
// do not edit: regenerate with `ess synthesize`

//! document — the `document` component of `surf` v1.
//!
//! Parses response bodies into documents.
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
    /// `surf.document.DocumentParsed`.
    DocumentParsed(surf_types::document::DocumentParsed),
}

/// document — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct Document<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> Document<B> {
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

impl<B> Document<B>
where
    B: surf_types::document::obligations::ParseDocumentBehavior + surf_types::document::obligations::DocumentsQuery,
{
    /// Accepts `surf.document.ParseDocument`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn parse_document(&mut self, input: surf_types::document::ParseDocument) -> Result<surf_types::document::ParseDocumentOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.parse_document(input)?;
        match &outcome {
            surf_types::document::ParseDocumentOutcome::Parsed { document_parsed, .. } => {
                self.outbox.push(PublishedEvent::DocumentParsed(document_parsed.clone()));
            }
        }
        Ok(outcome)
    }

    /// Serves `surf.document.Documents` at `read_your_writes` consistency, from the owed projection.
    pub fn documents(&self) -> Result<Vec<surf_types::document::Documents>, surf_types::obligation::UnmetObligation> {
        self.behaviors.documents()
    }
}
