//! What the tests of `surf-session` share: a directory per case under [`test_base`] (the file
//! the tests of `surf-agent` and `surfd` use, included, not copied), the built programs, a page
//! server on `127.0.0.1`, the golden texts, and guards that stop every process a case started,
//! also when it panics.

#![allow(dead_code)]

#[path = "../../../agent/tests/common/base.rs"]
mod base;

pub use base::test_base;
use surf_test_process::ended_within;

use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Output, Stdio};
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use surf_agent::wire::{self, Arg, Request};
use surf_secret::Token;
use surfd::link;

/// The bytes a session socket path adds to `XDG_RUNTIME_DIR`: `/surf/` 6, the identity 36, `/`
/// 1, `session.sock` 12.
pub const SESSION_SOCKET_ADDS: usize = 55;

/// The most bytes of a Unix socket path, as the story states it.
pub const SOCKET_PATH_MAX: usize = 107;

// --- The case's directory ---------------------------------------------------------------------

/// A directory of one case, removed when it is dropped, after every process of the case is
/// stopped: `r` is the `XDG_RUNTIME_DIR` of the case.
///
/// The processes of the case are those whose environment holds `XDG_RUNTIME_DIR=<r>`. A helper
/// that starts a built program (`agent`, `Started::start`) marks the directory, and its drop
/// looks for them among every process of the host ([`TestDir::processes`]): such a program can
/// leave the test's tree (the `surfd` a `surf-agent` starts is reparented when it exits). An
/// unmarked directory's drop looks only among the descendants of the test process, where an
/// in-process daemon's sessions are.
pub struct TestDir {
    pub path: PathBuf,
    /// Set by a helper that started a built program in this case.
    pub started_a_built_program: AtomicBool,
}

impl TestDir {
    /// A new directory under [`test_base`], `sst-<process id>-<counter>-<8 random digits>`.
    pub fn new() -> TestDir {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let mut random = [0u8; 4];
        surf_secret::fill_random(&mut random).expect("the random source");
        let name = format!(
            "sst-{}-{}-{:08x}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst),
            u32::from_be_bytes(random)
        );
        let path = test_base().join(name);
        let length = path.join("r").as_os_str().len() + SESSION_SOCKET_ADDS;
        assert!(
            length <= SOCKET_PATH_MAX,
            "the test's session socket path is {length} bytes, over {SOCKET_PATH_MAX}: the \
             base directory is too long; set XDG_RUNTIME_DIR to a short directory of yours"
        );
        fs::create_dir(&path).expect("a new test directory");
        let dir = TestDir {
            path,
            started_a_built_program: AtomicBool::new(false),
        };
        fs::set_permissions(&dir.path, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(dir.runtime()).expect("the runtime directory");
        fs::set_permissions(dir.runtime(), fs::Permissions::from_mode(0o700)).unwrap();
        dir
    }

    /// The case's `XDG_RUNTIME_DIR`.
    pub fn runtime(&self) -> PathBuf {
        self.path.join("r")
    }

    /// `$XDG_RUNTIME_DIR/surf` of the case.
    pub fn surf(&self) -> PathBuf {
        self.runtime().join("surf")
    }

    /// The session socket of `id`, its directory made as `surfd` makes it (mode 0700).
    pub fn session_socket(&self, id: &str) -> PathBuf {
        let dir = self.surf().join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::set_permissions(self.surf(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        dir.join("session.sock")
    }

    /// Marks the case as one that started a built program: its drop reads every process.
    pub fn mark_built_program(&self) {
        self.started_a_built_program.store(true, Ordering::SeqCst);
    }

    /// The process ids of the host whose environment holds `XDG_RUNTIME_DIR=<this case's
    /// runtime>`.
    pub fn processes(&self) -> Vec<u32> {
        let Ok(entries) = fs::read_dir("/proc") else {
            return Vec::new();
        };
        self.processes_among(
            entries
                .flatten()
                .filter_map(|entry| entry.file_name().to_str()?.parse::<u32>().ok()),
        )
    }

    /// The processes of `candidates`, this one left out, whose environment holds
    /// `XDG_RUNTIME_DIR=<this case's runtime>`.
    fn processes_among(&self, candidates: impl IntoIterator<Item = u32>) -> Vec<u32> {
        let mut needle = b"XDG_RUNTIME_DIR=".to_vec();
        needle.extend_from_slice(self.runtime().as_os_str().as_encoded_bytes());
        let own = std::process::id();
        candidates
            .into_iter()
            .filter(|&pid| pid != own)
            .filter(|pid| {
                fs::read(format!("/proc/{pid}/environ"))
                    .unwrap_or_default()
                    .split(|&b| b == 0)
                    .any(|pair| pair == needle.as_slice())
            })
            .collect()
    }
}

/// Every descendant of this process: the process ids in `/proc/<pid>/task/<tid>/children` of
/// each of its threads, followed down. `None` when one of those files cannot be read (a process
/// or thread ended while it was read, so that a child of it may have left the tree).
pub fn descendants() -> Option<Vec<u32>> {
    let mut found: Vec<u32> = Vec::new();
    let mut pending = vec![std::process::id()];
    while let Some(pid) = pending.pop() {
        for task in fs::read_dir(format!("/proc/{pid}/task")).ok()? {
            let children = fs::read_to_string(task.ok()?.path().join("children")).ok()?;
            for child in children.split_whitespace() {
                let child: u32 = child.parse().ok()?;
                if !found.contains(&child) {
                    found.push(child);
                    pending.push(child);
                }
            }
        }
    }
    Some(found)
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let walked = if self.started_a_built_program.load(Ordering::SeqCst) {
            None
        } else {
            descendants()
        };
        let pids = match walked {
            Some(descendants) => self.processes_among(descendants),
            None => self.processes(),
        };
        for &pid in &pids {
            signal_kill(pid);
        }
        // 10 s for all of them: a cleanup; SIGKILL ends each within milliseconds.
        let deadline = Instant::now() + Duration::from_secs(10);
        for pid in pids {
            let _ = ended_within(pid, deadline.saturating_duration_since(Instant::now()));
        }
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn signal_kill(pid: u32) {
    let _ = Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .stderr(Stdio::null())
        .status();
}

/// The arguments of process `pid`, split at their zero bytes.
pub fn cmdline(pid: u32) -> Vec<Vec<u8>> {
    fs::read(format!("/proc/{pid}/cmdline"))
        .unwrap_or_default()
        .split(|&b| b == 0)
        .filter(|part| !part.is_empty())
        .map(<[u8]>::to_vec)
        .collect()
}

/// Whether `haystack` holds `needle`.
pub fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// Every regular file under `path`, recursively, without following a symbolic link.
pub fn regular_files_under(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![path.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                files.push(entry.path());
            }
        }
    }
    files
}

/// Every entry under `path` that is not a directory, recursively.
pub fn entries_under(path: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![path.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => pending.push(entry.path()),
                _ => found.push(entry.path()),
            }
        }
    }
    found
}

/// A version 4 UUID in lower case from the random source.
pub fn new_identity() -> String {
    let mut bytes = [0u8; 16];
    surf_secret::fill_random(&mut bytes).unwrap();
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

// --- The built programs -----------------------------------------------------------------------

/// The directory of `surf-agent`, `surfd` and `surf-session`: `SURF_BIN_DIR` when it is set
/// (nothing is built), otherwise `target/<profile>/` of this test, where `cargo test -p
/// surf-session` put `surf-session` and where this builds `surfd` and `surf-agent` once per
/// test program, with the cargo running these tests (it does nothing when they are fresh).
pub fn bin_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        if let Some(dir) = std::env::var_os("SURF_BIN_DIR") {
            return PathBuf::from(dir);
        }
        let exe = std::env::current_exe().expect("the test's executable");
        let profile_dir = exe
            .parent()
            .and_then(Path::parent)
            .expect("target/<profile>")
            .to_path_buf();
        let profile = match profile_dir.file_name().and_then(|name| name.to_str()) {
            Some("debug") => "dev".to_string(),
            Some(other) => other.to_string(),
            None => panic!("no profile directory above {}", exe.display()),
        };
        let status = Command::new(env!("CARGO"))
            .args([
                "build",
                "--locked",
                "--quiet",
                "-p",
                "surfd",
                "-p",
                "surf-agent",
                "--profile",
                &profile,
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .stdin(Stdio::null())
            .status()
            .expect("cargo runs");
        assert!(status.success(), "building surfd and surf-agent: {status}");
        profile_dir
    })
}

/// The `surf-session` program: from `SURF_BIN_DIR` when it is set, otherwise the one `cargo
/// test` built.
pub fn session_program() -> PathBuf {
    match std::env::var_os("SURF_BIN_DIR") {
        Some(dir) => PathBuf::from(dir).join("surf-session"),
        None => PathBuf::from(env!("CARGO_BIN_EXE_surf-session")),
    }
}

/// `surf-agent` in the case's environment: nothing but its `XDG_RUNTIME_DIR`. Marks the
/// directory.
pub fn agent(dir: &TestDir) -> Command {
    dir.mark_built_program();
    let mut command = Command::new(bin_dir().join("surf-agent"));
    command
        .env_clear()
        .env("XDG_RUNTIME_DIR", dir.runtime())
        .stdin(Stdio::null());
    command
}

pub fn run(command: &mut Command) -> Output {
    command.output().expect("the program runs")
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A session opened by `surf-agent open`: its identity, its token and the answer.
pub struct Opened {
    pub id: String,
    pub token: String,
    pub text: String,
}

/// The identity of the head line and the token of an `open` answer.
pub fn opened(text: &str) -> Opened {
    let head = text.lines().next().expect("a head line");
    let id = head
        .strip_prefix("surf 1 session ")
        .and_then(|rest| rest.strip_suffix(" open"))
        .unwrap_or_else(|| panic!("not the head line of an open session: {head}"))
        .to_string();
    let token = text
        .lines()
        .find_map(|line| line.strip_prefix("token "))
        .expect("a token line")
        .to_string();
    Opened {
        id,
        token,
        text: text.to_string(),
    }
}

/// `surf-agent open [<url>]`, which must exit 0.
pub fn agent_open(dir: &TestDir, url: Option<&str>) -> Opened {
    let mut command = agent(dir);
    command.arg("open");
    if let Some(url) = url {
        command.arg(url);
    }
    let output = run(&mut command);
    assert_eq!(output.status.code(), Some(0), "open: {}", stderr(&output));
    opened(&stdout(&output))
}

/// `surf-agent <words>` with `SURF_SESSION` and `SURF_TOKEN` set.
pub fn agent_call(dir: &TestDir, words: &[&str], id: &str, token: &str) -> Output {
    run(agent(dir)
        .args(words)
        .env("SURF_SESSION", id)
        .env("SURF_TOKEN", token))
}

// --- A session started by the test ------------------------------------------------------------

/// The built `surf-session`, started by the test as `surfd` starts it: its identity, its socket,
/// standard input a pipe the test holds, standard output a pipe the test reads; killed and waited
/// for when dropped.
pub struct Started {
    pub id: String,
    pub socket: PathBuf,
    pub child: Child,
    pub stdin: Option<ChildStdin>,
    pub stdout: Option<BufReader<ChildStdout>>,
    /// The first line it wrote, its line feed included.
    pub line: String,
}

impl Started {
    /// Start `surf-session [options] <identity> <socket>` in `dir` and read its first line.
    /// Marks the directory.
    pub fn start(dir: &TestDir, options: &[&str]) -> Started {
        dir.mark_built_program();
        let id = new_identity();
        let socket = dir.session_socket(&id);
        let mut child = Command::new(session_program())
            .args(options)
            .arg(&id)
            .arg(&socket)
            .env_clear()
            .env("XDG_RUNTIME_DIR", dir.runtime())
            .current_dir("/")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("surf-session starts");
        let stdin = child.stdin.take();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        Started {
            id,
            socket,
            child,
            stdin,
            stdout: Some(stdout),
            line,
        }
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// One request over one connection to the session socket, and the link answer, or the
    /// error of reading it.
    pub fn call_raw(&self, bytes: &[u8]) -> Result<link::Answer, link::LinkError> {
        let mut stream = UnixStream::connect(&self.socket).expect("the session listens");
        let _ = stream.write_all(bytes);
        let _ = stream.shutdown(std::net::Shutdown::Write);
        link::read_answer(&mut stream, Instant::now() + Duration::from_secs(60))
    }

    pub fn call(&self, request: &Request) -> link::Answer {
        self.call_raw(&request.encode().unwrap())
            .expect("a link answer")
    }

    /// `open` over the link, which must answer status 0: the token it printed.
    pub fn open(&self) -> String {
        let answer = self.call(&request("open", None, None, &[]));
        assert_eq!(answer.status, 0, "open: {answer:?}");
        opened(&answer.text).token
    }

    /// Wait on the process, at most `within`: its exit status if it ended.
    pub fn wait_within(&mut self, within: Duration) -> Option<ExitStatus> {
        let deadline = Instant::now() + within;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return Some(status);
            }
            if Instant::now() >= deadline {
                return None;
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    /// What is left on its standard output, read to the end.
    pub fn rest_of_stdout(&mut self) -> String {
        let mut rest = String::new();
        if let Some(mut stdout) = self.stdout.take() {
            stdout.read_to_string(&mut rest).unwrap();
        }
        rest
    }
}

impl Drop for Started {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A request of the wire.
pub fn request(
    command: &str,
    session: Option<&str>,
    token: Option<&str>,
    args: &[(&str, &str)],
) -> Request {
    Request {
        command: command.to_string(),
        session: session.map(str::to_string),
        token: token.map(|t| Token::from_hex(t).unwrap()),
        args: args
            .iter()
            .map(|(name, value)| Arg {
                name: name.to_string(),
                value: value.to_string(),
            })
            .collect(),
    }
}

/// The lines of an answer from its `path` line on.
pub fn path_block(text: &str) -> String {
    let start = text.find("\npath ").map(|at| at + 1).unwrap_or(text.len());
    text[start..]
        .lines()
        .take_while(|line| line.starts_with("path ") || line.starts_with("  "))
        .map(|line| format!("{line}\n"))
        .collect()
}

// --- A surfd in the test process ---------------------------------------------------------------

/// `surfd` started in the test process through `Daemon::start` (decision D29), with the built
/// `surf-session` as its session program and the case's `XDG_RUNTIME_DIR` in the session's
/// environment, so that the case's guard finds every session process; `adjust` changes the
/// config first. Stopped when the returned value is dropped.
pub fn surfd_in_process(dir: &TestDir, adjust: impl FnOnce(&mut surfd::Config)) -> surfd::Daemon {
    let mut config = surfd::Config::new(Some(dir.runtime()));
    config.session_program = session_program();
    config.session_env = vec![("XDG_RUNTIME_DIR".into(), dir.runtime().into())];
    adjust(&mut config);
    match surfd::Daemon::start(config) {
        Ok(surfd::Started::Daemon(daemon)) => daemon,
        Ok(surfd::Started::Another) => panic!("another surfd holds the lock"),
        Err(refused) => panic!("surfd refused: {refused}"),
    }
}

/// One request of the wire to the case's `surfd` and its answer.
pub fn wire_call(dir: &TestDir, request: &Request) -> wire::Answer {
    let mut stream = UnixStream::connect(dir.surf().join("surfd.sock")).expect("surfd listens");
    stream.write_all(&request.encode().unwrap()).unwrap();
    wire::read_answer(&mut stream, wire::ANSWER_TIMEOUT).expect("an answer of the wire")
}

/// `open [<url>]` over the wire, which must answer status 0.
pub fn wire_open(dir: &TestDir, url: Option<&str>) -> Opened {
    let args: Vec<(&str, &str)> = url.map(|url| ("url", url)).into_iter().collect();
    let answer = wire_call(dir, &request("open", None, None, &args));
    assert_eq!(answer.status, 0, "open: {answer:?}");
    opened(&answer.text)
}

/// `<command>` with `args` over the wire, as the holder of `session`.
pub fn wire_as(
    dir: &TestDir,
    session: &Opened,
    command: &str,
    args: &[(&str, &str)],
) -> wire::Answer {
    wire_call(
        dir,
        &request(command, Some(&session.id), Some(&session.token), args),
    )
}

// --- The page server ----------------------------------------------------------------------------

/// An HTTP server on `127.0.0.1` port 0 that answers every `GET` with status 200, `text/html`
/// and one body; stopped when dropped.
pub struct PageServer {
    pub port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl PageServer {
    /// A server of a short body.
    pub fn start() -> PageServer {
        PageServer::with_body(b"<!doctype html><p>a page</p>".to_vec())
    }

    /// A server of `body`.
    pub fn with_body(body: Vec<u8>) -> PageServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port on 127.0.0.1");
        let port = listener.local_addr().unwrap().port();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            for stream in listener.incoming() {
                if stopping.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(stream) = stream {
                    answer_page(stream, &body);
                }
            }
        });
        PageServer {
            port,
            stop,
            thread: Some(thread),
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }
}

impl Drop for PageServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn answer_page(mut stream: TcpStream, body: &[u8]) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") && head.len() < 16 * 1024 {
        match stream.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            Ok(_) => return,
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(_) => return,
        }
    }
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: \
         close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}

// --- Golden texts -----------------------------------------------------------------------------

/// The golden text `name` under `tests/golden/`.
pub fn golden(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// `text` with the identity, the token and the port replaced by `<id>`, `<token>` and `<port>`.
pub fn placeholders(text: &str, id: &str, token: &str, port: u16) -> String {
    text.replace(id, "<id>")
        .replace(token, "<token>")
        .replace(&format!("127.0.0.1:{port}"), "127.0.0.1:<port>")
}
