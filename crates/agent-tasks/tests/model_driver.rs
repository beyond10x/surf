//! The model lane's driver, `taskfiles/agent-tasks-model.sh`, run with stand-in models and the
//! programs of `SURF_BIN_DIR`: no model is called. The stub `tests/stub-model.sh` runs
//! `surf-agent look` once and answers by the question it reads; the others are written here.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use surf_agent_tasks::format::{Server, read_tasks};
use surf_agent_tasks::tasks_file;

fn bin_dir() -> PathBuf {
    std::env::var_os("SURF_BIN_DIR")
        .map(PathBuf::from)
        .expect("SURF_BIN_DIR names the built programs, agent-tasks-serve among them")
}

/// Run the driver with the model command `model` and the extra variables `env`: its output and
/// the out file, when one is written.
fn drive(model: &str, name: &str, env: &[(&str, &str)]) -> (Output, Option<String>) {
    let bin = bin_dir();
    let manifest = env!("CARGO_MANIFEST_DIR");
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "agent-tasks-model-{name}-{}.txt",
        std::process::id()
    ));
    fs::write(&out, "agent-tasks-model: a file of an earlier run\n").unwrap();
    let mut command = Command::new("sh");
    command
        .arg(format!("{manifest}/../../taskfiles/agent-tasks-model.sh"))
        .arg(&bin)
        .arg(&out)
        .env("AGENT_TASKS_SERVE", bin.join("agent-tasks-serve"))
        .env("AGENT_TASKS_MODEL_COMMAND", model)
        .env("AGENT_TASKS_MODEL", "stub")
        .env_remove("AGENT_TASKS_MODEL_TIMEOUT")
        .current_dir(manifest)
        .stdin(Stdio::null());
    for (key, value) in env {
        command.env(key, value);
    }
    let output = command.output().expect("sh runs");
    let written = fs::read_to_string(&out).ok();
    let _ = fs::remove_file(&out);
    (output, written)
}

/// A stand-in model written to a file of its own: `body` after the `#!/bin/sh` line.
fn stand_in(name: &str, body: &str) -> String {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("agent-tasks-{name}-{}.sh", std::process::id()));
    fs::write(&path, format!("#!/bin/sh\n{body}")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path.display().to_string()
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn a_stub_model_is_counted() {
    let bin = bin_dir();
    let stub = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/stub-model.sh");
    let (output, written) = drive(stub, "stub", &[]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));

    let tasks = read_tasks(&fs::read_to_string(tasks_file()).unwrap()).unwrap();
    let n = tasks.len();
    let h = tasks.iter().filter(|t| t.server == Server::Http).count();
    let mut body = String::new();
    for task in &tasks {
        if task.server != Server::Http {
            body.push_str(&format!(
                "task {} skipped: server {}\n",
                task.name,
                task.server.word()
            ));
            continue;
        }
        let (verdict, answered) = match task.name.as_str() {
            "article" => ("pass", "1932"),
            "table" => ("fail", "2.41"),
            _ => ("fail", "unknown"),
        };
        body.push_str(&format!(
            "task {} {verdict} 1 calls\n  surf-agent look\n  answered \"{answered}\"\n",
            task.name
        ));
    }
    let written = written.expect("the out file is written");
    let (first, rest) = written.split_once('\n').expect("a first line");
    assert!(
        !crate::status::binaries_field(first).starts_with('/'),
        "a machine's path in {first:?}"
    );
    // SURF_BIN_DIR is target/debug of the repository: the line names it from the root.
    let root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .canonicalize()
        .unwrap();
    let shown = bin
        .canonicalize()
        .unwrap()
        .strip_prefix(&root)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| crate::status::binaries_field(first).to_string());
    let prefix = format!(
        "agent-tasks-model: {n} tasks, 1 pass, {} fail, {} skipped, command stub-model.sh, \
         model stub, lane model, binaries {shown}, ",
        h - 1,
        n - h,
    );
    let date = first
        .strip_prefix(&prefix)
        .unwrap_or_else(|| panic!("first line {first:?}, expected it to start {prefix:?}"));
    assert_eq!(date.len(), 10, "{date:?}");
    assert!(
        date.char_indices().all(|(i, c)| if i == 4 || i == 7 {
            c == '-'
        } else {
            c.is_ascii_digit()
        }),
        "{date:?} is not YYYY-MM-DD"
    );
    assert_eq!(rest, body);
}

#[test]
fn a_model_over_its_bound_fails_the_lane() {
    let model = stand_in("slow-model", "exec /bin/sleep 120\n");
    let (output, written) = drive(&model, "slow", &[("AGENT_TASKS_MODEL_TIMEOUT", "1")]);
    assert_ne!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(written, None, "no count line is written");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let name = model.rsplit('/').next().unwrap();
    assert!(
        stderr.contains(name) && stderr.contains("did not end within 1 s"),
        "{stderr}"
    );
    let _ = fs::remove_file(&model);
}

#[test]
fn a_model_that_exits_non_zero_fails_the_lane() {
    let model = stand_in("failing-model", "echo 'ANSWER: 1932'\nexit 3\n");
    let (output, written) = drive(&model, "failing", &[]);
    assert_ne!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(written, None, "no count line is written");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("exited 3"), "{stderr}");
    let _ = fs::remove_file(&model);
}

#[test]
fn a_model_that_exits_124_by_itself_is_not_over_its_bound() {
    let model = stand_in("exit-124-model", "exit 124\n");
    let (output, written) = drive(&model, "exit-124", &[]);
    assert_ne!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(written, None, "no count line is written");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("exited 124") && !stderr.contains("did not end"),
        "{stderr}"
    );
    let _ = fs::remove_file(&model);
}

#[test]
fn a_control_character_in_an_answer_is_written_as_hex() {
    let model = stand_in("esc-model", "printf 'ANSWER: 19\\03332\\t\\177\\n'\n");
    let (output, written) = drive(&model, "esc", &[]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let written = written.expect("the out file is written");
    assert!(
        written.contains("\ntask article fail 0 calls\n  answered \"19\\x1b32\\x09\\x7f\"\n"),
        "{written:?}"
    );
    let _ = fs::remove_file(&model);
}

#[test]
fn a_carriage_return_in_an_answer_is_escaped() {
    let model = stand_in("cr-model", "printf 'ANSWER: 19\\r32\\\\x\\n'\n");
    let (output, written) = drive(&model, "cr", &[]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let written = written.expect("the out file is written");
    assert!(!written.contains('\r'), "{written:?}");
    assert!(
        written.contains("\ntask article fail 0 calls\n  answered \"19\\r32\\\\x\"\n"),
        "{written}"
    );
    let _ = fs::remove_file(&model);
}
