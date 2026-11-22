//! The scripted lane: every task of `tasks.txt`, one after the other, with the `surf-agent`,
//! `surfd` and `surf-session` of `SURF_BIN_DIR`, against one fixture server (a thread of this
//! test) and one `surfd` in one private `XDG_RUNTIME_DIR`. It writes the root
//! `.out/agent-tasks.txt` and holds each task to its row of `status.txt`. No model call; every
//! address is on 127.0.0.1; nothing is built.

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::time::Duration;

use surf_agent_tasks::agent::CALL_BOUND;
use surf_agent_tasks::lane::{self, Config, LaneRun};
use surf_agent_tasks::report::Lane;
use surf_agent_tasks::root_report_file;

use crate::status::binaries_field;

/// `SURF_BIN_DIR`, which the step `test` sets to the programs `task build` made.
fn bin_dir() -> PathBuf {
    let dir = std::env::var_os("SURF_BIN_DIR").map(PathBuf::from).expect(
        "SURF_BIN_DIR names the directory of surf-agent, surfd and surf-session (task test sets \
         it to target/debug after task build); this test builds nothing",
    );
    assert!(
        dir.is_absolute(),
        "SURF_BIN_DIR is {}, not an absolute path",
        dir.display()
    );
    for program in ["surf-agent", "surfd", "surf-session"] {
        assert!(
            dir.join(program).is_file(),
            "{} is not there",
            dir.join(program).display()
        );
    }
    dir
}

/// The facts of /proc every lane run is held to: one surfd from the first `open` on, no cargo
/// below the test, surfd ended after SIGKILL, no process of the run left, and its directory
/// removed.
fn assert_clean(run: &LaneRun) {
    let surfd: BTreeSet<u32> = run.surfd_samples.iter().flatten().copied().collect();
    assert_eq!(surfd.len(), 1, "surfd processes seen: {surfd:?}");
    assert!(
        run.surfd_samples.iter().all(|sample| sample.len() == 1),
        "a sample without exactly one surfd: {:?}",
        run.surfd_samples
    );
    assert!(
        run.cargo_seen.is_empty(),
        "cargo below the test: {:?}",
        run.cargo_seen
    );
    for result in &run.ended {
        result.as_ref().expect("surfd ended after SIGKILL");
    }
    assert!(
        run.left.is_empty(),
        "processes of the run left: {:?}",
        run.left
    );
    assert!(
        !run.directory.exists(),
        "{} is left",
        run.directory.display()
    );
}

#[test]
fn every_task() {
    let bin = bin_dir();
    let measure = std::env::var_os("SURF_AGENT_TASKS").is_some_and(|value| value == "measure");
    // The archive task passes DIR as typed; the line names that, never the absolute form.
    let given = std::env::var_os("SURF_BIN_DIR_GIVEN")
        .map(PathBuf::from)
        .unwrap_or_else(|| bin.clone());
    let config = Config {
        bin: bin.clone(),
        fallback: PathBuf::from(env!("CARGO_TARGET_TMPDIR")),
        call_bound: CALL_BOUND,
        lane: Lane {
            name: if measure { "archive" } else { "gate" }.to_string(),
            binaries: given,
        },
        out: root_report_file(),
    };
    let run = lane::run(&config);
    let written = fs::read_to_string(root_report_file()).expect("the report is written");
    print!("{written}");
    let first = written.lines().next().unwrap_or_default();
    assert!(
        !binaries_field(first).starts_with('/'),
        "a machine's path in {first:?}"
    );
    assert_clean(&run);
    assert_eq!(
        written.lines().count(),
        1 + run.results.len() + written.matches("\nraise: ").count(),
        "{written}"
    );
    if let Err(error) = &run.outcome {
        panic!("{error}");
    }
}

/// A directory of the programs of `SURF_BIN_DIR` whose `surf-agent` never answers a `look
/// --as`: every other call goes to the real one.
fn hanging_bin(real: &Path) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("agent-tasks-hanging-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let script = format!(
        "#!/bin/sh\n\
         # A surf-agent whose look --as never answers.\n\
         if [ \"$1\" = look ] && [ \"$2\" = --as ]; then exec /bin/sleep 120; fi\n\
         exec '{}' \"$@\"\n",
        real.join("surf-agent").display()
    );
    fs::write(dir.join("surf-agent"), script).unwrap();
    fs::set_permissions(dir.join("surf-agent"), fs::Permissions::from_mode(0o755)).unwrap();
    for program in ["surfd", "surf-session"] {
        symlink(real.join(program), dir.join(program)).unwrap();
    }
    dir
}

#[test]
fn a_call_that_never_answers_ends_at_its_bound() {
    let real = bin_dir();
    let bin = hanging_bin(&real);
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("agent-tasks-hanging-{}.txt", std::process::id()));
    fs::write(&out, "agent-tasks: a file of an earlier run\n").unwrap();
    let config = Config {
        bin: bin.clone(),
        fallback: PathBuf::from(env!("CARGO_TARGET_TMPDIR")),
        call_bound: Duration::from_secs(1),
        lane: Lane {
            name: "gate".to_string(),
            binaries: bin.clone(),
        },
        out: out.clone(),
    };
    let run = lane::run(&config);
    let written = fs::read_to_string(&out).expect("the report is written");
    assert_clean(&run);
    assert_eq!(run.outcome, Ok(()), "{written}");
    let lines: Vec<&str> = written.lines().collect();
    let shown = binaries_field(lines[0]);
    assert!(
        !shown.starts_with('/'),
        "a machine's path in {:?}",
        lines[0]
    );
    assert!(
        shown.ends_with(&format!("/agent-tasks-hanging-{}", std::process::id())),
        "{shown}"
    );
    assert_eq!(
        lines[0],
        format!("agent-tasks: 4 tasks, 0 pass, 4 fail, 0 skipped, lane gate, binaries {shown}")
    );
    assert_eq!(
        lines[1],
        "task article fail 3 calls 1 connections: no answer within 1 s on look"
    );
    assert_eq!(
        lines[2],
        "task table fail 3 calls 1 connections: no answer within 1 s on look"
    );
    let _ = fs::remove_file(&out);
    let _ = fs::remove_dir_all(&bin);
}
