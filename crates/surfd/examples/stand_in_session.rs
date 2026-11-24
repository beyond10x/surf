//! A stand-in session process over [`surfd::link`], for the tests of `surfd`. `cargo test`
//! builds it. The in-process tests name it as the session program; the built tests copy it
//! beside `surfd` as `surf-session`.
//!
//! It is started as a session is: `<identity> <session socket path>`, standard input a pipe it
//! ends on when it closes (status 0), standard output its `ready` line. It draws a token with
//! `surf-secret`, prints it in its `open` answer, and compares every later request's token with
//! [`surf_secret::Token::matches`].
//!
//! Its behaviour is set by files in the directory `STAND_IN_DIR` names, each read when needed.
//! `surfd` gives a session no variable of its own environment, so a stand-in started by the
//! built `surfd`, which has no `Config::session_env`, finds no `STAND_IN_DIR`: it takes the
//! directory `c` beside the `XDG_RUNTIME_DIR` its socket path lies in (the layout of the tests'
//! `TestDir`) and starts itself again in place, with `STAND_IN_DIR` set to it, so that the tests
//! find it by its environment as before.
//!
//! - `start-behaviour`: `abort` calls `std::process::abort` before it writes a byte.
//! - `ready`: `never` writes no `ready` line.
//! - `open-status`: `1` or `2` answers `open` with that status (default `0`).
//! - `open-behaviour`, while it answers `open`, before it writes the answer: `abort` calls
//!   `std::process::abort`; `stall` waits for the lock on `hold`, which the test holds.
//! - `behaviour`, for every request but `open`: `answer` (the default); `stall` waits for the
//!   lock on `hold`, which the test holds, then answers; `exit` writes the start of an answer
//!   and exits with status 1; `break` writes a line that is not of the link and closes the
//!   connection; `oversize` sends answer lines of `wire::ANSWER_TEXT_MAX` plus one byte;
//!   `unprintable` sends an answer line holding the byte 0x01; `bad-entries` sends entries
//!   whose event line holds an address that is not quoted. Read after `open` too: `spin`
//!   loops on the processor once it answered `open`, until a CPU bound ends it; `abort` calls
//!   `std::process::abort` once it answered `open`.
//! - `address`, `title`: reported in the status record of every answer, exactly as the file
//!   holds them.
//!
//! The entries of its link answer, for the log `surfd` keeps: for `open` with status 0 its
//! action entry, one event entry (`surf.shell.WindowOpened`) and its answer entry without the
//! token line; for `go` and `close` the action entry and the answer entry; for `fill` and `press`
//! the action entry, for `fill` the `surf.agent.FillRequested` entry with the value as its length
//! (`"<n> bytes"`, as the session writes it), and the answer entry of a one-password form, with
//! its `now` and `changed` blocks (`story:view-controls-and-changes`); none for any other
//! request.
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
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use surf_agent::wire;
use surf_answer::{
    CHANGED_TEXT_CHARS_MAX, Change, Changed, ControlChange, Cut, Did, Element, Head,
    LABEL_CHARS_MAX, Now, Page, RefusalReason, SessionState as HeadState, State, Target, Value,
    Visibility,
};
use surf_secret::Token;
use surfd::link::{self, Answer, Entry, Quoted, SessionState};

const LIFETIME: Duration = Duration::from_secs(60);

/// The page a `fill` of the stand-in acts on.
const FORM: &str = "http://127.0.0.1:9/form.html";

/// The page a `press` of the stand-in loads.
const ANSWERED: &str = "http://127.0.0.1:9/answer.html";

struct Session {
    id: String,
    token: Token,
    control: PathBuf,
    /// The last sequence number of an action it sent an entry for.
    sequence: AtomicU64,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [id, socket] = args.as_slice() else {
        std::process::exit(2);
    };
    let Some(control) = std::env::var_os("STAND_IN_DIR").map(PathBuf::from) else {
        // `<runtime>/surf/<identity>/session.sock`, and `c` beside `<runtime>`.
        let Some(control) = Path::new(socket)
            .ancestors()
            .nth(4)
            .map(|case| case.join("c"))
        else {
            std::process::exit(2);
        };
        let Ok(program) = std::env::current_exe() else {
            std::process::exit(2);
        };
        let error = Command::new(program)
            .args(&args)
            .env("STAND_IN_DIR", control)
            .exec();
        drop(error);
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
    if setting(&control, "start-behaviour").as_deref() == Some("abort") {
        std::process::abort();
    }
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
        sequence: AtomicU64::new(0),
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
        entries: Vec::new(),
        text: String::new(),
    };
    let id = &session.id;
    if request.command == "open" {
        match setting(control, "open-behaviour").as_deref() {
            Some("abort") => std::process::abort(),
            Some("stall") => {
                if let Ok(hold) = File::open(control.join("hold")) {
                    let _ = hold.lock();
                }
            }
            _ => {}
        }
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
                open.did = Some(Did::Action {
                    command: "open".to_string(),
                    argument: request.arg("url").map(str::to_string),
                });
                open.page = Some(Page::None);
                let kept = surf_answer::render(&open).unwrap_or_default();
                open.token = Some(session.token.to_hex());
                answer.entries = entries(session, "open", None, &kept);
                answer.entries.insert(
                    1,
                    Entry::line(&format!(
                        "event surf.shell.WindowOpened window_id={}",
                        surf_answer::quote(id)
                    ))
                    .unwrap(),
                );
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
        drop(stream);
        if status == 0 {
            match setting(control, "behaviour").as_deref() {
                Some("spin") => {
                    let mut turns = 0u64;
                    loop {
                        turns = std::hint::black_box(turns.wrapping_add(1));
                    }
                }
                Some("abort") => std::process::abort(),
                _ => {}
            }
        }
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
                "{}\nstatus 0\nstate open\nentries 0\nanswer {count}\n",
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
        "bad-entries" => {
            let entries = "action 1 go \"a\"\nevent surf.net.RequestStarted url=http://a/\n";
            let bytes = format!(
                "{}\nstatus 0\nstate open\nentries {}\n{entries}answer 0\n",
                link::VERSION_LINE,
                entries.len()
            );
            let _ = stream.write_all(bytes.as_bytes());
            return;
        }
        "unprintable" => {
            let bytes = format!(
                "{}\nstatus 0\nstate open\nentries 0\nanswer 2\n\x01\n",
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
        answer.entries = entries(session, "close", None, &answer.text);
        write(&mut stream, &answer);
        drop(stream);
        std::process::exit(0);
    }
    answer.text = lines(id, HeadState::Open, did, true);
    if request.command == "go" {
        answer.entries = entries(session, "go", request.arg("url"), &answer.text);
    }
    if request.command == "fill" || request.command == "press" {
        let handle = request.arg("handle").unwrap_or_default();
        answer.text = control_answer(id, &request.command, handle);
        answer.entries = entries(session, &request.command, None, &answer.text);
        if request.command == "fill" {
            // As the session does (`crates/session/src/entries.rs`): the value as its length,
            // counted after `sanitized` in `crates/session/src/lib.rs`, which removes every
            // U+000A and U+000D from the value of a `text` or `password` control. The one
            // fillable control of [`FORM`] is a password.
            let length = request.arg("value").map_or(0, |value| {
                value
                    .chars()
                    .filter(|c| !matches!(c, '\n' | '\r'))
                    .map(char::len_utf8)
                    .sum()
            });
            let line = format!(
                "event surf.agent.FillRequested session_id={} element_id={} value={} handle={}",
                surf_answer::quote(id),
                surf_answer::quote(id),
                surf_answer::quote(&format!("{length} bytes")),
                surf_answer::quote(handle),
            );
            answer.entries.insert(1, Entry::line(&line).unwrap());
        }
    }
    write(&mut stream, &answer);
}

/// The answer of a `fill` or a `press` on the one-password form of [`FORM`]: after `fill`, the
/// password `(set)` and its `changed` line; after `press`, another page with one new text.
fn control_answer(id: &str, command: &str, handle: &str) -> String {
    let mut answer = surf_answer::Answer::new(Head {
        session: id.to_string(),
        state: HeadState::Open,
    });
    answer.did = Some(Did::Action {
        command: command.to_string(),
        argument: Some(handle.to_string()),
    });
    let loaded = |address: &str| Page::Loaded {
        status: 200,
        media_type: "text/html".to_string(),
        address: address.to_string(),
        title: None,
    };
    if command == "press" {
        answer.page = Some(loaded(ANSWERED));
        answer.changed = Some(Changed::Page {
            from: Some(FORM.to_string()),
            to: Some(ANSWERED.to_string()),
            texts: vec![Cut::of("Wrong password", CHANGED_TEXT_CHARS_MAX)],
            more: 0,
        });
        return surf_answer::render(&answer).unwrap_or_default();
    }
    answer.page = Some(loaded(FORM));
    answer.changed = Some(Changed::Controls(vec![ControlChange {
        handle: handle.to_string(),
        change: Change::Value {
            before: Value::Empty,
            after: Value::Set,
        },
    }]));
    answer.now = Some(Now {
        total: 1,
        elements: vec![Element {
            handle: handle.to_string(),
            kind: "password".to_string(),
            visibility: Visibility::Unknown,
            label: Cut::of("Password", LABEL_CHARS_MAX),
            target: Target::State(State {
                value: Some(Value::Set),
                ..State::default()
            }),
        }],
    });
    surf_answer::render(&answer).unwrap_or_default()
}

/// The entries of one recorded action: its action entry with the next sequence number, then
/// the answer entry of `text`.
fn entries(session: &Session, command: &str, url: Option<&str>, text: &str) -> Vec<Entry> {
    let sequence = session.sequence.fetch_add(1, Ordering::SeqCst) + 1;
    let line = match url {
        Some(url) => format!("action {sequence} {command} {}", surf_answer::quote(url)),
        None => format!("action {sequence} {command}"),
    };
    vec![Entry::line(&line).unwrap(), Entry::answer(text)]
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
