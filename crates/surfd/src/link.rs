//! The link between `surfd` and a session process, the one definition both sides use.
//!
//! # Start
//!
//! `surfd` starts the session program with two arguments, the session identity (36 bytes, lower
//! case) and the absolute path of the session socket, `$XDG_RUNTIME_DIR/surf/<identity>/
//! session.sock`; neither is secret. Its standard input is a pipe `surfd` holds: the session
//! ends itself, with status 0, when the pipe closes. Its standard output carries one line,
//! [`READY`] ([`write_ready`]), written once the session socket listens, and nothing else; its
//! standard error is discarded. Its environment is empty but the variables of
//! `Config::session_env`, none in the binary. It is started through `setpriv --no-new-privs` and
//! `prlimit` with the bounds of its form (`Config::session_bounds`, and
//! `Config::script_session_bounds` for an `open` with `arg scripts "true"`), both of which
//! replace themselves by the session program, so the process `surfd` holds is the session's.
//!
//! # One request
//!
//! Per request `surfd` makes one connection to the session socket and writes the request of
//! [`surf_agent::wire`] unchanged, token included, up to its `end` line. So the session's extra
//! trust anchors of `surf-agent open --ca-file` (`arg anchors` of the `open` request, the text
//! of the file) reach the session this way only, never as an argument of the session program or
//! in its environment ("Start"). The session writes one answer ([`Answer::encode`]) and closes
//! the connection:
//!
//! ```text
//! surf-link 1
//! status <0, 1, 2 or 3>
//! reason "<text>"
//! state <open, closed or unknown>
//! address "<the address, quoted>"
//! title "<the title, quoted>"
//! entries <n>
//! <n bytes: the entries>
//! answer <n>
//! <n bytes: the answer lines>
//! ```
//!
//! - `status`, `reason` and the answer lines are what `surfd` relays to the client as the
//!   answer of the wire; `reason` is at most once and absent when there is none.
//! - `state`, `address` and `title` are the status record: `closed` after the session was
//!   closed, `open` otherwise; the address and the title of the page as the session reports
//!   them, each a field of [`surf_answer::quote`], each absent when there is none. `surfd` keeps
//!   them quoted ([`Quoted`]) and does not read them. `unknown`, with no address and no title,
//!   is the record of a request the session refused without acting on it (a wrong token, a
//!   request that does not decode): it says nothing of the session, and `surfd` keeps what it
//!   knew before the request: the address, the title and the time of the last action.
//! - `entries <n>`, always there, `entries 0` for none: the entries of the request for the log
//!   `surfd` keeps ([`crate::log`]), at most [`ENTRIES_MAX`] bytes ([`Entry`], below).
//! - The answer lines are at most [`ANSWER_TEXT_MAX`] bytes; the lines of the record (every line
//!   but the bytes of the entries and the answer lines) at most [`RECORD_MAX`]; all of it
//!   printable ASCII and line feeds, every line ending in a line feed, but for the body lines of
//!   a `text` block in the answer lines and in the answer entry: a line that starts with `>` may
//!   also hold UTF-8 characters above U+007F that [`surf_answer::escaped_in_text`] does not name.
//!
//! # The entries
//!
//! The entries of one request, in the order of decision D19: for each action the session
//! recorded, its action entry, then the events its system published for it; then one answer
//! entry. A request that recorded no action (`look`, a refusal) has none. An action entry and an
//! event entry are one line each; an answer entry is a line `answer-entry <m>` and the `m` bytes
//! of its answer lines:
//!
//! ```text
//! action <sequence> <command>
//! action <sequence> <command> "<argument>"
//! event <qualified name> <field>="<value>" <field>= …
//! event <qualified name> unmapped
//! answer-entry <m>
//! <m bytes: the answer lines without the token line>
//! ```
//!
//! - An action entry is the action as the `path` line prints it (`docs/agent-answer.md`): its
//!   sequence number, its command, a word of `[a-z]+`, and its argument as a quoted field.
//! - An event entry names the event as `spec/` does (`surf.net.RequestStarted`), then every
//!   field of it in the order `spec/` declares them, each `<name>=` and its value as a quoted
//!   field of [`surf_answer::quote`], or nothing after `=` for an absent optional value; or the
//!   word `unmapped` and no field, for an event the session does not map.
//! - The block is empty, or it starts with an action entry and its one answer entry is its last.
//! - The answer entry is exactly the answer lines of the same link answer without their `token`
//!   line ([`without_token`]), and holds no `token` line.
//!
//! `surfd` checks the entries against these rules and keeps them as they arrived; it does not
//! read the quoted fields. It reads every link answer to its end, also one that comes after the
//! call's timeout (`crate::LATE_ANSWER_TIMEOUT`), so the entries of every request the session
//! answered reach the log in the order the session answered them.
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

/// The most bytes of the lines of a link answer's record, every line but the bytes of the
/// entries and the answer lines: 64 KiB.
pub const RECORD_MAX: usize = 64 * 1024;

/// The most bytes of the entries of one link answer: 8 MiB.
pub const ENTRIES_MAX: usize = 8 * 1024 * 1024;

/// The most bytes of one link answer: [`ANSWER_TEXT_MAX`] + [`RECORD_MAX`] + [`ENTRIES_MAX`].
pub const ANSWER_MAX: usize = ANSWER_TEXT_MAX + RECORD_MAX + ENTRIES_MAX;

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
    /// `unknown`: the request was refused before it was acted on, and the record holds nothing
    /// of the session; `surfd` keeps what it knew of it.
    Unknown,
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

/// The kind of an entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    /// An action the session recorded.
    Action,
    /// An event the session's system published.
    Event,
    /// The answer lines of the request, without the token line.
    Answer,
}

/// One entry of a link answer ("The entries", above).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Its kind.
    pub kind: EntryKind,
    /// For an action and an event entry its one line, line feed included; for an answer entry
    /// its answer lines. These are the bytes of the entry as the link carries them, without
    /// the `answer-entry <m>` line.
    pub text: String,
}

impl Entry {
    /// The action or event entry of `line` (without its line feed), by its first word; `None`
    /// for a line of neither kind.
    pub fn line(line: &str) -> Option<Entry> {
        let kind = line_kind(line)?;
        Some(Entry {
            kind,
            text: format!("{line}\n"),
        })
    }

    /// The answer entry of the answer lines `text`.
    pub fn answer(text: &str) -> Entry {
        Entry {
            kind: EntryKind::Answer,
            text: text.to_string(),
        }
    }

    /// Whether the entry keeps the rules of its kind.
    fn check(&self) -> Result<(), LinkError> {
        match self.kind {
            EntryKind::Action | EntryKind::Event => {
                let line = self
                    .text
                    .strip_suffix('\n')
                    .filter(|line| !line.contains('\n'))
                    .ok_or(LinkError::Malformed("an entry that is not one line"))?;
                if line_kind(line) != Some(self.kind) {
                    return Err(LinkError::Malformed("an entry line outside its grammar"));
                }
                Ok(())
            }
            EntryKind::Answer => {
                if self.text.is_empty() {
                    return Err(LinkError::Malformed("an empty answer entry"));
                }
                check_text(self.text.as_bytes())
            }
        }
    }
}

/// The kind of an action or event entry line, if it keeps the grammar of one.
fn line_kind(line: &str) -> Option<EntryKind> {
    if !line.bytes().all(|b| (b' '..=b'~').contains(&b)) {
        return None;
    }
    if let Some(rest) = line.strip_prefix("action ") {
        return action_line(rest).then_some(EntryKind::Action);
    }
    if let Some(rest) = line.strip_prefix("event ") {
        return event_line(rest).then_some(EntryKind::Event);
    }
    None
}

/// `<sequence> <command>` and an optional ` <quoted>`.
fn action_line(rest: &str) -> bool {
    read_action_rest(rest).is_some()
}

/// `<name>` and either ` unmapped` or any number of ` <field>=` with an optional quoted value.
fn event_line(rest: &str) -> bool {
    read_event_rest(rest).is_some()
}

/// An action entry line read into its parts ([`read_action`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionLine<'a> {
    /// The sequence number: decimal digits without a leading zero.
    pub sequence: &'a str,
    /// The command, a word of `[a-z]+`.
    pub command: &'a str,
    /// The argument as the quoted field the line holds, quotes included; `None` when the action
    /// has none.
    pub argument: Option<&'a str>,
}

/// An event entry line read into its parts ([`read_event`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventLine<'a> {
    /// The qualified event name.
    pub name: &'a str,
    /// Whether the line says `unmapped`; `fields` is then empty.
    pub unmapped: bool,
    /// Each field in the line's order: its name and its quoted field, quotes included, or
    /// `None` for an absent value.
    pub fields: Vec<(&'a str, Option<&'a str>)>,
}

/// The parts of an action entry line, `action <sequence> <command> ["<argument>"]`, with or
/// without its line feed; `None` for a line outside the grammar that [`Entry::line`] checks.
/// The one reading of an action line: the grammar check is this function. The argument stays
/// quoted; a reader takes its value through [`surf_answer::unquote`].
pub fn read_action(line: &str) -> Option<ActionLine<'_>> {
    let line = line.strip_suffix('\n').unwrap_or(line);
    if !printable(line) {
        return None;
    }
    read_action_rest(line.strip_prefix("action ")?)
}

/// The parts of an event entry line, `event <name> <field>="<value>" …` or `event <name>
/// unmapped`, with or without its line feed; `None` for a line outside the grammar that
/// [`Entry::line`] checks. The one reading of an event line: the grammar check is this
/// function. The values stay quoted; a reader takes each through [`surf_answer::unquote`].
pub fn read_event(line: &str) -> Option<EventLine<'_>> {
    let line = line.strip_suffix('\n').unwrap_or(line);
    if !printable(line) {
        return None;
    }
    read_event_rest(line.strip_prefix("event ")?)
}

fn printable(line: &str) -> bool {
    line.bytes().all(|b| (b' '..=b'~').contains(&b))
}

fn read_action_rest(rest: &str) -> Option<ActionLine<'_>> {
    let mut words = rest.splitn(3, ' ');
    let (Some(sequence), Some(command)) = (words.next(), words.next()) else {
        return None;
    };
    let argument = words.next();
    let argument_ok = argument.is_none_or(|field| surf_answer::unquote(field).is_ok());
    let ok = is_number(sequence)
        && !command.is_empty()
        && command.bytes().all(|b| b.is_ascii_lowercase())
        && argument_ok;
    ok.then_some(ActionLine {
        sequence,
        command,
        argument,
    })
}

fn read_event_rest(rest: &str) -> Option<EventLine<'_>> {
    let (name, mut fields) = rest.split_once(' ').unwrap_or((rest, ""));
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_')
    {
        return None;
    }
    let mut read = EventLine {
        name,
        unmapped: false,
        fields: Vec::new(),
    };
    if fields == "unmapped" {
        read.unmapped = true;
        return Some(read);
    }
    if rest.contains(' ') && fields.is_empty() {
        return None;
    }
    while !fields.is_empty() {
        let (field, after) = fields.split_once('=')?;
        if field.is_empty()
            || !field
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        {
            return None;
        }
        let (value, next) = match after.strip_prefix('"') {
            // A quoted field ends at the first `"` after its opening one.
            Some(inside) => {
                let end = inside.find('"')?;
                (&after[..end + 2], &after[end + 2..])
            }
            None => ("", after),
        };
        if !value.is_empty() && surf_answer::unquote(value).is_err() {
            return None;
        }
        read.fields
            .push((field, (!value.is_empty()).then_some(value)));
        fields = match next.strip_prefix(' ') {
            Some(more) if !more.is_empty() => more,
            Some(_) => return None,
            None if next.is_empty() => "",
            None => return None,
        };
    }
    Some(read)
}

/// A number of `docs/agent-answer.md`: decimal digits without a leading zero.
fn is_number(word: &str) -> bool {
    !word.is_empty()
        && word.bytes().all(|b| b.is_ascii_digit())
        && (word.len() == 1 || !word.starts_with('0'))
}

/// The bytes of `entries` as the link carries them, after checking each and their order.
fn encode_entries(entries: &[Entry]) -> Result<Vec<u8>, LinkError> {
    check_order(entries)?;
    let mut bytes = Vec::new();
    for entry in entries {
        entry.check()?;
        if entry.kind == EntryKind::Answer {
            bytes.extend_from_slice(format!("answer-entry {}\n", entry.text.len()).as_bytes());
        }
        bytes.extend_from_slice(entry.text.as_bytes());
        if bytes.len() > ENTRIES_MAX {
            return Err(LinkError::TooLarge);
        }
    }
    Ok(bytes)
}

/// The entries in `bytes`, which hold exactly the block.
fn decode_entries(mut rest: &[u8]) -> Result<Vec<Entry>, LinkError> {
    let mut entries = Vec::new();
    while !rest.is_empty() {
        let line = next_line(&mut rest)?;
        let entry = match line.strip_prefix("answer-entry ") {
            Some(count) => {
                if !is_number(count) {
                    return Err(LinkError::Malformed("no answer entry count"));
                }
                let count: usize = count.parse().map_err(|_| LinkError::TooLarge)?;
                if count > rest.len() {
                    return Err(LinkError::Malformed(
                        "an answer entry longer than the entries",
                    ));
                }
                let (text, after) = rest.split_at(count);
                rest = after;
                check_text(text)?;
                Entry::answer(
                    std::str::from_utf8(text)
                        .map_err(|_| LinkError::Malformed("an entry that is not UTF-8"))?,
                )
            }
            None => {
                Entry::line(line).ok_or(LinkError::Malformed("an entry outside the grammar"))?
            }
        };
        entry.check()?;
        entries.push(entry);
    }
    check_order(&entries)?;
    Ok(entries)
}

/// The answer lines `text` without the `token` line, which only the answer of `open` has and
/// which no entry keeps (decision D21): the one definition of an answer entry's bytes. No other
/// line of an answer starts with `token `: a quoted value never starts a line
/// (`docs/agent-answer.md`).
pub fn without_token(text: &str) -> String {
    text.split_inclusive('\n')
        .filter(|line| !line.starts_with("token "))
        .collect()
}

/// Every answer entry of `entries` (there is at most one) is exactly the answer lines `text`
/// without the `token` line, and holds no `token` line itself.
fn check_answer_entry(entries: &[Entry], text: &str) -> Result<(), LinkError> {
    for entry in entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::Answer)
    {
        if entry
            .text
            .split_inclusive('\n')
            .any(|line| line.starts_with("token "))
        {
            return Err(LinkError::Malformed(
                "an answer entry holding the token line",
            ));
        }
        if entry.text != without_token(text) {
            return Err(LinkError::Malformed(
                "an answer entry other than the answer lines without the token line",
            ));
        }
    }
    Ok(())
}

/// No entries, or an action entry first and the one answer entry last.
fn check_order(entries: &[Entry]) -> Result<(), LinkError> {
    let (Some(first), Some(last)) = (entries.first(), entries.last()) else {
        return Ok(());
    };
    let answers = entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::Answer)
        .count();
    if first.kind != EntryKind::Action || last.kind != EntryKind::Answer || answers != 1 {
        return Err(LinkError::Malformed(
            "entries that do not start with an action and end with the one answer entry",
        ));
    }
    Ok(())
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
    /// The entries for the log `surfd` keeps, in their order; none for a request that recorded
    /// no action.
    pub entries: Vec<Entry>,
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
        check_answer_entry(&self.entries, &self.text)?;
        let mut record = format!("{VERSION_LINE}\nstatus {}\n", self.status);
        if let Some(reason) = &self.reason {
            record.push_str(&format!("reason {}\n", surf_answer::quote(reason)));
        }
        record.push_str(match self.state {
            SessionState::Open => "state open\n",
            SessionState::Closed => "state closed\n",
            SessionState::Unknown => "state unknown\n",
        });
        if let Some(address) = &self.address {
            record.push_str(&format!("address {}\n", address.as_str()));
        }
        if let Some(title) = &self.title {
            record.push_str(&format!("title {}\n", title.as_str()));
        }
        let entries = encode_entries(&self.entries)?;
        record.push_str(&format!("entries {}\n", entries.len()));
        let answer_line = format!("answer {}\n", self.text.len());
        if record.len() + answer_line.len() > RECORD_MAX {
            return Err(LinkError::TooLarge);
        }
        let mut bytes = record.into_bytes();
        bytes.extend_from_slice(&entries);
        bytes.extend_from_slice(answer_line.as_bytes());
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
            "state unknown" => SessionState::Unknown,
            _ => return Err(LinkError::Malformed("no state of open, closed or unknown")),
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
        let entries_count = line
            .strip_prefix("entries ")
            .filter(|digits| is_number(digits))
            .ok_or(LinkError::Malformed("no entries count"))?;
        let head = bytes.len() - rest.len();
        if head > RECORD_MAX {
            return Err(LinkError::TooLarge);
        }
        let entries_count: usize = entries_count.parse().map_err(|_| LinkError::TooLarge)?;
        if entries_count > ENTRIES_MAX {
            return Err(LinkError::TooLarge);
        }
        if entries_count > rest.len() {
            return Err(LinkError::Malformed(
                "the entries are not as long as their count",
            ));
        }
        let (block, after) = rest.split_at(entries_count);
        let entries = decode_entries(block)?;
        rest = after;
        let before_answer = rest.len();
        let count = next_line(&mut rest)?
            .strip_prefix("answer ")
            .filter(|digits| is_number(digits))
            .ok_or(LinkError::Malformed("no answer count"))?;
        if head + (before_answer - rest.len()) > RECORD_MAX {
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
            .map_err(|_| LinkError::Malformed("answer lines that are not UTF-8"))?;
        check_answer_entry(&entries, &text)?;
        Ok(Answer {
            status,
            reason,
            state,
            address,
            title,
            entries,
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

/// Answer lines: each line ending in a line feed, at most [`ANSWER_TEXT_MAX`] bytes, each line
/// [`is_answer_line`].
fn check_text(text: &[u8]) -> Result<(), LinkError> {
    if text.len() > ANSWER_TEXT_MAX {
        return Err(LinkError::TooLarge);
    }
    if !text.split(|&b| b == b'\n').all(is_answer_line) {
        return Err(LinkError::Malformed(
            "answer lines outside printable ASCII and page text",
        ));
    }
    if !text.is_empty() && text.last() != Some(&b'\n') {
        return Err(LinkError::Malformed(
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

/// Read one link answer from `stream` to the end of the connection, at most [`ANSWER_MAX`]
/// bytes, all of it by `deadline`. Reading stops at the first byte over the limit.
pub fn read_answer(stream: &mut UnixStream, deadline: Instant) -> Result<Answer, LinkError> {
    let mut bytes = Vec::new();
    read_to_end(stream, &mut bytes, deadline)?;
    Answer::decode(&bytes)
}

/// Read the rest of one link answer from `stream` into `bytes`, which hold what was read of it
/// before, to the end of the connection, at most [`ANSWER_MAX`] bytes in all, by `deadline`.
/// After [`LinkError::Timeout`] `bytes` hold everything read so far, so that a later call can
/// go on reading the same answer.
pub fn read_to_end(
    stream: &mut UnixStream,
    bytes: &mut Vec<u8>,
    deadline: Instant,
) -> Result<(), LinkError> {
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
    Ok(())
}
