// generated from surf v1
// contract digest 815b5846cd605d5ad5bf889b9e1b2b924cfbeb7d6931ec05ff659e3ea3fc4b79
// do not edit: regenerate with `ess synthesize`

//! Rendering — `surf.render`.
//!
//! Turning a document into the pixels of one frame.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// The states of `surf.render.Frame`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Frame<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameState {
    /// `Painted`.
    Painted,
}

/// FrameId — `surf.render.FrameId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameId(pub crate::primitives::Uuid);

/// What Frame — `surf.render.Frame` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Frame<S>`], and at a boundary by [`FrameSnapshot::state`].
///
/// Every value satisfies `width > 0` — checked by [`FrameData::broken_invariant`].
/// Every value satisfies `height > 0` — checked by [`FrameData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameData {
    /// The identity: `frame_id` — `surf.render.FrameId`.
    pub frame_id: FrameId,
    /// `document_id` — `surf.document.DocumentId`.
    ///
    /// Carries `document`: `surf.render.Frame` references one `surf.document.Document`.
    pub document_id: crate::document::DocumentId,
    /// `width` — `Integer`.
    pub width: i64,
    /// `height` — `Integer`.
    pub height: i64,
}

impl FrameData {
    /// The first declared invariant of `surf.render.Frame` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.width)), iv::Op::Gt, iv::Fact::number("0"), false, true)) {
            return Some("width > 0");
        }
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.height)), iv::Op::Gt, iv::Fact::number("0"), false, true)) {
            return Some("height > 0");
        }
        None
    }
}

/// The states of `surf.render.Frame`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](frame_state::Marker), so [`Frame<S>`](Frame) can only ever rest in a real state.
pub mod frame_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Painted {}
    }

    /// A declared state of `Frame`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::FrameState;
    }

    /// `Painted`. Where a new instance starts.
    pub struct Painted;

    impl Marker for Painted {
        const STATE: super::FrameState = super::FrameState::Painted;
    }
}

/// Frame — `surf.render.Frame` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Painted`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`FrameSnapshot`]
/// and [`FrameSnapshot::refine`].
pub struct Frame<S: frame_state::Marker> {
    data: FrameData,
    state: core::marker::PhantomData<S>,
}

impl<S: frame_state::Marker> Frame<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> FrameState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &FrameData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> FrameData {
        self.data
    }
}

impl Frame<frame_state::Painted> {
    /// A new instance, resting in `Painted` — the only state the lifecycle starts one in.
    pub fn new(data: FrameData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `surf.render.Frame` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`FrameSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: FrameState,
    /// What it holds.
    pub data: FrameData,
}

/// An `Frame` in whichever declared state it was found.
pub enum AnyFrame {
    /// Resting in `Painted`.
    Painted(Frame<frame_state::Painted>),
}

impl FrameSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `FrameState` cannot spell one.
    pub fn refine(self) -> AnyFrame {
        match self.state {
            FrameState::Painted => AnyFrame::Painted(Frame {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyFrame {
    /// The state, as the runtime value.
    pub fn state(&self) -> FrameState {
        match self {
            Self::Painted(_) => FrameState::Painted,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> FrameSnapshot {
        match self {
            Self::Painted(instance) => FrameSnapshot {
                state: FrameState::Painted,
                data: instance.into_data(),
            },
        }
    }
}

/// Paint a frame — the input of `surf.render.PaintFrame`.
///
/// Paint a document at a viewport size given in pixels.
///
/// Everything it can result in is [`PaintFrameOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaintFrame {
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: crate::document::DocumentId,
    /// `width` — `Integer`.
    pub width: i64,
    /// `height` — `Integer`.
    pub height: i64,
}

/// Everything `surf.render.PaintFrame` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaintFrameOutcome {
    /// `invalid-viewport` — when `(width <= 0 or height <= 0)`.
    ///
    /// The viewport has no area.
    InvalidViewport {
        /// Why it was refused: `surf.render.InvalidViewport`.
        error: InvalidViewport,
    },
    /// `painted` — otherwise.
    ///
    /// The frame exists.
    Painted {
        /// The `surf.render.FramePainted` this outcome publishes.
        frame_painted: FramePainted,
    },
}

/// FramePainted — the event `surf.render.FramePainted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramePainted {
    /// `frame_id` — `surf.render.FrameId`.
    pub frame_id: FrameId,
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: crate::document::DocumentId,
    /// `width` — `Integer`.
    pub width: i64,
    /// `height` — `Integer`.
    pub height: i64,
}

/// The declared error `surf.render.InvalidViewport`.
///
/// The viewport has no area, so nothing was painted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidViewport {
    /// `width` — `Integer`.
    pub width: i64,
    /// `height` — `Integer`.
    pub height: i64,
}

/// Frames — one row of the view `surf.render.Frames`.
///
/// Projects `surf.render.Frame` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frames {
    /// `frame_id` — `surf.render.FrameId`.
    pub frame_id: FrameId,
    /// `document_id` — `surf.document.DocumentId`.
    pub document_id: crate::document::DocumentId,
    /// `width` — `Integer`.
    pub width: i64,
    /// `height` — `Integer`.
    pub height: i64,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
pub mod obligations {
    /// The behaviour `surf.render.PaintFrame` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait PaintFrameBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.render.PaintFrame`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn paint_frame(&mut self, input: super::PaintFrame) -> Result<super::PaintFrameOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `surf.render.Frames` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait FramesQuery {
        /// Serves `surf.render.Frames` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn frames(&self) -> Result<Vec<super::Frames>, crate::obligation::UnmetObligation>;
    }

}
