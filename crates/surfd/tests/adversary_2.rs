//! Adversary cases of the second review of `story:session-process-hardening`, on fix rounds 2
//! and 3: the `note ended` line on the paths of an `open` that `abandon` ends (another first
//! line, the ready timeout), the waits of `not_started` (a session that closes its standard
//! output and stays), the word `bound` from both sides, and a slow start with scripts. Each case
//! starts one `surfd` in the test process; a session program a case writes is a POSIX sh script
//! in the case's directory.

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

const BEFORE_READY: &str = "the session ended or wrote another line before it reported ready";

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

/// Send `signal` (`XCPU`) to process `pid` with kill(1).
fn send(pid: u32, signal: &str) {
    let status = Command::new("kill")
        .arg(format!("-{signal}"))
        .arg(pid.to_string())
        .status()
        .expect("kill runs");
    assert!(status.success(), "kill -{signal} {pid}: {status}");
}

/// The identity of the head line `surf 1 session <id> died`.
fn died_id(answer: &Answer) -> String {
    answer
        .text
        .lines()
        .next()
        .and_then(|head| head.strip_prefix("surf 1 session "))
        .and_then(|rest| rest.strip_suffix(" died"))
        .unwrap_or_else(|| panic!("no head line of a died session: {answer:?}"))
        .to_string()
}

fn died_with(answer: &Answer, note: &str) -> String {
    format!(
        "{}{note}\n",
        refusal(&died_id(answer), "died", "session-died")
    )
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

// --- abandon: the paths of another first line and of the ready timeout --------------------------

/// The session writes a first line that is not `ready` and stays: `surfd` kills it itself, so
/// the answer names no signal (its `SIGKILL` is its own).
#[test]
fn another_first_line_from_a_live_session_carries_no_note() {
    let dir = TestDir::new();
    let mut config = config(&dir);
    config.session_program = script(
        &dir,
        "session",
        "printf 'not ready\\n'\nexec /usr/bin/sleep 30\n",
    );
    let _daemon = start(config);
    let answer = call(&dir, &open_request(true));
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(answer.reason.as_deref(), Some(BEFORE_READY), "{answer:?}");
    assert_eq!(answer.text, "", "{answer:?}");
}

/// The session writes nothing within the ready timeout and stays: `surfd` kills it itself, so
/// the answer names no signal.
#[test]
fn a_ready_timeout_of_a_live_session_carries_no_note() {
    let dir = TestDir::new();
    dir.set("ready", "never");
    let mut config = config(&dir);
    config.session_ready_timeout = Duration::from_millis(300);
    let _daemon = start(config);
    let answer = call(&dir, &open_request(true));
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(
        answer.reason.as_deref(),
        Some("the session did not report ready within 0.3 seconds"),
        "{answer:?}"
    );
    assert_eq!(answer.text, "", "{answer:?}");
}

/// The session shell ends itself by `SIGUSR1` at once; a child it started writes a first line
/// that is not `ready` once the session shell has ended. The session died by a signal `surfd`
/// did not send before its first byte (`docs/agent-answer.md`, "from the session's start on"):
/// the answer names it, and not `bound cpu`.
///
/// The signal is one that dumps no core: after a signal that dumps core the shell is waitable
/// only once the host's core handler has taken the dump, which can take seconds under load.
///
/// The order is made by an event, not a clock: the child reads `/proc/<pid>/stat` of the shell
/// (`$$`) every 10 ms until the state after the last `)` is `Z` (ended, its status not yet
/// collected) or the file is gone, and only then writes. A read that fails is taken as the
/// shell's end. A closed pipe is no such event: the kernel closes a process's descriptors
/// before the process is waitable. After 300 turns the child gives up: it makes the file
/// `gave-up` and exits without a line, an end whose answer is the one asserted here, so the
/// case fails on that file. The ready timeout of 30 s lets the 300 turns end first; a green run
/// answers long before it.
#[test]
fn another_first_line_after_the_session_died_by_its_own_signal_names_it() {
    let dir = TestDir::new();
    let gave_up = dir.path.join("gave-up");
    let mut config = config(&dir);
    config.session_ready_timeout = Duration::from_secs(30);
    config.session_program = script(
        &dir,
        "session",
        &format!(
            "(\n\
             i=0\n\
             while [ \"$i\" -lt 300 ]; do\n\
             read -r stat < /proc/$$/stat || break\n\
             case ${{stat##*')'}} in ' Z '*) break ;; esac\n\
             /usr/bin/sleep 0.01\n\
             i=$((i + 1))\n\
             done\n\
             if [ \"$i\" -ge 300 ]; then : > {gave_up}; exit 0; fi\n\
             printf 'x\\n'\n\
             ) &\n\
             kill -s USR1 $$\n",
            gave_up = quoted(&gave_up),
        ),
    );
    let _daemon = start(config);
    let answer = call(&dir, &open_request(true));
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(answer.reason.as_deref(), Some(BEFORE_READY), "{answer:?}");
    assert_eq!(
        answer.text,
        died_with(&answer, "note ended: signal SIGUSR1")
    );
    assert!(
        !gave_up.exists(),
        "the child's loop of 300 turns gave up before the session shell was waitable: {answer:?}"
    );
}

/// The session shell ends itself by `SIGUSR1` at once; a child it started holds its standard
/// output until `surfd` has collected the shell at the ready timeout of 2 s. The answer names
/// `SIGUSR1`.
///
/// The signal is one that dumps no core: after a signal that dumps core the shell is waitable
/// only once the host's core handler has taken the dump, which can take seconds under load.
///
/// One order is a clock: the shell must have ended before the ready timeout, which starts
/// inside `open` right after the spawn and which a test cannot hold back; the margin is the
/// timeout of 2 s for a shell that ends in milliseconds. The other order is an event: the child
/// must hold the output past the ready timeout (a child that lets go earlier takes the path of
/// a session that wrote nothing) and let go within `abandon`'s wait. At the timeout `abandon`
/// collects the ended shell, and from then on `/proc/<pid>/stat` of the shell (`$$`) is gone.
/// The child tests for that file every 10 ms and exits once it is gone; after 1,000 turns it
/// gives up and exits, after `abandon`'s wait, and the answer is red. The second margin: the
/// child must see the file gone and exit within the kill grace of 2 s, against a turn of 10 ms
/// and the start of one `sleep`. The case lasts about 2 s.
#[test]
fn a_ready_timeout_after_the_session_died_by_its_own_signal_names_it() {
    let dir = TestDir::new();
    let mut config = config(&dir);
    config.session_ready_timeout = Duration::from_secs(2);
    config.session_program = script(
        &dir,
        "session",
        "(\n\
         i=0\n\
         while [ -e /proc/$$/stat ] && [ \"$i\" -lt 1000 ]; do\n\
         /usr/bin/sleep 0.01\n\
         i=$((i + 1))\n\
         done\n\
         ) &\n\
         kill -s USR1 $$\n",
    );
    let _daemon = start(config);
    let answer = call(&dir, &open_request(false));
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(
        answer.reason.as_deref(),
        Some("the session did not report ready within 2 seconds"),
        "{answer:?}"
    );
    assert_eq!(
        answer.text,
        died_with(&answer, "note ended: signal SIGUSR1")
    );
}

// --- not_started: a session that closes its standard output and stays ---------------------------

/// The session program runs the stand-in, or, while the file `mute` is in the stand-in's
/// directory, closes its standard output and sleeps. With one session open, 8 `open`s (4 with
/// scripts, 4 without) of muted sessions run at once. Each waits the kill grace (2 s) in
/// `not_started`, then `surfd` kills the session: the answer is the before-ready one, with no
/// text, never `scripts-unavailable` (the session did not end by an exit). The opens run side by
/// side, not one after another, and a `look` of the open session is answered while they wait.
#[test]
fn sessions_that_close_their_output_and_stay_hold_only_their_own_open() {
    let dir = TestDir::new();
    let mute = dir.control().join("mute");
    let mut config = config(&dir);
    config.session_program = script(
        &dir,
        "session",
        &format!(
            "if [ -e {mute} ]; then exec >&-; exec /usr/bin/sleep 30; fi\nexec {stand_in} \"$@\"\n",
            mute = quoted(&mute),
            stand_in = quoted(&stand_in_program()),
        ),
    );
    let _daemon = start(config);
    let session = open(&dir);
    fs::write(&mute, "").unwrap();
    let began = Instant::now();
    let (answers, looked) = thread::scope(|scope| {
        let opens: Vec<_> = (0..8)
            .map(|n| {
                let dir = &dir;
                scope.spawn(move || {
                    let answer = call(dir, &open_request(n % 2 == 0));
                    (answer, began.elapsed())
                })
            })
            .collect();
        // While the opens wait the kill grace.
        thread::sleep(Duration::from_millis(500));
        let asked = Instant::now();
        let answer = look(&dir, &session);
        let looked = (answer, asked.elapsed(), began.elapsed());
        let answers: Vec<_> = opens.into_iter().map(|o| o.join().unwrap()).collect();
        (answers, looked)
    });
    let (look_answer, look_took, look_at) = looked;
    eprintln!("look: answered in {look_took:?}, {look_at:?} after the opens began");
    assert_eq!(look_answer.status, 0, "{look_answer:?}");
    assert!(look_took < Duration::from_millis(500), "look took {look_took:?}");
    let mut longest = Duration::ZERO;
    for (answer, took) in &answers {
        eprintln!("open: {took:?}, {:?}", answer.reason);
        assert_eq!(answer.status, 3, "{answer:?}");
        assert_eq!(answer.reason.as_deref(), Some(BEFORE_READY), "{answer:?}");
        assert_eq!(answer.text, "", "{answer:?}");
        longest = longest.max(*took);
    }
    let shortest = answers.iter().map(|(_, took)| *took).min().unwrap();
    assert!(shortest >= KILL_GRACE_OF_SURFD, "shortest open {shortest:?}");
    // One after another they would take 8 kill graces.
    assert!(longest < KILL_GRACE_OF_SURFD * 2, "longest open {longest:?}");
}

/// `surfd::KILL_GRACE`, the default `Config::kill_grace`.
const KILL_GRACE_OF_SURFD: Duration = surfd::KILL_GRACE;

// --- The word bound, from both sides ------------------------------------------------------------

/// A session with scripts, whose bounds `surfd` sets without a CPU bound (a `Config` of the
/// test), that ends by `SIGXCPU`: the line has no `bound`; the view lists `SIGXCPU`.
#[test]
fn sigxcpu_without_a_cpu_bound_of_surfd_reads_no_bound_also_with_scripts() {
    let dir = TestDir::new();
    let mut config = config(&dir);
    config.script_session_bounds = vec![
        "--nofile=64:64".to_string(),
        config.session_bounds[0].clone(),
    ];
    let daemon = start(config);
    let answer = call(&dir, &open_request(true));
    assert_eq!(answer.status, 0, "{answer:?}");
    let session = opened(answer);
    send(dir.pid_of(&session.id), "XCPU");
    let answer = died_and_look(&daemon, &dir, &session);
    assert_eq!(
        answer.text,
        format!(
            "{}note ended: signal SIGXCPU\n",
            refusal(&session.id, "died", "session-died")
        )
    );
    let summary = listed(&sessions(&view_url(&dir)), &session.id);
    assert_eq!(summary.get("signal"), &json::Value::Str("SIGXCPU".into()));
}

/// A session with scripts under the shipped bounds that ends by `SIGXFSZ`: no `bound` (surfd
/// sets no file-size bound), and the answer parses.
#[test]
fn sigxfsz_reads_no_bound() {
    let dir = TestDir::new();
    let daemon = start(config(&dir));
    let answer = call(&dir, &open_request(true));
    assert_eq!(answer.status, 0, "{answer:?}");
    let session = opened(answer);
    send(dir.pid_of(&session.id), "XFSZ");
    let answer = died_and_look(&daemon, &dir, &session);
    assert_eq!(
        answer.text,
        format!(
            "{}note ended: signal SIGXFSZ\n",
            refusal(&session.id, "died", "session-died")
        )
    );
    assert!(surf_answer::parse(&answer.text).is_ok());
}

/// `surfd` started without `setpriv` (a path naming no file): a session without scripts runs
/// without either program and so without a CPU bound; ended by `SIGXCPU` it reads no `bound`.
/// No case before this one had a session without the programs end by a signal, so none read
/// `cpu_bound` of `open_session` for it (`is_some_and` there turned to `is_none_or`). The
/// session program is a sh script that sets the core argument `Config::new` chose on itself with
/// `prlimit --pid` and execs the stand-in: that limit is the test's own, not one of `surfd`.
#[test]
fn sigxcpu_to_a_session_started_without_the_programs_reads_no_bound() {
    let dir = TestDir::new();
    let mut config = config(&dir);
    config.setpriv = dir.path.join("no-setpriv");
    config.session_program = script(
        &dir,
        "session",
        &format!(
            "/usr/bin/prlimit {} --pid $$\nexec {} \"$@\"\n",
            config.session_bounds[0],
            quoted(&stand_in_program())
        ),
    );
    let daemon = start(config);
    let session = open(&dir);
    send(dir.pid_of(&session.id), "XCPU");
    let answer = died_and_look(&daemon, &dir, &session);
    assert_eq!(
        answer.text,
        format!(
            "{}note ended: signal SIGXCPU\n",
            refusal(&session.id, "died", "session-died")
        )
    );
}

// --- A slow start with scripts ------------------------------------------------------------------

/// A session with scripts that is fine but slow (0.5 s before its program runs) opens; it is
/// not answered `scripts-unavailable`.
#[test]
fn a_slow_session_with_scripts_opens() {
    let dir = TestDir::new();
    let mut config = config(&dir);
    config.session_program = script(
        &dir,
        "session",
        &format!(
            "/usr/bin/sleep 0.5\nexec {} \"$@\"\n",
            quoted(&stand_in_program())
        ),
    );
    let _daemon = start(config);
    let answer = call(&dir, &open_request(true));
    assert_eq!(answer.status, 0, "{answer:?}");
}
