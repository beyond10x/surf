//! Review 2: `task generated-check` against changes to `generated/rust` that are not what
//! `task generate` writes and that the check of review 1 may still pass. Its summary: it fails
//! "when generated/rust is not what task generate writes", and `.ess-output/` "holds only
//! state.json". Each case copies `Taskfile.yml`, `spec/` and `generated/rust` into a directory of
//! its own under `CARGO_TARGET_TMPDIR`, makes one change, and runs the comparison of the check
//! there (`task generated-compare`) against the one generation `task generated-fresh` wrote
//! (`common`). Needs `task` and `ess` 0.96.0 on `PATH`, as `task check` does.

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
        .join("review2-generated-check")
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

/// A file in `.ess-output/` beside `state.json`, which ess does not write.
#[test]
fn review2_a_file_ess_does_not_write_inside_ess_output_fails_the_check() {
    let dir = copy("ess-output-extra");
    fs::write(
        dir.join("generated/rust/.ess-output/notes.md"),
        "written by hand\n",
    )
    .expect("write");
    let (status, text) = check(&dir);
    assert_ne!(
        status,
        Some(0),
        "generated/rust/.ess-output/notes.md, a file ess never writes, passed:\n{text}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A generated source file made executable: same bytes, another mode than ess writes.
#[test]
fn review2_a_generated_file_made_executable_fails_the_check() {
    let dir = copy("executable-bit");
    let path = dir.join("generated/rust/crates/surf-types/src/lib.rs");
    let mut permissions = fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    fs::set_permissions(&path, permissions).expect("chmod");
    let (status, text) = check(&dir);
    assert_ne!(
        status,
        Some(0),
        "crates/surf-types/src/lib.rs with mode {:o} passed:\n{text}",
        fs::metadata(&path).expect("metadata").permissions().mode()
    );
    let _ = fs::remove_dir_all(&dir);
}
