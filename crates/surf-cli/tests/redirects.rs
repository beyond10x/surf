//! Cases 21 and 22 of `story:agent-fetch-redirects`: the `surf` binary against a server the test
//! starts on `127.0.0.1` port 0 and stops before it ends. The binary is killed if it runs longer
//! than `RUN_BOUND`.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// The longest the binary may run before the test kills it and fails.
const RUN_BOUND: Duration = Duration::from_secs(20);

/// The longest the server waits for a request.
const BOUND: Duration = Duration::from_secs(5);

/// An HTTP server where `/a` answers 302 with `Location: <location>` and every other target 200
/// `text/html`. Stopped when dropped.
struct Server {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    fn start(location: &'static str) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
        let port = listener.local_addr().expect("local address").port();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            for stream in listener.incoming() {
                if stopping.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(stream) = stream {
                    answer(stream, location);
                }
            }
        });
        Server {
            port,
            stop,
            thread: Some(thread),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn answer(mut stream: TcpStream, location: &str) {
    let _ = stream.set_read_timeout(Some(BOUND));
    let mut head = Vec::new();
    let mut buf = [0u8; 1024];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") && head.len() < 16 * 1024 {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => head.extend_from_slice(&buf[..n]),
        }
    }
    let reply = if head.starts_with(b"GET /a ") {
        format!(
            "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: \
             close\r\n\r\n"
        )
    } else {
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 13\r\nConnection: \
         close\r\n\r\n<p>a page</p>"
            .to_string()
    };
    let _ = stream.write_all(reply.as_bytes());
}

struct Run {
    status: i32,
    stdout: String,
    stderr: String,
}

/// Run the binary on `address`, killing it after `RUN_BOUND`.
fn surf(address: &str) -> Run {
    let child = Command::new(env!("CARGO_BIN_EXE_surf"))
        .arg(address)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn surf");
    let pid = child.id();
    let (sender, receiver) = mpsc::channel();
    let waiter = thread::spawn(move || {
        let _ = sender.send(child.wait_with_output());
    });
    let output = match receiver.recv_timeout(RUN_BOUND) {
        Ok(output) => output.expect("surf output"),
        Err(_) => {
            let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
            let _ = waiter.join();
            panic!("surf ran longer than {RUN_BOUND:?}");
        }
    };
    waiter.join().expect("the waiting thread");
    Run {
        status: output.status.code().expect("surf exited, not signalled"),
        stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
    }
}

/// The words of `line`: its kind, its identity, and the rest.
fn split(line: &str) -> (&str, &str, &str) {
    let mut words = line.splitn(3, ' ');
    let kind = words.next().unwrap_or_default();
    let id = words.next().unwrap_or_default();
    let rest = words.next().unwrap_or_default();
    assert!(!id.is_empty(), "no identity in {line:?}");
    (kind, id, rest)
}

#[test]
fn case_21_surf_prints_the_address_the_navigation_ended_at() {
    let server = Server::start("/b");
    let asked = server.url("/a");
    let ended = server.url("/b");
    let run = surf(&asked);
    drop(server);
    assert_eq!(run.status, 0, "stderr: {}", run.stderr);
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines.len(), 3, "{}", run.stdout);
    assert_eq!(
        split(lines[0]),
        ("navigation", split(lines[0]).1, format!("Committed {ended}").as_str())
    );
    assert_eq!(
        split(lines[1]),
        ("request", split(lines[1]).1, format!("Responded {asked}").as_str())
    );
    assert_eq!(
        split(lines[2]),
        ("document", split(lines[2]).1, ended.as_str())
    );
}

#[test]
fn case_22_surf_on_a_loop_fails_too_many_redirects() {
    let server = Server::start("/a");
    let asked = server.url("/a");
    let run = surf(&asked);
    drop(server);
    assert_eq!(run.status, 1, "stderr: {}", run.stderr);
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines.len(), 3, "{}", run.stdout);
    let (kind, _, rest) = split(lines[0]);
    assert_eq!((kind, rest), ("navigation", format!("Failed {asked}").as_str()));
    let (kind, request_id, rest) = split(lines[1]);
    assert_eq!((kind, rest), ("request", format!("Failed {asked}").as_str()));
    let (kind, failed_id, rest) = split(lines[2]);
    assert_eq!(
        (kind, failed_id, rest),
        (
            "fetch-failed",
            request_id,
            format!("TooManyRedirects {asked}").as_str()
        )
    );
    assert!(
        run.stderr
            .starts_with(&format!("surf: {asked}: too many redirects")),
        "{}",
        run.stderr
    );
}
