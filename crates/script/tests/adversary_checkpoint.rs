//! Review of `story:swell-0-9-0` (the pin at swell `0.10.0`): the microtask checkpoint after
//! each script, the clear of the queue after a limit, the error kind `AggregateError` and the
//! records of hostile thrown values. Each test makes its own runner with limits of its own.

use std::cell::Cell;
use std::time::Duration;

use surf_html::limits::Limits as ParserLimits;
use surf_html::{NodeData, NodeId, ParseOptions, parse_document};
use surf_script::bindings::{
    Attributes, DomCx, DomEnd, DomException, DomResult, Interface, Mutation, Reading, Selectors,
    Slices, TheWindow, Wrap, new_wrapper, wrapper_node,
};
use surf_script::{
    Document, Mode, Reason, Runner, RunnerLimits, ScriptEnd, ScriptErrorKind, run, run_document,
};
use swell::{Completion, Limits, Value};

use super::adversary_5;

/// Small limits: 4 MiB of heap, Strings up to 2^20 code units.
fn limits(steps_per_entry: u64) -> RunnerLimits {
    RunnerLimits {
        engine: Limits {
            steps_per_entry,
            memory_bytes: 4 << 20,
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

fn with_runner<R: Send>(limits: RunnerLimits, body: impl FnOnce(&mut Runner) -> R + Send) -> R {
    run(limits, Mode::Test, body).expect("a runner")
}

/// Allocates Strings of 2^17 code units and more, kept by a local array, until the memory
/// budget ends the run; nothing of it stays reachable after the end.
const EXHAUST: &str =
    "(function () { var a = []; for (;;) a.push(\"y\".repeat((1 << 17) + a.length)); })();";

/// The checkpoint is no script of its own: the count rises by one per script, whether its
/// checkpoint ran jobs, ran none, or ended at the step budget.
#[test]
fn a_checkpoint_counts_no_script() {
    let (ends, scripts) = with_runner(limits(100_000), |runner| {
        let ends = vec![
            runner.evaluate("Promise.resolve().then(function () { globalThis.c = 1; });"),
            runner.evaluate("1"),
            runner.evaluate("function f() { Promise.resolve().then(f); } f();"),
        ];
        (ends, runner.scripts())
    });
    assert_eq!(
        ends,
        [
            ScriptEnd::Ran,
            ScriptEnd::Ran,
            ScriptEnd::Limit(Reason::StepsPerScript)
        ]
    );
    assert_eq!(scripts, 3);
}

/// `limit: memory` of the script itself clears the queue too: the job the script queued before
/// it ran out of memory runs neither after it nor in the next script's checkpoint.
#[test]
fn a_memory_limit_drops_the_jobs() {
    let ends = with_runner(limits(100_000_000), |runner| {
        vec![
            runner.evaluate(&format!(
                "Promise.resolve().then(function () {{ globalThis.m = 1; }}); {EXHAUST}"
            )),
            runner.evaluate("1"),
            runner.evaluate("if (typeof m !== \"undefined\") throw new Error(\"m\")"),
        ]
    });
    assert_eq!(
        ends,
        [
            ScriptEnd::Limit(Reason::Memory),
            ScriptEnd::Ran,
            ScriptEnd::Ran
        ],
        "{ends:?}"
    );
}

/// After `limit: memory` the runner clears the queue first and then collects (Q4, HOST-200):
/// a megabyte that only a queued job's closure holds is freed by the runner's one collection,
/// so the live bytes after the script are those of the realm.
#[test]
fn the_clear_comes_before_the_collection() {
    let (baseline, end, live, collections) = with_runner(limits(100_000_000), |runner| {
        assert_eq!(runner.evaluate("1"), ScriptEnd::Ran);
        let agent = runner.agent_and_realm().0;
        let _ = agent.collect();
        let baseline = agent.live_bytes().expect("live bytes");
        let end = runner.evaluate(&format!(
            "(function () {{
               var hold = [];
               for (var i = 0; i < 8; i++) hold.push(\"h\".repeat((1 << 17) + i));
               Promise.resolve().then(function () {{ return hold.length; }});
             }})();
             {EXHAUST}"
        ));
        let live = runner.agent_and_realm().0.live_bytes().expect("live bytes");
        (baseline, end, live, runner.collections())
    });
    assert_eq!(end, ScriptEnd::Limit(Reason::Memory), "{end}");
    assert_eq!(collections, 1);
    assert!(
        live < baseline + (512 << 10),
        "{live} live bytes after the end, {baseline} before"
    );
}

/// A job that calls a host function which sets the interrupt and then throws (HOST-216) ends
/// the checkpoint at the interrupt: no `catch` of the job runs, the script ends
/// `limit: wall-clock`, the flag is clear after it, the queue is empty, and the next script's
/// own jobs run in its checkpoint.
#[test]
fn an_interrupt_in_a_job_ends_the_checkpoint_and_the_next_one_runs() {
    let (first, flag, second, third) = adversary_5::with(
        adversary_5::limits(16 << 20, 1 << 20),
        |runner| {
            let first = adversary_5::evaluate(
                runner,
                r#"Promise.resolve().then(function () {
                     try { document.getElementById("interrupt"); } catch (e) { G.caught = 1; }
                     G.after = 1;
                   });"#,
            );
            let flag = runner.agent_and_realm().0.interrupt_handle().is_set();
            let second = adversary_5::evaluate(
                runner,
                r#"check(typeof G.caught === "undefined" && typeof G.after === "undefined", "ran on");
                   Promise.resolve().then(function () { G.k = 1; });"#,
            );
            let third = adversary_5::evaluate(runner, r#"check(G.k === 1, "k");"#);
            (first, flag, second, third)
        },
    );
    assert_eq!(first, ScriptEnd::Limit(Reason::WallClock), "{first}");
    assert!(!flag, "the interrupt flag after the script");
    assert_eq!(second, ScriptEnd::Ran, "{second}");
    assert_eq!(third, ScriptEnd::Ran, "{third}");
}

/// Jobs that never end, in the forms swell `0.10.0` gives a page: an `async` function that
/// awaits in a loop, a thenable whose `then` loops, an async generator drained by `for await`
/// without end. Each ends `limit: steps-per-script` at the checkpoint's budget, the runner's
/// total grows by at most two entries' budgets, and the next script runs with an empty queue.
#[test]
fn endless_jobs_end_at_the_step_limit_and_leave_no_job() {
    let steps_per_entry = 100_000;
    let sources = [
        "(async function () { for (;;) await null; })();",
        "Promise.resolve({ then: function () { for (;;) {} } });",
        "(async function () { var g = (async function* () { for (;;) yield 1; })(); \
         for await (var v of g) {} })();",
    ];
    for source in sources {
        let (end, steps, next, after) = with_runner(limits(steps_per_entry), |runner| {
            let end = runner.evaluate(source);
            let steps = runner.steps();
            let next = runner.evaluate("1");
            (end, steps, next, runner.steps() - steps)
        });
        assert_eq!(
            end,
            ScriptEnd::Limit(Reason::StepsPerScript),
            "{source}: {end}"
        );
        assert!(steps <= 2 * steps_per_entry, "{source}: {steps} steps");
        assert_eq!(next, ScriptEnd::Ran, "{source}: {next}");
        assert!(after < 100, "{source}: the next script spent {after} steps");
    }
}

/// A job that queues 100,000 jobs: whatever end the checkpoint reaches, no job of it runs in a
/// later script's checkpoint.
#[test]
fn a_job_that_queues_many_jobs_leaves_none_for_the_next_script() {
    let ends = with_runner(limits(1_000_000), |runner| {
        vec![
            runner.evaluate(
                "globalThis.n = 0; Promise.resolve().then(function () { \
                 for (var i = 0; i < 100000; i++) Promise.resolve().then(function () { n++; }); });",
            ),
            runner.evaluate("globalThis.seen = n;"),
            runner.evaluate("if (n !== seen) throw new Error(\"ran on: \" + (n - seen))"),
        ]
    });
    assert!(
        matches!(ends[0], ScriptEnd::Ran | ScriptEnd::Limit(_)),
        "{}",
        ends[0]
    );
    assert_eq!(ends[1..], [ScriptEnd::Ran, ScriptEnd::Ran], "{ends:?}");
}

/// A rejection nobody handles, a `finally` on it and a job that throws: the script ends `ran`,
/// the `finally` runs in the checkpoint, and the next script runs.
#[test]
fn a_finally_on_an_unhandled_rejection_runs_in_the_checkpoint() {
    let ends = with_runner(limits(1_000_000), |runner| {
        vec![
            runner.evaluate(
                "Promise.reject(new Error(\"r\")).finally(function () { globalThis.f = 1; }); \
                 Promise.resolve().then(function () { throw 2; });",
            ),
            runner.evaluate("if (f !== 1) throw new Error(\"f\")"),
        ]
    });
    assert_eq!(ends, [ScriptEnd::Ran, ScriptEnd::Ran], "{ends:?}");
}

/// Hostile thrown values (Q7, Q16): accessors on `message`, `errors`, `stack`, `constructor`
/// and `name`, an `AggregateError` with 100,000 entries and with a cycle, a Symbol, an error
/// object without a prototype, an object made to look like an error. Each ends `threw` with
/// the kind of its [[ErrorData]] and only an own data `message`; no accessor of the page runs
/// while the record is made.
#[test]
fn hostile_thrown_values_end_with_a_record_and_run_no_code() {
    let cases: [(&str, ScriptErrorKind, Option<&str>); 8] = [
        (
            "var e = new AggregateError([1], \"m\"); Object.defineProperty(e, \"message\", \
             { get: function () { globalThis.ran = \"message\"; return \"x\"; } }); throw e;",
            ScriptErrorKind::AggregateError,
            None,
        ),
        (
            "var e = new AggregateError([], \"e\"); Object.defineProperty(e, \"errors\", \
             { get: function () { globalThis.ran = \"errors\"; throw 1; } }); throw e;",
            ScriptErrorKind::AggregateError,
            Some("e"),
        ),
        (
            "var e = new AggregateError([], \"c\"); e.errors.push(e); throw e;",
            ScriptErrorKind::AggregateError,
            Some("c"),
        ),
        ("throw Symbol(\"s\");", ScriptErrorKind::Value, None),
        (
            "var e = new TypeError(\"t\"); e.name = \"X\"; \
             Object.defineProperty(e, \"constructor\", { get: function () { globalThis.ran = \"constructor\"; } }); \
             Object.defineProperty(e, \"stack\", { get: function () { globalThis.ran = \"stack\"; } }); \
             throw e;",
            ScriptErrorKind::TypeError,
            Some("t"),
        ),
        (
            "throw Object.setPrototypeOf(new RangeError(\"r\"), null);",
            ScriptErrorKind::RangeError,
            Some("r"),
        ),
        (
            "function MyError(m) { this.message = m; } \
             MyError.prototype = Object.create(Error.prototype); throw new MyError(\"my\");",
            ScriptErrorKind::Value,
            None,
        ),
        (
            "Object.defineProperty(Error.prototype, \"message\", \
             { get: function () { globalThis.ran = \"proto\"; return \"p\"; } }); throw new Error();",
            ScriptErrorKind::Error,
            None,
        ),
    ];
    let (ends, check) = with_runner(limits(10_000_000), |runner| {
        let ends: Vec<ScriptEnd> = cases
            .iter()
            .map(|(source, _, _)| runner.evaluate(source))
            .collect();
        let check = runner.evaluate("if (typeof ran !== \"undefined\") throw new Error(ran)");
        (ends, check)
    });
    for ((source, kind, message), end) in cases.iter().zip(&ends) {
        let ScriptEnd::Threw(error) = end else {
            panic!("{source} ended {end}");
        };
        assert_eq!(error.kind, *kind, "{source}: {end}");
        assert_eq!(error.message.as_deref(), *message, "{source}: {end}");
    }
    assert_eq!(check, ScriptEnd::Ran, "{check}");
}

/// An `AggregateError` whose `errors` hold 100,000 entries: the record holds the kind and the
/// message and nothing of the entries.
#[test]
fn an_aggregate_error_of_many_entries_records_its_message() {
    let mut limits = limits(10_000_000);
    limits.engine.memory_bytes = 64 << 20;
    let end = with_runner(limits, |runner| {
        runner.evaluate("var a = []; a.length = 100000; throw new AggregateError(a, \"big\");")
    });
    assert_eq!(end.to_string(), "threw AggregateError \"big\" 1:32");
}

thread_local! {
    /// Set by `getElementById("arm")`: the next read of `Window.document` sets the interrupt
    /// and throws.
    static ARMED: Cell<bool> = const { Cell::new(false) };
}

/// A `Reading` slice whose `Window.document` getter, once armed, sets the agent's interrupt and
/// throws a `DOMException`: the flag is set at the end of the script, as the wall clock's
/// thread sets it when its bound passes between the script's last step and the runner's read
/// of the record.
struct ArmedReading;

impl Reading for ArmedReading {
    fn window_document(&mut self, cx: &mut DomCx<'_, '_>) -> DomResult<NodeId> {
        if ARMED.with(|armed| armed.replace(false)) {
            cx.interrupt().interrupt();
            return Err(DomEnd::Throw(DomException::new("m", "N")));
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
        if element_id == "arm" {
            ARMED.with(|armed| armed.set(true));
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

/// The wall clock's interrupt set after a script's last step and before the runner reads the
/// record of the `DOMException` it threw: the read takes the flag (`Runner::exception_text`),
/// and the checkpoint after the script then begins with no flag set, so the page's jobs run
/// after the wall clock's bound has passed, up to a whole `steps_per_entry`. An interrupt set
/// before the checkpoint must end the script at the wall clock with no job run, as it does for
/// a script that ended `ran` (the checkpoint's start reads the flag, HOST-198).
#[test]
fn an_interrupt_the_record_read_takes_still_ends_the_script_at_the_wall_clock() {
    let (first, flag, second) =
        run_document(limits(100_000_000), Mode::Test, armed_document, |runner| {
            let first = runner.evaluate(
                "var G = this; Promise.resolve().then(function () { G.job = 1; }); \
             G.document.getElementById(\"arm\"); G.document;",
            );
            let flag = runner.agent_and_realm().0.interrupt_handle().is_set();
            let second = runner.evaluate(
            "if (typeof job !== \"undefined\") throw new Error(\"a job ran after the interrupt\")",
        );
            (first, flag, second)
        })
        .expect("a runner with the bindings");
    assert!(!flag, "the interrupt flag after the script");
    assert_eq!(
        (first, second),
        (ScriptEnd::Limit(Reason::WallClock), ScriptEnd::Ran)
    );
}
