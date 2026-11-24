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
        return print_definition();
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

/// Write the definition to standard output: exit 0, also when the reader closed the pipe early
/// (`| head -1`); exit 1 with one line on standard error when it cannot be assembled or written.
fn print_definition() -> ExitCode {
    let failed = |message: String| {
        let _ = writeln!(io::stderr().lock(), "surf-agent: {message}");
        ExitCode::from(1)
    };
    let text = match help::assemble(&SOURCES) {
        Ok(text) => text,
        Err(error) => return failed(format!("the definition cannot be assembled: {error}")),
    };
    let mut out = io::stdout().lock();
    match out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => failed(format!("cannot write the definition: {}", error.kind())),
    }
}
