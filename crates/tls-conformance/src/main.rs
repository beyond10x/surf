//! `surf-tls-conformance`: runs the vendored x509-limbo cases through the path validation of
//! surf-tls and prints the counts (see the crate documentation for the lines). `--check --base
//! <revision>` holds them to `expected/limbo.txt` ([`check`]). `--root <dir>` reads the data
//! and the expected files from the repository at `<dir>` instead of the one the binary was
//! built in.
//!
//! Exit status: 0 after a run, whatever it counted, and after a check that found nothing; 1 when
//! a check failed or a file cannot be read; 2 for a wrong argument.
//!
//! [`check`]: surf_tls_conformance::check

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use surf_tls_conformance::{check, paths};

const USAGE: &str = "usage: surf-tls-conformance [--root <dir>] [--check --base <revision>]";

fn arguments(args: &[String]) -> Option<(PathBuf, Option<String>)> {
    let mut root = None;
    let mut check = false;
    let mut base = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--root" if root.is_none() => root = Some(PathBuf::from(rest.next()?)),
            "--check" if !check => check = true,
            "--base" if base.is_none() => base = Some(rest.next()?.clone()),
            _ => return None,
        }
    }
    if check != base.is_some() {
        return None;
    }
    Some((root.unwrap_or_else(paths::root), base))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((root, base)) = arguments(&args) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let printed = match &base {
        Some(base) => check::check(&root, base).map(|outcome| {
            let failed = !outcome.failures.is_empty();
            (outcome.printed(), failed)
        }),
        None => check::run(&root).map(|report| {
            let mut lines = report.count_lines();
            lines.extend(report.lines);
            (lines, false)
        }),
    };
    match printed {
        Ok((lines, failed)) => {
            for line in lines {
                println!("{line}");
            }
            if failed {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("surf-tls-conformance: {e}");
            ExitCode::from(1)
        }
    }
}
