//! Adversary cases against the fixture's command loop (`story:view-first-page`, "anything else:
//! `{"error":"command"}`"): a command line that is not UTF-8 is answered and stops nothing.

#![forbid(unsafe_code)]

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(10);

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
        let fixture = Fixture {
            child,
            stdin,
            lines,
        };
        assert!(fixture.answer().is_some(), "the fixture's first line");
        fixture
    }

    fn answer(&self) -> Option<String> {
        self.lines.recv_timeout(WAIT).ok()
    }

    fn send(&mut self, bytes: &[u8]) -> Option<String> {
        let stdin = self.stdin.as_mut().expect("standard input open");
        if stdin.write_all(bytes).and_then(|()| stdin.flush()).is_err() {
            return None;
        }
        self.answer()
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

#[test]
fn adversary_a_command_line_that_is_not_utf8_is_answered_and_stops_nothing() {
    let mut fixture = Fixture::start();
    assert_eq!(
        fixture.send(b"open\t\xff\xfe\n").as_deref(),
        Some("{\"error\":\"command\"}"),
        "story: anything else is answered {{\"error\":\"command\"}}, a line that is not UTF-8 too; \
         it does not end the fixture's command loop"
    );
    assert_eq!(
        fixture.send(b"mend\n").as_deref(),
        Some("{}"),
        "the fixture still answers after the line"
    );
}
