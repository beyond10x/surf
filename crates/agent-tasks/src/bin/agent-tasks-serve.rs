//! `agent-tasks-serve <sites dir>`: the fixture server of the agent tasks as a program, for the
//! model lane (`taskfiles/agent-tasks-model.sh`). It binds 127.0.0.1 port 0, prints
//! `http://127.0.0.1:<port>` as its first line and serves every task's site under `/<task>/`
//! until it is killed.
//!
//! `agent-tasks-serve --bounded <seconds> <state dir> -- <command> [<argument>...]`: the model
//! lane's runner of its model command, here so that no second program is linked. It starts the
//! command in a process group of its own, with standard input, output and error passed through,
//! and writes the group's id to `<state dir>/group`. When the command ends, or `<seconds>` pass
//! first, it sends SIGKILL to the whole group, so no child the command started in it is left,
//! and waits at most 2 s until no process of the group runs. It exits with the command's status
//! (128 + the signal when a signal ended it); when the bound passed it writes
//! `<state dir>/bound-passed` and exits 124, and the file, not the status, tells that apart
//! from a command that exits 124 by itself. A command that cannot start exits 127.

#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// The pause between two looks at the command.
const POLL: Duration = Duration::from_millis(10);

/// How long the group may take to end after SIGKILL.
const GROUP_END: Duration = Duration::from_secs(2);

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--bounded") {
        return bounded(&args[1..]);
    }
    let [sites] = args.as_slice() else {
        eprintln!(
            "usage: agent-tasks-serve <sites dir>\n       \
             agent-tasks-serve --bounded <seconds> <state dir> -- <command> [<argument>...]"
        );
        return ExitCode::from(2);
    };
    let server = match surf_agent_tasks::server::serve(PathBuf::from(sites)) {
        Ok(server) => server,
        Err(error) => {
            eprintln!("agent-tasks-serve: cannot listen on 127.0.0.1: {error}");
            return ExitCode::from(1);
        }
    };
    let mut out = std::io::stdout().lock();
    if writeln!(out, "{}", server.origin())
        .and_then(|()| out.flush())
        .is_err()
    {
        return ExitCode::from(1);
    }
    drop(out);
    loop {
        thread::park();
    }
}

/// `--bounded <seconds> <state dir> -- <command> [<argument>...]`, the arguments after the flag.
fn bounded(args: &[OsString]) -> ExitCode {
    let (Some(seconds), Some(state), Some(dashes), Some(command)) =
        (args.first(), args.get(1), args.get(2), args.get(3))
    else {
        eprintln!(
            "usage: agent-tasks-serve --bounded <seconds> <state dir> -- <command> [<argument>...]"
        );
        return ExitCode::from(2);
    };
    let bound = match seconds.to_str().and_then(|text| text.parse::<u64>().ok()) {
        Some(n) if n >= 1 && dashes == "--" => Duration::from_secs(n),
        _ => {
            eprintln!(
                "agent-tasks-serve: --bounded takes seconds of at least 1, a state directory and --"
            );
            return ExitCode::from(2);
        }
    };
    let state = Path::new(state);
    let mut child = match Command::new(command)
        .args(&args[4..])
        .process_group(0)
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            eprintln!(
                "agent-tasks-serve: cannot start {}: {error}",
                Path::new(command).display()
            );
            return ExitCode::from(127);
        }
    };
    let group = child.id();
    if let Err(error) = fs::write(state.join("group"), format!("{group}\n")) {
        eprintln!(
            "agent-tasks-serve: cannot write {}: {error}",
            state.display()
        );
        kill_group(group);
        let _ = child.wait();
        return ExitCode::from(1);
    }
    let deadline = Instant::now() + bound;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() >= deadline => break None,
            Ok(None) => thread::sleep(POLL),
            Err(_) => break None,
        }
    };
    // kill(1) fails when no process is in the group: then there is nothing to wait for, and
    // /proc is not read (reading it costs more than a model that left no child).
    let signalled = kill_group(group);
    let _ = child.wait();
    if signalled {
        wait_group_ended(group);
    }
    match status {
        Some(status) => ExitCode::from(match (status.code(), status.signal()) {
            (Some(code), _) => code as u8,
            (None, Some(signal)) => (128 + signal) as u8,
            (None, None) => 1,
        }),
        None => {
            let _ = fs::write(state.join("bound-passed"), format!("{}\n", bound.as_secs()));
            ExitCode::from(124)
        }
    }
}

/// SIGKILL to every process of the group `group`, with `kill -9 -- -<group>`: whether a process
/// of the group was there to be signalled.
fn kill_group(group: u32) -> bool {
    Command::new("kill")
        .args(["-9", "--", &format!("-{group}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Wait at most [`GROUP_END`] until no process of the group runs: none in /proc has it as its
/// group, a zombie aside. Without /proc it returns at once.
fn wait_group_ended(group: u32) {
    let deadline = Instant::now() + GROUP_END;
    while Instant::now() < deadline && group_runs(group) {
        thread::sleep(POLL);
    }
}

/// Whether a process in /proc that is not a zombie has the group `group`.
fn group_runs(group: u32) -> bool {
    let Ok(entries) = fs::read_dir("/proc") else {
        return false;
    };
    entries.flatten().any(|entry| {
        let Ok(stat) = fs::read_to_string(entry.path().join("stat")) else {
            return false;
        };
        let Some((_, rest)) = stat.rsplit_once(')') else {
            return false;
        };
        let fields: Vec<&str> = rest.split_whitespace().collect();
        // After the `)`: state, parent, group.
        fields.get(2).and_then(|text| text.parse::<u32>().ok()) == Some(group)
            && !matches!(fields.first(), Some(&("Z" | "X" | "x")))
    })
}
