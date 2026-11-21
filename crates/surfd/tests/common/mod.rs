//! What the tests of `surfd` share: a directory per case under [`test_base`] (the file the tests
//! of `surf-agent` use, included, not copied), the stand-in session, the two harnesses of
//! decision D29 (a `surfd` in the test process, and copies of the built programs), and guards
//! that stop every process a case started, also when it panics.

#![allow(dead_code)]

#[path = "../../../agent/tests/common/base.rs"]
mod base;

pub use base::test_base;

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use surf_agent::wire::{self, Answer, Request};
use surf_secret::Token;
use surfd::{Config, Daemon, Started};

/// The bytes a session socket path adds to `XDG_RUNTIME_DIR`: `/surf/` 6, the identity 36, `/`
/// 1, `session.sock` 12.
pub const SESSION_SOCKET_ADDS: usize = 55;

/// How long a case allows for an outcome: four times the 1-second timer it set (story, "Times
/// in the tests").
pub const ALLOWED: Duration = Duration::from_secs(5);

/// A directory of one case, removed when it is dropped, after every process of the case is
/// stopped: `r`, the `XDG_RUNTIME_DIR` of the case, and `c`, the stand-in's directory.
pub struct TestDir {
    pub path: PathBuf,
}

impl TestDir {
    /// A new directory under [`test_base`], named `sdt-<process id>-<counter>-<8 random
    /// hexadecimal digits>`.
    pub fn new() -> TestDir {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let mut random = [0u8; 4];
        surf_secret::fill_random(&mut random).expect("the random source");
        let name = format!(
            "sdt-{}-{}-{:08x}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst),
            u32::from_be_bytes(random)
        );
        let path = test_base().join(name);
        let runtime = path.join("r");
        let length = runtime.as_os_str().len() + SESSION_SOCKET_ADDS;
        assert!(
            length <= wire::SOCKET_PATH_MAX,
            "the test's session socket path is {length} bytes, over 107; set XDG_RUNTIME_DIR to \
             a short directory of yours"
        );
        fs::create_dir(&path).expect("a new test directory");
        // From here on the drop guard removes the directory, also when a case panics.
        let dir = TestDir { path };
        fs::set_permissions(&dir.path, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(dir.runtime()).expect("the runtime directory");
        fs::set_permissions(dir.runtime(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(dir.control()).expect("the stand-in's directory");
        dir
    }

    pub fn runtime(&self) -> PathBuf {
        self.path.join("r")
    }

    pub fn control(&self) -> PathBuf {
        self.path.join("c")
    }

    /// `$XDG_RUNTIME_DIR/surf` of the case.
    pub fn surf(&self) -> PathBuf {
        self.runtime().join("surf")
    }

    pub fn socket(&self) -> PathBuf {
        self.surf().join("surfd.sock")
    }

    /// Set the stand-in's `name` to `value`.
    pub fn set(&self, name: &str, value: &str) {
        fs::write(self.control().join(name), value).unwrap();
    }

    pub fn unset(&self, name: &str) {
        let _ = fs::remove_file(self.control().join(name));
    }

    /// Every start of the stand-in: its process id and the identity it was given.
    pub fn starts(&self) -> Vec<(u32, String)> {
        fs::read_to_string(self.control().join("starts"))
            .unwrap_or_default()
            .lines()
            .map(|line| {
                let (pid, id) = line.split_once(' ').expect("a start line");
                (pid.parse().unwrap(), id.to_string())
            })
            .collect()
    }

    /// The process id of the stand-in started for `id`.
    pub fn pid_of(&self, id: &str) -> u32 {
        self.starts()
            .into_iter()
            .find(|(_, started)| started == id)
            .map(|(pid, _)| pid)
            .unwrap_or_else(|| panic!("no stand-in was started for {id}"))
    }

    /// The bytes of every request the stand-in of `id` read.
    pub fn requests(&self, id: &str) -> String {
        fs::read_to_string(self.control().join(format!("requests-{id}"))).unwrap_or_default()
    }

    /// Hold the stand-in in `stall`: it answers once the returned guard is dropped.
    pub fn hold(&self) -> File {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.control().join("hold"))
            .unwrap();
        file.lock().unwrap();
        file
    }

    /// Whether the stand-in of process `pid` ended within `within`: it holds a lock on
    /// `alive-<pid>` from before it records its start to its end, so this waits on the kernel
    /// releasing it.
    pub fn stand_in_ends(&self, pid: u32, within: Duration) -> bool {
        wait_unlocked(&self.control().join(format!("alive-{pid}")), within)
    }

    /// Every file under `path`, recursively, without following a symbolic link.
    pub fn files_under(path: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let mut pending = vec![path.to_path_buf()];
        while let Some(dir) = pending.pop() {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let kind = entry.file_type().unwrap();
                if kind.is_dir() {
                    pending.push(entry.path());
                } else {
                    files.push(entry.path());
                }
            }
        }
        files
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        stop_processes_of(&self.control());
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Whether the lock on `path` can be taken within `within`, by a thread blocked on it; the
/// thread is left blocked when it cannot, and ends when the holder does.
pub fn wait_unlocked(path: &Path, within: Duration) -> bool {
    let Ok(file) = File::open(path) else {
        return true;
    };
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = file.lock();
        let _ = sender.send(());
    });
    receiver.recv_timeout(within).is_ok()
}

/// SIGKILL every process whose environment names `control` as `STAND_IN_DIR` (the stand-ins,
/// and the `surfd` and `surf-agent` of a built case), and wait until each is gone.
pub fn stop_processes_of(control: &Path) {
    let mut needle = b"STAND_IN_DIR=".to_vec();
    needle.extend_from_slice(control.as_os_str().as_encoded_bytes());
    let own = std::process::id();
    let mut pids = Vec::new();
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|n| n.parse::<u32>().ok())
            else {
                continue;
            };
            if pid == own {
                continue;
            }
            let environ = fs::read(entry.path().join("environ")).unwrap_or_default();
            if environ
                .split(|&b| b == 0)
                .any(|pair| pair == needle.as_slice())
            {
                pids.push(pid);
            }
        }
    }
    for &pid in &pids {
        signal_kill(pid);
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    for pid in pids {
        while alive(pid) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
    }
}

pub fn signal_kill(pid: u32) {
    let _ = Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .stderr(Stdio::null())
        .status();
}

/// Whether `pid` runs. A process gone from `/proc` and one whose `stat` state is `Z` (zombie),
/// `X` or `x` (dead, `proc_pid_stat(5)`) count as gone: `X` is read while the parent reaps the
/// zombie, before the process leaves `/proc`.
pub fn alive(pid: u32) -> bool {
    let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) else {
        return false;
    };
    let state = stat
        .rsplit(')')
        .next()
        .unwrap_or("")
        .trim_start()
        .chars()
        .next();
    state.is_some_and(|state| !matches!(state, 'Z' | 'X' | 'x'))
}

/// The directory `cargo test` builds the examples into.
pub fn examples_dir() -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
}

/// The stand-in session `cargo test` built.
pub fn stand_in_program() -> PathBuf {
    let path = examples_dir().join("stand_in_session");
    assert!(
        path.exists(),
        "{} is not built; run cargo test -p surfd",
        path.display()
    );
    path
}

// --- In process -------------------------------------------------------------------------------

/// The config of a `surfd` in the test process: the case's runtime directory, the stand-in as
/// the session program, steered by `STAND_IN_DIR`.
pub fn config(dir: &TestDir) -> Config {
    let mut config = Config::new(Some(dir.runtime()));
    config.session_program = stand_in_program();
    config.session_env = vec![(OsString::from("STAND_IN_DIR"), dir.control().into())];
    config
}

/// Start `surfd` in the test process; it is stopped when the returned value is dropped.
pub fn start(config: Config) -> Daemon {
    match Daemon::start(config) {
        Ok(Started::Daemon(daemon)) => daemon,
        Ok(Started::Another) => panic!("another surfd holds the lock"),
        Err(refused) => panic!("surfd refused: {refused}"),
    }
}

/// One request to the `surfd` of `dir` and its answer.
pub fn call(dir: &TestDir, request: &Request) -> Answer {
    let mut stream = UnixStream::connect(dir.socket()).expect("surfd listens");
    stream.write_all(&request.encode().unwrap()).unwrap();
    wire::read_answer(&mut stream, wire::ANSWER_TIMEOUT).expect("an answer")
}

pub fn request(command: &str, session: Option<&str>, token: Option<&str>) -> Request {
    Request {
        command: command.to_string(),
        session: session.map(str::to_string),
        token: token.map(|t| Token::from_hex(t).unwrap()),
        args: Vec::new(),
    }
}

/// A session opened through `open`: its identity and token read from the answer.
pub struct Opened {
    pub id: String,
    pub token: String,
    pub answer: Answer,
}

/// Read the identity of the head line and the token line of an `open` answer.
pub fn opened(answer: Answer) -> Opened {
    let head = answer.text.lines().next().expect("a head line");
    let id = head
        .strip_prefix("surf 1 session ")
        .and_then(|rest| rest.strip_suffix(" open"))
        .unwrap_or_else(|| panic!("not the head line of an open session: {head}"))
        .to_string();
    let token = answer
        .text
        .lines()
        .find_map(|line| line.strip_prefix("token "))
        .expect("a token line")
        .to_string();
    Opened { id, token, answer }
}

pub fn open(dir: &TestDir) -> Opened {
    let answer = call(dir, &request("open", None, None));
    assert_eq!(answer.status, 0, "open: {answer:?}");
    opened(answer)
}

pub fn look(dir: &TestDir, session: &Opened) -> Answer {
    call(
        dir,
        &request("look", Some(&session.id), Some(&session.token)),
    )
}

/// The answer lines `surfd` writes itself for a session it answers for.
pub fn refusal(id: &str, state: &str, reason: &str) -> String {
    format!("surf 1 session {id} {state}\ndid refused {reason}\n")
}

// --- Built ------------------------------------------------------------------------------------

/// A directory holding copies of `examples/agent` as `surf-agent`, of the built `surfd`, and of
/// the stand-in as `surf-session`, made once per test program.
pub fn bin_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let name = std::env::current_exe()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("surfd-bin-{name}"));
        fs::create_dir_all(&dir).unwrap();
        let agent = examples_dir().join("agent");
        assert!(agent.exists(), "{} is not built", agent.display());
        copy(&agent, &dir.join("surf-agent"));
        copy(Path::new(env!("CARGO_BIN_EXE_surfd")), &dir.join("surfd"));
        copy(&stand_in_program(), &dir.join("surf-session"));
        dir
    })
}

fn copy(from: &Path, to: &Path) {
    let part = to.with_extension("part");
    fs::copy(from, &part).unwrap();
    fs::rename(&part, to).unwrap();
}

/// The environment the client gives `surfd` and the stand-in reads: an empty one but
/// `XDG_RUNTIME_DIR` of the case and `STAND_IN_DIR`.
fn case_env(command: &mut Command, dir: &TestDir) {
    command
        .env_clear()
        .env("XDG_RUNTIME_DIR", dir.runtime())
        .env("STAND_IN_DIR", dir.control())
        .stdin(Stdio::null());
}

/// The copied `surf-agent` in the case's environment.
pub fn agent(dir: &TestDir) -> Command {
    let mut command = Command::new(bin_dir().join("surf-agent"));
    case_env(&mut command, dir);
    command
}

/// Run `command` to its end, retrying a start the kernel refuses because a copy of the program
/// was still open for writing in a process another case forked.
pub fn run(command: &mut Command) -> Output {
    for _ in 0..50 {
        match command.output() {
            Err(error) if error.kind() == ErrorKind::ExecutableFileBusy => {
                thread::sleep(Duration::from_millis(20));
            }
            result => return result.expect("the program runs"),
        }
    }
    panic!("the copied program stayed busy");
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// `surf-agent open` in the case, which must succeed: the identity and token it printed.
pub fn agent_open(dir: &TestDir) -> Opened {
    let output = run(agent(dir).arg("open"));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    opened(Answer {
        status: 0,
        reason: None,
        text: stdout(&output),
    })
}

/// `surf-agent <command>` naming `session` with its token in the environment.
pub fn agent_call(dir: &TestDir, command: &str, session: &Opened) -> Output {
    run(agent(dir)
        .arg(command)
        .env("SURF_SESSION", &session.id)
        .env("SURF_TOKEN", &session.token))
}

/// A built `surfd` the test started itself, with the environment the client gives it; killed
/// and waited for when dropped.
pub struct Surfd {
    pub child: Child,
    /// The line it wrote on standard output, its line feed included.
    pub line: String,
}

impl Surfd {
    /// Start the copied `surfd` with `args` in the case's environment, changed by `env`, and read
    /// its one line.
    pub fn start(dir: &TestDir, args: &[&str], env: impl FnOnce(&mut Command)) -> Surfd {
        let mut command = Command::new(bin_dir().join("surfd"));
        case_env(&mut command, dir);
        command
            .args(args)
            .current_dir(&dir.path)
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        env(&mut command);
        let mut child = None;
        for _ in 0..50 {
            match command.spawn() {
                Err(error) if error.kind() == ErrorKind::ExecutableFileBusy => {
                    thread::sleep(Duration::from_millis(20));
                }
                result => {
                    child = Some(result.expect("surfd starts"));
                    break;
                }
            }
        }
        let mut child = child.expect("the copied surfd stayed busy");
        let mut line = String::new();
        let stdout = child.stdout.take().unwrap();
        BufReader::new(stdout)
            .take(wire::START_LINE_MAX as u64)
            .read_line(&mut line)
            .unwrap();
        Surfd { child, line }
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Wait at most `within` for it to end; its exit status if it did.
    pub fn ends_within(&mut self, within: Duration) -> Option<std::process::ExitStatus> {
        let deadline = Instant::now() + within;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return Some(status);
            }
            if Instant::now() >= deadline {
                return None;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Surfd {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Write `bytes` raw to the case's `surfd.sock` and read what comes back, to the end.
pub fn raw(dir: &TestDir, bytes: &[u8]) -> Vec<u8> {
    let mut stream = UnixStream::connect(dir.socket()).expect("surfd listens");
    stream.set_read_timeout(Some(wire::ANSWER_TIMEOUT)).unwrap();
    let _ = stream.write_all(bytes);
    let mut back = Vec::new();
    match stream.read_to_end(&mut back) {
        Ok(_) => back,
        Err(error) if error.kind() == ErrorKind::ConnectionReset => back,
        Err(error) => panic!("reading from surfd: {error}"),
    }
}
