//! Review 1, security reviewer: what bytes from a server or from the command line can put on the
//! `surf` binary's standard output and standard error. Servers run on `127.0.0.1` port 0.

use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const BOUND: Duration = Duration::from_secs(5);
const RUN_BOUND: Duration = Duration::from_secs(20);

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
        let _ = stream.shutdown(std::net::Shutdown::Write);
        while let Ok(n) = stream.read(&mut buf) {
            if n == 0 {
                break;
            }
        }
    });
    (format!("http://127.0.0.1:{port}/"), handle)
}

struct Run {
    stdout: String,
    stderr: String,
}

fn surf(address: &str) -> Run {
    let mut child = Command::new(env!("CARGO_BIN_EXE_surf"))
        .arg(address)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let deadline = Instant::now() + RUN_BOUND;
    while child.try_wait().expect("wait").is_none() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("surf ran longer than {RUN_BOUND:?}");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().expect("output");
    Run {
        stdout: String::from_utf8(output.stdout).expect("UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("UTF-8"),
    }
}

/// No control character but the line feed that ends each line.
fn assert_no_control(stream: &str, text: &str, case: &str) {
    let bad: Vec<char> = text.chars().filter(|c| c.is_control() && *c != '\n').collect();
    assert!(bad.is_empty(), "{case}: {stream} carries {bad:?}: {text:?}");
}

const HOSTILE: &[&str] = &[
    "\u{1b}[2J\u{1b}]0;title\u{7}",
    "\u{9b}31m\u{9d}0;t\u{9c}",
    "\u{7f}\u{8}\u{8}\u{8}",
    "\r\nrequest 00000000-0000-4000-8000-000000000000 Responded http://forged/",
];

/// Command-line bytes in every part of the address: the scheme (whose detail `surf-fetch`
/// writes with `{:?}`), the host, the path; on both streams.
#[test]
fn review1_command_line_control_characters_reach_neither_stream() {
    for hostile in HOSTILE {
        for address in [
            format!("{hostile}x:y"),
            format!("x{hostile}:y"),
            format!("http://h{hostile}/"),
            format!("http://127.0.0.1:1/{hostile}"),
            format!("ftp://h/{hostile}"),
            hostile.to_string(),
        ] {
            let run = surf(&address);
            assert_no_control("stdout", &run.stdout, &address);
            assert_no_control("stderr", &run.stderr, &address);
            assert!(
                !run.stdout.contains("forged") || run.stdout.lines().all(|l| !l.starts_with("request 00000000")),
                "{address:?}: forged row {:?}",
                run.stdout
            );
        }
    }
}

/// Server bytes in the reason phrase, in header fields and in the body, on a committed load and
/// on a malformed one.
#[test]
fn review1_server_control_characters_reach_neither_stream() {
    for reply in [
        &b"HTTP/1.1 200 \x1b]0;t\x07\x1b[2J\r\nContent-Type: text/html\r\nX: \x1b[31m\r\nContent-Length: 9\r\n\r\n\x1b[2J\r\nrow\n"[..],
        b"HTTP/1.1 200 OK\r\nContent-Type: text/\x1b[31mhtml\r\n\r\n\x1b[2J",
        b"HTTP/1.1 200 OK\r\nX\x1b: y\r\n\r\n",
        b"\x1b[2J\r\nrequest 00000000-0000-4000-8000-000000000000 Responded http://forged/\r\n",
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\x1b[2J\r\n",
    ] {
        let (address, handle) = serve(reply);
        let run = surf(&address);
        handle.join().expect("server");
        let case = String::from_utf8_lossy(reply);
        assert_no_control("stdout", &run.stdout, &case);
        assert_no_control("stderr", &run.stderr, &case);
        assert!(!run.stdout.contains("forged") && !run.stderr.contains("forged"), "{case:?}");
    }
}

/// Unicode's mandatory line breaks (UAX #14 class BK: U+2028, U+2029) and the bidirectional
/// override and isolate characters are not control characters (general category Zl, Zp, Cf), so
/// `escape` writes them as themselves. A reader that splits lines the Unicode way sees one more
/// row than the binary printed, and a terminal that applies bidi shows the row reordered.
#[test]
fn review1_unicode_line_separators_and_bidi_controls_cannot_split_or_reorder_a_row() {
    let forged = "request 00000000-0000-4000-8000-000000000000 Responded http://forged/";
    let address = format!("http://127.0.0.1:1/\u{2028}{forged}\u{2029}\u{202e}gnp.exe");
    let run = surf(&address);
    for (stream, text) in [("stdout", &run.stdout), ("stderr", &run.stderr)] {
        let found: Vec<char> = text
            .chars()
            .filter(|c| matches!(c, '\u{2028}' | '\u{2029}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
            .collect();
        assert!(
            found.is_empty(),
            "{stream} holds {found:?}; split on U+2028/U+2029 it reads {:#?}",
            text.split(['\n', '\u{2028}', '\u{2029}']).collect::<Vec<_>>()
        );
    }
}
