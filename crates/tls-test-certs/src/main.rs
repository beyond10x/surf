//! `surf-tls-test-certs --out <dir> [--keys <dir>]`: makes the cases of `src/cases.rs` from the
//! keys (by default the `keys/` of this crate) and writes their files and `INDEX` under `<dir>`.
//! Run by the lane `slow` only (`task tls-certs-slow`).

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use surf_tls_test_certs::{cases, keys::Keys, output};

fn main() -> ExitCode {
    let mut out = None;
    let mut keys = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/keys"));
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match (arg.as_str(), args.next()) {
            ("--out", Some(dir)) => out = Some(PathBuf::from(dir)),
            ("--keys", Some(dir)) => keys = PathBuf::from(dir),
            _ => {
                eprintln!("usage: surf-tls-test-certs --out <dir> [--keys <dir>]");
                return ExitCode::from(2);
            }
        }
    }
    let Some(out) = out else {
        eprintln!("usage: surf-tls-test-certs --out <dir> [--keys <dir>]");
        return ExitCode::from(2);
    };
    let made = Keys::read(&keys).and_then(|keys| cases::build_all(&keys));
    let files = match made {
        Ok(cases) => output::files(&cases),
        Err(e) => {
            eprintln!("surf-tls-test-certs: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = output::write(&out, &files) {
        eprintln!("surf-tls-test-certs: {e}");
        return ExitCode::FAILURE;
    }
    println!(
        "surf-tls-test-certs: {} cases, {} files in {}",
        cases::specs().len(),
        files.len(),
        out.display()
    );
    ExitCode::SUCCESS
}
