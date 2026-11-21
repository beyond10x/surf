//! `surf-view-fixture` started as the page tests start it (`story:view-first-page`): `open` and a
//! `go` to `a.html`, the session list read with the view token, the pipe cut and mended, then
//! standard input closed: the fixture exits, the session's process is gone and the fixture's
//! directory is removed.

#![forbid(unsafe_code)]

use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(10);

/// `target/<profile>/` of this test, where `surf-agent` and `surf-session` are built with the
/// cargo running these tests (nothing is built when they are fresh), as `bin_dir()` of
/// `crates/session/tests/common/mod.rs` does.
fn bin_dir() -> PathBuf {
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
            "surf-agent",
            "-p",
            "surf-session",
            "--profile",
            &profile,
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::null())
        .status()
        .expect("cargo runs");
    assert!(status.success(), "building surf-agent and surf-session: {status}");
    profile_dir
}

/// The running fixture. When dropped while it runs (a failed assertion), its standard input is
/// closed so that it stops everything and removes its directory; killed only when it has not
/// ended within [`WAIT`].
struct Fixture {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
}

impl Fixture {
    fn start() -> Fixture {
        let bin = bin_dir();
        for program in ["surf-agent", "surf-session"] {
            assert!(bin.join(program).is_file(), "{program} beside the fixture");
        }
        let mut child = Command::new(env!("CARGO_BIN_EXE_surf-view-fixture"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start the fixture");
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("its standard output");
        let (sender, lines) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                if sender.send(line).is_err() {
                    return;
                }
            }
        });
        Fixture {
            child,
            stdin,
            lines,
        }
    }

    fn line(&self) -> String {
        self.lines
            .recv_timeout(WAIT)
            .unwrap_or_else(|error| panic!("no line from the fixture within {WAIT:?}: {error:?}"))
    }

    fn command(&mut self, words: &[&str]) -> String {
        let stdin = self.stdin.as_mut().expect("standard input open");
        writeln!(stdin, "{}", words.join("\t")).expect("send a command");
        stdin.flush().expect("flush");
        self.line()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.stdin.take());
        let deadline = Instant::now() + WAIT;
        while matches!(self.child.try_wait(), Ok(None)) {
            if Instant::now() >= deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}

/// The string value of member `name` in a one-line JSON object of strings without escapes.
fn member(json: &str, name: &str) -> String {
    let start = json
        .find(&format!("\"{name}\":\""))
        .unwrap_or_else(|| panic!("no member {name} in {json}"))
        + name.len()
        + 4;
    let end = json[start..].find('"').expect("a closed string") + start;
    json[start..end].to_string()
}

/// `http://127.0.0.1:<port>/#<token>`: the port and the token.
fn view_address(url: &str) -> (u16, String) {
    let rest = url
        .strip_prefix("http://127.0.0.1:")
        .unwrap_or_else(|| panic!("not a view address: {url}"));
    let (port, token) = rest.split_once("/#").expect("a token fragment");
    assert_eq!(token.len(), 64, "a token of 64 digits");
    (port.parse().expect("a port"), token.to_string())
}

/// `GET <target>` with the token: the status and the body; `None` when the connection ends
/// with no answer.
fn get(port: u16, target: &str, token: &str) -> Option<(u16, String)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream.set_read_timeout(Some(WAIT)).expect("read timeout");
    let request = format!(
        "GET {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\n\r\n"
    );
    if stream.write_all(request.as_bytes()).is_err() {
        return None;
    }
    let mut bytes = Vec::new();
    match stream.read_to_end(&mut bytes) {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::ConnectionReset => return None,
        Err(error) => panic!("reading the answer: {error}"),
    }
    if bytes.is_empty() {
        return None;
    }
    let text = String::from_utf8(bytes).expect("a UTF-8 answer");
    let status = text
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .expect("a status code");
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_default();
    Some((status, body))
}

/// The text of `/proc/<pid>/stat`, or why it could not be read.
fn stat(pid: u32) -> Result<String, String> {
    fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|error| format!("the read failed: {error}"))
}

/// Whether a `/proc/<pid>/stat` line is that of a process that runs: its state, the first
/// character after the last `)` (the command name may hold `)` and spaces), is there and is not
/// one that `proc_pid_stat(5)` gives an ended process: `Z` (zombie), `X` (dead) or `x` (dead,
/// Linux 2.6.33 to 3.13). `X` is read while the parent reaps the zombie, before the process
/// leaves `/proc`.
fn runs(stat: &str) -> bool {
    let state = stat
        .rsplit_once(')')
        .and_then(|(_, rest)| rest.trim_start().chars().next());
    state.is_some_and(|state| !matches!(state, 'Z' | 'X' | 'x'))
}

/// Whether `pid` runs: its `stat` reads and [`runs`] says so. A process gone from `/proc`, a
/// zombie and a dead process being reaped count as ended.
fn alive(pid: u32) -> bool {
    stat(pid).is_ok_and(|text| runs(&text))
}

/// A process that is not the fixture's: `surf-view-foreign <args>` of this package (it waits
/// until it is killed, whatever its name) started through a symbolic link named `surf-session`
/// in a directory of this test, with an empty environment. Not a program of the system: one
/// that chooses what it does by its name would exit at once. Killed and its directory removed
/// when dropped.
struct Foreign {
    child: Child,
    dir: PathBuf,
}

impl Foreign {
    fn start(args: &[&str]) -> Foreign {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("foreign-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("the foreign directory");
        let program = dir.join("surf-session");
        std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_surf-view-foreign"), &program)
            .expect("the symbolic link");
        let child = Command::new(&program)
            .args(args)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start the foreign process");
        let foreign = Foreign { child, dir };
        let deadline = Instant::now() + WAIT;
        while !cmdline_of(foreign.child.id()).first().is_some_and(|p| p.ends_with(b"surf-session")) {
            assert!(Instant::now() < deadline, "the foreign process did not start");
            thread::sleep(Duration::from_millis(10));
        }
        foreign
    }

    fn alive(&self) -> bool {
        alive(self.child.id())
    }
}

impl Drop for Foreign {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn cmdline_of(pid: u32) -> Vec<Vec<u8>> {
    fs::read(format!("/proc/{pid}/cmdline"))
        .unwrap_or_default()
        .split(|&b| b == 0)
        .filter(|part| !part.is_empty())
        .map(<[u8]>::to_vec)
        .collect()
}

/// The process of `surf-session` with `id` among its arguments.
fn session_pid(id: &str) -> u32 {
    session_pid_in(id, |_| true)
}

/// The process of `surf-session` with `id` among its arguments, of those `keep` admits.
fn session_pid_in(id: &str, keep: impl Fn(u32) -> bool) -> u32 {
    let entries = fs::read_dir("/proc").expect("read /proc");
    for entry in entries.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else {
            continue;
        };
        if !keep(pid) {
            continue;
        }
        let cmdline = fs::read(entry.path().join("cmdline")).unwrap_or_default();
        let args: Vec<&[u8]> = cmdline.split(|&b| b == 0).collect();
        if args.first().is_some_and(|p| p.ends_with(b"surf-session"))
            && args.contains(&id.as_bytes())
        {
            return pid;
        }
    }
    panic!("no surf-session process of {id}");
}

/// The fixture directories of process `pid`: `svf-<pid>-*` under the base the fixture uses.
fn directories_of(pid: u32) -> Vec<PathBuf> {
    let caller = fs::metadata("/proc/self").expect("/proc/self").uid();
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .filter(|dir| fs::symlink_metadata(dir).is_ok_and(|m| m.is_dir() && m.uid() == caller))
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_TARGET_TMPDIR")));
    let prefix = format!("svf-{pid}-");
    fs::read_dir(&base)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
                .map(|entry| entry.path())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_session_is_listed_the_pipe_cuts_and_mends_and_the_end_of_input_stops_everything() {
    let mut fixture = Fixture::start();
    let first = fixture.line();
    let view = member(&first, "view");
    let piped = member(&first, "piped");
    let pages = member(&first, "pages");
    assert!(member(&first, "unreachable").starts_with("http://127.0.0.1:"));
    let (port, token) = view_address(&view);
    let (pipe_port, pipe_token) = view_address(&piped);
    assert_eq!(pipe_token, token, "piped holds the view's token");
    assert_ne!(pipe_port, port, "piped is another port");
    let pid = fixture.child.id();
    let dirs = directories_of(pid);
    assert_eq!(dirs.len(), 1, "one directory of the fixture: {dirs:?}");

    let opened = fixture.command(&["open"]);
    let id = member(&opened, "id");
    assert_eq!(id.len(), 36, "a session identity: {opened}");
    let url = format!("{pages}/first/a.html");
    assert_eq!(fixture.command(&["agent", &id, "go", &url]), "{\"status\":0}");

    let (status, body) = get(port, "/api/sessions", &token).expect("an answer of the view");
    assert_eq!(status, 200, "{body}");
    assert!(body.contains(&format!("\"id\":\"{id}\"")), "{body}");
    assert!(
        body.contains(&format!(
            "\"page\":{{\"kind\":\"loaded\",\"status\":200,\"media_type\":\"text/html\",\"address\":\"{url}\""
        )),
        "a loaded page line of status 200: {body}"
    );

    // The pipe.
    let (status, through) = get(pipe_port, "/api/sessions", &token).expect("an answer through the pipe");
    assert_eq!(status, 200, "through the pipe: {through}");
    assert!(through.contains(&format!("\"id\":\"{id}\"")), "{through}");
    assert_eq!(fixture.command(&["cut"]), "{}");
    assert_eq!(get(pipe_port, "/api/sessions", &token), None, "no answer after cut");
    assert_eq!(fixture.command(&["mend"]), "{}");
    let (status, _) = get(pipe_port, "/api/sessions", &token).expect("an answer after mend");
    assert_eq!(status, 200, "through the pipe after mend");

    // `kill` never reaches a process that is not this fixture's: a foreign `surf-session` with
    // an opened id and a never-opened id among its arguments, outside the fixture's
    // XDG_RUNTIME_DIR, stays alive.
    let second = member(&fixture.command(&["open"]), "id");
    let never = "0a1b2c3d-1111-4222-8333-444455556666";
    let foreign = Foreign::start(&[&second, never]);
    let second_pid = session_pid_in(&second, |pid| pid != foreign.child.id());
    assert_eq!(fixture.command(&["kill", &second]), "{}");
    let read = stat(second_pid);
    assert!(
        !read.as_deref().is_ok_and(runs),
        "the fixture's own session {second} is killed: /proc/{second_pid}/stat read {read:?}"
    );
    assert!(foreign.alive(), "a foreign surf-session survives kill of an opened id");
    assert_eq!(fixture.command(&["kill", never]), "{\"error\":\"session\"}");
    assert!(foreign.alive(), "a foreign surf-session survives kill of an id never opened");
    drop(foreign);

    assert_eq!(fixture.command(&["what"]), "{\"error\":\"command\"}");

    let session = session_pid(&id);
    assert!(alive(session));
    drop(fixture.stdin.take());
    let deadline = Instant::now() + WAIT;
    let exit = loop {
        if let Some(status) = fixture.child.try_wait().expect("try_wait") {
            break status;
        }
        assert!(Instant::now() < deadline, "the fixture did not exit within {WAIT:?}");
        thread::sleep(Duration::from_millis(20));
    };
    assert!(exit.success(), "the fixture exited with {exit}");
    while Path::new(&format!("/proc/{session}")).exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        !Path::new(&format!("/proc/{session}")).exists(),
        "the session's process {session} is gone"
    );
    assert!(directories_of(pid).is_empty(), "the fixture's directory is removed");
}

#[test]
fn a_stat_line_of_a_zombie_or_a_dead_process_is_one_of_an_ended_process() {
    for (line, running) in [
        ("4242 (surf-session) S 1 4242 4242 0 -1", true),
        ("4242 (surf-session) R 1 4242 4242 0 -1", true),
        ("4242 (surf-session) Z 1 4242 4242 0 -1", false),
        ("4242 (surf-session) X 1 4242 4242 0 -1", false),
        ("4242 (surf-session) x 1 4242 4242 0 -1", false),
        ("4242 (a) b) R 1 4242 4242 0 -1", true),
        ("4242 (a) b) Z 1 4242 4242 0 -1", false),
        ("", false),
    ] {
        assert_eq!(runs(line), running, "runs({line:?})");
    }
}
