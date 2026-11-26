//! Adversary cases against `story:session-process-hardening`: the start of a session through
//! `setpriv` and `prlimit` (the arguments and environment of both, a program that is no
//! executable file or that fails before it runs the session), the bounds as a boundary (a child
//! of the session, a raise of a hard limit), and the `note ended` line (an exit status that
//! reads like a signal, a signal outside the table, a request in flight, two clients at once).
//! Each case starts one `surfd` in the test process and one session; a session program or a
//! wrapper a case writes is a POSIX sh script in the case's directory.

use crate::common;
use crate::view::{json, listed, sessions, view_url};

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use common::*;
use surf_agent::wire::{Answer, Arg, Request};
use surfd::{Daemon, State};

/// `/usr/bin/setpriv` and `/usr/bin/prlimit`, the shipped paths.
const SETPRIV: &str = "/usr/bin/setpriv";
const PRLIMIT: &str = "/usr/bin/prlimit";

/// The arguments of `prlimit` for a session with scripts, as the story ships them, with the core
/// argument of [`expected_core`].
fn script_bounds() -> [String; 4] {
    [
        "--as=4294967296:4294967296".to_string(),
        "--cpu=3600:3601".to_string(),
        "--nofile=64:64".to_string(),
        expected_core().1.to_string(),
    ]
}

/// An executable POSIX sh script `name` in the case's directory, `body` after its first line.
fn script(dir: &TestDir, name: &str, body: &str) -> PathBuf {
    let path = dir.path.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// `path` in single quotes for a sh script; the test directories hold no quote.
fn quoted(path: &Path) -> String {
    let text = path.display().to_string();
    assert!(!text.contains('\''), "{text}");
    format!("'{text}'")
}

fn open_with(args: &[(&str, &str)]) -> Request {
    let mut open = request("open", None, None);
    open.args = args
        .iter()
        .map(|(name, value)| Arg {
            name: (*name).to_string(),
            value: (*value).to_string(),
        })
        .collect();
    open
}

/// The soft and hard value of the limit `name` of process `pid` (`/proc/<pid>/limits`).
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

/// The value of `NoNewPrivs:` in a text of `/proc/<pid>/status`.
fn no_new_privs_in(status: &str) -> String {
    status
        .lines()
        .find_map(|line| line.strip_prefix("NoNewPrivs:"))
        .expect("a NoNewPrivs line")
        .trim()
        .to_string()
}

fn no_new_privs(pid: u32) -> String {
    no_new_privs_in(&fs::read_to_string(format!("/proc/{pid}/status")).expect("the status"))
}

/// Send `signal` (`TERM`, `40`) to process `pid` with kill(1).
fn send(pid: u32, signal: &str) {
    let status = Command::new("kill")
        .arg(format!("-{signal}"))
        .arg(pid.to_string())
        .status()
        .expect("kill runs");
    assert!(status.success(), "kill -{signal} {pid}: {status}");
}

/// Wait until `surfd` lists the session `died`, then answer `look` for it.
fn died_and_look(daemon: &Daemon, dir: &TestDir, session: &Opened) -> Answer {
    assert!(
        daemon.wait_sessions(ALLOWED, |list| list
            .get(&session.id)
            .is_some_and(|listed| listed.state == State::Died)),
        "the session is not listed died: {:?}",
        daemon.sessions()
    );
    look(dir, session)
}

/// The `signal` member of the session's summary in `/api/sessions`.
fn listed_signal(dir: &TestDir, session: &Opened) -> json::Value {
    let summary = listed(&sessions(&view_url(dir)), &session.id);
    assert_eq!(summary.get("state").str(), "died");
    summary.get("signal").clone()
}

// --- The end of a session ----------------------------------------------------------------------

/// The session program is a sh script that runs the stand-in as its child and then exits with
/// `code`; the stand-in aborts once it answered `open`. `surfd` reaps the script's exit status,
/// no signal, and kills nothing.
fn a_session_that_exits_with(code: u8) {
    let dir = TestDir::new();
    dir.set("behaviour", "abort");
    let program = script(
        &dir,
        "session",
        &format!("{} \"$@\"\nexit {code}\n", quoted(&stand_in_program())),
    );
    let mut config = config(&dir);
    config.session_program = program;
    let daemon = start(config);
    let session = open(&dir);
    let answer = died_and_look(&daemon, &dir, &session);
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(answer.text, refusal(&session.id, "died", "session-died"));
    assert_eq!(listed_signal(&dir, &session), json::Value::Null);
}

/// 137 is 128 + 9: an exit status, not `SIGKILL`.
#[test]
fn a_session_that_exits_with_status_137_names_no_signal() {
    a_session_that_exits_with(137);
}

/// 152 is 128 + 24: an exit status, not `SIGXCPU`, so no `bound cpu` either.
#[test]
fn a_session_that_exits_with_status_152_names_no_signal() {
    a_session_that_exits_with(152);
}

/// Signal 40, a real-time signal outside the unit's table of 31 names, reads `SIG40`, which the
/// grammar of `docs/agent-answer.md` takes (`"SIG" 1*(%x41-5A / DIGIT)`), with no bound.
#[test]
fn a_signal_outside_the_table_is_named_by_its_number() {
    let dir = TestDir::new();
    let daemon = start(config(&dir));
    let session = open(&dir);
    send(dir.pid_of(&session.id), "40");
    let answer = died_and_look(&daemon, &dir, &session);
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(
        answer.text,
        format!(
            "{}note ended: signal SIG40\n",
            refusal(&session.id, "died", "session-died")
        )
    );
    assert!(surf_answer::parse(&answer.text).is_ok(), "{answer:?}");
    assert_eq!(
        listed_signal(&dir, &session),
        json::Value::Str("SIG40".to_string())
    );
}

/// A session that dies by a signal while `surfd` waits for its answer: the command that waited
/// is answered from `finish` (`daemon.rs:1192-1203`), which must carry the note as the later
/// answers from the table do.
#[test]
fn a_request_in_flight_when_the_session_dies_by_a_signal_names_it() {
    let dir = TestDir::new();
    let _daemon = start(config(&dir));
    let session = open(&dir);
    let pid = dir.pid_of(&session.id);
    let hold = dir.hold();
    dir.set("behaviour", "stall");
    let answer = thread::scope(|scope| {
        let asking = scope.spawn(|| look(&dir, &session));
        let deadline = Instant::now() + ALLOWED;
        while !dir.requests(&session.id).contains("look") {
            assert!(Instant::now() < deadline, "the stand-in did not read look");
            thread::sleep(Duration::from_millis(5));
        }
        send(pid, "TERM");
        asking.join().expect("the look")
    });
    drop(hold);
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(
        answer.text,
        format!(
            "{}note ended: signal SIGTERM\n",
            refusal(&session.id, "died", "session-died")
        )
    );
}

/// Two clients ask about the same dead session at once, 20 rounds: each answer is the same.
#[test]
fn two_clients_asking_about_one_dead_session_read_the_same_note() {
    let dir = TestDir::new();
    dir.set("behaviour", "abort");
    let daemon = start(config(&dir));
    let session = open(&dir);
    let first = died_and_look(&daemon, &dir, &session);
    let expected = format!(
        "{}note ended: signal SIGABRT\n",
        refusal(&session.id, "died", "session-died")
    );
    assert_eq!(first.text, expected);
    for round in 0..20 {
        let (a, b) = thread::scope(|scope| {
            let a = scope.spawn(|| look(&dir, &session));
            let b = scope.spawn(|| look(&dir, &session));
            (a.join().unwrap(), b.join().unwrap())
        });
        assert_eq!((a.status, b.status), (3, 3), "round {round}");
        assert_eq!(a.text, expected, "round {round}");
        assert_eq!(b.text, expected, "round {round}");
    }
}

// --- The start of a session --------------------------------------------------------------------

/// The arguments of `setpriv` and `prlimit` are the fixed ones and the session's own, whatever
/// the `open` request holds, and the environment `surfd` gives `setpriv` is `Config::session_env`.
/// Both programs are sh scripts that record `"$@"` and their environment at exec, then exec the
/// real program.
#[test]
fn the_wrappers_get_fixed_arguments_and_session_env_only() {
    let dir = TestDir::new();
    let record = |name: &str, real: &str| {
        let control = dir.control();
        script(
            &dir,
            name,
            &format!(
                "/usr/bin/cat /proc/$$/environ > {environ}\n\
                 for argument in \"$@\"; do printf '%s\\n' \"$argument\"; done > {args}\n\
                 exec {real} \"$@\"\n",
                environ = quoted(&control.join(format!("{name}-environ"))),
                args = quoted(&control.join(format!("{name}-args"))),
            ),
        )
    };
    let setpriv = record("setpriv", SETPRIV);
    let prlimit = record("prlimit", PRLIMIT);
    let mut config = config(&dir);
    config.setpriv = setpriv;
    config.prlimit = prlimit.clone();
    let _daemon = start(config);
    let answer = call(
        &dir,
        &open_with(&[
            ("url", "--nofile=1:1 --core=unlimited -- /bin/sh"),
            ("scripts", "true"),
        ]),
    );
    assert_eq!(answer.status, 0, "{answer:?}");
    let session = opened(answer);
    let socket = dir.surf().join(&session.id).join("session.sock");
    let program = stand_in_program();
    let tail = [
        "--".to_string(),
        program.display().to_string(),
        session.id.clone(),
        socket.display().to_string(),
    ];
    let mut prlimit_args: Vec<String> = script_bounds().to_vec();
    prlimit_args.extend(tail.iter().cloned());
    let mut setpriv_args = vec!["--no-new-privs".to_string(), prlimit.display().to_string()];
    setpriv_args.extend(prlimit_args.iter().cloned());
    let read_args = |name: &str| -> Vec<String> {
        fs::read_to_string(dir.control().join(format!("{name}-args")))
            .expect("the recorded arguments")
            .lines()
            .map(str::to_string)
            .collect()
    };
    assert_eq!(read_args("setpriv"), setpriv_args);
    assert_eq!(read_args("prlimit"), prlimit_args);
    // The environment `surfd` gives the first program. The recording shell of `setpriv` adds
    // variables of its own (`PWD`, `SHLVL`) to what it execs, so the `prlimit` wrapper's
    // environment says nothing about `surfd`; the real `setpriv` and `prlimit` pass theirs on
    // unchanged (the session's own is `Config::session_env` only, `hardening.rs`).
    assert_eq!(
        fs::read(dir.control().join("setpriv-environ")).expect("the environment"),
        format!("STAND_IN_DIR={}\0", dir.control().display()).into_bytes()
    );
}

/// A value of `arg scripts` other than `true` (here `TRUE`) takes the form without scripts:
/// only the core bound.
#[test]
fn an_arg_scripts_other_than_true_takes_the_form_without_scripts() {
    let dir = TestDir::new();
    let _daemon = start(config(&dir));
    let answer = call(&dir, &open_with(&[("scripts", "TRUE")]));
    assert_eq!(answer.status, 0, "{answer:?}");
    let session = opened(answer);
    let pid = dir.pid_of(&session.id);
    let own = std::process::id();
    for name in ["Max cpu time", "Max address space", "Max open files"] {
        assert_eq!(limit(pid, name), limit(own, name), "{name}");
    }
    assert_eq!(limit(pid, "Max core file size"), expected_core().0);
    assert_eq!(no_new_privs(pid), "1");
}

/// `setpriv` named by a path that is no executable file (a file of mode 0644, then a
/// directory): it counts as missing. A session without scripts starts without either program,
/// and an `open` with scripts is refused `scripts-unavailable` with status 3.
fn setpriv_that_is_no_executable_file(make: impl FnOnce(&Path)) {
    let dir = TestDir::new();
    let path = dir.path.join("setpriv");
    make(&path);
    let mut config = config(&dir);
    config.setpriv = path;
    let _daemon = start(config);
    let session = open(&dir);
    let pid = dir.pid_of(&session.id);
    assert_eq!(no_new_privs(pid), "0");
    let answer = call(&dir, &open_with(&[("scripts", "true")]));
    assert_eq!(answer.status, 3, "{answer:?}");
    assert!(
        answer
            .text
            .lines()
            .any(|line| line == "did refused scripts-unavailable"),
        "{answer:?}"
    );
}

#[test]
fn a_setpriv_without_the_execute_bit_counts_as_missing() {
    setpriv_that_is_no_executable_file(|path| {
        fs::write(path, "#!/bin/sh\nexec /usr/bin/setpriv \"$@\"\n").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o644)).unwrap();
    });
}

#[test]
fn a_setpriv_that_is_a_directory_counts_as_missing() {
    setpriv_that_is_no_executable_file(|path| fs::create_dir(path).unwrap());
}

/// `surfd` runs under a hard CPU limit below the 3,601 s the form with scripts asks for (a
/// service manager's `LimitCPU=`, a job's `ulimit -t`): `prlimit` cannot raise it and exits
/// non-zero before it runs the session. Q11 and the brief read that as a session with scripts
/// that cannot run under both programs: `did refused scripts-unavailable`, status 3. A session
/// without scripts still opens (its one bound lowers a limit).
#[test]
fn a_prlimit_that_fails_before_the_session_answers_scripts_unavailable() {
    let own = std::process::id();
    let lowered = Command::new(PRLIMIT)
        .args(["--pid", &own.to_string(), "--cpu=1000:1000"])
        .status()
        .expect("prlimit runs");
    assert!(lowered.success(), "{lowered}");
    assert_eq!(limit(own, "Max cpu time"), ("1000".to_string(), "1000".to_string()));
    let dir = TestDir::new();
    let _daemon = start(config(&dir));
    let session = open(&dir);
    assert_eq!(no_new_privs(dir.pid_of(&session.id)), "1");
    let answer = call(&dir, &open_with(&[("scripts", "true")]));
    assert_eq!(answer.status, 3, "{answer:?}");
    assert!(
        answer
            .text
            .lines()
            .any(|line| line == "did refused scripts-unavailable"),
        "{answer:?}"
    );
}

// --- The bounds as a boundary ------------------------------------------------------------------

/// The session program of a session with scripts is a sh script that, before it execs the
/// stand-in, starts a child that reads its own `NoNewPrivs`, and tries to raise the hard CPU
/// limit (3,601 to 3,602) and the hard open-files limit (64 to 65) with `prlimit`. The child has
/// no new privileges, and neither hard limit can be raised.
#[test]
fn a_child_of_a_session_has_no_new_privileges_and_cannot_raise_a_hard_limit() {
    let dir = TestDir::new();
    let control = dir.control();
    let program = script(
        &dir,
        "session",
        &format!(
            "/usr/bin/cat /proc/self/status > {status}\n\
             {PRLIMIT} --cpu=3600:3602 -- /usr/bin/true\n\
             echo $? > {cpu}\n\
             {PRLIMIT} --nofile=64:65 -- /usr/bin/true\n\
             echo $? > {nofile}\n\
             exec {stand_in} \"$@\"\n",
            status = quoted(&control.join("child-status")),
            cpu = quoted(&control.join("raise-cpu")),
            nofile = quoted(&control.join("raise-nofile")),
            stand_in = quoted(&stand_in_program()),
        ),
    );
    let mut config = config(&dir);
    config.session_program = program;
    let _daemon = start(config);
    let answer = call(&dir, &open_with(&[("scripts", "true")]));
    assert_eq!(answer.status, 0, "{answer:?}");
    let session = opened(answer);
    let pid = dir.pid_of(&session.id);
    assert_eq!(limit(pid, "Max cpu time"), ("3600".to_string(), "3601".to_string()));
    let child = fs::read_to_string(control.join("child-status")).expect("the child's status");
    assert_eq!(no_new_privs_in(&child), "1");
    for name in ["raise-cpu", "raise-nofile"] {
        let code = fs::read_to_string(control.join(name)).expect("the exit status");
        assert_ne!(code.trim(), "0", "{name}: the hard limit was raised");
    }
}
