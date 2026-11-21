//! The binary `surf-session`: [`surf_session::process::run`] over this process's words, with
//! standard output for its one `ready` line.

#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    ExitCode::from(surf_session::process::run(&args, &mut io::stdout()))
}
