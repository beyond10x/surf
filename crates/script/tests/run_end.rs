//! An interrupt that is set when a script ends is the script's end: the runner asks whether
//! the wall clock ran out or the interrupt is set before it reads a thrown value and before
//! the checkpoint, and again after a checkpoint that emptied the queue. A test slice's
//! `Window.document` getter, once armed by `getElementById`, sets the agent's interrupt. swell
//! reads the flag at the next call, loop jump or return and after a function called from Rust,
//! so a throw right after the getter ends the run with the flag still set, while a script that
//! runs on to its end, or a job whose function answers, has it taken by swell. A clock that
//! ran out is the runner's own question: a bound of 0 makes it so at the first script's end.

use std::cell::Cell;
use std::time::Duration;

use surf_html::limits::{Limit, Limits as ParserLimits};
use surf_html::{NodeData, NodeId, ParseOptions, parse_document};
use surf_script::bindings::{
    Attributes, DomCx, DomEnd, DomException, DomResult, Interface, Mutation, Reading, Selectors,
    Slices, TheWindow, Wrap, end_with_limit, new_wrapper, wrapper_node,
};
use surf_script::{Document, Mode, Reason, Runner, RunnerLimits, ScriptEnd, run, run_document};
use swell::{Completion, Limits, Value};

/// What the next read of `Window.document` does.
#[derive(Clone, Copy)]
enum Arm {
    /// Answers the document.
    Off,
    /// Sets the interrupt and answers the document.
    Set,
    /// Sets the interrupt and throws a `DOMException`.
    Throw,
    /// Records the limit of the tree's nodes, as a write over it does, and throws a
    /// `DOMException`.
    LimitThrow,
}

thread_local! {
    static ARM: Cell<Arm> = const { Cell::new(Arm::Off) };
}

struct ArmedReading;

impl Reading for ArmedReading {
    fn window_document(&mut self, cx: &mut DomCx<'_, '_>) -> DomResult<NodeId> {
        match ARM.with(|arm| arm.replace(Arm::Off)) {
            Arm::Off => {}
            Arm::Set => cx.interrupt().interrupt(),
            Arm::Throw => {
                cx.interrupt().interrupt();
                return Err(DomEnd::Throw(DomException::new("m", "N")));
            }
            Arm::LimitThrow => {
                let _ = end_with_limit(cx, Limit::Nodes);
                return Err(DomEnd::Throw(DomException::new("m", "N")));
            }
        }
        Ok(cx.tree().root())
    }

    fn window_window(&mut self, _cx: &mut DomCx<'_, '_>) -> DomResult<TheWindow> {
        Ok(TheWindow)
    }

    fn document_get_element_by_id(
        &mut self,
        _cx: &mut DomCx<'_, '_>,
        _this: NodeId,
        element_id: String,
    ) -> DomResult<Option<NodeId>> {
        let arm = match element_id.as_str() {
            "set" => Arm::Set,
            "throw" => Arm::Throw,
            "limit-throw" => Arm::LimitThrow,
            _ => Arm::Off,
        };
        ARM.with(|cell| cell.set(arm));
        Ok(None)
    }
}

struct NoAttributes;
impl Attributes for NoAttributes {}
struct NoMutation;
impl Mutation for NoMutation {}
struct NoSelectors;
impl Selectors for NoSelectors {}
struct DocumentWrap;

impl Wrap for DocumentWrap {
    fn wrap(&mut self, cx: &mut DomCx<'_, '_>, node: NodeId) -> Completion<Value> {
        let interface = match cx.tree().data(node) {
            NodeData::Document => Interface::Document,
            _ => Interface::Element,
        };
        new_wrapper(cx, interface, node).map(Value::Object)
    }

    fn unwrap(&self, cx: &DomCx<'_, '_>, value: Value) -> Option<NodeId> {
        wrapper_node(cx, value)
    }
}

fn armed_document() -> Document {
    Document {
        tree: parse_document("<!DOCTYPE html><p>a</p>", &ParseOptions::default()),
        limits: ParserLimits::default(),
        slices: Slices {
            reading: Box::new(ArmedReading),
            attributes: Box::new(NoAttributes),
            mutation: Box::new(NoMutation),
            selectors: Box::new(NoSelectors),
            wrap: Box::new(DocumentWrap),
        },
    }
}

fn limits() -> RunnerLimits {
    RunnerLimits {
        engine: Limits {
            steps_per_entry: 10_000_000,
            memory_bytes: 16 << 20,
            frame_depth: 1_000,
            reentry_depth: 64,
            parser_nesting: 100,
            string_length: 1 << 20,
            bigint_bits: 1 << 10,
        },
        steps_per_runner: 100_000_000,
        scripts_per_runner: 100,
        source_bytes: 1 << 20,
        wall_clock: Duration::from_secs(20),
    }
}

/// The script queues one job that sets `job`.
const JOB: &str = "var G = this; Promise.resolve().then(function () { G.job = 1; }); ";

/// Throws when the job of [`JOB`] ran.
const NO_JOB: &str = "if (typeof job !== \"undefined\") throw new Error(\"a job ran\");";

/// The end of `source`, whether the interrupt is set after it, and the end of [`NO_JOB`] after
/// it.
fn ends(source: &str) -> (ScriptEnd, bool, ScriptEnd) {
    run_document(
        limits(),
        Mode::Test,
        armed_document,
        |runner: &mut Runner| {
            let end = runner.evaluate(source);
            let flag = runner.agent_and_realm().0.interrupt_handle().is_set();
            (end, flag, runner.evaluate(NO_JOB))
        },
    )
    .expect("a runner with the bindings")
}

/// A script whose last statement sets the interrupt and that then ends normally with an empty
/// queue ends `limit: wall-clock` (swell's read at the script's return takes the flag), and no
/// flag is left: the next script runs.
#[test]
fn a_normal_end_with_the_interrupt_set_ends_at_the_wall_clock() {
    let end = ends("var G = this; G.document.getElementById(\"set\"); G.document;");
    assert_eq!(
        end,
        (ScriptEnd::Limit(Reason::WallClock), false, ScriptEnd::Ran)
    );
}

/// The same with one job queued: the job does not run, the queue is cleared.
#[test]
fn a_normal_end_with_a_job_and_the_interrupt_set_runs_no_job() {
    let end = ends(&format!(
        "{JOB}G.document.getElementById(\"set\"); G.document;"
    ));
    assert_eq!(
        end,
        (ScriptEnd::Limit(Reason::WallClock), false, ScriptEnd::Ran)
    );
}

/// A thrown value that is no error object, a number, thrown while the interrupt is set: the
/// end is `limit: wall-clock`, no record is made and no job runs.
#[test]
fn a_thrown_number_with_the_interrupt_set_ends_at_the_wall_clock() {
    let end = ends(&format!(
        "{JOB}G.document.getElementById(\"set\"); G.document; throw 1;"
    ));
    assert_eq!(
        end,
        (ScriptEnd::Limit(Reason::WallClock), false, ScriptEnd::Ran)
    );
}

/// A script whose entry recorded a limit of the bindings and then threw: the end is that
/// limit's word, no job runs and the flag is withdrawn.
#[test]
fn a_recorded_limit_and_a_throw_end_at_the_limit_and_run_no_job() {
    let end = ends(&format!(
        "{JOB}G.document.getElementById(\"limit-throw\"); G.document;"
    ));
    assert_eq!(
        end,
        (ScriptEnd::Limit(Reason::Nodes), false, ScriptEnd::Ran)
    );
}

/// A script that ran and whose one job set the interrupt and then threw: swell reads the flag
/// after the job's function answered, the checkpoint ends there and the script ends
/// `limit: wall-clock`; no flag is left, so the next script runs.
#[test]
fn a_job_that_sets_the_interrupt_and_throws_ends_the_script_at_the_wall_clock() {
    let end = ends(
        "var G = this; Promise.resolve().then(function () { \
         G.document.getElementById(\"throw\"); G.document; });",
    );
    assert_eq!(
        end,
        (ScriptEnd::Limit(Reason::WallClock), false, ScriptEnd::Ran)
    );
}

/// A wall-clock bound of 0: the clock ran out when the first script ends, whether or not its
/// thread set the interrupt yet. The script, which ran to its end and queued one job, ends
/// `limit: wall-clock` with no checkpoint run, and the next script is `not run: wall-clock`.
#[test]
fn a_clock_that_ran_out_at_a_normal_end_ends_the_script_at_the_wall_clock() {
    let mut limits = limits();
    limits.wall_clock = Duration::ZERO;
    let ends = run(limits, Mode::Test, |runner| {
        [runner.evaluate(JOB), runner.evaluate("1")]
    })
    .expect("a runner");
    assert_eq!(
        ends,
        [
            ScriptEnd::Limit(Reason::WallClock),
            ScriptEnd::NotRun(Reason::WallClock)
        ]
    );
}
