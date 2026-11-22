//! The run's file and the rule from (listed state, judged result) to what is written.
//!
//! The file, exactly: the first line `agent-tasks: <n> tasks, <p> pass, <f> fail, <s> skipped,
//! lane <lane>, binaries <directory>`, `<n>` the rows of `status.txt` (one per block of
//! `tasks.txt`), `<lane>` `gate` or `archive`, `<directory>` that of the binaries the run used, never a path of this
//! machine ([`shown_binaries`]);
//! one line per task in their order, `task <name> pass <c> calls <k> connections` or `task
//! <name> fail <c> calls <k> connections: <reason>` (a row without a result: `task <name>
//! skipped`); then `raise: <name>` for each task listed `pending` that passed.
//!
//! A `task` line is the judged result alone. (`pending`, pass) adds `raise: <name>`; (`pass`,
//! fail) makes [`report`] return an error after the file is written; (`pass`, pass) and
//! (`pending`, fail) add nothing. With `SURF_AGENT_TASKS=measure` the listed states are not
//! enforced and no `raise:` line is written.
//!
//! The file is written whole at the end of a run (to `<file>.part.<pid>`, then renamed), and a run
//! removes it before it starts ([`crate::lane::run`]), so a file left by an earlier run is
//! never read as this run's.

use std::fs;
use std::path::{Path, PathBuf};

use crate::format::{Listed, Status};
use crate::judge::Verdict;

/// One task's judged result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskResult {
    /// The task's name.
    pub task: String,
    /// Every `surf-agent` call of the task, `close` included.
    pub calls: usize,
    /// The connections the fixture server accepted for it.
    pub connections: usize,
    /// The judged result.
    pub verdict: Verdict,
}

/// The lane a report is of, named on its first line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lane {
    /// `gate` or `archive` (the model lane writes its own file).
    pub name: String,
    /// The directory of the binaries the run used, as its caller gave it; the line names it by
    /// [`shown_binaries`].
    pub binaries: PathBuf,
}

/// The repository's root, from this crate's directory.
fn repository_root() -> Option<PathBuf> {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .canonicalize()
        .ok()
}

/// How the first line names the directory of the binaries, so that it holds no path of this
/// machine and no user's name (the line goes into release notes): a relative directory as
/// given; an absolute one inside the repository relative to its root (`target/debug`); the
/// home directory itself and `/` as `-`; any other absolute one by its last component.
pub fn shown_binaries(dir: &Path) -> String {
    if !dir.is_absolute() {
        return dir.display().to_string();
    }
    let real = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    if let Some(root) = repository_root()
        && let Ok(relative) = real.strip_prefix(&root)
    {
        return match relative.as_os_str().is_empty() {
            true => ".".to_string(),
            false => relative.display().to_string(),
        };
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let is_home = home.is_some_and(|home| {
        let home_real = home.canonicalize().unwrap_or_else(|_| home.clone());
        real == home_real || dir.components().eq(home.components())
    });
    match real.file_name() {
        Some(name) if !is_home => name.to_string_lossy().into_owned(),
        _ => "-".to_string(),
    }
}

/// Whether `SURF_AGENT_TASKS=measure` is set: the listed states play no part.
fn measuring() -> bool {
    std::env::var_os("SURF_AGENT_TASKS").is_some_and(|value| value == "measure")
}

/// Write the report of `lane` to `out` (its directory made when missing), then hold each task
/// to its listed state: the error of the first task listed `pass` that failed.
pub fn report(
    results: &[TaskResult],
    status: &[Status],
    lane: &Lane,
    out: &Path,
) -> Result<(), String> {
    let enforce = !measuring();
    let (mut pass, mut fail, mut skipped) = (0, 0, 0);
    let mut lines = String::new();
    let mut raises = String::new();
    let mut first_error = None;
    for row in status {
        let Some(result) = results.iter().find(|result| result.task == row.task) else {
            skipped += 1;
            lines.push_str(&format!("task {} skipped\n", row.task));
            continue;
        };
        let counts = format!("{} calls {} connections", result.calls, result.connections);
        match &result.verdict {
            Verdict::Pass => {
                pass += 1;
                lines.push_str(&format!("task {} pass {counts}\n", row.task));
                if enforce && matches!(row.listed, Listed::Pending(_)) {
                    raises.push_str(&format!("raise: {}\n", row.task));
                }
            }
            Verdict::Fail(reason) => {
                fail += 1;
                lines.push_str(&format!("task {} fail {counts}: {reason}\n", row.task));
                if enforce && row.listed == Listed::Pass && first_error.is_none() {
                    first_error = Some(format!(
                        "agent-tasks: task {} is listed pass and failed: {reason}",
                        row.task
                    ));
                }
            }
        }
    }
    let text = format!(
        "agent-tasks: {} tasks, {pass} pass, {fail} fail, {skipped} skipped, lane {}, binaries \
         {}\n{lines}{raises}",
        status.len(),
        lane.name,
        shown_binaries(&lane.binaries)
    );
    if let Some(dir) = out.parent() {
        fs::create_dir_all(dir)
            .map_err(|error| format!("agent-tasks: cannot make {}: {error}", dir.display()))?;
    }
    let mut part = out.as_os_str().to_owned();
    // Two runs at once write two part files; the last rename wins.
    part.push(format!(".part.{}", std::process::id()));
    let part = PathBuf::from(part);
    fs::write(&part, text)
        .and_then(|()| fs::rename(&part, out))
        .map_err(|error| format!("agent-tasks: cannot write {}: {error}", out.display()))?;
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
