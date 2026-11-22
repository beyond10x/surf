//! The binary `surfd`: [`surfd::Daemon::start`] with this process's `XDG_RUNTIME_DIR` and the
//! options `--idle-exit <seconds>` and `--print-view-url`, its start line on standard output
//! and one line about the live view on standard error. By default that line names the file that
//! holds the view's address and token, `surfd: the live view's address is in
//! $XDG_RUNTIME_DIR/surf/view-url`, and holds no token; with `--print-view-url` it is the
//! address itself, `http://127.0.0.1:<port>/#<token>`, for a person who asks for it. Exit status
//! 0 after the idle exit or when another `surfd` serves; 1 when it refuses to start; 2 for any
//! other argument.

#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use surfd::{Config, Daemon, Started};

const USAGE: &str = "usage: surfd [--idle-exit <seconds>] [--print-view-url]";

/// What the arguments ask for.
struct Options {
    idle_exit: Option<Duration>,
    print_view_url: bool,
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let Some(options) = options(&args) else {
        let mut out = io::stdout().lock();
        let _ = writeln!(out, "{} {USAGE}", surf_agent::wire::REFUSED);
        let _ = out.flush();
        return ExitCode::from(2);
    };
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from);
    let view_url = runtime_dir.as_ref().map(|dir| {
        dir.join(surf_agent::wire::DIRECTORY)
            .join(surfd::directory::VIEW_URL)
    });
    let mut config = Config::new(runtime_dir);
    if let Some(idle_exit) = options.idle_exit {
        config.idle_exit = idle_exit;
    }
    config.start_line = Some(Box::new(io::stdout()));
    if options.print_view_url {
        config.view_line = Some(Box::new(io::stderr()));
    }
    match Daemon::start(config) {
        Ok(Started::Daemon(daemon)) => {
            if let (false, Some(path)) = (options.print_view_url, &view_url) {
                let mut err = io::stderr().lock();
                let _ = writeln!(
                    err,
                    "surfd: the live view's address is in {}",
                    path.display()
                );
                let _ = err.flush();
            }
            daemon.wait();
            ExitCode::SUCCESS
        }
        Ok(Started::Another) => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(1),
    }
}

/// The options of `args`, each at most once in any order: `--idle-exit <seconds>` (a whole
/// number from 1) and `--print-view-url`; `None` for anything else.
fn options(args: &[OsString]) -> Option<Options> {
    let mut options = Options {
        idle_exit: None,
        print_view_url: false,
    };
    let mut rest = args.iter();
    while let Some(option) = rest.next() {
        if option == "--idle-exit" && options.idle_exit.is_none() {
            let seconds = rest.next()?.to_str()?;
            if seconds.is_empty() || !seconds.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let seconds: u32 = seconds.parse().ok()?;
            if seconds == 0 {
                return None;
            }
            options.idle_exit = Some(Duration::from_secs(u64::from(seconds)));
        } else if option == "--print-view-url" && !options.print_view_url {
            options.print_view_url = true;
        } else {
            return None;
        }
    }
    Some(options)
}
