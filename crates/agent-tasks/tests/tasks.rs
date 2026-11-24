//! The scripted lane: every task of `tasks.txt`, one after the other, with the `surf-agent`,
//! `surfd` and `surf-session` of `SURF_BIN_DIR`, against one fixture server (a thread of this
//! test) and one `surfd` in one private `XDG_RUNTIME_DIR`. `every_task` also starts the one TLS
//! test server of the run, for the rows whose `server` is `https` and the check
//! `https-without-root`; the other test of the lane starts none and skips those rows. It writes
//! the root `.out/agent-tasks.txt` and holds each task to its row of `status.txt`. No model
//! call; every address is on 127.0.0.1 (`localhost` from `/etc/hosts`); nothing is built.

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use surf_agent_tasks::agent::CALL_BOUND;
use surf_agent_tasks::format::{Listed, Server, read_status, read_tasks};
use surf_agent_tasks::lane::{self, Config, Https, LaneRun};
use surf_agent_tasks::report::Lane;
use surf_agent_tasks::server::file_for;
use surf_agent_tasks::{root_report_file, sites_dir, status_file, tasks_file};
use surf_tls::crypto::Scheme;
use surf_tls_test_certs::cases;
use surf_tls_test_server::{Config as TlsConfig, Response, TestServer};

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
/// below the test, no process of the run but surfd after the last task, surfd ended after
/// SIGKILL, no process of the run left, and its directory removed. The two lists of processes
/// come from waits with a bound ([`surf_agent_tasks::process::END_BOUND`]), not from one read:
/// a process that is still ending is in /proc, and one that stays is in the list.
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
    assert!(
        run.after_tasks.is_empty(),
        "processes of the run but surfd after its last task: {:?}",
        run.after_tasks
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

/// The certificate case of the TLS server: its leaf is `localhost` (dNSName and iPAddress
/// 127.0.0.1), the host of every `https` address of the run.
const TLS_CASE: &str = "ok-rsa2048";

/// The root the chain of [`TLS_CASE`] leads to, the file `open --ca-file` names.
const TLS_ROOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tls-test-certs/certs/ok-rsa2048/root.pem"
);

/// The TLS test server of the `https` rows, on 127.0.0.1 port 0: the start and the target of
/// each such row answered 200 `text/html` with the bytes of its file under `sites/<task>/`,
/// every other path 404.
fn tls_server() -> TestServer {
    let spec = cases::specs()
        .into_iter()
        .find(|spec| spec.name == TLS_CASE)
        .expect("the certificate case is listed");
    assert_eq!((spec.host, spec.outcome), ("localhost", cases::Outcome::Ok));
    let mut config = TlsConfig::new(TLS_CASE, Scheme::RsaPssRsaeSha256);
    config.responses.clear();
    let tasks = read_tasks(&fs::read_to_string(tasks_file()).unwrap()).unwrap();
    for task in tasks.iter().filter(|task| task.server == Server::Https) {
        let site = sites_dir().join(&task.name);
        for path in [&task.start, &task.target] {
            let file = file_for(&site, path)
                .unwrap_or_else(|| panic!("{}: no file for {path}", task.name));
            config.responses.insert(
                path.clone(),
                Response {
                    status: 200,
                    reason: "OK".to_string(),
                    fields: vec![("Content-Type".to_string(), "text/html".to_string())],
                    body: fs::read(&file).unwrap(),
                },
            );
        }
    }
    TestServer::start(config).expect("the TLS test server starts")
}

#[test]
fn every_task() {
    let bin = bin_dir();
    let tls = Arc::new(tls_server());
    let counted = Arc::clone(&tls);
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
        https: Some(Https {
            origin: format!("https://localhost:{}", tls.port()),
            root: PathBuf::from(TLS_ROOT),
            accepted: Box::new(move || counted.accepted()),
        }),
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
    let checks: Vec<&str> = run
        .checks
        .iter()
        .map(|check| check.check.as_str())
        .collect();
    assert_eq!(checks, ["https-without-root"]);
    assert_eq!(
        written.lines().count(),
        1 + run.results.len() + run.checks.len() + written.matches("\nraise: ").count(),
        "{written}"
    );
    if let Err(error) = &run.outcome {
        panic!("{error}");
    }
}

/// A directory of the programs of `SURF_BIN_DIR` whose `surf-agent` never answers its first
/// `look --as` (that of the first task, article): every other call goes to the real one, so
/// the test waits for one bound only.
fn hanging_bin(real: &Path) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("agent-tasks-hanging-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let hung = dir.join("hung");
    let script = format!(
        "#!/bin/sh\n\
         # A surf-agent whose first look --as never answers.\n\
         if [ \"$1\" = look ] && [ \"$2\" = --as ] && [ ! -e '{}' ]; then\n\
         \x20 : > '{}'\n\
         \x20 exec /bin/sleep 120\n\
         fi\n\
         exec '{}' \"$@\"\n",
        hung.display(),
        hung.display(),
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
        // The gate's bound, sat out once: every other call is a real one and must answer
        // within it. With 1 s an `open` did not, at a load of 98 to 136 on 20 CPUs (2026-11-23).
        call_bound: CALL_BOUND,
        lane: Lane {
            name: "gate".to_string(),
            binaries: bin.clone(),
        },
        out: out.clone(),
        // No TLS server: one runs per test run, in `every_task`; the `https` rows are skipped.
        https: None,
    };
    let run = lane::run(&config);
    let written = fs::read_to_string(&out).expect("the report is written");
    assert_clean(&run);

    // What the hanging look causes, whatever the real surf-agent makes of the other tasks.
    let tasks = read_tasks(&fs::read_to_string(tasks_file()).unwrap()).unwrap();
    let status = read_status(&fs::read_to_string(status_file()).unwrap(), &tasks).unwrap();
    let lines: Vec<&str> = written.lines().collect();
    assert!(lines.len() > tasks.len(), "{written}");
    let task_lines = &lines[1..=tasks.len()];
    for (line, task) in task_lines.iter().zip(&tasks) {
        assert!(
            line.starts_with(&format!("task {} ", task.name)),
            "{line:?} is not the line of {}",
            task.name
        );
    }
    assert!(
        lines[1 + tasks.len()..]
            .iter()
            .all(|line| line.starts_with("raise: ")),
        "{written}"
    );
    for (line, task) in task_lines.iter().zip(&tasks) {
        if task.server == Server::Https {
            assert_eq!(*line, format!("task {} skipped", task.name), "{written}");
        }
    }
    assert_eq!(
        task_lines[0],
        format!(
            "task article fail 3 calls 1 connections: no answer within {} s on look",
            CALL_BOUND.as_secs()
        )
    );
    assert!(
        !task_lines[1..]
            .iter()
            .any(|line| line.contains("no answer within")),
        "only the first look hangs: {written}"
    );

    // The first line's counts, from the task lines of the same report.
    let state = |line: &str| line.split(' ').nth(2).unwrap_or_default().to_string();
    let pass = task_lines
        .iter()
        .filter(|line| state(line) == "pass")
        .count();
    let fail = task_lines
        .iter()
        .filter(|line| state(line) == "fail")
        .count();
    let skipped = task_lines
        .iter()
        .filter(|line| state(line) == "skipped")
        .count();
    assert_eq!(pass + fail + skipped, tasks.len(), "{written}");
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
        format!(
            "agent-tasks: {} tasks, {pass} pass, {fail} fail, {skipped} skipped, lane gate, \
             binaries {shown}",
            tasks.len()
        )
    );

    // The outcome: the error of the first task listed pass whose line is a failure, if any.
    let measuring = std::env::var_os("SURF_AGENT_TASKS").is_some_and(|value| value == "measure");
    let expected = status
        .iter()
        .zip(task_lines)
        .find(|(row, line)| row.listed == Listed::Pass && state(line) == "fail")
        .filter(|_| !measuring)
        .map(|(row, line)| {
            let (_, reason) = line.split_once(" connections: ").unwrap();
            format!(
                "agent-tasks: task {} is listed pass and failed: {reason}",
                row.task
            )
        });
    assert_eq!(run.outcome, expected.map_or(Ok(()), Err), "{written}");
    let _ = fs::remove_file(&out);
    let _ = fs::remove_dir_all(&bin);
}
