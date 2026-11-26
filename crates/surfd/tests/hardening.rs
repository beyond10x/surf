//! The cases of `story:session-process-hardening`: `surfd` starts a session with an empty
//! environment but `Config::session_env`, through `setpriv --no-new-privs` and `prlimit` with
//! the bounds of its form, refuses an `open` with scripts when either program is missing, and
//! names the signal a session ended by. A case starts at most one `surfd` and opens its
//! sessions through it; one starts the `surfd` binary, the effect case starts none, and every
//! other case runs `surfd` in the test process (`common::start`) with the stand-in session.

use crate::common;
use crate::view::{json, listed, sessions, view_url};

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use common::*;
use surf_agent::wire::{Answer, Arg, Request};
use surfd::{Config, Daemon, State};

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
    assert_eq!(limit(pid, "Max core file size"), expected_core().0);
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
    assert_eq!(limit(pid, "Max core file size"), expected_core().0);
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
    config.script_session_bounds = vec!["--cpu=1:2".to_string(), config.session_bounds[0].clone()];
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

/// Statement 4: a session `surfd` kills itself with `SIGKILL` after the late read of a
/// timed-out answer keeps today's answers: `session-timeout` for the command that waited, then
/// `died` and `session-died` with no `note ended` line; the view lists no signal.
#[test]
fn a_session_surfd_killed_after_its_late_answer_names_no_signal() {
    let dir = TestDir::new();
    let mut config = config(&dir);
    config.session_answer_timeout = Duration::from_secs(1);
    config.late_answer_timeout = Duration::from_secs(1);
    let daemon = start(config);
    let session = open(&dir);
    let hold = dir.hold();
    dir.set("behaviour", "stall");
    let first = look(&dir, &session);
    assert_eq!(first.status, 3);
    assert_eq!(first.text, refusal(&session.id, "open", "session-timeout"));
    let answer = died_and_look(&daemon, &dir, &session, ALLOWED);
    drop(hold);
    assert_eq!(answer.status, 3);
    assert_eq!(answer.text, refusal(&session.id, "died", "session-died"));
    assert_eq!(listed_signal(&dir, &session), json::Value::Null);
}

/// Send `signal` (`XCPU`, `KILL`) to process `pid` with kill(1).
fn send(pid: u32, signal: &str) {
    let status = Command::new("kill")
        .arg(format!("-{signal}"))
        .arg(pid.to_string())
        .status()
        .expect("kill runs");
    assert!(status.success(), "kill -{signal} {pid}: {status}");
}

/// Statement 4 under the rule of the bound's word: a session without scripts has no CPU bound,
/// so `SIGXCPU`, sent here by the test, is named without `bound cpu`.
#[test]
fn a_session_without_a_cpu_bound_ended_by_sigxcpu_names_no_bound() {
    let dir = TestDir::new();
    let daemon = start(config(&dir));
    let session = open(&dir);
    send(session_pid(&dir, &daemon, &session), "XCPU");
    let answer = died_and_look(&daemon, &dir, &session, ALLOWED);
    assert_eq!(answer.status, 3);
    assert_eq!(
        answer.text,
        format!(
            "{}note ended: signal SIGXCPU\n",
            refusal(&session.id, "died", "session-died")
        )
    );
    assert_eq!(
        listed_signal(&dir, &session),
        json::Value::Str("SIGXCPU".to_string())
    );
}

/// A start chain that ends by an exit before the session wrote a byte: `prlimit` is a sh script
/// that exits with status 1. An `open` without scripts is answered as a missing program is,
/// status 3 and the reason `cannot start <program>: ...` with no answer text, and nothing is
/// listed or left running.
#[test]
fn a_start_chain_that_exits_first_answers_an_open_without_scripts_as_a_missing_program() {
    let dir = TestDir::new();
    let prlimit = dir.path.join("prlimit");
    fs::write(&prlimit, "#!/bin/sh\nexit 1\n").unwrap();
    fs::set_permissions(&prlimit, fs::Permissions::from_mode(0o755)).unwrap();
    let mut config = config(&dir);
    config.prlimit = prlimit;
    let program = config.session_program.clone();
    let daemon = start(config);
    let answer = call(&dir, &open_request(false));
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(answer.text, "", "{answer:?}");
    let reason = answer.reason.clone().unwrap_or_default();
    assert!(
        reason.starts_with(&format!("cannot start {}: ", program.display())),
        "{answer:?}"
    );
    assert!(daemon.sessions().is_empty());
    assert_eq!(children_of(std::process::id()), [] as [u32; 0]);
}

/// The id of the head line `surf 1 session <id> died` of `answer`.
fn died_id(answer: &Answer) -> String {
    let head = answer.text.lines().next().unwrap_or_default();
    head.strip_prefix("surf 1 session ")
        .and_then(|rest| rest.strip_suffix(" died"))
        .unwrap_or_else(|| panic!("no head line of a died session: {answer:?}"))
        .to_string()
}

/// A session that dies by a signal before it wrote its first byte gets the answer it gets
/// without the signal, status 3 and its reason, with the lines of a session that died that way,
/// as one that dies while it answers `open`: the stand-in aborts before its `ready` line.
fn a_session_that_aborts_before_its_first_byte(scripts: bool) {
    let dir = TestDir::new();
    dir.set("start-behaviour", "abort");
    let daemon = start(config(&dir));
    let answer = call(&dir, &open_request(scripts));
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(
        answer.reason.as_deref(),
        Some("the session ended or wrote another line before it reported ready")
    );
    let id = died_id(&answer);
    assert_eq!(
        answer.text,
        format!(
            "{}note ended: signal SIGABRT\n",
            refusal(&id, "died", "session-died")
        )
    );
    assert!(daemon.sessions().is_empty());
    assert_eq!(children_of(std::process::id()), [] as [u32; 0]);
}

#[test]
fn a_session_with_scripts_that_aborts_before_its_first_byte_names_its_signal() {
    a_session_that_aborts_before_its_first_byte(true);
}

#[test]
fn a_session_without_scripts_that_aborts_before_its_first_byte_names_its_signal() {
    a_session_that_aborts_before_its_first_byte(false);
}

/// A session that dies by a signal while it answers `open` gets the answer it gets without the
/// signal, status 3 and its reason, with the lines of a session that died that way: here a
/// session with scripts that aborts before it writes its `open` answer.
#[test]
fn a_session_that_aborts_while_it_answers_open_names_its_signal() {
    let dir = TestDir::new();
    dir.set("open-behaviour", "abort");
    let daemon = start(config(&dir));
    let answer = call(&dir, &open_request(true));
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(
        answer.reason.as_deref(),
        Some("the session ended or broke the link while it answered open")
    );
    let id = died_id(&answer);
    assert_eq!(
        answer.text,
        format!(
            "{}note ended: signal SIGABRT\n",
            refusal(&id, "died", "session-died")
        )
    );
    assert!(daemon.sessions().is_empty());
}

/// As above, a session without scripts that the test kills with `SIGKILL` while it holds its
/// `open` answer back.
#[test]
fn a_session_killed_by_another_process_while_it_answers_open_names_sigkill() {
    let dir = TestDir::new();
    dir.set("open-behaviour", "stall");
    let hold = dir.hold();
    let daemon = start(config(&dir));
    let opening = {
        let socket = dir.socket();
        thread::spawn(move || {
            let mut stream = std::os::unix::net::UnixStream::connect(socket).unwrap();
            std::io::Write::write_all(&mut stream, &open_request(false).encode().unwrap()).unwrap();
            surf_agent::wire::read_answer(&mut stream, surf_agent::wire::ANSWER_TIMEOUT)
                .expect("an answer")
        })
    };
    let began = Instant::now();
    let pid = loop {
        let started = dir.starts();
        if let Some((pid, id)) = started.first()
            && dir.requests(id).contains("command open")
        {
            break *pid;
        }
        assert!(began.elapsed() < ALLOWED, "the stand-in did not read open");
        thread::sleep(Duration::from_millis(10));
    };
    send(pid, "KILL");
    let answer = opening.join().unwrap();
    drop(hold);
    assert_eq!(answer.status, 3, "{answer:?}");
    let id = died_id(&answer);
    assert_eq!(
        answer.text,
        format!(
            "{}note ended: signal SIGKILL\n",
            refusal(&id, "died", "session-died")
        )
    );
    assert!(daemon.sessions().is_empty());
}

/// The test process lowers its own core limit to 0 with `prlimit --pid` before it builds its
/// `Config`, as a `surfd` under a hard core limit of 0 runs (a service's `LimitCORE=0`): a
/// session without scripts and one with scripts each open, and each keeps a core limit of 0.
/// `prlimit` cannot raise a hard limit of 0, so a choice of a limit of 1 there fails the start.
/// It needs one process per test, which cargo-nextest gives.
#[test]
fn a_session_keeps_a_core_limit_of_0_under_a_hard_limit_of_0() {
    let own = std::process::id();
    let lowered = Command::new("/usr/bin/prlimit")
        .args(["--pid", &own.to_string(), CORE_ARGUMENT_ZERO])
        .status()
        .expect("prlimit runs");
    assert!(lowered.success(), "{lowered}");
    assert_eq!(limit(own, "Max core file size"), pair("0", "0"));
    let dir = TestDir::new();
    let daemon = start(config(&dir));
    let session = open(&dir);
    let pid = session_pid(&dir, &daemon, &session);
    assert_eq!(limit(pid, "Max core file size"), pair("0", "0"));
    let scripted = open_with_scripts(&dir);
    let pid = session_pid(&dir, &daemon, &scripted);
    assert_eq!(limit(pid, "Max core file size"), pair("0", "0"));
}

/// The effect on the host the tests run on: a shell started under the core argument
/// `Config::new` chose for a session ends itself by `SIGABRT`, in an empty directory; its status
/// is the signal 6 with no dump, and the directory stays empty. It prints the argument, the
/// first line of `/proc/sys/kernel/core_pattern` and the hard core limit it read.
///
/// Where it holds something: a host whose core pattern is a pipe and whose hard core limit is at
/// least 1, where a shell under a core limit of 0 shows a dump in its status. Where it holds
/// nothing, it says so and ends: a hard core limit of 0 (it prints `not shown: hard core limit
/// 0`), and a core pattern whose first byte is not `|` (it prints `not shown: no pipe pattern`).
/// That under a limit of 1 the kernel does not start a pipe handler was shown on one host with a
/// pipe pattern, by the host's crash list and the kernel's log line `RLIMIT_CORE is set to 1,
/// aborting core`; what the kernel does under a file pattern or a socket pattern is not shown.
#[test]
fn a_process_under_the_core_bound_surfd_chose_ends_by_abort_without_a_dump() {
    use std::os::unix::process::ExitStatusExt;

    let argument = Config::new(None)
        .session_bounds
        .first()
        .expect("the core argument")
        .clone();
    let pattern = fs::read_to_string("/proc/sys/kernel/core_pattern")
        .unwrap_or_default()
        .lines()
        .next()
        .unwrap_or_default()
        .to_string();
    let (_, hard) = limit(std::process::id(), "Max core file size");
    eprintln!("argument {argument} core_pattern {pattern} hard core limit {hard}");
    if hard == "0" {
        eprintln!("not shown: hard core limit 0");
        return;
    }
    if !pattern.starts_with('|') {
        eprintln!("not shown: no pipe pattern");
        return;
    }
    let dir = TestDir::new();
    let empty = dir.path.join("empty");
    fs::create_dir(&empty).expect("an empty directory");
    let status = Command::new("/usr/bin/prlimit")
        .arg(&argument)
        .args(["--", "/bin/sh", "-c", "kill -s ABRT $$"])
        .current_dir(&empty)
        .status()
        .expect("prlimit runs");
    assert_eq!(status.signal(), Some(6), "{status}");
    assert!(!status.core_dumped(), "{status}");
    let entries: Vec<_> = fs::read_dir(&empty)
        .expect("the directory")
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect();
    assert!(entries.is_empty(), "{entries:?}");
}
