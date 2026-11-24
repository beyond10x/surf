//! Review cases of the runner for `story:scripts-engine-dependency`: the limits of a runner and
//! where one script's end may land, the mapping of swell's ends onto the words of Q7, and inputs
//! the module `engine` does not give. Each runner is made in the test mode with limits of the
//! test's own. The first case shows a difference with the note; the cases marked "Held" record
//! what held.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use surf_script::{
    MESSAGE_CHARS, Mode, Reason, Runner, RunnerLimits, ScriptEnd, ScriptErrorKind, run,
};
use swell::Limits;

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
        steps_per_runner: u64::MAX,
        scripts_per_runner: u32::MAX,
        source_bytes: usize::MAX,
        wall_clock: Duration::from_secs(30),
    }
}

fn with_runner<R: Send>(limits: RunnerLimits, body: impl FnOnce(&mut Runner) -> R + Send) -> R {
    run(limits, Mode::Test, body).expect("a runner")
}

fn end(source: &str) -> ScriptEnd {
    with_runner(limits(), |runner| runner.evaluate(source))
}

/// After the wall clock ended a script, each later script of the runner is `not run:
/// wall-clock` and is not counted (`docs/design/scripts.md`, Q4, the row "wall clock per
/// document": "the running script: `limit: wall-clock`; each later script: `not run:
/// wall-clock`"; `Runner::scripts`: "a script not run is not counted"). The runner sets the
/// interrupt before the later script and runs its parse, so it ends `limit: wall-clock` and
/// counts it.
#[test]
fn a_script_after_the_wall_clock_is_not_run() {
    let mut limits = limits();
    limits.engine.steps_per_entry = u64::MAX;
    limits.wall_clock = Duration::from_millis(50);
    let (ends, scripts) = with_runner(limits, |runner| {
        let ends = [runner.evaluate("while (true) {}"), runner.evaluate("1")];
        (ends, runner.scripts())
    });
    assert_eq!(
        ends,
        [
            ScriptEnd::Limit(Reason::WallClock),
            ScriptEnd::NotRun(Reason::WallClock)
        ]
    );
    assert_eq!(scripts, 1);
}

/// Held: no script after the deadline runs to `ran`, whatever word it gets.
#[test]
fn no_script_after_the_deadline_runs() {
    let mut limits = limits();
    limits.engine.steps_per_entry = u64::MAX;
    limits.wall_clock = Duration::from_millis(50);
    let ends = with_runner(limits, |runner| {
        [
            runner.evaluate("while (true) {}"),
            runner.evaluate("1"),
            runner.evaluate("1 +;"),
            runner.evaluate("1"),
        ]
    });
    assert_eq!(ends[0], ScriptEnd::Limit(Reason::WallClock));
    for later in &ends[1..] {
        assert!(
            matches!(
                later,
                ScriptEnd::Limit(Reason::WallClock) | ScriptEnd::NotRun(Reason::WallClock)
            ),
            "{ends:?}"
        );
    }
}

/// Held: each of swell's seven error kinds keeps its own word; none is read as `Error` or
/// `value`.
#[test]
fn every_error_kind_keeps_its_word() {
    let kinds = [
        ("Error", ScriptErrorKind::Error),
        ("EvalError", ScriptErrorKind::EvalError),
        ("RangeError", ScriptErrorKind::RangeError),
        ("ReferenceError", ScriptErrorKind::ReferenceError),
        ("SyntaxError", ScriptErrorKind::SyntaxError),
        ("TypeError", ScriptErrorKind::TypeError),
        ("URIError", ScriptErrorKind::UriError),
    ];
    let ends = with_runner(limits(), |runner| {
        kinds
            .iter()
            .map(|(name, _)| runner.evaluate(&format!("throw new {name}(\"m\")")))
            .collect::<Vec<_>>()
    });
    for ((name, kind), end) in kinds.iter().zip(&ends) {
        let ScriptEnd::Threw(error) = end else {
            panic!("{name} ended {end}");
        };
        assert_eq!(error.kind, *kind, "{end}");
        assert_eq!(end.to_string(), format!("threw {name} \"m\" 1:1"));
    }
}

/// Held: a message is cut to `MESSAGE_CHARS` characters, and an unpaired surrogate is
/// replaced.
#[test]
fn a_long_message_is_cut_and_a_lone_surrogate_replaced() {
    let ends = with_runner(limits(), |runner| {
        [
            runner.evaluate(
                "var s = \"ab\"; for (var i = 0; i < 10; i = i + 1) s = s + s; throw new Error(s);",
            ),
            runner.evaluate("throw new Error(\"\\uD800x\")"),
        ]
    });
    let ScriptEnd::Threw(long) = &ends[0] else {
        panic!("{}", ends[0]);
    };
    let message = long.message.as_deref().expect("a message");
    assert_eq!(message.chars().count(), MESSAGE_CHARS);
    assert_eq!(message, "ab".repeat(MESSAGE_CHARS / 2));
    let ScriptEnd::Threw(lone) = &ends[1] else {
        panic!("{}", ends[1]);
    };
    assert_eq!(lone.message.as_deref(), Some("\u{FFFD}x"));
}

/// Held: a throw in eval code is named as such in the end's words.
#[test]
fn an_error_in_eval_code_names_its_site_as_eval() {
    let end = end("eval(\"1;\\nthrow new Error('e')\")");
    let ScriptEnd::Threw(error) = &end else {
        panic!("{end}");
    };
    assert!(error.site.is_some_and(|s| s.dynamic), "{end}");
    assert!(
        end.to_string().starts_with("threw Error \"e\" eval "),
        "{end}"
    );
}

/// Held: empty, blank and comment-only sources run, and one source run again runs again.
#[test]
fn empty_blank_and_repeated_sources_run() {
    let sources = [
        "",
        " \n\t",
        "// a comment",
        "/* */",
        "\u{FEFF}1",
        "{ let x = 1; }",
        "{ let x = 1; }",
        "{ let x = 1; }",
    ];
    let ends = with_runner(limits(), |runner| {
        sources.map(|source| runner.evaluate(source))
    });
    assert!(ends.iter().all(|e| *e == ScriptEnd::Ran), "{ends:?}");
}

/// Held: a script the step limit ended adds at most its budget to the runner's total, and a
/// script not run is not counted.
#[test]
fn a_step_limited_script_adds_at_most_its_budget() {
    let mut limits = limits();
    limits.engine.steps_per_entry = 100_000;
    let (end, steps, scripts) = with_runner(limits, |runner| {
        let end = runner.evaluate("while (true) {}");
        (end, runner.steps(), runner.scripts())
    });
    assert_eq!(end, ScriptEnd::Limit(Reason::StepsPerScript));
    assert!(steps > 0 && steps <= 100_000, "{steps}");
    assert_eq!(scripts, 1);

    let grow = "{ const a = []; for (;;) a.push(new Array(1000)); }";
    let mut limits = self::limits();
    limits.engine.memory_bytes = 1024 * 1024;
    limits.engine.steps_per_entry = u64::MAX;
    let (ends, scripts) = with_runner(limits, |runner| {
        let ends = [
            runner.evaluate(grow),
            runner.evaluate(grow),
            runner.evaluate("1"),
        ];
        (ends, runner.scripts())
    });
    assert_eq!(ends[2], ScriptEnd::NotRun(Reason::Memory), "{ends:?}");
    assert_eq!(scripts, 2);
}

/// The threads of this process (`/proc/self/status`).
fn threads() -> usize {
    std::fs::read_to_string("/proc/self/status")
        .expect("read /proc/self/status")
        .lines()
        .find_map(|l| l.strip_prefix("Threads:"))
        .and_then(|n| n.trim().parse().ok())
        .expect("a Threads line")
}

/// Held: 100 runners, half of them never started, leave no thread behind: each runner's thread
/// and its clock's thread end when the runner is dropped. The count is read right after each
/// runner is dropped, and a started runner's clock (30 s) is still waiting then, so a clock
/// thread that is not joined on the drop is still counted (fix round 1, item 12 a).
#[test]
fn a_hundred_runners_leave_no_thread_behind() {
    let before = threads();
    for i in 0..100 {
        let end = with_runner(limits(), |runner| {
            if i % 2 == 0 {
                Some(runner.evaluate("1"))
            } else {
                None
            }
        });
        assert!(matches!(end, Some(ScriptEnd::Ran) | None), "{end:?}");
        assert_eq!(threads(), before, "after runner {i}");
    }
}

/// Held: the clock of one runner ends no script of another. Runner B runs an endless loop under
/// its own step limit; runner A, started once B's script is about to start, runs an endless loop
/// under a wall clock of 5 ms, which passes long before B's steps run out. B ends at its step
/// limit, A at its wall clock.
#[test]
fn the_wall_clock_of_one_runner_ends_no_script_of_another() {
    let (go, started) = mpsc::channel::<()>();
    let a_done = AtomicBool::new(false);
    let (a_end, b_end) = thread::scope(|scope| {
        let b = scope.spawn(|| {
            let mut limits = limits();
            limits.engine.steps_per_entry = 500_000;
            with_runner(limits, move |runner| {
                go.send(()).expect("send");
                runner.evaluate("while (true) {}")
            })
        });
        started.recv().expect("B started");
        let a = scope.spawn(|| {
            let mut limits = limits();
            limits.engine.steps_per_entry = u64::MAX;
            limits.wall_clock = Duration::from_millis(5);
            let end = with_runner(limits, |runner| runner.evaluate("while (true) {}"));
            a_done.store(true, Ordering::SeqCst);
            end
        });
        (a.join().expect("A"), b.join().expect("B"))
    });
    assert!(a_done.load(Ordering::SeqCst));
    assert_eq!(a_end, ScriptEnd::Limit(Reason::WallClock));
    assert_eq!(b_end, ScriptEnd::Limit(Reason::StepsPerScript));
}

/// Held: a source whose parse alone takes longer than the bound ends `limit: wall-clock`, not
/// `ran`: about 230 KB of declarations under a wall clock of 1 ms.
#[test]
fn a_source_that_parses_past_the_bound_ends_at_the_wall_clock() {
    let source: String = (0..15_000).map(|i| format!("var a{i} = {i};\n")).collect();
    let mut limits = limits();
    limits.wall_clock = Duration::from_millis(1);
    let end = with_runner(limits, |runner| runner.evaluate(&source));
    assert_eq!(end, ScriptEnd::Limit(Reason::WallClock));
}
