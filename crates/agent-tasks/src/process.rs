//! The processes of a private `XDG_RUNTIME_DIR`, found through `/proc`: every `/proc/<pid>`
//! whose `environ` holds `XDG_RUNTIME_DIR=<the directory>`, as the tests of `surf-session` find
//! a case's processes. Ending one is SIGKILL through `kill -9` and a wait with
//! [`surf_test_process::ended_within`].

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use surf_test_process::ended_within;

/// The bytes a session socket path adds to `XDG_RUNTIME_DIR`: `/surf/` 6, the identity 36, `/`
/// 1, `session.sock` 12.
pub const SESSION_SOCKET_ADDS: usize = 55;

/// The most bytes of a Unix socket path.
pub const SOCKET_PATH_MAX: usize = 107;

/// How long a cleanup waits for the processes it sent SIGKILL: SIGKILL ends each within
/// milliseconds.
pub const KILL_BOUND: Duration = Duration::from_secs(5);

/// A private `XDG_RUNTIME_DIR`, `<base>/atk-<pid>-<counter>-<nanoseconds>/r`, both directories
/// of mode 0700. When it is dropped, every process whose environment holds it is sent SIGKILL
/// and waited for, and the directory is removed.
pub struct RuntimeDir {
    path: PathBuf,
}

impl RuntimeDir {
    /// A new one under the caller's own `XDG_RUNTIME_DIR` when that is an absolute directory of
    /// the caller (short, so that a socket path fits), otherwise under `fallback`.
    pub fn new(fallback: &Path) -> RuntimeDir {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let base = base_dir(fallback);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|time| time.subsec_nanos())
            .unwrap_or(0);
        let path = base.join(format!(
            "atk-{}-{}-{nanos:09}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        let length = path.join("r").as_os_str().len() + SESSION_SOCKET_ADDS;
        assert!(
            length <= SOCKET_PATH_MAX,
            "the session socket path would be {length} bytes, over {SOCKET_PATH_MAX}: set \
             XDG_RUNTIME_DIR to a short directory of yours"
        );
        fs::create_dir(&path).expect("a new private directory");
        let dir = RuntimeDir { path };
        fs::set_permissions(&dir.path, fs::Permissions::from_mode(0o700)).expect("mode 0700");
        fs::create_dir(dir.runtime()).expect("the private XDG_RUNTIME_DIR");
        fs::set_permissions(dir.runtime(), fs::Permissions::from_mode(0o700)).expect("mode 0700");
        dir
    }

    /// The directory that holds the `XDG_RUNTIME_DIR`, removed when this is dropped.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The `XDG_RUNTIME_DIR`.
    pub fn runtime(&self) -> PathBuf {
        self.path.join("r")
    }

    /// The process ids whose environment holds `XDG_RUNTIME_DIR=<this directory>`.
    pub fn processes(&self) -> Vec<u32> {
        with_runtime(&self.runtime())
    }

    /// Those of [`RuntimeDir::processes`] whose `comm` is `name`; the `comm` is read first, so
    /// only those processes have their environment read.
    pub fn processes_named(&self, name: &str) -> Vec<u32> {
        let needle = needle(&self.runtime());
        pids()
            .into_iter()
            .filter(|&pid| comm(pid).as_deref() == Some(name))
            .filter(|&pid| holds(pid, &needle))
            .collect()
    }
}

impl Drop for RuntimeDir {
    fn drop(&mut self) {
        let pids = self.processes();
        for &pid in &pids {
            signal_kill(pid);
        }
        let deadline = Instant::now() + KILL_BOUND;
        for pid in pids {
            let _ = ended_within(pid, deadline.saturating_duration_since(Instant::now()));
        }
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// The caller's `XDG_RUNTIME_DIR` when it is an absolute directory owned by the caller, else
/// `fallback`.
fn base_dir(fallback: &Path) -> PathBuf {
    let caller = fs::metadata("/proc/self").map(|m| m.uid()).ok();
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .filter(|dir| {
            fs::symlink_metadata(dir).is_ok_and(|m| m.is_dir() && Some(m.uid()) == caller)
        })
        .unwrap_or_else(|| fallback.to_path_buf())
}

/// The process ids, other than this process, whose environment holds
/// `XDG_RUNTIME_DIR=<runtime>`.
pub fn with_runtime(runtime: &Path) -> Vec<u32> {
    let needle = needle(runtime);
    let own = std::process::id();
    pids()
        .into_iter()
        .filter(|&pid| pid != own && holds(pid, &needle))
        .collect()
}

/// `XDG_RUNTIME_DIR=<runtime>`, as it stands in an `environ` file.
fn needle(runtime: &Path) -> Vec<u8> {
    let mut needle = b"XDG_RUNTIME_DIR=".to_vec();
    needle.extend_from_slice(runtime.as_os_str().as_encoded_bytes());
    needle
}

/// Whether the environment of process `pid` holds the pair `needle`.
fn holds(pid: u32, needle: &[u8]) -> bool {
    fs::read(format!("/proc/{pid}/environ"))
        .unwrap_or_default()
        .split(|&b| b == 0)
        .any(|pair| pair == needle)
}

/// Every process id listed in `/proc`.
pub fn pids() -> Vec<u32> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| entry.file_name().to_str()?.parse::<u32>().ok())
        .collect()
}

/// The `comm` of a process, without its line feed.
pub fn comm(pid: u32) -> Option<String> {
    fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|text| text.trim_end_matches('\n').to_string())
}

/// The parent of a process, from its `/proc/<pid>/stat` (the fourth field, after the `)` that
/// ends the command name).
pub fn parent(pid: u32) -> Option<u32> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (_, rest) = stat.rsplit_once(')')?;
    rest.split_whitespace().nth(1)?.parse().ok()
}

/// Every process below `root` in the tree of parents.
pub fn descendants(root: u32) -> Vec<u32> {
    let links: Vec<(u32, u32)> = pids()
        .into_iter()
        .filter_map(|pid| Some((pid, parent(pid)?)))
        .collect();
    let mut found = Vec::new();
    let mut pending = vec![root];
    while let Some(above) = pending.pop() {
        for &(pid, parent) in &links {
            if parent == above && !found.contains(&pid) {
                found.push(pid);
                pending.push(pid);
            }
        }
    }
    found
}

/// Send SIGKILL to a process with `kill -9`.
pub fn signal_kill(pid: u32) {
    let _ = Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
