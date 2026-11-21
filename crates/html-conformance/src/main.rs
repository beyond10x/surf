//! `surf-html-conformance`: runs the vendored parser data against `surf-html` and prints the
//! counts (see the crate documentation for the lines). `--list-failing <tag>` prints
//! `<path> <case number>` for each case of that tag that runs and does not pass.
//!
//! Exit status: 0 after a run, whatever it counted; 1 when the note or a data file cannot be
//! read; 2 for a wrong argument.

#![forbid(unsafe_code)]

use std::process::ExitCode;

use surf_html_conformance::{inventory, paths, runner};

const USAGE: &str = "usage: surf-html-conformance [--list-failing <tag>]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let list_failing = match args.as_slice() {
        [] => None,
        [flag, tag] if flag == "--list-failing" => Some(tag.clone()),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let inventory = match inventory::load(&paths::note()) {
        Ok(inventory) => inventory,
        Err(e) => {
            eprintln!("surf-html-conformance: {e}");
            return ExitCode::from(1);
        }
    };
    if let Some(tag) = &list_failing
        && !inventory.tags.iter().any(|(name, _)| name == tag)
    {
        eprintln!("surf-html-conformance: `{tag}` is not a tag of the inventory\n{USAGE}");
        return ExitCode::from(2);
    }
    let report = match runner::run_all(&paths::data(), &inventory) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("surf-html-conformance: {e}");
            return ExitCode::from(1);
        }
    };
    match list_failing {
        Some(tag) => {
            for (path, number, name) in &report.failing {
                if *name == tag {
                    println!("{path} {number}");
                }
            }
        }
        None => {
            for line in report.lines() {
                println!("{line}");
            }
        }
    }
    ExitCode::SUCCESS
}
