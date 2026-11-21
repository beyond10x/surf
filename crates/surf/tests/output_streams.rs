//! The `surf` binary when an output stream cannot be written. README.md: exit 3 when standard
//! output cannot be written, the reason on standard error; and a standard error that cannot be
//! written changes no exit status. Servers run on `127.0.0.1` port 0 and end on their own.

use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const BOUND: Duration = Duration::from_secs(5);

fn serve(reply: &'static [u8]) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("address").port();
    listener.set_nonblocking(true).expect("nonblocking");
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + BOUND;
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == ErrorKind::WouldBlock && Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => return,
            }
        };
        drop(listener);
        stream.set_nonblocking(false).expect("blocking");
        stream.set_read_timeout(Some(BOUND)).expect("timeout");
        let mut request = Vec::new();
        let mut buf = [0u8; 1024];
        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => request.extend_from_slice(&buf[..n]),
            }
        }
        let _ = stream.write_all(reply);
        let _ = stream.flush();
        while let Ok(n) = stream.read(&mut buf) {
            if n == 0 {
                break;
            }
        }
    });
    (format!("http://127.0.0.1:{port}/"), handle)
}

fn full() -> File {
    OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("open /dev/full")
}

fn surf(args: &[&str], stdout: Stdio, stderr: Stdio) -> Output {
    Command::new(env!("CARGO_BIN_EXE_surf"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .output()
        .expect("run surf")
}

fn closed_port_address() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("address").port();
    drop(listener);
    format!("http://127.0.0.1:{port}/")
}

/// A committed load whose rows cannot be written exits 3, not 0, and says why on standard error.
#[test]
fn a_committed_load_whose_rows_cannot_be_written_exits_3() {
    let (address, server) =
        serve(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nok");
    let output = surf(&[&address], Stdio::from(full()), Stdio::piped());
    server.join().expect("server");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(3), "stderr {stderr:?}");
    assert!(
        stderr.starts_with("surf: standard output: "),
        "stderr {stderr:?}"
    );
}

/// A failed load whose rows cannot be written exits 3 as well, not 1.
#[test]
fn a_failed_load_whose_rows_cannot_be_written_exits_3() {
    let address = closed_port_address();
    let output = surf(&[&address], Stdio::from(full()), Stdio::piped());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(3), "stderr {stderr:?}");
    assert!(
        stderr
            .lines()
            .any(|l| l.starts_with("surf: standard output: ")),
        "stderr {stderr:?}"
    );
}

/// With standard error unwritable, every exit status is the one README.md gives: the binary does
/// not panic on a diagnostic it cannot write.
#[test]
fn an_unwritable_standard_error_changes_no_exit_status() {
    let usage = surf(&[], Stdio::piped(), Stdio::from(full()));
    assert_eq!(usage.status.code(), Some(2));
    assert!(usage.stdout.is_empty());

    let address = closed_port_address();
    let failed = surf(&[&address], Stdio::piped(), Stdio::from(full()));
    assert_eq!(failed.status.code(), Some(1));
    let stdout = String::from_utf8(failed.stdout).expect("UTF-8");
    assert_eq!(stdout.lines().count(), 3, "{stdout:?}");

    let both = surf(&[&address], Stdio::from(full()), Stdio::from(full()));
    assert_eq!(both.status.code(), Some(3));
}
