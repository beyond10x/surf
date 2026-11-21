//! The one generation of `spec/` the tests of `task generated-check` compare against.
//!
//! `task generated-fresh` writes it into `.out/generated-check/fresh` of the checkout, once per
//! run of task: `task check` runs it for `generated-check`, and `task test` depends on it. The
//! tests find it there and run `task generated-compare FRESH=<it>` in their copies, so no test
//! and no case generates the workspace again. nextest starts one process per test, so a
//! `OnceLock` alone would still generate once per test.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every regular file under `dir`, relative to it, with its bytes, in path order.
fn tree(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(dir: &Path, prefix: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in fs::read_dir(dir).expect("read directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                walk(&path, prefix, out);
            } else {
                let bytes = fs::read(&path).expect("read file");
                out.push((
                    path.strip_prefix(prefix).expect("prefix").to_path_buf(),
                    bytes,
                ));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// The fresh tree, after checking that it was generated from the `spec/` of this checkout.
pub fn fresh() -> &'static Path {
    static FRESH: OnceLock<PathBuf> = OnceLock::new();
    FRESH.get_or_init(|| {
        let root = repository();
        let dir = root.join(".out/generated-check");
        let fresh = dir.join("fresh");
        assert!(
            fresh.is_dir() && dir.join("fresh.log").is_file() && dir.join("fresh.spec").is_dir(),
            "{}: no generation; run `task generated-fresh` first (`task test` does)",
            fresh.display()
        );
        assert!(
            tree(&dir.join("fresh.spec")) == tree(&root.join("spec")),
            "{}: generated from another spec/; run `task generated-fresh` again",
            fresh.display()
        );
        fresh.canonicalize().expect("canonical path")
    })
}

/// `task generated-compare` in `dir` against the fresh tree: its exit status and everything it
/// printed.
pub fn check(dir: &Path) -> (Option<i32>, String) {
    let output = Command::new("task")
        .arg("generated-compare")
        .arg(format!("FRESH={}", fresh().display()))
        .current_dir(dir)
        .output()
        .expect("run task");
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.code(), text)
}
