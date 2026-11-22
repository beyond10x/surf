// generated from surf v1
// contract digest 9cf563aeb85e0ebaf274c9cab36fdfa5c94c5872239a1808e0be06ff915e5e98
// do not edit: regenerate with `ess synthesize`

//! Semantic types synthesised from the `surf` specification, v1.
//!
//! A web browser built from scratch as a set of components that share no state: each holds its own rows, accepts commands and publishes events, and the bindings in `components.yaml` are the only coupling between them. This first slice is the page load: a tab is opened, a navigation is started, its address is fetched, and an HTML response is parsed into a document. Painting a frame is specified and nothing invokes it yet.
//!
//! Generated, not written: the specification is the source of truth, and the door to changing
//! anything here is `ess synthesize`. What is deliberately absent — behaviour, queries,
//! escalations — is listed with reasons in the `PLAN.md` beside this workspace, and every entry
//! there is owed through a typed seam in an `obligations` module here.

// `deny`, not the source workspace's lint set: this crate must hold on its own, and an undocumented
// public item here is an emitter defect worth failing the gate over.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod actor;
pub mod agent;
pub mod behaviour;
pub mod document;
pub mod navigation;
pub mod net;
pub mod obligation;
pub mod primitives;
pub mod render;
pub mod shell;
pub mod url;
