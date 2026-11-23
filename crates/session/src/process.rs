//! The session process: `surf-session [--idle-timeout <seconds>] [--request-timeout <seconds>] <identity> <socket path>`.
//!
//! `surfd` starts it with the two arguments (`crates/surfd/src/link.rs`, "Start"): the identity, a
//! UUID in lower-case text form, and the absolute path of the session socket in a directory
//! `surfd` made, mode 0700. It checks the identity, draws the token, binds the socket, writes
//! [`surfd::link::READY`] on standard output and never writes there again. Any other argument
//! shape exits 2 without `ready`; a token that cannot be drawn or a socket that cannot be bound
//! exits 1 without it.
//!
//! It serves one connection at a time: one request of [`surf_agent::wire`], read up to its `end`
//! line within [`wire::REQUEST_TIMEOUT`] of the accept (`--request-timeout` sets another, of the
//! form of `--idle-timeout`), at most [`wire::REQUEST_MAX`] bytes; one
//! link answer ([`surfd::link::Answer::encode`]), written within [`WRITE_TIMEOUT`]; then the
//! connection is closed. A request that does not decode is answered with status 2; one over the
//! limit, one slower than the deadline and one whose connection ends before `end` are closed
//! unanswered. The next request is served by the same process either way.
//!
//! It ends with status 0, removing its socket file first: after the answer to a `close` that
//! closed the session, after [`IDLE_TIMEOUT`] without a connection (`--idle-timeout` sets
//! another, a whole number of seconds from 1, for a test that starts the binary itself; `surfd`
//! passes no option), and when its standard input reaches its end (`surfd` closes the pipe).
//! Each of the two options comes at most once, before the identity, in either order.
//!
//! Bounds: at most one connection is read at a time and at most one more is held accepted, in a
//! channel of no capacity; the rest wait in the kernel's backlog. Standard input is read into a
//! buffer of 256 bytes that is thrown away. Nothing a connection sends is written to standard
//! output, standard error or a file.

use std::ffi::OsString;
use std::fs;
use std::io::{self, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

use surf_agent::wire::{self, RequestError};
use surfd::link;

use crate::{Session, fetch};

/// How long the session waits for a request before it ends: 30 minutes (design section 3).
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// How long writing one answer may take: 10 seconds.
pub const WRITE_TIMEOUT: Duration = Duration::from_secs(10);

/// The exit status of an argument shape other than the one above.
pub const USAGE_STATUS: u8 = 2;

/// The exit status when the token cannot be drawn, the socket cannot be bound or `ready`
/// cannot be written.
pub const START_FAILED_STATUS: u8 = 1;

/// The arguments the process runs with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arguments {
    /// The idle timeout: [`IDLE_TIMEOUT`] or `--idle-timeout`.
    pub idle_timeout: Duration,
    /// The read deadline of one request: [`wire::REQUEST_TIMEOUT`] or `--request-timeout`.
    pub request_timeout: Duration,
    /// The session identity, a UUID in lower-case text form.
    pub identity: String,
    /// The path of the session socket.
    pub socket: PathBuf,
}

/// The arguments in `args` (the words after the program name), or `None` for any other shape.
pub fn arguments(args: &[OsString]) -> Option<Arguments> {
    let (mut idle_timeout, mut request_timeout) = (None, None);
    let mut rest = args;
    loop {
        let option = match rest {
            [option, ..] if option == "--idle-timeout" => &mut idle_timeout,
            [option, ..] if option == "--request-timeout" => &mut request_timeout,
            _ => break,
        };
        let [_, seconds, tail @ ..] = rest else {
            return None;
        };
        if option.replace(whole_seconds(seconds)?).is_some() {
            return None;
        }
        rest = tail;
    }
    let [identity, socket] = rest else {
        return None;
    };
    let identity = identity.to_str()?;
    if wire::session_id(identity).as_deref() != Some(identity) {
        return None;
    }
    Some(Arguments {
        idle_timeout: idle_timeout.unwrap_or(IDLE_TIMEOUT),
        request_timeout: request_timeout.unwrap_or(wire::REQUEST_TIMEOUT),
        identity: identity.to_string(),
        socket: PathBuf::from(socket),
    })
}

/// The number of seconds in `word`, a whole number from 1, or `None`.
fn whole_seconds(word: &OsString) -> Option<Duration> {
    let seconds = word.to_str()?;
    if seconds.is_empty() || !seconds.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let seconds: u32 = seconds.parse().ok()?;
    if seconds == 0 {
        return None;
    }
    Some(Duration::from_secs(u64::from(seconds)))
}

/// What the main loop waits for.
enum Event {
    Connection(UnixStream),
    End,
}

/// Run the process over `args`, writing `ready` to `out`; the exit status.
pub fn run(args: &[OsString], out: &mut impl Write) -> u8 {
    let Some(arguments) = arguments(args) else {
        return USAGE_STATUS;
    };
    let Ok(mut session) = Session::new(&arguments.identity, fetch) else {
        return START_FAILED_STATUS;
    };
    let Ok(listener) = UnixListener::bind(&arguments.socket) else {
        return START_FAILED_STATUS;
    };
    if link::write_ready(out).is_err() {
        let _ = fs::remove_file(&arguments.socket);
        return START_FAILED_STATUS;
    }
    let (sender, events) = mpsc::sync_channel(0);
    let started = watch_input(sender.clone()).is_ok() && accept(listener, sender).is_ok();
    if started {
        // A timeout, the end of standard input and a closed channel all end the loop.
        while let Ok(Event::Connection(stream)) = events.recv_timeout(arguments.idle_timeout) {
            if serve(&mut session, stream, arguments.request_timeout) {
                break;
            }
        }
    }
    let _ = fs::remove_file(&arguments.socket);
    if started { 0 } else { START_FAILED_STATUS }
}

/// A thread that reads standard input to its end and then sends [`Event::End`].
fn watch_input(sender: SyncSender<Event>) -> io::Result<()> {
    thread::Builder::new()
        .name("surf-session-input".into())
        .spawn(move || {
            let mut sink = [0u8; 256];
            let mut input = io::stdin();
            loop {
                match input.read(&mut sink) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
            let _ = sender.send(Event::End);
        })
        .map(drop)
}

/// A thread that accepts connections and hands each to the main loop, waiting until it takes
/// it. An accept that fails for another reason than an interruption or an aborted connection
/// ends the session.
fn accept(listener: UnixListener, sender: SyncSender<Event>) -> io::Result<()> {
    thread::Builder::new()
        .name("surf-session-accept".into())
        .spawn(move || {
            loop {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if sender.send(Event::Connection(stream)).is_err() {
                            return;
                        }
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            io::ErrorKind::Interrupted | io::ErrorKind::ConnectionAborted
                        ) => {}
                    Err(_) => {
                        let _ = sender.send(Event::End);
                        return;
                    }
                }
            }
        })
        .map(drop)
}

/// Serve one connection, reading its request within `request_timeout`; whether the session was
/// closed by it.
fn serve<T: surf::Transport>(
    session: &mut Session<T>,
    stream: UnixStream,
    request_timeout: Duration,
) -> bool {
    let deadline = Instant::now() + request_timeout;
    let request = {
        let mut reader = BufReader::new(Deadline {
            stream: &stream,
            deadline,
        });
        wire::read_request(&mut reader)
    };
    let reply = match request {
        Ok(request) => session.answer(&request),
        Err(RequestError::TooLarge | RequestError::Closed | RequestError::Io(_)) => return false,
        Err(_) => session.malformed(),
    };
    let closed = reply.status == 0 && reply.state == link::SessionState::Closed;
    if let Ok(bytes) = reply.to_link().encode() {
        write_by(&stream, &bytes, Instant::now() + WRITE_TIMEOUT);
    }
    closed
}

/// Reads from the stream, each read given the time left before the deadline.
struct Deadline<'a> {
    stream: &'a UnixStream,
    deadline: Instant,
}

impl Read for Deadline<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let left = self.deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::ErrorKind::TimedOut.into());
        }
        self.stream.set_read_timeout(Some(left))?;
        let mut stream = self.stream;
        stream.read(buffer)
    }
}

/// Write `bytes` by `deadline`, or stop writing.
fn write_by(stream: &UnixStream, mut bytes: &[u8], deadline: Instant) {
    let mut stream = stream;
    while !bytes.is_empty() {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() || stream.set_write_timeout(Some(left)).is_err() {
            return;
        }
        match stream.write(bytes) {
            Ok(0) => return,
            Ok(n) => bytes = &bytes[n..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return,
        }
    }
}
