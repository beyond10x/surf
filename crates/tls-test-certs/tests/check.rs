//! `taskfiles/tls-certs-check.sh` on a copy of the crate (story `tls-test-certs`, acceptance 6):
//! the copy as it is matches its `DIGEST`; one byte of any source changed fails it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const FAILED: &str =
    "tls-certs: the sources changed since certs/ was generated; run task tls-certs-slow";

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn script() -> PathBuf {
    crate_dir().join("../../taskfiles/tls-certs-check.sh")
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// A fresh copy of the sources and `certs/` under `CARGO_TARGET_TMPDIR`.
fn copy_crate() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "tls-certs-check-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    for part in ["src", "keys", "openssl", "certs"] {
        copy_tree(&crate_dir().join(part), &dir.join(part));
    }
    fs::copy(crate_dir().join("Cargo.toml"), dir.join("Cargo.toml")).unwrap();
    dir
}

/// Flips the lowest bit of the middle byte of `path`.
fn change_one_byte(path: &Path) {
    let mut bytes = fs::read(path).unwrap();
    let middle = bytes.len() / 2;
    bytes[middle] ^= 1;
    fs::write(path, bytes).unwrap();
}

/// The exit status and the standard output of the script on `dir`.
fn run(dir: &Path) -> (Option<i32>, String) {
    let out = Command::new("sh").arg(script()).arg(dir).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    (out.status.code(), stdout)
}

#[test]
fn unchanged_copy_matches() {
    let dir = copy_crate();
    let (code, stdout) = run(&dir);
    let cases = fs::read_to_string(dir.join("certs/INDEX"))
        .unwrap()
        .lines()
        .count();
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout.lines().last(),
        Some(format!("tls-certs: {cases} cases, certs/ matches its sources").as_str())
    );
    fs::remove_dir_all(dir).unwrap();
}

fn fails_after_changing(source: &str) {
    let dir = copy_crate();
    change_one_byte(&dir.join(source));
    let (code, stdout) = run(&dir);
    assert_eq!(code, Some(1), "{source}: {stdout}");
    assert!(stdout.lines().any(|l| l == FAILED), "{source}: {stdout}");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn one_byte_of_cases_rs_fails() {
    fails_after_changing("src/cases.rs");
}

#[test]
fn one_byte_of_make_sh_fails() {
    fails_after_changing("openssl/make.sh");
}

#[test]
fn one_byte_of_a_key_or_the_manifest_fails() {
    fails_after_changing("keys/p256-00.pk8");
    fails_after_changing("Cargo.toml");
}
