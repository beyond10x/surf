// generated from surf v1
// contract digest a04361f387b990efa6d72ce0d03efd246b075d09426d6d76adc1571cfa05800d
// do not edit: regenerate with `ess synthesize`

//! Document — `surf.document`.
//!
//! Turning the bytes of a response into a document, its title and its actionable elements.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// The states of `surf.document.Document`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Document<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentState {
    /// `Parsed`.
    Parsed,
    /// `Parsing`.
    Parsing,
}

/// DocumentId — `surf.document.DocumentId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentId(pub crate::primitives::Uuid);

/// The states of `surf.document.Element`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Element<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementState {
    /// `Added`.
    Added,
}

/// ElementId — `surf.document.ElementId`: a distinct wrapper around `Uuid`.
///
/// The identity of one actionable element of a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementId(pub crate::primitives::Uuid);

/// ElementKind — `surf.document.ElementKind`: one of a closed set of names.
///
/// What an actionable element is, as the agent's answer names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    /// `link`.
    Link,
    /// `button`.
    Button,
    /// `text`.
    Text,
    /// `password`.
    Password,
    /// `checkbox`.
    Checkbox,
    /// `radio`.
    Radio,
    /// `file`.
    File,
    /// `select`.
    Select,
    /// `textarea`.
    Textarea,
}

/// EncodingName — `surf.document.EncodingName`: a distinct wrapper around `String`.
///
/// The name of an encoding, as the Encoding Standard writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodingName(pub String);

/// ParseLimit — `surf.document.ParseLimit`: one of a closed set of names.
///
/// The limit that stopped parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseLimit {
    /// `InputBytes`.
    InputBytes,
    /// `Depth`.
    Depth,
    /// `Nodes`.
    Nodes,
    /// `Attributes`.
    Attributes,
    /// `StoredBytes`.
    StoredBytes,
    /// `Steps`.
    Steps,
}

/// What Document — `surf.document.Document` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Document<S>`], and at a boundary by [`DocumentSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentData {
    /// The identity: `document_id` — `surf.document.DocumentId`.
    pub document_id: DocumentId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `navigation_id` — `surf.net.RequestId`.
    ///
    /// Carries `navigation`: `surf.document.Document` references one `surf.navigation.Navigation`.
    pub navigation_id: crate::net::RequestId,
    /// `title` — `Optional<String>`.
    pub title: Option<String>,
    /// `encoding` — `Optional<surf.document.EncodingName>`.
    pub encoding: Option<EncodingName>,
    /// `unsupported_encoding` — `Optional<surf.document.EncodingName>`.
    pub unsupported_encoding: Option<EncodingName>,
    /// `stopped_by` — `Optional<surf.document.ParseLimit>`.
    pub stopped_by: Option<ParseLimit>,
}

/// The states of `surf.document.Document`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](document_state::Marker), so [`Document<S>`](Document) can only ever rest in a real state.
pub mod document_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Parsed {}
        impl Sealed for super::Parsing {}
    }

    /// A declared state of `Document`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::DocumentState;
    }

    /// `Parsed`. Terminal: an instance may rest here forever.
    pub struct Parsed;

    impl Marker for Parsed {
        const STATE: super::DocumentState = super::DocumentState::Parsed;
    }

    /// `Parsing`. Where a new instance starts.
    pub struct Parsing;

    impl Marker for Parsing {
        const STATE: super::DocumentState = super::DocumentState::Parsing;
    }
}

/// Document — `surf.document.Document` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Parsing`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`DocumentSnapshot`]
/// and [`DocumentSnapshot::refine`].
pub struct Document<S: document_state::Marker> {
    data: DocumentData,
    state: core::marker::PhantomData<S>,
}

impl<S: document_state::Marker> Document<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> DocumentState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &DocumentData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> DocumentData {
        self.data
    }
}

impl Document<document_state::Parsing> {
    /// A new instance, resting in `Parsing` — the only state the lifecycle starts one in.
    pub fn new(data: DocumentData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Document<document_state::Parsing> {
    /// `finish` — `Parsing` → `Parsed`. Taken by the `parsed` outcome of `surf.document.FinishParsing`.
    pub fn finish(self) -> Document<document_state::Parsed> {
        Document {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `surf.document.Document` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`DocumentSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: DocumentState,
    /// What it holds.
    pub data: DocumentData,
}

/// An `Document` in whichever declared state it was found.
pub enum AnyDocument {
    /// Resting in `Parsed`.
    Parsed(Document<document_state::Parsed>),
    /// Resting in `Parsing`.
    Parsing(Document<document_state::Parsing>),
}

impl DocumentSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `DocumentState` cannot spell one.
    pub fn refine(self) -> AnyDocument {
        match self.state {
            DocumentState::Parsed => AnyDocument::Parsed(Document {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            DocumentState::Parsing => AnyDocument::Parsing(Document {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyDocument {
    /// The state, as the runtime value.
    pub fn state(&self) -> DocumentState {
        match self {
            Self::Parsed(_) => DocumentState::Parsed,
            Self::Parsing(_) => DocumentState::Parsing,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> DocumentSnapshot {
        match self {
            Self::Parsed(instance) => DocumentSnapshot {
                state: DocumentState::Parsed,
                data: instance.into_data(),
            },
            Self::Parsing(instance) => DocumentSnapshot {
                state: DocumentState::Parsing,
                data: instance.into_data(),
            },
        }
    }
}

/// What Element — `surf.document.Element` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Element<S>`], and at a boundary by [`ElementSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementData {
    /// The identity: `element_id` — `surf.document.ElementId`.
    pub element_id: ElementId,
    /// `document_id` — `surf.document.DocumentId`.
    ///
    /// Carries `document`: `surf.document.Element` references one `surf.document.Document`.
    pub document_id: DocumentId,
    /// `position` — `Integer`.
    pub position: i64,
    /// `kind` — `surf.document.ElementKind`.
    pub kind: ElementKind,
    /// `label` — `String`.
    pub label: String,
    /// `target` — `Optional<String>`.
    pub target: Option<String>,
    /// `value` — `Optional<String>`.
    pub value: Option<String>,
    /// `hidden` — `Boolean`.
    pub hidden: bool,
    /// `disabled` — `Boolean`.
    pub disabled: bool,
    /// `checked` — `Boolean`.
    pub checked: bool,
    /// `required` — `Boolean`.
    pub required: bool,
}

/// The states of `surf.document.Element`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](element_state::Marker), so [`Element<S>`](Element) can only ever rest in a real state.
pub mod element_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Added {}
    }

    /// A declared state of `Element`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ElementState;
    }

    /// `Added`. Where a new instance starts.
    pub struct Added;

    impl Marker for Added {
        const STATE: super::ElementState = super::ElementState::Added;
    }
}

/// Element — `surf.document.Element` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Added`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ElementSnapshot`]
/// and [`ElementSnapshot::refine`].
pub struct Element<S: element_state::Marker> {
    data: ElementData,
    state: core::marker::PhantomData<S>,
}

impl<S: element_state::Marker> Element<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ElementState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ElementData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ElementData {
        self.data
    }
}

impl Element<element_state::Added> {
    /// A new instance, resting in `Added` — the only state the lifecycle starts one in.
    pub fn new(data: ElementData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `surf.document.Element` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ElementSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ElementState,
    /// What it holds.
    pub data: ElementData,
}

/// An `Element` in whichever declared state it was found.
pub enum AnyElement {
    /// Resting in `Added`.
    Added(Element<element_state::Added>),
}

impl ElementSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ElementState` cannot spell one.
    pub fn refine(self) -> AnyElement {
        match self.state {
            ElementState::Added => AnyElement::Added(Element {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyElement {
    /// The state, as the runtime value.
    pub fn state(&self) -> ElementState {
        match self {
            Self::Added(_) => ElementState::Added,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ElementSnapshot {
        match self {
            Self::Added(instance) => ElementSnapshot {
                state: ElementState::Added,
                data: instance.into_data(),
            },
        }
    }
}

/// Add an element — the input of `surf.document.AddElement`.
///
/// Record one actionable element of a document being parsed.
///
/// Everything it can result in is [`AddElementOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddElement {
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: DocumentId,
    /// `position` — `Integer`.
    pub position: i64,
    /// `kind` — `surf.document.ElementKind`.
    pub kind: ElementKind,
    /// `label` — `String`.
    pub label: String,
    /// `target` — `Optional<String>`.
    pub target: Option<String>,
    /// `value` — `Optional<String>`.
    pub value: Option<String>,
    /// `hidden` — `Boolean`.
    pub hidden: bool,
    /// `disabled` — `Boolean`.
    pub disabled: bool,
    /// `checked` — `Boolean`.
    pub checked: bool,
    /// `required` — `Boolean`.
    pub required: bool,
}

/// Everything `surf.document.AddElement` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddElementOutcome {
    /// `no-such-document` — when no `surf.document.Document` carries the identity `input.document_id` names.
    ///
    /// No document has this identity; nothing is added.
    NoSuchDocument {
        /// Why it was refused: `surf.document.DocumentNotFound`.
        error: DocumentNotFound,
    },
    /// `already-parsed` — when the `surf.document.Document` that `input.document_id` names satisfies `state == Parsed`.
    ///
    /// The document was already Parsed; nothing is added.
    AlreadyParsed {
        /// Why it was refused: `surf.document.DocumentStateConflict`.
        error: DocumentStateConflict,
    },
    /// `added` — otherwise.
    ///
    /// The element is a row of its document.
    Added {
        /// The `surf.document.ElementAdded` this outcome publishes.
        element_added: ElementAdded,
    },
}

/// Finish parsing — the input of `surf.document.FinishParsing`.
///
/// End a document's parsing with its title, its encoding and the limit that stopped it.
///
/// Everything it can result in is [`FinishParsingOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishParsing {
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: DocumentId,
    /// `title` — `String`.
    pub title: String,
    /// `encoding` — `surf.document.EncodingName`.
    pub encoding: EncodingName,
    /// `unsupported_encoding` — `Optional<surf.document.EncodingName>`.
    pub unsupported_encoding: Option<EncodingName>,
    /// `stopped_by` — `Optional<surf.document.ParseLimit>`.
    pub stopped_by: Option<ParseLimit>,
}

/// Everything `surf.document.FinishParsing` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishParsingOutcome {
    /// `no-such-document` — for an identity no record carries.
    NoSuchDocument {
        /// Why it was refused: `surf.document.DocumentNotFound`.
        error: DocumentNotFound,
    },
    /// `parsed` — otherwise.
    ///
    /// The document is Parsed.
    Parsed {
        /// The `surf.document.DocumentParsed` this outcome publishes.
        document_parsed: DocumentParsed,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The document's parsing had already ended.
    WrongState {
        /// Why it was refused: `surf.document.DocumentStateConflict`.
        error: DocumentStateConflict,
    },
}

/// Parse a document — the input of `surf.document.ParseDocument`.
///
/// Start parsing an HTML response body into a document.
///
/// Everything it can result in is [`ParseDocumentOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDocument {
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `body` — `Bytes`.
    pub body: Vec<u8>,
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
}

/// Everything `surf.document.ParseDocument` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseDocumentOutcome {
    /// `started` — otherwise.
    ///
    /// The document is Parsing.
    Started {
        /// The `surf.document.ParsingStarted` this outcome publishes.
        parsing_started: ParsingStarted,
    },
}

/// DocumentParsed — the event `surf.document.DocumentParsed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentParsed {
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: DocumentId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
    /// `title` — `String`.
    pub title: String,
    /// `encoding` — `surf.document.EncodingName`.
    pub encoding: EncodingName,
    /// `unsupported_encoding` — `Optional<surf.document.EncodingName>`.
    pub unsupported_encoding: Option<EncodingName>,
    /// `stopped_by` — `Optional<surf.document.ParseLimit>`.
    pub stopped_by: Option<ParseLimit>,
}

/// ElementAdded — the event `surf.document.ElementAdded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementAdded {
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: ElementId,
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: DocumentId,
    /// `position` — `Integer`.
    pub position: i64,
}

/// ParsingStarted — the event `surf.document.ParsingStarted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsingStarted {
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: DocumentId,
    /// `body` — `Bytes`.
    pub body: Vec<u8>,
}

/// The declared error `surf.document.DocumentNotFound`.
///
/// No document has this identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentNotFound {
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: DocumentId,
}

/// The declared error `surf.document.DocumentStateConflict`.
///
/// The document is not being parsed, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentStateConflict {
    /// `state` — `surf.document.Document.State`.
    pub state: DocumentState,
}

/// Documents — one row of the view `surf.document.Documents`.
///
/// Projects `surf.document.Document` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Documents {
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: DocumentId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `navigation_id` — `surf.net.RequestId`.
    pub navigation_id: crate::net::RequestId,
    /// `state` — `surf.document.Document.State`.
    pub state: DocumentState,
    /// `title` — `Optional<String>`.
    pub title: Option<String>,
    /// `encoding` — `Optional<surf.document.EncodingName>`.
    pub encoding: Option<EncodingName>,
    /// `unsupported_encoding` — `Optional<surf.document.EncodingName>`.
    pub unsupported_encoding: Option<EncodingName>,
    /// `stopped_by` — `Optional<surf.document.ParseLimit>`.
    pub stopped_by: Option<ParseLimit>,
}

/// Elements — one row of the view `surf.document.Elements`.
///
/// Projects `surf.document.Element` at `read_your_writes` consistency, containing instances where `document_id == param.document_id`.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Elements {
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: ElementId,
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: DocumentId,
    /// `position` — `Integer`.
    pub position: i64,
    /// `kind` — `surf.document.ElementKind`.
    pub kind: ElementKind,
    /// `label` — `String`.
    pub label: String,
    /// `target` — `Optional<String>`.
    pub target: Option<String>,
    /// `value` — `Optional<String>`.
    pub value: Option<String>,
    /// `hidden` — `Boolean`.
    pub hidden: bool,
    /// `disabled` — `Boolean`.
    pub disabled: bool,
    /// `checked` — `Boolean`.
    pub checked: bool,
    /// `required` — `Boolean`.
    pub required: bool,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
pub mod obligations {
    /// The behaviour `surf.document.AddElement` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait AddElementBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.document.AddElement`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn add_element(&mut self, input: super::AddElement) -> Result<super::AddElementOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.document.FinishParsing` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait FinishParsingBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.document.FinishParsing`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn finish_parsing(&mut self, input: super::FinishParsing) -> Result<super::FinishParsingOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.document.ParseDocument` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ParseDocumentBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.document.ParseDocument`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn parse_document(&mut self, input: super::ParseDocument) -> Result<super::ParseDocumentOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `surf.document.Documents` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait DocumentsQuery {
        /// Serves `surf.document.Documents` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn documents(&self) -> Result<Vec<super::Documents>, crate::obligation::UnmetObligation>;
    }

    /// The query `surf.document.Elements` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ElementsQuery {
        /// Serves `surf.document.Elements` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn elements(&self, document_id: crate::document::DocumentId) -> Result<Vec<super::Elements>, crate::obligation::UnmetObligation>;
    }

}
