//! The one generation of `spec/` the tests of `task generated-check` compare against.
//!
//! `task generated-fresh` writes it into `.out/generated-check/fresh` of the checkout, with its
//! three listings `fresh.list`, `fresh.all` and `fresh.executable` beside it, once per run of
//! task: `task check` runs it for `generated-check`, and `task test` depends on it. The tests
//! find it there and run the script of `task generated-compare`,
//! `sh taskfiles/generated-compare.sh <it>`, in their copies, so no test and no case generates
//! or lists the tree again, and no case starts `task`. nextest starts one process per test, so a
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

/// The fresh tree, after checking that it was generated from the `spec/` of this checkout and
/// that its three listings lie beside it.
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
        let missing: Vec<String> = ["fresh.list", "fresh.all", "fresh.executable"]
            .iter()
            .map(|name| dir.join(name))
            .filter(|listing| !listing.is_file())
            .map(|listing| listing.display().to_string())
            .collect();
        assert!(
            missing.is_empty(),
            "{}: the listings of the generation are missing; run `task generated-fresh` (`task \
             test` does)",
            missing.join(", ")
        );
        assert!(
            tree(&dir.join("fresh.spec")) == tree(&root.join("spec")),
            "{}: generated from another spec/; run `task generated-fresh` again",
            fresh.display()
        );
        fresh.canonicalize().expect("canonical path")
    })
}

/// `sh taskfiles/generated-compare.sh` of this checkout in `dir` against the fresh tree, as
/// `task generated-compare` runs it: its exit status and everything it printed.
pub fn check(dir: &Path) -> (Option<i32>, String) {
    let output = Command::new("sh")
        .arg(repository().join("taskfiles/generated-compare.sh"))
        .arg(fresh())
        .current_dir(dir)
        .output()
        .expect("run sh");
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.code(), text)
}
