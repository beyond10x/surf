//! Review cases of the runner for `story:scripts-engine-dependency`: the limits of a runner and
//! where one script's end may land, the mapping of swell's ends onto the words of Q7, and inputs
//! the module `engine` does not give. Each runner is made in the test mode with limits of the
//! test's own. The first case shows a difference with the note; the cases marked "Held" record
//! what held.

use std::time::Duration;

use surf_script::{Mode, Reason, Runner, RunnerLimits, ScriptEnd, run};
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
