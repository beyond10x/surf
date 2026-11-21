//! `surf-html-conformance`: runs the vendored parser data against `surf-html` and prints the
//! counts (see the crate documentation for the lines). `--list-failing <tag>` prints
//! `<path> <case number>` for each case of that tag that runs and does not pass. `--check
//! --base <revision>` holds the counts against the expected files ([`check`]). `--root <dir>`
//! reads the note, the data and the expected files from the repository at `<dir>` instead of
//! the one the binary was built in.
//!
//! Exit status: 0 after a run, whatever it counted, and after a check that found nothing; 1 when
//! a check failed, or the note or a data file cannot be read; 2 for a wrong argument.
//!
//! [`check`]: surf_html_conformance::check

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use surf_html_conformance::{check, inventory, paths, runner};

const USAGE: &str = "usage: surf-html-conformance [--root <dir>] \
                     [--list-failing <tag> | --check --base <revision>]";

enum Mode {
    Print,
    ListFailing(String),
    Check(String),
}

fn arguments(args: &[String]) -> Option<(PathBuf, Mode)> {
    let mut root = None;
    let mut list_failing = None;
    let mut check = false;
    let mut base = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--root" if root.is_none() => root = Some(PathBuf::from(rest.next()?)),
            "--list-failing" if list_failing.is_none() => list_failing = Some(rest.next()?.clone()),
            "--check" if !check => check = true,
            "--base" if base.is_none() => base = Some(rest.next()?.clone()),
            _ => return None,
        }
    }
    let mode = match (list_failing, check, base) {
        (None, false, None) => Mode::Print,
        (Some(tag), false, None) => Mode::ListFailing(tag),
        (None, true, Some(base)) => Mode::Check(base),
        _ => return None,
    };
    Some((root.unwrap_or_else(paths::root), mode))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((root, mode)) = arguments(&args) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    if let Mode::Check(base) = &mode {
        return match check::check(&root, base) {
            Ok(outcome) => {
                for line in outcome.printed() {
                    println!("{line}");
                }
                if outcome.failures.is_empty() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
            Err(e) => {
                eprintln!("surf-html-conformance: {e}");
                ExitCode::from(1)
            }
        };
    }
    let inventory = match inventory::load(&root.join(check::NOTE)) {
        Ok(inventory) => inventory,
        Err(e) => {
            eprintln!("surf-html-conformance: {e}");
            return ExitCode::from(1);
        }
    };
    if let Mode::ListFailing(tag) = &mode
        && !inventory.tags.iter().any(|(name, _)| name == tag)
    {
        eprintln!("surf-html-conformance: `{tag}` is not a tag of the inventory\n{USAGE}");
        return ExitCode::from(2);
    }
    let report = match runner::run_all(&root.join(check::DATA), &inventory) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("surf-html-conformance: {e}");
            return ExitCode::from(1);
        }
    };
    match mode {
        Mode::ListFailing(tag) => {
            for (path, number, name) in &report.failing {
                if *name == tag {
                    println!("{path} {number}");
                }
            }
        }
        _ => {
            for line in report.lines() {
                println!("{line}");
            }
        }
    }
    ExitCode::SUCCESS
}
