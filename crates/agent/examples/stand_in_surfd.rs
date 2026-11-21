//! A stand-in `surfd` over `surf_agent::wire`, for the tests of `surf-agent`. `cargo test`
//! builds it. A test starts it itself, or copies it beside a copy of `surf-agent` as `surfd`
//! so that `open` starts it.
//!
//! Its behaviour is set by files in the directory `STAND_IN_DIR` names, each read when it is
//! needed:
//!
//! - `start`: `ready` (the default) binds `$XDG_RUNTIME_DIR/surf/surfd.sock` and writes `ready`;
//!   a line `refused <reason>` is written as it is, and the stand-in exits; `silent` writes
//!   nothing and sleeps.
//! - `behaviour`, per connection: `answer` (the default) answers; `close` reads the request
//!   and closes; `stall` reads the request and sleeps; `oversize` sends an answer one byte over
//!   `ANSWER_TEXT_MAX`.
//! - `status` (default `0`), `reason` (default none) and `answer` (default one head line): the
//!   answer.
//!
//! What it records there: `starts`, one line with its process id per start; `requests`, the
//! bytes of every request it read, one after the other.
//!
//! It ends itself after 60 seconds, so that a test that fails leaves nothing running.

#![forbid(unsafe_code)]

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use surf_agent::wire::{self, Answer, Request};

const LIFETIME: Duration = Duration::from_secs(60);
const DEFAULT_ANSWER: &str = "surf 1 session 00000000-0000-4000-8000-000000000001 open\n";

fn main() {
    thread::spawn(|| {
        thread::sleep(LIFETIME);
        std::process::exit(0);
    });
    let Some(control) = std::env::var_os("STAND_IN_DIR").map(PathBuf::from) else {
        println!("refused STAND_IN_DIR is not set");
        std::process::exit(1);
    };
    append(
        &control.join("starts"),
        format!("{}\n", std::process::id()).as_bytes(),
    );
    let start = setting(&control, "start").unwrap_or_else(|| "ready".to_string());
    if start == "silent" {
        thread::sleep(LIFETIME);
        return;
    }
    if start.starts_with("refused") {
        println!("{start}");
        std::process::exit(1);
    }
    let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from) else {
        println!("refused XDG_RUNTIME_DIR is not set");
        std::process::exit(1);
    };
    let directory = runtime.join(wire::DIRECTORY);
    let socket = directory.join(wire::SOCKET);
    let listener = fs::create_dir_all(&directory)
        .and_then(|()| fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)))
        .and_then(|()| UnixListener::bind(&socket));
    let listener = match listener {
        Ok(listener) => listener,
        Err(error) => {
            println!("refused {error}");
            std::process::exit(1);
        }
    };
    println!("{}", wire::READY);
    let _ = std::io::stdout().flush();
    for stream in listener.incoming().flatten() {
        let control = control.clone();
        thread::spawn(move || serve(stream, &control));
    }
}

fn serve(stream: UnixStream, control: &Path) {
    let _ = stream.set_read_timeout(Some(wire::REQUEST_TIMEOUT));
    let mut reader = BufReader::new(stream.try_clone().expect("a second handle"));
    let mut raw = Vec::new();
    loop {
        let start = raw.len();
        let read = (&mut reader)
            .take((wire::REQUEST_MAX + 1 - start) as u64)
            .read_until(b'\n', &mut raw);
        if !matches!(read, Ok(n) if n > 0)
            || raw.len() > wire::REQUEST_MAX
            || raw.last() != Some(&b'\n')
        {
            break;
        }
        if &raw[start..] == b"end\n" {
            break;
        }
    }
    append(&control.join("requests"), &raw);
    let mut stream = stream;
    match setting(control, "behaviour").as_deref().unwrap_or("answer") {
        "close" => {}
        "stall" => thread::sleep(LIFETIME),
        "oversize" => {
            let count = wire::ANSWER_TEXT_MAX + 1;
            let head = format!("{}\nstatus 0\nanswer {count}\n", wire::VERSION_LINE);
            let _ = stream.write_all(head.as_bytes());
            let mut line = vec![b'a'; 1023];
            line.push(b'\n');
            let mut sent = 0;
            while sent < count {
                let part = &line[..line.len().min(count - sent)];
                if stream.write_all(part).is_err() {
                    break;
                }
                sent += part.len();
            }
        }
        _ => {
            let answer = match Request::decode(&raw) {
                Err(error) => Answer {
                    status: 2,
                    reason: Some(error.to_string()),
                    text: String::new(),
                },
                Ok(request) => match request.unknown_arg(&["url"]) {
                    Some(name) => Answer {
                        status: 2,
                        reason: Some(format!(
                            "the argument {name} is not one of {}",
                            request.command
                        )),
                        text: String::new(),
                    },
                    None => Answer {
                        status: setting(control, "status")
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0),
                        reason: setting(control, "reason"),
                        text: fs::read_to_string(control.join("answer"))
                            .unwrap_or_else(|_| DEFAULT_ANSWER.to_string()),
                    },
                },
            };
            if let Ok(bytes) = answer.encode() {
                let _ = stream.write_all(&bytes);
            }
        }
    }
}

/// The first line of the file `name` in `control`, if there is one.
fn setting(control: &Path, name: &str) -> Option<String> {
    let text = fs::read_to_string(control.join(name)).ok()?;
    Some(text.lines().next().unwrap_or_default().to_string())
}

fn append(path: &Path, bytes: &[u8]) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(bytes);
    }
}
