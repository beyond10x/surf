//! `task generated-check` (Taskfile.yml) against a file under `generated/rust` whose executable
//! bit is not the one ess writes: the check fails and names the path, for a generated artifact and
//! for ess's record `.ess-output/state.json` alike. Each case copies `Taskfile.yml`, `spec/` and
//! `generated/rust` (without `target/` and `Cargo.lock`) into a directory of its own under
//! `CARGO_TARGET_TMPDIR`, makes one file executable, and runs the comparison of `task
//! generated-check` there (`task generated-compare`) against the one generation `task
//! generated-fresh` wrote (`common`). Needs `task` and `ess` 0.96.0 on `PATH`, as `task check`
//! does.

use crate::common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use common::check;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn copy_tree(from: &Path, to: &Path, skip_top: &[&str]) {
    fs::create_dir_all(to).expect("create directory");
    for entry in fs::read_dir(from).expect("read directory") {
        let entry = entry.expect("entry");
        let name = entry.file_name();
        if skip_top.iter().any(|s| name == *s) {
            continue;
        }
        let kind = entry.file_type().expect("file type");
        let target = to.join(&name);
        if kind.is_dir() {
            copy_tree(&entry.path(), &target, &[]);
        } else if kind.is_file() {
            fs::copy(entry.path(), &target).expect("copy file");
        } else {
            panic!("unexpected entry {:?}", entry.path());
        }
    }
}

fn copy(name: &str) -> PathBuf {
    let root = repository();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("generated-check-modes")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create copy");
    fs::copy(root.join("Taskfile.yml"), dir.join("Taskfile.yml")).expect("copy Taskfile.yml");
    copy_tree(&root.join("spec"), &dir.join("spec"), &[]);
    copy_tree(
        &root.join("generated/rust"),
        &dir.join("generated/rust"),
        &["target", "Cargo.lock"],
    );
    dir
}

/// Makes `path` (relative to `generated/rust`) executable in a fresh copy, runs the check and
/// asserts it fails naming the path.
fn assert_executable_fails_naming_it(case: &str, path: &str) {
    let dir = copy(case);
    let file = dir.join("generated/rust").join(path);
    let mut permissions = fs::metadata(&file).expect("metadata").permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    fs::set_permissions(&file, permissions).expect("chmod");
    let (status, text) = check(&dir);
    assert_ne!(status, Some(0), "{case}: {text}");
    let line = format!("generated/rust/{path}: executable, and ess writes it not executable");
    assert!(
        text.lines().any(|l| l == line),
        "{case}: no line {line:?} in\n{text}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A generated source file: the bytes are what ess writes, the mode is not.
#[test]
fn a_generated_file_made_executable_fails_naming_it() {
    assert_executable_fails_naming_it("artifact", "crates/surf-types/src/lib.rs");
}

/// ess's record is compared by its bytes after regeneration in place, which does not read the
/// mode either.
#[test]
fn the_record_made_executable_fails_naming_it() {
    assert_executable_fails_naming_it("record", ".ess-output/state.json");
}
