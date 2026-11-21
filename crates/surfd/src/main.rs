//! The binary `surfd`: [`surfd::Daemon::start`] with this process's `XDG_RUNTIME_DIR` and the
//! one option `--idle-exit <seconds>`, its start line on standard output and the live view's
//! line, `http://127.0.0.1:<port>/#<token>`, on standard error. Exit status 0 after
//! the idle exit or when another `surfd` serves; 1 when it refuses to start; 2 for any other
//! argument.

#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use surfd::{Config, Daemon, Started};

const USAGE: &str = "usage: surfd [--idle-exit <seconds>]";

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let Some(idle_exit) = idle_exit(&args) else {
        let mut out = io::stdout().lock();
        let _ = writeln!(out, "{} {USAGE}", surf_agent::wire::REFUSED);
        let _ = out.flush();
        return ExitCode::from(2);
    };
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from);
    let mut config = Config::new(runtime_dir);
    if let Some(idle_exit) = idle_exit {
        config.idle_exit = idle_exit;
    }
    config.start_line = Some(Box::new(io::stdout()));
    config.view_line = Some(Box::new(io::stderr()));
    match Daemon::start(config) {
        Ok(Started::Daemon(daemon)) => {
            daemon.wait();
            ExitCode::SUCCESS
        }
        Ok(Started::Another) => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(1),
    }
}

/// `Some(None)` for no argument, `Some(Some(seconds))` for `--idle-exit <seconds>` (a whole
/// number from 1), `None` for anything else.
fn idle_exit(args: &[OsString]) -> Option<Option<Duration>> {
    match args {
        [] => Some(None),
        [option, seconds] if option == "--idle-exit" => {
            let seconds = seconds.to_str()?;
            if seconds.is_empty() || !seconds.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let seconds: u32 = seconds.parse().ok()?;
            (seconds > 0).then(|| Some(Duration::from_secs(u64::from(seconds))))
        }
        _ => None,
    }
}
