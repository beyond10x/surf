//! What the tests of `surf-agent` share: a directory per test under a short base
//! ([`test_base`]), the stand-in `surfd`, and copies of the built `surf-agent` with the stand-in
//! beside it as `surfd`, so that no test can start a real `surfd`.

#![allow(dead_code)]

mod base;

pub use base::test_base;

use std::fs::{self, File};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// A session identity with no letter in it, so that no window of [`TOKEN`] can occur in it.
pub const SESSION: &str = "01234567-0123-4012-8012-012345678901";
/// A second session identity.
pub const OTHER_SESSION: &str = "11111111-2222-4333-8444-555555555555";
/// A token of letters only, in a pattern no word of a `Debug` output holds.
pub const TOKEN: &str = "fedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedc";
/// A second token.
pub const OTHER_TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// A directory of one test, removed when it is dropped: `r`, the `XDG_RUNTIME_DIR` of the test,
/// and `c`, the stand-in's control directory.
pub struct TestDir {
    pub path: PathBuf,
}

impl TestDir {
    /// A new directory under [`test_base`], named `sat-<process id>-<counter>-<8 random
    /// hexadecimal digits>` and created anew, so that two test processes of any checkouts cannot
    /// share one.
    pub fn new() -> TestDir {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let mut random = [0u8; 4];
        surf_secret::fill_random(&mut random).expect("the random source");
        let name = format!(
            "sat-{}-{}-{:08x}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst),
            u32::from_be_bytes(random)
        );
        let path = test_base().join(name);
        fs::create_dir(&path).expect("a new test directory");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        // From here on the drop guard removes the directory, also when a test panics.
        let dir = TestDir { path };
        fs::create_dir(dir.runtime()).expect("the runtime directory");
        fs::create_dir(dir.control()).expect("the control directory");
        fs::set_permissions(dir.runtime(), fs::Permissions::from_mode(0o700)).unwrap();
        let socket = dir.runtime().join("surf/surfd.sock");
        assert!(
            socket.as_os_str().len() <= 107,
            "the test's socket path is {} bytes; set XDG_RUNTIME_DIR to a short directory of \
             yours, or check the tree out at a shorter path",
            socket.as_os_str().len()
        );
        dir
    }

    pub fn runtime(&self) -> PathBuf {
        self.path.join("r")
    }

    pub fn control(&self) -> PathBuf {
        self.path.join("c")
    }

    /// Set the stand-in's `name` to `value`.
    pub fn set(&self, name: &str, value: &str) {
        fs::write(self.control().join(name), value).unwrap();
    }

    /// The bytes of every request the stand-in read, empty when it read none.
    pub fn requests(&self) -> String {
        fs::read_to_string(self.control().join("requests")).unwrap_or_default()
    }

    pub fn clear_requests(&self) {
        let _ = fs::remove_file(self.control().join("requests"));
    }

    /// The process ids of every start of the stand-in.
    pub fn starts(&self) -> Vec<u32> {
        fs::read_to_string(self.control().join("starts"))
            .unwrap_or_default()
            .lines()
            .map(|line| line.parse().unwrap())
            .collect()
    }

    /// A token file of `contents` with `mode`.
    pub fn token_file(&self, name: &str, contents: &str, mode: u32) -> PathBuf {
        let path = self.path.join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        for pid in self.starts() {
            kill(pid);
        }
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Send SIGKILL to `pid` and wait until it is gone, at most 2 seconds, if it is still a stand-in
/// of these tests (a process id can be taken again by another program).
pub fn kill(pid: u32) {
    let Ok(exe) = fs::read_link(format!("/proc/{pid}/exe")) else {
        return;
    };
    let ours = exe.starts_with(env!("CARGO_TARGET_TMPDIR"))
        || exe.file_name().is_some_and(|name| name == "stand_in_surfd");
    if !ours {
        return;
    }
    let _ = Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .stderr(Stdio::null())
        .status();
    let deadline = Instant::now() + Duration::from_secs(2);
    while alive(pid) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
}

/// Whether `pid` runs, as `kill -0` says. A process whose `/proc/<pid>/stat` state is `Z`
/// (zombie), `X` or `x` (dead, `proc_pid_stat(5)`) counts as gone: `X` is read while the parent
/// reaps the zombie, before the process leaves `/proc`.
pub fn alive(pid: u32) -> bool {
    let status = fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
    let state = status
        .rsplit(')')
        .next()
        .unwrap_or("")
        .trim_start()
        .chars()
        .next();
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
        && !matches!(state, Some('Z' | 'X' | 'x'))
}

/// The stand-in `surfd` that `cargo test` built.
pub fn stand_in_program() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let path = exe
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/stand_in_surfd");
    assert!(
        path.exists(),
        "{} is not built; run cargo test -p surf-agent",
        path.display()
    );
    path
}

/// A directory holding a copy of the built `surf-agent` and the stand-in as `surfd`, made once
/// per test program.
pub fn bin_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let name = std::env::current_exe()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("bin-{name}"));
        fs::create_dir_all(&dir).unwrap();
        copy(
            Path::new(env!("CARGO_BIN_EXE_surf-agent")),
            &dir.join("surf-agent"),
        );
        copy(&stand_in_program(), &dir.join("surfd"));
        dir
    })
}

/// Copies `from` to `to` through a partial file of this process: nextest runs each test in a
/// process of its own, so several processes copy the same program at once.
fn copy(from: &Path, to: &Path) {
    let part = to.with_extension(format!("part-{}", std::process::id()));
    fs::copy(from, &part).unwrap();
    fs::rename(&part, to).unwrap();
}

/// The copied `surf-agent` with an empty environment but `XDG_RUNTIME_DIR` of `dir`, and
/// `STAND_IN_DIR` for a stand-in it starts.
pub fn agent(dir: &TestDir) -> Command {
    let mut command = Command::new(bin_dir().join("surf-agent"));
    command
        .env_clear()
        .env("XDG_RUNTIME_DIR", dir.runtime())
        .env("STAND_IN_DIR", dir.control())
        .stdin(Stdio::null());
    command
}

/// Run `command` to its end, retrying a start the kernel refuses because a copy of the program
/// was still open for writing in a process another test forked.
pub fn run(command: &mut Command) -> Output {
    for _ in 0..50 {
        match command.output() {
            Err(error) if error.kind() == ErrorKind::ExecutableFileBusy => {
                thread::sleep(Duration::from_millis(20));
            }
            result => return result.expect("surf-agent runs"),
        }
    }
    panic!("the copied surf-agent stayed busy");
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A stand-in the test started, listening on `$XDG_RUNTIME_DIR/surf/surfd.sock` of its
/// [`TestDir`]; killed when dropped.
pub struct StandIn {
    child: Child,
}

impl StandIn {
    pub fn start(dir: &TestDir) -> StandIn {
        let mut child = Command::new(stand_in_program())
            .env_clear()
            .env("XDG_RUNTIME_DIR", dir.runtime())
            .env("STAND_IN_DIR", dir.control())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the stand-in starts");
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        assert_eq!(line, "ready\n", "the stand-in did not report ready");
        StandIn { child }
    }
}

impl Drop for StandIn {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Every window of four digits of [`TOKEN`] that occurs in `text`.
pub fn token_windows_in(text: &str) -> Vec<String> {
    let bytes = TOKEN.as_bytes();
    (0..=bytes.len() - 4)
        .map(|i| &TOKEN[i..i + 4])
        .filter(|window| text.contains(window))
        .map(str::to_string)
        .collect()
}
