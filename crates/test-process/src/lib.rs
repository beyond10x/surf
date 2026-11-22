//! Whether another process has ended, decided once for the tests of this workspace and for the
//! test fixture program `surf-view-fixture`. Test tooling only: no shipped program depends on
//! it. Standard library only.
//!
//! The decision reads `/proc/<pid>/stat` (`proc_pid_stat(5)`). A process has ended when that
//! file cannot be read (it left `/proc`, or never was there) or when its state is one that the
//! manual page gives an ended process: `Z` (zombie), `X` (dead) or `x` (dead, Linux 2.6.33 to
//! 3.13). `X` is read while the parent reaps a zombie, before the process leaves `/proc`: a read
//! that counted it as running made the forge's run of `61fb7a9` fail (fixed in `36e0dfb`).
//!
//! A single read right after a kill, a close or an end of input decides too early: the signal
//! is delivered and the process exits after the call that sent it returned. A test that asserts
//! that a process ended waits with [`ended_within`], whose bound names its reason, and its error
//! holds the last `stat` text read, so that a failure names the state.

#![forbid(unsafe_code)]

use std::fs;
use std::thread;
use std::time::{Duration, Instant};

/// The pause between two reads of [`ended_within`].
const POLL: Duration = Duration::from_millis(10);

/// Whether a `/proc/<pid>/stat` text is that of a process that runs: its state, the first
/// character after the last `)` (the command name before it may hold `)` and spaces), is there
/// and is not `Z`, `X` or `x`. Every other state `proc_pid_stat(5)` lists (`R`, `S`, `D`, `T`,
/// `t`, `W`, `K`, `P`, `I`) is one of a process that runs. The empty text, a text without `)`
/// and one with nothing after its last `)` hold no state and are those of an ended process.
pub fn runs(stat: &str) -> bool {
    let state = stat
        .rsplit_once(')')
        .and_then(|(_, rest)| rest.trim_start().chars().next());
    state.is_some_and(|state| !matches!(state, 'Z' | 'X' | 'x'))
}

/// Whether process `pid` runs, from one read of its `/proc/<pid>/stat`: `false` when the file
/// cannot be read, otherwise what [`runs`] says of its text. One read decides only for a
/// process that is expected to run; to see that a process ended, use [`ended_within`].
pub fn alive(pid: u32) -> bool {
    stat(pid).is_ok_and(|text| runs(&text))
}

/// Wait until process `pid` has ended, reading its `/proc/<pid>/stat` every 10 ms, at most
/// `bound`: the time it took when it ended, otherwise an error that holds the last text read.
/// A pid that never existed has ended at the first read.
pub fn ended_within(pid: u32, bound: Duration) -> Result<Duration, String> {
    let started = Instant::now();
    loop {
        let text = match stat(pid) {
            Err(_) => return Ok(started.elapsed()),
            Ok(text) if !runs(&text) => return Ok(started.elapsed()),
            Ok(text) => text,
        };
        let waited = started.elapsed();
        if waited >= bound {
            return Err(format!(
                "process {pid} has not ended within {bound:?}: /proc/{pid}/stat read {:?}",
                text.trim_end()
            ));
        }
        thread::sleep(POLL.min(bound - waited));
    }
}

/// The text of `/proc/<pid>/stat`.
fn stat(pid: u32) -> Result<String, std::io::Error> {
    fs::read_to_string(format!("/proc/{pid}/stat"))
}
