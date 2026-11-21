//! Review 2: the `surf` binary's exit status when standard output is closed rather than full, and
//! when its reader has gone. docs/surf-command.md: exit 3 "when writing to standard output reports an error,
//! whatever the navigation's state", and a closed standard output behaves like one redirected to
//! `/dev/null`, because the Rust runtime opens `/dev/null` on a closed descriptor 1 before `main`
//! runs. The descriptor is closed by `/bin/sh` (`>&-`) before `surf` starts.
//!
//! The two closed-output cases asserted exit 3 when written (review 2, F7). That is infeasible
//! without unsafe code, and the coordinator decided to document the behaviour instead; they now
//! assert it.

use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
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

fn closed_port_address() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("address").port();
    drop(listener);
    format!("http://127.0.0.1:{port}/")
}

/// `surf <address>` started by `/bin/sh` with `redirect` applied (`>&-` closes standard output).
fn surf_through_sh(address: &str, redirect: &str) -> (Option<i32>, String) {
    let output = Command::new("/bin/sh")
        .arg("-c")
        .arg(format!("exec \"$0\" \"$1\" {redirect}"))
        .arg(env!("CARGO_BIN_EXE_surf"))
        .arg(address)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run /bin/sh");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// `surf <address>` started by `/bin/sh` with standard output closed (`>&-`): its exit status,
/// what reached the pipe the shell was given as standard output, and its standard error.
fn surf_with_standard_output_closed(address: &str) -> (Option<i32>, Vec<u8>, String) {
    let output = Command::new("/bin/sh")
        .arg("-c")
        .arg("exec \"$0\" \"$1\" >&-")
        .arg(env!("CARGO_BIN_EXE_surf"))
        .arg(address)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run /bin/sh");
    (
        output.status.code(),
        output.stdout,
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// A committed load with standard output closed behaves as with `>/dev/null`, as docs/surf-command.md says:
/// exit 0, the rows lost, nothing on standard error.
#[test]
fn review2_a_committed_load_with_standard_output_closed_exits_0_and_its_rows_are_lost() {
    let (address, server) =
        serve(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nok");
    let (status, stdout, stderr) = surf_with_standard_output_closed(&address);
    server.join().expect("server");
    assert_eq!(
        status,
        Some(0),
        "standard output closed, committed load; stderr {stderr:?}"
    );
    assert!(
        stdout.is_empty(),
        "rows reached standard output: {stdout:?}"
    );
    assert_eq!(stderr, "", "standard output closed, committed load");
}

/// A failed load with standard output closed behaves as with `>/dev/null`, as docs/surf-command.md says:
/// exit 1, the rows lost, the reason on standard error.
#[test]
fn review2_a_failed_load_with_standard_output_closed_exits_1_and_its_rows_are_lost() {
    let address = closed_port_address();
    let (status, stdout, stderr) = surf_with_standard_output_closed(&address);
    assert_eq!(
        status,
        Some(1),
        "standard output closed, failed load; stderr {stderr:?}"
    );
    assert!(
        stdout.is_empty(),
        "rows reached standard output: {stdout:?}"
    );
    assert!(
        stderr.starts_with(&format!("surf: {address}: unreachable: ")),
        "standard output closed, failed load; stderr {stderr:?}"
    );
}

/// Standard error closed changes no exit status: a failed load still exits 1.
#[test]
fn review2_standard_error_closed_changes_no_exit_status() {
    let (status, _) = surf_through_sh(&closed_port_address(), "2>&-");
    assert_eq!(status, Some(1));
}

/// Standard output whose reader has gone (a broken pipe) exits 3 and does not die of SIGPIPE.
#[test]
fn review2_a_standard_output_whose_reader_has_gone_exits_3() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_surf"))
        .arg(closed_port_address())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn surf");
    drop(child.stdout.take());
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("stderr")
        .read_to_string(&mut stderr)
        .expect("read stderr");
    let status = child.wait().expect("wait");
    assert_eq!(status.code(), Some(3), "stderr {stderr:?}");
}
