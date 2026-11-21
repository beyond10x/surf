// generated from surf v1
// contract digest 055477f7d70d2837ffe9e39fee6cbc6130c982cf9d0650a28bb00f7c8ca52d2c
// do not edit: regenerate with `ess synthesize`

//! Document — `surf.document`.
//!
//! Turning the bytes of a response into a document.
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
}

/// DocumentId — `surf.document.DocumentId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentId(pub crate::primitives::Uuid);

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
    }

    /// A declared state of `Document`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::DocumentState;
    }

    /// `Parsed`. Where a new instance starts.
    pub struct Parsed;

    impl Marker for Parsed {
        const STATE: super::DocumentState = super::DocumentState::Parsed;
    }
}

/// Document — `surf.document.Document` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Parsed`, and the only way to change `S` is a method generated from
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

impl Document<document_state::Parsed> {
    /// A new instance, resting in `Parsed` — the only state the lifecycle starts one in.
    pub fn new(data: DocumentData) -> Self {
        Self {
            data,
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
        }
    }
}

impl AnyDocument {
    /// The state, as the runtime value.
    pub fn state(&self) -> DocumentState {
        match self {
            Self::Parsed(_) => DocumentState::Parsed,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> DocumentSnapshot {
        match self {
            Self::Parsed(instance) => DocumentSnapshot {
                state: DocumentState::Parsed,
                data: instance.into_data(),
            },
        }
    }
}

/// Parse a document — the input of `surf.document.ParseDocument`.
///
/// Parse an HTML response body into a document.
///
/// Everything it can result in is [`ParseDocumentOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDocument {
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `body` — `Bytes`.
    pub body: Vec<u8>,
}

/// Everything `surf.document.ParseDocument` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseDocumentOutcome {
    /// `parsed` — otherwise.
    ///
    /// The document exists.
    Parsed {
        /// The `surf.document.DocumentParsed` this outcome publishes.
        document_parsed: DocumentParsed,
    },
}

/// DocumentParsed — the event `surf.document.DocumentParsed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentParsed {
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: DocumentId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
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
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
pub mod obligations {
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

}
