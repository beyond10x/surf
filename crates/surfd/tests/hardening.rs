//! The cases of `story:session-process-hardening`: `surfd` starts a session with an empty
//! environment but `Config::session_env`, through `setpriv --no-new-privs` and `prlimit` with
//! the bounds of its form, refuses an `open` with scripts when either program is missing, and
//! names the signal a session ended by. Each case starts one `surfd` and one session and reads
//! the session process from outside, in `/proc/<pid>/`; every case but one runs `surfd` in the
//! test process (`common::start`) with the stand-in session.

use crate::common;
use crate::view::{json, listed, sessions, view_url};

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use common::*;
use surf_agent::wire::{Answer, Arg, Request};
use surfd::{Daemon, State};

/// How long a case waits for a session to end at its CPU bound of 1 s.
const BOUND_ENDS_WITHIN: Duration = Duration::from_secs(10);

/// The variables of process `pid`, each `<name>=<value>`, in its order.
fn environ(pid: u32) -> Vec<String> {
    fs::read(format!("/proc/{pid}/environ"))
        .expect("the environment of the process")
        .split(|&byte| byte == 0)
        .filter(|pair| !pair.is_empty())
        .map(|pair| String::from_utf8_lossy(pair).into_owned())
        .collect()
}

/// The soft and the hard value of the limit `name` (`Max cpu time`) of process `pid`, as
/// `/proc/<pid>/limits` writes them.
fn limit(pid: u32, name: &str) -> (String, String) {
    let limits = fs::read_to_string(format!("/proc/{pid}/limits")).expect("the limits");
    let line = limits
        .lines()
        .find(|line| line.starts_with(name) && line[name.len()..].starts_with("  "))
        .unwrap_or_else(|| panic!("no {name} in {limits}"));
    let mut values = line[name.len()..].split_whitespace();
    (
        values.next().expect("a soft value").to_string(),
        values.next().expect("a hard value").to_string(),
    )
}

/// The value of `NoNewPrivs:` in `/proc/<pid>/status`.
fn no_new_privs(pid: u32) -> String {
    let status = fs::read_to_string(format!("/proc/{pid}/status")).expect("the status");
    status
        .lines()
        .find_map(|line| line.strip_prefix("NoNewPrivs:"))
        .expect("a NoNewPrivs line")
        .trim()
        .to_string()
}

/// The program process `pid` runs.
fn exe(pid: u32) -> PathBuf {
    fs::read_link(format!("/proc/{pid}/exe")).expect("the program of the process")
}

/// The four limits of Q11: CPU time, address space, open files and core file size.
const BOUNDED: [&str; 4] = [
    "Max cpu time",
    "Max address space",
    "Max open files",
    "Max core file size",
];

fn pair(soft: &str, hard: &str) -> (String, String) {
    (soft.to_string(), hard.to_string())
}

/// The process ids whose parent is process `parent`: the fourth field of `/proc/*/stat`.
fn children_of(parent: u32) -> Vec<u32> {
    let mut children = Vec::new();
    for entry in fs::read_dir("/proc").unwrap().flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(stat) = fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        // `<pid> (<comm>) <state> <ppid> …`; the name may hold spaces and parentheses.
        let after = &stat[stat.rfind(')').expect("the end of the name") + 1..];
        let ppid: u32 = after
            .split_whitespace()
            .nth(1)
            .expect("the parent")
            .parse()
            .expect("a number");
        if ppid == parent {
            children.push(pid);
        }
    }
    children.sort_unstable();
    children
}

fn open_request(scripts: bool) -> Request {
    let mut open = request("open", None, None);
    if scripts {
        open.args = vec![Arg {
            name: "scripts".to_string(),
            value: "true".to_string(),
        }];
    }
    open
}

/// An `open` with `arg scripts "true"`, which must succeed.
fn open_with_scripts(dir: &TestDir) -> Opened {
    let answer = call(dir, &open_request(true));
    assert_eq!(answer.status, 0, "open with scripts: {answer:?}");
    opened(answer)
}

/// The process id `surfd` lists for the session, which is the stand-in's own: `setpriv` and
/// `prlimit` replaced themselves by it.
fn session_pid(dir: &TestDir, daemon: &Daemon, session: &Opened) -> u32 {
    let listed = daemon.sessions().get(&session.id).expect("listed").pid;
    assert_eq!(listed, dir.pid_of(&session.id), "the process surfd holds");
    listed
}

/// Statement 1, in process: the session's environment is exactly `Config::session_env`, while
/// the process `surfd` runs in has variables of its own.
#[test]
fn a_session_has_the_variables_of_session_env_only() {
    let dir = TestDir::new();
    let own = environ(std::process::id());
    assert!(
        own.iter().any(|pair| !pair.starts_with("STAND_IN_DIR=")),
        "the test process has no variable to leave out: {own:?}"
    );
    let daemon = start(config(&dir));
    let session = open(&dir);
    let pid = session_pid(&dir, &daemon, &session);
    assert_eq!(
        environ(pid),
        [format!("STAND_IN_DIR={}", dir.control().display())]
    );
}

/// Statement 1, with the built `surfd` and `SURF_PROBE_SECRET=1` in its environment: the session
/// gets none of it. The binary's `Config::session_env` is empty; the one variable the session
/// holds is the `STAND_IN_DIR` the stand-in sets itself when it is started without one.
#[test]
fn a_variable_of_surfds_environment_does_not_reach_the_session() {
    let dir = TestDir::new();
    let surfd = Surfd::start(&dir, &[], |command| {
        command.env("SURF_PROBE_SECRET", "1");
    });
    assert_eq!(surfd.line, "ready\n");
    assert!(environ(surfd.pid()).contains(&"SURF_PROBE_SECRET=1".to_string()));
    let session = agent_open(&dir);
    let pid = dir.pid_of(&session.id);
    assert_eq!(
        environ(pid),
        [format!("STAND_IN_DIR={}", dir.control().display())]
    );
}

/// Statement 2, a session without scripts: no new privileges and no core file; its CPU time,
/// address space and open files are those of the process `surfd` runs in.
#[test]
fn a_session_without_scripts_has_no_core_file_and_no_new_privileges() {
    let dir = TestDir::new();
    let daemon = start(config(&dir));
    let session = open(&dir);
    let pid = session_pid(&dir, &daemon, &session);
    let own = std::process::id();
    assert_eq!(limit(pid, "Max core file size"), pair("0", "0"));
    for name in &BOUNDED[..3] {
        assert_eq!(limit(pid, name), limit(own, name), "{name}");
    }
    assert_eq!(
        no_new_privs(own),
        "0",
        "the test process has no new privileges"
    );
    assert_eq!(no_new_privs(pid), "1");
    assert_eq!(exe(pid), stand_in_program().canonicalize().unwrap());
}

/// Statement 2, a session with scripts: the four bounds of Q11.
#[test]
fn a_session_with_scripts_has_the_bounds_of_q11() {
    let dir = TestDir::new();
    let daemon = start(config(&dir));
    let session = open_with_scripts(&dir);
    let pid = session_pid(&dir, &daemon, &session);
    assert_eq!(
        limit(pid, "Max address space"),
        pair("4294967296", "4294967296")
    );
    assert_eq!(limit(pid, "Max cpu time"), pair("3600", "3601"));
    assert_eq!(limit(pid, "Max open files"), pair("64", "64"));
    assert_eq!(limit(pid, "Max core file size"), pair("0", "0"));
    assert_eq!(no_new_privs(std::process::id()), "0");
    assert_eq!(no_new_privs(pid), "1");
    assert_eq!(exe(pid), stand_in_program().canonicalize().unwrap());
}

/// Statement 3 for one program: with its path naming no file, a session without scripts starts
/// as before, without either program, and an `open` with scripts is refused and starts nothing.
fn without(program: &str) {
    let dir = TestDir::new();
    let mut config = config(&dir);
    let missing = dir.path.join(format!("no-{program}"));
    match program {
        "setpriv" => config.setpriv = missing,
        _ => config.prlimit = missing,
    }
    let daemon = start(config);
    let session = open(&dir);
    let pid = session_pid(&dir, &daemon, &session);
    let own = std::process::id();
    assert_eq!(no_new_privs(pid), no_new_privs(own));
    assert_eq!(no_new_privs(pid), "0");
    for name in BOUNDED {
        assert_eq!(limit(pid, name), limit(own, name), "{name}");
    }
    let answer = call(&dir, &open_request(true));
    assert_eq!(answer.status, 3, "{answer:?}");
    assert!(
        answer
            .text
            .lines()
            .any(|line| line == "did refused scripts-unavailable"),
        "{answer:?}"
    );
    assert_eq!(children_of(own), [pid], "the processes surfd started");
    assert_eq!(daemon.sessions().len(), 1);
}

#[test]
fn without_setpriv_a_session_with_scripts_is_refused() {
    without("setpriv");
}

#[test]
fn without_prlimit_a_session_with_scripts_is_refused() {
    without("prlimit");
}

/// Wait until `surfd` lists the session `died`, then answer `look` for it.
fn died_and_look(daemon: &Daemon, dir: &TestDir, session: &Opened, within: Duration) -> Answer {
    assert!(
        daemon.wait_sessions(within, |list| list
            .get(&session.id)
            .is_some_and(|listed| listed.state == State::Died)),
        "the session is not listed died: {:?}",
        daemon.sessions()
    );
    look(dir, session)
}

/// The `signal` member of the session's summary in `/api/sessions`, after its `state` is
/// `died`.
fn listed_signal(dir: &TestDir, session: &Opened) -> json::Value {
    let summary = listed(&sessions(&view_url(dir)), &session.id);
    assert_eq!(summary.get("state").str(), "died");
    summary.get("signal").clone()
}

/// Statements 4 and 5: a stand-in that spins under the CPU bound `--cpu=1:2` ends at the soft
/// one by `SIGXCPU`, and the next answer and the view name the signal and the bound.
#[test]
fn bound_ends_session() {
    let dir = TestDir::new();
    dir.set("behaviour", "spin");
    let mut config = config(&dir);
    config.script_session_bounds = vec!["--cpu=1:2".to_string(), "--core=0:0".to_string()];
    let daemon = start(config);
    let began = Instant::now();
    let session = open_with_scripts(&dir);
    let answer = died_and_look(&daemon, &dir, &session, BOUND_ENDS_WITHIN);
    assert!(began.elapsed() < BOUND_ENDS_WITHIN);
    assert_eq!(answer.status, 3);
    assert_eq!(
        answer.text,
        format!(
            "{}note ended: signal SIGXCPU bound cpu\n",
            refusal(&session.id, "died", "session-died")
        )
    );
    assert_eq!(
        listed_signal(&dir, &session),
        json::Value::Str("SIGXCPU".to_string())
    );
}

/// Statements 4 and 5: a stand-in that aborts ends by `SIGABRT`, which names no bound.
#[test]
fn an_aborted_session_names_its_signal() {
    let dir = TestDir::new();
    dir.set("behaviour", "abort");
    let daemon = start(config(&dir));
    let session = open(&dir);
    let answer = died_and_look(&daemon, &dir, &session, ALLOWED);
    assert_eq!(answer.status, 3);
    assert_eq!(
        answer.text,
        format!(
            "{}note ended: signal SIGABRT\n",
            refusal(&session.id, "died", "session-died")
        )
    );
    assert_eq!(
        listed_signal(&dir, &session),
        json::Value::Str("SIGABRT".to_string())
    );
}

/// Statements 4 and 5: a stand-in that exits with status 1 ended by no signal.
#[test]
fn a_session_that_exits_names_no_signal() {
    let dir = TestDir::new();
    dir.set("behaviour", "exit");
    let daemon = start(config(&dir));
    let session = open(&dir);
    let first = look(&dir, &session);
    assert_eq!(first.text, refusal(&session.id, "died", "session-died"));
    let answer = died_and_look(&daemon, &dir, &session, ALLOWED);
    assert_eq!(answer.status, 3);
    assert_eq!(answer.text, refusal(&session.id, "died", "session-died"));
    assert_eq!(listed_signal(&dir, &session), json::Value::Null);
}

/// Statements 4 and 5: a stand-in that `surfd` kills itself, after it broke the link, keeps
/// today's answer and names no signal.
#[test]
fn a_session_surfd_killed_names_no_signal() {
    let dir = TestDir::new();
    dir.set("behaviour", "break");
    let daemon = start(config(&dir));
    let session = open(&dir);
    let first = look(&dir, &session);
    assert_eq!(first.status, 3);
    assert_eq!(first.text, refusal(&session.id, "died", "session-died"));
    let answer = died_and_look(&daemon, &dir, &session, ALLOWED);
    assert_eq!(answer.status, 3);
    assert_eq!(answer.text, refusal(&session.id, "died", "session-died"));
    assert_eq!(listed_signal(&dir, &session), json::Value::Null);
}
