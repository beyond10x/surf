//! Second adversary pass, on what the two fix commits changed: `match row` on a table
//! flattened into one row of cells and on a last pipe escaped after an escaped backslash; the
//! name of an absolute directory of the binaries in the home directory; a `surf-agent` call
//! whose child keeps its output open, and one past its bound whose child must end with it; a
//! report left by an earlier run when the lane panics; the model lane's driver on a model that
//! leaves a child, a bound of 0, and words with a backslash, a tab, ESC and a carriage return.
//! No model is called.

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use surf_agent_tasks::agent::{Agent, Step};
use surf_agent_tasks::format::{Task, read_tasks};
use surf_agent_tasks::judge::row_cells;
use surf_agent_tasks::lane::{self, Config};
use surf_agent_tasks::process::{END_BOUND, RuntimeDir, comm, signal_kill};
use surf_agent_tasks::report::{Lane, shown_binaries};
use surf_agent_tasks::{Run, Verdict, judge, tasks_file};

use surf_answer::{Answer, Form, Head, SessionState, render, text_page};
use surf_test_process::ended_within;

use crate::model_driver::{ONE_TASK, assert_one_task};

fn tmp() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
}

fn task(name: &str) -> Task {
    read_tasks(&fs::read_to_string(tasks_file()).unwrap())
        .unwrap()
        .into_iter()
        .find(|task| task.name == name)
        .unwrap()
}

fn one_answer(lines: &[&str]) -> Run {
    let mut answer = Answer::new(Head {
        session: "4b0f6d2e-8c1a-4e5b-9f3d-2a7c6e1b0d94".to_string(),
        state: SessionState::Open,
    });
    answer.text = Some(text_page(
        Form::Markdown,
        lines.iter().map(|line| line.to_string()),
        1,
    ));
    Run {
        calls: 3,
        connections: 1,
        failure: None,
        answers: vec![render(&answer).unwrap()],
        opened: None,
    }
}

#[test]
fn a_table_flattened_into_one_row_of_cells_fails() {
    // Every row of prices.html in one table row: no row of the page is Pears with its price.
    let rows = [
        "# Fruit prices",
        "",
        "| Fruit | Price | Apples | 1.80 | Pears | 2.40 | Plums | 3.10 | Cherries | 6.50 | Grapes | 2.90 | Lemons | 0.60 |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ];
    assert_ne!(judge(&task("table"), &one_answer(&rows)), Verdict::Pass);
}

#[test]
fn a_last_pipe_escaped_after_an_escaped_backslash_ends_no_row() {
    // `\\` is one backslash, then `\|` an escaped pipe: the line does not end with a pipe.
    assert_eq!(row_cells(r"| a | b \|"), None);
    assert_eq!(row_cells(r"| a | b \\\|"), None);
}

#[test]
fn a_directory_in_the_home_directory_is_named_without_the_user() {
    let home = PathBuf::from(std::env::var_os("HOME").expect("HOME is set"));
    let user = home
        .file_name()
        .expect("HOME has a last component")
        .to_string_lossy()
        .into_owned();
    let unpacked = home.join("surf-0.5.0-x86_64-unknown-linux-musl");
    let shown = shown_binaries(&unpacked);
    assert!(
        !shown.split('/').any(|component| component == user),
        "the first line would name the home directory's user: binaries {shown}"
    );
}

/// A directory with a `surf-agent` that is the shell script `body`.
fn stand_in_bin(name: &str, body: &str) -> PathBuf {
    let dir = tmp().join(format!("adversary2-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("surf-agent"), format!("#!/bin/sh\n{body}")).unwrap();
    fs::set_permissions(dir.join("surf-agent"), fs::Permissions::from_mode(0o755)).unwrap();
    dir
}

#[test]
fn a_call_whose_child_keeps_its_output_open_ends_at_its_bound() {
    // surf-agent exits at once; a child it started holds its standard output for 5 s.
    let bin = stand_in_bin("open-output", "/bin/sleep 5 &\nexit 0\n");
    let dir = RuntimeDir::new(&tmp());
    let agent = Agent::new(bin.clone(), dir.runtime()).with_call_bound(Duration::from_secs(1));
    let started = Instant::now();
    let run = agent.run(
        &[Step::Open],
        "http://127.0.0.1:9/article/article.html",
        &mut || {},
    );
    let took = started.elapsed();
    drop(dir);
    let _ = fs::remove_dir_all(&bin);
    assert!(
        took < Duration::from_secs(3),
        "one call bounded by 1 s took {took:?}: {:?}",
        run.failure
    );
}

#[test]
fn a_call_past_its_bound_ends_with_its_child() {
    // surf-agent starts a child in its own process group and waits for it.
    let bin = stand_in_bin("forking", "/bin/sleep 30 &\nwait\n");
    let dir = RuntimeDir::new(&tmp());
    let agent = Agent::new(bin.clone(), dir.runtime()).with_call_bound(Duration::from_secs(1));
    let run = agent.run(
        &[Step::Open],
        "http://127.0.0.1:9/article/article.html",
        &mut || {},
    );
    // The child got SIGKILL with its group and ends after it.
    let left = dir.left_within(&BTreeSet::new(), END_BOUND);
    drop(dir);
    let _ = fs::remove_dir_all(&bin);
    assert_eq!(
        run.failure.map(|failure| failure.reason()),
        Some("no answer within 1 s on open".to_string())
    );
    assert!(left.is_empty(), "processes of the call left: {left:?}");
}

#[test]
fn a_lane_that_panics_leaves_no_report_of_an_earlier_run() {
    // No surf-agent in the directory: the first call panics.
    let bin = tmp().join(format!("adversary2-empty-{}", std::process::id()));
    let _ = fs::remove_dir_all(&bin);
    fs::create_dir_all(&bin).unwrap();
    let out = tmp().join(format!("adversary2-panic-{}.txt", std::process::id()));
    fs::write(&out, "agent-tasks: a file of an earlier run\n").unwrap();
    let config = Config {
        bin: bin.clone(),
        fallback: tmp(),
        call_bound: Duration::from_secs(1),
        lane: Lane {
            name: "gate".to_string(),
            binaries: bin.clone(),
        },
        out: out.clone(),
        https: None,
    };
    let panicked = catch_unwind(AssertUnwindSafe(|| lane::run(&config))).is_err();
    let left = out.exists();
    let _ = fs::remove_file(&out);
    let _ = fs::remove_dir_all(&bin);
    assert!(panicked, "the lane ran without surf-agent");
    assert!(!left, "the report of an earlier run is still there");
}

/// The driver, `ADVERSARY_MODEL_SCRIPT` or `taskfiles/agent-tasks-model.sh`.
fn driver() -> PathBuf {
    std::env::var_os("ADVERSARY_MODEL_SCRIPT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../taskfiles/agent-tasks-model.sh"
            ))
        })
}

/// Run the driver with the model command `model` and the extra variables `env`: its output and
/// the out file, when one is written.
fn drive(model: &Path, name: &str, env: &[(&str, &str)]) -> (Output, Option<String>) {
    drive_without(model, name, env, &[])
}

/// [`drive`] with the variables `removed` taken out of the driver's environment after `env`.
fn drive_without(
    model: &Path,
    name: &str,
    env: &[(&str, &str)],
    removed: &[&str],
) -> (Output, Option<String>) {
    let bin = std::env::var_os("SURF_BIN_DIR")
        .map(PathBuf::from)
        .expect("SURF_BIN_DIR names the built programs, agent-tasks-serve among them");
    let out = tmp().join(format!(
        "adversary2-model-{name}-{}.txt",
        std::process::id()
    ));
    fs::write(&out, "agent-tasks-model: a file of an earlier run\n").unwrap();
    let mut command = Command::new("sh");
    command
        .arg(driver())
        .arg(&bin)
        .arg(&out)
        .env("AGENT_TASKS_SERVE", bin.join("agent-tasks-serve"))
        .env("AGENT_TASKS_MODEL_COMMAND", model)
        .env("AGENT_TASKS_MODEL", "stub")
        .env_remove("AGENT_TASKS_MODEL_TIMEOUT")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::null());
    for (key, value) in env {
        command.env(key, value);
    }
    for key in removed {
        command.env_remove(key);
    }
    let output = command.output().expect("sh runs");
    let written = fs::read_to_string(&out).ok();
    let _ = fs::remove_file(&out);
    (output, written)
}

/// A stand-in model: `body` after the `#!/bin/sh` line.
fn model(name: &str, body: &str) -> PathBuf {
    let path = tmp().join(format!("adversary2-{name}-{}.sh", std::process::id()));
    fs::write(&path, format!("#!/bin/sh\n{body}")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// The processes named in `pids` that run a `sleep` and have not ended within `bound`; each is
/// sent SIGKILL. Decided by a wait, `ended_within`, never by one read: a process sent SIGKILL
/// keeps its entry and its name under /proc until its parent has collected it.
fn sleeping(pids: &Path, bound: Duration) -> Vec<u32> {
    let listed = fs::read_to_string(pids).unwrap_or_default();
    let _ = fs::remove_file(pids);
    let left: Vec<u32> = listed
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .filter(|&pid| comm(pid).as_deref() == Some("sleep"))
        .filter(|&pid| ended_within(pid, bound).is_err())
        .collect();
    for &pid in &left {
        signal_kill(pid);
    }
    left
}

/// A child the test started: dropping it sends it SIGKILL and collects it, so a test that
/// panics leaves no process behind.
struct Spawned(Child);

impl Drop for Spawned {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// `/bin/sleep 30`, none of its standard streams the test's.
fn spawn_sleep() -> Spawned {
    let child = Command::new("/bin/sleep")
        .arg("30")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("/bin/sleep starts");
    Spawned(child)
}

#[test]
fn a_killed_process_nobody_has_collected_is_not_left() {
    let mut child = spawn_sleep();
    let pid = child.0.id();
    let pids = tmp().join(format!(
        "adversary2-a_killed_process_nobody_has_collected_is_not_left-{}",
        std::process::id()
    ));
    fs::write(&pids, format!("{pid}\n")).unwrap();
    signal_kill(pid);
    // The test is its parent and has not collected it: it ends as a zombie.
    ended_within(pid, END_BOUND).expect("the child ends after SIGKILL");
    assert_eq!(
        comm(pid).as_deref(),
        Some("sleep"),
        "an ended child keeps its name until its parent collects it"
    );
    assert_eq!(
        sleeping(&pids, END_BOUND),
        Vec::<u32>::new(),
        "sleeping took a killed process nobody has collected for one left"
    );
    let _ = child.0.wait();
}

#[test]
fn a_process_that_still_runs_is_left_and_ended() {
    let mut child = spawn_sleep();
    let pid = child.0.id();
    let pids = tmp().join(format!(
        "adversary2-a_process_that_still_runs_is_left_and_ended-{}",
        std::process::id()
    ));
    fs::write(&pids, format!("{pid}\n")).unwrap();
    assert_eq!(
        sleeping(&pids, Duration::ZERO),
        vec![pid],
        "sleeping did not take a process that runs for one left"
    );
    let ended = ended_within(pid, END_BOUND);
    assert!(
        ended.is_ok(),
        "sleeping did not end what it found: {ended:?}"
    );
    let _ = child.0.wait();
}

#[test]
fn a_model_that_leaves_a_child_leaves_no_process() {
    let pids = tmp().join(format!("adversary2-child-pids-{}", std::process::id()));
    let _ = fs::remove_file(&pids);
    let stand_in = model(
        "child-model",
        &format!(
            "/bin/sleep 20 < /dev/null > /dev/null 2>&1 &\necho $! >> '{}'\necho 'ANSWER: unknown'\n",
            pids.display()
        ),
    );
    let (output, written) = drive(&stand_in, "child", &[("AGENT_TASKS_FILE", ONE_TASK)]);
    let left = sleeping(&pids, END_BOUND);
    let _ = fs::remove_file(&stand_in);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_one_task(&written.expect("the out file is written"));
    assert!(left.is_empty(), "children of the model left: {left:?}");
}

#[test]
fn a_model_over_its_bound_leaves_no_process() {
    let pids = tmp().join(format!("adversary2-session-pids-{}", std::process::id()));
    let _ = fs::remove_file(&pids);
    let stand_in = model(
        "session-model",
        &format!(
            "/bin/sleep 20 < /dev/null > /dev/null 2>&1 &\necho $! >> '{}'\nexec /bin/sleep 120\n",
            pids.display()
        ),
    );
    let (output, written) = drive(&stand_in, "session", &[("AGENT_TASKS_MODEL_TIMEOUT", "1")]);
    let left = sleeping(&pids, END_BOUND);
    let _ = fs::remove_file(&stand_in);
    assert_ne!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(written, None);
    assert!(left.is_empty(), "children of the model left: {left:?}");
}

#[test]
fn a_bound_of_zero_or_no_number_ends_the_lane_before_any_model_runs() {
    let ran = tmp().join(format!("adversary2-ran-{}", std::process::id()));
    let _ = fs::remove_file(&ran);
    let stand_in = model(
        "zero-model",
        &format!("touch '{}'\necho 'ANSWER: 1932'\n", ran.display()),
    );
    for bound in ["0", "00", "-1", "1s", "abc"] {
        let (output, written) = drive(&stand_in, "zero", &[("AGENT_TASKS_MODEL_TIMEOUT", bound)]);
        assert_ne!(output.status.code(), Some(0), "{bound}: {output:?}");
        assert_eq!(written, None, "{bound}");
        assert!(!ran.exists(), "{bound}: the model ran");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("AGENT_TASKS_MODEL_TIMEOUT"),
            "{bound}: {stderr}"
        );
    }
    // Another task list without a model command of the caller: refused before anything runs or
    // the out file of the earlier run is removed.
    let (output, written) = drive_without(
        &stand_in,
        "zero",
        &[("AGENT_TASKS_FILE", ONE_TASK)],
        &["AGENT_TASKS_MODEL_COMMAND", "AGENT_TASKS_MODEL"],
    );
    assert_ne!(output.status.code(), Some(0), "{output:?}");
    assert!(!ran.exists(), "the model ran");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("AGENT_TASKS_FILE"), "{stderr}");
    assert_eq!(
        written.as_deref(),
        Some("agent-tasks-model: a file of an earlier run\n"),
        "the out file of the earlier run is not left as it was"
    );
    let _ = fs::remove_file(&stand_in);
}

#[test]
fn every_call_is_one_line_that_reads_back() {
    // A literal backslash-n, a line feed, a tab with ESC and a final backslash, a carriage return.
    let stand_in = model(
        "words-model",
        "surf-agent 'x\\ny' > /dev/null 2>&1\n\
         surf-agent 'x\ny' > /dev/null 2>&1\n\
         surf-agent \"$(printf 'a\\tb\\033c\\\\')\" > /dev/null 2>&1\n\
         surf-agent \"$(printf 'cr\\rz')\" > /dev/null 2>&1\n\
         echo 'ANSWER: unknown'\n",
    );
    let (output, written) = drive(&stand_in, "words", &[("AGENT_TASKS_FILE", ONE_TASK)]);
    let _ = fs::remove_file(&stand_in);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let written = written.expect("the out file is written");
    assert_one_task(&written);
    let expected = "\ntask article fail 4 calls\n  surf-agent x\\\\ny\n  surf-agent x\\ny\n  \
                    surf-agent a\\x09b\\x1bc\\\\\n  surf-agent cr\\rz\n  answered \"unknown\"\n";
    assert!(written.contains(expected), "{written:?}");
}

mod wait;
