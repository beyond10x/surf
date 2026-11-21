//! `task lock-check` (Taskfile.yml): nothing that ships uses a third-party crate, so the root
//! `Cargo.lock` lists no package with a `source` line, and the check fails naming each one that
//! has one. Each case copies `Taskfile.yml` and `Cargo.lock` into a directory of its own under
//! `CARGO_TARGET_TMPDIR`, changes the lock or not, and runs `task lock-check` there. Needs `task`
//! on `PATH`, as `task check` does.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fresh copy named `name` of `Taskfile.yml` and the root `Cargo.lock`, with `extra` appended
/// to the lock.
fn copy(name: &str, extra: &str) -> PathBuf {
    let root = repository();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("lock-check")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create copy");
    fs::copy(root.join("Taskfile.yml"), dir.join("Taskfile.yml")).expect("copy Taskfile.yml");
    let mut lock = fs::read_to_string(root.join("Cargo.lock")).expect("read Cargo.lock");
    lock.push_str(extra);
    fs::write(dir.join("Cargo.lock"), lock).expect("write Cargo.lock");
    dir
}

/// `task lock-check` in `dir`: its exit status and everything it printed.
fn check(dir: &Path) -> (Option<i32>, String) {
    let output = Command::new("task")
        .arg("lock-check")
        .current_dir(dir)
        .output()
        .expect("run task");
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.code(), text)
}

/// Appends `package` to a copy of the lock, runs the check and asserts it fails with `line`.
fn assert_fails_naming(case: &str, package: &str, line: &str) {
    let dir = copy(case, package);
    let (status, text) = check(&dir);
    assert_ne!(status, Some(0), "{case}: {text}");
    assert!(
        text.lines().any(|l| l == line),
        "{case}: no line {line:?} in\n{text}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The lock as committed lists only crates of this repository.
#[test]
fn the_unchanged_lock_passes() {
    let dir = copy("unchanged", "");
    let (status, text) = check(&dir);
    assert_eq!(status, Some(0), "{text}");
    let _ = fs::remove_dir_all(&dir);
}

/// A crate from crates.io, as `cargo generate-lockfile` writes it for `cfg-if = "1"`.
#[test]
fn a_registry_package_fails_naming_it() {
    assert_fails_naming(
        "registry",
        concat!(
            "\n[[package]]\n",
            "name = \"cfg-if\"\n",
            "version = \"1.0.5\"\n",
            "source = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
            "checksum = \"0000000000000000000000000000000000000000000000000000000000000000\"\n",
        ),
        "Cargo.lock: cfg-if 1.0.5 from registry+https://github.com/rust-lang/crates.io-index",
    );
}

/// A crate from a git repository is from outside this one too.
#[test]
fn a_git_package_fails_naming_it() {
    assert_fails_naming(
        "git",
        concat!(
            "\n[[package]]\n",
            "name = \"ess-conformance\"\n",
            "version = \"0.96.0\"\n",
            "source = \"git+https://github.com/beyond10x/ess?tag=0.96.0#9e91e8b5\"\n",
        ),
        "Cargo.lock: ess-conformance 0.96.0 from git+https://github.com/beyond10x/ess?tag=0.96.0#9e91e8b5",
    );
}

/// No root lock at all proves nothing, so it fails too.
#[test]
fn a_missing_lock_fails() {
    let dir = copy("missing", "");
    fs::remove_file(dir.join("Cargo.lock")).expect("remove Cargo.lock");
    let (status, text) = check(&dir);
    assert_ne!(status, Some(0), "{text}");
    let _ = fs::remove_dir_all(&dir);
}
