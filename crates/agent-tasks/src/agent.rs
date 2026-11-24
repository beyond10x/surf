//! The scripted agent of the gate's lane: the steps of each task, and the `surf-agent` calls
//! that take them.
//!
//! Per task: `open <start address>` (`open --ca-file <root> <start address>` for a row whose
//! server is `https`); then the task's own steps, each handle found through
//! [`surf_answer::parse`] by its label in the `now` block of a `look` without `--as`, never a
//! handle written here; then `look --as markdown`, and `look --as markdown --from-line <n>` for
//! every `more` line, until the text block ends; then `close`, also after a failed step. Every
//! call runs `surf-agent` of one directory with an environment of nothing but `PATH` (that
//! directory alone), the private `XDG_RUNTIME_DIR`, and after `open` the session and its token.
//! A call ends within its bound ([`CALL_BOUND`] unless [`Agent::with_call_bound`] sets
//! another), its output read to the end included: at the bound its process group is killed,
//! the reading stops and its step fails. A call whose output passes [`OUTPUT_CAP`] (unless
//! [`Agent::with_output_cap`] sets another) is killed the same way and its step fails.

use std::ffi::OsString;
use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use surf_answer::Answer;

use crate::judge::{Run, StepFailure};

/// The most `look --as markdown` calls one task makes, a bound against an answer whose `more`
/// line never ends.
const PAGES_MAX: usize = 1_000;

/// The bound of one `surf-agent` call: a call still running after it is killed and its step
/// fails with `no answer within 10 s on <command>`. It ends a hung call; no test asserts a time
/// with it.
pub const CALL_BOUND: Duration = Duration::from_secs(10);

/// The most bytes of a call's standard output and standard error together that are kept: a
/// call that writes more is killed and its step fails with `output over 1048576 bytes on
/// <command>`.
pub const OUTPUT_CAP: usize = 1 << 20;

/// The pause between two looks at a running call.
const POLL: Duration = Duration::from_millis(10);

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
        "article" | "table" | "https" => vec![Open, LookMarkdown, Close],
        "link" => vec![Open, Look, Follow("Opening hours"), LookMarkdown, Close],
        "search" => vec![
            Open,
            Look,
            Fill("Search", "lighthouse"),
            Press("Find"),
            LookMarkdown,
            Close,
        ],
        "login" => vec![
            Open,
            Look,
            Fill("Name", "ada"),
            Fill("Password", "lovelace"),
            Press("Sign in"),
            LookMarkdown,
            Close,
        ],
        _ => return None,
    })
}

/// The handle of the first line labelled `label` in the `now` block of `answer`, read through
/// [`surf_answer::parse`]: a label the block shows whole, not cut. `None` when no line has it,
/// and a step that needs a handle then fails with `no now line labelled "<label>"`.
fn now_handle(answer: Option<&Answer>, label: &str) -> Option<String> {
    answer?
        .now
        .as_ref()?
        .elements
        .iter()
        .find(|element| !element.label.cut && element.label.kept == label)
        .map(|element| element.handle.clone())
}

/// `surf-agent` of one directory in one private `XDG_RUNTIME_DIR`.
pub struct Agent {
    bin: PathBuf,
    runtime: PathBuf,
    bound: Duration,
    cap: usize,
}

/// The session `open` gave.
struct Session {
    id: String,
    token: String,
}

/// How a `surf-agent` call ended.
enum Called {
    /// It exited and its output was read to the end.
    Exited(Output),
    /// It, or the reading of its output, ran past the bound, and it was killed.
    TimedOut,
    /// Its output passed the cap, and it was killed.
    TooLarge,
}

/// A piece of a call's output, from the thread that reads one of its pipes.
enum Piece {
    /// Bytes of standard output (`true`) or standard error (`false`).
    Bytes(bool, Vec<u8>),
    /// The end of one pipe.
    End,
}

/// Read a pipe in a thread, sending each piece as it comes and [`Piece::End`] at its end. The
/// thread ends when the pipe ends or the receiver is gone.
fn drain(pipe: Option<impl Read + Send + 'static>, stdout: bool, sender: mpsc::Sender<Piece>) {
    let Some(mut pipe) = pipe else {
        let _ = sender.send(Piece::End);
        return;
    };
    thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match pipe.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if sender
                        .send(Piece::Bytes(stdout, buffer[..n].to_vec()))
                        .is_err()
                    {
                        return;
                    }
                }
            }
        }
        let _ = sender.send(Piece::End);
    });
}

impl Agent {
    /// The agent of the `surf-agent` in `bin`, in the `XDG_RUNTIME_DIR` `runtime`, each call
    /// bounded by [`CALL_BOUND`].
    pub fn new(bin: PathBuf, runtime: PathBuf) -> Agent {
        Agent {
            bin,
            runtime,
            bound: CALL_BOUND,
            cap: OUTPUT_CAP,
        }
    }

    /// The same agent with another bound for each call.
    pub fn with_call_bound(self, bound: Duration) -> Agent {
        Agent { bound, ..self }
    }

    /// The same agent with another cap on what is kept of a call's output.
    pub fn with_output_cap(self, cap: usize) -> Agent {
        Agent { cap, ..self }
    }

    /// One `surf-agent` call, in a process group of its own; `after_call` runs when it has
    /// ended. The call is done when the process has exited and both its pipes have ended. A
    /// call not done at the bound, or whose output passes the cap, is sent SIGKILL, its whole
    /// group with it, and the reading stops: what was read is dropped.
    fn call(
        &self,
        words: &[&str],
        session: Option<&Session>,
        input: Option<&str>,
        after_call: &mut dyn FnMut(),
    ) -> Called {
        let mut command = Command::new(self.bin.join("surf-agent"));
        command
            .args(words)
            .env_clear()
            .env("PATH", OsString::from(self.bin.as_os_str()))
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
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
        let deadline = Instant::now() + self.bound;
        let (sender, pieces) = mpsc::channel();
        drain(child.stdout.take(), true, sender.clone());
        drain(child.stderr.take(), false, sender);
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let mut ends = 0;
        let mut status = None;
        let called = loop {
            if status.is_none() {
                status = child.try_wait().expect("surf-agent is waited for");
            }
            if let (Some(status), 2) = (status, ends) {
                break Called::Exited(Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            let now = Instant::now();
            if now >= deadline {
                break Called::TimedOut;
            }
            match pieces.recv_timeout(POLL.min(deadline - now)) {
                Ok(Piece::Bytes(out, bytes)) => {
                    if stdout.len() + stderr.len() + bytes.len() > self.cap {
                        break Called::TooLarge;
                    }
                    if out { &mut stdout } else { &mut stderr }.extend_from_slice(&bytes);
                }
                Ok(Piece::End) => ends += 1,
                Err(_) => {}
            }
        };
        if !matches!(called, Called::Exited(_)) {
            // The group may outlive its first process: a child that holds a pipe is in it.
            signal_kill_group(child.id());
            let _ = child.kill();
        }
        // Waited for on every path; after an exit `try_wait` saw, this returns at once.
        let _ = child.wait();
        after_call();
        called
    }

    /// One call of a step: counted, and its failure recorded unless an earlier one is. The
    /// output when it exited 0.
    fn step(
        &self,
        run: &mut Run,
        words: &[&str],
        session: Option<&Session>,
        input: Option<&str>,
        after_call: &mut dyn FnMut(),
    ) -> Option<Output> {
        run.calls += 1;
        let command = words[0].to_string();
        let failure = match self.call(words, session, input, after_call) {
            Called::Exited(output) if output.status.success() => return Some(output),
            Called::Exited(output) => StepFailure::Exit {
                status: output.status.code().unwrap_or(-1),
                command,
            },
            Called::TimedOut => StepFailure::Timeout {
                bound: self.bound,
                command,
            },
            Called::TooLarge => StepFailure::TooLarge {
                cap: self.cap,
                command,
            },
        };
        if run.failure.is_none() {
            run.failure = Some(failure);
        }
        None
    }

    /// Run one task's steps from `start`, the address of its start page. The connections of
    /// the returned [`Run`] are left at 0: the caller counts them at the fixture server.
    pub fn run(&self, steps: &[Step], start: &str, after_call: &mut dyn FnMut()) -> Run {
        self.run_with_ca_file(steps, start, None, after_call)
    }

    /// [`Agent::run`], its `open` step `open --ca-file <ca_file> <start>` when `ca_file` is
    /// given: the session trusts the roots of that file besides the bundled ones.
    pub fn run_with_ca_file(
        &self,
        steps: &[Step],
        start: &str,
        ca_file: Option<&str>,
        after_call: &mut dyn FnMut(),
    ) -> Run {
        let mut run = Run::default();
        let mut session: Option<Session> = None;
        let mut last_look: Option<Answer> = None;
        for step in steps {
            if run.failure.is_some() && *step != Step::Close {
                continue;
            }
            match step {
                Step::Open => {
                    let mut words = vec!["open"];
                    if let Some(file) = ca_file {
                        words.extend(["--ca-file", file]);
                    }
                    words.push(start);
                    let Some(output) = self.step(&mut run, &words, None, None, after_call) else {
                        continue;
                    };
                    let opened = String::from_utf8_lossy(&output.stdout).into_owned();
                    let parsed = surf_answer::parse(&opened);
                    run.opened = Some(opened);
                    match parsed.ok().and_then(|a| Some((a.head.session, a.token?))) {
                        Some((id, token)) => session = Some(Session { id, token }),
                        None => {
                            run.failure = Some(StepFailure::Exit {
                                status: 0,
                                command: "open".to_string(),
                            })
                        }
                    }
                }
                Step::Look => {
                    if let Some(output) =
                        self.step(&mut run, &["look"], session.as_ref(), None, after_call)
                    {
                        last_look =
                            surf_answer::parse(&String::from_utf8_lossy(&output.stdout)).ok();
                    }
                }
                Step::LookMarkdown => {
                    let mut from: Option<u64> = None;
                    for _ in 0..PAGES_MAX {
                        let from_text = from.map(|n| n.to_string());
                        let mut words = vec!["look", "--as", "markdown"];
                        if let Some(n) = &from_text {
                            words.extend(["--from-line", n.as_str()]);
                        }
                        let Some(output) =
                            self.step(&mut run, &words, session.as_ref(), None, after_call)
                        else {
                            break;
                        };
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
                    let words = [step.word(), handle.as_str()];
                    self.step(&mut run, &words, session.as_ref(), None, after_call);
                }
                Step::Fill(label, value) => {
                    let Some(handle) = now_handle(last_look.as_ref(), label) else {
                        run.failure = Some(StepFailure::NoNowLine {
                            label: label.to_string(),
                        });
                        continue;
                    };
                    let words = ["fill", handle.as_str(), "-"];
                    self.step(&mut run, &words, session.as_ref(), Some(value), after_call);
                }
                Step::Close => {
                    if session.is_some() {
                        self.step(&mut run, &["close"], session.as_ref(), None, after_call);
                    }
                }
            }
        }
        run
    }

    /// A session of its own on `address`: `open <address>` without `--ca-file`, then `look
    /// --as markdown`, then `close` whenever `open` gave a session. The standard output of
    /// `open` and of `look`, or the reason of the first of the two that did not go through
    /// (`exit <status> on open`, `open answered no session`, ...). What `close` does is not
    /// part of the result: a session it left is a process of the run that the lane finds.
    pub fn open_look_close(
        &self,
        address: &str,
        after_call: &mut dyn FnMut(),
    ) -> Result<(String, String), String> {
        let mut run = Run::default();
        let failed = |run: Run| run.failure.map_or_else(String::new, |f| f.reason());
        let Some(opened) = self.step(&mut run, &["open", address], None, None, after_call) else {
            return Err(failed(run));
        };
        let opened = String::from_utf8_lossy(&opened.stdout).into_owned();
        let session = surf_answer::parse(&opened)
            .ok()
            .and_then(|answer| Some((answer.head.session, answer.token?)));
        let Some((id, token)) = session else {
            return Err("open answered no session".to_string());
        };
        let session = Session { id, token };
        let words = ["look", "--as", "markdown"];
        let looked = self.step(&mut run, &words, Some(&session), None, after_call);
        let looked = match looked {
            Some(output) => Ok(String::from_utf8_lossy(&output.stdout).into_owned()),
            None => Err(failed(run)),
        };
        let mut closing = Run::default();
        self.step(&mut closing, &["close"], Some(&session), None, after_call);
        Ok((opened, looked?))
    }
}

/// Send SIGKILL to the process group `group` with `kill -9 -- -<group>`.
fn signal_kill_group(group: u32) {
    let _ = Command::new("kill")
        .args(["-9", "--", &format!("-{group}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
