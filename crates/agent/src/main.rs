//! The binary `surf-agent`: [`surf_agent::client::run_with_input`] over this process's words,
//! environment and standard streams; and `surf-agent help --for-agent`, which prints the
//! definition of [`surf_agent::help`] and asks no session for anything.

#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::io::{self, Write};
use std::process::ExitCode;

use surf_agent::cli::{self, Environment};
use surf_agent::client::{self, Config};
use surf_agent::help::{self, Sources};

/// The sources of the definition, compiled into the binary.
const SOURCES: Sources<'static> = Sources {
    usage: cli::USAGE,
    binding: include_str!("../../../cli/surf-agent.yaml"),
    answer_document: include_str!("../../../docs/agent-answer.md"),
    open: include_str!("../../session/tests/golden/open.txt"),
    go: include_str!("../../session/tests/golden/go.txt"),
    look: include_str!("../../session/tests/golden/look.txt"),
};

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    // Before the environment is read: the definition needs no XDG_RUNTIME_DIR and no `surfd`.
    if cli::is_help_for_agent(&args) {
        let mut out = io::stdout().lock();
        let _ = out.write_all(help::assemble(&SOURCES).as_bytes());
        let _ = out.flush();
        return ExitCode::SUCCESS;
    }
    let status = client::run_with_input(
        &args,
        &Environment::from_process(),
        &Config::default(),
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    );
    ExitCode::from(status)
}
