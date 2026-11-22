//! Cases 19 to 24: the runtime directory, and `open` starting `surfd`, with a copy of the built
//! `surf-agent` in a directory of the test and the stand-in beside it as `surfd`. Since the
//! story `security-review-fixes`, the checks of the runtime directory, `surf/` and the socket
//! before any connection, each against a listener of the test that must receive nothing.

use crate::common;

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::*;
use surf_agent::client::Config;
use surf_agent::wire;
use surf_test_process::{alive, ended_within};

const OPENED: &str = "surf 1 session 01234567-0123-4012-8012-012345678901 open\n\
token fedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedc\n\
did open \"http://example.org/\"\npage none\n";

#[test]
fn no_runtime_dir_exits_3_and_starts_nothing() {
    let dir = TestDir::new();
    let output = run(agent(&dir)
        .env_remove("XDG_RUNTIME_DIR")
        .args(["open", "http://example.org/"]));
    assert_eq!(output.status.code(), Some(3));
    assert!(
        stderr(&output).contains("XDG_RUNTIME_DIR is not set"),
        "{}",
        stderr(&output)
    );
    assert!(dir.starts().is_empty(), "surfd was started");
    assert_eq!(fs::read_dir(dir.runtime()).unwrap().count(), 0);
    let output = run(agent(&dir).env("XDG_RUNTIME_DIR", "").arg("open"));
    assert_eq!(output.status.code(), Some(3));
    assert!(dir.starts().is_empty(), "surfd was started");
}

#[test]
fn socket_path_over_107_bytes_exits_3() {
    let dir = TestDir::new();
    let suffix = "/surf/surfd.sock".len();
    let base = dir.path.join("l");
    let pad = |total: usize| -> PathBuf {
        let length = total - suffix - base.as_os_str().len();
        PathBuf::from(format!("{}{}", base.display(), "x".repeat(length)))
    };
    assert_eq!(wire::socket_path(&pad(107)).unwrap().as_os_str().len(), 107);
    assert_eq!(
        wire::socket_path(&pad(108)),
        Err(wire::SocketPathError::TooLong(108))
    );
    let long = pad(108);
    fs::create_dir_all(&long).unwrap();
    let output = run(agent(&dir).env("XDG_RUNTIME_DIR", &long).arg("open"));
    assert_eq!(output.status.code(), Some(3));
    assert!(
        stderr(&output).contains("108 bytes long"),
        "{}",
        stderr(&output)
    );
    assert!(
        stderr(&output).contains("at most 107"),
        "{}",
        stderr(&output)
    );
    assert_eq!(fs::read_dir(&long).unwrap().count(), 0);
    assert!(dir.starts().is_empty(), "surfd was started");
}

#[test]
fn open_with_no_listener_starts_surfd_once() {
    let dir = TestDir::new();
    dir.set("answer", OPENED);
    let output = run(agent(&dir).args(["open", "http://example.org/"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), OPENED);
    assert_eq!(dir.starts().len(), 1, "surfd started once");
    assert_eq!(
        dir.requests(),
        "surf-wire 1\ncommand open\narg url \"http://example.org/\"\nend\n"
    );
    // surf-agent did not wait for surfd to end: it still runs.
    assert!(alive(dir.starts()[0]));
}

#[test]
fn open_with_surfd_listening_starts_nothing() {
    let dir = TestDir::new();
    let _surfd = StandIn::start(&dir);
    dir.set("answer", OPENED);
    assert_eq!(dir.starts().len(), 1);
    let output = run(agent(&dir).arg("open"));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), OPENED);
    assert_eq!(dir.starts().len(), 1, "a second surfd was started");
    assert_eq!(dir.requests(), "surf-wire 1\ncommand open\nend\n");
}

#[test]
fn surfd_that_refuses_exits_3_with_its_reason() {
    let dir = TestDir::new();
    dir.set("start", "refused surf is a symbolic link\n");
    let output = run(agent(&dir).arg("open"));
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "surf-agent: surfd refused to start: surf is a symbolic link\n"
    );
    assert_eq!(dir.starts().len(), 1);
}

/// `open` against a `surfd` that writes nothing, run in this process by `client::run` with a
/// start timeout of 0.5 s (`Config::start_timeout`; the shipped one is `wire::START_TIMEOUT`,
/// 5 s), so the case waits 0.5 s, not 5 s. That the binary exits with `client::run`'s status 3
/// is held by `surfd_that_refuses_exits_3_with_its_reason`, a run of the binary.
#[test]
fn surfd_that_writes_nothing_exits_3_within_its_start_timeout() {
    assert_eq!(Config::default().start_timeout, Duration::from_secs(5));
    assert_eq!(wire::START_TIMEOUT, Duration::from_secs(5));
    let dir = TestDir::new();
    dir.set("start", "silent");
    let config = Config {
        start_timeout: Duration::from_millis(500),
        ..Config::default()
    };
    let started = Instant::now();
    let (status, _, err) = run_in_process(&dir, &["open"], &config);
    assert!(
        started.elapsed() < wire::START_TIMEOUT,
        "{:?}",
        started.elapsed()
    );
    assert_eq!(status, 3);
    assert!(
        err.contains("surfd did not report ready within 0.5 seconds"),
        "{err}"
    );
    let starts = dir.starts();
    assert_eq!(starts.len(), 1);
    // 2 s: surf-agent killed and waited for the silent surfd before it exited, so the first read
    // decides; the bound only ends a defect.
    if let Err(error) = ended_within(starts[0], Duration::from_secs(2)) {
        panic!("the silent surfd was left running: {error}");
    }
    assert!(!Path::new(&dir.runtime().join("surf/surfd.sock")).exists());
}

// --- The checks before a connection (story `security-review-fixes`) ---------------------------

/// A listener of the test at `path`, which must receive no connection.
fn listener_at(path: &Path) -> UnixListener {
    let listener = UnixListener::bind(path).expect("the test's listener");
    listener.set_nonblocking(true).unwrap();
    listener
}

/// `surf-agent look` with a session and its token; it must exit 3 naming `path` and `what`, and
/// `listener` must have received no connection, so no byte. `surf-agent` has ended when this
/// reads the listener, so one read decides.
fn look_is_refused(dir: &TestDir, path: &Path, what: &str, listener: &UnixListener) {
    let output = run(agent(dir)
        .arg("look")
        .env("SURF_SESSION", SESSION)
        .env("SURF_TOKEN", TOKEN));
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    let expected = format!("{} {what}", path.display());
    assert!(stderr(&output).contains(&expected), "{}", stderr(&output));
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    match listener.accept() {
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
        Err(error) => panic!("the test's listener: {error}"),
        Ok(_) => panic!("surf-agent connected to the listener"),
    }
    assert!(dir.starts().is_empty(), "surfd was started");
}

#[test]
fn a_runtime_dir_of_mode_0755_is_refused_before_any_byte() {
    let dir = TestDir::new();
    let surf = dir.runtime().join(wire::DIRECTORY);
    fs::create_dir(&surf).unwrap();
    fs::set_permissions(&surf, fs::Permissions::from_mode(0o700)).unwrap();
    let listener = listener_at(&surf.join(wire::SOCKET));
    fs::set_permissions(dir.runtime(), fs::Permissions::from_mode(0o755)).unwrap();
    look_is_refused(&dir, &dir.runtime(), "has mode 0755", &listener);
}

#[test]
fn surfs_directory_behind_a_symbolic_link_is_refused_before_any_byte() {
    let dir = TestDir::new();
    let target = dir.path.join("t");
    fs::create_dir(&target).unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).unwrap();
    let listener = listener_at(&target.join(wire::SOCKET));
    let surf = dir.runtime().join(wire::DIRECTORY);
    symlink(&target, &surf).unwrap();
    look_is_refused(&dir, &surf, "is a symbolic link", &listener);
}

#[test]
fn a_socket_path_that_is_no_socket_is_refused_before_any_byte() {
    let dir = TestDir::new();
    let surf = dir.runtime().join(wire::DIRECTORY);
    fs::create_dir(&surf).unwrap();
    fs::set_permissions(&surf, fs::Permissions::from_mode(0o700)).unwrap();
    let behind = dir.path.join("l.sock");
    let listener = listener_at(&behind);
    let socket = surf.join(wire::SOCKET);
    symlink(&behind, &socket).unwrap();
    look_is_refused(&dir, &socket, "is a symbolic link", &listener);
    fs::remove_file(&socket).unwrap();
    fs::write(&socket, "a regular file").unwrap();
    look_is_refused(&dir, &socket, "is not a socket", &listener);
}

#[test]
fn a_directory_of_another_user_is_refused() {
    let dir = TestDir::new();
    let caller = wire::caller_uid().unwrap();
    let other = caller.wrapping_add(1);
    let refusal = wire::check_socket_path(&dir.runtime(), other).unwrap_err();
    assert_eq!(
        refusal,
        format!(
            "{} is owned by user {caller}, not by the caller, user {other}",
            dir.runtime().display()
        )
    );
    assert_eq!(
        wire::check_socket_path(&dir.runtime(), caller),
        Ok(wire::SocketCheck::Absent)
    );
}

/// Security review of `security-review-fixes`: `XDG_RUNTIME_DIR` naming a symbolic link with a
/// trailing `/` (or `/.`) is still a runtime directory reached through a symbolic link, which the
/// design says is refused; `symlink_metadata` follows a link whose path ends in `/`. The client
/// must exit 3 naming the link, and the test's listener behind it must receive no byte.
#[test]
fn review_a_runtime_dir_that_is_a_symbolic_link_written_with_a_trailing_slash_is_refused() {
    use std::io::Read as _;
    use std::process::Stdio;

    let dir = TestDir::new();
    let surf = dir.runtime().join(wire::DIRECTORY);
    fs::create_dir(&surf).unwrap();
    fs::set_permissions(&surf, fs::Permissions::from_mode(0o700)).unwrap();
    let listener = listener_at(&surf.join(wire::SOCKET));
    let link = dir.path.join("l");
    symlink(dir.runtime(), &link).unwrap();
    let caller = wire::caller_uid().unwrap();
    for spelled in [format!("{}/", link.display()), format!("{}/.", link.display())] {
        let library = wire::check_socket_path(Path::new(&spelled), caller);
        let mut child = None;
        for _ in 0..50 {
            match agent(&dir)
                .env("XDG_RUNTIME_DIR", &spelled)
                .env("SURF_SESSION", SESSION)
                .env("SURF_TOKEN", TOKEN)
                .arg("look")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                started => {
                    child = Some(started.expect("surf-agent runs"));
                    break;
                }
            }
        }
        let mut child = child.expect("the copied surf-agent stayed busy");
        // Accept what the client sends, at most 5 s, and close it so the client ends.
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut received = Vec::new();
        let mut connected = false;
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    connected = true;
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut buffer = [0u8; 4096];
                    while let Ok(n) = stream.read(&mut buffer) {
                        if n == 0 {
                            break;
                        }
                        received.extend_from_slice(&buffer[..n]);
                        if received.ends_with(b"end\n") {
                            break;
                        }
                    }
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if child.try_wait().unwrap().is_some() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("the test's listener: {error}"),
            }
        }
        let output = child.wait_with_output().unwrap();
        let received = String::from_utf8_lossy(&received).into_owned();
        assert!(
            !connected,
            "XDG_RUNTIME_DIR={spelled:?}: check_socket_path gave {library:?}; surf-agent \
             connected through the symbolic link and wrote {} bytes, the token among them: {}",
            received.len(),
            received.contains(TOKEN)
        );
        assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
        assert!(
            stderr(&output).contains("is a symbolic link"),
            "{}",
            stderr(&output)
        );
    }
}

/// Every spelling of a runtime directory that is a symbolic link is refused naming the link, a
/// `..` component and a relative path are refused, and the real directory is accepted with a
/// trailing `/`, as some logins set `XDG_RUNTIME_DIR`.
#[test]
fn every_spelling_of_a_runtime_dir_behind_a_symbolic_link_is_refused() {
    let dir = TestDir::new();
    let surf = dir.runtime().join(wire::DIRECTORY);
    fs::create_dir(&surf).unwrap();
    fs::set_permissions(&surf, fs::Permissions::from_mode(0o700)).unwrap();
    let _listener = listener_at(&surf.join(wire::SOCKET));
    let link = dir.path.join("l");
    symlink(dir.runtime(), &link).unwrap();
    let caller = wire::caller_uid().unwrap();
    let named = format!("{} is a symbolic link", link.display());
    for suffix in ["", "/", "//", "/.", "/./", "/.//."] {
        let spelled = format!("{}{suffix}", link.display());
        assert_eq!(
            wire::check_socket_path(Path::new(&spelled), caller),
            Err(named.clone()),
            "{spelled}"
        );
    }
    let up = format!("{}/../{}", link.display(), "l");
    assert_eq!(
        wire::check_socket_path(Path::new(&up), caller),
        Err(format!("{up} has a .. component"))
    );
    assert_eq!(
        wire::check_socket_path(Path::new("run/user"), caller),
        Err("run/user is not an absolute path".to_string())
    );
    for suffix in ["", "/", "//", "/."] {
        let spelled = format!("{}{suffix}", dir.runtime().display());
        assert_eq!(
            wire::check_socket_path(Path::new(&spelled), caller),
            Ok(wire::SocketCheck::Present),
            "{spelled}"
        );
    }
}
