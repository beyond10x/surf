//! Second review pass on the runner for `story:scripts-engine-dependency` (fix round 1): the
//! scripts after the wall clock ran out. Held cases.

use std::time::Duration;

use surf_script::{Mode, Reason, RunnerLimits, ScriptEnd, run};
use swell::Limits;

/// Held: every script after the wall clock ran out is `not run: wall-clock`, whatever its
/// source, and adds neither a script nor a step to the runner's totals.
#[test]
fn scripts_after_the_deadline_add_nothing_to_the_totals() {
    let limits = RunnerLimits {
        engine: Limits {
            steps_per_entry: u64::MAX,
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
        wall_clock: Duration::from_millis(50),
    };
    let (first, after, later, totals) = run(limits, Mode::Test, |runner| {
        let first = runner.evaluate("while (true) {}");
        let after = (runner.steps(), runner.scripts());
        let later = ["1", "1 +;", "class A {}", "while (true) {}"].map(|s| runner.evaluate(s));
        (first, after, later, (runner.steps(), runner.scripts()))
    })
    .expect("a runner");
    assert_eq!(first, ScriptEnd::Limit(Reason::WallClock));
    for end in &later {
        assert_eq!(*end, ScriptEnd::NotRun(Reason::WallClock), "{later:?}");
        assert_eq!(end.to_string(), "not run: wall-clock");
    }
    assert_eq!(totals, after);
    assert_eq!(after.1, 1);
}
