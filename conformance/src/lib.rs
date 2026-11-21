//! The synthesized ESS conformance suite, run against the Rust `surf` system in process.
//!
//! [`SurfTarget`] is the adapter between the runner of `ess-conformance` and the generated typed
//! API that the `surf` library assembles: it turns the runner's named values into typed commands,
//! and typed outcomes, view rows, published events and binding invocations back into named
//! values. [`run`] admits a suite's bytes, runs it against a target and returns the runner's
//! result with its `ess-conformance-report/2`.
//!
//! [`memory_target`] is the system the `surf` binary ships, over the in-memory ports of
//! `surf-ports`. A test assembles a system of its own with [`SurfTarget::new`], as the
//! planted-defect test does with a `Navigation` storage that drops every `put`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod target;
mod values;

pub use target::{ConformanceBrowser, SurfTarget, Unanswered};

use ess_conformance::target::{ConformanceTarget, ImplementationIdentity};
use ess_conformance::{AdmittedSuite, CountReport, ExecutedRun, Runner};
use surf::Browser;
use surf_ports::{
    ComponentPorts, DocumentPorts, NavigatorPorts, NetPorts, RendererPorts, ShellPorts,
};
use surf_types::behaviour::Generated;

/// The system the `surf` binary ships, as a target: the in-memory ports of `surf-ports`.
pub type MemoryTarget = SurfTarget<
    Generated<DocumentPorts>,
    Generated<NavigatorPorts>,
    Generated<NetPorts>,
    Generated<RendererPorts>,
    Generated<ShellPorts>,
>;

/// The system the `surf` binary ships, each scenario over fresh in-memory ports.
pub fn memory_target() -> MemoryTarget {
    SurfTarget::new(
        ImplementationIdentity::new("surf", "rust, in process, surf-ports in memory"),
        || Browser::from_ports(ComponentPorts::new(), Unanswered),
    )
}

/// A run of one suite: what the runner executed, and its `ess-conformance-report/2`.
pub struct Conformance {
    /// The runner's result, scenario by scenario.
    pub run: ExecutedRun,
    /// The count report over the exact suite bytes that ran.
    pub report: CountReport,
}

/// Admits `suite`, the bytes `task suite` writes, and runs it against `target`.
pub fn run(suite: &str, target: &impl ConformanceTarget) -> Result<Conformance, String> {
    let admitted = AdmittedSuite::from_json(suite).map_err(|error| error.to_string())?;
    let run = Runner::for_suite(admitted.suite()).run_admitted(&admitted, target);
    let report = CountReport::from_run(&run, &admitted).map_err(|error| error.to_string())?;
    Ok(Conformance { run, report })
}
