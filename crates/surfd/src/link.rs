//! The link between `surfd` and a session process, the one definition both sides use.
//!
//! # Start
//!
//! `surfd` starts the session program with two arguments, the session identity (36 bytes, lower
//! case) and the absolute path of the session socket, `$XDG_RUNTIME_DIR/surf/<identity>/
//! session.sock`; neither is secret. Its standard input is a pipe `surfd` holds: the session
//! ends itself, with status 0, when the pipe closes. Its standard output carries one line,
//! [`READY`] ([`write_ready`]), written once the session socket listens, and nothing else; its
//! standard error is discarded.
//!
//! # One request
//!
//! Per request `surfd` makes one connection to the session socket and writes the request of
//! [`surf_agent::wire`] unchanged, token included, up to its `end` line. The session writes one
//! answer ([`Answer::encode`]) and closes the connection:
//!
//! ```text
//! surf-link 1
//! status <0, 1, 2 or 3>
//! reason "<text>"
//! state <open or closed>
//! address "<the address, quoted>"
//! title "<the title, quoted>"
//! answer <n>
//! <n bytes: the answer lines>
//! ```
//!
//! - `status`, `reason` and the answer lines are what `surfd` relays to the client as the
//!   answer of the wire; `reason` is at most once and absent when there is none.
//! - `state`, `address` and `title` are the status record: `closed` after the session was
//!   closed, `open` otherwise; the address and the title of the page as the session reports
//!   them, each a field of [`surf_answer::quote`], each absent when there is none. `surfd` keeps
//!   them quoted ([`Quoted`]) and does not read them.
//! - The answer lines are at most [`ANSWER_TEXT_MAX`] bytes; everything before them at most
//!   [`RECORD_MAX`]; all of it printable ASCII and line feeds, every line ending in a line feed.
//!
//! A session that breaks any of these rules (a byte on standard output after `ready`, an answer
//! that does not decode, one over a limit, a connection closed in the middle of an answer, a
//! session socket that is not a socket) is killed by `surfd` and listed `died`.

use std::fmt;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Instant;

use surf_agent::wire;

/// The first line of every link answer.
pub const VERSION_LINE: &str = "surf-link 1";

/// The line a session writes on standard output once its socket listens.
pub const READY: &str = wire::READY;

/// The most bytes of the answer lines of one link answer: [`wire::ANSWER_TEXT_MAX`], 4 MiB, the
/// same limit the client takes, so that no answer passes the link and is then refused.
pub const ANSWER_TEXT_MAX: usize = wire::ANSWER_TEXT_MAX;

/// The most bytes of a link answer before its answer lines: 64 KiB.
pub const RECORD_MAX: usize = 64 * 1024;

/// The most bytes of one link answer: [`wire::ANSWER_MAX`].
pub const ANSWER_MAX: usize = wire::ANSWER_MAX;

/// The name of the session socket in the session's directory.
pub const SESSION_SOCKET: &str = "session.sock";

/// Write the [`READY`] line and flush it.
pub fn write_ready(out: &mut impl Write) -> io::Result<()> {
    out.write_all(READY.as_bytes())?;
    out.write_all(b"\n")?;
    out.flush()
}

/// The state of the status record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState {
    /// `open`: the session goes on.
    Open,
    /// `closed`: the session was closed by this request and ends.
    Closed,
}

/// A quoted field of [`surf_answer::quote`], kept as it arrived: printable ASCII, read back only
/// through [`surf_answer::unquote`] ([`Quoted::value`]).
#[derive(Clone, PartialEq, Eq)]
pub struct Quoted(String);

impl Quoted {
    /// The field of `value`.
    pub fn new(value: &str) -> Quoted {
        Quoted(surf_answer::quote(value))
    }

    /// `field` if it is a field [`surf_answer::quote`] can have written.
    pub fn from_field(field: &str) -> Option<Quoted> {
        surf_answer::unquote(field)
            .ok()
            .map(|_| Quoted(field.to_string()))
    }

    /// The field, quotes included.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The exact value, through [`surf_answer::unquote`].
    pub fn value(&self) -> String {
        surf_answer::unquote(&self.0).unwrap_or_default()
    }
}

impl fmt::Debug for Quoted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One link answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    /// The exit status of the call, 0 to 3.
    pub status: u8,
    /// A sentence for standard error, if any.
    pub reason: Option<String>,
    /// The state of the status record.
    pub state: SessionState,
    /// The address of the page, if the session has one.
    pub address: Option<Quoted>,
    /// The title of the page, if it has one.
    pub title: Option<Quoted>,
    /// The answer lines, each ending in a line feed.
    pub text: String,
}

/// Why a link answer was not read or not written.
#[derive(Debug)]
pub enum LinkError {
    /// The connection ended with no byte of an answer.
    Empty,
    /// Over [`ANSWER_MAX`], or answer lines over [`ANSWER_TEXT_MAX`], or a record over
    /// [`RECORD_MAX`].
    TooLarge,
    /// No answer by the deadline.
    Timeout,
    /// Not an answer of the link: what is wrong.
    Malformed(&'static str),
    /// Reading or writing failed.
    Io(io::Error),
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinkError::Empty => f.write_str("the connection closed without an answer"),
            LinkError::TooLarge => f.write_str("the answer is over a limit of the link"),
            LinkError::Timeout => f.write_str("no answer in time"),
            LinkError::Malformed(what) => write!(f, "the answer is not of the link: {what}"),
            LinkError::Io(error) => write!(f, "the link failed: {error}"),
        }
    }
}

impl std::error::Error for LinkError {}

impl Answer {
    /// The bytes of the answer; refuses what [`Answer::decode`] would refuse.
    pub fn encode(&self) -> Result<Vec<u8>, LinkError> {
        if self.status > 3 {
            return Err(LinkError::Malformed("a status above 3"));
        }
        check_text(self.text.as_bytes())?;
        let mut record = format!("{VERSION_LINE}\nstatus {}\n", self.status);
        if let Some(reason) = &self.reason {
            record.push_str(&format!("reason {}\n", surf_answer::quote(reason)));
        }
        record.push_str(match self.state {
            SessionState::Open => "state open\n",
            SessionState::Closed => "state closed\n",
        });
        if let Some(address) = &self.address {
            record.push_str(&format!("address {}\n", address.as_str()));
        }
        if let Some(title) = &self.title {
            record.push_str(&format!("title {}\n", title.as_str()));
        }
        record.push_str(&format!("answer {}\n", self.text.len()));
        if record.len() > RECORD_MAX {
            return Err(LinkError::TooLarge);
        }
        let mut bytes = record.into_bytes();
        bytes.extend_from_slice(self.text.as_bytes());
        Ok(bytes)
    }

    /// The answer in `bytes`, which hold exactly one answer.
    pub fn decode(bytes: &[u8]) -> Result<Answer, LinkError> {
        if bytes.is_empty() {
            return Err(LinkError::Empty);
        }
        if bytes.len() > ANSWER_MAX {
            return Err(LinkError::TooLarge);
        }
        let mut rest = bytes;
        if next_line(&mut rest)? != VERSION_LINE {
            return Err(LinkError::Malformed("no version line"));
        }
        let status = match next_line(&mut rest)? {
            "status 0" => 0,
            "status 1" => 1,
            "status 2" => 2,
            "status 3" => 3,
            _ => return Err(LinkError::Malformed("no status of 0 to 3")),
        };
        let mut line = next_line(&mut rest)?;
        let mut reason = None;
        if let Some(field) = line.strip_prefix("reason ") {
            reason = Some(
                surf_answer::unquote(field)
                    .map_err(|_| LinkError::Malformed("the reason is not a quoted field"))?,
            );
            line = next_line(&mut rest)?;
        }
        let state = match line {
            "state open" => SessionState::Open,
            "state closed" => SessionState::Closed,
            _ => return Err(LinkError::Malformed("no state of open or closed")),
        };
        line = next_line(&mut rest)?;
        let mut address = None;
        if let Some(field) = line.strip_prefix("address ") {
            address = Some(
                Quoted::from_field(field)
                    .ok_or(LinkError::Malformed("the address is not a quoted field"))?,
            );
            line = next_line(&mut rest)?;
        }
        let mut title = None;
        if let Some(field) = line.strip_prefix("title ") {
            title = Some(
                Quoted::from_field(field)
                    .ok_or(LinkError::Malformed("the title is not a quoted field"))?,
            );
            line = next_line(&mut rest)?;
        }
        let count = line
            .strip_prefix("answer ")
            .filter(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
            .filter(|digits| digits.len() == 1 || !digits.starts_with('0'))
            .ok_or(LinkError::Malformed("no answer count"))?;
        if bytes.len() - rest.len() > RECORD_MAX {
            return Err(LinkError::TooLarge);
        }
        let count: usize = count.parse().map_err(|_| LinkError::TooLarge)?;
        if count > ANSWER_TEXT_MAX {
            return Err(LinkError::TooLarge);
        }
        if rest.len() != count {
            return Err(LinkError::Malformed(
                "the answer lines are not as long as their count",
            ));
        }
        check_text(rest)?;
        let text = String::from_utf8(rest.to_vec())
            .map_err(|_| LinkError::Malformed("answer lines outside printable ASCII"))?;
        Ok(Answer {
            status,
            reason,
            state,
            address,
            title,
            text,
        })
    }
}

/// The next line of `rest` without its line feed, which must be there; printable ASCII only.
fn next_line<'a>(rest: &mut &'a [u8]) -> Result<&'a str, LinkError> {
    let current: &'a [u8] = rest;
    let end = current
        .iter()
        .position(|&b| b == b'\n')
        .ok_or(LinkError::Malformed("a line without its line feed"))?;
    let line = &current[..end];
    *rest = &current[end + 1..];
    if !line.iter().all(|b| (b' '..=b'~').contains(b)) {
        return Err(LinkError::Malformed("a byte outside printable ASCII"));
    }
    std::str::from_utf8(line).map_err(|_| LinkError::Malformed("a byte outside printable ASCII"))
}

/// Answer lines: printable ASCII, each line ending in a line feed, at most [`ANSWER_TEXT_MAX`].
fn check_text(text: &[u8]) -> Result<(), LinkError> {
    if text.len() > ANSWER_TEXT_MAX {
        return Err(LinkError::TooLarge);
    }
    if !text
        .iter()
        .all(|&b| b == b'\n' || (b' '..=b'~').contains(&b))
    {
        return Err(LinkError::Malformed("answer lines outside printable ASCII"));
    }
    if !text.is_empty() && text.last() != Some(&b'\n') {
        return Err(LinkError::Malformed(
            "the last answer line has no line feed",
        ));
    }
    Ok(())
}

/// Read one link answer from `stream` to the end of the connection, at most [`ANSWER_MAX`]
/// bytes, all of it by `deadline`. Reading stops at the first byte over the limit.
pub fn read_answer(stream: &mut UnixStream, deadline: Instant) -> Result<Answer, LinkError> {
    let mut bytes = Vec::new();
    let mut chunk = vec![0u8; 64 * 1024];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(LinkError::Timeout);
        }
        stream.set_read_timeout(Some(left)).map_err(LinkError::Io)?;
        let room = ANSWER_MAX + 1 - bytes.len();
        let want = room.min(chunk.len());
        match stream.read(&mut chunk[..want]) {
            Ok(0) => break,
            Ok(n) => {
                bytes.extend_from_slice(&chunk[..n]);
                if bytes.len() > ANSWER_MAX {
                    return Err(LinkError::TooLarge);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Err(LinkError::Timeout);
            }
            Err(error) if error.kind() == io::ErrorKind::ConnectionReset => {
                if bytes.is_empty() {
                    return Err(LinkError::Empty);
                }
                return Err(LinkError::Malformed(
                    "the connection ended inside the answer",
                ));
            }
            Err(error) => return Err(LinkError::Io(error)),
        }
    }
    Answer::decode(&bytes)
}
