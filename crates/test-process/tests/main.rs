//! The tests of `surf-test-process`: the decision on `/proc/<pid>/stat` lines without a process
//! (`lines`), and the reads and the bounded wait on real children (`children`). The children are
//! `examples/sleeper.rs` of this package, not a program of the machine.

#![forbid(unsafe_code)]

mod lines {
    use surf_test_process::runs;

    /// The lines the two copies of `runs` of `surf-view-fixture` were given (the binary's and
    /// `tests/fixture.rs`'s, both added by `36e0dfb`), moved here unchanged with their name.
    #[test]
    fn a_stat_line_of_a_zombie_or_a_dead_process_is_one_of_an_ended_process() {
        for (line, running) in [
            ("4242 (surf-session) S 1 4242 4242 0 -1", true),
            ("4242 (surf-session) R 1 4242 4242 0 -1", true),
            ("4242 (surf-session) Z 1 4242 4242 0 -1", false),
            ("4242 (surf-session) X 1 4242 4242 0 -1", false),
            ("4242 (surf-session) x 1 4242 4242 0 -1", false),
            ("4242 (a) b) R 1 4242 4242 0 -1", true),
            ("4242 (a) b) Z 1 4242 4242 0 -1", false),
            ("", false),
        ] {
            assert_eq!(runs(line), running, "runs({line:?})");
        }
    }

    /// Every state `proc_pid_stat(5)` lists: `Z` (zombie), `X` (dead) and `x` (dead, Linux
    /// 2.6.33 to 3.13) are those of an ended process; `R`, `S`, `D`, `T`, `t`, `W` (paging
    /// before 2.6.0, waking 2.6.33 to 3.13), `K`, `P` and `I` are not.
    #[test]
    fn every_state_of_proc_pid_stat_is_decided() {
        let states = [
            ('R', true),
            ('S', true),
            ('D', true),
            ('Z', false),
            ('T', true),
            ('t', true),
            ('W', true),
            ('X', false),
            ('x', false),
            ('K', true),
            ('P', true),
            ('I', true),
        ];
        for (state, running) in states {
            let line = format!("4242 (surf-session) {state} 1 4242 4242 0 -1 4194560 0 0");
            assert_eq!(runs(&line), running, "runs({line:?})");
        }
    }

    /// The state is the first character after the last `)`: the command name may hold `)` and
    /// spaces, and a state letter inside it decides nothing.
    #[test]
    fn a_command_name_that_holds_a_parenthesis_and_spaces_is_skipped() {
        for (line, running) in [
            ("4242 (a) b c) R 1 4242", true),
            ("4242 (a) Z) R 1 4242", true),
            ("4242 (a) X x) S 1 4242", true),
            ("4242 (a) R) Z 1 4242", false),
            ("4242 (a) S ) X 1 4242", false),
            ("4242 ()) R 1 4242", true),
        ] {
            assert_eq!(runs(line), running, "runs({line:?})");
        }
    }

    /// No state to read is an ended process: the empty text, a line without `)`, and a line
    /// that ends at its `)`.
    #[test]
    fn a_line_without_a_state_is_one_of_an_ended_process() {
        for line in [
            "",
            "4242 (surf-session R 1 4242",
            "4242 R 1",
            "4242 (surf-session)",
            "4242 (surf-session)   ",
        ] {
            assert!(!runs(line), "runs({line:?})");
        }
    }
}

mod children {
    use std::fs;
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    use surf_test_process::{alive, ended_within, runs};

    /// `examples/sleeper` of this package, which the cargo running these tests built.
    fn sleeper() -> PathBuf {
        let exe = std::env::current_exe().expect("the test's executable");
        let path = exe
            .parent()
            .and_then(|deps| deps.parent())
            .expect("target/<profile>")
            .join("examples/sleeper");
        assert!(
            path.is_file(),
            "{} is not built; run cargo test -p surf-test-process",
            path.display()
        );
        path
    }

    /// A sleeper of `millis`, killed and waited for when dropped.
    struct Sleeper(Child);

    impl Sleeper {
        fn start(millis: u64) -> Sleeper {
            let child = Command::new(sleeper())
                .arg(millis.to_string())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("start the sleeper");
            Sleeper(child)
        }

        fn pid(&self) -> u32 {
            self.0.id()
        }
    }

    impl Drop for Sleeper {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    fn stat(pid: u32) -> String {
        fs::read_to_string(format!("/proc/{pid}/stat"))
            .unwrap_or_else(|error| panic!("/proc/{pid}/stat: {error}"))
    }

    fn state(stat: &str) -> Option<char> {
        stat.rsplit_once(')')
            .and_then(|(_, rest)| rest.trim_start().chars().next())
    }

    #[test]
    fn a_child_that_exits_and_is_waited_for_has_ended() {
        let mut child = Sleeper::start(0);
        let pid = child.pid();
        let status = child.0.wait().expect("wait for the sleeper");
        assert!(status.success(), "{status}");
        assert!(!alive(pid), "a reaped child reads as alive");
        // 1 s: the child is reaped, so the first read decides; the bound only ends a defect.
        let waited = ended_within(pid, Duration::from_secs(1)).expect("a reaped child has ended");
        assert!(waited < Duration::from_secs(1), "{waited:?}");
    }

    #[test]
    fn a_child_that_exits_and_is_not_waited_for_is_a_zombie_and_has_ended() {
        let mut child = Sleeper::start(0);
        let pid = child.pid();
        // 10 s: the time the sleeper may take to start and exit on a loaded machine.
        let waited = ended_within(pid, Duration::from_secs(10)).expect("a zombie has ended");
        assert!(waited < Duration::from_secs(10), "{waited:?}");
        let text = stat(pid);
        assert_eq!(
            state(&text),
            Some('Z'),
            "not waited for, the child is a zombie: {text:?}"
        );
        assert!(!runs(&text), "{text:?}");
        assert!(!alive(pid), "a zombie reads as alive: {text:?}");
        let status = child.0.wait().expect("reap the zombie");
        assert!(status.success(), "{status}");
    }

    #[test]
    fn a_child_that_sleeps_has_not_ended_within_a_short_bound_and_the_error_holds_its_stat() {
        let child = Sleeper::start(10_000);
        let pid = child.pid();
        assert!(
            alive(pid),
            "a sleeping child reads as ended: {:?}",
            stat(pid)
        );
        // 300 ms: far below the sleeper's 10 s, long enough for several reads.
        let bound = Duration::from_millis(300);
        let started = Instant::now();
        let error = ended_within(pid, bound).expect_err("a sleeping child has not ended");
        let took = started.elapsed();
        assert!(took >= bound, "the wait ended before its bound: {took:?}");
        assert!(
            took < Duration::from_secs(3),
            "the wait did not end at its bound: {took:?}"
        );
        let prefix = format!("{pid} (sleeper) ");
        let at = error
            .find(&prefix)
            .unwrap_or_else(|| panic!("the error holds no stat text of {pid}: {error}"));
        assert!(
            matches!(state(&error[at..]), Some('S' | 'R')),
            "the stat text in the error is not that of a sleeping or running process: {error}"
        );
        assert!(error.contains(&format!("/proc/{pid}/stat")), "{error}");
        assert!(alive(pid), "the sleeper ended: {:?}", stat(pid));
    }

    #[test]
    fn a_pid_that_never_existed_has_ended() {
        let max = fs::read_to_string("/proc/sys/kernel/pid_max").expect("/proc/sys/kernel/pid_max");
        // No process ever has the process id `pid_max`: the kernel gives ids below it.
        let pid: u32 = max.trim().parse().expect("a number");
        assert!(
            fs::metadata(format!("/proc/{pid}")).is_err(),
            "/proc/{pid} exists"
        );
        assert!(!alive(pid), "a pid that never existed reads as alive");
        let waited =
            ended_within(pid, Duration::from_secs(1)).expect("a pid that never existed has ended");
        assert!(waited < Duration::from_secs(1), "{waited:?}");
    }
}
