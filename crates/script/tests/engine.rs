//! The runner of `surf-script` on swell `0.10.0`, no DOM (`story:scripts-engine-dependency`):
//! each end of Q7 the runner can reach from a script, the limits of Q4 at small values of the
//! test's own, the stack of Q5, the hooks of Q6, the microtask checkpoint after each script
//! (`story:swell-0-9-0`), and swell's HOST-17, HOST-152 and HOST-163 cases with surf's limits
//! on the runner's thread (`docs/design/scripts.md`). Each test makes its own runner, a realm
//! with the intrinsics and nothing else, in the test mode (a fixed clock and fixed seeds)
//! unless it tests the session's hooks.
//!
//! Two tests are ignored until swell reaches what they call (HOST-26, HOST-127).
//! Two claims of the runner no test holds yet, because swell `0.10.0` gives no way to see them
//! (review S5, fix round 1): that a fault stays set, so that each script after `faulted` ends
//! `not run: faulted` (no script makes swell fault on purpose), and that a runner of the test
//! mode hands its agent the seeds of [`Mode::seeds`] (no script reads `hash_key` or
//! `random_seed` before `Math.random`, HOST-26).

use std::cell::Cell;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use surf_script::{
    Mode, Reason, Runner, RunnerLimits, ScriptEnd, ScriptError, ScriptErrorKind, ScriptHost, Site,
    TEST_CLOCK_START, TEST_CLOCK_STEP, run,
};
use swell::{
    Agent, Brand, Completion, Cx, GlobalObjects, HookCx, Host, HostCall, HostFn, HostObjectSpec,
    Limits, ObjectRef, Outcome, Parsed, PropertyKey, Prototype, RealmRef, Source, Value,
};

/// The message swell refuses a class declaration at 1:1 with: `SyntaxError::refused("class",
/// 8, ..)` (`crates/syntax/src/parser/statement.rs:221 @ 0.10.0`), whose text is
/// `format!("{construct}{REFUSED}{slice}")` (`crates/syntax/src/error.rs:85 @ 0.10.0`),
/// "<construct> is not supported before slice <n>" (`crates/syntax/src/lib.rs:16 @ 0.10.0`),
/// to which the error object's message adds the position, `format!("{} ({}:{})", ..)`
/// (`crates/swell/src/script.rs:407 @ 0.10.0`).
const CLASS_REFUSED: &str = "class is not supported before slice 8 (1:1)";

/// The message of the `RangeError` a call past `frame_depth` throws
/// (`crates/swell/src/interp/budget.rs:58 @ 0.10.0`).
const FRAME_DEPTH_EXCEEDED: &str = "frame_depth exceeded";

/// The limits of the tests: small, so that no test reaches one by accident or runs long.
fn limits() -> RunnerLimits {
    RunnerLimits {
        engine: Limits {
            steps_per_entry: 1_000_000,
            memory_bytes: 16 * 1024 * 1024,
            frame_depth: 1_000,
            reentry_depth: 64,
            parser_nesting: 100,
            string_length: 1 << 20,
            bigint_bits: 1 << 10,
        },
        steps_per_runner: 10_000_000,
        scripts_per_runner: 100,
        source_bytes: 65_536,
        wall_clock: Duration::from_secs(5),
    }
}

/// The runner of `limits` in the test mode, `body` run on its thread.
fn with_runner<R: Send>(limits: RunnerLimits, body: impl FnOnce(&mut Runner) -> R + Send) -> R {
    run(limits, Mode::Test, body).expect("a runner")
}

/// The ends of `sources`, run one after another in one runner of `limits`.
fn ends(limits: RunnerLimits, sources: &[&str]) -> Vec<ScriptEnd> {
    with_runner(limits, |runner| {
        sources
            .iter()
            .map(|source| runner.evaluate(source))
            .collect()
    })
}

/// The one end of `source` in a runner of [`limits`].
fn end(source: &str) -> ScriptEnd {
    ends(limits(), &[source]).remove(0)
}

#[test]
fn ran() {
    let end = end("1 + 1");
    assert_eq!(end, ScriptEnd::Ran);
    assert_eq!(end.to_string(), "ran");
}

#[test]
fn threw() {
    let error = end("throw new Error(\"x\")");
    assert_eq!(
        error,
        ScriptEnd::Threw(ScriptError {
            kind: ScriptErrorKind::Error,
            message: Some("x".to_string()),
            site: Some(Site {
                line: 1,
                column: 1,
                dynamic: false
            }),
        })
    );
    assert_eq!(error.to_string(), "threw Error \"x\" 1:1");
    let value = end("throw 1");
    let ScriptEnd::Threw(thrown) = &value else {
        panic!("throw 1 ended {value}");
    };
    assert_eq!(thrown.kind, ScriptErrorKind::Value, "{value}");
    assert_eq!(thrown.message, None, "{value}");
    assert!(value.to_string().starts_with("threw value"), "{value}");
}

/// A parse error that is not a refusal ends `threw SyntaxError` with its message and site.
#[test]
fn threw_syntax() {
    let end = end("1 +;");
    let ScriptEnd::Threw(error) = &end else {
        panic!("1 +; ended {end}");
    };
    assert_eq!(error.kind, ScriptErrorKind::SyntaxError, "{end}");
    assert!(
        error.message.as_ref().is_some_and(|m| !m.is_empty()),
        "{end}"
    );
    assert_eq!(
        error.site.map(|s| (s.line, s.dynamic)),
        Some((1, false)),
        "{end}"
    );
    assert!(end.to_string().starts_with("threw SyntaxError \""), "{end}");
}

/// A class, a construct of swell's slice 8, is refused with its `SyntaxError`.
#[test]
fn refused() {
    let end = end("class A {}");
    assert_eq!(
        end,
        ScriptEnd::Refused(ScriptError {
            kind: ScriptErrorKind::SyntaxError,
            message: Some(CLASS_REFUSED.to_string()),
            site: Some(Site {
                line: 1,
                column: 1,
                dynamic: false
            }),
        })
    );
    assert_eq!(end.to_string(), format!("refused \"{CLASS_REFUSED}\" 1:1"));
}

#[test]
fn step_limit() {
    let mut limits = limits();
    limits.engine.steps_per_entry = 1_000_000;
    let ends = ends(limits, &["while (true) {}", "1"]);
    assert_eq!(
        ends,
        [ScriptEnd::Limit(Reason::StepsPerScript), ScriptEnd::Ran]
    );
    assert_eq!(ends[0].to_string(), "limit: steps-per-script");
}

#[test]
fn memory_limit() {
    let grow = "{ const a = []; for (;;) a.push(new Array(1000)); }";
    let mut limits = limits();
    limits.engine.memory_bytes = 1024 * 1024;
    limits.engine.steps_per_entry = u64::MAX;
    let (first, collections) = with_runner(limits, |runner| {
        let ends = [runner.evaluate(grow), runner.evaluate("1")];
        (ends, runner.collections())
    });
    assert_eq!(first, [ScriptEnd::Limit(Reason::Memory), ScriptEnd::Ran]);
    assert_eq!(first[0].to_string(), "limit: memory");
    assert_eq!(collections, 1);
    let second = ends(limits, &[grow, grow, "1"]);
    assert_eq!(
        second,
        [
            ScriptEnd::Limit(Reason::Memory),
            ScriptEnd::Limit(Reason::Memory),
            ScriptEnd::NotRun(Reason::Memory),
        ]
    );
    assert_eq!(second[2].to_string(), "not run: memory");
}

#[test]
fn frame_depth() {
    let end = end("function f(n) { return 1 + f(n + 1); } f(0)");
    let ScriptEnd::Threw(error) = &end else {
        panic!("the recursion ended {end}");
    };
    assert_eq!(error.kind, ScriptErrorKind::RangeError, "{end}");
    assert_eq!(
        error.message.as_deref(),
        Some(FRAME_DEPTH_EXCEEDED),
        "{end}"
    );
    assert!(
        end.to_string()
            .starts_with(&format!("threw RangeError \"{FRAME_DEPTH_EXCEEDED}\"")),
        "{end}"
    );
}

/// The wall clock of 100 ms ends an endless loop the step limit does not end: not before the
/// bound, and well within 10 s.
#[test]
fn wall_clock() {
    let mut limits = limits();
    limits.engine.steps_per_entry = u64::MAX;
    limits.wall_clock = Duration::from_millis(100);
    let started = Instant::now();
    let ends = ends(limits, &["while (true) {}"]);
    let elapsed = started.elapsed();
    assert_eq!(ends, [ScriptEnd::Limit(Reason::WallClock)]);
    assert_eq!(ends[0].to_string(), "limit: wall-clock");
    assert!(elapsed >= Duration::from_millis(100), "{elapsed:?}");
    assert!(elapsed < Duration::from_secs(10), "{elapsed:?}");
}

#[test]
fn totals() {
    let loop_of = "var i = 0; while (i < 100) i = i + 1;";
    let totals = with_runner(limits(), |runner| {
        let mut totals = vec![(runner.steps(), runner.scripts())];
        for _ in 0..3 {
            assert_eq!(runner.evaluate(loop_of), ScriptEnd::Ran);
            totals.push((runner.steps(), runner.scripts()));
        }
        totals
    });
    assert_eq!(totals[0], (0, 0));
    for pair in totals.windows(2) {
        assert!(pair[1].0 > pair[0].0, "steps {totals:?}");
        assert_eq!(pair[1].1, pair[0].1 + 1, "scripts {totals:?}");
    }
}

/// The runner's thread with the session's limits has `min_stack_bytes()` plus the margin of Q5:
/// 4,006,656 + 257 × 14,336 + 1,048,576 = 8,739,584 bytes.
#[test]
fn stack_size() {
    let session = RunnerLimits::SESSION;
    assert_eq!(session.engine.min_stack_bytes(), 4_006_656);
    assert_eq!(session.stack_margin(), 257 * 14_336 + 1_048_576);
    assert_eq!(session.stack_bytes(), 8_739_584);
    let made = run(session, Mode::Test, |runner| runner.stack_bytes()).expect("a runner");
    assert_eq!(made, 8_739_584);
}

/// Runs `f` inside the setup closure of a new realm of `agent`, the place a host meets a
/// `HookCx` outside a hook, as swell's `tests/hooks.rs` does, and answers what `f` returned.
fn in_hook<R>(
    agent: &mut Agent<ScriptHost>,
    f: impl FnOnce(&mut HookCx<'_, ScriptHost>, ObjectRef) -> R,
) -> R {
    let mut f = Some(f);
    let mut out = None;
    let mut setup =
        |cx: &mut HookCx<'_, ScriptHost>, _realm: RealmRef| -> Completion<GlobalObjects> {
            let global = cx
                .new_host_object(HostObjectSpec {
                    prototype: Prototype::Null,
                    brand: Brand(1),
                    data: None,
                    error_kind: None,
                })
                .expect("a host object")?;
            if let Some(f) = f.take() {
                out = Some(f(cx, global));
            }
            Ok(GlobalObjects { global, this: None })
        };
    let created = agent
        .create_realm(Some(&mut setup), None)
        .expect("create_realm");
    assert!(matches!(created, Outcome::Normal(_)), "{created:?}");
    out.expect("the setup ran")
}

/// The clock of the host's mode, twice.
fn two_clock_readings(mode: Mode) -> (i128, i128) {
    run(limits(), mode, |runner| {
        let (agent, _) = runner.agent_and_realm();
        in_hook(agent, |cx, global| {
            (
                ScriptHost::host_system_utc_epoch_nanoseconds(cx, global),
                ScriptHost::host_system_utc_epoch_nanoseconds(cx, global),
            )
        })
    })
    .expect("a runner")
}

/// The hooks of Q6, called from Rust: the test host's clock starts at its fixed value and
/// advances 1 ms per call, its time zone is "UTC" with the offset 0, its seeds are swell's test
/// values; the session host's clock reads the system clock, and two agents of a session get
/// different seeds.
#[test]
fn host_hooks() {
    let (first, second) = two_clock_readings(Mode::Test);
    assert_eq!(first, TEST_CLOCK_START);
    assert_eq!(second - first, 1_000_000);
    assert_eq!(TEST_CLOCK_STEP, 1_000_000);
    let (zone, offsets) = run(limits(), Mode::Test, |runner| {
        let (agent, _) = runner.agent_and_realm();
        in_hook(agent, |cx, _| {
            (
                ScriptHost::system_time_zone_identifier(cx),
                [0, TEST_CLOCK_START].map(|epoch_ns| {
                    ScriptHost::get_named_time_zone_offset_nanoseconds(cx, "UTC", epoch_ns)
                }),
            )
        })
    })
    .expect("a runner");
    assert_eq!(zone, "UTC");
    assert_eq!(offsets, [0, 0]);
    let seeds = Mode::Test.seeds().expect("test seeds");
    assert_eq!(seeds.hash_key, [7; 16]);
    assert_eq!(seeds.random_seed, [11; 32]);

    let before = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_nanos() as i128;
    let (now, _) = two_clock_readings(Mode::Session);
    assert!(
        (now - before).abs() < 1_000_000_000,
        "the session clock read {now}, the system clock {before}"
    );
    let one = Mode::Session.seeds().expect("session seeds");
    let two = Mode::Session.seeds().expect("session seeds");
    assert_ne!(one.hash_key, two.hash_key);
    assert_ne!(one.random_seed, two.random_seed);
    assert_ne!(one.hash_key, [7; 16]);
}

/// `Date.now()` reads the test clock, which advances 1 ms per call.
#[test]
fn clock_advances() {
    assert_eq!(
        end(
            "var a = Date.now(); var b = Date.now(); if (b - a !== 1) throw new Error(String(b - a));"
        ),
        ScriptEnd::Ran
    );
}

/// `Math.random()` gives the same first value in two runners of the test mode.
#[test]
#[ignore = "HOST-26"]
fn seeds_fixed() {
    let source = "throw new Error(String(Math.random()))";
    let one = end(source);
    let ScriptEnd::Threw(thrown) = &one else {
        panic!("the script ended {one}");
    };
    assert_eq!(thrown.kind, ScriptErrorKind::Error, "{one}");
    let value: f64 = thrown
        .message
        .as_deref()
        .and_then(|m| m.parse().ok())
        .unwrap_or_else(|| panic!("no number in {one}"));
    assert!((0.0..1.0).contains(&value), "{one}");
    assert_eq!(one, end(source));
}

/// The time zone is UTC with the offset 0.
#[test]
#[ignore = "HOST-127"]
fn time_zone() {
    assert_eq!(
        end("if (new Date(0).getTimezoneOffset() !== 0) throw new Error(\"zone\");"),
        ScriptEnd::Ran
    );
}

/// `eval` and `Function` are allowed (Q4).
#[test]
fn compile_strings() {
    assert_eq!(
        end(
            "if (eval(\"1\") !== 1 || Function(\"return 1\")() !== 1) throw new Error(\"compile\")"
        ),
        ScriptEnd::Ran
    );
}

/// An `AggregateError` keeps its own word and its message (Q7).
#[test]
fn aggregate_error() {
    let end = end("throw new AggregateError([], \"m\")");
    let ScriptEnd::Threw(thrown) = &end else {
        panic!("the throw ended {end}");
    };
    assert_eq!(thrown.kind, ScriptErrorKind::AggregateError, "{end}");
    assert_eq!(end.to_string(), "threw AggregateError \"m\" 1:1");
}

/// The kinds `error_kind` must know: every own property of the global object whose name ends in
/// `Error`, as the realm lists them. Each `new <name>([])` ends `threw` with the kind whose word
/// is `<name>`; a kind swell adds that the runner reads through its wildcard arm reads `Error`
/// and fails here, naming it.
#[test]
fn every_error_constructor_keeps_its_word() {
    let listed = end("throw new Error(Object.getOwnPropertyNames(globalThis)\
         .filter(function (n) { return /Error$/.test(n); }).join(\",\"))");
    let ScriptEnd::Threw(list) = &listed else {
        panic!("the list ended {listed}");
    };
    let names: Vec<&str> = list
        .message
        .as_deref()
        .unwrap_or_else(|| panic!("no list in {listed}"))
        .split(',')
        .collect();
    for name in [
        "AggregateError",
        "Error",
        "EvalError",
        "RangeError",
        "ReferenceError",
        "SyntaxError",
        "TypeError",
        "URIError",
    ] {
        assert!(names.contains(&name), "{name} is not in {names:?}");
    }
    let sources: Vec<String> = names
        .iter()
        .map(|name| format!("throw new {name}([])"))
        .collect();
    let sources: Vec<&str> = sources.iter().map(String::as_str).collect();
    let thrown = ends(limits(), &sources);
    let wrong: Vec<String> = names
        .iter()
        .zip(&thrown)
        .filter_map(|(name, end)| match end {
            ScriptEnd::Threw(error) if error.kind.word() == *name => None,
            other => Some(format!("{name}: {other}")),
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "kinds that do not keep their word: {wrong:?}"
    );
}

/// HTML's microtask checkpoint after a script that ran: its promise job runs before the next
/// script.
#[test]
fn checkpoint_runs_after_the_script() {
    assert_eq!(
        ends(
            limits(),
            &[
                "var x = 0; Promise.resolve().then(function () { x = 1; });",
                "if (x !== 1) throw new Error(\"x\")",
            ]
        ),
        [ScriptEnd::Ran, ScriptEnd::Ran]
    );
}

/// An `async` function's continuation is a job of the checkpoint.
#[test]
fn async_function_runs() {
    assert_eq!(
        ends(
            limits(),
            &[
                "(async function () { await null; globalThis.y = 2; })();",
                "if (y !== 2) throw new Error(\"y\")",
            ]
        ),
        [ScriptEnd::Ran, ScriptEnd::Ran]
    );
}

/// A script that threw gets its checkpoint too ("clean up after running script" on both paths).
#[test]
fn checkpoint_after_threw() {
    let after = ends(
        limits(),
        &[
            "Promise.resolve().then(function () { globalThis.z = 3; }); throw 1",
            "if (z !== 3) throw new Error(\"z\")",
        ],
    );
    let ScriptEnd::Threw(thrown) = &after[0] else {
        panic!("the script ended {}", after[0]);
    };
    assert_eq!(thrown.kind, ScriptErrorKind::Value, "{}", after[0]);
    assert_eq!(after[1], ScriptEnd::Ran, "{}", after[1]);
}

/// A job that throws and a rejection no handler takes leave the script's end as it was.
#[test]
fn job_throw_keeps_the_end() {
    assert_eq!(
        ends(
            limits(),
            &[
                "Promise.resolve().then(function () { throw new Error(\"j\"); });",
                "Promise.reject(new Error(\"r\"));",
            ]
        ),
        [ScriptEnd::Ran, ScriptEnd::Ran]
    );
}

/// The steps the flood's script spends, read off the first run (the runner's total before it
/// performed a checkpoint).
const FLOOD_SCRIPT_STEPS: u64 = 33;

/// The steps the flood's checkpoint spends, read off the first run: all of its
/// `steps_per_entry`, as swell's HOST-197 example reads after `StepBudget`.
const FLOOD_CHECKPOINT_STEPS: u64 = 100_000;

/// A job that queues itself ends the script at the checkpoint's step budget, which is
/// `steps_per_entry` of its own (HOST-193): the runner's total is the script's steps and the
/// checkpoint's (HOST-197), and the queue is cleared, so the next script runs.
#[test]
fn job_flood_ends_at_the_step_limit() {
    let mut limits = limits();
    limits.engine.steps_per_entry = 100_000;
    let (flood, steps, after) = with_runner(limits, |runner| {
        let flood = runner.evaluate("function f() { Promise.resolve().then(f); } f();");
        (flood, runner.steps(), runner.evaluate("1"))
    });
    assert_eq!(flood, ScriptEnd::Limit(Reason::StepsPerScript), "{flood}");
    assert_eq!(after, ScriptEnd::Ran, "{after}");
    assert!(
        steps <= FLOOD_SCRIPT_STEPS + limits.engine.steps_per_entry,
        "{steps} steps"
    );
    assert_eq!(steps, FLOOD_SCRIPT_STEPS + FLOOD_CHECKPOINT_STEPS);
}

/// A limit clears the queue: the job a script queued before its limit does not run in the next
/// script's checkpoint.
#[test]
fn limit_clears_the_queue() {
    let mut limits = limits();
    limits.engine.steps_per_entry = 100_000;
    assert_eq!(
        ends(
            limits,
            &[
                "Promise.resolve().then(function () { globalThis.w = 1; }); while (true) {}",
                "1",
                "if (typeof w !== \"undefined\") throw new Error(\"w\")",
            ]
        ),
        [
            ScriptEnd::Limit(Reason::StepsPerScript),
            ScriptEnd::Ran,
            ScriptEnd::Ran
        ]
    );
}

/// The wall clock ends a checkpoint as it ends a script (HOST-198).
#[test]
fn checkpoint_wall_clock() {
    let mut limits = limits();
    limits.engine.steps_per_entry = u64::MAX;
    limits.wall_clock = Duration::from_millis(100);
    let started = Instant::now();
    let (first, elapsed, later) = with_runner(limits, |runner| {
        let first = runner.evaluate("Promise.resolve().then(function () { while (true) {} });");
        (first, started.elapsed(), runner.evaluate("1"))
    });
    assert_eq!(first, ScriptEnd::Limit(Reason::WallClock), "{first}");
    assert!(elapsed < Duration::from_secs(10), "{elapsed:?}");
    assert_eq!(later, ScriptEnd::NotRun(Reason::WallClock), "{later}");
}

/// The memory budget ends a checkpoint as it ends a script: `limit: memory`, the queue cleared,
/// one collection, and the next script runs.
#[test]
fn checkpoint_memory() {
    let mut limits = limits();
    limits.engine.memory_bytes = 1024 * 1024;
    limits.engine.steps_per_entry = u64::MAX;
    let (first, collections, after) = with_runner(limits, |runner| {
        let first = runner.evaluate(
            "Promise.resolve().then(function () { var a = []; for (;;) a.push(new Array(1000)); });",
        );
        (first, runner.collections(), runner.evaluate("1"))
    });
    assert_eq!(first, ScriptEnd::Limit(Reason::Memory), "{first}");
    assert_eq!(collections, 1);
    assert_eq!(after, ScriptEnd::Ran, "{after}");
}

thread_local! {
    /// The realm `parse_deep` parses in, on the runner's thread.
    static REALM: Cell<Option<RealmRef>> = const { Cell::new(None) };
    /// How many times `parse_deep` parsed, on the runner's thread.
    static PARSED: Cell<u32> = const { Cell::new(0) };
}

/// A host function that calls its first argument through `cx.call`: one re-entry (swell's
/// `tests/support/stack.rs @ 0.10.0`, `reenter`).
fn reenter(cx: &mut Cx<'_, ScriptHost>, call: HostCall<'_>) -> Completion<Value> {
    let function = call.args.first().copied().unwrap_or(Value::Undefined);
    cx.call(function, Value::Undefined, &[]).expect("call")
}

/// A host function that parses `parser_nesting` nested brackets through `cx.parse_script`
/// (swell's `parse_deep`), which must answer `Parsed` or `Errors`, never `Terminated`.
fn parse_deep(cx: &mut Cx<'_, ScriptHost>, _call: HostCall<'_>) -> Completion<Value> {
    let realm = REALM.with(Cell::get).expect("a realm");
    let n = RunnerLimits::SESSION.engine.parser_nesting as usize;
    let source = format!("{}0{};", "(".repeat(n), ")".repeat(n));
    let parsed = cx
        .parse_script(realm, Source::Utf8(&source), None)
        .expect("parse_script");
    assert!(!matches!(parsed, Parsed::Terminated(_)), "{parsed:?}");
    PARSED.with(|p| p.set(p.get() + 1));
    Ok(Value::Undefined)
}

/// Defines the host function `steps` as the global `name` of the runner's realm.
fn define(runner: &mut Runner, name: &str, length: u32, steps: HostFn<ScriptHost>) {
    let (agent, realm) = runner.agent_and_realm();
    let global_root = agent.global_object(realm).expect("global");
    agent
        .with_cx(realm, |cx| {
            let realm_ref = cx.realm_ref(realm).expect("realm ref");
            REALM.with(|r| r.set(Some(realm_ref)));
            let function = cx
                .new_host_function(realm_ref, name, length, steps, false, None)
                .expect("host function")
                .expect("host function");
            let Value::Object(global) = cx.get(&global_root).expect("global") else {
                panic!("the global is no object");
            };
            let key = PropertyKey::String(cx.new_string(name).expect("name"));
            cx.set(global, key, Value::Object(function), true)
                .expect("set")
                .expect("set");
        })
        .expect("with_cx");
}

/// The `n` the scripts below count `f`'s runs in: `reentry_depth` + 1.
fn full_runs() -> u32 {
    RunnerLimits::SESSION.engine.reentry_depth + 1
}

/// Runs `deepest` with surf's limits on the runner's thread, `reenter` and `parseDeep` defined,
/// and asserts it ends in the `RangeError` of `reentry_depth` without a crash, and that `check`,
/// run after it, ends `ran`; answers how many times `parseDeep` parsed.
fn at_full_nesting(deepest: &str, check: &str) -> u32 {
    let (deep, after, parsed) = run(RunnerLimits::SESSION, Mode::Test, |runner| {
        define(runner, "reenter", 1, reenter);
        define(runner, "parseDeep", 0, parse_deep);
        let deep = runner.evaluate(deepest);
        let after = runner.evaluate(check);
        (deep, after, PARSED.with(Cell::get))
    })
    .expect("a runner");
    let ScriptEnd::Threw(error) = &deep else {
        panic!("the deepest script ended {deep}");
    };
    assert_eq!(error.kind, ScriptErrorKind::RangeError, "{deep}");
    assert_eq!(after, ScriptEnd::Ran, "{after}");
    parsed
}

/// swell's HOST-17 (`host_17_deepest_reentry_parses_at_full_nesting`,
/// `crates/swell/tests/limits_scripts.rs:126 @ 0.10.0`) with surf's limits: a script reaches
/// `reentry_depth` re-entries through a host function, and at the deepest a host function
/// parses a source nested `parser_nesting` levels through `cx.parse_script`.
#[test]
fn host_17_reentry_at_full_nesting() {
    let deepest = "var n = 0, parsed = false;\n\
                   function f(k, v) {\n\
                     n++;\n\
                     try { reenter(f); }\n\
                     catch (e) { if (!parsed) { parsed = true; parseDeep(); } throw e; }\n\
                     return v;\n\
                   }\n\
                   f();";
    let check = format!("if (n !== {}) throw new Error(String(n));", full_runs());
    assert_eq!(
        at_full_nesting(deepest, &check),
        1,
        "parsed once at the deepest"
    );
}

/// The script of swell's `tests/eval_stack.rs @ 0.10.0`, `deepest`: `f` calls itself through
/// `reenter` until the re-entry past `reentry_depth` throws; the deepest `f` runs `at_deepest`
/// once. `deep` is a source nested `parser_nesting` levels.
fn eval_stack_script(at_deepest: &str) -> String {
    let n = RunnerLimits::SESSION.engine.parser_nesting as usize;
    format!(
        "var n = 0, done = false, kinds = \"\", ran = 0;\n\
         var deep = \"{}0{}\";\n\
         function kind(run) {{\n\
           try {{ return typeof run(); }} catch (e) {{ return e.name; }}\n\
         }}\n\
         function f() {{\n\
           n++;\n\
           try {{ reenter(f); }}\n\
           catch (e) {{\n\
             if (!done) {{ done = true; {at_deepest} ran++; }}\n\
             throw e;\n\
           }}\n\
         }}\n\
         f();",
        "(".repeat(n),
        ")".repeat(n)
    )
}

/// The check after an `eval_stack_script`: `f` ran `reentry_depth` + 1 times, the code at the
/// deepest ran once, and each kind it recorded matches `kinds`.
fn eval_stack_check(kinds: &str) -> String {
    format!(
        "if (n !== {} || ran !== 1 || !/^({kinds}) ({kinds})$/.test(kinds)) \
         throw new Error(n + \" \" + ran + \" \" + kinds);",
        full_runs()
    )
}

/// swell's HOST-152 (`host_152_deepest_reentry_evals_at_full_nesting`,
/// `crates/swell/tests/eval_stack.rs:173 @ 0.10.0`) with surf's limits: at the deepest re-entry
/// a direct and an indirect `eval` of a source nested `parser_nesting` levels, each answering a
/// Number or throwing a `SyntaxError` or a `RangeError`.
#[test]
fn host_152_eval_at_full_nesting() {
    at_full_nesting(
        &eval_stack_script(
            "kinds += kind(function () { return eval(deep); }) + \" \"; \
             kinds += kind(function () { return (0, eval)(deep); });",
        ),
        &eval_stack_check("number|SyntaxError|RangeError"),
    );
}

/// swell's HOST-163
/// (`host_163_deepest_reentry_calls_function_with_parameters_and_body_at_full_nesting`,
/// `crates/swell/tests/eval_stack.rs:201 @ 0.10.0`) with surf's limits: at the deepest re-entry
/// `Function(p, b)` with `p` and `b` each nested `parser_nesting` levels, and
/// `Function("a", "return " + b)`, each answering a function or throwing a `SyntaxError`.
#[test]
fn host_163_call_at_full_nesting() {
    at_full_nesting(
        &eval_stack_script(
            "kinds += kind(function () { return Function(deep, \"return \" + deep); }) + \" \"; \
             kinds += kind(function () { return Function(\"a\", \"return \" + deep); });",
        ),
        &eval_stack_check("function|SyntaxError"),
    );
}
