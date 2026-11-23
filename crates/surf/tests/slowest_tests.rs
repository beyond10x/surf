//! `taskfiles/slowest-tests.sh`, which task test and task test-tls run on their JUnit reports: a
//! report whose list is larger than a pipe's buffer prints its ten slowest tests and exits 0.
//! Before, the task's
//! pipeline ended `sort -rn | head -n 10`, and under pipefail `sort` died of SIGPIPE once `head`
//! had its ten lines: the task failed with 141 although every test passed (2026-11-22, 892
//! tests, a list of 70,331 bytes). The script sets pipefail where its shell has it. Each case
//! writes into a directory of its own under `CARGO_TARGET_TMPDIR`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("slowest-tests")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create scratch directory");
    dir
}

/// A JUnit report as cargo-nextest writes it, with `count` test cases; case `i` takes
/// `i % 97` seconds and `i % 1000` milliseconds.
fn report(count: usize) -> String {
    let mut text = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuites>\n");
    text.push_str("<testsuite name=\"planted::main\">\n");
    for i in 1..=count {
        text.push_str(&format!(
            "<testcase name=\"module_{i:04}::a_planted_test_whose_name_fills_the_pipe_{i:04}\" \
             classname=\"planted-package::main\" timestamp=\"2026-11-22T00:00:00Z\" \
             time=\"{}.{:03}\">\n</testcase>\n",
            i % 97,
            i % 1000
        ));
    }
    text.push_str("</testsuite>\n</testsuites>\n");
    text
}

fn run(args: &[&Path]) -> (Option<i32>, String, String) {
    let output = Command::new("sh")
        .arg(repository().join("taskfiles/slowest-tests.sh"))
        .args(args)
        .output()
        .expect("run sh");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn a_list_larger_than_a_pipe_prints_its_ten_slowest_and_exits_0() {
    let dir = scratch("large");
    let junit = dir.join("junit.xml");
    fs::write(&junit, report(2_000)).expect("write report");
    // The list the script sorts: one line per case, "<time> <classname> <name>".
    let list: usize = (1..=2_000usize)
        .map(|i| {
            format!(
                "{}.{:03} planted-package::main module_{i:04}::a_planted_test_whose_name_fills_the_pipe_{i:04}\n",
                i % 97,
                i % 1000
            )
            .len()
        })
        .sum();
    assert!(list > 128 * 1024, "the list is {list} bytes");

    let (status, out, err) = run(&[&junit]);
    assert_eq!(status, Some(0), "{err}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 10, "{out}");
    // 96.969 s is case 969 (969 % 97 = 96, 969 % 1000 = 969); 96.939 s is case 1,939.
    assert_eq!(
        lines[0],
        "    96.969s planted-package::main module_0969::a_planted_test_whose_name_fills_the_pipe_0969"
    );
    assert_eq!(
        lines[1],
        "    96.939s planted-package::main module_1939::a_planted_test_whose_name_fills_the_pipe_1939"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_report_exits_1_with_one_line() {
    let dir = scratch("missing");
    let (status, out, err) = run(&[&dir.join("junit.xml")]);
    assert_eq!(status, Some(1));
    assert_eq!(out, "");
    assert_eq!(err.lines().count(), 1, "{err}");
    let _ = fs::remove_dir_all(&dir);
}

/// No `sort` is followed by a reader that ends before its input, in the script or in the task
/// that calls it.
#[test]
fn no_sort_feeds_a_head() {
    for file in ["taskfiles/slowest-tests.sh", "Taskfile.yml"] {
        let text = fs::read_to_string(repository().join(file)).expect("read file");
        let joined = text.replace("|\n", "| ");
        for line in joined.lines() {
            if let Some(at) = line.find("sort") {
                let rest = &line[at..];
                assert!(
                    !rest.contains("| head") && !rest.contains("|head"),
                    "{file}: {line}"
                );
            }
        }
    }
    let task = fs::read_to_string(repository().join("Taskfile.yml")).expect("read Taskfile.yml");
    // task test and task test-tls each print their ten slowest tests, each from its own copy.
    assert_eq!(
        task.matches("sh taskfiles/slowest-tests.sh \"$junit\" 10").count(),
        2,
        "task test and task test-tls no longer each run taskfiles/slowest-tests.sh"
    );
    for copy in ["junit=.out/test-junit.xml", "junit=.out/test-tls-junit.xml"] {
        assert!(task.contains(copy), "Taskfile.yml no longer reads {copy}");
    }
}
