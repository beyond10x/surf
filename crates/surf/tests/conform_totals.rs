//! `task conform-totals` (Taskfile.yml): the Rust system ran the same suite as the interpreter and
//! as many scenarios. It reads the two `ess-conformance-report/2` files `task conform` and
//! `task conform-rust` write under `.out/`, and fails, printing both totals, when they are of
//! different suites or differ in their total. Each case writes two reports into a directory of its
//! own under `CARGO_TARGET_TMPDIR` and runs the task's script there,
//! `sh taskfiles/conform-totals.sh`, without `task`. The reports carry only the members the task
//! reads, plus a decoy `total` outside `counts`. Both writers indent their reports today; the
//! Rust one is written here on one line as well, so the task is shown not to depend on the
//! layout.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SUITE: &str = "sha256:1778f0743d6581d241eab897a596eecbc96b8ab1125febc21b270eed2f68528a";
const OTHER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The interpreter's report, as `ess verify conform run --report-format 2` indents it.
fn interpreter(total: u32, digest: &str) -> String {
    format!(
        "{{\n  \"counts\": {{\n    \"error\": 0,\n    \"failed\": 0,\n    \"passed\": {total},\n    \
         \"total\": {total},\n    \"unsupported\": 0\n  }},\n  \"outcomes\": {{\n    \"total\": 7\n  }},\n  \
         \"suite\": {{\n    \"digest\": \"{digest}\",\n    \"version\": \"ess-conformance/36\"\n  }}\n}}\n"
    )
}

/// A report on one line.
fn rust(total: u32, digest: &str) -> String {
    format!(
        "{{\"counts\":{{\"error\":0,\"failed\":0,\"passed\":{total},\"total\":{total},\
         \"unsupported\":0}},\"outcomes\":{{\"total\":7}},\"suite\":{{\"digest\":\"{digest}\",\
         \"version\":\"ess-conformance/36\"}}}}"
    )
}

/// A copy named `name` with the interpreter's report `report` and the Rust one `rust_report`;
/// `None` leaves that file out.
fn copy(name: &str, report: Option<String>, rust_report: Option<String>) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("conform-totals")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".out")).expect("create copy");
    if let Some(report) = report {
        fs::write(dir.join(".out/report.json"), report).expect("write report.json");
    }
    if let Some(rust_report) = rust_report {
        fs::write(dir.join(".out/conform-rust.json"), rust_report)
            .expect("write conform-rust.json");
    }
    dir
}

/// `sh taskfiles/conform-totals.sh` of this checkout in `dir`, as `task conform-totals` runs
/// it: its exit status and everything it printed.
fn totals(dir: &Path) -> (Option<i32>, String) {
    let output = Command::new("sh")
        .arg(repository().join("taskfiles/conform-totals.sh"))
        .current_dir(dir)
        .output()
        .expect("run sh");
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.code(), text)
}

fn has_line(text: &str, line: &str) -> bool {
    text.lines().any(|l| l == line)
}

#[test]
fn equal_totals_of_one_suite_pass() {
    let dir = copy("equal", Some(interpreter(51, SUITE)), Some(rust(51, SUITE)));
    let (status, text) = totals(&dir);
    assert_eq!(status, Some(0), "{text}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_rust_run_of_fewer_scenarios_fails_printing_both_totals() {
    let dir = copy("fewer", Some(interpreter(51, SUITE)), Some(rust(50, SUITE)));
    let (status, text) = totals(&dir);
    assert_ne!(status, Some(0), "{text}");
    for line in [
        format!("interpreter: 51 scenario(s) of suite {SUITE} (.out/report.json)"),
        format!("rust: 50 scenario(s) of suite {SUITE} (.out/conform-rust.json)"),
    ] {
        assert!(has_line(&text, &line), "no line {line:?} in\n{text}");
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn reports_of_two_suites_fail_even_with_equal_totals() {
    let dir = copy(
        "other-suite",
        Some(interpreter(51, OTHER)),
        Some(rust(51, SUITE)),
    );
    let (status, text) = totals(&dir);
    assert_ne!(status, Some(0), "{text}");
    let line = format!("interpreter: 51 scenario(s) of suite {OTHER} (.out/report.json)");
    assert!(has_line(&text, &line), "no line {line:?} in\n{text}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_report_fails_naming_it() {
    let dir = copy("missing", Some(interpreter(51, SUITE)), None);
    let (status, text) = totals(&dir);
    assert_ne!(status, Some(0), "{text}");
    assert!(text.contains(".out/conform-rust.json"), "{text}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_report_without_a_total_fails() {
    let dir = copy(
        "no-total",
        Some(interpreter(51, SUITE)),
        Some(format!("{{\"suite\":{{\"digest\":\"{SUITE}\"}}}}")),
    );
    let (status, text) = totals(&dir);
    assert_ne!(status, Some(0), "{text}");
    let _ = fs::remove_dir_all(&dir);
}
