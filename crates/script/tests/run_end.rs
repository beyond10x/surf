//! An interrupt that is set when a script ends is the script's end: the runner asks before it
//! reads a thrown value and before the checkpoint, and again after a checkpoint that emptied
//! the queue. A test slice's `Window.document` getter, once armed by `getElementById`, sets the
//! agent's interrupt at the end of a script or of a job, where the wall clock's thread sets it
//! when its bound passes there; a getter's answer is followed by no read of the flag, so it is
//! still set when the run answers.

use std::cell::Cell;
use std::time::Duration;

use surf_html::limits::{Limit, Limits as ParserLimits};
use surf_html::{NodeData, NodeId, ParseOptions, parse_document};
use surf_script::bindings::{
    Attributes, DomCx, DomEnd, DomException, DomResult, Interface, Mutation, Reading, Selectors,
    Slices, TheWindow, Wrap, end_with_limit, new_wrapper, wrapper_node,
};
use surf_script::{Document, Mode, Reason, Runner, RunnerLimits, ScriptEnd, run_document};
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

/// A script that ended normally with an empty queue while the interrupt is set ends
/// `limit: wall-clock`, and the flag is withdrawn: the next script runs.
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

/// A script that ran and whose one job set the interrupt and then threw: the checkpoint
/// emptied the queue and answered normally with the flag still set, and the script ends
/// `limit: wall-clock` with the flag withdrawn, so the next script runs.
#[test]
fn a_job_that_leaves_the_interrupt_set_ends_the_script_at_the_wall_clock() {
    let end = ends(
        "var G = this; Promise.resolve().then(function () { \
         G.document.getElementById(\"throw\"); G.document; });",
    );
    assert_eq!(
        end,
        (ScriptEnd::Limit(Reason::WallClock), false, ScriptEnd::Ran)
    );
}
