//! The step of `agent-tasks-archive` in `taskfiles/agent-tasks.yml` that finds `DIR`: its own
//! text, read from the Taskfile, run by `sh` from the repository's root as Task runs it, with a
//! stand-in `cargo` that writes what it was given. Nothing is built.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// The first command of `agent-tasks-archive`: the block under its `cmds:`, without the
/// block's indent.
fn archive_step() -> String {
    let taskfile = fs::read_to_string(format!("{ROOT}/taskfiles/agent-tasks.yml")).unwrap();
    let mut lines = taskfile
        .lines()
        .skip_while(|line| *line != "  agent-tasks-archive:")
        .skip_while(|line| *line != "    cmds:")
        .skip(1);
    assert_eq!(lines.next(), Some("      - |"), "the step is a block");
    let step: Vec<&str> = lines
        .take_while(|line| line.starts_with("        "))
        .map(|line| &line[8..])
        .collect();
    assert!(!step.is_empty(), "agent-tasks-archive has no step");
    step.join("\n") + "\n"
}

/// A directory of its own under `CARGO_TARGET_TMPDIR`, empty.
fn fresh(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("archive-task-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run the step with `DIR` given as `given` from the directory `caller`; its output and what the
/// stand-in `cargo` was given (`SURF_BIN_DIR`, `SURF_BIN_DIR_GIVEN`, `SURF_AGENT_TASKS`, its
/// arguments), when it ran.
fn run_step(caller: &Path, given: &str) -> (Output, Option<String>) {
    let stand_ins = fresh("bin");
    let seen = stand_ins.join("seen");
    fs::write(
        stand_ins.join("cargo"),
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$SURF_BIN_DIR\" \"$SURF_BIN_DIR_GIVEN\" \"$SURF_AGENT_TASKS\" \"$*\" > '{}'\n",
            seen.display()
        ),
    )
    .unwrap();
    fs::set_permissions(stand_ins.join("cargo"), fs::Permissions::from_mode(0o755)).unwrap();
    let output = Command::new("sh")
        .arg("-c")
        .arg(archive_step())
        .current_dir(ROOT)
        .env_clear()
        .env("PATH", format!("{}:/usr/bin:/bin", stand_ins.display()))
        .env("GIVEN_DIR", given)
        .env("CALLER_DIR", caller)
        .stdin(Stdio::null())
        .output()
        .expect("sh runs");
    let seen = fs::read_to_string(&seen).ok();
    let _ = fs::remove_dir_all(&stand_ins);
    (output, seen)
}

#[test]
fn a_relative_dir_is_taken_from_the_caller_and_named_as_typed() {
    let caller = fresh("caller");
    let unpacked = "archive-0.3.0/surf-0.3.0-x86_64-unknown-linux-musl";
    fs::create_dir_all(caller.join(unpacked)).unwrap();
    let absolute = caller.join(unpacked).canonicalize().unwrap();

    let (output, seen) = run_step(&caller, unpacked);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        seen.as_deref(),
        Some(
            format!(
                "{}\n{unpacked}\nmeasure\nnextest run -p surf-agent-tasks -E test(=tasks::every_task)\n",
                absolute.display()
            )
            .as_str()
        )
    );

    let (output, seen) = run_step(&caller, &absolute.display().to_string());
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let seen = seen.expect("cargo ran");
    assert!(
        seen.starts_with(&format!("{0}\n{0}\n", absolute.display())),
        "{seen}"
    );

    let (output, seen) = run_step(&caller, "no-such-dir");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(seen, None, "cargo ran on a missing DIR");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("agent-tasks-archive: no-such-dir is not a directory"),
        "{output:?}"
    );
    let _ = fs::remove_dir_all(&caller);
}
