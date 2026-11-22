//! The binary `surf-agent`: [`surf_agent::client::run_with_input`] over this process's words,
//! environment and standard streams.

#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::io;
use std::process::ExitCode;

use surf_agent::cli::Environment;
use surf_agent::client::{self, Config};

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
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
