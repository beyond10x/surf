// generated from surf v1
// contract digest aac42d8ac87218cacd79d661cdfc6da87c457bcea21b6e55cbec893553faf526
// do not edit: regenerate with `ess synthesize`

//! Document — `surf.document`.
//!
//! Turning the bytes of a response into a document, its title, its actionable elements and its forms; the state of its controls; and the submission of a form.
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

/// The states of `surf.document.Form`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Form<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormState {
    /// `Added`.
    Added,
}

/// FormId — `surf.document.FormId`: a distinct wrapper around `Uuid`.
///
/// The identity of one form of a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormId(pub crate::primitives::Uuid);

/// The states of `surf.document.Option`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `ListOption<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionState {
    /// `Added`.
    Added,
}

/// OptionId — `surf.document.OptionId`: a distinct wrapper around `Uuid`.
///
/// The identity of one option of a select.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionId(pub crate::primitives::Uuid);

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

/// The states of `surf.document.Submission`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Submission<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmissionState {
    /// `Abandoned`.
    Abandoned,
    /// `Started`.
    Started,
    /// `Submitted`.
    Submitted,
}

/// SubmissionEndReason — `surf.document.SubmissionEndReason`: one of a closed set of names.
///
/// Why a submission ended without a navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmissionEndReason {
    /// `NoFormOwner`.
    NoFormOwner,
    /// `NotSubmitButton`.
    NotSubmitButton,
    /// `InvalidControls`.
    InvalidControls,
    /// `InvalidAction`.
    InvalidAction,
    /// `UnsupportedScheme`.
    UnsupportedScheme,
    /// `UnsupportedEnctype`.
    UnsupportedEnctype,
    /// `DialogMethod`.
    DialogMethod,
    /// `AddressTooLong`.
    AddressTooLong,
}

/// SubmissionId — `surf.document.SubmissionId`: a distinct wrapper around `Uuid`.
///
/// The identity of one submission of a form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmissionId(pub crate::primitives::Uuid);

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
    /// `tab_id` — `surf.shell.TabId`.
    ///
    /// Carries `tab`: `surf.document.Document` references one `surf.shell.Tab`.
    pub tab_id: crate::shell::TabId,
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
    /// `readonly` — `Boolean`.
    pub readonly: bool,
    /// `name` — `Optional<String>`.
    pub name: Option<String>,
    /// `form_id` — `Optional<surf.document.FormId>`.
    ///
    /// Carries `form`: `surf.document.Element` references one `surf.document.Form`.
    pub form_id: Option<FormId>,
    /// `radio_group` — `Optional<String>`.
    pub radio_group: Option<String>,
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

/// What Form — `surf.document.Form` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Form<S>`], and at a boundary by [`FormSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormData {
    /// The identity: `form_id` — `surf.document.FormId`.
    pub form_id: FormId,
    /// `document_id` — `surf.document.DocumentId`.
    ///
    /// Carries `forms`: `surf.document.Document` owns many `surf.document.Form`.
    pub document_id: DocumentId,
    /// `position` — `Integer`.
    pub position: i64,
    /// `action` — `Optional<String>`.
    pub action: Option<String>,
    /// `method` — `Optional<String>`.
    pub method: Option<String>,
    /// `enctype` — `Optional<String>`.
    pub enctype: Option<String>,
}

/// The states of `surf.document.Form`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](form_state::Marker), so [`Form<S>`](Form) can only ever rest in a real state.
pub mod form_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Added {}
    }

    /// A declared state of `Form`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::FormState;
    }

    /// `Added`. Where a new instance starts.
    pub struct Added;

    impl Marker for Added {
        const STATE: super::FormState = super::FormState::Added;
    }
}

/// Form — `surf.document.Form` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Added`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`FormSnapshot`]
/// and [`FormSnapshot::refine`].
pub struct Form<S: form_state::Marker> {
    data: FormData,
    state: core::marker::PhantomData<S>,
}

impl<S: form_state::Marker> Form<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> FormState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &FormData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> FormData {
        self.data
    }
}

impl Form<form_state::Added> {
    /// A new instance, resting in `Added` — the only state the lifecycle starts one in.
    pub fn new(data: FormData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `surf.document.Form` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`FormSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: FormState,
    /// What it holds.
    pub data: FormData,
}

/// An `Form` in whichever declared state it was found.
pub enum AnyForm {
    /// Resting in `Added`.
    Added(Form<form_state::Added>),
}

impl FormSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `FormState` cannot spell one.
    pub fn refine(self) -> AnyForm {
        match self.state {
            FormState::Added => AnyForm::Added(Form {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyForm {
    /// The state, as the runtime value.
    pub fn state(&self) -> FormState {
        match self {
            Self::Added(_) => FormState::Added,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> FormSnapshot {
        match self {
            Self::Added(instance) => FormSnapshot {
                state: FormState::Added,
                data: instance.into_data(),
            },
        }
    }
}

/// What Option — `surf.document.Option` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`ListOption<S>`], and at a boundary by [`ListOptionSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListOptionData {
    /// The identity: `option_id` — `surf.document.OptionId`.
    pub option_id: OptionId,
    /// `select_id` — `surf.document.ElementId`.
    ///
    /// Carries `options`: `surf.document.Element` owns many `surf.document.Option`.
    pub select_id: ElementId,
    /// `position` — `Integer`.
    pub position: i64,
    /// `value` — `String`.
    pub value: String,
    /// `label` — `String`.
    pub label: String,
    /// `selected` — `Boolean`.
    pub selected: bool,
    /// `disabled` — `Boolean`.
    pub disabled: bool,
    /// `hidden` — `Boolean`.
    pub hidden: bool,
}

/// The states of `surf.document.Option`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](list_option_state::Marker), so [`ListOption<S>`](ListOption) can only ever rest in a real state.
pub mod list_option_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Added {}
    }

    /// A declared state of `ListOption`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::OptionState;
    }

    /// `Added`. Where a new instance starts.
    pub struct Added;

    impl Marker for Added {
        const STATE: super::OptionState = super::OptionState::Added;
    }
}

/// Option — `surf.document.Option` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Added`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ListOptionSnapshot`]
/// and [`ListOptionSnapshot::refine`].
pub struct ListOption<S: list_option_state::Marker> {
    data: ListOptionData,
    state: core::marker::PhantomData<S>,
}

impl<S: list_option_state::Marker> ListOption<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> OptionState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ListOptionData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ListOptionData {
        self.data
    }
}

impl ListOption<list_option_state::Added> {
    /// A new instance, resting in `Added` — the only state the lifecycle starts one in.
    pub fn new(data: ListOptionData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `surf.document.Option` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ListOptionSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListOptionSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: OptionState,
    /// What it holds.
    pub data: ListOptionData,
}

/// An `ListOption` in whichever declared state it was found.
pub enum AnyListOption {
    /// Resting in `Added`.
    Added(ListOption<list_option_state::Added>),
}

impl ListOptionSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `OptionState` cannot spell one.
    pub fn refine(self) -> AnyListOption {
        match self.state {
            OptionState::Added => AnyListOption::Added(ListOption {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyListOption {
    /// The state, as the runtime value.
    pub fn state(&self) -> OptionState {
        match self {
            Self::Added(_) => OptionState::Added,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ListOptionSnapshot {
        match self {
            Self::Added(instance) => ListOptionSnapshot {
                state: OptionState::Added,
                data: instance.into_data(),
            },
        }
    }
}

/// What Submission — `surf.document.Submission` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Submission<S>`], and at a boundary by [`SubmissionSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmissionData {
    /// The identity: `submission_id` — `surf.document.SubmissionId`.
    pub submission_id: SubmissionId,
    /// `submitter` — `surf.document.ElementId`.
    ///
    /// Carries `submitter`: `surf.document.Submission` references one `surf.document.Element`.
    pub submitter: ElementId,
}

/// The states of `surf.document.Submission`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](submission_state::Marker), so [`Submission<S>`](Submission) can only ever rest in a real state.
pub mod submission_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Abandoned {}
        impl Sealed for super::Started {}
        impl Sealed for super::Submitted {}
    }

    /// A declared state of `Submission`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SubmissionState;
    }

    /// `Abandoned`. Terminal: an instance may rest here forever.
    pub struct Abandoned;

    impl Marker for Abandoned {
        const STATE: super::SubmissionState = super::SubmissionState::Abandoned;
    }

    /// `Started`. Where a new instance starts.
    pub struct Started;

    impl Marker for Started {
        const STATE: super::SubmissionState = super::SubmissionState::Started;
    }

    /// `Submitted`. Terminal: an instance may rest here forever.
    pub struct Submitted;

    impl Marker for Submitted {
        const STATE: super::SubmissionState = super::SubmissionState::Submitted;
    }
}

/// Submission — `surf.document.Submission` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Started`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SubmissionSnapshot`]
/// and [`SubmissionSnapshot::refine`].
pub struct Submission<S: submission_state::Marker> {
    data: SubmissionData,
    state: core::marker::PhantomData<S>,
}

impl<S: submission_state::Marker> Submission<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SubmissionState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SubmissionData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SubmissionData {
        self.data
    }
}

impl Submission<submission_state::Started> {
    /// A new instance, resting in `Started` — the only state the lifecycle starts one in.
    pub fn new(data: SubmissionData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Submission<submission_state::Started> {
    /// `submit` — `Started` → `Submitted`. Taken by the `submitted` outcome of `surf.document.SubmitForm`.
    pub fn submit(self) -> Submission<submission_state::Submitted> {
        Submission {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `abandon` — `Started` → `Abandoned`. Taken by the `abandoned` outcome of `surf.document.AbandonSubmission`.
    pub fn abandon(self) -> Submission<submission_state::Abandoned> {
        Submission {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `surf.document.Submission` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SubmissionSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmissionSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SubmissionState,
    /// What it holds.
    pub data: SubmissionData,
}

/// An `Submission` in whichever declared state it was found.
pub enum AnySubmission {
    /// Resting in `Abandoned`.
    Abandoned(Submission<submission_state::Abandoned>),
    /// Resting in `Started`.
    Started(Submission<submission_state::Started>),
    /// Resting in `Submitted`.
    Submitted(Submission<submission_state::Submitted>),
}

impl SubmissionSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SubmissionState` cannot spell one.
    pub fn refine(self) -> AnySubmission {
        match self.state {
            SubmissionState::Abandoned => AnySubmission::Abandoned(Submission {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            SubmissionState::Started => AnySubmission::Started(Submission {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            SubmissionState::Submitted => AnySubmission::Submitted(Submission {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySubmission {
    /// The state, as the runtime value.
    pub fn state(&self) -> SubmissionState {
        match self {
            Self::Abandoned(_) => SubmissionState::Abandoned,
            Self::Started(_) => SubmissionState::Started,
            Self::Submitted(_) => SubmissionState::Submitted,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SubmissionSnapshot {
        match self {
            Self::Abandoned(instance) => SubmissionSnapshot {
                state: SubmissionState::Abandoned,
                data: instance.into_data(),
            },
            Self::Started(instance) => SubmissionSnapshot {
                state: SubmissionState::Started,
                data: instance.into_data(),
            },
            Self::Submitted(instance) => SubmissionSnapshot {
                state: SubmissionState::Submitted,
                data: instance.into_data(),
            },
        }
    }
}

/// Abandon a submission — the input of `surf.document.AbandonSubmission`.
///
/// End a submission without a navigation, with the reason.
///
/// Everything it can result in is [`AbandonSubmissionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbandonSubmission {
    /// `submission_id` — `surf.document.SubmissionId`.
    pub submission_id: SubmissionId,
    /// `reason` — `surf.document.SubmissionEndReason`.
    pub reason: SubmissionEndReason,
}

/// Everything `surf.document.AbandonSubmission` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbandonSubmissionOutcome {
    /// `no-such-submission` — for an identity no record carries.
    ///
    /// No submission has this identity.
    NoSuchSubmission {
        /// Why it was refused: `surf.document.SubmissionNotFound`.
        error: SubmissionNotFound,
    },
    /// `abandoned` — otherwise.
    ///
    /// The submission is Abandoned; nothing navigates.
    Abandoned {
        /// The `surf.document.SubmissionAbandoned` this outcome publishes.
        submission_abandoned: SubmissionAbandoned,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The submission had already ended.
    WrongState {
        /// Why it was refused: `surf.document.SubmissionStateConflict`.
        error: SubmissionStateConflict,
    },
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
    /// `readonly` — `Boolean`.
    pub readonly: bool,
    /// `name` — `Optional<String>`.
    pub name: Option<String>,
    /// `form_id` — `Optional<surf.document.FormId>`.
    pub form_id: Option<FormId>,
    /// `radio_group` — `Optional<String>`.
    pub radio_group: Option<String>,
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

/// Add a form — the input of `surf.document.AddForm`.
///
/// Record one form of a document being parsed.
///
/// Everything it can result in is [`AddFormOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddForm {
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: DocumentId,
    /// `position` — `Integer`.
    pub position: i64,
    /// `action` — `Optional<String>`.
    pub action: Option<String>,
    /// `method` — `Optional<String>`.
    pub method: Option<String>,
    /// `enctype` — `Optional<String>`.
    pub enctype: Option<String>,
}

/// Everything `surf.document.AddForm` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddFormOutcome {
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
    /// The form is a row of its document.
    Added {
        /// The `surf.document.FormAdded` this outcome publishes.
        form_added: FormAdded,
    },
}

/// Add an option — the input of `surf.document.AddOption`.
///
/// Record one option of a select of a document being parsed.
///
/// Everything it can result in is [`AddOptionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddOption {
    /// `select_id` — `surf.document.ElementId`.
    pub select_id: ElementId,
    /// `position` — `Integer`.
    pub position: i64,
    /// `value` — `String`.
    pub value: String,
    /// `label` — `String`.
    pub label: String,
    /// `selected` — `Boolean`.
    pub selected: bool,
    /// `disabled` — `Boolean`.
    pub disabled: bool,
    /// `hidden` — `Boolean`.
    pub hidden: bool,
}

/// Everything `surf.document.AddOption` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddOptionOutcome {
    /// `no-such-select` — when no `surf.document.Element` carries the identity `input.select_id` names.
    ///
    /// No element has this identity; nothing is added.
    NoSuchSelect {
        /// Why it was refused: `surf.document.ElementNotFound`.
        error: ElementNotFound,
    },
    /// `not-a-select` — when the `surf.document.Element` that `input.select_id` names satisfies `kind != select`.
    ///
    /// The element is not a select; nothing is added.
    NotASelect {
        /// Why it was refused: `surf.document.ControlKindConflict`.
        error: ControlKindConflict,
    },
    /// `added` — otherwise.
    ///
    /// The option is a row of its select.
    Added {
        /// The `surf.document.OptionAdded` this outcome publishes.
        option_added: OptionAdded,
    },
}

/// Check a control — the input of `surf.document.Check`.
///
/// Toggle a checkbox, or check a radio button and uncheck the others of its group.
///
/// Everything it can result in is [`CheckOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: ElementId,
}

/// Everything `surf.document.Check` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutcome {
    /// `no-such-element` — for an identity no record carries.
    ///
    /// No element has this identity.
    NoSuchElement {
        /// Why it was refused: `surf.document.ElementNotFound`.
        error: ElementNotFound,
    },
    /// `not-checkable` — when the existing subject's stored fields satisfy `(kind != checkbox and kind != radio)`.
    ///
    /// The element is neither a checkbox nor a radio button; nothing changed.
    NotCheckable {
        /// Why it was refused: `surf.document.ControlKindConflict`.
        error: ControlKindConflict,
    },
    /// `disabled` — when the existing subject's stored fields satisfy `disabled == true`.
    ///
    /// The control is disabled; nothing changed.
    Disabled {
        /// Why it was refused: `surf.document.ControlDisabled`.
        error: ControlDisabled,
    },
    /// `hidden` — when the existing subject's stored fields satisfy `hidden == true`.
    ///
    /// The control is hidden; nothing changed.
    Hidden {
        /// Why it was refused: `surf.document.ControlHidden`.
        error: ControlHidden,
    },
    /// `radio-checked` — when the existing subject's stored fields satisfy `kind == radio`.
    ///
    /// The radio button is checked and the others of its group are not.
    RadioChecked {
        /// The `surf.document.CheckednessSet` this outcome publishes.
        checkedness_set: CheckednessSet,
    },
    /// `unchecked` — when the existing subject's stored fields satisfy `checked == true`.
    ///
    /// The checked checkbox is unchecked.
    Unchecked {
        /// The `surf.document.CheckednessSet` this outcome publishes.
        checkedness_set: CheckednessSet,
    },
    /// `checked` — otherwise.
    ///
    /// The unchecked checkbox is checked.
    Checked {
        /// The `surf.document.CheckednessSet` this outcome publishes.
        checkedness_set: CheckednessSet,
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
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
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

/// Select an option — the input of `surf.document.SelectOption`.
///
/// Select one option of a select and unselect the others.
///
/// Everything it can result in is [`SelectOptionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectOption {
    /// `option_id` — `surf.document.OptionId`.
    pub option_id: OptionId,
}

/// Everything `surf.document.SelectOption` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectOptionOutcome {
    /// `no-such-option` — for an identity no record carries.
    ///
    /// No option has this identity.
    NoSuchOption {
        /// Why it was refused: `surf.document.OptionNotFound`.
        error: OptionNotFound,
    },
    /// `disabled` — when the existing subject's stored fields satisfy `disabled == true`.
    ///
    /// The option or its select is disabled; nothing changed.
    Disabled {
        /// Why it was refused: `surf.document.ControlDisabled`.
        error: ControlDisabled,
    },
    /// `hidden` — when the existing subject's stored fields satisfy `hidden == true`.
    ///
    /// The select is hidden; nothing changed.
    Hidden {
        /// Why it was refused: `surf.document.ControlHidden`.
        error: ControlHidden,
    },
    /// `selected` — otherwise.
    ///
    /// The option is selected and the other options of its select are not.
    Selected {
        /// The `surf.document.OptionSelected` this outcome publishes.
        option_selected: OptionSelected,
    },
}

/// Set a control's value — the input of `surf.document.SetValue`.
///
/// Give a text, password or textarea control a value.
///
/// Everything it can result in is [`SetValueOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetValue {
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: ElementId,
    /// `value` — `String`.
    pub value: String,
}

/// Everything `surf.document.SetValue` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetValueOutcome {
    /// `no-such-element` — for an identity no record carries.
    ///
    /// No element has this identity.
    NoSuchElement {
        /// Why it was refused: `surf.document.ElementNotFound`.
        error: ElementNotFound,
    },
    /// `not-text-entry` — when the existing subject's stored fields satisfy `(kind != text and kind != password and kind != textarea)`.
    ///
    /// The element takes no typed value; nothing changed.
    NotTextEntry {
        /// Why it was refused: `surf.document.ControlKindConflict`.
        error: ControlKindConflict,
    },
    /// `disabled` — when the existing subject's stored fields satisfy `disabled == true`.
    ///
    /// The control is disabled; nothing changed.
    Disabled {
        /// Why it was refused: `surf.document.ControlDisabled`.
        error: ControlDisabled,
    },
    /// `hidden` — when the existing subject's stored fields satisfy `hidden == true`.
    ///
    /// The control is hidden; nothing changed.
    Hidden {
        /// Why it was refused: `surf.document.ControlHidden`.
        error: ControlHidden,
    },
    /// `readonly` — when the existing subject's stored fields satisfy `readonly == true`.
    ///
    /// The control is read-only; nothing changed.
    Readonly {
        /// Why it was refused: `surf.document.ControlReadonly`.
        error: ControlReadonly,
    },
    /// `set` — otherwise.
    ///
    /// The control holds the value.
    Set {
        /// The `surf.document.ValueSet` this outcome publishes.
        value_set: ValueSet,
    },
}

/// Start a submission — the input of `surf.document.StartSubmission`.
///
/// Activate a button, which starts the submission of its form.
///
/// Everything it can result in is [`StartSubmissionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartSubmission {
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: ElementId,
}

/// Everything `surf.document.StartSubmission` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartSubmissionOutcome {
    /// `no-such-element` — when no `surf.document.Element` carries the identity `input.element_id` names.
    ///
    /// No element has this identity; nothing started.
    NoSuchElement {
        /// Why it was refused: `surf.document.ElementNotFound`.
        error: ElementNotFound,
    },
    /// `not-a-button` — when the `surf.document.Element` that `input.element_id` names satisfies `kind != button`.
    ///
    /// The element is not a button; nothing started.
    NotAButton {
        /// Why it was refused: `surf.document.ControlKindConflict`.
        error: ControlKindConflict,
    },
    /// `disabled` — when the `surf.document.Element` that `input.element_id` names satisfies `disabled == true`.
    ///
    /// The button is disabled; nothing started.
    Disabled {
        /// Why it was refused: `surf.document.ControlDisabled`.
        error: ControlDisabled,
    },
    /// `hidden` — when the `surf.document.Element` that `input.element_id` names satisfies `hidden == true`.
    ///
    /// The button is hidden; nothing started.
    Hidden {
        /// Why it was refused: `surf.document.ControlHidden`.
        error: ControlHidden,
    },
    /// `started` — otherwise.
    ///
    /// The submission is Started.
    Started {
        /// The `surf.document.SubmissionStarted` this outcome publishes.
        submission_started: SubmissionStarted,
    },
}

/// Submit a form — the input of `surf.document.SubmitForm`.
///
/// End a submission with the request it navigates to.
///
/// Everything it can result in is [`SubmitFormOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitForm {
    /// `submission_id` — `surf.document.SubmissionId`.
    pub submission_id: SubmissionId,
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `method` — `surf.net.Method`.
    pub method: crate::net::Method,
    /// `body` — `Optional<Bytes>`.
    pub body: Option<Vec<u8>>,
    /// `body_type` — `Optional<surf.net.MediaType>`.
    pub body_type: Option<crate::net::MediaType>,
}

/// Everything `surf.document.SubmitForm` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitFormOutcome {
    /// `no-such-submission` — for an identity no record carries.
    ///
    /// No submission has this identity.
    NoSuchSubmission {
        /// Why it was refused: `surf.document.SubmissionNotFound`.
        error: SubmissionNotFound,
    },
    /// `submitted` — otherwise.
    ///
    /// The submission is Submitted, and its tab is to navigate to the request.
    Submitted {
        /// The `surf.document.FormSubmitted` this outcome publishes.
        form_submitted: FormSubmitted,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The submission had already ended.
    WrongState {
        /// Why it was refused: `surf.document.SubmissionStateConflict`.
        error: SubmissionStateConflict,
    },
}

/// CheckednessSet — the event `surf.document.CheckednessSet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckednessSet {
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: ElementId,
    /// `checked` — `Boolean`.
    pub checked: bool,
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

/// FormAdded — the event `surf.document.FormAdded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormAdded {
    /// `form_id` — `surf.document.FormId`.
    pub form_id: FormId,
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: DocumentId,
    /// `position` — `Integer`.
    pub position: i64,
}

/// FormSubmitted — the event `surf.document.FormSubmitted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormSubmitted {
    /// `submission_id` — `surf.document.SubmissionId`.
    pub submission_id: SubmissionId,
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
    /// `method` — `surf.net.Method`.
    pub method: crate::net::Method,
    /// `body` — `Optional<Bytes>`.
    pub body: Option<Vec<u8>>,
    /// `body_type` — `Optional<surf.net.MediaType>`.
    pub body_type: Option<crate::net::MediaType>,
}

/// OptionAdded — the event `surf.document.OptionAdded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionAdded {
    /// `option_id` — `surf.document.OptionId`.
    pub option_id: OptionId,
    /// `select_id` — `surf.document.ElementId`.
    pub select_id: ElementId,
    /// `position` — `Integer`.
    pub position: i64,
}

/// OptionSelected — the event `surf.document.OptionSelected`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionSelected {
    /// `option_id` — `surf.document.OptionId`.
    pub option_id: OptionId,
    /// `select_id` — `surf.document.ElementId`.
    pub select_id: ElementId,
}

/// ParsingStarted — the event `surf.document.ParsingStarted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsingStarted {
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: DocumentId,
    /// `body` — `Bytes`.
    pub body: Vec<u8>,
}

/// SubmissionAbandoned — the event `surf.document.SubmissionAbandoned`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmissionAbandoned {
    /// `submission_id` — `surf.document.SubmissionId`.
    pub submission_id: SubmissionId,
    /// `reason` — `surf.document.SubmissionEndReason`.
    pub reason: SubmissionEndReason,
}

/// SubmissionStarted — the event `surf.document.SubmissionStarted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmissionStarted {
    /// `submission_id` — `surf.document.SubmissionId`.
    pub submission_id: SubmissionId,
    /// `submitter` — `surf.document.ElementId`.
    pub submitter: ElementId,
}

/// ValueSet — the event `surf.document.ValueSet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueSet {
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: ElementId,
}

/// The declared error `surf.document.ControlDisabled`.
///
/// The control is disabled, so nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlDisabled {
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: ElementId,
}

/// The declared error `surf.document.ControlHidden`.
///
/// The control is hidden, so nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlHidden {
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: ElementId,
}

/// The declared error `surf.document.ControlKindConflict`.
///
/// The element is not of a kind this command acts on, so nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlKindConflict {
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: ElementId,
}

/// The declared error `surf.document.ControlReadonly`.
///
/// The control is read-only, so nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlReadonly {
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: ElementId,
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

/// The declared error `surf.document.ElementNotFound`.
///
/// No element has this identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementNotFound {
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: ElementId,
}

/// The declared error `surf.document.OptionNotFound`.
///
/// No option has this identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionNotFound {
    /// `option_id` — `surf.document.OptionId`.
    pub option_id: OptionId,
}

/// The declared error `surf.document.SubmissionNotFound`.
///
/// No submission has this identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmissionNotFound {
    /// `submission_id` — `surf.document.SubmissionId`.
    pub submission_id: SubmissionId,
}

/// The declared error `surf.document.SubmissionStateConflict`.
///
/// The submission already ended, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmissionStateConflict {
    /// `state` — `surf.document.Submission.State`.
    pub state: SubmissionState,
}

/// Controls — one row of the view `surf.document.Controls`.
///
/// Projects `surf.document.Element` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Controls {
    /// `element_id` — `surf.document.ElementId`.
    pub element_id: ElementId,
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: DocumentId,
    /// `state` — `surf.document.Element.State`.
    pub state: ElementState,
    /// `kind` — `surf.document.ElementKind`.
    pub kind: ElementKind,
    /// `value` — `Optional<String>`.
    pub value: Option<String>,
    /// `hidden` — `Boolean`.
    pub hidden: bool,
    /// `disabled` — `Boolean`.
    pub disabled: bool,
    /// `checked` — `Boolean`.
    pub checked: bool,
    /// `name` — `Optional<String>`.
    pub name: Option<String>,
    /// `form_id` — `Optional<surf.document.FormId>`.
    pub form_id: Option<FormId>,
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
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: crate::shell::TabId,
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
    /// `readonly` — `Boolean`.
    pub readonly: bool,
    /// `name` — `Optional<String>`.
    pub name: Option<String>,
    /// `form_id` — `Optional<surf.document.FormId>`.
    pub form_id: Option<FormId>,
}

/// Forms — one row of the view `surf.document.Forms`.
///
/// Projects `surf.document.Form` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Forms {
    /// `form_id` — `surf.document.FormId`.
    pub form_id: FormId,
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: DocumentId,
    /// `state` — `surf.document.Form.State`.
    pub state: FormState,
    /// `position` — `Integer`.
    pub position: i64,
    /// `action` — `Optional<String>`.
    pub action: Option<String>,
    /// `method` — `Optional<String>`.
    pub method: Option<String>,
    /// `enctype` — `Optional<String>`.
    pub enctype: Option<String>,
}

/// Options — one row of the view `surf.document.Options`.
///
/// Projects `surf.document.Option` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// `option_id` — `surf.document.OptionId`.
    pub option_id: OptionId,
    /// `select_id` — `surf.document.ElementId`.
    pub select_id: ElementId,
    /// `state` — `surf.document.Option.State`.
    pub state: OptionState,
    /// `position` — `Integer`.
    pub position: i64,
    /// `value` — `String`.
    pub value: String,
    /// `label` — `String`.
    pub label: String,
    /// `selected` — `Boolean`.
    pub selected: bool,
    /// `disabled` — `Boolean`.
    pub disabled: bool,
    /// `hidden` — `Boolean`.
    pub hidden: bool,
}

/// Submissions — one row of the view `surf.document.Submissions`.
///
/// Projects `surf.document.Submission` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submissions {
    /// `submission_id` — `surf.document.SubmissionId`.
    pub submission_id: SubmissionId,
    /// `submitter` — `surf.document.ElementId`.
    pub submitter: ElementId,
    /// `state` — `surf.document.Submission.State`.
    pub state: SubmissionState,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `surf.document.AbandonSubmission` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait AbandonSubmissionBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.document.AbandonSubmission`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn abandon_submission(&mut self, input: super::AbandonSubmission) -> Result<super::AbandonSubmissionOutcome, crate::obligation::UnmetObligation>;
    }

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

    /// The behaviour `surf.document.AddForm` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait AddFormBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.document.AddForm`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn add_form(&mut self, input: super::AddForm) -> Result<super::AddFormOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.document.AddOption` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait AddOptionBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.document.AddOption`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn add_option(&mut self, input: super::AddOption) -> Result<super::AddOptionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.document.Check` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by effects on filtered rows (`affects:`), in `radio-checked`.
    ///
    /// Contract: given `surf.document.Check` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `no-such-element` for an identity no record carries, error `surf.document.ElementNotFound`; `not-checkable` when the existing subject's stored fields satisfy `(kind != checkbox and kind != radio)`, error `surf.document.ControlKindConflict`; `disabled` when the existing subject's stored fields satisfy `disabled == true`, error `surf.document.ControlDisabled`; `hidden` when the existing subject's stored fields satisfy `hidden == true`, error `surf.document.ControlHidden`; `radio-checked` when the existing subject's stored fields satisfy `kind == radio`, updates `surf.document.Element`, changes, beside its subject, every `surf.document.Element` where `(kind == radio and document_id == subject.document_id and radio_group == subject.radio_group)`, setting `checked`, emits `surf.document.CheckednessSet`; `unchecked` when the existing subject's stored fields satisfy `checked == true`, updates `surf.document.Element`, emits `surf.document.CheckednessSet`; `checked` otherwise, updates `surf.document.Element`, emits `surf.document.CheckednessSet`.
    pub trait CheckBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.document.Check`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn check(&mut self, input: super::Check) -> Result<super::CheckOutcome, crate::obligation::UnmetObligation>;
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

    /// The behaviour `surf.document.SelectOption` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by effects on filtered rows (`affects:`), in `selected`.
    ///
    /// Contract: given `surf.document.SelectOption` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `no-such-option` for an identity no record carries, error `surf.document.OptionNotFound`; `disabled` when the existing subject's stored fields satisfy `disabled == true`, error `surf.document.ControlDisabled`; `hidden` when the existing subject's stored fields satisfy `hidden == true`, error `surf.document.ControlHidden`; `selected` otherwise, updates `surf.document.Option`, changes, beside its subject, every `surf.document.Option` where `select_id == subject.select_id`, setting `selected`, emits `surf.document.OptionSelected`.
    pub trait SelectOptionBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.document.SelectOption`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn select_option(&mut self, input: super::SelectOption) -> Result<super::SelectOptionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.document.SetValue` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait SetValueBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.document.SetValue`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn set_value(&mut self, input: super::SetValue) -> Result<super::SetValueOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.document.StartSubmission` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait StartSubmissionBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.document.StartSubmission`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn start_submission(&mut self, input: super::StartSubmission) -> Result<super::StartSubmissionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.document.SubmitForm` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait SubmitFormBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.document.SubmitForm`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn submit_form(&mut self, input: super::SubmitForm) -> Result<super::SubmitFormOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `surf.document.Controls` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ControlsQuery {
        /// Serves `surf.document.Controls` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn controls(&self) -> Result<Vec<super::Controls>, crate::obligation::UnmetObligation>;
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

    /// The query `surf.document.Forms` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait FormsQuery {
        /// Serves `surf.document.Forms` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn forms(&self) -> Result<Vec<super::Forms>, crate::obligation::UnmetObligation>;
    }

    /// The query `surf.document.Options` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait OptionsQuery {
        /// Serves `surf.document.Options` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn options(&self) -> Result<Vec<super::Options>, crate::obligation::UnmetObligation>;
    }

    /// The query `surf.document.Submissions` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait SubmissionsQuery {
        /// Serves `surf.document.Submissions` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn submissions(&self) -> Result<Vec<super::Submissions>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl CheckBehavior for Unimplemented {
        fn check(&mut self, _input: super::Check) -> Result<super::CheckOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "surf.document.Check" })
        }
    }

    impl SelectOptionBehavior for Unimplemented {
        fn select_option(&mut self, _input: super::SelectOption) -> Result<super::SelectOptionOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "surf.document.SelectOption" })
        }
    }
}
