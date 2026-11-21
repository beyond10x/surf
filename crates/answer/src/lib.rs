//! The agent answer of surf: the typed blocks, one function that renders them to lines, one
//! quoting function for every value from a page, a server or an address, and the reverse of
//! both, [`unquote`] and [`parse`].
//!
//! The grammar is `docs/agent-answer.md`. Every line is printable ASCII (U+0020 to U+007E) and
//! ends in a line feed. A value from outside is written only through [`quote`] (or
//! [`quote_cut`] for a field with a length limit), so it stands inside double quotes and holds
//! no `"`, no `\` and no character outside U+0020 to U+007E. Every other part of a line is a
//! number or a word of the format, and [`render`] refuses a word outside its character set
//! rather than write it.
//!
//! [`parse`] accepts exactly what [`render`] writes: parsing a rendered answer gives the value
//! back, and every answer [`parse`] accepts renders to the same text. `surfd`, the live view and
//! the scenario runner read an answer only through [`parse`] and [`unquote`] (decision D20 of
//! `docs/plan/agent-plan-revision-2.md`).
//!
//! # Adding a block
//!
//! A block is one field of [`Answer`], one `render_<block>` function called from [`render`] and
//! one `parse_<block>` function in the list `BLOCKS`, both in the block's place in the order of
//! the answer, plus a golden file and a round-trip case under `tests/`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt;

/// The version of the answer format, the second word of the head line.
pub const VERSION: u32 = 1;

// --- The typed answer -----------------------------------------------------------------------

/// One answer: the head line and the blocks that follow it, in this order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    /// `surf 1 session <id> <state>`, always the first line.
    pub head: Head,
    /// `token <64 hexadecimal digits>`, in the answer of `open` only.
    pub token: Option<String>,
    /// `did <command> ["<argument>"]` or `did refused <reason>`.
    pub did: Option<Did>,
    /// `page none`, `page <status> "<media type>" "<address>" ["<title>"]` or
    /// `page failed <reason> "<address>"`.
    pub page: Option<Page>,
    /// `path <shown> of <total>` and one indented line per action shown.
    pub path: Option<Path>,
    /// `note` lines, in order.
    pub notes: Vec<Note>,
}

impl Answer {
    /// An answer of the head line alone.
    pub fn new(head: Head) -> Self {
        Answer {
            head,
            token: None,
            did: None,
            page: None,
            path: None,
            notes: Vec::new(),
        }
    }
}

/// The head line: the session and its state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Head {
    /// The session identity, a word of `[0-9a-z-]+`.
    pub session: String,
    /// The state of the session.
    pub state: SessionState,
}

/// The state on the head line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState {
    /// `open`.
    Open,
    /// `closed`: the session was closed.
    Closed,
    /// `died`: the session process ended under `surfd` without `close`.
    Died,
    /// `unknown`: an identity `surfd` does not list.
    Unknown,
}

impl SessionState {
    const ALL: [SessionState; 4] = [
        SessionState::Open,
        SessionState::Closed,
        SessionState::Died,
        SessionState::Unknown,
    ];

    /// The word of the format.
    pub fn word(self) -> &'static str {
        match self {
            SessionState::Open => "open",
            SessionState::Closed => "closed",
            SessionState::Died => "died",
            SessionState::Unknown => "unknown",
        }
    }
}

/// The `did` line: the action the answer is for, or the refusal of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Did {
    /// `did <command>` or `did <command> "<argument>"`. The command is a word of `[a-z]+`
    /// other than `refused`.
    Action {
        /// The command word, such as `go`.
        command: String,
        /// The argument, quoted on the line; `None` for a command given none.
        argument: Option<String>,
    },
    /// `did refused <reason>`.
    Refused(RefusalReason),
}

/// The reasons of `did refused`, a fixed list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusalReason {
    /// `token`: the token does not belong to the session.
    Token,
    /// `closed`: the session is closed.
    Closed,
    /// `usage`: the command line is not one of the command.
    Usage,
    /// `no-such-session`: `surfd` lists no session of that identity.
    NoSuchSession,
    /// `session-died`: the session process ended without `close`.
    SessionDied,
    /// `session-timeout`: the session process did not answer in time.
    SessionTimeout,
}

impl RefusalReason {
    const ALL: [RefusalReason; 6] = [
        RefusalReason::Token,
        RefusalReason::Closed,
        RefusalReason::Usage,
        RefusalReason::NoSuchSession,
        RefusalReason::SessionDied,
        RefusalReason::SessionTimeout,
    ];

    /// The word of the format.
    pub fn word(self) -> &'static str {
        match self {
            RefusalReason::Token => "token",
            RefusalReason::Closed => "closed",
            RefusalReason::Usage => "usage",
            RefusalReason::NoSuchSession => "no-such-session",
            RefusalReason::SessionDied => "session-died",
            RefusalReason::SessionTimeout => "session-timeout",
        }
    }
}

/// The `page` line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    /// `page none`: nothing was loaded yet.
    None,
    /// `page <status> "<media type>" "<address>"`, then `"<title>"` when the page has one.
    Loaded {
        /// The response status.
        status: u16,
        /// The media type, from the server.
        media_type: String,
        /// The address of the page.
        address: String,
        /// The title, from the page.
        title: Option<String>,
    },
    /// `page failed <reason> "<address>"`. The reason is a word of `[A-Za-z]+`.
    Failed {
        /// Why the load failed, such as `Unreachable`.
        reason: String,
        /// The address that failed.
        address: String,
    },
}

/// The `path` block: the last actions of the session, oldest first, and their total.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    /// How many actions the session has taken; at least `entries.len()`.
    pub total: u64,
    /// The actions shown, one indented line each.
    pub entries: Vec<PathEntry>,
}

/// One indented line of the `path` block: `  <n> <command>` or `  <n> <command> "<argument>"`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathEntry {
    /// The action's sequence number in the session.
    pub sequence: u64,
    /// The command word, `[a-z]+`.
    pub command: String,
    /// The argument, quoted on the line; `None` for a command given none.
    pub argument: Option<String>,
}

/// A `note` line, from a fixed list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `note parser: <parts>`: the parts of the tree builder not implemented yet, at least one,
    /// each a word of `[a-z-]+`, joined by `,`.
    Parser(Vec<String>),
}

// --- Quoting ------------------------------------------------------------------------------

/// Write one value from a page, a server or an address as a quoted field.
///
/// Inside the double quotes, `"`, `\` and every character outside U+0020 to U+007E are written
/// as `\x` and two lower-case hexadecimal digits per UTF-8 byte; every other character is
/// written as itself.
pub fn quote(value: &str) -> String {
    let mut field = String::with_capacity(value.len() + 2);
    field.push('"');
    for character in value.chars() {
        if character == '"' || character == '\\' || !matches!(character, ' '..='~') {
            let mut bytes = [0u8; 4];
            for byte in character.encode_utf8(&mut bytes).bytes() {
                field.push_str(&format!("\\x{byte:02x}"));
            }
        } else {
            field.push(character);
        }
    }
    field.push('"');
    field
}

/// Write one value with a length limit of `limit` characters (Unicode scalar values).
///
/// A value of at most `limit` characters is written as [`quote`] writes it. A longer one is cut
/// to its first `limit` characters before escaping, quoted, and followed by `...` after the
/// closing quote, so the mark cannot be confused with dots the value holds.
pub fn quote_cut(value: &str, limit: usize) -> String {
    match value.char_indices().nth(limit) {
        Some((end, _)) => {
            let mut field = quote(&value[..end]);
            field.push_str("...");
            field
        }
        None => quote(value),
    }
}

/// A field read back by [`unquote_cut`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cut {
    /// The value as written, the kept part when it was cut.
    pub kept: String,
    /// Whether the value was cut on its length limit.
    pub cut: bool,
}

/// Why a quoted field was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnquoteError {
    /// The field does not start with `"`.
    NoOpeningQuote,
    /// The field has no closing `"`.
    NoClosingQuote,
    /// Something follows the closing `"`.
    AfterClosingQuote,
    /// A `\` not followed by `x` and two hexadecimal digits.
    ShortEscape,
    /// A hexadecimal digit in upper case.
    UpperCaseHex,
    /// A byte outside U+0020 to U+007E between the quotes.
    Unprintable,
    /// The escaped bytes are not UTF-8.
    NotUtf8,
    /// A character written escaped that [`quote`] writes as itself.
    NotCanonical,
    /// A cut field whose kept part is not exactly its limit, or an uncut one longer than it.
    Length,
}

impl fmt::Display for UnquoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            UnquoteError::NoOpeningQuote => "the field does not start with a double quote",
            UnquoteError::NoClosingQuote => "the field has no closing double quote",
            UnquoteError::AfterClosingQuote => "something follows the closing double quote",
            UnquoteError::ShortEscape => "a backslash is not followed by x and two digits",
            UnquoteError::UpperCaseHex => "a hexadecimal digit is in upper case",
            UnquoteError::Unprintable => "a byte outside U+0020 to U+007E is between the quotes",
            UnquoteError::NotUtf8 => "the escaped bytes are not UTF-8",
            UnquoteError::NotCanonical => "a character is escaped that is written as itself",
            UnquoteError::Length => "the field's length does not match its limit",
        })
    }
}

impl std::error::Error for UnquoteError {}

/// Read one quoted field, the whole of `field`, back to its exact value.
///
/// Refuses anything [`quote`] cannot have written.
pub fn unquote(field: &str) -> Result<String, UnquoteError> {
    let bytes = field.as_bytes();
    if bytes.first() != Some(&b'"') {
        return Err(UnquoteError::NoOpeningQuote);
    }
    let mut value = Vec::with_capacity(bytes.len());
    let mut i = 1;
    loop {
        let Some(&byte) = bytes.get(i) else {
            return Err(UnquoteError::NoClosingQuote);
        };
        match byte {
            b'"' => {
                if i + 1 != bytes.len() {
                    return Err(UnquoteError::AfterClosingQuote);
                }
                break;
            }
            b'\\' => {
                if bytes.get(i + 1) != Some(&b'x') {
                    return Err(UnquoteError::ShortEscape);
                }
                let high = hex_digit(bytes.get(i + 2).copied())?;
                let low = hex_digit(bytes.get(i + 3).copied())?;
                value.push((high << 4) | low);
                i += 4;
            }
            b' '..=b'~' => {
                value.push(byte);
                i += 1;
            }
            _ => return Err(UnquoteError::Unprintable),
        }
    }
    let value = String::from_utf8(value).map_err(|_| UnquoteError::NotUtf8)?;
    if quote(&value) != field {
        return Err(UnquoteError::NotCanonical);
    }
    Ok(value)
}

fn hex_digit(byte: Option<u8>) -> Result<u8, UnquoteError> {
    match byte {
        Some(digit @ b'0'..=b'9') => Ok(digit - b'0'),
        Some(digit @ b'a'..=b'f') => Ok(digit - b'a' + 10),
        Some(b'A'..=b'F') => Err(UnquoteError::UpperCaseHex),
        _ => Err(UnquoteError::ShortEscape),
    }
}

/// Read one field written by [`quote_cut`] with the same `limit`: the kept part, and whether
/// the value was cut.
pub fn unquote_cut(field: &str, limit: usize) -> Result<Cut, UnquoteError> {
    let (quoted, cut) = match field.strip_suffix("...") {
        Some(quoted) if quoted.ends_with('"') && quoted.len() >= 2 => (quoted, true),
        _ => (field, false),
    };
    let kept = unquote(quoted)?;
    let length = kept.chars().count();
    if (cut && length != limit) || (!cut && length > limit) {
        return Err(UnquoteError::Length);
    }
    Ok(Cut { kept, cut })
}

// --- Rendering ------------------------------------------------------------------------------

/// Why an answer was not rendered: a word of the format outside its character set, which the
/// renderer refuses rather than write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderError {
    /// A word field holds a character outside its set, or is empty.
    NotAWord {
        /// Which field, such as `note parser part`.
        field: &'static str,
        /// The value, as [`quote`] writes it.
        value: String,
    },
    /// The `did` command is `refused`, which would read as a refusal.
    ReservedCommand,
    /// The token is not 64 lower-case hexadecimal digits.
    TokenForm,
    /// The `path` block shows more actions than its total.
    PathLongerThanTotal,
    /// A `note parser` line with no part.
    EmptyNote,
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::NotAWord { field, value } => {
                write!(f, "the {field} {value} is outside its character set")
            }
            RenderError::ReservedCommand => f.write_str("the did command is the word refused"),
            RenderError::TokenForm => {
                f.write_str("the token is not 64 lower-case hexadecimal digits")
            }
            RenderError::PathLongerThanTotal => {
                f.write_str("the path shows more actions than its total")
            }
            RenderError::EmptyNote => f.write_str("a note parser line has no part"),
        }
    }
}

impl std::error::Error for RenderError {}

/// Render an answer to its lines, each ending in a line feed.
pub fn render(answer: &Answer) -> Result<String, RenderError> {
    let mut out = String::new();
    render_head(&answer.head, &mut out)?;
    if let Some(token) = &answer.token {
        render_token(token, &mut out)?;
    }
    if let Some(did) = &answer.did {
        render_did(did, &mut out)?;
    }
    if let Some(page) = &answer.page {
        render_page(page, &mut out)?;
    }
    if let Some(path) = &answer.path {
        render_path(path, &mut out)?;
    }
    for note in &answer.notes {
        render_note(note, &mut out)?;
    }
    Ok(out)
}

fn render_head(head: &Head, out: &mut String) -> Result<(), RenderError> {
    let session = checked(&head.session, "session", is_session_char)?;
    out.push_str(&format!(
        "surf {VERSION} session {session} {}\n",
        head.state.word()
    ));
    Ok(())
}

fn render_token(token: &str, out: &mut String) -> Result<(), RenderError> {
    if !is_token(token) {
        return Err(RenderError::TokenForm);
    }
    out.push_str(&format!("token {token}\n"));
    Ok(())
}

fn render_did(did: &Did, out: &mut String) -> Result<(), RenderError> {
    match did {
        Did::Action { command, argument } => {
            let command = checked(command, "did command", is_command_char)?;
            if command == "refused" {
                return Err(RenderError::ReservedCommand);
            }
            out.push_str("did ");
            out.push_str(command);
            push_argument(argument.as_deref(), out);
        }
        Did::Refused(reason) => {
            out.push_str("did refused ");
            out.push_str(reason.word());
        }
    }
    out.push('\n');
    Ok(())
}

fn render_page(page: &Page, out: &mut String) -> Result<(), RenderError> {
    match page {
        Page::None => out.push_str("page none"),
        Page::Loaded {
            status,
            media_type,
            address,
            title,
        } => {
            out.push_str(&format!(
                "page {status} {} {}",
                quote(media_type),
                quote(address)
            ));
            push_argument(title.as_deref(), out);
        }
        Page::Failed { reason, address } => {
            let reason = checked(reason, "page failed reason", |c| c.is_ascii_alphabetic())?;
            out.push_str(&format!("page failed {reason} {}", quote(address)));
        }
    }
    out.push('\n');
    Ok(())
}

fn render_path(path: &Path, out: &mut String) -> Result<(), RenderError> {
    let shown = path.entries.len() as u64;
    if shown > path.total {
        return Err(RenderError::PathLongerThanTotal);
    }
    out.push_str(&format!("path {shown} of {}\n", path.total));
    for entry in &path.entries {
        let command = checked(&entry.command, "path command", is_command_char)?;
        out.push_str(&format!("  {} {command}", entry.sequence));
        push_argument(entry.argument.as_deref(), out);
        out.push('\n');
    }
    Ok(())
}

fn render_note(note: &Note, out: &mut String) -> Result<(), RenderError> {
    match note {
        Note::Parser(parts) => {
            if parts.is_empty() {
                return Err(RenderError::EmptyNote);
            }
            for part in parts {
                checked(part, "note parser part", is_part_char)?;
            }
            out.push_str("note parser: ");
            out.push_str(&parts.join(","));
        }
    }
    out.push('\n');
    Ok(())
}

/// ` "<value>"` when there is a value.
fn push_argument(value: Option<&str>, out: &mut String) {
    if let Some(value) = value {
        out.push(' ');
        out.push_str(&quote(value));
    }
}

fn checked<'a>(
    value: &'a str,
    field: &'static str,
    allowed: fn(char) -> bool,
) -> Result<&'a str, RenderError> {
    if is_word(value, allowed) {
        Ok(value)
    } else {
        Err(RenderError::NotAWord {
            field,
            value: quote(value),
        })
    }
}

fn is_word(value: &str, allowed: fn(char) -> bool) -> bool {
    !value.is_empty() && value.chars().all(allowed)
}

fn is_session_char(c: char) -> bool {
    c.is_ascii_digit() || c.is_ascii_lowercase() || c == '-'
}

fn is_command_char(c: char) -> bool {
    c.is_ascii_lowercase()
}

fn is_part_char(c: char) -> bool {
    c.is_ascii_lowercase() || c == '-'
}

fn is_token(token: &str) -> bool {
    token.len() == 64
        && token
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

// --- Parsing --------------------------------------------------------------------------------

/// Why lines were refused as an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// The line, counted from 1.
    pub line: usize,
    /// What is wrong with it.
    pub kind: ParseErrorKind,
}

/// What is wrong with a refused line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// The first line is not a head line, or there is no line.
    NoHead,
    /// The line is outside the grammar, or out of the order of the blocks.
    OutsideGrammar,
    /// `did refused` with a reason not in the list.
    UnknownRefusalReason,
    /// A quoted field [`unquote`] refuses.
    Field(UnquoteError),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: ", self.line)?;
        match self.kind {
            ParseErrorKind::NoHead => f.write_str("the answer does not start with its head line"),
            ParseErrorKind::OutsideGrammar => f.write_str("the line is outside the grammar"),
            ParseErrorKind::UnknownRefusalReason => {
                f.write_str("the refusal reason is not in the list")
            }
            ParseErrorKind::Field(error) => write!(f, "a quoted field: {error}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Read the lines of an answer back to the typed answer.
///
/// Accepts exactly what [`render`] writes: every line ends in a line feed, the head line comes
/// first, the blocks follow in their order, and a line outside the grammar is refused with its
/// number.
pub fn parse(text: &str) -> Result<Answer, ParseError> {
    let mut lines = Lines::new(text)?;
    let Some(first) = lines.next() else {
        return Err(ParseError {
            line: 1,
            kind: ParseErrorKind::NoHead,
        });
    };
    let head = parse_head(first)?;
    let mut answer = Answer::new(head);
    for block in BLOCKS {
        block(&mut lines, &mut answer)?;
    }
    match lines.next() {
        None => Ok(answer),
        Some(line) => Err(line.error(ParseErrorKind::OutsideGrammar)),
    }
}

/// The blocks after the head line, in their order. Each reads its lines when the next line is
/// one of its own and leaves the lines alone otherwise.
type Block = fn(&mut Lines<'_>, &mut Answer) -> Result<(), ParseError>;

const BLOCKS: [Block; 5] = [parse_token, parse_did, parse_page, parse_path, parse_notes];

fn parse_head(line: Line<'_>) -> Result<Head, ParseError> {
    let mut cursor = line.cursor();
    let no_head = |_| line.error(ParseErrorKind::NoHead);
    cursor.literal("surf ").map_err(no_head)?;
    let rest = line.text.get(cursor.at..).unwrap_or("");
    let version = format!("{VERSION} session ");
    if !rest.starts_with(&version) {
        return Err(line.error(ParseErrorKind::OutsideGrammar));
    }
    cursor.at += version.len();
    let session = cursor.word(is_session_char)?.to_string();
    cursor.space()?;
    let state_word = cursor.word(|c| c.is_ascii_lowercase())?;
    let state = SessionState::ALL
        .into_iter()
        .find(|state| state.word() == state_word)
        .ok_or_else(|| line.error(ParseErrorKind::OutsideGrammar))?;
    cursor.end()?;
    Ok(Head { session, state })
}

fn parse_token(lines: &mut Lines<'_>, answer: &mut Answer) -> Result<(), ParseError> {
    let Some(line) = lines.take_if_starts("token ") else {
        return Ok(());
    };
    let token = &line.text["token ".len()..];
    if !is_token(token) {
        return Err(line.error(ParseErrorKind::OutsideGrammar));
    }
    answer.token = Some(token.to_string());
    Ok(())
}

fn parse_did(lines: &mut Lines<'_>, answer: &mut Answer) -> Result<(), ParseError> {
    let Some(line) = lines.take_if_starts("did ") else {
        return Ok(());
    };
    let mut cursor = line.cursor();
    cursor.at = "did ".len();
    let command = cursor.word(is_command_char)?;
    if command == "refused" {
        cursor.space()?;
        let word = cursor.word(|c| c.is_ascii_graphic())?;
        cursor.end()?;
        let reason = RefusalReason::ALL
            .into_iter()
            .find(|reason| reason.word() == word)
            .ok_or_else(|| line.error(ParseErrorKind::UnknownRefusalReason))?;
        answer.did = Some(Did::Refused(reason));
    } else {
        let command = command.to_string();
        let argument = cursor.optional_quoted()?;
        cursor.end()?;
        answer.did = Some(Did::Action { command, argument });
    }
    Ok(())
}

fn parse_page(lines: &mut Lines<'_>, answer: &mut Answer) -> Result<(), ParseError> {
    let Some(line) = lines.take_if_starts("page ") else {
        return Ok(());
    };
    let mut cursor = line.cursor();
    cursor.at = "page ".len();
    let page = if line.text == "page none" {
        cursor.at = line.text.len();
        Page::None
    } else if line.text.starts_with("page failed ") {
        cursor.at = "page failed ".len();
        let reason = cursor.word(|c| c.is_ascii_alphabetic())?.to_string();
        cursor.space()?;
        let address = cursor.quoted()?;
        Page::Failed { reason, address }
    } else {
        let status = cursor.number()?;
        cursor.space()?;
        let media_type = cursor.quoted()?;
        cursor.space()?;
        let address = cursor.quoted()?;
        let title = cursor.optional_quoted()?;
        Page::Loaded {
            status,
            media_type,
            address,
            title,
        }
    };
    cursor.end()?;
    answer.page = Some(page);
    Ok(())
}

fn parse_path(lines: &mut Lines<'_>, answer: &mut Answer) -> Result<(), ParseError> {
    let Some(line) = lines.take_if_starts("path ") else {
        return Ok(());
    };
    let mut cursor = line.cursor();
    cursor.at = "path ".len();
    let shown: u64 = cursor.number()?;
    cursor.literal(" of ")?;
    let total: u64 = cursor.number()?;
    cursor.end()?;
    if shown > total {
        return Err(line.error(ParseErrorKind::OutsideGrammar));
    }
    let mut entries = Vec::new();
    for _ in 0..shown {
        let Some(entry_line) = lines.take_if_starts("  ") else {
            let number = lines.number();
            return Err(ParseError {
                line: number,
                kind: ParseErrorKind::OutsideGrammar,
            });
        };
        let mut cursor = entry_line.cursor();
        cursor.at = 2;
        let sequence = cursor.number()?;
        cursor.space()?;
        let command = cursor.word(is_command_char)?.to_string();
        let argument = cursor.optional_quoted()?;
        cursor.end()?;
        entries.push(PathEntry {
            sequence,
            command,
            argument,
        });
    }
    answer.path = Some(Path { total, entries });
    Ok(())
}

fn parse_notes(lines: &mut Lines<'_>, answer: &mut Answer) -> Result<(), ParseError> {
    while let Some(line) = lines.take_if_starts("note ") {
        let Some(parts) = line.text.strip_prefix("note parser: ") else {
            return Err(line.error(ParseErrorKind::OutsideGrammar));
        };
        let parts: Vec<String> = parts.split(',').map(str::to_string).collect();
        if !parts.iter().all(|part| is_word(part, is_part_char)) {
            return Err(line.error(ParseErrorKind::OutsideGrammar));
        }
        answer.notes.push(Note::Parser(parts));
    }
    Ok(())
}

/// The lines of an answer, each with its number.
struct Lines<'a> {
    lines: Vec<&'a str>,
    next: usize,
}

impl<'a> Lines<'a> {
    fn new(text: &'a str) -> Result<Self, ParseError> {
        if text.is_empty() {
            return Ok(Lines {
                lines: Vec::new(),
                next: 0,
            });
        }
        let Some(body) = text.strip_suffix('\n') else {
            return Err(ParseError {
                line: text.split('\n').count(),
                kind: ParseErrorKind::OutsideGrammar,
            });
        };
        Ok(Lines {
            lines: body.split('\n').collect(),
            next: 0,
        })
    }

    /// The number the next line has.
    fn number(&self) -> usize {
        self.next + 1
    }

    fn next(&mut self) -> Option<Line<'a>> {
        let text = *self.lines.get(self.next)?;
        self.next += 1;
        Some(Line {
            text,
            number: self.next,
        })
    }

    fn take_if_starts(&mut self, prefix: &str) -> Option<Line<'a>> {
        if self.lines.get(self.next)?.starts_with(prefix) {
            self.next()
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
struct Line<'a> {
    text: &'a str,
    number: usize,
}

impl<'a> Line<'a> {
    fn error(&self, kind: ParseErrorKind) -> ParseError {
        ParseError {
            line: self.number,
            kind,
        }
    }

    fn cursor(&self) -> Cursor<'a> {
        Cursor { line: *self, at: 0 }
    }
}

/// A position in one line, reading it left to right.
struct Cursor<'a> {
    line: Line<'a>,
    at: usize,
}

impl<'a> Cursor<'a> {
    fn rest(&self) -> &'a str {
        self.line.text.get(self.at..).unwrap_or("")
    }

    fn outside(&self) -> ParseError {
        self.line.error(ParseErrorKind::OutsideGrammar)
    }

    fn literal(&mut self, expected: &str) -> Result<(), ParseError> {
        if self.rest().starts_with(expected) {
            self.at += expected.len();
            Ok(())
        } else {
            Err(self.outside())
        }
    }

    fn space(&mut self) -> Result<(), ParseError> {
        self.literal(" ")
    }

    fn end(&self) -> Result<(), ParseError> {
        if self.rest().is_empty() {
            Ok(())
        } else {
            Err(self.outside())
        }
    }

    /// A word of the format, up to the next space or the end of the line.
    fn word(&mut self, allowed: fn(char) -> bool) -> Result<&'a str, ParseError> {
        let rest = self.rest();
        let word = rest.split(' ').next().unwrap_or("");
        if !is_word(word, allowed) {
            return Err(self.outside());
        }
        self.at += word.len();
        Ok(word)
    }

    /// A number in decimal, without a sign and without a leading zero.
    fn number<T: std::str::FromStr>(&mut self) -> Result<T, ParseError> {
        let digits = self.word(|c| c.is_ascii_digit())?;
        if digits.len() > 1 && digits.starts_with('0') {
            return Err(self.outside());
        }
        digits.parse().map_err(|_| self.outside())
    }

    /// One quoted field: from `"` to the next `"`.
    fn quoted(&mut self) -> Result<String, ParseError> {
        let rest = self.rest();
        if !rest.starts_with('"') {
            return Err(self.outside());
        }
        let end = match rest[1..].find('"') {
            Some(close) => close + 2,
            None => rest.len(),
        };
        let value =
            unquote(&rest[..end]).map_err(|error| self.line.error(ParseErrorKind::Field(error)))?;
        self.at += end;
        Ok(value)
    }

    /// ` "<value>"` when the line goes on, nothing at its end.
    fn optional_quoted(&mut self) -> Result<Option<String>, ParseError> {
        if self.rest().is_empty() {
            return Ok(None);
        }
        self.space()?;
        self.quoted().map(Some)
    }
}
