//! Adversary cases against `story:session-core-limit-one`: a process of a session cannot raise
//! the core limit `surfd` chose, in either form; the whole start chain of both forms
//! (`setpriv --no-new-privs`, then `prlimit` with the bounds of the form) ends a process by
//! `SIGABRT` without a dump; and a hard core limit that is lowered after `Config::new` read it
//! refuses every `open`. A case that starts `surfd` runs it in the test process; a session
//! program a case writes is a POSIX sh script in the case's directory.

use crate::common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::*;
use surf_agent::wire::{Arg, Request};
use surfd::Config;

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

/// The soft and the hard value of the line `Max core file size` of process `pid`.
fn core_limit(pid: u32) -> (String, String) {
    const NAME: &str = "Max core file size";
    let limits = fs::read_to_string(format!("/proc/{pid}/limits")).expect("the limits");
    let line = limits
        .lines()
        .find(|line| line.starts_with(NAME) && line[NAME.len()..].starts_with("  "))
        .unwrap_or_else(|| panic!("no {NAME} in {limits}"));
    let mut values = line[NAME.len()..].split_whitespace();
    (
        values.next().expect("a soft value").to_string(),
        values.next().expect("a hard value").to_string(),
    )
}

/// The first line of the host's core pattern, empty when it cannot be read.
fn core_pattern() -> String {
    fs::read_to_string("/proc/sys/kernel/core_pattern")
        .unwrap_or_default()
        .lines()
        .next()
        .unwrap_or_default()
        .to_string()
}

/// The session program, before it execs the stand-in, tries to raise its own core limit with
/// `prlimit --pid $$`: to 4,096 bytes, one page, the smallest limit the kernel writes a core
/// file of the ELF format under, and to `unlimited`. Each try is refused, and the session keeps
/// the pair of [`expected_core`]. A hard value above 1 in the core item `surfd` chose would let
/// the session raise its soft value back and dump.
fn a_session_cannot_raise_its_core_limit(scripts: bool) {
    let dir = TestDir::new();
    let control = dir.control();
    let program = script(
        &dir,
        "session",
        &format!(
            "/usr/bin/prlimit --pid $$ --core=4096:4096 2> {page_errors}\n\
             echo $? > {page}\n\
             /usr/bin/prlimit --pid $$ --core=unlimited:unlimited 2> {unlimited_errors}\n\
             echo $? > {unlimited}\n\
             exec {stand_in} \"$@\"\n",
            page_errors = quoted(&control.join("raise-page-errors")),
            page = quoted(&control.join("raise-page")),
            unlimited_errors = quoted(&control.join("raise-unlimited-errors")),
            unlimited = quoted(&control.join("raise-unlimited")),
            stand_in = quoted(&stand_in_program()),
        ),
    );
    let mut config = config(&dir);
    config.session_program = program;
    let _daemon = start(config);
    let answer = call(&dir, &open_request(scripts));
    assert_eq!(answer.status, 0, "{answer:?}");
    let session = opened(answer);
    let pid = dir.pid_of(&session.id);
    for name in ["raise-page", "raise-unlimited"] {
        let code = fs::read_to_string(control.join(name)).expect("the exit status");
        assert_ne!(
            code.trim(),
            "0",
            "{name}: a process of the session raised its core limit"
        );
    }
    assert_eq!(core_limit(pid), expected_core().0);
}

#[test]
fn a_session_without_scripts_cannot_raise_its_core_limit() {
    a_session_cannot_raise_its_core_limit(false);
}

#[test]
fn a_session_with_scripts_cannot_raise_its_core_limit() {
    a_session_cannot_raise_its_core_limit(true);
}

/// The effect of the whole start chain, as `surfd` builds it for each form: `setpriv
/// --no-new-privs prlimit <the bounds of the form> -- /bin/sh -c 'kill -s ABRT $$'`, in an
/// empty directory. Each ends by the signal 6 with no dump, and the directory stays empty. It
/// prints the core item and the host's core pattern. Where it holds something: a pipe pattern
/// and a hard core limit of at least 1. Where it holds nothing: a hard core limit of 0 (it
/// prints `not shown: hard core limit 0` and ends) and a host without a pipe pattern (it prints
/// `not shown: no pipe pattern` and still asserts).
#[test]
fn the_start_chain_of_both_forms_ends_a_process_by_abort_without_a_dump() {
    let config = Config::new(None);
    let item = config
        .session_bounds
        .first()
        .expect("the core item")
        .clone();
    let pattern = core_pattern();
    eprintln!("core item {item} core_pattern {pattern}");
    if item == CORE_ARGUMENT_ZERO {
        eprintln!("not shown: hard core limit 0");
        return;
    }
    if !pattern.starts_with('|') {
        eprintln!("not shown: no pipe pattern");
    }
    assert_eq!(config.script_session_bounds.last(), Some(&item));
    let dir = TestDir::new();
    let empty = dir.path.join("empty");
    fs::create_dir(&empty).expect("an empty directory");
    for (form, bounds) in [
        ("without scripts", &config.session_bounds),
        ("with scripts", &config.script_session_bounds),
    ] {
        let status = Command::new(&config.setpriv)
            .arg("--no-new-privs")
            .arg(&config.prlimit)
            .args(bounds)
            .args(["--", "/bin/sh", "-c", "kill -s ABRT $$"])
            .current_dir(&empty)
            .status()
            .expect("setpriv runs");
        assert_eq!(status.signal(), Some(6), "{form}: {status}");
        assert!(!status.core_dumped(), "{form}: {status}");
    }
    let entries: Vec<_> = fs::read_dir(&empty)
        .expect("the directory")
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect();
    assert!(entries.is_empty(), "{entries:?}");
}

/// `surfd` reads its hard core limit once, in `Config::new`. When that limit is lowered to 0
/// after the read (`prlimit --pid <surfd> --core=0:0`, from outside), `prlimit` cannot set the
/// item it chose, and every `open` fails: without scripts with status 3 and `cannot start
/// <program>: ...`, with scripts with `did refused scripts-unavailable`. Release `0.9.0`, with
/// a core limit of 0, opened a session there. It needs one process per test, which
/// cargo-nextest gives.
#[test]
fn a_hard_core_limit_lowered_after_the_read_refuses_every_open() {
    let dir = TestDir::new();
    let config = config(&dir);
    if config.session_bounds[0] == CORE_ARGUMENT_ZERO {
        eprintln!("not shown: hard core limit 0");
        return;
    }
    let program = config.session_program.clone();
    let daemon = start(config);
    let own = std::process::id();
    let lowered = Command::new("/usr/bin/prlimit")
        .args(["--pid", &own.to_string(), CORE_ARGUMENT_ZERO])
        .status()
        .expect("prlimit runs");
    assert!(lowered.success(), "{lowered}");
    assert_eq!(core_limit(own), ("0".to_string(), "0".to_string()));
    let answer = call(&dir, &open_request(false));
    assert_eq!(answer.status, 3, "{answer:?}");
    assert!(
        answer
            .reason
            .as_deref()
            .unwrap_or_default()
            .starts_with(&format!("cannot start {}: ", program.display())),
        "{answer:?}"
    );
    let answer = call(&dir, &open_request(true));
    assert_eq!(answer.status, 3, "{answer:?}");
    assert!(
        answer
            .text
            .lines()
            .any(|line| line == "did refused scripts-unavailable"),
        "{answer:?}"
    );
    assert!(daemon.sessions().is_empty());
}
