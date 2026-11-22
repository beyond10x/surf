//! The scripted agent of the gate's lane: the steps of each task, and the `surf-agent` calls
//! that take them.
//!
//! Per task: `open <start address>`; then the task's own steps, each handle found through
//! [`surf_answer::parse`] by its label in the `now` block of a `look` without `--as`, never a
//! handle written here; then `look --as markdown`, and `look --as markdown --from-line <n>` for
//! every `more` line, until the text block ends; then `close`, also after a failed step. Every
//! call runs `surf-agent` of one directory with an environment of nothing but `PATH` (that
//! directory alone), the private `XDG_RUNTIME_DIR`, and after `open` the session and its token.

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use surf_answer::Answer;

use crate::judge::{Run, StepFailure};

/// The most `look --as markdown` calls one task makes, a bound against an answer whose `more`
/// line never ends.
const PAGES_MAX: usize = 1_000;

/// One step of the scripted agent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// `open <start address>`.
    Open,
    /// `look`, without `--as`: the answer whose `now` block the next handles come from.
    Look,
    /// `look --as markdown`, then `look --as markdown --from-line <n>` for every `more` line.
    LookMarkdown,
    /// `follow <handle>`, the handle of the `now` line with this label.
    Follow(&'static str),
    /// `fill <handle> -`, the handle of the `now` line with the first label, the second on
    /// standard input.
    Fill(&'static str, &'static str),
    /// `press <handle>`, the handle of the `now` line with this label.
    Press(&'static str),
    /// `close`.
    Close,
}

impl Step {
    /// The command word.
    pub fn word(&self) -> &'static str {
        match self {
            Step::Open => "open",
            Step::Look | Step::LookMarkdown => "look",
            Step::Follow(_) => "follow",
            Step::Fill(..) => "fill",
            Step::Press(_) => "press",
            Step::Close => "close",
        }
    }
}

/// The steps of a task, by its name; `None` for a task this agent has no steps for.
pub fn steps(task: &str) -> Option<Vec<Step>> {
    use Step::*;
    Some(match task {
        "article" | "table" => vec![Open, LookMarkdown, Close],
        "link" => vec![Open, Look, Follow("Opening hours"), LookMarkdown, Close],
        "search" => vec![
            Open,
            Look,
            Fill("Search", "lighthouse"),
            Press("Find"),
            LookMarkdown,
            Close,
        ],
        _ => return None,
    })
}

/// The handle of the line labelled `label` in the `now` block of `answer`.
///
/// `surf_answer::Answer` has no `now` block before `story:agent-follow-links`, which adds it to
/// the answer and reads its lines here; until then no answer has a `now` line, and a step that
/// needs a handle fails with `no now line labelled "<label>"`.
fn now_handle(answer: Option<&Answer>, label: &str) -> Option<String> {
    let _ = (answer?, label);
    None
}

/// `surf-agent` of one directory in one private `XDG_RUNTIME_DIR`.
pub struct Agent {
    bin: PathBuf,
    runtime: PathBuf,
}

/// The session `open` gave.
struct Session {
    id: String,
    token: String,
}

impl Agent {
    /// The agent of the `surf-agent` in `bin`, in the `XDG_RUNTIME_DIR` `runtime`.
    pub fn new(bin: PathBuf, runtime: PathBuf) -> Agent {
        Agent { bin, runtime }
    }

    /// One `surf-agent` call; `after_call` runs when it has returned.
    fn call(
        &self,
        words: &[&str],
        session: Option<&Session>,
        input: Option<&str>,
        after_call: &mut dyn FnMut(),
    ) -> Output {
        let mut command = Command::new(self.bin.join("surf-agent"));
        command
            .args(words)
            .env_clear()
            .env("PATH", OsString::from(self.bin.as_os_str()))
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(session) = session {
            command
                .env("SURF_SESSION", &session.id)
                .env("SURF_TOKEN", &session.token);
        }
        command.stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        let mut child = command.spawn().expect("surf-agent starts");
        if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
            let _ = stdin.write_all(input.as_bytes());
        }
        let output = child.wait_with_output().expect("surf-agent is waited for");
        after_call();
        output
    }

    /// Run one task's steps from `start`, the address of its start page. The connections of
    /// the returned [`Run`] are left at 0: the caller counts them at the fixture server.
    pub fn run(&self, steps: &[Step], start: &str, after_call: &mut dyn FnMut()) -> Run {
        let mut run = Run::default();
        let mut session: Option<Session> = None;
        let mut last_look: Option<Answer> = None;
        let exit = |output: &Output, command: &str| StepFailure::Exit {
            status: output.status.code().unwrap_or(-1),
            command: command.to_string(),
        };
        for step in steps {
            if run.failure.is_some() && *step != Step::Close {
                continue;
            }
            match step {
                Step::Open => {
                    run.calls += 1;
                    let output = self.call(&["open", start], None, None, after_call);
                    if !output.status.success() {
                        run.failure = Some(exit(&output, "open"));
                        continue;
                    }
                    let parsed = surf_answer::parse(&String::from_utf8_lossy(&output.stdout));
                    match parsed.ok().and_then(|a| Some((a.head.session, a.token?))) {
                        Some((id, token)) => session = Some(Session { id, token }),
                        None => run.failure = Some(exit(&output, "open")),
                    }
                }
                Step::Look => {
                    run.calls += 1;
                    let output = self.call(&["look"], session.as_ref(), None, after_call);
                    if !output.status.success() {
                        run.failure = Some(exit(&output, "look"));
                        continue;
                    }
                    last_look = surf_answer::parse(&String::from_utf8_lossy(&output.stdout)).ok();
                }
                Step::LookMarkdown => {
                    let mut from: Option<u64> = None;
                    for _ in 0..PAGES_MAX {
                        let from_text = from.map(|n| n.to_string());
                        let mut words = vec!["look", "--as", "markdown"];
                        if let Some(n) = &from_text {
                            words.extend(["--from-line", n.as_str()]);
                        }
                        run.calls += 1;
                        let output = self.call(&words, session.as_ref(), None, after_call);
                        if !output.status.success() {
                            run.failure = Some(exit(&output, "look"));
                            break;
                        }
                        let text = String::from_utf8_lossy(&output.stdout).into_owned();
                        let next = surf_answer::parse(&text)
                            .ok()
                            .and_then(|answer| answer.text)
                            .and_then(|block| {
                                let next = block.first + block.lines.len() as u64;
                                (!block.lines.is_empty() && next <= block.total).then_some(next)
                            });
                        run.answers.push(text);
                        match next {
                            Some(next) => from = Some(next),
                            None => break,
                        }
                    }
                }
                Step::Follow(label) | Step::Press(label) => {
                    let Some(handle) = now_handle(last_look.as_ref(), label) else {
                        run.failure = Some(StepFailure::NoNowLine {
                            label: label.to_string(),
                        });
                        continue;
                    };
                    run.calls += 1;
                    let word = step.word();
                    let output = self.call(&[word, &handle], session.as_ref(), None, after_call);
                    if !output.status.success() {
                        run.failure = Some(exit(&output, word));
                    }
                }
                Step::Fill(label, value) => {
                    let Some(handle) = now_handle(last_look.as_ref(), label) else {
                        run.failure = Some(StepFailure::NoNowLine {
                            label: label.to_string(),
                        });
                        continue;
                    };
                    run.calls += 1;
                    let output = self.call(
                        &["fill", &handle, "-"],
                        session.as_ref(),
                        Some(value),
                        after_call,
                    );
                    if !output.status.success() {
                        run.failure = Some(exit(&output, "fill"));
                    }
                }
                Step::Close => {
                    if session.is_none() {
                        continue;
                    }
                    run.calls += 1;
                    let output = self.call(&["close"], session.as_ref(), None, after_call);
                    if !output.status.success() && run.failure.is_none() {
                        run.failure = Some(exit(&output, "close"));
                    }
                }
            }
        }
        run
    }
}
