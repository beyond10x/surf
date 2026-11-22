//! Review 1 (adversary): `task generated-check` (Taskfile.yml) against its own summary and the
//! acceptance of story `generated-rust-workspace`. Each case copies `Taskfile.yml`, `spec/` and
//! `generated/rust` (without `target/` and `Cargo.lock`) into a directory of its own under
//! `CARGO_TARGET_TMPDIR`, changes the copy, and runs the comparison of `task generated-check`
//! there (`task generated-compare`) against the one generation `task generated-fresh` wrote
//! (`common`). Needs `task` and `ess` 0.96.0 on `PATH`, as `task check` does.

mod common;

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

/// A fresh copy named `name`, ready for `task generated-check`.
fn copy(name: &str) -> PathBuf {
    let root = repository();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("review1-generated-check").join(name);
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

fn files(dir: &Path, prefix: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read directory") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if entry.file_type().expect("file type").is_dir() {
            files(&path, prefix, out);
        } else {
            out.push(path.strip_prefix(prefix).expect("prefix").to_path_buf());
        }
    }
}

#[test]
fn review1_unchanged_copy_passes() {
    let dir = copy("unchanged");
    let (status, text) = check(&dir);
    assert_eq!(status, Some(0), "{text}");
}

/// Changes the byte in the middle of `file` and returns the bytes it held before.
fn change_one_byte(file: &Path) -> Vec<u8> {
    let original = fs::read(file).expect("read");
    let mut bytes = original.clone();
    let at = bytes.len() / 2;
    bytes[at] ^= 0x01;
    fs::write(file, &bytes).expect("write");
    original
}

/// Story `generated-rust-workspace`, Acceptance: "exits non-zero naming the file after one byte
/// of any file under `generated/rust` is changed", for every committed file.
///
/// The comparison of the files ess writes (step 1 of `task generated-compare`) checks every file
/// and lists each one that differs before it fails, so one run with one byte changed in each of
/// them must name all of them. The record under `.ess-output` is checked by step 2, which runs
/// only when step 1 passes, so each of its files is changed alone. One file of step 1 changed
/// alone shows that a single change fails too. Three runs of the check in one copy, whatever the
/// number of files.
#[test]
fn review1_one_changed_byte_in_any_generated_file_fails_naming_it() {
    let root = repository().join("generated/rust");
    let mut all = Vec::new();
    files(&root, &root, &mut all);
    all.retain(|p| !p.starts_with("target") && p != Path::new("Cargo.lock"));
    all.sort();
    assert!(all.len() >= 28, "{all:?}");
    let (record, written): (Vec<_>, Vec<_>) =
        all.iter().partition(|p| p.starts_with(".ess-output"));
    assert!(!record.is_empty() && !written.is_empty(), "{all:?}");

    let dir = copy("byte");
    let generated = dir.join("generated/rust");
    let mut missed = Vec::new();

    let originals: Vec<_> = written
        .iter()
        .map(|path| change_one_byte(&generated.join(path)))
        .collect();
    let (status, text) = check(&dir);
    if status == Some(0) {
        missed.push(format!(
            "every written file changed: exit {status:?}\n{text}"
        ));
    }
    for path in &written {
        let named = format!("generated/rust/{}:", path.display());
        if !text.contains(&named) {
            missed.push(format!(
                "{named} not named among every written file changed\n{text}"
            ));
        }
    }
    for (path, original) in written.iter().zip(&originals) {
        fs::write(generated.join(path), original).expect("put the byte back");
    }

    for path in record.iter().chain(written.first()) {
        let file = generated.join(path);
        let original = change_one_byte(&file);
        let (status, text) = check(&dir);
        let named = format!("generated/rust/{}", path.display());
        if status == Some(0) || !text.contains(&named) {
            missed.push(format!("{named} changed alone: exit {status:?}\n{text}"));
        }
        fs::write(&file, &original).expect("put the byte back");
    }

    let _ = fs::remove_dir_all(&dir);
    assert!(missed.is_empty(), "{}", missed.join("\n"));
}

/// A regular `build.rs` beside a generated crate's manifest is reported: "any other file in
/// generated/rust" fails.
#[test]
fn review1_a_hand_written_build_script_in_a_generated_crate_fails_naming_it() {
    let dir = copy("build-script-file");
    fs::write(
        dir.join("generated/rust/crates/net/build.rs"),
        "fn main() { println!(\"cargo::warning=hand-written\"); }\n",
    )
    .expect("write build.rs");
    let (status, text) = check(&dir);
    assert_ne!(status, Some(0), "{text}");
    assert!(text.contains("generated/rust/crates/net/build.rs"), "{text}");
}

/// The same build script committed as a symbolic link. git stores a link; cargo follows it and
/// runs the script for the generated crate `net` in the root workspace's `cargo build`.
#[test]
fn review1_a_symlinked_build_script_in_a_generated_crate_fails_naming_it() {
    let dir = copy("build-script-symlink");
    fs::create_dir_all(dir.join("elsewhere")).expect("create elsewhere");
    fs::write(
        dir.join("elsewhere/build.rs"),
        "fn main() { println!(\"cargo::warning=hand-written\"); }\n",
    )
    .expect("write build.rs");
    symlink(
        "../../../../elsewhere/build.rs",
        dir.join("generated/rust/crates/net/build.rs"),
    )
    .expect("symlink");
    let (status, text) = check(&dir);
    assert_ne!(status, Some(0), "{text}");
    assert!(text.contains("generated/rust/crates/net/build.rs"), "{text}");
}
