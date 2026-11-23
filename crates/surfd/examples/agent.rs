//! `surf-agent` for the built harness of `surfd`'s tests: the same thin `main` as
//! `crates/agent/src/main.rs`, over [`surf_agent::client::run_with_input`] with this process's
//! standard input, so that `fill <handle> -` reads its value there. `cargo test -p surfd` builds no
//! binary of another package, so the tests copy this example as `surf-agent`.

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
