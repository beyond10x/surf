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
//! planted-defect test does with a `Navigation` storage that drops every `put`. The assembly is
//! handed the scenario's [`Setup`]: the caller the target sends each command as, and the sessions
//! it establishes, for the `agent` component's ports ([`Setup::agent_ports`]), and the elements,
//! options and submissions it establishes, for the `document` component's
//! ([`Setup::document_ports`]).
//!
//! A planted-defect test runs its defect over the few scenarios that notice it, as the committed
//! list [`NOTICED`] names them ([`planted_suite`]), not over the whole suite; `task
//! conform-slow` runs every defect over the whole suite with [`NOTICED_OUT`] set, and
//! [`note_noticed`] writes the list again.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod target;
mod values;

pub use target::{ConformanceBrowser, Setup, SurfTarget, Unanswered, Unparsed};

use ess_conformance::target::{ConformanceTarget, ImplementationIdentity};
use std::collections::BTreeMap;

use ess_conformance::{AdmittedSuite, ConformanceSuite, CountReport, ExecutedRun, Runner};
use surf::Browser;
use surf_ports::{
    AgentPorts, ComponentPorts, DocumentPorts, NavigatorPorts, NetPorts, RendererPorts, ShellPorts,
    document_port,
};
use surf_types::behaviour::Generated;

/// The system the `surf` binary ships, as a target: the in-memory ports of `surf-ports`.
pub type MemoryTarget = SurfTarget<
    Generated<AgentPorts>,
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
        |setup| {
            let mut ports = ComponentPorts::with_agent(setup.agent_ports());
            ports.document = document_port(setup.document_ports());
            Browser::from_ports(ports, Unanswered)
        },
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

/// The list every planted-defect test of `tests/` reads, committed: on its first line `sources
/// <sha256>`, the digest of its sources, which `task conform-rust` compares before it runs the
/// tests; then one line per planted defect, `<label>: <id> <id> <id>`, the first
/// [`NOTICED_PER_DEFECT`] scenarios, in the suite's order, that do not pass over it.
pub const NOTICED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/noticed.txt");

/// The variable `task conform-slow` sets to a directory: each planted defect then runs over the
/// whole suite, and [`note_noticed`] writes its line of [`NOTICED`] there, one file per defect.
pub const NOTICED_OUT: &str = "SURF_NOTICED_OUT";

/// How many scenarios a line of [`NOTICED`] names at most.
pub const NOTICED_PER_DEFECT: usize = 3;

/// Whether [`NOTICED_OUT`] is set: the planted defects run over the whole suite.
pub fn writing_noticed() -> bool {
    std::env::var_os(NOTICED_OUT).is_some()
}

/// Admits `suite`, the bytes `task suite` writes, once for every planted defect of a test.
pub fn admit(suite: &str) -> Result<AdmittedSuite, String> {
    AdmittedSuite::from_json(suite).map_err(|error| error.to_string())
}

/// The suite the planted defect `label` runs over: the scenarios its line of [`NOTICED`] names,
/// or the whole of `admitted` when [`NOTICED_OUT`] is set. A defect without a line, or a line
/// that names a scenario the suite does not hold, is an error that names [`NOTICED`].
pub fn planted_suite(admitted: &AdmittedSuite, label: &str) -> Result<String, String> {
    if writing_noticed() {
        return Ok(admitted.original_json().to_owned());
    }
    let list = std::fs::read_to_string(NOTICED).map_err(|error| format!("{NOTICED}: {error}"))?;
    let ids: Vec<&str> = list
        .lines()
        .skip(1)
        .find_map(|line| line.strip_prefix(label)?.strip_prefix(": "))
        .ok_or_else(|| format!("{NOTICED}: no line for {label}; run task conform-slow"))?
        .split(' ')
        .collect();
    let whole = admitted.suite();
    let scenarios: BTreeMap<_, _> = whole
        .scenarios
        .iter()
        .filter(|(id, _)| ids.contains(&id.to_string().as_str()))
        .map(|(id, scenario)| (id.clone(), scenario.clone()))
        .collect();
    if scenarios.len() != ids.len() {
        return Err(format!(
            "{NOTICED}: {label}: the suite does not hold each of {ids:?}; run task conform-slow"
        ));
    }
    let suite = ConformanceSuite {
        provenance: whole.provenance.clone(),
        scenarios,
    };
    suite.to_canonical_json().map_err(|error| error.to_string())
}

/// When [`NOTICED_OUT`] is set, writes the line of [`NOTICED`] for the planted defect `label`
/// into that directory: the first [`NOTICED_PER_DEFECT`] scenarios of `report`, in the suite's
/// order, that did not pass. Writes nothing when it is not set.
pub fn note_noticed(label: &str, report: &CountReport) -> Result<(), String> {
    let Some(directory) = std::env::var_os(NOTICED_OUT) else {
        return Ok(());
    };
    let ids: Vec<String> = report
        .statuses()
        .into_iter()
        .filter(|(_, status)| *status != "passed")
        .map(|(scenario, _)| scenario)
        .take(NOTICED_PER_DEFECT)
        .collect();
    let name: String = label
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let path = std::path::Path::new(&directory).join(name);
    std::fs::write(&path, format!("{label}: {}\n", ids.join(" ")))
        .map_err(|error| format!("{}: {error}", path.display()))
}

/// `run` over each of `items`, in order: one after another, or each on a thread of its own when
/// [`NOTICED_OUT`] is set, since then each runs the whole suite.
pub fn each_planted<T: Sync, R: Send>(items: &[T], run: impl Fn(&T) -> R + Sync) -> Vec<R> {
    if !writing_noticed() {
        return items.iter().map(&run).collect();
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = items.iter().map(|item| scope.spawn(|| run(item))).collect();
        handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect()
    })
}
