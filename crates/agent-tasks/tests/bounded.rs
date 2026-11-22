//! `agent-tasks-serve --bounded`, the helper the model lane runs its model command with: the
//! command in a process group of its own, a bound in seconds, SIGKILL to the whole group when
//! the command ends or the bound passes. Each case starts a background child in the group and
//! checks by its PID that it is gone. And the model lane's script asks its sh for no job
//! control and nothing outside POSIX sh.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

fn helper() -> PathBuf {
    std::env::var_os("SURF_BIN_DIR")
        .map(PathBuf::from)
        .expect("SURF_BIN_DIR names the built programs, agent-tasks-serve among them")
        .join("agent-tasks-serve")
}

/// An empty state directory of its own.
fn state(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("bounded-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run the helper with `bound` on `sh -c <script>`, `$1` being the state directory.
fn bounded(bound: &str, state: &Path, script: &str) -> Output {
    Command::new(helper())
        .args(["--bounded", bound])
        .arg(state)
        .args(["--", "/bin/sh", "-c", script, "sh"])
        .arg(state)
        .stdin(Stdio::null())
        .output()
        .expect("agent-tasks-serve runs")
}

/// Whether process `pid` still runs: in /proc and not a zombie.
fn running(pid: u32) -> bool {
    match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => !matches!(
            stat.rsplit_once(')')
                .and_then(|(_, rest)| rest.split_whitespace().next()),
            Some("Z" | "X" | "x") | None
        ),
        Err(_) => false,
    }
}

/// The PID the command wrote to `<state>/child`; it is sent SIGKILL if it still runs.
fn child_left(state: &Path) -> Option<u32> {
    let pid: u32 = fs::read_to_string(state.join("child"))
        .expect("the command wrote its child's PID")
        .trim()
        .parse()
        .unwrap();
    if running(pid) {
        let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
        return Some(pid);
    }
    None
}

#[test]
fn a_command_that_leaves_a_child_leaves_no_process() {
    let state = state("child");
    let output = bounded(
        "5",
        &state,
        "/bin/sleep 30 < /dev/null > /dev/null 2>&1 &\necho $! > \"$1/child\"\necho out\nexit 3\n",
    );
    let left = child_left(&state);
    let passed = state.join("bound-passed").exists();
    let _ = fs::remove_dir_all(&state);
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "out\n");
    assert!(!passed, "the bound did not pass");
    assert_eq!(left, None, "the child of the command is left");
}

#[test]
fn a_command_over_its_bound_is_ended_with_its_group() {
    let state = state("over");
    let started = Instant::now();
    let output = bounded(
        "1",
        &state,
        "/bin/sleep 30 < /dev/null > /dev/null 2>&1 &\necho $! > \"$1/child\"\nexec /bin/sleep 30\n",
    );
    let took = started.elapsed();
    let left = child_left(&state);
    let passed = state.join("bound-passed").exists();
    let _ = fs::remove_dir_all(&state);
    assert!(passed, "{output:?}");
    assert!(took < Duration::from_secs(4), "the helper took {took:?}");
    assert_eq!(left, None, "the child of the command is left");
}

#[test]
fn a_command_that_exits_124_by_itself_did_not_pass_its_bound() {
    let state = state("exit-124");
    let output = bounded("5", &state, "echo $$ > \"$1/child\"\nexit 124\n");
    let passed = state.join("bound-passed").exists();
    let _ = fs::remove_dir_all(&state);
    assert_eq!(output.status.code(), Some(124), "{output:?}");
    assert!(!passed, "the bound did not pass");
}

#[test]
fn the_model_script_asks_its_sh_for_nothing_outside_posix() {
    let script = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../taskfiles/agent-tasks-model.sh"
    ))
    .unwrap();
    let code: Vec<&str> = script
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect();
    for word in [
        "set -m", "set -o", "timeout", "[[", "local ", "$'", "echo -e", "<(", ">(", "declare",
        "pipefail",
    ] {
        let found: Vec<&&str> = code.iter().filter(|line| line.contains(word)).collect();
        assert!(found.is_empty(), "{word:?} in {found:?}");
    }
}
