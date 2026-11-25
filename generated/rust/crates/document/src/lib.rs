// generated from surf v1
// contract digest a8bb2ff5574d285e12f296eab75213256cac446ad01b23b9d6c58aea817d842c
// do not edit: regenerate with `ess synthesize`

//! document — the `document` component of `surf` v1.
//!
//! Parses response bodies into documents, holds the state of their controls and submits their forms.
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
    /// `surf.document.CheckednessSet`.
    CheckednessSet(surf_types::document::CheckednessSet),
    /// `surf.document.DocumentParsed`.
    DocumentParsed(surf_types::document::DocumentParsed),
    /// `surf.document.ElementAdded`.
    ElementAdded(surf_types::document::ElementAdded),
    /// `surf.document.FormAdded`.
    FormAdded(surf_types::document::FormAdded),
    /// `surf.document.FormSubmitted`.
    FormSubmitted(surf_types::document::FormSubmitted),
    /// `surf.document.OptionAdded`.
    OptionAdded(surf_types::document::OptionAdded),
    /// `surf.document.OptionSelected`.
    OptionSelected(surf_types::document::OptionSelected),
    /// `surf.document.ParsingStarted`.
    ParsingStarted(surf_types::document::ParsingStarted),
    /// `surf.document.ScriptEndAdded`.
    ScriptEndAdded(surf_types::document::ScriptEndAdded),
    /// `surf.document.SubmissionAbandoned`.
    SubmissionAbandoned(surf_types::document::SubmissionAbandoned),
    /// `surf.document.SubmissionStarted`.
    SubmissionStarted(surf_types::document::SubmissionStarted),
    /// `surf.document.ValueSet`.
    ValueSet(surf_types::document::ValueSet),
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
    B: surf_types::document::obligations::AbandonSubmissionBehavior + surf_types::document::obligations::AddElementBehavior + surf_types::document::obligations::AddFormBehavior + surf_types::document::obligations::AddOptionBehavior + surf_types::document::obligations::AddScriptEndBehavior + surf_types::document::obligations::CheckBehavior + surf_types::document::obligations::FinishParsingBehavior + surf_types::document::obligations::ParseDocumentBehavior + surf_types::document::obligations::SelectOptionBehavior + surf_types::document::obligations::SetValueBehavior + surf_types::document::obligations::StartSubmissionBehavior + surf_types::document::obligations::SubmitFormBehavior + surf_types::document::obligations::ControlsQuery + surf_types::document::obligations::DocumentsQuery + surf_types::document::obligations::ElementsQuery + surf_types::document::obligations::FormsQuery + surf_types::document::obligations::OptionsQuery + surf_types::document::obligations::ScriptsQuery + surf_types::document::obligations::SubmissionsQuery,
{
    /// Accepts `surf.document.AbandonSubmission`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn abandon_submission(&mut self, input: surf_types::document::AbandonSubmission) -> Result<surf_types::document::AbandonSubmissionOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.abandon_submission(input)?;
        match &outcome {
            surf_types::document::AbandonSubmissionOutcome::NoSuchSubmission { .. } => {}
            surf_types::document::AbandonSubmissionOutcome::Abandoned { submission_abandoned, .. } => {
                self.outbox.push(PublishedEvent::SubmissionAbandoned(submission_abandoned.clone()));
            }
            surf_types::document::AbandonSubmissionOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

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

    /// Accepts `surf.document.AddForm`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn add_form(&mut self, input: surf_types::document::AddForm) -> Result<surf_types::document::AddFormOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.add_form(input)?;
        match &outcome {
            surf_types::document::AddFormOutcome::NoSuchDocument { .. } => {}
            surf_types::document::AddFormOutcome::AlreadyParsed { .. } => {}
            surf_types::document::AddFormOutcome::Added { form_added, .. } => {
                self.outbox.push(PublishedEvent::FormAdded(form_added.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.document.AddOption`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn add_option(&mut self, input: surf_types::document::AddOption) -> Result<surf_types::document::AddOptionOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.add_option(input)?;
        match &outcome {
            surf_types::document::AddOptionOutcome::NoSuchSelect { .. } => {}
            surf_types::document::AddOptionOutcome::NotASelect { .. } => {}
            surf_types::document::AddOptionOutcome::Added { option_added, .. } => {
                self.outbox.push(PublishedEvent::OptionAdded(option_added.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.document.AddScriptEnd`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn add_script_end(&mut self, input: surf_types::document::AddScriptEnd) -> Result<surf_types::document::AddScriptEndOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.add_script_end(input)?;
        match &outcome {
            surf_types::document::AddScriptEndOutcome::InvalidEnd { .. } => {}
            surf_types::document::AddScriptEndOutcome::InvalidPosition { .. } => {}
            surf_types::document::AddScriptEndOutcome::NoSuchDocument { .. } => {}
            surf_types::document::AddScriptEndOutcome::AlreadyParsed { .. } => {}
            surf_types::document::AddScriptEndOutcome::Added { script_end_added, .. } => {
                self.outbox.push(PublishedEvent::ScriptEndAdded(script_end_added.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.document.Check`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn check(&mut self, input: surf_types::document::Check) -> Result<surf_types::document::CheckOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.check(input)?;
        match &outcome {
            surf_types::document::CheckOutcome::NoSuchElement { .. } => {}
            surf_types::document::CheckOutcome::NotCheckable { .. } => {}
            surf_types::document::CheckOutcome::Disabled { .. } => {}
            surf_types::document::CheckOutcome::Hidden { .. } => {}
            surf_types::document::CheckOutcome::RadioChecked { checkedness_set, .. } => {
                self.outbox.push(PublishedEvent::CheckednessSet(checkedness_set.clone()));
            }
            surf_types::document::CheckOutcome::Unchecked { checkedness_set, .. } => {
                self.outbox.push(PublishedEvent::CheckednessSet(checkedness_set.clone()));
            }
            surf_types::document::CheckOutcome::Checked { checkedness_set, .. } => {
                self.outbox.push(PublishedEvent::CheckednessSet(checkedness_set.clone()));
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

    /// Accepts `surf.document.SelectOption`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn select_option(&mut self, input: surf_types::document::SelectOption) -> Result<surf_types::document::SelectOptionOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.select_option(input)?;
        match &outcome {
            surf_types::document::SelectOptionOutcome::NoSuchOption { .. } => {}
            surf_types::document::SelectOptionOutcome::Disabled { .. } => {}
            surf_types::document::SelectOptionOutcome::Hidden { .. } => {}
            surf_types::document::SelectOptionOutcome::Selected { option_selected, .. } => {
                self.outbox.push(PublishedEvent::OptionSelected(option_selected.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.document.SetValue`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn set_value(&mut self, input: surf_types::document::SetValue) -> Result<surf_types::document::SetValueOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.set_value(input)?;
        match &outcome {
            surf_types::document::SetValueOutcome::NoSuchElement { .. } => {}
            surf_types::document::SetValueOutcome::NotTextEntry { .. } => {}
            surf_types::document::SetValueOutcome::Disabled { .. } => {}
            surf_types::document::SetValueOutcome::Hidden { .. } => {}
            surf_types::document::SetValueOutcome::Readonly { .. } => {}
            surf_types::document::SetValueOutcome::Set { value_set, .. } => {
                self.outbox.push(PublishedEvent::ValueSet(value_set.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.document.StartSubmission`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn start_submission(&mut self, input: surf_types::document::StartSubmission) -> Result<surf_types::document::StartSubmissionOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.start_submission(input)?;
        match &outcome {
            surf_types::document::StartSubmissionOutcome::NoSuchElement { .. } => {}
            surf_types::document::StartSubmissionOutcome::NotAButton { .. } => {}
            surf_types::document::StartSubmissionOutcome::Disabled { .. } => {}
            surf_types::document::StartSubmissionOutcome::Hidden { .. } => {}
            surf_types::document::StartSubmissionOutcome::Started { submission_started, .. } => {
                self.outbox.push(PublishedEvent::SubmissionStarted(submission_started.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `surf.document.SubmitForm`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn submit_form(&mut self, input: surf_types::document::SubmitForm) -> Result<surf_types::document::SubmitFormOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.submit_form(input)?;
        match &outcome {
            surf_types::document::SubmitFormOutcome::NoSuchSubmission { .. } => {}
            surf_types::document::SubmitFormOutcome::Submitted { form_submitted, .. } => {
                self.outbox.push(PublishedEvent::FormSubmitted(form_submitted.clone()));
            }
            surf_types::document::SubmitFormOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Serves `surf.document.Controls` at `read_your_writes` consistency, from the owed projection.
    pub fn controls(&self) -> Result<Vec<surf_types::document::Controls>, surf_types::obligation::UnmetObligation> {
        self.behaviors.controls()
    }

    /// Serves `surf.document.Documents` at `read_your_writes` consistency, from the owed projection.
    pub fn documents(&self) -> Result<Vec<surf_types::document::Documents>, surf_types::obligation::UnmetObligation> {
        self.behaviors.documents()
    }

    /// Serves `surf.document.Elements` at `read_your_writes` consistency, from the owed projection.
    pub fn elements(&self, document_id: surf_types::document::DocumentId) -> Result<Vec<surf_types::document::Elements>, surf_types::obligation::UnmetObligation> {
        self.behaviors.elements(document_id)
    }

    /// Serves `surf.document.Forms` at `read_your_writes` consistency, from the owed projection.
    pub fn forms(&self) -> Result<Vec<surf_types::document::Forms>, surf_types::obligation::UnmetObligation> {
        self.behaviors.forms()
    }

    /// Serves `surf.document.Options` at `read_your_writes` consistency, from the owed projection.
    pub fn options(&self) -> Result<Vec<surf_types::document::Options>, surf_types::obligation::UnmetObligation> {
        self.behaviors.options()
    }

    /// Serves `surf.document.Scripts` at `read_your_writes` consistency, from the owed projection.
    pub fn scripts(&self, document_id: surf_types::document::DocumentId) -> Result<Vec<surf_types::document::Scripts>, surf_types::obligation::UnmetObligation> {
        self.behaviors.scripts(document_id)
    }

    /// Serves `surf.document.Submissions` at `read_your_writes` consistency, from the owed projection.
    pub fn submissions(&self) -> Result<Vec<surf_types::document::Submissions>, surf_types::obligation::UnmetObligation> {
        self.behaviors.submissions()
    }
}
