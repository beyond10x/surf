//! A second review of the rule "an interrupt that is set when a script ends is the script's
//! end": the states in which each half of the runner's question decides alone.
//!
//! A test slice's `Window.document` getter, armed by `getElementById`, does one of two things
//! at its next read. `set` sets the agent's interrupt and answers the document. `wait` waits
//! until the wall clock's thread set the interrupt, withdraws it and answers the document: the
//! state of a clock that ran out while its thread has not yet set the flag (the thread is
//! scheduled by the system, so the runner may ask before it ran), made without a race.
//! `getElementById("count")` counts one call, which shows that a job ran.

use std::cell::Cell;
use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{NodeData, NodeId, ParseOptions, parse_document};
use surf_script::bindings::{
    Attributes, DomCx, DomResult, Interface, Mutation, Reading, Selectors, Slices, TheWindow, Wrap,
    new_wrapper, wrapper_node,
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
    /// Waits for the wall clock's thread to set the interrupt, withdraws it and answers the
    /// document.
    Wait,
}

thread_local! {
    static ARM: Cell<Arm> = const { Cell::new(Arm::Off) };
    static COUNT: Cell<u32> = const { Cell::new(0) };
}

/// The longest wait of [`Arm::Wait`]: far beyond every bound these tests set.
const WAIT_CAP: Duration = Duration::from_secs(10);

struct ArmedReading;

impl Reading for ArmedReading {
    fn window_document(&mut self, cx: &mut DomCx<'_, '_>) -> DomResult<NodeId> {
        match ARM.with(|arm| arm.replace(Arm::Off)) {
            Arm::Off => {}
            Arm::Set => cx.interrupt().interrupt(),
            Arm::Wait => {
                let start = Instant::now();
                while !cx.interrupt().is_set() && start.elapsed() < WAIT_CAP {
                    std::thread::sleep(Duration::from_millis(1));
                }
                cx.interrupt().clear();
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
        match element_id.as_str() {
            "set" => ARM.with(|cell| cell.set(Arm::Set)),
            "wait" => ARM.with(|cell| cell.set(Arm::Wait)),
            "count" => COUNT.with(|count| count.set(count.get() + 1)),
            _ => ARM.with(|cell| cell.set(Arm::Off)),
        }
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

fn limits(wall_clock: Duration, steps_per_entry: u64) -> RunnerLimits {
    RunnerLimits {
        engine: Limits {
            steps_per_entry,
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
        wall_clock,
    }
}

/// The end of `source`, whether the interrupt is set after it, how many `count` calls ran in
/// it and its checkpoint, and the end of the script `1` after it.
fn ends(limits: RunnerLimits, source: &str) -> (ScriptEnd, bool, u32, ScriptEnd) {
    run_document(limits, Mode::Test, armed_document, |runner: &mut Runner| {
        ARM.with(|arm| arm.set(Arm::Off));
        COUNT.with(|count| count.set(0));
        let end = runner.evaluate(source);
        let flag = runner.agent_and_realm().0.interrupt_handle().is_set();
        let count = COUNT.with(Cell::get);
        (end, flag, count, runner.evaluate("1"))
    })
    .expect("a runner with the bindings")
}

/// `get` is the getter of `Window.document`, found on the global or its prototypes.
const GETTER: &str = "var G = this; var o = G, d; \
    while (!(d = Object.getOwnPropertyDescriptor(o, \"document\"))) o = Object.getPrototypeOf(o); \
    var get = d.get; ";

/// The checkpoint's last job is a thenable job whose `then` is the `Window.document` getter,
/// bound: swell calls it from Rust, and nothing reads the flag after a host function a job
/// called, so the checkpoint answers `Normal` with the interrupt set. Only the runner's
/// question after the checkpoint, through its flag half (the clock did not run out), ends the
/// script `limit: wall-clock` and withdraws the flag; without it the script ends `ran` and the
/// flag ends the next script before its first step.
#[test]
fn a_host_function_as_the_last_job_leaves_the_flag_to_the_runner() {
    let end = ends(
        limits(Duration::from_secs(20), 10_000_000),
        &format!(
            "{GETTER}G.document.getElementById(\"set\"); Promise.resolve({{ then: get.bind(G) }});"
        ),
    );
    assert_eq!(
        end,
        (
            ScriptEnd::Limit(Reason::WallClock),
            false,
            0,
            ScriptEnd::Ran
        )
    );
}

/// A clock that ran out while its thread has not set the flag, at the script's normal end with
/// one job queued: the runner's question before the checkpoint, through its clock half, ends
/// the script `limit: wall-clock`, and the job never runs. Without that question the
/// checkpoint runs the job (the flag is clear), and only the question after it ends the script.
#[test]
fn a_clock_run_out_before_its_thread_runs_no_job_of_the_script() {
    let end = ends(
        limits(Duration::from_millis(200), 10_000_000),
        "var G = this; Promise.resolve().then(function () { G.document.getElementById(\"count\"); }); \
         G.document.getElementById(\"wait\"); G.document;",
    );
    assert_eq!(
        end,
        (
            ScriptEnd::Limit(Reason::WallClock),
            false,
            0,
            ScriptEnd::NotRun(Reason::WallClock)
        )
    );
}

/// The same state after the last job: the job waits until the clock ran out, its thread's flag
/// is withdrawn and the job returns, so the checkpoint answers `Normal`. Only the question
/// after the checkpoint, through its clock half, ends the script `limit: wall-clock`.
#[test]
fn a_clock_run_out_before_its_thread_after_the_last_job_ends_the_script() {
    let end = ends(
        limits(Duration::from_millis(200), 10_000_000),
        "var G = this; Promise.resolve().then(function () { \
         G.document.getElementById(\"wait\"); G.document; G.document.getElementById(\"count\"); });",
    );
    assert_eq!(
        end,
        (
            ScriptEnd::Limit(Reason::WallClock),
            false,
            1,
            ScriptEnd::NotRun(Reason::WallClock)
        )
    );
}

/// After a script the rule ended `limit: wall-clock` (the interrupt set, then a `throw`), the
/// job it queued never runs: not in the next script's checkpoint, not in a later one. The
/// next script finds it gone, and so does the one after.
#[test]
fn a_wall_clock_end_by_the_rule_leaves_no_job_for_a_later_script() {
    let ends = run_document(
        limits(Duration::from_secs(20), 10_000_000),
        Mode::Test,
        armed_document,
        |runner: &mut Runner| {
            let first = runner.evaluate(
                "var G = this; Promise.resolve().then(function () { G.job = 1; }); \
                 G.document.getElementById(\"set\"); G.document; throw 1;",
            );
            let pending = runner.agent_and_realm().0.pending_microtasks();
            [
                format!("{first}"),
                format!("{pending:?}"),
                format!("{}", runner.evaluate("1")),
                format!(
                    "{}",
                    runner.evaluate(
                        "if (typeof job !== \"undefined\") throw new Error(\"a job ran\");"
                    )
                ),
            ]
        },
    )
    .expect("a runner with the bindings");
    assert_eq!(ends, ["limit: wall-clock", "Ok(0)", "ran", "ran"]);
}

/// The interrupt is set by a getter (a property get reads no flag after it) and the step limit
/// ends the script in straight-line code before swell reads the flag. The script ends
/// `limit: steps-per-script`; the clock did not run out, so by Q4 ("a flag set while the clock
/// had not run out leaves the next script to run") the next script runs. Red: the flag is left
/// set and ends the next script `limit: wall-clock` before its first step.
#[test]
fn a_step_limit_with_the_interrupt_set_leaves_no_flag_for_the_next_script() {
    let source = format!(
        "var G = this; let x = 0; G.document.getElementById(\"set\"); G.document; {}",
        "x = x + 1; ".repeat(5_000)
    );
    let end = ends(limits(Duration::from_secs(20), 1_000), &source);
    assert_eq!(
        end,
        (
            ScriptEnd::Limit(Reason::StepsPerScript),
            false,
            0,
            ScriptEnd::Ran
        )
    );
}
