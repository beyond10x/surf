// generated from surf v1
// contract digest 96684badf51201e308af373a0097db9fb63321aefd9a08fd216a1e28745125c0
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
    /// `surf.document.ElementAdded`.
    ElementAdded(surf_types::document::ElementAdded),
    /// `surf.document.ParsingStarted`.
    ParsingStarted(surf_types::document::ParsingStarted),
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
    B: surf_types::document::obligations::AddElementBehavior + surf_types::document::obligations::FinishParsingBehavior + surf_types::document::obligations::ParseDocumentBehavior + surf_types::document::obligations::DocumentsQuery + surf_types::document::obligations::ElementsQuery,
{
    /// Accepts `surf.document.AddElement`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn add_element(&mut self, input: surf_types::document::AddElement) -> Result<surf_types::document::AddElementOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.add_element(input)?;
        match &outcome {
            surf_types::document::AddElementOutcome::NoSuchDocument { .. } => {}
            surf_types::document::AddElementOutcome::AlreadyParsed { .. } => {}
            surf_types::document::AddElementOutcome::Added { element_added, .. } => {
                self.outbox.push(PublishedEvent::ElementAdded(element_added.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.document.FinishParsing`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn finish_parsing(&mut self, input: surf_types::document::FinishParsing) -> Result<surf_types::document::FinishParsingOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.finish_parsing(input)?;
        match &outcome {
            surf_types::document::FinishParsingOutcome::NoSuchDocument { .. } => {}
            surf_types::document::FinishParsingOutcome::Parsed { document_parsed, .. } => {
                self.outbox.push(PublishedEvent::DocumentParsed(document_parsed.clone()));
            }
            surf_types::document::FinishParsingOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `surf.document.ParseDocument`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn parse_document(&mut self, input: surf_types::document::ParseDocument) -> Result<surf_types::document::ParseDocumentOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.parse_document(input)?;
        match &outcome {
            surf_types::document::ParseDocumentOutcome::Started { parsing_started, .. } => {
                self.outbox.push(PublishedEvent::ParsingStarted(parsing_started.clone()));
            }
        }
        Ok(outcome)
    }

    /// Serves `surf.document.Documents` at `read_your_writes` consistency, from the owed projection.
    pub fn documents(&self) -> Result<Vec<surf_types::document::Documents>, surf_types::obligation::UnmetObligation> {
        self.behaviors.documents()
    }

    /// Serves `surf.document.Elements` at `read_your_writes` consistency, from the owed projection.
    pub fn elements(&self, document_id: surf_types::document::DocumentId) -> Result<Vec<surf_types::document::Elements>, surf_types::obligation::UnmetObligation> {
        self.behaviors.elements(document_id)
    }
}
