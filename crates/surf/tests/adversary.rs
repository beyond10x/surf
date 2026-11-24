//! Review cases of `task lock-check` for `story:scripts-engine-dependency`: locks, lists and
//! manifests of the test's own, each in a directory of its own under `CARGO_TARGET_TMPDIR`, run
//! through the task's line as `lock_check` runs it. The first three cases show what the check
//! lets through; the cases marked "Held" record what it refused.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SWELL_PACKAGES: [&str; 6] = [
    "swell",
    "swell-compiler",
    "swell-num",
    "swell-regexp",
    "swell-syntax",
    "swell-unicode",
];

const LINE: &str = "sh taskfiles/lock-check.sh Cargo.lock taskfiles/lock-allow.txt taskfiles/lock-owners.txt Cargo.toml";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    fs::read_to_string(repository().join(path)).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

/// The files of one copy; `None` leaves the file out.
#[derive(Clone)]
struct Files {
    lock: Option<String>,
    allow: Option<String>,
    owners: Option<String>,
    manifest: Option<String>,
    cargo_config: Option<String>,
}

impl Files {
    fn committed() -> Self {
        Self {
            lock: Some(read("Cargo.lock")),
            allow: Some(read("taskfiles/lock-allow.txt")),
            owners: Some(read("taskfiles/lock-owners.txt")),
            manifest: Some(read("Cargo.toml")),
            cargo_config: Some(read(".cargo/config.toml")),
        }
    }

    fn lock(&self) -> &str {
        self.lock.as_deref().expect("a lock")
    }
}

fn write(dir: &Path, path: &str, text: &Option<String>) {
    if let Some(text) = text {
        let file = dir.join(path);
        fs::create_dir_all(file.parent().expect("a parent")).expect("create dir");
        fs::write(&file, text).unwrap_or_else(|e| panic!("write {path}: {e}"));
    }
}

/// Runs the task's line in a fresh copy named `name` holding `files`: exit status and output.
fn check(name: &str, files: &Files) -> (Option<i32>, String) {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("lock-check-adversary")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("taskfiles")).expect("create copy");
    fs::copy(
        repository().join("taskfiles/lock-check.sh"),
        dir.join("taskfiles/lock-check.sh"),
    )
    .expect("copy lock-check.sh");
    write(&dir, "Cargo.lock", &files.lock);
    write(&dir, "taskfiles/lock-allow.txt", &files.allow);
    write(&dir, "taskfiles/lock-owners.txt", &files.owners);
    write(&dir, "Cargo.toml", &files.manifest);
    write(&dir, ".cargo/config.toml", &files.cargo_config);
    let mut words = LINE.split(' ');
    let output = Command::new(words.next().expect("a program"))
        .args(words)
        .current_dir(&dir)
        .output()
        .expect("run sh");
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    let _ = fs::remove_dir_all(&dir);
    (output.status.code(), text)
}

/// Asserts the check fails on `files` with a line for which `wanted` holds.
fn assert_fails(name: &str, files: &Files, what: &str, wanted: impl Fn(&str) -> bool) {
    let (status, text) = check(name, files);
    assert_ne!(status, Some(0), "{name}: the check passed:\n{text}");
    assert!(
        text.lines().any(wanted),
        "{name}: no line {what} in\n{text}"
    );
}

/// `lock` with the block of the package `name` (the first of that name) passed through `edit`.
fn edit_package(lock: &str, name: &str, edit: impl Fn(&str) -> String) -> String {
    let head = format!("[[package]]\nname = \"{name}\"\n");
    let start = lock
        .find(&head)
        .unwrap_or_else(|| panic!("no package {name}"));
    let end = lock[start + head.len()..]
        .find("\n[[package]]")
        .map_or(lock.len(), |i| start + head.len() + i + 1);
    let block = &lock[start..end];
    let edited = edit(block);
    assert_ne!(block, edited, "the edit of {name} changed nothing");
    format!("{}{}{}", &lock[..start], edited, &lock[end..])
}

/// The section of the root manifest that turns overflow checks on for `name` in release.
fn release_section(name: &str) -> String {
    format!("[profile.release.package.{name}]\noverflow-checks = true\n")
}

/// A path package (no `source` line) that bears the name of a listed package from outside, here
/// `swell` 0.8.1 beside the git `swell` 0.8.0, used by `surf-script`: cargo writes it so for a
/// dependency `swell2 = { package = "swell", path = "<a directory outside the tree>" }`. The
/// check reads a package without `source` as a crate of this repository, and `surf-script` may
/// depend on a package named `swell`, so the copy passes.
#[test]
fn a_path_package_named_like_a_listed_one_fails() {
    let mut files = Files::committed();
    let lock = edit_package(files.lock(), "surf-script", |b| {
        b.replace(" \"swell\",\n", " \"swell 0.8.0\",\n \"swell 0.8.1\",\n")
    });
    let shadow = "[[package]]\nname = \"swell\"\nversion = \"0.8.1\"\n\n";
    let at = lock
        .find("[[package]]\nname = \"swell-compiler\"\n")
        .expect("swell-compiler in the lock");
    files.lock = Some(format!("{}{shadow}{}", &lock[..at], &lock[at..]));
    assert_fails("path-shadow", &files, "naming swell 0.8.1", |l| {
        l.contains("swell 0.8.1")
    });
}

/// The repository's `.cargo/config.toml` turning overflow checks off for a swell package in the
/// release profile. cargo reads `[profile]` from config files too, and a config file's profile
/// overrides the manifest's (The Cargo Book, "Configuration", `[profile]`: "can be used to
/// globally change profile settings, and override settings specified in Cargo.toml"); the file
/// already holds `[profile.dev.package.ring]`. The check reads the root manifest only and
/// passes, so a release build compiles swell-num without overflow checks.
#[test]
fn a_cargo_config_turning_overflow_checks_off_fails() {
    let mut files = Files::committed();
    let mut config = files.cargo_config.take().expect("a config");
    config.push_str("\n[profile.release.package.swell-num]\noverflow-checks = false\n");
    files.cargo_config = Some(config);
    assert_fails("config-overflow-off", &files, "naming swell-num", |l| {
        l.contains("swell-num")
    });
}

/// The six sections removed from the root manifest and written inside a multi-line string of
/// `[workspace.metadata]`, which cargo does not read. TOML reads no table there, so the release
/// profile checks no overflow in swell; the check reads the lines as tables and passes.
#[test]
fn profile_sections_inside_a_string_fail() {
    let mut files = Files::committed();
    let mut manifest = files.manifest.take().expect("a manifest");
    for name in SWELL_PACKAGES {
        let section = release_section(name);
        assert!(manifest.contains(&section), "no {section:?} in Cargo.toml");
        manifest = manifest.replace(&section, "");
    }
    manifest.push_str("\n[workspace.metadata]\nnote = \"\"\"\n");
    for name in SWELL_PACKAGES {
        manifest.push_str(&release_section(name));
    }
    manifest.push_str("\"\"\"\n");
    files.manifest = Some(manifest);
    assert_fails(
        "overflow-in-string",
        &files,
        "naming swell-num and overflow-checks",
        |l| l.contains("swell-num") && l.contains("overflow-checks"),
    );
}
