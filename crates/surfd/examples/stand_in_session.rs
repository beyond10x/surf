//! A stand-in session process over [`surfd::link`], for the tests of `surfd`. `cargo test`
//! builds it. The in-process tests name it as the session program; the built tests copy it
//! beside `surfd` as `surf-session`.
//!
//! It is started as a session is: `<identity> <session socket path>`, standard input a pipe it
//! ends on when it closes (status 0), standard output its `ready` line. It draws a token with
//! `surf-secret`, prints it in its `open` answer, and compares every later request's token with
//! [`surf_secret::Token::matches`].
//!
//! Its behaviour is set by files in the directory `STAND_IN_DIR` names, each read when needed:
//!
//! - `ready`: `never` writes no `ready` line.
//! - `open-status`: `1` or `2` answers `open` with that status (default `0`).
//! - `behaviour`, for every request but `open`: `answer` (the default); `stall` waits for the
//!   lock on `hold`, which the test holds, then answers; `exit` writes the start of an answer
//!   and exits with status 1; `break` writes a line that is not of the link and closes the
//!   connection; `oversize` sends answer lines of `wire::ANSWER_TEXT_MAX` plus one byte;
//!   `unprintable` sends an answer line holding the byte 0x01.
//! - `address`, `title`: reported in the status record of every answer, exactly as the file
//!   holds them.
//!
//! What it records there: `alive-<pid>`, a file it holds a lock on from before it records its
//! start to its end; `starts`, a line `<pid> <identity>` per start; `requests-<identity>`, every
//! request it read, encoded again, one after the other.
//!
//! It ends itself after 60 seconds, so that a test that fails leaves nothing running.

#![forbid(unsafe_code)]

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use surf_agent::wire;
use surf_answer::{Did, Head, Page, RefusalReason, SessionState as HeadState};
use surf_secret::Token;
use surfd::link::{self, Answer, Quoted, SessionState};

const LIFETIME: Duration = Duration::from_secs(60);

struct Session {
    id: String,
    token: Token,
    control: PathBuf,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [id, socket] = args.as_slice() else {
        std::process::exit(2);
    };
    let Some(control) = std::env::var_os("STAND_IN_DIR").map(PathBuf::from) else {
        std::process::exit(2);
    };
    let pid = std::process::id();
    let alive = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(control.join(format!("alive-{pid}")));
    let Ok(alive) = alive else {
        std::process::exit(1);
    };
    if alive.lock().is_err() {
        std::process::exit(1);
    }
    append(&control.join("starts"), format!("{pid} {id}\n").as_bytes());
    thread::spawn(|| {
        thread::sleep(LIFETIME);
        std::process::exit(0);
    });
    thread::spawn(|| {
        let mut sink = [0u8; 256];
        let mut stdin = io::stdin();
        loop {
            match stdin.read(&mut sink) {
                Ok(0) | Err(_) => std::process::exit(0),
                Ok(_) => {}
            }
        }
    });
    if setting(&control, "ready").as_deref() == Some("never") {
        loop {
            thread::park();
        }
    }
    let Ok(listener) = UnixListener::bind(socket) else {
        std::process::exit(1);
    };
    if link::write_ready(&mut io::stdout()).is_err() {
        std::process::exit(1);
    }
    let Ok(token) = Token::draw() else {
        std::process::exit(1);
    };
    let session = Arc::new(Session {
        id: id.clone(),
        token,
        control,
    });
    for stream in listener.incoming().flatten() {
        let session = Arc::clone(&session);
        thread::spawn(move || serve(stream, &session));
    }
    drop(alive);
}

fn lines(id: &str, state: HeadState, did: Did, page: bool) -> String {
    let mut answer = surf_answer::Answer::new(Head {
        session: id.to_string(),
        state,
    });
    answer.did = Some(did);
    if page {
        answer.page = Some(Page::None);
    }
    surf_answer::render(&answer).unwrap_or_default()
}

fn serve(mut stream: UnixStream, session: &Session) {
    let _ = stream.set_read_timeout(Some(wire::REQUEST_TIMEOUT));
    let request = {
        let mut reader = BufReader::new(&stream);
        match wire::read_request(&mut reader) {
            Ok(request) => request,
            Err(_) => return,
        }
    };
    append(
        &session.control.join(format!("requests-{}", session.id)),
        &request.encode().unwrap_or_default(),
    );
    let control = &session.control;
    let mut answer = Answer {
        status: 0,
        reason: None,
        state: SessionState::Open,
        address: fs::read_to_string(control.join("address"))
            .ok()
            .map(|address| Quoted::new(&address)),
        title: fs::read_to_string(control.join("title"))
            .ok()
            .map(|title| Quoted::new(&title)),
        text: String::new(),
    };
    let id = &session.id;
    if request.command == "open" {
        let status = setting(control, "open-status")
            .and_then(|status| status.parse().ok())
            .unwrap_or(0u8);
        answer.status = status;
        answer.text = match status {
            0 => {
                let mut open = surf_answer::Answer::new(Head {
                    session: id.clone(),
                    state: HeadState::Open,
                });
                open.token = Some(session.token.to_hex());
                open.did = Some(Did::Action {
                    command: "open".to_string(),
                    argument: request.arg("url").map(str::to_string),
                });
                open.page = Some(Page::None);
                surf_answer::render(&open).unwrap_or_default()
            }
            1 => {
                answer.reason = Some("the stand-in refused open".to_string());
                lines(
                    id,
                    HeadState::Open,
                    Did::Refused(RefusalReason::Token),
                    false,
                )
            }
            _ => {
                answer.reason = Some("the stand-in refused open as usage".to_string());
                lines(
                    id,
                    HeadState::Open,
                    Did::Refused(RefusalReason::Usage),
                    false,
                )
            }
        };
        write(&mut stream, &answer);
        return;
    }
    let named = request.session.as_deref().and_then(wire::session_id);
    let token_ok = request
        .token
        .as_ref()
        .is_some_and(|token| token.matches(&session.token));
    if named.as_deref() != Some(id.as_str()) || !token_ok {
        answer.status = 1;
        answer.text = lines(
            id,
            HeadState::Open,
            Did::Refused(RefusalReason::Token),
            true,
        );
        write(&mut stream, &answer);
        return;
    }
    match setting(control, "behaviour").as_deref().unwrap_or("answer") {
        "stall" => {
            if let Ok(hold) = File::open(control.join("hold")) {
                let _ = hold.lock();
            }
        }
        "exit" => {
            let _ = stream.write_all(format!("{}\nstatus 0\n", link::VERSION_LINE).as_bytes());
            std::process::exit(1);
        }
        "break" => {
            let _ = stream.write_all(b"not the link\n");
            return;
        }
        "oversize" => {
            let count = wire::ANSWER_TEXT_MAX + 1;
            let head = format!(
                "{}\nstatus 0\nstate open\nanswer {count}\n",
                link::VERSION_LINE
            );
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
            return;
        }
        "unprintable" => {
            let bytes = format!(
                "{}\nstatus 0\nstate open\nanswer 2\n\x01\n",
                link::VERSION_LINE
            );
            let _ = stream.write_all(bytes.as_bytes());
            return;
        }
        _ => {}
    }
    let did = Did::Action {
        command: request.command.clone(),
        argument: request.arg("url").map(str::to_string),
    };
    if request.command == "close" {
        answer.state = SessionState::Closed;
        answer.text = lines(id, HeadState::Closed, did, false);
        write(&mut stream, &answer);
        drop(stream);
        std::process::exit(0);
    }
    answer.text = lines(id, HeadState::Open, did, true);
    write(&mut stream, &answer);
}

fn write(stream: &mut UnixStream, answer: &Answer) {
    if let Ok(bytes) = answer.encode() {
        let _ = stream.write_all(&bytes);
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
