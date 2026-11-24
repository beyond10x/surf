//! Second review pass on `task lock-check` for `story:scripts-engine-dependency` (fix round 1):
//! the rule on the tree's cargo config files. Each case writes the committed files into a
//! directory of its own under `CARGO_TARGET_TMPDIR`, with a `.cargo/config.toml` (or
//! `.cargo/config`) of its own, and runs the task's line there. The first three cases show forms
//! that turn overflow checks off in swell and pass; the cases marked "Held" record what the rule
//! refused or let pass as it should.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const LINE: &str = "sh taskfiles/lock-check.sh Cargo.lock taskfiles/lock-allow.txt taskfiles/lock-owners.txt Cargo.toml";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    fs::read_to_string(repository().join(path)).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

/// Runs the task's line in a fresh copy named `name`: the committed lock, lists and manifest
/// (the manifest passed through `manifest`), and `extra` files written at their paths.
fn check_with(
    name: &str,
    manifest: impl Fn(String) -> String,
    extra: &[(&str, String)],
) -> (Option<i32>, String) {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("lock-check-adversary-2")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("taskfiles")).expect("create copy");
    for path in [
        "taskfiles/lock-check.sh",
        "taskfiles/lock-allow.txt",
        "taskfiles/lock-owners.txt",
        "Cargo.lock",
    ] {
        fs::write(dir.join(path), read(path)).unwrap_or_else(|e| panic!("write {path}: {e}"));
    }
    fs::write(dir.join("Cargo.toml"), manifest(read("Cargo.toml"))).expect("write Cargo.toml");
    for (path, text) in extra {
        let file = dir.join(path);
        fs::create_dir_all(file.parent().expect("a parent")).expect("create dir");
        fs::write(&file, text).unwrap_or_else(|e| panic!("write {path}: {e}"));
    }
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

/// Runs the check with `config` as the tree's `.cargo/config.toml`.
fn check_config(name: &str, config: String) -> (Option<i32>, String) {
    check_with(name, |m| m, &[(".cargo/config.toml", config)])
}

/// The committed `.cargo/config.toml` with `text` after it.
fn config_after(text: &str) -> String {
    format!("{}\n{text}", read(".cargo/config.toml"))
}

fn assert_fails(name: &str, (status, text): (Option<i32>, String)) {
    assert_ne!(status, Some(0), "{name}: the check passed:\n{text}");
}

/// A swell package and the key written with TOML's `\u` escapes, which cargo decodes: a release
/// build of swell-num with this config file has no `-C overflow-checks=on` (probe
/// `cargo build --release -p swell-num -v --config <this file>`, review report2). The check
/// compares the escaped text with the package names and with `overflow-checks`, and passes.
#[test]
fn a_config_with_escaped_keys_fails() {
    let config = config_after(
        "[profile.release.package.\"swell\\u002Dnum\"]\n\"overflow\\u002Dchecks\" = false\n",
    );
    assert_fails("config-escaped", check_config("config-escaped", config));
}

/// A root key holding a multi-line string with a line `[env]` in it, then the dotted key that
/// turns swell-num's overflow checks off. TOML reads one root key and one dotted key of
/// `profile`; the check reads the line `[env]` as a table and files the dotted key under it, and
/// passes. The probe with the same lines as a config file built swell-num without
/// `-C overflow-checks=on`. The config file has no rule against a multi-line string.
#[test]
fn a_config_hiding_a_profile_key_after_a_string_fails() {
    let config = format!(
        "note = \"\"\"\n[env]\n\"\"\"\nprofile.release.package.swell-num.overflow-checks = false\n\n{}",
        read(".cargo/config.toml")
    );
    assert_fails("config-hidden", check_config("config-hidden", config));
}

/// `rustflags` holding `-C overflow-checks=off`: cargo adds them after the profile's flags, and
/// rustc takes the last (probe: swell-num's rustc line read `-C overflow-checks=on ...
/// -C overflow-checks=off`; a program built with `-C overflow-checks=on -C overflow-checks=off`
/// did not panic on an overflow). The check reads only `profile` tables, and passes. Two forms:
/// `[target.<triple>]`, and `[build]`, which cargo uses where no `target` rustflags are set.
#[test]
fn a_config_with_rustflags_turning_overflow_checks_off_fails() {
    for (name, table) in [
        (
            "config-target-rustflags",
            "[target.x86_64-unknown-linux-gnu]",
        ),
        ("config-build-rustflags", "[build]"),
    ] {
        let config = config_after(&format!(
            "{table}\nrustflags = [\"-C\", \"overflow-checks=off\"]\n"
        ));
        assert_fails(name, check_config(name, config));
    }
}
