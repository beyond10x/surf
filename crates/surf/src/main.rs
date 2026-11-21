//! `surf <url>`: load one address and print the rows it ends with.
//!
//! The output format and the exit statuses are documented in the repository's `README.md`.

#![forbid(unsafe_code)]

use std::env;
use std::fmt;
use std::io::{self, Write};
use std::process::ExitCode;

use surf::{Answer, EXIT_STOPPED, EXIT_USAGE, MemoryBrowser, answer, escape, exit_status};
use surf_fetch::{Limits, fetch};
use surf_ports::ComponentPorts;

const USAGE: &str = "usage: surf <url>";

/// Write one line to standard error. A standard error that cannot be written is ignored: the
/// exit status still says what happened.
fn diagnose(line: fmt::Arguments<'_>) {
    let _ = writeln!(io::stderr().lock(), "{line}");
}

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let (Some(address), None) = (args.next(), args.next()) else {
        diagnose(format_args!("{USAGE}"));
        return ExitCode::from(EXIT_USAGE);
    };
    let Ok(address) = address.into_string() else {
        diagnose(format_args!("surf: the address is not valid UTF-8"));
        diagnose(format_args!("{USAGE}"));
        return ExitCode::from(EXIT_USAGE);
    };
    let limits = Limits::default();
    let transport = move |address: &str| -> Answer {
        let result = fetch(address, &limits);
        if let Err(failure) = &result {
            // The failure's detail can quote the address and the operating system's text, so
            // the whole line is escaped, not only the address.
            diagnose(format_args!(
                "surf: {}",
                escape(&format!("{address}: {failure}"))
            ));
        }
        answer(result)
    };
    let mut browser = MemoryBrowser::from_ports(ComponentPorts::new(), transport);
    let result = browser.load(&address);
    match &result {
        Ok(report) => {
            let mut out = io::stdout().lock();
            let written = report
                .lines()
                .iter()
                .try_for_each(|line| writeln!(out, "{line}"))
                .and_then(|()| out.flush());
            if let Err(error) = written {
                diagnose(format_args!(
                    "surf: standard output: {}",
                    escape(&error.to_string())
                ));
                return ExitCode::from(EXIT_STOPPED);
            }
        }
        // An `Error` displays static text of the generated code and identities this process
        // minted; nothing in it came from the address or a server.
        Err(error) => diagnose(format_args!("surf: {error}")),
    }
    ExitCode::from(exit_status(&result))
}
