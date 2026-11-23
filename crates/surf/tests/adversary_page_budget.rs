//! Adversary cases against `story:page-test-step-within-budget`, decision D249: the setup runs
//! `npm ci` "only when `view-tests/package-lock.json` differs from the installed tree", and
//! `taskfiles/view-test-setup.sh` compares only the sha256 of the lock with the digest it stored
//! beside `node_modules`. Each case builds a root of its own under `CARGO_TARGET_TMPDIR` with the
//! repository's `view-tests/package.json` and `package-lock.json`, puts a stub `npm` first on
//! `PATH` that records its arguments and installs nothing, and runs the script with `sh` from that
//! root, as the task does. No case reaches a registry.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const PACKAGES: [&str; 3] = ["@playwright/test", "playwright", "playwright-core"];

/// A root named `name`: view-tests/ with the repository's package.json (or `package_json`) and
/// package-lock.json, node_modules holding the digest the last install stored (of the
/// repository's two files) and the packages of `installed`, and bin/npm, the stub.
fn root(name: &str, package_json: Option<&str>, installed: &[&str]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("adversary-page-budget")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    let modules = dir.join("view-tests/node_modules");
    fs::create_dir_all(&modules).expect("create node_modules");
    let lock = fs::read(repository().join("view-tests/package-lock.json")).expect("read lock");
    fs::write(dir.join("view-tests/package-lock.json"), &lock).expect("write lock");
    // The digest the last install stored: that of the repository's package.json and lock, as
    // the script itself computes it (D320: both files), before package_json replaces the one.
    let installed_package = fs::read_to_string(repository().join("view-tests/package.json"))
        .expect("read package.json");
    fs::write(dir.join("view-tests/package.json"), &installed_package).expect("write package.json");
    let digest = Command::new("sh")
        .arg(repository().join("taskfiles/view-test-setup.sh"))
        .arg("digest")
        .current_dir(&dir)
        .output()
        .expect("run the setup's digest");
    assert!(digest.status.success(), "{digest:?}");
    let digest = String::from_utf8_lossy(&digest.stdout);
    fs::write(modules.join(".surf-lock.sha256"), digest.as_bytes()).expect("write digest");
    if let Some(text) = package_json {
        fs::write(dir.join("view-tests/package.json"), text).expect("write package.json");
    }
    for package in installed {
        let at = modules.join(package);
        fs::create_dir_all(&at).expect("create package");
        fs::write(at.join("package.json"), "{}\n").expect("write package's package.json");
    }
    fs::create_dir_all(dir.join("bin")).expect("create bin");
    fs::write(
        dir.join("bin/npm"),
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$(dirname \"$0\")/npm-calls\"\nexit 0\n",
    )
    .expect("write stub npm");
    let made = Command::new("chmod")
        .arg("+x")
        .arg(dir.join("bin/npm"))
        .status()
        .expect("run chmod");
    assert!(made.success());
    dir
}

/// Runs the setup script from `dir`; its exit code, its output and the stub's calls.
fn setup(dir: &Path) -> (Option<i32>, String, String) {
    let path = format!(
        "{}:{}",
        dir.join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = Command::new("sh")
        .arg(repository().join("taskfiles/view-test-setup.sh"))
        .current_dir(dir)
        .env("PATH", path)
        .env("npm_config_offline", "true")
        .output()
        .expect("run sh");
    let calls = fs::read_to_string(dir.join("bin/npm-calls")).unwrap_or_default();
    (
        output.status.code(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
        calls,
    )
}

/// The control: every package of the lock installed and the digest equal, so no npm.
#[test]
fn adversary_setup_control_an_installed_tree_starts_no_npm() {
    let dir = root("control", None, &PACKAGES);
    let (status, out, calls) = setup(&dir);
    assert_eq!(status, Some(0), "{out}");
    assert_eq!(out, "view-tests: node_modules matches package-lock.json\n");
    assert_eq!(calls, "");
    let _ = fs::remove_dir_all(&dir);
}

/// A package of the lock removed by hand (or by an interrupted `npm uninstall`), the digest
/// kept: the installed tree is not the lock's, and the setup says it matches and starts no npm.
/// The timed part then fails with "Cannot find module 'playwright-core/lib/bootstrap'"
/// (adv/setup-cases2.txt), and every later run says "matches" again: before the change
/// `npm ci` ran each time and mended it.
#[test]
fn adversary_setup_reinstalls_a_tree_missing_a_package_of_the_lock() {
    let dir = root("missing-package", None, &["@playwright/test", "playwright"]);
    let (status, out, calls) = setup(&dir);
    assert_eq!(status, Some(0), "{out}");
    assert!(
        calls.contains("ci"),
        "node_modules lacks playwright-core, which package-lock.json pins, and the setup \
         started no npm ci: {out}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// package.json changed and the lock not: `npm ci` on the forge and in task check-runner (both
/// start without node_modules) fails ("No matching version found for @playwright/test@1.66.0",
/// adv/setup-cases.txt), and the setup on a tree installed before says "matches": task check
/// passes where the forge fails.
#[test]
fn adversary_setup_runs_npm_ci_when_package_json_left_the_lock() {
    let package = fs::read_to_string(repository().join("view-tests/package.json"))
        .expect("read package.json");
    let changed = package.replace(
        "\"@playwright/test\": \"1.65.0\"",
        "\"@playwright/test\": \"1.66.0\"",
    );
    assert_ne!(package, changed, "view-tests/package.json no longer pins 1.65.0");
    let dir = root("package-json", Some(&changed), &PACKAGES);
    let (_, out, calls) = setup(&dir);
    assert!(
        calls.contains("ci"),
        "package.json asks @playwright/test 1.66.0, the lock and node_modules hold 1.65.0, and \
         the setup started no npm ci, which would fail as on the forge: {out}"
    );
    let _ = fs::remove_dir_all(&dir);
}
