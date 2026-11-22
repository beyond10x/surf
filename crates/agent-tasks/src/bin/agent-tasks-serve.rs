//! `agent-tasks-serve <sites dir>`: the fixture server of the agent tasks as a program, for the
//! model lane (`taskfiles/agent-tasks-model.sh`). It binds 127.0.0.1 port 0, prints
//! `http://127.0.0.1:<port>` as its first line and serves every task's site under `/<task>/`
//! until it is killed.

#![forbid(unsafe_code)]

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let (Some(sites), None) = (args.next(), args.next()) else {
        eprintln!("usage: agent-tasks-serve <sites dir>");
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
