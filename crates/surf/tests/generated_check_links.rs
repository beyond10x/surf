//! `task generated-check` (Taskfile.yml) against entries under `generated/rust` that are neither a
//! regular file nor a directory: each fails the check and is named `generated/rust/<path>: not a
//! regular file`, wherever it is. Each case copies `Taskfile.yml`, `spec/` and `generated/rust`
//! (without `target/` and `Cargo.lock`) into a directory of its own under `CARGO_TARGET_TMPDIR`,
//! adds one symbolic link, and runs the comparison of `task generated-check` there (`task
//! generated-compare`) against the one generation `task generated-fresh` wrote (`common`). Needs
//! `task` and `ess` 0.96.0 on `PATH`, as `task check` does.

use crate::common;

use std::fs;
use std::os::unix::fs::symlink;
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

/// A fresh copy named `name`, with a directory `elsewhere/` beside `generated/` that holds a
/// crate, a file and a directory for the links to point at.
fn copy(name: &str) -> PathBuf {
    let root = repository();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("generated-check-links")
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
    let crate_dir = dir.join("elsewhere/extra");
    fs::create_dir_all(crate_dir.join("src")).expect("create elsewhere/extra");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"extra\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write Cargo.toml");
    fs::write(crate_dir.join("src/lib.rs"), "").expect("write lib.rs");
    fs::write(dir.join("elsewhere/file"), "x\n").expect("write file");
    dir
}

/// Link `path` (relative to `generated/rust`) to `to` in a fresh copy, run the check, and assert
/// it fails naming the link.
fn assert_link_fails(case: &str, path: &str, to: &str) {
    let dir = copy(case);
    let link = dir.join("generated/rust").join(path);
    symlink(to, &link).expect("symlink");
    assert!(
        fs::metadata(&link).is_ok(),
        "{case}: the link {path} -> {to} does not resolve, so it tests nothing"
    );
    let (status, text) = check(&dir);
    assert_ne!(status, Some(0), "{case}: {text}");
    let line = format!("generated/rust/{path}: not a regular file");
    assert!(
        text.lines().any(|l| l == line),
        "{case}: no line {line:?} in\n{text}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A linked directory under `generated/rust/crates/` matches the root workspace's member glob
/// `generated/rust/crates/*`, so cargo would build a crate that ess never wrote.
#[test]
fn a_symlinked_directory_under_generated_crates_fails_naming_it() {
    assert_link_fails(
        "crate-directory",
        "crates/extra",
        "../../../elsewhere/extra",
    );
}

/// `target` is left out of the comparison because cargo writes it, but a link of that name is
/// not something cargo writes.
#[test]
fn a_symlink_named_target_fails_naming_it() {
    assert_link_fails("target", "target", "../../elsewhere/extra");
}

/// The same for `Cargo.lock`.
#[test]
fn a_symlink_named_cargo_lock_fails_naming_it() {
    assert_link_fails("cargo-lock", "Cargo.lock", "../../elsewhere/file");
}

/// `.ess-output` is ess's record and is left out of the comparison; a link inside it is not.
#[test]
fn a_symlink_inside_ess_output_fails_naming_it() {
    assert_link_fails("ess-output", ".ess-output/extra", "../../../elsewhere/file");
}

/// A generated file replaced by a link to a copy of itself: same bytes, still not a regular file.
#[test]
fn a_generated_file_replaced_by_a_symlink_to_its_own_bytes_fails_naming_it() {
    let dir = copy("same-bytes");
    let path = "crates/net/Cargo.toml";
    let file = dir.join("generated/rust").join(path);
    fs::copy(&file, dir.join("elsewhere/Cargo.toml")).expect("copy Cargo.toml");
    fs::remove_file(&file).expect("remove Cargo.toml");
    symlink("../../../../elsewhere/Cargo.toml", &file).expect("symlink");
    let (status, text) = check(&dir);
    assert_ne!(status, Some(0), "{text}");
    let line = format!("generated/rust/{path}: not a regular file");
    assert!(
        text.lines().any(|l| l == line),
        "no line {line:?} in\n{text}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// `generated/rust` itself a link to a byte-identical tree elsewhere.
#[test]
fn generated_rust_as_a_symlink_fails_naming_it() {
    let dir = copy("generated-rust");
    fs::rename(dir.join("generated/rust"), dir.join("elsewhere/rust")).expect("move");
    symlink("../elsewhere/rust", dir.join("generated/rust")).expect("symlink");
    let (status, text) = check(&dir);
    assert_ne!(status, Some(0), "{text}");
    let line = "generated/rust: a symbolic link, not a directory";
    assert!(
        text.lines().any(|l| l == line),
        "no line {line:?} in\n{text}"
    );
    let _ = fs::remove_dir_all(&dir);
}
