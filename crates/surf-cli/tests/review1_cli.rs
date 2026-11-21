//! Review 1 (adversary): the `surf` binary against docs/surf-command.md. A `Content-Type` spelled the way
//! servers spell it reaches the `text/html` binding; usage errors; and the exit status when the
//! rows cannot be written. Servers run on `127.0.0.1` port 0 and end on their own.

use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::os::unix::ffi::OsStrExt;
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

fn surf<I: IntoIterator<Item = S>, S: AsRef<OsStr>>(args: I, stdout: Stdio) -> Output {
    Command::new(env!("CARGO_BIN_EXE_surf"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(Stdio::piped())
        .output()
        .expect("run surf")
}

#[test]
fn review1_html_with_upper_case_parameters_and_trailing_space_is_parsed_into_a_document() {
    let (address, server) = serve(
        b"HTTP/1.1 200 OK\r\nContent-Type: TEXT/HTML ;Charset=UTF-8 \r\nContent-Length: 4\r\n\r\n<p/>",
    );
    let output = surf([&address], Stdio::piped());
    server.join().expect("server");
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    assert_eq!(output.status.code(), Some(0), "{stdout:?}");
    let documents: Vec<&str> = stdout.lines().filter(|l| l.starts_with("document ")).collect();
    assert_eq!(documents.len(), 1, "{stdout:?}");
    assert!(documents[0].ends_with(&format!(" {address}")), "{stdout:?}");
}

/// docs/surf-command.md: exit 2 for "not exactly one argument, or an argument that is not UTF-8; a usage
/// line on standard error".
#[test]
fn review1_two_arguments_and_a_non_utf8_argument_are_usage_errors_with_nothing_on_stdout() {
    let two = surf(["http://127.0.0.1:1/", "x"], Stdio::piped());
    assert_eq!(two.status.code(), Some(2));
    assert!(two.stdout.is_empty());
    assert_eq!(String::from_utf8_lossy(&two.stderr), "usage: surf <url>\n");
    let bad = surf([OsStr::from_bytes(b"http://h/\xff")], Stdio::piped());
    assert_eq!(bad.status.code(), Some(2));
    assert!(bad.stdout.is_empty());
    assert!(String::from_utf8_lossy(&bad.stderr).ends_with("usage: surf <url>\n"));
}

/// The rows are the binary's only product. When none of them can be written, exit 0 reports a
/// committed load whose report was lost.
#[test]
fn review1_rows_that_cannot_be_written_do_not_exit_0() {
    let (address, server) =
        serve(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nok");
    let full = OpenOptions::new().write(true).open("/dev/full").expect("open /dev/full");
    let output = surf([&address], Stdio::from(full));
    server.join().expect("server");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(output.status.code(), Some(0), "stderr {stderr:?}");
}
