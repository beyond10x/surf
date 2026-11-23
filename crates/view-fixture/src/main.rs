//! `surf-view-fixture`: what one page test of the live view needs (`story:view-first-page`),
//! test tooling only, not shipped.
//!
//! It makes a directory of its own under `$XDG_RUNTIME_DIR` (as `test_base()` of
//! `crates/agent/tests/common/base.rs` decides: never under the checkout, so that a session
//! socket path stays within 107 bytes), starts a page server on 127.0.0.1 that serves the files
//! of `crates/view-fixture/pages/`, and runs `surfd` in this process (`surfd::Daemon::start`, as
//! `surfd_in_process` of `crates/session/tests/common/mod.rs` does, decision D29) with the
//! `surf-session` built beside this executable and that directory as its `XDG_RUNTIME_DIR`. Then
//! it starts a pipe to the view's port (below) and writes one JSON line:
//!
//! ```text
//! {"view":"http://127.0.0.1:<port>/#<token>","piped":"http://127.0.0.1:<port>/#<token>","pages":"http://127.0.0.1:<port>","unreachable":"http://127.0.0.1:<port>/"}
//! ```
//!
//! `view` is `surfd`'s own address, `piped` the same view and token through the pipe, and
//! `unreachable` a port this fixture bound and released.
//!
//! and reads one command per line on standard input, the words separated by a tab, each
//! answered with one JSON line:
//!
//! - `open` or `open<TAB><url>`: `surf-agent open`, answered `{"id":"<id>"}`; the session's token
//!   stays here;
//! - `agent<TAB><id><TAB><word>…`: `surf-agent <word>…` as that session, answered
//!   `{"status":<exit status>}`;
//! - `agent-stdin<TAB><id><TAB><input><TAB><word>…`: the same with `<input>` on the standard
//!   input of `surf-agent` (`fill <handle> -`), so that the value is in no command line;
//! - `kill<TAB><id>`: kills the process of that session, answered `{}`; only for a session this
//!   fixture opened, and only a `surf-session` whose environment holds this fixture's
//!   `XDG_RUNTIME_DIR`, so no other process of the machine is ever killed; any other id is
//!   answered `{"error":"session"}`;
//! - `stop-surfd`: drops the daemon, answered `{}`; every later `open` or `agent` is answered
//!   `{"error":"stopped"}`, so no `surf-agent` starts a `surfd` of its own;
//! - `cut`: shuts down both sides of every connection open through the pipe, closes every
//!   connection it accepts from then on without reading it, and answers `{}` once the open ones
//!   are shut;
//! - `mend`: the pipe passes connections again, answered `{}`;
//! - anything else, a line that is not UTF-8 too: `{"error":"command"}`; only the end of
//!   standard input or a read error ends the loop.
//!
//! The pipe passes each connection's bytes to the view's port and back, with the one `Host`
//! line of the request head written as `Host: 127.0.0.1:<the view's port>`; the view answers
//! one request per connection, so only the first head is read, up to 64 KiB.
//!
//! The end of standard input stops the daemon (which kills its session processes) and the page
//! server, and removes the directory.
//!
//! Options: `--entries-per-session <n>` and `--bytes-total <n>` set those fields of
//! `surfd::log::Bounds`.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use surf_test_process::ended_within;
use surf_view::to_json;

/// The most bytes of a request head the page server reads: 64 KiB.
const HEAD_MAX: usize = 64 * 1024;

/// How long the page server waits for a request head.
const HEAD_TIMEOUT: Duration = Duration::from_secs(5);

/// `surfd`'s idle exit here: one hour, longer than any page test.
const IDLE_EXIT: Duration = Duration::from_secs(3600);

/// The bytes a session socket path adds to `XDG_RUNTIME_DIR`: `/surf/` 6, the identity 36, `/`
/// 1, `session.sock` 12.
const SESSION_SOCKET_ADDS: usize = 55;

/// The most bytes of a Unix socket path.
const SOCKET_PATH_MAX: usize = 107;

/// The answer to a command line this fixture does not know, a line that is not UTF-8 too.
const UNKNOWN: &str = "{\"error\":\"command\"}";

/// How long `kill` waits for the session's process to end.
const KILL_WAIT: Duration = Duration::from_secs(10);

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("surf-view-fixture: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let bounds = options(std::env::args().skip(1))?;
    let exe = std::env::current_exe().map_err(|error| format!("own executable: {error}"))?;
    let bin = exe
        .parent()
        .ok_or("the directory of this executable is not known")?
        .to_path_buf();
    for program in ["surf-agent", "surf-session"] {
        if !bin.join(program).is_file() {
            return Err(format!(
                "{} is missing; build it with cargo build -p {program}",
                bin.join(program).display()
            ));
        }
    }
    let dir = Directory::new(&bin)?;
    let result = serve(&dir, &bin, bounds);
    kill_left(&dir.runtime());
    drop(dir);
    result
}

/// The bounds of the log: the shipped ones, with the two options applied.
fn options(mut args: impl Iterator<Item = String>) -> Result<surfd::log::Bounds, String> {
    let mut bounds = surfd::log::Bounds::default();
    while let Some(arg) = args.next() {
        let field = match arg.as_str() {
            "--entries-per-session" => &mut bounds.entries_per_session,
            "--bytes-total" => &mut bounds.bytes_total,
            other => return Err(format!("unknown argument {other}")),
        };
        *field = args
            .next()
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| format!("{arg} takes a number"))?;
    }
    Ok(bounds)
}

/// The page server, `surfd` and the command loop, until standard input ends.
fn serve(dir: &Directory, bin: &Path, bounds: surfd::log::Bounds) -> Result<(), String> {
    let pages = Pages::start(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("pages"))
        .map_err(|error| format!("the page server: {error}"))?;
    let unreachable = {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("a port to release: {error}"))?;
        listener
            .local_addr()
            .map_err(|error| format!("a port to release: {error}"))?
            .port()
    };
    let runtime = dir.runtime();
    let view_line = Arc::new(Mutex::new(Vec::new()));
    let mut config = surfd::Config::new(Some(runtime.clone()));
    config.session_program = bin.join("surf-session");
    config.session_env = vec![("XDG_RUNTIME_DIR".into(), runtime.clone().into())];
    config.idle_exit = IDLE_EXIT;
    config.log = bounds;
    config.view_line = Some(Box::new(Shared(Arc::clone(&view_line))));
    let mut daemon = match surfd::Daemon::start(config) {
        Ok(surfd::Started::Daemon(daemon)) => Some(daemon),
        Ok(surfd::Started::Another) => return Err("another surfd holds the lock".into()),
        Err(refused) => return Err(format!("surfd refused: {refused}")),
    };
    let view = String::from_utf8_lossy(&guard(&view_line))
        .trim_end()
        .to_string();
    let (view_port, token) = view
        .strip_prefix("http://127.0.0.1:")
        .and_then(|rest| rest.split_once("/#"))
        .and_then(|(port, token)| Some((port.parse::<u16>().ok()?, token.to_string())))
        .ok_or("surfd wrote no view line")?;
    let pipe = Pipe::start(view_port).map_err(|error| format!("the pipe: {error}"))?;
    let piped = format!("http://127.0.0.1:{}/#{token}", pipe.port);

    let mut line = String::from("{\"view\":");
    line.push_str(&json_text(&view));
    line.push_str(",\"piped\":");
    line.push_str(&json_text(&piped));
    line.push_str(",\"pages\":");
    line.push_str(&json_text(&format!("http://127.0.0.1:{}", pages.port)));
    line.push_str(",\"unreachable\":");
    line.push_str(&json_text(&format!("http://127.0.0.1:{unreachable}/")));
    line.push('}');
    say(&line);

    let mut agent = Agent {
        program: bin.join("surf-agent"),
        runtime,
        tokens: HashMap::new(),
    };
    let mut input = io::stdin().lock();
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        match input.read_until(b'\n', &mut bytes) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
        }
        // A line that is not UTF-8 is a command like any other this loop does not know.
        let Ok(command) = std::str::from_utf8(&bytes) else {
            say(UNKNOWN);
            continue;
        };
        let words: Vec<&str> = command.split('\t').collect();
        let answer = match words.as_slice() {
            ["open"] | ["open", _] if daemon.is_none() => stopped(),
            ["agent", _, ..] if daemon.is_none() => stopped(),
            ["open"] => agent.open(None),
            ["open", url] => agent.open(Some(url)),
            ["agent-stdin", _, ..] if daemon.is_none() => stopped(),
            ["agent", id, rest @ ..] if !rest.is_empty() => agent.run(id, None, rest),
            ["agent-stdin", id, input, rest @ ..] if !rest.is_empty() => {
                agent.run(id, Some(input), rest)
            }
            ["kill", id] => agent.kill(id),
            ["stop-surfd"] => {
                drop(daemon.take());
                "{}".to_string()
            }
            ["cut"] => {
                pipe.cut();
                "{}".to_string()
            }
            ["mend"] => {
                pipe.mend();
                "{}".to_string()
            }
            _ => UNKNOWN.to_string(),
        };
        say(&answer);
    }
    drop(pipe);
    drop(daemon);
    drop(pages);
    Ok(())
}

fn stopped() -> String {
    "{\"error\":\"stopped\"}".to_string()
}

/// One line on standard output, flushed.
fn say(line: &str) {
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

/// `value` as a JSON string, written by `surf_view::to_json`.
fn json_text(value: &str) -> String {
    String::from_utf8_lossy(&to_json(value)).into_owned()
}

fn guard<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A writer into shared bytes: where `surfd` writes the view line.
struct Shared(Arc<Mutex<Vec<u8>>>);

impl Write for Shared {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        guard(&self.0).extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------------------------
// The directory.

/// This fixture's directory, `svf-<process id>-<8 hexadecimal digits>` under the base; `r` in it
/// is the `XDG_RUNTIME_DIR` of `surfd` and every `surf-agent`. Removed when dropped.
struct Directory {
    path: PathBuf,
}

impl Directory {
    fn new(bin: &Path) -> Result<Directory, String> {
        let base = base(bin)?;
        let mut attempt = 0u32;
        loop {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.subsec_nanos());
            let name = format!(
                "svf-{}-{:08x}",
                std::process::id(),
                nanos.wrapping_add(attempt)
            );
            let path = base.join(name);
            let length = path.join("r").as_os_str().len() + SESSION_SOCKET_ADDS;
            if length > SOCKET_PATH_MAX {
                return Err(format!(
                    "a session socket under {} is {length} bytes, over {SOCKET_PATH_MAX}; set \
                     XDG_RUNTIME_DIR to a short directory of yours",
                    base.display()
                ));
            }
            match fs::create_dir(&path) {
                Ok(()) => {
                    let dir = Directory { path };
                    let mode = fs::Permissions::from_mode(0o700);
                    fs::set_permissions(&dir.path, mode.clone())
                        .and_then(|()| fs::create_dir(dir.runtime()))
                        .and_then(|()| fs::set_permissions(dir.runtime(), mode))
                        .map_err(|error| format!("{}: {error}", dir.path.display()))?;
                    return Ok(dir);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists && attempt < 100 => {
                    attempt += 1;
                }
                Err(error) => return Err(format!("{}: {error}", path.display())),
            }
        }
    }

    fn runtime(&self) -> PathBuf {
        self.path.join("r")
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Where the directory goes, as `test_base()` decides: `XDG_RUNTIME_DIR` when it is set,
/// absolute, and a directory owned by this user; otherwise `tmp` beside the profile directory
/// of this executable, `CARGO_TARGET_TMPDIR` of a test. Never `/tmp`.
fn base(bin: &Path) -> Result<PathBuf, String> {
    let caller = fs::metadata("/proc/self")
        .map(|meta| meta.uid())
        .map_err(|error| format!("the user id from /proc/self: {error}"))?;
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .filter(|dir| fs::symlink_metadata(dir).is_ok_and(|m| m.is_dir() && m.uid() == caller));
    if let Some(dir) = runtime {
        return Ok(dir);
    }
    let tmp = bin
        .parent()
        .ok_or("no directory above the profile directory")?
        .join("tmp");
    fs::create_dir_all(&tmp).map_err(|error| format!("{}: {error}", tmp.display()))?;
    Ok(tmp)
}

// ---------------------------------------------------------------------------------------------
// The sessions.

struct Agent {
    program: PathBuf,
    runtime: PathBuf,
    /// The token of each session this fixture opened.
    tokens: HashMap<String, String>,
}

impl Agent {
    /// `surf-agent` with nothing in its environment but the fixture's `XDG_RUNTIME_DIR`.
    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command
            .env_clear()
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .stdin(Stdio::null())
            .stderr(Stdio::piped());
        command
    }

    fn open(&mut self, url: Option<&str>) -> String {
        let mut command = self.command();
        command.arg("open");
        if let Some(url) = url {
            command.arg(url);
        }
        let Ok(output) = command.output() else {
            return "{\"error\":\"open\"}".to_string();
        };
        let text = String::from_utf8_lossy(&output.stdout);
        let id = text
            .lines()
            .next()
            .and_then(|head| head.strip_prefix("surf 1 session "))
            .and_then(|rest| rest.strip_suffix(" open"));
        let token = text.lines().find_map(|line| line.strip_prefix("token "));
        match (output.status.success(), id, token) {
            (true, Some(id), Some(token)) => {
                self.tokens.insert(id.to_string(), token.to_string());
                format!("{{\"id\":{}}}", json_text(id))
            }
            _ => {
                eprintln!(
                    "surf-view-fixture: open failed ({}): {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr).trim_end()
                );
                "{\"error\":\"open\"}".to_string()
            }
        }
    }

    /// `surf-agent <words>…` as session `id`, with `input` on its standard input when given.
    fn run(&self, id: &str, input: Option<&str>, words: &[&str]) -> String {
        let Some(token) = self.tokens.get(id) else {
            return "{\"error\":\"session\"}".to_string();
        };
        let mut command = self.command();
        command
            .env("SURF_SESSION", id)
            .env("SURF_TOKEN", token)
            .args(words)
            .stdout(Stdio::piped());
        if input.is_some() {
            command.stdin(Stdio::piped());
        }
        let output = command.spawn().and_then(|mut child| {
            if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
                stdin.write_all(input.as_bytes())?;
            }
            child.wait_with_output()
        });
        match output {
            Ok(output) => {
                let status = output.status.code().unwrap_or(-1);
                format!("{{\"status\":{status}}}")
            }
            Err(_) => "{\"error\":\"agent\"}".to_string(),
        }
    }
}

impl Agent {
    /// Kill the process of session `id`, which this fixture opened, and wait until it has
    /// ended. A candidate under `/proc` is `surf-session` with `id` among its arguments and
    /// this fixture's `XDG_RUNTIME_DIR=<runtime>` in its environment, so no process of
    /// another `surfd` on the machine is ever a candidate. Any other id searches nothing.
    fn kill(&self, id: &str) -> String {
        if !self.tokens.contains_key(id) {
            return "{\"error\":\"session\"}".to_string();
        }
        kill_session(id, &self.runtime)
    }
}

fn kill_session(id: &str, runtime: &Path) -> String {
    let pids: Vec<u32> = processes()
        .into_iter()
        .filter(|&pid| {
            let args = cmdline(pid);
            args.first()
                .is_some_and(|program| program.ends_with(b"surf-session"))
                && args.iter().any(|arg| arg == id.as_bytes())
                && in_runtime(pid, runtime)
        })
        .collect();
    if pids.is_empty() {
        return "{\"error\":\"session\"}".to_string();
    }
    for &pid in &pids {
        signal_kill(pid);
    }
    let deadline = Instant::now() + KILL_WAIT;
    for pid in pids {
        let _ = ended_within(pid, deadline.saturating_duration_since(Instant::now()));
    }
    "{}".to_string()
}

/// Whether the environment of `pid` holds `XDG_RUNTIME_DIR=<runtime>`: a process of this
/// fixture's `surfd` or `surf-agent`.
fn in_runtime(pid: u32, runtime: &Path) -> bool {
    let mut needle = b"XDG_RUNTIME_DIR=".to_vec();
    needle.extend_from_slice(runtime.as_os_str().as_encoded_bytes());
    fs::read(format!("/proc/{pid}/environ"))
        .unwrap_or_default()
        .split(|&b| b == 0)
        .any(|pair| pair == needle.as_slice())
}

/// Kill every process left whose environment holds `XDG_RUNTIME_DIR=<runtime>`.
fn kill_left(runtime: &Path) {
    let own = std::process::id();
    let left: Vec<u32> = processes()
        .into_iter()
        .filter(|&pid| pid != own)
        .filter(|&pid| in_runtime(pid, runtime))
        .collect();
    for &pid in &left {
        signal_kill(pid);
    }
    let deadline = Instant::now() + KILL_WAIT;
    for pid in left {
        let _ = ended_within(pid, deadline.saturating_duration_since(Instant::now()));
    }
}

fn processes() -> Vec<u32> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| entry.file_name().to_str()?.parse().ok())
        .collect()
}

fn cmdline(pid: u32) -> Vec<Vec<u8>> {
    fs::read(format!("/proc/{pid}/cmdline"))
        .unwrap_or_default()
        .split(|&b| b == 0)
        .filter(|part| !part.is_empty())
        .map(<[u8]>::to_vec)
        .collect()
}

fn signal_kill(pid: u32) {
    let _ = Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

// ---------------------------------------------------------------------------------------------
// The page server.

/// An HTTP server on 127.0.0.1 port 0 over the files under a directory, by path, the query
/// ignored: `text/html` for `.html`, 404 for anything else. Stopped when dropped.
struct Pages {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Pages {
    fn start(root: PathBuf) -> io::Result<Pages> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let thread = thread::Builder::new().name("pages".into()).spawn(move || {
            for stream in listener.incoming() {
                if stopping.load(Ordering::SeqCst) {
                    return;
                }
                let Ok(stream) = stream else { continue };
                let root = root.clone();
                let _ = thread::Builder::new()
                    .name("page".into())
                    .spawn(move || answer_page(stream, &root));
            }
        })?;
        Ok(Pages {
            port,
            stop,
            thread: Some(thread),
        })
    }
}

impl Drop for Pages {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// One request: its head, up to [`HEAD_MAX`] bytes, then one answer, then the close.
fn answer_page(mut stream: TcpStream, root: &Path) {
    let _ = stream.set_read_timeout(Some(HEAD_TIMEOUT));
    let _ = stream.set_write_timeout(Some(HEAD_TIMEOUT));
    let mut head = Vec::new();
    let mut chunk = [0u8; 4096];
    let complete = loop {
        if head.windows(4).any(|w| w == b"\r\n\r\n") {
            break true;
        }
        if head.len() >= HEAD_MAX {
            break false;
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break false,
            Ok(n) => head.extend_from_slice(&chunk[..n]),
        }
    };
    if complete {
        read_body(&mut stream, &head);
    }
    let file = if complete { page(&head, root) } else { None };
    let answer = match file {
        Some(body) => {
            let mut answer = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .into_bytes();
            answer.extend_from_slice(&body);
            answer
        }
        None => b"HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 10\r\nConnection: close\r\n\r\nnot found\n"
            .to_vec(),
    };
    let _ = stream.write_all(&answer);
    let _ = stream.flush();
}

/// Reads and drops the body a request's `Content-Length` announces, up to [`HEAD_MAX`] bytes
/// after the head, so that closing the connection does not reset it before the answer is read.
fn read_body(stream: &mut TcpStream, head: &[u8]) {
    let Some(end) = head.windows(4).position(|w| w == b"\r\n\r\n") else {
        return;
    };
    let length = std::str::from_utf8(&head[..end])
        .ok()
        .and_then(|text| {
            text.split("\r\n").find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
        })
        .unwrap_or(0)
        .min(HEAD_MAX);
    let mut left = length.saturating_sub(head.len() - (end + 4));
    let mut chunk = [0u8; 4096];
    while left > 0 {
        match stream.read(&mut chunk[..left.min(4096)]) {
            Ok(0) | Err(_) => return,
            Ok(n) => left -= n,
        }
    }
}

/// The bytes of the `.html` file a `GET` or a `POST` of `head` names under `root`, the query
/// and a body ignored. A `POST` is answered as the `GET` of its target, so that a form's
/// submission reaches a page (`pages/forms/`).
fn page(head: &[u8], root: &Path) -> Option<Vec<u8>> {
    let line = head.split(|&b| b == b'\r').next()?;
    let line = std::str::from_utf8(line).ok()?;
    let mut parts = line.split(' ');
    let (method, target) = (parts.next()?, parts.next()?);
    if method != "GET" && method != "POST" {
        return None;
    }
    let path = target.split('?').next()?.strip_prefix('/')?;
    let mut file = root.to_path_buf();
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return None;
        }
        file.push(segment);
    }
    if file.extension().and_then(|ext| ext.to_str()) != Some("html") {
        return None;
    }
    fs::read(file).ok()
}

// ---------------------------------------------------------------------------------------------
// The pipe.

/// A listener on 127.0.0.1 port 0 that passes each connection to the view's port and back,
/// with the one `Host` line of the request head written as `Host: 127.0.0.1:<the view's port>`
/// (the view answers 403 to any other). The view answers one request per connection, so only
/// the first head of a connection is read, up to [`HEAD_MAX`] bytes. [`Pipe::cut`] takes every
/// connection away; [`Pipe::mend`] passes them again. Stopped when dropped.
struct Pipe {
    port: u16,
    state: Arc<Mutex<PipeState>>,
    thread: Option<JoinHandle<()>>,
}

#[derive(Default)]
struct PipeState {
    cut: bool,
    stopped: bool,
    next: u64,
    /// The sockets of each open connection, both sides, by the connection's number.
    open: HashMap<u64, Vec<TcpStream>>,
}

impl Pipe {
    fn start(view_port: u16) -> io::Result<Pipe> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let state = Arc::new(Mutex::new(PipeState::default()));
        let shared = Arc::clone(&state);
        let thread = thread::Builder::new().name("pipe".into()).spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let number = {
                    let mut state = guard(&shared);
                    if state.stopped {
                        return;
                    }
                    if state.cut {
                        // Closed without being read.
                        continue;
                    }
                    let Ok(clone) = stream.try_clone() else {
                        continue;
                    };
                    state.next += 1;
                    let number = state.next;
                    state.open.insert(number, vec![clone]);
                    number
                };
                let state = Arc::clone(&shared);
                let _ = thread::Builder::new().name("piped".into()).spawn(move || {
                    pass(&state, number, stream, view_port);
                    guard(&state).open.remove(&number);
                });
            }
        })?;
        Ok(Pipe {
            port,
            state,
            thread: Some(thread),
        })
    }

    /// Shut both sides of every open connection; refuse every new one until [`Pipe::mend`].
    fn cut(&self) {
        let mut state = guard(&self.state);
        state.cut = true;
        for streams in state.open.values() {
            for stream in streams {
                let _ = stream.shutdown(Shutdown::Both);
            }
        }
    }

    fn mend(&self) {
        guard(&self.state).cut = false;
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        {
            let mut state = guard(&self.state);
            state.stopped = true;
            for streams in state.open.values() {
                for stream in streams {
                    let _ = stream.shutdown(Shutdown::Both);
                }
            }
        }
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// One connection through the pipe: its head read and its `Host` line rewritten, then the
/// bytes passed both ways until either side ends.
fn pass(state: &Mutex<PipeState>, number: u64, mut client: TcpStream, view_port: u16) {
    let _ = client.set_read_timeout(Some(HEAD_TIMEOUT));
    let mut head = Vec::new();
    let mut chunk = [0u8; 4096];
    let end = loop {
        if let Some(at) = head.windows(4).position(|w| w == b"\r\n\r\n") {
            break at + 4;
        }
        if head.len() >= HEAD_MAX {
            return;
        }
        match client.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => head.extend_from_slice(&chunk[..n]),
        }
    };
    let _ = client.set_read_timeout(None);
    let mut rewritten = Vec::with_capacity(head.len());
    let mut host_done = false;
    for line in head[..end - 2].split_inclusive(|&b| b == b'\n') {
        let name = line.split(|&b| b == b':').next().unwrap_or_default();
        if !host_done && line.contains(&b':') && name.eq_ignore_ascii_case(b"host") {
            rewritten.extend_from_slice(format!("Host: 127.0.0.1:{view_port}\r\n").as_bytes());
            host_done = true;
        } else {
            rewritten.extend_from_slice(line);
        }
    }
    rewritten.extend_from_slice(b"\r\n");
    rewritten.extend_from_slice(&head[end..]);
    let Ok(mut upstream) = TcpStream::connect(("127.0.0.1", view_port)) else {
        return;
    };
    {
        let mut state = guard(state);
        let Some(streams) = state.open.get_mut(&number) else {
            return;
        };
        let Ok(clone) = upstream.try_clone() else {
            return;
        };
        streams.push(clone);
        if state.cut || state.stopped {
            let _ = client.shutdown(Shutdown::Both);
            let _ = upstream.shutdown(Shutdown::Both);
            return;
        }
    }
    if upstream.write_all(&rewritten).is_err() {
        return;
    }
    let (Ok(mut back_from), Ok(mut back_to)) = (upstream.try_clone(), client.try_clone()) else {
        return;
    };
    let back = thread::Builder::new()
        .name("piped-back".into())
        .spawn(move || {
            let _ = io::copy(&mut back_from, &mut back_to);
            let _ = back_to.shutdown(Shutdown::Both);
            let _ = back_from.shutdown(Shutdown::Both);
        });
    let _ = io::copy(&mut client, &mut upstream);
    let _ = upstream.shutdown(Shutdown::Write);
    if let Ok(back) = back {
        let _ = back.join();
    }
}
