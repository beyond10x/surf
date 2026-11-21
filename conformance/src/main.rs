//! `surf-conformance <suite.json> <report.json>`: runs the suite against the Rust `surf` system,
//! prints every diagnostic of a check that did not pass, then one line per scenario and the
//! summary line last, and writes the `ess-conformance-report/2` to `<report.json>`.
//!
//! Exit status 0 when every scenario passed, 1 when any failed, errored or was unsupported, or
//! when the suite has none, and 2 when the run could not be made.

#![forbid(unsafe_code)]

use std::process::ExitCode;

use ess_conformance::Status;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [suite, report] = arguments.as_slice() else {
        eprintln!("usage: surf-conformance <suite.json> <report.json>");
        return ExitCode::from(2);
    };
    let text = match std::fs::read_to_string(suite) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("{suite}: {error}");
            return ExitCode::from(2);
        }
    };
    let conformance = match surf_conformance::run(&text, &surf_conformance::memory_target()) {
        Ok(conformance) => conformance,
        Err(error) => {
            eprintln!("{suite}: {error}");
            return ExitCode::from(2);
        }
    };
    let written = conformance
        .report
        .to_canonical_json()
        .map_err(|error| error.to_string())
        .and_then(|json| std::fs::write(report, json).map_err(|error| error.to_string()));
    if let Err(error) = written {
        eprintln!("{report}: {error}");
        return ExitCode::from(2);
    }
    for scenario in &conformance.run.scenarios {
        for check in &scenario.checks {
            if check.status == Status::Passed {
                continue;
            }
            if let Some(diagnostic) = &check.diagnostic {
                println!("{check}\n{diagnostic}\n");
            }
        }
    }
    print!("{}", conformance.run.report());
    let counts = conformance.report.counts();
    if counts.total > 0 && counts.passed == counts.total {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
