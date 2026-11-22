//! The model lane's driver, `taskfiles/agent-tasks-model.sh`, run with the stub model
//! `tests/stub-model.sh` and the programs of `SURF_BIN_DIR`: no model is called. The stub runs
//! `surf-agent look` once and answers by the question it reads.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use surf_agent_tasks::format::{Server, read_tasks};
use surf_agent_tasks::tasks_file;

#[test]
fn a_stub_model_is_counted() {
    let bin = std::env::var_os("SURF_BIN_DIR")
        .map(PathBuf::from)
        .expect("SURF_BIN_DIR names the built programs, agent-tasks-serve among them");
    let manifest = env!("CARGO_MANIFEST_DIR");
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("agent-tasks-model-{}.txt", std::process::id()));
    let _ = fs::remove_file(&out);

    let output = Command::new("sh")
        .arg(format!("{manifest}/../../taskfiles/agent-tasks-model.sh"))
        .arg(&bin)
        .arg(&out)
        .env("AGENT_TASKS_SERVE", bin.join("agent-tasks-serve"))
        .env(
            "AGENT_TASKS_MODEL_COMMAND",
            format!("{manifest}/tests/stub-model.sh"),
        )
        .env("AGENT_TASKS_MODEL", "stub")
        .current_dir(manifest)
        .stdin(Stdio::null())
        .output()
        .expect("sh runs");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

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
    let written = fs::read_to_string(&out).expect("the out file is written");
    let (first, rest) = written.split_once('\n').expect("a first line");
    let prefix = format!(
        "agent-tasks-model: {n} tasks, 1 pass, {} fail, {} skipped, command stub-model.sh, model stub, ",
        h - 1,
        n - h
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
    let _ = fs::remove_file(&out);
}
