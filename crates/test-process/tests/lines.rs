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
