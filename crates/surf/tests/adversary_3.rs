//! Third review pass on `task lock-check` for `story:scripts-engine-dependency` (fix round 2):
//! the text rules on the tree's cargo config files and the flag rule on its task and workflow
//! files. Each case writes the committed lock, lists, manifest and `.cargo/config.toml` into a
//! directory of its own under `CARGO_TARGET_TMPDIR`, adds the files it names, and runs the
//! task's line there. The first four cases show files that turn swell's overflow checks off and
//! pass; the cases marked "Held" record what passes as it should.

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

/// Runs the task's line in a fresh copy named `name` with the committed files and `extra`
/// written at their paths (an `extra` file replaces a committed one of the same path).
fn check(name: &str, extra: &[(&str, String)]) -> (Option<i32>, String) {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("lock-check-adversary-3")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    let mut files: Vec<(&str, String)> = [
        "taskfiles/lock-check.sh",
        "taskfiles/lock-allow.txt",
        "taskfiles/lock-owners.txt",
        "Cargo.lock",
        "Cargo.toml",
        ".cargo/config.toml",
    ]
    .into_iter()
    .map(|path| (path, read(path)))
    .collect();
    files.extend(extra.iter().cloned());
    for (path, text) in &files {
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

fn assert_fails(name: &str, extra: &[(&str, String)]) {
    let (status, text) = check(name, extra);
    assert_ne!(status, Some(0), "{name}: the check passed:\n{text}");
}

fn assert_passes(name: &str, extra: &[(&str, String)]) {
    let (status, text) = check(name, extra);
    assert_eq!(status, Some(0), "{name}:\n{text}");
}

/// The committed `.cargo/config.toml` with `text` after it.
fn config_after(text: &str) -> String {
    format!("{}\n{text}", read(".cargo/config.toml"))
}

/// rustflags naming an argument file: rustc reads `@<path>` as the arguments the file holds,
/// after the profile's flag. Probe (review report3): with
/// `rustflags = ["@<absolute path>/flags"]` under `[target.x86_64-unknown-linux-gnu]` and the
/// file holding `-Coverflow-checks=off`, swell-num's rustc line read `-C overflow-checks=on
/// ... @<path>/flags`, and a program built with `-C overflow-checks=on @flags` did not panic on
/// an overflow. The config line holds no `overflow`, and the check reads no argument file.
/// A relative path fails for a package from git, whose rustc runs in its own directory; any
/// fixed absolute path does it (a runner's checkout path is one).
#[test]
fn a_config_rustflags_argument_file_fails() {
    assert_fails(
        "config-argfile",
        &[
            (
                ".cargo/config.toml",
                config_after(
                    "[target.x86_64-unknown-linux-gnu]\nrustflags = [\"@/srv/build/surf/.cargo/flags\"]\n",
                ),
            ),
            (".cargo/flags", "-Coverflow-checks=off\n".to_string()),
        ],
    );
}

/// The workflow and task lines that the flag rule (`overflow-checks=`) misses: rustc reads
/// `-C overflow_checks=off` as `-C overflow-checks=off` (probe: swell-num's rustc line read
/// `overflow-checks=on overflow_checks=off`, and a program so built did not panic), and
/// `--config` takes TOML, where spaces around `=` are allowed (probe: `--config
/// 'profile.release.package.swell-num.overflow-checks = false'` built swell-num with no
/// overflow-checks flag).
#[test]
fn task_and_workflow_lines_the_flag_rule_misses_fail() {
    assert_fails(
        "workflow-underscore",
        &[(
            ".github/workflows/build.yml",
            "on: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n    env:\n      RUSTFLAGS: -Coverflow_checks=off\n    steps:\n      - run: cargo build --release\n".to_string(),
        )],
    );
    assert_fails(
        "task-config-spaced",
        &[(
            "taskfiles/build.yml",
            "version: '3'\ntasks:\n  build:\n    cmds:\n      - cargo build --release --config 'profile.release.package.swell-num.overflow-checks = false'\n".to_string(),
        )],
    );
}

/// A workflow file named `.yaml`, which GitHub runs as it runs `.yml`; the check reads
/// `.github/workflows/*.yml` only.
#[test]
fn a_workflow_named_yaml_fails() {
    assert_fails(
        "workflow-yaml",
        &[(
            ".github/workflows/build.yaml",
            "on: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n    env:\n      RUSTFLAGS: -C overflow-checks=off\n    steps:\n      - run: cargo build --release\n".to_string(),
        )],
    );
}

/// A script under `taskfiles/` that a task runs, setting the flag for its cargo; the check
/// reads the task files, not the scripts they run (`taskfiles/release-smoke.sh` and others run
/// cargo today).
#[test]
fn a_script_a_task_runs_fails() {
    assert_fails(
        "task-script",
        &[(
            "taskfiles/build-release.sh",
            "#!/bin/sh\nRUSTFLAGS='-C overflow-checks=off' cargo build --release\n".to_string(),
        )],
    );
}

/// Held: the word in a comment, and `overflow-checks = true` in the release profile with a
/// comment after it, pass.
#[test]
fn comments_and_the_true_line_pass() {
    assert_passes(
        "config-comments",
        &[(
            ".cargo/config.toml",
            config_after(
                "# overflow checks stay on in every profile\n[profile.release] # not overflow\nopt-level = 3\noverflow-checks = true # on\n",
            ),
        )],
    );
}
