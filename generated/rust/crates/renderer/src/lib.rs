// generated from surf v1
// contract digest 153f12c9bc6989d7738d48778d89afd6338830f138bc654b26d0f1a53611f3d7
// do not edit: regenerate with `ess synthesize`

//! renderer — the `renderer` component of `surf` v1.
//!
//! Styles, lays out and paints a document into a frame.
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
    /// `surf.render.FramePainted`.
    FramePainted(surf_types::render::FramePainted),
}

/// renderer — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct Renderer<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> Renderer<B> {
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

impl<B> Renderer<B>
where
    B: surf_types::render::obligations::PaintFrameBehavior + surf_types::render::obligations::FramesQuery,
{
    /// Accepts `surf.render.PaintFrame`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn paint_frame(&mut self, input: surf_types::render::PaintFrame) -> Result<surf_types::render::PaintFrameOutcome, surf_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.paint_frame(input)?;
        match &outcome {
            surf_types::render::PaintFrameOutcome::InvalidViewport { .. } => {}
            surf_types::render::PaintFrameOutcome::Painted { frame_painted, .. } => {
                self.outbox.push(PublishedEvent::FramePainted(frame_painted.clone()));
            }
        }
        Ok(outcome)
    }

    /// Serves `surf.render.Frames` at `read_your_writes` consistency, from the owed projection.
    pub fn frames(&self) -> Result<Vec<surf_types::render::Frames>, surf_types::obligation::UnmetObligation> {
        self.behaviors.frames()
    }
}
