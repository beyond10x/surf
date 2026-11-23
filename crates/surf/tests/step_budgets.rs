//! `task step-budgets` (Taskfile.yml), the last step of `task check`: the sum of the four timed
//! steps. It reads the seconds `task test`, `task test-tls`, `task conform-rust` and
//! `task view-test` write into `.out/test-step.seconds`, `.out/tls-test-step.seconds`,
//! `.out/conform-step.seconds` and `.out/page-step.seconds`, prints `four steps: N s (test step A
//! s, tls test step B s, conformance step C s, page step D s)`, and fails naming the file when
//! one is missing or holds no number. It has no budget of its own. Each case writes the files
//! into a directory of its own under `CARGO_TARGET_TMPDIR` and runs the task's script,
//! `sh taskfiles/step-budgets.sh <directory>`, without `task`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const FILES: [&str; 4] = [
    "test-step.seconds",
    "tls-test-step.seconds",
    "conform-step.seconds",
    "page-step.seconds",
];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A directory named `name` holding the four files with `values`; `None` leaves that file out.
fn out(name: &str, values: [Option<&str>; 4]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("step-budgets")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create scratch directory");
    for (file, value) in FILES.iter().zip(values) {
        if let Some(value) = value {
            fs::write(dir.join(file), format!("{value}\n")).expect("write seconds");
        }
    }
    dir
}

fn run(dir: &Path) -> (Option<i32>, String, String) {
    let output = Command::new("sh")
        .arg(repository().join("taskfiles/step-budgets.sh"))
        .arg(dir)
        .output()
        .expect("run sh");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn the_four_steps_are_summed_in_one_line() {
    let dir = out(
        "four",
        [Some("45.2"), Some("20.4"), Some("17.4"), Some("68.9")],
    );
    let (status, out, err) = run(&dir);
    assert_eq!(status, Some(0), "{out}{err}");
    assert_eq!(
        out,
        "four steps: 151.9 s (test step 45.2 s, tls test step 20.4 s, conformance step 17.4 s, \
         page step 68.9 s)\n"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A sum over four times 120 s still passes: the step has no budget of its own.
#[test]
fn a_sum_over_any_budget_passes() {
    let dir = out(
        "over",
        [Some("119.9"), Some("119.9"), Some("119.9"), Some("119.9")],
    );
    let (status, out, err) = run(&dir);
    assert_eq!(status, Some(0), "{out}{err}");
    assert!(out.starts_with("four steps: 479.6 s ("), "{out}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn each_missing_file_fails_naming_it() {
    for missing in 0..FILES.len() {
        let mut values = [Some("1.0"); 4];
        values[missing] = None;
        let dir = out(&format!("missing-{missing}"), values);
        let (status, out, err) = run(&dir);
        assert_eq!(status, Some(1), "{}: {out}{err}", FILES[missing]);
        assert!(
            out.contains(&format!(
                "{}: no such file",
                dir.join(FILES[missing]).display()
            )),
            "{}: {out}{err}",
            FILES[missing]
        );
        assert!(!out.contains("four steps:"), "{out}");
        let _ = fs::remove_dir_all(&dir);
    }
}

#[test]
fn a_file_without_a_number_fails_naming_it() {
    let dir = out(
        "not-a-number",
        [Some("45.2"), Some("20.4"), Some("over"), Some("68.9")],
    );
    let (status, out, err) = run(&dir);
    assert_eq!(status, Some(1), "{out}{err}");
    assert!(
        out.contains(&format!(
            "{}: not a number of seconds",
            dir.join("conform-step.seconds").display()
        )),
        "{out}{err}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// `task check` runs `step-budgets` as its last step.
#[test]
fn check_ends_with_step_budgets() {
    let task = fs::read_to_string(repository().join("Taskfile.yml")).expect("read Taskfile.yml");
    let check: Vec<&str> = task
        .lines()
        .skip_while(|line| *line != "  check:")
        .skip(1)
        .take_while(|line| !(line.starts_with("  ") && !line.starts_with("   ")))
        .filter(|line| line.trim_start().starts_with("- task: "))
        .collect();
    assert_eq!(
        check.last().map(|line| line.trim()),
        Some("- task: step-budgets"),
        "{check:?}"
    );
}
