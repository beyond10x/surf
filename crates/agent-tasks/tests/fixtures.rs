//! The fixture files: each task's expected answer is on its target page once and on no other
//! page of its site.

use std::fs;
use std::path::{Path, PathBuf};

use surf_agent_tasks::format::read_tasks;
use surf_agent_tasks::server::file_for;
use surf_agent_tasks::{sites_dir, tasks_file};

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            let kind = entry.file_type().unwrap();
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                files.push(entry.path());
            }
        }
    }
    files.sort();
    files
}

#[test]
fn each_answer_is_on_its_target_page_only() {
    let tasks = read_tasks(&fs::read_to_string(tasks_file()).unwrap()).unwrap();
    // Every row, `https` included: the TLS server of `tasks::every_task` serves the target's
    // file of `sites/<task>/` under its path, as the fixture server does under `/<task>/`.
    for task in &tasks {
        let site = sites_dir().join(&task.name);
        let target = file_for(&site, &task.target)
            .unwrap_or_else(|| panic!("{}: no file for {}", task.name, task.target));
        let start = file_for(&site, &task.start)
            .unwrap_or_else(|| panic!("{}: no file for {}", task.name, task.start));
        assert!(start.is_file() && target.is_file());
        let files = files_under(&site);
        assert!(files.contains(&target), "{}", target.display());
        for file in &files {
            let text = fs::read_to_string(file).unwrap();
            let count = text.matches(task.answer.as_str()).count();
            let expected = usize::from(*file == target);
            assert_eq!(
                count,
                expected,
                "{}: {:?} occurs {count} times in {}",
                task.name,
                task.answer,
                file.display()
            );
        }
    }
}
