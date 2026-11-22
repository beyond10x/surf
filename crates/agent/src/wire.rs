//! The wire between `surf-agent` and `surfd`: one request and one answer per connection, over
//! the Unix socket `$XDG_RUNTIME_DIR/surf/surfd.sock` ([`socket_path`]). `surfd` and the session
//! process read and write requests with this module; nothing else defines the format.
//!
//! Every byte of a request and of an answer is printable ASCII (U+0020 to U+007E) or a line feed
//! (U+000A), and every line ends in a line feed; the one exception is a body line of a `text`
//! block in the answer lines (it starts with `>`), which may also hold UTF-8 characters above
//! U+007F that [`surf_answer::escaped_in_text`] does not name. A value from outside (an address,
//! a path, a reason) is written only through [`surf_answer::quote`] and read back only through
//! [`surf_answer::unquote`].
//!
//! # The request
//!
//! ```text
//! surf-wire 1
//! command <name>
//! session <id>
//! token <64 lower-case hexadecimal digits>
//! arg <name> "<value>"
//! end
//! ```
//!
//! - `surf-wire 1`, the version line ([`VERSION_LINE`]), always first.
//! - `command <name>`: a word of `[a-z]+` (`open`, `go`, `look`, `close`).
//! - `session <id>`: the session identity, a UUID in text form, which the client writes in lower
//!   case. Absent for `open`. [`Request::decode`] keeps the word as it was sent (any printable
//!   ASCII without a space); the receiver reads it with [`session_id`], so that an identity that
//!   is not a UUID reaches it and can be answered.
//! - `token <64 digits>`: the session's token, read with [`surf_secret::Token::from_hex`] and held
//!   only as a [`surf_secret::Token`]. Absent for `open`.
//! - `arg <name> "<value>"`: one line per argument, in any order, each name once; the name a word
//!   of `[a-z]+`, the value a field of [`surf_answer::quote`]. `open [<url>]` and `go <url>` send
//!   `arg url`; `--as <form>` is `arg form` (`tree` or `markdown`) and `--from-line <n>` is
//!   `arg line` (decimal digits), on `open <url>`, `go` and `look`. A later argument is a new name and needs no new version; the receiver answers an
//!   argument name it does not know with status 2 ([`Request::unknown_arg`]).
//! - `end`: the request is complete. The receiver reads nothing after it.
//!
//! The lines come in this order, `session` and `token` each at most once, `session` before
//! `token`. A request is at most [`REQUEST_MAX`] bytes, `end` included; the receiver reads it
//! within [`REQUEST_TIMEOUT`] ([`read_request`]).
//!
//! # The answer
//!
//! ```text
//! surf-wire 1
//! status <0, 1, 2 or 3>
//! reason "<text>"
//! answer <n>
//! <n bytes: the answer lines>
//! ```
//!
//! - `status`: the exit status of the call (design section 4): 0 the action was taken, 1
//!   refused, 2 usage, 3 the session could not be reached; for a refusal the status the reason
//!   table of `docs/agent-answer.md` gives. The client exits with it.
//! - `reason "<text>"`: at most once, absent when there is none: a sentence for standard error,
//!   quoted by [`surf_answer::quote`].
//! - `answer <n>`: the number of bytes that follow, at most [`ANSWER_TEXT_MAX`]; then exactly
//!   those bytes, the answer lines of `docs/agent-answer.md` (printable ASCII but for the body
//!   lines of a `text` block, each line ending in a line feed; zero bytes for an answer of no
//!   lines). The client prints them unchanged.
//!
//! The receiver closes the connection after the answer. The client reads to the end of the
//! connection, at most [`ANSWER_MAX`] bytes, within [`ANSWER_TIMEOUT`] ([`read_answer`]), and
//! treats anything else (no answer, a longer one, one that does not decode, a timeout) as status
//! 3.
//!
//! # Starting `surfd`
//!
//! `surf-agent open` that finds no listener on the socket starts the program `surfd` from the
//! directory of its own executable, with no argument, in a process group of its own, standard
//! input and standard error discarded and standard output a pipe. `surfd` writes one line on
//! that pipe: [`READY`] once its socket listens, or [`REFUSED`] followed by a space and a reason
//! when it does not start. The line is at most [`START_LINE_MAX`] bytes and comes within
//! [`START_TIMEOUT`]; a `surfd` that writes nothing in time is killed and `open` exits 3.

use std::fmt;
use std::fs;
use std::io::{self, BufRead, Read};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use surf_secret::Token;

/// The first line of every request and every answer.
pub const VERSION_LINE: &str = "surf-wire 1";

/// The most bytes of one request, its `end` line included: 64 KiB.
pub const REQUEST_MAX: usize = 64 * 1024;

/// The most bytes of the answer lines of one answer: 4 MiB, the limit of an answer on the link
/// between `surfd` and a session.
pub const ANSWER_TEXT_MAX: usize = 4 * 1024 * 1024;

/// The most bytes of one whole answer: [`ANSWER_TEXT_MAX`] and 64 KiB for the lines before it.
pub const ANSWER_MAX: usize = ANSWER_TEXT_MAX + 64 * 1024;

/// How long the client waits for an answer: 75 seconds, longer than `surfd` waits for a session
/// (60 seconds).
pub const ANSWER_TIMEOUT: Duration = Duration::from_secs(75);

/// How long the receiver waits for a whole request: 10 seconds.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// How long `open` waits for the one line of a `surfd` it started: 5 seconds.
pub const START_TIMEOUT: Duration = Duration::from_secs(5);

/// The most bytes of the line a starting `surfd` writes, its line feed included.
pub const START_LINE_MAX: usize = 4096;

/// The line of a `surfd` whose socket listens.
pub const READY: &str = "ready";

/// The first word of the line of a `surfd` that does not start; a space and the reason follow.
pub const REFUSED: &str = "refused";

/// The most bytes of a Unix socket path: 107. Linux holds the path in 108 bytes with a closing
/// zero byte; the standard library refuses to bind or connect to a longer one ("path must be
/// shorter than SUN_LEN", measured with rustc 1.99.1 on Linux, 2026-11-21).
pub const SOCKET_PATH_MAX: usize = 107;

/// The directory under `$XDG_RUNTIME_DIR` that holds `surfd`'s socket.
pub const DIRECTORY: &str = "surf";

/// The name of `surfd`'s socket in [`DIRECTORY`].
pub const SOCKET: &str = "surfd.sock";

// --- The request --------------------------------------------------------------------------------

/// One request. Its `Debug` writes the token as `Token(..)`.
#[derive(Clone, Debug)]
pub struct Request {
    /// The command, a word of `[a-z]+`.
    pub command: String,
    /// The session identity as sent; `None` for `open`. Read it with [`session_id`].
    pub session: Option<String>,
    /// The token; `None` for `open`.
    pub token: Option<Token>,
    /// The arguments, in the order sent, each name once.
    pub args: Vec<Arg>,
}

/// One `arg` line: a name and its exact value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arg {
    /// The argument name, a word of `[a-z]+`.
    pub name: String,
    /// The value, unquoted.
    pub value: String,
}

/// Why a request was not read or not written.
#[derive(Debug)]
pub enum RequestError {
    /// The first line is not [`VERSION_LINE`].
    Version,
    /// A line is out of the order of the format or not one of its lines; its number, from 1.
    Line(usize),
    /// The command is not a word of `[a-z]+`.
    Command,
    /// The session is empty or holds a space.
    Session,
    /// The token is not 64 lower-case hexadecimal digits. The text is not kept.
    Token,
    /// An argument name is not a word of `[a-z]+`, or its value is not a quoted field.
    Arg(usize),
    /// An argument name comes twice.
    DuplicateArg(String),
    /// A byte outside printable ASCII and the line feed, or a line without its line feed.
    Unprintable,
    /// No `end` line.
    NoEnd,
    /// Bytes after the `end` line.
    AfterEnd,
    /// More than [`REQUEST_MAX`] bytes.
    TooLarge,
    /// The connection ended before `end`.
    Closed,
    /// Reading or writing failed; a timeout is `WouldBlock` or `TimedOut`.
    Io(io::Error),
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::Version => write!(f, "the request does not start with {VERSION_LINE}"),
            RequestError::Line(n) => write!(f, "line {n} of the request is not in the format"),
            RequestError::Command => f.write_str("the command is not a word of a to z"),
            RequestError::Session => f.write_str("the session is not one word"),
            RequestError::Token => f.write_str("the token is not 64 lower-case hexadecimal digits"),
            RequestError::Arg(n) => write!(f, "line {n} of the request is not an argument"),
            RequestError::DuplicateArg(name) => write!(f, "the argument {name} comes twice"),
            RequestError::Unprintable => {
                f.write_str("the request holds a byte outside printable ASCII")
            }
            RequestError::NoEnd => f.write_str("the request has no end line"),
            RequestError::AfterEnd => f.write_str("bytes follow the end line"),
            RequestError::TooLarge => write!(f, "the request is longer than {REQUEST_MAX} bytes"),
            RequestError::Closed => f.write_str("the connection ended before the request did"),
            RequestError::Io(error) => write!(f, "reading the request failed: {error}"),
        }
    }
}

impl std::error::Error for RequestError {}

impl Request {
    /// The value of the argument `name`, if it was sent.
    pub fn arg(&self, name: &str) -> Option<&str> {
        self.args
            .iter()
            .find(|arg| arg.name == name)
            .map(|arg| arg.value.as_str())
    }

    /// The first argument whose name is not in `known`: the receiver answers it with status 2.
    pub fn unknown_arg(&self, known: &[&str]) -> Option<&str> {
        self.args
            .iter()
            .map(|arg| arg.name.as_str())
            .find(|name| !known.contains(name))
    }

    /// The bytes of the request, `end` line included. Refuses a command, a session or an
    /// argument name the format cannot carry, and a request over [`REQUEST_MAX`].
    pub fn encode(&self) -> Result<Vec<u8>, RequestError> {
        if !is_word(&self.command) {
            return Err(RequestError::Command);
        }
        let mut text = format!("{VERSION_LINE}\ncommand {}\n", self.command);
        if let Some(session) = &self.session {
            if !is_session_word(session) {
                return Err(RequestError::Session);
            }
            text.push_str(&format!("session {session}\n"));
        }
        if let Some(token) = &self.token {
            text.push_str("token ");
            text.push_str(&token.to_hex());
            text.push('\n');
        }
        for (index, arg) in self.args.iter().enumerate() {
            if !is_word(&arg.name) {
                return Err(RequestError::Arg(index));
            }
            if self.args[..index]
                .iter()
                .any(|earlier| earlier.name == arg.name)
            {
                return Err(RequestError::DuplicateArg(arg.name.clone()));
            }
            text.push_str(&format!(
                "arg {} {}\n",
                arg.name,
                surf_answer::quote(&arg.value)
            ));
        }
        text.push_str("end\n");
        if text.len() > REQUEST_MAX {
            return Err(RequestError::TooLarge);
        }
        Ok(text.into_bytes())
    }

    /// The request in `bytes`, which hold exactly one request, `end` line included.
    pub fn decode(bytes: &[u8]) -> Result<Request, RequestError> {
        if bytes.len() > REQUEST_MAX {
            return Err(RequestError::TooLarge);
        }
        if !bytes
            .iter()
            .all(|&b| b == b'\n' || (b' '..=b'~').contains(&b))
        {
            return Err(RequestError::Unprintable);
        }
        let text = std::str::from_utf8(bytes).map_err(|_| RequestError::Unprintable)?;
        let Some(body) = text.strip_suffix('\n') else {
            return Err(if text.is_empty() {
                RequestError::Closed
            } else {
                RequestError::Unprintable
            });
        };
        let mut lines = body
            .split('\n')
            .enumerate()
            .map(|(index, line)| (index + 1, line));
        if lines.next().map(|(_, line)| line) != Some(VERSION_LINE) {
            return Err(RequestError::Version);
        }
        let command = match lines.next() {
            Some((_, line)) if line.starts_with("command ") => {
                let command = &line["command ".len()..];
                if !is_word(command) {
                    return Err(RequestError::Command);
                }
                command.to_string()
            }
            Some((number, _)) => return Err(RequestError::Line(number)),
            None => return Err(RequestError::NoEnd),
        };
        let mut request = Request {
            command,
            session: None,
            token: None,
            args: Vec::new(),
        };
        let mut ended = false;
        for (number, line) in lines {
            if ended {
                return Err(RequestError::AfterEnd);
            }
            if line == "end" {
                ended = true;
            } else if let Some(session) = line.strip_prefix("session ") {
                if request.session.is_some() || request.token.is_some() || !request.args.is_empty()
                {
                    return Err(RequestError::Line(number));
                }
                if !is_session_word(session) {
                    return Err(RequestError::Session);
                }
                request.session = Some(session.to_string());
            } else if let Some(token) = line.strip_prefix("token ") {
                if request.token.is_some() || !request.args.is_empty() {
                    return Err(RequestError::Line(number));
                }
                request.token = Some(Token::from_hex(token).ok_or(RequestError::Token)?);
            } else if let Some(rest) = line.strip_prefix("arg ") {
                let (name, field) = rest.split_once(' ').ok_or(RequestError::Arg(number))?;
                if !is_word(name) {
                    return Err(RequestError::Arg(number));
                }
                let value = surf_answer::unquote(field).map_err(|_| RequestError::Arg(number))?;
                if request.arg(name).is_some() {
                    return Err(RequestError::DuplicateArg(name.to_string()));
                }
                request.args.push(Arg {
                    name: name.to_string(),
                    value,
                });
            } else {
                return Err(RequestError::Line(number));
            }
        }
        if !ended {
            return Err(RequestError::NoEnd);
        }
        Ok(request)
    }
}

/// Read one request from `reader`, up to and including its `end` line and nothing after it, at
/// most [`REQUEST_MAX`] bytes. A request over the limit is [`RequestError::TooLarge`] as soon as
/// the limit is passed. The caller sets the timeout on the stream ([`REQUEST_TIMEOUT`]).
pub fn read_request(reader: &mut impl BufRead) -> Result<Request, RequestError> {
    let mut bytes = Vec::new();
    loop {
        let start = bytes.len();
        let room = (REQUEST_MAX - start) as u64;
        let read = reader
            .by_ref()
            .take(room + 1)
            .read_until(b'\n', &mut bytes)
            .map_err(RequestError::Io)?;
        if bytes.len() > REQUEST_MAX {
            return Err(RequestError::TooLarge);
        }
        if read == 0 || bytes.last() != Some(&b'\n') {
            return Err(RequestError::Closed);
        }
        if &bytes[start..] == b"end\n" {
            return Request::decode(&bytes);
        }
    }
}

// --- The answer ---------------------------------------------------------------------------------

/// One answer: the exit status, the reason for standard error, and the answer lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    /// The exit status, 0 to 3.
    pub status: u8,
    /// A sentence for standard error, if any.
    pub reason: Option<String>,
    /// The answer lines, each ending in a line feed, printed unchanged.
    pub text: String,
}

/// Why an answer was not read or not written.
#[derive(Debug)]
pub enum AnswerError {
    /// The connection ended with no byte of an answer.
    Empty,
    /// More than [`ANSWER_MAX`] bytes, or an `answer` count over [`ANSWER_TEXT_MAX`].
    TooLarge,
    /// No answer within the time given.
    Timeout,
    /// The bytes are not an answer of the format: what is wrong.
    Malformed(&'static str),
    /// Reading or writing failed.
    Io(io::Error),
}

impl fmt::Display for AnswerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnswerError::Empty => f.write_str("the connection closed without an answer"),
            AnswerError::TooLarge => write!(f, "the answer is longer than {ANSWER_MAX} bytes"),
            AnswerError::Timeout => f.write_str("no answer in time"),
            AnswerError::Malformed(what) => write!(f, "the answer is not in the format: {what}"),
            AnswerError::Io(error) => write!(f, "reading the answer failed: {error}"),
        }
    }
}

impl std::error::Error for AnswerError {}

impl Answer {
    /// The bytes of the answer. Refuses a status above 3, answer lines that are not printable
    /// ASCII ending in a line feed, and answer lines over [`ANSWER_TEXT_MAX`].
    pub fn encode(&self) -> Result<Vec<u8>, AnswerError> {
        if self.status > 3 {
            return Err(AnswerError::Malformed("a status above 3"));
        }
        check_text(self.text.as_bytes())?;
        let mut out = format!("{VERSION_LINE}\nstatus {}\n", self.status);
        if let Some(reason) = &self.reason {
            out.push_str(&format!("reason {}\n", surf_answer::quote(reason)));
        }
        out.push_str(&format!("answer {}\n", self.text.len()));
        let mut bytes = out.into_bytes();
        bytes.extend_from_slice(self.text.as_bytes());
        Ok(bytes)
    }

    /// The answer in `bytes`, which hold exactly one answer.
    pub fn decode(bytes: &[u8]) -> Result<Answer, AnswerError> {
        if bytes.is_empty() {
            return Err(AnswerError::Empty);
        }
        if bytes.len() > ANSWER_MAX {
            return Err(AnswerError::TooLarge);
        }
        let mut rest = bytes;
        if next_line(&mut rest)? != VERSION_LINE {
            return Err(AnswerError::Malformed("no version line"));
        }
        let status = match next_line(&mut rest)? {
            "status 0" => 0,
            "status 1" => 1,
            "status 2" => 2,
            "status 3" => 3,
            _ => return Err(AnswerError::Malformed("no status of 0 to 3")),
        };
        let mut line = next_line(&mut rest)?;
        let mut reason = None;
        if let Some(field) = line.strip_prefix("reason ") {
            reason = Some(
                surf_answer::unquote(field)
                    .map_err(|_| AnswerError::Malformed("the reason is not a quoted field"))?,
            );
            line = next_line(&mut rest)?;
        }
        let count = line
            .strip_prefix("answer ")
            .filter(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
            .filter(|digits| digits.len() == 1 || !digits.starts_with('0'))
            .ok_or(AnswerError::Malformed("no answer count"))?;
        let count: usize = count.parse().map_err(|_| AnswerError::TooLarge)?;
        if count > ANSWER_TEXT_MAX {
            return Err(AnswerError::TooLarge);
        }
        if rest.len() != count {
            return Err(AnswerError::Malformed(
                "the answer lines are not as long as their count",
            ));
        }
        check_text(rest)?;
        let text = String::from_utf8(rest.to_vec()).expect("lines check_text read as UTF-8");
        Ok(Answer {
            status,
            reason,
            text,
        })
    }
}

/// The next line of `rest`, without its line feed, which must be there; printable ASCII only.
fn next_line<'a>(rest: &mut &'a [u8]) -> Result<&'a str, AnswerError> {
    let current: &'a [u8] = rest;
    let end = current
        .iter()
        .position(|&b| b == b'\n')
        .ok_or(AnswerError::Malformed("a line without its line feed"))?;
    let line = &current[..end];
    *rest = &current[end + 1..];
    if !line.iter().all(|b| (b' '..=b'~').contains(b)) {
        return Err(AnswerError::Malformed("a byte outside printable ASCII"));
    }
    Ok(std::str::from_utf8(line).expect("printable ASCII"))
}

/// Answer lines: every line ending in a line feed, at most [`ANSWER_TEXT_MAX`] bytes, each line
/// [`is_answer_line`].
fn check_text(text: &[u8]) -> Result<(), AnswerError> {
    if text.len() > ANSWER_TEXT_MAX {
        return Err(AnswerError::TooLarge);
    }
    if !text.split(|&b| b == b'\n').all(is_answer_line) {
        return Err(AnswerError::Malformed(
            "answer lines outside printable ASCII and page text",
        ));
    }
    if !text.is_empty() && text.last() != Some(&b'\n') {
        return Err(AnswerError::Malformed(
            "the last answer line has no line feed",
        ));
    }
    Ok(())
}

/// Whether one answer line, without its line feed, keeps the Lines rule of
/// `docs/agent-answer.md`: printable ASCII, and in a body line of a `text` block (a line that
/// starts with `>`) also UTF-8 characters above U+007F that [`surf_answer::escaped_in_text`] does
/// not name (decision D1 B of `docs/design/page-as-text.md`).
fn is_answer_line(line: &[u8]) -> bool {
    if line.iter().all(|b| (b' '..=b'~').contains(b)) {
        return true;
    }
    line.first() == Some(&b'>')
        && std::str::from_utf8(line).is_ok_and(|text| {
            text.chars().all(|c| {
                (' '..='~').contains(&c) || (c > '\u{7f}' && !surf_answer::escaped_in_text(c))
            })
        })
}

/// Read one answer from `stream` to the end of the connection, at most [`ANSWER_MAX`] bytes, all
/// of it within `timeout` ([`ANSWER_TIMEOUT`] in the client). Reading stops at the first byte
/// over the limit.
pub fn read_answer(stream: &mut UnixStream, timeout: Duration) -> Result<Answer, AnswerError> {
    let deadline = Instant::now() + timeout;
    let mut bytes = Vec::new();
    let mut chunk = vec![0u8; 64 * 1024];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(AnswerError::Timeout);
        }
        stream
            .set_read_timeout(Some(left))
            .map_err(AnswerError::Io)?;
        let room = ANSWER_MAX + 1 - bytes.len();
        let want = room.min(chunk.len());
        match stream.read(&mut chunk[..want]) {
            Ok(0) => break,
            Ok(n) => {
                bytes.extend_from_slice(&chunk[..n]);
                if bytes.len() > ANSWER_MAX {
                    return Err(AnswerError::TooLarge);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Err(AnswerError::Timeout);
            }
            Err(error) if error.kind() == io::ErrorKind::ConnectionReset && bytes.is_empty() => {
                return Err(AnswerError::Empty);
            }
            Err(error) => return Err(AnswerError::Io(error)),
        }
    }
    Answer::decode(&bytes)
}

// --- Identities and paths -----------------------------------------------------------------------

/// The caller's user id: the owner of `/proc/self`, which Linux gives the process's effective
/// user id. The standard library has no call for it, and this crate uses no `unsafe` code.
///
/// Linux only: on a system without `/proc` it returns the error of reading `/proc/self`. One
/// definition for the client (the owner check of a token file) and `surfd` (the owner check of
/// its directory).
pub fn caller_uid() -> std::io::Result<u32> {
    fs::metadata("/proc/self").map(|metadata| metadata.uid())
}

/// The session identity in `text` if it is a UUID in text form (8, 4, 4, 4 and 12 hexadecimal
/// digits joined by `-`, of either case), written in lower case, the only form the answer
/// renderer writes; `None` for anything else.
pub fn session_id(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    if bytes.len() != 36 {
        return None;
    }
    for (index, &byte) in bytes.iter().enumerate() {
        let dash = matches!(index, 8 | 13 | 18 | 23);
        if dash != (byte == b'-') || (!dash && !byte.is_ascii_hexdigit()) {
            return None;
        }
    }
    Some(text.to_ascii_lowercase())
}

/// Why there is no socket path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketPathError {
    /// `XDG_RUNTIME_DIR` is empty or not an absolute path.
    NotAbsolute,
    /// The socket path is this many bytes, more than [`SOCKET_PATH_MAX`].
    TooLong(usize),
}

impl fmt::Display for SocketPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SocketPathError::NotAbsolute => f.write_str("XDG_RUNTIME_DIR is not an absolute path"),
            SocketPathError::TooLong(length) => write!(
                f,
                "the socket path $XDG_RUNTIME_DIR/{DIRECTORY}/{SOCKET} is {length} bytes long; \
                 a Unix socket path takes at most {SOCKET_PATH_MAX}"
            ),
        }
    }
}

impl std::error::Error for SocketPathError {}

/// `<runtime_dir>/surf/surfd.sock`, refused when `runtime_dir` is not absolute or the path is
/// over [`SOCKET_PATH_MAX`] bytes. Touches no path.
pub fn socket_path(runtime_dir: &Path) -> Result<PathBuf, SocketPathError> {
    if !runtime_dir.is_absolute() {
        return Err(SocketPathError::NotAbsolute);
    }
    let path = runtime_dir.join(DIRECTORY).join(SOCKET);
    let length = path.as_os_str().len();
    if length > SOCKET_PATH_MAX {
        return Err(SocketPathError::TooLong(length));
    }
    Ok(path)
}

/// What [`check_socket_path`] found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SocketCheck {
    /// A socket of the caller, in directories only the caller can enter.
    Present,
    /// No socket, or no directory [`DIRECTORY`] yet: no `surfd` has been started there.
    Absent,
}

/// Check what the client trusts before it connects to `<runtime_dir>/surf/surfd.sock` with a
/// token, looking at each path without following a symbolic link: `runtime_dir` and
/// [`DIRECTORY`] in it are directories, not symbolic links, owned by `caller`, with no access
/// for group and others; the socket, when it is there, is a socket owned by `caller`. Within
/// two such directories nobody but the caller (and the superuser) can put a socket in its
/// place. The error names the path and what is wrong. `runtime_dir` is first written without a
/// trailing `/`, a doubled `/` or a `.` component, so that `<link>/` is refused as the symbolic
/// link it is; a relative path and a `..` component are refused.
///
/// The peer of the connection is not checked: the standard library of rustc 1.99.1 reads the
/// peer's credentials only through `UnixStream::peer_cred`, which is unstable, and this crate
/// uses no `unsafe` code to call the operating system itself.
pub fn check_socket_path(runtime_dir: &Path, caller: u32) -> Result<SocketCheck, String> {
    let runtime_dir = plain_spelling(runtime_dir)?;
    let runtime_dir = runtime_dir.as_path();
    check_directory(runtime_dir, caller)?;
    let surf = runtime_dir.join(DIRECTORY);
    match fs::symlink_metadata(&surf) {
        Ok(_) => check_directory(&surf, caller)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(SocketCheck::Absent),
        Err(error) => return Err(format!("cannot look at {}: {error}", surf.display())),
    }
    let socket = surf.join(SOCKET);
    let metadata = match fs::symlink_metadata(&socket) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(SocketCheck::Absent),
        Err(error) => return Err(format!("cannot look at {}: {error}", socket.display())),
    };
    let kind = metadata.file_type();
    if kind.is_symlink() {
        return Err(format!("{} is a symbolic link", socket.display()));
    }
    if !kind.is_socket() {
        return Err(format!("{} is not a socket", socket.display()));
    }
    owned_by(&socket, &metadata, caller)?;
    Ok(SocketCheck::Present)
}

/// `path` without a trailing `/`, a doubled `/` or a `.` component, so that looking at it does not
/// follow a symbolic link in its last component: `<link>/`, `<link>//` and `<link>/.` name what
/// the link points to, `<link>` names the link. `/run/user/1000/` becomes `/run/user/1000`. A
/// `..` component is refused: after a symbolic link it leads to the parent of the link's target,
/// which the spelling does not show. A relative path is refused.
fn plain_spelling(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err(format!("{} is not an absolute path", path.display()));
    }
    if path
        .components()
        .any(|part| part == std::path::Component::ParentDir)
    {
        return Err(format!("{} has a .. component", path.display()));
    }
    Ok(path.components().collect())
}

/// `path` is a directory, not a symbolic link, of `caller`, with no access for group and others.
fn check_directory(path: &Path, caller: u32) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot look at {}: {error}", path.display()))?;
    let kind = metadata.file_type();
    if kind.is_symlink() {
        return Err(format!("{} is a symbolic link", path.display()));
    }
    if !kind.is_dir() {
        return Err(format!("{} is not a directory", path.display()));
    }
    owned_by(path, &metadata, caller)?;
    let mode = metadata.mode() & 0o7777;
    if mode & 0o077 != 0 {
        return Err(format!(
            "{} has mode {mode:04o}; group and others must have no access",
            path.display()
        ));
    }
    Ok(())
}

fn owned_by(path: &Path, metadata: &fs::Metadata, caller: u32) -> Result<(), String> {
    if metadata.uid() != caller {
        return Err(format!(
            "{} is owned by user {}, not by the caller, user {caller}",
            path.display(),
            metadata.uid()
        ));
    }
    Ok(())
}

fn is_word(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_lowercase())
}

fn is_session_word(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| (b'!'..=b'~').contains(&b))
}
